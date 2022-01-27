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
use crate::utils::extract_server_id_from_query;
use crate::utils::extract_server_pubkey_from_query;
use crate::utils::Pubkey;
use concordconfig::ConcordConfig;
use concorddata::concord::DSContext;
use concorddata::concord::Member;
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use nioruntime_tor::ov3::OnionV3Address;

debug!(); // set log level to debug

#[derive(Serialize)]
struct MemberJson {
	server_id: String,
	user_pubkey: String,
	user_name: String,
	user_bio: String,
	user_pubkey_urlencoded: String,
	user_type: u8,
}

impl From<&Member> for MemberJson {
	fn from(member: &Member) -> MemberJson {
		let server_id = base64::encode(member.server_id);
		let server_id = urlencoding::encode(&server_id).to_string();
		let user_pubkey = OnionV3Address::from_bytes(member.user_pubkey).to_string();
		let user_pubkey_urlencoded = Pubkey::from_bytes(member.user_pubkey)
			.to_urlencoding()
			.unwrap_or("".to_string());
		let (user_name, user_bio) = match &member.profile {
			Some(profile) => (
				profile.profile_data.user_name.clone(),
				profile.profile_data.user_bio.clone(),
			),
			None => ("".to_string(), "".to_string()),
		};

		MemberJson {
			server_id,
			user_pubkey,
			user_name,
			user_bio,
			user_pubkey_urlencoded,
			user_type: if member.auth_flags == 3 { 1u8 } else { 0u8 },
		}
	}
}

pub fn init_members(config: &ConcordConfig, _context: ConcordContext) -> Result<(), ConcordError> {
	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	rustlet!("get_members", {
		let server_id = extract_server_id_from_query()?;
		let server_pubkey = match extract_server_pubkey_from_query() {
			Ok(server_pubkey) => server_pubkey,
			Err(_) => Pubkey::from_bytes(pubkey!()),
		};

		let mut members = ds_context
			.get_members(
				server_pubkey.to_bytes(),
				server_id.to_bytes(),
				0,
				true,
				true,
			)
			.map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(format!(
					"error accepting invite - members: {}",
					e.to_string()
				))
				.into();
				error
			})?;

		let mut other_members = ds_context
			.get_members(
				server_pubkey.to_bytes(),
				server_id.to_bytes(),
				0,
				true,
				false,
			)
			.map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(format!(
					"error accepting invite - members: {}",
					e.to_string()
				))
				.into();
				error
			})?;
		members.append(&mut other_members);

		let mut members_json: Vec<MemberJson> = vec![];

		for member in &members {
			members_json.push(member.into());
		}

		let json = serde_json::to_string(&members_json).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string())).into();
			error
		})?;
		response!("{}", json);
	});
	rustlet_mapping!("/get_members", "get_members");

	Ok(())
}
