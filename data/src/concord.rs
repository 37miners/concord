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
use crate::ser::{Readable, Writeable, Reader, Writer, BinReader, ProtocolVersion};
use concorderror::Error;
use crate::ser::serialize_default;

use std::convert::TryInto;
use std::path::PathBuf;
use std::io::Cursor;

const DB_NAME: &str = "concord";

// the context to use for accessing concord data. Multiple instances
// may exist and LMDB handles concurrency.
pub struct DSContext {
	store: Store,
}

// Message types
#[derive(Clone)]
pub enum MessageType {
	Text,
	Binary,
}

// The key to a message entry
#[derive(Clone)]
pub struct MessageKey {
	pub server_pubkey: [u8; 32],
	pub server_id: [u8; 8],
	pub channel_id: u64,
	pub timestamp: u64,
	pub user_pubkey: [u8; 32],
	pub nonce: u16,
}

// the Writeable implmenetation for serializing MessageKey
impl Writeable for MessageKey {
        fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(MESSAGE_PREFIX)?;
                for i in 0..32 {
                        writer.write_u8(self.server_pubkey[i])?;
                }
		for i in 0..8 {
			writer.write_u8(self.server_id[i])?;
		}
		writer.write_u64(self.channel_id)?;
		writer.write_u64(self.timestamp)?;
		for i in 0..32 {
			writer.write_u8(self.user_pubkey[i])?;
		}
		writer.write_u16(self.nonce)?;
		Ok(())
	}
}

// the Readable implmentation for deserializing MessageKey
impl Readable for MessageKey {
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
		let timestamp = reader.read_u64()?;
		let mut user_pubkey = vec![];
                for _ in 0..32 {
                        user_pubkey.push(reader.read_u8()?);
                }
		let nonce = reader.read_u16()?;

		let server_pubkey = server_pubkey.as_slice().try_into()?;
		let server_id = server_id.as_slice().try_into()?;
		let user_pubkey = user_pubkey.as_slice().try_into()?;

		Ok(
			MessageKey {
				server_pubkey,
				server_id,
				channel_id,
				timestamp,
				user_pubkey,
				nonce,
			}
		)
	}
}

// information associated with a message
#[derive(Clone)]
pub struct Message {
	pub payload: Vec<u8>,
	pub signature: [u8; 64],
	pub message_type: MessageType,
}

// the Writeable implmenetation for serializing Message
impl Writeable for Message {
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

// the Readable implmentation for deserializing Message
impl Readable for Message {
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

		Ok(
			Message {
				payload,
				signature,
				message_type,
			}
		)
	}
}

// information about the server
#[derive(Debug)]
pub struct ServerInfo {
	pub address: String,
	pub name: String,
	pub icon: Vec<u8>,
}

// the Writeable implmenetation for serializing ServerInfo
impl Writeable for ServerInfo {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let address_len = self.address.len();
		let address_bytes = self.address.as_bytes();
		writer.write_u32(address_len.try_into().unwrap_or(0))?;
		for i in 0..address_len {
			writer.write_u8(address_bytes[i])?;
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
		Ok(())
	}
}

