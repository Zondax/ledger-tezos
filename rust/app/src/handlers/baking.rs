use std::convert::TryFrom;

use nom::{
    bytes::complete::take,
    number::complete::{be_u32, le_u8},
};

use zemu_sys::{Show, ViewError, Viewable};

use bolos::{
    crypto::bip32::BIP32Path,
    hash::{Blake2b, Hasher},
    pic_str, PIC,
};

use crate::handlers::{handle_ui_message, parser_common::ParserError};
use crate::{
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::{self, Curve},
    dispatcher::ApduHandler,
    handlers::hwm::{WaterMark, HWM},
    handlers::public_key::GetAddress,
    sys::{self, flash_slot::Wear, new_flash_slot},
    utils::{ApduBufferRead, Uploader},
};
use bolos::flash_slot::WearError;

const N_PAGES_BAKINGPATH: usize = 1;

type WearLeveller = Wear<'static, N_PAGES_BAKINGPATH>;

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

pub struct EndorsementData<'b> {
    pub baker_preemble: Preemble,
    pub chain_id: u32,
    pub branch: &'b [u8; 32],
    pub tag: u8, //TODO: what to do with this??
    pub level: u32,
}

impl<'b> EndorsementData<'b> {
    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> nom::IResult<&[u8], Self, ParserError> {
        let (rem, preemble) = le_u8(bytes)?;
        let baker_preemble: Preemble =
            Preemble::try_from(preemble).map_err(|_| ParserError::parser_unexpected_error)?;
        let (rem, chain_id) = be_u32(rem)?;
        let (rem, branch) = take(32usize)(rem)?;
        let branch = arrayref::array_ref!(branch, 0, 32);
        let (rem, tag) = le_u8(rem)?;
        let (rem, level) = be_u32(rem)?;

        Ok((
            rem,
            Self {
                baker_preemble,
                chain_id,
                branch,
                tag,
                level,
            },
        ))
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        WaterMark::is_valid_blocklevel(self.level)
            && (self.level > hw.level || (hw.level == self.level && !hw.endorsement))
    }

    pub fn num_items(&self) -> usize {
        4
    }

    pub fn render_item(
        &self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use lexical_core::{write as itoa, Number};

        match item_n {
            0 => {
                let title_content = pic_str!(b"Baking Sign");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Endorsement")[..], message, page)
            }
            1 => {
                let title_content = pic_str!(b"Branch");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut hex_buf = [0; 32 * 2];
                //this is impossible that will error since the sizes are all checked
                hex::encode_to_slice(&self.branch[..], &mut hex_buf).unwrap();

                handle_ui_message(&hex_buf[..], message, page)
            }
            2 => {
                let title_content = pic_str!(b"Blocklevel");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.level, &mut itoa_buf), message, page)
            }
            3 => {
                let title_content = pic_str!(b"ChainID");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.chain_id, &mut itoa_buf), message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}

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

        Ok((
            rem,
            Self {
                baker_preemble,
                chain_id,
                level,
                proto,
            },
        ))
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        WaterMark::is_valid_blocklevel(self.level) && (self.level > hw.level)
    }

    pub fn num_items(&self) -> usize {
        3
    }

    pub fn render_item(
        &self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use lexical_core::{write as itoa, Number};

        match item_n {
            0 => {
                let title_content = pic_str!(b"Baking Sign");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Blocklevel")[..], message, page)
            }
            1 => {
                let title_content = pic_str!(b"ChainID");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.chain_id, &mut itoa_buf), message, page)
            }
            2 => {
                let title_content = pic_str!(b"Blocklevel");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.level, &mut itoa_buf), message, page)
            }
            _ => Err(ViewError::NoData),
        }
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
    fn baker_sign(buffer: ApduBufferRead<'_>, flags: &mut u32) -> Result<u32, Error> {
        if let Some(upload) = Uploader::new(Self).upload(&buffer)? {
            let curve = Curve::try_from(upload.p2).map_err(|_| Error::InvalidP1P2)?;
            let path = BIP32Path::<BIP32_MAX_LENGTH>::read(upload.first)
                .map_err(|_| Error::DataInvalid)?;

            Self::check_with_nvm_pathandcurve(&curve, &path)?;

            unsafe { PATH.replace((path, curve)) };
            let hw = HWM::read()?;
            //do watermarks checks

            let cdata = upload.data;
            let preemble = Preemble::try_from(cdata[0])?;

            let baking_ui = match preemble {
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

                    let digest = Self::blake2b_digest(&cdata)?;

                    BakingSignUI {
                        endorsement: Some(endorsement),
                        blocklevel: None,
                        digest,
                    }
                }
                Preemble::BlockPreemble => {
                    let (_, blockdata) =
                        BlockData::from_bytes(&cdata).map_err(|_| Error::DataInvalid)?;

                    if !blockdata.validate_with_watermark(&hw) {
                        return Err(Error::DataInvalid);
                    }

                    let digest = Self::blake2b_digest(&cdata)?;
                    //TODO: show blocklevel on screen
                    BakingSignUI {
                        endorsement: None,
                        blocklevel: Some(blockdata),
                        digest,
                    }
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
            };

            unsafe { baking_ui.show(flags) }
                .map_err(|_| Error::ExecutionError)
                .map(|_| 0)
        } else {
            Ok(0)
        }
    }
}

struct BakingSignUI {
    pub endorsement: Option<EndorsementData<'static>>,
    pub blocklevel: Option<BlockData>,
    pub digest: [u8; 32],
}

//FIXME: split the below code for endorsements and blocklevel signing
impl Viewable for BakingSignUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        if let Some(endorsement) = &self.endorsement {
            Ok(endorsement.num_items() as u8)
        } else if let Some(blocklevel) = &self.blocklevel {
            Ok(blocklevel.num_items() as u8)
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
            endorsement.render_item(item_n, title, message, page)
        } else if let Some(block) = &self.blocklevel {
            block.render_item(item_n, title, message, page)
        } else {
            Err(ViewError::NoData)
        }
    }

    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let (blocklevel, is_endorsement) = match (&self.endorsement, &self.blocklevel) {
            (Some(endorsement), None) => (endorsement.level, true),
            (None, Some(block)) => (block.level, false),
            _ => return (0, Error::DataInvalid as _),
        };

        let new_hw = WaterMark {
            level: blocklevel,
            endorsement: is_endorsement,
        };
        if HWM::write(new_hw).is_err() {
            return (0, Error::ExecutionError as _);
        }

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
        let sz = match secret.sign(&self.digest, &mut sig[..]) {
            Ok(sz) => sz,
            Err(_) => return (0, Error::ExecutionError as _),
        };

        //reset globals to avoid skipping `Init`
        if let Err(e) = cleanup_globals() {
            return (0, e as _);
        }

        let mut tx = 0;

        //write unsigned_hash to buffer
        out[tx..tx + 32].copy_from_slice(&self.digest);
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
pub struct BakerSign;

impl ApduHandler for AuthorizeBaking {
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

impl ApduHandler for BakerSign {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        *tx = Baking::baker_sign(buffer, flags)?;

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
        assert_eq!(endorsement.branch, &[0u8; 32]);
        assert_eq!(endorsement.tag, 5);
        assert_eq!(endorsement.level, 15);
    }
}
