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
use concorddata::concord::DSContext;
use concorddata::concord::ServerInfo as DataServerInfo;
use concorddata::concord::AUTH_FLAG_OWNER;
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use std::fs::File;
use std::io::Read;
use std::io::Write;

use crate::types::{ConnectionInfo, Event, EventType, GetServersResponse, ServerInfo};
use crate::{owner, send};
use concorddata::types::{Pubkey, ServerId};

const NOT_AUTHORIZED: &str = "{\"error\": \"not authorized\"}";
const MAIN_LOG: &str = "mainlog";

info!();

#[derive(Serialize, Deserialize)]
struct ServerInfoMin {
	name: String,
	server_pubkey: String,
	id: String,
}

fn _get_icon(
	server_id: [u8; 8],
	pubkey: [u8; 32],
	root_dir: String,
) -> Result<Vec<u8>, ConcordError> {
	let server_id = ServerId::from_bytes(server_id).to_base58()?;
	let pubkey = Pubkey::from_bytes(pubkey).to_base58()?;
	let file_name = format!(
		"{}/www/images/user_images/{}-{}",
		root_dir, server_id, pubkey
	);
	error!("start read: {}", file_name);
	let start = std::time::Instant::now();
	let mut f = File::open(&file_name)?;
	let metadata = std::fs::metadata(&file_name)?;
	let mut data = vec![0; metadata.len() as usize];
	f.read(&mut data)?;
	error!(
		"end read of {} bytes, time = {}",
		metadata.len(),
		start.elapsed().as_nanos()
	);

	Ok(data)
}

fn set_icon(
	root_dir: String,
	server_id: [u8; 8],
	pubkey: [u8; 32],
	icon: Vec<u8>,
) -> Result<(), ConcordError> {
	let server_id = ServerId::from_bytes(server_id).to_base58()?;
	let pubkey = Pubkey::from_bytes(pubkey).to_base58()?;
	let file_name = format!(
		"{}/www/images/user_images/{}-{}",
		root_dir, server_id, pubkey
	);

	let mut file = std::fs::OpenOptions::new()
		.write(true)
		.create_new(true)
		.truncate(true)
		.open(file_name)?;
	file.write_all(&icon)?;
	Ok(())
}

pub fn get_servers(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
) -> Result<bool, ConcordError> {
	owner!(conn_info);

	let mut servers = vec![];
	let data = ds_context.get_servers()?;

	let now = std::time::Instant::now();
	for d in data {
		servers.push(ServerInfo {
			name: d.name.into(),
			description: "none".into(),
			//icon: get_icon(d.server_id, d.pubkey)?.into(),
			server_id: d.server_id.into(),
			server_pubkey: Pubkey::from_bytes(d.pubkey),
			seqno: d.seqno,
		});
	}
	error!(
		"end of adding ServerInfo, time = {}",
		now.elapsed().as_nanos()
	);

	let event = Event {
		event_type: EventType::GetServersResponse,
		get_servers_response: Some(GetServersResponse { servers }).into(),
		..Default::default()
	};
	error!("end of building event, time = {}", now.elapsed().as_nanos());
	send!(conn_info.handle, event);
	error!("end of sending event, time = {}", now.elapsed().as_nanos());

	Ok(false)
}

pub fn create_server(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
	config: &ConcordConfig,
) -> Result<bool, ConcordError> {
	owner!(conn_info);

	let icon = match event.create_server_event.0.as_ref() {
		Some(event) => event.icon.clone(),
		None => {
			warn!("Malformed create server event. No icon: {:?}", event);
			return Ok(true);
		}
	};

	let name = match event.create_server_event.0.as_ref() {
		Some(event) => event.name.data.clone(),
		None => {
			warn!("Malformed create server event. No name: {:?}", event);
			return Ok(true);
		}
	};

	let pubkey = pubkey!();
	let data_server_info = DataServerInfo {
		pubkey,
		name,
		joined: true,
		seqno: 1,
	};

	let server_id = ds_context.add_server(data_server_info, None, None, false)?;

	set_icon(config.root_dir.clone(), server_id, pubkey, icon)?;

	info!("create server complete");

	Ok(false)
}

pub fn delete_server(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	owner!(conn_info);

	let (server_id, server_pubkey) = match event.delete_server_event.0.as_ref() {
		Some(event) => (event.server_id.to_bytes(), event.server_pubkey.to_bytes()),
		None => {
			warn!(
				"Malformed delete server event. No server_id/server_pubkey: {:?}",
				event
			);
			return Ok(true);
		}
	};

	ds_context.delete_server(server_id, server_pubkey)?;

	Ok(false)
}

pub fn modify_server(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
	config: &ConcordConfig,
) -> Result<bool, ConcordError> {
	owner!(conn_info);

	let (server_id, server_pubkey, name, icon) = match event.modify_server_event.0.as_ref() {
		Some(event) => (
			event.server_id.to_bytes(),
			event.server_pubkey.to_bytes(),
			event.name.clone(),
			event.icon.clone(),
		),
		None => {
			warn!(
				"Malformed modify server event. No server_id/server_pubkey: {:?}",
				event
			);
			return Ok(true);
		}
	};

	match icon.0 {
		Some(icon) => {
			set_icon(config.root_dir.clone(), server_id, server_pubkey, icon.data)?;
		}
		None => {}
	}

	match name.0 {
		Some(name) => ds_context.modify_server(server_id, server_pubkey, name.to_string())?,
		None => {}
	}

	Ok(false)
}

pub fn init_server(config: &ConcordConfig, _context: ConcordContext) -> Result<(), ConcordError> {
	/*
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
						let pubkey = pubkey!();
						let server_info = DataServerInfo {
							pubkey,
							name: name.clone(),
							icon: buf,
							joined: true,
							seqno: 1,
						};

						let server_id = ds_context
							.add_server(server_info, None, None, false)
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
	*/

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

	/*
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

			let server_id = query!("server_id").unwrap_or("".to_string());

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

			let server_id = query!("server_id").unwrap_or("".to_string());

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
						let pubkey = pubkey!();
						let server_info = DataServerInfo {
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
	*/

	Ok(())
}
