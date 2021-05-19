use std::convert::TryFrom;

use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS},
    crypto,
    dispatcher::{ApduHandler, INS_GET_ADDRESS},
    sys,
};

pub struct GetAddress;

impl ApduHandler for GetAddress {
    fn handle(_flags: &mut u32, tx: &mut u32, _rx: u32, buffer: &mut [u8]) -> Result<(), Error> {
        *tx = 0;
        if buffer[APDU_INDEX_INS] != INS_GET_ADDRESS {
            return Err(Error::InsNotSupported);
        }

        let req_confirmation = buffer[2] >= 1;
        let curve = crypto::Curve::try_from(buffer[3]).map_err(|_| Error::InvalidP1P2)?;

        let cdata_len = buffer[4] as usize;
        if cdata_len > buffer[5..].len() {
            return Err(Error::DataInvalid);
        }
        let cdata = &buffer[5..cdata_len];

        //read_bip32_path(&mut G.key.bip32_path, buffer[4..], cdata_len)
        let bip32_path =
            sys::crypto::bip32::BIP32Path::read(cdata).map_err(|_| Error::DataInvalid)?;

        let key = curve.gen_keypair(&bip32_path);

        todo!()
    }
}
