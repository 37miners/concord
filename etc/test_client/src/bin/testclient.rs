// Copyright 2021 The Grin Developers
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
use concordlib::client::WSListenerClient;
use concordlib::types::{Event, EventType, GetServersEvent};
use concordutil::nioruntime_log;
use nioruntime_log::*;
use std::sync::{Arc, RwLock};

debug!();

fn main() -> Result<(), Error> {
	let mut client = WSListenerClient::new(
		"bjnwu6l4vmps25kwf33wrjy226xkywnwrjwbvrdbvxf74f4dbilawqid.onion".to_string(),
		11990,
		Some("325980726909577586712199253994964265498".to_string()),
	);

	let time_now = std::time::Instant::now();
	let time = Arc::new(RwLock::new(time_now));

	client.set_callback(move |event, writer| {
		let mut time = match time.write() {
			Ok(time) => time,
			Err(e) => {
				error!("obtaining time write lock generated error: {}", e);
				return Ok(());
			}
		};
		debug!(
			"elapsed time since last event={}ms",
			(*time).elapsed().as_nanos() as f64 / 1_000_000 as f64
		);
		*time = std::time::Instant::now();

		match event.event_type {
			EventType::AuthResponse => {
				let event = Event {
					event_type: EventType::GetServersEvent,
					get_servers_event: Some(GetServersEvent {}).into(),
					..Default::default()
				};

				writer.send(event)?;
				info!("Processing auth message");
			}
			EventType::GetServersResponse => {
				info!("Got a servers response: {:?}", event);
				//writer.close()?;
			}
			_ => {
				error!("Unexpected event type: {:?}", event);
			}
		}

		Ok(())
	})?;
	client.set_error(move |e| {
		error!("got an error: {}", e);
		Ok(())
	})?;

	client.start()?;

	std::thread::park();
	Ok(())
}
