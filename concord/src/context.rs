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

use concorderror::Error;
use concorderror::ErrorKind;
use librustlet::RustletAsyncContext;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct ServerConnectionInfo {
	pub server_pubkey: [u8; 32],
	pub server_id: [u8; 8],
}

pub struct ConnectionInfo {
	ac: RustletAsyncContext,
	subscriptions: HashSet<ServerConnectionInfo>,
	ping_time: u128,
}

#[derive(Clone)]
pub struct ConcordContext {
	// listener_id => ConnectionInfo
	listener_map: Arc<RwLock<HashMap<u128, ConnectionInfo>>>,
	// ServerConnectionInfo => Set(listener_ids)
	subscription_map: Arc<RwLock<HashMap<ServerConnectionInfo, HashSet<u128>>>>,
}

impl ConcordContext {
	pub fn new() -> Self {
		ConcordContext {
			listener_map: Arc::new(RwLock::new(HashMap::new())),
			subscription_map: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	pub fn ping(&self, listener_id: u128) -> Result<Option<RustletAsyncContext>, Error> {
		let ping_time = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_millis();
		let mut listener_map = self.listener_map.write().map_err(|e| {
			let error: Error =
				ErrorKind::LockError(format!("Error obtaining listener lock: {}", e)).into();
			error
		})?;

		match listener_map.get_mut(&listener_id) {
			Some(connection_info) => {
				connection_info.ping_time = ping_time;
				Ok(Some(connection_info.ac.clone()))
			}
			None => Ok(None),
		}
	}

	pub fn get_ping_time(&self, listener_id: u128) -> Result<Option<u128>, Error> {
		let listener_map = self.listener_map.write().map_err(|e| {
			let error: Error =
				ErrorKind::LockError(format!("Error obtaining listener lock: {}", e)).into();
			error
		})?;

		match listener_map.get(&listener_id) {
			Some(connection_info) => Ok(Some(connection_info.ping_time)),
			None => Ok(None),
		}
	}

	pub fn add_listener(&self, listener_id: u128, ac: RustletAsyncContext) -> Result<(), Error> {
		let ping_time = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_millis();

		let mut listener_map = self.listener_map.write().map_err(|e| {
			let error: Error =
				ErrorKind::LockError(format!("Error obtaining listener lock: {}", e)).into();
			error
		})?;

		let connection_info = ConnectionInfo {
			ac,
			subscriptions: HashSet::new(),
			ping_time,
		};
		listener_map.insert(listener_id, connection_info);

		Ok(())
	}

	pub fn add_connection_info(
		&self,
		listener_id: u128,
		server_connection_info: ServerConnectionInfo,
	) -> Result<(), Error> {
		let mut listener_map = self.listener_map.write().map_err(|e| {
			let error: Error = ErrorKind::LockError(format!("Error obtaining lock: {}", e)).into();
			error
		})?;

		let mut subscription_map = self.subscription_map.write().map_err(|e| {
			let error: Error =
				ErrorKind::LockError(format!("Error obtaining subscription_map lock: {}", e))
					.into();
			error
		})?;

		let mut connection_info = listener_map.get_mut(&listener_id);

		match connection_info.as_mut() {
			Some(connection_info) => {
				connection_info
					.subscriptions
					.insert(server_connection_info.clone());
			}
			None => {
				return Err(ErrorKind::ListenerNotFound(format!(
					"listener_id {} was not found",
					listener_id
				))
				.into())
			}
		}

		let listener_set: Option<&mut HashSet<u128>> =
			subscription_map.get_mut(&server_connection_info);

		match listener_set {
			Some(listener_set) => {
				listener_set.insert(listener_id);
			}
			None => {
				let mut hash_set = HashSet::new();
				hash_set.insert(listener_id);
				subscription_map.insert(server_connection_info, hash_set);
			}
		}

		Ok(())
	}

	pub fn remove_listener(&self, listener_id: u128) -> Result<Option<RustletAsyncContext>, Error> {
		let mut listener_map = self.listener_map.write().map_err(|e| {
			let error: Error = ErrorKind::LockError(format!("Error obtaining lock: {}", e)).into();
			error
		})?;

		let mut subscription_map = self.subscription_map.write().map_err(|e| {
			let error: Error =
				ErrorKind::LockError(format!("Error obtaining subscription_map lock: {}", e))
					.into();
			error
		})?;

		let connection_info = listener_map.remove(&listener_id);

		match connection_info {
			Some(connection_info) => {
				for server_connection_info in connection_info.subscriptions {
					let listener_hash_set = subscription_map.get_mut(&server_connection_info);

					match listener_hash_set {
						Some(listener_hash_set) => {
							listener_hash_set.remove(&listener_id);
						}
						None => {}
					}
				}

				Ok(Some(connection_info.ac))
			}
			None => Ok(None),
		}
	}

	pub fn get_all_listeners(&self) -> Result<Vec<(u128, RustletAsyncContext)>, Error> {
		let listener_map = self.listener_map.write().map_err(|e| {
			let error: Error = ErrorKind::LockError(format!("Error obtaining lock: {}", e)).into();
			error
		})?;

		let mut ret = vec![];
		for (listener_id, connection_info) in &*listener_map {
			ret.push((*listener_id, connection_info.ac.clone()));
		}

		Ok(ret)
	}

	pub fn get_listeners(
		&self,
		connection_info: &ServerConnectionInfo,
	) -> Result<Vec<RustletAsyncContext>, Error> {
		let listener_map = self.listener_map.write().map_err(|e| {
			let error: Error = ErrorKind::LockError(format!("Error obtaining lock: {}", e)).into();
			error
		})?;

		let subscription_map = self.subscription_map.write().map_err(|e| {
			let error: Error =
				ErrorKind::LockError(format!("Error obtaining subscription_map lock: {}", e))
					.into();
			error
		})?;

		let hash_set = subscription_map.get(connection_info);

		let mut ret = vec![];

		match hash_set {
			Some(hash_set) => {
				for listener_id in hash_set {
					let connection_info = listener_map.get(listener_id);
					match connection_info {
						Some(connection_info) => {
							ret.push(connection_info.ac.clone());
						}
						None => {}
					}
				}
			}
			None => {}
		}

		Ok(ret)
	}
}
