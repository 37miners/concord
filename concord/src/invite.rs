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
use crate::context::ConcordContext;
use crate::send;
use crate::types::{
	ConnectionInfo, CreateInviteResponse, DeleteInviteResponse, Event, EventBody, Image,
	InviteResponseInfo, ListInvitesResponse, ViewInviteResponse,
};
use concordconfig::ConcordConfig;
use concorddata::concord::Channel;
use concorddata::concord::DSContext;
use concorddata::concord::JoinInfoReply;
use concorddata::concord::MemberList;
use concorddata::concord::Profile;
use concorddata::concord::ServerInfo;
use concorddata::concord::ServerInfoReply;
use concorddata::types::Pubkey;
use concorddata::types::SerString;
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use concordutil::torclient;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use nioruntime_tor::ov3::OnionV3Address;
use serde_json::Value;
use std::convert::TryInto;
use std::sync::{Arc, RwLock};
use substring::Substring;
use url::Host::Domain;
use url::Url;

const ACCEPT_INVITE_FAIL: &str = "{\"success\": false}";
const SUCCESS: &str = "{\"success\": true}";

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
		_ => {
			warn!("Malformed view invite event. No event present: {:?}", event);
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
	_conn_info: &ConnectionInfo,
	_ds_context: &DSContext,
	_event: &Event,
) -> Result<bool, ConcordError> {
	Ok(false)
}

