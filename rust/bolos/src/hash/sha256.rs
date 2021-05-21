use crate::{exceptions::catch_exception, raw::cx_sha256_t, SyscallError};

pub struct Sha256 {
    state: cx_sha256_t,
}

impl Sha256 {
    pub fn new() -> Result<Self, SyscallError> {
        let mut state = cx_sha256_t::default();

        let might_throw = || unsafe {
            //this does not throw
            crate::raw::cx_sha256_init(&mut state as *mut _);
        };

        catch_exception::<SyscallError, _, _>(might_throw)?;

        Ok(Self { state })
    }

    pub fn digest(input: &[u8]) -> Result<[u8; 32], SyscallError> {
        use super::Hasher;

        let mut digest = Self::new()?;
        digest.update(input)?;
        digest.finalize()
    }
}

impl super::CxHash<32> for Sha256 {
    fn cx_header(&mut self) -> &mut crate::raw::cx_hash_t {
        &mut self.state.header
    }
}
