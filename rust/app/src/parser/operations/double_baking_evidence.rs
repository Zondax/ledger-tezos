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
use crate::{handlers::parser_common::ParserError, parser::boolean};
use arrayref::array_ref;
use nom::{
    bytes::complete::take,
    number::complete::{be_i32, be_i64, be_u16, be_u32, be_u8},
    IResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Fitness<'b> {
    fitness: &'b [u8],
}

impl<'b> Fitness<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, length) = be_u32(input)?;
        let (rem, fitness) = take(length)(rem)?;

        Ok((rem, Self { fitness }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
pub struct Fitnesses<'b> {
    source: &'b [u8],
    //number of bytes read
    read: usize,
}

impl<'b> Fitnesses<'b> {
    pub fn new(source: &'b [u8]) -> Self {
        Self { source, read: 0 }
    }

    pub fn count(&self) -> Result<usize, nom::Err<ParserError>> {
        let mut count = 0;
        let mut this = *self;

        while this.parse_next()?.is_some() {
            count += 1;
        }

        Ok(count)
    }

    #[inline(never)]
    fn parse(&self) -> Result<Option<(Fitness<'b>, usize)>, nom::Err<ParserError>> {
        let input = &self.source[self.read..];
        let input_len = input.len();

        if input_len == 0 {
            return Ok(None);
        }

        let (rem, data) = Fitness::from_bytes(input)?;

        //calculate the number of bytes read based
        // on the number of bytes left in the remaning section
        //this will also take into account the bytes removed earlier
        // to skip already read bytes
        let read = self.source.len() - rem.len();

        Ok(Some((data, read)))
    }

    pub fn peek_next(&self) -> Result<Option<Fitness<'b>>, nom::Err<ParserError>> {
        match self.parse() {
            Ok(Some((data, _))) => Ok(Some(data)),
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn parse_next(&mut self) -> Result<Option<Fitness<'b>>, nom::Err<ParserError>> {
        match self.parse() {
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
    ///
    /// # Safety
    /// If the specified "read" argument is a byte in the middle of an operation
    /// it will make further reading impossible
    pub unsafe fn set_source_index(&mut self, read: usize) {
        self.read = read;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct FullBlockHeader<'b> {
    level: i32,
    proto: u8,
    predecessor: &'b [u8; 32],
    timestamp: i64,
    validation_pass: u8,
    operation_hash: &'b [u8; 32],
    num_fitnesses: usize,
    fitnesses: Fitnesses<'b>,
    context: &'b [u8; 32],
    priority: u16,
    proof_of_work_nonce: &'b [u8; 8],
    seed_nonce_hash: Option<&'b [u8; 32]>,
    liquidity_baking_escape_vote: bool,
    signature: &'b [u8; 64],
}

impl<'b> FullBlockHeader<'b> {
    #[inline(never)]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, level) = be_i32(input)?;

        let (rem, proto) = be_u8(rem)?;

        let (rem, predecessor) = {
            let (rem, predecessor) = take(32usize)(rem)?;
            (rem, array_ref!(predecessor, 0, 32))
        };

        let (rem, timestamp) = be_i64(rem)?;

        let (rem, validation_pass) = be_u8(rem)?;

        let (rem, operation_hash) = {
            let (rem, operation_hash) = take(32usize)(rem)?;
            (rem, array_ref!(operation_hash, 0, 32))
        };

        let (rem, num_fitnesses, fitnesses) = {
            let (rem, length) = be_u32(rem)?;
            let length = length as usize;
            if length > rem.len() {
                return Err(ParserError::parser_value_out_of_range.into());
            }

            //get the fitnesses slice
            // and the remaing data slice
            let (fitnesses, rem) = rem.split_at(length);
            let fitnesses = Fitnesses::new(fitnesses);

            (rem, fitnesses.count()?, fitnesses)
        };

        let (rem, context) = {
            let (rem, context) = take(32usize)(rem)?;

            (rem, array_ref!(context, 0, 32))
        };

        let (rem, priority) = be_u16(rem)?;

        let (rem, proof_of_work_nonce) = {
            let (rem, proof_of_work_nonce) = take(8usize)(rem)?;

            (rem, array_ref!(proof_of_work_nonce, 0, 8))
        };

        let (rem, seed_nonce_hash) = {
            use nom::{cond, take};
            let (rem, is_present) = boolean(rem)?;

            let (rem, seed_nonce_hash) = cond!(rem, is_present, take!(32))?;

            (rem, seed_nonce_hash.map(|s| array_ref!(s, 0, 32)))
        };

        let (rem, liquidity_baking_escape_vote) = boolean(rem)?;

        let (rem, signature) = {
            let (rem, signature) = take(64usize)(rem)?;

            (rem, array_ref!(signature, 0, 64))
        };

        Ok((
            rem,
            Self {
                level,
                proto,
                predecessor,
                timestamp,
                validation_pass,
                operation_hash,
                num_fitnesses,
                fitnesses,
                context,
                priority,
                proof_of_work_nonce,
                seed_nonce_hash,
                liquidity_baking_escape_vote,
                signature,
            },
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct DoubleBakingEvidence<'b> {
    first_header: FullBlockHeader<'b>,
    second_header: FullBlockHeader<'b>,
}

impl<'b> DoubleBakingEvidence<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, first_header) = FullBlockHeader::from_bytes(input)?;
        let (rem, second_header) = FullBlockHeader::from_bytes(rem)?;

        Ok((
            rem,
            Self {
                first_header,
                second_header,
            },
        ))
    }
}

#[cfg(test)]
impl<'b> DoubleBakingEvidence<'b> {
    pub fn is(&self, _json: &serde_json::Map<std::string::String, serde_json::Value>) {
        //TODO
    }
}
