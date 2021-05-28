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

#[lazy_static]
static mut SOMETHING: u32 = 33;

#[test]
fn check_lazy() {
    let something: &mut __IMPL_LAZY_SOMETHING::__LAZY_SOMETHING = unsafe { &mut SOMETHING };
    let something = &mut **something;

    assert_eq!(33, *something);

    *something += 1;

    assert_eq!(34, *something);
}
