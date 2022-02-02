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

use crate::auth::ws_auth;
use crate::context::ConcordContext;
use crate::server::get_servers;
use crate::types::*;
use crate::{bin_event, close, send, try2};
use concordconfig::ConcordConfig;
use concorddata::concord::DSContext;
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

debug!(); // set log level to debug

fn process_open(
	handle: ConnData,
	conn_info: Arc<RwLock<HashMap<u128, ConnectionInfo>>>,
) -> Result<(), Error> {
	let id = handle.get_connection_id();
	debug!("websocket open: {}", id);

	let event = Event {
		event_type: EventType::ChallengeEvent,
		challenge_event: Some(ChallengeEvent { challenge: id }).into(),
		..Default::default()
	};
	send!(handle, event);

	let mut conn_info = nioruntime_util::lockw!(conn_info)?;

	conn_info.insert(
		id,
		ConnectionInfo {
			handle: handle.clone(),
			pubkey: None,
		},
	);

	Ok(())
}

fn process_binary(
	handle: ConnData,
	conn_info: Arc<RwLock<HashMap<u128, ConnectionInfo>>>,
	ds_context: &DSContext,
) -> Result<(), Error> {
	let id = handle.get_connection_id();
	let event = bin_event!();

	debug!("event on connection[{}]={:?}", id, event);

	match event.event_type {
		EventType::AuthEvent => {
			try2!(
				ws_auth(handle, &event, ds_context, conn_info),
				"ws_auth error"
			);
		}
		_ => {
			// for all other event types, we ensure that the user has
			// a pubkey (meaning they've authenticated)

			let close;
			{
				let conn_info = nioruntime_util::lockr!(conn_info)?;
				close = match conn_info.get(&id) {
					Some(connection_info) => {
						match &connection_info.pubkey {
							Some(pubkey) => {
								debug!("authed event on {}: {:?}, pubkey={:?}", id, event, pubkey);
								// we know the user is authed, now process events
								match event.event_type {
									EventType::GetServersEvent => {
										try2!(
											get_servers(connection_info, ds_context),
											"get_servers error"
										)
									}
									_ => {
										debug!("unexpected event type. Closing conn = {}", id);
										//close!(handle, conn_info);
										true
									}
								}
							}
							None => {
								//close!(handle, conn_info);
								true
							}
						}
					}
					None => {
						//close!(handle, conn_info);
						true
					}
				};
			}

			if close {
				let mut conn_info = nioruntime_util::lockw!(conn_info)?;
				close!(handle, conn_info);
			}
		}
	}

	Ok(())
}

fn process_close(
	handle: ConnData,
	conn_info: Arc<RwLock<HashMap<u128, ConnectionInfo>>>,
) -> Result<(), Error> {
	let id = handle.get_connection_id();
	debug!("close : {},", id);
	let mut conn_info = nioruntime_util::lockw!(conn_info)?;
	conn_info.remove(&id);
	Ok(())
}

pub fn init_ws(cconfig: &ConcordConfig, _context: ConcordContext) -> Result<(), ConcordError> {
	let conn_info = Arc::new(RwLock::new(HashMap::new()));

	let ds_context = DSContext::new(cconfig.root_dir.clone())?;

	socklet!("ws", {
		let conn_info = conn_info.clone();
		let handle = handle!()?;
		match event!()? {
			Socklet::Open => {
				process_open(handle, conn_info)?;
			}
			Socklet::Binary => {
				process_binary(handle, conn_info, &ds_context)?;
			}
			Socklet::Close => {
				process_close(handle, conn_info)?;
			}
			_ => {}
		}
	});

	socklet_mapping!("/ws", "ws");
	Ok(())
}
