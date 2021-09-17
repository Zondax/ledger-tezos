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
use crate::{
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::{self, Curve},
    dispatcher::ApduHandler,
    handlers::{
        handle_ui_message,
        hwm::HWM,
        public_key::{Addr, GetAddress},
    },
    sys::{self, crypto::bip32::BIP32Path},
    utils::ApduBufferRead,
};
use bolos::{errors::SyscallError, pic_str, PIC};
use core::convert::TryFrom;
use zemu_sys::{Show, ViewError, Viewable};

use super::{Baking, Bip32PathAndCurve, BAKINGPATH};

pub struct AuthorizeBaking;

impl AuthorizeBaking {
    #[inline(always)]
    pub fn authorize(
        curve: Curve,
        path: BIP32Path<BIP32_MAX_LENGTH>,
        flags: &mut u32,
    ) -> Result<u32, Error> {
        sys::zemu_log_stack("AuthorizeBaking::auth\x00");
        let ui = AuthorizeUI::new(curve, path).map_err(|_| Error::ExecutionError)?;

        unsafe { ui.show(flags) }
            .map_err(|_| Error::ExecutionError)
            .map(|_| 0)
    }
}

struct AuthorizeUI {
    curve: Curve,
    path: BIP32Path<BIP32_MAX_LENGTH>,
    //this is a bit reduntant info
    // but it helps speed up the UI
    addr: Addr,
}

impl AuthorizeUI {
    #[inline(never)]
    pub fn new(curve: Curve, path: BIP32Path<BIP32_MAX_LENGTH>) -> Result<Self, Error> {
        sys::zemu_log_stack("AuthorizeUI::new\x00");
        let addr = GetAddress::new_key(curve, &path)
            .and_then(|k| Addr::new(&k))
            .map_err(|_| Error::ExecutionError)?;
        Ok(Self { curve, path, addr })
    }
}

impl ApduHandler for AuthorizeBaking {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        sys::zemu_log_stack("AuthorizeBaking::handle\x00");
        let req_confirmation = buffer.p1() >= 1;

        if !req_confirmation {
            //confirmation is mandatory
            return Err(Error::ApduCodeConditionsNotSatisfied);
        }

        let curve = Curve::try_from(buffer.p2()).map_err(|_| Error::InvalidP1P2)?;

        let cdata = buffer.payload().map_err(|_| Error::DataInvalid)?;
        let bip32_path =
            BIP32Path::<BIP32_MAX_LENGTH>::read(cdata).map_err(|_| Error::DataInvalid)?;

        *tx = Self::authorize(curve, bip32_path, flags)?;

        Ok(())
    }
}

impl Viewable for AuthorizeUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        Ok(2)
    }

    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        match item_n {
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Authorize Baking")[..], message, page)
            }
            1 => {
                let title_content = pic_str!(b"Address");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&self.addr.to_base58()[..], message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }

    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        //get public key
        let pk = match GetAddress::new_key(self.curve, &self.path) {
            Ok(pk) => pk,
            Err(_) => return (0, Error::ExecutionError as _),
        };

        //store in memory
        let path_and_curve = Bip32PathAndCurve::new(self.curve, self.path);
        if unsafe { BAKINGPATH.write(path_and_curve.into()) }.is_err() {
            return (0, Error::ExecutionError as _);
        }

        //reset watermark
        if HWM::reset(0).is_err() {
            return (0, Error::Busy as _);
        }

        //write to out
        let key = pk.as_ref();
        let len = key.len();
        out[0] = len as u8;
        out[1..1 + len].copy_from_slice(&key);

        (1 + len as usize, Error::Success as _)
    }

    fn reject(&mut self, out: &mut [u8]) -> (usize, u16) {
        (0, Error::CommandNotAllowed as _)
    }
}

pub struct DeAuthorizeBaking;

impl DeAuthorizeBaking {
    #[inline(never)]
    pub fn deauthorize(flags: &mut u32) -> Result<u32, Error> {
        let path = unsafe { BAKINGPATH.read() }
            .map_err(|_| Error::ApduCodeConditionsNotSatisfied)
            .and_then(|slot| Bip32PathAndCurve::try_from_bytes(&slot))?
            .ok_or(Error::ApduCodeConditionsNotSatisfied)?;

        let addr = GetAddress::new_key(path.curve, &path.path)
            .and_then(|key| Addr::new(&key))
            .map_err(|_| Error::ExecutionError)?;

        let ui = DeAuthorizeUI { addr };

        unsafe { ui.show(flags) }
            .map_err(|_| Error::ExecutionError)
            .map(|_| 0)
    }
}

struct DeAuthorizeUI {
    addr: Addr,
}

impl ApduHandler for DeAuthorizeBaking {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        let req_confirmation = buffer.p1() >= 1;

        //confirmation mandatory
        if !req_confirmation {
            return Err(Error::ApduCodeConditionsNotSatisfied);
        }

        *tx = Self::deauthorize(flags)?;

        Ok(())
    }
}

impl Viewable for DeAuthorizeUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        Ok(2)
    }

    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        match item_n {
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"DeAuthorize Baking")[..], message, page)
            }
            1 => {
                let title_content = pic_str!(b"Address");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&self.addr.to_base58()[..], message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }

    fn accept(&mut self, _: &mut [u8]) -> (usize, u16) {
        if HWM::reset(0).is_err() {
            return (0, Error::ExecutionError as _);
        }

        if unsafe { BAKINGPATH.write(Bip32PathAndCurve::empty()) }.is_err() {
            return (0, Error::ExecutionError as _);
        }

        (0, Error::Success as _)
    }

    fn reject(&mut self, _: &mut [u8]) -> (usize, u16) {
        (0, Error::CommandNotAllowed as _)
    }
}
