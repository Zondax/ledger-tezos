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
use std::collections::VecDeque;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    Attribute, Error, Expr, Ident, Token, Type, TypeArray, Visibility,
};

#[allow(dead_code)]
//nvm attribute input
struct NVMInput {
    attrs: Vec<Attribute>,
    vis: Visibility,
    mutability: Option<Token![mut]>,
    name: Ident,
    ty: Box<Type>,
    maybe_init: Option<Expr>,
}

impl Parse for NVMInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let _: Token![static] = input.parse()?;
        let mutability = input.parse()?;
        let name = input.parse()?;
        let _: Token![:] = input.parse()?;
        let ty = input.parse()?;
        let maybe_equals: Option<Token![=]> = input.parse()?;
        let maybe_init = match maybe_equals {
            None => None,
            Some(_) => {
                let expr = input.parse()?;
                Some(expr)
            }
        };
        let _: Token![;] = input.parse()?;

        Ok(Self {
            attrs,
            vis,
            mutability,
            name,
            ty,
            maybe_init,
        })
    }
}

//helper to add #[link_section] to NVM
struct OnlyOuterAttr(Vec<Attribute>);

impl Parse for OnlyOuterAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.call(Attribute::parse_outer).map(Self)
    }
}

//this function will walk the type to find as many nested arrays as possible
// until the inner one, verify that it's an u8 array, and return a tuple containing
// the number of inner arrays and a vector with the lengths of each array
fn walk_multi_array(ty: Box<Type>) -> Result<(usize, VecDeque<Expr>), Error> {
    let u8_ty = syn::parse2::<Type>(quote! {u8}).unwrap();
    match *ty {
        Type::Array(TypeArray { len, elem, .. }) if *elem == u8_ty => Ok((1, vec![len].into())),
        Type::Array(TypeArray { len, elem, .. }) => match *elem {
            array @ Type::Array(_) => {
                let (inner_arrays, mut lens) = walk_multi_array(Box::new(array))?;
                lens.push_back(len); //append current len at end of vec
                Ok((inner_arrays + 1, lens))
            }
            ty => {
                let ty = quote! {#ty};
                Err(Error::new(
                    ty.span(),
                    format!("nested {} is not an array", ty),
                ))
            }
        },
        ty => {
            let ty = quote! {#ty};
            Err(Error::new(
                ty.span(),
                format!(
                    "{} is neither an u8 array, nor a multi-dimensional array",
                    ty
                ),
            ))
        }
    }
}

pub fn nvm(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as NVMInput);

    let NVMInput {
        mut attrs,
        name,
        mutability,
        ty,
        vis,
        maybe_init,
    } = input;

    //add link_section when in BOLOS
    if cfg!(bolos_sdk) {
        let link_tokens = quote! {#[link_section = ".rodata.N_"]};

        let mut link_attr = syn::parse2::<OnlyOuterAttr>(link_tokens).unwrap().0;
        attrs.append(&mut link_attr);
    }

    match walk_multi_array(ty).map_err(|e| e.to_compile_error()) {
        Err(e) => e,
        Ok((_, mut lens)) => {
            //reduce length by mutliplying (it's a matrix)
            let total_len = reduce_lens(&lens);

            //construct input based on input and dims
            let init = {
                match maybe_init {
                    None => quote! {::bolos::NVM::zeroed()},
                    Some(init) => {
                        //get first len (init len)
                        let init_len = lens.pop_front().unwrap();

                        let expand_array = expand_array_tokens();
                        quote! {
                            {
                                #expand_array

                                let out = expand_array::<#init_len, #total_len>(#init);
                               //  #![allow(dead_code)]
                               // //entire array len
                               //  const total_len: usize = #len;
                               //  const init_len: usize = #init_len;
                               //  const init: [u8; init_len] = #init;
                               //  let mut out = [0u8; total_len];

                               //  //guaranteed integer since len = init.len() * ...
                               //  const loop_len: usize = total_len / init_len;

                               //  let mut i = 0;
                               //  while i < loop_len {
                               //      let mut j = 0;
                               //      let offset = i * init_len;
                               //      while j < init_len {
                               //          out[offset + j] = init[j];
                               //          j += 1;
                               //      }
                               //      i += 1;
                               //  }

                                ::bolos::NVM::new(out)
                            }
                        }
                    }
                }
            };

            quote! {
                #(#attrs)*
                #[bolos::pic]
                #vis static #mutability #name: ::bolos::NVM<#total_len> = #init;
            }
        }
    }
    .into()
}

fn expand_array_tokens() -> proc_macro2::TokenStream {
    quote! {
    const fn expand_array<const INIT: usize, const TOTAL: usize>(init: [u8; INIT]) -> [u8; TOTAL] {
        //entire array len
        let mut out = [0u8; TOTAL];

        //guaranteed integer since len = init.len() * ...
        let loop_len: usize = TOTAL / INIT;

        let mut i = 0;
        while i < loop_len {
            let mut j = 0;
            let offset = i * INIT;
            while j < INIT {
                out[offset + j] = init[j];
                j += 1;
            }
            i += 1;
        }

        out
    }
    }
}

fn reduce_lens(lens: &VecDeque<Expr>) -> Expr {
    let acc_expr = quote! { (1 * 1) };
    let acc_expr = syn::parse2::<Expr>(acc_expr).unwrap();

    lens.iter().fold(acc_expr, |acc_expr, expr| {
        let new = quote! {{(#acc_expr) * (#expr)}};

        //all are valid `Expr`
        syn::parse2::<Expr>(new).unwrap()
    })
}
