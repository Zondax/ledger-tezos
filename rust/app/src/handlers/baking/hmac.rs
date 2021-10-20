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
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::Curve,
    dispatcher::ApduHandler,
    handlers::signing::Sign,
    sys::{
        self,
        crypto::bip32::BIP32Path,
        hash::{Hasher, Sha512},
        hmac::Sha256HMAC,
    },
    utils::ApduBufferRead,
};

use core::convert::TryFrom;

pub struct HMAC;

//apdu_hmac.c:23
const KEY_SHA256: &[u8] = &[
    0x6c, 0x4e, 0x7e, 0x70, 0x6c, 0x54, 0xd3, 0x67, 0xc8, 0x7a, 0x8d, 0x89, 0xc1, 0x6a, 0xdf, 0xe0,
    0x6c, 0xb5, 0x68, 0x0c, 0xb7, 0xd1, 0x8e, 0x62, 0x5a, 0x90, 0x47, 0x5e, 0xc0, 0xdb, 0xdb, 0x9f,
];

impl HMAC {
    #[inline(never)]
    fn sig_and_hash_hmac_key(
        curve: Curve,
        path: BIP32Path<BIP32_MAX_LENGTH>,
    ) -> Result<[u8; 64], Error> {
        //sign the hmac key
        let (sig_size, sig_hmac_key) = Sign::sign(curve, &path, KEY_SHA256)?;

        //and hash the signature
        Sha512::digest(&sig_hmac_key[..sig_size]).map_err(|_| Error::ExecutionError)
    }

    #[inline(never)]
    fn do_hmac(key: [u8; 64], offset: usize, buffer: ApduBufferRead<'_>) -> Result<u32, Error> {
        let mut hmac = {
            let mut loc = core::mem::MaybeUninit::uninit();
            Sha256HMAC::new_gce(&mut loc, &key[..]).map_err(|_| Error::ExecutionError)?;
            unsafe { loc.assume_init() }
        };

        {
            let input = &buffer.payload().map_err(|_| Error::DataInvalid)?[offset..];
            hmac.update(input).map_err(|_| Error::ExecutionError)?;
        }

        let buffer = buffer.write();
        hmac.finalize_hmac_into(arrayref::array_mut_ref!(buffer, 0, 32))
            .map_err(|_| Error::ExecutionError)?;

        Ok(32)
    }

    #[inline(never)]
    pub fn hmac(
        curve: Curve,
        path: BIP32Path<BIP32_MAX_LENGTH>,
        //offset in `buffer.payload()` of the bytes we already read
        offset: usize,
        buffer: ApduBufferRead<'_>,
    ) -> Result<u32, Error> {
        let hash_hmac_key_sig = Self::sig_and_hash_hmac_key(curve, path)?;

        Self::do_hmac(hash_hmac_key_sig, offset, buffer)
    }
}

impl ApduHandler for HMAC {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        sys::zemu_log_stack("HMAC::handle\x00");

        let curve = Curve::try_from(buffer.p2()).map_err(|_| Error::InvalidP1P2)?;

        let cdata = buffer.payload().map_err(|_| Error::DataInvalid)?;
        if cdata.is_empty() {
            return Err(Error::WrongLength);
        }

        let path_len = cdata[0] as usize;
        let bip32_path = BIP32Path::<BIP32_MAX_LENGTH>::read(
            cdata.get(..1 + 4 * path_len).ok_or(Error::WrongLength)?,
        )
        .map_err(|_| Error::DataInvalid)?;

        *tx = Self::hmac(curve, bip32_path, 1 + 4 * path_len, buffer)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bolos::crypto::{bip32::BIP32Path, Curve};
    use std::convert::TryInto;

    use crate::{
        assert_error_code,
        constants::ApduError,
        dispatcher::{handle_apdu, CLA, INS_LEGACY_HMAC},
    };

    fn prepare_buffer<const LEN: usize>(
        buffer: &mut [u8; 260],
        path: &[u32],
        curve: Curve,
    ) -> usize {
        let crv: u8 = curve.into();
        let path = BIP32Path::<LEN>::new(path.iter().map(|n| 0x8000_0000 + n))
            .unwrap()
            .serialize();

        buffer[3] = crv;
        buffer[4] = path.len() as u8;
        buffer[5..5 + path.len()].copy_from_slice(path.as_slice());

        5 + path.len()
    }

    #[test]
    pub fn apdu_hmac() {
        let mut flags = 0u32;
        let mut tx = 0u32;
        let rx = 5;
        let mut buffer = [0u8; 260];

        const HMAC_MSG: &[u8] = b"zondax.ch";

        buffer[..3].copy_from_slice(&[CLA, INS_LEGACY_HMAC, 0]);
        let offset = prepare_buffer::<4>(&mut buffer, &[44, 1729, 0, 0], Curve::Ed25519);

        buffer[offset..offset + HMAC_MSG.len()].copy_from_slice(&HMAC_MSG[..]);

        handle_apdu(&mut flags, &mut tx, rx, &mut buffer);

        assert_error_code!(tx, buffer, ApduError::Success);
        assert_eq!(tx as usize, 32 + 2);
    }
}
