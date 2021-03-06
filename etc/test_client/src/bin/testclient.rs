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

use concorddata::types::*;
use concorderror::Error;
use concordlib::client::{AuthParams, WSListenerClient};
use concordlib::types::*;
use concordutil::nioruntime_log;
use ed25519_dalek::{ExpandedSecretKey, PublicKey, SecretKey};
use nioruntime_log::*;
use std::sync::{Arc, RwLock};

debug!();

fn main() -> Result<(), Error> {
	let secret = [0u8; 32];
	let secret_key = SecretKey::from_bytes(&secret)?;
	let user_pubkey: PublicKey = (&secret_key).into();
	let user_pubkey = Pubkey::from_bytes(user_pubkey.to_bytes());
	let secret_key: ExpandedSecretKey = (&secret_key).into();
	let secret = secret_key.to_bytes();

	info!("starting with user_pubkey {:?}", user_pubkey);

	let mut client = WSListenerClient::new(
		"hlcmyr6xwkgg4sjsnttms5gyc2imtoj6ecbtm34rddklla6lky4i7bad.onion".to_string(),
		11990,
		AuthParams::Secret(secret),
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

		match &event.body {
			EventBody::AuthResponse(_e) => {
				info!("Processing auth message: {:?}", event);

				let event = Event {
					body: EventBody::ViewInviteRequest(
					ViewInviteRequest { invite_url: "http://bjnwu6l4vmps25kwf33wrjy226xkywnwrjwbvrdbvxf74f4dbilawqid.onion/i/74447668189693976956422911572595666066".into()}),
					..Default::default()
				};
/*
				let server_pubkey = Pubkey::from_onion("bjnwu6l4vmps25kwf33wrjy226xkywnwrjwbvrdbvxf74f4dbilawqid")?;
				info!("setting to pubkey = {:?}", server_pubkey);
				let event = Event {
					body: EventBody::SetProfileRequest(
						SetProfileRequest {
							//server_pubkey: Pubkey::from_bytes([0u8; 32]),
							server_pubkey,
							server_id: ServerId::from_bytes([0u8; 8]),
							avatar: Some(Image { data: [56,56,56].to_vec() }).into(),
							profile_data: Some(
								ProfileData { user_bio: "my bio".to_string().into(), user_name: "usrabc".to_string().into() }
							).into(),
						}
					),
					..Default::default()
				};
*/
				writer.send(event)?;
			}
			EventBody::SetProfileResponse(e) => {
				info!("got a set profile response: {:?}", e);
				let server_pubkey = Pubkey::from_onion("bjnwu6l4vmps25kwf33wrjy226xkywnwrjwbvrdbvxf74f4dbilawqid")?;
				let user_pubkey: [u8; 32] = [58, 196, 204, 71, 215, 178, 140, 110, 73, 50, 108, 230, 201, 116, 216, 22, 144, 201, 185, 62, 32, 131, 54, 111, 145, 24, 212, 181, 131, 203, 86, 56];
				let user_pubkey = Pubkey::from_bytes(user_pubkey);
				let event = Event {
					body: EventBody::GetProfileRequest(
						GetProfileRequest {
							server_pubkey,
							server_id: ServerId::from_bytes([0u8; 8]),
							user_pubkeys: vec![user_pubkey.clone()],
							image_request_type: ProfileImageRequestType::SaveAvatars,
							include_profile_data: true,
						}
					),
					..Default::default()
				};
				writer.send(event)?;
			}
			EventBody::GetServersResponse(_e) => {
				info!("Got a servers response: {:?}", event);
				//writer.close()?;
			}
			EventBody::ViewInviteResponse(_) => {
				info!("got the view invite response. Joining.");
				let event = Event {
					body: EventBody::JoinServerRequest(
						JoinServerRequest {
							invite_url: "http://bjnwu6l4vmps25kwf33wrjy226xkywnwrjwbvrdbvxf74f4dbilawqid.onion/i/74447668189693976956422911572595666066".into()
						}
					),
					..Default::default()
				};
				writer.send(event)?;
			}
			_ => {
				error!("Unexpected event type: {:?}", event);
			}
		}

		Ok(())
	})?;

	client.set_error(move |e, onion| {
		error!("got an error: {}", e);
		Ok(())
	})?;

	client.start(None)?;
	/*
		let mut client2 = WSListenerClient::new(
					"bjnwu6l4vmps25kwf33wrjy226xkywnwrjwbvrdbvxf74f4dbilawqid.onion".to_string(),
					11990,
					AuthParams::Token("315673539131917711960422798728697383638".to_string()),
			);
		client2.set_callback(move |event, writer| {
			info!("event on client2: {:?}", event);
			match event.event_type {
							EventType::AuthResponse => {
									let event = Event {
											event_type: EventType::GetServersEvent,
											get_servers_event: Some(GetServersEvent {}).into(),
											..Default::default()
									};
									writer.send(event)?;
							}
				_ => {
					error!("Unexpected event type on 2: {:?}", event);
					writer.close()?;
				},
			}
			Ok(())
		})?;

		client2.set_error(move |e| {
			error!("client 2 error: {}", e);
			Ok(())
		})?;


		client2.start()?;
	*/
	std::thread::park();
	Ok(())
}
