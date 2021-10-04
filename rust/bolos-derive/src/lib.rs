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

//! This crate exports 4 macros that are useful if not essential for correct
//! and ergonomic rust in a ledger app
//!
//! The currently exported macros are:
//! * [macro@nvm]
//! * [macro@pic]
//! * [macro@pic_str]
//! * [macro@lazy_static]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStatic};

// #[bolos::nvm]
// static mut __FLASH: [[u8; 0xFFFF]; ..];
//
// static mut __FLASH: PIC<NVM<0xFFFF * ..>> = PIC::new(NVM::zeroed());
mod nvm;
#[proc_macro_attribute]
/// This attribute macro is to be applied on top of static items to make sure
/// the item will end up in non-volatile memory (NVM), it will also wrap the item in
/// appropricate types so runtime usage it correct aswell.
///
/// # What's possible
/// ## Initialization
/// NVM storage can be initialized by default with zeros or specifying the initialization array.
/// ```rust
/// #[bolos_derive::nvm]
/// static mut FOO: [u8; 42]; //initialized with all zeroes
///
/// # assert_eq!(unsafe { FOO[30] }, 0);
/// #[bolos_derive::nvm]
/// static mut ONES_AND_TWOS: [u8; 2] = [1, 2];
///
/// assert_eq!(unsafe { ONES_AND_TWOS[0] }, 1);
/// assert_eq!(unsafe { ONES_AND_TWOS[1] }, 2);
/// ```
///
/// ## Multi dimensional arrays
/// It's possible to declare a multi dimensional array, but it will be flattened to one-dimensional.
/// Initialization follows the same rules as above,
/// where the init subarray must be the first dimension initialization array.
///
/// ```rust
/// #[bolos_derive::nvm]
/// static mut FOO: [[u8; 10]; 20] = [33; 10]; //initialize a 10 * 20 array with all 33
///
/// assert_eq!(unsafe { **FOO }, [33u8; 10*20]);
/// ```
pub fn nvm(metadata: TokenStream, input: TokenStream) -> TokenStream {
    nvm::nvm(metadata, input)
}

// #[bolos::pic]
// static mut BUFFER: MyBuffer = MyBuffer::new();
//
// static mut BUFFER: PIC<MyBuffer> = PIC::new(MyBuffer::new());

#[proc_macro_attribute]
pub fn pic(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStatic);

    let ItemStatic {
        attrs,
        ident: name,
        mutability,
        ty,
        vis,
        expr,
        ..
    } = input;
    let ty = *ty;
    let expr = *expr;

    let output = quote! {
        #(#attrs)*
        #vis static #mutability #name: ::bolos::PIC<#ty> = ::bolos::PIC::new(#expr);
    };

    output.into()
}

mod pic_str;
#[proc_macro]
/// This macro is to be used when a str literal is needed.
/// The macro will automatically use `PIC` to guarantee proper access at runtime
/// as well as null terminate the string (if not already).
///
/// It's possible to avoid null termination by appending a `!` at the end of the string
pub fn pic_str(input: TokenStream) -> TokenStream {
    pic_str::pic_str(input)
}

// #[bolos::static]
// static mut LAZY_STATIC_OBJECT: Object = Object::new()
mod lazy_static;

#[proc_macro_attribute]
pub fn lazy_static(metadata: TokenStream, input: TokenStream) -> TokenStream {
    lazy_static::lazy_static(metadata, input)
}
