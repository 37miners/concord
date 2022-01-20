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
use crate::context::ServerConnectionInfo;
use crate::persistence::Event;
use crate::persistence::EVENT_TYPE_MESSAGE;
use concordconfig::ConcordConfig;
use concorddata::concord::DSContext;
use concorddata::concord::Message;
use concorddata::concord::MessageType;
use concorddata::concord::AUTH_FLAG_MEMBER;
use concorderror::Error as ConcordError;
use concordutil::torclient;
use librustlet::*;
use nioruntime_log::*;
use nioruntime_tor::ov3::OnionV3Address;
use serde_json::Value;
use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
use url::Host::Domain;
use url::Url;

nioruntime_log::debug!(); // set log level to debug
const MAIN_LOG: &str = "mainlog";

const NOT_AUTHORIZED: &str = "{\"error\": \"not authorized\"}";
const SUCCESS: &str = "{\"success\": true}";

#[derive(Serialize, Deserialize)]
struct MessageInfo {
	text: String,
	verified: bool,
	timestamp: String,
	user_pubkey: String,
}

fn get_auth_token(
	ds_context: &DSContext,
	server_pubkey: [u8; 32],
	tor_port: u16,
	bypass_db: bool,
) -> Result<String, Error> {
	let onion_address = OnionV3Address::from_bytes(server_pubkey);

	if !bypass_db {
		let token = ds_context.get_auth_token(server_pubkey).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("get_auth_token db error: {}", e.to_string()))
					.into();
			error
		})?;

		if token != 0 {
			return Ok(format!("{}", token));
		}
	}

	let pubkey = pubkey!().unwrap_or([0u8; 32]);
	let pubkey = base64::encode(pubkey);
	let pubkey = urlencoding::encode(&pubkey).to_string();

	let challenge_link = format!(
		"http://{}.onion/get_challenge?user_pubkey={}",
		onion_address, pubkey
	);

	let url = Url::parse(&challenge_link).map_err(|e| {
		let error: Error =
			ErrorKind::ApplicationError(format!("url parse error: {}", e.to_string())).into();
		error
	})?;
	let host = format!("{}", url.host().unwrap_or(Domain("notfound")));
	let path = format!("{}?{}", url.path(), url.query().unwrap_or(""));

	let res = torclient::do_get(host.clone(), path.clone(), tor_port).map_err(|e| {
		let error: Error =
			ErrorKind::ApplicationError(format!("tor client error: {}", e.to_string())).into();
		error
	})?;

	let start = res.find("\r\n\r\n");

	let challenge = match start {
		Some(start) => {
			let json_text = &res[(start + 4)..];

			let value: Value = serde_json::from_str(json_text).map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("json parse error3: {}", e)).into();
				error
			})?;

			let challenge = match value.get("challenge") {
				Some(challenge) => challenge.as_str().unwrap_or(""),
				None => "",
			}
			.to_string();

			challenge
		}
		None => {
			return Err(ErrorKind::ApplicationError(
				"server returned invalid response".to_string(),
			)
			.into());
		}
	};

	let challenge_str = challenge.clone();
	let challenge = urlencoding::decode(&challenge)?;
	let challenge = base64::decode(&*challenge)?;
	let challenge: [u8; 8] = challenge.as_slice().try_into()?;

	let signature = sign!(&challenge).unwrap_or([0u8; 64]);
	let signature = base64::encode(&signature);
	let signature = urlencoding::encode(&signature);

	let url_string = format!(
		"http://{}.onion/challenge_auth?user_pubkey={}&challenge={}&signature={}",
		onion_address, pubkey, challenge_str, signature,
	);

	let url = Url::parse(&url_string).map_err(|e| {
		let error: Error =
			ErrorKind::ApplicationError(format!("url parse error: {}", e.to_string())).into();
		error
	})?;
	let host = format!("{}", url.host().unwrap_or(Domain("notfound")));
	let path = format!("{}?{}", url.path(), url.query().unwrap_or(""));

	let res = torclient::do_get(host.clone(), path.clone(), tor_port).map_err(|e| {
		let error: Error =
			ErrorKind::ApplicationError(format!("tor client error: {}", e.to_string())).into();
		error
	})?;

	let start = res.find("\r\n\r\n");

	let token = match start {
		Some(start) => {
			let json_text = &res[(start + 4)..];

			let value: Value = serde_json::from_str(json_text).map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("json parse error4: {}", e)).into();
				error
			})?;

			let token = match value.get("token") {
				Some(token) => token.as_str().unwrap_or(""),
				None => "",
			}
			.to_string();

			token
		}
		None => {
			return Err(ErrorKind::ApplicationError(
				"server returned invalid response".to_string(),
			)
			.into());
		}
	};

	ds_context
		.save_auth_token(server_pubkey, token.parse()?)
		.map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("save_auth_token db error: {}", e.to_string()))
					.into();
			error
		})?;

	Ok(token)
}

