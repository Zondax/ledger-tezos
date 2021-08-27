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
    call, cond, do_parse,
    number::complete::{be_u32, le_u8},
    take, Finish, IResult,
};

use crate::{
    constants::tzprefix::{B, KT1, TZ1, TZ2, TZ3},
    crypto::Curve,
    handlers::{parser_common::ParserError, sha256x2},
};

use super::{boolean, public_key_hash, Zarith};

#[derive(Debug, Clone, Copy, property::Property)]
#[property(get(public), mut(public), set(disable))]
pub struct Operation<'b> {
    #[property(mut(disable))]
    branch: &'b [u8; 32],

    ops: EncodedOperations<'b>,
}

impl<'b> Operation<'b> {
    pub fn new(input: &'b [u8]) -> Result<Self, ParserError> {
        let (rem, branch) = take::<_, _, ParserError>(32usize)(input).finish()?;
        let branch = arrayref::array_ref!(branch, 0, 32);

        Ok(Self {
            branch,
            ops: EncodedOperations::new(rem),
        })
    }

    pub fn base58_branch(&self, out: &mut [u8; 51]) -> Result<(), bolos::Error> {
        let mut checksum = [0; 4];

        sha256x2(&[B, &self.branch[..]], &mut checksum)?;

        let input = {
            let mut array = [0; 2 + 32 + 4];
            array[..2].copy_from_slice(B);
            array[2..2 + 32].copy_from_slice(&self.branch[..]);
            array[2 + 32..].copy_from_slice(&checksum[..]);
            array
        };

        bs58::encode(input)
            .into(&mut out[..])
            .expect("encoded in base58 is not of the right length");

        Ok(())
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
            _ => return Err(ParserError::UnknownOperation.into()),
        };

        Ok((rem, data))
    }

    pub fn is_transfer(&self) -> bool {
        matches!(self, OperationType::Transfer(_))
    }

    pub fn ui_items(&self) -> usize {
        match self {
            OperationType::Transfer(_tx) => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
                let (rem2, name) = take!(rem2, length)?;
                rem = rem2;

                Self::Custom(name)
            }
            _ => return Err(ParserError::parser_invalid_contract_name.into()),
        };

        Ok((rem, data))
    }
}

impl<'b> core::fmt::Display for Entrypoint<'b> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Entrypoint::Default => write!(f, "default"),
            Entrypoint::Root => write!(f, "root"),
            Entrypoint::Do => write!(f, "do"),
            Entrypoint::SetDelegate => write!(f, "set_delegate"),
            Entrypoint::RemoveDelegate => write!(f, "remove_delegate"),
            Entrypoint::Custom(custom) => {
                let custom = core::str::from_utf8(custom).expect("custom entrypoint was not utf8");
                f.write_str(custom)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Parameters<'b> {
    entrypoint: Entrypoint<'b>,
    michelson: &'b [u8],
}

