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
const KEY_SHA256: &'static [u8] = &[
    0x6c, 0x4e, 0x7e, 0x70, 0x6c, 0x54, 0xd3, 0x67, 0xc8, 0x7a, 0x8d, 0x89, 0xc1, 0x6a, 0xdf, 0xe0,
    0x6c, 0xb5, 0x68, 0x0c, 0xb7, 0xd1, 0x8e, 0x62, 0x5a, 0x90, 0x47, 0x5e, 0xc0, 0xdb, 0xdb, 0x9f,
];

impl HMAC {
    #[inline(always)]
    pub fn hmac<'apdu>(
        curve: Curve,
        path: BIP32Path<BIP32_MAX_LENGTH>,
        //offset in `buffer.payload()` of the bytes we already read
        offset: usize,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<u32, Error> {
        sys::zemu_log_stack("HMAC::auth\x00");

        //sign the hmac key
        let (sig_size, sig_hmac_key) = Sign::sign(curve, &path, KEY_SHA256)?;

        //and hash the signature
        let hash_hmac_key_sig =
            Sha512::digest(&sig_hmac_key[..sig_size]).map_err(|_| Error::ExecutionError)?;

        let input = &buffer.payload().map_err(|_| Error::DataInvalid)?[offset..];

        let hmac = Sha256HMAC::new(&hash_hmac_key_sig[..])
            .and_then(|hmac| hmac.finalize_hmac(input))
            .map_err(|_| Error::ExecutionError)?;

        let buffer = buffer.write();
        buffer[..hmac.len()].copy_from_slice(&hmac[..]);

        Ok(hmac.len() as u32)
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

        let path_len = cdata[0] as usize;
        let bip32_path = BIP32Path::<BIP32_MAX_LENGTH>::read(&cdata[..1 + 4 * path_len])
            .map_err(|_| Error::DataInvalid)?;

        *tx = Self::hmac(curve, bip32_path, 1 + 4 * path_len, buffer)?;

        Ok(())
    }
}
