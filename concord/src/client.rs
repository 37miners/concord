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

use crate::librustlet::nioruntime_http::{build_messages, WebSocketMessageType};
use crate::librustlet::WebSocketMessage;
use crate::types::{AuthEvent, Event, EventType};
use concorddata::ser::serialize_default;
use concorddata::types::U128;
use concorderror::{Error, ErrorKind};
use concordutil::nioruntime_log;
use nioruntime_log::*;
use std::io::prelude::*;
use std::net::*;
use std::pin::Pin;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use tor_stream::TorStream;

debug!();

/// This struct is used to send messages in the [`WSListenerClient`] closure.
/// See the examples there on how to use it.
pub struct WSListenerClientWriter {
	sender: SyncSender<Option<Event>>,
	stream: TcpStream,
}

impl WSListenerClientWriter {
	fn new(sender: SyncSender<Option<Event>>, stream: TcpStream) -> Self {
		Self { sender, stream }
	}

	/// Send an event to the server from within the callback closure.
	/// See examples in [`WSListenerClient`] for more details.
	pub fn send(&self, event: Event) -> Result<(), Error> {
		self.sender.send(Some(event))?;
		Ok(())
	}

	/// Close the [`WSListenerClient`] from within the callback closure.
	/// See examples in [`WSListenerClient`] for more details.
	pub fn close(&self) -> Result<(), Error> {
		self.sender.send(None)?; // None is used to indicate shutdown.

		// fully shutdown both send/receive side of the socket.
		self.stream.shutdown(std::net::Shutdown::Both)?;
		Ok(())
	}
}

/// Websocket listener client for concord. This struct can be used to communicate with
/// A concord server.
///
/// # Examples
/// ```
/// use concorderror::Error;
/// use concordutil::nioruntime_log as nioruntime_log;
/// use nioruntime_log::*;
/// use concordlib::client::WSListenerClient;
/// use concordlib::types::{Event,EventType,GetServersEvent};
/// use std::sync::{RwLock,Arc};
///
/// debug!();
///
/// fn test() -> Result<(), Error> {
///        // create a WS Listener Client.
///        // onion address, tor proxy port, and an optional token is specified.
///        let mut client = WSListenerClient::new(
///                "shvl2mrbmd7lbbunvulsfom3d6kfvvgmkczev4qnymbojswtofbqrdad.onion".to_string(),
///                11998,
///                Some("325980786977777586718199287994764265498".to_string()),
///        );
///
///        let time_now = std::time::Instant::now();
///        let time = Arc::new(RwLock::new(time_now));
///
///        // set a callback which is called when the server sends the client an event.
///        client.set_callback(move |event, writer| {
///                let mut time = match time.write() {
///                        Ok(time) => time,
///                        Err(e) => {
///                                error!("obtaining time write lock generated error: {}", e);
///                                return Ok(());
///                       },
///                };
///                debug!("elapsed time since last event={}ms", (*time).elapsed().as_nanos() as f64 / 1_000_000 as f64);
///                *time = std::time::Instant::now();
///
///                // respond to specific event types
///                match event.event_type {
///                        EventType::AuthResponse => {
///                                let event = Event {
///                                        event_type: EventType::GetServersEvent,
///                                        get_servers_event: Some(GetServersEvent {
///                                        }).into(),
///                                        ..Default::default()
///                                };
///
///                                // send an event to the server
///                                writer.send(event)?;
///                                info!("Processing auth message");
///                        },
///                        EventType::GetServersResponse => {
///                                info!("Got a servers response: {:?}", event);
///                                // close the connection and free all resources
///                                writer.close()?;
///                        },
///                        _ => {
///                                error!("Unexpected event type: {:?}", event);
///                        },
///                }
///
///                Ok(())
///        })?;
///
///        // set an error callback handler
///        client.set_error(move |e| {
///                error!("got an error: {}", e);
///                Ok(())
///        })?;
///
///        // start the client
///        client.start()?;
///
///        // park the thread so that the client doesn't immediately exit.
///        std::thread::park();
///	Ok(())
/// }
/// ```
pub struct WSListenerClient<Callback, ErrHandler> {
	callback: Option<Pin<Box<Callback>>>,
	error: Option<Pin<Box<ErrHandler>>>,
	onion: String,
	tor_proxy_port: u16,
	sender: Option<SyncSender<Option<Event>>>,
	token: Option<String>,
}

