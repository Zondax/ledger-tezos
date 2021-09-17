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

use zemu_sys::{Show, ViewError, Viewable};

use bolos::{
    crypto::bip32::BIP32Path,
    hash::{Blake2b, Hasher},
};

use crate::{
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::{self, Curve},
    dispatcher::ApduHandler,
    handlers::hwm::{WaterMark, HWM},
    parser::{
        baking::{BlockData, EndorsementData},
        operations::Delegation,
        DisplayableItem, Preemble,
    },
    sys::{self, flash_slot::Wear, new_flash_slot},
    utils::{ApduBufferRead, Uploader},
};

const N_PAGES_BAKINGPATH: usize = 1;

type WearLeveller = Wear<'static, N_PAGES_BAKINGPATH>;

#[bolos::lazy_static]
static mut BAKINGPATH: WearLeveller =
    new_flash_slot!(N_PAGES_BAKINGPATH).expect("NVM might be corrupted");

#[derive(Debug, PartialEq, Clone)]
/// Utility struct to store and read BIP32Path and Curve from NVM slots
///
/// # Codec
///
/// [0] = 0x00 | 0x2A; 0x00 indicates that the data left blank
///
/// [1] = `Curve`; byte representation of (`Curve`)[crypto::Curve]
///
/// [2] = number of components in the path (i)
///
/// [3..3+i*4] = `BIP32Path`;
/// byte representation of (`BIP32Path`)[sys::crypto::bip32::BIP32Path]
pub struct Bip32PathAndCurve {
    pub curve: crypto::Curve,
    pub path: sys::crypto::bip32::BIP32Path<BIP32_MAX_LENGTH>,
}

impl Bip32PathAndCurve {
    pub fn new(
        curve: crypto::Curve,
        path: sys::crypto::bip32::BIP32Path<BIP32_MAX_LENGTH>,
    ) -> Self {
        Self { curve, path }
    }

    /// Attempt to read a Bip32PathAndCurve from some bytes
    pub fn try_from_bytes(from: &[u8; 52]) -> Result<Option<Self>, Error> {
        //the slot could have been purposely emptied of data
        // for example when we deauthorize
        if from[0] == 0 {
            return Ok(None);
        }
        let curve = crypto::Curve::try_from(from[1]).map_err(|_| Error::DataInvalid)?;
        let components_length = from[2];

        if components_length > BIP32_MAX_LENGTH as u8 {
            return Err(Error::WrongLength);
        }

        //we reread from 2 since `read` expectes
        // the components prefixed with the number of components,
        // so we also + 1 to get that prefix
        let path = sys::crypto::bip32::BIP32Path::<BIP32_MAX_LENGTH>::read(
            &from[2..2 + 1 + 4 * components_length as usize],
        )
        .map_err(|_| Error::DataInvalid)?;

        Ok(Some(Self { curve, path }))
    }

    ///Used to set a slot as empty when writing to NVM
    ///
    /// Useful on deauthorization
    pub fn empty() -> [u8; 52] {
        [0; 52]
    }
}

impl From<Bip32PathAndCurve> for [u8; 52] {
    fn from(from: Bip32PathAndCurve) -> Self {
        let mut out = [0; 52];
        out[0] = 42; //we write to indicate that we actually have data here

        let curve = from.curve.into();
        out[1] = curve;

        let components = from.path.components();
        out[2] = components.len() as u8;

        out[3..]
            .chunks_exact_mut(4)
            .zip(components)
            .for_each(|(chunk, comp)| chunk.copy_from_slice(&comp.to_be_bytes()[..]));

        out
    }
}

pub struct Baking;

impl Baking {
    //FIXME: Ideally grab this function from public_key.rs?
    #[inline(never)]
    fn get_public(key: crypto::PublicKey, buffer: &mut [u8]) -> Result<u32, Error> {
        let key = key.as_ref();
        let len = key.len();
        buffer[..len].copy_from_slice(key);
        Ok(len as u32)
    }

    //FIXME: grab this from signing.rs
    #[inline(never)]
    fn blake2b_digest(buffer: &[u8]) -> Result<[u8; 32], Error> {
        Blake2b::digest(buffer).map_err(|_| Error::ExecutionError)
    }

