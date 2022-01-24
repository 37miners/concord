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

use crate::lmdb::Store;
use crate::ser::serialize_default;
use crate::ser::{BinReader, ProtocolVersion, Readable, Reader, Writeable, Writer};
use concorderror::{Error, ErrorKind};
use nioruntime_log::*;

use std::convert::TryInto;
use std::io::Cursor;
use std::path::PathBuf;

const DB_NAME: &str = "concord";
const BATCH_SIZE: u64 = 100;

pub const TOKEN_EXPIRATION: u128 = 1000 * 60 * 60;

nioruntime_log::debug!();

pub fn get_default_profile() -> Profile {
	Profile {
		avatar: vec![],
		profile_data: ProfileData {
			user_name: "User Default".to_string(),
			user_bio: "Tell us about you..".to_string(),
		},
		server_id: [0u8; 8],
		server_pubkey: [0u8; 32],
		user_pubkey: [0u8; 32],
	}
}

// the context to use for accessing concord data. Multiple instances
// may exist and LMDB handles concurrency.
pub struct DSContext {
	store: Store,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileData {
	pub user_name: String,
	pub user_bio: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Profile {
	pub user_pubkey: [u8; 32],
	pub server_pubkey: [u8; 32],
	pub server_id: [u8; 8],
	pub avatar: Vec<u8>,
	pub profile_data: ProfileData,
}

struct ProfileKey {
	user_pubkey: [u8; 32],
	server_pubkey: [u8; 32],
	server_id: [u8; 8],
}

struct ProfileValue {
	avatar: Vec<u8>,
	profile_data: ProfileData,
}

impl From<Profile> for ProfileKey {
	fn from(profile: Profile) -> ProfileKey {
		ProfileKey {
			user_pubkey: profile.user_pubkey,
			server_pubkey: profile.server_pubkey,
			server_id: profile.server_id,
		}
	}
}

impl Writeable for ProfileKey {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(PROFILE_PREFIX)?;
		for i in 0..32 {
			writer.write_u8(self.user_pubkey[i])?;
		}
		for i in 0..32 {
			writer.write_u8(self.server_pubkey[i])?;
		}
		for i in 0..8 {
			writer.write_u8(self.server_id[i])?;
		}
		Ok(())
	}
}

impl Readable for ProfileKey {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut user_pubkey = [0u8; 32];
		let mut server_pubkey = [0u8; 32];
		let mut server_id = [0u8; 8];
		let _ = reader.read_u8()?;
		for i in 0..32 {
			user_pubkey[i] = reader.read_u8()?;
		}
		for i in 0..32 {
			server_pubkey[i] = reader.read_u8()?;
		}
		for i in 0..8 {
			server_id[i] = reader.read_u8()?;
		}
		Ok(ProfileKey {
			user_pubkey,
			server_pubkey,
			server_id,
		})
	}
}

impl From<Profile> for ProfileValue {
	fn from(profile: Profile) -> ProfileValue {
		ProfileValue {
			profile_data: profile.profile_data,
			avatar: profile.avatar.clone(),
		}
	}
}

impl Writeable for ProfileValue {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let avatar_len = self.avatar.len();
		writer.write_u64(avatar_len.try_into()?)?;
		for i in 0..avatar_len {
			writer.write_u8(self.avatar[i])?;
		}

		Writeable::write(&self.profile_data, writer)?;

		Ok(())
	}
}

impl Readable for ProfileValue {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut avatar = vec![];
		let avatar_len = reader.read_u64()?;
		for _ in 0..avatar_len {
			avatar.push(reader.read_u8()?);
		}
		let profile_data = ProfileData::read(reader)?;

		Ok(ProfileValue {
			avatar,
			profile_data,
		})
	}
}

impl Writeable for ProfileData {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let name_len = self.user_name.len();
		writer.write_u64(name_len.try_into()?)?;
		let name_bytes = self.user_name.as_bytes();
		for i in 0..name_len {
			writer.write_u8(name_bytes[i])?;
		}
		let bio_len = self.user_bio.len();
		writer.write_u64(bio_len.try_into()?)?;
		let bio_bytes = self.user_bio.as_bytes();
		for i in 0..bio_len {
			writer.write_u8(bio_bytes[i])?;
		}
		Ok(())
	}
}

