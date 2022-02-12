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
use crate::types::{
	ConnectionInfo, CreateInviteResponse, DeleteInviteResponse, Event, EventBody,
	InviteResponseInfo, ListInvitesResponse, ViewInviteResponse,
};
use concordconfig::ConcordConfig;
use concorddata::concord::Channel;
use concorddata::concord::DSContext;
use concorddata::concord::ServerInfoReply;
use concorddata::types::Image;
use concorddata::types::Pubkey;
use concorddata::types::SerString;
use concorddata::types::ServerId;
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use std::sync::{Arc, RwLock};
use substring::Substring;

const LOCAL_SERVER_ID: [u8; 8] = [0u8; 8];

debug!(); // set log level to debug

pub fn create_invite(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	let request_id = event.request_id;
	let (server_id, server_pubkey, count, expiration) = match &event.body {
		EventBody::CreateInviteRequest(event) => (
			event.server_id.to_bytes(),
			event.server_pubkey.to_bytes(),
			event.count,
			event.expiration,
		),
		_ => {
			warn!(
				"Malformed create invite event. No event present: {:?}",
				event
			);
			return Ok(true);
		}
	};

	let user_pubkey = match &conn_info.pubkey {
		Some(user_pubkey) => user_pubkey,
		None => {
			warn!("expected a user pubkey at this point. Event = {:?}", event);
			return Ok(true);
		}
	};

	if server_pubkey == pubkey!() {
		let invite_id =
			ds_context.create_invite(user_pubkey.to_bytes(), server_id, expiration, count)?;

		let event = Event {
			request_id,
			body: EventBody::CreateInviteResponse(CreateInviteResponse {
				invite_id,
				success: true,
			})
			.into(),
			..Default::default()
		};
		send!(conn_info.handle, event);
	} else {
		// TODO: implement remote invites
		warn!("remote invites not implemented yet.");
	}

	Ok(false)
}

pub fn list_invites(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	let request_id = event.request_id;
	let (server_id, server_pubkey) = match &event.body {
		EventBody::ListInvitesRequest(event) => {
			(event.server_id.to_bytes(), event.server_pubkey.to_bytes())
		}
		_ => {
			warn!(
				"Malformed list invites event. No event present: {:?}",
				event
			);
			return Ok(true);
		}
	};

	if server_pubkey == pubkey!() {
		let user_pubkey = match &conn_info.pubkey {
			Some(user_pubkey) => user_pubkey,
			None => {
				warn!("expected pubkey at this point. Event: {:?}", event);
				return Ok(true);
			}
		};
		let invites = ds_context.get_invites(Some(user_pubkey.to_bytes()), server_id)?;
		let event = Event {
			request_id,
			body: EventBody::ListInvitesResponse(ListInvitesResponse { invites }).into(),
			..Default::default()
		};
		send!(conn_info.handle, event);
	} else {
		// TODO: implement remote invites
		warn!("remote invites not implemented yet.");
	}
	Ok(false)
}

pub fn modify_invite(
	_conn_info: &ConnectionInfo,
	_ds_context: &DSContext,
	_event: &Event,
) -> Result<bool, ConcordError> {
	/*
			let (request_id, invite_id, max, expiration) =
					match event.modify_invite_request.0.as_ref() {
							Some(event) => (
									event.request_id,
									event.invite_id,
									event.max,
									event.expiration,
							),
							None => {
									warn!("Malformed modify invite event. No event present: {:?}", event);
									return Ok(true);
							}
					};
	*/
	warn!("modify not implemented");
	Ok(false)
}

pub fn delete_invite(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	let request_id = event.request_id;
	let invite_id = match &event.body {
		EventBody::DeleteInviteRequest(event) => event.invite_id,
		_ => {
			warn!(
				"Malformed delete invite event. No event present: {:?}",
				event
			);
			return Ok(true);
		}
	};

	info!("delete invite: {}", invite_id);
	ds_context.delete_invite(invite_id)?;

	let event = Event {
		request_id,
		body: EventBody::DeleteInviteResponse(DeleteInviteResponse { success: true }).into(),
		..Default::default()
	};
	send!(conn_info.handle, event);

	Ok(false)
}

fn parse_invite(event: &Event) -> Result<Option<([u8; 32], u32, u128, SerString)>, ConcordError> {
	let request_id = event.request_id;
	let invite_url = match &event.body {
		EventBody::ViewInviteRequest(event) => event.invite_url.clone(),
		EventBody::JoinServerRequest(event) => event.invite_url.clone(),
		_ => {
			warn!("Malformed event. Incorrect body present: {:?}", event);
			return Ok(None);
		}
	};

	let url = url::Url::parse(&invite_url.to_string())?;
	let onion = url.host();

	let onion = match onion {
		Some(onion) => onion,
		None => {
			warn!("Invalid link address in check_invite. No host.");
			return Ok(None);
		}
	}
	.to_string();

	let path = url.path();
	if path.find("/i/") != Some(0) {
		return Ok(None);
	}
	let id: u128 = (path.substring(3, path.len())).parse()?;
	let pubkey = Pubkey::from_onion(&onion)?.to_bytes();

	Ok(Some((pubkey, request_id, id, invite_url)))
}

