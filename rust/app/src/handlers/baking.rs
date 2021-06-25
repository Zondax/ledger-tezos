use std::convert::TryFrom;

use nom::{
    bytes::complete::take,
    number::complete::{be_u32, le_u32, le_u64, le_u8},
    IResult,
};

use zemu_sys::{Show, ViewError, Viewable};

use bolos::{
    crypto::bip32::BIP32Path,
    hash::{Blake2b, Hasher},
};

use crate::constants::{ApduError, APDU_INDEX_P1, APDU_INDEX_P2};
use crate::handlers::parser_common::ParserError;
use crate::handlers::PacketType;
use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS, BIP32_MAX_LENGTH},
    crypto::{self, Curve},
    dispatcher::{
        ApduHandler, INS_AUTHORIZE_BAKING, INS_BAKER_SIGN, INS_DEAUTHORIZE_BAKING,
        INS_LEGACY_AUTHORIZE_BAKING, INS_LEGACY_DEAUTHORIZE, INS_LEGACY_QUERY_AUTH_KEY,
        INS_LEGACY_QUERY_AUTH_KEY_WITH_CURVE, INS_QUERY_AUTH_KEY, INS_QUERY_AUTH_KEY_WITH_CURVE,
    },
    handlers::hwm::{LegacyHWM, WaterMark},
    handlers::public_key::GetAddress,
    sys::{self, new_wear_leveller, wear_leveller::Wear},
};
use bolos::wear_leveller::WearError;

const N_PAGES_BAKINGPATH: usize = 1;

type WearLeveller = Wear<'static, N_PAGES_BAKINGPATH>;

#[derive(Debug, Clone, Copy)]
enum Action {
    AuthorizeBaking,
    LegacyAuthorize,
    DeAuthorizeBaking,
    LegacyDeAuthorize,
    QueryAuthKey,
    LegacyQueryAuthKey,
    QueryAuthKeyWithCurve,
    LegacyQueryAuthKeyWithCurve,
    BakerSign,
}

//TODO: move this to other file??
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Preemble {
    InvalidPreemble = 0x00,
    BlockPreemble = 0x01,
    EndorsementPreemble = 0x02,
    GenericPreemble = 0x03,
}

impl Preemble {
    #[inline(never)]
    pub fn from_bytes(bytes: &[u8]) -> nom::IResult<&[u8], Self, ParserError> {
        let version_res = le_u8(bytes)?;
        let tx_version =
            Self::from_u8(version_res.1).ok_or(ParserError::parser_unexpected_error)?;
        Ok((version_res.0, tx_version))
    }

