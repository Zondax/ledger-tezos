pub mod legacy_version;
pub mod public_key;
pub mod signing;
pub mod version;

#[cfg(feature = "dev")]
pub mod dev;

#[cfg(feature = "baking")]
pub mod hwm;

mod utils {
    #[repr(u8)]
    pub enum PacketType {
        Init = 0,
        Add = 1,
        Last = 2,
    }

    impl std::convert::TryFrom<u8> for PacketType {
        type Error = ();

        fn try_from(from: u8) -> Result<Self, ()> {
            match from {
                0 => Ok(Self::Init),
                1 => Ok(Self::Add),
                2 => Ok(Self::Last),
                _ => Err(()),
            }
        }
    }

    impl Into<u8> for PacketType {
        fn into(self) -> u8 {
            self as _
        }
    }
}
pub(self) use utils::*;

pub(self) mod resources {
    use super::lock::Lock;
    use bolos::{lazy_static, new_swapping_buffer, SwappingBuffer};

    #[lazy_static]
    pub static mut BUFFER: Lock<SwappingBuffer<'static, 'static, 0xFF, 0xFFFF>, BUFFERAccessors> =
        Lock::new(new_swapping_buffer!(0xFF, 0xFFFF));

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum BUFFERAccessors {
        Sign,
        #[cfg(feature = "dev")]
        Sha256,
    }

    impl From<super::signing::Sign> for BUFFERAccessors {
        fn from(_: super::signing::Sign) -> Self {
            Self::Sign
        }
    }

    #[cfg(feature = "dev")]
    impl From<super::dev::Sha256> for BUFFERAccessors {
        fn from(_: super::dev::Sha256) -> Self {
            Self::Sha256
        }
    }
}

mod lock {
    use crate::constants::ApduError;

    pub struct Lock<T, A> {
        item: T,
        lock: Option<A>,
    }

    #[derive(Debug)]
    pub enum LockError {
        Busy,
        NotLocked,
        BadId,
    }

    impl<T, A> Lock<T, A> {
        pub const fn new(item: T) -> Self {
            Self { item, lock: None }
        }
    }

    impl<T, A: Eq> Lock<T, A> {
        ///Locks the resource (if available) and retrieve it
        pub fn lock(&mut self, acquirer: impl Into<A>) -> Result<&mut T, LockError> {
            let acq = acquirer.into();
            match self.lock {
                Some(ref a) if a == &acq => Ok(&mut self.item),
                //if it's busy we forcefully acquire the lock
                Some(_) | None => {
                    self.lock = Some(acq);
                    Ok(&mut self.item)
                }
            }
        }

        ///Acquire the resource if locked by `acquirer`
        pub fn acquire(&mut self, acquirer: impl Into<A>) -> Result<&mut T, LockError> {
            let acq = acquirer.into();
            match self.lock {
                Some(ref a) if a == &acq => Ok(&mut self.item),
                Some(_) => Err(LockError::Busy),
                None => Err(LockError::NotLocked),
            }
        }

        ///Release the resource if locker by `acquirer`
        pub fn release(&mut self, acquirer: impl Into<A>) -> Result<(), LockError> {
            let acq = acquirer.into();
            match self.lock {
                Some(ref a) if a == &acq => {
                    self.lock = None;
                    Ok(())
                }
                Some(_) => Err(LockError::BadId),
                None => Err(LockError::NotLocked),
            }
        }
    }

    impl From<LockError> for ApduError {
        fn from(lock: LockError) -> Self {
            match lock {
                LockError::NotLocked => Self::ExecutionError,
                LockError::Busy | LockError::BadId => Self::Busy,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const fn build_lock(init: u32) -> Lock<u32, i32> {
            Lock::new(init)
        }

        #[test]
        fn nominal_use() {
            let mut lock = build_lock(0);

            *lock.lock(0).unwrap() += 1;

            assert_eq!(1, *lock.acquire(0).unwrap());

            lock.release(0).unwrap();
        }

        #[test]
        fn bad_accessors() {
            let mut lock = build_lock(32);
            lock.lock(32).unwrap();

            lock.acquire(0).unwrap_err();
            lock.release(0).unwrap_err();
        }

        #[test]
        fn lock_released() {
            let mut lock = build_lock(42);
            lock.lock(0).unwrap();
            lock.release(0).unwrap();

            lock.lock(1).unwrap();
        }

        #[test]
        fn force_lock() {
            let mut lock = build_lock(2);
            lock.lock(0).unwrap();
            lock.lock(1).unwrap();

            lock.acquire(0).unwrap_err();
            lock.acquire(1).unwrap();
        }
    }
}
