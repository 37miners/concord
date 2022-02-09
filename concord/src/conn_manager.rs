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

use crate::client::AuthParams;
use crate::client::WSListenerClient;
use crate::librustlet;
use crate::types::Event;
use concorddata::types::Pubkey;
use concorderror::Error;
use concordutil::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, RwLock};

debug!();

// Fn(&[u8], usize, WriteHandle) -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,

#[derive(Clone)]
pub struct ConnManager {
	map: Arc<RwLock<HashMap<[u8; 32], SyncSender<Option<Event>>>>>,
	callbacks: Arc<
		RwLock<
			HashMap<
				u32,
				Pin<Box<dyn Fn(&Event) -> Result<(), Error> + Send + 'static + Sync + Unpin>>,
			>,
		>,
	>,
}

impl ConnManager {
	pub fn new() -> Self {
		ConnManager {
			map: Arc::new(RwLock::new(HashMap::new())),
			callbacks: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	pub fn send_event(
		&mut self,
		server_pubkey: [u8; 32],
		event: Event,
		tor_port: u16,
		callback: Pin<Box<dyn Fn(&Event) -> Result<(), Error> + Send + 'static + Sync + Unpin>>,
	) -> Result<(), Error> {
		{
			let mut callbacks = nioruntime_util::lockw!(self.callbacks)?;
			callbacks.insert(event.request_id, callback);
		}

		{
			let map = nioruntime_util::lockw!(self.map)?;

			match map.get(&server_pubkey) {
				Some(sender) => {
					sender.send(Some(event))?;
					return Ok(());
				}
				None => {}
			}
		}

		let sender = Self::start_listener(
			server_pubkey,
			self.map.clone(),
			tor_port,
			self.callbacks.clone(),
		)?;
		sender.send(Some(event))?;
		Ok(())
	}

	fn start_listener(
		server_pubkey: [u8; 32],
		map: Arc<RwLock<HashMap<[u8; 32], SyncSender<Option<Event>>>>>,
		tor_port: u16,
		callbacks: Arc<
			RwLock<
				HashMap<
					u32,
					Pin<Box<dyn Fn(&Event) -> Result<(), Error> + Send + 'static + Sync + Unpin>>,
				>,
			>,
		>,
	) -> Result<SyncSender<Option<Event>>, Error> {
		let (sender, receiver) = sync_channel(2);

		{
			let mut map = nioruntime_util::lockw!(map)?;
			map.insert(server_pubkey, sender.clone());
		}

		let sender_clone = sender.clone();

		std::thread::spawn(move || {
			match Self::do_recv(
				sender_clone,
				receiver,
				server_pubkey,
				tor_port,
				callbacks,
				map,
			) {
				Ok(_) => {}
				Err(e) => {
					error!("Recv thread exited with error: {}", e);
				}
			}
		});

		Ok(sender)
	}

	fn do_recv(
		sender: SyncSender<Option<Event>>,
		receiver: Receiver<Option<Event>>,
		server_pubkey: [u8; 32],
		tor_port: u16,
		callbacks: Arc<
			RwLock<
				HashMap<
					u32,
					Pin<Box<dyn Fn(&Event) -> Result<(), Error> + Send + 'static + Sync + Unpin>>,
				>,
			>,
		>,
		map: Arc<RwLock<HashMap<[u8; 32], SyncSender<Option<Event>>>>>,
	) -> Result<(), Error> {
		let secret = match secret!() {
			Some(secret) => secret,
			None => {
				return Err(ErrorKind::ApplicationError(
					"onion address not configured".to_string(),
				)
				.into());
			}
		};

		let onion = format!("{}.onion", Pubkey::from_bytes(server_pubkey).to_onion()?);
		info!("connecting listener to {}", onion);
		let mut client = WSListenerClient::new(onion, tor_port, AuthParams::Secret(secret));

		client.set_callback(move |event, _writer| {
			info!("client callback on event: {:?}", event);
			let callback = {
				let mut callbacks = nioruntime_util::lockw!(callbacks)?;
				callbacks.remove(&event.request_id)
			};

			match callback {
				Some(callback) => (callback)(event)?,
				None => {
					error!("Callback not found for event: {:?}", event);
				}
			}
			Ok(())
		})?;
		client.set_error(move |error, onion| {
			error!("client [{}] received error: {}", onion, error);
			let server_pubkey = Pubkey::from_onion(&onion)?.to_bytes();

			{
				let mut map = nioruntime_util::lockw!(map)?;
				map.remove(&server_pubkey);
			}
			// TODO: handle reconnect and resend logic.
			// currently we just drop messages on disconnect.

			Ok(())
		})?;

		client.start(Some((sender, receiver)))?;

		warn!("client start thread exiting");
		Ok(())
	}
}
