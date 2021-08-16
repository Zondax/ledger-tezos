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

use nom::{
    bytes::complete::{take_until, take_while},
    IResult,
};

use crate::{crypto::Curve, handlers::parser_common::ParserError};

//TODO: determine actual size so we can use other libs and pass this type around more armoniously
// alternative: implement all the necessary traits and handle everything manually...
pub struct Zarith<'b> {
    bytes: &'b [u8],
    is_negative: Option<bool>,
}

fn zarith(input: &[u8], want_sign: bool) -> IResult<&[u8], Zarith<'_>, ParserError> {
    //keep taking bytes while the MSB is 1
    let (rem, bytes) = take_while(|byte| byte & 0x80 != 0)(input)?;

    if bytes.len() < 1 {
        return Err(ParserError::parser_unexpected_buffer_end);
    }

    let is_negative = if want_sign {
        //if the second bit of the first byte is set, then it's negative
        Some(bytes[0] & 0x40 != 0)
    } else {
        None
    };

    Ok((rem, Self { bytes, is_negative }))
}

impl<'b> Zarith<'b> {
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}

fn public_key_hash(input: &[u8]) -> IResult<&[u8], (Curve, [u8; 20]), ParserError> {
    let (rem, crv) = take(1)(input)?;

    let crv = match crv {
        0x00 => Curve::Bip32Ed25519,
        0x01 => Curve::Secp256K1,
        0x02 => Curve::Secp256R1,
        _ => return Err(ParserError::parser_invalid_pubkey_encoding),
    };

    let mut out = [0; 20];

    let (rem, hash) = take(20)(rem)?;
    out.copy_from_slice(hash);

    Ok((rem, (crv, out)))
}

fn public_key(input: &[u8]) -> IResult<&[u8], (Curve, &[u8]), ParserError> {
    let (rem, crv) = take(1)(input)?;

    let (crv, (rem, pk)) = match crv {
        0x00 => (Curve::Bip32Ed25519, take(32)(rem)?),
        0x01 => (Curve::Secp256K1, take(33)(rem)?),
        0x02 => (Curve::Secp256R1, take(33)(rem)?),
        _ => return Err(ParserError::parser_invalid_pubkey_encoding),
    };

    Ok((rem, (crv, pk)))
}

pub enum ContractID {
    Implicit(Curve, [u8; 20]),
    Originated([u8; 20])
}

fn contract_id(input: &[u8]) -> IResult<&[u8], ContractID, ParserError> {
    let (rem, tag) = take(1)(input)?;
    match tag {
        0x00 => {
            Ok((rem, Self::Implicit(public_key_hash(rem)?)))
        },
        0x01 => {
            let mut out = [0; 20];
            let (rem, hash) = take(20)(rem)?;

            let (rem, _) = take(1)(rem)?; //skip padding
            out.copy_from_slice(hash);

            Ok((rem, Self::Originated(out)))
        }
    }
}

fn bool(input: &[u8]) -> IResult<&[u8], bool, ParseError> {
    let (rem, b) = take(1)(input);

    Ok((rem, b == 255))
}
