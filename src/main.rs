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

use clap::load_yaml;
use clap::App;
use concordconfig::ConcordConfig;
use concorderror::Error;
use concordlib::*;
use librustlet::*;
use std::path::PathBuf;

nioruntime_log::debug!(); // set log level to debug
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

// main which just calls 'real_main'
fn main() {
	match real_main() {
		Ok(_) => {}
		Err(e) => {
			println!("Unexpected error in real_main: {}", e.to_string());
			return;
		}
	}
	std::thread::park(); // park the thread so we don't exit
}

// real main which handles errors
fn real_main() -> Result<(), Error> {
	let yml = load_yaml!("concord.yml");
	let args = App::from_yaml(yml)
		.version(built_info::PKG_VERSION)
		.get_matches();

	let debug = args.is_present("debug");

	let tor_port = args.is_present("tor_port");
	let tor_port = match tor_port {
		true => args.value_of("tor_port").unwrap().parse().unwrap(),
		false => 1234,
	};

	let port = args.is_present("port");
	let port = match port {
		true => args.value_of("port").unwrap().parse().unwrap(),
		false => 8093,
	};

	let host = args.is_present("host");
	let host = match host {
		true => args.value_of("host").unwrap(),
		false => "127.0.0.1",
	}
	.to_string();

	let root_dir = args.is_present("root_dir");
	let root_dir = match root_dir {
		true => args.value_of("root_dir").unwrap().to_string(),
		false => "~/.concord".to_string(),
	};

	// get homedir updated root_dir
	let home_dir = match dirs::home_dir() {
		Some(p) => p,
		None => PathBuf::new(),
	}
	.as_path()
	.display()
	.to_string();
	let root_dir = root_dir.replace("~", &home_dir);

	let config = ConcordConfig {
		tor_port,
		port,
		root_dir,
		host,
		..Default::default()
	};

	// init our rustlet container
	rustlet_init!(RustletConfig {
		http_config: HttpConfig {
			host: config.host.clone(),
			port: config.port.clone(),
			tor_port: config.tor_port,
			root_dir: config.root_dir.clone(),
			server_name: format!("Concord {}", VERSION),
			debug,
			..HttpConfig::default()
		},
		..RustletConfig::default()
	});

	// init concord
	concord_init(config)?; // init concord
	Ok(())
}