impl<Callback, ErrHandler> WSListenerClient<Callback, ErrHandler>
where
	Callback: Fn(&Event, &WSListenerClientWriter) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	ErrHandler: Fn(Error) -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,
{
	/// Create a new WSListenerClient connection to the onion address specified using the specified
	/// tor_proxy_port.
	/// Optional token is specified. TODO: implement optional private key for the other method of authentication.
	pub fn new(onion: String, tor_proxy_port: u16, token: Option<String>) -> Self {
		Self {
			callback: None,
			error: None,
			onion,
			tor_proxy_port,
			sender: None,
			token,
		}
	}

	/// Set the callback for this WSListenerClient. See [`WSListenerClient`] for an example.
	pub fn set_callback(&mut self, callback: Callback) -> Result<(), Error> {
		self.callback = Some(Box::pin(callback));
		Ok(())
	}

	/// Set the error handler for this WSListenerClient. See [`WSListenerClient`] for an example.
	pub fn set_error(&mut self, error: ErrHandler) -> Result<(), Error> {
		self.error = Some(Box::pin(error));
		Ok(())
	}

	/// Start the WSListenerClient. See [`WSListenerClient`] for an example.
	pub fn start(&mut self) -> Result<(), Error> {
		let sec_value: [u8; 4] = rand::random();
		let tor_proxy_port = self.tor_proxy_port;
		let onion = self.onion.clone();

		let (sender, receiver) = sync_channel(2);
		self.sender = Some(sender.clone());

		let callback = match self.callback.as_ref() {
			Some(callback) => callback,
			None => {
				error!("callback not initialized!");
				return Err(ErrorKind::NotInitialized(
					"'set_callback' function must be called prior to 'start'".to_string(),
				)
				.into());
			}
		}
		.clone();

		let error = match self.error.as_ref() {
			Some(error) => error,
			None => {
				error!("error callback not initialized!");
				return Err(ErrorKind::NotInitialized(
					"'set_error' function must be called prior to 'start'".to_string(),
				)
				.into());
			}
		}
		.clone();

		let error_clone = error.clone();

		let token = self.token.clone();

		let sec_value_base64 = base64::encode(sec_value);
		let proxy_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), tor_proxy_port);
		let target: socks::TargetAddr = socks::TargetAddr::Domain(onion, 80);
		let mut stream = TorStream::connect_with_address(proxy_addr, target)?;
		stream.write_all(
			format!(
				"GET /ws HTTP/1.1\r\n\
Host: localhost\r\n\
Upgrade: websocket\r\n\
Sec-WebSocket-Key: {}\
\r\n\r\n",
				sec_value_base64
			)
			.as_bytes(),
		)?;

		let stream = stream.into_inner();
		let stream_clone = stream.try_clone().unwrap();

		// start read thread
		std::thread::spawn(move || {
			let stream = stream.try_clone().unwrap();
			match Self::do_proxy_read_loop(stream, callback, token.clone(), sender) {
				Ok(_) => {}
				Err(e) => match (error)(e) {
					Ok(_) => {}
					Err(e) => {
						error!("error occurred in error callback: {}", e);
					}
				},
			}
		});

		// start write thread
		std::thread::spawn(move || {
			let stream = stream_clone.try_clone().unwrap();
			match Self::do_proxy_write_loop(&stream, receiver, &error_clone) {
				Ok(_) => {}
				Err(e) => match (error_clone)(e) {
					Ok(_) => {}
					Err(e) => {
						error!("error occurred in error callback: {}", e);
					}
				},
			}
		});

		Ok(())
	}

	/// Send an event to the server from this WSListenerClient.
	pub fn send(&self, event: Event) -> Result<(), Error> {
		match self.sender.as_ref() {
			Some(sender) => {
				sender.send(Some(event))?;
				Ok(())
			}
			None => Err(ErrorKind::NotInitialized(
				"'start' must be called before 'send'".to_string(),
			)
			.into()),
		}
	}

	/// Close the WSListenerClient freeing all it's resources.
	pub fn close(&self) -> Result<(), Error> {
		Ok(())
	}

	fn do_proxy_write_loop(
		mut stream: &TcpStream,
		receiver: Receiver<Option<Event>>,
		error: &Pin<Box<ErrHandler>>,
	) -> Result<(), Error> {
		loop {
			let event = receiver.recv()?;

			let event = match event {
				Some(event) => event,
				None => {
					// shutdown thread
					break;
				}
			};

			let mut buffer = vec![];
			serialize_default(&mut buffer, &event)?;
			let wsm: WebSocketMessage = WebSocketMessage {
				mtype: WebSocketMessageType::Binary,
				payload: buffer,
				mask: false,
				header_info: None,
			};
			let bin_data: Vec<u8> = wsm.into();
			stream.write(&bin_data)?;
		}

		// if we get here we were disconnected. Send an error event to the error
		// handler callback.
		match (error)(ErrorKind::Disconnect("Socket disconnect".to_string()).into()) {
			Ok(_) => {}
			Err(e) => {
				error!("error occurred in error callback: {}", e);
			}
		}
		Ok(())
	}

	fn do_proxy_read_loop(
		mut stream: TcpStream,
		callback: Pin<Box<Callback>>,
		token: Option<String>,
		sender: SyncSender<Option<Event>>,
	) -> Result<(), Error> {
		let mut buf = Self::skip_headers(&mut stream)?;
		let mut rbuf = [0u8; 4096];

		let mut writer = WSListenerClientWriter::new(sender, stream);

		loop {
			Self::process_buffer(&mut buf, &token, &callback, &writer)?;
			let len = writer.stream.read(&mut rbuf)?;
			if len == 0 {
				break;
			}
			buf.append(&mut rbuf[0..len].to_vec());
		}

		// make sure write thread closes too
		let _ = writer.close();
		Ok(())
	}

	fn process_buffer(
		buf: &mut Vec<u8>,
		token: &Option<String>,
		callback: &Pin<Box<Callback>>,
		writer: &WSListenerClientWriter,
	) -> Result<(), Error> {
		// try to build as many messages as we can.
		let messages = build_messages(buf)?;

		for message in messages {
			match message.mtype {
				WebSocketMessageType::Binary => {
					let mut cursor = std::io::Cursor::new(message.payload);
					cursor.set_position(0);
					let mut reader = concorddata::ser::BinReader::new(
						&mut cursor,
						concorddata::ser::ProtocolVersion::local(),
					);

					let event: Event = concorddata::ser::Readable::read(&mut reader)?;

					match event.event_type {
						EventType::ChallengeEvent => {
							// for now we only implement token auth
							let token = match token {
								Some(ref token) => Some(U128(token.parse()?)).into(),
								None => None.into(),
							};
							let auth_event = Event {
								event_type: EventType::AuthEvent,
								auth_event: Some(AuthEvent {
									pubkey: None.into(),
									signature: None.into(),
									token,
								})
								.into(),
								..Default::default()
							};
							writer.sender.send(Some(auth_event))?;
						}
						_ => (callback)(&event, writer)?,
					}
				}
				WebSocketMessageType::Close => {}
				_ => {}
			}
		}

		Ok(())
	}

	// skip over the headers returned by the server and return any additional
	// data.
	fn skip_headers(stream: &mut TcpStream) -> Result<Vec<u8>, Error> {
		let mut buf = [0u8; 4096]; // headers less than 4096 bytes
		let mut offset = 0;
		let mut len;
		let mut end = 0;
		loop {
			len = stream.read(&mut buf[offset..])?;
			for i in 3..len + offset {
				if buf[i - 3] == '\r' as u8
					&& buf[i - 2] == '\n' as u8
					&& buf[i - 1] == '\r' as u8
					&& buf[i] == '\n' as u8
				{
					end = i + 1;
					len = len + offset;
					// TODO confirm sec header is signed correctly.
					break;
				}
			}
			if end > 0 {
				break;
			}
			offset += len;
		}
		Ok((&buf[end..len]).to_vec())
	}
}
