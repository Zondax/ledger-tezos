use bolos_derive::*;

use bolos_sys::{NVM, PIC};

#[pic]
static PIC: u8 = 42;

#[nvm]
pub static mut FLASH: [u8; 0xFFFF] = [0; 1];

#[test]
fn check_access() {
    let flash: &PIC<NVM<0xFFFF>> = unsafe { &FLASH };

    let flash = unsafe { &**FLASH };

    assert_eq!(flash, [0; 0xFFFF]);
}

#[test]
fn check_pic() {
    let pic: &PIC<u8> = unsafe { &PIC };

    assert_eq!(42, **pic);
}
