use std::convert::TryFrom;

use bolos::{
    crypto::bip32::BIP32Path,
    hash::{Blake2b, Hasher, HasherId},
    new_swapping_buffer, SwappingBuffer,
};

use super::PacketType;
use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS},
    crypto::{self, Curve},
    dispatcher::{ApduHandler, INS_LEGACY_SIGN, INS_LEGACY_SIGN_WITH_HASH, INS_SIGN},
    sys::{self, Error as SysError},
};

#[bolos::lazy_static]
static mut PATH: Option<(BIP32Path<6>, Curve)> = None;

#[bolos::lazy_static]
static mut BUFFER: SwappingBuffer<'static, 'static, 0xFF, 0xFFFF> =
    new_swapping_buffer!(0xFF, 0xFFFF);

pub struct Sign;

impl Sign {
    pub const SIGN_HASH_SIZE: usize = 32;

    fn get_derivation_info() -> Result<&'static (BIP32Path<6>, Curve), Error> {
        match unsafe { &*PATH } {
            None => Err(Error::ApduCodeConditionsNotSatisfied),
            Some(some) => Ok(some),
        }
    }

    //(actual_size, [u8; MAX_SIGNATURE_SIZE])
    #[inline(never)]
    fn sign<const LEN: usize>(
        curve: Curve,
        path: &BIP32Path<LEN>,
        data: &[u8],
    ) -> Result<(usize, [u8; 100]), Error> {
        let mut keypair = curve.gen_keypair(path).map_err(|_| Error::ExecutionError)?;

        let mut out = [0; 100];
        let sz = keypair
            .sign(data, &mut out[..])
            .map_err(|_| Error::ExecutionError)?;

        Ok((sz, out))
    }

    #[inline(never)]
    fn blake2b_digest(buffer: &[u8]) -> Result<[u8; Self::SIGN_HASH_SIZE], Error> {
        Blake2b::digest(buffer).map_err(|_| Error::ExecutionError)
    }

    #[inline(never)]
    pub fn blind_sign(packet_type: PacketType, buffer: &mut [u8]) -> Result<u32, Error> {
        let mut tx = 0;
        let cdata_len = buffer[4] as usize;
        let cdata = &buffer[5..5 + cdata_len];

        match packet_type {
            PacketType::Init => {
                //first packet contains the curve data on the second parameter
                // and the bip32 path as payload only

                let curve = Curve::try_from(buffer[3]).map_err(|_| Error::InvalidP1P2)?;
                let path = BIP32Path::<6>::read(cdata).map_err(|_| Error::DataInvalid)?;

                unsafe { BUFFER.reset() };
                unsafe { PATH.replace((path, curve)) };
            }
            PacketType::Add => {
                //this is pure data

                //check if we initialized first
                Self::get_derivation_info()?;

                unsafe { BUFFER.write(cdata) }.map_err(|_| Error::DataInvalid)?;
            }
            PacketType::Last => {
                //this is also pure data, but we need to return data this time!

                let (path, curve) = Self::get_derivation_info()?;

                unsafe { BUFFER.write(cdata) }.map_err(|_| Error::DataInvalid)?;

                let unsigned_hash = Self::blake2b_digest(unsafe { BUFFER.read_exact() })?;

                let (sig_size, sig) =
                    Self::sign(*curve, path, &unsigned_hash[..])?;

                //write unsigned_hash to buffer
                tx += Self::SIGN_HASH_SIZE as u32;
                buffer[0..Self::SIGN_HASH_SIZE].copy_from_slice(&unsigned_hash[..]);

                //wrte signature to buffer
                tx += sig_size as u32;
                buffer[Self::SIGN_HASH_SIZE..Self::SIGN_HASH_SIZE + sig_size]
                    .copy_from_slice(&sig[..sig_size]);

                //reset globals to avoid skipping `Init`
                unsafe { PATH.take() };
                unsafe { BUFFER.reset() };
            }
        }

        Ok(tx)
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    //NEW API: TODO
    Sign,

    //LEGACY API: TODO
    LegacySign,

    //LEGACY API: TODO
    LegacySignWithHash,

    #[cfg(feature = "wallet")]
    //LEGACY API: TODO
    LegacySignUnsafe,
}

impl ApduHandler for Sign {
    #[inline(never)]
    fn handle(_flags: &mut u32, tx: &mut u32, _rx: u32, buffer: &mut [u8]) -> Result<(), Error> {
        sys::zemu_log_stack("Sign::handle\x00");

        *tx = 0;
        let action = match buffer[APDU_INDEX_INS] {
            INS_SIGN => Action::Sign,
            INS_LEGACY_SIGN => Action::LegacySign,
            INS_LEGACY_SIGN_WITH_HASH => Action::LegacySignWithHash,
            #[cfg(feature = "wallet")]
            crate::dispatcher::INS_LEGACY_SIGN_UNSAFE => Action::LegacySignUnsafe,
            _ => return Err(Error::InsNotSupported),
        };

        let packet_type = PacketType::try_from(buffer[2]).map_err(|_| Error::InvalidP1P2)?;

        *tx = match action {
            Action::Sign => {
                Self::blind_sign(packet_type, buffer).map_err(|_| Error::ExecutionError)?
            }
            Action::LegacySign => todo!(),
            Action::LegacySignWithHash => todo!(),
            #[cfg(feature = "wallet")]
            Action::LegacySignUnsafe => todo!(),
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
        use sys::hash::Sha256;
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
