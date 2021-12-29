// Copyright 2021 The 37 Miners Developers
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
use crate::core::ser::{self, Readable, Writeable, Reader, Writer};
use crate::error::Error;

use std::convert::TryInto;

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
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), ser::Error> {
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
	fn read<R: Reader>(reader: &mut R) -> Result<Self, ser::Error> {
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

impl DSContext {
	pub fn get_servers_from_db(&self) -> Result<Option<ServerInfo>, Error> {
        	let batch = self.store.batch()?;
        	let res: Option<ServerInfo> = batch.get_ser(&[1])?;
        	Ok(res)
	}

	pub fn new(db_root: String) -> Result<DSContext, Error> {
		let store = Store::new(&db_root, None, Some(DB_NAME), None, true)?;

        	Ok(DSContext { store } )
	}
}
