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

use concorderror::Error as ConcordError;
use concorddata::concord::DSContext;
use librustlet::*;
use nioruntime_log::*;

nioruntime_log::debug!(); // set log level to debug
const MAIN_LOG: &str = "mainlog";

// not auth json message
const NOT_AUTHORIZED: &str = "{\"error\": \"not authorized\"}";

// check whether a session is authorized. We assume that we are in the rustlet context
// here.
pub fn check_auth(root_dir: &String) -> bool {
	let auth = cookie!("auth"); // get auth cookie
	let ds_context = DSContext::new(root_dir.clone());
	match ds_context {
		Ok(_) => {},
		Err(e) => {
			log_multi!(ERROR, MAIN_LOG, "Could not obtain ds context: {}", e.to_string());
			response!("{}", NOT_AUTHORIZED);
			return false;
		},
	}

	// if there's no auth cookie, we're not authed
	if auth.is_none() {
		response!("{}", NOT_AUTHORIZED);
		return false;
	}

	// ok because we checked is_none and returned
	let auth = auth.unwrap();

	// ok because we checked is_err
	let ds_context = ds_context.unwrap();

	// check auth cookie value in db
	let res = ds_context.check_auth_cookie(auth);

	match res {
		Ok(_) => {},
		Err(e) => {
			log_multi!(ERROR, MAIN_LOG, "check_auth_cookie returned error: {}", e.to_string());
			response!("{}", NOT_AUTHORIZED);
			return false;
		},
	}

	// ok because we checked for error and returned
	let res = res.unwrap();

	// if we're not authed return error message
	if !res {
		response!("{}", NOT_AUTHORIZED);
	}

	// otherwise we just return true

	return res;
}

// initialize this module. Create rustlets, log info, open browser.
pub fn init_auth(root_dir: String, uri: String) -> Result<(), ConcordError> {
	let ds_context = DSContext::new(root_dir.clone())?;
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
        log_multi!(
                INFO,
                MAIN_LOG,
                "Authentication Token: {}",
                auth_token,
        );

        log_no_ts_multi!(
                INFO,
                MAIN_LOG,
                "-------------------------------------------------------------------------------------------------------------------------------"
        );

	config.show_stdout = false;
        log_config_multi!(MAIN_LOG, config)?;


        // auth on this concord instance
        rustlet!("auth", {
                let query = request!("query");
                let query_vec = querystring::querify(&query);
                let mut token = "".to_string();
                for query_param in query_vec {
                        if query_param.0 == "token" {
                                token = query_param.1.to_string();
                                break;
                        }
                }

		if token.parse().unwrap_or(0) == auth_token.clone() {
			let auth: u128 = rand::random();
			let auth = &format!("{}", auth);
			set_cookie!("auth", auth, "Expires=Fri, 01 Jan 2100 01:00:00 GMT;");
			ds_context.add_auth_cookie(auth.to_string()).map_err(|e| {
				let error: Error = ErrorKind::ApplicationError(
					format!("Error adding auth cookie: {}", e.to_string())
				).into();
				error
			})?;
		} 
		// we redirect in either case. App will handle invalid token display issues.
		set_redirect!("/");
        });
        rustlet_mapping!("/auth", "auth");

	// open the web browser	
	//webbrowser::open(&format!("http://{}/auth?token={}", uri, auth_token))?;

	Ok(())
}


