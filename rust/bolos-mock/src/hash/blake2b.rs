use blake2::digest::{Update, VariableOutput};

pub struct Blake2b<const S: usize>(blake2::VarBlake2b);

impl<const S: usize> Blake2b<S> {
    pub fn new() -> Result<Self, crate::Error> {
        blake2::VarBlake2b::new(S)
            .map(Self)
            .map_err(|e| S as u16)
            .map_err(|e| e.into())
    }
}

impl<const S: usize> super::Hasher<S> for Blake2b<S> {
    type Error = crate::Error;

    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.0.update(input);
        Ok(())
    }

    fn finalize(self) -> Result<[u8; S], Self::Error> {
        let mut out = [0; S];

        self.0
            .finalize_variable(|digest| out.copy_from_slice(digest));

        Ok(out)
    }

    fn digest(input: &[u8]) -> Result<[u8; S], Self::Error> {
        let mut hasher = Self::new()?;
        hasher.update(input)?;
        hasher.finalize()
    }
}