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
//! This module contains a struct to handle wear levelling for flash memory
use crate::{nvm::NVMError, NVM, PIC};

pub const PAGE_SIZE: usize = 64;
const COUNTER_SIZE: usize = std::mem::size_of::<u64>();
const CRC_SIZE: usize = std::mem::size_of::<u32>();

pub const SLOT_SIZE: usize = PAGE_SIZE - COUNTER_SIZE - CRC_SIZE;

pub const ZEROED_STORAGE: [u8; PAGE_SIZE] = Slot::zeroed().as_storage();

pub(self) struct Slot<'nvm> {
    pub counter: u64,
    payload: &'nvm [u8; SLOT_SIZE],
    crc: u32,
}

#[cfg_attr(any(feature = "derive-debug", test), derive(Debug))]
enum SlotError {
    Crc { expected: u32, found: u32 },
}

impl<'nvm> Slot<'nvm> {
    fn crc32(counter: u64, payload: &[u8; SLOT_SIZE]) -> u32 {
        use crc::crc32::*;

        let mut digest = Digest::new(IEEE);
        digest.write(&counter.to_be_bytes()[..]);
        digest.write(&payload[..]);

        digest.sum32()
    }

    pub const fn zeroed() -> Slot<'static> {
        const PAYLOAD_ZERO: [u8; SLOT_SIZE] = [0; SLOT_SIZE];
        const CRC_ZERO: u32 = 0x4128908;

        Slot {
            counter: 0,
            payload: &PAYLOAD_ZERO,
            crc: CRC_ZERO,
        }
    }

    pub fn from_storage(storage: &'nvm [u8; PAGE_SIZE]) -> Result<Self, SlotError> {
        let cnt = {
            let mut array = [0; COUNTER_SIZE];
            array.copy_from_slice(&storage[..COUNTER_SIZE]);
            u64::from_be_bytes(array)
        };

        //safety: this is safe because we are reinrepreting a reference so we uphold
        // borrow checker rules
        // also the size matches
        let payload = &storage[COUNTER_SIZE..COUNTER_SIZE + SLOT_SIZE];
        let payload = unsafe { &*(payload.as_ptr() as *const [u8; SLOT_SIZE]) };

        let crc = {
            let mut array = [0; CRC_SIZE];
            array.copy_from_slice(&storage[COUNTER_SIZE + SLOT_SIZE..]);
            u32::from_be_bytes(array)
        };

        let expected = Self::crc32(cnt, payload);
        if crc != expected {
            return Err(SlotError::Crc {
                expected,
                found: crc,
            });
        }

        Ok(Slot {
            counter: cnt,
            crc,
            payload,
        })
    }

    pub const fn as_storage(&self) -> [u8; PAGE_SIZE] {
        let counter = self.counter.to_be_bytes();
        let crc = self.crc.to_be_bytes();

        let mut storage = [0; PAGE_SIZE];

        //storage[..COUNTER_SIZE].copy_from_slice(&counter);
        {
            let mut i = 0;
            while i < COUNTER_SIZE {
                storage[i] = counter[i];
                i += 1;
            }
        }
        //storage[COUNTER_SIZE..COUNTER_SIZE + SLOT_SIZE].copy_from_slice(&self.payload[..])
        {
            let mut i = 0;
            while i < SLOT_SIZE {
                storage[COUNTER_SIZE + i] = self.payload[i];
                i += 1;
            }
        }
        //storage[COUNTER_SIZE + SLOT_SIZE..].copy_from_slice(&crc);
        {
            let mut i = 0;
            while i < CRC_SIZE {
                storage[COUNTER_SIZE + SLOT_SIZE + i] = crc[i];
                i += 1;
            }
        }

        storage
    }

    pub fn modify<'new>(&self, payload: &'new [u8; SLOT_SIZE], counter: u64) -> Slot<'new> {
        let crc = Self::crc32(counter, payload);

        Slot {
            counter,
            payload,
            crc,
        }
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct NVMWearSlot {
    storage: NVM<64>,
}

#[derive(PartialEq)]
#[cfg_attr(any(feature = "derive-debug", test), derive(Debug))]
pub enum WearError {
    Crc { expected: u32, found: u32 },
    NVMWrite,
    Uninitialized,
}

impl NVMWearSlot {
    pub const fn new() -> Self {
        Self {
            storage: NVM::zeroed(),
        }
    }

    pub fn with_baking<const ARRAY_SIZE: usize, const BYTES: usize>(
        storage: &mut PIC<NVM<BYTES>>,
    ) -> &mut PIC<[NVMWearSlot; ARRAY_SIZE]> {
        //we need to make sure we passed the right details
        assert_eq!(BYTES, 64 * ARRAY_SIZE);

        let storage: *mut _ = storage;
        //Safety: this is ok because the memory layout is the same, since PIC is transparent
        // as well as NVMWearSlot
        unsafe {
            match storage.cast::<PIC<[NVMWearSlot; ARRAY_SIZE]>>().as_mut() {
                Some(ptr) => ptr, //impossible to fail, the source is a reference
                None => core::hint::unreachable_unchecked(),
            }
        }
    }

    pub(self) fn as_slot(&self) -> Result<Slot<'_>, WearError> {
        Slot::from_storage(&self.storage)
            .map_err(|SlotError::Crc { expected, found }| WearError::Crc { expected, found })
    }

    /// Clears out all data on the slot
    ///
    /// Will reset the counter and the CRC to a valid zero state
    pub(self) fn format(&mut self) -> Result<(), WearError> {
        self.write([0; SLOT_SIZE], 0)
    }

    /// Reads the payload of the slot (if valid)
    #[allow(dead_code)]
    pub(self) fn read(&self) -> Result<&[u8; SLOT_SIZE], WearError> {
        self.as_slot().map(|s| s.payload)
    }

    /// Write `slice` to the inner slot
    pub(self) fn write(&mut self, write: [u8; SLOT_SIZE], counter: u64) -> Result<(), WearError> {
        let storage = Slot::zeroed().modify(&write, counter).as_storage();

        self.storage.write(0, &storage).map_err(|e| match e {
            NVMError::Internal(_) => WearError::NVMWrite,
            _ => unreachable!("size is checked already"),
        })
    }
}

