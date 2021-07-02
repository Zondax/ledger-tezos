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
use std::prelude::v1::*;

use crate::{
    constants::ApduError as Error,
    dispatcher::{ApduHandler, INS_DEV_HASH},
    sys::hash::{Hasher, Sha256 as HashSha256},
    utils::{ApduBufferRead, Uploader},
};

pub struct Sha256;

impl ApduHandler for Sha256 {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        apdu_buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;
        if apdu_buffer.ins() != INS_DEV_HASH {
            return Err(Error::InsNotSupported);
        }

        //collect all data
        if let Some(upload) = Uploader::new(Self).upload(&apdu_buffer)? {
            let digest = {
                let mut hasher = HashSha256::new().map_err(|_| Error::ExecutionError)?;
                hasher
                    .update(upload.first) //hash the first section
                    .map_err(|_| Error::ExecutionError)?;
                hasher
                    .update(upload.data) //and the rest
                    .map_err(|_| Error::ExecutionError)?;
                hasher.finalize().map_err(|_| Error::ExecutionError)?
            };

            let apdu_buffer = apdu_buffer.write();

            if apdu_buffer.len() < digest.len() {
                return Err(Error::OutputBufferTooSmall);
            }

            apdu_buffer[..digest.len()].copy_from_slice(&digest[..]);
            *tx = digest.len() as u32;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_error_code,
        constants::ApduError as Error,
        dispatcher::{handle_apdu, CLA, INS_DEV_HASH},
        handlers::ZPacketType,
    };
    use std::convert::TryInto;

    use serial_test::serial;
    use sha2::{Digest, Sha256};

    #[test]
    #[serial(dev_hash)]
    fn apdu_dev_hash() {
        const MSG: [u8; 0xFF] = [42; 0xFF];

        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];

        //Init
        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = ZPacketType::Init.into();
        buffer[3] = 0;
        buffer[4] = 255;
        buffer[5..].copy_from_slice(&MSG[..255]);

        handle_apdu(&mut flags, &mut tx, 260, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        //Add
        MSG[255..].chunks(255).enumerate().for_each(|(i, c)| {
            let len = c.len();
            flags = 0;
            tx = 0;

            buffer[0] = CLA;
            buffer[1] = INS_DEV_HASH;
            buffer[2] = ZPacketType::Add.into();
            buffer[3] = 0;
            buffer[4] = len as u8;

            let msg_sent = (i + 1) * 255; //send 255 bytes * i chunks + 1 (init)
            buffer[5..len].copy_from_slice(&MSG[msg_sent..msg_sent + len]);

            handle_apdu(&mut flags, &mut tx, 5 + len as u32, &mut buffer);
            assert_error_code!(tx, buffer, Error::Success);
        });

        //Last
        flags = 0;
        tx = 0;
        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = ZPacketType::Last.into();
        buffer[3] = 0;
        buffer[4] = 0;

        handle_apdu(&mut flags, &mut tx, 5, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        let expected = Sha256::digest(&MSG);
        let digest = &buffer[..tx as usize - 2];
        assert_eq!(digest, expected.as_slice());
    }

    #[test]
    #[serial(dev_hash)]
    fn apdu_dev_hash_short() {
        const MSG: &[u8] = b"francesco@zondax.ch";
        let len = MSG.len();

        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];

        //Init
        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = ZPacketType::Init.into();
        buffer[3] = 0;
        buffer[4] = len as u8;
        buffer[5..5 + len].copy_from_slice(MSG);

        handle_apdu(&mut flags, &mut tx, 5 + len as u32, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = ZPacketType::Last.into();
        buffer[3] = 0;
        buffer[4] = 0;

        handle_apdu(&mut flags, &mut tx, 5, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        let expected = Sha256::digest(&MSG);
        let digest = &buffer[..tx as usize - 2];
        assert_eq!(digest, expected.as_slice());
    }
}
