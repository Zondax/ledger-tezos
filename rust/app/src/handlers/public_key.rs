use std::convert::TryFrom;

use bolos_sys::Error as SysError;

use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS},
    crypto,
    dispatcher::{
        ApduHandler, INS_GET_ADDRESS, INS_LEGACY_GET_PUBLIC_KEY, INS_LEGACY_PROMPT_PUBLIC_KEY,
    },
    sys,
};

pub struct GetAddress;

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
    fn handle(_flags: &mut u32, tx: &mut u32, _rx: u32, buffer: &mut [u8]) -> Result<(), Error> {
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
        let cdata = &buffer[5..cdata_len];

        let bip32_path =
            sys::crypto::bip32::BIP32Path::read(cdata).map_err(|_| Error::DataInvalid)?;

        let key = curve
            .gen_keypair(&bip32_path)
            .map_err(|_| Error::ExecutionError)?
            .public()
            .compress()
            .map_err(|_| Error::ExecutionError)?;

        match action {
            Action::GetPublicAndAddress => {
                let addr = Addr::new(&key).map_err(|_| Error::DataInvalid)?;

                if req_confirmation {
                    //TODO: show(&addr)
                }

                let key = key.as_ref();
                let len = key.len();
                buffer[..len].copy_from_slice(&key);
                *tx = len as u32;

                let addr = addr.to_base58();
                let alen = addr.len();
                buffer[len..len + alen].copy_from_slice(&addr[..]);
                *tx += alen as u32;
            }
            Action::LegacyGetPublic => {
                let key = key.as_ref();
                let len = key.len();
                buffer[..len].copy_from_slice(&key);
                *tx = len as u32;
            }
            Action::LegacyPromptAddressButGetPublic => {
                let addr = Addr::new(&key).map_err(|_| Error::DataInvalid)?;

                //TODO: show(&addr)

                let key = key.as_ref();
                let len = key.len();
                buffer[..len].copy_from_slice(&key);
                *tx = len as u32;
            }
        }

        Ok(())
    }
}

struct Addr {
    prefix: [u8; 3],
    hash: [u8; 20],
    checksum: [u8; 4],
}

impl Addr {
    pub fn new(pubkey: &crypto::PublicKey) -> Result<Self, SysError> {
        use crypto::Curve;
        use sys::hash::{Hasher, Sha256};

        let hash = pubkey.hash()?;

        //legacy/src/to_string.c:135
        let prefix: [u8; 3] = {
            match pubkey.curve() {
                Curve::Ed25519 | Curve::Bip32Ed25519 => [6, 161, 159],
                Curve::Secp256K1 => [6, 161, 161],
                Curve::Secp256R1 => [6, 161, 164],
            }
        };

        //legacy/src/to_string.c:94
        // hash prefix + hash
        let checksum_hash = {
            let mut digest = Sha256::new()?;
            digest.update(&prefix[..])?;
            digest.update(&hash[..])?;
            digest.finalize()?
        };

        //and hash that to get the checksum
        let big_checksum = Sha256::digest(&checksum_hash[..])?;
        let checksum = {
            //but only get the first 4 bytes
            let mut array = [0; 4];
            array.copy_from_slice(&big_checksum[..4]);
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
    pub fn to_base58(&self) -> [u8; 39] {
        let mut input = {
            let mut array = [0; 29];
            array[..3].copy_from_slice(&self.prefix[..]);
            array[3..3 + 20].copy_from_slice(&self.hash[..]);
            array[3 + 20..3 + 20 + 4].copy_from_slice(&self.checksum[..]);
            array
        };

        let mut out = [0; 39];

        //the expect is ok since we know all the sizes
        bs58::encode(input)
            .into(&mut out[..])
            .expect("encoded in base58 is not of the right length");

        out
    }
}
