use bolos_derive::*;

use bolos_sys::{NVM, PIC};

#[pic]
static PIC: u8 = 42;

#[nvm]
static FLASH: [u8; 0xFFFF];

#[test]
fn check_access() {
    let flash: &PIC<NVM<0xFFFF>> = unsafe { &FLASH };

    assert_eq!(flash.get_ref().read(), &[0; 0xFFFF]);
}

#[test]
fn check_pic() {
    let pic: &PIC<u8> = &PIC;

    assert_eq!(42, **pic);
}
