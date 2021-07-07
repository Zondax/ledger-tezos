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

use crate::{
    constants::ApduError as Error,
    dispatcher::ApduHandler,
    handlers::signing::Sign,
    utils::{ApduBufferRead, Uploader},
};

pub struct LegacySign;
pub struct LegacySignWithHash;

#[cfg(feature = "wallet")]
pub struct LegacySignUnsafe;

impl ApduHandler for LegacySign {
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        if let Some(upload) = Uploader::new(Sign).upload(&buffer)? {
            *tx = Sign::blind_sign(false, upload.p2, upload.first, upload.data, flags)?;
        }

        Ok(())
    }
}

impl ApduHandler for LegacySignWithHash {
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        if let Some(upload) = Uploader::new(Sign).upload(&buffer)? {
            *tx = Sign::blind_sign(true, upload.p2, upload.first, upload.data, flags)?;
        }

        Ok(())
    }
}

#[cfg(feature = "wallet")]
impl ApduHandler for LegacySignUnsafe {
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        if let Some(upload) = Uploader::new(Sign).upload(&buffer)? {
            *tx = Sign::blind_sign(false, upload.p2, upload.first, upload.data, flags)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        assert_error_code,
        dispatcher::{handle_apdu, CLA, INS_LEGACY_SIGN_WITH_HASH},
        handlers::LegacyPacketType,
        sys::{
            crypto::{bip32::BIP32Path, Curve},
            hash::{Blake2b, Hasher},
            set_out,
        },
    };
    use std::convert::TryInto;

    use serial_test::serial;

    fn prepare_buffer(buffer: &mut [u8; 260], path: &[u32], curve: Curve) -> usize {
        let crv: u8 = curve.into();
        let path = BIP32Path::<10>::new(path.iter().map(|n| 0x8000_0000 + n))
            .unwrap()
            .serialize();

        buffer[3] = crv;
        buffer[4] = path.len() as u8;
        buffer[5..5 + path.len()].copy_from_slice(path.as_slice());

        path.len()
    }

    #[test]
    #[ignore]
    #[serial(ui)]
    fn apdu_blind_sign_legacy() {
        const MSG: [u8; 18] = *b"franceco@zondax.ch";

        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];

        buffer[0] = CLA;
        buffer[1] = INS_LEGACY_SIGN_WITH_HASH;
        buffer[2] = LegacyPacketType::Init.into();
        let len = prepare_buffer(&mut buffer, &[44, 1729, 0, 0], Curve::Ed25519);

        handle_apdu(&mut flags, &mut tx, 5 + len as u32, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        buffer[0] = CLA;
        buffer[1] = INS_LEGACY_SIGN_WITH_HASH;
        buffer[2] = LegacyPacketType::AddAndLast.into();
        buffer[3] = 0;
        buffer[4] = MSG.len() as u8;
        buffer[5..5 + MSG.len()].copy_from_slice(&MSG[..]);

        set_out(&mut buffer);
        handle_apdu(&mut flags, &mut tx, 5 + MSG.len() as u32, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        let out_hash = &buffer[..32];
        let expected = Blake2b::<32>::digest(&MSG).unwrap();
        assert_eq!(&expected, out_hash);
    }
}
