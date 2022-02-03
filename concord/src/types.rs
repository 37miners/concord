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

use crate::librustlet::nioruntime_tor::ov3::OnionV3Address;
use crate::librustlet::ConnData;
use concorddata::ser::{Readable, Reader, Writeable, Writer};
use concorderror::{Error, ErrorKind};
use concordutil::nioruntime_log;
use nioruntime_log::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::convert::TryFrom;
use std::convert::TryInto;

info!();

const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug)]
pub struct U128(pub u128);

impl Writeable for U128 {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.0)?;
		Ok(())
	}
}

impl Readable for U128 {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		Ok(Self(reader.read_u128()?))
	}
}

#[derive(Debug)]
pub struct Signature(pub [u8; 64]);

impl Writeable for Signature {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..64 {
			writer.write_u8(self.0[i])?;
		}
		Ok(())
	}
}

impl Readable for Signature {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut signature = [0u8; 64];
		for i in 0..64 {
			signature[i] = reader.read_u8()?;
		}

		Ok(Self(signature))
	}
}

#[derive(Debug, Clone)]
pub struct SerOption<T>(pub Option<T>);

impl<T: Writeable> Writeable for SerOption<T> {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match &self.0 {
			Some(writeable) => {
				debug!("ser option is some");
				writer.write_u8(1)?;
				Writeable::write(&writeable, writer)
			}
			None => {
				debug!("ser option is none");
				writer.write_u8(0)
			}
		}
	}
}

impl<T: Readable> Readable for SerOption<T> {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		Ok(match reader.read_u8()? {
			0 => Self(None),
			_ => Self(Some(Readable::read(reader)?)),
		})
	}
}

impl<T> From<Option<T>> for SerOption<T> {
	fn from(opt: Option<T>) -> SerOption<T> {
		SerOption(opt)
	}
}

#[derive(Debug)]
pub struct ServerId {
	data: [u8; 8],
}

impl ServerId {
	pub fn _from_bytes(data: [u8; 8]) -> Self {
		Self { data }
	}

	pub fn to_bytes(&self) -> [u8; 8] {
		self.data
	}

	pub fn from_urlencoding(data: String) -> Result<Self, Error> {
		let data = urlencoding::decode(&data)?.to_string();
		let data = base64::decode(data)?;
		let data = data.as_slice().try_into()?;
		Ok(Self { data })
	}

	pub fn to_urlencoding(&self) -> Result<String, Error> {
		let data = base64::encode(self.data);
		let data = urlencoding::encode(&data).to_string();
		Ok(data)
	}
}

impl Writeable for ServerId {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..8 {
			writer.write_u8(self.data[i])?;
		}
		Ok(())
	}
}

impl Readable for ServerId {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut data = [0u8; 8];

		for i in 0..8 {
			data[i] = reader.read_u8()?;
		}

		Ok(Self { data })
	}
}

impl From<u64> for ServerId {
	fn from(data: u64) -> Self {
		let data = data.to_be_bytes();
		Self { data }
	}
}

impl From<[u8; 8]> for ServerId {
	fn from(data: [u8; 8]) -> Self {
		Self { data }
	}
}

#[derive(Debug, Clone)]
pub struct Pubkey {
	data: [u8; 32],
}

impl Pubkey {
	pub fn from_bytes(data: [u8; 32]) -> Self {
		Pubkey { data }
	}

	pub fn to_bytes(&self) -> [u8; 32] {
		self.data
	}

	pub fn from_urlencoding(data: String) -> Result<Self, Error> {
		let data = urlencoding::decode(&data)?.to_string();
		let data = base64::decode(data)?;
		let data = data.as_slice().try_into()?;
		Ok(Self { data })
	}

	pub fn to_urlencoding(&self) -> Result<String, Error> {
		let data = base64::encode(self.data);
		let data = urlencoding::encode(&data).to_string();
		Ok(data)
	}

	pub fn _from_onion(onion_address: &str) -> Result<Self, Error> {
		let onion_address: OnionV3Address = onion_address.try_into()?;
		Ok(Self {
			data: *onion_address.as_bytes(),
		})
	}

	pub fn to_onion(&self) -> Result<String, Error> {
		Ok(OnionV3Address::from_bytes(self.data).to_string())
	}
}

impl Writeable for Pubkey {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..32 {
			writer.write_u8(self.data[i])?;
		}
		Ok(())
	}
}

impl Readable for Pubkey {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut data = [0u8; 32];

		for i in 0..32 {
			data[i] = reader.read_u8()?;
		}

		Ok(Self { data })
	}
}

pub struct ConnectionInfo {
	pub handle: ConnData,
	pub pubkey: Option<Pubkey>,
}

#[derive(Debug)]
pub struct DeleteServerEvent {
	pub server_pubkey: Pubkey,
	pub server_id: ServerId,
}

impl Writeable for DeleteServerEvent {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;
		Ok(())
	}
}

impl Readable for DeleteServerEvent {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;

