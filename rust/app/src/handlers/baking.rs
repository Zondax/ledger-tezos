use std::convert::TryFrom;

use nom::{
    bytes::complete::take,
    number::complete::{be_u32, le_u8},
};

use zemu_sys::{Show, ViewError, Viewable};

use bolos::{
    crypto::bip32::BIP32Path,
    hash::{Blake2b, Hasher},
};

use crate::handlers::parser_common::ParserError;
use crate::handlers::{PacketType, PacketTypes};
use crate::{
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::{self, Curve},
    dispatcher::{
        ApduHandler, INS_AUTHORIZE_BAKING, INS_BAKER_SIGN, INS_DEAUTHORIZE_BAKING,
        INS_LEGACY_AUTHORIZE_BAKING, INS_LEGACY_DEAUTHORIZE, INS_LEGACY_QUERY_AUTH_KEY,
        INS_LEGACY_QUERY_AUTH_KEY_WITH_CURVE, INS_QUERY_AUTH_KEY, INS_QUERY_AUTH_KEY_WITH_CURVE,
    },
    handlers::hwm::{LegacyHWM, WaterMark},
    handlers::public_key::GetAddress,
    sys::{self, flash_slot::Wear, new_flash_slot},
    utils::ApduBufferRead,
};
use bolos::flash_slot::WearError;

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
    InvalidPreemble = 0x00, //FIXME: should we error when this is the Preemble?
    BlockPreemble = 0x01,
    EndorsementPreemble = 0x02,
    GenericPreemble = 0x03,
}

impl From<Preemble> for u8 {
    fn from(from: Preemble) -> Self {
        from as u8
    }
}

