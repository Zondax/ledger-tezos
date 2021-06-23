#[path = "ui/comm.rs"]
mod comm;
pub use comm::*;

static mut OUT: Option<&mut [u8]> = None;

pub fn set_out(buf: &mut [u8]) {
    unsafe {
        let buf: &'static mut [u8] = core::mem::transmute(buf);
        OUT.replace(buf);
    }
}

impl<T: Viewable + Sized> Show for T {
    unsafe fn show(mut self, _: &mut u32) -> Result<(), ()> {
        let out = unsafe { OUT.as_mut() }.ok_or(())?;

        self.accept(out);

        unsafe { OUT.take(); }

        Ok(())
    }
}
