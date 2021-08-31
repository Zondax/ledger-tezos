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
use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};
use std::prelude::v1::*;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::parser::operations::Operation;

use super::operations::OperationType;

fn data_dir_path() -> PathBuf {
    std::env::var_os("TEZOS_TEST_DATA")
        .unwrap_or_else(|| "../../zemu/tests/data".to_string().into())
        .into()
}

fn test_vectors_path() -> PathBuf {
    std::env::var_os("TEZOS_TEST_VECTORS")
        .unwrap_or_else(|| "../../zemu/test-vectors".to_string().into())
        .into()
}

fn get_json_from_data<P, T>(path: P) -> T
where
    P: AsRef<Path>,
    T: DeserializeOwned,
{
    let file = File::open(&path)
        .unwrap_or_else(|e| panic!("couldn't read file at {:?}; err: {:?}", path.as_ref(), e));

    serde_json::from_reader(file)
        .unwrap_or_else(|e| panic!("couldn't parse json at {:?}; err: {:?}", path.as_ref(), e))
}

#[derive(Clone, Serialize, Deserialize)]
struct JsonOperation {
    branch: String,
    contents: Vec<Map<String, Value>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Sample {
    #[serde(default)]
    name: String,
    operation: JsonOperation,
    blob: String,
}

fn test_sample(name: &str, blob: String, branch: String, contents: Vec<Map<String, Value>>) {
    let blob = hex::decode(&blob)
        .unwrap_or_else(|e| panic!("sample {} .blob wasn't a hex string; err: {:?}", name, e));

    //parse forged op blob
    let mut parsed = Operation::new(&blob)
        .unwrap_or_else(|e| panic!("sample {} couldn't be parsed; err: {:?}", name, e));

    let mut branch_bs58 = [0; 51];
    parsed.base58_branch(&mut branch_bs58).unwrap_or_else(|e| {
        panic!(
            "couldn't compute base 58 branch of sample {}; err: {:?}",
            name, e
        )
    });

    assert_eq!(branch_bs58, branch.as_bytes());

    //retrieve ops from parsed and also ops in the operation
    let ops = parsed.mut_ops();

    //how many ops we expect to parse
    let expected_n_ops = contents.len();
    //n of ops parsed and checked
    let mut n_ops = 0;
    loop {
        let item = ops.parse_next();

        match item {
            //if we reached the end, then we good
            Ok(None) if n_ops == expected_n_ops => break,
            //we were expecting more ops to parse, but we stopped early
            Ok(None) => panic!(
                "expected #{} operations, only #{} were parsed",
                expected_n_ops, n_ops
            ),
            //generic error
            Err(e) => panic!("error parsing operation #{}: {:?}", n_ops, e),
            Ok(Some(op)) => {
                //retrieve the operation as object
                let json_op = &contents[n_ops];

                //verify the parsed one
                verify_operation(op, &json_op, &name, n_ops);
                n_ops += 1;
            }
        }
    }
}

//returns number of samples tested
// if all samples were ok
fn test_samples_in_file<P: AsRef<Path>>(filename: P) -> usize {
    let samples: Vec<Sample> = get_json_from_data(filename);

    let mut n_samples = 0;
    for (
        i,
        Sample {
            name,
            blob,
            operation: JsonOperation { branch, contents },
        },
    ) in samples.into_iter().enumerate()
    {
        let name = if name.is_empty() {
            format!("#{}", i)
        } else {
            name
        };

        test_sample(&name, blob, branch, contents);
        n_samples += 1;
    }

    n_samples
}

#[test]
#[should_panic(expected = "not yet implemented")] //TODO
fn common_samples() {
    test_samples_in_file(data_dir_path().join("samples.json"));
}

#[test]
#[should_panic(expected = "not yet implemented")] //TODO
fn michelson_samples() {
    test_samples_in_file(data_dir_path().join("michelson.json"));
}

#[test]
fn transfer_sample() {
    //retrieve all samples
    let samples: Vec<Sample> = get_json_from_data(data_dir_path().join("samples.json"));

    //get 6th sample
    let Sample {
        name: _,
        operation: JsonOperation { branch, contents },
        blob,
    } = samples[6].clone();

    //we should only have a single operation to parse
    assert_eq!(contents.len(), 1);

    test_sample("#6", blob, branch, contents);
}

#[test]
fn delegation_sample() {
    //retrieve all samples
    let samples: Vec<Sample> = get_json_from_data(data_dir_path().join("samples.json"));

    //get 6th sample
    let Sample {
        name: _,
        operation: JsonOperation { branch, contents },
        blob,
    } = samples[0].clone();

    //we should only have a single operation to parse
    assert_eq!(contents.len(), 1);

    test_sample("#0", blob, branch, contents);
}

#[test]
fn endorsement_sample() {
    //retrieve all samples
    let samples: Vec<Sample> = get_json_from_data(data_dir_path().join("samples.json"));

    //get 6th sample
    let Sample {
        name: _,
        operation: JsonOperation { branch, contents },
        blob,
    } = samples[3].clone();

    //we should only have a single operation to parse
    assert_eq!(contents.len(), 1);

    test_sample("#3", blob, branch, contents);
}

#[test]
fn seed_nonce_revelation_sample() {
    //retrieve all samples
    let samples: Vec<Sample> = get_json_from_data(data_dir_path().join("samples.json"));

    //get 6th sample
    let Sample {
        name: _,
        operation: JsonOperation { branch, contents },
        blob,
    } = samples[4].clone();

    //we should only have a single operation to parse
    assert_eq!(contents.len(), 1);

    test_sample("#4", blob, branch, contents);
}

#[test]
fn ballot_sample() {
    //retrieve all samples
    let samples: Vec<Sample> = get_json_from_data(data_dir_path().join("samples.json"));

    //get 6th sample
    let Sample {
        name: _,
        operation: JsonOperation { branch, contents },
        blob,
    } = samples[2].clone();

    //we should only have a single operation to parse
    assert_eq!(contents.len(), 1);

    test_sample("#2", blob, branch, contents);
}

#[test]
fn reveal_sample() {
    //retrieve all samples
    let samples: Vec<Sample> = get_json_from_data(data_dir_path().join("samples.json"));

    //get 6th sample
    let Sample {
        name: _,
        operation: JsonOperation { branch, contents },
        blob,
    } = samples[1].clone();

    //we should only have a single operation to parse
    assert_eq!(contents.len(), 1);

    test_sample("#1", blob, branch, contents);
}

#[test]
fn test_vectors() {
    let mut test_vectors_found = 0;
    let mut total_tests = 0;

    //get test_vector folder
    let folder_path = test_vectors_path();
    //ignore if the folder doesn't exist
    if let Ok(dir) = read_dir(&folder_path) {
        //go thru each item in `dir` and ignore all errors
        for filename in dir
            .filter_map(Result::ok)
            .filter(|dir_entry| {
                // but we only want the files
                dir_entry
                    .file_type()
                    .map(|ft| ft.is_file())
                    .unwrap_or_default()
            })
            //and we only interested in the path
            .map(|dir_entry| dir_entry.path())
        {
            test_vectors_found += 1;
            total_tests += test_samples_in_file(filename);
        }
    }

    eprintln!(
        "Test Vector files found #{}; Total tests run: #{}",
        test_vectors_found, total_tests
    )
}

fn verify_operation<'b>(
    op: OperationType<'b>,
    json: &Map<String, Value>,
    sample_name: &str,
    op_n: usize,
) {
    //get operation kind as string
    let kind = json["kind"].as_str().unwrap_or_else(|| {
        panic!(
            "sample {} .operation.contents[{}].kind was not a string",
            sample_name, op_n
        )
    });

    //verify we parsed the right kind of operation
    // and check against it
    match (op, kind) {
        (OperationType::Transfer(tx), "transaction") => tx.is(json),
        (OperationType::Delegation(del), "delegation") => del.is(json),
        (OperationType::Endorsement(endorsement), "endorsement") => endorsement.is(json),
        (OperationType::SeedNonceRevelation(snr), "seed_nonce_revelation") => snr.is(json),
        (OperationType::Ballot(vote), "ballot") => vote.is(json),
        (OperationType::Reveal(rev), "reveal") => rev.is(json),
        (op, other) => panic!(
            "sample {}[{}]; expected op kind: {}, parsed as: {:?}",
            sample_name, op_n, other, op
        ),
    }
}
