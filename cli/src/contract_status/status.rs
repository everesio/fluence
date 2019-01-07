/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use contract_func::ContractCaller;
use contract_status::app::{get_enqueued_apps, App};
use std::boxed::Box;
use std::error::Error;
use web3::types::Address;

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Status {
    enqueued_apps: Vec<App>,
}

impl Status {
    pub fn new(enqueued_apps: Vec<App>) -> Status {
        Status { enqueued_apps }
    }
}

/// Gets status about Fluence contract from ethereum blockchain.
pub fn get_status(contract_address: Address, eth_url: &str) -> Result<Status, Box<Error>> {
    let contract = ContractCaller::new(contract_address, eth_url)?;

    // TODO get more data
    let apps = get_enqueued_apps(&contract)?;

    Ok(Status::new(apps))
}