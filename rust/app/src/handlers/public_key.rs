use core::u8;
use std::convert::TryFrom;

use zemu_sys::{Show, ViewError, Viewable};

use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS},
    crypto,
    dispatcher::{
        ApduHandler, INS_GET_ADDRESS, INS_LEGACY_GET_PUBLIC_KEY, INS_LEGACY_PROMPT_PUBLIC_KEY,
    },
    sys::{self, Error as SysError},
};

pub struct GetAddress;

impl GetAddress {
    /// Retrieve the public key with the given curve and bip32 path
    #[inline(never)]
    pub fn new_key<const B: usize>(
        curve: crypto::Curve,
        path: &sys::crypto::bip32::BIP32Path<B>,
    ) -> Result<crypto::PublicKey, SysError> {
        sys::zemu_log_stack("GetAddres::new_key\x00");
        let mut pkey = curve.gen_keypair(path)?.into_public();
        pkey.compress().map(|_| pkey)
    }

    #[inline(never)]
    fn get_public_and_address(
        key: crypto::PublicKey,
        req_confirmation: bool,
        buffer: &mut [u8],
        flags: &mut u32,
    ) -> Result<u32, Error> {
        sys::zemu_log_stack("GetAddres::get_public_and_address\x00");

        let addr = Addr::new(&key).map_err(|_| Error::DataInvalid)?;
        let mut ui = AddrUI {
            addr,
            pkey: key,
            with_addr: true,
        };
        if req_confirmation {
            unsafe { ui.show(flags) }
                .map_err(|_| Error::ExecutionError)
                .map(|_| 0)
        } else {
            //we don't need to show so we execute the "accept" already
            // this way the "formatting" to `buffer` is all in the ui code
            let (sz, code) = ui.accept(buffer);

            if code != Error::Success as _ {
                Err(Error::try_from(code).map_err(|_| Error::ExecutionError)?)
            } else {
                Ok(sz as u32)
            }
        }
    }

    #[inline(never)]
    fn legacy_get_public(key: crypto::PublicKey, buffer: &mut [u8]) -> Result<u32, Error> {
        let key = key.as_ref();
        let len = key.len();
        buffer[..len].copy_from_slice(&key);
        Ok(len as u32)
    }

    #[inline(never)]
    fn legacy_prompt_address_get_public(
        key: crypto::PublicKey,
        _buffer: &mut [u8],
        flags: &mut u32,
    ) -> Result<u32, Error> {
        let addr = Addr::new(&key).map_err(|_| Error::DataInvalid)?;

        let ui = AddrUI {
            addr,
            pkey: key,
            with_addr: false,
        };

        unsafe { ui.show(flags) }
            .map_err(|_| Error::ExecutionError)
            .map(|_| 0)
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    //NEW API: return concat(public_key,address)
    GetPublicAndAddress,

    //LEGACY API: return only public_key
    LegacyGetPublic,

    //LEGACY API: prompt user with address and if okay return public_key
    LegacyPromptAddressButGetPublic,
}

impl ApduHandler for GetAddress {
    #[inline(never)]
    fn handle(flags: &mut u32, tx: &mut u32, _rx: u32, buffer: &mut [u8]) -> Result<(), Error> {
        sys::zemu_log_stack("GetAddress::handle\x00");

        *tx = 0;
        let action = match buffer[APDU_INDEX_INS] {
            INS_GET_ADDRESS => Action::GetPublicAndAddress,
            INS_LEGACY_GET_PUBLIC_KEY => Action::LegacyGetPublic,
            INS_LEGACY_PROMPT_PUBLIC_KEY => Action::LegacyPromptAddressButGetPublic,
            _ => return Err(Error::InsNotSupported),
        };

        if let Action::LegacyGetPublic = action {
            //TODO: require_hid ?
            // see: https://github.com/Zondax/ledger-tezos/issues/35
        }

        let req_confirmation = buffer[2] >= 1;
        let curve = crypto::Curve::try_from(buffer[3]).map_err(|_| Error::InvalidP1P2)?;

        let cdata_len = buffer[4] as usize;
        if cdata_len > buffer[5..].len() {
            return Err(Error::DataInvalid);
        }
        let cdata = &buffer[5..5 + cdata_len];

        let bip32_path =
            sys::crypto::bip32::BIP32Path::<6>::read(cdata).map_err(|_| Error::DataInvalid)?;

        let key = Self::new_key(curve, &bip32_path).map_err(|_| Error::ExecutionError)?;

        *tx = match action {
            Action::GetPublicAndAddress => {
                Self::get_public_and_address(key, req_confirmation, buffer, flags)?
            }
            Action::LegacyGetPublic => Self::legacy_get_public(key, buffer)?,
            Action::LegacyPromptAddressButGetPublic => {
                Self::legacy_prompt_address_get_public(key, buffer, flags)?
            }
        };

        Ok(())
    }
}

#[derive(Default)]
pub struct Addr {
    prefix: [u8; 3],
    hash: [u8; 20],
    checksum: [u8; 4],
}

impl Addr {
    pub fn new(pubkey: &crypto::PublicKey) -> Result<Self, SysError> {
        use crypto::Curve;
        use sys::hash::{Hasher, Sha256};
        sys::zemu_log_stack("Addr::new\x00");

        let mut this: Self = Default::default();

        pubkey.hash(&mut this.hash)?;
        sys::zemu_log_stack("Addr::new after hash\x00");

        //legacy/src/to_string.c:135
        this.prefix.copy_from_slice(
            &sys::PIC::new(match pubkey.curve() {
                Curve::Ed25519 | Curve::Bip32Ed25519 => [6, 161, 159],
                Curve::Secp256K1 => [6, 161, 161],
                Curve::Secp256R1 => [6, 161, 164],
            })
            .into_inner()[..],
        );

        #[inline(never)]
        fn sha256x2(pieces: &[&[u8]], out: &mut [u8; 4]) -> Result<(), SysError> {
            sys::zemu_log_stack("Addr::new::sha256x2\x00");
            let mut digest = Sha256::new()?;
            for p in pieces {
                digest.update(p)?;
            }

            let x1 = digest.finalize_dirty()?;
            digest.reset()?;
            digest.update(&x1[..])?;

            let complete_digest = digest.finalize()?;

            out.copy_from_slice(&complete_digest[..4]);

            Ok(())
        }

        sha256x2(&[&this.prefix[..], &this.hash[..]], &mut this.checksum)?;

        Ok(this)
    }

