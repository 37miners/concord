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

use crate::types::{
	AddChannelResponse, DeleteChannelResponse, GetChannelsResponse, ModifyChannelResponse,
};
use crate::types::{Channel, ConnectionInfo, Event, EventBody};
use crate::{member, send};
use concorddata::concord::DSContext;
use concorddata::types::{Pubkey, ServerId};
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;

info!(); // set log level to debug

#[derive(Serialize, Deserialize)]
struct ChannelInfo {
	name: String,
	description: String,
	id: String,
}

pub fn add_channel(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	let request_id = event.request_id;
	let (server_id, server_pubkey, name, description) = match &event.body {
		EventBody::AddChannelRequest(event) => (
			event.server_id.to_bytes(),
			event.server_pubkey.to_bytes(),
			event.name.to_string(),
			event.description.to_string(),
		),
		_ => {
			warn!("Malformed add channel event. No event present: {:?}", event);
			return Ok(true);
		}
	};

	let channel_id = ds_context.add_channel(server_id, server_pubkey, name, description)?;

	let event = Event {
		request_id,
		body: EventBody::AddChannelResponse(AddChannelResponse {
			channel_id,
			success: true,
		})
		.into(),
		..Default::default()
	};

	send!(conn_info.handle, event);

	Ok(false)
}

pub fn modify_channel(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	let request_id = event.request_id;
	let (server_id, server_pubkey, name, description, channel_id) = match &event.body {
		EventBody::ModifyChannelRequest(event) => (
			event.server_id.to_bytes(),
			event.server_pubkey.to_bytes(),
			event.name.to_string(),
			event.description.to_string(),
			event.channel_id,
		),
		_ => {
			warn!(
				"Malformed modify channel event. No event present: {:?}",
				event
			);
			return Ok(true);
		}
	};

	ds_context.modify_channel(server_id, server_pubkey, channel_id, name, description)?;

	let event = Event {
		request_id,
		body: EventBody::ModifyChannelResponse(ModifyChannelResponse { success: true }).into(),
		..Default::default()
	};

	send!(conn_info.handle, event);

	Ok(false)
}

pub fn delete_channel(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	let request_id = event.request_id;
	let (server_id, server_pubkey, channel_id) = match &event.body {
		EventBody::DeleteChannelRequest(event) => (
			event.server_id.to_bytes(),
			event.server_pubkey.to_bytes(),
			event.channel_id,
		),
		_ => {
			warn!(
				"Malformed delete channel event. No event present: {:?}",
				event
			);
			return Ok(true);
		}
	};

	ds_context.delete_channel(server_id, server_pubkey, channel_id)?;

	let event = Event {
		request_id,
		body: EventBody::DeleteChannelResponse(DeleteChannelResponse { success: true }).into(),
		..Default::default()
	};

	send!(conn_info.handle, event);

	Ok(false)
}

pub fn get_channels(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	member!(conn_info, ds_context);

	let request_id = event.request_id;
	let (server_id, server_pubkey) = match &event.body {
		EventBody::GetChannelsRequest(event) => {
			(event.server_id.to_bytes(), event.server_pubkey.to_bytes())
		}
		_ => {
			warn!("Malformed get channel event. No event present: {:?}", event);
			return Ok(true);
		}
	};

	let channels = ds_context.get_channels(server_pubkey, server_id)?;
	let mut channels_event = vec![];
	for channel in channels {
		channels_event.push(Channel {
			name: channel.name.into(),
			description: channel.description.into(),
			channel_id: channel.channel_id,
		});
	}

	let event = Event {
		request_id,
		body: EventBody::GetChannelsResponse(GetChannelsResponse {
			channels: channels_event,
			server_id: ServerId::from_bytes(server_id),
			server_pubkey: Pubkey::from_bytes(server_pubkey),
		})
		.into(),
		..Default::default()
	};

	send!(conn_info.handle, event);

	Ok(false)
}
