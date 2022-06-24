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
use core::{
    mem::MaybeUninit,
    ptr::{addr_of, addr_of_mut},
};
use std::convert::TryFrom;

use zemu_sys::{Show, ViewError, Viewable};

use crate::{
    constants::ApduError as Error,
    crypto,
    dispatcher::ApduHandler,
    handlers::handle_ui_message,
    sys::{self, Error as SysError},
    utils::{ApduBufferRead, ApduPanic},
};

pub struct GetAddress;

impl GetAddress {
    /// Retrieve the public key with the given curve and bip32 path
    #[inline(never)]
    pub fn new_key_into<const B: usize>(
        curve: crypto::Curve,
        path: &sys::crypto::bip32::BIP32Path<B>,
        out: &mut MaybeUninit<crypto::PublicKey>,
    ) -> Result<(), SysError> {
        sys::zemu_log_stack("GetAddres::new_key\x00");
        curve.to_secret(path).into_public_into(out)?;

        //this is safe because it's initialized
        // also unwrapping is fine because the ptr is valid
        let pkey = unsafe { out.as_mut_ptr().as_mut().apdu_unwrap() };
        pkey.compress()
    }

    /// Retrieve the addr with the given curve and bip32 path
    #[inline(never)]
    #[allow(dead_code)]
    pub fn new_addr_into<const B: usize>(
        curve: crypto::Curve,
        path: &sys::crypto::bip32::BIP32Path<B>,
        out: &mut MaybeUninit<Addr>,
    ) -> Result<(), SysError> {
        sys::zemu_log_stack("GetAddres::new_addr_into\x00");

        let mut pkey = MaybeUninit::uninit();
        curve.to_secret(path).into_public_into(&mut pkey)?;

        //safe because we initialized it above
        let mut pkey = unsafe { pkey.assume_init() };
        pkey.compress()?;

        Addr::new_into(&pkey, out)
    }
}

impl ApduHandler for GetAddress {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        sys::zemu_log_stack("GetAddress::handle\x00");

        *tx = 0;

        let req_confirmation = buffer.p1() >= 1;
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
            Self::new_key_into(curve, &bip32_path, key).map_err(|_| Error::ExecutionError)?;
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

            Addr::new_into(key, addr).map_err(|_| Error::DataInvalid)?;
        }

        //safe because pointers are all valid, initialize with_addr
        unsafe { addr_of_mut!((*ui.as_mut_ptr()).with_addr).write(true) }

        //safe because it's all initialized now
        let mut ui = unsafe { ui.assume_init() };

        if req_confirmation {
            unsafe { ui.show(flags) }.map_err(|_| Error::ExecutionError)
        } else {
            //we don't need to show so we execute the "accept" already
            // this way the "formatting" to `buffer` is all in the ui code
            let (sz, code) = ui.accept(buffer.write());

            if code != Error::Success as u16 {
                Err(Error::try_from(code).map_err(|_| Error::ExecutionError)?)
            } else {
                *tx = sz as u32;
                Ok(())
            }
        }
    }
}

#[derive(Default, Clone, Copy)]
pub struct Addr {
    prefix: [u8; 3],
    hash: [u8; 20],
    checksum: [u8; 4],
}

impl Addr {
    pub const BASE58_LEN: usize = 37;

    #[inline(never)]
    #[allow(dead_code)]
    pub fn new(pubkey: &crypto::PublicKey) -> Result<Self, SysError> {
        sys::zemu_log_stack("Addr::new\x00");

        let mut this = MaybeUninit::uninit();
        Self::new_into(pubkey, &mut this)?;

        Ok(unsafe { this.assume_init() })
    }

    #[inline(never)]
    pub fn new_into(
        pubkey: &crypto::PublicKey,
        out: &mut MaybeUninit<Self>,
    ) -> Result<(), SysError> {
        sys::zemu_log_stack("Addr::new_into\x00");

        let out = out.as_mut_ptr();

        unsafe {
            //this is okay because while out is unitialized
            // the "hash" is just an array of bytes, and all configurations are valid
            // but we don't read anyways
            let hash = addr_of_mut!((*out).hash).as_mut().apdu_unwrap();
            pubkey.hash(hash)?;
        }
        sys::zemu_log_stack("Addr::new after hash\x00");

        //legacy/src/to_string.c:135
        unsafe {
            //same as above, the pointer is valid and "initialized"
            // in a sense that no matter the value of `prefix` it's valid
            addr_of_mut!((*out).prefix)
                .as_mut()
                .apdu_unwrap()
                .copy_from_slice(pubkey.curve().to_hash_prefix());
        }

        //safe because all pointers here are valid
        unsafe {
            super::sha256x2(
                &[
                    &addr_of!((*out).prefix).as_ref().apdu_unwrap()[..], //this has been initialized
                    &addr_of!((*out).hash).as_ref().apdu_unwrap()[..], //this has also been initialized
                ],
                //same as when we wrote into prefix and hash
                // the data is always valid, even when uninitialized
                // and the pointer comes from a proper reference
                addr_of_mut!((*out).checksum).as_mut().apdu_unwrap(),
            )?;
        }

        Ok(())
    }

    //[u8; PKH_STRING] without null byte
    // legacy/src/types.h:156
    //
    /// Returns the address encoded with base58 and also the actual number of bytes written in the buffer
    pub fn base58(&self) -> (usize, [u8; Addr::BASE58_LEN]) {
        let input = {
            let mut array = [0; 27];
            array[..3].copy_from_slice(&self.prefix[..]);
            array[3..3 + 20].copy_from_slice(&self.hash[..]);
            array[3 + 20..3 + 20 + 4].copy_from_slice(&self.checksum[..]);
            array
        };

        let mut out = [0; Self::BASE58_LEN];

        //the expect is ok since we know all the sizes
        let len = bs58::encode(input)
            .into(&mut out[..])
            .apdu_expect("encoded in base58 is not of the right length");

        (len, out)
    }

