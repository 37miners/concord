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

use crate::librustlet::ConnData;
use concorddata::ser::{chunk_read, chunk_write, Readable, Reader, Writeable, Writer};
use concorddata::types::{Invite, Pubkey, SerOption, SerString, ServerId, Signature, U128};
use concorderror::{Error, ErrorKind};
use concordutil::nioruntime_log;
use nioruntime_log::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::convert::TryFrom;
use std::convert::TryInto;

info!();

const PROTOCOL_VERSION: u8 = 1;

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
	pub icon: SerOption<Image>,
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
pub struct Channel {
	pub name: SerString,
	pub description: SerString,
	pub channel_id: u64,
}

impl Writeable for Channel {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u64(self.channel_id)?;
		Writeable::write(&self.name, writer)?;
		Writeable::write(&self.description, writer)?;
		Ok(())
	}
}

impl Readable for Channel {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let channel_id = reader.read_u64()?;
		let name = SerString::read(reader)?;
		let description = SerString::read(reader)?;
		Ok(Self {
			channel_id,
			name,
			description,
		})
	}
}

#[derive(Debug)]
pub struct AddChannelRequest {
	pub request_id: u128,
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
	pub name: SerString,
	pub description: SerString,
}

impl Writeable for AddChannelRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.name, writer)?;
		Writeable::write(&self.description, writer)?;
		Ok(())
	}
}

impl Readable for AddChannelRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let name = SerString::read(reader)?;
		let description = SerString::read(reader)?;
		Ok(Self {
			request_id,
			server_id,
			server_pubkey,
			name,
			description,
		})
	}
}

#[derive(Debug)]
pub struct AddChannelResponse {
	pub request_id: u128,
	pub channel_id: u64,
	pub success: bool,
}

impl Writeable for AddChannelResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		writer.write_u64(self.channel_id)?;
		Ok(())
	}
}

impl Readable for AddChannelResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		let channel_id = reader.read_u64()?;
		Ok(Self {
			request_id,
			success,
			channel_id,
		})
	}
}

#[derive(Debug)]
pub struct ModifyChannelRequest {
	pub request_id: u128,
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
	pub channel_id: u64,
	pub name: SerString,
	pub description: SerString,
}

impl Writeable for ModifyChannelRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		writer.write_u64(self.channel_id)?;
		Writeable::write(&self.name, writer)?;
		Writeable::write(&self.description, writer)?;
		Ok(())
	}
}

impl Readable for ModifyChannelRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let channel_id = reader.read_u64()?;
		let name = SerString::read(reader)?;
		let description = SerString::read(reader)?;
		Ok(Self {
			request_id,
			channel_id,
			server_id,
			server_pubkey,
			name,
			description,
		})
	}
}

#[derive(Debug)]
pub struct ModifyChannelResponse {
	pub request_id: u128,
	pub success: bool,
}

impl Writeable for ModifyChannelResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for ModifyChannelResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self {
			request_id,
			success,
		})
	}
}

#[derive(Debug)]
pub struct DeleteChannelRequest {
	pub request_id: u128,
	pub channel_id: u64,
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
}

impl Writeable for DeleteChannelRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		writer.write_u64(self.channel_id)?;
		Ok(())
	}
}

impl Readable for DeleteChannelRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let channel_id = reader.read_u64()?;
		Ok(Self {
			request_id,
			channel_id,
			server_id,
			server_pubkey,
		})
	}
}

#[derive(Debug)]
pub struct DeleteChannelResponse {
	pub request_id: u128,
	pub success: bool,
}

impl Writeable for DeleteChannelResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for DeleteChannelResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self {
			request_id,
			success,
		})
	}
}

#[derive(Debug)]
pub struct GetChannelsResponse {
	pub channels: Vec<Channel>,
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
}

impl Writeable for GetChannelsResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		writer.write_u64(self.channels.len().try_into()?)?;
		for channel in &self.channels {
			Writeable::write(&channel, writer)?;
		}
		Ok(())
	}
}

