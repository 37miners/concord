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
use crate::context::ServerConnectionInfo;
use concordconfig::ConcordConfig;
use concorderror::Error as ConcordError;
use librustlet::*;
use nioruntime_log::*;
use std::convert::TryInto;

nioruntime_log::debug!(); // set log level to debug
const MAIN_LOG: &str = "mainlog";
const PING_TIMEOUT: u128 = 1000 * 60;

#[derive(Serialize)]
pub struct Event {
	pub etype: u16,
	pub ebody: String,
	pub timestamp: String,
	pub user_pubkey: String,
}

pub const EVENT_TYPE_LISTENER_ID: u16 = 0;
pub const EVENT_TYPE_MESSAGE: u16 = 1;
pub const EVENT_TYPE_PONG: u16 = 2;
pub const EVENT_TYPE_PONG_COMPLETE: u16 = 3;

pub fn init_persistence(
	_config: &ConcordConfig,
	context: ConcordContext,
) -> Result<(), ConcordError> {
	// create a ds context. Each rustlet needs its own
	let context1 = context.clone();
	let context2 = context.clone();
	let context3 = context.clone();
	let context4 = context.clone();
	let context5 = context.clone();
	let context6 = context.clone();

	rustlet!("ping", {
		let listener_id: u128 = query!("listener_id").parse()?;
		let disconnect = query!("disconnect").len() > 0;
		let ac = context4.ping(listener_id).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("ping generated error: {}", e)).into();
			error
		})?;

		let time_now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_millis();
		let event = Event {
			etype: if disconnect {
				EVENT_TYPE_PONG_COMPLETE
			} else {
				EVENT_TYPE_PONG
			},
			ebody: "".to_string(),
			timestamp: time_now.to_string(),
			user_pubkey: "".to_string(),
		};
		let json = serde_json::to_string(&event).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("json parse error1: {}", e)).into();
			error
		})?;

		match ac {
			Some(mut ac) => {
				let context6 = context6.clone();
				std::thread::spawn(move || {
					// using async_context must be done in another thread.
					async_context!(ac.clone());
					response!("{}-----BREAK\r\n", json);
					flush!();

					if disconnect {
						match context6.clone().remove_listener(listener_id).map_err(|e| {
							let error: Error = ErrorKind::ApplicationError(format!(
								"remove listener generated error: {}",
								e
							))
							.into();
							error
						}) {
							Ok(_) => {}
							Err(e) => {
								log_multi!(
									ERROR,
									MAIN_LOG,
									"remove listener generated error: {}",
									e
								);
							}
						}
						match ac.complete() {
							Ok(_) => {}
							Err(e) => {
								log_multi!(ERROR, MAIN_LOG, "complete generated error: {}", e);
							}
						}
					}
				});
			}
			None => {}
		}
	});
	rustlet_mapping!("/ping", "ping");

	rustlet!("disconnect", {
		let listener_id: u128 = query!("listener_id").parse()?;
		let ac = context5.remove_listener(listener_id).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("remove listener generated error: {}", e))
					.into();
			error
		})?;
		match ac {
			Some(mut ac) => {
				std::thread::spawn(move || {
					let _ = ac.complete();
				});
			}
			None => {}
		}
	});
	rustlet_mapping!("/disconnect", "disconnect");

	// listen to this server for events
	rustlet!("listen", {
		set_content_type!("application/octet-stream");
		let listener_id: u128 = rand::random();
		let ac = async_context!();
		context1.add_listener(listener_id, ac).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("add listener generated error: {}", e)).into();
			error
		})?;

		let event = Event {
			etype: EVENT_TYPE_LISTENER_ID,
			ebody: format!("{}", listener_id),
			timestamp: std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap_or(std::time::Duration::from_millis(0))
				.as_millis()
				.to_string(),
			user_pubkey: "".to_string(),
		};
		let json = serde_json::to_string(&event).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("json parse error: {}", e)).into();
			error
		})?;
		response!("{}-----BREAK\r\n", json);
		flush!();
	});
	rustlet_mapping!("/listen", "listen");

	// subscribe to a remote server to get events
	rustlet!("subscribe", {
		let server_id = query!("server_id");
		let server_id = urlencoding::decode(&server_id)?;
		let server_id = base64::decode(&*server_id)?;
		let server_id: [u8; 8] = server_id.as_slice().try_into()?;

		let server_pubkey = query!("server_pubkey");
		let server_pubkey = urlencoding::decode(&server_pubkey)?;
		let server_pubkey = base64::decode(&*server_pubkey)?;
		let server_pubkey: [u8; 32] = server_pubkey.as_slice().try_into()?;

		let listener_id: u128 = query!("listener_id").parse()?;

		context2
			.add_connection_info(
				listener_id,
				ServerConnectionInfo {
					server_pubkey,
					server_id,
				},
			)
			.map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(format!(
					"add_connection_info generated error: {}",
					e
				))
				.into();
				error
			})?;
	});
	rustlet_mapping!("/subscribe", "subscribe");

	std::thread::spawn(move || match listener_cleanup_thread(&context3) {
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
		let listeners = context.get_all_listeners()?;
		let time_now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_millis();
		for (listener_id, mut ac) in listeners {
			match context.get_ping_time(listener_id)? {
				Some(time) => {
					if time_now - time > PING_TIMEOUT {
						context.remove_listener(listener_id)?;
						ac.complete()?;
					}
				}
				None => {
					context.remove_listener(listener_id)?;
					ac.complete()?;
				}
			}
		}

		std::thread::sleep(std::time::Duration::from_millis(10000));
	}
}
