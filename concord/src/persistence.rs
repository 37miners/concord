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

use crate::context::ConcordContext;
use crate::context::Interest;
use concordconfig::ConcordConfig;
use concorderror::Error as ConcordError;
use librustlet::*;
use nioruntime_log::*;
use std::collections::HashMap;
use std::convert::TryInto;

nioruntime_log::debug!(); // set log level to debug
const MAIN_LOG: &str = "mainlog";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
	pub etype: u16,
	pub ebody: String,
	pub timestamp: String,
	pub user_pubkey: String,
	pub server_pubkey: String,
	pub server_id: String,
	pub channel_id: String,
}

pub const EVENT_TYPE_MESSAGE: u16 = 1;
pub const EVENT_TYPE_TIMEOUT: u16 = 4;

fn parse_interest_list(content: &str) -> Result<Vec<Interest>, Error> {
	let subscriptions = content.split("\r\n");

	let mut interest_list = vec![];

	for subscription in subscriptions {
		if subscription.len() == 0 {
			continue;
		}

		let qs = querystring::querify(subscription);
		let mut qmap = HashMap::new();
		for param in qs {
			qmap.insert(param.0, param.1);
		}

		let server_pubkey = qmap.get("server_pubkey");
		let server_id = qmap.get("server_id");
		let channel_id = qmap.get("channel_id");
		let seqno = qmap.get("seqno");

		let server_pubkey = match server_pubkey {
			Some(server_pubkey) => server_pubkey,
			None => {
				return Err(
					ErrorKind::ApplicationError(format!("server_pubkey must be specified")).into(),
				)
			}
		};

		let server_id = match server_id {
			Some(server_id) => server_id,
			None => {
				return Err(
					ErrorKind::ApplicationError(format!("server_id must be specified")).into(),
				)
			}
		};

		let _channel_id: u64 = match channel_id {
			Some(channel_id) => channel_id.parse()?,
			None => u64::MAX,
		};

		let _seqno: u64 = match seqno {
			Some(seqno) => seqno.parse()?,
			None => u64::MAX,
		};

		let server_pubkey = urlencoding::decode(&server_pubkey)?;
		let server_pubkey = base64::decode(&*server_pubkey)?;
		let server_pubkey: [u8; 32] = server_pubkey.as_slice().try_into()?;

		let server_id = urlencoding::decode(&server_id)?;
		let server_id = base64::decode(&*server_id)?;
		let server_id: [u8; 8] = server_id.as_slice().try_into()?;

		interest_list.push(Interest {
			server_pubkey,
			server_id,
			channel_id: None,
		});
	}
	Ok(interest_list)
}

fn process_subscriptions(
	content: &str,
	listener_id: u128,
	context: &ConcordContext,
	tor_port: u16,
	mut ac: RustletAsyncContext,
) -> Result<(), Error> {
	let interest_list = parse_interest_list(content)?;

	let (_, events) = context
		.set_listener_interest(
			pubkey!().unwrap_or([0u8; 32]),
			listener_id,
			Some(ac.clone()),
			interest_list,
			tor_port,
		)
		.map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("set_listener_interest error: {}", e)).into();
			error
		})?;

	if events.len() > 0 {
		let json = serde_json::to_string(&events)
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("json parse error listen events: {}", e))
						.into();
				error
			})
			.unwrap();
		response!("{}//-----ENDJSON-----", json);
		ac.complete()?;
	}

	Ok(())
}

// Persistence model:
// 1.) Call listen with parameters client randomly selects listener_id
// in js use String(Math.floor(Math.random() * 9007199254740991)) +
// String(Math.floor(Math.random() * 9007199254740991))
// this creates a string that is less than u128::MAX so rust will accept it.
// (random u128, collisions as not likely)
// listen takes as post parameters a set of server_pubkey/server_id/channel_id/lastmsgseqno.
// 2.) Call subscribe with parameters listener_id, list of server_pubkey, server_id,
//     channel_id, lastmsgseqno as needed to change the set of subscriptions
// 3.) Also subscribe to server_id/server_pubkey pairs for non-chat related messages
// 4.) Listen will long poll for a certain amount of time. If no message comes through
//     an empty message will be sent which tells the listener to reconnect.
pub fn init_persistence(
	config: &ConcordConfig,
	context: ConcordContext,
) -> Result<(), ConcordError> {
	let context1 = context.clone();
	let context2 = context.clone();
	let tor_port = config.tor_port;

	rustlet!("ping", {});
	rustlet_mapping!("/ping", "ping");

	rustlet!("disconnect", {});
	rustlet_mapping!("/disconnect", "disconnect");

	// listen to this server for events
	rustlet!("listen", {
		set_content_type!("application/octet-stream");
		let listener_id: u128 = query!("listener_id").parse()?;
		let ac = async_context!();
		let content = request_content!();
		let content = std::str::from_utf8(&content)?;
		process_subscriptions(content, listener_id, &context, tor_port, ac)?;
	});
	rustlet_mapping!("/listen", "listen");

	let tor_port = config.tor_port;

	// subscribe to change the interest list
	rustlet!("subscribe", {
		let listener_id: u128 = query!("listener_id").parse()?;
		let content = request_content!();
		let content = std::str::from_utf8(&content)?;
		let interest_list = parse_interest_list(content)?;

		let (ac, events) = context1
			.set_listener_interest(
				pubkey!().unwrap_or([0u8; 32]),
				listener_id,
				None,
				interest_list,
				tor_port,
			)
			.map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(format!(
					"set_listener_interest subscribe error: {}",
					e
				))
				.into();
				error
			})?;
		let events = serde_json::to_string(&events).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("serde_json parse on events error: {}", e))
					.into();
			error
		})?;

		match ac {
			Some(ac) => {
				// we have an ac to write back with if there's events
				if events.len() > 0 {
					std::thread::spawn(move || {
						async_context!(ac);
						response!("{}", events);
						async_complete!();
					});
				}
			}
			None => {}
		}
	});
	rustlet_mapping!("/subscribe", "subscribe");

	std::thread::spawn(move || match listener_cleanup_thread(&context2) {
		Ok(_) => {}
		Err(e) => {
			log_multi!(
				ERROR,
				MAIN_LOG,
				"listner_cleanup_thread generated error: {}",
				e
			);
		}
	});

	Ok(())
}

fn listener_cleanup_thread(context: &ConcordContext) -> Result<(), ConcordError> {
	loop {
		std::thread::sleep(std::time::Duration::from_millis(10000));
		let time_now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_millis();
		let timeout_event = Event {
			etype: EVENT_TYPE_TIMEOUT,
			ebody: "".to_string(),
			timestamp: time_now.to_string(),
			user_pubkey: "".to_string(),
			server_pubkey: "".to_string(),
			server_id: "".to_string(),
			channel_id: "".to_string(),
		};
		let json = serde_json::to_string(&vec![timeout_event]).unwrap_or("".to_string());

		let time_out_list = context.get_timed_out_listeners_and_purge()?;

		for ac in time_out_list {
			async_context!(ac);
			response!("{}//-----ENDJSON-----", json);
			async_complete!();
		}
	}
}