impl Readable for GetChannelsResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let len = reader.read_u64()?;
		let mut channels = vec![];
		for _ in 0..len {
			channels.push(Channel::read(reader)?);
		}

		Ok(Self {
			channels,
			server_id,
			server_pubkey,
		})
	}
}
#[derive(Debug)]
pub struct GetChannelsRequest {
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
}

impl Writeable for GetChannelsRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Ok(())
	}
}

impl Readable for GetChannelsRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;

		Ok(Self {
			server_id,
			server_pubkey,
		})
	}
}

#[derive(Debug)]
pub struct CreateInviteRequest {
	pub request_id: u128,
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
	pub count: u64,
	pub expiration: u128,
}

impl Writeable for CreateInviteRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		writer.write_u64(self.count)?;
		writer.write_u128(self.expiration)?;
		Ok(())
	}
}

impl Readable for CreateInviteRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let count = reader.read_u64()?;
		let expiration = reader.read_u128()?;
		Ok(Self {
			request_id,
			server_id,
			server_pubkey,
			count,
			expiration,
		})
	}
}

#[derive(Debug)]
pub struct CreateInviteResponse {
	pub request_id: u128,
	pub success: bool,
	pub invite_id: u128,
}

impl Writeable for CreateInviteResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}

		writer.write_u128(self.invite_id)?;

		Ok(())
	}
}

impl Readable for CreateInviteResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		let invite_id = reader.read_u128()?;

		Ok(Self {
			request_id,
			success,
			invite_id,
		})
	}
}

#[derive(Debug)]
pub struct ListInvitesRequest {
	pub request_id: u128,
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
}

impl Writeable for ListInvitesRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Ok(())
	}
}

impl Readable for ListInvitesRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		Ok(Self {
			request_id,
			server_id,
			server_pubkey,
		})
	}
}

#[derive(Debug)]
pub struct ListInvitesResponse {
	pub request_id: u128,
	pub invites: Vec<Invite>,
}

impl Writeable for ListInvitesResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		writer.write_u64(self.invites.len().try_into()?)?;
		for invite in &self.invites {
			Writeable::write(invite, writer)?;
		}
		Ok(())
	}
}

impl Readable for ListInvitesResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let len = reader.read_u64()?;
		let mut invites = vec![];
		for _ in 0..len {
			invites.push(Invite::read(reader)?);
		}

		Ok(Self {
			request_id,
			invites,
		})
	}
}

#[derive(Debug)]
pub struct ModifyInviteRequest {
	pub request_id: u128,
	pub invite_id: u128,
	pub max: u64,
	pub expiration: u128,
}

impl Writeable for ModifyInviteRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		writer.write_u128(self.invite_id)?;
		writer.write_u64(self.max)?;
		writer.write_u128(self.expiration)?;
		Ok(())
	}
}

impl Readable for ModifyInviteRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let invite_id = reader.read_u128()?;
		let max = reader.read_u64()?;
		let expiration = reader.read_u128()?;

		Ok(Self {
			request_id,
			invite_id,
			max,
			expiration,
		})
	}
}

#[derive(Debug)]
pub struct ModifyInviteResponse {
	pub request_id: u128,
	pub success: bool,
}

impl Writeable for ModifyInviteResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for ModifyInviteResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self {
			request_id,
			success,
		})
	}
}

#[derive(Debug)]
pub struct DeleteInviteRequest {
	pub request_id: u128,
	pub invite_id: u128,
}

impl Writeable for DeleteInviteRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		writer.write_u128(self.invite_id)?;
		Ok(())
	}
}

impl Readable for DeleteInviteRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let invite_id = reader.read_u128()?;

		Ok(Self {
			request_id,
			invite_id,
		})
	}
}

#[derive(Debug)]
pub struct DeleteInviteResponse {
	pub request_id: u128,
	pub success: bool,
}

impl Writeable for DeleteInviteResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for DeleteInviteResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self {
			request_id,
			success,
		})
	}
}

