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

use concorddata::concord::DSContext;
use concorddata::concord::Channel;
use concorddata::concord::ChannelKey;
use concorderror::Error as ConcordError;
use std::convert::TryInto;
use librustlet::*;

nioruntime_log::debug!(); // set log level to debug

#[derive(Serialize, Deserialize)]
struct ChannelInfo {
        name: String,
        description: String,
        id: String,
}

pub fn init_channels(root_dir: String) -> Result<(), ConcordError> {
        // create a ds context. Each rustlet needs it's own
        let ds_context = DSContext::new(root_dir.clone())?;

	// set channel information
        rustlet!("set_channel", {
                // get query parameters
                let query = request!("query");
                let query_vec = querystring::querify(&query);

                let mut server_pubkey: Option<[u8; 32]> = None;
                let mut server_id: Option<[u8; 8]> = None;
                let mut channel_id: Option<u64> = None;
		let mut name: Option<String> = None;
		let mut description: Option<String> = None;
                for query_param in query_vec {
                        let param_as_str = query_param.1.to_string();
                        if query_param.0 == "server_pubkey" {
                                let local_server_pubkey = urlencoding::decode(&param_as_str)?;
                                let local_server_pubkey = base64::decode(&*local_server_pubkey)?;
                                server_pubkey = Some(local_server_pubkey.as_slice().try_into()?);
                        } else if query_param.0 == "server_id" {
                                let local_server_id = urlencoding::decode(&param_as_str)?;
                                let local_server_id = base64::decode(&*local_server_id)?;
                                server_id = Some(local_server_id.as_slice().try_into()?);
                        } else if query_param.0 == "channel_id" {
				channel_id = Some(param_as_str.parse()?);
			} else if query_param.0 == "name" {
				name = Some(param_as_str);
			} else if query_param.0 == "description" {
				description = Some(param_as_str);
			}
		}

		// if channel_id is not specified, we create one.
		let channel_id = match channel_id {
			Some(channel_id) => channel_id,
			None => rand::random(),
		};

		if name.is_none() {
			response!("name must be specified!");
			return Ok(());
		}

		let name = name.unwrap();

		if description.is_none() {
			response!("description must be specified!");
			return Ok(());
		}

		let description = description.unwrap();

		if server_id.is_none() {
			response!("server_id must be specified!");
			return Ok(());
		}

		let server_id = server_id.unwrap();

		let server_pubkey = match server_pubkey {
			Some(server_pubkey) => server_pubkey,
			None => {
				let server_pubkey = pubkey!();
				match server_pubkey {
					Some(server_pubkey) => server_pubkey,
					None => {
						response!("tor pubkey not configured!");
						return Ok(());
					},
				}
			},
		};

		let channel_key = ChannelKey {
			channel_id,
			server_id,
			server_pubkey,
		};

		let channel = Channel {
			name,
			description,
			channel_id,
		};

                ds_context.set_channel(channel_key, channel).map_err(|e| {
                        let error: Error = ErrorKind::ApplicationError(
                                format!(
                                        "Error setting channel: {}",
                                        e.to_string()
                                )
                        ).into();
                        error
                })?;

	});
	rustlet_mapping!("/set_channel", "set_channel");

	// create a ds context. Each rustlet needs it's own
        let ds_context = DSContext::new(root_dir.clone())?;

	// delete a channel
	rustlet!("delete_channel", {
                // get query parameters
                let query = request!("query");
                let query_vec = querystring::querify(&query);

                let mut server_pubkey: Option<[u8; 32]> = None;
                let mut server_id: Option<[u8; 8]> = None;
                let mut channel_id: Option<u64> = None;

                for query_param in query_vec {
                        let param_as_str = query_param.1.to_string();
                        if query_param.0 == "server_pubkey" {
                                let local_server_pubkey = urlencoding::decode(&param_as_str)?;
                                let local_server_pubkey = base64::decode(&*local_server_pubkey)?;
                                server_pubkey = Some(local_server_pubkey.as_slice().try_into()?);
                        } else if query_param.0 == "server_id" {
                                let local_server_id = urlencoding::decode(&param_as_str)?;
                                let local_server_id = base64::decode(&*local_server_id)?;
                                server_id = Some(local_server_id.as_slice().try_into()?);
                        } else if query_param.0 == "channel_id" {
                                channel_id = Some(param_as_str.parse()?);
                        }
                }

                if server_id.is_none() {
                        response!("server_id must be specified!");
                        return Ok(());
                }

                let server_id = server_id.unwrap();

                let server_pubkey = match server_pubkey {
                        Some(server_pubkey) => server_pubkey,
                        None => {
                                let server_pubkey = pubkey!();
                                match server_pubkey {
                                        Some(server_pubkey) => server_pubkey,
                                        None => {
                                                response!("tor pubkey not configured!");
                                                return Ok(());
                                        },
                                }
                        },
                };

		if channel_id.is_none() {
			response!("channel_id must be specified!");
			return Ok(());
		}

		let channel_id = channel_id.unwrap();

                let channel_key = ChannelKey {
                        channel_id,
                        server_id,
                        server_pubkey,
                };

                ds_context.delete_channel(channel_key).map_err(|e| {
                        let error: Error = ErrorKind::ApplicationError(
                                format!(
                                        "Error deleting channel: {}",
                                        e.to_string()
                                )
                        ).into();
                        error
                })?;
	});

	rustlet_mapping!("/delete_channel", "delete_channel");

        // create a ds context. Each rustlet needs it's own
        let ds_context = DSContext::new(root_dir.clone())?;

	rustlet!("get_channels", {
                let query = request!("query");
                let query_vec = querystring::querify(&query);

                let mut server_pubkey: Option<[u8; 32]> = None;
                let mut server_id: Option<[u8; 8]> = None;

                for query_param in query_vec {
                        let param_as_str = query_param.1.to_string();
                        if query_param.0 == "server_pubkey" {
                                let local_server_pubkey = urlencoding::decode(&param_as_str)?;
                                let local_server_pubkey = base64::decode(&*local_server_pubkey)?;
                                server_pubkey = Some(local_server_pubkey.as_slice().try_into()?);
                        } else if query_param.0 == "server_id" {
                                let local_server_id = urlencoding::decode(&param_as_str)?;
                                let local_server_id = base64::decode(&*local_server_id)?;
                                server_id = Some(local_server_id.as_slice().try_into()?);
                        }
		}

		let server_pubkey = if server_pubkey.is_none() {
			let pubkey = pubkey!();
			if pubkey.is_none() {
				response!("tor not configured!");
				return Ok(());
			}

			pubkey.unwrap()
		} else {
			server_pubkey.unwrap()
		};

		if server_id.is_none() {
			response!("server id must be specified!");
			return Ok(());
		}

		let server_id = server_id.unwrap();
                let channels = ds_context.get_channels(server_pubkey, server_id).map_err(|e| {
                        let error: Error = ErrorKind::ApplicationError(
                                format!(
                                        "Error getting channels: {}",
                                        e.to_string()
                                )
                        ).into();
                        error
                })?;

                let mut channel_json = vec![];
                for channel in channels {
                        channel_json.push(
                                ChannelInfo {
                                        name: channel.name,
                                        description: channel.description,
                                        id: format!("{}", channel.channel_id),
                                }
                        );
                }
                let json = serde_json::to_string(&channel_json).map_err(|e| {
                        let error: Error = ErrorKind::ApplicationError(
                                format!("Json Error: {}", e.to_string())
                        ).into();
                        error
                })?;
                response!("{}", json);

	});

	rustlet_mapping!("/get_channels", "get_channels");

	Ok(())
}

