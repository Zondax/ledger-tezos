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

    fn finalize_dirty(&mut self) -> Result<[u8; 64], Self::Error> {
        let mut out = [0; 64];

        let tmp = self.0.finalize_fixed_reset();
        out.copy_from_slice(tmp.as_ref());

        Ok(out)
    }

    fn finalize(mut self) -> Result<[u8; 64], Self::Error> {
        self.finalize_dirty()
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        self.0.reset();
        Ok(())
    }

    fn digest(input: &[u8]) -> Result<[u8; 64], Self::Error> {
        let mut hasher = Self::new()?;
        hasher.update(input)?;
        hasher.finalize()
    }
}

impl super::HasherId for Sha512 {
    type Id = u8;

    fn id() -> Self::Id {
        5
    }
}
