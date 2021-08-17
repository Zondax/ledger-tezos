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
    bytes::complete::{take, take_while},
    number::complete::le_u8,
    sequence::tuple,
    IResult,
};

use crate::{crypto::Curve, handlers::parser_common::ParserError};

pub mod operations;

//TODO: determine actual size so we can use other libs and pass this type around more armoniously
// alternative: implement all the necessary traits and handle everything manually...
#[derive(Debug, Clone, Copy)]
pub struct Zarith<'b> {
    bytes: &'b [u8],
    is_negative: Option<bool>,
}

impl<'b> Zarith<'b> {
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn from_bytes(input: &'b [u8], want_sign: bool) -> IResult<&[u8], Self, ParserError> {
        //keep taking bytes while the MSB is 1
        let (rem, bytes) = take_while(|byte| byte & 0x80 != 0)(input)?;

        if bytes.len() < 1 {
            Err(ParserError::parser_unexpected_buffer_end)?;
        }

        let is_negative = if want_sign {
            //if the second bit of the first byte is set, then it's negative
            Some(bytes[0] & 0x40 != 0)
        } else {
            None
        };

        Ok((rem, Self { bytes, is_negative }))
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

#[derive(Debug, Clone, Copy)]
pub enum ContractID<'b> {
    Implicit(Curve, &'b [u8; 20]),
    Originated(&'b [u8; 20]),
}

impl<'b> ContractID<'b> {
    fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, tag) = le_u8(input)?;
        match tag {
            0x00 => {
                let (rem, (crv, hash)) = public_key_hash(rem)?;
                Ok((rem, Self::Implicit(crv, hash)))
            }
            0x01 => {
                //discard last byte (padding)
                let (rem, (hash, _)) = tuple((take(20usize), le_u8))(rem)?;
                let hash = array_ref!(hash, 0, 20);

                Ok((rem, Self::Originated(hash)))
            }
            _ => Err(ParserError::parser_invalid_address)?,
        }
    }
}

fn boolean(input: &[u8]) -> IResult<&[u8], bool, ParserError> {
    let (rem, b) = le_u8(input)?;

    Ok((rem, b == 255))
}
