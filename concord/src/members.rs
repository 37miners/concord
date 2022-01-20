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
use concorddata::concord::DSContext;
use concorddata::concord::Member;
use concorddata::concord::AUTH_FLAG_OWNER;
use concorderror::Error as ConcordError;
use librustlet::*;
use nioruntime_tor::ov3::OnionV3Address;
use std::convert::TryInto;

nioruntime_log::debug!(); // set log level to debug

#[derive(Serialize)]
struct MemberJson {
	server_id: String,
	user_pubkey: String,
	user_type: u8,
}

impl From<Member> for MemberJson {
	fn from(member: Member) -> MemberJson {
		let server_id = base64::encode(member.server_id);
		let server_id = urlencoding::encode(&server_id).to_string();

		let user_pubkey = OnionV3Address::from_bytes(member.user_pubkey).to_string();

		MemberJson {
			server_id,
			user_pubkey,
			user_type: 0u8,
		}
	}
}

pub fn init_members(config: &ConcordConfig, _context: ConcordContext) -> Result<(), ConcordError> {
	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;

	rustlet!("get_members", {
		let server_id = query!("server_id");
		let server_id = urlencoding::decode(&server_id)?;
		let server_id = base64::decode(&*server_id)?;
		let server_id: [u8; 8] = server_id.as_slice().try_into()?;

		let members = ds_context.get_members(server_id).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("get members error: {}", e.to_string())).into();
			error
		})?;

		let mut members_json: Vec<MemberJson> = vec![];

		for member in &members {
			let mut member_json: MemberJson = member.0.clone().into();
			member_json.user_type = if (member.1 & AUTH_FLAG_OWNER) != 0 {
				1u8
			} else {
				0u8
			};
			members_json.push(member_json);
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
