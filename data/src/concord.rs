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

use crate::lmdb::{Batch, Store};
use crate::nioruntime_log;
use crate::ser::serialize_default;
use crate::ser::{BinReader, ProtocolVersion, Readable, Reader, Writeable, Writer};
use crate::types::{Invite, ProfileData, Pubkey, SerString, ServerId};
use concorderror::{Error, ErrorKind};
use nioruntime_log::*;

use std::convert::TryInto;
use std::io::Cursor;
use std::path::PathBuf;

const DB_NAME: &str = "concord";
const MESSAGE_BATCH_SIZE: u64 = 100;
const MEMBER_BATCH_SIZE: u64 = 100;

pub const TOKEN_EXPIRATION: u128 = 1000 * 60 * 60;

info!();

pub fn get_default_profile() -> Profile {
	Profile {
		profile_data: ProfileData {
			user_name: SerString {
				data: "User Default".to_string(),
			},
			user_bio: SerString {
				data: "Tell us about you..".to_string(),
			},
		},
		server_id: ServerId::from_bytes([0u8; 8]),
		server_pubkey: Pubkey::from_bytes([0u8; 32]),
		user_pubkey: Pubkey::from_bytes([0u8; 32]),
	}
}

// the context to use for accessing concord data. Multiple instances
// may exist and LMDB handles concurrency.
pub struct DSContext {
	store: Store,
}

#[derive(Debug, Clone)]
pub struct Profile {
	pub user_pubkey: Pubkey,
	pub server_pubkey: Pubkey,
	pub server_id: ServerId,
	pub profile_data: ProfileData,
}

struct ProfileKey {
	user_pubkey: Pubkey,
	server_pubkey: Pubkey,
	server_id: ServerId,
}

impl Default for Profile {
	fn default() -> Self {
		Self {
			user_pubkey: Pubkey::from_bytes([0u8; 32]),
			server_pubkey: Pubkey::from_bytes([0u8; 32]),
			server_id: ServerId::from_bytes([0u8; 8]),
			profile_data: ProfileData::default(),
		}
	}
}

impl Writeable for Profile {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.user_pubkey, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.profile_data, writer)?;
		Ok(())
	}
}

impl Readable for Profile {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let user_pubkey = Pubkey::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;
		let profile_data = ProfileData::read(reader)?;

		Ok(Profile {
			profile_data,
			server_pubkey,
			server_id,
			user_pubkey,
		})
	}
}

impl Writeable for ProfileKey {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(PROFILE_PREFIX)?;
		Writeable::write(&self.user_pubkey, writer)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;
		Ok(())
	}
}

impl Readable for ProfileKey {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let _ = reader.read_u8()?; // for prefix
		let user_pubkey = Pubkey::read(reader)?;
		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;
		Ok(Self {
			user_pubkey,
			server_pubkey,
			server_id,
		})
	}
}

struct AuthToken {
	auth_token: u128,
}

impl Writeable for AuthToken {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.auth_token)?;
		Ok(())
	}
}

impl Readable for AuthToken {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		Ok(AuthToken {
			auth_token: reader.read_u128()?,
		})
	}
}

pub struct UserPubKey {
	pub user_pubkey: [u8; 32],
}

impl Writeable for UserPubKey {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..32 {
			writer.write_u8(self.user_pubkey[i])?;
		}

		Ok(())
	}
}

impl Readable for UserPubKey {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut user_pubkey = [0u8; 32];

		for i in 0..32 {
			user_pubkey[i] = reader.read_u8()?;
		}

		Ok(UserPubKey { user_pubkey })
	}
}

pub struct Challenge {
	pub challenge: [u8; 8],
}

impl Writeable for Challenge {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..8 {
			writer.write_u8(self.challenge[i])?;
		}

		Ok(())
	}
}

impl Readable for Challenge {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut challenge = [0u8; 8];

		for i in 0..8 {
			challenge[i] = reader.read_u8()?;
		}

		Ok(Challenge { challenge })
	}
}

pub struct InviteKey {
	server_id: [u8; 8],
	inviter: [u8; 32],
	id: u128,
}

impl Writeable for InviteKey {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..8 {
			writer.write_u8(self.server_id[i])?;
		}
		for i in 0..32 {
			writer.write_u8(self.inviter[i])?;
		}

		writer.write_u128(self.id)?;

		Ok(())
	}
}

impl Readable for InviteKey {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut inviter = [0u8; 32];
		let mut server_id = [0u8; 8];

		for i in 0..8 {
			server_id[i] = reader.read_u8()?;
		}

		for i in 0..32 {
			inviter[i] = reader.read_u8()?;
		}

		let id = reader.read_u128()?;

		Ok(InviteKey {
			server_id,
			inviter,
			id,
		})
	}
}

// member data structures

// member structure returned by get_members
#[derive(Debug, Clone)]
pub struct Member {
	pub user_pubkey: Pubkey,
	pub profile_data: Option<ProfileData>,
	pub roles: u128,
	pub join_time: u64,
	pub modified_time: u64,
}

// internal member datastructure
#[derive(Clone, Debug)]
pub struct MemberImpl {
	pub user_pubkey: Pubkey,
	pub server_pubkey: Pubkey,
	pub server_id: ServerId,
	pub roles: u128,
	pub batch_num: u64,
	pub join_time: u64,
	pub modified_time: u64,
}

struct MemberMetaDataKey {
	server_pubkey: Pubkey,
	server_id: ServerId,
}

struct MemberMetaDataValue {
	member_count: u64,
}

struct MemberKeyIttImpl {
	server_pubkey: Pubkey,
	server_id: ServerId,
	batch_num: u64,
	join_time: u64,
	user_pubkey: Pubkey,
}

struct MemberKeyHashImpl {
	server_pubkey: Pubkey,
	server_id: ServerId,
	user_pubkey: Pubkey,
}

struct MemberKeyAuthImpl {
	server_pubkey: Pubkey,
	server_id: ServerId,
	roles: u128,
	batch_num: u64,
	join_time: u64,
	user_pubkey: Pubkey,
}

#[derive(Debug)]
struct MemberBatchLookupKeyImpl {
	server_pubkey: Pubkey,
	server_id: ServerId,
	batch_num: u64,
}

#[derive(Debug)]
struct AuthBatchLookupKeyImpl {
	server_pubkey: Pubkey,
	server_id: ServerId,
	batch_num: u64,
}

struct MemberValueImpl {
	roles: u128,
	join_time: u64,
	modified_time: u64,
	batch_num: u64,
}

impl From<MemberImpl> for Member {
	fn from(mi: MemberImpl) -> Self {
		Self {
			user_pubkey: mi.user_pubkey,
			roles: mi.roles,
			profile_data: None,
			join_time: mi.join_time,
			modified_time: mi.modified_time,
		}
	}
}

impl Writeable for MemberMetaDataValue {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u64(self.member_count)?;
		Ok(())
	}
}

impl Readable for MemberMetaDataValue {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		Ok(Self {
			member_count: reader.read_u64()?,
		})
	}
}

impl Writeable for MemberMetaDataKey {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(MEMBER_META_DATA_PREFIX)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;

		Ok(())
	}
}

impl Readable for MemberMetaDataKey {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		reader.read_u8()?;

		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;

		Ok(Self {
			server_pubkey,
			server_id,
		})
	}
}

impl From<&MemberImpl> for MemberKeyIttImpl {
	fn from(member: &MemberImpl) -> MemberKeyIttImpl {
		Self {
			server_pubkey: member.server_pubkey.clone(),
			server_id: member.server_id.clone(),
			batch_num: member.batch_num,
			join_time: member.join_time,
			user_pubkey: member.user_pubkey.clone(),
		}
	}
}

impl Writeable for MemberKeyIttImpl {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(MEMBER_ITT_PREFIX)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;
		writer.write_u64(self.batch_num)?;
		writer.write_u64(self.join_time)?;
		Writeable::write(&self.user_pubkey, writer)?;

		Ok(())
	}
}

impl Readable for MemberKeyIttImpl {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		reader.read_u8()?;
		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;
		let batch_num = reader.read_u64()?;
		let join_time = reader.read_u64()?;
		let user_pubkey = Pubkey::read(reader)?;

		Ok(Self {
			server_pubkey,
			server_id,
			batch_num,
			join_time,
			user_pubkey,
		})
	}
}