// build a signable message from a message/key.
fn build_signable_message(message: &Message) -> Result<Vec<u8>, Error> {
	let mut ret = vec![];
	ret.append(&mut message.server_pubkey.to_vec());
	ret.append(&mut message.server_id.to_vec());
	ret.append(&mut message.channel_id.to_be_bytes().to_vec());
	ret.append(&mut message.timestamp.to_be_bytes().to_vec());
	ret.append(&mut message.user_pubkey.to_vec());
	ret.append(&mut message.nonce.to_be_bytes().to_vec());
	ret.append(&mut message.payload.to_vec());
	match message.message_type {
		MessageType::Text => ret.push(0),
		_ => ret.push(1),
	}
	Ok(ret)
}

fn process_remote_messages(
	ds_context: &DSContext,
	server_pubkey: [u8; 32],
	server_id: [u8; 8],
	channel_id: u64,
	tor_port: u16,
) -> Result<(), Error> {
	let server_id = base64::encode(server_id);
	let server_id = urlencoding::encode(&server_id).to_string();
	let user_pubkey = pubkey!().unwrap_or([0u8; 32]);
	let user_pubkey = base64::encode(user_pubkey);
	let user_pubkey = urlencoding::encode(&user_pubkey);

	let onion = OnionV3Address::from_bytes(server_pubkey);
	let mut bypass_db = false;
	loop {
		let token = get_auth_token(ds_context, server_pubkey, tor_port, bypass_db)?;
		let server_pubkey = base64::encode(server_pubkey);
		let server_pubkey = urlencoding::encode(&server_pubkey);

		let message_link = format!(
		"http://{}.onion/query_messages?server_pubkey={}&channel_id={}&server_id={}&user_pubkey={}&token={}",
		onion, server_pubkey, channel_id, server_id, user_pubkey, token,
	);
		let url = Url::parse(&message_link).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("url parse error: {}", e.to_string())).into();
			error
		})?;
		let host = format!("{}", url.host().unwrap_or(Domain("notfound")));
		let path = format!("{}?{}", url.path(), url.query().unwrap_or(""));

		let res = torclient::do_get(host.clone(), path.clone(), tor_port).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("tor client error: {}", e.to_string())).into();
			error
		})?;

		let start = res.find("\r\n\r\n");

		match start {
			Some(start) => {
				let json_text = &res[(start + 4)..];

				let value: Value = serde_json::from_str(json_text).map_err(|e| {
					let error: Error =
						ErrorKind::ApplicationError(format!("json parse error5: {}", e)).into();
					error
				})?;

				match value.get("error") {
					Some(_) => {
						// error occured.
						// get new auth token
						bypass_db = true;
						continue;
					}
					None => {}
				}

				response!("{}", json_text);
			}
			None => {}
		}
		break;
	}

	Ok(())
}

fn process_local_messages(
	ds_context: &DSContext,
	server_id: [u8; 8],
	channel_id: u64,
) -> Result<(), Error> {
	let server_pubkey = pubkey!().unwrap_or([0u8; 32]);

	let mut messages = ds_context
		.get_messages(server_pubkey, server_id, channel_id, u64::MAX)
		.map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("error querying messages: {}", e.to_string()))
					.into();
			error
		})?;

	let messages = if messages.0 > 0 {
		let mut messages2 = ds_context
			.get_messages(server_pubkey, server_id, channel_id, messages.0 - 1)
			.map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(format!(
					"error querying messages: {}",
					e.to_string()
				))
				.into();
				error
			})?;

		messages2.1.append(&mut messages.1);
		messages2.1
	} else {
		messages.1
	};

	let mut message_json = vec![];
	for message in &messages {
		let message_to_sign = build_signable_message(message)?;
		message_json.push(MessageInfo {
			text: std::str::from_utf8(&message.payload.clone())?.to_string(),
			verified: verify!(
				&message_to_sign,
				Some(message.user_pubkey),
				message.signature
			)
			.unwrap_or(false),
			timestamp: format!("{}", message.timestamp),
			user_pubkey: OnionV3Address::from_bytes(message.user_pubkey).to_string(),
		});
	}
	let json = serde_json::to_string(&message_json).map_err(|e| {
		let error: Error =
			ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string())).into();
		error
	})?;
	response!("{}", json);
	Ok(())
}

