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
use concordconfig::ConcordConfig;
use concorddata::concord::Channel;
use concorddata::concord::DSContext;
use concorddata::concord::JoinInfoReply;
use concorddata::concord::MemberList;
use concorddata::concord::ServerInfo;
use concorddata::concord::ServerInfoReply;
use concorderror::Error as ConcordError;
use concordutil::torclient;
use librustlet::*;
use nioruntime_tor::ov3::OnionV3Address;
use serde_json::Value;
use std::convert::TryInto;
use url::Host::Domain;
use url::Url;

const ACCEPT_INVITE_FAIL: &str = "{\"success\": false}";
const SUCCESS: &str = "{\"success\": true}";

nioruntime_log::debug!(); // set log level to debug

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
		let icon = base64::encode(&si.icon[..]);
		let icon = urlencoding::encode(&icon).to_string();
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
			.create_invite(inviter, server_id, expiry, count)
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

		let join_info_reply = ds_context.check_invite(invite_id).map_err(|e| {
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
			let join_info_reply = ds_context.check_invite(invite_id).map_err(|e| {
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

		let sinfo = ds_context
			.accept_invite(invite_id, user_pubkey, server_pubkey)
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

				let members = ds_context.get_members(sinfo.server_id).map_err(|e| {
					let error: Error = ErrorKind::ApplicationError(format!(
						"error accepting invite - members: {}",
						e.to_string()
					))
					.into();
					error
				})?;

				let members = MemberList::new(members, server_pubkey, sinfo.server_id)
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

		let invites = ds_context.get_invites(inviter, server_id).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("error listing invite: {}", e.to_string()))
					.into();
			error
		})?;

		let mut invites_serde = vec![];
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
							icon: jri.icon,
							joined: false,
						};
						ds_context
							.add_server(server_info, Some(jri.server_id), Some(pubkey!()))
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
					None => {}
				}
			}
			None => {}
		}
	});
	rustlet_mapping!("/view_invite", "view_invite");

	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	rustlet!("join_server", {
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
			"{}&timestamp={}&user_pubkey={}&signature={}",
			link, timestamp, onion, signature,
		);
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
				let members = MemberList::from_b58(members)
					.unwrap()
					.read_member_list()
					.unwrap();

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
						let icon = base64::decode(icon)?;

						let server_info = ServerInfo {
							pubkey: pubkey,
							name,
							icon,
							joined: true,
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
