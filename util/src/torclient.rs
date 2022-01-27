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

use crate::nioruntime_log;
use crate::nioruntime_log::*;
use concorderror::{Error, ErrorKind};
use std::io::prelude::*;
use std::net::*;
use tor_stream::TorStream;
use url::Host::Domain;
use url::Url;

debug!();

pub fn listen(
	url: String,
	post_data: String,
	tor_port: u16,
	callback: &dyn Fn(String) -> (),
) -> Result<(), Error> {
	let url = Url::parse(&url).map_err(|e| {
		let error: Error =
			ErrorKind::TorError(format!("url parse error: {}", e.to_string())).into();
		error
	})?;
	let host = format!("{}", url.host().unwrap_or(Domain("notfound")));
	let path = format!("{}?{}", url.path(), url.query().unwrap_or(""));
	let proxy_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), tor_port);
	let target: socks::TargetAddr = socks::TargetAddr::Domain(host.clone(), 80);
	let mut stream = TorStream::connect_with_address(proxy_addr, target)?;
	let content_len = post_data.len();
	stream
                .write_all(
                        format!(
                                "POST {} HTTP/1.1\r\nConnection: Close\r\nContent-Length: {}\r\nHost: localhost\r\n\r\n{}",
                                path,
				content_len,
				post_data,
                        )
                        .as_bytes(),
                )
                .expect("Failed to send request");

	let mut stream = stream.into_inner();
	let mut passed_headers = false;
	let mut data = vec![];

	loop {
		let mut buffer = [0u8; 1024];
		let len = stream.read(&mut buffer)?;
		if len <= 0 {
			break;
		}
		data.append(&mut buffer[0..len].to_vec());

		let str = std::str::from_utf8(&data)?;
		if !passed_headers {
			match str.find("\r\n\r\n") {
				Some(end_headers) => {
					passed_headers = true;
					data = data[(end_headers + 4)..].to_vec();
				}
				None => {}
			}
		}

		let str = std::str::from_utf8(&data)?;

		match str.find("//-----ENDJSON-----") {
			Some(index) => {
				let json_text = str[0..index].to_string();
				(callback)(json_text);
				data = data[(index + 17)..].to_vec();
			}
			None => { // we don't have the complete json
			}
		}
	}

	Ok(())
}

pub fn do_get_bin(onion_address: String, path: String, tor_port: u16) -> Result<Vec<u8>, Error> {
	let proxy_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), tor_port);
	let target: socks::TargetAddr = socks::TargetAddr::Domain(onion_address.clone(), 80);
	let mut stream = TorStream::connect_with_address(proxy_addr, target)?;
	error!("in doget_bin: {}, {}", onion_address, path);
	stream
		.write_all(
			format!(
				"GET {} HTTP/1.1\r\nConnection: Close\r\nHost: localhost\r\n\r\n",
				path
			)
			.as_bytes(),
		)
		.expect("Failed to send request");
	error!("wrote data");
	let mut stream = stream.into_inner();

	let mut data = vec![];

	loop {
		let mut buffer = [0u8; 1024];
		let len = stream.read(&mut buffer)?;
		if len <= 0 {
			break;
		}
		data.append(&mut buffer[0..len].to_vec());
	}
	error!("read complete len = {}", data.len());
	Ok(data)
}

pub fn do_get(onion_address: String, path: String, tor_port: u16) -> Result<String, Error> {
	let proxy_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), tor_port);
	let target: socks::TargetAddr = socks::TargetAddr::Domain(onion_address.clone(), 80);
	let mut stream = TorStream::connect_with_address(proxy_addr, target)?;
	error!("in doget: {}, {}", onion_address, path);
	stream
		.write_all(
			format!(
				"GET {} HTTP/1.1\r\nConnection: Close\r\nHost: localhost\r\n\r\n",
				path
			)
			.as_bytes(),
		)
		.expect("Failed to send request");
	error!("wrote data");
	let mut stream = stream.into_inner();

	let mut buf = String::new();
	stream
		.read_to_string(&mut buf)
		.expect("Failed to read response");
	error!("read complete len = {}", buf.len());
	Ok(buf)
}

pub fn do_post(
	onion_address: String,
	path: String,
	tor_port: u16,
	data: Vec<u8>,
) -> Result<String, Error> {
	let proxy_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), tor_port);
	let target: socks::TargetAddr = socks::TargetAddr::Domain(onion_address, 80);
	let mut stream = TorStream::connect_with_address(proxy_addr, target)?;
	let headers = format!(
		"POST {} HTTP/1.1\r\nConnection: Close\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
		path,
		data.len(),
	);

	let mut bytes = vec![];
	bytes.append(&mut headers.as_bytes().to_vec());
	bytes.append(&mut data.clone());

	stream.write_all(&bytes).expect("Failed to send request");

	//stream.write_all(&data).expect("failed to send request");

	let mut stream = stream.into_inner();

	let mut buf = String::new();
	stream
		.read_to_string(&mut buf)
		.expect("Failed to read response");

	Ok(buf)
}