impl<'b> Parameters<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        #[rustfmt::skip]
        let (rem, (entrypoint, michelson)) =
            do_parse!(input,
                entrypoint: call!(Entrypoint::from_bytes) >>
                length: be_u32 >>
                out: take!(length) >>
                (entrypoint, out)
            )?;

        Ok((
            rem,
            Self {
                entrypoint,
                michelson,
            },
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractID<'b> {
    Implicit(Curve, &'b [u8; 20]),
    Originated(&'b [u8; 20]),
}

impl<'b> ContractID<'b> {
    #[cfg(test)]
    fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        use nom::{dbg_basic, tuple as tuplem};
        use std::{eprintln, println};

        let (rem, tag) = dbg_basic!(input, le_u8)?;
        match tag {
            0x00 => {
                let (rem, (crv, hash)) = public_key_hash(rem)?;
                Ok((rem, Self::Implicit(crv, hash)))
            }
            0x01 => {
                //discard last byte (padding)
                let (rem, (hash, _)) = dbg_basic!(rem, tuplem!(take!(20usize), le_u8))?;
                let hash = arrayref::array_ref!(hash, 0, 20);
                Ok((rem, Self::Originated(hash)))
            }
            err => {
                eprintln!(
                    "found {:x} at {}; {:x?}",
                    err,
                    input.len() - rem.len(),
                    input
                );
                Err(ParserError::parser_invalid_address.into())
            }
        }
    }

    #[cfg(not(test))]
    fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, tag) = le_u8(input)?;
        match tag {
            0x00 => {
                let (rem, (crv, hash)) = public_key_hash(rem)?;
                Ok((rem, Self::Implicit(crv, hash)))
            }
            0x01 => {
                //discard last byte (padding)
                let (rem, (hash, _)) = nom::sequence::tuple((take(20usize), le_u8))(rem)?;
                let hash = arrayref::array_ref!(hash, 0, 20);
                Ok((rem, Self::Originated(hash)))
            }
            _ => Err(ParserError::parser_invalid_address.into()),
        }
    }

    pub fn hash(&self) -> &[u8; 20] {
        match self {
            ContractID::Implicit(_, h) | ContractID::Originated(h) => h,
        }
    }

    pub fn base58(&self, out: &mut [u8; 36]) -> Result<(), bolos::Error> {
        let (prefix, hash) = match *self {
            Self::Originated(h) => (KT1, h),
            Self::Implicit(Curve::Bip32Ed25519 | Curve::Ed25519, h) => (TZ1, h),
            Self::Implicit(Curve::Secp256K1, h) => (TZ2, h),
            Self::Implicit(Curve::Secp256R1, h) => (TZ3, h),
        };

        let mut checksum = [0; 4];
        sha256x2(&[prefix, &hash[..]], &mut checksum)?;

        let input = {
            let mut array = [0; 3 + 20 + 4];
            array[..3].copy_from_slice(prefix);
            array[3..3 + 20].copy_from_slice(&hash[..]);
            array[3 + 20..].copy_from_slice(&checksum[..]);
            array
        };

        bs58::encode(input)
            .into(&mut out[..])
            .expect("encoded in base58 is not the right length");

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
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
    #[cfg(not(test))]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        #[rustfmt::skip]
        let (rem, (source, fee, counter, gas_limit, storage_limit, amount, destination, parameters)) =
            do_parse! {input,
                source: public_key_hash >>
                fee: call!(Zarith::from_bytes, false) >>
                counter: call!(Zarith::from_bytes, false) >>
                gas_limit: call!(Zarith::from_bytes, false) >>
                storage_limit: call!(Zarith::from_bytes, false) >>
                amount: call!(Zarith::from_bytes, false) >>
                destination: call!(ContractID::from_bytes) >>
                has_params: boolean >>
                params: cond!(has_params, Parameters::from_bytes) >>
                (source, fee, counter, gas_limit, storage_limit, amount, destination, params)
            }?;

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
                parameters,
            },
        ))
    }

    #[cfg(test)]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        use nom::dbg_basic;
        use std::println;

        #[rustfmt::skip]
        let (rem, (source, fee, counter, gas_limit, storage_limit, amount, destination, parameters)) =
            dbg_basic! {input,
                do_parse!(
                source: public_key_hash >>
                fee: call!(Zarith::from_bytes, false) >>
                counter: call!(Zarith::from_bytes, false) >>
                gas_limit: call!(Zarith::from_bytes, false) >>
                storage_limit: call!(Zarith::from_bytes, false) >>
                amount: call!(Zarith::from_bytes, false) >>
                destination: call!(ContractID::from_bytes) >>
                has_params: boolean >>
                params: cond!(has_params, Parameters::from_bytes) >>
                (source, fee, counter, gas_limit, storage_limit, amount, destination, params)
            )}?;

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
                parameters,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        crypto::Curve,
        handlers::public_key::Addr,
        parser::{
            operations::{ContractID, Operation, OperationType, Parameters, Transfer},
            Zarith,
        },
    };

    use super::Entrypoint;

    #[test]
    fn contract_id_pkh() {
        const INPUT_HEX: &str = "000035e993d8c7aaa42b5e3ccd86a33390ececc73abd";
        const PKH_BASE58: &str = "tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) =
            ContractID::from_bytes(&input).expect("failed to parse contract id input");

        assert_eq!(rem.len(), 0);
        assert_eq!(
            parsed,
            ContractID::Implicit(Curve::Bip32Ed25519, arrayref::array_ref!(input, 2, 20))
        );

        let addr = Addr::from_hash(parsed.hash(), Curve::Bip32Ed25519).unwrap();
        assert_eq!(&addr.to_base58()[..], PKH_BASE58.as_bytes());
    }

    #[test]
    fn contract_id_contract() {
        const INPUT_HEX: &str = "016a7d4a43f51be0934a441fba4f13f9beaa47575100";
        const CONTRACT_BASE58: &str = "KT1JHqHQdHSgWBKo6H4UfG8dw3JnZSyjGkHA";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) =
            ContractID::from_bytes(&input).expect("failed to parse contract id input");

        assert_eq!(rem.len(), 0);
        assert_eq!(
            parsed,
            ContractID::Originated(arrayref::array_ref!(input, 1, 20))
        );

        let mut cid = crate::constants::tzprefix::KT1.to_vec();
        cid.extend_from_slice(&parsed.hash()[..]);

        let cid = bs58::encode(cid).with_check().into_string();
        assert_eq!(cid.as_str(), CONTRACT_BASE58);
    }

    #[test]
    fn entrypoint() {
        let (_, default) =
            Entrypoint::from_bytes(&[0]).expect("failed to parse default entrypoint");
        assert_eq!(default, Entrypoint::Default);

        let (_, root) = Entrypoint::from_bytes(&[1]).expect("failed to parse root entrypoint");
        assert_eq!(root, Entrypoint::Root);

        let (_, do_entrypoint) =
            Entrypoint::from_bytes(&[2]).expect("failed to parse do entrypoint");
        assert_eq!(do_entrypoint, Entrypoint::Do);

        let (_, set_delegate) =
            Entrypoint::from_bytes(&[3]).expect("failed to parse set_delegate entrypoint");
        assert_eq!(set_delegate, Entrypoint::SetDelegate);

        let (_, remove_delegate) =
            Entrypoint::from_bytes(&[4]).expect("failed to parse remove_delegate entrypoint");
        assert_eq!(remove_delegate, Entrypoint::RemoveDelegate);

        let (rem, custom) = Entrypoint::from_bytes(&[0xFF, 0x03, 0x61, 0x62, 0x63, 0xaa])
            .expect("failed to parse custom entrypoint");
        assert_eq!(rem.len(), 1);
        assert_eq!(custom, Entrypoint::Custom(b"abc"));
    }

    #[test]
    #[should_panic(expected = "Incomplete(Size(8))")]
    fn entrypoint_eof() {
        Entrypoint::from_bytes(&[0xFF, 10, 0x61, 0x62]).expect("failed to parse custom entrypoint");
    }

    mod parameters {
        use crate::parser::operations::{Entrypoint, Parameters};

        #[test]
        fn manual() {
            const MICHELSON_CODE: &[u8] = &[0xab, 0xcd];

            let mut input = std::vec![0];
            input.extend_from_slice(&(MICHELSON_CODE.len() as u32).to_be_bytes()[..]);
            input.extend_from_slice(MICHELSON_CODE);
            input.push(0xAA); //dummy byte to verify length

            let (rem, parameters) =
                Parameters::from_bytes(&input).expect("faled to parse parameters");

            assert_eq!(rem.len(), 1);
            assert_eq!(
                parameters,
                Parameters {
                    entrypoint: Entrypoint::Default,
                    michelson: &[0xab, 0xcd]
                }
            )
        }

        #[test]
        fn simple() {
            const INPUT_HEX: &str = "02000000070a000000020202";

            let input = hex::decode(INPUT_HEX).expect("invalid hex input");

            let (_, parameters) =
                Parameters::from_bytes(&input).expect("faled to parse parameters");

            assert_eq!(
                parameters,
                Parameters {
                    entrypoint: Entrypoint::Do,
                    michelson: &input[5..]
                }
            )
        }

        #[test]
        #[should_panic(expected = "Incomplete(Size(8))")]
        fn manual_eof() {
            const MICHELSON_CODE: &[u8] = &[0xab, 0xcd];

            let mut input = std::vec![0];
            input.extend_from_slice(&(10u32).to_be_bytes()[..]);
            input.extend_from_slice(MICHELSON_CODE);

            Parameters::from_bytes(&input).expect("failed to parse parameters");
        }
    }

    #[test]
    fn transfer() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 904e\
                                 01\
                                 0a\
                                 0a\
                                 e807\
                                 000035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 00";

        let mut input = hex::decode(INPUT_HEX).expect("invalid input hex");
        input.extend_from_slice(&[0xDE, 0xEA, 0xBE, 0xEF]);

        let (rem, parsed) = Transfer::from_bytes(&input).expect("couldn't parse transfer");
        assert_eq!(rem.len(), 4);

        let expected = Transfer {
            source: (Curve::Bip32Ed25519, arrayref::array_ref!(input, 1, 20)),
            fee: Zarith {
                is_negative: None,
                bytes: &input[21..23],
            },
            counter: Zarith {
                is_negative: None,
                bytes: &input[23..24],
            },
            gas_limit: Zarith {
                is_negative: None,
                bytes: &input[24..25],
            },
            storage_limit: Zarith {
                is_negative: None,
                bytes: &input[25..26],
            },
            amount: Zarith {
                is_negative: None,
                bytes: &input[26..28],
            },
            destination: ContractID::Implicit(
                Curve::Bip32Ed25519,
                arrayref::array_ref!(input, 28 + 2, 20),
            ),
            parameters: None,
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    #[should_panic(expected = "Incomplete")]
    fn transfer_eof() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e01";

        let mut input = hex::decode(INPUT_HEX).expect("invalid input hex");
        input.extend_from_slice(&[0xDE, 0xEA, 0xBE, 0xEF]);

        let (rem, _parsed) = Transfer::from_bytes(&input).expect("couldn't parse transfer");
        assert_eq!(rem.len(), 4);
    }

    #[test]
    fn contract_call() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 904e\
                                 01\
                                 0a\
                                 0a\
                                 e807\
                                 000035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 ff\
                                 02000000070a000000020202";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) = Transfer::from_bytes(&input).expect("couldn't parse transfer");
        assert_eq!(rem.len(), 0);

        let expected = Transfer {
            source: (Curve::Bip32Ed25519, arrayref::array_ref!(input, 1, 20)),
            fee: Zarith {
                is_negative: None,
                bytes: &input[21..23],
            },
            counter: Zarith {
                is_negative: None,
                bytes: &input[23..24],
            },
            gas_limit: Zarith {
                is_negative: None,
                bytes: &input[24..25],
            },
            storage_limit: Zarith {
                is_negative: None,
                bytes: &input[25..26],
            },
            amount: Zarith {
                is_negative: None,
                bytes: &input[26..28],
            },
            destination: ContractID::Implicit(
                Curve::Bip32Ed25519,
                arrayref::array_ref!(input, 28 + 2, 20),
            ),
            parameters: Some(Parameters {
                entrypoint: Entrypoint::Do,
                michelson: &input[56..],
            }),
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn operation() {
        const INPUT_HEX: &str = "a99b946c97ada0f42c1bdeae0383db7893351232a832d00d0cd716eb6f66e5616c0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e010a0ae807000035e993d8c7aaa42b5e3ccd86a33390ececc73abdff02000000070a000000020202";
        const BRANCH_BASE58: &str = "BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");
        let mut parsed = Operation::new(&input).expect("couldn't parse branch");

        let mut vbr = crate::constants::tzprefix::B.to_vec();
        vbr.extend_from_slice(&parsed.branch()[..]);

        let branch = bs58::encode(vbr).with_check().into_string();
        assert_eq!(&branch, BRANCH_BASE58);

        let ops = parsed.mut_ops();
        let op = ops
            .parse_next()
            .expect("failed to parse operation")
            .expect("no next operation found");

        match op {
            OperationType::Transfer(_) => {
                //we don't check transfer here to avoid redundancy
            }
            #[allow(unreachable_patterns)]
            opt => panic!("not the expected operation type, found: {:x?}", opt),
        }

        match ops.parse_next().expect("failed to parse operation") {
            None => {}
            Some(s) => panic!("expected no operations, found {:x?}", s),
        }
    }

    #[test]
    fn operations() {
        const INPUT_HEX: &str = "a99b946c97ada0f42c1bdeae0383db7893351232a832d00d0cd716eb6f66e561\
                                 6c0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e010a0ae807000035e993d8c7aaa42b5e3ccd86a33390ececc73abdff02000000070a000000020202\
                                 6c0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e010a0ae807016a7d4a43f51be0934a441fba4f13f9beaa4757510000\
                                 6c0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e010a0ae807016a7d4a43f51be0934a441fba4f13f9beaa47575100ff03000000290100000024747a31515a364b5937643342755a4454316431396455786f51727446504e32514a33686e";
        const BRANCH_BASE58: &str = "BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");
        let mut parsed = Operation::new(&input).expect("couldn't parse branch");

        let mut vbr = crate::constants::tzprefix::B.to_vec();
        vbr.extend_from_slice(&parsed.branch()[..]);

        let branch = bs58::encode(vbr).with_check().into_string();
        assert_eq!(&branch, BRANCH_BASE58);

        let ops = parsed.mut_ops();
        let op1 = ops
            .parse_next()
            .expect("failed to parse operation")
            .expect("no next operation found");
        let op2 = ops
            .parse_next()
            .expect("failed to parse operation")
            .expect("no next operation found");
        let op3 = ops
            .parse_next()
            .expect("failed to parse operation")
            .expect("no next operation found");

        match (op1, op2, op3) {
            (
                OperationType::Transfer(_),
                OperationType::Transfer(_),
                OperationType::Transfer(_),
            ) => {
                //we don't check transfer here to avoid redundancy
            }
            #[allow(unreachable_patterns)]
            opt => panic!("not the expected operation type, found: {:x?}", opt),
        }
    }
}
