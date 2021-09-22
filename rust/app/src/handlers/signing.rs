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
    parser::{
        operations::{Operation, OperationType},
        DisplayableItem, Preemble,
    },
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
    pub fn sign<const LEN: usize>(
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
        let (rem, preemble) = Preemble::from_bytes(data).map_err(|_| Error::DataInvalid)?;

        let ui = match preemble {
            Preemble::Operation => SignUI {
                hash: unsigned_hash,
                send_hash,
                parsed: Some(Operation::new(rem).map_err(|_| Error::DataInvalid)?),
            },
            Preemble::Michelson => SignUI {
                hash: unsigned_hash,
                send_hash,
                parsed: None,
            },
            _ => return Err(Error::CommandNotAllowed),
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

pub(crate) struct SignUI {
    hash: [u8; Sign::SIGN_HASH_SIZE],
    send_hash: bool,
    parsed: Option<Operation<'static>>,
}

#[cfg(test)]
impl Operation<'static> {
    pub(crate) fn to_sign_ui(self) -> SignUI {
        SignUI {
            hash: [0; Sign::SIGN_HASH_SIZE],
            send_hash: false,
            parsed: Some(self),
        }
    }
}

impl SignUI {
    // Will find the operation that contains said item, as well as
    // return the index of the item in the operation
    fn find_op_with_item(
        &self,
        mut item_idx: u8,
    ) -> Result<Option<(u8, OperationType<'static>)>, ViewError> {
        item_idx -= 1; //remove branch idx

        //we shouldn't be here if parsed is None
        let mut parsed = self.parsed.ok_or(ViewError::Unknown)?;
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
        match self.parsed {
            None => Ok(1),
            Some(mut parsed) => {
                let ops = parsed.mut_ops();

                let mut items_counter = 1; //start with branch

                while let Some(op) = ops.parse_next().map_err(|_| ViewError::Unknown)? {
                    items_counter += op.ui_items();
                }

                Ok(items_counter as u8)
            }
        }
    }

    #[inline(never)]
    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        match self.parsed {
            None => match item_n {
                0 => {
                    let title_content = pic_str!(b"Sign Michelson Hash");
                    title[..title_content.len()].copy_from_slice(title_content);

                    let mut hex_buf = [0; Sign::SIGN_HASH_SIZE * 2];
                    //this is impossible that will error since the sizes are all checked
                    hex::encode_to_slice(self.hash, &mut hex_buf).unwrap();

                    handle_ui_message(&hex_buf[..], message, page)
                }
                _ => Err(ViewError::NoData),
            },
            Some(parsed) => {
                if let 0 = item_n {
                    let title_content = pic_str!(b"Operation");
                    title[..title_content.len()].copy_from_slice(title_content);

                    let mex = parsed.get_base58_branch().map_err(|_| ViewError::Unknown)?;

                    handle_ui_message(&mex[..], message, page)
                } else if let Some((item_n, op)) = self.find_op_with_item(item_n)? {
                    match op {
                        OperationType::Transfer(tx) => tx.render_item(item_n, title, message, page),
                        OperationType::Delegation(delegation) => {
                            delegation.render_item(item_n, title, message, page)
                        }
                        OperationType::Endorsement(endorsement) => {
                            endorsement.render_item(item_n, title, message, page)
                        }
                        OperationType::EndorsementWithSlot(endorsement) => {
                            endorsement.render_item(item_n, title, message, page)
                        }
                        OperationType::SeedNonceRevelation(snr) => {
                            snr.render_item(item_n, title, message, page)
                        }
                        OperationType::Ballot(vote) => {
                            vote.render_item(item_n, title, message, page)
                        }
                        OperationType::Reveal(rev) => rev.render_item(item_n, title, message, page),
                        OperationType::Proposals(props) => {
                            props.render_item(item_n, title, message, page)
                        }
                        OperationType::Origination(orig) => {
                            orig.render_item(item_n, title, message, page)
                        }
                        OperationType::ActivateAccount(act) => {
                            act.render_item(item_n, title, message, page)
                        }
                        OperationType::FailingNoop(fail) => {
                            fail.render_item(item_n, title, message, page)
                        }
                    }
                } else {
                    Err(ViewError::NoData)
                }
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
