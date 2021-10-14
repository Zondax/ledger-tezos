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
use sha2::digest::{Digest, FixedOutput};

pub struct Sha512(sha2::Sha512);

impl Sha512 {
    pub fn new() -> Result<Self, std::convert::Infallible> {
        Ok(Self(sha2::Sha512::new()))
    }
}

/*
 * pub trait Hasher<const S: usize> {
        type Error;

        /// Add data to hasher
        fn update(&mut self, input: &[u8]) -> Result<(), Self::Error>;

        /// Consume hasher and retrieve output
        fn finalize(mut self) -> Result<[u8; S], Self::Error>;

        /// One-short digest
        fn digest(input: &[u8]) -> Result<[u8; S], Error>;
    }
*/
impl super::Hasher<64> for Sha512 {
    type Error = std::convert::Infallible;

    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.0.update(input);
        Ok(())
    }

    fn finalize_dirty_into(&mut self, out: &mut [u8; 64]) -> Result<(), Self::Error> {
        let tmp = self.0.finalize_fixed_reset();
        out.copy_from_slice(tmp.as_ref());

        Ok(())
    }

    fn finalize_into(mut self, out: &mut [u8; 64]) -> Result<(), Self::Error> {
        out.copy_from_slice(self.0.finalize_fixed_reset().as_ref());

        Ok(())
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        self.0.reset();
        Ok(())
    }

    fn digest_into(input: &[u8], out: &mut [u8; 64]) -> Result<(), Self::Error> {
        let mut hasher = Self::new()?;
        hasher.update(input)?;
        hasher.finalize_into(out)
    }
}

impl super::HasherId for Sha512 {
    type Id = u8;

    fn id() -> Self::Id {
        5
    }
}
