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
    handlers::public_key::GetAddress,
    parser::{
        baking::{BlockData, EndorsementData},
        operations::Delegation,
        DisplayableItem, Preemble,
    },
    sys::{self, flash_slot::Wear, new_flash_slot},
    utils::{ApduBufferRead, Uploader},
};
use bolos::flash_slot::WearError;

const N_PAGES_BAKINGPATH: usize = 1;

type WearLeveller = Wear<'static, N_PAGES_BAKINGPATH>;

#[bolos::lazy_static]
static mut BAKINGPATH: WearLeveller =
    new_flash_slot!(N_PAGES_BAKINGPATH).expect("NVM might be corrupted");

#[derive(Debug, PartialEq, Clone)]
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

    pub fn try_from_bytes(from: &[u8; 52]) -> Result<Self, Error> {
        let curve = crypto::Curve::try_from(from[0]).map_err(|_| Error::DataInvalid)?;
        let components_length = from[1];

        if components_length > BIP32_MAX_LENGTH as u8 {
            return Err(Error::WrongLength);
        }

        let path = sys::crypto::bip32::BIP32Path::<BIP32_MAX_LENGTH>::read(
            &from[1..(2 + components_length * 4).into()],
        )
        .map_err(|_| Error::DataInvalid)?;

        Ok(Self { curve, path })
    }
}

impl From<Bip32PathAndCurve> for [u8; 52] {
    fn from(from: Bip32PathAndCurve) -> Self {
        let mut out = [0; 52];

        let curve = from.curve.into();
        out[0] = curve;

        let components = from.path.components();
        out[1] = components.len() as u8;

        for i in 0..components.len() {
            out[2 + i * 4..2 + (i + 1) * 4].copy_from_slice(&components[i].to_be_bytes()[..]);
        }
        out
    }
}

//TODO: Or grab this from signing.rs??
#[bolos::lazy_static]
static mut PATH: Option<(BIP32Path<BIP32_MAX_LENGTH>, Curve)> = None;

pub struct Baking;

impl Baking {
    //FIXME: Ideally grab this function from public_key.rs?
    #[inline(never)]
    fn get_public(key: crypto::PublicKey, buffer: &mut [u8]) -> Result<u32, Error> {
        let key = key.as_ref();
        let len = key.len();
        buffer[..len].copy_from_slice(&key);
        Ok(len as u32)
    }

    //FIXME: make this part of impl Bip32PathAndCurve?
    #[inline(never)]
    fn check_and_store_path(path_and_curve: Bip32PathAndCurve) -> Result<(), Error> {
        //Check if the current baking path in NVM is un-initialized
        let current_path = unsafe { BAKINGPATH.read() };
        if let Err(error_msg) = current_path {
            if error_msg != WearError::Uninitialized {
                //Something else went wrong
                Err(Error::ExecutionError)
            } else {
                //store path and curve in NVM
                let data: [u8; 52] = path_and_curve.into();
                unsafe { BAKINGPATH.write(data) }.map_err(|_| Error::ExecutionError)?;
                Ok(())
            }
        } else {
            //path seems to be initialized
            Err(Error::ApduCodeConditionsNotSatisfied)
        }
    }

    //FIXME: make this part of impl Bip32PathAndCurve?
    #[inline(never)]
    fn check_and_delete_path() -> Result<(), Error> {
        //Check if the current baking path in NVM is initialized
        let current_path = unsafe { BAKINGPATH.read() };
        if current_path.is_err() {
            //There was no initial path
            Err(Error::ApduCodeConditionsNotSatisfied)
        } else {
            //path seems to be initialized so we can remove it
            unsafe { BAKINGPATH.format() }.map_err(|_| Error::ExecutionError)?;
            Ok(())
        }
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
        let current_path = unsafe { BAKINGPATH.read() }.map_err(|_| Error::DataInvalid)?;
        //path seems to be initialized so we can return it
        //check if it is a good path
        //TODO: otherwise return an error and show that on screen (corrupted NVM??)
        let nvm_bip = Bip32PathAndCurve::try_from_bytes(&current_path)?;

        if nvm_bip.path != *path || nvm_bip.curve != *curve {
            //TODO: show that bip32 paths don't match??
            Err(Error::DataInvalid)
        } else {
            Ok(())
        }
    }

