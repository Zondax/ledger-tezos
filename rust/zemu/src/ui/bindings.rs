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
pub type zxerr_t = cty::c_uint;
pub const zxerr_t_zxerr_unknown: zxerr_t = 0;
pub const zxerr_t_zxerr_ok: zxerr_t = 3;
pub const zxerr_t_zxerr_no_data: zxerr_t = 5;
pub const zxerr_t_zxerr_buffer_too_small: zxerr_t = 6;
pub const zxerr_t_zxerr_out_of_bounds: zxerr_t = 9;
pub const zxerr_t_zxerr_encoding_failed: zxerr_t = 10;
pub const zxerr_t_zxerr_invalid_crypto_settings: zxerr_t = 12;
pub const zxerr_t_zxerr_ledger_api_error: zxerr_t = 15;

pub type viewfunc_getNumItems_t =
    ::core::option::Option<unsafe extern "C" fn(num_items: *mut u8) -> zxerr_t>;
pub type viewfunc_getItem_t = ::core::option::Option<
    unsafe extern "C" fn(
        displayIdx: i8,
        outKey: *mut cty::c_char,
        outKeyLen: u16,
        outVal: *mut cty::c_char,
        outValLen: u16,
        pageIdx: u8,
        pageCount: *mut u8,
    ) -> zxerr_t,
>;
pub type viewfunc_accept_t = ::core::option::Option<unsafe extern "C" fn()>;
extern "C" {
    #[doc = " view_init (initializes UI)"]
    pub fn view_init();
}
extern "C" {
    #[doc = " view_idle_show (idle view - main menu + status)"]
    pub fn view_idle_show(item_idx: u8, statusString: *mut cty::c_char);
}
extern "C" {
    pub fn view_message_show(title: *mut cty::c_char, message: *mut cty::c_char);
}
extern "C" {
    #[doc = " view_error (error view)"]
    pub fn view_error_show();
}
extern "C" {
    pub fn view_review_init(
        viewfuncGetItem: viewfunc_getItem_t,
        viewfuncGetNumItems: viewfunc_getNumItems_t,
        viewfuncAccept: viewfunc_accept_t,
    );
}
extern "C" {
    pub fn view_review_show();
}
