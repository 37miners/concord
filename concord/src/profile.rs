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

use crate::conn_manager::ConnManager;
use crate::send;
use crate::types::EventBody;
use crate::types::{
	ConnectionInfo, Event, GetProfileResponse, ProfileImageRequestType, SetProfileResponse,
};
use concordconfig::ConcordConfig;
use concorddata::concord::DSContext;
use concorddata::types::Pubkey;
use concorddata::types::ServerId;
use concorddata::types::{Image, ProfileValue, SerOption};
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::sync::{Arc, RwLock};

debug!(); // set log level to debug

fn get_avatar(
	root_dir: String,
	server_id: [u8; 8],
	pubkey: [u8; 32],
) -> Result<Vec<u8>, ConcordError> {
	let server_id = ServerId::from_bytes(server_id).to_base58()?;
	let pubkey = Pubkey::from_bytes(pubkey).to_base58()?;
	let file_name = format!(
		"{}/www/images/user_images/{}-{}",
		root_dir, server_id, pubkey
	);
	error!("start read: {}", file_name);
	let start = std::time::Instant::now();
	let mut f = File::open(&file_name)?;
	let metadata = std::fs::metadata(&file_name)?;
	let mut data = vec![0; metadata.len() as usize];
	f.read(&mut data)?;
	error!(
		"end read of {} bytes, time = {}",
		metadata.len(),
		start.elapsed().as_nanos()
	);

	Ok(data)
}

fn set_avatar(
	root_dir: String,
	server_id: [u8; 8],
	pubkey: [u8; 32],
	icon: Vec<u8>,
) -> Result<(), ConcordError> {
	let server_id = ServerId::from_bytes(server_id).to_base58()?;
	let pubkey = Pubkey::from_bytes(pubkey).to_base58()?;
	let file_name = format!(
		"{}/www/images/user_images/avatars-{}-{}",
		root_dir, server_id, pubkey
	);

	let mut file = if std::path::Path::new(&file_name).exists() {
		std::fs::OpenOptions::new()
			.write(true)
			.truncate(true)
			.open(file_name)?
	} else {
		std::fs::OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(file_name)?
	};
	file.write_all(&icon)?;
	Ok(())
}