    #[inline(never)]
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x00 => Some(Self::InvalidPreemble),
            0x01 => Some(Self::BlockPreemble),
            0x02 => Some(Self::EndorsementPreemble),
            0x03 => Some(Self::GenericPreemble),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Branch(pub [u8; 32]);

//return !(lvl & 0xC0000000);
#[inline(never)]
pub fn is_valid_blocklevel(level: u32) -> bool {
    level.leading_zeros() > 0
}

impl Branch {
    #[inline(never)]
    pub fn from_bytes(bytes: &[u8]) -> nom::IResult<&[u8], Self, ParserError> {
        let (raw, branchbytes) = take(32usize)(bytes)?;
        let mut branch = [0u8; 32];
        branch.copy_from_slice(branchbytes);
        Ok((raw, Self(branch)))
    }
}

pub struct EndorsementData {
    pub baker_preemble: Preemble,
    pub chain_id: u32,
    pub branch: Branch,
    pub tag: u8,
    pub level: u32,
}

impl EndorsementData {
    #[inline(never)]
    pub fn from_bytes(bytes: &[u8]) -> nom::IResult<&[u8], Self, ParserError> {
        let (rem, preemble) = le_u8(bytes)?;
        let baker_preemble =
            Preemble::from_u8(preemble).ok_or(ParserError::parser_unexpected_error)?;
        let (rem, chain_id) = be_u32(rem)?;
        let (rem, branch) = Branch::from_bytes(rem)?;
        let (rem, tag) = le_u8(rem)?;
        let (rem, level) = be_u32(rem)?;
        let result = EndorsementData {
            baker_preemble,
            chain_id,
            branch,
            tag,
            level,
        };
        Ok((rem, result))
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        is_valid_blocklevel(self.level)
            && (self.level > hw.level || (hw.level == self.level && !hw.endorsement))
    }
}

pub struct BlockData {
    pub baker_preemble: Preemble,
    pub chain_id: u32,
    pub level: u32,
    pub proto: u8,
}

impl BlockData {
    #[inline(never)]
    pub fn from_bytes(bytes: &[u8]) -> nom::IResult<&[u8], Self, ParserError> {
        let (rem, preemble) = le_u8(bytes)?;
        let baker_preemble =
            Preemble::from_u8(preemble).ok_or(ParserError::parser_unexpected_error)?;
        let (rem, chain_id) = be_u32(rem)?;
        let (rem, level) = be_u32(rem)?;
        let (rem, proto) = le_u8(rem)?;
        let result = BlockData {
            baker_preemble,
            chain_id,
            level,
            proto,
        };
        Ok((rem, result))
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        is_valid_blocklevel(self.level) && (self.level > hw.level)
    }
}

#[bolos::lazy_static]
static mut BAKINGPATH: WearLeveller =
    new_wear_leveller!(N_PAGES_BAKINGPATH).expect("NVM might be corrupted");

pub const BIP32_MAX_BYTES_LENGTH: usize = 1 + 4 * BIP32_MAX_LENGTH;

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
        if (components_length > BIP32_MAX_LENGTH as u8) {
            return Err(Error::WrongLength);
        }
        let path = sys::crypto::bip32::BIP32Path::<BIP32_MAX_LENGTH>::read(
            &from[1..(2 + components_length * 4).into()],
        )
        .map_err(|_| Error::DataInvalid)?;
        Ok(Self { curve, path })
    }
}

impl Into<[u8; 52]> for Bip32PathAndCurve {
    fn into(self) -> [u8; 52] {
        let mut out = [0; 52];

        let curve = self.curve.into();
        out[0] = curve;
        let components = self.path.components();
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

    //FIXME: ideally grab this function from signing.rs?
    #[inline(never)]
    fn get_derivation_info() -> Result<&'static (BIP32Path<BIP32_MAX_LENGTH>, Curve), Error> {
        match unsafe { &*PATH } {
            None => Err(Error::ApduCodeConditionsNotSatisfied),
            Some(some) => Ok(some),
        }
    }

