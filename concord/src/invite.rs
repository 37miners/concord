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

use concorddata::concord::DSContext;
use concorderror::Error as ConcordError;
use concordutil::torclient;
use librustlet::*;
use nioruntime_tor::ov3::OnionV3Address;
use std::convert::TryInto;
use url::Host::Domain;
use url::Url;

const ACCEPT_INVITE_SUCCESS: &str = "{\"success\": true}";
const ACCEPT_INVITE_FAIL: &str = "{\"success\": false}";

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

// build a signable message from a message/key.
fn build_signable_message(pubkey: String, timestamp: u128, link: String) -> Result<Vec<u8>, Error> {
	let mut ret = vec![];
	ret.append(&mut pubkey.as_bytes().to_vec());
	ret.append(&mut timestamp.to_be_bytes().to_vec());
	ret.append(&mut link.as_bytes().to_vec());
	Ok(ret)
}

pub fn init_invite(root_dir: String) -> Result<(), ConcordError> {
	// create a ds context. Each rustlet needs it's own
	let ds_context = DSContext::new(root_dir.clone())?;

	// send a message
	rustlet!("create_invite", {
		let pubkey = pubkey!().unwrap_or([0u8; 32]);
		let server_id = query!("server_id");
		let inviter = query!("inviter");
		let count = query!("count").parse()?;
		let expiry = query!("expiry").parse();

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

	// create a ds context. Each rustlet needs it's own
	let ds_context = DSContext::new(root_dir.clone())?;

	// accept an invite
	rustlet!("accept_invite", {
		let invite_id = query!("id");
		let user_pubkey = query!("user_pubkey");
		let _timestamp: u64 = query!("timestamp").parse()?;
		let _signature = query!("signature");
		let invite_id = urlencoding::decode(&invite_id)?;
		let invite_id = base64::decode(&*invite_id)?;
		let invite_id: [u8; 16] = invite_id.as_slice().try_into()?;
		let invite_id = u128::from_be_bytes(invite_id);

		let user_pubkey = urlencoding::decode(&user_pubkey)?;
		let user_pubkey = base64::decode(&*user_pubkey)?;
		let user_pubkey: [u8; 32] = user_pubkey.as_slice().try_into()?;

		let success = ds_context
			.accept_invite(invite_id, user_pubkey)
			.map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(format!(
					"error accepting invite: {}",
					e.to_string()
				))
				.into();
				error
			})?;

		match success {
			true => {
				response!("{}", ACCEPT_INVITE_SUCCESS);
			}
			false => {
				response!("{}", ACCEPT_INVITE_FAIL);
			}
		}
	});
	rustlet_mapping!("/i", "accept_invite");

	// create a ds context. Each rustlet needs it's own
	let ds_context = DSContext::new(root_dir.clone())?;

	rustlet!("revoke_invite", {
		let invite_id = query!("invite_id").parse()?;
		ds_context.delete_invite(invite_id).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("error deleting invite: {}", e.to_string()))
					.into();
			error
		})?;
	});
	rustlet_mapping!("/revoke_invite", "revoke_invite");

	// create a ds context. Each rustlet needs it's own
	let ds_context = DSContext::new(root_dir.clone())?;

	rustlet!("list_invites", {
		let server_id = query!("server_id");
		let inviter = query!("inviter");

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
		let pubkey = pubkey!().unwrap_or([0u8; 32]);
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

	rustlet!("join_server", {
		let link = query!("link");
		let link = urlencoding::decode(&link)?.to_string();

		let timestamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("time error: {}", e.to_string())).into();
				error
			})?
			.as_millis();

		let pubkey = pubkey!().unwrap_or([0u8; 32]);
		let onion = base64::encode(pubkey);
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
		let res = torclient::do_get(host.clone(), path.clone(), 9050).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("tor client error: {}", e.to_string())).into();
			error
		})?;
		println!("join server: host: {} path: {}, res={:?}", host, path, res);
	});
	rustlet_mapping!("/join_server", "join_server");

	Ok(())
}
