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

use crate::persistence::Event;
use concorderror::Error as ConcordError;
use concorderror::ErrorKind as ConcordErrorKind;
use concordutil::librustlet;
use concordutil::torclient;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use nioruntime_tor::ov3::OnionV3Address;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::RwLock;

const MAIN_LOG: &str = "mainlog";

debug!();

const PING_TIMEOUT: u128 = 1000 * 30; // 30 seconds
const PURGE_TIMEOUT: u128 = 1000 * 60 * 1; // 1 minutes

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct Interest {
	pub server_pubkey: [u8; 32],
	pub server_id: [u8; 8],
	pub channel_id: Option<u64>,
}

#[derive(Clone)]
pub struct ConnectionInfo {
	ac: Option<RustletAsyncContext>,
	subscriptions: HashSet<Interest>,
	pending: Vec<Event>,
	ping_time: u128,
}

#[derive(Clone)]
struct RemoteConnection {
	interest_list: HashSet<Interest>,
}

#[derive(Clone)]
pub struct ConcordContext {
	// listener_id => ConnectionInfo
	listener_map: Arc<RwLock<HashMap<u128, ConnectionInfo>>>,
	// Interest => Set(listener_ids)
	subscription_map: Arc<RwLock<HashMap<Interest, HashSet<u128>>>>,
	// server_pubkey => RemoteConnection info
	remote_interest: Arc<RwLock<HashMap<[u8; 32], RemoteConnection>>>,
}

