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

fn main() {
	match real_main() {
		Ok(_) => {},
		Err(e) => {
			println!("Unexpected error in real_main: {}", e.to_string());
		},
	}
	std::thread::park(); // park the thread so we don't exit
}

fn real_main() -> Result<(), Error> {
	rustlet_init!(RustletConfig {
		http_config: HttpConfig {
			port: 8093,
			root_dir: "~/.concord".to_string(),
			server_name: format!("Concord {}", VERSION),
			..HttpConfig::default()
		},
		..RustletConfig::default()
	});

	concord_init("~/.concord".to_string())?; // init concord
	Ok(())
}

