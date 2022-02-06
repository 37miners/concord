// Copyright 2021 The BMW Developers
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

/// Main configuration for concord
#[derive(Clone)]
pub struct ConcordConfig {
	pub tor_port: u16,
	pub port: u16,
	pub host: String,
	pub root_dir: String,
}

impl Default for ConcordConfig {
	fn default() -> Self {
		ConcordConfig {
			tor_port: 19901,
			port: 9919,
			host: "127.0.0.1".to_string(),
			root_dir: "~/.concord".to_string(),
		}
	}
}
