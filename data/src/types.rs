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

use crate::nioruntime_log;
use crate::nioruntime_tor::ov3::OnionV3Address;
use crate::ser::{chunk_read, chunk_write, Readable, Reader, Writeable, Writer};
use concorderror::{Error, ErrorKind};
use ed25519_dalek::PublicKey;
use nioruntime_log::*;
use std::convert::TryInto;

info!();

#[derive(Debug, Clone)]
pub struct SerString {
	pub data: String,
}

impl SerString {
	pub fn to_string(&self) -> String {
		self.data.clone()
	}
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
		info!("serstring len={}", len);
		let mut byte_vec = vec![];
		for _ in 0..len {
			byte_vec.push(reader.read_u8()?);
		}

		Ok(Self {
			data: std::str::from_utf8(&byte_vec)?.to_string(),
		})
	}
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, PartialEq)]
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

impl From<ed25519_dalek::Signature> for Signature {
	fn from(signature: ed25519_dalek::Signature) -> Self {
		Self(signature.to_bytes())
	}
}

impl Signature {
	pub fn to_dalek(&self) -> Result<ed25519_dalek::Signature, Error> {
		ed25519_dalek::Signature::from_bytes(&self.0).map_err(|e| {
			let error: Error = ErrorKind::DalekError(format!("{}", e)).into();
			error
		})
	}
}

#[derive(Debug, Clone)]
pub struct SerOption<T>(pub Option<T>);

impl<T: Writeable + std::fmt::Debug> Writeable for SerOption<T> {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match &self.0 {
			Some(writeable) => {
				debug!("ser option is some: {:?}", writeable);
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServerId {
	data: [u8; 8],
}

impl ServerId {
	pub fn from_bytes(data: [u8; 8]) -> Self {
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

	pub fn to_base58(&self) -> Result<String, Error> {
		Ok(bs58::encode(self.data).into_string())
	}

	pub fn from_base58(&mut self, s: String) -> Result<(), Error> {
		self.data = (bs58::decode(s).into_vec()?)[..].try_into()?;
		Ok(())
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

#[derive(Debug, Clone, Copy, PartialEq)]
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

	pub fn from_onion(onion_address: &str) -> Result<Self, Error> {
		let onion_address: OnionV3Address = onion_address.try_into()?;
		Ok(Self {
			data: *onion_address.as_bytes(),
		})
	}

	pub fn to_onion(&self) -> Result<String, Error> {
		Ok(OnionV3Address::from_bytes(self.data).to_string())
	}

	pub fn to_base58(&self) -> Result<String, Error> {
		Ok(bs58::encode(self.data).into_string())
	}

	pub fn from_base58(&mut self, s: String) -> Result<(), Error> {
		self.data = (bs58::decode(s).into_vec()?)[..].try_into()?;
		Ok(())
	}

	pub fn to_dalek(&self) -> Result<PublicKey, Error> {
		PublicKey::from_bytes(&self.data).map_err(|e| {
			let error: Error = ErrorKind::DalekError(format!("{}", e)).into();
			error
		})
	}

	pub fn from_dalek(pubkey: PublicKey) -> Self {
		Self {
			data: *pubkey.as_bytes(),
		}
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

#[derive(Serialize, Debug, Clone)]
pub struct Invite {
	pub inviter: [u8; 32],
	pub server_id: [u8; 8],
	pub expiry: u128,
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
		writer.write_u128(self.expiry)?;
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

		let expiry = reader.read_u128()?;
		let cur = reader.read_u64()?;
		let max = reader.read_u64()?;
		let id = reader.read_u128()?;
		info!("read invite id={}", id);
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

#[derive(Debug, Clone)]
pub struct Image {
	pub data: Vec<u8>,
}

impl Default for Image {
	fn default() -> Self {
		Self { data: vec![] }
	}
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

#[derive(Debug, Clone)]
pub struct ProfileData {
	pub user_name: SerString,
	pub user_bio: SerString,
}

impl Default for ProfileData {
	fn default() -> Self {
		Self {
			user_name: SerString {
				data: "User Default".to_string(),
			},
			user_bio: SerString {
				data: "Tell us about you..".to_string(),
			},
		}
	}
}

impl Writeable for ProfileData {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Writeable::write(&self.user_name, writer)?;
		Writeable::write(&self.user_bio, writer)?;

		Ok(())
	}
}

impl Readable for ProfileData {
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let user_name = SerString::read(reader)?;
		let user_bio = SerString::read(reader)?;

		Ok(Self {
			user_name,
			user_bio,
		})
	}
}
