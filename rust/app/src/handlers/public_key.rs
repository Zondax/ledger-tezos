use std::convert::TryFrom;

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
        let mut pkey = curve.gen_keypair(path)?.into_public();
        pkey.compress().map(|_| pkey)
    }

    #[inline(never)]
    fn get_public_and_address(
        key: crypto::PublicKey,
        req_confirmation: bool,
        buffer: &mut [u8],
    ) -> Result<u32, Error> {
        let mut tx = 0;

        let addr = Addr::new(&key).map_err(|_| Error::DataInvalid)?.to_base58();
        if req_confirmation {
            //TODO: show(&addr)
        }

        let key = key.as_ref();
        let len = key.len();
        //prepend pubkey with len
        buffer[0] = len as u8;
        tx += 1;

        buffer[1..1 + len].copy_from_slice(&key);
        tx += len as u32;

        let alen = addr.len();
        buffer[1 + len..1 + len + alen].copy_from_slice(&addr[..]);
        tx += alen as u32;

        Ok(tx)
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
        buffer: &mut [u8],
    ) -> Result<u32, Error> {
        let addr = Addr::new(&key).map_err(|_| Error::DataInvalid)?.to_base58();

        //TODO: show(&addr)

        let key = key.as_ref();
        let len = key.len();
        buffer[..len].copy_from_slice(&key);
        Ok(len as u32)
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
    fn handle(_flags: &mut u32, tx: &mut u32, _rx: u32, buffer: &mut [u8]) -> Result<(), Error> {
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
                Self::get_public_and_address(key, req_confirmation, buffer)?
            }
            Action::LegacyGetPublic => Self::legacy_get_public(key, buffer)?,
            Action::LegacyPromptAddressButGetPublic => {
                Self::legacy_prompt_address_get_public(key, buffer)?
            }
        };

        Ok(())
    }
}

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

        let hash = pubkey.hash()?;

        //legacy/src/to_string.c:135
        let prefix: [u8; 3] = {
            sys::PIC::new(match pubkey.curve() {
                Curve::Ed25519 | Curve::Bip32Ed25519 => [6, 161, 159],
                Curve::Secp256K1 => [6, 161, 161],
                Curve::Secp256R1 => [6, 161, 164],
            })
            .into_inner()
        };

        #[inline(never)]
        fn sha256x2(pieces: &[&[u8]]) -> Result<[u8; 32], SysError> {
            let mut digest = Sha256::new()?;
            for p in pieces {
                digest.update(p)?;
            }

            let x1 = digest.finalize_dirty()?;
            digest.reset()?;
            digest.update(&x1[..])?;
            digest.finalize().map_err(Into::into)
        }

        //legacy/src/to_string.c:94
        // hash(hash(prefix + hash))
        let checksum = sha256x2(&[&prefix[..], &hash[..]])?;

        let checksum = {
            //but only get the first 4 bytes
            let mut array = [0; 4];
            array.copy_from_slice(&checksum[..4]);
            array
        };

        Ok(Self {
            prefix,
            hash,
            checksum,
        })
    }

    //[u8; PKH_STRING] without null byte
    // legacy/src/types.h:156
    pub fn to_base58(&self) -> [u8; 36] {
        let mut input = {
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
    use super::*;

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
}