#[derive(Debug)]
pub struct ViewInviteRequest {
	pub request_id: u128,
	pub invite_url: SerString,
}

impl Writeable for ViewInviteRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		Writeable::write(&self.invite_url, writer)?;
		Ok(())
	}
}

impl Readable for ViewInviteRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let invite_url = SerString::read(reader)?;

		Ok(Self {
			request_id,
			invite_url,
		})
	}
}

#[derive(Debug)]
pub struct ViewInviteResponse {
	pub request_id: u128,
	pub inviter_name: SerString,
	pub inviter_icon: Image,
	pub server_icon: Image,
	pub current_members: u64,
	pub online_members: u64,
}

impl Writeable for ViewInviteResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		Writeable::write(&self.inviter_name, writer)?;
		Writeable::write(&self.inviter_icon, writer)?;
		Writeable::write(&self.server_icon, writer)?;
		writer.write_u64(self.current_members)?;
		writer.write_u64(self.online_members)?;
		Ok(())
	}
}

impl Readable for ViewInviteResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let inviter_name = SerString::read(reader)?;
		let inviter_icon = Image::read(reader)?;
		let server_icon = Image::read(reader)?;
		let current_members = reader.read_u64()?;
		let online_members = reader.read_u64()?;
		Ok(Self {
			request_id,
			inviter_name,
			inviter_icon,
			server_icon,
			current_members,
			online_members,
		})
	}
}

#[derive(Debug)]
pub struct AcceptInviteRequest {
	pub request_id: u128,
	pub invite_id: u128,
}

impl Writeable for AcceptInviteRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		writer.write_u128(self.invite_id)?;
		Ok(())
	}
}

impl Readable for AcceptInviteRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let invite_id = reader.read_u128()?;

		Ok(Self {
			request_id,
			invite_id,
		})
	}
}

#[derive(Debug)]
pub struct AcceptInviteResponse {
	pub request_id: u128,
	pub success: bool,
}

impl Writeable for AcceptInviteResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.request_id)?;
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for AcceptInviteResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let request_id = reader.read_u128()?;
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self {
			request_id,
			success,
		})
	}
}

// for now just two
#[derive(Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive, Clone)]
#[repr(u8)]
pub enum OnlineStatus {
	Offline,
	Online,
}

#[derive(Debug)]
pub struct Member {
	user_pubkey: Pubkey,
	user_name: SerString,
	user_bio: SerString,
	roles: u128,
	profile_seqno: u64,
	online_status: OnlineStatus,
}

impl From<concorddata::concord::Member> for Member {
	fn from(dmember: concorddata::concord::Member) -> Member {
		let user_pubkey = dmember.user_pubkey;
		let (user_name, user_bio) = match dmember.profile {
			Some(profile) => (
				profile.profile_data.user_name,
				profile.profile_data.user_bio,
			),
			None => ("".to_string(), "".to_string()),
		};
		let roles = 0;
		let profile_seqno = 0;
		let online_status = OnlineStatus::Offline;
		let user_name = user_name.into();
		let user_bio = user_bio.into();
		let user_pubkey = Pubkey::from_bytes(user_pubkey);

		Member {
			user_pubkey,
			user_name,
			user_bio,
			roles,
			profile_seqno,
			online_status,
		}
	}
}

impl Writeable for Member {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.user_pubkey, writer)?;
		Writeable::write(&self.user_name, writer)?;
		Writeable::write(&self.user_bio, writer)?;
		writer.write_u128(self.roles)?;
		writer.write_u64(self.profile_seqno)?;
		writer.write_u8(self.online_status.clone().into())?;
		Ok(())
	}
}

impl Readable for Member {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let user_pubkey = Pubkey::read(reader)?;
		let user_name = SerString::read(reader)?;
		let user_bio = SerString::read(reader)?;
		let roles = reader.read_u128()?;
		let profile_seqno = reader.read_u64()?;