pub struct Wear<'s, const SLOTS: usize> {
    slots: &'s mut PIC<[NVMWearSlot; SLOTS]>,
    idx: u64,
}

impl<'s, const S: usize> Wear<'s, S> {
    pub fn new(slots: &'s mut PIC<[NVMWearSlot; S]>) -> Result<Self, WearError> {
        let mut me = Self { slots, idx: 0 };
        me.align()?;

        Ok(me)
    }

    /// Aligns `idx` to the correct position on the tape
    ///
    /// This is most useful when `slots` is not blank data
    fn align(&mut self) -> Result<(), WearError> {
        let mut max = Slot::zeroed();

        for slot in self.slots.iter() {
            let slot = slot.as_slot()?;
            if slot.counter > max.counter {
                max = slot;
            }
        }

        self.idx = max.counter;
        Ok(())
    }

    const fn idx(&self) -> usize {
        (self.idx % (S as u64)) as _
    }

    /// Clears out all information in `Wear`
    ///
    /// Will reset all wear counters and uninitialize all data
    pub fn format(&mut self) -> Result<(), WearError> {
        for s in self.slots.get_mut().iter_mut() {
            s.format()?;
        }
        self.idx = 0;

        Ok(())
    }

    /// Retrieves the next slot to write, which should be also the youngest
    ///
    /// Will wrap when the end has been reached
    pub fn write(&mut self, payload: [u8; SLOT_SIZE]) -> Result<(), WearError> {
        self.idx += 1;

        let idx = self.idx();
        let slot = &mut self.slots.get_mut()[idx];
        slot.write(payload, self.idx)?;

        Ok(())
    }

    /// Retrieves the last written slot, which should also be the oldest
    pub fn read(&self) -> Result<&[u8; SLOT_SIZE], WearError> {
        //will only return CRC error
        let slot = self.slots.get_ref()[self.idx()].as_slot()?;

        if slot.counter == 0 {
            Err(WearError::Uninitialized)
        } else {
            Ok(slot.payload)
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl<'s, const S: usize> Wear<'s, S> {
    pub fn counter(&mut self) -> &mut u64 {
        &mut self.idx
    }

    pub fn slots(&mut self) -> &mut [NVMWearSlot; S] {
        &mut *self.slots.get_mut()
    }
}

#[macro_export]
macro_rules! new_flash_slot {
    ($slots:expr) => {{
        use $crate::flash_slot::{NVMWearSlot, Wear, PAGE_SIZE, ZEROED_STORAGE};

        const SLOTS: usize = $slots;
        const BYTES: usize = SLOTS * PAGE_SIZE;

        #[$crate::nvm]
        static mut __BAKING_STORAGE: [[u8; PAGE_SIZE]; SLOTS] = ZEROED_STORAGE;

        unsafe {
            Wear::new(NVMWearSlot::with_baking::<$slots, BYTES>(
                &mut __BAKING_STORAGE,
            ))
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_works() {
        let mut wear = new_flash_slot!(2).expect("no nvm/crc issues");

        assert_eq!(0, wear.idx());
        assert_eq!(2, wear.slots().len())
    }

    #[test]
    fn idx_increase() {
        let mut wear = new_flash_slot!(5).expect("no nvm/crc issues");

        wear.write([42; SLOT_SIZE]).expect("no nvm issues");
        assert_eq!(1, wear.idx());
    }

    #[test]
    fn idx_loop() {
        let mut wear = new_flash_slot!(2).expect("no nvm/crc issues");
        assert_eq!(0, wear.idx());

        wear.write([42; SLOT_SIZE]).expect("no nvm issues");
        assert_eq!(1, wear.idx());
        wear.write([24; SLOT_SIZE]).expect("no nvm issues");
        assert_eq!(0, wear.idx());
    }

    #[test]
    fn counter_increase() {
        let mut wear = new_flash_slot!(2).expect("no nvm/crc issues");
        assert_eq!(0, *wear.counter());

        wear.write([0; SLOT_SIZE]).expect("no nvm issues");
        wear.write([1; SLOT_SIZE]).expect("no nvm issues");
        wear.write([2; SLOT_SIZE]).expect("no nvm issues");
        assert_eq!(3, *wear.counter());
        assert_eq!(1, wear.idx());
    }

    #[test]
    fn read_back() {
        let mut wear = new_flash_slot!(1).expect("no nvm/crc issues");

        const MSG: [u8; SLOT_SIZE] = [42; SLOT_SIZE];

        wear.write(MSG).expect("no nvm issues");
        assert_eq!(&MSG, wear.read().expect("no nvm/crc issues"));
    }

    #[test]
    fn no_uninitialized_read() {
        let wear = new_flash_slot!(1).expect("no nvm/crc issues");

        wear.read()
            .expect_err("can't read without writing once first");
    }

    #[test]
    fn format() {
        let mut wear = new_flash_slot!(1).expect("no nvm/crc issues");

        wear.write([42; SLOT_SIZE]).expect("no nvm issues");
        wear.write([24; SLOT_SIZE]).expect("no nvm issues");
        assert_eq!(2, *wear.counter());

        wear.format().expect("no nvm issues");
        assert_eq!(0, *wear.counter());
        wear.read()
            .expect_err("can't read without writing once first");
    }
}