    #[inline(never)]
    fn check_with_nvm_pathandcurve(
        curve: &crypto::Curve,
        path: &sys::crypto::bip32::BIP32Path<BIP32_MAX_LENGTH>,
    ) -> Result<(), Error> {
        //Check if the current baking path in NVM is initialized
        let current_path =
            unsafe { BAKINGPATH.read() }.map_err(|_| Error::ApduCodeConditionsNotSatisfied)?;
        //path seems to be initialized so we can return it
        //check if it is a good path
        //TODO: otherwise return an error and show that on screen (corrupted NVM??)
        let nvm_bip = Bip32PathAndCurve::try_from_bytes(current_path)?
            .ok_or(Error::ApduCodeConditionsNotSatisfied)?;

        if nvm_bip.path != *path || nvm_bip.curve != *curve {
            //TODO: show that bip32 paths don't match??
            Err(Error::DataInvalid)
        } else {
            Ok(())
        }
    }

    #[inline(never)]
    fn handle_endorsement(
        input: &'static [u8],
        send_hash: bool,
        digest: [u8; 32],
    ) -> Result<BakingSignUI, Error> {
        let hw = HWM::read()?;

        let (_, endorsement) =
            EndorsementData::from_bytes(input).map_err(|_| Error::DataInvalid)?;
        if !endorsement.validate_with_watermark(&hw) {
            return Err(Error::DataInvalid);
            //TODO: show endorsement data on screen
        }

        Ok(BakingSignUI::Endorsement {
            send_hash,
            data: endorsement,
            digest,
        })
    }

    #[inline(never)]
    fn handle_blockdata(
        input: &'static [u8],
        send_hash: bool,
        digest: [u8; 32],
    ) -> Result<BakingSignUI, Error> {
        let hw = HWM::read()?;

        let (_, blockdata) = BlockData::from_bytes(input).map_err(|_| Error::DataInvalid)?;

        if !blockdata.validate_with_watermark(&hw) {
            return Err(Error::DataInvalid);
        }

        Ok(BakingSignUI::BlockLevel {
            send_hash,
            data: blockdata,
            digest,
        })
    }

    #[inline(never)]
    fn handle_delegation(
        input: &'static [u8],
        send_hash: bool,
        digest: [u8; 32],
    ) -> Result<BakingSignUI, Error> {
        use crate::parser::operations::{Operation, OperationType};

        let operation = Operation::new(input).map_err(|_| Error::DataInvalid)?;

        let op = operation
            .ops()
            .peek_next()
            .map_err(|_| Error::DataInvalid)?
            .ok_or(Error::DataInvalid)?;

        match op {
            OperationType::Delegation(deleg) => {
                //verify that delegation.source == delegation.delegate
                // and it matches the authorized key for baking
                // (BAKINGPATH)

                Ok(BakingSignUI::Delegation {
                    send_hash,
                    digest,
                    branch: operation.branch(),
                    data: deleg,
                })
            }
            _ => Err(Error::CommandNotAllowed),
        }
    }

    #[inline(never)]
    pub fn baker_sign(
        send_hash: bool,
        p2: u8,
        init_data: &[u8],
        cdata: &'static [u8],
        flags: &mut u32,
    ) -> Result<u32, Error> {
        crate::sys::zemu_log_stack("Baking::baker_sign\x00");

        let curve = Curve::try_from(p2).map_err(|_| Error::InvalidP1P2)?;
        let path =
            BIP32Path::<BIP32_MAX_LENGTH>::read(init_data).map_err(|_| Error::DataInvalid)?;

        Self::check_with_nvm_pathandcurve(&curve, &path)?;

        let digest = Self::blake2b_digest(cdata)?;
        let (rem, preemble) = Preemble::from_bytes(cdata).map_err(|_| Error::DataInvalid)?;

        let baking_ui = match preemble {
            Preemble::Endorsement => Self::handle_endorsement(rem, send_hash, digest)?,
            Preemble::Block => Self::handle_blockdata(rem, send_hash, digest)?,
            Preemble::Operation => Self::handle_delegation(rem, send_hash, digest)?,
            _ => return Err(Error::CommandNotAllowed),
        };

        unsafe { baking_ui.show(flags) }
            .map_err(|_| Error::ExecutionError)
            .map(|_| 0)
    }
}

enum BakingSignUI {
    Endorsement {
        send_hash: bool,
        digest: [u8; 32],
        data: EndorsementData<'static>,
    },
    BlockLevel {
        send_hash: bool,
        digest: [u8; 32],
        data: BlockData,
    },
    Delegation {
        send_hash: bool,
        digest: [u8; 32],
        branch: &'static [u8; 32],
        data: Delegation<'static>,
    },
}

