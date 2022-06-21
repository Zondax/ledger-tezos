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
#![cfg(feature = "baking")]

const SAMPLES: &'static [(&str, usize, &str, &str)] = include!("signatory_samples.bin");

mod prelude;
use prelude::*;

mod baking_prelude;
use baking_prelude::*;

const PATH: &[u32] = &[44, 1729, 0, 0];
const CURVE: Curve = Curve::Bip32Ed25519;

#[test]
#[serial]
fn baking_flow() {
    reset_state(0);

    authorize_baking(PATH, CURVE);

    //prepare command
    let command = APDUCommand {
        cla: CLA,
        ins: INS_BAKER_SIGN,
        p1: PacketType::Init.into(),
        p2: CURVE.into(),
        data: prepare_path::<{ constants::BIP32_MAX_LENGTH }>(PATH),
    };

    for (i, (time, level, sign_type, data)) in SAMPLES.iter().enumerate() {
        let data = hex::decode(data).unwrap_or_else(|_| panic!("sample #{} data was not hex", i));

        println!(
            "Processing sample #{}; time={}; op={}; level={}",
            i, time, sign_type, level
        );

        let answer = process_apdu_chunks(command.clone(), &data);
        assert_eq!(answer.retcode(), ApduError::Success as u16);

        println!("processed sample #{}", i);
        //TODO: check that the signature is valid
    }
}

#[test]
#[serial]
fn zemu_tb_block_blob() {
    //the second line of the payload should be
    // gotten from the baking_utils `get_blockdata_info`
    // of the zemu folder.
    // the first line (0x11) is added manually (tenderbake block magic byte)
    const BLOB: &str = "11\
                        000000000000007b2a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000090202020202000001c8";
    let data = hex::decode(BLOB).unwrap();

    reset_state(0);

    authorize_baking(PATH, CURVE);

    //prepare command
    let command = APDUCommand {
        cla: CLA,
        ins: INS_BAKER_SIGN,
        p1: PacketType::Init.into(),
        p2: CURVE.into(),
        data: prepare_path::<{ constants::BIP32_MAX_LENGTH }>(PATH),
    };

    let answer = process_apdu_chunks(command.clone(), &data);
    assert_eq!(answer.retcode(), ApduError::Success as u16);
}

#[test]
#[serial]
fn zemu_endorsement_blob() {
    //the second line of the payloads should be
    // gotten from the baking_utils `get_endorsement_info`
    // of the zemu folder.
    // the first line is added manually (magic bytes)
    const EMMY_BLOB: &str = "02\
                             000000000000000000000000000000000000000000000000000000000000000000000000000000007b";

    const TB_PREENDORSEMENT: &str = "12\
                                     00000000000000000000000000000000000000000000000000000000000000000000000014000000001092000001c80000000000000000000000000000000000000000000000000000000000000000";

    const TB_ENDORSEMENT: &str = "13\
                                  00000000000000000000000000000000000000000000000000000000000000000000000015000000001092000001c80000000000000000000000000000000000000000000000000000000000000000";

    reset_state(0);

    authorize_baking(PATH, CURVE);

    //prepare command
    let command = APDUCommand {
        cla: CLA,
        ins: INS_BAKER_SIGN,
        p1: PacketType::Init.into(),
        p2: CURVE.into(),
        data: prepare_path::<{ constants::BIP32_MAX_LENGTH }>(PATH),
    };

    for (i, blob) in [EMMY_BLOB, TB_PREENDORSEMENT, TB_ENDORSEMENT]
        .iter()
        .enumerate()
    {
        println!("processing #{}", i);
        let data = hex::decode(blob).unwrap();

        let answer = process_apdu_chunks(command.clone(), &data);
        assert_eq!(answer.retcode(), ApduError::Success as u16);
    }
}
