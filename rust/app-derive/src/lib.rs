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

//! This crate exports a single macro with a specific use case for the ledger-tezos app
//!
//! See [macro@unroll] for more documentation

use proc_macro::TokenStream;

mod unroll;
#[proc_macro]
/// Reads the file located at the provided input path and "unrolls" it.
///
/// The expected contents of the file is a JSON array of [unroll::KnownBaker],
/// these will be read and the addresses will be compacted slightly before
/// being all put in a function that will convert a given address to a static string
///
/// # Note
///
/// The provided path will be made relative to the `CARGO_MANIFEST_DIR` of the invoking crate.
///
/// In other words, the provided input path will have the current crate's root directory prepended
pub fn unroll(input: TokenStream) -> TokenStream {
    unroll::unroll(input)
}