pub fn get_profile(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
	conn_manager: Arc<RwLock<ConnManager>>,
	config: &ConcordConfig,
) -> Result<bool, ConcordError> {
	let request_id = event.request_id;
	let (user_pubkeys, server_pubkey, server_id, image_request_type, include_profile_data) =
		match &event.body {
			EventBody::GetProfileRequest(event) => (
				event.user_pubkeys.clone(),
				event.server_pubkey.clone(),
				event.server_id.clone(),
				event.image_request_type.clone(),
				event.include_profile_data,
			),
			_ => {
				warn!(
					"Malformed GetProfileRequest. Wrong body present: {:?}",
					event
				);
				return Ok(true);
			}
		};

	let pubkey = pubkey!();

	if pubkey == server_pubkey.to_bytes() {
		// local request
		let mut data: Vec<(SerOption<Image>, SerOption<ProfileValue>)> = vec![];

		if include_profile_data {
			let profiles =
				ds_context.get_profiles(user_pubkeys.clone(), server_pubkey, server_id.clone())?;
			for profile in profiles {
				match profile {
					Some(profile) => data.push((None.into(), Some(profile.profile_data).into())),
					None => data.push((None.into(), None.into())),
				}
			}
		}

		if image_request_type == ProfileImageRequestType::ReturnAvatars {
			for i in 0..user_pubkeys.len() {
				data[i].0 = match get_avatar(
					config.root_dir.clone(),
					server_id.to_bytes(),
					user_pubkeys[i].to_bytes(),
				) {
					Ok(avatar) => Some(Image { data: avatar }).into(),
					Err(e) => {
						warn!("error getting profile image: {}", e);
						None.into()
					}
				};
			}
		}

		let response = Event {
			request_id,
			body: EventBody::GetProfileResponse(GetProfileResponse { data }),
			..Default::default()
		};

		let handle = conn_info.handle.clone();
		send!(handle, response);
	} else {
		// need to request a remote server
		let mut conn_manager = nioruntime_util::lockw!(conn_manager)?;
		let handle = conn_info.handle.clone();
		let mut new_event = event.clone();

		let mut is_save = false;
		new_event.body = match new_event.body {
			EventBody::GetProfileRequest(get_profile_request) => {
				if get_profile_request.image_request_type == ProfileImageRequestType::SaveAvatars {
					is_save = true;
					let mut updated = get_profile_request.clone();
					updated.image_request_type = ProfileImageRequestType::ReturnAvatars;
					EventBody::GetProfileRequest(updated)
				} else {
					EventBody::GetProfileRequest(get_profile_request)
				}
			}
			_ => {
				error!(
					"unexpected event body. Expected GetProfileRequest: {:?}",
					new_event
				);
				return Ok(true);
			}
		};

		let root_dir = config.root_dir.clone();
		conn_manager.send_event(
			server_pubkey.to_bytes(),
			new_event.clone(),
			config.tor_port,
			Box::pin(move |event| {
				if is_save {
					// strip out and save avatars.
					let new_body = match &event.body {
						EventBody::GetProfileResponse(get_profile_response) => {
							let mut data = vec![];
							let mut i = 0;
							for elem in &get_profile_response.data {
								match &elem.0 .0 {
									Some(image) => {
										set_avatar(
											root_dir.clone(),
											server_id.to_bytes(),
											user_pubkeys[i].to_bytes(),
											image.data.clone(),
										)?;
									}
									None => {}
								}
								i += 1;

								data.push((None.into(), elem.1.clone()));
							}
							EventBody::GetProfileResponse(GetProfileResponse { data })
						}
						_ => {
							warn!(
								"unexpected event type returned when expecting get_profile_request: {:?}",
								event
							);
							return Ok(()); // not allowing event to propogate further.
						}
					};

					let new_event = Event {
						request_id: event.request_id,
						timestamp: event.timestamp,
						body: new_body,
						..Default::default()
					};

					send!(handle, new_event);
				} else {
					send!(handle, event);
				}
				Ok(())
			}),
		)?;
	}

	Ok(false)
}

pub fn set_profile(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
	conn_manager: Arc<RwLock<ConnManager>>,
	config: &ConcordConfig,
) -> Result<bool, ConcordError> {
	let request_id = event.request_id;
	let user_pubkey = match &conn_info.pubkey {
		Some(user_pubkey) => user_pubkey,
		None => {
			warn!(
				"unexpected unauthed call in set_profile for event: {:?}",
				event
			);
			return Ok(true);
		}
	};

	let (server_id, server_pubkey, avatar, profile_data) = match &event.body {
		EventBody::SetProfileRequest(event) => (
			&event.server_id,
			&event.server_pubkey,
			&event.avatar,
			&event.profile_data,
		),
		_ => {
			warn!("Malformed SetProfileRequest. No event present: {:?}", event);
			return Ok(true);
		}
	};

	if pubkey!() == server_pubkey.to_bytes() {
		// local data
		match &profile_data.0 {
			Some(profile_data) => {
				ds_context.set_profile(
					user_pubkey.clone(),
					server_pubkey.clone(),
					server_id.clone(),
					profile_data.clone(),
				)?;
			}
			None => {}
		}

		match &avatar.0 {
			Some(avatar) => {
				set_avatar(
					config.root_dir.clone(),
					server_id.to_bytes(),
					server_pubkey.to_bytes(),
					avatar.data.clone(),
				)?;
			}
			None => {}
		}

		let event = Event {
			request_id,
			body: EventBody::SetProfileResponse(SetProfileResponse { success: true }),
			..Default::default()
		};

		let handle = conn_info.handle.clone();
		send!(handle, event);
	} else {
		// remote server
		let mut conn_manager = nioruntime_util::lockw!(conn_manager)?;
		let handle = conn_info.handle.clone();
		conn_manager.send_event(
			server_pubkey.to_bytes(),
			event.clone(),
			config.tor_port,
			Box::pin(move |event| {
				send!(handle, event);
				Ok(())
			}),
		)?;
	}

	Ok(false)
}