pub fn join_server(
	conn_info: &ConnectionInfo,
	_ds_context: &DSContext,
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
		let event = Event {
			request_id,
			body: EventBody::AcceptInviteRequest(crate::types::AcceptInviteRequest {
				invite_id,
				user_pubkey: pubkey!(),
				server_pubkey,
				user_name: "ourname".to_string().into(),
				user_bio: "ourbio".to_string().into(),
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
	/*
							EventBody::JoinServerRequest(event) => event.invite_url.clone(),
							_ => {
									warn!("Malformed join server event. No event present: {:?}", event);
									return Ok(true);
							},
					};

			let url = url::Url::parse(&invite_url.to_string())?;
			let onion = url.host();

			let onion = match onion {
					Some(onion) => onion,
					None => {
							warn!("Invalid link address in check_invite. No host.");
							return Ok(true);
					}
			}
			.to_string();

			let path = url.path();
			if path.find("/i/") != Some(0) {
					return Ok(true);
			}
			let id: u128 = (path.substring(3, path.len())).parse()?;
			let pubkey = Pubkey::from_onion(&onion)?.to_bytes();

			if pubkey == pubkey!() {
			// Something's wrong. We can't join our own server
			warn!("tried to join our own server: {:?}", event);
		} else {
					// this is our host, we can process directly
					let sinfo = ds_context
							.accept_invite(
									invite_id,
									user_pubkey,
									server_pubkey,
									user_name,
									user_bio,
									avatar,
							)
							.map_err(|e| {
									let error: Error = ErrorKind::ApplicationError(format!(
											"error accepting invite: {}",
											e.to_string()
									))
									.into();
									error
							})?;
					info!("jri={:?}", jri);
					match jri {
							Some(jri) => {
									let event = Event {
											request_id,
											body: EventBody::JoinServerResponse(JoinServerResponse {
							success: true,
											}),
											..Default::default()
									};

									send!(conn_info.handle, event);
							}
							None => {
									info!("no jri!");
									let event = Event {
											request_id,
											body: EventBody::(ViewInviteResponse {
													response_info: None,
											}),
											..Default::default()
									};
	 //                               send!(conn_info.handle, event);
							}
					}

			} else {
					let event = Event {
							body: EventBody::ViewInviteRequest(crate::types::ViewInviteRequest{
									invite_url,
							}),
							..Default::default()
					};
		}
	*/

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

// build a signable message from a message/key.
fn build_signable_message(pubkey: String, timestamp: u128, link: String) -> Result<Vec<u8>, Error> {
	let mut ret = vec![];
	ret.append(&mut pubkey.as_bytes().to_vec());
	ret.append(&mut timestamp.to_be_bytes().to_vec());
	ret.append(&mut link.as_bytes().to_vec());
	Ok(ret)
}

pub fn init_invite(config: &ConcordConfig, _context: ConcordContext) -> Result<(), ConcordError> {
	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	// send a message
	rustlet!("create_invite", {
		let pubkey = pubkey!();
		let server_id = query!("server_id").unwrap_or("".to_string());
		let inviter = query!("inviter").unwrap_or("".to_string());
		let count = query!("count").unwrap_or("".to_string()).parse()?;
		let expiry = query!("expiry").unwrap_or("".to_string()).parse();

		// if not specified, use indefinite.
		let expiry = match expiry {
			Ok(e) => {
				if e == 0 {
					u64::MAX
				} else {
					e
				}
			}
			Err(_) => u64::MAX,
		};

		let server_id = urlencoding::decode(&server_id)?;
		let server_id = base64::decode(&*server_id)?;
		let server_id: [u8; 8] = server_id.as_slice().try_into()?;

		let inviter = if inviter == "" {
			pubkey
		} else {
			let inviter = urlencoding::decode(&inviter)?;
			let inviter = base64::decode(&*inviter)?;
			let inviter: [u8; 32] = inviter.as_slice().try_into()?;
			inviter
		};

		let id = ds_context
			.create_invite(inviter, server_id, expiry.try_into().unwrap_or(0), count)
			.map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(format!(
					"error creating invite: {}",
					e.to_string()
				))
				.into();
				error
			})?;

		let id = base64::encode(id.to_be_bytes());
		let id = urlencoding::encode(&id);

		let onion = OnionV3Address::from_bytes(pubkey);
		let invite_url = InviteResponse {
			invite_url: format!("http://{}.onion/i?id={}", onion, id),
		};

		let json = serde_json::to_string(&invite_url).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string())).into();
			error
		})?;
		response!("{}", json);
	});
	rustlet_mapping!("/create_invite", "create_invite");

	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	rustlet!("check_invite", {
		let invite_id = query!("id").unwrap_or("".to_string());
		let invite_id = urlencoding::decode(&invite_id)?;
		let invite_id = base64::decode(&*invite_id)?;
		let invite_id: [u8; 16] = invite_id.as_slice().try_into()?;
		let invite_id = u128::from_be_bytes(invite_id);

		let join_info_reply = ds_context.check_invite(invite_id, pubkey!()).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("error checking invite: {}", e)).into();
			error
		})?;

		match join_info_reply {
			Some(join_info) => {
				let json = serde_json::to_string(&join_info).map_err(|e| {
					let error: Error =
						ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string()))
							.into();
					error
				})?;
				response!("{}", json);
			}
			None => {
				response!("{}", ACCEPT_INVITE_FAIL);
			}
		}
	});
	rustlet_mapping!("/check_invite", "check_invite");

	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	// accept an invite
	rustlet!("accept_invite", {
		let view_only = query!("view_only").is_some();
		let invite_id = query!("id").unwrap_or("".to_string());
		let invite_id = urlencoding::decode(&invite_id)?;
		let invite_id = base64::decode(&*invite_id)?;
		let invite_id: [u8; 16] = invite_id.as_slice().try_into()?;
		let invite_id = u128::from_be_bytes(invite_id);
		if view_only {
			let join_info_reply = ds_context.check_invite(invite_id, pubkey!()).map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("error checking invite: {}", e)).into();
				error
			})?;

			match join_info_reply {
				Some(join_info) => {
					let json = serde_json::to_string(&join_info).map_err(|e| {
						let error: Error =
							ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string()))
								.into();
						error
					})?;
					response!("{}", json);
				}
				None => {
					response!("{}", ACCEPT_INVITE_FAIL);
				}
			}
			return Ok(());
		}

		let user_pubkey = query!("user_pubkey").unwrap_or("".to_string());
		let _timestamp: u64 = query!("timestamp").unwrap_or("".to_string()).parse()?;
		let _signature = query!("signature").unwrap_or("".to_string());

		let user_pubkey = urlencoding::decode(&user_pubkey)?;
		let user_pubkey = base64::decode(&*user_pubkey)?;
		let user_pubkey: [u8; 32] = user_pubkey.as_slice().try_into()?;
		let server_pubkey = pubkey!();

		let user_name = match query!("user_name") {
			Some(user_name) => urlencoding::decode(&user_name)?.to_string(),
			None => "".to_string(),
		};

		let user_bio = match query!("user_bio") {
			Some(user_bio) => urlencoding::decode(&user_bio)?.to_string(),
			None => "".to_string(),
		};

		let avatar: Vec<u8> = request_content!().to_vec();

		let sinfo = ds_context
			.accept_invite(
				invite_id,
				user_pubkey,
				server_pubkey,
				user_name,
				user_bio,
				avatar,
			)
			.map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(format!(
					"error accepting invite: {}",
					e.to_string()
				))
				.into();
				error
			})?;

		match sinfo {
			Some(sinfo) => {
				// get channel info and member info
				let server_pubkey = pubkey!();
				let channels = ds_context
					.get_channels(server_pubkey, sinfo.server_id)
					.map_err(|e| {
						let error: Error = ErrorKind::ApplicationError(format!(
							"error accepting invite - channels: {}",
							e.to_string()
						))
						.into();
						error
					})?;

				let mut members = ds_context
					.get_members(sinfo.pubkey, sinfo.server_id, 0, false, true)
					.map_err(|e| {
						let error: Error = ErrorKind::ApplicationError(format!(
							"error accepting invite - members: {}",
							e.to_string()
						))
						.into();
						error
					})?;

				let mut other_members = ds_context
					.get_members(sinfo.pubkey, sinfo.server_id, 0, false, false)
					.map_err(|e| {
						let error: Error = ErrorKind::ApplicationError(format!(
							"error accepting invite - members: {}",
							e.to_string()
						))
						.into();
						error
					})?;

				members.append(&mut other_members);

				let mut user_pubkeys = vec![];
				for member in &members {
					user_pubkeys.push(member.user_pubkey);
				}
				let profile_data_vec = ds_context
					.get_profile_data(user_pubkeys, sinfo.pubkey, sinfo.server_id)
					.map_err(|e| {
						let error: Error = ErrorKind::ApplicationError(format!(
							"error getting profile data: {}",
							e
						))
						.into();
						error
					})?;

				let mut i = 0;
				for profile_data in profile_data_vec {
					match profile_data {
						Some(profile_data) => {
							members[i].profile = Some(Profile {
								server_id: sinfo.server_id,
								server_pubkey: sinfo.pubkey,
								user_pubkey: members[i].user_pubkey,
								avatar: vec![],
								profile_data,
							});
						}
						None => {}
					}
					i += 1;
				}

				let members = MemberList::new(members)
					.map_err(|e| {
						let error: Error = ErrorKind::ApplicationError(format!(
							"error accepting invite - b58 members: {}",
							e.to_string()
						))
						.into();
						error
					})?
					.to_b58()
					.map_err(|e| {
						let error: Error = ErrorKind::ApplicationError(format!(
							"error accepting invite - b58 members: {}",
							e.to_string()
						))
						.into();
						error
					})?;
				let sinfo: ServerInfoSerde = sinfo.into();
				let server_state = ServerStateSerde {
					sinfo,
					members,
					channels,
				};

				let json = serde_json::to_string(&server_state).map_err(|e| {
					let error: Error =
						ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string()))
							.into();
					error
				})?;
				response!("{}", json);
			}
			None => {
				response!("{}", ACCEPT_INVITE_FAIL);
			}
		}
	});
	rustlet_mapping!("/i", "accept_invite");

	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	rustlet!("revoke_invite", {
		let invite_id = query!("invite_id").unwrap_or("".to_string()).parse()?;
		ds_context.delete_invite(invite_id).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("error deleting invite: {}", e.to_string()))
					.into();
			error
		})?;
	});
	rustlet_mapping!("/revoke_invite", "revoke_invite");

	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	rustlet!("list_invites", {
		let server_id = query!("server_id").unwrap_or("".to_string());
		let inviter = query!("inviter").unwrap_or("".to_string());

		let server_id = urlencoding::decode(&server_id)?;
		let server_id = base64::decode(&*server_id)?;
		let server_id: [u8; 8] = server_id.as_slice().try_into()?;

		let inviter = if inviter.len() > 0 {
			let inviter = urlencoding::decode(&inviter)?;
			let inviter = base64::decode(&*inviter)?;
			let inviter: [u8; 32] = inviter.as_slice().try_into()?;
			Some(inviter)
		} else {
			None
		};

		let _invites = ds_context.get_invites(inviter, server_id).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("error listing invite: {}", e.to_string()))
					.into();
			error
		})?;

		/*
				//let mut invites_serde = vec![];
				let pubkey = pubkey!();
				let onion = OnionV3Address::from_bytes(pubkey);
				for invite in invites {
					let id = base64::encode(invite.id.to_be_bytes());
					let id = urlencoding::encode(&id);

					let url = format!("http://{}.onion/i?id={}", onion, id);
					invites_serde.push(InviteSerde {
						inviter: invite.inviter,
						server_id: invite.server_id,
						url,
						expiry: invite.expiry,
						cur: invite.cur,
						max: invite.max,
						id: invite.id.to_string(),
					});
				}

				let json = serde_json::to_string(&invites_serde).map_err(|e| {
					let error: Error =
						ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string())).into();
					error
				})?;
				response!("{}", json);
		*/
	});
	rustlet_mapping!("/list_invites", "list_invites");

	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	let tor_port = config.tor_port;
	rustlet!("view_invite", {
		let link = query!("link").unwrap_or("".to_string());
		let link = urlencoding::decode(&link)?.to_string();
		let join_link = format!("{}&view_only=true", link,);
		let url = Url::parse(&join_link).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("url parse error: {}", e.to_string())).into();
			error
		})?;
		let host = format!("{}", url.host().unwrap_or(Domain("notfound")));
		let path = format!("{}?{}", url.path(), url.query().unwrap_or(""));
		let res = torclient::do_get(host.clone(), path.clone(), tor_port).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("tor client error: {}", e.to_string())).into();
			error
		})?;
		let start = res.find("\r\n\r\n");

		match start {
			Some(start) => {
				let json_text = &res[(start + 4)..];
				let join_reply_info: Option<JoinInfoReply> = serde_json::from_str(json_text)
					.map_err(|e| {
						let error: Error = ErrorKind::ApplicationError(format!(
							"serde json parse error join_reply_info: {}",
							e
						))
						.into();
						error
					})?;
				match join_reply_info {
					Some(jri) => {
						let server_info = ServerInfo {
							pubkey: jri.server_pubkey,
							name: jri.name.clone(),
							joined: false,
							seqno: 1, // TODO: this is just to get this to compile.
							          // when join is revisited need to get the seqno from
							          // remote server
						};
						ds_context
							.add_server(server_info, Some(jri.server_id), Some(pubkey!()), true)
							.map_err(|e| {
								let error: Error = ErrorKind::ApplicationError(format!(
									"add server generated error: {}",
									e
								))
								.into();
								error
							})?;
						let server_pubkey = base64::encode(jri.server_pubkey);
						let server_pubkey = urlencoding::encode(&server_pubkey).to_string();
						let server_id = base64::encode(jri.server_id);
						let server_id = urlencoding::encode(&server_id).to_string();
						let inviter_pubkey = base64::encode(jri.inviter_pubkey);
						let inviter_pubkey = urlencoding::encode(&inviter_pubkey).to_string();
						let invite_response = InviteResponseDisplay {
							server_pubkey,
							server_id,
							inviter_pubkey,
							name: jri.name,
						};

						let json = serde_json::to_string(&invite_response).map_err(|e| {
							let error: Error = ErrorKind::ApplicationError(format!(
								"json parse error on invite response: {}",
								e
							))
							.into();
							error
						})?;
						response!("{}", json);
					}
					None => {
						error!("no jri!");
					}
				}
			}
			None => {
				error!("no proper response found!");
			}
		}
	});
	rustlet_mapping!("/view_invite", "view_invite");

	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	rustlet!("join_server", {
		let user_pubkey = pubkey!();
		let profile = ds_context
			.get_profile(user_pubkey, user_pubkey, [0u8; 8])
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("error getting profile: {}", e)).into();
				error
			})?;
		let (user_name, user_bio, avatar) = match profile {
			Some(profile) => (
				profile.profile_data.user_name,
				profile.profile_data.user_bio,
				profile.avatar,
			),
			None => ("".to_string(), "".to_string(), vec![]),
		};

		let user_name = urlencoding::encode(&user_name);
		let user_bio = urlencoding::encode(&user_bio);

		let link = query!("link").unwrap_or("".to_string());
		let link = urlencoding::decode(&link)?.to_string();

		let timestamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("time error: {}", e.to_string())).into();
				error
			})?
			.as_millis();

		let user_pubkey = pubkey!();
		let onion = base64::encode(user_pubkey);
		let onion = urlencoding::encode(&onion).to_string();

		let signature = sign!(&build_signable_message(
			onion.clone(),
			timestamp,
			link.clone()
		)?)
		.unwrap_or([0u8; 64]);
		let signature = base64::encode(signature);
		let signature = urlencoding::encode(&signature);

		let join_link = format!(
			"{}&timestamp={}&user_pubkey={}&signature={}&user_name={}&user_bio={}",
			link, timestamp, onion, signature, user_name, user_bio
		);
		let url = Url::parse(&join_link).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("url parse error: {}", e.to_string())).into();
			error
		})?;
		let host = format!("{}", url.host().unwrap_or(Domain("notfound")));
		let path = format!("{}?{}", url.path(), url.query().unwrap_or(""));
		let res =
			torclient::do_post(host.clone(), path.clone(), tor_port, avatar).map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("tor client error: {}", e.to_string()))
						.into();
				error
			})?;

		let start = res.find("\r\n\r\n");

		match start {
			Some(start) => {
				let json_text = &res[(start + 4)..];
				let value: Value = serde_json::from_str(json_text).map_err(|e| {
					let error: Error =
						ErrorKind::ApplicationError(format!("json parse error: {}", e.to_string()))
							.into();
					error
				})?;

				let value_sinfo: Option<&Value> = value.get("sinfo");

				let empty = vec![];
				let empty = Value::Array(empty);
				let empty2 = vec![];
				let channels = value
					.get("channels")
					.unwrap_or(&empty)
					.as_array()
					.unwrap_or(&empty2);

				let mut channels_vec = vec![];
				for channel in channels {
					let channel = Channel {
						name: channel.get("name").unwrap().as_str().unwrap().to_string(),
						description: channel
							.get("description")
							.unwrap()
							.as_str()
							.unwrap()
							.to_string(),
						channel_id: channel.get("channel_id").unwrap().as_u64().unwrap(),
					};
					channels_vec.push(channel);
				}
				let channels = channels_vec;

				let members = value.get("members").unwrap().as_str().unwrap().to_string();
				let members = MemberList::from_b58(members).map_err(|e| {
					let error: Error = ErrorKind::ApplicationError(format!(
						"error converting memberlist to b58: {}",
						e
					))
					.into();
					error
				})?;

				match value_sinfo {
					Some(value) => {
						let server_id = match value.get("server_id") {
							Some(server_id) => server_id.as_str().unwrap_or(""),
							None => "",
						}
						.to_string();

						let pubkey = match value.get("pubkey") {
							Some(pubkey) => pubkey.as_str().unwrap_or(""),
							None => "",
						};
						let pubkey = urlencoding::decode(pubkey)?.to_string();
						let pubkey = base64::decode(pubkey)?.as_slice().try_into()?;

						let name = match value.get("name") {
							Some(name) => name.as_str().unwrap_or(""),
							None => "",
						}
						.to_string();
						let icon = match value.get("icon") {
							Some(icon) => icon.as_str().unwrap_or(""),
							None => "",
						};
						let icon = urlencoding::decode(icon)?.to_string();
						let _icon = base64::decode(icon)?;

						let server_info = ServerInfo {
							pubkey: pubkey,
							name,
							joined: true,
							seqno: 1, // TODO: need to revisit this. Just making it compile now.
						};
						ds_context
							.add_remote_server(server_id, server_info, channels, members)
							.map_err(|e| {
								let error: Error = ErrorKind::ApplicationError(format!(
									"add remote server error: {}",
									e
								))
								.into();
								error
							})?;
						response!("{}", SUCCESS);
						return Ok(());
					}
					None => {}
				}
			}
			None => {}
		}
		response!("{}", ACCEPT_INVITE_FAIL);
	});
	rustlet_mapping!("/join_server", "join_server");

	Ok(())
}
