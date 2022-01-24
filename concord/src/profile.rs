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

use crate::context::ConcordContext;
use crate::utils::extract_server_id_from_query;
use crate::utils::extract_server_pubkey_from_query;
use crate::utils::extract_user_pubkey_from_query;
use crate::utils::Pubkey;
use concordconfig::ConcordConfig;
use concorddata::concord::get_default_profile;
use concorddata::concord::DSContext;
use concorddata::concord::ProfileData;
use concorderror::Error as ConcordError;
use librustlet::*;
use nioruntime_log::*;
use std::fs::File;
use std::io::Read;

debug!(); // set log level to debug

fn get_default_user_icon(root_dir: String) -> Result<Vec<u8>, Error> {
	let user1 = format!("{}/www/images/user1.png", root_dir);
	let mut file = File::open(user1)?;
	let mut data = vec![];
	file.read_to_end(&mut data)?;
	Ok(data)
}

// initialize this module.
pub fn init_profile(cconfig: &ConcordConfig, _context: ConcordContext) -> Result<(), ConcordError> {
	let ds_context = DSContext::new(cconfig.root_dir.clone())?;
	// sets profile image for the specified user on this server
	rustlet!("set_profile_image", {
		let user_pubkey = extract_user_pubkey_from_query()?;
		let server_pubkey = extract_server_pubkey_from_query()?;
		let server_id = extract_server_id_from_query()?;

		let content = request_content!();
		let content = &mut &content[..];
		let mut headers = hyper::header::Headers::new();
		for i in 0..header_len!() {
			headers.append_raw(header_name!(i), header_value!(i).as_bytes().to_vec());
		}
		// parse the mime_multipart data in this request
		let res = mime_multipart::read_multipart_body(content, &headers, false).unwrap_or(vec![]);

		let mut avatar = None;
		for node in &res {
			match node {
				mime_multipart::Node::File(filepart) => {
					let mut f = File::open(&filepart.path)?;
					let size = filepart.size.unwrap_or(0);
					let mut buf = vec![0 as u8; size];
					f.read(&mut buf)?;
					avatar = Some(buf);
				}
				_ => {}
			}
		}

		let avatar = match avatar {
			Some(avatar) => avatar,
			None => {
				return Err(
					ErrorKind::ApplicationError("avatar must specified".to_string()).into(),
				);
			}
		};

		ds_context
			.set_profile_image(
				user_pubkey.to_bytes(),
				server_pubkey.to_bytes(),
				server_id.to_bytes(),
				avatar,
			)
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("ds error setting user image: {}", e))
						.into();
				error
			})?;
	});
	rustlet_mapping!("/set_profile_image", "set_profile_image");

	let ds_context = DSContext::new(cconfig.root_dir.clone())?;
	// sets the profile data for the specified user on this server
	rustlet!("set_profile_data", {
		let user_pubkey = extract_user_pubkey_from_query()?;
		let server_pubkey = extract_server_pubkey_from_query()?;
		let server_id = extract_server_id_from_query()?;

		let user_bio = match query!("user_bio") {
			Some(user_bio) => user_bio,
			None => {
				return Err(
					ErrorKind::ApplicationError("user_bio must be specified".to_string()).into(),
				);
			}
		};
		let user_name = match query!("user_name") {
			Some(user_name) => user_name,
			None => {
				return Err(
					ErrorKind::ApplicationError("user_name must be specified".to_string()).into(),
				);
			}
		};

		ds_context
			.set_profile_data(
				user_pubkey.to_bytes(),
				server_pubkey.to_bytes(),
				server_id.to_bytes(),
				ProfileData {
					user_name,
					user_bio,
				},
			)
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("set_profile_data generated error: {}", e))
						.into();
				error
			})?;
	});
	rustlet_mapping!("/set_profile_data", "set_profile_data");

	let ds_context = DSContext::new(cconfig.root_dir.clone())?;
	// get the profile images for the specified users on the specified server
	rustlet!("get_profile_images", {
		let server_pubkey = extract_server_pubkey_from_query()?;
		let server_id = extract_server_id_from_query()?;

		let content = request_content!();
		let content = std::str::from_utf8(&content)?;
		let lines = content.split("\r\n");

		let mut user_pubkeys = vec![];

		for line in lines {
			if line.len() == 0 {
				continue;
			}

			let user_pubkey = Pubkey::from_urlencoding(line.to_string()).map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("error parsing user_pubkey: {}", e)).into();
				error
			})?;

			user_pubkeys.push(user_pubkey.to_bytes());
		}

		let images = ds_context
			.get_profile_images(user_pubkeys, server_pubkey.to_bytes(), server_id.to_bytes())
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("get_profile_data error: {}", e)).into();
				error
			})?;

		response!("{:?}", images);
	});
	rustlet_mapping!("/get_profile_images", "get_profile_images");

	let root_dir = cconfig.root_dir.clone();
	let ds_context = DSContext::new(cconfig.root_dir.clone())?;
	// get the profile images for the specified users on the specified server
	rustlet!("get_profile_image", {
		let user_pubkey = extract_user_pubkey_from_query()?;
		let server_pubkey = extract_server_pubkey_from_query()?;
		let server_id = extract_server_id_from_query()?;

		let image = ds_context
			.get_profile_images(
				vec![user_pubkey.to_bytes()],
				server_pubkey.to_bytes(),
				server_id.to_bytes(),
			)
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("get_profile_image error: {}", e)).into();
				error
			})?;

		match image.len() {
			1 => match &image[0] {
				Some(image) => bin_write!(&image),
				None => {
					let def_user_icon = get_default_user_icon(root_dir.clone())?;
					bin_write!(&def_user_icon);
				}
			},
			_ => response!("error! image not found!"),
		}
	});
	rustlet_mapping!("/get_profile_image", "get_profile_image");

	let ds_context = DSContext::new(cconfig.root_dir.clone())?;
	// get the profile data for the specified users on this server
	rustlet!("get_profile_data", {
		let server_pubkey = extract_server_pubkey_from_query()?;
		let server_id = extract_server_id_from_query()?;

		let content = request_content!();
		let content = std::str::from_utf8(&content)?;
		let lines = content.split("\r\n");

		let mut user_pubkeys = vec![];

		for line in lines {
			if line.len() == 0 {
				continue;
			}

			let user_pubkey = Pubkey::from_urlencoding(line.to_string()).map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("error parsing user_pubkey: {}", e)).into();
				error
			})?;

			user_pubkeys.push(user_pubkey.to_bytes());
		}

		let data = ds_context
			.get_profile_data(user_pubkeys, server_pubkey.to_bytes(), server_id.to_bytes())
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("get_profile_data error: {}", e)).into();
				error
			})?;

		let json = serde_json::to_string(&data).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("json parse error: {}", e)).into();
			error
		})?;
		response!("{}", json);
	});
	rustlet_mapping!("/get_profile_data", "get_profile_data");

	let ds_context = DSContext::new(cconfig.root_dir.clone())?;

	rustlet!("get_mini_profile", {
		let user_pubkey = pubkey!();
		let server_pubkey = pubkey!();
		let server_id = [0u8; 8]; // special server_id for our global data

		let profile = ds_context
			.get_profile(user_pubkey, server_pubkey, server_id)
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("get_profile error: {}", e)).into();
				error
			})?;

		let profile = match profile {
			Some(profile) => profile,
			None => get_default_profile(),
		};

		let user_pubkey = Pubkey::from_bytes(user_pubkey)
			.to_urlencoding()
			.map_err(|e| {
				let error: Error =
					ErrorKind::ApplicationError(format!("pubkey parse error: {}", e)).into();
				error
			})?;

		let json = serde_json::to_string(&(profile.profile_data, user_pubkey)).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("json parse error: {}", e)).into();
			error
		})?;
		response!("{}", json);
	});
	rustlet_mapping!("/get_mini_profile", "get_mini_profile");

	Ok(())
}
