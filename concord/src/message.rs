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

use concorddata::concord::DSContext;
use concorddata::concord::Message;
use concorddata::concord::MessageType;
use concorddata::concord::MessageKey;
use concorderror::Error as ConcordError;
use librustlet::*;
use std::fs::File;
use std::io::Read;
use std::convert::TryInto;

nioruntime_log::debug!(); // set log level to debug

// build a signable message from a message/key.
fn build_signable_message(key: MessageKey, message: Message) -> Result<Vec<u8>, Error> {
	let mut ret = vec![];
	ret.append(&mut key.server_pubkey.to_vec());
	ret.append(&mut key.server_id.to_vec());
	ret.append(&mut key.channel_id.to_be_bytes().to_vec());
	ret.append(&mut key.timestamp.to_be_bytes().to_vec());
	ret.append(&mut key.user_pubkey.to_vec());
	ret.append(&mut key.nonce.to_be_bytes().to_vec());
	ret.append(&mut message.payload.to_vec());
	match message.message_type {
		MessageType::Text => ret.push(0),
		_ => ret.push(1),
	}
	Ok(ret)
}

pub fn init_message(root_dir: String) -> Result<(), ConcordError> {
        // create a ds context. Each rustlet needs it's own
        let ds_context = DSContext::new(root_dir.clone())?;

        // send a message
        rustlet!("send_message", {
                // get query parameters
                let query = request!("query");
                let query_vec = querystring::querify(&query);

		let mut server_pubkey: Option<[u8; 32]> = None;
		let mut server_id: Option<[u8; 8]> = None;
		let mut channel_id: Option<u64> = None;
		let mut timestamp: Option<u64> = None;
		let mut nonce: Option<u16> = None;
		let mut payload: Option<Vec<u8>> = None;
		let mut message_type: Option<u8> = None;

                for query_param in query_vec {
			let param_as_str = query_param.1.to_string();
                        if query_param.0 == "server_pubkey" {
                                let local_server_pubkey = urlencoding::decode(&param_as_str)?;
                                let local_server_pubkey = base64::decode(&*local_server_pubkey)?;
                                server_pubkey = Some(local_server_pubkey.as_slice().try_into()?);
                        } else if query_param.0 == "server_id" {
				let local_server_id = urlencoding::decode(&param_as_str)?;
				let local_server_id = base64::decode(&*local_server_id)?;
				server_id = Some(local_server_id.as_slice().try_into()?);
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
                                },
				mime_multipart::Node::Part(part) => {
					payload = Some(part.body.clone());
				},
                                _ => {

				}
                        }
                }

		let message_type = match message_type {
			Some(message_type) => message_type,
			None => 0,
		};

                if payload.is_none() {
                        response!("payload must be specified!");
                        return Ok(());
                }

                let payload = payload.unwrap();

		let nonce = match nonce {
			Some(nonce) => nonce,
			None => 0,
		};

		let user_pubkey = tor_pubkey!();
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
			payload,
			signature: [0u8; 64],
			message_type: match message_type { 0 => MessageType::Text, _ => MessageType::Binary },
		};

		let message_key = MessageKey {
			server_pubkey,
			server_id,
			channel_id,
			timestamp,
			user_pubkey,
			nonce,
		};

                let signature = sign!(&build_signable_message(message_key.clone(), message.clone())?);

                if signature.is_none() {
                        response!("Tor must be configured!");
                        return Ok(());
                }
                let signature = signature.unwrap();
		message.signature = signature;

		ds_context.post_message(message_key, message).map_err(|e| {
			let error: Error = ErrorKind::ApplicationError(
				format!("error posting message: {}", e.to_string())
			).into();
			error
		})?;
	});
	rustlet_mapping!("/send_message", "send_message");

	// create a ds context. Each rustlet needs it's own
        let ds_context = DSContext::new(root_dir.clone())?;

	// query messages
	rustlet!("query_messages", {
                // get query parameters
                let query = request!("query");
                let query_vec = querystring::querify(&query);

                let mut server_pubkey: Option<[u8; 32]> = None;
                let mut server_id: Option<[u8; 8]> = None;
                let mut channel_id: Option<u64> = None;
		let mut len = 100;
		let mut offset = 0;

                for query_param in query_vec {
                        let param_as_str = query_param.1.to_string();
                        if query_param.0 == "server_pubkey" {
                                let local_server_pubkey = urlencoding::decode(&param_as_str)?;
                                let local_server_pubkey = base64::decode(&*local_server_pubkey)?;
                                server_pubkey = Some(local_server_pubkey.as_slice().try_into()?);
                        } else if query_param.0 == "server_id" {
                                let local_server_id = urlencoding::decode(&param_as_str)?;
                                let local_server_id = base64::decode(&*local_server_id)?;
                                server_id = Some(local_server_id.as_slice().try_into()?);
                        } else if query_param.0 == "channel_id" {
                                channel_id = Some(param_as_str.parse()?);
                        } else if query_param.0 == "len" {
				len = param_as_str.parse()?;
			} else if query_param.0 == "offset" {
				offset = param_as_str.parse()?;
			}
		}

		let server_pubkey = match server_pubkey {
			Some(server_pubkey) => server_pubkey,
			None => {
				match tor_pubkey!() {
					Some(key) => key,
					None => {
						response!("tor not configured!");
						return Ok(());
					},
				}
			},
		};

		if server_id.is_none() {
			response!("server_id must be specified!");
			return Ok(());
		}

		let server_id = server_id.unwrap();

		if channel_id.is_none() {
			response!("channel_id must be specified!");
			return Ok(());
		}

		let channel_id = channel_id.unwrap();

		let messages = ds_context.get_messages(
			server_pubkey,
			server_id,
			channel_id,
			offset,
			len,
		).map_err(|e| {
                        let error: Error = ErrorKind::ApplicationError(
                                format!("error querying messages: {}", e.to_string())
                        ).into();
                        error
                })?;

		response!("<html><body>");
		for message in messages {
			let payload = message.1.payload.clone();
			let text = std::str::from_utf8(&payload)?;
			let message_to_sign = build_signable_message(message.0.clone(), message.1.clone())?;
			let verify_result = verify!(&message_to_sign, Some(message.0.user_pubkey), message.1.signature).unwrap_or(false);
			response!("message={},sig={:?},verify={}</br>", text, message.1.signature, verify_result);
		}
		response!("</body></html>");
	});

	rustlet_mapping!("/query_messages", "query_messages");

	Ok(())
}