		Ok(Self {
			server_id,
			server_pubkey,
		})
	}
}

#[derive(Debug)]
pub struct ModifyServerEvent {
	pub name: SerOption<SerString>,
	pub icon: SerOption<Icon>,
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
}

impl Writeable for ModifyServerEvent {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.name, writer)?;
		Writeable::write(&self.icon, writer)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Ok(())
	}
}

impl Readable for ModifyServerEvent {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let name = SerOption::read(reader)?;
		let icon = SerOption::read(reader)?;
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;

		Ok(Self {
			server_id,
			name,
			icon,
			server_pubkey,
		})
	}
}

#[derive(Debug)]
pub struct CreateServerEvent {
	pub name: SerString,
	pub icon: Vec<u8>,
}

impl Writeable for CreateServerEvent {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.name, writer)?;
		let len = self.icon.len();
		writer.write_u64(len.try_into()?)?;
		for i in 0..len {
			writer.write_u8(self.icon[i])?;
		}
		Ok(())
	}
}

impl Readable for CreateServerEvent {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut icon = vec![];
		let name = SerString::read(reader)?;
		let len: u64 = reader.read_u64()?.try_into()?;
		for _ in 0..len {
			icon.push(reader.read_u8()?);
		}

		Ok(Self { name, icon })
	}
}

#[derive(Debug)]
pub struct GetServersEvent {}

impl Writeable for GetServersEvent {
	fn write<W: Writer>(&self, _: &mut W) -> Result<(), Error> {
		Ok(())
	}
}

impl Readable for GetServersEvent {
	fn read<R: Reader>(_: &mut R) -> Result<Self, Error> {
		Ok(Self {})
	}
}

#[derive(Debug)]
pub struct ChallengeEvent {
	pub challenge: u128,
}

impl Writeable for ChallengeEvent {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.challenge)?;
		Ok(())
	}
}

impl Readable for ChallengeEvent {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let challenge = reader.read_u128()?;
		Ok(Self { challenge })
	}
}

// There are two methods to authenticate.
// 1.) Use auth token (u128) provided on startup of concord server.
// 2.) Sign a ChallengeEvent with your pubkey.
#[derive(Debug)]
pub struct AuthEvent {
	pub signature: SerOption<Signature>,
	pub token: SerOption<U128>,
	pub pubkey: SerOption<Pubkey>,
}

impl Writeable for AuthEvent {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.signature, writer)?;
		Writeable::write(&self.token, writer)?;
		Writeable::write(&self.pubkey, writer)?;
		Ok(())
	}
}

impl Readable for AuthEvent {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let signature = SerOption::read(reader)?;
		let token = SerOption::read(reader)?;
		let pubkey = SerOption::read(reader)?;

		Ok(Self {
			signature,
			token,
			pubkey,
		})
	}
}

#[derive(Debug, Clone)]
pub struct SerString {
	pub data: String,
}

impl From<String> for SerString {
	fn from(data: String) -> Self {
		Self { data }
	}
}

impl From<&str> for SerString {
	fn from(data: &str) -> Self {
		let data = data.to_string();
		Self { data }
	}
}

impl Writeable for SerString {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let len = self.data.len();
		let bytes = self.data.as_bytes();
		writer.write_u64(len.try_into()?)?;
		for i in 0..len {
			writer.write_u8(bytes[i])?;
		}
		Ok(())
	}
}

impl Readable for SerString {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let len = reader.read_u64()?;
		let mut byte_vec = vec![];
		for _ in 0..len {
			byte_vec.push(reader.read_u8()?);
		}

		Ok(Self {
			data: std::str::from_utf8(&byte_vec)?.to_string(),
		})
	}
}

#[derive(Debug)]
pub struct AuthResponse {
	pub success: bool,
	pub redirect: SerOption<SerString>,
}

impl Writeable for AuthResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		debug!("writing self.redirect");
		Writeable::write(&self.redirect, writer)?;
		Ok(())
	}
}

impl Readable for AuthResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		let redirect = SerOption::read(reader)?;

		Ok(Self { success, redirect })
	}
}

#[derive(Debug, Clone)]
pub struct Icon {
	pub data: Vec<u8>,
}

impl Writeable for Icon {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let len = self.data.len();
		writer.write_u64(len.try_into()?)?;
		for i in 0..len {
			writer.write_u8(self.data[i])?;
		}
		Ok(())
	}
}

impl Readable for Icon {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let len = reader.read_u64()?;
		let mut data = vec![];
		for _ in 0..len {
			data.push(reader.read_u8()?);
		}

		Ok(Self { data })
	}
}

impl From<Vec<u8>> for Icon {
	fn from(data: Vec<u8>) -> Self {
		Self { data }
	}
}

#[derive(Debug)]
pub struct ServerInfo {
	pub name: SerString,
	pub description: SerString,
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
	pub icon: Icon,
	pub seqno: u64,
}

