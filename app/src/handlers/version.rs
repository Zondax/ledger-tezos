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
use crate::dispatcher::{ApduHandler, INS_GET_VERSION};

pub const VERSION_MAJOR: u8 = 1;
pub const VERSION_MINOR: u8 = 2;
pub const VERSION_PATCH: u8 = 3;

pub struct GetVersion {}

pub fn get_target_id() -> Result<u32, ApduError> {
    // FIXME: return target id here. Move to bolos
    Ok(0u32)
}

impl ApduHandler for GetVersion {
    fn handle(
        _flags: &mut u32,
        tx: &mut u32,
        _rx: u32,
        apdu_buffer: &mut [u8],
    ) -> Result<(), ApduError> {
        if apdu_buffer[APDU_INDEX_INS] != INS_GET_VERSION {
            return Err(InsNotSupported);
        }

        *tx = 0;

        apdu_buffer[0] = 0; // FIXME: Debug mode enabled?
                            // Version
        apdu_buffer[1] = VERSION_MAJOR;
        apdu_buffer[2] = VERSION_MINOR;
        apdu_buffer[3] = VERSION_PATCH;
        apdu_buffer[4] = 0; // FIXME: Is UX allowed?

        // target id
        let target_id_slice = get_target_id()?.to_be_bytes();
        apdu_buffer[5..9].clone_from_slice(&target_id_slice);
        *tx = 9;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::constants::ApduError::Success;
    use crate::dispatcher::{handle_apdu, CLA, INS_GET_VERSION};
    use crate::handlers::version::{VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH};
    use crate::utils::assert_error_code;

    #[test]
    fn apdu_get_version() {
        let flags = &mut 0u32;
        let tx = &mut 0u32;
        let rx = 5u32;
        let buffer = &mut [0u8; 260];

        buffer[0] = CLA;
        buffer[1] = INS_GET_VERSION;
        buffer[2] = 0;
        buffer[3] = 0;
        buffer[4] = 0;

        handle_apdu(flags, tx, rx, buffer);

        assert_eq!(*tx, 1 + 4 + 4 + 2);
        assert_error_code(tx, buffer, Success);

        assert_eq!(buffer[1], VERSION_MAJOR);
        assert_eq!(buffer[2], VERSION_MINOR);
        assert_eq!(buffer[3], VERSION_PATCH);
    }
}