impl Readable for ProfileData {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let name_len = reader.read_u64()?;
		let mut user_name = vec![];
		for _ in 0..name_len {
			user_name.push(reader.read_u8()?);
		}
		let user_name = std::str::from_utf8(&user_name)?.to_string();
		let bio_len = reader.read_u64()?;
		let mut user_bio = vec![];
		for _ in 0..bio_len {
			user_bio.push(reader.read_u8()?);
		}
		let user_bio = std::str::from_utf8(&user_bio)?.to_string();
		Ok(ProfileData {
			user_name,
			user_bio,
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

#[derive(Serialize, Debug)]
pub struct Invite {
	pub inviter: [u8; 32],
	pub server_id: [u8; 8],
	pub expiry: u64,
	pub cur: u64,
	pub max: u64,
	pub id: u128,
}

impl Writeable for Invite {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..8 {
			writer.write_u8(self.server_id[i])?;
		}
		for i in 0..32 {
			writer.write_u8(self.inviter[i])?;
		}
		writer.write_u64(self.expiry)?;
		writer.write_u64(self.cur)?;
		writer.write_u64(self.max)?;
		writer.write_u128(self.id)?;

		Ok(())
	}
}

impl Readable for Invite {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut inviter = [0u8; 32];
		let mut server_id = [0u8; 8];

		for i in 0..8 {
			server_id[i] = reader.read_u8()?;
		}

		for i in 0..32 {
			inviter[i] = reader.read_u8()?;
		}

		let expiry = reader.read_u64()?;
		let cur = reader.read_u64()?;
		let max = reader.read_u64()?;
		let id = reader.read_u128()?;

		Ok(Invite {
			server_id,
			inviter,
			expiry,
			cur,
			max,
			id,
		})
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

struct MinMember {
	user_pubkey: [u8; 32],
	auth_flags: u64,
}

impl Writeable for MinMember {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..32 {
			writer.write_u8(self.user_pubkey[i])?;
		}
		writer.write_u64(self.auth_flags)?;

		Ok(())
	}
}

impl Readable for MinMember {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut user_pubkey = [0u8; 32];

		for i in 0..32 {
			user_pubkey[i] = reader.read_u8()?;
		}
		let auth_flags = reader.read_u64()?;

		Ok(MinMember {
			user_pubkey,
			auth_flags,
		})
	}
}

pub struct MemberList {
	pub member_data: Vec<u8>,
}

impl MemberList {
	pub fn read_member_list(&self) -> Result<Vec<(Member, u64)>, Error> {
		let mut cursor = Cursor::new(&self.member_data);
		cursor.set_position(0);
		let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
		let mut server_pubkey = [0u8; 32];
		let mut server_id = [0u8; 8];

		for i in 0..32 {
			server_pubkey[i] = reader.read_u8()?;
		}

		for i in 0..8 {
			server_id[i] = reader.read_u8()?;
		}

		let mut ret = vec![];
		loop {
			let next = MinMember::read(&mut reader);

			match next {
				Ok(next) => ret.push((
					Member {
						server_pubkey,
						server_id,
						user_pubkey: next.user_pubkey,
					},
					next.auth_flags,
				)),
				Err(_) => {
					break;
				}
			}
		}

		Ok(ret)
	}

	pub fn new(
		member_list: Vec<(Member, u64)>,
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
	) -> Result<Self, Error> {
		let mut member_data = vec![];
		member_data.append(&mut server_pubkey.to_vec());
		member_data.append(&mut server_id.to_vec());

		for (member, auth_flags) in member_list {
			let mut buffer = vec![];
			let min_member = MinMember {
				user_pubkey: member.user_pubkey,
				auth_flags,
			};
			serialize_default(&mut buffer, &min_member)?;
			member_data.append(&mut buffer);
		}

		Ok(MemberList { member_data })
	}

	pub fn from_b58(data: String) -> Result<Self, Error> {
		let member_data = bs58::decode(data).into_vec()?;
		Ok(MemberList { member_data })
	}

