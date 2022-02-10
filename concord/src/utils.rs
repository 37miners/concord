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

use concorddata::types::{Pubkey, ServerId};
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;

info!();

pub fn _extract_server_id_from_query() -> Result<ServerId, librustlet::Error> {
	let server_id = match query!("server_id") {
		Some(server_id) => ServerId::from_urlencoding(server_id).map_err(|e| {
			let error: librustlet::Error = librustlet::ErrorKind::ApplicationError(format!(
				"parsing server id generated error: {}",
				e
			))
			.into();
			error
		})?,
		None => {
			return Err(
				ErrorKind::ApplicationError("server_id must be specified".to_string()).into(),
			);
		}
	};
	Ok(server_id)
}

pub fn _extract_server_pubkey_from_query() -> Result<Pubkey, librustlet::Error> {
	let server_pubkey = match query!("server_pubkey") {
		Some(server_pubkey) => Pubkey::from_urlencoding(server_pubkey).map_err(|e| {
			let error: librustlet::Error = librustlet::ErrorKind::ApplicationError(format!(
				"parsing server pubkey generated error: {}",
				e
			))
			.into();
			error
		})?,
		None => {
			return Err(
				ErrorKind::ApplicationError("server_pubkey must be specified".to_string()).into(),
			);
		}
	};
	Ok(server_pubkey)
}

pub fn _extract_user_pubkey_from_query() -> Result<Pubkey, librustlet::Error> {
	let user_pubkey = match query!("user_pubkey") {
		Some(user_pubkey) => Pubkey::from_urlencoding(user_pubkey).map_err(|e| {
			let error: librustlet::Error = librustlet::ErrorKind::ApplicationError(format!(
				"parsing user pubkey generated error: {}",
				e
			))
			.into();
			error
		})?,
		None => {
			return Err(
				ErrorKind::ApplicationError("user_pubkey must be specified".to_string()).into(),
			);
		}
	};
	Ok(user_pubkey)
}

#[macro_export]
macro_rules! try2 {
	($a:expr,$b:expr) => {{
		$a.map_err(|e| {
			let error: librustlet::Error =
				librustlet::ErrorKind::ApplicationError(format!("{}: {}", $b, e)).into();
			error
		})?
	}};
}

#[macro_export]
macro_rules! send {
	($handle:expr,$event:expr) => {{
		error!("send event {:?}", $event);
		let mut buffer = vec![];
		crate::try2!(
			concorddata::ser::serialize_default(&mut buffer, &$event,),
			"serialize event error"
		);
		info!("bin={:?}", buffer);
		binary!($handle, buffer);
	}};
}

#[macro_export]
macro_rules! bin_event {
	() => {{
		let bin = binary!()?;
		info!("bindata={:?}", bin);
		let mut cursor = std::io::Cursor::new(bin);
		cursor.set_position(0);
		let mut reader = concorddata::ser::BinReader::new(
			&mut cursor,
			concorddata::ser::ProtocolVersion::local(),
		);

		let event: Event = concorddata::ser::Readable::read(&mut reader).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("deserialization error: {}", e)).into();
			error
		})?;

		event
	}};
}

#[macro_export]
macro_rules! close {
	($handle:expr, $conn_info:expr) => {{
		let id = $handle.get_connection_id();
		$conn_info.remove(&id);
		$handle.get_wh().close()?;
	}};
}

#[macro_export]
macro_rules! owner {
	($conn_info:expr) => {{
		match &$conn_info.pubkey {
			None => {
				return Ok(true);
			}
			Some(pubkey) => {
				let server_pubkey = pubkey!();
				if pubkey.to_bytes() != server_pubkey {
					info!("not the owner!");
					return Ok(true);
				}
			}
		}
	}};
}

#[macro_export]
macro_rules! member {
	($conn_info:expr, $ds_context:expr) => {{
		// for now just continue. TODO: implement this to check if the user is a member
	}};
}
