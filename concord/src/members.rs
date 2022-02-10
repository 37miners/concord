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

use crate::send;
use crate::types::ConnectionInfo;
use crate::types::GetMembersResponse;
use crate::types::{Event, EventBody};
use concorddata::concord::DSContext;
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;

debug!(); // set log level to debug

pub fn get_members(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	let (server_id, server_pubkey, batch_num) = match &event.body {
		EventBody::GetMembersRequest(event) => (
			event.server_id.clone(),
			event.server_pubkey.clone(),
			event.batch_num,
		),
		_ => {
			warn!(
				"Malformed get_members_request event. No event present: {:?}",
				event
			);
			return Ok(true);
		}
	};
	// TODO: online/offline and role ordering

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

	let mut types_members: Vec<crate::types::Member> = vec![];
	for member in members {
		types_members.push(member.into());
	}

	let event = Event {
		body: EventBody::GetMembersResponse(GetMembersResponse {
			batch_num,
			server_id,
			server_pubkey,
			members: types_members,
		})
		.into(),
		..Default::default()
	};

	send!(conn_info.handle, event);

	Ok(false)
}

#[derive(Serialize)]
struct MemberJson {
	server_id: String,
	user_pubkey: String,
	user_name: String,
	user_bio: String,
	user_pubkey_urlencoded: String,
	user_type: u8,
}
