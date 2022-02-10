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
use concorddata::ser::{Readable, Reader, Writeable, Writer};
use concorddata::types::{
	Image, Invite, ProfileValue, Pubkey, SerOption, SerString, ServerId, Signature, U128,
};
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct AddChannelRequest {
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
	pub name: SerString,
	pub description: SerString,
}

impl Writeable for AddChannelRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.name, writer)?;
		Writeable::write(&self.description, writer)?;
		Ok(())
	}
}

impl Readable for AddChannelRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let name = SerString::read(reader)?;
		let description = SerString::read(reader)?;
		Ok(Self {
			server_id,
			server_pubkey,
			name,
			description,
		})
	}
}

#[derive(Debug, Clone)]
pub struct AddChannelResponse {
	pub channel_id: u64,
	pub success: bool,
}

impl Writeable for AddChannelResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
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
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		let channel_id = reader.read_u64()?;
		Ok(Self {
			success,
			channel_id,
		})
	}
}

#[derive(Debug, Clone)]
pub struct ModifyChannelRequest {
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
	pub channel_id: u64,
	pub name: SerString,
	pub description: SerString,
}

impl Writeable for ModifyChannelRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
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
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let channel_id = reader.read_u64()?;
		let name = SerString::read(reader)?;
		let description = SerString::read(reader)?;
		Ok(Self {
			channel_id,
			server_id,
			server_pubkey,
			name,
			description,
		})
	}
}

#[derive(Debug, Clone)]
pub struct ModifyChannelResponse {
	pub success: bool,
}

impl Writeable for ModifyChannelResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for ModifyChannelResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self { success })
	}
}

#[derive(Debug, Clone)]
pub struct DeleteChannelRequest {
	pub channel_id: u64,
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
}

impl Writeable for DeleteChannelRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		writer.write_u64(self.channel_id)?;
		Ok(())
	}
}

impl Readable for DeleteChannelRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let channel_id = reader.read_u64()?;
		Ok(Self {
			channel_id,
			server_id,
			server_pubkey,
		})
	}
}

#[derive(Debug, Clone)]
pub struct DeleteChannelResponse {
	pub success: bool,
}

impl Writeable for DeleteChannelResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for DeleteChannelResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self { success })
	}
}

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct CreateInviteRequest {
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
	pub count: u64,
	pub expiration: u128,
}

impl Writeable for CreateInviteRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		writer.write_u64(self.count)?;
		writer.write_u128(self.expiration)?;
		Ok(())
	}
}

impl Readable for CreateInviteRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let count = reader.read_u64()?;
		let expiration = reader.read_u128()?;
		Ok(Self {
			server_id,
			server_pubkey,
			count,
			expiration,
		})
	}
}

#[derive(Debug, Clone)]
pub struct CreateInviteResponse {
	pub success: bool,
	pub invite_id: u128,
}

impl Writeable for CreateInviteResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
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
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		let invite_id = reader.read_u128()?;

		Ok(Self { success, invite_id })
	}
}

#[derive(Debug, Clone)]
pub struct ListInvitesRequest {
	pub server_id: ServerId,
	pub server_pubkey: Pubkey,
}

impl Writeable for ListInvitesRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Ok(())
	}
}

impl Readable for ListInvitesRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let server_id = ServerId::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		Ok(Self {
			server_id,
			server_pubkey,
		})
	}
}

#[derive(Debug, Clone)]
pub struct ListInvitesResponse {
	pub invites: Vec<Invite>,
}

impl Writeable for ListInvitesResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u64(self.invites.len().try_into()?)?;
		for invite in &self.invites {
			Writeable::write(invite, writer)?;
		}
		Ok(())
	}
}

impl Readable for ListInvitesResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let len = reader.read_u64()?;
		let mut invites = vec![];
		for _ in 0..len {
			invites.push(Invite::read(reader)?);
		}

		Ok(Self { invites })
	}
}

#[derive(Debug, Clone)]
pub struct ModifyInviteRequest {
	pub invite_id: u128,
	pub max: u64,
	pub expiration: u128,
}

impl Writeable for ModifyInviteRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.invite_id)?;
		writer.write_u64(self.max)?;
		writer.write_u128(self.expiration)?;
		Ok(())
	}
}

