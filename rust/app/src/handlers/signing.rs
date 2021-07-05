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
use std::convert::TryFrom;

use bolos::{
    crypto::bip32::BIP32Path,
    hash::{Blake2b, Hasher},
};
use zemu_sys::{Show, ViewError, Viewable};

use crate::{
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::Curve,
    dispatcher::{ApduHandler, INS_LEGACY_SIGN, INS_LEGACY_SIGN_WITH_HASH, INS_SIGN},
    sys,
    utils::{ApduBufferRead, Uploader},
};

#[bolos::lazy_static]
static mut PATH: Option<(BIP32Path<BIP32_MAX_LENGTH>, Curve)> = None;

pub struct Sign;

impl Sign {
    pub const SIGN_HASH_SIZE: usize = 32;

    fn get_derivation_info() -> Result<&'static (BIP32Path<BIP32_MAX_LENGTH>, Curve), Error> {
        match unsafe { &*PATH } {
            None => Err(Error::ApduCodeConditionsNotSatisfied),
            Some(some) => Ok(some),
        }
    }

    //(actual_size, [u8; MAX_SIGNATURE_SIZE])
    #[inline(never)]
    fn sign<const LEN: usize>(
        curve: Curve,
        path: &BIP32Path<LEN>,
        data: &[u8],
    ) -> Result<(usize, [u8; 100]), Error> {
        let sk = curve.to_secret(path);

        let mut out = [0; 100];
        let sz = sk
            .sign(data, &mut out[..])
            .map_err(|_| Error::ExecutionError)?;

        Ok((sz, out))
    }

    #[inline(never)]
    fn blake2b_digest(buffer: &[u8]) -> Result<[u8; Self::SIGN_HASH_SIZE], Error> {
        Blake2b::digest(buffer).map_err(|_| Error::ExecutionError)
    }

    #[inline(never)]
    pub fn blind_sign(
        send_hash: bool,
        p2: u8,
        init_data: &[u8],
        data: &[u8],
        flags: &mut u32,
    ) -> Result<u32, Error> {
        let curve = Curve::try_from(p2).map_err(|_| Error::InvalidP1P2)?;
        let path =
            BIP32Path::<BIP32_MAX_LENGTH>::read(init_data).map_err(|_| Error::DataInvalid)?;

        unsafe {
            PATH.replace((path, curve));
        }

        let unsigned_hash = Self::blake2b_digest(data)?;

        let ui = BlindSignUi {
            hash: unsigned_hash,
            send_hash,
        };

        unsafe { ui.show(flags) }
            .map_err(|_| Error::ExecutionError)
            .map(|_| 0)
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    //NEW API: TODO
    Sign,

    //LEGACY API: TODO
    LegacySign,

    //LEGACY API: TODO
    LegacySignWithHash,

    #[cfg(feature = "wallet")]
    //LEGACY API: TODO
    LegacySignUnsafe,
}

impl ApduHandler for Sign {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        sys::zemu_log_stack("Sign::handle\x00");

        *tx = 0;
        let action = match buffer.ins() {
            INS_SIGN => Action::Sign,
            INS_LEGACY_SIGN => Action::LegacySign,
            INS_LEGACY_SIGN_WITH_HASH => Action::LegacySignWithHash,
            #[cfg(feature = "wallet")]
            crate::dispatcher::INS_LEGACY_SIGN_UNSAFE => Action::LegacySignUnsafe,
            _ => return Err(Error::InsNotSupported),
        };

        if let Some(upload) = Uploader::new(Self).upload(&buffer)? {
            *tx = match action {
                Action::Sign | Action::LegacySignWithHash => {
                    Self::blind_sign(true, upload.p2, upload.first, upload.data, flags)?
                }
                Action::LegacySign => {
                    Self::blind_sign(false, upload.p2, upload.first, upload.data, flags)?
                }
                #[cfg(feature = "wallet")]
                Action::LegacySignUnsafe => {
                    Self::blind_sign(false, upload.p2, upload.first, upload.data, flags)?
                }
            };
        }

        Ok(())
    }
}

struct BlindSignUi {
    hash: [u8; Sign::SIGN_HASH_SIZE],
    send_hash: bool,
}

impl Viewable for BlindSignUi {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        Ok(1)
    }

    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        if let 0 = item_n {
            let title_content = bolos::PIC::new(b"Blind Sign\x00").into_inner();

            title[..title_content.len()].copy_from_slice(title_content);

            let m_len = message.len() - 1; //null byte terminator
            if m_len <= Sign::SIGN_HASH_SIZE * 2 {
                let chunk = self
                    .hash
                    .chunks(m_len / 2) //divide in non-overlapping chunks
                    .nth(page as usize) //get the nth chunk
                    .ok_or(ViewError::Unknown)?;

                hex::encode_to_slice(chunk, &mut message[..chunk.len() * 2])
                    .map_err(|_| ViewError::Unknown)?;
                message[chunk.len() * 2] = 0; //null terminate

                let n_pages = (Sign::SIGN_HASH_SIZE * 2) / m_len;
                Ok(1 + n_pages as u8)
            } else {
                hex::encode_to_slice(&self.hash[..], &mut message[..Sign::SIGN_HASH_SIZE * 2])
                    .map_err(|_| ViewError::Unknown)?;
                message[Sign::SIGN_HASH_SIZE * 2] = 0; //null terminate
                Ok(1)
            }
        } else {
            Err(ViewError::NoData)
        }
    }

    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let (path, curve) = match Sign::get_derivation_info() {
            Err(e) => return (0, e as _),
            Ok(k) => k,
        };

        let (sig_size, sig) = match Sign::sign(*curve, path, &self.hash[..]) {
            Err(e) => return (0, e as _),
            Ok(k) => k,
        };

        let mut tx = 0;

        //reset globals to avoid skipping `Init`
        if let Err(e) = cleanup_globals() {
            return (0, e as _);
        }

        //write unsigned_hash to buffer
        if self.send_hash {
            tx += Sign::SIGN_HASH_SIZE;
            out[..Sign::SIGN_HASH_SIZE].copy_from_slice(&self.hash[..]);
        }

        //wrte signature to buffer
        tx += sig_size;
        out[Sign::SIGN_HASH_SIZE..Sign::SIGN_HASH_SIZE + sig_size]
            .copy_from_slice(&sig[..sig_size]);

        (tx, Error::Success as _)
    }

    fn reject(&mut self, _: &mut [u8]) -> (usize, u16) {
        let _ = cleanup_globals();
        (0, Error::CommandNotAllowed as _)
    }
}

fn cleanup_globals() -> Result<(), Error> {
    unsafe { PATH.take() };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        assert_error_code,
        dispatcher::{handle_apdu, CLA},
        handlers::{LegacyPacketType, ZPacketType},
        sys::set_out,
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
    fn apdu_blind_sign() {
        const MSG: [u8; 18] = *b"franceco@zondax.ch";

        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];

        buffer[0] = CLA;
        buffer[1] = INS_SIGN;
        buffer[2] = ZPacketType::Init.into();
        let len = prepare_buffer(&mut buffer, &[44, 1729, 0, 0], Curve::Ed25519);

        handle_apdu(&mut flags, &mut tx, 5 + len as u32, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        buffer[0] = CLA;
        buffer[1] = INS_SIGN;
        buffer[2] = ZPacketType::Last.into();
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
        buffer[1] = INS_SIGN;
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