impl Viewable for BakingSignUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        let n = match self {
            BakingSignUI::Endorsement { data, .. } => data.num_items(),
            BakingSignUI::BlockLevel { data, .. } => data.num_items(),
            BakingSignUI::Delegation { data, .. } => 1 + data.num_items(),
        };

        Ok(n as u8)
    }

    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        match self {
            BakingSignUI::Endorsement { data, .. } => {
                data.render_item(item_n, title, message, page)
            }
            BakingSignUI::BlockLevel { data, .. } => data.render_item(item_n, title, message, page),
            BakingSignUI::Delegation { data, branch, .. } => {
                if let 0 = item_n {
                    use bolos::{pic_str, PIC};

                    let title_content = pic_str!(b"Operation");
                    title[..title_content.len()].copy_from_slice(title_content);

                    let mex = crate::parser::operations::Operation::base58_branch(branch)
                        .map_err(|_| ViewError::Unknown)?;

                    crate::handlers::handle_ui_message(&mex[..], message, page)
                } else {
                    data.render_item(item_n - 1, title, message, page)
                }
            }
        }
    }

    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let (digest, send_hash) = match self {
            BakingSignUI::Endorsement {
                digest,
                data,
                send_hash,
            } => {
                if HWM::write(WaterMark {
                    level: data.level,
                    endorsement: true,
                })
                .is_err()
                {
                    return (0, Error::ExecutionError as _);
                }

                (digest, send_hash)
            }
            BakingSignUI::BlockLevel {
                digest,
                data,
                send_hash,
            } => {
                if HWM::write(WaterMark {
                    level: data.level,
                    endorsement: false,
                })
                .is_err()
                {
                    return (0, Error::ExecutionError as _);
                }

                (digest, send_hash)
            }
            BakingSignUI::Delegation {
                digest, send_hash, ..
            } => (digest, send_hash),
        };

        let current_path = match unsafe { BAKINGPATH.read() } {
            Ok(path) => path,
            Err(_) => return (0, Error::ExecutionError as _),
        };

        //path seems to be initialized so we can return it
        //check if it is a good path
        //TODO: otherwise return an error and show that on screen (corrupted NVM??)
        let bip32_nvm = match Bip32PathAndCurve::try_from_bytes(current_path) {
            Ok(Some(bip)) => bip,
            //should never reach here since we had checked it earlier
            Ok(None) => return (0, Error::ApduCodeConditionsNotSatisfied as _),
            Err(e) => return (0, e as _),
        };

        let secret = bip32_nvm.curve.to_secret(&bip32_nvm.path);

        let mut sig = [0; 100];
        let sz = match secret.sign(digest, &mut sig[..]) {
            Ok(sz) => sz,
            Err(_) => return (0, Error::ExecutionError as _),
        };

        let mut tx = 0;

        if *send_hash {
            //write unsigned_hash to buffer
            out[tx..tx + 32].copy_from_slice(digest);
            tx += 32;
        }

        //wrte signature to buffer
        out[tx..tx + sz].copy_from_slice(&sig[..sz]);
        tx += sz;

        (tx, Error::Success as _)
    }

    fn reject(&mut self, _: &mut [u8]) -> (usize, u16) {
        (0, Error::CommandNotAllowed as _)
    }
}

mod authorization;
pub use authorization::{AuthorizeBaking, DeAuthorizeBaking};

mod queryauth;
pub use queryauth::{QueryAuthKey, QueryAuthKeyWithCurve};

impl ApduHandler for Baking {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        if let Some(upload) = Uploader::new(Self).upload(&buffer)? {
            *tx = Self::baker_sign(true, upload.p2, upload.first, upload.data, flags)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto;
    use bolos::crypto::bip32::BIP32Path;

    use super::*;

    #[test]
    fn check_bip32andpath_frombytes() {
        let curve = crypto::Curve::Ed25519;
        let pathdata = &[44, 1729, 0, 0];

        let path =
            BIP32Path::<BIP32_MAX_LENGTH>::new(pathdata.iter().map(|n| 0x8000_0000 + n)).unwrap();
        let path_and_curve = Bip32PathAndCurve::new(curve, path);

        let data: [u8; 52] = path_and_curve.clone().into();
        let derived = Bip32PathAndCurve::try_from_bytes(&data);

        assert!(derived.is_ok());
        assert_eq!(derived.unwrap().unwrap(), path_and_curve);
    }

    #[test]
    fn test_endorsement_data() {
        let mut v = std::vec::Vec::with_capacity(1 + 4 + 32);
        v.push(0x00); //invalid preemble
        v.extend_from_slice(&1_u32.to_be_bytes());
        v.extend_from_slice(&[0u8; 32]);
        v.push(0x05);
        v.extend_from_slice(&15_u32.to_be_bytes());

        let (_, endorsement) = EndorsementData::from_bytes(&v[1..]).unwrap();
        assert_eq!(endorsement.chain_id, 1);
        assert_eq!(endorsement.branch, &[0u8; 32]);
        assert_eq!(endorsement.tag, 5);
        assert_eq!(endorsement.level, 15);
    }
}
