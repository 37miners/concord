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

#[macro_use]
extern crate serde_derive;

pub mod client;
pub mod types;

mod auth;
mod channel;
mod concord;
mod conn_manager;
mod invite;
mod members;
mod message;
mod profile;
mod server;
mod ws;

#[macro_use]
mod utils;

pub use crate::concord::concord_init;
pub use concordutil::librustlet;
