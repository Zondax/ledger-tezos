pub type zxerr_t = cty::c_uint;

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

pub fn view_init() {
    todo!("view_init stub")
}

pub fn view_idle_show(item_idx: u8, statusString: *mut cty::c_char) {
    todo!("view_idle_show stub")
}

pub fn view_message_show(title: *mut cty::c_char, message: *mut cty::c_char) {
    todo!("view_message_show stub")
}

pub fn view_error_show() {
    todo!("view_error_show stub")
}

pub fn view_review_init(
    viewfuncGetItem: viewfunc_getItem_t,
    viewfuncGetNumItems: viewfunc_getNumItems_t,
    viewfuncAccept: viewfunc_accept_t,
) {
    todo!("view_review_init stub")
}

pub fn view_review_show() {
    todo!("view_review_show stub")
}
