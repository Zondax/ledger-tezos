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
    pic::PIC,
};

use crate::{
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::Curve,
    dispatcher::ApduHandler,
    handlers::{hwm::HWM, signing::Sign},
    parser::{
        baking::{BlockData, EndorsementData},
        operations::{Delegation, Reveal},
        DisplayableItem, Preemble,
    },
    sys::{flash_slot::Wear, new_flash_slot},
    utils::{ApduBufferRead, ApduPanic, Uploader},
};

const N_PAGES_BAKINGPATH: usize = 1;

type WearLeveller = Wear<'static, N_PAGES_BAKINGPATH>;

#[bolos::lazy_static]
static mut BAKINGPATH: WearLeveller =
    new_flash_slot!(N_PAGES_BAKINGPATH).apdu_expect("NVM might be corrupted");

#[derive(PartialEq, Clone)]
#[cfg_attr(test, derive(Debug))]
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
struct Bip32PathAndCurve {
    curve: Curve,
    path: BIP32Path<BIP32_MAX_LENGTH>,
}

impl Bip32PathAndCurve {
    pub fn new(curve: Curve, path: BIP32Path<BIP32_MAX_LENGTH>) -> Self {
        Self { curve, path }
    }

    /// Attempt to read a Bip32PathAndCurve from some bytes
    pub fn try_from_bytes(from: &[u8; 52]) -> Result<Option<Self>, Error> {
        //the slot could have been purposely emptied of data
        // for example when we deauthorize
        if from[0] == 0 {
            return Ok(None);
        }
        let curve = Curve::try_from(from[1]).map_err(|_| Error::DataInvalid)?;
        let components_length = from[2];

        if components_length > BIP32_MAX_LENGTH as u8 {
            return Err(Error::WrongLength);
        }

        //we reread from 2 since `read` expectes
        // the components prefixed with the number of components,
        // so we also + 1 to get that prefix
        let path =
            BIP32Path::<BIP32_MAX_LENGTH>::read(&from[2..2 + 1 + 4 * components_length as usize])
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
    #[inline(never)]
    fn blake2b_digest_into(
        buffer: &[u8],
        out: &mut [u8; Sign::SIGN_HASH_SIZE],
    ) -> Result<(), Error> {
        Blake2b::digest_into(buffer, out).map_err(|_| Error::ExecutionError)
    }

    #[inline(never)]
    fn check_with_stored(curve: Curve, path: &BIP32Path<BIP32_MAX_LENGTH>) -> Result<bool, Error> {
        //Check if the current baking path in NVM is initialized
        let current_path =
            unsafe { BAKINGPATH.read() }.map_err(|_| Error::ApduCodeConditionsNotSatisfied)?;
        //path seems to be initialized so we can return it
        //check if it is a good path
        let nvm_bip = Bip32PathAndCurve::try_from_bytes(current_path)?
            .ok_or(Error::ApduCodeConditionsNotSatisfied)?;

        Ok(nvm_bip.path == *path && nvm_bip.curve == curve)
    }

    /// Will store a curve and path in NVM memory
    pub fn store_baking_key(curve: Curve, path: BIP32Path<BIP32_MAX_LENGTH>) -> Result<(), Error> {
        let path_and_curve = Bip32PathAndCurve::new(curve, path);

        unsafe { BAKINGPATH.write(path_and_curve.into()) }.map_err(|_| Error::ExecutionError)
    }

    /// Will remove the stored baking key
    pub fn remove_baking_key() -> Result<(), Error> {
        unsafe { BAKINGPATH.write(Bip32PathAndCurve::empty()) }.map_err(|_| Error::ExecutionError)
    }

    /// Will attempt to read a curve and path stored in NVM memory
    pub fn read_baking_key() -> Result<Option<(Curve, BIP32Path<BIP32_MAX_LENGTH>)>, Error> {
        let current =
            unsafe { BAKINGPATH.read() }.map_err(|_| Error::ApduCodeConditionsNotSatisfied)?;

        let path_and_curve = Bip32PathAndCurve::try_from_bytes(current)?;

        Ok(path_and_curve.map(|both| (both.curve, both.path)))
    }

    #[inline(never)]
    fn sign(digest: &[u8; 32]) -> Result<(usize, [u8; 100]), Error> {
        let current_path = unsafe { BAKINGPATH.read() }.map_err(|_| Error::ExecutionError)?;

        //path seems to be initialized so we can return it
        //check if it is a good path
        let bip32_nvm = match Bip32PathAndCurve::try_from_bytes(current_path) {
            Ok(Some(bip)) => bip,
            Ok(None) => return Err(Error::ApduCodeConditionsNotSatisfied as _),
            Err(e) => return Err(e),
        };

        let secret = bip32_nvm.curve.to_secret(&bip32_nvm.path);

        let mut sig = [0; 100];
        secret
            .sign(digest, &mut sig[..])
            .map_err(|_| Error::ExecutionError)
            .map(|sz| (sz, sig))
    }

    #[inline(never)]
    fn handle_endorsement(
        input: &'static [u8],
        send_hash: bool,
        digest: [u8; 32],
        out: &mut [u8],
    ) -> Result<usize, Error> {
        let hw = HWM::read().map_err(|_| Error::ExecutionError)?;

        let (_, endorsement) =
            EndorsementData::from_bytes(input).map_err(|_| Error::DataInvalid)?;
        if !endorsement.validate_with_watermark(&hw) {
            return Err(Error::DataInvalid);
        }

        HWM::write(endorsement.derive_watermark()).map_err(|_| Error::ExecutionError)?;

        let (sz, sig) = Self::sign(&digest)?;

        let mut tx = 0;

        if send_hash {
            //write unsigned_hash to buffer
            out[tx..tx + 32].copy_from_slice(&digest[..]);
            tx += 32;
        }

        //wrte signature to buffer
        out[tx..tx + sz].copy_from_slice(&sig[..sz]);
        tx += sz;

        Ok(tx)
    }

    #[inline(never)]
    fn handle_blockdata(
        input: &'static [u8],
        send_hash: bool,
        digest: [u8; 32],
        out: &mut [u8],
    ) -> Result<usize, Error> {
        let hw = HWM::read().map_err(|_| Error::ExecutionError)?;

        let (_, blockdata) = BlockData::from_bytes(input).map_err(|_| Error::DataInvalid)?;

        if !blockdata.validate_with_watermark(&hw) {
            return Err(Error::DataInvalid);
        }

        HWM::write(blockdata.derive_watermark()).map_err(|_| Error::ExecutionError)?;

        let (sz, sig) = Self::sign(&digest)?;

        let mut tx = 0;

        if send_hash {
            //write unsigned_hash to buffer
            out[tx..tx + 32].copy_from_slice(&digest[..]);
            tx += 32;
        }

        //wrte signature to buffer
        out[tx..tx + sz].copy_from_slice(&sig[..sz]);
        tx += sz;

        Ok(tx)
    }

    #[inline(never)]
    fn handle_delegation(
        input: &'static [u8],
        send_hash: bool,
        digest: [u8; 32],
        flags: &mut u32,
    ) -> Result<u32, Error> {
        crate::sys::zemu_log_stack("Baking::handle_delegation\x00");
        use crate::parser::operations::{Operation, OperationType};

        let mut op = core::mem::MaybeUninit::uninit();
        let mut operation = Operation::new(input).map_err(|_| Error::DataInvalid)?;
        operation
            .mut_ops()
            .parse_next_into(&mut op)
            .map_err(|_| Error::DataInvalid)?
            .ok_or(Error::DataInvalid)?;

        let (data, branch) = match unsafe { op.assume_init() } {
            OperationType::Delegation(deleg) => {
                //verify that delegation.source == delegation.delegate
                // and it matches the authorized key for baking
                // (BAKINGPATH)
                Ok((BakingTransactionType::Delegation(deleg), operation.branch()))
            }
            OperationType::Reveal(reveal) => {
                //what checks do we need here?
                Ok((BakingTransactionType::Reveal(reveal), operation.branch()))
            }
            _ => Err(Error::CommandNotAllowed),
        }?;

        let ui = BakingSignUI {
            send_hash,
            digest,
            branch,
            data,
        };

        unsafe { ui.show(flags).map(|_| 0).map_err(|_| Error::ExecutionError) }
    }

    #[inline(never)]
    pub fn baker_sign(
        send_hash: bool,
        p2: u8,
        init_data: &[u8],
        cdata: &'static [u8],
        out: &mut [u8],
        flags: &mut u32,
    ) -> Result<u32, Error> {
        crate::sys::zemu_log_stack("Baking::baker_sign\x00");

        let curve = Curve::try_from(p2).map_err(|_| Error::InvalidP1P2)?;
        let path =
            BIP32Path::<BIP32_MAX_LENGTH>::read(init_data).map_err(|_| Error::DataInvalid)?;

        if !Self::check_with_stored(curve, &path)? {
            return Err(Error::DataInvalid);
        }

        let mut digest = [0; Sign::SIGN_HASH_SIZE];
        Self::blake2b_digest_into(cdata, &mut digest)?;

        let (rem, preemble) = Preemble::from_bytes(cdata).map_err(|_| Error::DataInvalid)?;

        //endorses and bakes are automatically signed without any review
        match preemble {
            Preemble::TenderbakePreendorsement
            | Preemble::TenderbakeEndorsement
            | Preemble::Endorsement => {
                Self::handle_endorsement(rem, send_hash, digest, out).map(|n| n as u32)
            }
            Preemble::TenderbakeBlock | Preemble::Block => {
                Self::handle_blockdata(rem, send_hash, digest, out).map(|n| n as u32)
            }
            Preemble::Operation => Self::handle_delegation(rem, send_hash, digest, flags),
            _ => Err(Error::CommandNotAllowed),
        }
    }
}

enum BakingTransactionType<'b> {
    Delegation(Delegation<'b>),
    Reveal(Reveal<'b>),
}

struct BakingSignUI {
    send_hash: bool,
    digest: [u8; 32],
    branch: &'static [u8; 32],
    data: BakingTransactionType<'static>,
}

impl Viewable for BakingSignUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        let n = match self.data {
            BakingTransactionType::Delegation(data) => data.num_items(),
            BakingTransactionType::Reveal(data) => data.num_items(),
        } + 1;

        Ok(n as u8)
    }