impl Readable for ModifyInviteRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let invite_id = reader.read_u128()?;
		let max = reader.read_u64()?;
		let expiration = reader.read_u128()?;

		Ok(Self {
			invite_id,
			max,
			expiration,
		})
	}
}

#[derive(Debug, Clone)]
pub struct ModifyInviteResponse {
	pub success: bool,
}

impl Writeable for ModifyInviteResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for ModifyInviteResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self { success })
	}
}

#[derive(Debug, Clone)]
pub struct DeleteInviteRequest {
	pub invite_id: u128,
}

impl Writeable for DeleteInviteRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.invite_id)?;
		Ok(())
	}
}

impl Readable for DeleteInviteRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let invite_id = reader.read_u128()?;

		Ok(Self { invite_id })
	}
}

#[derive(Debug, Clone)]
pub struct DeleteInviteResponse {
	pub success: bool,
}

impl Writeable for DeleteInviteResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for DeleteInviteResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self { success })
	}
}

#[derive(Debug, Clone)]
pub struct JoinServerRequest {
	pub invite_url: SerString,
}

impl Writeable for JoinServerRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.invite_url, writer)?;
		Ok(())
	}
}

impl Readable for JoinServerRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let invite_url = SerString::read(reader)?;

		Ok(Self { invite_url })
	}
}

#[derive(Debug, Clone)]
pub struct JoinServerResponse {
	pub success: bool,
}

impl Writeable for JoinServerResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for JoinServerResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self { success })
	}
}

#[derive(Debug, Clone)]
pub struct ViewInviteRequest {
	pub invite_url: SerString,
}

impl Writeable for ViewInviteRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.invite_url, writer)?;
		Ok(())
	}
}

impl Readable for ViewInviteRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let invite_url = SerString::read(reader)?;

		Ok(Self { invite_url })
	}
}

#[derive(Debug, Clone)]
pub struct InviteResponseInfo {
	pub inviter_name: SerString,
	pub inviter_icon: Image,
	pub server_icon: Image,
	pub server_name: SerString,
	pub current_members: u64,
	pub online_members: u64,
}

#[derive(Debug, Clone)]
pub struct ViewInviteResponse {
	pub response_info: Option<InviteResponseInfo>,
}

impl Writeable for ViewInviteResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match &self.response_info {
			Some(rinfo) => {
				writer.write_u8(1)?;
				Writeable::write(&rinfo.inviter_name, writer)?;
				Writeable::write(&rinfo.inviter_icon, writer)?;
				Writeable::write(&rinfo.server_icon, writer)?;
				writer.write_u64(rinfo.current_members)?;
				writer.write_u64(rinfo.online_members)?;
				Writeable::write(&rinfo.server_name, writer)?;
			}
			None => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for ViewInviteResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let response_info = match reader.read_u8()? {
			0 => None,
			_ => {
				let inviter_name = SerString::read(reader)?;
				let inviter_icon = Image::read(reader)?;
				let server_icon = Image::read(reader)?;
				let current_members = reader.read_u64()?;
				let online_members = reader.read_u64()?;
				let server_name = SerString::read(reader)?;

				let rinfo = InviteResponseInfo {
					inviter_name,
					inviter_icon,
					server_icon,
					current_members,
					online_members,
					server_name,
				};
				Some(rinfo)
			}
		};
		Ok(Self { response_info })
	}
}

#[derive(Debug, Clone)]
pub struct AcceptInviteRequest {
	pub invite_id: u128,
	pub user_pubkey: [u8; 32],
	pub server_pubkey: [u8; 32],
	pub user_name: SerString,
	pub user_bio: SerString,
	pub avatar: Image,
}

impl Writeable for AcceptInviteRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.invite_id)?;
		for i in 0..32 {
			writer.write_u8(self.user_pubkey[i])?;
		}
		for i in 0..32 {
			writer.write_u8(self.server_pubkey[i])?;
		}
		Writeable::write(&self.user_name, writer)?;
		Writeable::write(&self.user_bio, writer)?;
		Writeable::write(&self.avatar, writer)?;
		Ok(())
	}
}