impl Writeable for AuthBatchLookupKeyImpl {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(MEMBER_AUTH_PREFIX)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;
		writer.write_u64(self.batch_num)?;

		Ok(())
	}
}

impl Readable for AuthBatchLookupKeyImpl {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		reader.read_u8()?;
		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;
		let batch_num = reader.read_u64()?;

		Ok(Self {
			server_pubkey,
			server_id,
			batch_num,
		})
	}
}

impl Writeable for MemberBatchLookupKeyImpl {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(MEMBER_ITT_PREFIX)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;
		writer.write_u64(self.batch_num)?;

		Ok(())
	}
}

impl Readable for MemberBatchLookupKeyImpl {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		reader.read_u8()?;
		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;
		let batch_num = reader.read_u64()?;

		Ok(Self {
			server_pubkey,
			server_id,
			batch_num,
		})
	}
}

impl From<&MemberImpl> for MemberKeyHashImpl {
	fn from(member: &MemberImpl) -> MemberKeyHashImpl {
		Self {
			server_pubkey: member.server_pubkey.clone(),
			server_id: member.server_id.clone(),
			user_pubkey: member.user_pubkey.clone(),
		}
	}
}

impl Writeable for MemberKeyHashImpl {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(MEMBER_HASH_PREFIX)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;
		Writeable::write(&self.user_pubkey, writer)?;

		Ok(())
	}
}

impl Readable for MemberKeyHashImpl {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		reader.read_u8()?;
		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;
		let user_pubkey = Pubkey::read(reader)?;

		Ok(Self {
			server_pubkey,
			server_id,
			user_pubkey,
		})
	}
}

impl From<&MemberImpl> for MemberKeyAuthImpl {
	fn from(member: &MemberImpl) -> MemberKeyAuthImpl {
		Self {
			server_pubkey: member.server_pubkey.clone(),
			server_id: member.server_id.clone(),
			roles: member.roles,
			batch_num: member.batch_num,
			join_time: member.join_time,
			user_pubkey: member.user_pubkey.clone(),
		}
	}
}

impl Writeable for MemberKeyAuthImpl {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(MEMBER_AUTH_PREFIX)?;
		Writeable::write(&self.server_pubkey, writer)?;
		Writeable::write(&self.server_id, writer)?;
		writer.write_u64(self.batch_num)?;
		writer.write_u128(self.roles)?;
		writer.write_u64(self.join_time)?;
		Writeable::write(&self.user_pubkey, writer)?;

		Ok(())
	}
}

impl Readable for MemberKeyAuthImpl {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		reader.read_u8()?;
		let server_pubkey = Pubkey::read(reader)?;
		let server_id = ServerId::read(reader)?;
		let batch_num = reader.read_u64()?;
		let roles = reader.read_u128()?;
		let join_time = reader.read_u64()?;
		let user_pubkey = Pubkey::read(reader)?;

		Ok(Self {
			server_pubkey,
			server_id,
			roles,
			batch_num,
			join_time,
			user_pubkey,
		})
	}
}

impl From<&MemberImpl> for MemberValueImpl {
	fn from(member: &MemberImpl) -> MemberValueImpl {
		Self {
			roles: member.roles,
			join_time: member.join_time,
			modified_time: member.modified_time,
			batch_num: member.batch_num,
		}
	}
}

impl Writeable for MemberValueImpl {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.roles)?;
		writer.write_u64(self.join_time)?;
		writer.write_u64(self.modified_time)?;
		writer.write_u64(self.batch_num)?;
		Ok(())
	}
}

impl Readable for MemberValueImpl {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let roles = reader.read_u128()?;
		let join_time = reader.read_u64()?;
		let modified_time = reader.read_u64()?;
		let batch_num = reader.read_u64()?;

		Ok(MemberValueImpl {
			roles,
			join_time,
			modified_time,
			batch_num,
		})
	}
}

// type of message
#[derive(Debug, Clone)]
pub enum MessageType {
	Text,
	Binary,
}

// information associated with a message
#[derive(Debug, Clone)]
pub struct Message {
	pub payload: Vec<u8>,
	pub signature: [u8; 64],
	pub message_type: MessageType,
	pub server_pubkey: [u8; 32],
	pub server_id: [u8; 8],
	pub channel_id: u64,
	pub timestamp: u64,
	pub user_pubkey: [u8; 32],
	pub nonce: u16,
	pub seqno: u64,
	pub user_name: String,
	pub user_bio: String,
}

#[derive(Debug)]
struct MessageKeyImpl {
	server_pubkey: [u8; 32],
	server_id: [u8; 8],
	channel_id: u64,
	batch_num: u64,
	timestamp: u64,
	user_pubkey: [u8; 32],
	nonce: u16,
}

#[derive(Debug)]
struct MessageValueImpl {
	payload: Vec<u8>,
	signature: [u8; 64],
	message_type: MessageType,
}

#[derive(Debug)]
struct MessageMetaDataKey {
	server_pubkey: [u8; 32],
	server_id: [u8; 8],
	channel_id: u64,
}

#[derive(Debug)]
struct MessageMetaDataValue {
	message_count: u64,
}

// the Writeable implmenetation for serializing MessageKey
impl Writeable for MessageKeyImpl {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(MESSAGE_PREFIX)?;
		for i in 0..32 {
			writer.write_u8(self.server_pubkey[i])?;
		}
		for i in 0..8 {
			writer.write_u8(self.server_id[i])?;
		}
		writer.write_u64(self.channel_id)?;
		writer.write_u64(self.batch_num)?;
		writer.write_u64(self.timestamp)?;
		for i in 0..32 {
			writer.write_u8(self.user_pubkey[i])?;
		}
		writer.write_u16(self.nonce)?;
		Ok(())
	}
}

// the Readable implmentation for deserializing MessageKey
impl Readable for MessageKeyImpl {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let _ = reader.read_u8()?;
		let mut server_pubkey = vec![];
		for _ in 0..32 {
			server_pubkey.push(reader.read_u8()?);
		}
		let mut server_id = vec![];
		for _ in 0..8 {
			server_id.push(reader.read_u8()?);
		}
		let channel_id = reader.read_u64()?;
		let batch_num = reader.read_u64()?;
		let timestamp = reader.read_u64()?;
		let mut user_pubkey = vec![];
		for _ in 0..32 {
			user_pubkey.push(reader.read_u8()?);
		}
		let nonce = reader.read_u16()?;

		let server_pubkey = server_pubkey.as_slice().try_into()?;
		let server_id = server_id.as_slice().try_into()?;
		let user_pubkey = user_pubkey.as_slice().try_into()?;

		Ok(MessageKeyImpl {
			server_pubkey,
			server_id,
			channel_id,
			batch_num,
			timestamp,
			user_pubkey,
			nonce,
		})
	}
}

// the Writeable implmenetation for serializing MessageValueImpl
impl Writeable for MessageValueImpl {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let payload_len = self.payload.len();
		writer.write_u32(payload_len.try_into().unwrap_or(0))?;
		for i in 0..payload_len {
			writer.write_u8(self.payload[i])?;
		}
		for i in 0..64 {
			writer.write_u8(self.signature[i])?;
		}
		match self.message_type {
			MessageType::Text => writer.write_u8(0)?,
			MessageType::Binary => writer.write_u8(1)?,
		}

		Ok(())
	}
}

// the Readable implmentation for deserializing MessageValueImpl
impl Readable for MessageValueImpl {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let payload_len = reader.read_u32()?;
		let mut payload = vec![];
		for _ in 0..payload_len {
			payload.push(reader.read_u8()?);
		}
		let mut signature = vec![];
		for _ in 0..64 {
			signature.push(reader.read_u8()?);
		}
		let message_type = match reader.read_u8()? {
			0 => MessageType::Text,
			_ => MessageType::Binary,
		};

		let signature = signature.as_slice().try_into()?;

		Ok(MessageValueImpl {
			payload,
			signature,
			message_type,
		})
	}
}

impl Writeable for MessageMetaDataKey {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(MESSAGE_METADATA_PREFIX)?;
		for i in 0..32 {
			writer.write_u8(self.server_pubkey[i])?;
		}
		for i in 0..8 {
			writer.write_u8(self.server_id[i])?;
		}
		writer.write_u64(self.channel_id)?;
		Ok(())
	}
}