impl ConcordContext {
	pub fn new() -> Self {
		ConcordContext {
			listener_map: Arc::new(RwLock::new(HashMap::new())),
			subscription_map: Arc::new(RwLock::new(HashMap::new())),
			remote_interest: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	fn process_event(
		context: ConcordContext,
		event: String,
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
		_tor_port: u16,
	) -> Result<(), ConcordError> {
		// send event
		let event: Vec<Event> = serde_json::from_str(&event)?;
		let json = serde_json::to_string(&event)?;

		let interest = Interest {
			server_pubkey,
			server_id,
			channel_id: None,
		};

		let acs = context.add_event(event.clone(), interest)?;

		std::thread::spawn(move || {
			for ac in acs {
				async_context!(ac);
				response!("{}", json);
				async_complete!();
			}
		});

		Ok(())
	}

	fn start_remote(&self, interest: Vec<Interest>, tor_port: u16) -> Result<(), ConcordError> {
		let listener_id: u128 = rand::random();
		let context = self.clone();

		std::thread::spawn(move || loop {
			let interest = interest.clone();
			let onion = OnionV3Address::from_bytes(interest[0].server_pubkey);
			let listen_url = format!("http://{}.onion/listen?listener_id={}", onion, listener_id);
			let mut post_data = "".to_string();
			for ri in &interest {
				let server_pubkey = ri.server_pubkey;
				let server_pubkey = base64::encode(server_pubkey);
				let server_pubkey = urlencoding::encode(&server_pubkey).to_string();

				let server_id = ri.server_id;
				let server_id = base64::encode(server_id);
				let server_id = urlencoding::encode(&server_id).to_string();

				if post_data.len() > 0 {
					post_data = format!(
						"{}\r\nserver_pubkey={}&server_id={}&channel_id={}&seqno={}",
						post_data,
						server_pubkey,
						server_id,
						ri.channel_id.unwrap_or(0),
						0
					);
				} else {
					post_data = format!(
						"server_pubkey={}&server_id={}&channel_id={}&seqno={}",
						server_pubkey,
						server_id,
						ri.channel_id.unwrap_or(0),
						0
					);
				}
			}
			let context = context.clone();
			match torclient::listen(listen_url, post_data, tor_port, &move |event| {
				match Self::process_event(
					context.clone(),
					event,
					interest[0].server_pubkey,
					interest[0].server_id,
					tor_port,
				) {
					Ok(_) => {}
					Err(e) => {
						log_multi!(ERROR, MAIN_LOG, "process event generated error: {}", e);
					}
				}
			}) {
				Ok(_) => {}
				Err(e) => {
					log_multi!(ERROR, MAIN_LOG, "torclient::listen generated error: {}", e);
				}
			}
		});

		Ok(())
	}

	// check if there is already a remote connection for this server, if so, check if we're already paying
	// attention to this interest. If so, do nothing. If there is no remote connection, create one. If
	// we're not paying attention to that interest, use the subscribe url to update our interests.
	fn check_add_remote(&self, interest: Interest, tor_port: u16) -> Result<(), ConcordError> {
		let mut remote_interest = nioruntime_util::lockw!(self.remote_interest).map_err(|e| {
			let error: ConcordError = ConcordErrorKind::LibRustletError(format!("{}", e)).into();
			error
		})?;
		let current_interest = remote_interest.get_mut(&interest.server_pubkey);
		match current_interest {
			Some(current_interest) => {
				match current_interest.interest_list.get(&interest) {
					Some(_) => {} // already interested in it
					None => {
						// use /subscribe to notify of a new interest
					}
				}
			}
			None => {
				self.start_remote(vec![interest.clone()], tor_port)?;
				let mut interest_list = HashSet::new();
				interest_list.insert(interest.clone());
				remote_interest.insert(interest.server_pubkey, RemoteConnection { interest_list });
			}
		}

		Ok(())
	}

	// set the interest list. Also optionally sets the ac for replies.
	// also pending vec is returned. If there are entries in it, ac is not set because
	// it's expected that the thread calling will use the ac to respond.
	pub fn set_listener_interest(
		&self,
		user_pubkey: [u8; 32],
		listener_id: u128,
		ac: Option<RustletAsyncContext>,
		interest_list: Vec<Interest>,
		tor_port: u16,
	) -> Result<(Option<RustletAsyncContext>, Vec<Event>), ConcordError> {
		let mut listener_map = nioruntime_util::lockw!(self.listener_map).map_err(|e| {
			let error: ConcordError =
				ConcordErrorKind::LockError(format!("Error obtaining listener lock: {}", e)).into();
			error
		})?;

		let mut subscription_map = nioruntime_util::lockw!(self.subscription_map).map_err(|e| {
			let error: ConcordError = ConcordErrorKind::LockError(format!(
				"Error obtaining subscription_map lock: {}",
				e
			))
			.into();
			error
		})?;

		let conn_info = listener_map.get_mut(&listener_id);
		let ping_time = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_millis();
		let mut interest_hash_set = HashSet::new();
		for interest in interest_list.clone() {
			interest_hash_set.insert(interest.clone());
			if interest.server_pubkey != user_pubkey {
				self.check_add_remote(interest, tor_port)?;
			}
		}

		for interest in interest_list {
			let hash_set = subscription_map.get_mut(&interest);
			match hash_set {
				Some(hash_set) => {
					hash_set.insert(listener_id);
				}
				None => {
					let mut hash_set = HashSet::new();
					hash_set.insert(listener_id);
					subscription_map.insert(interest, hash_set);
				}
			}
		}

		match conn_info {
			Some(mut conn_info) => {
				conn_info.ping_time = ping_time;
				conn_info.subscriptions = interest_hash_set;
				let ret_ac;
				if conn_info.pending.len() > 0 {
					ret_ac = conn_info.ac.clone();
					conn_info.ac = None;
				} else {
					ret_ac = None;
					conn_info.ac = ac;
				}
				let ret_pending = conn_info.pending.clone();
				conn_info.pending.clear();
				Ok((ret_ac, ret_pending))
			}
			None => {
				let conn_info = ConnectionInfo {
					ac,
					subscriptions: interest_hash_set,
					pending: vec![],
					ping_time,
				};
				listener_map.insert(listener_id, conn_info);

				Ok((None, vec![]))
			}
		}
	}

	// adds an event to any queues of listeners that are not connected and returns a list
	// to RustletAsyncContext that are connected to reply to. The acs are also removed so that
	// any new events will go to pending until next time set_listener_interest is called
	// with a new RustletAsyncContext.
	pub fn add_event(
		&self,
		events: Vec<Event>,
		interest: Interest,
	) -> Result<Vec<RustletAsyncContext>, ConcordError> {
		let mut listener_map = nioruntime_util::lockw!(self.listener_map).map_err(|e| {
			let error: ConcordError =
				ConcordErrorKind::LockError(format!("Error obtaining listener lock: {}", e)).into();
			error
		})?;

		let subscription_map = nioruntime_util::lockw!(self.subscription_map).map_err(|e| {
			let error: ConcordError = ConcordErrorKind::LockError(format!(
				"Error obtaining subscription_map lock: {}",
				e
			))
			.into();
			error
		})?;

		let listeners = subscription_map.get(&interest);

		let mut ret = vec![];
		match listeners {
			Some(listeners) => {
				for listener in listeners {
					let conn_info = listener_map.get_mut(&listener);
					match conn_info {
						Some(conn_info) => match &conn_info.ac {
							Some(ac) => {
								ret.push(ac.clone());
								conn_info.ac = None;
							}
							None => {
								conn_info.pending.append(&mut events.clone());
							}
						},
						None => {}
					}
				}
			}
			None => {}
		}

		Ok(ret)
	}

	// gets any timed out listeners and also purges any listeners that haven't been seen for
	// a specified time
	pub fn get_timed_out_listeners_and_purge(
		&self,
	) -> Result<Vec<RustletAsyncContext>, ConcordError> {
		let time_now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_millis();

		let mut listener_map = nioruntime_util::lockw!(self.listener_map).map_err(|e| {
			let error: ConcordError =
				ConcordErrorKind::LockError(format!("Error obtaining listener lock: {}", e)).into();
			error
		})?;

		let mut subscription_map = nioruntime_util::lockw!(self.subscription_map).map_err(|e| {
			let error: ConcordError = ConcordErrorKind::LockError(format!(
				"Error obtaining subscription_map lock: {}",
				e
			))
			.into();
			error
		})?;

		let mut purge_list = vec![];
		let mut time_out_list = vec![];

		for (listener_id, conn_info) in &mut *listener_map {
			if time_now - conn_info.ping_time > PURGE_TIMEOUT {
				purge_list.push((listener_id.to_owned(), conn_info.clone()));
			} else if time_now - conn_info.ping_time > PING_TIMEOUT {
				match &conn_info.ac {
					Some(ac) => time_out_list.push(ac.clone()),
					None => {}
				}
				conn_info.ac = None;
			}
		}

		for (listener_id, conn_info) in purge_list {
			for interest in &conn_info.subscriptions {
				let listener_hash_set = subscription_map.get_mut(&interest);
				match listener_hash_set {
					Some(listener_hash_set) => {
						listener_hash_set.remove(&listener_id);
					}
					None => {}
				}
			}
			listener_map.remove(&listener_id);
		}

		Ok(time_out_list)
	}
}