impl Readable for AcceptInviteRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let invite_id = reader.read_u128()?;
		let mut user_pubkey = [0u8; 32];
		let mut server_pubkey = [0u8; 32];
		for i in 0..32 {
			user_pubkey[i] = reader.read_u8()?;
		}
		for i in 0..32 {
			server_pubkey[i] = reader.read_u8()?;
		}
		let user_name = SerString::read(reader)?;
		let user_bio = SerString::read(reader)?;
		let avatar = Image::read(reader)?;

		Ok(Self {
			invite_id,
			user_pubkey,
			server_pubkey,
			user_name,
			user_bio,
			avatar,
		})
	}
}

#[derive(Debug, Clone)]
pub struct AcceptInviteResponse {
	pub success: bool,
}

impl Writeable for AcceptInviteResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match self.success {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for AcceptInviteResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let success = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self { success })
	}
}

#[derive(PartialEq, Debug, Clone)]
pub enum ProfileImageRequestType {
	ReturnAvatars,
	SaveAvatars,
	NoAvatars,
}

#[derive(Debug, Clone)]
pub struct GetProfileRequest {
	pub user_pubkeys: Vec<Pubkey>,
	pub server_pubkey: Pubkey,
	pub server_id: ServerId,
	pub image_request_type: ProfileImageRequestType,
	pub include_profile_data: bool,
}

impl Writeable for GetProfileRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let len = self.user_pubkeys.len();
		writer.write_u64(len.try_into()?)?;
		for i in 0..len {
			Writeable::write(&self.user_pubkeys[i], writer)?;
		}
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;
		match self.image_request_type {
			ProfileImageRequestType::ReturnAvatars => writer.write_u8(0)?,
			ProfileImageRequestType::SaveAvatars => writer.write_u8(1)?,
			ProfileImageRequestType::NoAvatars => writer.write_u8(2)?,
		}
		match self.include_profile_data {
			true => writer.write_u8(1)?,
			false => writer.write_u8(0)?,
		}
		Ok(())
	}
}

impl Readable for GetProfileRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut user_pubkeys = vec![];
		let len = reader.read_u64()?;
		for _ in 0..len {
			user_pubkeys.push(Pubkey::read(reader)?);
		}
		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;
		let image_request_type = match reader.read_u8()? {
			0 => ProfileImageRequestType::ReturnAvatars,
			1 => ProfileImageRequestType::SaveAvatars,
			_ => ProfileImageRequestType::NoAvatars,
		};
		let include_profile_data = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		Ok(Self {
			user_pubkeys,
			server_pubkey,
			server_id,
			image_request_type,
			include_profile_data,
		})
	}
}

#[derive(Debug, Clone)]
pub struct GetProfileResponse {
	pub data: Vec<(SerOption<Image>, SerOption<ProfileValue>)>,
}

impl Writeable for GetProfileResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let len = self.data.len();
		writer.write_u64(len.try_into()?)?;
		for i in 0..len {
			Writeable::write(&self.data[i], writer)?;
		}
		Ok(())
	}
}

impl Readable for GetProfileResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let len = reader.read_u64()?;
		let mut data = vec![];
		for _ in 0..len {
			data.push((SerOption::read(reader)?, SerOption::read(reader)?));
		}
		Ok(Self { data })
	}
}

#[derive(Debug, Clone)]
pub struct SetProfileRequest {
	pub server_pubkey: Pubkey,
	pub server_id: ServerId,
	pub avatar: SerOption<Image>,
	pub profile_data: SerOption<ProfileValue>,
}

impl Writeable for SetProfileRequest {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.avatar, writer)?;
		Writeable::write(&self.profile_data, writer)?;
		Ok(())
	}
}

impl Readable for SetProfileRequest {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;
		let avatar = SerOption::read(reader)?;
		let profile_data = SerOption::read(reader)?;
		Ok(Self {
			server_pubkey,
			server_id,
			avatar,
			profile_data,
		})
	}
}

#[derive(Debug, Clone)]
pub struct SetProfileResponse {
	pub success: bool,
}

impl Writeable for SetProfileResponse {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match self.success {
			true => writer.write_u8(1),
			false => writer.write_u8(0),
		}
	}
}

