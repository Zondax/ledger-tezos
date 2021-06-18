use bolos_sys::pic::PIC;
use bolos_sys::raw::{io_exchange, CHANNEL_APDU, IO_ASYNCH_REPLY, IO_RETURN_AFTER_TX};
use core::ptr::NonNull;

pub(self) mod bindings {
    #![allow(non_snake_case)]

    cfg_if::cfg_if! {
        if #[cfg(zemu_sdk)] {
            include!("ui/bindings.rs");
        } else {
            include!("ui/bindings_stub.rs");
        }
    }
}

mod manual_vtable;
use manual_vtable::RefMutDynViewable;

//This is _terribly_ unsafe, as we assume the pointer hasn't been invalidated!
#[bolos_derive::lazy_static]
static mut CURRENT_VIEWABLE: Option<RefMutDynViewable> = None;

//contains a pointer to the apdu_buffer
#[bolos_derive::lazy_static]
static mut APDU_BUFFER: Option<NonNull<[u8]>> = None;

pub enum ViewError {
    Unknown,
}

impl Into<bindings::zxerr_t> for ViewError {
    fn into(self) -> bindings::zxerr_t {
        match self {
            Self::Unknown => bindings::zxerr_t_zxerr_unknown,
        }
    }
}

pub trait Viewable {
    /// Return the number of items to render
    fn num_items(&mut self) -> Result<u8, ViewError>;

    /// Render `item_n` into `title` and `message`
    ///
    /// If an item is too long to render in the output, the number of "pages" is returned,
    /// and each page can be retrieved via the `page` parameter
    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError>;

    /// Called when the last item shown has been "accepted"
    ///
    /// `out` is the apdu_buffer
    ///
    /// Return is number of bytes written to out and the return code
    fn accept(&mut self, out: &mut [u8]) -> (usize, u16);

    /// Called when the last item shows has been "rejected"
    /// `out` is the apdu_buffer
    ///
    /// Return is number of bytes written to out and the return code
    fn reject(&mut self, out: &mut [u8]) -> (usize, u16);
}

pub trait Show: Viewable + Sized {
    /// This is to be called when you wish to show the item
    ///
    /// `flags` should be the same parameter in `ApduHandler::handle`
    // for now we consume the item so we can guarantee
    // safe usage
    fn show(mut self, out: &mut [u8]) -> ! {
        //set `CURRENT_VIEWABLE`
        unsafe {
            CURRENT_VIEWABLE.replace((&mut self).into());
            *APDU_BUFFER = NonNull::new(out as *mut [u8]);
        }

        //self will be dropped when `CURRENT_VIEWABLE` is taken
        ::core::mem::forget(self);

        //set view_review
        view_review_init();

        //start the show
        unsafe {
            bindings::view_review_show();
        }

        //keep driving the event loop
        // this will keep the stack alive
        // while allowing the ui to handle events
        drive()
    }
}

fn cleanup_ui() {
    unsafe {
        bindings::view_review_init(None, None, None);
        CURRENT_VIEWABLE.take();
        APDU_BUFFER.take();
    }
}

//keep driving the event loop and io
// so the ui is "run" here instead of the top level
fn drive() -> ! {
    //"give" control to the event code
    unsafe {
        io_exchange((CHANNEL_APDU | IO_ASYNCH_REPLY) as u8, 0);
        core::hint::unreachable_unchecked()
    }
}

impl<T: Viewable + Sized> Show for T {}

fn get_current_viewable<'v>() -> Result<(&'v mut RefMutDynViewable, &'v mut [u8]), ViewError> {
    match unsafe {
        (
            CURRENT_VIEWABLE.as_mut(),
            APDU_BUFFER.as_mut().map(|p| p.as_mut()),
        )
    } {
        (Some(refmut), Some(buf)) => Ok((refmut, buf)),
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
                },
            }
        }
    }
}

unsafe extern "C" fn viewfunc_accept() {
    if let Ok((obj, out)) = get_current_viewable() {
        let (len, code) = obj.accept(out);

        //set code
        out[len..len+2].copy_from_slice(&code.to_be_bytes()[..]);
        cleanup_ui();
        io_exchange((CHANNEL_APDU | IO_RETURN_AFTER_TX) as u8, 2 + len as u16);
    }
}

unsafe extern "C" fn viewfunc_reject() {
    if let Ok((obj, out)) = get_current_viewable() {
        let (len, code) = obj.reject(out);

        //set code
        out[len..len+2].copy_from_slice(&code.to_be_bytes()[..]);
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