    //FIXME: make this part of impl Bip32PathAndCurve?
    #[inline(never)]
    fn check_and_store_path(path_and_curve: Bip32PathAndCurve) -> Result<(), Error> {
        //Check if the current baking path in NVM is un-initialized
        let current_path = unsafe { BAKINGPATH.read() };
        if let Err(error_msg) = current_path {
            if (error_msg != WearError::Uninitialized) {
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
        if let Err(_) = current_path {
            //There was no initial path
            Err(Error::ApduCodeConditionsNotSatisfied)
        } else {
            //path seems to be initialized so we can remove it
            unsafe { BAKINGPATH.format() }.map_err(|_| Error::ExecutionError)?;
            Ok(())
        }
    }

    #[inline(never)]
    fn deauthorize_baking(req_confirmation: bool) -> Result<u32, Error> {
        if (!req_confirmation) {
            return Err(Error::ApduCodeConditionsNotSatisfied);
        }
        //TODO: show confirmation of deletion on screen
        //FIXME: check if we need to format the HWM??
        LegacyHWM::format()?;
        Self::check_and_delete_path()?;
        Ok((0))
    }

    #[inline(never)]
    fn authorize_baking(req_confirmation: bool, buffer: &mut [u8]) -> Result<u32, Error> {
        if (!req_confirmation) {
            return Err(Error::ApduCodeConditionsNotSatisfied);
        }

        let curve =
            crypto::Curve::try_from(buffer[APDU_INDEX_P2]).map_err(|_| Error::InvalidP1P2)?;

        let cdata_len = buffer[4] as usize;
        if cdata_len > buffer[5..].len() {
            return Err(Error::DataInvalid);
        }
        let cdata = &buffer[5..5 + cdata_len];
        let bip32_path = sys::crypto::bip32::BIP32Path::<BIP32_MAX_LENGTH>::read(cdata)
            .map_err(|_| Error::DataInvalid)?;
        let key = GetAddress::new_key(curve, &bip32_path).map_err(|_| Error::ExecutionError)?;
        let path_and_data = Bip32PathAndCurve::new(curve, bip32_path);
        Self::check_and_store_path(path_and_data)?;
        let pkLen = Self::get_public(key, &mut buffer[1..])?;
        buffer[0] = pkLen as u8;

        LegacyHWM::reset(0).map_err(|_| Error::Busy)?;

        Ok(pkLen + 1)
    }

    #[inline(never)]
    fn query_authkey(req_confirmation: bool, buffer: &mut [u8]) -> Result<u32, Error> {
        if (req_confirmation) {
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
        buffer[0..1 + 4 * bip32_pathsize].copy_from_slice(&current_path[1..2 + 4 * bip32_pathsize]);
        Ok((1 + 4 * bip32_pathsize as u32))
    }

    #[inline(never)]
    fn query_authkey_withcurve(req_confirmation: bool, buffer: &mut [u8]) -> Result<u32, Error> {
        if (req_confirmation) {
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
        buffer[0..2 + 4 * bip32_pathsize].copy_from_slice(&current_path[0..2 + 4 * bip32_pathsize]);
        Ok((2 + 4 * bip32_pathsize as u32))
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

        if (nvm_bip.path != *path || nvm_bip.curve != *curve) {
            //TODO: show that bip32 paths don't match??
            return Err(Error::DataInvalid);
        } else {
            Ok(())
        }
    }

    #[inline(never)]
    fn baker_sign(buffer: &mut [u8], flags: &mut u32) -> Result<u32, Error> {
        let cdata_len = buffer[4] as usize;
        let cdata = &buffer[5..5 + cdata_len];
        let packet_type = PacketType::try_from(buffer[2]).map_err(|_| Error::InvalidP1P2)?;
        let bakingUI: BakingSignUI;
        match packet_type {
            PacketType::Init => {
                //first packet contains the curve data on the second parameter
                // and the bip32 path as payload only

                let curve = Curve::try_from(buffer[3]).map_err(|_| Error::InvalidP1P2)?;
                let path =
                    BIP32Path::<BIP32_MAX_LENGTH>::read(cdata).map_err(|_| Error::DataInvalid)?;

                //Check if the current baking path in NVM is initialized
                Self::check_with_nvm_pathandcurve(&curve, &path)?;

                unsafe { PATH.replace((path, curve)) };

                Ok(0)
            }
            PacketType::Add => {
                return Err(Error::InsNotSupported);
                //this should not happen as there is only 1 packet??
            }
            PacketType::Last => {
                let (path, curve) = Self::get_derivation_info()?;

                let hw = LegacyHWM::read()?;
                //do watermarks checks

                let preemble = Preemble::from_u8(cdata[0]).ok_or(Error::DataInvalid)?;

                match preemble {
                    Preemble::InvalidPreemble => {
                        return Err(Error::DataInvalid);
                    }

                    Preemble::EndorsementPreemble => {
                        let (_, endorsement) =
                            EndorsementData::from_bytes(&cdata).map_err(|_| Error::DataInvalid)?;
                        if (!endorsement.validate_with_watermark(&hw)) {
                            return Err(Error::DataInvalid);
                            //TODO: show endorsement data on screen
                        }
                        bakingUI = BakingSignUI {
                            endorsement: Some(endorsement),
                            blocklevel: None,
                        };
                    }
                    Preemble::BlockPreemble => {
                        let (_, blockdata) =
                            BlockData::from_bytes(&cdata).map_err(|_| Error::DataInvalid)?;

                        if (!blockdata.validate_with_watermark(&hw)) {
                            return Err(Error::DataInvalid);
                        }
                        //TODO: show blocklevel on screen
                        bakingUI = BakingSignUI {
                            endorsement: None,
                            blocklevel: Some(blockdata),
                        };
                    }

                    Preemble::GenericPreemble => {
                        /*
                                        case MAGIC_BYTE_UNSAFE_OP: {
                            if (!G.maybe_ops.is_valid) PARSE_ERROR();

                            // Must be self-delegation signed by the *authorized* baking key
                            if (bip32_path_with_curve_eq(&global.path_with_curve, &N_data.baking_key) &&

                                // ops->signing is generated from G.bip32_path and G.curve
                                COMPARE(&G.maybe_ops.v.operation.source, &G.maybe_ops.v.signing) == 0 &&
                                COMPARE(&G.maybe_ops.v.operation.destination, &G.maybe_ops.v.signing) == 0) {
                                ui_callback_t const ok_c = send_hash ? sign_with_hash_ok : sign_without_hash_ok;
                                prompt_register_delegate(ok_c, sign_reject);
                            }
                            THROW(EXC_SECURITY);
                            break;
                        }
                         */
                        //not implemnted yet
                        return Err(Error::DataInvalid);
                    }
                }

                return unsafe { bakingUI.show(flags) }
                    .map_err(|_| Error::ExecutionError)
                    .map(|_| 0);
            }
        }
    }
}

struct BakingSignUI {
    pub endorsement: Option<EndorsementData>,
    pub blocklevel: Option<BlockData>,
}

impl Viewable for BakingSignUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        if self.endorsement.is_some() {
            Ok(1)
        } else if self.blocklevel.is_some() {
            Ok(1)
        } else {
            Err(ViewError::NoData)
        }
    }

    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        if let 0 = item_n {
            if let Some(endorsement) = &self.endorsement {
                let title_content = bolos::PIC::new(b"Baking End\x00").into_inner();

                title[..title_content.len()].copy_from_slice(title_content);

                let m_len = message.len() - 1; //null byte terminator
                if m_len <= 64 {
                    let chunk = endorsement
                        .branch
                        .0
                        .chunks(m_len / 2) //divide in non-overlapping chunks
                        .nth(page as usize) //get the nth chunk
                        .ok_or(ViewError::Unknown)?;

                    hex::encode_to_slice(chunk, &mut message[..chunk.len() * 2])
                        .map_err(|_| ViewError::Unknown)?;
                    message[chunk.len() * 2] = 0; //null terminate

                    let n_pages = (32 * 2) / m_len;
                    Ok(1 + n_pages as u8)
                } else {
                    hex::encode_to_slice(&endorsement.branch.0[..], &mut message[..64])
                        .map_err(|_| ViewError::Unknown)?;
                    message[64] = 0; //null terminate
                    Ok(1)
                }
            } else if let Some(_) = &self.blocklevel {
                let title_content = bolos::PIC::new(b"Baking Block\x00").into_inner();

                title[..title_content.len()].copy_from_slice(title_content);

                hex::encode_to_slice(&[0u8; 6], &mut message[..12]);
                message[12] = 0;
                Ok(1)
            } else {
                Err(ViewError::NoData)
            }
        } else {
            Err(ViewError::NoData)
        }
    }

    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let mut blocklevel: u32 = 0;
        let mut is_endorsement: bool = false;
        if let Some(endorsement) = &self.endorsement {
            blocklevel = endorsement.level;
            is_endorsement = true;
        } else if let Some(block) = &self.blocklevel {
            blocklevel = block.level;
            is_endorsement = false;
        } else {
            return (0, Error::DataInvalid as u16);
        }
        let new_hw = WaterMark {
            level: blocklevel,
            endorsement: is_endorsement,
        };
        match LegacyHWM::write(new_hw) {
            Err(_) => return (0, Error::ExecutionError as _),
            Ok(()) => (),
        }

        //TODO: we need a macro for this
        let current_path = match unsafe { BAKINGPATH.read() } {
            Err(_) => return (0, Error::ExecutionError as _),
            Ok(k) => k,
        };
        //path seems to be initialized so we can return it
        //check if it is a good path
        //TODO: otherwise return an error and show that on screen (corrupted NVM??)
        let bip32_nvm = match Bip32PathAndCurve::try_from_bytes(&current_path) {
            Err(e) => return (0, e as _),
            Ok(k) => k,
        };

        let mut keypair = match bip32_nvm.curve.gen_keypair(&bip32_nvm.path) {
            Err(_) => return (0, Error::ExecutionError as _),
            Ok(k) => k,
        };

        let mut sig = [0; 100];

        let unsigned_hash = [0u8; 32];

        let sz = keypair
            .sign(&unsigned_hash, &mut sig[..])
            .unwrap_or_else(|e| 0);
        if sz == 0 {
            return (0, Error::ExecutionError as _);
        }

        let mut tx = 0;

        //reset globals to avoid skipping `Init`
        if let Err(e) = cleanup_globals() {
            return (0, e as _);
        }

        //write unsigned_hash to buffer
        tx += 32;
        out[0..32].copy_from_slice(&unsigned_hash);

        //wrte signature to buffer
        tx += sz;
        out[32..32 + sz].copy_from_slice(&sig[..sz]);

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

impl ApduHandler for Baking {
    #[inline(never)]
    fn handle(flags: &mut u32, tx: &mut u32, _rx: u32, buffer: &mut [u8]) -> Result<(), Error> {
        sys::zemu_log_stack("Baking::handle\x00");

        *tx = 0;
        let action = match buffer[APDU_INDEX_INS] {
            INS_AUTHORIZE_BAKING => Action::AuthorizeBaking,
            INS_LEGACY_AUTHORIZE_BAKING => Action::LegacyAuthorize,
            INS_LEGACY_DEAUTHORIZE => Action::LegacyDeAuthorize,
            INS_DEAUTHORIZE_BAKING => Action::DeAuthorizeBaking,
            INS_QUERY_AUTH_KEY => Action::QueryAuthKey,
            INS_LEGACY_QUERY_AUTH_KEY => Action::LegacyQueryAuthKey,
            INS_QUERY_AUTH_KEY_WITH_CURVE => Action::QueryAuthKeyWithCurve,
            INS_LEGACY_QUERY_AUTH_KEY_WITH_CURVE => Action::LegacyQueryAuthKeyWithCurve,
            INS_BAKER_SIGN => Action::BakerSign,
            _ => return Err(Error::InsNotSupported),
        };

        let req_confirmation = buffer[APDU_INDEX_P1] >= 1;

        *tx = match action {
            Action::AuthorizeBaking => Self::authorize_baking(req_confirmation, buffer)?,
            Action::LegacyAuthorize => 32,
            Action::DeAuthorizeBaking => Self::deauthorize_baking(req_confirmation)?,
            Action::LegacyDeAuthorize => 0,
            Action::QueryAuthKey => Self::query_authkey(req_confirmation, buffer)?,
            Action::LegacyQueryAuthKey => 0,
            Action::QueryAuthKeyWithCurve => {
                Self::query_authkey_withcurve(req_confirmation, buffer)?
            }
            Action::LegacyQueryAuthKeyWithCurve => 0,
            Action::BakerSign => Self::baker_sign(buffer, flags)?,
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto;
    use bolos::crypto::bip32::BIP32Path;

    use super::*;
    use crate::dispatcher::{handle_apdu, CLA, INS_AUTHORIZE_BAKING};
    use crate::{assert_error_code, crypto::Curve};

    use std::vec;

    #[test]
    fn check_bip32andpath_frombytes() {
        let curve = crypto::Curve::Ed25519;
        let pathdata = &[44, 1729, 0, 0];
        let path =
            BIP32Path::<BIP32_MAX_LENGTH>::new(pathdata.into_iter().map(|n| 0x8000_0000 + n))
                .unwrap();
        let path_and_curve = Bip32PathAndCurve::new(curve, path);
        let data: [u8; 52] = path_and_curve.clone().into();
        let derived = Bip32PathAndCurve::try_from_bytes(&data);
        assert!(derived.is_ok());
        assert_eq!(derived.unwrap(), path_and_curve);
    }

    #[test]
    fn test_endorsement_data() {
        let mut v = std::vec::Vec::with_capacity(1 + 4 + 32);
        v.push(0x00);
        v.extend_from_slice(&1_u32.to_be_bytes());
        v.extend_from_slice(&[0u8; 32]);
        v.push(0x05);
        v.extend_from_slice(&15_u32.to_be_bytes());
        assert_eq!(v.len(), 1 + 4 + 32 + 1 + 4);
        let (_, endorsement) = EndorsementData::from_bytes(&v).unwrap();
        assert_eq!(endorsement.baker_preemble, Preemble::InvalidPreemble);
        assert_eq!(endorsement.chain_id, 1);
        assert_eq!(endorsement.branch, Branch([0u8; 32]));
        assert_eq!(endorsement.tag, 5);
        assert_eq!(endorsement.level, 15);
    }
}
