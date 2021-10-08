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
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Error, Expr, ExprArray, ExprLit, LitByte, LitStr};

use std::{
    convert::{TryFrom, TryInto},
    path::Path,
};

use arrayref::{array_ref, array_refs};
use serde::{Deserialize, Serialize};

/// This structs represents the expected schematic of the baker data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnownBaker {
    #[serde(alias = "bakerName")]
    name: String,
    #[serde(alias = "bakerAccount")]
    addr: String,
}

///This struct is the baker data decoded (for the address) and ready to be used for code generation
struct ReducedBaker {
    name: String,
    prefix: [u8; 3],
    hash: [u8; 20],
}

impl TryFrom<KnownBaker> for ReducedBaker {
    type Error = bs58::decode::Error;
    fn try_from(from: KnownBaker) -> Result<Self, Self::Error> {
        let addr = bs58::decode(from.addr.as_bytes()).into_vec()?;

        let addr = array_ref!(&addr[..], 0, 27);
        let (prefix, hash, _checksum) = array_refs!(addr, 3, 20, 4);

        Ok(Self {
            name: from.name,
            prefix: *prefix,
            hash: *hash,
        })
    }
}

pub fn unroll(input: TokenStream) -> TokenStream {
    let data_filepath = parse_macro_input!(input as LitStr);

    let data = match retrieve_data(data_filepath.value(), data_filepath.span()) {
        Err(e) => return e.into_compile_error().into(),
        Ok(data) => data,
    };

    let arms = data.into_iter().map(|ReducedBaker { name, prefix, hash }| {
        let name = name.as_str();
        let prefix = ExprArray {
            attrs: vec![],
            bracket_token: Default::default(),
            elems: prefix
                .iter()
                .map(|&num| LitByte::new(num, Span::call_site()))
                .map(|lit| {
                    Expr::Lit(ExprLit {
                        attrs: vec![],
                        lit: lit.into(),
                    })
                })
                .collect(),
        };
        let hash = ExprArray {
            attrs: vec![],
            bracket_token: Default::default(),
            elems: hash
                .iter()
                .map(|&num| LitByte::new(num, Span::call_site()))
                .map(|lit| {
                    Expr::Lit(ExprLit {
                        attrs: vec![],
                        lit: lit.into(),
                    })
                })
                .collect(),
        };

        quote! {
            (&#prefix, &#hash) => #name
        }
    });

    let out = quote! {
        #[derive(Debug)]
        pub struct BakerNotFound;

        pub fn baker_lookup(prefix: &[u8; 3], hash: &[u8; 20]) -> Result<&'static str, BakerNotFound> {
            let out = match (prefix, hash) {
                #(#arms ,)*
                _ => return Err(BakerNotFound),
            };

            Ok(PIC::new(out).into_inner())
        }

    };

    out.into()
}

fn retrieve_data(path: impl AsRef<Path>, path_span: Span) -> Result<Vec<ReducedBaker>, Error> {
    let data_path = match std::fs::canonicalize(path.as_ref()) {
        Ok(path) => path,
        Err(err) => {
            return Err(Error::new(
                path_span,
                format!(
                    "Invalid path provided. Detected path: {:?}; err={:?}",
                    path.as_ref(),
                    err
                ),
            ));
        }
    };

    match std::fs::File::open(&data_path) {
        Ok(file) => match serde_json::from_reader::<_, Vec<KnownBaker>>(file) {
            Ok(data) => {
                let data: Result<Vec<_>, _> = data
                    .into_iter()
                    .enumerate()
                    .map(|(i, item)| item.try_into().map_err(|e| (i, e)))
                    .collect();

                data.map_err(|(i, e)| {
                    Error::new(
                        path_span,
                        format!("Entry #{}'s address was not valid base58; err={:?}", i, e),
                    )
                })
            }
            Err(err) => Err(Error::new(
                path_span,
                format!("File was not valid JSON. err={:?}", err),
            )),
        },
        Err(err) => Err(Error::new(
            path_span,
            format!("Could not read file. Path: {:?}; err={:?}", data_path, err),
        )),
    }
}
