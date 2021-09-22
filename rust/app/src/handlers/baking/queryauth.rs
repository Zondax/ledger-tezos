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
    crypto::Curve,
    dispatcher::ApduHandler,
    handlers::{
        handle_ui_message,
        public_key::{Addr, GetAddress},
    },
    sys::crypto::bip32::BIP32Path,
    utils::ApduBufferRead,
};
use bolos::{pic_str, PIC};
use zemu_sys::{Show, ViewError, Viewable};

use core::convert::TryFrom;

use super::{Bip32PathAndCurve, BAKINGPATH};

#[inline(never)]
fn query(
    req_confirmation: bool,
    with_curve: bool,
    buffer: &mut [u8],
    flags: &mut u32,
) -> Result<u32, Error> {
    //Check if the current baking path in NVM is initialized
    let current_path =
        unsafe { BAKINGPATH.read() }.map_err(|_| Error::ApduCodeConditionsNotSatisfied)?;

    //path seems to be initialized so we can return it
    //check if it is a good path
    let curve_and_path = Bip32PathAndCurve::try_from_bytes(current_path)?
        .ok_or(Error::ApduCodeConditionsNotSatisfied)?;

    let mut ui = QueryAuthUI::new(curve_and_path, with_curve)?;

    if req_confirmation {
        unsafe { ui.show(flags).map_err(|_| Error::ExecutionError).map(|_| 0) }
    } else {
        let (sz, code) = ui.accept(buffer);

        if code != Error::Success as u16 {
            Err(Error::try_from(code).map_err(|_| Error::ExecutionError)?)
        } else {
            Ok(sz as u32)
        }
    }
}

pub struct QueryAuthKey;

impl QueryAuthKey {
    pub fn query(req_confirmation: bool, out: &mut [u8], flags: &mut u32) -> Result<u32, Error> {
        query(req_confirmation, false, out, flags)
    }
}

impl ApduHandler for QueryAuthKey {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        let req_confirmation = buffer.p1() >= 1;

        *tx = Self::query(req_confirmation, buffer.write(), flags)?;

        Ok(())
    }
}

pub struct QueryAuthKeyWithCurve;

impl QueryAuthKeyWithCurve {
    pub fn query(req_confirmation: bool, out: &mut [u8], flags: &mut u32) -> Result<u32, Error> {
        query(req_confirmation, true, out, flags)
    }
}

impl ApduHandler for QueryAuthKeyWithCurve {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        let req_confirmation = buffer.p1() >= 1;

        *tx = Self::query(req_confirmation, buffer.write(), flags)?;

        Ok(())
    }
}

struct QueryAuthUI {
    curve: Curve,
    path: BIP32Path<BIP32_MAX_LENGTH>,
    //this is reduntant, but it helps to speed up the UI
    addr: Addr,
    with_curve: bool,
}

impl QueryAuthUI {
    #[inline(never)]
    pub fn new(data: Bip32PathAndCurve, with_curve: bool) -> Result<Self, Error> {
        let addr = GetAddress::new_key(data.curve, &data.path)
            .and_then(|k| Addr::new(&k))
            .map_err(|_| Error::ExecutionError)?;

        Ok(Self {
            curve: data.curve,
            path: data.path,
            addr,
            with_curve,
        })
    }
}

impl Viewable for QueryAuthUI {
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

                handle_ui_message(&pic_str!(b"Query Authorized")[..], message, page)
            }
            1 => {
                let title_content = pic_str!(b"Address");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&self.addr.base58()[..], message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }

    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let mut tx = 0;

        if self.with_curve {
            out[tx] = self.curve.into();
            tx += 1;
        }

        let components = self.path.components();
        out[tx] = components.len() as u8;
        tx += 1;

        out[tx..]
            .chunks_exact_mut(4)
            .zip(components)
            .for_each(|(chunk, component)| {
                chunk.copy_from_slice(&component.to_be_bytes()[..]);
                tx += 4
            });

        (tx, Error::Success as _)
    }

    fn reject(&mut self, _: &mut [u8]) -> (usize, u16) {
        (0, Error::CommandNotAllowed as _)
    }
}