	pub fn to_b58(&self) -> Result<String, Error> {
		Ok(bs58::encode(&self.member_data).into_string())
	}
}

#[derive(Serialize, Debug, Clone)]
pub struct Member {
	pub server_id: [u8; 8],
	pub user_pubkey: [u8; 32],
	pub server_pubkey: [u8; 32],
}

impl Writeable for Member {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		for i in 0..8 {
			writer.write_u8(self.server_id[i])?;
		}
		for i in 0..32 {
			writer.write_u8(self.user_pubkey[i])?;
		}
		for i in 0..32 {
			writer.write_u8(self.server_pubkey[i])?;
		}

		Ok(())
	}
}

impl Readable for Member {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let mut user_pubkey = [0u8; 32];
		let mut server_pubkey = [0u8; 32];
		let mut server_id = [0u8; 8];

		for i in 0..8 {
			server_id[i] = reader.read_u8()?;
		}
		for i in 0..32 {
			user_pubkey[i] = reader.read_u8()?;
		}
		for i in 0..32 {
			server_pubkey[i] = reader.read_u8()?;
		}

		let member = Member {
			user_pubkey,
			server_id,
			server_pubkey,
		};

		Ok(member)
	}
}

// type of message
#[derive(Debug)]
pub enum MessageType {
	Text,
	Binary,
}

// information associated with a message
#[derive(Debug)]
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
	pub icon: Vec<u8>,
	pub inviter_pubkey: [u8; 32],
}

// information about the server
#[derive(Debug)]
pub struct ServerInfo {
	pub pubkey: [u8; 32],
	pub name: String,
	pub icon: Vec<u8>,
	pub joined: bool,
}

#[derive(Debug)]
pub struct ServerInfoReply {
	pub pubkey: [u8; 32],
	pub server_id: [u8; 8],
	pub name: String,
	pub icon: Vec<u8>,
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

		writer.write_u32(self.icon.len().try_into().unwrap_or(0))?;
		for b in &self.icon {
			writer.write_u8(*b)?;
		}

		match self.joined {
			false => writer.write_u8(0)?,
			true => writer.write_u8(1)?,
		}

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

		let icon_len = reader.read_u32()?;
		let mut icon = vec![];
		for _ in 0..icon_len {
			icon.push(reader.read_u8()?);
		}

		let name = std::str::from_utf8(&name)?;
		let name = name.to_string();

		let joined = reader.read_u8()? != 0;

