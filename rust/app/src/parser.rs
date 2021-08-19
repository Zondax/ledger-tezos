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
use arrayref::array_ref;
use nom::{
    bytes::complete::{take, take_till},
    number::complete::le_u8,
    sequence::tuple,
    IResult,
};

use crate::{crypto::Curve, handlers::parser_common::ParserError};

pub mod operations;

//TODO: determine actual size so we can use other libs and pass this type around more armoniously
// alternative: implement all the necessary traits and handle everything manually...
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zarith<'b> {
    bytes: &'b [u8],
    is_negative: Option<bool>,
}

impl<'b> Zarith<'b> {
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    #[cfg(not(test))]
    pub fn from_bytes(input: &'b [u8], want_sign: bool) -> IResult<&[u8], Self, ParserError> {
        //keep taking bytes while the MSB is 1
        let (_, bytes) = take_till(|byte| byte & 0x80 == 0)(input)?;

        //take bytes + 1 since we miss the last byte with `take_till`
        let (rem, bytes) = take(bytes.len() + 1)(input)?;

        let is_negative = if want_sign {
            //if the second bit of the first byte is set, then it's negative
            Some(bytes[0] & 0x40 != 0)
        } else {
            None
        };

        Ok((rem, Self { bytes, is_negative }))
    }

    #[cfg(test)]
    pub fn from_bytes(input: &'b [u8], want_sign: bool) -> IResult<&[u8], Self, ParserError> {
        use std::println;
        use nom::{take, dbg_basic, take_till};

        //keep taking bytes while the MSB is 1
        let (_, bytes) = dbg_basic!(input, take_till!(|byte| byte & 0x80 == 0) )?;

        //take bytes + 1 since we miss the last byte with `take_till`
        let (rem, bytes) = dbg_basic!(input, take!(bytes.len() + 1))?;

        let is_negative = if want_sign {
            //if the second bit of the first byte is set, then it's negative
            Some(bytes[0] & 0x40 != 0)
        } else {
            None
        };

        Ok((rem, Self { bytes, is_negative }))
    }

    pub fn is_negative(&self) -> Option<bool> {
        self.is_negative
    }
}

pub fn public_key_hash(input: &[u8]) -> IResult<&[u8], (Curve, &[u8; 20]), ParserError> {
    let (rem, (crv, hash)) = tuple((le_u8, take(20usize)))(input)?;

    let crv = match crv {
        0x00 => Curve::Bip32Ed25519,
        0x01 => Curve::Secp256K1,
        0x02 => Curve::Secp256R1,
        _ => Err(ParserError::parser_invalid_pubkey_encoding)?,
    };

    let out = array_ref!(hash, 0, 20);

    Ok((rem, (crv, out)))
}

pub fn public_key(input: &[u8]) -> IResult<&[u8], (Curve, &[u8]), ParserError> {
    let (rem, crv) = le_u8(input)?;

    let (crv, take_pk) = match crv {
        0x00 => (Curve::Bip32Ed25519, take(32usize)),
        0x01 => (Curve::Secp256K1, take(33usize)),
        0x02 => (Curve::Secp256R1, take(33usize)),
        _ => Err(ParserError::parser_invalid_pubkey_encoding)?,
    };

    let (rem, pk) = take_pk(rem)?;

    Ok((rem, (crv, pk)))
}


fn boolean(input: &[u8]) -> IResult<&[u8], bool, ParserError> {
    let (rem, b) = le_u8(input)?;

    Ok((rem, b == 255))
}

#[cfg(test)]
mod tests {
    use crate::{crypto::Curve, handlers::public_key::Addr, parser::{boolean, public_key, public_key_hash}};

    use super::Zarith;

    #[test]
    fn pkh_ed() {
        const PKH_BASE58: &str = "tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn";
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, (crv, hash)) = public_key_hash(&input).expect("failed to parse input");

        assert_eq!(rem.len(), 0);
        assert_eq!(crv, Curve::Bip32Ed25519);

        let addr = Addr::from_hash(hash, crv).unwrap();
        assert_eq!(&addr.to_base58()[..], PKH_BASE58.as_bytes());
    }

    #[test]
    #[should_panic(expected = "failed to parse pkh input")]
    fn pkh_ed_eof() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a333";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        public_key_hash(&input).expect("failed to parse pkh input");
    }

    #[test]
    fn pk_ed() {
        const INPUT_HEX: &str = "00ebcf82872f4942052704e95dc4bfa0538503dbece27414a39b6650bcecbff896";
        const PK_BASE58: &str = "edpkvS5QFv7KRGfa3b87gg9DBpxSm3NpSwnjhUjNBQrRUUR66F7C9g";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, (crv, pk)) = public_key(&input).expect("failed to parse input");

        assert_eq!(rem.len(), 0);
        assert_eq!(crv, Curve::Bip32Ed25519);

        let mut vpk = crate::constants::tzprefix::EDPK.to_vec();
        vpk.extend_from_slice(pk);

        let pk = bs58::encode(vpk).with_check().into_string();
        assert_eq!(pk.as_str(), PK_BASE58);
    }

    #[test]
    #[should_panic(expected = "failed to parse pk input")]
    fn pk_ed_eof() {
        const INPUT_HEX: &str = "00ebcf82872f4942052704e95dc4bfa0538503dbece27414a39b";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        public_key(&input).expect("failed to parse pk input");
    }

    #[test]
    fn parse_boolean() {
        assert_eq!(true, boolean(&[255]).expect("invalid input").1);
        assert_eq!(false, boolean(&[0]).expect("invalid input").1);
        assert_eq!(false, boolean(&[123]).expect("invalid input").1);
    }

    #[test]
    fn zarith() {
        //should ignore last byte
        let end_early = &[0b1000_0001, 0x11, 0x33][..];

        let (_, num) = Zarith::from_bytes(end_early, false).expect("invalid input");
        assert_eq!(num.len(), 2);
        assert_eq!(num.is_negative(), None);

        //should get a single byte
        let single_byte = &[0x0a][..];

        let (_, num) = Zarith::from_bytes(single_byte, false).expect("invalid input");
        assert_eq!(num.len(), 1);
        assert_eq!(num.is_negative(), None);

        //should get a bunch of bytes
        let multi_byte = &[0x8a, 0x90, 0xf2, 0xe4, 0x88, 0x00][..];

        let (_, num) = Zarith::from_bytes(multi_byte, false).expect("invalid input");
        assert_eq!(num.len(), 6);
        assert_eq!(num.is_negative(), None);

        //should be considered negative
        let negative = &[0b1100_0011, 0x23][..];

        let (_, num) = Zarith::from_bytes(negative, true).expect("invalid input");
        assert_eq!(num.len(), 2);
        assert_eq!(num.is_negative(), Some(true));

        //should be considered positive
        let positive = &[0b1000_0011, 0x23][..];

        let (_, num) = Zarith::from_bytes(positive, true).expect("invalid input");
        assert_eq!(num.len(), 2);
        assert_eq!(num.is_negative(), Some(false))
    }

}