    #[allow(dead_code)]
    pub fn into_ui(self, pkey: crypto::PublicKey, with_addr: bool) -> AddrUI {
        AddrUI {
            addr: self,
            pkey,
            with_addr,
        }
    }

    pub fn from_hash(hash: &[u8; 20], crv: crypto::Curve) -> Result<Self, SysError> {
        let mut this: Self = Default::default();

        this.hash.copy_from_slice(&hash[..]);
        this.prefix.copy_from_slice(crv.to_hash_prefix());

        super::sha256x2(&[&this.prefix[..], &this.hash[..]], &mut this.checksum)?;

        Ok(this)
    }
}

pub struct AddrUI {
    //this is here to faciliate the UI,
    // it would otherwise be redundant with the pkey
    pub addr: Addr,
    pub pkey: crypto::PublicKey,

    /// indicates whether to write `add` to out or not
    pub with_addr: bool,
}

impl Viewable for AddrUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        Ok(1)
    }

    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use bolos::{pic_str, PIC};

        if let 0 = item_n {
            let title_content = pic_str!(b"Address");
            title[..title_content.len()].copy_from_slice(title_content);

            let (len, mex) = self.addr.base58();
            handle_ui_message(&mex[..len], message, page)
        } else {
            Err(ViewError::NoData)
        }
    }

    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let pkey = self.pkey.as_ref();
        let mut tx = 0;

        out[tx] = pkey.len() as u8;
        tx += 1;
        out[tx..tx + pkey.len()].copy_from_slice(pkey);
        tx += pkey.len();

        if self.with_addr {
            let (len, addr) = self.addr.base58();
            out[tx..tx + len].copy_from_slice(&addr[..len]);

            tx += len;
        }

        (tx, Error::Success as _)
    }

    fn reject(&mut self, _: &mut [u8]) -> (usize, u16) {
        (0, Error::CommandNotAllowed as _)
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl Addr {
    pub fn from_parts(prefix: [u8; 3], hash: [u8; 20], checksum: [u8; 4]) -> Self {
        Self {
            prefix,
            hash,
            checksum,
        }
    }

    pub fn bytes(&self) -> std::vec::Vec<u8> {
        let mut out = std::vec::Vec::with_capacity(3 + 20 + 4);
        out.extend_from_slice(&self.prefix[..]);
        out.extend_from_slice(&self.hash[..]);
        out.extend_from_slice(&self.checksum[..]);

        out
    }
}

#[cfg(test)]
mod tests {
    use bolos::crypto::{bip32::BIP32Path, Curve};
    use std::convert::TryInto;

    use super::*;
    use crate::{
        assert_error_code,
        constants::ApduError,
        dispatcher::{handle_apdu, CLA, INS_LEGACY_GET_PUBLIC_KEY},
    };

    #[test]
    fn check_bs58() {
        let addr = Addr::from_parts(
            [0x6, 0xa1, 0x9f],
            [
                0xc8, 0x60, 0xbe, 0x67, 0x3a, 0xe4, 0x7e, 0xc5, 0x49, 0xf9, 0xb5, 0xa0, 0x1a, 0x8c,
                0xcb, 0x65, 0x7b, 0xe7, 0x5b, 0x6a,
            ],
            [0x88, 0x8a, 0x19, 0x84],
        );

        let expected = "tz1duXjMpT43K7F1nQajzH5oJLTytLUNxoTZ";
        let (len, output) = addr.base58();

        assert_eq!(expected.as_bytes(), &output[..len]);
    }

    fn prepare_buffer<const LEN: usize>(buffer: &mut [u8; 260], path: &[u32], curve: Curve) {
        let crv: u8 = curve.into();
        let path = BIP32Path::<LEN>::new(path.iter().map(|n| 0x8000_0000 + n))
            .unwrap()
            .serialize();

        buffer[3] = crv;
        buffer[4] = path.len() as u8;
        buffer[5..5 + path.len()].copy_from_slice(path.as_slice());
    }

    #[test]
    fn apdu_legacy_get_public_key() {
        let mut flags = 0u32;
        let mut tx = 0u32;
        let rx = 5;
        let mut buffer = [0u8; 260];

        buffer[..3].copy_from_slice(&[CLA, INS_LEGACY_GET_PUBLIC_KEY, 0]);
        prepare_buffer::<4>(&mut buffer, &[44, 1729, 0, 0], Curve::Ed25519);

        handle_apdu(&mut flags, &mut tx, rx, &mut buffer);

        assert_error_code!(tx, buffer, ApduError::Success);
        assert_eq!(tx as usize, 1 + 32 + 2); //32 bytes for ed25519
    }

    #[test]
    fn freeze_with_tezos_client() {
        const PAYLOAD: &[u8] = &[
            0x80, 0x02, 0x00, 0x00, 0x09, 0x02, 0x80, 0x00, 0x00, 0x2c, 0x80, 0x00, 0x06, 0xc1,
        ];

        let mut flags = 0u32;
        let mut tx = 0u32;
        let rx = 5;
        let mut buffer = [0u8; 260];
        buffer[..PAYLOAD.len()].copy_from_slice(PAYLOAD);

        handle_apdu(&mut flags, &mut tx, rx, &mut buffer);

        assert_error_code!(tx, buffer, ApduError::Success);
        assert_eq!(tx as usize, 1 + 32 + 2);
    }
}
