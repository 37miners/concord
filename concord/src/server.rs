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
use concorddata::concord::ServerInfo;
use concorderror::Error as ConcordError;
use crate::auth::check_auth;
use librustlet::*;

use std::fs::File;
use std::io::Read;

nioruntime_log::debug!(); // set log level to debug

#[derive(Serialize, Deserialize)]
struct ServerInfoMin {
        name: String,
        address: String,
        id: String,
}

pub fn init_server(root_dir: String) -> Result<(), ConcordError> {
	// create a ds context. Each rustlet needs it's own
	let ds_context = DSContext::new(root_dir.clone())?;

	// each rustlet also needs a root_dir_clone
	let root_dir_clone = root_dir.clone();
	let root_dir_clone2 = root_dir.clone();
	let root_dir_clone3 = root_dir.clone();
	let root_dir_clone4 = root_dir.clone();
	let root_dir_clone5 = root_dir.clone();

        // create a server on this concord instance
        rustlet!("create_server", {
		// make sure we're authenticated
		if ! check_auth(&root_dir_clone) {
			return Ok(());
		}

		// get query parameters
                let query = request!("query");
                let query_vec = querystring::querify(&query);
                let mut name = "".to_string();
                for query_param in query_vec {
                        if query_param.0 == "name" {
                                name = query_param.1.to_string();
                                break;
                        }
                }

                let content = request_content!();
                let content = &mut &content[..];
                let mut headers = hyper::header::Headers::new();
                for i in 0..header_len!() {
                        headers.append_raw(header_name!(i), header_value!(i).as_bytes().to_vec());
                }
		// parse the mime_multipart data in this request
                let res = mime_multipart::read_multipart_body(content, &headers, false).unwrap_or(vec![]);
                for node in &res {
                        match node {
                                mime_multipart::Node::File(filepart) => {
                                        let mut f = File::open(&filepart.path)?;
                                        let size = filepart.size.unwrap_or(0);
                                        let mut buf = vec![0 as u8; size];
                                        f.read(&mut buf)?;
                                        let server_info = ServerInfo {
                                                address: "address".to_string(),
                                                name: name.clone(),
                                                icon: buf,
                                        };

                                        ds_context.add_server(server_info).map_err(|e| {
                                                let error: Error = ErrorKind::ApplicationError(format!(
                                                        "error adding server: {}",
                                                        e.to_string()
                                                ))
                                                .into();
                                                error
                                        })?;
                                        break;
                                }
                                _ => {}
                        }
                }
        });
        rustlet_mapping!("/create_server", "create_server");

        // create a new context for each rustlet, synchronization handled by batches
        let ds_context = DSContext::new(root_dir.clone())?;

        // get all servers associated with this instance of concord
        rustlet!("get_servers", {
		// make sure we're authenticated
                if ! check_auth(&root_dir_clone2) {
                        return Ok(());
                }

                let servers = ds_context.get_servers().map_err(|e| {
                        let error: Error = ErrorKind::ApplicationError(
                                format!(
                                        "Error getting servers: {}",
                                        e.to_string()
                                )
                        ).into();
                        error
                })?;

                let mut server_json = vec![];
                for server in servers {
                        server_json.push(
                                ServerInfoMin {
                                        name: server.0.name.clone(),
                                        address: server.0.address.clone(),
                                        id: server.1,
                                }
                        );
                }
                let json = serde_json::to_string(&server_json).map_err(|e| {
                        let error: Error = ErrorKind::ApplicationError(
                                format!("Json Error: {}", e.to_string())
                        ).into();
                        error
                })?;
                response!("{}", json);

        });
        rustlet_mapping!("/get_servers", "get_servers");

        // create a new context for each rustlet, synchronization handled by batches
        let ds_context = DSContext::new(root_dir.clone())?;

        // get the icon for the specified server
        rustlet!("get_server_icon", {
		// make sure we're authenticated
                if ! check_auth(&root_dir_clone3) {
                        return Ok(());
                }
                let query = request!("query");
                let query_vec = querystring::querify(&query);
                let mut id = "".to_string();
                for query_param in query_vec {
                        if query_param.0 == "id" {
                                id = query_param.1.to_string();
                                break;
                        }
                }

                let sinfo = ds_context.get_server_info(id).map_err(|e| {
                        let error: Error = ErrorKind::ApplicationError(
                                format!("error getting server info: {}", e.to_string())
                        ).into();
                        error
                })?;

                match sinfo {
                        Some(sinfo) => {
				// write back with binary method
                                bin_write!(&sinfo.icon[..]);
                        },
                        None => {

                        },
                }
        });
        rustlet_mapping!("/get_server_icon", "get_server_icon");

	// create a new context for each rustlet, synchronization handled by batches
        let ds_context = DSContext::new(root_dir.clone())?;

        // delete the specified server
        rustlet!("delete_server", {
		// make sure we're authenticated
                if ! check_auth(&root_dir_clone4) {
                        return Ok(());
                }

		// parse query
                let query = request!("query");
                let query_vec = querystring::querify(&query);
                let mut id = "".to_string();
                for query_param in query_vec {
                        if query_param.0 == "id" {
                                id = query_param.1.to_string();
                                break;
                        }
                }

                ds_context.delete_server(id).map_err(|e| {
                        let error: Error = ErrorKind::ApplicationError(
                                format!("error deleting server: {}", e.to_string())
                        ).into();
                        error
                })?;
        });
        rustlet_mapping!("/delete_server", "delete_server");

	// create a new context for each rustlet, synchronization handled by batches
        let ds_context = DSContext::new(root_dir.clone())?;

        // modify the specified server
        rustlet!("modify_server", {
                if ! check_auth(&root_dir_clone5) {
                        return Ok(());
                }
                let query = request!("query");
                let query_vec = querystring::querify(&query);
                let mut id = "".to_string();
                let mut name = "".to_string();
                for query_param in query_vec {
                        if query_param.0 == "id" {
                                id = query_param.1.to_string();
                        } else if query_param.0 == "name" {
                                name = query_param.1.to_string();
                        }
                }

                let content = request_content!();
                let content = &mut &content[..];
                let mut headers = hyper::header::Headers::new();
                for i in 0..header_len!() {
                        headers.append_raw(header_name!(i), header_value!(i).as_bytes().to_vec());
                }
                let res = mime_multipart::read_multipart_body(content, &headers, false).unwrap_or(vec![]);
                for node in &res {
                        match node {
                                mime_multipart::Node::File(filepart) => {
                                        let mut f = File::open(&filepart.path)?;
                                        let size = filepart.size.unwrap_or(0);
                                        let mut buf = vec![0 as u8; size];
                                        f.read(&mut buf)?;
                                        let server_info = ServerInfo {
                                                address: "address".to_string(),
                                                name: name.clone(),
                                                icon: buf,
                                        };

                                        ds_context.modify_server(id.clone(), server_info).map_err(|e| {
                                                let error: Error = ErrorKind::ApplicationError(format!(
                                                        "error modifying server: {}",
                                                        e.to_string()
                                                ))
                                                .into();
                                                error
                                        })?;
                                        break;
                                }
                                _ => {}
                        }
                }
        });
        rustlet_mapping!("/modify_server", "modify_server");

	Ok(())
}


