#![no_std]
#![no_builtins]

extern crate no_std_compat as std;

pub mod bip32;

pub mod hash {
    pub trait Hasher<const S: usize> {
        type Error;

        /// Add data to hasher
        fn update(&mut self, input: &[u8]) -> Result<(), Self::Error>;

        /// Retrieve digest output without resetting or consuming
        fn finalize_dirty(&mut self) -> Result<[u8; S], Self::Error>;

        /// Consume hasher and retrieve output
        fn finalize(self) -> Result<[u8; S], Self::Error>;

        /// Reset the state of the hasher
        fn reset(&mut self) -> Result<(), Self::Error>;

        /// One-short digest
        fn digest(input: &[u8]) -> Result<[u8; S], Self::Error>;
    }
}
