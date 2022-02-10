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

use crate::auth::ws_auth;
use crate::channel::{add_channel, delete_channel, get_channels, modify_channel};
use crate::conn_manager::ConnManager;
use crate::invite::{
	accept_invite, create_invite, delete_invite, join_server, list_invites, modify_invite,
	view_invite,
};
use crate::members::get_members;
use crate::profile::{get_profile, set_profile};
use crate::server::{create_server, delete_server, get_servers, modify_server};
use crate::types::*;
use crate::{bin_event, close, send, try2};
use concordconfig::ConcordConfig;
use concorddata::concord::DSContext;
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

info!();

fn process_open(
	handle: ConnData,
	conn_info: Arc<RwLock<HashMap<u128, ConnectionInfo>>>,
) -> Result<(), Error> {
	let id = handle.get_connection_id();
	debug!("websocket open: {}", id);

	let event = Event {
		body: EventBody::ChallengeEvent(ChallengeEvent { challenge: id }).into(),
		..Default::default()
	};
	send!(handle, event);

	let mut conn_info = nioruntime_util::lockw!(conn_info)?;

	conn_info.insert(
		id,
		ConnectionInfo {
			handle: handle.clone(),
			pubkey: None,
		},
	);

	Ok(())
}

fn process_authed_event(
	event: &Event,
	connection_info: &ConnectionInfo,
	ds_context: &DSContext,
	config: &ConcordConfig,
	conn_manager: Arc<RwLock<ConnManager>>,
	id: u128,
) -> Result<bool, Error> {
	let res = match event.body {
		EventBody::GetServersEvent(_) => {
			try2!(
				get_servers(connection_info, ds_context),
				"get_servers error"
			)
		}
		EventBody::CreateServerEvent(_) => {
			try2!(
				create_server(connection_info, ds_context, &event, config),
				"create_server error"
			)
		}
		EventBody::DeleteServerEvent(_) => {
			try2!(
				delete_server(connection_info, ds_context, &event),
				"delete_server error"
			)
		}
		EventBody::ModifyServerEvent(_) => {
			try2!(
				modify_server(connection_info, ds_context, &event, config),
				"modify_server error"
			)
		}
		EventBody::GetChannelsRequest(_) => {
			try2!(
				get_channels(connection_info, ds_context, &event),
				"get_channels error"
			)
		}
		EventBody::AddChannelRequest(_) => {
			try2!(
				add_channel(connection_info, ds_context, &event),
				"add_channel error"
			)
		}
		EventBody::ModifyChannelRequest(_) => {
			try2!(
				modify_channel(connection_info, ds_context, &event),
				"modify channel error"
			)
		}
		EventBody::DeleteChannelRequest(_) => {
			try2!(
				delete_channel(connection_info, ds_context, &event),
				"delete channel error"
			)
		}
		EventBody::GetMembersRequest(_) => {
			try2!(
				get_members(connection_info, ds_context, &event),
				"get members error"
			)
		}
		EventBody::CreateInviteRequest(_) => {
			try2!(
				create_invite(connection_info, ds_context, &event),
				"create invite request error"
			)
		}
		EventBody::ListInvitesRequest(_) => {
			try2!(
				list_invites(connection_info, ds_context, &event),
				"list invites request error"
			)
		}
		EventBody::ModifyInviteRequest(_) => {
			try2!(
				modify_invite(connection_info, ds_context, &event),
				"modify invite error"
			)
		}
		EventBody::DeleteInviteRequest(_) => {
			try2!(
				delete_invite(connection_info, ds_context, &event),
				"delete invite request error"
			)
		}
		EventBody::ViewInviteRequest(_) => {
			try2!(
				view_invite(connection_info, ds_context, &event, conn_manager, config),
				"view invite request error"
			)
		}
		EventBody::AcceptInviteRequest(_) => {
			try2!(
				accept_invite(connection_info, ds_context, &event),
				"accept invite request error"
			)
		}
		EventBody::JoinServerRequest(_) => {
			try2!(
				join_server(connection_info, ds_context, &event, conn_manager, config),
				"join server request error"
			)
		}
		EventBody::GetProfileRequest(_) => {
			try2!(
				get_profile(connection_info, ds_context, &event, conn_manager, config,),
				"get profile request error"
			)
		}
		EventBody::SetProfileRequest(_) => {
			try2!(
				set_profile(connection_info, ds_context, &event, conn_manager, config),
				"set profile request error"
			)
		}
		_ => {
			warn!(
				"unexpected event type in event {:?}. Closing conn = {}",
				event, id
			);
			true
		}
	};

	Ok(res)
}

fn process_binary(
	handle: ConnData,
	conn_info: Arc<RwLock<HashMap<u128, ConnectionInfo>>>,
	ds_context: &DSContext,
	config: &ConcordConfig,
	conn_manager: Arc<RwLock<ConnManager>>,
) -> Result<(), Error> {
	let id = handle.get_connection_id();
	let event = bin_event!();

	info!("event on connection[{}]={:?}", id, event);

	match event.body {
		EventBody::AuthEvent(_) => {
			let close = try2!(
				ws_auth(handle.clone(), &event, ds_context, conn_info.clone()),
				"ws_auth error"
			);

			if close {
				let mut conn_info = nioruntime_util::lockw!(conn_info)?;
				close!(handle, conn_info);
			}
		}
		_ => {
			// for all other event types, we ensure that the user has
			// a pubkey (meaning they've authenticated)

			let close;
			{
				let conn_info = nioruntime_util::lockr!(conn_info)?;
				close = match conn_info.get(&id) {
					Some(connection_info) => {
						match &connection_info.pubkey {
							Some(pubkey) => {
								debug!("authed event on {}: {:?}, pubkey={:?}", id, event, pubkey);
								// we know the user is authed, now process events

								process_authed_event(
									&event,
									connection_info,
									ds_context,
									config,
									conn_manager,
									id,
								)?
							}
							None => true,
						}
					}
					None => true,
				};
			}

			if close {
				let mut conn_info = nioruntime_util::lockw!(conn_info)?;
				close!(handle, conn_info);
			}
		}
	}
	Ok(())
}

fn process_close(
	handle: ConnData,
	conn_info: Arc<RwLock<HashMap<u128, ConnectionInfo>>>,
) -> Result<(), Error> {
	let id = handle.get_connection_id();
	debug!("close : {},", id);
	let mut conn_info = nioruntime_util::lockw!(conn_info)?;
	conn_info.remove(&id);
	Ok(())
}

pub fn init_ws(cconfig: ConcordConfig) -> Result<(), ConcordError> {
	let conn_info = Arc::new(RwLock::new(HashMap::new()));
	let ds_context = DSContext::new(cconfig.root_dir.clone())?;
	let conn_manager = Arc::new(RwLock::new(ConnManager::new()));

	socklet!("ws", {
		let conn_info = conn_info.clone();
		let conn_manager = conn_manager.clone();
		let handle = handle!()?;
		match event!()? {
			Socklet::Open => {
				process_open(handle, conn_info)?;
			}
			Socklet::Binary => {
				process_binary(handle, conn_info, &ds_context, &cconfig, conn_manager)?;
			}
			Socklet::Close => {
				process_close(handle, conn_info)?;
			}
			_ => {
				warn!(
					"unexpected request type on cid={}",
					handle.get_connection_id()
				);
			}
		}
	});

	socklet_mapping!("/ws", "ws");
	Ok(())
}
