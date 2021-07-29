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
use super::{ViewError, Viewable};
use bolos_sys::pic::PIC;

use core::ptr;

use private::This;
mod private {
    pub struct This(());
}

type NumItemsFn = unsafe fn(*mut This) -> Result<u8, ViewError>;
type RenderItemFn = unsafe fn(*mut This, u8, &mut [u8], &mut [u8], u8) -> Result<u8, ViewError>;
type AcceptFn = unsafe fn(*mut This, &mut [u8]) -> (usize, u16);
type RejectFn = unsafe fn(*mut This, &mut [u8]) -> (usize, u16);
type DropFn = unsafe fn(*mut This);

struct ViewableVTable {
    num_items: NumItemsFn,
    render_item: RenderItemFn,
    accept: AcceptFn,
    reject: RejectFn,
    drop: DropFn,
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
        },
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
        let ptr: NumItemsFn = unsafe { core::mem::transmute(picced) };

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
        let ptr: RenderItemFn = unsafe { core::mem::transmute(picced) };

        unsafe { (ptr)(self.ptr.as_ptr(), item_n, title, message, page) }
    }

    pub fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        let to_pic = self.vtable.accept as usize;
        let picced = unsafe { PIC::manual(to_pic) };
        let ptr: AcceptFn = unsafe { core::mem::transmute(picced) };

        unsafe { (ptr)(self.ptr.as_ptr(), out) }
    }

    pub fn reject(&mut self, out: &mut [u8]) -> (usize, u16) {
        let to_pic = self.vtable.reject as usize;
        let picced = unsafe { PIC::manual(to_pic) };
        let ptr: RejectFn = unsafe { core::mem::transmute(picced) };

        unsafe { (ptr)(self.ptr.as_ptr(), out) }
    }

    pub fn drop_item(&mut self) {
        let to_pic = self.vtable.drop as usize;
        let picced = unsafe { PIC::manual(to_pic) };
        let ptr: DropFn = unsafe { core::mem::transmute(picced) };

        unsafe { (ptr)(self.ptr.as_ptr()) }
    }
}

impl<T: Viewable> From<&'_ mut T> for RefMutDynViewable {
    fn from(it: &mut T) -> Self {
        Self {
            ptr: ptr::NonNull::from(it).cast(),
            vtable: PIC::new(&<T as ViewableWithVTable>::VTABLE).into_inner(),
        }
    }
}

impl Drop for RefMutDynViewable {
    fn drop(&mut self) {
        self.drop_item();
    }
}