    #[inline(never)]
    pub fn baker_sign(
        p2: u8,
        init_data: &[u8],
        cdata: &'static [u8],
        flags: &mut u32,
    ) -> Result<u32, Error> {
        let curve = Curve::try_from(p2).map_err(|_| Error::InvalidP1P2)?;
        let path =
            BIP32Path::<BIP32_MAX_LENGTH>::read(init_data).map_err(|_| Error::DataInvalid)?;

        Self::check_with_nvm_pathandcurve(&curve, &path)?;

        unsafe { PATH.replace((path, curve)) };
        let hw = HWM::read()?;
        //do watermarks checks

        let digest = Self::blake2b_digest(cdata)?;
        let (rem, preemble) = Preemble::from_bytes(cdata).map_err(|_| Error::DataInvalid)?;

        let baking_ui = match preemble {
            Preemble::Endorsement => {
                let (_, endorsement) =
                    EndorsementData::from_bytes(rem).map_err(|_| Error::DataInvalid)?;
                if !endorsement.validate_with_watermark(&hw) {
                    return Err(Error::DataInvalid);
                    //TODO: show endorsement data on screen
                }

                BakingSignUI::Endorsement {
                    data: endorsement,
                    digest,
                }
            }
            Preemble::Block => {
                let (_, blockdata) = BlockData::from_bytes(rem).map_err(|_| Error::DataInvalid)?;

                if !blockdata.validate_with_watermark(&hw) {
                    return Err(Error::DataInvalid);
                }

                BakingSignUI::BlockLevel {
                    data: blockdata,
                    digest,
                }
            }
            Preemble::Operation => {
                use crate::parser::operations::{Operation, OperationType};

                let operation = Operation::new(rem).map_err(|_| Error::DataInvalid)?;

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

                        BakingSignUI::Delegation {
                            digest,
                            data: deleg,
                        }
                    }
                    _ => return Err(Error::CommandNotAllowed),
                }
            }
            _ => return Err(Error::CommandNotAllowed),
        };

        unsafe { baking_ui.show(flags) }
            .map_err(|_| Error::ExecutionError)
            .map(|_| 0)
    }
}

enum BakingSignUI {
    Endorsement {
        digest: [u8; 32],
        data: EndorsementData<'static>,
    },
    BlockLevel {
        digest: [u8; 32],
        data: BlockData,
    },
    Delegation {
        digest: [u8; 32],
        data: Delegation<'static>,
    },
}

impl Viewable for BakingSignUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        let n = match self {
            BakingSignUI::Endorsement { data, .. } => data.num_items(),
            BakingSignUI::BlockLevel { data, .. } => data.num_items(),
            BakingSignUI::Delegation { data, .. } => data.num_items(),
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
            BakingSignUI::Delegation { data, .. } => data.render_item(item_n, title, message, page),
        }
    }

    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let digest = match self {
            BakingSignUI::Endorsement { digest, data } => {
                if HWM::write(WaterMark {
                    level: data.level,
                    endorsement: true,
                })
                .is_err()
                {
                    return (0, Error::ExecutionError as _);
                }

                digest
            }
            BakingSignUI::BlockLevel { digest, data } => {
                if HWM::write(WaterMark {
                    level: data.level,
                    endorsement: false,
                })
                .is_err()
                {
                    return (0, Error::ExecutionError as _);
                }

                digest
            }
            BakingSignUI::Delegation { digest, .. } => digest,
        };

        let current_path = match unsafe { BAKINGPATH.read() } {
            Ok(path) => path,
            Err(_) => return (0, Error::ExecutionError as _),
        };

        //path seems to be initialized so we can return it
        //check if it is a good path
        //TODO: otherwise return an error and show that on screen (corrupted NVM??)
        let bip32_nvm = match Bip32PathAndCurve::try_from_bytes(&current_path) {
            Ok(bip) => bip,
            Err(e) => return (0, e as _),
        };

        let secret = bip32_nvm.curve.to_secret(&bip32_nvm.path);

        let mut sig = [0; 100];
        let sz = match secret.sign(digest, &mut sig[..]) {
            Ok(sz) => sz,
            Err(_) => return (0, Error::ExecutionError as _),
        };

        //reset globals to avoid skipping `Init`
        if let Err(e) = cleanup_globals() {
            return (0, e as _);
        }

        let mut tx = 0;

        //write unsigned_hash to buffer
        out[tx..tx + 32].copy_from_slice(digest);
        tx += 32;

        //wrte signature to buffer
        out[tx..tx + sz].copy_from_slice(&sig[..sz]);
        tx += sz;

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

