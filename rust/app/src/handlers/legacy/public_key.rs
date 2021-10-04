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

use crate::{
    constants::ApduError as Error,
    crypto,
    dispatcher::ApduHandler,
    handlers::public_key::{Addr, GetAddress},
    sys::{self, Show},
};

pub struct LegacyGetPublic;
pub struct LegacyPromptAddress;

impl ApduHandler for LegacyGetPublic {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        buffer: crate::utils::ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        //TODO: require_hid ?
        // see: https://github.com/Zondax/ledger-tezos/issues/35

        let curve = crypto::Curve::try_from(buffer.p2()).map_err(|_| Error::InvalidP1P2)?;

        let cdata = buffer.payload().map_err(|_| Error::DataInvalid)?;
        let bip32_path =
            sys::crypto::bip32::BIP32Path::<6>::read(cdata).map_err(|_| Error::DataInvalid)?;

        let key = GetAddress::new_key(curve, &bip32_path).map_err(|_| Error::ExecutionError)?;

        let key = key.as_ref();
        let len = key.len();

        let out = buffer.write();
        out[0] = len as u8;
        *tx += 1;

        out[1..1 + len].copy_from_slice(key);
        *tx += len as u32;

        Ok(())
    }
}

impl ApduHandler for LegacyPromptAddress {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: crate::utils::ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        let curve = crypto::Curve::try_from(buffer.p2()).map_err(|_| Error::InvalidP1P2)?;

        let cdata = buffer.payload().map_err(|_| Error::DataInvalid)?;
        let bip32_path =
            sys::crypto::bip32::BIP32Path::<6>::read(cdata).map_err(|_| Error::DataInvalid)?;

        let key = GetAddress::new_key(curve, &bip32_path).map_err(|_| Error::ExecutionError)?;

        let ui = Addr::new(&key)
            .map_err(|_| Error::DataInvalid)?
            .into_ui(key, false);

        unsafe { ui.show(flags) }.map_err(|_| Error::ExecutionError)
    }
}
