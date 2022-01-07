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

use concordlib::*; // import our rustlets
use librustlet::rustlet_init;
use librustlet::nioruntime_log;
use librustlet::HttpConfig;
use librustlet::RustletConfig;
use concorderror::Error;
nioruntime_log::debug!(); // set log level to debug
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

// main which just calls 'real_main'
fn main() {
	match real_main() {
		Ok(_) => {},
		Err(e) => {
			println!("Unexpected error in real_main: {}", e.to_string());
		},
	}
	std::thread::park(); // park the thread so we don't exit
}

// real main which handles errors
fn real_main() -> Result<(), Error> {
	// for now we just hard code host/port here. TODO: make configurable
	let host = "127.0.0.1".to_string();
	let port = 8093;
	let uri = format!("{}:{}", host, port);

	// init our rustlet container
	rustlet_init!(RustletConfig {
		http_config: HttpConfig {
			host,
			port,
			tor_port: 1234,
			root_dir: "~/.concord".to_string(),
			server_name: format!("Concord {}", VERSION),
			..HttpConfig::default()
		},
		..RustletConfig::default()
	});

	// init concord
	concord_init("~/.concord".to_string(), uri)?; // init concord
	Ok(())
}

