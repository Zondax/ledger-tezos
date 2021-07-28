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
use bolos::PIC;
use bolos_derive::*;

#[test]
fn check_pic_str() {
    let pic: &str = pic_str!("hello");
    let pic_no_null: &str = pic_str!("hello"!);

    assert_eq!(pic, "hello\x00");
    assert_eq!(pic_no_null, "hello");
}

#[test]
fn check_pic_bstr() {
    let pic: &[u8] = pic_str!(b"hello");
    let pic_no_null: &[u8] = pic_str!(b"hello"!);

    assert_eq!(pic, b"hello\x00");
    assert_eq!(pic_no_null, b"hello");
}