		Ok(ServerInfo {
			pubkey,
			name,
			icon,
			joined,
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
const MEMBER_PREFIX: u8 = 4;
const INVITE_PREFIX: u8 = 5;
const INVITE_ID_PREFIX: u8 = 6;
const CHALLENGE_PREFIX: u8 = 7;
const STORED_AUTH_TOKEN_PREFIX: u8 = 8;
const MESSAGE_METADATA_PREFIX: u8 = 9;
const PROFILE_PREFIX: u8 = 10;

// auth levels
pub const AUTH_FLAG_OWNER: u64 = 1;
pub const AUTH_FLAG_MEMBER: u64 = 1 << 1;

impl DSContext {
	// get a list of servers in the local database
	pub fn get_servers(&self) -> Result<Vec<ServerInfoReply>, Error> {
		let batch = self.store.batch()?;
		// get the iterator for each server info
		let mut itt = batch.iter(&(vec![SERVER_PREFIX])[..], |k, v| {
			let id = base64::encode(&k[1..]);
			let mut cursor = Cursor::new(v.to_vec());
			cursor.set_position(0);
			let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
			Ok((ServerInfo::read(&mut reader)?, id.clone()))
		})?;

		let mut ret = vec![];
		loop {
			match itt.next() {
				Some((server, server_id)) => {
					let server_id = base64::decode(server_id)?.as_slice().try_into()?;
					if server.joined {
						ret.push(ServerInfoReply {
							server_id,
							name: server.name,
							pubkey: server.pubkey,
							icon: server.icon,
						});
					}
				}
				None => break,
			}
		}

		Ok(ret)
	}

	// get server info about a specific server id
	pub fn get_server_info(&self, server_id: String) -> Result<Option<ServerInfoReply>, Error> {
		let batch = self.store.batch()?;
		let server_id = urlencoding::decode(&server_id)?;
		let server_id = base64::decode(&*server_id)?;

		let mut key = vec![SERVER_PREFIX];
		key.append(&mut server_id.clone());
		let ret: Option<ServerInfo> = batch.get_ser(&key)?;
		match ret {
			None => Ok(None),
			Some(ret) => Ok(Some(ServerInfoReply {
				server_id: server_id.as_slice().try_into()?,
				icon: ret.icon,
				pubkey: ret.pubkey,
				name: ret.name,
			})),
		}
	}

	// add a server
	pub fn add_server(
		&self,
		server_info: ServerInfo,
		server_id: Option<[u8; 8]>,
		user_pubkey: Option<[u8; 32]>,
	) -> Result<[u8; 8], Error> {
		let batch = self.store.batch()?;
		let mut key = vec![SERVER_PREFIX];
		let server_id: [u8; 8] = match server_id {
			Some(server_id) => server_id,
			None => rand::random(),
		};
		key.append(&mut server_id.to_vec());
		batch.put_ser(&key, &server_info)?;

		// add ourselves as the server owner
		let user_pubkey = match user_pubkey {
			Some(user_pubkey) => user_pubkey,
			None => server_info.pubkey,
		};
		let member = Member {
			user_pubkey,
			server_pubkey: server_info.pubkey,
			server_id,
		};
		let mut buffer = vec![];
		serialize_default(&mut buffer, &member)?;
		let mut buffer2 = vec![MEMBER_PREFIX];
		buffer2.append(&mut buffer);
		batch.put_ser(&buffer2, &(AUTH_FLAG_OWNER | AUTH_FLAG_MEMBER))?;

		batch.commit()?;
		Ok(server_id)
	}

	// delete a server
	pub fn delete_server(&self, id: String) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut key = vec![SERVER_PREFIX];
		let id = urlencoding::decode(&id)?;
		let mut id = base64::decode(&*id)?;
		key.append(&mut id);
		batch.delete(&key)?;
		batch.commit()?;
		Ok(())
	}

	// modify a server
	pub fn modify_server(&self, id: String, server_info: ServerInfo) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut key = vec![SERVER_PREFIX];
		let id = urlencoding::decode(&id)?;
		let mut id = base64::decode(&*id)?;
		key.append(&mut id);
		batch.put_ser(&key, &server_info)?;
		batch.commit()?;
		Ok(())
	}

	// add a remote server
	pub fn add_remote_server(
		&self,
		id: String,
		server_info: ServerInfo,
		channels: Vec<Channel>,
		members: Vec<(Member, u64)>,
	) -> Result<(), Error> {
		// add ourselves as a member
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

			for (member, auth_flags) in members {
				let mut buffer = vec![];
				serialize_default(&mut buffer, &member)?;
				let mut buffer2 = vec![MEMBER_PREFIX];
				buffer2.append(&mut buffer);
				batch.put_ser(&buffer2, &auth_flags)?;
			}

			batch.commit()?;
		}

		self.modify_server(id, server_info)
	}

	// delete a remote server
	pub fn delete_remove_server(&self, id: String) -> Result<(), Error> {
		self.delete_server(id)
	}

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
			batch_num: message_count / BATCH_SIZE,
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
				let batches = message_count / BATCH_SIZE;
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
					let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
					let mkey = MessageKeyImpl::read(&mut reader)?;