pub fn view_invite(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
	conn_manager: Arc<RwLock<ConnManager>>,
	config: &ConcordConfig,
) -> Result<bool, ConcordError> {
	let (pubkey, request_id, id, invite_url) = match parse_invite(event)? {
		Some((pubkey, request_id, id, invite_url)) => (pubkey, request_id, id, invite_url),
		None => {
			return Ok(true);
		}
	};

	if pubkey == pubkey!() {
		// this is our host, we can process directly
		let jri = ds_context.check_invite(id, pubkey)?;
		info!("jri={:?}", jri);
		match jri {
			Some(jri) => {
				let event = Event {
					request_id,
					body: EventBody::ViewInviteResponse(ViewInviteResponse {
						response_info: Some(InviteResponseInfo {
							inviter_name: "".into(),
							inviter_icon: Image { data: vec![] },
							server_icon: Image { data: vec![] },
							server_name: jri.name.into(),
							current_members: 0,
							online_members: 0,
						}),
					}),
					..Default::default()
				};

				send!(conn_info.handle, event);
			}
			None => {
				info!("no jri!");
				let event = Event {
					request_id,
					body: EventBody::ViewInviteResponse(ViewInviteResponse {
						response_info: None,
					}),
					..Default::default()
				};

				send!(conn_info.handle, event);
			}
		}
	} else {
		let event = Event {
			body: EventBody::ViewInviteRequest(crate::types::ViewInviteRequest { invite_url }),
			..Default::default()
		};

		let mut conn_manager = nioruntime_util::lockw!(conn_manager)?;
		let handle = conn_info.handle.clone();
		conn_manager.send_event(
			pubkey,
			event,
			config.tor_port,
			Box::pin(move |event| {
				send!(handle, event);
				Ok(())
			}),
		)?;
	}

	Ok(false)
}

pub fn accept_invite(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	let request_id = event.request_id;
	let (invite_id, user_pubkey, server_pubkey, user_name, user_bio, avatar) = match &event.body {
		EventBody::AcceptInviteRequest(event) => (
			event.invite_id,
			event.user_pubkey,
			event.server_pubkey,
			event.user_name.data.clone(),
			event.user_bio.data.clone(),
			event.avatar.data.clone(),
		),
		_ => {
			warn!("Malformed event in accept_invite. Event = {:?}", event);
			return Ok(true);
		}
	};

	ds_context.accept_invite(
		invite_id,
		user_pubkey,
		server_pubkey,
		user_name,
		user_bio,
		avatar,
	)?;

	let event = Event {
		request_id,
		body: EventBody::AcceptInviteResponse(crate::types::AcceptInviteResponse { success: true }),
		..Default::default()
	};

	send!(conn_info.handle, event);

	Ok(false)
}

pub fn join_server(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
	conn_manager: Arc<RwLock<ConnManager>>,
	config: &ConcordConfig,
) -> Result<bool, ConcordError> {
	let (server_pubkey, request_id, invite_id, _invite_url) = match parse_invite(event)? {
		Some((pubkey, request_id, id, invite_url)) => (pubkey, request_id, id, invite_url),
		None => {
			return Ok(true);
		}
	};

	if server_pubkey == pubkey!() {
		// Something's wrong. We can't join our own server
		warn!("tried to join our own server: {:?}", event);
	} else {
		let pubkey = Pubkey::from_bytes(pubkey!());
		let profiles = ds_context.get_profiles(
			vec![pubkey.clone()],
			pubkey,
			ServerId::from_bytes(LOCAL_SERVER_ID),
		)?;

		let (user_name, user_bio) = match profiles.len() == 1 {
			true => match &profiles[0] {
				Some(profile_data) => (
					profile_data.profile_data.user_name.clone(),
					profile_data.profile_data.user_bio.clone(),
				),
				None => ("".to_string().into(), "".to_string().into()),
			},
			false => ("".to_string().into(), "".to_string().into()),
		};

		let event = Event {
			request_id,
			body: EventBody::AcceptInviteRequest(crate::types::AcceptInviteRequest {
				invite_id,
				user_pubkey: pubkey!(),
				server_pubkey,
				user_name,
				user_bio,
				avatar: Image { data: vec![] },
			}),
			..Default::default()
		};

		let mut conn_manager = nioruntime_util::lockw!(conn_manager)?;
		let handle = conn_info.handle.clone();
		conn_manager.send_event(
			server_pubkey,
			event,
			config.tor_port,
			Box::pin(move |event| {
				send!(handle, event);
				Ok(())
			}),
		)?;
	}

	Ok(false)
}

#[derive(Serialize)]
struct InviteResponse {
	invite_url: String,
}

#[derive(Serialize)]
pub struct InviteSerde {
	inviter: [u8; 32],
	server_id: [u8; 8],
	url: String,
	expiry: u64,
	cur: u64,
	max: u64,
	id: String,
}

#[derive(Serialize)]
pub struct ServerInfoSerde {
	pubkey: String,
	server_id: String,
	name: String,
	icon: String,
}

#[derive(Serialize)]
pub struct ServerStateSerde {
	sinfo: ServerInfoSerde,
	channels: Vec<Channel>,
	members: String,
}

#[derive(Serialize)]
pub struct InviteResponseDisplay {
	server_pubkey: String,
	server_id: String,
	name: String,
	inviter_pubkey: String,
}

impl From<ServerInfoReply> for ServerInfoSerde {
	fn from(si: ServerInfoReply) -> ServerInfoSerde {
		let pubkey = base64::encode(si.pubkey);
		let pubkey = urlencoding::encode(&pubkey).to_string();
		let server_id = base64::encode(si.server_id);
		let server_id = urlencoding::encode(&server_id).to_string();
		let name = si.name;
		/*
		let icon = base64::encode(&si.icon[..]);
		let icon = urlencoding::encode(&icon).to_string();
		*/
		let icon = "".to_string();
		ServerInfoSerde {
			pubkey,
			server_id,
			name,
			icon,
		}
	}
}
