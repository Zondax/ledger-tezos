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
    pic_str, PIC,
};
use zemu_sys::{Show, ViewError, Viewable};

use crate::{
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::Curve,
    dispatcher::ApduHandler,
    handlers::handle_ui_message,
    parser::operations::{Operation, OperationType},
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
    pub fn start_sign(
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

        let ui = SignUI {
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
            *tx = Self::start_sign(true, upload.p2, upload.first, upload.data, flags)?;
        }

        Ok(())
    }
}

struct SignUI {
    hash: [u8; Sign::SIGN_HASH_SIZE],
    send_hash: bool,
    parsed: Operation<'static>,
}

impl SignUI {
    // Will find the operation that contains said item, as well as
    // return the index of the item in the operation
    fn find_op_with_item(
        &self,
        mut item_idx: u8,
    ) -> Result<Option<(u8, OperationType<'static>)>, ViewError> {
        item_idx -= 1; //remove branch idx

        let mut parsed = self.parsed;
        let ops = parsed.mut_ops();

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

impl Viewable for SignUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        let mut parsed = self.parsed;
        let ops = parsed.mut_ops();

        let mut items_counter = 1; //start with branch

        while let Some(op) = ops.parse_next().map_err(|_| ViewError::Unknown)? {
            items_counter += op.ui_items();
        }

        Ok(items_counter as u8)
    }

    #[inline(never)]
    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        if let 0 = item_n {
            let title_content = pic_str!(b"Operation");
            title[..title_content.len()].copy_from_slice(title_content);

            let mut mex = [0; 51];
            self.parsed
                .base58_branch(&mut mex)
                .map_err(|_| ViewError::Unknown)?;

            handle_ui_message(&mex[..], message, page)
        } else if let Some((item_n, op)) = self.find_op_with_item(item_n)? {
            use lexical_core::{write as itoa, Number};
            let mut zarith_buf = [0; usize::FORMATTED_SIZE_DECIMAL];

            match op {
                OperationType::Transfer(tx) => {
                    match item_n {
                        //source
                        0 => {
                            let title_content = pic_str!(b"Source");
                            title[..title_content.len()].copy_from_slice(title_content);

                            let (crv, hash) = tx.source();

                            let addr = crate::handlers::public_key::Addr::from_hash(hash, *crv)
                                .map_err(|_| ViewError::Unknown)?;

                            let mex = addr.to_base58();
                            handle_ui_message(&mex[..], message, page)
                        }
                        //destination
                        1 => {
                            let title_content = pic_str!(b"Destination");
                            title[..title_content.len()].copy_from_slice(title_content);

                            let mut cid = [0; 36];
                            tx.destination()
                                .base58(&mut cid)
                                .map_err(|_| ViewError::Unknown)?;

                            handle_ui_message(&cid[..], message, page)
                        }
                        //amount
                        2 => {
                            let title_content = pic_str!(b"Amount");
                            title[..title_content.len()].copy_from_slice(title_content);

                            let (_, amount) =
                                tx.amount().read_as::<usize>().ok_or(ViewError::Unknown)?;

                            handle_ui_message(itoa(amount, &mut zarith_buf), message, page)
                        }
                        //fee
                        3 => {
                            let title_content = pic_str!(b"Fee");
                            title[..title_content.len()].copy_from_slice(title_content);

                            let (_, fee) = tx.fee().read_as::<usize>().ok_or(ViewError::Unknown)?;

                            handle_ui_message(itoa(fee, &mut zarith_buf), message, page)
                        }
                        //has_parameters
                        4 => {
                            let title_content = pic_str!(b"Parameters");
                            title[..title_content.len()].copy_from_slice(title_content);

                            let parameters = tx.parameters();

                            let msg = match parameters {
                                Some(_) => pic_str!("has parameters..."),
                                None => pic_str!("no parameters"),
                            };

                            handle_ui_message(msg.as_bytes(), message, page)
                        }
                        //gas_limit
                        5 => {
                            let title_content = pic_str!(b"Gas Limit");
                            title[..title_content.len()].copy_from_slice(title_content);

                            let (_, gas_limit) = tx
                                .gas_limit()
                                .read_as::<usize>()
                                .ok_or(ViewError::Unknown)?;

                            handle_ui_message(itoa(gas_limit, &mut zarith_buf), message, page)
                        }
                        //storage_limit
                        6 => {
                            let title_content = pic_str!(b"Storage Limit");
                            title[..title_content.len()].copy_from_slice(title_content);

                            let (_, storage_limit) = tx
                                .storage_limit()
                                .read_as::<usize>()
                                .ok_or(ViewError::Unknown)?;

                            handle_ui_message(itoa(storage_limit, &mut zarith_buf), message, page)
                        }
                        //counter
                        7 => {
                            let title_content = pic_str!(b"Counter");
                            title[..title_content.len()].copy_from_slice(title_content);

                            let (_, counter) =
                                tx.counter().read_as::<usize>().ok_or(ViewError::Unknown)?;

                            handle_ui_message(itoa(counter, &mut zarith_buf), message, page)
                        }
                        _ => panic!("should be next operation"),
                    }
                }
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
            out[tx..tx + Sign::SIGN_HASH_SIZE].copy_from_slice(&self.hash[..]);
            tx += Sign::SIGN_HASH_SIZE;
        }

        //wrte signature to buffer
        out[tx..tx + sig_size].copy_from_slice(&sig[..sig_size]);
        tx += sig_size;

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
