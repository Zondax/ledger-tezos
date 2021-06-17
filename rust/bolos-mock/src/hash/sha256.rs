use sha2::digest::{Digest, FixedOutput};

pub struct Sha256(sha2::Sha256);

impl Sha256 {
    pub fn new() -> Result<Self, std::convert::Infallible> {
        Ok(Self(sha2::Sha256::new()))
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
impl super::Hasher<32> for Sha256 {
    type Error = std::convert::Infallible;

    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.0.update(input);
        Ok(())
    }

    fn finalize_dirty(&mut self) -> Result<[u8; 32], Self::Error> {
        Ok(*self.0.finalize_fixed_reset().as_ref())
    }

    fn finalize(self) -> Result<[u8; 32], Self::Error> {
        Ok(*self.0.finalize().as_ref())
    }

    fn finalize_into(self, out: &mut [u8; 32]) -> Result<(), Self::Error> {
        out.copy_from_slice(self.0.finalize().as_ref());

        Ok(())
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        self.0.reset();
        Ok(())
    }

    fn digest(input: &[u8]) -> Result<[u8; 32], Self::Error> {
        let mut hasher = Self::new()?;
        hasher.update(input)?;
        hasher.finalize()
    }
}

impl super::HasherId for Sha256 {
    type Id = u8;

    fn id() -> Self::Id {
        3
    }
}