pub struct AuthorizeBaking;
pub struct DeAuthorizeBaking;
pub struct QueryAuthKey;
pub struct QueryAuthKeyWithCurve;

impl ApduHandler for AuthorizeBaking {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        let req_confirmation = buffer.p1() >= 1;

        //TODO: show confirmation
        if !req_confirmation {
            return Err(Error::ApduCodeConditionsNotSatisfied);
        }

        let curve = crypto::Curve::try_from(buffer.p2()).map_err(|_| Error::InvalidP1P2)?;

        let cdata = buffer.payload().map_err(|_| Error::DataInvalid)?;
        let bip32_path = sys::crypto::bip32::BIP32Path::<BIP32_MAX_LENGTH>::read(cdata)
            .map_err(|_| Error::DataInvalid)?;

        let key = GetAddress::new_key(curve, &bip32_path).map_err(|_| Error::ExecutionError)?;
        let path_and_data = Bip32PathAndCurve::new(curve, bip32_path);
        Baking::check_and_store_path(path_and_data)?;

        let buffer = buffer.write();
        let pk_len = Baking::get_public(key, &mut buffer[1..])?;
        buffer[0] = pk_len as u8;

        HWM::reset(0).map_err(|_| Error::Busy)?;

        *tx = pk_len + 1;

        Ok(())
    }
}

impl ApduHandler for DeAuthorizeBaking {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        let req_confirmation = buffer.p1() >= 1;

        if !req_confirmation {
            return Err(Error::ApduCodeConditionsNotSatisfied);
        }

        //TODO: show confirmation of deletion on screen
        //FIXME: check if we need to format the HWM??
        HWM::format()?;
        Baking::check_and_delete_path()?;

        Ok(())
    }
}

impl ApduHandler for QueryAuthKey {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        let req_confirmation = buffer.p1() >= 1;

        if req_confirmation {
            //TODO show confirmation on screen??
        }
        //Check if the current baking path in NVM is initialized
        let current_path =
            unsafe { BAKINGPATH.read() }.map_err(|_| Error::ApduCodeConditionsNotSatisfied)?;
        //path seems to be initialized so we can return it
        //check if it is a good path
        //TODO: otherwise return an error and show that on screen (corrupted NVM??)
        Bip32PathAndCurve::try_from_bytes(&current_path)?;

        let bip32_pathsize = current_path[1] as usize;

        let buffer = buffer.write();
        buffer[0..1 + 4 * bip32_pathsize].copy_from_slice(&current_path[1..2 + 4 * bip32_pathsize]);

        *tx = 1 + 4 * bip32_pathsize as u32;
        Ok(())
    }
}

impl ApduHandler for QueryAuthKeyWithCurve {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        let req_confirmation = buffer.p1() >= 1;

        if req_confirmation {
            //TODO show confirmation on screen??
        }
        //Check if the current baking path in NVM is initialized
        let current_path =
            unsafe { BAKINGPATH.read() }.map_err(|_| Error::ApduCodeConditionsNotSatisfied)?;
        //path seems to be initialized so we can return it
        //check if it is a good path
        //TODO: otherwise return an error and show that on screen (corrupted NVM??)
        Bip32PathAndCurve::try_from_bytes(&current_path)?;

        let bip32_pathsize = current_path[1] as usize;

        let buffer = buffer.write();
        buffer[0..2 + 4 * bip32_pathsize].copy_from_slice(&current_path[0..2 + 4 * bip32_pathsize]);
        *tx = 2 + 4 * bip32_pathsize as u32;

        Ok(())
    }
}

impl ApduHandler for Baking {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        if let Some(upload) = Uploader::new(Self).upload(&buffer)? {
            *tx = Self::baker_sign(upload.p2, upload.first, upload.data, flags)?;
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
        assert_eq!(derived.unwrap(), path_and_curve);
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
