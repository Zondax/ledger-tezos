#![no_std]
#![no_builtins]

pub mod bip32;

pub mod hash {
    pub trait Hasher<const S: usize> {
        type Error;

        /// Add data to hasher
        fn update(&mut self, input: &[u8]) -> Result<(), Self::Error>;

        /// Consume hasher and retrieve output
        fn finalize(self) -> Result<[u8; S], Self::Error>;

        /// One-short digest
        fn digest(input: &[u8]) -> Result<[u8; S], Self::Error>;
    }
}