					let mut cursor = Cursor::new(v.to_vec());
					cursor.set_position(0);
					let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
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
						nonce: mkey.nonce,
						seqno: 0,
					})
				})?;

				let mut ret = vec![];
				loop {
					let next = itt.next();
					match next {
						Some(mut m) => {
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
			let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
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

	pub fn set_channel(&self, channel_key: ChannelKey, channel: Channel) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut buffer = vec![];
		serialize_default(&mut buffer, &channel_key)?;
		let mut buffer2 = vec![CHANNEL_PREFIX];
		buffer2.append(&mut buffer);
		batch.put_ser(&buffer2, &channel)?;
		batch.commit()?;
		Ok(())
	}

	pub fn delete_channel(&self, channel_key: ChannelKey) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut buffer = vec![];
		serialize_default(&mut buffer, &channel_key)?;
		let mut buffer2 = vec![CHANNEL_PREFIX];
		buffer2.append(&mut buffer);

		// this throws an error if the item is not found. Don't think that's correct.
		// ignore this error
		let _ = batch.delete(&buffer2);
		batch.commit()?;
		Ok(())
	}

	pub fn revoke(&self, user_pubkey: [u8; 32], server_id: [u8; 8]) -> Result<(), Error> {
		let batch = self.store.batch()?;

		let member = Member {
			server_pubkey: user_pubkey,
			user_pubkey,
			server_id,
		};

		let mut buffer = vec![];
		serialize_default(&mut buffer, &member)?;
		let mut buffer2 = vec![MEMBER_PREFIX];
		buffer2.append(&mut buffer);
		batch.delete(&buffer2)?;

		batch.commit()?;
		Ok(())
	}

	pub fn check(&self, user_pubkey: [u8; 32], server_id: [u8; 8]) -> Result<bool, Error> {
		let batch = self.store.batch()?;

		let member = Member {
			server_pubkey: user_pubkey,
			user_pubkey,
			server_id,
		};

		let mut buffer = vec![];
		serialize_default(&mut buffer, &member)?;
		let mut buffer2 = vec![MEMBER_PREFIX];
		buffer2.append(&mut buffer);
		let res: Option<u8> = batch.get_ser(&buffer2)?;
		Ok(res.is_some())
	}

	pub fn create_invite(
		&self,
		inviter: [u8; 32],
		server_id: [u8; 8],
		expiry: u64,
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
			let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
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

	pub fn check_invite(&self, invite_id: u128) -> Result<Option<JoinInfoReply>, Error> {
		let batch = self.store.batch()?;
		let mut key = vec![INVITE_ID_PREFIX];
		key.append(&mut invite_id.to_be_bytes().to_vec());
		let invite: Option<Invite> = batch.get_ser(&key)?;
		match invite {
			Some(invite) => {
				match invite.cur >= invite.max {
					true => Ok(None), // accepted too many times
					false => {
						let mut key = vec![SERVER_PREFIX];
						key.append(&mut invite.server_id.to_vec());
						let ret: Option<ServerInfo> = batch.get_ser(&key)?;

						match ret {
							Some(ret) => Ok(Some(JoinInfoReply {
								server_pubkey: ret.pubkey,
								icon: ret.icon,
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
	) -> Result<Option<ServerInfoReply>, Error> {
		let batch = self.store.batch()?;
		let mut key = vec![INVITE_ID_PREFIX];
		key.append(&mut invite_id.to_be_bytes().to_vec());
		let invite: Option<Invite> = batch.get_ser(&key)?;
		match invite {
			Some(mut invite) => {
				error!("accept invite: {:?}", invite);
				if invite.cur >= invite.max {
					// this invite has been accepted too many times
					Ok(None)
				} else {
					// success, increment accept counter and write back
					invite.cur += 1;
					let mut buffer = vec![];
					serialize_default(&mut buffer, &invite)?;
					batch.put_ser(&key, &buffer)?;

					// add to member table
					let member = Member {
						server_pubkey,
						user_pubkey,
						server_id: invite.server_id,
					};

					let mut buffer = vec![];
					serialize_default(&mut buffer, &member)?;
					let mut buffer2 = vec![MEMBER_PREFIX];
					buffer2.append(&mut buffer);
					batch.put_ser(&buffer2, &AUTH_FLAG_MEMBER)?;

					let mut key = vec![SERVER_PREFIX];
					key.append(&mut invite.server_id.to_vec());
					let ret: Option<ServerInfo> = batch.get_ser(&key)?;

					batch.commit()?;

					match ret {
						Some(ret) => Ok(Some(ServerInfoReply {
							pubkey: ret.pubkey,
							icon: ret.icon,
							name: ret.name,
							server_id: invite.server_id,
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

	pub fn get_members(&self, server_id: [u8; 8]) -> Result<Vec<(Member, u64)>, Error> {
		let batch = self.store.batch()?;
		let mut key = vec![MEMBER_PREFIX];
		key.append(&mut server_id.to_vec());

		let mut itt = batch.iter(&(key[..]), |k, v| {
			let mut cursor = Cursor::new(k[1..].to_vec());
			cursor.set_position(0);
			let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
			let member = Member::read(&mut reader)?;
			Ok((member, u64::from_be_bytes(v.try_into().unwrap_or([0u8; 8]))))
		})?;

		let mut ret = vec![];
		loop {
			let next = itt.next();
			match next {
				Some(member) => {
					ret.push(member);
				}
				None => {
					break;
				}
			}
		}

		Ok(ret)
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
		server_pubkey: [u8; 32],
		challenge: [u8; 8],
		expiration_millis: u128,
		auth_flags: u64,
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

				// add ourselves as the server owner
				let member = Member {
					user_pubkey,
					server_id: [0u8; 8],
					server_pubkey,
				};
				let mut buffer = vec![];
				serialize_default(&mut buffer, &member)?;
				let mut buffer2 = vec![MEMBER_PREFIX];
				buffer2.append(&mut buffer);
				batch.put_ser(&buffer2, &auth_flags)?;

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
		requested_auth_flag: u64,
	) -> Result<(), Error> {
		let batch = self.store.batch()?;

		let mut auth_key = vec![TOKEN_PREFIX];
		auth_key.append(&mut user_pubkey.to_vec());
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
					let member = Member {
						user_pubkey,
						server_pubkey,
						server_id,
					};
					let mut buffer = vec![];
					serialize_default(&mut buffer, &member)?;
					let mut buffer2 = vec![MEMBER_PREFIX];
					buffer2.append(&mut buffer);

					let auth_level: Option<u64> = batch.get_ser(&buffer2)?;

					match auth_level {
						Some(auth_level) => {
							if (auth_level & requested_auth_flag) != 0 {
								Ok(())
							} else {
								Err(ErrorKind::NotAuthorized(format!(
									"not authorized. requested = {}, level = {}",
									requested_auth_flag, auth_level
								))
								.into())
							}
						}
						None => Err(ErrorKind::NotAuthorized(
							"not authorized for this resource".to_string(),
						)
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
			let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
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

	pub fn get_profile(
		&self,
		user_pubkey: [u8; 32],
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
	) -> Result<Option<Profile>, Error> {
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
		let profile_value: Option<ProfileValue> = batch.get_ser(&profile_key_buffer)?;

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
	}

	pub fn save_profile(&self, profile: Profile) -> Result<(), Error> {
		let profile_key: ProfileKey = profile.clone().into();
		let profile_value: ProfileValue = profile.into();

		let batch = self.store.batch()?;
		let mut profile_key_buffer = vec![];
		serialize_default(&mut profile_key_buffer, &profile_key)?;
		batch.put_ser(&profile_key_buffer, &profile_value)?;
		batch.commit()?;

		Ok(())
	}

	pub fn set_profile_image(
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
		let profile_value: Option<ProfileValue> = batch.get_ser(&profile_key_buffer)?;
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
				let profile_value: ProfileValue = profile.into();

				batch.put_ser(&profile_key_buffer, &profile_value)?;
				batch.commit()?;
			}
		}

		Ok(())
	}

	pub fn get_profile_images(
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
			let profile_value: Option<ProfileValue> = batch.get_ser(&profile_key_buffer)?;
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
	}

	pub fn get_profile_data(
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

			let profile_value: Option<ProfileValue> = batch.get_ser(&profile_key_buffer)?;
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
	}

	pub fn set_profile_data(
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
		let profile_value: Option<ProfileValue> = batch.get_ser(&profile_key_buffer)?;
		match profile_value {
			Some(mut profile_value) => {
				profile_value.profile_data = profile_data;
				batch.put_ser(&profile_key_buffer, &profile_value)?;
				batch.commit()?;
				Ok(())
			}
			None => Err(ErrorKind::ProfileNotFoundErr("profile not found".to_string()).into()),
		}
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
