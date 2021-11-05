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
use crate::constants::ApduError;
use crate::dispatcher::ApduHandler;
use crate::utils::ApduBufferRead;

pub const VERSION_MAJOR: u8 = 3;
pub const VERSION_MINOR: u8 = 0;
pub const VERSION_PATCH: u8 = 3;

pub struct GetVersion {}

pub fn get_target_id() -> Result<u32, ApduError> {
    Ok(0u32)
}

impl ApduHandler for GetVersion {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        apdu_buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), ApduError> {
        *tx = 0;

        let apdu_buffer = apdu_buffer.write();
        apdu_buffer[0] = 0; //Debug mode
                            // Version
        apdu_buffer[1] = VERSION_MAJOR;
        apdu_buffer[2] = VERSION_MINOR;
        apdu_buffer[3] = VERSION_PATCH;
        apdu_buffer[4] = 0; //UX allowed

        // target id
        let target_id_slice = get_target_id()?.to_be_bytes();
        apdu_buffer[5..9].clone_from_slice(&target_id_slice);
        *tx = 9;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_error_code;
    use crate::constants::ApduError::Success;
    use crate::dispatcher::{handle_apdu, CLA, INS_GET_VERSION};
    use crate::handlers::version::{VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH};
    use std::convert::TryInto;

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
        assert_error_code!(*tx, buffer, Success);

        assert_eq!(buffer[1], VERSION_MAJOR);
        assert_eq!(buffer[2], VERSION_MINOR);
        assert_eq!(buffer[3], VERSION_PATCH);
    }
}
