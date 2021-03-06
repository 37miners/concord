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

use crate::types::ConnectionInfo;
use crate::types::{AuthResponse, Event, EventBody};
use crate::{send, try2};
use concordconfig::ConcordConfig;
use concorddata::concord::DSContext;
use concorddata::concord::{AUTH_FLAG_MEMBER, AUTH_FLAG_OWNER, TOKEN_EXPIRATION};
use concorddata::types::Pubkey;
use concorderror::Error as ConcordError;
use concordutil::librustlet;
use librustlet::nioruntime_log;
use librustlet::*;
use nioruntime_log::*;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::{Arc, RwLock};

info!(); // set log level to debug
const MAIN_LOG: &str = "mainlog";

const NOT_AUTHORIZED: &str = "{\"error\": \"not authorized\"}";

#[derive(Serialize)]
struct TokenResponse {
	token: String,
}

#[derive(Serialize)]
struct ChallengeResponse {
	challenge: String,
}

pub fn ws_auth(
	handle: ConnData,
	event: &Event,
	ds_context: &DSContext,
	conn_info: Arc<RwLock<HashMap<u128, ConnectionInfo>>>,
) -> Result<bool, ConcordError> {
	let success;
	let id = handle.get_connection_id();
	let pubkey: Option<Pubkey>;

	match &event.body {
		EventBody::AuthEvent(auth_event) => match &auth_event.token.0 {
			Some(token) => {
				let token = token.0;
				success = ds_context.check_ws_auth_token(token)?;
				info!("success={}", success);
				send!(
					handle,
					Event {
						body: EventBody::AuthResponse(AuthResponse {
							redirect: None.into(),
							success,
						})
						.into(),
						..Default::default()
					}
				);
				if success {
					pubkey = Some(Pubkey::from_bytes(pubkey!()));
				} else {
					debug!("return true");
					return Ok(true);
				}
			}
			None => {
				let message = format!("{}", handle.get_connection_id());
				let message = message.as_bytes();
				let spec_pubkey = match &auth_event.pubkey.0 {
					Some(pubkey) => pubkey,
					None => {
						warn!("malformed event: no pubkey: {:?}", event);
						return Ok(true);
					}
				};

				let signature = match &auth_event.signature.0 {
					Some(signature) => signature,
					None => {
						warn!("malformed event: no signature: {:?}", event);
						return Ok(true);
					}
				};

				success = verify!(message, spec_pubkey.to_bytes(), signature.0).unwrap_or(false);
				info!("success w/sig={}", success);
				info!(
					"message={:?},spec_pubkey={:?},signature={:?}",
					message,
					spec_pubkey.to_bytes(),
					signature.0
				);
				send!(
					handle,
					Event {
						body: EventBody::AuthResponse(AuthResponse {
							redirect: None.into(),
							success,
						}),
						..Default::default()
					}
				);
				if success {
					pubkey = Some((*spec_pubkey).clone());
				} else {
					return Ok(true);
				}
			}
		},
		_ => {
			warn!("invalid auth event. Type not AuthEvent: {:?}", event);
			return Ok(true);
		}
	}

	if success {
		let mut conn_info = nioruntime_util::lockw!(conn_info)?;
		let info = conn_info.get_mut(&id);
		match info {
			Some(mut info) => {
				info.pubkey = pubkey;
			}
			None => {
				// already closed.
			}
		}
	}

	Ok(!success)
}

