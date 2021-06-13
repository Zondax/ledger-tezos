use std::convert::TryFrom;

use bolos::{
    crypto::bip32::BIP32Path,
    hash::{Blake2b, Hasher},
};

use super::{resources::BUFFER, PacketType};
use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS, BIP32_MAX_LENGTH},
    crypto::Curve,
    dispatcher::{ApduHandler, INS_LEGACY_SIGN, INS_LEGACY_SIGN_WITH_HASH, INS_SIGN},
    sys,
};

#[bolos::lazy_static]
static mut PATH: Option<(BIP32Path<BIP32_MAX_LENGTH>, Curve)> = None;

pub struct Sign;

impl Sign {
    pub const SIGN_HASH_SIZE: usize = 32;

    fn get_derivation_info() -> Result<&'static (BIP32Path<BIP32_MAX_LENGTH>, Curve), Error> {
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
                let path =
                    BIP32Path::<BIP32_MAX_LENGTH>::read(cdata).map_err(|_| Error::DataInvalid)?;

                unsafe { BUFFER.lock(Self)?.reset() };
                unsafe { PATH.replace((path, curve)) };
            }
            PacketType::Add => {
                //this is pure data

                //check if we initialized first
                Self::get_derivation_info()?;

                unsafe { BUFFER.acquire(Self)?.write(cdata) }.map_err(|_| Error::DataInvalid)?;
            }
            PacketType::Last => {
                //this is also pure data, but we need to return data this time!

                let (path, curve) = Self::get_derivation_info()?;

                let mut zbuffer = unsafe { BUFFER.acquire(Self)? };
                zbuffer.write(cdata).map_err(|_| Error::DataInvalid)?;

                let unsigned_hash = Self::blake2b_digest(zbuffer.read_exact())?;

                let (sig_size, sig) = Self::sign(*curve, path, &unsigned_hash[..])?;

                //write unsigned_hash to buffer
                tx += Self::SIGN_HASH_SIZE as u32;
                buffer[0..Self::SIGN_HASH_SIZE].copy_from_slice(&unsigned_hash[..]);

                //wrte signature to buffer
                tx += sig_size as u32;
                buffer[Self::SIGN_HASH_SIZE..Self::SIGN_HASH_SIZE + sig_size]
                    .copy_from_slice(&sig[..sig_size]);

                //reset globals to avoid skipping `Init`
                zbuffer.reset();
                unsafe { BUFFER.release(Self)? };
                unsafe { PATH.take() };
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
            Action::LegacySign => return Err(Error::CommandNotAllowed), //TODO
            Action::LegacySignWithHash => return Err(Error::CommandNotAllowed), //TODO
            #[cfg(feature = "wallet")]
            Action::LegacySignUnsafe => return Err(Error::CommandNotAllowed), //TODO
        };

        Ok(())
    }
}