    //[u8; PKH_STRING] without null byte
    // legacy/src/types.h:156
    pub fn to_base58(&self) -> [u8; 36] {
        let input = {
            let mut array = [0; 27];
            array[..3].copy_from_slice(&self.prefix[..]);
            array[3..3 + 20].copy_from_slice(&self.hash[..]);
            array[3 + 20..3 + 20 + 4].copy_from_slice(&self.checksum[..]);
            array
        };

        let mut out = [0; 36];

        //the expect is ok since we know all the sizes
        bs58::encode(input)
            .into(&mut out[..])
            .expect("encoded in base58 is not of the right length");

        out
    }
}

struct AddrUI {
    addr: Addr,
    pkey: crypto::PublicKey,

    /// indicates whether to write `add` to out or not
    with_addr: bool,
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
        if let 0 = item_n {
            let title_content = bolos::PIC::new(b"Address\x00").into_inner();

            title[..title_content.len()].copy_from_slice(title_content);

            let addr_bytes = self.addr.to_base58();
            let m_len = message.len() - 1;
            if addr_bytes.len() > m_len {
                let chunk = addr_bytes
                    .chunks(m_len)
                    .nth(page as usize)
                    .ok_or(ViewError::Unknown)?;

                let len = std::cmp::min(chunk.len(), m_len);
                message[..len].copy_from_slice(chunk);

                message[len] = 0;
                let n_pages = addr_bytes.len() / m_len;
                Ok(1 + n_pages as u8)
            } else {
                message[..addr_bytes.len()].copy_from_slice(&addr_bytes[..]);
                message[addr_bytes.len()] = 0;
                Ok(1)
            }
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
            let addr = self.addr.to_base58();

            out[tx..tx + addr.len()].copy_from_slice(&addr[..]);

            tx += addr.len();
        }

        (tx, Error::Success as _)
    }

    fn reject(&mut self, _: &mut [u8]) -> (usize, u16) {
        (0, Error::CommandNotAllowed as _)
    }
}

#[cfg(test)]
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
        //TODO: use mocked hashing instead
        let addr = Addr::from_parts(
            [0x6, 0xa1, 0x9f],
            [
                0xc8, 0x60, 0xbe, 0x67, 0x3a, 0xe4, 0x7e, 0xc5, 0x49, 0xf9, 0xb5, 0xa0, 0x1a, 0x8c,
                0xcb, 0x65, 0x7b, 0xe7, 0x5b, 0x6a,
            ],
            [0x88, 0x8a, 0x19, 0x84],
        );

        let expected = "tz1duXjMpT43K7F1nQajzH5oJLTytLUNxoTZ";
        let output = addr.to_base58();
        let output = std::str::from_utf8(&output[..]).unwrap();

        assert_eq!(expected, output);
    }

    fn prepare_buffer<const LEN: usize>(buffer: &mut [u8; 260], path: &[u32], curve: Curve) {
        let crv: u8 = curve.into();
        let path = BIP32Path::<LEN>::new(path.into_iter().map(|n| 0x8000_0000 + n))
            .unwrap()
            .serialize();

        buffer[3] = crv;
        buffer[4] = path.len() as u8;
        buffer[5..5 + path.len()].copy_from_slice(path.as_slice());
    }

    #[test]
    #[should_panic(expected = "not yet implemented")]
    fn apdu_legacy_get_public_key() {
        let mut flags = 0u32;
        let mut tx = 0u32;
        let rx = 5;
        let mut buffer = [0u8; 260];

        buffer[..3].copy_from_slice(&[CLA, INS_LEGACY_GET_PUBLIC_KEY, 0]);
        prepare_buffer::<4>(&mut buffer, &[44, 1729, 0, 0], Curve::Ed25519);

        handle_apdu(&mut flags, &mut tx, rx, &mut buffer);

        assert_error_code!(tx, buffer, ApduError::Success);
        assert_eq!(tx as usize, 1 + 33 + 2);

        // FIXME: Complete the test
    }
}
