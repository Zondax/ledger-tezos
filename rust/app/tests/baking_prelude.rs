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
#![allow(unused_imports)]
#![cfg(feature = "baking")]

#[path = "prelude.rs"]
mod prelude;
use prelude::*;

/// This will reset the watermark and other state that may invalidate the tests
pub fn reset_state(level: u32) {
    let apdu = APDUCommand {
        cla: CLA,
        ins: INS_LEGACY_RESET,
        p1: 0,
        p2: 0,
        data: level.to_be_bytes().to_vec(),
    };

    let answer = process_apdu(&apdu);
    assert_eq!(answer.retcode(), ApduError::Success as u16);
}

pub fn authorize_baking(path: &[u32], curve: Curve) {
    let apdu = APDUCommand {
        cla: CLA,
        ins: INS_AUTHORIZE_BAKING,
        p1: 1,
        p2: curve.into(),
        data: prepare_path::<{ constants::BIP32_MAX_LENGTH }>(path),
    };

    let answer = process_apdu(&apdu);
    assert_eq!(answer.retcode(), ApduError::Success as u16);
}
