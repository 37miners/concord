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

use crate::conn_manager::ConnManager;
use crate::types::{ConnectionInfo, Event};
use concordconfig::ConcordConfig;
use concorddata::concord::DSContext;
use concorderror::Error;
use std::sync::{Arc, RwLock};

pub fn get_messages(
	_conn_info: &ConnectionInfo,
	_ds_context: &DSContext,
	_event: &Event,
	_conn_manager: Arc<RwLock<ConnManager>>,
	_config: &ConcordConfig,
) -> Result<bool, Error> {
	Ok(false)
}

pub fn send_message(
	_conn_info: &ConnectionInfo,
	_ds_context: &DSContext,
	_event: &Event,
	_conn_manager: Arc<RwLock<ConnManager>>,
	_config: &ConcordConfig,
) -> Result<bool, Error> {
	Ok(false)
}

pub fn subscribe_channel(
	_conn_info: &ConnectionInfo,
	_ds_context: &DSContext,
	_event: &Event,
	_conn_manager: Arc<RwLock<ConnManager>>,
	_config: &ConcordConfig,
) -> Result<bool, Error> {
	Ok(false)
}
