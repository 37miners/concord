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

use concorderror::Error as ConcordError;
use librustlet::*;
use nioruntime_log::*;

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

nioruntime_log::debug!(); // set log level to debug
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
fn init_webroot(root_dir: String) {
	// create the directory structure
	let js_dir = format!("{}/www/js", root_dir);
	let css_dir = format!("{}/www/css", root_dir);
	let images_dir = format!("{}/www/images", root_dir);

	// if the js dir doesn't exist it means we're going to init things
	if !Path::new(&js_dir).exists() {
		fsutils::mkdir(&js_dir);
		fsutils::mkdir(&css_dir);
		fsutils::mkdir(&images_dir);
		let bytes = include_bytes!("resources/jquery-3.6.0.min.js");
		match create_file_from_bytes(
			"js/jquery-3.6.0.min.js".to_string(),
			root_dir.clone(),
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
                match create_file_from_bytes("js/contextMenu.min.js".to_string(), root_dir.clone(), bytes) {
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
                match create_file_from_bytes("css/contextMenu.min.css".to_string(), root_dir.clone(), bytes) {
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
		match create_file_from_bytes("js/concord.js".to_string(), root_dir.clone(), bytes) {
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
		match create_file_from_bytes("index.html".to_string(), root_dir.clone(), bytes) {
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
		match create_file_from_bytes("css/style.css".to_string(), root_dir.clone(), bytes) {
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
		match create_file_from_bytes("images/plus.png".to_string(), root_dir.clone(), bytes) {
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
                match create_file_from_bytes("images/delete.png".to_string(), root_dir.clone(), bytes) {
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
                match create_file_from_bytes("images/create.png".to_string(), root_dir.clone(), bytes) {
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
                match create_file_from_bytes("images/update.png".to_string(), root_dir.clone(), bytes) {
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
		match create_file_from_bytes("images/plus_fill.png".to_string(), root_dir.clone(), bytes) {
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
		match create_file_from_bytes("images/close_icon.png".to_string(), root_dir.clone(), bytes) {
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
pub fn concord_init(root_dir: String, uri: String) -> Result<(), ConcordError> {
	// get homedir updated root_dir
        let home_dir = match dirs::home_dir() {
                Some(p) => p,
                None => PathBuf::new(),
        }
        .as_path()
        .display()
        .to_string();
        let root_dir = root_dir.replace("~", &home_dir);

	init_webroot(root_dir.clone()); // setup webroot
	crate::auth::init_auth(root_dir.clone(), uri)?; // auth module
	crate::server::init_server(root_dir.clone())?; // server module
	crate::message::init_message(root_dir.clone())?; // message module

	Ok(())
}