pub fn init_message(config: &ConcordConfig, context: ConcordContext) -> Result<(), ConcordError> {
	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;
	let tor_port = config.tor_port;

	// send a message
	rustlet!("send_message", {
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

		// get query parameters
		let query = request!("query");
		let query_vec = querystring::querify(&query);

		let pubkey = pubkey!().unwrap_or([0u8; 32]);

		let mut server_pubkey: Option<[u8; 32]> = None;
		let mut user_pubkey: Option<[u8; 32]> = None;
		let mut server_id: Option<[u8; 8]> = None;
		let mut channel_id: Option<u64> = None;
		let mut timestamp: Option<u64> = None;
		let mut nonce: Option<u16> = None;
		let mut payload: Option<Vec<u8>> = None;
		let mut message_type: Option<u8> = None;
		let mut signature: Option<[u8; 64]> = None;

		for query_param in query_vec {
			let param_as_str = query_param.1.to_string();
			if query_param.0 == "server_pubkey" {
				let local_server_pubkey = urlencoding::decode(&param_as_str)?;
				let local_server_pubkey = base64::decode(&*local_server_pubkey)?;
				server_pubkey = Some(local_server_pubkey.as_slice().try_into()?);
			} else if query_param.0 == "user_pubkey" {
				let local_user_pubkey = urlencoding::decode(&param_as_str)?;
				let local_user_pubkey = base64::decode(&*local_user_pubkey)?;
				user_pubkey = Some(local_user_pubkey.as_slice().try_into()?);
			} else if query_param.0 == "server_id" {
				let local_server_id = urlencoding::decode(&param_as_str)?;
				let local_server_id = base64::decode(&*local_server_id)?;
				server_id = Some(local_server_id.as_slice().try_into()?);
			} else if query_param.0 == "signature" {
				let local_signature = urlencoding::decode(&param_as_str)?;
				let local_signature = base64::decode(&*local_signature)?;
				signature = Some(local_signature.as_slice().try_into()?);
			} else if query_param.0 == "channel_id" {
				channel_id = Some(param_as_str.parse()?);
			} else if query_param.0 == "timestamp" {
				timestamp = Some(param_as_str.parse()?);
			} else if query_param.0 == "nonce" {
				nonce = Some(param_as_str.parse()?);
			} else if query_param.0 == "message_type" {
				message_type = Some(param_as_str.parse()?);
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
					payload = Some(buf);
					break;
				}
				mime_multipart::Node::Part(part) => {
					payload = Some(part.body.clone());
				}
				_ => {}
			}
		}

		let message_type = match message_type {
			Some(message_type) => message_type,
			None => 0,
		};

		// for tor connections, we just take the request_content
		if payload.is_none() {
			let content = request_content!();
			if content.len() > 0 {
				payload = Some(content);
			}
		}

		if payload.is_none() {
			response!("payload must be specified!");
			return Ok(());
		}

		let payload = payload.unwrap();

		let nonce = match nonce {
			Some(nonce) => nonce,
			None => 0,
		};

		if user_pubkey.is_none() {
			user_pubkey = pubkey!();
		}

		if user_pubkey.is_none() {
			response!("Configuration error! Tor must be configured!");
			return Ok(());
		}

		let user_pubkey = user_pubkey.unwrap();

		let server_pubkey = match server_pubkey {
			Some(server_pubkey) => server_pubkey,
			None => user_pubkey,
		};

		if server_id.is_none() {
			response!("server id must be specified!");
			return Ok(());
		}

		let server_id = server_id.unwrap();

		if channel_id.is_none() {
			response!("channel id must be specified!");
			return Ok(());
		}

		let channel_id = channel_id.unwrap();

		if timestamp.is_none() {
			response!("timestamp id must be specified!");
			return Ok(());
		}

		let timestamp = timestamp.unwrap();

		let mut message = Message {
			payload: payload.clone(),
			signature: [0u8; 64],
			message_type: match message_type {
				0 => MessageType::Text,
				_ => MessageType::Binary,
			},
			server_pubkey,
			server_id,
			channel_id,
			timestamp,
			user_pubkey,
			nonce,
		};

		if signature.is_some() {
			message.signature = signature.unwrap();
		} else {
			let signature = sign!(&build_signable_message(&message)?);

			if signature.is_none() {
				response!("Tor must be configured!");
				return Ok(());
			}
			let signature = signature.unwrap();
			message.signature = signature;
		}

		if pubkey == server_pubkey {
			ds_context.post_message(message).map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(format!(
					"error posting message: {}",
					e.to_string()
				))
				.into();
				error
			})?;
			response!("{}", SUCCESS);
			flush!();
		} else {
			let signature = base64::encode(message.signature);
			let signature = urlencoding::encode(&signature);
			let user_pubkey = pubkey!().unwrap_or([0u8; 32]);
			let user_pubkey = base64::encode(user_pubkey);
			let user_pubkey = urlencoding::encode(&user_pubkey);
			let onion = OnionV3Address::from_bytes(server_pubkey);

			let mut bypass_db = false;
			loop {
				let token = get_auth_token(&ds_context, server_pubkey, tor_port, bypass_db)?;
				let query = format!(
					"{}&signature={}&user_pubkey={}&token={}",
					query, signature, user_pubkey, token
				);
				let url = format!("http://{}.onion/send_message?{}", onion, query);

				let url = Url::parse(&url).map_err(|e| {
					let error: Error =
						ErrorKind::ApplicationError(format!("url parse error: {}", e.to_string()))
							.into();
					error
				})?;
				let host = format!("{}", url.host().unwrap_or(Domain("notfound")));
				let path = format!("{}?{}", url.path(), url.query().unwrap_or(""));
				let res = torclient::do_post(host.clone(), path.clone(), tor_port, payload.clone())
					.map_err(|e| {
						let error: Error = ErrorKind::ApplicationError(format!(
							"tor client error: {}",
							e.to_string()
						))
						.into();
						error
					})?;

				let start = res.find("\r\n\r\n");

				match start {
					Some(start) => {
						let json_text = &res[(start + 4)..];

						let value: Value = serde_json::from_str(json_text).map_err(|e| {
							let error: Error =
								ErrorKind::ApplicationError(format!("json parse error2: {}", e))
									.into();
							error
						})?;

						match value.get("error") {
							Some(_) => {
								// error occured.
								// get new auth token
								bypass_db = true;
								continue;
							}
							None => {}
						}
					}
					None => {}
				}

				break;
			}
			response!("{}", SUCCESS);
			flush!();
		}

		// send event
		let user_pubkey_str = OnionV3Address::from_bytes(user_pubkey).to_string();
		let event = Event {
			etype: EVENT_TYPE_MESSAGE,
			ebody: std::str::from_utf8(&payload)?.to_string(),
			timestamp: timestamp.to_string(),
			user_pubkey: user_pubkey_str,
		};
		let json = serde_json::to_string(&event)
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("json parse error1: {}", e)).into();
				error
			})
			.unwrap();

		let acs = context
			.get_listeners(&ServerConnectionInfo {
				server_pubkey,
				server_id,
			})
			.unwrap_or(vec![]);

		std::thread::spawn(move || {
			// using async_context must be done in another thread.

			for ac in acs {
				async_context!(ac);
				response!("{}-----BREAK\r\n", json);
				flush!();
			}
		});
	});
	rustlet_mapping!("/send_message", "send_message");

	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;
	let tor_port = config.tor_port;

	// query messages
	rustlet!("query_messages", {
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

		let local_pubkey = pubkey!().unwrap_or([0u8; 32]);
		let server_pubkey = query!("server_pubkey");
		let server_pubkey = if server_pubkey != "" {
			let server_pubkey = urlencoding::decode(&server_pubkey)?;
			let server_pubkey = base64::decode(&*server_pubkey)?;
			server_pubkey.as_slice().try_into()?
		} else {
			local_pubkey
		};

		let channel_id = query!("channel_id").parse()?;

		let server_id = query!("server_id");
		let server_id = urlencoding::decode(&server_id)?;
		let server_id = base64::decode(&*server_id)?;
		let server_id = server_id.as_slice().try_into()?;

		if local_pubkey != server_pubkey {
			process_remote_messages(&ds_context, server_pubkey, server_id, channel_id, tor_port)?;
		} else {
			process_local_messages(&ds_context, server_id, channel_id)?;
		}
	});

	rustlet_mapping!("/query_messages", "query_messages");

	Ok(())
}
