/*******************************************************************************
 *   (c) 2021 Zondax GmbH
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 ********************************************************************************/

use bolos::{NVM, PIC};
use bolos_derive::*;

#[test]
fn check_access() {
    #[nvm]
    static FLASH: [u8; 0xFFFF];
    let flash: &PIC<NVM<0xFFFF>> = &FLASH;

    assert_eq!(flash.get_ref().read(), &[0; 0xFFFF]);
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
