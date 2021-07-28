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
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    LitByteStr, LitStr, Token,
};

enum LiteralStr {
    Str(LitStr),
    BStr(LitByteStr),
}

impl LiteralStr {
    fn null_terminate(&mut self) {
        match self {
            LiteralStr::Str(ref mut s) => {
                let mut copy = s.value();
                if !copy.ends_with('\x00') {
                    copy.push('\x00')
                }

                *s = LitStr::new(&copy, s.span());
            }
            LiteralStr::BStr(ref mut s) => {
                let mut copy = s.value();
                if !copy.ends_with(&[0]) {
                    copy.push(0)
                }

                *s = LitByteStr::new(&copy, s.span());
            }
        }
    }
}

impl Parse for LiteralStr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            Ok(Self::Str(input.parse()?))
        } else if input.peek(LitByteStr) {
            Ok(Self::BStr(input.parse()?))
        } else {
            Err(input.error("expected str literal or byte str literal"))
        }
    }
}

impl ToTokens for LiteralStr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            LiteralStr::Str(s) => s.to_tokens(tokens),
            LiteralStr::BStr(s) => s.to_tokens(tokens),
        }
    }
}

struct PicStrInput {
    s: LiteralStr,
    skip_null: bool,
}

impl Parse for PicStrInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            s: input.parse()?,
            skip_null: input.parse::<Option<Token![!]>>()?.is_some(),
        })
    }
}

pub fn pic_str(input: TokenStream) -> TokenStream {
    let PicStrInput { mut s, skip_null } = parse_macro_input!(input as PicStrInput);

    if !skip_null {
        s.null_terminate();
    }

    let output = quote! {
        PIC::new(#s).into_inner()
    };

    output.into()
}
