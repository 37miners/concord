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
use concorderror::Error as ConcordError;
use librustlet::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

nioruntime_log::debug!(); // set log level to debug

struct MapEntry {
	count: u64,
	//ac: AsyncContext,
	ac: RustletAsyncContext,
}

pub fn init_persistence(config: &ConcordConfig) -> Result<(), ConcordError> {
	// create a ds context. Each rustlet needs its own
	let ds_context = DSContext::new(config.root_dir.clone())?;
	let map = Arc::new(RwLock::new(HashMap::new()));
	let map_clone = map.clone();

	rustlet!("subscribe", {
		response!("first message\n");
		flush!();

		let ac = async_context!();
		let v: u64 = rand::random();
		let mut map = map_clone.write().unwrap();
		map.insert(v, MapEntry { count: 0, ac });
	});
	rustlet_mapping!("/subscribe", "subscribe");

	std::thread::spawn(move || loop {
		let mut rem_list = vec![];
		let mut map = map.write().unwrap();
		println!("map.len() = {}", map.len());

		for (k, me) in &mut *map {
			let ac = me.ac.clone();
			let count = me.count;
			async_context!(ac);
			response!("message {}\n", count);
			me.count += 1;
			flush!();

			if me.count == 25 {
				rem_list.push(k.to_owned());
				async_complete!();
			}
		}

		for k in rem_list {
			map.remove(&k);
		}
		std::thread::sleep(std::time::Duration::from_millis(1000));
	});

	Ok(())
}
