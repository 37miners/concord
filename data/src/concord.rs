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

use std::convert::TryInto;
use std::path::PathBuf;
use std::io::Cursor;

const DB_NAME: &str = "concord";

pub struct DSContext {
	store: Store,
}

#[derive(Debug)]
pub struct ServerInfo {
	pub address: String,
	pub name: String,
	pub icon: Vec<u8>,
}

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

const SERVER_PREFIX: u8 = 0;

impl DSContext {
	pub fn get_servers(&self) -> Result<Vec<(ServerInfo, String)>, Error> {
        	let batch = self.store.batch()?;
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

	pub fn get_server_info(&self, id: String) -> Result<Option<ServerInfo>, Error> {
		let batch = self.store.batch()?;
		let id = urlencoding::decode(&id)?;
		let mut id = base64::decode(&*id)?;
		let mut key = vec![SERVER_PREFIX];
		key.append(&mut id);
		let ret: Option<ServerInfo> = batch.get_ser(&key)?;

		Ok(ret)
	}

	pub fn add_server(&self, server_info: ServerInfo) -> Result<(), Error> {
		let batch = self.store.batch()?;
		let mut key = vec![SERVER_PREFIX];
		let id: [u8; 8] = rand::random();
		let id = base64::encode(&id);
		let id = urlencoding::encode(&id);
		key.append(&mut id.as_bytes().to_vec());
		batch.put_ser(&key, &server_info)?;
		batch.commit()?;
		Ok(())
	}

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
