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