    #[inline(never)]
    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        crate::sys::zemu_log_stack("Baking::render_item\x00");
        if let 0 = item_n {
            use crate::parser::operations::Operation;
            use bolos::pic_str;

            let title_content = pic_str!(b"Operation");
            title[..title_content.len()].copy_from_slice(title_content);

            let mut mex = [0; Operation::BASE58_BRANCH_LEN];
            let len = Operation::base58_branch_into(self.branch, &mut mex)
                .map_err(|_| ViewError::Unknown)?;

            crate::handlers::handle_ui_message(&mex[..len], message, page)
        } else {
            match self.data {
                BakingTransactionType::Delegation(data) => {
                    data.render_item(item_n - 1, title, message, page)
                }
                BakingTransactionType::Reveal(data) => {
                    data.render_item(item_n - 1, title, message, page)
                }
            }
        }
    }

    #[inline(never)]
    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let (sz, sig) = match Baking::sign(&self.digest) {
            Ok(ok) => ok,
            Err(e) => return (0, e as _),
        };

        let mut tx = 0;

        if self.send_hash {
            //write unsigned_hash to buffer
            out[tx..tx + 32].copy_from_slice(&self.digest[..]);
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

mod hmac;
pub use hmac::HMAC;

impl ApduHandler for Baking {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        crate::sys::zemu_log_stack("Baking::handle\x00");

        if let Some(upload) = Uploader::new(Self).upload(&buffer)? {
            *tx = Self::baker_sign(
                true,
                upload.p2,
                upload.first,
                upload.data,
                buffer.write(),
                flags,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{crypto, utils::MaybeNullTerminatedToString};
    use bolos::crypto::bip32::BIP32Path;

    use arrayref::array_ref;
    use zuit::{MockDriver, Page};

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
    fn test_emmy_endorsement_data() {
        let mut v = std::vec::Vec::with_capacity(1 + 4 + 32 + 1 + 4);
        v.push(0x00); //invalid preemble
        v.extend_from_slice(&1_u32.to_be_bytes());
        v.extend_from_slice(&[0u8; 32]);
        v.push(0x00); //emmy endorsement (without slot)
        v.extend_from_slice(&15_u32.to_be_bytes());

        let (_, endorsement) = EndorsementData::from_bytes(&v[1..]).unwrap();
        assert!(!endorsement.is_tenderbake());
        assert_eq!(endorsement.chain_id(), 1);
        assert_eq!(endorsement.branch(), &[0u8; 32]);
        assert_eq!(endorsement.level(), 15);
        assert_eq!(endorsement.endorsement_type(), b"Endorsement\x00");
    }

    #[test]
    fn test_tenderbake_preendorsement_data() {
        let mut v = std::vec::Vec::with_capacity(1 + 4 + 32 + 1 + 2 + 4 + 4 + 32);
        v.push(0x00); //invalid preemble
        v.extend_from_slice(&1_u32.to_be_bytes());
        v.extend_from_slice(&[0u8; 32]);
        v.push(20); //tenderbake preendorsement
        v.extend_from_slice(&0_u16.to_be_bytes()); //slot
        v.extend_from_slice(&15_u32.to_be_bytes()); //level
        v.extend_from_slice(&42_u32.to_be_bytes()); //round
        v.extend_from_slice(&[0u8; 32]); //block payload hash

        let (_, endorsement) = EndorsementData::from_bytes(&v[1..]).unwrap();
        assert!(endorsement.is_tenderbake());
        assert_eq!(endorsement.chain_id(), 1);
        assert_eq!(endorsement.branch(), &[0u8; 32]);
        assert_eq!(endorsement.level(), 15);
        assert_eq!(endorsement.round(), Some(42));
        assert_eq!(endorsement.endorsement_type(), b"Preendorsement\x00");
    }

    #[test]
    fn test_tenderbake_endorsement_data() {
        let mut v = std::vec::Vec::with_capacity(1 + 4 + 32 + 1 + 2 + 4 + 4 + 32);
        v.push(0x00); //invalid preemble
        v.extend_from_slice(&1_u32.to_be_bytes());
        v.extend_from_slice(&[0u8; 32]);
        v.push(21); //tenderbake endorsement
        v.extend_from_slice(&0_u16.to_be_bytes()); //slot
        v.extend_from_slice(&15_u32.to_be_bytes()); //level
        v.extend_from_slice(&42_u32.to_be_bytes()); //round
        v.extend_from_slice(&[0u8; 32]); //block payload hash

        let (_, endorsement) = EndorsementData::from_bytes(&v[1..]).unwrap();
        assert!(endorsement.is_tenderbake());
        assert_eq!(endorsement.chain_id(), 1);
        assert_eq!(endorsement.branch(), &[0u8; 32]);
        assert_eq!(endorsement.level(), 15);
        assert_eq!(endorsement.round(), Some(42));
        assert_eq!(endorsement.endorsement_type(), b"Endorsement\x00");
    }

    #[test]
    fn known_delegation() {
        const PARTIAL_INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 904e\
                                 01\
                                 0a\
                                 0a\
                                 ff\
                                 00";

        const KNOWN_BAKER_ADDR: &str = "tz1RV1MBbZMR68tacosb7Mwj6LkbPSUS1er1";
        const KNOWN_BAKER_NAME: &str = "Baking Tacos";

        let addr = bs58::decode(KNOWN_BAKER_ADDR)
            .into_vec()
            .apdu_expect("unable to decode known baker addr base58");
        let hash = array_ref!(&addr[3..], 0, 20);

        let mut input = hex::decode(PARTIAL_INPUT_HEX).expect("invalid input hex");
        input.extend_from_slice(hash); //add the known baker hash data to the input
        let input = &*input.leak();

        let (_, delegation) = Delegation::from_bytes(input).expect("couldn't parse delegation");

        let ui = BakingSignUI {
            send_hash: false,
            digest: [0; 32],
            branch: &[0; 32],
            data: BakingTransactionType::Delegation(delegation),
        };
        let mut driver = MockDriver::<_, 18, 4096>::new(ui);
        driver.drive();

        let produced_ui = driver.out_ui();
        let delegation_item = produced_ui
            .iter()
            .find(|item_pages| {
                item_pages
                    .iter()
                    .all(|Page { title, .. }| title.starts_with("Delegation".as_bytes()))
            })
            .expect("Couldn't find delegation item in UI");

        let title = delegation_item[0]
            .message
            .to_string_with_check_null()
            .expect("message was invalid UTF8");

        //verify that the message is the same as the name we expect in the test
        assert_eq!(title, KNOWN_BAKER_NAME);
    }
}