impl TryFrom<u8> for Preemble {
    type Error = Error;
    fn try_from(from: u8) -> Result<Self, Error> {
        match from {
            0x00 => Ok(Self::InvalidPreemble),
            0x01 => Ok(Self::BlockPreemble),
            0x02 => Ok(Self::EndorsementPreemble),
            0x03 => Ok(Self::GenericPreemble),
            _ => Err(Error::DataInvalid),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Branch(pub [u8; 32]);

impl Branch {
    pub const HEX_LEN: usize = 32 * 2;

    #[inline(never)]
    pub fn from_bytes(bytes: &[u8]) -> nom::IResult<&[u8], Self, ParserError> {
        let (raw, branchbytes) = take(32usize)(bytes)?;
        let mut branch = [0u8; 32];
        branch.copy_from_slice(branchbytes);
        Ok((raw, Self(branch)))
    }
}

pub const ENDORSEMENT_DATA_LENGTH: usize = 42;

pub struct EndorsementData {
    pub baker_preemble: Preemble,
    pub chain_id: u32,
    pub branch: Branch,
    pub tag: u8, //TODO: what to do with this??
    pub level: u32,
}

impl EndorsementData {
    #[inline(never)]
    pub fn from_bytes(bytes: &[u8]) -> nom::IResult<&[u8], Self, ParserError> {
        let (rem, preemble) = le_u8(bytes)?;
        let baker_preemble: Preemble =
            Preemble::try_from(preemble).map_err(|_| ParserError::parser_unexpected_error)?;
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
    pub fn to_bytes(&self) -> [u8; ENDORSEMENT_DATA_LENGTH] {
        let mut result = [0u8; ENDORSEMENT_DATA_LENGTH];
        result[0] = self.baker_preemble.into();
        result[1..5].copy_from_slice(&self.chain_id.to_be_bytes());
        result[5..37].copy_from_slice(&self.branch.0);
        result[37..41].copy_from_slice(&self.level.to_be_bytes());
        result[41] = self.tag;
        result
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        WaterMark::is_valid_blocklevel(self.level)
            && (self.level > hw.level || (hw.level == self.level && !hw.endorsement))
    }
}

pub const BLOCKDATA_LENGTH: usize = 10;

pub struct BlockData {
    pub baker_preemble: Preemble,
    pub chain_id: u32,
    pub level: u32,
    pub proto: u8, //FIXME: what to do with this byte?
}

impl BlockData {
    #[inline(never)]
    pub fn from_bytes(bytes: &[u8]) -> nom::IResult<&[u8], Self, ParserError> {
        let (rem, preemble) = le_u8(bytes)?;
        let baker_preemble =
            Preemble::try_from(preemble).map_err(|_| ParserError::parser_context_invalid_chars)?;
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
    pub fn to_bytes(&self) -> [u8; BLOCKDATA_LENGTH] {
        let mut result = [0u8; BLOCKDATA_LENGTH];
        result[0] = self.baker_preemble.into();
        result[1..5].copy_from_slice(&self.chain_id.to_be_bytes());
        result[5..9].copy_from_slice(&self.level.to_be_bytes());
        result[9] = self.proto;
        result
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        WaterMark::is_valid_blocklevel(self.level) && (self.level > hw.level)
    }
}

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

    #[inline(never)]
    fn deauthorize_baking(req_confirmation: bool) -> Result<u32, Error> {
        if !req_confirmation {
            return Err(Error::ApduCodeConditionsNotSatisfied);
        }
        //TODO: show confirmation of deletion on screen
        //FIXME: check if we need to format the HWM??
        LegacyHWM::format()?;
        Self::check_and_delete_path()?;
        Ok(0)
    }

    #[inline(never)]
    fn authorize_baking(req_confirmation: bool, buffer: ApduBufferRead<'_>) -> Result<u32, Error> {
        if !req_confirmation {
            return Err(Error::ApduCodeConditionsNotSatisfied);
        }

        let curve = crypto::Curve::try_from(buffer.p2()).map_err(|_| Error::InvalidP1P2)?;

        let cdata = buffer.payload().map_err(|_| Error::DataInvalid)?;
        let bip32_path = sys::crypto::bip32::BIP32Path::<BIP32_MAX_LENGTH>::read(cdata)
            .map_err(|_| Error::DataInvalid)?;

        let key = GetAddress::new_key(curve, &bip32_path).map_err(|_| Error::ExecutionError)?;
        let path_and_data = Bip32PathAndCurve::new(curve, bip32_path);
        Self::check_and_store_path(path_and_data)?;

        let buffer = buffer.write();
        let pk_len = Self::get_public(key, &mut buffer[1..])?;
        buffer[0] = pk_len as u8;

        LegacyHWM::reset(0).map_err(|_| Error::Busy)?;

        Ok(pk_len + 1)
    }

    #[inline(never)]
    fn query_authkey(req_confirmation: bool, buffer: &mut [u8]) -> Result<u32, Error> {
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

        buffer[0..1 + 4 * bip32_pathsize].copy_from_slice(&current_path[1..2 + 4 * bip32_pathsize]);
        Ok(1 + 4 * bip32_pathsize as u32)
    }

    #[inline(never)]
    fn query_authkey_withcurve(req_confirmation: bool, buffer: &mut [u8]) -> Result<u32, Error> {
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

        buffer[0..2 + 4 * bip32_pathsize].copy_from_slice(&current_path[0..2 + 4 * bip32_pathsize]);
        Ok(2 + 4 * bip32_pathsize as u32)
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
    fn baker_sign(
        packet_type: PacketTypes,
        buffer: ApduBufferRead<'_>,
        flags: &mut u32,
    ) -> Result<u32, Error> {
        let cdata = buffer.payload().map_err(|_| Error::DataInvalid)?;
        let baking_ui: BakingSignUI;

        if packet_type.is_init() {
            //first packet contains the curve data on the second parameter
            // and the bip32 path as payload only

            let curve = Curve::try_from(buffer.p2()).map_err(|_| Error::InvalidP1P2)?;
            let path =
                BIP32Path::<BIP32_MAX_LENGTH>::read(cdata).map_err(|_| Error::DataInvalid)?;

            //Check if the current baking path in NVM is initialized
            Self::check_with_nvm_pathandcurve(&curve, &path)?;

            unsafe { PATH.replace((path, curve)) };

            Ok(0)
        } else if packet_type.is_next() {
            Err(Error::InsNotSupported)
            //this should not happen as there is only 1 packet??
        } else if packet_type.is_last() {
            let (_path, _curve) = Self::get_derivation_info()?;

            let hw = LegacyHWM::read()?;
            //do watermarks checks

            let preemble = Preemble::try_from(cdata[0])?;

            match preemble {
                Preemble::InvalidPreemble => {
                    return Err(Error::DataInvalid);
                }

                Preemble::EndorsementPreemble => {
                    let (_, endorsement) =
                        EndorsementData::from_bytes(&cdata).map_err(|_| Error::DataInvalid)?;
                    if !endorsement.validate_with_watermark(&hw) {
                        return Err(Error::DataInvalid);
                        //TODO: show endorsement data on screen
                    }

                    let digest = Self::blake2b_digest(&endorsement.to_bytes())?;

                    baking_ui = BakingSignUI {
                        endorsement: Some(endorsement),
                        blocklevel: None,
                        digest,
                    };
                }
                Preemble::BlockPreemble => {
                    let (_, blockdata) =
                        BlockData::from_bytes(&cdata).map_err(|_| Error::DataInvalid)?;

                    if !blockdata.validate_with_watermark(&hw) {
                        return Err(Error::DataInvalid);
                    }

                    let digest = Self::blake2b_digest(&blockdata.to_bytes())?;
                    //TODO: show blocklevel on screen
                    baking_ui = BakingSignUI {
                        endorsement: None,
                        blocklevel: Some(blockdata),
                        digest,
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

            unsafe { baking_ui.show(flags) }
                .map_err(|_| Error::ExecutionError)
                .map(|_| 0)
        } else {
            Err(Error::DataInvalid)
        }
    }
}

struct BakingSignUI {
    pub endorsement: Option<EndorsementData>,
    pub blocklevel: Option<BlockData>,
    pub digest: [u8; 32],
}

fn write_u32_to_ui_buffer(num: u32, buffer: &mut [u8]) -> Result<usize, Error> {
    //TODO: use a zxlib function for this
    let mut num_size: usize = 0;
    if num == 0 {
        buffer[0] = 48;
        num_size = 1;
    } else {
        let mut digits = num;
        while digits > 0 {
            buffer[num_size] = (48 + digits % 10) as u8; //0x30 + digit
            digits /= 10;
            num_size += 1;
            if num_size >= buffer.len() {
                return Err(Error::OutputBufferTooSmall);
            }
        }
    }
    buffer.reverse();
    Ok(num_size)
}

//FIXME: split the below code for endorsements and blocklevel signing
impl Viewable for BakingSignUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        if self.endorsement.is_some() {
            Ok(4)
        } else if self.blocklevel.is_some() {
            Ok(3)
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
        if let Some(endorsement) = &self.endorsement {
            const BRANCH_HEX_LEN: usize = Branch::HEX_LEN;

            match item_n {
                0 => {
                    let title_content = bolos::PIC::new(b"Baking Sign\x00").into_inner();
                    title[..title_content.len()].copy_from_slice(title_content);

                    let message_content = bolos::PIC::new(b"Endorsement\x00").into_inner();
                    message[..message_content.len()].copy_from_slice(message_content);

                    Ok(1)
                }
                1 => {
                    let title_content = bolos::PIC::new(b"Branch\x00").into_inner();
                    title[..title_content.len()].copy_from_slice(title_content);

                    let m_len = message.len() - 1; //null byte terminator
                    if m_len <= BRANCH_HEX_LEN {
                        let chunk = endorsement
                            .branch
                            .0
                            .chunks(m_len / 2) //divide in non-overlapping chunks
                            .nth(page as usize) //get the nth chunk
                            .ok_or(ViewError::Unknown)?;

                        hex::encode_to_slice(chunk, &mut message[..chunk.len() * 2])
                            .map_err(|_| ViewError::Unknown)?;
                        message[chunk.len() * 2] = 0; //null terminate

                        let n_pages = BRANCH_HEX_LEN / m_len;

                        Ok(1 + n_pages as u8)
                    } else {
                        hex::encode_to_slice(
                            &endorsement.branch.0[..],
                            &mut message[..BRANCH_HEX_LEN],
                        )
                        .map_err(|_| ViewError::Unknown)?;
                        message[BRANCH_HEX_LEN] = 0; //null terminate

                        Ok(1)
                    }
                }
                2 => {
                    let title_content = bolos::PIC::new(b"Blocklevel\x00").into_inner();
                    title[..title_content.len()].copy_from_slice(title_content);

                    let mut buffer = [0u8; 100];
                    let num_digits = write_u32_to_ui_buffer(endorsement.level, &mut buffer)
                        .map_err(|_| ViewError::Unknown)?;
                    message[0..num_digits].copy_from_slice(&buffer[100 - num_digits..]);
                    message[num_digits] = 0;

                    Ok(1)
                }
                3 => {
                    let title_content = bolos::PIC::new(b"ChainID\x00").into_inner();
                    title[..title_content.len()].copy_from_slice(title_content);

                    let mut buffer = [0u8; 100];
                    let num_digits = write_u32_to_ui_buffer(endorsement.chain_id, &mut buffer)
                        .map_err(|_| ViewError::Unknown)?;
                    message[0..num_digits].copy_from_slice(&buffer[100 - num_digits..]);
                    message[num_digits] = 0;

                    Ok(1)
                }
                _ => Err(ViewError::NoData),
            }
        } else if let Some(block) = &self.blocklevel {
            match item_n {
                0 => {
                    let title_content = bolos::PIC::new(b"Baking Sign\x00").into_inner();
                    title[..title_content.len()].copy_from_slice(title_content);

                    let message_content = bolos::PIC::new(b"Blocklevel\x00").into_inner();
                    message[..message_content.len()].copy_from_slice(message_content);

                    Ok(1)
                }
                1 => {
                    let title_content = bolos::PIC::new(b"Chain_ID\x00").into_inner();
                    title[..title_content.len()].copy_from_slice(title_content);

                    let mut buffer = [0u8; 100];
                    let num_digits = write_u32_to_ui_buffer(block.chain_id, &mut buffer)
                        .map_err(|_| ViewError::Unknown)?;
                    message[0..num_digits].copy_from_slice(&buffer[100 - num_digits..]);
                    message[num_digits] = 0;

                    Ok(1)
                }
                2 => {
                    let title_content = bolos::PIC::new(b"Blocklevel\x00").into_inner();
                    title[..title_content.len()].copy_from_slice(title_content);

                    let mut buffer = [0u8; 100];
                    let num_digits = write_u32_to_ui_buffer(block.level, &mut buffer)
                        .map_err(|_| ViewError::Unknown)?;
                    message[0..num_digits].copy_from_slice(&buffer[100 - num_digits..]);
                    message[num_digits] = 0;

                    Ok(1)
                }
                _ => Err(ViewError::NoData),
            }
        } else {
            Err(ViewError::NoData)
        }
    }

    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let blocklevel: u32;
        let is_endorsement: bool;
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

        let sz = keypair.sign(&self.digest, &mut sig[..]).unwrap_or(0);
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
        out[..32].copy_from_slice(&self.digest);

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
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        sys::zemu_log_stack("Baking::handle\x00");

        *tx = 0;
        let action = match buffer.ins() {
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
        let is_legacy = false; //FIXME: take this from action type

        //read P1, one exludes the other
        let req_confirmation = buffer.p1() >= 1;
        let packet_type =
            PacketTypes::new(buffer.p1(), is_legacy).map_err(|_| Error::InvalidP1P2)?;

        *tx = match action {
            Action::AuthorizeBaking => Self::authorize_baking(req_confirmation, buffer)?,
            Action::DeAuthorizeBaking => Self::deauthorize_baking(req_confirmation)?,
            Action::QueryAuthKey => Self::query_authkey(req_confirmation, buffer.write())?,
            Action::QueryAuthKeyWithCurve => {
                Self::query_authkey_withcurve(req_confirmation, buffer.write())?
            }
            Action::BakerSign => Self::baker_sign(packet_type, buffer, flags)?,

            Action::LegacyAuthorize
            | Action::LegacyDeAuthorize
            | Action::LegacyQueryAuthKey
            | Action::LegacyQueryAuthKeyWithCurve => return Err(Error::CommandNotAllowed),
        };

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