impl Writeable for ServerInfo {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.name, writer)?;
		Writeable::write(&self.description, writer)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.icon, writer)?;
		writer.write_u64(self.seqno)?;
		Ok(())
	}
}

impl Readable for ServerInfo {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let name = SerString::read(reader)?;
		let description = SerString::read(reader)?;
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let icon = Icon::read(reader)?;
		let seqno = reader.read_u64()?;
		Ok(Self {
			name,
			description,
			server_id,
			server_pubkey,
			icon,
			seqno,
		})
	}
}

#[derive(Debug)]
pub struct GetServersResponse {
	pub servers: Vec<ServerInfo>,
}

impl Writeable for GetServersResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u64(self.servers.len().try_into().unwrap_or(0))?;
		for server in &self.servers {
			Writeable::write(server, writer)?;
		}
		Ok(())
	}
}

impl Readable for GetServersResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut servers = vec![];
		let len = reader.read_u64()?;
		for _ in 0..len {
			servers.push(ServerInfo::read(reader)?);
		}
		Ok(Self { servers })
	}
}

#[derive(Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive, Clone)]
#[repr(u8)]
pub enum EventType {
	AuthEvent,
	ChallengeEvent,
	AuthResponse,
	GetServersEvent,
	GetServersResponse,
	CreateServerEvent,
	DeleteServerEvent,
	ModifyServerEvent,
}

#[derive(Debug)]
pub struct Event {
	pub event_type: EventType,
	pub auth_event: SerOption<AuthEvent>,
	pub challenge_event: SerOption<ChallengeEvent>,
	pub auth_response: SerOption<AuthResponse>,
	pub get_servers_event: SerOption<GetServersEvent>,
	pub get_servers_response: SerOption<GetServersResponse>,
	pub create_server_event: SerOption<CreateServerEvent>,
	pub delete_server_event: SerOption<DeleteServerEvent>,
	pub modify_server_event: SerOption<ModifyServerEvent>,
	pub version: u8,
	pub timestamp: u128,
}

impl Default for Event {
	fn default() -> Self {
		Self {
			event_type: EventType::ChallengeEvent,
			auth_event: None.into(),
			auth_response: None.into(),
			challenge_event: None.into(),
			get_servers_event: None.into(),
			get_servers_response: None.into(),
			create_server_event: None.into(),
			delete_server_event: None.into(),
			modify_server_event: None.into(),
			version: PROTOCOL_VERSION,
			timestamp: std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap_or(std::time::Duration::from_millis(0))
				.as_millis(),
		}
	}
}

impl Writeable for Event {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(self.version)?;
		writer.write_u128(self.timestamp)?;
		writer.write_u8(self.event_type.clone().into())?;
		match &self.event_type {
			EventType::AuthEvent => Writeable::write(&self.auth_event, writer),
			EventType::ChallengeEvent => Writeable::write(&self.challenge_event, writer),
			EventType::AuthResponse => Writeable::write(&self.auth_response, writer),
			EventType::GetServersEvent => Writeable::write(&self.get_servers_event, writer),
			EventType::GetServersResponse => Writeable::write(&self.get_servers_response, writer),
			EventType::CreateServerEvent => Writeable::write(&self.create_server_event, writer),
			EventType::DeleteServerEvent => Writeable::write(&self.delete_server_event, writer),
			EventType::ModifyServerEvent => Writeable::write(&self.modify_server_event, writer),
		}
	}
}

impl Readable for Event {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut auth_event = None.into();
		let mut challenge_event = None.into();
		let mut auth_response = None.into();
		let mut get_servers_event = None.into();
		let mut get_servers_response = None.into();
		let mut create_server_event = None.into();
		let mut delete_server_event = None.into();
		let mut modify_server_event = None.into();

		let version = reader.read_u8()?;
		let timestamp = reader.read_u128()?;

		let event_type: EventType = EventType::try_from(reader.read_u8()?).map_err(|e| {
			let error: Error =
				ErrorKind::SerializationError(format!("invalid event, unkown event type: {}", e))
					.into();
			error
		})?;

		match event_type {
			EventType::AuthEvent => auth_event = SerOption::read(reader)?,
			EventType::ChallengeEvent => challenge_event = SerOption::read(reader)?,
			EventType::AuthResponse => auth_response = SerOption::read(reader)?,
			EventType::GetServersEvent => get_servers_event = SerOption::read(reader)?,
			EventType::GetServersResponse => get_servers_response = SerOption::read(reader)?,
			EventType::CreateServerEvent => create_server_event = SerOption::read(reader)?,
			EventType::DeleteServerEvent => delete_server_event = SerOption::read(reader)?,
			EventType::ModifyServerEvent => modify_server_event = SerOption::read(reader)?,
		};

		Ok(Self {
			version,
			event_type,
			challenge_event,
			auth_event,
			auth_response,
			get_servers_event,
			get_servers_response,
			create_server_event,
			delete_server_event,
			modify_server_event,
			timestamp,
		})
	}
}
