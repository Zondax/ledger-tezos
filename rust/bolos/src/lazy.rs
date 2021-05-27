//! This module provides a wrapper around `once_cell`'s `Lazy` struct
//!
//! This wrapper makes sure the stored closure is used thru PIC as to not go out of memory bounds

use super::PIC;

use once_cell::unsync::OnceCell;
use std::cell::Cell;
use std::ops::{Deref, DerefMut};

pub struct Lazy<T, F = fn() -> T> {
    cell: OnceCell<T>,
    init: Cell<Option<PIC<F>>>,
}

impl<T, F> Lazy<T, F> {
    pub const fn new(init: F) -> Lazy<T, F> {
        Self {
            cell: OnceCell::new(),
            init: Cell::new(Some(PIC::new(init))),
        }
    }

    pub const fn pic(init: F) -> PIC<Lazy<T, F>> {
        PIC::new(Lazy::new(init))
    }

    pub fn into_value(this: Lazy<T, F>) -> Result<T, PIC<F>> {
        let cell = this.cell;
        let init = this.init;
        cell.into_inner().ok_or_else(|| {
            init.take()
                .unwrap_or_else(|| panic!("Lazy instance has previously been poisoned"))
        })
    }
}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    pub fn force(this: &Lazy<T, F>) -> &T {
        this.cell.get_or_init(|| match this.init.take() {
            Some(f) => f.into_inner()(),
            None => panic!("Lazy instance has previously been poisoned"),
        })
    }
}

impl<T, F: FnOnce() -> T> Deref for Lazy<T, F> {
    type Target = T;
    fn deref(&self) -> &T {
        Lazy::force(self)
    }
}

impl<T, F: FnOnce() -> T> DerefMut for Lazy<T, F> {
    fn deref_mut(&mut self) -> &mut T {
        Lazy::force(self);
        self.cell.get_mut().unwrap_or_else(|| unreachable!())
    }
}

impl<T: Default> Default for Lazy<T> {
    /// Creates a new lazy value using `Default` as the initializing function.
    fn default() -> Lazy<T> {
        Lazy::new(T::default)
    }
}