// the Readable implmentation for deserializing MessageKey
impl Readable for MessageMetaDataKey {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let _ = reader.read_u8()?;
		let mut server_pubkey = vec![];
		for _ in 0..32 {
			server_pubkey.push(reader.read_u8()?);
		}
		let mut server_id = vec![];
		for _ in 0..8 {
			server_id.push(reader.read_u8()?);
		}
		let channel_id = reader.read_u64()?;

		let server_pubkey = server_pubkey.as_slice().try_into()?;
		let server_id = server_id.as_slice().try_into()?;

		Ok(MessageMetaDataKey {
			server_pubkey,
			server_id,
			channel_id,
		})
	}
}

// the Writeable implmenetation for serializing MessageMetaDataValue
impl Writeable for MessageMetaDataValue {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u64(self.message_count)?;
		Ok(())
	}
}

// the Readable implmentation for deserializing MessageKey
impl Readable for MessageMetaDataValue {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let message_count = reader.read_u64()?;

		Ok(MessageMetaDataValue { message_count })
	}
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JoinInfoReply {
	pub server_pubkey: [u8; 32],
	pub server_id: [u8; 8],
	pub name: String,
	pub inviter_pubkey: [u8; 32],
}

// information about the server
#[derive(Debug)]
pub struct ServerInfo {
	pub pubkey: [u8; 32],
	pub name: String,
	pub joined: bool,
	pub seqno: u64,
}

#[derive(Debug)]
pub struct ServerInfoReply {
	pub pubkey: [u8; 32],
	pub server_id: [u8; 8],
	pub name: String,
	pub seqno: u64,
}

// the Writeable implmenetation for serializing ServerInfo
impl Writeable for ServerInfo {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..32 {
			writer.write_u8(self.pubkey[i])?;
		}

		let name_len = self.name.len();
		let name_bytes = self.name.as_bytes();
		writer.write_u32(name_len.try_into().unwrap_or(0))?;
		for i in 0..name_len {
			writer.write_u8(name_bytes[i])?;
		}

		match self.joined {
			false => writer.write_u8(0)?,
			true => writer.write_u8(1)?,
		}

		writer.write_u64(self.seqno)?;

		Ok(())
	}
}

// the Readable implmentation for deserializing ServerInfo
impl Readable for ServerInfo {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut pubkey = [0u8; 32];

		for i in 0..32 {
			pubkey[i] = reader.read_u8()?;
		}

		let name_len = reader.read_u32()?;
		let mut name = vec![];
		for _ in 0..name_len {
			name.push(reader.read_u8()?);
		}

		let name = std::str::from_utf8(&name)?;
		let name = name.to_string();

		let joined = reader.read_u8()? != 0;

		let seqno = reader.read_u64()?;

		Ok(ServerInfo {
			pubkey,
			name,
			joined,
			seqno,
		})
	}
}

#[derive(Debug)]
pub struct ChannelKey {
	pub server_pubkey: [u8; 32],
	pub server_id: [u8; 8],
	pub channel_id: u64,
}

impl Writeable for ChannelKey {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..32 {
			writer.write_u8(self.server_pubkey[i])?;
		}
		for i in 0..8 {
			writer.write_u8(self.server_id[i])?;
		}
		writer.write_u64(self.channel_id)?;

		Ok(())
	}
}

impl Readable for ChannelKey {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut server_pubkey = [0u8; 32];
		let mut server_id = [0u8; 8];

		for i in 0..32 {
			server_pubkey[i] = reader.read_u8()?;
		}

		for i in 0..8 {
			server_id[i] = reader.read_u8()?;
		}

		let channel_id = reader.read_u64()?;

		let channel_key = ChannelKey {
			server_pubkey,
			server_id,
			channel_id,
		};

		Ok(channel_key)
	}
}

#[derive(Debug, Serialize)]
pub struct Channel {
	pub name: String,
	pub description: String,
	pub channel_id: u64,
}

// the Writeable implmenetation for serializing Channel
impl Writeable for Channel {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let name_len = self.name.len();
		let name_bytes = self.name.as_bytes();
		writer.write_u32(name_len.try_into().unwrap_or(0))?;
		for i in 0..name_len {
			writer.write_u8(name_bytes[i])?;
		}
		let description_len = self.description.len();
		let description_bytes = self.description.as_bytes();
		writer.write_u32(description_len.try_into().unwrap_or(0))?;
		for i in 0..description_len {
			writer.write_u8(description_bytes[i])?;
		}
		writer.write_u64(self.channel_id)?;

		Ok(())
	}
}

// the Readable implmentation for deserializing Channel
impl Readable for Channel {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let len = reader.read_u32()?;
		let mut name = vec![];
		for _ in 0..len {
			name.push(reader.read_u8()?);
		}
		let name = std::str::from_utf8(&name)?.to_string();

		let len = reader.read_u32()?;
		let mut description = vec![];
		for _ in 0..len {
			description.push(reader.read_u8()?);
		}
		let description = std::str::from_utf8(&description)?.to_string();
		let channel_id = reader.read_u64()?;

		let channel = Channel {
			name,
			description,
			channel_id,
		};

		Ok(channel)
	}
}

pub struct WSAuthToken {
	pub token: u128,
}

impl Writeable for WSAuthToken {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(WS_AUTH_TOKEN)?;
		writer.write_u128(self.token)?;
		Ok(())
	}
}

impl Readable for WSAuthToken {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let _ = reader.read_u8()?;
		Ok(WSAuthToken {
			token: reader.read_u128()?,
		})
	}
}

pub struct AuthInfo {
	creation_time: u128,
	pub last_access_time: u128,
	pub expiration_millis: u128,
}

impl Writeable for AuthInfo {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u128(self.creation_time)?;
		writer.write_u128(self.last_access_time)?;
		writer.write_u128(self.expiration_millis)?;
		Ok(())
	}
}

impl Readable for AuthInfo {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let creation_time = reader.read_u128()?;
		let last_access_time = reader.read_u128()?;
		let expiration_millis = reader.read_u128()?;

		Ok(AuthInfo {
			creation_time,
			last_access_time,
			expiration_millis,
		})
	}
}

// data prefixes
const SERVER_PREFIX: u8 = 0;
const TOKEN_PREFIX: u8 = 1;
const MESSAGE_PREFIX: u8 = 2;
const CHANNEL_PREFIX: u8 = 3;
const MEMBER_ITT_PREFIX: u8 = 4;
const INVITE_PREFIX: u8 = 5;
const INVITE_ID_PREFIX: u8 = 6;
const CHALLENGE_PREFIX: u8 = 7;
const STORED_AUTH_TOKEN_PREFIX: u8 = 8;
const MESSAGE_METADATA_PREFIX: u8 = 9;
const PROFILE_PREFIX: u8 = 10;
const MEMBER_HASH_PREFIX: u8 = 11;
const MEMBER_META_DATA_PREFIX: u8 = 12;
const MEMBER_AUTH_PREFIX: u8 = 13;
const WS_AUTH_TOKEN: u8 = 14;

// auth levels
pub const AUTH_FLAG_OWNER: u128 = 1;
pub const AUTH_FLAG_MEMBER: u128 = 1 << 1;

impl DSContext {
	// get a list of servers in the local database
	pub fn get_servers(&self) -> Result<Vec<ServerInfoReply>, Error> {
		let batch = self.store.batch()?;
		// get the iterator for each server info
		let mut itt = batch.iter(&(vec![SERVER_PREFIX])[..], |k, v| {
			let mut id = [0u8; 8];
			id.clone_from_slice(&k[1..9]);
			let mut cursor = Cursor::new(v.to_vec());
			cursor.set_position(0);
			let mut reader = BinReader::new(&mut cursor, ProtocolVersion::local());
			Ok((ServerInfo::read(&mut reader)?, id.clone()))
		})?;

		let mut ret = vec![];
		loop {
			match itt.next() {
				Some((server, server_id)) => {
					let server_id = *(&server_id[..].try_into()?);
					if server.joined {
						ret.push(ServerInfoReply {
							server_id,
							name: server.name,
							pubkey: server.pubkey,
							seqno: server.seqno,
						});
					}
				}
				None => break,
			}
		}

		Ok(ret)
	}

