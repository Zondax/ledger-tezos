use bolos_derive::*;

use bolos::{NVM, PIC};

#[test]
fn check_access() {
    #[nvm]
    static FLASH: [u8; 0xFFFF];
    let flash: &PIC<NVM<0xFFFF>> = &FLASH;

    assert_eq!(flash.get_ref().read(), &[0; 0xFFFF]);
}

#[test]
fn check_pic() {
    #[pic]
    static PIC: u8 = 42;
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

#[test]
fn check_2_dim() {
    #[nvm]
    static MULTI2: [[u8; 8]; 8];

    let multi2: &NVM<{ 8 * 8 }> = &MULTI2;

    assert_eq!(multi2.read(), &[0; 8 * 8])
}

#[test]
fn check_3_dim() {
    #[nvm]
    static MULTI3: [[[u8; 8]; 8]; 8];

    let multi3: &NVM<{ 8 * 8 * 8 }> = &MULTI3;

    assert_eq!(multi3.read(), &[0; 8 * 8 * 8])
}

#[test]
fn check_initialized() {
    #[nvm]
    static NON_ZERO: [u8; 8] = [42u8; 8];

    let non_zero: &NVM<8> = &NON_ZERO;

    assert_eq!(non_zero.read(), &[42; 8]);
}

#[test]
fn check_initialized_2_dim() {
    #[nvm]
    static NON_ZERO2: [[u8; 3]; 4] = [13u8; 3];

    let non_zero2: &NVM<{ 3 * 4 }> = &NON_ZERO2;
    let expected: Vec<u8> = vec![vec![13; 3]; 4].into_iter().flatten().collect();

    assert_eq!(&non_zero2.read()[..], &expected);
}

#[test]
fn check_initialized_with_expr() {
    const INIT: [u8; 10] = [42; 10];

    #[nvm]
    static EXPR: [[u8; 10]; 2] = INIT;

    let expr: &NVM<{ 10 * 2 }> = &EXPR;
    let expected: Vec<u8> = vec![vec![42; 10]; 2].into_iter().flatten().collect();

    assert_eq!(&expr.read()[..], &expected);
}