// the Readable implmentation for deserializing ServerInfo
impl Readable for ServerInfo {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let address_len = reader.read_u32()?;
		let mut address = vec![];
		for _ in 0..address_len {
			address.push(reader.read_u8()?);
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
		let address = std::str::from_utf8(&address)?;
		let address = address.to_string();
		Ok(ServerInfo { address, name, icon})
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

#[derive(Debug)]
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

// data prefixes
const SERVER_PREFIX: u8 = 0;
const AUTH_PREFIX: u8 = 1;
const MESSAGE_PREFIX: u8 = 2;
const CHANNEL_PREFIX: u8 = 3;

impl DSContext {
	// get a list of servers in the local database
	pub fn get_servers(&self) -> Result<Vec<(ServerInfo, String)>, Error> {
        	let batch = self.store.batch()?;
		// get the iterator for each server info
        	let mut itt = batch.iter(&(vec![SERVER_PREFIX])[..], |k, v| {
			let id = base64::encode(&k[1..]);
                	let id = urlencoding::encode(&id).to_string();
			let mut cursor = Cursor::new(v.to_vec());
        		cursor.set_position(0);
        		let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
			Ok((ServerInfo::read(&mut reader)?, id.clone()))
		})?;

		let mut ret = vec![];
		loop {
			match itt.next() {
				Some((server,id)) => ret.push((server, id.to_string())),
				None => break,
			}
		}

		Ok(ret)
	}

	// get server info about a specific server id
	pub fn get_server_info(&self, id: String) -> Result<Option<ServerInfo>, Error> {
		let batch = self.store.batch()?;
		let id = urlencoding::decode(&id)?;
		let mut id = base64::decode(&*id)?;
		let mut key = vec![SERVER_PREFIX];
		key.append(&mut id);
		let ret: Option<ServerInfo> = batch.get_ser(&key)?;

		Ok(ret)
	}

	// add a server
	pub fn add_server(&self, server_info: ServerInfo) -> Result<[u8; 8], Error> {
		let batch = self.store.batch()?;
		let mut key = vec![SERVER_PREFIX];
		let id: [u8; 8] = rand::random();
		key.append(&mut id.to_vec());
		batch.put_ser(&key, &server_info)?;
		batch.commit()?;
		Ok(id)
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

	// add an auth cookie to the db
	pub fn add_auth_cookie(&self, token: String) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut key = vec![AUTH_PREFIX];
		let token: u128 = token.parse()?;
		let mut token = token.to_be_bytes().to_vec();
		key.append(&mut token);
		let value = 0u8;
		batch.put_ser(&key, &value)?;
		batch.commit()?;
		Ok(())
	}

	// validate an auth cookie
	pub fn check_auth_cookie(&self, token: String) -> Result<bool, Error> {
                let batch = self.store.batch()?;
                let mut key = vec![AUTH_PREFIX];
		let token: u128 = token.parse()?;
                let mut token = token.to_be_bytes().to_vec();
                key.append(&mut token);
		let value: Option<u8> = batch.get_ser(&key)?;
		match value {
			Some(_) => Ok(true),
			None => Ok(false),
		}
	}

	// post a message to the db
	pub fn post_message(&self, key: MessageKey, message: Message) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut buffer = vec![];
                serialize_default(&mut buffer, &key)?;
		batch.put_ser(&buffer, &message)?;
		batch.commit()?;
		Ok(())
	}

	// for now we just create a new iterator each time we paginate.
	// TODO: store iterators so that faster access can be acheived.
	pub fn get_messages(
		&self,
		server_pubkey: [u8; 32],
		server_id: [u8; 8],
		channel_id: u64,
		offset: u64,
		len: usize,
	) -> Result<Vec<(MessageKey, Message)>, Error> {
		let batch = self.store.batch()?;
                // get the iterator for each message
		let mut key_vec = vec![MESSAGE_PREFIX];
		key_vec.append(&mut server_pubkey.to_vec());
		key_vec.append(&mut server_id.to_vec());
		key_vec.append(&mut channel_id.to_be_bytes().to_vec());
		
                let mut itt = batch.iter(&(key_vec[..]), |k, v| {
                        let mut cursor = Cursor::new(k.to_vec());
                        cursor.set_position(0);
                        let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
			let mkey = MessageKey::read(&mut reader)?;

			let mut cursor = Cursor::new(v.to_vec());
			cursor.set_position(0);
			let mut reader = BinReader::new(&mut cursor, ProtocolVersion(1));
			let mval = Message::read(&mut reader)?;
			Ok((mkey, mval))
                })?;

                let mut ret = vec![];
		let mut itt_count = 0;
                loop {
			let next = itt.next();
                        match next {
                                Some((mkey,mval)) => {
					if itt_count >= offset {
						ret.push((mkey, mval));
					}
					if ret.len() >= len {
						break;
					}
					itt_count += 1;
				},
                                None => { break; },
                        }
                }

		Ok(ret)
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
                                },
                                None => { break; },
                        }
                }

                Ok(ret)
	}

	pub fn set_channel(
		&self,
		channel_key: ChannelKey,
		channel: Channel,
	) -> Result<(), Error> {
                let batch = self.store.batch()?;
                let mut buffer = vec![];
                serialize_default(&mut buffer, &channel_key)?;
		let mut buffer2 = vec![CHANNEL_PREFIX];
		buffer2.append(&mut buffer);
                batch.put_ser(&buffer2, &channel)?;
                batch.commit()?;
                Ok(())
	}

	pub fn delete_channel(
		&self,
		channel_key: ChannelKey,
	) -> Result<(), Error> {
                let batch = self.store.batch()?;
                let mut buffer = vec![];
                serialize_default(&mut buffer, &channel_key)?;
                batch.delete(&buffer)?;
                batch.commit()?;
                Ok(())
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
        	Ok(DSContext { store } )
	}
}
