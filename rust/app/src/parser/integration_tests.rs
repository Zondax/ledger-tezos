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
use std::fs::File;
use std::path::{Path, PathBuf};
use std::prelude::v1::*;

use serde_json::{Map, Value};

use crate::handlers::public_key::Addr;
use crate::parser::operations::{Entrypoint, Operation};

use super::operations::OperationType;

fn data_file_path() -> PathBuf {
    std::env::var_os("TEZOS_TEST_DATA")
        .unwrap_or("../../zemu/tests/data".to_string().into())
        .into()
}

fn get_json_from_data<P: AsRef<Path>>(filename: P) -> Value {
    let path = data_file_path().join(filename);

    let file = File::open(&path).expect(&format!("couldn't read file at {:?}", path));

    serde_json::from_reader(file).expect(&format!("couldn't parse json at {:?}", path))
}

#[test]
fn simple_transfer_sample() {
    //retrieve all samples
    let samples = get_json_from_data("samples.json");
    let samples = samples.as_array().expect("samples json wasn't an array");

    //get 6th sample
    let sample = samples[6].as_object().expect("sample #6 wasn't an object");

    //get blob and decode hexstring
    let blob = sample["blob"]
        .as_str()
        .expect("sample #6 .blob wasn't a string");
    let blob = hex::decode(&blob).expect("sample #6 .blob wasn't a hex string");

    //parse forged op blob
    let mut parsed = Operation::new(&blob).expect("sample #6 couldn't be parsed");

    //get operation object
    let operation = sample["operation"]
        .as_object()
        .expect("sample #6 .operation wasn't an object");

    //verify that the branches match
    let expected_branch_bs58 = operation["branch"]
        .as_str()
        .expect("sample #6 .branch wasn't a string");

    let mut branch_bs58 = [0; 51];
    parsed
        .base58_branch(&mut branch_bs58)
        .expect("couldn't compute base 58 branch of sample #6");

    assert_eq!(branch_bs58, expected_branch_bs58.as_bytes());

    //retrieve ops from parsed and also ops in the operation
    let ops = parsed.mut_ops();
    let contents = operation["contents"]
        .as_array()
        .expect("sample #6 .operation.contents wasn't an array");

    //we should only have a single operation to parse
    assert_eq!(contents.len(), 1);

    let op = ops
        .parse_next()
        .expect("unable to parse operation")
        .expect("0 operations parsed?");

    let json_op = contents[0]
        .as_object()
        .expect("sample #6 .operation.contents[0] wasn't an object");

    //we should verify that this is indeed a transfer
    assert!(op.is_transfer());

    //verify the parsed one
    verify_operation(op, json_op, 6, 0);
}

#[test]
#[should_panic] //TODO
fn all_samples() {
    let samples = get_json_from_data("samples.json");

    let samples = samples.as_array().expect("samples json wasn't an array");

    for (i, sample) in samples.into_iter().enumerate() {
        //get sample object
        let sample = sample
            .as_object()
            .expect(&format!("sample #{} wasn't an object", i));

        //get blob and decode from hexstring
        let blob = sample["blob"]
            .as_str()
            .expect(&format!("sample #{} .blob wasn't a string", i));
        let blob = hex::decode(&blob).expect(&format!("sample #{} .blob wasn't a hex string", i));

        //parse forged op blob
        let mut parsed = Operation::new(&blob).expect(&format!("sample #{} couldn't be parsed", i));

        //get operation object
        let operation = sample["operation"]
            .as_object()
            .expect(&format!("sample #{} .operation wasn't an object", i));

        //verify that the branches match
        let expected_branch_bs58 = operation["branch"]
            .as_str()
            .expect(&format!("sample #{} .branch wasn't a string", i));

        let mut branch_bs58 = [0; 51];
        parsed
            .base58_branch(&mut branch_bs58)
            .expect(&format!("couldn't compute base 58 branch of sample #{}", i));

        assert_eq!(branch_bs58, expected_branch_bs58.as_bytes());

        //retrieve ops from parsed and also ops in the operation
        let ops = parsed.mut_ops();
        let contents = operation["contents"].as_array().expect(&format!(
            "sample #{} .operation.contents wasn't an array",
            i
        ));

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
                    let json_op = contents[n_ops].as_object().expect(&format!(
                        "sample #{} .operation.contents[{}] wasn't an object",
                        i, n_ops
                    ));

                    //verify the parsed one
                    verify_operation(op, json_op, i, n_ops);
                    n_ops += 1;
                }
            }
        }
    }
}

fn verify_operation<'b>(
    op: OperationType<'b>,
    json: &Map<String, Value>,
    sample_n: usize,
    op_n: usize,
) {
    //get operation kind as string
    let kind = json["kind"].as_str().expect(&format!(
        "sample #{} .operation.contents[{}].kind",
        sample_n, op_n
    ));

    //verify we parsed the right kind of operation
    // and check against it
    match (op, kind) {
        (OperationType::Transfer(tx), "transaction") => tx.is(json),
        (op, other) => panic!(
            "sample #{}[{}]; expected op kind: {}, parsed as: {:?}",
            sample_n, op_n, other, op
        ),
    }
}

impl<'b> super::operations::Transfer<'b> {
    fn source_base58(&self) -> Result<[u8; 36], bolos::Error> {
        let source = self.source();
        let addr = Addr::from_hash(source.1, source.0)?;

        Ok(addr.to_base58())
    }

    fn is(&self, json: &Map<String, Value>) {
        //verify source address of the transfer
        let source_base58 = self
            .source_base58()
            .expect("couldn't compute source base58");
        let expected_source_base58 = json["source"]
            .as_str()
            .expect("given json .source is not a string");
        assert_eq!(source_base58, expected_source_base58.as_bytes());

        //TODO: fee, counter, gas_limit, storage_limit, amount temporarily

        //verify the destination
        let destination_bs58 = {
            let mut out = [0; 36];
            self.destination()
                .base58(&mut out)
                .expect("couldn't compute destination base58");
            out
        };
        let expected_destination_base58 = json["destination"]
            .as_str()
            .expect("given json .destination is not a string");
        assert_eq!(destination_bs58, expected_destination_base58.as_bytes());

        //check parameters, either they are both in json and the parsed,
        // or they are missing in both
        match (
            self.parameters(),
            json.get("parameters").map(|j| {
                j.as_object()
                    .expect("given json .parameters is not an object")
            }),
        ) {
            (None, None) => {}
            (Some(_), None) => panic!("parsed parameters where none were given"),
            (None, Some(_)) => panic!("parameters were not parsed where some were given"),
            (Some(parsed), Some(expected)) => {
                //if they are present, verify the entrypoint
                // get entrypoint from json as string
                let expected_entrypoint = expected["entrypoint"]
                    .as_str()
                    .expect("given json .parameters.entrypoint is not a string");

                //verify entrypoint
                match (parsed.entrypoint(), expected_entrypoint) {
                    (Entrypoint::Default, "default")
                    | (Entrypoint::Root, "root")
                    | (Entrypoint::Do, "do")
                    | (Entrypoint::SetDelegate, "set_delegate")
                    | (Entrypoint::RemoveDelegate, "remove_delegate") => {}
                    (Entrypoint::Custom(s), js) if s == &js.as_bytes() => {}
                    (parsed, expected) => {
                        panic!("expected entrypoint: {}, parsed: {}", expected, parsed)
                    }
                }

                //TODO: verify michelson code (parameters.value)
            }
        }
    }
}
