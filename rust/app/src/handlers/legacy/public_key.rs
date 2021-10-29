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
use std::{
    convert::TryFrom,
    mem::MaybeUninit,
    ptr::{addr_of, addr_of_mut},
};

use crate::{
    constants::ApduError as Error,
    crypto,
    dispatcher::ApduHandler,
    handlers::public_key::{Addr, AddrUI, GetAddress},
    sys::{self, Show},
    utils::ApduPanic,
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

        let mut key = MaybeUninit::uninit();
        GetAddress::new_key_into(curve, &bip32_path, &mut key)
            .map_err(|_| Error::ExecutionError)?;

        //safe beause it's initialized
        let key = unsafe { key.assume_init() };
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

        let mut ui = MaybeUninit::<AddrUI>::uninit();

        //initialize public key
        {
            //get ui *mut
            let ui = ui.as_mut_ptr();
            //get `pkey` *mut,
            // cast to MaybeUninit *mut
            //SAFE: `as_mut` it to &mut MaybeUninit (safe because it's MaybeUninit)
            // unwrap the option as it's guarantee valid pointer
            let key =
                unsafe { addr_of_mut!((*ui).pkey).cast::<MaybeUninit<_>>().as_mut() }.apdu_unwrap();
            GetAddress::new_key_into(curve, &bip32_path, key).map_err(|_| Error::ExecutionError)?;
        }

        //initialize address
        {
            let ui = ui.as_mut_ptr();

            //get &mut MaybeUninit<Addr>
            let addr =
                unsafe { addr_of_mut!((*ui).addr).cast::<MaybeUninit<_>>().as_mut() }.apdu_unwrap();

            //get _initialized_ key
            //SAFE: pkey is valid pointer and INITIALIZED in the block above
            let key = unsafe { addr_of!((*ui).pkey).as_ref() }.apdu_unwrap();

            Addr::new_into(&key, addr).map_err(|_| Error::DataInvalid)?;
        }

        //safe because pointers are all valid, initialize with_addr
        unsafe { addr_of_mut!((*ui.as_mut_ptr()).with_addr).write(true) }

        //safe because it's all initialized now
        let ui = unsafe { ui.assume_init() };

        unsafe { ui.show(flags) }.map_err(|_| Error::ExecutionError)
    }
}
