//! This module contains a struct to handle wear levelling for flash memory

use std::{cmp::Ordering, ops::Deref};

use crate::{NVM, PIC};

const USIZE: usize = std::mem::size_of::<usize>();

//only useful for optimized write
#[repr(C)]
struct NVMWearSlotRam<const SIZE: usize> {
    pub counter: [u8; USIZE],
    pub slot: [u8; SIZE],
}

impl<const S: usize> From<&NVMWearSlot<S>> for NVMWearSlotRam<S> {
    fn from(s: &NVMWearSlot<S>) -> Self {
        let counter = *s.counter.read();
        let slot = *s.slot.read();

        Self { counter, slot }
    }
}

impl<const S: usize> NVMWearSlotRam<S> {
    fn as_slice(&self) -> &[u8] {
        let s = (&self.counter[..]).as_ptr();

        unsafe {
            std::slice::from_raw_parts(s, S + USIZE)
        }
    }
}

#[derive(Copy, Clone, Eq)]
#[repr(C)]
pub struct NVMWearSlot<const SIZE: usize> {
    counter: NVM<USIZE>,
    slot: NVM<SIZE>,
}

impl<const S: usize> Ord for NVMWearSlot<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.counter().cmp(&other.counter())
    }
}

impl<const S: usize> PartialOrd for NVMWearSlot<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<const S: usize> PartialEq for NVMWearSlot<S> {
    fn eq(&self, other: &Self) -> bool {
        self.counter() == other.counter()
    }
}

impl<const S: usize> NVMWearSlot<S> {
    pub const fn new() -> Self {
        Self {
            slot: NVM::new(),
            counter: NVM::new(),
        }
    }

    fn counter(&self) -> usize {
        usize::from_be_bytes(*self.counter)
    }

    /// This write is optimized to avoid writing to flash twice, but might not work always...
    ///
    // TODO: test VERY thoroughly!! also remove the attribute and `unsafe` modifier when it's been tested
    #[allow(unused_unsafe)]
    pub unsafe fn optimized_write(&mut self, from: usize, slice: &[u8]) -> Result<(), ()> {
        let len = slice.len();

        //if write won't fit then error
        if from + len > self.slot.len() {
            return Err(());
        }

        let counter = (self.counter() + 1).to_be_bytes();

        //this is a temporary location to write our changes
        // so we can use this later to write to ourselves thru nvm
        let mut src: NVMWearSlotRam<S> = (&*self).into();
        src.counter = counter;
        src.slot[from..from+len].copy_from_slice(slice);

        //safety: this is safe because we only use the mutable ref inside ManualNVM
        // where it's meant to be used
        let p = unsafe { self.counter.get_mut() };
        let p: std::ptr::NonNull<_> = (&mut p[0]).into();

        let mut nvm = crate::nvm::ManualNVM::new(p, USIZE + S);

        //safety: this is safe because it comes from NVM in the first place (self)
        unsafe {
            nvm.write(0, src.as_slice())
        }
    }

    /// Write `slice` to the inner slot, starting at `from`.
    ///
    /// # Warning
    /// This will write to NVM twice! Once for the `slice` and once to update
    /// the wear counter!
    pub fn write(&mut self, from: usize, slice: &[u8]) -> Result<(), ()> {
        self.slot.write(from, slice)?;

        let mut cnt = self.counter();
        cnt += 1;

        //this can't fail as we prepare the data correctly
        self.counter.write(0, &cnt.to_be_bytes()).unwrap();

        Ok(())
    }
}

impl<const S: usize> Deref for NVMWearSlot<S> {
    type Target = [u8; S];

    fn deref(&self) -> &Self::Target {
        &self.slot
    }
}

pub struct Wear<'s, 'm, const SLOTS: usize, const SLOT_SIZE: usize> {
    slots: &'s mut PIC<[NVMWearSlot<SLOT_SIZE>; SLOTS]>,
    idx: &'m mut usize,
}

impl<'s, 'm, const S: usize, const SS: usize> Wear<'s, 'm, S, SS> {
    pub fn new(slots: &'s mut PIC<[NVMWearSlot<SS>; S]>, idx: &'m mut usize) -> Self {
        let mut me = Self { slots, idx };
        me.align();

        me
    }

    /// Increments idx staying in the bounds of the slot
    fn inc(&mut self) {
        if *self.idx == S - 1 {
            *self.idx = 0;
        } else {
            *self.idx += 1;
        }
    }

    /// Aligns `idx` to the correct position on the tape
    ///
    /// This is most useful when `slots` is not blank data
    fn align(&mut self) {
        *self.idx = self.find_eldest_idx();
    }

    /// Retrieves the next slot to write, which should be also the youngest
    ///
    /// Will wrap when the end has been reached
    pub fn next(&mut self) -> &mut NVMWearSlot<SS> {
        self.inc();
        &mut self.slots.get_mut()[*self.idx]
    }

    /// Retrieves the last written slot, which should also be the oldest
    pub fn prev(&self) -> &NVMWearSlot<SS> {
        &self.slots[*self.idx]
    }

    /// Retrieves the slot index with the most writes
    fn find_eldest_idx(&self) -> usize {
        self.slots
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .unwrap()
            .0
    }

    /// Retrieves the slot with the most writes
    pub fn find_eldest(&mut self) -> &mut NVMWearSlot<SS> {
        self.slots.iter_mut().max().unwrap()
    }

    /// Retrieves the slot index with the least writes
    fn find_youngest_idx(&self) -> usize {
        self.slots
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.cmp(b))
            .unwrap()
            .0
    }

    /// Retrieves the slot index with the least writes
    fn find_youngest(&mut self) -> &mut NVMWearSlot<SS> {
        self.slots.iter_mut().min().unwrap()
    }
}

#[macro_export]
macro_rules! new_wear_leveller {
    ($slot_size:expr, $slots:expr) => {{
        #[$crate::pic]
        static mut __SLOTS: [$crate::wear_leveller::NVMWearSlot<$slot_size>; $slots] =
            [$crate::wear_leveller::NVMWearSlot::new(); $slots];

        #[$crate::pic]
        static mut __IDX: usize = 0;

        unsafe { $crate::wear_leveller::Wear::new(&mut __SLOTS, &mut __IDX) }
    }};
}
