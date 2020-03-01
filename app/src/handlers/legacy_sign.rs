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
use crate::constants::ApduError::InsNotSupported;
use crate::constants::{ApduError, APDU_INDEX_INS};
use crate::dispatcher::{
    ApduHandler, INS_LEGACY_SIGN, INS_LEGACY_SIGN_UNSAFE, INS_LEGACY_SIGN_WITH_HASH,
};

pub struct LegacySign {}

impl ApduHandler for LegacySign {
    fn handle(
        _flags: &mut u32,
        tx: &mut u32,
        _rx: u32,
        apdu_buffer: &mut [u8],
    ) -> Result<(), ApduError> {
        // Reference: https://github.com/obsidiansystems/ledger-app-tezos/blob/58797b2f9606c5a30dd1ccc9e5b9962e45e10356/src/apdu_sign.c#L641-L651

        let (_enable_hashing, _enable_parsing) = match apdu_buffer[APDU_INDEX_INS] {
            x if x == INS_LEGACY_SIGN => Ok((false, false)),
            x if x == INS_LEGACY_SIGN_UNSAFE => Ok((true, false)),
            x if x == INS_LEGACY_SIGN_WITH_HASH => Ok((true, false)),
            _ => Err(InsNotSupported),
        }?;

        *tx = 0;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::dispatcher::{handle_apdu, CLA, INS_LEGACY_SIGN};

    #[test]
    fn apdu_check() {
        let flags = &mut 0u32;
        let tx = &mut 0u32;
        let rx = 5u32;
        let buffer = &mut [0u8; 260];

        buffer[0] = CLA;
        buffer[1] = INS_LEGACY_SIGN;
        buffer[2] = 0;
        buffer[3] = 0;
        buffer[4] = 0;

        handle_apdu(flags, tx, rx, buffer);

        // FIXME: complete the test
    }
}
