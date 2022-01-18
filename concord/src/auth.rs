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

use concordconfig::ConcordConfig;
use concorddata::concord::DSContext;
use concorddata::concord::{AUTH_FLAG_MEMBER, AUTH_FLAG_OWNER, TOKEN_EXPIRATION};
use concorderror::Error as ConcordError;
use librustlet::*;
use nioruntime_log::*;
use std::borrow::Cow::Borrowed;
use std::convert::TryInto;

nioruntime_log::debug!(); // set log level to debug
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

// check whether a session is authorized. We assume that we are in the rustlet context
// here.
pub fn check_auth(ds_context: &DSContext, auth_flag: u64) -> Result<(), ConcordError> {
	let token = match cookie!("auth") {
		Some(auth) => auth,
		None => {
			query!("token")
		}
	};

	let user_pubkey = query!("user_pubkey");
	let user_pubkey = if user_pubkey != "".to_string() {
		let user_pubkey = urlencoding::decode(&user_pubkey)
			.unwrap_or(Borrowed(""))
			.to_string();
		base64::decode(user_pubkey).unwrap_or([0u8; 32].to_vec())[..]
			.try_into()
			.unwrap_or([0u8; 32])
	} else {
		pubkey!().unwrap_or([0u8; 32])
	};

	let server_pubkey = query!("server_pubkey");
	let server_pubkey = if server_pubkey != "".to_string() {
		let server_pubkey = urlencoding::decode(&server_pubkey)
			.unwrap_or(Borrowed(""))
			.to_string();
		base64::decode(server_pubkey).unwrap_or([0u8; 32].to_vec())[..]
			.try_into()
			.unwrap_or([0u8; 32])
	} else {
		pubkey!().unwrap_or([0u8; 32])
	};

	let server_id = query!("server_id");
	let server_id = urlencoding::decode(&server_id)
		.unwrap_or(Borrowed(""))
		.to_string();
	let server_id = base64::decode(server_id).unwrap_or([0u8; 8].to_vec())[..]
		.try_into()
		.unwrap_or([0u8; 8]);

	ds_context.is_authorized(
		user_pubkey,
		server_pubkey,
		token.parse().unwrap_or(0),
		server_id,
		auth_flag,
	)?;

	Ok(())
}

// initialize this module. Create rustlets, log info, open browser.
pub fn init_auth(cconfig: &ConcordConfig) -> Result<(), ConcordError> {
	let uri = format!("{}:{}", cconfig.host, cconfig.port);

	let ds_context = DSContext::new(cconfig.root_dir.clone())?;
	let auth_token: u128 = rand::random(); // generate a 128 bit auth token.

	let mut config = get_config_multi!(MAIN_LOG)?;

	// print auth token to stdout as well as log
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

	config.show_stdout = false;
	log_config_multi!(MAIN_LOG, config)?;

	// auth on this concord instance
	rustlet!("auth", {
		let token = query!("token");

		if token.parse().unwrap_or(0) == auth_token.clone() {
			let user_pubkey = pubkey!().unwrap_or([0u8; 32]);
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
		let user_pubkey = query!("user_pubkey");
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
		let server_pubkey = pubkey!().unwrap_or([0u8; 32]);
		let user_pubkey = query!("user_pubkey");
		let user_pubkey = urlencoding::decode(&user_pubkey)?;
		let user_pubkey = base64::decode(&*user_pubkey)?;
		let user_pubkey: [u8; 32] = user_pubkey.as_slice().try_into()?;

		let challenge = query!("challenge");
		let challenge = urlencoding::decode(&challenge)?;
		let challenge = base64::decode(&*challenge)?;
		let challenge: [u8; 8] = challenge.as_slice().try_into()?;

		let signature = query!("signature");
		let signature = urlencoding::decode(&signature)?;
		let signature = base64::decode(&*signature)?;
		let signature: [u8; 64] = signature.as_slice().try_into()?;

		let verification = verify!(&challenge, Some(user_pubkey), signature);
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
