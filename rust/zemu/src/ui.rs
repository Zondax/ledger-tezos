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
use bolos_sys::raw::{
    io_exchange, G_io_apdu_buffer as APDU_BUFFER, CHANNEL_APDU, IO_ASYNCH_REPLY, IO_RETURN_AFTER_TX,
};

mod comm;
pub use comm::*;

/// cbindgen:ignore
pub(self) mod bindings {
    #![allow(non_snake_case)]
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]

    include!("ui/bindings.rs");
}

pub(crate) mod manual_vtable;
use manual_vtable::RefMutDynViewable;

//This is _terribly_ unsafe, as we assume the pointer hasn't been invalidated!
#[bolos_derive::lazy_static]
static mut CURRENT_VIEWABLE: Option<RefMutDynViewable> = None;

//no need to lazy static as we won't be reading this before writing
// (not even dropping, as it's usize)
static mut BUSY_BYTES: usize = 0;

impl Into<bindings::zxerr_t> for ViewError {
    fn into(self) -> bindings::zxerr_t {
        match self {
            Self::Unknown | Self::Reject => bindings::zxerr_t_zxerr_unknown,
            Self::NoData => bindings::zxerr_t_zxerr_no_data,
        }
    }
}

fn move_to_global_storage<T: Sized>(item: T) -> Option<&'static mut T> {
    let size = core::mem::size_of::<T>();
    unsafe {
        let buf_len = APDU_BUFFER.len();
        if size > buf_len {
            //if we don't have enough space
            // we can even check for a max size, say 64 bytes
            return None;
        }

        let new_loc_slice = &mut APDU_BUFFER[buf_len - size..];
        let new_loc_raw_ptr: *mut u8 = new_loc_slice.as_mut_ptr();
        let new_loc: *mut T = new_loc_raw_ptr.cast();

        //write but we don't want to drop `new_loc` since
        // it's not actually valid T data
        core::ptr::write(new_loc, item);

        //write how many bytes we have occupied
        BUSY_BYTES = size;

        //we can unwrap as we know this ptr is valid
        Some(new_loc.as_mut().unwrap())
    }
}

impl<T: Viewable + Sized + 'static> Show for T {
    unsafe fn show(self, flags: &mut u32) -> Result<(), ShowTooBig> {
        use crate::ui_toolkit::RUST_ZUI;

        RUST_ZUI.show(self)?;

        *flags |= IO_ASYNCH_REPLY;
        Ok(())
    }
}

fn cleanup_ui() {
    unsafe {
        bindings::view_review_init(None, None, None);

        //RefMutDynViewable takes care of dropping the inner item
        CURRENT_VIEWABLE.take();
    }
}

fn get_current_viewable<'v>() -> Result<(&'v mut RefMutDynViewable, &'v mut [u8]), ViewError> {
    match unsafe {
        (
            CURRENT_VIEWABLE.as_mut(),
            &mut APDU_BUFFER[..APDU_BUFFER.len() - BUSY_BYTES],
        )
    } {
        (Some(refmut), buf) => Ok((refmut, buf)),
        _ => Err(ViewError::Unknown),
    }
}

unsafe extern "C" fn viewfunc_get_num_items(num_items: *mut u8) -> bindings::zxerr_t {
    match get_current_viewable() {
        Err(e) => e.into(),
        Ok((obj, _)) => match obj.num_items() {
            Ok(n) => {
                num_items.write(n);
                bindings::zxerr_t_zxerr_ok
            }
            Err(e) => e.into(),
        },
    }
}

//unsafe here is because it's required by the signature later
unsafe extern "C" fn viewfunc_get_item(
    item_n: i8,
    out_key: *mut cty::c_char,
    out_key_len: u16,
    out_val: *mut cty::c_char,
    out_val_len: u16,
    page_idx: u8,
    page_count: *mut u8,
) -> bindings::zxerr_t {
    match get_current_viewable() {
        Err(e) => e.into(),
        Ok((obj, _)) => {
            let out_key =
                core::slice::from_raw_parts_mut(out_key as *mut cty::c_uchar, out_key_len as usize);
            let out_val =
                core::slice::from_raw_parts_mut(out_val as *mut cty::c_uchar, out_val_len as usize);

            match obj.render_item(item_n as u8, out_key, out_val, page_idx) {
                Err(e @ ViewError::Reject) => {
                    viewfunc_reject();
                    e.into()
                }
                Err(e) => e.into(),
                Ok(count) => {
                    //asciify
                    out_key
                        .iter_mut()
                        .filter(|&&mut c| c != 0 && (c < 32 || c > 0x7F))
                        .for_each(|c| {
                            *c = '.' as u8;
                        });
                    out_val
                        .iter_mut()
                        .filter(|&&mut c| c != 0 && (c < 32 || c > 0x7F))
                        .for_each(|c| {
                            *c = '.' as u8;
                        });

                    page_count.write(count);
                    bindings::zxerr_t_zxerr_ok
                }
            }
        }
    }
}

unsafe extern "C" fn viewfunc_accept() {
    if let Ok((obj, out)) = get_current_viewable() {
        let (len, code) = obj.accept(out);

        //set code
        out[len..len + 2].copy_from_slice(&code.to_be_bytes()[..]);
        cleanup_ui();
        io_exchange((CHANNEL_APDU | IO_RETURN_AFTER_TX) as u8, 2 + len as u16);
    }
}

unsafe extern "C" fn viewfunc_reject() {
    if let Ok((obj, out)) = get_current_viewable() {
        let (len, code) = obj.reject(out);

        //set code
        out[len..len + 2].copy_from_slice(&code.to_be_bytes()[..]);
        cleanup_ui();
        io_exchange((CHANNEL_APDU | IO_RETURN_AFTER_TX) as u8, 2 + len as u16);
    }
}

fn view_review_init() {
    unsafe {
        bindings::view_review_init(
            Some(viewfunc_get_item),
            Some(viewfunc_get_num_items),
            Some(viewfunc_accept),
        );
    }
}
