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
    bytes::complete::take,
    number::complete::{le_u32, le_u8},
    Finish, IResult,
};

use crate::{crypto::Curve, handlers::parser_common::ParserError};

use super::{bool, public_key_hash, ContractID, Zarith};

#[derive(Debug, Clone, Copy)]
pub struct Operation<'b> {
    branch: &'b [u8; 32],
    contents: EncodedOperations<'b>,
}

impl<'b> Operation<'b> {
    pub fn new(input: &'b [u8]) -> Result<Self, ParserError> {
        let (rem, branch) = take::<_, _, ParserError>(32usize)(input).finish()?;
        let branch = arrayref::array_ref!(branch, 0, 32);

        Ok(Self {
            branch,
            contents: EncodedOperations::new(rem),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EncodedOperations<'b> {
    source: &'b [u8],
    read: usize,
}

impl<'b> EncodedOperations<'b> {
    pub fn new(source: &'b [u8]) -> Self {
        Self { source, read: 0 }
    }

    fn parse(&self) -> Result<Option<(OperationType<'b>, usize)>, nom::Err<ParserError>> {
        let input = &self.source[self.read..];
        let input_len = input.len();

        if input_len == 0 {
            return Ok(None);
        }

        let (rem, data) = match OperationType::from_bytes(input) {
            Ok(ok) => ok,
            //there was some remaing data but it's probably the signature
            // since we don't recognize the operation tag
            Err(nom::Err::Error(ParserError::UnknownOperation)) if input_len == 64 => {
                return Ok(None)
            }
            Err(err) => return Err(err),
        };

        //calculate the number of bytes read based
        // on the number of bytes left in the remaning section
        //this will also take into account the bytes removed earlier
        // to skip already read bytes
        let read = self.source.len() - rem.len();

        Ok(Some((data, read)))
    }

    pub fn peek_next(&self) -> Result<Option<OperationType<'b>>, nom::Err<ParserError>> {
        match self.parse() {
            Ok(Some((data, _))) => Ok(Some(data)),
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn parse_next(&mut self) -> Result<Option<OperationType<'b>>, nom::Err<ParserError>> {
        match self.parse(){
            Ok(None) => Ok(None),
            Err(err) => Err(err),
            Ok(Some((data, read))) => {
                self.read = read;
                Ok(Some(data))
            }
        }
    }

    pub fn source_index(&self) -> usize {
        self.read
    }

    /// Sets the inner index to the specified one.
    /// This is unsafe because it could make further reading impossible
    pub unsafe fn set_source_index(&mut self, read: usize) {
        self.read = read;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OperationType<'b> {
    Transfer(Transfer<'b>),
}

impl<'b> OperationType<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, tag) = le_u8(input)?;

        let (rem, data) = match tag {
            0x00 => todo!("endorsement"),
            0x01 => todo!("seed nonce revelation"),
            0x02 => todo!("double endorsement evidence"),
            0x03 => todo!("double baking evidence"),
            0x04 => todo!("activate account"),
            0x05 => todo!("proposalas"),
            0x06 => todo!("ballot"),
            0x0A => todo!("endorsement with slot"),
            0x11 => todo!("failing noop"),
            0x6B => todo!("reveal"),
            0x6C => {
                let (rem, data) = Transfer::from_bytes(rem)?;
                (rem, Self::Transfer(data))
            }
            0x6D => todo!("origination"),
            0x6E => todo!("delegation"),
            _ => Err(ParserError::UnknownOperation)?,
        };

        Ok((rem, data))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Entrypoint<'b> {
    Default,
    Root,
    Do,
    SetDelegate,
    RemoveDelegate,
    Custom(&'b [u8]),
}

impl<'b> Entrypoint<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (mut rem, tag) = le_u8(input)?;

        let data = match tag {
            0x00 => Self::Default,
            0x01 => Self::Root,
            0x02 => Self::Do,
            0x03 => Self::SetDelegate,
            0x04 => Self::RemoveDelegate,
            0xFF => {
                let (rem2, length) = le_u8(rem)?;
                let (rem2, name) = take(length)(rem2)?;
                rem = rem2;

                Self::Custom(name)
            }
            _ => Err(ParserError::parser_invalid_contract_name)?,
        };

        Ok((rem, data))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Parameters<'b> {
    entrypoint: Entrypoint<'b>,
    michelson: &'b [u8],
}

impl<'b> Parameters<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, entrypoint) = Entrypoint::from_bytes(input)?;

        let (rem, length) = le_u32(rem)?;
        let (rem, michelson) = take(length as usize)(rem)?;

        Ok((
            rem,
            Self {
                entrypoint,
                michelson,
            },
        ))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Transfer<'b> {
    source: (Curve, &'b [u8; 20]),
    fee: Zarith<'b>,
    counter: Zarith<'b>,
    gas_limit: Zarith<'b>,
    storage_limit: Zarith<'b>,
    amount: Zarith<'b>,
    destination: ContractID<'b>,
    parameters: Option<Parameters<'b>>,
}

impl<'b> Transfer<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, source) = public_key_hash(input)?;
        let (rem, fee) = Zarith::from_bytes(rem, false)?;
        let (rem, counter) = Zarith::from_bytes(rem, false)?;
        let (rem, gas_limit) = Zarith::from_bytes(rem, false)?;
        let (rem, storage_limit) = Zarith::from_bytes(rem, false)?;
        let (rem, amount) = Zarith::from_bytes(rem, false)?;
        let (rem, destination) = ContractID::from_bytes(rem)?;

        let (rem, has_params) = bool(rem)?;
        let (rem, params) = if has_params {
            let (rem, params) = Parameters::from_bytes(rem)?;
            (rem, Some(params))
        } else {
            (rem, None)
        };

        Ok((
            rem,
            Self {
                source,
                fee,
                counter,
                gas_limit,
                storage_limit,
                amount,
                destination,
                parameters: params,
            },
        ))
    }
}