// initialize this module. Create rustlets, log info, open browser.
pub fn init_auth(cconfig: &ConcordConfig) -> Result<(), ConcordError> {
	let uri = format!("{}:{}", cconfig.host, cconfig.port);

	let ds_context = DSContext::new(cconfig.root_dir.clone())?;
	let auth_token: u128 = rand::random(); // generate a 128 bit auth token.

	// save the auth token for ws_auth
	try2!(
		ds_context.save_ws_auth_token(auth_token),
		"save auth token error"
	);

	let mut config = get_config_multi!(MAIN_LOG)?;

	// print auth token to stdout as well as log
	let prev_show_stdout = config.show_stdout;
	config.show_stdout = true;
	log_config_multi!(MAIN_LOG, config.clone())?;

	log_no_ts_multi!(
		INFO,
		MAIN_LOG,
		"-------------------------------------------------------------------------------------------------------------------------------"
	);

	// log to stdout/log for future reference.
	log_multi!(
		INFO,
		MAIN_LOG,
		"Authentication URL:   http://{}/auth?token={}",
		uri,
		auth_token,
	);
	log_multi!(INFO, MAIN_LOG, "Authentication Token: {}", auth_token,);

	log_no_ts_multi!(
                INFO,
                MAIN_LOG,
                "-------------------------------------------------------------------------------------------------------------------------------"
        );

	config.show_stdout = prev_show_stdout;
	log_config_multi!(MAIN_LOG, config)?;

	// auth on this concord instance
	rustlet!("auth", {
		let token = query!("token").unwrap_or("".to_string());

		if token.parse().unwrap_or(0) == auth_token.clone() {
			let user_pubkey = pubkey!();
			let challenge = ds_context.create_auth_challenge(user_pubkey).map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(format!(
					"Error with auth challenge generation: {}",
					e.to_string()
				))
				.into();
				error
			})?;
			let token = ds_context
				.validate_challenge(
					user_pubkey,
					user_pubkey,
					challenge,
					u128::MAX, // never expire
					AUTH_FLAG_OWNER | AUTH_FLAG_MEMBER,
				)
				.map_err(|e| {
					let error: Error =
						ErrorKind::ApplicationError(format!("Error with auth: {}", e.to_string()))
							.into();
					error
				})?;

			match token {
				Some(token) => {
					set_cookie!("auth", &token, "Expires=Fri, 01 Jan 2100 01:00:00 GMT;");
				}
				None => {}
			}
		}
		// we redirect in either case. App will handle invalid token display issues.
		set_redirect!("/");
	});
	rustlet_mapping!("/auth", "auth");

	let ds_context = DSContext::new(cconfig.root_dir.clone())?;

	rustlet!("get_challenge", {
		let user_pubkey = query!("user_pubkey").unwrap_or("".to_string());
		let user_pubkey = urlencoding::decode(&user_pubkey)?;
		let user_pubkey = base64::decode(&*user_pubkey)?;
		let user_pubkey: [u8; 32] = user_pubkey.as_slice().try_into()?;
		let challenge = ds_context.create_auth_challenge(user_pubkey).map_err(|e| {
			let error: Error = ErrorKind::ApplicationError(format!(
				"create auth challenge error: {}",
				e.to_string()
			))
			.into();
			error
		})?;

		let challenge = base64::encode(challenge);
		let challenge = urlencoding::encode(&challenge).to_string();

		let challenge = ChallengeResponse { challenge };
		let json = serde_json::to_string(&challenge).map_err(|e| {
			let error: Error =
				ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string())).into();
			error
		})?;
		response!("{}", json);
	});
	rustlet_mapping!("/get_challenge", "get_challenge");

	let ds_context = DSContext::new(cconfig.root_dir.clone())?;

	rustlet!("challenge_auth", {
		let server_pubkey = pubkey!();
		let user_pubkey = query!("user_pubkey").unwrap_or("".to_string());
		let user_pubkey = urlencoding::decode(&user_pubkey)?;
		let user_pubkey = base64::decode(&*user_pubkey)?;
		let user_pubkey: [u8; 32] = user_pubkey.as_slice().try_into()?;

		let challenge = query!("challenge").unwrap_or("".to_string());
		let challenge = urlencoding::decode(&challenge)?;
		let challenge = base64::decode(&*challenge)?;
		let challenge: [u8; 8] = challenge.as_slice().try_into()?;

		let signature = query!("signature").unwrap_or("".to_string());
		let signature = urlencoding::decode(&signature)?;
		let signature = base64::decode(&*signature)?;
		let signature: [u8; 64] = signature.as_slice().try_into()?;

		let verification = verify!(&challenge, user_pubkey, signature);
		let verification = verification.unwrap_or(false);

		if verification {
			let valid = ds_context
				.validate_challenge(
					user_pubkey,
					server_pubkey,
					challenge,
					TOKEN_EXPIRATION,
					AUTH_FLAG_MEMBER,
				)
				.map_err(|e| {
					let error: Error = ErrorKind::ApplicationError(format!(
						"valid auth challenge error: {}",
						e.to_string()
					))
					.into();
					error
				})?;
			if valid.is_some() {
				let token = TokenResponse {
					token: valid.unwrap(),
				};

				let json = serde_json::to_string(&token).map_err(|e| {
					let error: Error =
						ErrorKind::ApplicationError(format!("Json Error: {}", e.to_string()))
							.into();
					error
				})?;

				response!("{}", json);
			} else {
				response!("{}", NOT_AUTHORIZED);
			}
		} else {
			response!("{}", NOT_AUTHORIZED);
		}
	});
	rustlet_mapping!("/challenge_auth", "challenge_auth");

	// open the web browser
	//webbrowser::open(&format!("http://{}/auth?token={}", uri, auth_token))?;

	// create purge thread
	let ds_context = DSContext::new(cconfig.root_dir.clone())?;
	std::thread::spawn(move || loop {
		match ds_context.purge_tokens() {
			Ok(_) => {}
			Err(e) => {
				log_multi!(
					ERROR,
					MAIN_LOG,
					"Purge thread generated error: {}",
					e.to_string(),
				);
			}
		}

		std::thread::sleep(std::time::Duration::from_millis(1000 * 60 * 5));
	});

	Ok(())
}
