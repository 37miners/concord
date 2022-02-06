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

//! Entry point for concord module initilization logic

use crate::context::ConcordContext;
use concordconfig::ConcordConfig;
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use nioruntime_log::*;

use std::fs::File;
use std::io::Write;
use std::path::Path;

debug!(); // set log level to debug
const MAIN_LOG: &str = "mainlog";

// create a file from bytes that are included in resources.
fn create_file_from_bytes(
	resource: String,
	root_dir: String,
	bytes: &[u8],
) -> Result<(), ConcordError> {
	let path = format!("{}/www/{}", root_dir, resource);
	let mut file = File::create(&path)?;
	file.write_all(bytes)?;
	Ok(())
}

// setup the webroot dir for concord
fn init_webroot(config: &ConcordConfig) {
	// create the directory structure
	let js_dir = format!("{}/www/js", config.root_dir.clone());
	let css_dir = format!("{}/www/css", config.root_dir.clone());
	let images_dir = format!("{}/www/images", config.root_dir.clone());
	let user_images = format!("{}/www/images/user_images", config.root_dir.clone());

	// if the js dir doesn't exist it means we're going to init things
	if !Path::new(&js_dir).exists() {
		fsutils::mkdir(&js_dir);
		fsutils::mkdir(&css_dir);
		fsutils::mkdir(&images_dir);
		fsutils::mkdir(&user_images);
		let bytes = include_bytes!("resources/jquery-3.6.0.min.js");
		match create_file_from_bytes(
			"js/jquery-3.6.0.min.js".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}
		let bytes = include_bytes!("resources/contextMenu.min.js");
		match create_file_from_bytes(
			"js/contextMenu.min.js".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/contextMenu.min.css");
		match create_file_from_bytes(
			"css/contextMenu.min.css".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/jsbn.js");
		match create_file_from_bytes("js/jsbn.js".to_string(), config.root_dir.clone(), bytes) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/jsbn2.js");
		match create_file_from_bytes("js/jsbn2.js".to_string(), config.root_dir.clone(), bytes) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/ser.js");
		match create_file_from_bytes("js/ser.js".to_string(), config.root_dir.clone(), bytes) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/concord.js");
		match create_file_from_bytes("js/concord.js".to_string(), config.root_dir.clone(), bytes) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/ws.html");
		match create_file_from_bytes("ws.html".to_string(), config.root_dir.clone(), bytes) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/index.html");
		match create_file_from_bytes("index.html".to_string(), config.root_dir.clone(), bytes) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/style.css");
		match create_file_from_bytes("css/style.css".to_string(), config.root_dir.clone(), bytes) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/add_channel.png");
		match create_file_from_bytes(
			"images/add_channel.png".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/crown.png");
		match create_file_from_bytes(
			"images/crown.png".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/gear.png");
		match create_file_from_bytes(
			"images/gear.png".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/user1.png");
		match create_file_from_bytes(
			"images/user1.png".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/Loading_icon.gif");
		match create_file_from_bytes(
			"images/Loading_icon.gif".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/plus.png");
		match create_file_from_bytes(
			"images/plus.png".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/delete.png");
		match create_file_from_bytes(
			"images/delete.png".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/create.png");
		match create_file_from_bytes(
			"images/create.png".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/update.png");
		match create_file_from_bytes(
			"images/update.png".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/plus_fill.png");
		match create_file_from_bytes(
			"images/plus_fill.png".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/close_icon.png");
		match create_file_from_bytes(
			"images/close_icon.png".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/creative-commons.png");
		match create_file_from_bytes("favicon.ico".to_string(), config.root_dir.clone(), bytes) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/creative-commons.png");
		match create_file_from_bytes(
			"favicon-32x32.ico".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}

		let bytes = include_bytes!("resources/images/creative-commons.png");
		match create_file_from_bytes(
			"favicon-16x16.ico".to_string(),
			config.root_dir.clone(),
			bytes,
		) {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Creating file resulted in error: {}",
					e.to_string()
				);
			}
		}
	}
}

// We initialize concord here.
pub fn concord_init(config: ConcordConfig) -> Result<(), ConcordError> {
	let context = ConcordContext::new();

	init_webroot(&config); // setup webroot
	crate::auth::init_auth(&config, context.clone())?; // auth module
	crate::server::init_server(&config, context.clone())?; // server module
	crate::message::init_message(&config, context.clone())?; // message module
	crate::channel::init_channels(&config, context.clone())?; // channel module
	crate::invite::init_invite(&config, context.clone())?; // invite module
	crate::members::init_members(&config, context.clone())?; // members module
	crate::persistence::init_persistence(&config, context.clone())?; // persistence module
	crate::profile::init_profile(&config, context.clone())?; // profile module
	crate::ws::init_ws(config, context.clone())?; // websocket module

	Ok(())
}
