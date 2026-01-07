#![no_std]

use core::{error::Error, fmt, marker::PhantomData, ops::Deref};

pub struct ThinRef<'a, T>
where
    T: 'a + ?Sized,
{
    inner: &'a T,
    phantom: PhantomData<&'a T>,
}

// TODO: uncomment after actually making it a thin reference.
// static_assertions::assert_eq_size!(ThinRef<[i32]>, usize);

impl<'a, T> ThinRef<'a, T>
where
    T: 'a,
{
    pub fn new(value: &'a T) -> Self {
        ThinRef {
            inner: value,
            phantom: PhantomData,
        }
    }
}

impl<'a, T> ThinRef<'a, [T]>
where
    [T]: 'a,
{
    pub fn from_slice(value: &'a [T]) -> Self {
        ThinRef {
            inner: value,
            phantom: PhantomData,
        }
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for ThinRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
impl<'a, T: ?Sized + fmt::Display> fmt::Display for ThinRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
impl<'a, T: ?Sized + Error> Error for ThinRef<'a, T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}
impl<'a, T: ?Sized> Deref for ThinRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}
impl<'a, T: ?Sized> Drop for ThinRef<'a, T> {
    fn drop(&mut self) {}
}

unsafe impl<T: ?Sized + Send> Send for ThinRef<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for ThinRef<'_, T> {}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(1 + 1, 2);
    }
}
