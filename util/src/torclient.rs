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

use concorderror::Error;
use std::io::prelude::*;
use std::net::*;
use tor_stream::TorStream;

pub fn do_get(onion_address: String, path: String, tor_port: u16) -> Result<String, Error> {
	let proxy_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), tor_port);
	let target: socks::TargetAddr = socks::TargetAddr::Domain(onion_address, 80);
	let mut stream = TorStream::connect_with_address(proxy_addr, target)?;

	stream
		.write_all(
			format!(
				"GET {} HTTP/1.1\r\nConnection: Close\r\nHost: www.example.com\r\n\r\n",
				path
			)
			.as_bytes(),
		)
		.expect("Failed to send request");

	let mut stream = stream.into_inner();

	let mut buf = String::new();
	stream
		.read_to_string(&mut buf)
		.expect("Failed to read response");

	Ok(buf)
}