		let online_status: OnlineStatus =
			OnlineStatus::try_from(reader.read_u8()?).map_err(|e| {
				let error: Error = ErrorKind::SerializationError(format!(
					"invalid online_status, unkown online_status type: {}",
					e
				))
				.into();
				error
			})?;
		Ok(Self {
			user_pubkey,
			user_name,
			user_bio,
			roles,
			profile_seqno,
			online_status,
		})
	}
}

#[derive(Debug)]
pub struct GetMembersRequest {
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
	pub batch_num: u64,
}

impl Writeable for GetMembersRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		writer.write_u64(self.batch_num)?;
		Ok(())
	}
}

impl Readable for GetMembersRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let batch_num = reader.read_u64()?;

		Ok(Self {
			server_id,
			server_pubkey,
			batch_num,
		})
	}
}

#[derive(Debug)]
pub struct GetMembersResponse {
	pub members: Vec<Member>,
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
	pub batch_num: u64,
}

impl Writeable for GetMembersResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let members_len = self.members.len();
		writer.write_u64(members_len.try_into()?)?;
		for _ in 0..members_len {
			Writeable::write(&self.members, writer)?;
		}
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		writer.write_u64(self.batch_num)?;
		Ok(())
	}
}

impl Readable for GetMembersResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut members = vec![];
		let members_len = reader.read_u64()?;
		for _ in 0..members_len {
			members.push(Member::read(reader)?);
		}
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let batch_num = reader.read_u64()?;

		Ok(Self {
			members,
			server_id,
			server_pubkey,
			batch_num,
		})
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
pub struct Image {
	pub data: Vec<u8>,
}

impl Writeable for Image {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let len = self.data.len();
		writer.write_u64(len.try_into()?)?;
		chunk_write(writer, &self.data)?;

		Ok(())
	}
}

impl Readable for Image {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let len = reader.read_u64()?;
		let data = chunk_read(reader, len.try_into()?)?;

		Ok(Self { data })
	}
}

impl From<Vec<u8>> for Image {
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
	pub seqno: u64,
}

