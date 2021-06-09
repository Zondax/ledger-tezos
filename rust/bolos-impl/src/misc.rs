use std::ops::{Deref, DerefMut};

///This struct provides a way to "fake" a lifetime for an owned item
///
/// It becomes useful when you want provide something derived from yourself,
/// but it doesn't have any fields that reference yourself,
/// you also don't want to leak outside a certain scope (the lifetime of yourself, for example)
/// and maybe even still hold a reference to yourself to "enforce" a certain usage
pub struct FakeLifetimeRef<'a, U: 'a, T> {
    item: T,
    source: &'a U,
}

impl<'a, T, U: 'a> FakeLifetimeRef<'a, U, T> {
    pub fn new(source: &'a U, item: T) -> Self {
        Self { item, source }
    }
}

///Same as FakeLifetimeRef, but borrows source mutably
pub struct FakeLifetimeMut<'a, U: 'a, T> {
    item: T,
    source: &'a mut U,
}

impl<'a, T, U: 'a> FakeLifetimeMut<'a, U, T> {
    pub fn new(source: &'a mut U, item: T) -> Self {
        Self { item, source }
    }
}

impl<'a, T, U: 'a> Deref for FakeLifetimeRef<'a, U, T>
where
    T: Deref,
{
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.item.deref()
    }
}

// impl<'a, T, U: 'a> Deref for FakeLifetimeRef<'a, U, T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         &self.item
//     }
// }

impl<'a, T, U: 'a> Deref for FakeLifetimeMut<'a, U, T>
where
    T: Deref,
{
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.item.deref()
    }
}

impl<'a, T, U: 'a> DerefMut for FakeLifetimeMut<'a, U, T>
where
    T: DerefMut,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.item.deref_mut()
    }
}

// impl<'a, T, U: 'a> Deref for FakeLifetimeMut<'a, U, T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         &self.item
//     }
// }

// impl<'a, T, U: 'a> DerefMut for FakeLifetimeMut<'a, U, T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.item
//     }
// }
