use super::{ViewError, Viewable};
use bolos_sys::pic::PIC;

use core::ptr;

use private::This;
mod private {
    pub struct This(());
}

struct ViewableVTable {
    num_items: unsafe fn(*mut This) -> Result<u8, ViewError>,
    render_item: unsafe fn(
        *mut This,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError>,
    accept: unsafe fn(*mut This, out: &mut [u8]) -> (usize, u16),
    reject: unsafe fn(*mut This, out: &mut [u8]) -> (usize, u16),
    drop: unsafe fn(*mut This),
}

trait ViewableWithVTable: Viewable + Sized {
    const VTABLE: ViewableVTable = ViewableVTable {
        num_items: |this: *mut This| -> Result<u8, ViewError> {
            unsafe {
                let this = this.cast::<Self>().as_mut().expect("Got NULL");

                this.num_items()
            }
        },
        render_item: |this: *mut This,
                      item_n: u8,
                      title: &mut [u8],
                      message: &mut [u8],
                      page: u8|
         -> Result<u8, ViewError> {
            unsafe {
                let this = this.cast::<Self>().as_mut().expect("Got NULL");

                this.render_item(item_n, title, message, page)
            }
        },
        accept: |this: *mut This, out: &mut [u8]| -> (usize, u16) {
            unsafe {
                let this = this.cast::<Self>().as_mut().expect("Got NULL");

                this.accept(out)
            }
        },
        reject: |this: *mut This, out: &mut [u8]| -> (usize, u16) {
            unsafe {
                let this = this.cast::<Self>().as_mut().expect("Got NULL");

                this.reject(out)
            }
        },
        drop: |this: *mut This| unsafe {
            let this = this.cast::<Self>();

            ::core::ptr::drop_in_place(this);
        }
    };
}
impl<T: Viewable> ViewableWithVTable for T {}

pub struct RefMutDynViewable {
    ptr: ptr::NonNull<This>,

    vtable: &'static ViewableVTable,
}

impl RefMutDynViewable {
    pub fn num_items(&mut self) -> Result<u8, ViewError> {
        let to_pic = self.vtable.num_items as usize;
        let picced = unsafe { PIC::manual(to_pic) };
        let ptr: unsafe fn(*mut This) -> Result<u8, ViewError> =
            unsafe { core::mem::transmute(picced) };

        unsafe { (ptr)(self.ptr.as_ptr()) }
    }

    pub fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        let to_pic = self.vtable.render_item as usize;
        let picced = unsafe { PIC::manual(to_pic) };
        let ptr: unsafe fn(
            *mut This,
            item_n: u8,
            title: &mut [u8],
            message: &mut [u8],
            page: u8,
        ) -> Result<u8, ViewError> = unsafe { core::mem::transmute(picced) };

        unsafe { (ptr)(self.ptr.as_ptr(), item_n, title, message, page) }
    }

    pub fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let to_pic = self.vtable.accept as usize;
        let picced = unsafe { PIC::manual(to_pic) };
        let ptr: unsafe fn(*mut This, out: &mut [u8]) -> (usize, u16) =
            unsafe { core::mem::transmute(picced) };

        unsafe { (ptr)(self.ptr.as_ptr(), out) }
    }

    pub fn reject(&mut self, out: &mut [u8]) -> (usize, u16) {
        let to_pic = self.vtable.reject as usize;
        let picced = unsafe { PIC::manual(to_pic) };
        let ptr: unsafe fn(*mut This, out: &mut [u8]) -> (usize, u16) =
            unsafe { core::mem::transmute(picced) };

        unsafe { (ptr)(self.ptr.as_ptr(), out) }
    }

    pub fn drop_item(&mut self) {
        let to_pic = self.vtable.drop as usize;
        let picced = unsafe { PIC::manual(to_pic) };
        let ptr: unsafe fn(*mut This) =
            unsafe { core::mem::transmute(picced) };

        unsafe { (ptr)(self.ptr.as_ptr()) }

    }
}

impl<T: Viewable> From<&'_ mut T> for RefMutDynViewable {
    fn from(it: &mut T) -> Self {
        Self {
            ptr: ptr::NonNull::from(it).cast(),
            vtable: &<T as ViewableWithVTable>::VTABLE,
        }
    }
}

impl Drop for RefMutDynViewable {
    fn drop(&mut self) {
        self.drop_item();
    }
}
