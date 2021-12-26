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

use librustlet::*; // use the librustlet library
use concordlib::*; // import our rustlets
nioruntime_log::debug!(); // set log level to debug

fn main() {
	rustlet_init!(RustletConfig::default()); // initialize with the default config
	concord_init();
	rustlet!("myrustlet", { response!("Hello World!"); }); // hello world rustlet
	rustlet_mapping!("/", "myrustlet"); // create mapping to '/'
	std::thread::park(); // park the thread so we don't exit
}