	pub fn get_server_info(
		&self,
		server_id: [u8; 8],
		server_pubkey: [u8; 32],
	) -> Result<Option<ServerInfoReply>, Error> {
		let batch = self.store.batch()?;
		let mut key = vec![SERVER_PREFIX];
		key.append(&mut server_id.to_vec());
		key.append(&mut server_pubkey.to_vec());
		let ret: Option<ServerInfo> = batch.get_ser(&key)?;
		match ret {
			None => Ok(None),
			Some(ret) => Ok(Some(ServerInfoReply {
				server_id,
				pubkey: ret.pubkey,
				name: ret.name,
				seqno: ret.seqno,
			})),
		}
	}

	pub fn modify_server(
		&self,
		server_id: [u8; 8],
		server_pubkey: [u8; 32],
		name: String,
	) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut key = vec![SERVER_PREFIX];
		key.append(&mut server_id.to_vec());
		key.append(&mut server_pubkey.to_vec());
		let server_info: Option<ServerInfo> = batch.get_ser(&key)?;

		let server_info = match server_info {
			None => return Ok(()), // shouldn't happen, but deal with it in client
			Some(mut server_info) => {
				server_info.name = name;
				server_info.seqno = server_info.seqno + 1;
				server_info
			}
		};

		batch.put_ser(&key, &server_info)?;
		batch.commit()?;
		Ok(())
	}

	// add a server
	pub fn add_server(
		&self,
		server_info: ServerInfo,
		server_id: Option<[u8; 8]>,
		user_pubkey: Option<[u8; 32]>,
		remote: bool,
	) -> Result<[u8; 8], Error> {
		let batch = self.store.batch()?;
		let mut key = vec![SERVER_PREFIX];
		let server_id: [u8; 8] = match server_id {
			Some(server_id) => server_id,
			None => rand::random(),
		};
		key.append(&mut server_id.to_vec());
		key.append(&mut server_info.pubkey.to_vec());
		batch.put_ser(&key, &server_info)?;
		// add ourselves as the server owner
		let user_pubkey = match user_pubkey {
			Some(user_pubkey) => user_pubkey,
			None => server_info.pubkey,
		};

		let roles = if remote {
			AUTH_FLAG_MEMBER
		} else {
			AUTH_FLAG_OWNER | AUTH_FLAG_MEMBER
		};

		//let profile = self.get_profile_impl(user_pubkey, user_pubkey, [0u8; 8], &batch)?;
		//let profile = Some(get_default_profile());
		//let profile = Some(ProfileData::default());

		/*
				let profile = match profile {
					Some(mut profile) => {
						profile.server_id = ServerId::from_bytes(server_id);
						profile.server_pubkey = Pubkey::from_bytes(server_info.pubkey);
						Some(profile)
					}
					None => None,
				};
		*/

		self.set_member(
			Pubkey::from_bytes(user_pubkey),
			ServerId::from_bytes(server_id),
			Pubkey::from_bytes(server_info.pubkey),
			roles,
			None,
			None,
			&batch,
		)?;

		// add the default channel
		let channel_id = rand::random();
		let channel_key = ChannelKey {
			server_pubkey: user_pubkey,
			server_id,
			channel_id,
		};
		let channel = Channel {
			name: "mainchat".to_string(),
			description: "Welcome to mainchat!".to_string(),
			channel_id,
		};
		self.set_channel_impl(channel_key, channel, &batch)?;

		batch.commit()?;
		Ok(server_id)
	}

	pub fn delete_server(&self, server_id: [u8; 8], pubkey: [u8; 32]) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut key = vec![SERVER_PREFIX];
		key.append(&mut server_id.to_vec());
		key.append(&mut pubkey.to_vec());
		batch.delete(&key)?;
		batch.commit()?;
		Ok(())
	}

	/*
		// add a remote server
		pub fn add_remote_server(
			&self,
			id: String,
			server_info: ServerInfo,
			channels: Vec<Channel>,
			members: MemberList,
		) -> Result<(), Error> {
			let members = members.members;
			{
				let batch = self.store.batch()?;
				let server_id = urlencoding::decode(&id)?;
				let server_id = base64::decode(&*server_id)?;
				let server_id = server_id.as_slice().try_into()?;

				for channel in channels {
					let channel_key = ChannelKey {
						channel_id: channel.channel_id,
						server_id,
						server_pubkey: server_info.pubkey,
					};
					let mut buffer = vec![];
					serialize_default(&mut buffer, &channel_key)?;
					let mut buffer2 = vec![CHANNEL_PREFIX];
					buffer2.append(&mut buffer);
					batch.put_ser(&buffer2, &channel)?;
				}

				for member in members {
					self.set_member(
						member.user_pubkey,
						server_id,
						server_info.pubkey,
						member.roles,
						Some(member.modified_time),
						Some(member.join_time),
						&batch,
					)?;
				}

				batch.commit()?;
			}

			//self.modify_server(id, server_info)
			Ok(())
		}

		// delete a remote server
		pub fn delete_remove_server(&self, _id: String) -> Result<(), Error> {
			//self.delete_server(id)
			Ok(())
		}
	*/

	// post the specified message to our local DB.
	pub fn post_message(&self, message: Message) -> Result<(), Error> {
		let batch = self.store.batch()?;

		let message_metadata_key = MessageMetaDataKey {
			server_pubkey: message.server_pubkey,
			server_id: message.server_id,
			channel_id: message.channel_id,
		};

		let mut buffer = vec![];
		serialize_default(&mut buffer, &message_metadata_key)?;
		let res: Option<MessageMetaDataValue> = batch.get_ser(&buffer)?;

		let message_count = match res {
			Some(mmdv) => mmdv.message_count,
			None => 0,
		};

		batch.put_ser(
			&buffer,
			&MessageMetaDataValue {
				message_count: message_count + 1,
			},
		)?;

		let message_value_impl = MessageValueImpl {
			payload: message.payload,
			signature: message.signature,
			message_type: message.message_type,
		};
		let message_key_impl = MessageKeyImpl {
			server_pubkey: message.server_pubkey,
			server_id: message.server_id,
			channel_id: message.channel_id,
			batch_num: message_count / MESSAGE_BATCH_SIZE,
			timestamp: message.timestamp,
			user_pubkey: message.user_pubkey,
			nonce: message.nonce,
		};
		let mut buffer = vec![];
		serialize_default(&mut buffer, &message_key_impl)?;
		batch.put_ser(&buffer, &message_value_impl)?;

		batch.commit()?;

		Ok(())
	}

	// get the messages from the message db. Messages are batched in groups of 100.
	// if the batch_num is greater than the highest batch num, the last batch (most recent)
	// messages are returned. The u64 returned is the number of batches that currently exist.
	pub fn get_messages(
		&self,
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
		channel_id: u64,
		batch_num: u64,
	) -> Result<(u64, Vec<Message>), Error> {
		let batch = self.store.batch()?;

		let message_metadata_key = MessageMetaDataKey {
			server_pubkey,
			server_id,
			channel_id,
		};

		let mut buffer = vec![];
		serialize_default(&mut buffer, &message_metadata_key)?;
		let res: Option<MessageMetaDataValue> = batch.get_ser(&buffer)?;
		match res {
			Some(mmdv) => {
				let message_count = mmdv.message_count;
				let batches = message_count / MESSAGE_BATCH_SIZE;
				let batch_num = if batches < batch_num {
					batches
				} else {
					batch_num
				};

				let mut prefix = vec![MESSAGE_PREFIX];
				prefix.append(&mut server_pubkey.to_vec());
				prefix.append(&mut server_id.to_vec());
				prefix.append(&mut channel_id.to_be_bytes().to_vec());
				prefix.append(&mut batch_num.to_be_bytes().to_vec());

				let mut message_num = batch_num * 100;

				let mut itt = batch.iter(&(prefix[..]), move |k, v| {
					let mut cursor = Cursor::new(k.to_vec());
					cursor.set_position(0);
					let mut reader = BinReader::new(&mut cursor, ProtocolVersion::local());
					let mkey = MessageKeyImpl::read(&mut reader)?;

					let mut cursor = Cursor::new(v.to_vec());
					cursor.set_position(0);
					let mut reader = BinReader::new(&mut cursor, ProtocolVersion::local());
					let mval = MessageValueImpl::read(&mut reader)?;

					Ok(Message {
						payload: mval.payload,
						signature: mval.signature,
						message_type: mval.message_type,
						server_pubkey: mkey.server_pubkey,
						server_id: mkey.server_id,
						channel_id: mkey.channel_id,
						timestamp: mkey.timestamp,
						user_pubkey: mkey.user_pubkey,
						user_name: "".to_string(),
						user_bio: "".to_string(),
						nonce: mkey.nonce,
						seqno: 0,
					})
				})?;

				let mut ret = vec![];
				loop {
					let next = itt.next();
					match next {
						Some(mut m) => {
							m.user_name = "not implemented".to_string();
							m.user_bio = "not implemented".to_string();
							m.seqno = message_num;
							message_num += 1;
							ret.push(m);
						}
						None => {
							break;
						}
					}
				}

				Ok((batches, ret))
			}
			None => Ok((0, vec![])),
		}
	}

	pub fn get_channels(
		&self,
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
	) -> Result<Vec<Channel>, Error> {
		let batch = self.store.batch()?;
		// get the iterator for each channel
		let mut key_vec = vec![CHANNEL_PREFIX];
		key_vec.append(&mut server_pubkey.to_vec());
		key_vec.append(&mut server_id.to_vec());

		let mut itt = batch.iter(&(key_vec[..]), |_, v| {
			let mut cursor = Cursor::new(v.to_vec());
			cursor.set_position(0);
			let mut reader = BinReader::new(&mut cursor, ProtocolVersion::local());
			let cval = Channel::read(&mut reader)?;
			Ok(cval)
		})?;

		let mut ret = vec![];
		loop {
			let next = itt.next();
			match next {
				Some(cval) => {
					ret.push(cval);
				}
				None => {
					break;
				}
			}
		}

		Ok(ret)
	}

	pub fn add_channel(
		&self,
		server_id: [u8; 8],
		server_pubkey: [u8; 32],
		name: String,
		description: String,
	) -> Result<u64, Error> {
		let channel_id = rand::random();
		let channel_key = ChannelKey {
			channel_id,
			server_id,
			server_pubkey,
		};
		let channel = Channel {
			name,
			description,
			channel_id,
		};
		self.set_channel(channel_key, channel)?;
		Ok(channel_id)
	}

	pub fn modify_channel(
		&self,
		server_id: [u8; 8],
		server_pubkey: [u8; 32],
		channel_id: u64,
		name: String,
		description: String,
	) -> Result<(), Error> {
		let channel_key = ChannelKey {
			channel_id,
			server_id,
			server_pubkey,
		};
		let channel = Channel {
			name,
			description,
			channel_id,
		};
		self.set_channel(channel_key, channel)?;
		Ok(())
	}

	fn set_channel(&self, channel_key: ChannelKey, channel: Channel) -> Result<(), Error> {
		let batch = self.store.batch()?;
		self.set_channel_impl(channel_key, channel, &batch)?;
		batch.commit()?;
		Ok(())
	}

	fn set_channel_impl(
		&self,
		channel_key: ChannelKey,
		channel: Channel,
		batch: &Batch,
	) -> Result<(), Error> {
		let mut buffer = vec![];
		serialize_default(&mut buffer, &channel_key)?;
		let mut buffer2 = vec![CHANNEL_PREFIX];
		buffer2.append(&mut buffer);
		batch.put_ser(&buffer2, &channel)?;
		Ok(())
	}

	pub fn delete_channel(
		&self,
		server_id: [u8; 8],
		server_pubkey: [u8; 32],
		channel_id: u64,
	) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let channel_key = ChannelKey {
			channel_id,
			server_id,
			server_pubkey,
		};
		self.delete_channel_impl(channel_key, &batch)?;

		batch.commit()?;
		Ok(())
	}

	fn delete_channel_impl(&self, channel_key: ChannelKey, batch: &Batch) -> Result<(), Error> {
		let mut buffer = vec![];
		serialize_default(&mut buffer, &channel_key)?;
		let mut buffer2 = vec![CHANNEL_PREFIX];
		buffer2.append(&mut buffer);
		let _ = batch.delete(&buffer2);
		Ok(())
	}

	pub fn create_invite(
		&self,
		inviter: [u8; 32],
		server_id: [u8; 8],
		expiry: u128,
		count: u64,
	) -> Result<u128, Error> {
		let batch = self.store.batch()?;

		let id: u128 = rand::random();

		let invite = Invite {
			inviter,
			server_id,
			expiry,
			cur: 0,
			max: count,
			id,
		};

		let invite_key = InviteKey {
			server_id,
			inviter,
			id,
		};

		let mut buffer = vec![];
		serialize_default(&mut buffer, &invite_key)?;
		let mut invite_key_buf = vec![INVITE_PREFIX];
		invite_key_buf.append(&mut buffer);

		serialize_default(&mut buffer, &invite)?;
		batch.put_ser(&invite_key_buf, &buffer)?;

		// create second index for invite id.
		let mut invite_id_key = vec![INVITE_ID_PREFIX];
		invite_id_key.append(&mut id.to_be_bytes().to_vec());
		batch.put_ser(&invite_id_key, &buffer)?;

		batch.commit()?;

		Ok(id)
	}

	pub fn get_invites(
		&self,
		inviter: Option<[u8; 32]>,
		server_id: [u8; 8],
	) -> Result<Vec<Invite>, Error> {
		let batch = self.store.batch()?;
		// get the iterator for each invite
		let mut key_vec = vec![INVITE_PREFIX];
		key_vec.append(&mut server_id.to_vec());
		// if inviter is specfied
		match inviter {
			Some(inviter) => {
				key_vec.append(&mut inviter.to_vec());
			}
			None => {}
		}

		let mut itt = batch.iter(&(key_vec[..]), |_, v| {
			let mut cursor = Cursor::new(v.to_vec());
			cursor.set_position(0);
			let mut reader = BinReader::new(&mut cursor, ProtocolVersion::local());
			let invite = Invite::read(&mut reader)?;
			Ok(invite)
		})?;

		let mut ret = vec![];
		loop {
			let next = itt.next();
			match next {
				Some(invite) => {
					ret.push(invite);
				}
				None => {
					break;
				}
			}
		}

		Ok(ret)
	}

	pub fn delete_invite(&self, invite_id: u128) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut key = vec![INVITE_ID_PREFIX];
		key.append(&mut invite_id.to_be_bytes().to_vec());
		let invite: Option<Invite> = batch.get_ser(&key)?;

		match invite {
			Some(invite) => {
				batch.delete(&key)?;
				let invite_key = InviteKey {
					server_id: invite.server_id,
					inviter: invite.inviter,
					id: invite_id,
				};

				let mut buffer = vec![];
				serialize_default(&mut buffer, &invite_key)?;
				let mut invite_key_buf = vec![INVITE_PREFIX];
				invite_key_buf.append(&mut buffer);
				batch.delete(&invite_key_buf)?;

				batch.commit()?;
			}
			None => {}
		}
		Ok(())
	}

	pub fn check_invite(
		&self,
		invite_id: u128,
		server_pubkey: [u8; 32],
	) -> Result<Option<JoinInfoReply>, Error> {
		error!(
			"checking id={}, be={:?}",
			invite_id,
			invite_id.to_be_bytes().to_vec()
		);

		let batch = self.store.batch()?;
		let mut key = vec![INVITE_ID_PREFIX];
		key.append(&mut invite_id.to_be_bytes().to_vec());
		let invite: Option<Invite> = batch.get_ser(&key)?;
		match invite {
			Some(invite) => {
				info!("found a match = {:?}", invite);
				match invite.cur >= invite.max {
					true => Ok(None), // accepted too many times
					false => {
						let mut key = vec![SERVER_PREFIX];
						key.append(&mut invite.server_id.to_vec());
						key.append(&mut server_pubkey.to_vec());
						let ret: Option<ServerInfo> = batch.get_ser(&key)?;
						info!(
							"ret on server info was {:?}, server_id={:?}",
							ret, invite.server_id
						);

						match ret {
							Some(ret) => Ok(Some(JoinInfoReply {
								server_pubkey: ret.pubkey,
								name: ret.name,
								server_id: invite.server_id,
								inviter_pubkey: invite.inviter,
							})),
							None => Ok(None),
						}
					}
				}
			}
			None => Ok(None),
		}
	}

	pub fn accept_invite(
		&self,
		invite_id: u128,
		user_pubkey: [u8; 32],
		server_pubkey: [u8; 32],
		user_name: String,
		user_bio: String,
		_avatar: Vec<u8>,
	) -> Result<Option<ServerInfoReply>, Error> {
		let batch = self.store.batch()?;
		let mut key = vec![INVITE_ID_PREFIX];
		key.append(&mut invite_id.to_be_bytes().to_vec());
		let invite: Option<Invite> = batch.get_ser(&key)?;
		match invite {
			Some(mut invite) => {
				if invite.cur >= invite.max {
					// this invite has been accepted too many times
					Ok(None)
				} else {
					// success, increment accept counter and write back
					invite.cur += 1;
					let mut buffer = vec![];
					serialize_default(&mut buffer, &invite)?;
					batch.put_ser(&key, &buffer)?;

					let user_name = user_name.into();
					let user_bio = user_bio.into();
					// build the profile
					self.set_profile_impl(
						Pubkey::from_bytes(user_pubkey),
						Pubkey::from_bytes(server_pubkey),
						ServerId::from_bytes(invite.server_id),
						ProfileData {
							user_name,
							user_bio,
						},
						&batch,
					)?;

					// add to member table
					self.set_member(
						Pubkey::from_bytes(user_pubkey),
						ServerId::from_bytes(invite.server_id),
						Pubkey::from_bytes(server_pubkey),
						AUTH_FLAG_MEMBER,
						None,
						None,
						&batch,
					)?;

					let mut key = vec![SERVER_PREFIX];
					key.append(&mut invite.server_id.to_vec());
					let ret: Option<ServerInfo> = batch.get_ser(&key)?;

					batch.commit()?;

					match ret {
						Some(ret) => Ok(Some(ServerInfoReply {
							pubkey: ret.pubkey,
							name: ret.name,
							server_id: invite.server_id,
							seqno: ret.seqno,
						})),
						None => Ok(None),
					}
				}
			}
			None => {
				// this is not a valid invite, reject
				Ok(None)
			}
		}
	}

	fn get_member(
		&self,
		user_pubkey: Pubkey,
		server_id: ServerId,
		server_pubkey: Pubkey,
		batch: &Batch,
	) -> Result<Option<MemberImpl>, Error> {
		let hash_key = MemberKeyHashImpl {
			server_pubkey: server_pubkey.clone(),
			server_id: server_id.clone(),
			user_pubkey: user_pubkey.clone(),
		};
		let mut hash_key_buffer = vec![];
		serialize_default(&mut hash_key_buffer, &hash_key)?;
		let member_value_impl: Option<MemberValueImpl> = batch.get_ser(&hash_key_buffer)?;

		match member_value_impl {
			Some(m) => Ok(Some(MemberImpl {
				server_pubkey,
				server_id,
				user_pubkey,
				roles: m.roles,
				join_time: m.join_time,
				modified_time: m.modified_time,
				batch_num: m.batch_num,
			})),
			None => Ok(None),
		}
	}

	/*
		fn update_roles(
			&self,
			user_pubkey: [u8; 32],
			server_id: [u8; 8],
			server_pubkey: [u8; 32],
			roles: u128,
			batch: &Batch,
		) -> Result<(), Error> {
			match self.get_member(user_pubkey, server_id, server_pubkey, false, batch)? {
				Some(member) => {
					self.set_member(
						user_pubkey,
						server_id,
						server_pubkey,
						roles,
						Some(member.modified_time),
						Some(member.join_time),
						batch,
					)?;
				}
				None => {
					self.set_member(
						user_pubkey,
						server_id,
						server_pubkey,
						roles,
						None,
						None,
						batch,
					)?;
				}
			}

			Ok(())
		}
	*/

	fn set_member(
		&self,
		user_pubkey: Pubkey,
		server_id: ServerId,
		server_pubkey: Pubkey,
		roles: u128,
		modified_time: Option<u64>,
		join_time: Option<u64>,
		batch: &Batch,
	) -> Result<Member, Error> {
		let time_now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_millis()
			.try_into()?;

		match self.get_member(user_pubkey, server_id, server_pubkey, batch)? {
			Some(member) => {
				warn!("member already joined the server");
				Ok(member.into())
			}
			None => {
				let member = Member {
					user_pubkey,
					profile_data: None,
					roles,
					join_time: join_time.unwrap_or(time_now),
					modified_time: modified_time.unwrap_or(time_now),
				};

				self.save_member(server_pubkey, server_id, &member, batch)?;
				Ok(member.into())
			}
		}
	}

	fn update_metadata(
		&self,
		server_id: ServerId,
		server_pubkey: Pubkey,
		batch: &Batch,
	) -> Result<u64, Error> {
		let mut member_meta_data_key = vec![];
		serialize_default(
			&mut member_meta_data_key,
			&MemberMetaDataKey {
				server_pubkey,
				server_id,
			},
		)?;

		let member_meta_data_value: Option<MemberMetaDataValue> =
			batch.get_ser(&member_meta_data_key)?;

		let batch_num = match member_meta_data_value {
			Some(meta_data) => {
				let member_count = meta_data.member_count;
				batch.put_ser(
					&member_meta_data_key,
					&MemberMetaDataValue {
						member_count: member_count + 1,
					},
				)?;
				member_count / MEMBER_BATCH_SIZE
			}
			None => {
				batch.put_ser(
					&member_meta_data_key,
					&MemberMetaDataValue { member_count: 1 },
				)?;
				0
			}
		};

		Ok(batch_num)
	}

	// note that this function does not save profile data associated with this member struct.
	// use save_profile to do that.
	fn save_member(
		&self,
		server_pubkey: Pubkey,
		server_id: ServerId,
		member: &Member,
		batch: &Batch,
	) -> Result<(), Error> {
		let batch_num = self.update_metadata(server_id, server_pubkey, batch)?;

		let member_impl = &MemberImpl {
			user_pubkey: member.user_pubkey.clone(),
			roles: member.roles,
			join_time: member.join_time,
			modified_time: member.modified_time,
			batch_num,
			server_id,
			server_pubkey,
		};

		// create key/value structs
		let member_key_hash: MemberKeyHashImpl = member_impl.into();
		let member_key_itt: MemberKeyIttImpl = member_impl.into();
		let member_key_auth: MemberKeyAuthImpl = member_impl.into();
		let member_value: MemberValueImpl = member_impl.into();

		// key vectors
		let mut member_key_hash_buffer = vec![];
		let mut member_key_itt_buffer = vec![];
		let mut member_key_auth_buffer = vec![];

		// serialize the keys
		serialize_default(&mut member_key_hash_buffer, &member_key_hash)?;
		serialize_default(&mut member_key_itt_buffer, &member_key_itt)?;
		serialize_default(&mut member_key_auth_buffer, &member_key_auth)?;

		// write the value for each key
		batch.put_ser(&member_key_hash_buffer, &member_value)?;

		// only want to add the user to one of the two tables
		// for now roles == 0 is member, anything else is auth table
		// meaning it will be listed first
		match member.roles == 0 {
			true => {
				batch.put_ser(&member_key_itt_buffer, &member_value)?;
				// have to remove incase of auth changes
				let _ = batch.delete(&member_key_auth_buffer);
			}
			false => {
				batch.put_ser(&member_key_auth_buffer, &member_value)?;
				// have to remove incase of auth changes
				let _ = batch.delete(&member_key_itt_buffer);
			}
		}

		Ok(())
	}

	pub fn get_members(
		&self,
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
		batch_num: u64,
		auth: bool, // if true send back users with auth privileges (moderators), otherwise members.
	) -> Result<Vec<Member>, Error> {
		let batch = self.store.batch()?;
		let mut ret = vec![];
		match auth {
			true => {
				let ablki = AuthBatchLookupKeyImpl {
					server_pubkey: Pubkey::from_bytes(server_pubkey),
					server_id: ServerId::from_bytes(server_id),
					batch_num,
				};
				let mut ablki_buffer = vec![];
				serialize_default(&mut ablki_buffer, &ablki)?;
				let mut itt = batch.iter(&(ablki_buffer[..]), |k, v| {
					let mut cursor = Cursor::new(k.to_vec());
					cursor.set_position(0);
					let mut reader = BinReader::new(&mut cursor, ProtocolVersion::local());
					let member_key = MemberKeyAuthImpl::read(&mut reader)?;

					let mut cursor = Cursor::new(v.to_vec());
					cursor.set_position(0);
					let mut reader = BinReader::new(&mut cursor, ProtocolVersion::local());
					let member_value = MemberValueImpl::read(&mut reader)?;

					let member = MemberImpl {
						user_pubkey: member_key.user_pubkey,
						server_pubkey: member_key.server_pubkey,
						server_id: member_key.server_id,
						batch_num: member_key.batch_num,
						roles: member_value.roles,
						join_time: member_value.join_time,
						modified_time: member_value.modified_time,
					};

					Ok(member)
				})?;

				loop {
					let next = itt.next();
					match next {
						Some(member) => ret.push(member),
						None => break,
					}
				}
			}
			false => {
				let mblki = MemberBatchLookupKeyImpl {
					server_pubkey: Pubkey::from_bytes(server_pubkey),
					server_id: ServerId::from_bytes(server_id),
					batch_num,
				};

				let mut mblki_buffer = vec![];
				serialize_default(&mut mblki_buffer, &mblki)?;
				let mut itt = batch.iter(&(mblki_buffer[..]), |k, v| {
					let mut cursor = Cursor::new(k.to_vec());
					cursor.set_position(0);
					let mut reader = BinReader::new(&mut cursor, ProtocolVersion::local());
					let member_key = MemberKeyIttImpl::read(&mut reader)?;

					let mut cursor = Cursor::new(v.to_vec());
					cursor.set_position(0);
					let mut reader = BinReader::new(&mut cursor, ProtocolVersion::local());
					let member_value = MemberValueImpl::read(&mut reader)?;

					let member = MemberImpl {
						user_pubkey: member_key.user_pubkey,
						server_pubkey: member_key.server_pubkey,
						server_id: member_key.server_id,
						batch_num: member_key.batch_num,
						roles: member_value.roles,
						join_time: member_value.join_time,
						modified_time: member_value.modified_time,
					};

					Ok(member)
				})?;

				loop {
					let next = itt.next();
					match next {
						Some(member) => ret.push(member),
						None => break,
					}
				}
			}
		}

		let mut members: Vec<Member> = vec![];
		let mut pubkeys = vec![];
		for member in ret {
			pubkeys.push(member.user_pubkey);
			members.push(member.into());
			// handle including profile here.
		}

		let profiles = self.get_profiles_impl(
			pubkeys,
			Pubkey::from_bytes(server_pubkey),
			ServerId::from_bytes(server_id),
			&batch,
		)?;

		let mut i = 0;
		for profile in profiles {
			members[i].profile_data = match profile {
				Some(profile) => Some(profile.profile_data),
				None => None,
			};
			i += 1;
		}

		Ok(members)
	}

	pub fn create_auth_challenge(&self, user_pubkey: [u8; 32]) -> Result<[u8; 8], Error> {
		let batch = self.store.batch()?;
		let challenge: [u8; 8] = rand::random();
		let mut key = vec![CHALLENGE_PREFIX];
		key.append(&mut user_pubkey.to_vec());
		let challenge_value = Challenge { challenge };
		batch.put_ser(&key, &challenge_value)?;
		batch.commit()?;
		Ok(challenge)
	}

	// validate the challenge and generate a token, store it, and return it.
	pub fn validate_challenge(
		&self,
		user_pubkey: [u8; 32],
		_server_pubkey: [u8; 32],
		challenge: [u8; 8],
		expiration_millis: u128,
		roles: u128,
	) -> Result<Option<String>, Error> {
		let batch = self.store.batch()?;
		let mut key = vec![CHALLENGE_PREFIX];
		key.append(&mut user_pubkey.to_vec());
		let challenge_stored: Option<Challenge> = batch.get_ser(&key)?;

		match challenge_stored {
			Some(challenge_stored) => {
				let challenge_stored = challenge_stored.challenge;
				for i in 0..challenge_stored.len() {
					if challenge_stored[i] != challenge[i] {
						return Ok(None);
					}
				}

				// generate and store token
				let token: u128 = rand::random();
				let creation_time = std::time::SystemTime::now()
					.duration_since(std::time::UNIX_EPOCH)?
					.as_millis();
				let auth_info = AuthInfo {
					creation_time,
					last_access_time: creation_time,
					expiration_millis,
				};
				let mut auth_key = vec![TOKEN_PREFIX];
				auth_key.append(&mut user_pubkey.to_vec());
				auth_key.append(&mut token.to_be_bytes().to_vec());
				batch.put_ser(&auth_key, &auth_info)?;

				// needed for local access
				if (roles & AUTH_FLAG_OWNER) != 0 {
					//self.update_roles(user_pubkey, [0u8; 8], server_pubkey, roles, &batch)?;
				}

				batch.commit()?;

				Ok(Some(format!("{}", token)))
			}
			None => Ok(None),
		}
	}

	// get auth info
	pub fn is_authorized(
		&self,
		user_pubkey: [u8; 32],
		server_pubkey: [u8; 32],
		token: u128,
		server_id: [u8; 8],
		requested_roles: u128,
	) -> Result<(), Error> {
		let user_pubkey = Pubkey::from_bytes(user_pubkey);
		let server_pubkey = Pubkey::from_bytes(server_pubkey);
		let server_id = ServerId::from_bytes(server_id);

		let batch = self.store.batch()?;

		let mut auth_key = vec![TOKEN_PREFIX];
		auth_key.append(&mut user_pubkey.to_bytes().to_vec());
		auth_key.append(&mut token.to_be_bytes().to_vec());

		let auth_info: Option<AuthInfo> = batch.get_ser(&auth_key)?;

		match auth_info {
			Some(auth_info) => {
				let time_now = std::time::SystemTime::now()
					.duration_since(std::time::UNIX_EPOCH)?
					.as_millis();
				if time_now - auth_info.last_access_time > auth_info.expiration_millis {
					Err(ErrorKind::NotAuthorized("invalid token - expired".to_string()).into())
				} else {
					let member = self.get_member(user_pubkey, server_id, server_pubkey, &batch)?;

					match member {
						Some(member) => {
							if (member.roles & requested_roles) != 0 {
								Ok(())
							} else {
								Err(ErrorKind::NotAuthorized(format!(
									"not authorized. requested = {}, actual = {}",
									requested_roles, member.roles,
								))
								.into())
							}
						}
						None => Err(ErrorKind::NotAuthorized(format!(
							"not authorized. requested = {}, level = {}",
							requested_roles, 0
						))
						.into()),
					}
				}
			}
			None => Err(ErrorKind::NotAuthorized("invalid token - not found".to_string()).into()),
		}
	}

	// purge any expired tokens
	pub fn purge_tokens(&self) -> Result<(), Error> {
		let time_now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_millis();

		let batch = self.store.batch()?;
		// get the iterator for each server info
		let mut itt = batch.iter(&(vec![TOKEN_PREFIX])[..], |k, v| {
			let mut cursor = Cursor::new(v.to_vec());
			cursor.set_position(0);
			let mut reader = BinReader::new(&mut cursor, ProtocolVersion::local());
			Ok((k.to_vec(), AuthInfo::read(&mut reader)?))
		})?;

		loop {
			match itt.next() {
				Some((k, auth_info)) => {
					if time_now - auth_info.last_access_time > auth_info.expiration_millis {
						batch.delete(&k)?;
					}
				}
				None => break,
			}
		}

		batch.commit()?;

		Ok(())
	}

	// save an auth token
	pub fn save_auth_token(&self, server_pubkey: [u8; 32], auth_token: u128) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut key = vec![STORED_AUTH_TOKEN_PREFIX];
		key.append(&mut server_pubkey.to_vec());
		batch.put_ser(&key, &AuthToken { auth_token })?;
		batch.commit()?;

		Ok(())
	}

	/*
		// get an auth token
		pub fn get_auth_token(&self, server_pubkey: [u8; 32]) -> Result<u128, Error> {
			let batch = self.store.batch()?;
			let mut key = vec![STORED_AUTH_TOKEN_PREFIX];
			key.append(&mut server_pubkey.to_vec());
			let auth_token: Option<AuthToken> = batch.get_ser(&key)?;
			Ok(match auth_token {
				Some(auth_token) => auth_token.auth_token,
				None => 0,
			})
		}
	*/

	pub fn get_profiles(
		&self,
		user_pubkeys: Vec<Pubkey>,
		server_pubkey: Pubkey,
		server_id: ServerId,
	) -> Result<Vec<Option<Profile>>, Error> {
		let batch = self.store.batch()?;
		let res = self.get_profiles_impl(user_pubkeys, server_pubkey, server_id, &batch)?;
		batch.commit()?;
		Ok(res)
	}

	pub fn set_profile(
		&self,
		user_pubkey: Pubkey,
		server_pubkey: Pubkey,
		server_id: ServerId,
		profile_data: ProfileData,
	) -> Result<(), Error> {
		let batch = self.store.batch()?;
		self.set_profile_impl(user_pubkey, server_pubkey, server_id, profile_data, &batch)?;
		batch.commit()?;
		Ok(())
	}

	fn get_profiles_impl(
		&self,
		user_pubkeys: Vec<Pubkey>,
		server_pubkey: Pubkey,
		server_id: ServerId,
		batch: &Batch,
	) -> Result<Vec<Option<Profile>>, Error> {
		let mut ret = vec![];
		for user_pubkey in user_pubkeys {
			let profile_key = ProfileKey {
				user_pubkey: user_pubkey.clone(),
				server_pubkey: server_pubkey.clone(),
				server_id: server_id.clone(),
			};
			let mut profile_key_buffer = vec![];
			serialize_default(&mut profile_key_buffer, &profile_key)?;
			let profile_value: Option<ProfileData> = batch.get_ser(&profile_key_buffer)?;
			match profile_value {
				Some(profile_data) => {
					ret.push(Some(Profile {
						user_pubkey: user_pubkey.clone(),
						server_pubkey: server_pubkey.clone(),
						server_id: server_id.clone(),
						profile_data,
					}));
				}
				None => ret.push(None),
			}
		}
		Ok(ret)
	}

	fn set_profile_impl(
		&self,
		user_pubkey: Pubkey,
		server_pubkey: Pubkey,
		server_id: ServerId,
		profile_data: ProfileData,
		batch: &Batch,
	) -> Result<(), Error> {
		info!(
			"setting profile: user_pubkey={:?},server_pubkey={:?},server_id={:?},profile_data={:?}",
			user_pubkey, server_pubkey, server_id, profile_data
		);
		let profile_key = ProfileKey {
			user_pubkey,
			server_pubkey,
			server_id,
		};
		let mut profile_key_buffer = vec![];
		serialize_default(&mut profile_key_buffer, &profile_key)?;
		let mut profile_data_buffer = vec![];
		serialize_default(&mut profile_data_buffer, &profile_data)?;
		batch.put_ser(&profile_key_buffer, &profile_data_buffer)?;
		Ok(())
	}

	/*pub fn get_profile(
		&self,
		user_pubkey: [u8; 32],
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
	) -> Result<Option<Profile>, Error> {
		let batch = self.store.batch()?;
		self.get_profile_impl(user_pubkey, server_pubkey, server_id, &batch)
	}*/

	/*fn get_profile_impl(
		&self,
		user_pubkey: [u8; 32],
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
		batch: &Batch,
	) -> Result<Option<Profile>, Error> {
		let mut profile_key_buffer = vec![];
		serialize_default(
			&mut profile_key_buffer,
			&ProfileKey {
				user_pubkey,
				server_pubkey,
				server_id,
			},
		)?;
		let profile_value: Option<ProfileData> = batch.get_ser(&profile_key_buffer)?;

		match profile_value {
			Some(profile_value) => Ok(Some(Profile {
				user_pubkey,
				server_pubkey,
				server_id,
				avatar: profile_value.avatar,
				profile_data: profile_value.profile_data,
			})),
			None => Ok(None),
		}
	}*/

	/*fn save_profile_impl(&self, profile: Profile, batch: &Batch) -> Result<(), Error> {
		let profile_key: ProfileKey = profile.clone().into();
		let profile_value: ProfileData = profile.into();
		let mut profile_key_buffer = vec![];
		serialize_default(&mut profile_key_buffer, &profile_key)?;
		batch.put_ser(&profile_key_buffer, &profile_value)?;

		Ok(())
	}*/

	/*pub fn save_profile(&self, profile: Profile) -> Result<(), Error> {
		let batch = self.store.batch()?;
		self.save_profile_impl(profile, &batch)?;
		batch.commit()?;
		Ok(())
	}*/

	/*pub fn set_profile_image(
		&self,
		user_pubkey: [u8; 32],
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
		avatar: Vec<u8>,
	) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut profile_key_buffer = vec![];

		serialize_default(
			&mut profile_key_buffer,
			&ProfileKey {
				user_pubkey,
				server_pubkey,
				server_id,
			},
		)?;
		let profile_value: Option<ProfileData> = batch.get_ser(&profile_key_buffer)?;
		match profile_value {
			Some(mut profile_value) => {
				profile_value.avatar = avatar;
				batch.put_ser(&profile_key_buffer, &profile_value)?;
				batch.commit()?;
			}
			None => {
				let mut profile = get_default_profile();
				profile.user_pubkey = user_pubkey;
				profile.server_pubkey = server_pubkey;
				profile.server_id = server_id;
				profile.avatar = avatar;
				let profile_value: ProfileData = profile.into();

				batch.put_ser(&profile_key_buffer, &profile_value)?;
				batch.commit()?;
			}
		}

		Ok(())
	}*/

	/*pub fn get_profile_images(
		&self,
		user_pubkeys: Vec<[u8; 32]>,
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
	) -> Result<Vec<Option<Vec<u8>>>, Error> {
		let batch = self.store.batch()?;
		let mut ret = vec![];

		for user_pubkey in user_pubkeys {
			let mut profile_key_buffer = vec![];
			serialize_default(
				&mut profile_key_buffer,
				&ProfileKey {
					user_pubkey,
					server_pubkey,
					server_id,
				},
			)?;
			let profile_value: Option<ProfileData> = batch.get_ser(&profile_key_buffer)?;
			match profile_value {
				Some(profile_value) => {
					ret.push(Some(profile_value.avatar));
				}
				None => {
					ret.push(None);
				}
			}
		}

		Ok(ret)
	}*/

	/*pub fn get_profile_data(
		&self,
		user_pubkeys: Vec<[u8; 32]>,
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
	) -> Result<Vec<Option<ProfileData>>, Error> {
		let batch = self.store.batch()?;
		let mut ret = vec![];

		for user_pubkey in user_pubkeys {
			let mut profile_key_buffer = vec![];
			serialize_default(
				&mut profile_key_buffer,
				&ProfileKey {
					user_pubkey,
					server_pubkey,
					server_id,
				},
			)?;

			let profile_value: Option<ProfileData> = batch.get_ser(&profile_key_buffer)?;
			match profile_value {
				Some(profile_value) => {
					ret.push(Some(ProfileData {
						user_name: profile_value.profile_data.user_name,
						user_bio: profile_value.profile_data.user_bio,
					}));
				}
				None => {
					ret.push(None);
				}
			}
		}

		Ok(ret)
	}*/

	/*pub fn set_profile_data(
		&self,
		user_pubkey: [u8; 32],
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
		profile_data: ProfileData,
	) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut profile_key_buffer = vec![];

		serialize_default(
			&mut profile_key_buffer,
			&ProfileKey {
				user_pubkey,
				server_pubkey,
				server_id,
			},
		)?;
		let profile_value: Option<ProfileData> = batch.get_ser(&profile_key_buffer)?;
		match profile_value {
			Some(mut profile_value) => {
				profile_value.profile_data = profile_data;
				batch.put_ser(&profile_key_buffer, &profile_value)?;
				batch.commit()?;
				Ok(())
			}
			None => Err(ErrorKind::ProfileNotFoundErr("profile not found".to_string()).into()),
		}
	}*/

	pub fn save_ws_auth_token(&self, token: u128) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut key_buffer = vec![];
		serialize_default(&mut key_buffer, &WSAuthToken { token })?;
		batch.put_ser(&key_buffer, &0u8)?;
		batch.commit()?;
		Ok(())
	}

	pub fn check_ws_auth_token(&self, token: u128) -> Result<bool, Error> {
		let batch = self.store.batch()?;
		let mut key_buffer = vec![];
		serialize_default(&mut key_buffer, &WSAuthToken { token })?;
		let v: Option<u8> = batch.get_ser(&key_buffer)?;
		Ok(v.is_some())
	}

	// create a dscontext instance
	pub fn new(db_root: String) -> Result<DSContext, Error> {
		let home_dir = match dirs::home_dir() {
			Some(p) => p,
			None => PathBuf::new(),
		}
		.as_path()
		.display()
		.to_string();
		let db_root = db_root.replace("~", &home_dir);
		fsutils::mkdir(&db_root);
		let store = Store::new(&db_root, None, Some(DB_NAME), None, true)?;
		Ok(DSContext { store })
	}
}
