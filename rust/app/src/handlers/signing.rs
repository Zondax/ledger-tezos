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
    constants::{tzprefix::KT1, ApduError as Error, BIP32_MAX_LENGTH},
    crypto::Curve,
    dispatcher::ApduHandler,
    handlers::handle_ui_message,
    parser::operations::{ContractID, Operation, OperationType},
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
        data: &'static [u8],
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
            parsed: Operation::new(data).map_err(|_| Error::DataInvalid)?,
        };

        unsafe { ui.show(flags) }
            .map_err(|_| Error::ExecutionError)
            .map(|_| 0)
    }
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

        if let Some(upload) = Uploader::new(Self).upload(&buffer)? {
            *tx = Self::blind_sign(true, upload.p2, upload.first, upload.data, flags)?;
        }

        Ok(())
    }
}

struct BlindSignUi {
    hash: [u8; Sign::SIGN_HASH_SIZE],
    send_hash: bool,
    parsed: Operation<'static>,
}

impl BlindSignUi {
    // Will find the operation that contains said item, as well as
    // return the index of the item in the operation
    fn find_op_with_item(
        &self,
        mut item_idx: u8,
    ) -> Result<Option<(u8, OperationType<'static>)>, ViewError> {
        item_idx -= 1; //remove branch idx

        let mut parsed = self.parsed.clone();
        let mut ops = parsed.mut_ops();

        //we don't call this if we haven't verified all info first
        while let Some(op) = ops.parse_next().map_err(|_| ViewError::Unknown)? {
            let n = op.ui_items() as u8;

            if n > item_idx {
                //we return the remaining item_idx so we can navigate to it
                return Ok(Some((item_idx, op)));
            } else {
                //decrease item_idx by n items and check next operation
                item_idx -= n;
            }
        }

        Ok(None)
    }
}

impl Viewable for BlindSignUi {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        let mut parsed = self.parsed.clone();
        let mut ops = parsed.mut_ops();

        let mut items_counter = 1; //start with branch

        while let Some(op) = ops.parse_next().map_err(|_| ViewError::Unknown)? {
            items_counter += op.ui_items();
        }

        Ok(items_counter as u8)
    }

    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        if let 0 = item_n {
            let title_content = bolos::PIC::new(b"Operation\x00").into_inner();
            title[..title_content.len()].copy_from_slice(title_content);

            let mut mex = [0; 51];
            self.parsed
                .base58_branch(&mut mex)
                .map_err(|_| ViewError::Unknown);

            let m_len = message.len() - 1; //null byte terminator
            if m_len <= mex.len() {
                let chunk = mex
                    .chunks(m_len / 2) //divide in non-overlapping chunks
                    .nth(page as usize) //get the nth chunk
                    .ok_or(ViewError::Unknown)?;

                message[..chunk.len()].copy_from_slice(&chunk[..]);
                message[chunk.len() * 2] = 0; //null terminate

                let n_pages = mex.len() / m_len;
                Ok(1 + n_pages as u8)
            } else {
                message[..mex.len()].copy_from_slice(&mex[..]);
                message[mex.len()] = 0; //null terminate
                Ok(1)
            }
        } else {
            if let Some((item_n, op)) = self.find_op_with_item(item_n)? {
                match op {
                    OperationType::Transfer(tx) => {
                        let zarith_str = bolos::PIC::new(b"zarith").into_inner();

                        let n_pages = match item_n {
                            //source
                            0 => {
                                let title_content = bolos::PIC::new(b"Source\x00").into_inner();
                                title[..title_content.len()].copy_from_slice(title_content);

                                let (crv, hash) = tx.source();

                                let addr = crate::handlers::public_key::Addr::from_hash(hash, *crv)
                                    .map_err(|_| ViewError::Unknown)?;

                                let mex = addr.to_base58();
                                handle_ui_message(&mex[..], message, page)
                            }
                            //destination
                            1 => {
                                let title_content =
                                    bolos::PIC::new(b"Destination\x00").into_inner();
                                title[..title_content.len()].copy_from_slice(title_content);

                                let mut cid = [0; 36];
                                tx.destination()
                                    .base58(&mut cid)
                                    .map_err(|_| ViewError::Unknown)?;

                                handle_ui_message(&cid[..], message, page)
                            }
                            //amount
                            2 => {
                                let title_content = bolos::PIC::new(b"Amount\x00").into_inner();
                                title[..title_content.len()].copy_from_slice(title_content);

                                let _amount = tx.amount();

                                handle_ui_message(zarith_str, message, page)
                            }
                            //fee
                            3 => {
                                let title_content = bolos::PIC::new(b"Fee\x00").into_inner();
                                title[..title_content.len()].copy_from_slice(title_content);

                                let _fee = tx.fee();

                                handle_ui_message(zarith_str, message, page)
                            }
                            //has_parameters
                            4 => {
                                let title_content = bolos::PIC::new(b"Parameters\x00").into_inner();
                                title[..title_content.len()].copy_from_slice(title_content);

                                let parameters = tx.parameters();

                                let msg = match parameters {
                                    Some(_) => "has parameters...",
                                    None => "no parameters",
                                };

                                handle_ui_message(
                                    bolos::PIC::new(msg).into_inner().as_bytes(),
                                    message,
                                    page,
                                )
                            }
                            //gas_limit
                            5 => {
                                let title_content = bolos::PIC::new(b"Gas Limit\x00").into_inner();
                                title[..title_content.len()].copy_from_slice(title_content);

                                let _gas_limit = tx.gas_limit();

                                handle_ui_message(zarith_str, message, page)
                            }
                            //storage_limit
                            6 => {
                                let title_content =
                                    bolos::PIC::new(b"Storage Limit\x00").into_inner();
                                title[..title_content.len()].copy_from_slice(title_content);

                                let _storage_limit = tx.storage_limit();

                                handle_ui_message(zarith_str, message, page)
                            }
                            //counter
                            7 => {
                                let title_content = bolos::PIC::new(b"Counter\x00").into_inner();
                                title[..title_content.len()].copy_from_slice(title_content);

                                let _counter = tx.counter();

                                handle_ui_message(zarith_str, message, page)
                            }
                            _ => panic!("should be next operation"),
                        }?;

                        Ok(1 + n_pages)
                    }
                }
            } else {
                Err(ViewError::NoData)
            }
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
        dispatcher::{handle_apdu, CLA, INS_SIGN},
        handlers::ZPacketType,
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
}
