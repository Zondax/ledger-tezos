/*******************************************************************************
 *   (c) 2021 Zondax GmbH
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 ********************************************************************************/

//! Test the unroll macro
use std::{cmp::Ordering, path::PathBuf};

use arrayref::array_ref;

use bolos::PIC;
use zemu_sys::zemu_log_stack;

use ledger_tezos_derive::unroll;

use serde::{Deserialize, Serialize};

unroll!("../app/vendor/BakersRegistryCoreUnfilteredData.json");

/// This structs represents the expected schematic of the baker dat
///
/// Here it's used to read data more easily for the tests
#[derive(Clone, Serialize, Deserialize)]
struct KnownBaker {
    #[serde(alias = "bakerName")]
    name: String,
    #[serde(alias = "bakerAccount")]
    addr: String,
}

#[test]
fn sorted_deduped() {
    let mut copy = Vec::from(KNOWN_BAKERS);
    copy.dedup();
    copy.sort_by(
        |(prefix_a, hash_a, _), (prefix_b, hash_b, _)| match prefix_a.cmp(prefix_b) {
            Ordering::Equal => hash_a.cmp(hash_b),
            ord => ord,
        },
    );

    assert_eq!(&copy[..], KNOWN_BAKERS);
}
#[test]
fn binary_search() {
    let (sample_prefix, sample_hash, _) = KNOWN_BAKERS[6];

    let idx = KNOWN_BAKERS
        .binary_search_by(
            |&(probe_prefix, probe_hash, _)| match probe_prefix.cmp(sample_prefix) {
                Ordering::Equal => probe_hash.cmp(sample_hash),
                ord => ord,
            },
        )
        .expect("couldn't find from list");

    assert_eq!(idx, 6)
}

#[test]
fn unroll_check() {
    let path = "../app/vendor/BakersRegistryCoreUnfilteredData.json";
    let file = std::fs::File::open(path).unwrap_or_else(|err| {
        panic!(
            "unable to open data file at: {:?}; err={:?}",
            PathBuf::from("../app/vendor").canonicalize(),
            err
        )
    });

    let data: Vec<KnownBaker> =
        serde_json::from_reader(file).expect("unable to read JSON data from file");

    let entry = &data[0];

    let addr = bs58::decode(&entry.addr)
        .into_vec()
        .expect("entry addr wasn't valid base58");
    let addr = addr.as_slice();

    let prefix = array_ref!(addr, 0, 3);
    let hash = array_ref!(addr, 3, 20);

    let name = baker_lookup(prefix, hash).expect("couldn't find baker in lookup");

    assert_eq!(name, entry.name.as_str());
}
