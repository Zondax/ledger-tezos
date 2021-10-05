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
#![no_std]
#![no_builtins]

extern crate no_std_compat as std;

pub mod bip32;

pub mod hash {
    pub trait Hasher<const S: usize>: Sized {
        type Error;

        /// Add data to hasher
        fn update(&mut self, input: &[u8]) -> Result<(), Self::Error>;

        /// Retrieve digest output without resetting or consuming
        fn finalize_dirty(&mut self) -> Result<[u8; S], Self::Error> {
            let mut out = [0; S];
            self.finalize_dirty_into(&mut out).map(|_| out)
        }

        /// Retrieve digest output by writing it into a preallocated buffer
        fn finalize_dirty_into(&mut self, out: &mut [u8; S]) -> Result<(), Self::Error>;

        /// Consume hasher and retrieve output
        fn finalize(self) -> Result<[u8; S], Self::Error> {
            let mut out = [0; S];
            self.finalize_into(&mut out).map(|_| out)
        }

        /// Consume hasher and write output to given location
        fn finalize_into(self, out: &mut [u8; S]) -> Result<(), Self::Error>;

        /// Reset the state of the hasher
        fn reset(&mut self) -> Result<(), Self::Error>;

        /// One-shot digest
        fn digest(input: &[u8]) -> Result<[u8; S], Self::Error> {
            let mut out = [0; S];
            Self::digest_into(input, &mut out).map(|_| out)
        }

        /// One-shot digest into preallocated buffer
        fn digest_into(input: &[u8], out: &mut [u8; S]) -> Result<(), Self::Error>;
    }

    pub trait HasherId {
        type Id;

        fn id() -> Self::Id;
    }
}