impl Readable for SetProfileResponse {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		Ok(Self {
			success: match reader.read_u8()? {
				0 => false,
				_ => true,
			},
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

#[derive(Debug, Clone)]
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
			None => ("".to_string().into(), "".to_string().into()),
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
	JoinServerRequest,
	JoinServerResponse,
	GetProfileRequest,
	GetProfileResponse,
	SetProfileRequest,
	SetProfileResponse,
}

#[derive(Debug, Clone)]
pub enum EventBody {
	AuthEvent(AuthEvent),
	ChallengeEvent(ChallengeEvent),
	AuthResponse(AuthResponse),
	GetServersEvent(GetServersEvent),
	GetServersResponse(GetServersResponse),
	CreateServerEvent(CreateServerEvent),
	DeleteServerEvent(DeleteServerEvent),
	ModifyServerEvent(ModifyServerEvent),
	GetChannelsRequest(GetChannelsRequest),
	GetChannelsResponse(GetChannelsResponse),
	AddChannelRequest(AddChannelRequest),
	DeleteChannelRequest(DeleteChannelRequest),
	ModifyChannelRequest(ModifyChannelRequest),
	AddChannelResponse(AddChannelResponse),
	DeleteChannelResponse(DeleteChannelResponse),
	ModifyChannelResponse(ModifyChannelResponse),
	GetMembersRequest(GetMembersRequest),
	GetMembersResponse(GetMembersResponse),
	CreateInviteRequest(CreateInviteRequest),
	CreateInviteResponse(CreateInviteResponse),
	ListInvitesRequest(ListInvitesRequest),
	ListInvitesResponse(ListInvitesResponse),
	ModifyInviteRequest(ModifyInviteRequest),
	ModifyInviteResponse(ModifyInviteResponse),
	DeleteInviteRequest(DeleteInviteRequest),
	DeleteInviteResponse(DeleteInviteResponse),
	ViewInviteRequest(ViewInviteRequest),
	ViewInviteResponse(ViewInviteResponse),
	AcceptInviteRequest(AcceptInviteRequest),
	AcceptInviteResponse(AcceptInviteResponse),
	JoinServerRequest(JoinServerRequest),
	JoinServerResponse(JoinServerResponse),
	GetProfileRequest(GetProfileRequest),
	GetProfileResponse(GetProfileResponse),
	SetProfileRequest(SetProfileRequest),
	SetProfileResponse(SetProfileResponse),
}

impl Writeable for EventBody {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match self {
			EventBody::AuthEvent(e) => {
				writer.write_u16(0)?;
				Writeable::write(e, writer)?;
			}
			EventBody::ChallengeEvent(e) => {
				writer.write_u16(1)?;
				Writeable::write(e, writer)?;
			}
			EventBody::AuthResponse(e) => {
				writer.write_u16(2)?;
				Writeable::write(e, writer)?;
			}
			EventBody::GetServersEvent(e) => {
				writer.write_u16(3)?;
				Writeable::write(e, writer)?;
			}
			EventBody::GetServersResponse(e) => {
				writer.write_u16(4)?;
				Writeable::write(e, writer)?;
			}
			EventBody::CreateServerEvent(e) => {
				writer.write_u16(5)?;
				Writeable::write(e, writer)?;
			}
			EventBody::DeleteServerEvent(e) => {
				writer.write_u16(6)?;
				Writeable::write(e, writer)?;
			}
			EventBody::ModifyServerEvent(e) => {
				writer.write_u16(7)?;
				Writeable::write(e, writer)?;
			}
			EventBody::GetChannelsRequest(e) => {
				writer.write_u16(8)?;
				Writeable::write(e, writer)?;
			}
			EventBody::GetChannelsResponse(e) => {
				writer.write_u16(9)?;
				Writeable::write(e, writer)?;
			}
			EventBody::DeleteChannelRequest(e) => {
				writer.write_u16(10)?;
				Writeable::write(e, writer)?;
			}
			EventBody::DeleteChannelResponse(e) => {
				writer.write_u16(11)?;
				Writeable::write(e, writer)?;
			}
			EventBody::ModifyChannelRequest(e) => {
				writer.write_u16(12)?;
				Writeable::write(e, writer)?;
			}
			EventBody::ModifyChannelResponse(e) => {
				writer.write_u16(13)?;
				Writeable::write(e, writer)?;
			}
			EventBody::AddChannelRequest(e) => {
				writer.write_u16(14)?;
				Writeable::write(e, writer)?;
			}
			EventBody::AddChannelResponse(e) => {
				writer.write_u16(15)?;
				Writeable::write(e, writer)?;
			}
			EventBody::GetMembersRequest(e) => {
				writer.write_u16(16)?;
				Writeable::write(e, writer)?;
			}
			EventBody::GetMembersResponse(e) => {
				writer.write_u16(17)?;
				Writeable::write(e, writer)?;
			}
			EventBody::CreateInviteRequest(e) => {
				writer.write_u16(18)?;
				Writeable::write(e, writer)?;
			}
			EventBody::CreateInviteResponse(e) => {
				writer.write_u16(19)?;
				Writeable::write(e, writer)?;
			}
			EventBody::ListInvitesRequest(e) => {
				writer.write_u16(20)?;
				Writeable::write(e, writer)?;
			}
			EventBody::ListInvitesResponse(e) => {
				writer.write_u16(21)?;
				Writeable::write(e, writer)?;
			}
			EventBody::ModifyInviteRequest(e) => {
				writer.write_u16(22)?;
				Writeable::write(e, writer)?;
			}
			EventBody::ModifyInviteResponse(e) => {
				writer.write_u16(23)?;
				Writeable::write(e, writer)?;
			}
			EventBody::DeleteInviteRequest(e) => {
				writer.write_u16(24)?;
				Writeable::write(e, writer)?;
			}
			EventBody::DeleteInviteResponse(e) => {
				writer.write_u16(25)?;
				Writeable::write(e, writer)?;
			}
			EventBody::ViewInviteRequest(e) => {
				writer.write_u16(26)?;
				Writeable::write(e, writer)?;
			}
			EventBody::ViewInviteResponse(e) => {
				writer.write_u16(27)?;
				Writeable::write(e, writer)?;
			}
			EventBody::AcceptInviteRequest(e) => {
				writer.write_u16(28)?;
				Writeable::write(e, writer)?;
			}
			EventBody::AcceptInviteResponse(e) => {
				writer.write_u16(29)?;
				Writeable::write(e, writer)?;
			}
			EventBody::JoinServerRequest(e) => {
				writer.write_u16(30)?;
				Writeable::write(e, writer)?;
			}
			EventBody::JoinServerResponse(e) => {
				writer.write_u16(31)?;
				Writeable::write(e, writer)?;
			}
			EventBody::GetProfileRequest(e) => {
				writer.write_u16(32)?;
				Writeable::write(e, writer)?;
			}
			EventBody::GetProfileResponse(e) => {
				writer.write_u16(33)?;
				Writeable::write(e, writer)?;
			}
			EventBody::SetProfileRequest(e) => {
				writer.write_u16(34)?;
				Writeable::write(e, writer)?;
			}
			EventBody::SetProfileResponse(e) => {
				writer.write_u16(35)?;
				Writeable::write(e, writer)?;
			}
		}
		Ok(())
	}
}

impl Readable for EventBody {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let body_type = reader.read_u16()?;
		match body_type {
			0 => Ok(EventBody::AuthEvent(AuthEvent::read(reader)?)),
			1 => Ok(EventBody::ChallengeEvent(ChallengeEvent::read(reader)?)),
			2 => Ok(EventBody::AuthResponse(AuthResponse::read(reader)?)),
			3 => Ok(EventBody::GetServersEvent(GetServersEvent::read(reader)?)),
			4 => Ok(EventBody::GetServersResponse(GetServersResponse::read(
				reader,
			)?)),
			5 => Ok(EventBody::CreateServerEvent(CreateServerEvent::read(
				reader,
			)?)),
			6 => Ok(EventBody::DeleteServerEvent(DeleteServerEvent::read(
				reader,
			)?)),
			7 => Ok(EventBody::ModifyServerEvent(ModifyServerEvent::read(
				reader,
			)?)),
			8 => Ok(EventBody::GetChannelsRequest(GetChannelsRequest::read(
				reader,
			)?)),
			9 => Ok(EventBody::GetChannelsResponse(GetChannelsResponse::read(
				reader,
			)?)),
			10 => Ok(EventBody::DeleteChannelRequest(DeleteChannelRequest::read(
				reader,
			)?)),
			11 => Ok(EventBody::DeleteChannelResponse(
				DeleteChannelResponse::read(reader)?,
			)),
			12 => Ok(EventBody::ModifyChannelRequest(ModifyChannelRequest::read(
				reader,
			)?)),
			13 => Ok(EventBody::ModifyChannelResponse(
				ModifyChannelResponse::read(reader)?,
			)),
			14 => Ok(EventBody::AddChannelRequest(AddChannelRequest::read(
				reader,
			)?)),
			15 => Ok(EventBody::AddChannelResponse(AddChannelResponse::read(
				reader,
			)?)),
			16 => Ok(EventBody::GetMembersRequest(GetMembersRequest::read(
				reader,
			)?)),
			17 => Ok(EventBody::GetMembersResponse(GetMembersResponse::read(
				reader,
			)?)),
			18 => Ok(EventBody::CreateInviteRequest(CreateInviteRequest::read(
				reader,
			)?)),
			19 => Ok(EventBody::CreateInviteResponse(CreateInviteResponse::read(
				reader,
			)?)),
			20 => Ok(EventBody::ListInvitesRequest(ListInvitesRequest::read(
				reader,
			)?)),
			21 => Ok(EventBody::ListInvitesResponse(ListInvitesResponse::read(
				reader,
			)?)),
			22 => Ok(EventBody::ModifyInviteRequest(ModifyInviteRequest::read(
				reader,
			)?)),
			23 => Ok(EventBody::ModifyInviteResponse(ModifyInviteResponse::read(
				reader,
			)?)),
			24 => Ok(EventBody::DeleteInviteRequest(DeleteInviteRequest::read(
				reader,
			)?)),
			25 => Ok(EventBody::DeleteInviteResponse(DeleteInviteResponse::read(
				reader,
			)?)),
			26 => Ok(EventBody::ViewInviteRequest(ViewInviteRequest::read(
				reader,
			)?)),
			27 => Ok(EventBody::ViewInviteResponse(ViewInviteResponse::read(
				reader,
			)?)),
			28 => Ok(EventBody::AcceptInviteRequest(AcceptInviteRequest::read(
				reader,
			)?)),
			29 => Ok(EventBody::AcceptInviteResponse(AcceptInviteResponse::read(
				reader,
			)?)),
			30 => Ok(EventBody::JoinServerRequest(JoinServerRequest::read(
				reader,
			)?)),
			31 => Ok(EventBody::JoinServerResponse(JoinServerResponse::read(
				reader,
			)?)),
			32 => Ok(EventBody::GetProfileRequest(GetProfileRequest::read(
				reader,
			)?)),
			33 => Ok(EventBody::GetProfileResponse(GetProfileResponse::read(
				reader,
			)?)),
			34 => Ok(EventBody::SetProfileRequest(SetProfileRequest::read(
				reader,
			)?)),
			35 => Ok(EventBody::SetProfileResponse(SetProfileResponse::read(
				reader,
			)?)),
			_ => Err(ErrorKind::CorruptedData("corrupted data in EventBody".to_string()).into()),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Event {
	pub body: EventBody,
	pub version: u8,
	pub timestamp: u128,
	pub request_id: u32,
}

impl Default for Event {
	fn default() -> Self {
		Self {
			version: PROTOCOL_VERSION,
			timestamp: std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap_or(std::time::Duration::from_millis(0))
				.as_millis(),
			body: EventBody::GetServersEvent(GetServersEvent {}), // always replace
			request_id: rand::random(),
		}
	}
}

impl Writeable for Event {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		info!("writing event {:?}", self);
		writer.write_u8(self.version)?;
		writer.write_u128(self.timestamp)?;
		writer.write_u32(self.request_id)?;
		Writeable::write(&self.body, writer)
	}
}

impl Readable for Event {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let version = reader.read_u8()?;
		let timestamp = reader.read_u128()?;
		let request_id = reader.read_u32()?;
		let body = EventBody::read(reader)?;

		Ok(Self {
			version,
			timestamp,
			body,
			request_id,
		})
	}
}
