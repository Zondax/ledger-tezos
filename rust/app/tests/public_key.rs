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
mod prelude;
use prelude::*;

const PATH: &[u32] = &[44, 1729];
const CURVE: Curve = Curve::Ed25519;

#[test]
fn legacy_get_public_key() {
    //prepare command
    let command = APDUCommand {
        cla: CLA,
        ins: INS_LEGACY_GET_PUBLIC_KEY,
        p1: 0,
        p2: CURVE.into(),
        data: prepare_path::<{ constants::BIP32_MAX_LENGTH }>(PATH),
    };

    let answer = process_apdu(&command);
    assert_eq!(answer.retcode(), ApduError::Success as u16);
}

#[test]
fn get_public_key() {
    //prepare command
    let command = APDUCommand {
        cla: CLA,
        ins: INS_GET_ADDRESS,
        p1: 0,
        p2: CURVE.into(),
        data: prepare_path::<{ constants::BIP32_MAX_LENGTH }>(PATH),
    };

    let answer = process_apdu(&command);
    assert_eq!(answer.retcode(), ApduError::Success as u16);
}

#[test]
fn public_key_blob() {
    //blob from issue reported internally
    const BLOB_HEX: &str = "80\
                            02\
                            00\
                            00\
                            09028000002c800006c1";
    let data = hex::decode(BLOB_HEX).expect("invalid hex data");

    //prepare command
    let command = APDUCommand {
        cla: data[0],
        ins: data[1],
        p1: data[2],
        p2: data[3],
        data: Vec::from(&data[5..]),
    };

    let answer = process_apdu(&command);
    assert_eq!(answer.retcode(), ApduError::Success as u16);
}
