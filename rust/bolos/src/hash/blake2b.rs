use crate::{errors::catch, raw::cx_blake2b_t, Error};

pub struct Blake2b<const S: usize> {
    state: cx_blake2b_t,
}

impl<const S: usize> Blake2b<S> {
    pub fn new() -> Result<Self, Error> {
        let mut state = cx_blake2b_t::default();

        let might_throw = || unsafe {
            //this does not throw
            crate::raw::cx_blake2b_init(&mut state as *mut _, (S * 8) as u32);
        };

        catch(might_throw)?;

        Ok(Self { state })
    }

    pub fn digest(input: &[u8]) -> Result<[u8; S], Error> {
        use super::Hasher;

        let mut digest = Self::new()?;
        digest.update(input)?;
        digest.finalize()
    }
}

impl<const S: usize> super::CxHash<S> for Blake2b<S> {
    fn cx_header(&mut self) -> &mut crate::raw::cx_hash_t {
        &mut self.state.header
    }
}
