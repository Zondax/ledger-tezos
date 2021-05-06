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
use crate::dispatcher::{ApduHandler, INS_LEGACY_GET_VERSION, INS_LEGACY_GIT};
use crate::handlers::version::{VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH};

pub struct LegacyGetVersion {}

pub struct LegacyGit {}

impl LegacyGit {
    pub const COMMIT_HASH_LEN: usize = 8;

    pub const fn commit_hash() -> &'static [u8] {
        &crate::utils::GIT_COMMIT_HASH.as_bytes()
    }
}

impl ApduHandler for LegacyGetVersion {
    fn handle(
        _flags: &mut u32,
        tx: &mut u32,
        _rx: u32,
        apdu_buffer: &mut [u8],
    ) -> Result<(), ApduError> {
        if apdu_buffer[APDU_INDEX_INS] != INS_LEGACY_GET_VERSION {
            return Err(InsNotSupported);
        }

        // https://github.com/obsidiansystems/ledger-app-tezos/blob/58797b2f9606c5a30dd1ccc9e5b9962e45e10356/src/apdu.c#L24
        apdu_buffer[0] = 0; // FIXME: Baking app = 1
        apdu_buffer[1] = VERSION_MAJOR;
        apdu_buffer[2] = VERSION_MINOR;
        apdu_buffer[3] = VERSION_PATCH;
        *tx = 4;

        Ok(())
    }
}

impl ApduHandler for LegacyGit {
    fn handle(
        _flags: &mut u32,
        tx: &mut u32,
        _rx: u32,
        apdu_buffer: &mut [u8],
    ) -> Result<(), ApduError> {
        if apdu_buffer[APDU_INDEX_INS] != INS_LEGACY_GIT {
            return Err(InsNotSupported);
        }

        let commit = &Self::commit_hash()[..Self::COMMIT_HASH_LEN];

        if apdu_buffer.len() < commit.len() {
            return Err(ApduError::OutputBufferTooSmall);
        }

        // Reference: https://github.com/obsidiansystems/ledger-app-tezos/blob/58797b2f9606c5a30dd1ccc9e5b9962e45e10356/src/apdu.c#L30
        apdu_buffer[..Self::COMMIT_HASH_LEN].copy_from_slice(&commit);
        apdu_buffer[Self::COMMIT_HASH_LEN] = 0; //null terminate the string
        *tx = 1 + Self::COMMIT_HASH_LEN as u32;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::LegacyGit;
    use crate::constants::ApduError::Success;
    use crate::dispatcher::{handle_apdu, CLA, INS_LEGACY_GET_VERSION, INS_LEGACY_GIT};
    use crate::handlers::version::{VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH};
    use crate::utils::assert_error_code;

    #[test]
    fn apdu_get_version() {
        let flags = &mut 0u32;
        let tx = &mut 0u32;
        let rx = 5u32;
        let buffer = &mut [0u8; 260];

        buffer[0] = CLA;
        buffer[1] = INS_LEGACY_GET_VERSION;
        buffer[2] = 0;
        buffer[3] = 0;
        buffer[4] = 0;

        handle_apdu(flags, tx, rx, buffer);

        assert_eq!(*tx, 4 + 2);
        assert_error_code(tx, buffer, Success);

        assert_eq!(buffer[0], 0);
        assert_eq!(buffer[1], VERSION_MAJOR);
        assert_eq!(buffer[2], VERSION_MINOR);
        assert_eq!(buffer[3], VERSION_PATCH);
    }

    #[test]
    fn apdu_get_git() {
        let mut flags = 0;
        let mut tx = 0;
        let rx = 5;
        let mut buffer = [0; 260];

        let len = LegacyGit::COMMIT_HASH_LEN;

        buffer[..5].copy_from_slice(&[CLA, INS_LEGACY_GIT, 0, 0, 0]);
        handle_apdu(&mut flags, &mut tx, rx, &mut buffer);

        assert_eq!(tx as usize, len + 1 + 2);
        assert_error_code(&tx, &buffer, Success);

        let commit_hash = LegacyGit::commit_hash();
        assert_eq!(&buffer[..len], &commit_hash[..len]);
    }
}
