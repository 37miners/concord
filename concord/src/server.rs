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

use crate::types::EventBody;
use concordconfig::ConcordConfig;
use concorddata::concord::DSContext;
use concorddata::concord::ServerInfo as DataServerInfo;
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use std::fs::File;
use std::io::Read;
use std::io::Write;

use crate::types::{ConnectionInfo, Event, GetServersResponse, ServerInfo};
use crate::{owner, send};
use concorddata::types::{Pubkey, ServerId};

info!();

#[derive(Serialize, Deserialize)]
struct ServerInfoMin {
	name: String,
	server_pubkey: String,
	id: String,
}

fn _get_icon(
	server_id: [u8; 8],
	pubkey: [u8; 32],
	root_dir: String,
) -> Result<Vec<u8>, ConcordError> {
	let server_id = ServerId::from_bytes(server_id).to_base58()?;
	let pubkey = Pubkey::from_bytes(pubkey).to_base58()?;
	let file_name = format!(
		"{}/www/images/user_images/servers-{}-{}",
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

fn set_icon(
	root_dir: String,
	server_id: [u8; 8],
	pubkey: [u8; 32],
	icon: Vec<u8>,
) -> Result<(), ConcordError> {
	let server_id = ServerId::from_bytes(server_id).to_base58()?;
	let pubkey = Pubkey::from_bytes(pubkey).to_base58()?;
	let file_name = format!(
		"{}/www/images/user_images/servers-{}-{}",
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

pub fn get_servers(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
) -> Result<bool, ConcordError> {
	owner!(conn_info);

	let mut servers = vec![];
	let data = ds_context.get_servers()?;

	let now = std::time::Instant::now();
	for d in data {
		servers.push(ServerInfo {
			name: d.name.into(),
			description: "none".into(),
			//icon: get_icon(d.server_id, d.pubkey)?.into(),
			server_id: d.server_id.into(),
			server_pubkey: Pubkey::from_bytes(d.pubkey),
			seqno: d.seqno,
		});
	}
	error!(
		"end of adding ServerInfo, time = {}",
		now.elapsed().as_nanos()
	);

	let event = Event {
		body: EventBody::GetServersResponse(GetServersResponse { servers }).into(),
		..Default::default()
	};
	error!("end of building event, time = {}", now.elapsed().as_nanos());
	send!(conn_info.handle, event);
	error!("end of sending event, time = {}", now.elapsed().as_nanos());

	Ok(false)
}

pub fn create_server(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
	config: &ConcordConfig,
) -> Result<bool, ConcordError> {
	owner!(conn_info);

	let (icon, name) = match &event.body {
		EventBody::CreateServerEvent(body) => (body.icon.clone(), body.name.data.clone()),
		_ => {
			warn!("Unexpected EventBody in create server: {:?}", event);
			return Ok(true);
		}
	};

	let pubkey = pubkey!();
	let data_server_info = DataServerInfo {
		pubkey,
		name,
		joined: true,
		seqno: 1,
	};

	let server_id = ds_context.add_server(data_server_info, None, None, false)?;

	set_icon(config.root_dir.clone(), server_id, pubkey, icon)?;

	info!("create server complete");

	Ok(false)
}

pub fn delete_server(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
) -> Result<bool, ConcordError> {
	owner!(conn_info);

	let (server_id, server_pubkey) = match &event.body {
		EventBody::DeleteServerEvent(event) => {
			(event.server_id.to_bytes(), event.server_pubkey.to_bytes())
		}
		_ => {
			warn!(
				"Malformed delete server event. No server_id/server_pubkey: {:?}",
				event
			);
			return Ok(true);
		}
	};

	ds_context.delete_server(server_id, server_pubkey)?;

	Ok(false)
}

pub fn modify_server(
	conn_info: &ConnectionInfo,
	ds_context: &DSContext,
	event: &Event,
	config: &ConcordConfig,
) -> Result<bool, ConcordError> {
	owner!(conn_info);

	let (server_id, server_pubkey, name, icon) = match &event.body {
		EventBody::ModifyServerEvent(event) => (
			event.server_id.to_bytes(),
			event.server_pubkey.to_bytes(),
			event.name.clone(),
			event.icon.clone(),
		),
		_ => {
			warn!(
				"Malformed modify server event. No server_id/server_pubkey: {:?}",
				event
			);
			return Ok(true);
		}
	};

	match icon.0 {
		Some(icon) => {
			set_icon(config.root_dir.clone(), server_id, server_pubkey, icon.data)?;
		}
		None => {}
	}

	match name.0 {
		Some(name) => ds_context.modify_server(server_id, server_pubkey, name.to_string())?,
		None => {}
	}

	Ok(false)
}