impl Writeable for ServerInfo {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.name, writer)?;
		Writeable::write(&self.description, writer)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
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
		let seqno = reader.read_u64()?;
		Ok(Self {
			name,
			description,
			server_id,
			server_pubkey,
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
#[repr(u16)]
pub enum EventType {
	AuthEvent,
	ChallengeEvent,
	AuthResponse,
	GetServersEvent,
	GetServersResponse,
	CreateServerEvent,
	DeleteServerEvent,
	ModifyServerEvent,
	GetChannelsRequest,
	GetChannelsResponse,
	AddChannelRequest,
	DeleteChannelRequest,
	ModifyChannelRequest,
	AddChannelResponse,
	DeleteChannelResponse,
	ModifyChannelResponse,
	GetMembersRequest,
	GetMembersResponse,
	CreateInviteRequest,
	CreateInviteResponse,
	ListInvitesRequest,
	ListInvitesResponse,
	ModifyInviteRequest,
	ModifyInviteResponse,
	DeleteInviteRequest,
	DeleteInviteResponse,
	ViewInviteRequest,
	ViewInviteResponse,
	AcceptInviteRequest,
	AcceptInviteResponse,
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
	pub get_channels_request: SerOption<GetChannelsRequest>,
	pub get_channels_response: SerOption<GetChannelsResponse>,
	pub delete_channel_request: SerOption<DeleteChannelRequest>,
	pub delete_channel_response: SerOption<DeleteChannelResponse>,
	pub modify_channel_request: SerOption<ModifyChannelRequest>,
	pub modify_channel_response: SerOption<ModifyChannelResponse>,
	pub add_channel_request: SerOption<AddChannelRequest>,
	pub add_channel_response: SerOption<AddChannelResponse>,
	pub get_members_request: SerOption<GetMembersRequest>,
	pub get_members_response: SerOption<GetMembersResponse>,
	pub create_invite_request: SerOption<CreateInviteRequest>,
	pub create_invite_response: SerOption<CreateInviteResponse>,
	pub list_invites_request: SerOption<ListInvitesRequest>,
	pub list_invites_response: SerOption<ListInvitesResponse>,
	pub modify_invite_request: SerOption<ModifyInviteRequest>,
	pub modify_invite_response: SerOption<ModifyInviteResponse>,
	pub delete_invite_request: SerOption<DeleteInviteRequest>,
	pub delete_invite_response: SerOption<DeleteInviteResponse>,
	pub view_invite_request: SerOption<ViewInviteRequest>,
	pub view_invite_response: SerOption<ViewInviteResponse>,
	pub accept_invite_request: SerOption<AcceptInviteRequest>,
	pub accept_invite_response: SerOption<AcceptInviteResponse>,
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
			get_channels_request: None.into(),
			get_channels_response: None.into(),
			delete_channel_request: None.into(),
			delete_channel_response: None.into(),
			modify_channel_request: None.into(),
			modify_channel_response: None.into(),
			add_channel_request: None.into(),
			add_channel_response: None.into(),
			get_members_request: None.into(),
			get_members_response: None.into(),
			create_invite_request: None.into(),
			create_invite_response: None.into(),
			list_invites_request: None.into(),
			list_invites_response: None.into(),
			modify_invite_request: None.into(),
			modify_invite_response: None.into(),
			delete_invite_request: None.into(),
			delete_invite_response: None.into(),
			view_invite_request: None.into(),
			view_invite_response: None.into(),
			accept_invite_request: None.into(),
			accept_invite_response: None.into(),
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
		writer.write_u16(self.event_type.clone().into())?;
		match &self.event_type {
			EventType::AuthEvent => Writeable::write(&self.auth_event, writer),
			EventType::ChallengeEvent => Writeable::write(&self.challenge_event, writer),
			EventType::AuthResponse => Writeable::write(&self.auth_response, writer),
			EventType::GetServersEvent => Writeable::write(&self.get_servers_event, writer),
			EventType::GetServersResponse => Writeable::write(&self.get_servers_response, writer),
			EventType::CreateServerEvent => Writeable::write(&self.create_server_event, writer),
			EventType::DeleteServerEvent => Writeable::write(&self.delete_server_event, writer),
			EventType::ModifyServerEvent => Writeable::write(&self.modify_server_event, writer),
			EventType::GetChannelsRequest => Writeable::write(&self.get_channels_request, writer),
			EventType::GetChannelsResponse => Writeable::write(&self.get_channels_response, writer),
			EventType::AddChannelResponse => Writeable::write(&self.add_channel_response, writer),
			EventType::ModifyChannelResponse => {
				Writeable::write(&self.modify_channel_response, writer)
			}
			EventType::DeleteChannelResponse => {
				Writeable::write(&self.delete_channel_response, writer)
			}
			EventType::AddChannelRequest => Writeable::write(&self.add_channel_request, writer),
			EventType::ModifyChannelRequest => {
				Writeable::write(&self.modify_channel_request, writer)
			}
			EventType::DeleteChannelRequest => {
				Writeable::write(&self.delete_channel_request, writer)
			}
			EventType::GetMembersRequest => Writeable::write(&self.get_members_request, writer),
			EventType::GetMembersResponse => Writeable::write(&self.get_members_response, writer),
			EventType::CreateInviteRequest => Writeable::write(&self.create_invite_request, writer),
			EventType::CreateInviteResponse => {
				Writeable::write(&self.create_invite_response, writer)
			}
			EventType::ListInvitesRequest => Writeable::write(&self.list_invites_request, writer),
			EventType::ListInvitesResponse => Writeable::write(&self.list_invites_response, writer),
			EventType::ModifyInviteRequest => Writeable::write(&self.modify_invite_request, writer),
			EventType::ModifyInviteResponse => {
				Writeable::write(&self.modify_invite_response, writer)
			}
			EventType::DeleteInviteRequest => Writeable::write(&self.delete_invite_request, writer),
			EventType::DeleteInviteResponse => {
				Writeable::write(&self.delete_invite_response, writer)
			}
			EventType::ViewInviteRequest => Writeable::write(&self.view_invite_request, writer),
			EventType::ViewInviteResponse => Writeable::write(&self.view_invite_response, writer),
			EventType::AcceptInviteRequest => Writeable::write(&self.accept_invite_request, writer),
			EventType::AcceptInviteResponse => {
				Writeable::write(&self.accept_invite_response, writer)
			}
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
		let mut get_channels_request = None.into();
		let mut get_channels_response = None.into();
		let mut add_channel_request = None.into();
		let mut add_channel_response = None.into();
		let mut modify_channel_request = None.into();
		let mut modify_channel_response = None.into();
		let mut delete_channel_request = None.into();
		let mut delete_channel_response = None.into();
		let mut get_members_request = None.into();
		let mut get_members_response = None.into();
		let mut create_invite_request = None.into();
		let mut create_invite_response = None.into();
		let mut list_invites_request = None.into();
		let mut list_invites_response = None.into();
		let mut modify_invite_request = None.into();
		let mut modify_invite_response = None.into();
		let mut delete_invite_request = None.into();
		let mut delete_invite_response = None.into();
		let mut view_invite_request = None.into();
		let mut view_invite_response = None.into();
		let mut accept_invite_request = None.into();
		let mut accept_invite_response = None.into();

		let version = reader.read_u8()?;
		let timestamp = reader.read_u128()?;

		let event_type: EventType = EventType::try_from(reader.read_u16()?).map_err(|e| {
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
			EventType::GetChannelsRequest => get_channels_request = SerOption::read(reader)?,
			EventType::GetChannelsResponse => get_channels_response = SerOption::read(reader)?,
			EventType::AddChannelRequest => add_channel_request = SerOption::read(reader)?,
			EventType::ModifyChannelRequest => modify_channel_request = SerOption::read(reader)?,
			EventType::DeleteChannelRequest => delete_channel_request = SerOption::read(reader)?,
			EventType::AddChannelResponse => add_channel_response = SerOption::read(reader)?,
			EventType::ModifyChannelResponse => modify_channel_response = SerOption::read(reader)?,
			EventType::DeleteChannelResponse => delete_channel_response = SerOption::read(reader)?,
			EventType::GetMembersRequest => get_members_request = SerOption::read(reader)?,
			EventType::GetMembersResponse => get_members_response = SerOption::read(reader)?,
			EventType::CreateInviteRequest => create_invite_request = SerOption::read(reader)?,
			EventType::CreateInviteResponse => create_invite_response = SerOption::read(reader)?,
			EventType::ListInvitesRequest => list_invites_request = SerOption::read(reader)?,
			EventType::ListInvitesResponse => list_invites_response = SerOption::read(reader)?,
			EventType::ModifyInviteRequest => modify_invite_request = SerOption::read(reader)?,
			EventType::ModifyInviteResponse => modify_invite_response = SerOption::read(reader)?,
			EventType::DeleteInviteRequest => delete_invite_request = SerOption::read(reader)?,
			EventType::DeleteInviteResponse => delete_invite_response = SerOption::read(reader)?,
			EventType::ViewInviteRequest => view_invite_request = SerOption::read(reader)?,
			EventType::ViewInviteResponse => view_invite_response = SerOption::read(reader)?,
			EventType::AcceptInviteRequest => accept_invite_request = SerOption::read(reader)?,
			EventType::AcceptInviteResponse => accept_invite_response = SerOption::read(reader)?,
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
			get_channels_request,
			get_channels_response,
			add_channel_request,
			add_channel_response,
			modify_channel_request,
			modify_channel_response,
			delete_channel_request,
			delete_channel_response,
			get_members_request,
			get_members_response,
			create_invite_request,
			create_invite_response,
			list_invites_request,
			list_invites_response,
			modify_invite_request,
			modify_invite_response,
			delete_invite_request,
			delete_invite_response,
			view_invite_request,
			view_invite_response,
			accept_invite_request,
			accept_invite_response,
			timestamp,
		})
	}
}
