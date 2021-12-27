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

use librustlet::*;
use nioruntime_log::*;

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

nioruntime_log::debug!(); // set log level to debug
const MAIN_LOG: &str = "mainlog";

// create a file from bytes that are included in resources.
fn create_file_from_bytes(resource: String, root_dir: String, bytes: &[u8]) -> Result<(), Error> {
	let path = format!("{}/www/{}", root_dir, resource);
	let mut file = File::create(&path)?;
	file.write_all(bytes)?;
	Ok(())
}

// setup the webroot dir for concord
fn init_webroot(root_dir: String) {
	let home_dir = match dirs::home_dir() {
		Some(p) => p,
		None => PathBuf::new(),
	}
	.as_path()
	.display()
	.to_string();
	let root_dir = root_dir.replace("~", &home_dir);
	let js_dir = format!("{}/www/js", root_dir);
	if !Path::new(&js_dir).exists() {
		fsutils::mkdir(&js_dir);
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
	}
}

pub fn concord_init(root_dir: String) {
	init_webroot(root_dir);
	rustlet!("myrustlet2", {
		response!("Concord!");
	}); // hello world rustlet
	rustlet_mapping!("/r2", "myrustlet2"); // create mapping to '/'
}