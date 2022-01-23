// Copyright 2022 37 Miners, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::auth::check_auth;
use crate::context::ConcordContext;
use concordconfig::ConcordConfig;
use concorddata::concord::Channel;
use concorddata::concord::ChannelKey;
use concorddata::concord::DSContext;
use concorddata::concord::ServerInfo;
use concorddata::concord::{AUTH_FLAG_MEMBER, AUTH_FLAG_OWNER};
use concorderror::Error as ConcordError;
use librustlet::*;
use nioruntime_log::*;

use std::fs::File;
use std::io::Read;

const NOT_AUTHORIZED: &str = "{\"error\": \"not authorized\"}";
const MAIN_LOG: &str = "mainlog";

nioruntime_log::debug!(); // set log level to debug

#[derive(Serialize, Deserialize)]
struct ServerInfoMin {
	name: String,
	server_pubkey: String,
	id: String,
}

pub fn init_server(config: &ConcordConfig, _context: ConcordContext) -> Result<(), ConcordError> {
	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	// create a server on this concord instance
	rustlet!("create_server", {
		// make sure we're authenticated
		let res = check_auth(&ds_context, AUTH_FLAG_OWNER);
		match res {
			Ok(_) => {}
			Err(e) => {
				log_multi!(ERROR, MAIN_LOG, "auth error: {}", e);
				response!("{}", NOT_AUTHORIZED);
				return Ok(());
			}
		}

		let server_pubkey = pubkey!();
		if server_pubkey.is_none() {
			response!("tor not configured!");
			return Ok(());
		}
		let server_pubkey = server_pubkey.unwrap();

		// get query parameters
		let query = request!("query");
		let query_vec = querystring::querify(&query);
		let mut name = "".to_string();
		for query_param in query_vec {
			if query_param.0 == "name" {
				name = query_param.1.to_string();
				break;
			}
		}

		let content = request_content!();
		let content = &mut &content[..];
		let mut headers = hyper::header::Headers::new();
		for i in 0..header_len!() {
			headers.append_raw(header_name!(i), header_value!(i).as_bytes().to_vec());
		}
		// parse the mime_multipart data in this request
		let res = mime_multipart::read_multipart_body(content, &headers, false).unwrap_or(vec![]);
		for node in &res {
			match node {
				mime_multipart::Node::File(filepart) => {
					let mut f = File::open(&filepart.path)?;
					let size = filepart.size.unwrap_or(0);
					let mut buf = vec![0 as u8; size];
					f.read(&mut buf)?;
					let pubkey = pubkey!().unwrap_or([0u8; 32]);
					let server_info = ServerInfo {
						pubkey,
						name: name.clone(),
						icon: buf,
						joined: true,
					};

					let server_id =
						ds_context
							.add_server(server_info, None, None)
							.map_err(|e| {
								let error: Error = ErrorKind::ApplicationError(format!(
									"error adding server: {}",
									e.to_string()
								))
								.into();
								error
							})?;

					let channel_key = ChannelKey {
						server_pubkey,
						server_id,
						channel_id: 0,
					};
					let channel = Channel {
						name: "mainchat".to_string(),
						description: "Welcome to mainchat!".to_string(),
						channel_id: 0,
					};

					ds_context.set_channel(channel_key, channel).map_err(|e| {
						let error: Error = ErrorKind::ApplicationError(format!(
							"error adding channel: {}",
							e.to_string()
						))
						.into();
						error
					})?;

					break;
				}
				_ => {}
			}
		}
	});
	rustlet_mapping!("/create_server", "create_server");

	// create a new context for each rustlet, synchronization handled by batches
	let ds_context = DSContext::new(config.root_dir.clone())?;

	// get all servers associated with this instance of concord
	rustlet!("get_servers", {
		// make sure we're authenticated
		let res = check_auth(&ds_context, AUTH_FLAG_OWNER);
		match res {
			Ok(_) => {}
			Err(e) => {
				log_multi!(ERROR, MAIN_LOG, "auth error: {}", e);
				response!("{}", NOT_AUTHORIZED);
				return Ok(());
			}
		}

		let servers = ds_context.get_servers().map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("Error getting servers: {}", e.to_string()))
					.into();
			error
		})?;

		let mut server_json = vec![];
		for server in servers {
			let server_id = base64::encode(server.server_id);
			let server_id = urlencoding::encode(&server_id).to_string();
			let server_pubkey = base64::encode(server.pubkey);
			let server_pubkey = urlencoding::encode(&server_pubkey).to_string();
			server_json.push(ServerInfoMin {
				name: server.name.clone(),
				server_pubkey,
				id: server_id,
			});
		}
		let json = serde_json::to_string(&server_json).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string())).into();
			error
		})?;
		response!("{}", json);
	});
	rustlet_mapping!("/get_servers", "get_servers");

	// create a new context for each rustlet, synchronization handled by batches
	let ds_context = DSContext::new(config.root_dir.clone())?;

	rustlet!("get_server_info", {
		// make sure we're authenticated
		let res = check_auth(&ds_context, AUTH_FLAG_OWNER);
		match res {
			Ok(_) => {}
			Err(e) => {
				log_multi!(ERROR, MAIN_LOG, "auth error: {}", e);
				response!("{}", NOT_AUTHORIZED);
				return Ok(());
			}
		}

		let server_id = query!("server_id");

		let sinfo = ds_context.get_server_info(server_id.clone()).map_err(|e| {
			let error: Error = ErrorKind::ApplicationError(format!(
				"error getting server info: {}",
				e.to_string()
			))
			.into();
			error
		})?;
		match sinfo {
			Some(sinfo) => {
				let mut server_json = vec![];
				let server_pubkey = base64::encode(sinfo.pubkey);
				let server_pubkey = urlencoding::encode(&server_pubkey).to_string();
				server_json.push(ServerInfoMin {
					name: sinfo.name.clone(),
					server_pubkey,
					id: server_id,
				});

				let json = serde_json::to_string(&server_json).map_err(|e| {
					let error: Error =
						ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string()))
							.into();
					error
				})?;
				response!("{}", json);
			}
			None => {}
		}
	});
	rustlet_mapping!("/get_server_info", "get_server_info");

	// create a new context for each rustlet, synchronization handled by batches
	let ds_context = DSContext::new(config.root_dir.clone())?;

	// get the icon for the specified server
	rustlet!("get_server_icon", {
		// make sure we're authenticated
		let res = check_auth(&ds_context, AUTH_FLAG_MEMBER);
		match res {
			Ok(_) => {}
			Err(e) => {
				log_multi!(ERROR, MAIN_LOG, "auth error: {}", e);
				response!("{}", NOT_AUTHORIZED);
				return Ok(());
			}
		}

		let server_id = query!("server_id");

		let sinfo = ds_context.get_server_info(server_id).map_err(|e| {
			let error: Error = ErrorKind::ApplicationError(format!(
				"error getting server info: {}",
				e.to_string()
			))
			.into();
			error
		})?;

		match sinfo {
			Some(sinfo) => {
				// write back with binary method
				bin_write!(&sinfo.icon[..]);
			}
			None => {}
		}
	});
	rustlet_mapping!("/get_server_icon", "get_server_icon");

	// create a new context for each rustlet, synchronization handled by batches
	let ds_context = DSContext::new(config.root_dir.clone())?;

	// delete the specified server
	rustlet!("delete_server", {
		// make sure we're authenticated
		let res = check_auth(&ds_context, AUTH_FLAG_OWNER);
		match res {
			Ok(_) => {}
			Err(e) => {
				log_multi!(ERROR, MAIN_LOG, "auth error: {}", e);
				response!("{}", NOT_AUTHORIZED);
				return Ok(());
			}
		}

		// parse query
		let query = request!("query");
		let query_vec = querystring::querify(&query);
		let mut server_id = "".to_string();
		for query_param in query_vec {
			if query_param.0 == "server_id" {
				server_id = query_param.1.to_string();
				break;
			}
		}

		ds_context.delete_server(server_id).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("error deleting server: {}", e.to_string()))
					.into();
			error
		})?;
	});
	rustlet_mapping!("/delete_server", "delete_server");

	// create a new context for each rustlet, synchronization handled by batches
	let ds_context = DSContext::new(config.root_dir.clone())?;

	// modify the specified server
	rustlet!("modify_server", {
		let res = check_auth(&ds_context, AUTH_FLAG_OWNER);
		match res {
			Ok(_) => {}
			Err(e) => {
				log_multi!(ERROR, MAIN_LOG, "auth error: {}", e);
				response!("{}", NOT_AUTHORIZED);
				return Ok(());
			}
		}

		let query = request!("query");
		let query_vec = querystring::querify(&query);
		let mut server_id_str = "".to_string();
		let mut name = "".to_string();
		for query_param in query_vec {
			if query_param.0 == "server_id" {
				server_id_str = query_param.1.to_string();
			} else if query_param.0 == "name" {
				name = query_param.1.to_string();
			}
		}

		let content = request_content!();
		let content = &mut &content[..];
		let mut headers = hyper::header::Headers::new();
		for i in 0..header_len!() {
			headers.append_raw(header_name!(i), header_value!(i).as_bytes().to_vec());
		}
		let res = mime_multipart::read_multipart_body(content, &headers, false).unwrap_or(vec![]);
		for node in &res {
			match node {
				mime_multipart::Node::File(filepart) => {
					let mut f = File::open(&filepart.path)?;
					let size = filepart.size.unwrap_or(0);
					let mut buf = vec![0 as u8; size];
					f.read(&mut buf)?;
					let pubkey = pubkey!().unwrap_or([0u8; 32]);
					let server_info = ServerInfo {
						pubkey,
						name: name.clone(),
						icon: buf,
						joined: true,
					};

					ds_context
						.modify_server(server_id_str.clone(), server_info)
						.map_err(|e| {
							let error: Error = ErrorKind::ApplicationError(format!(
								"error modifying server: {}",
								e.to_string()
							))
							.into();
							error
						})?;
					break;
				}
				_ => {}
			}
		}
	});
	rustlet_mapping!("/modify_server", "modify_server");

	Ok(())
}
