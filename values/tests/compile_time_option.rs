#[derive(Clone)]
struct Cond<const B: bool>;

pub trait StoragePolicy<T: Clone>: Clone {
    type Inner: Clone;
    fn from_value(value: T) -> Self::Inner;
    fn to_option(inner: &Self::Inner) -> Option<T>;
}

impl<T: Clone> StoragePolicy<T> for Cond<false> {
    type Inner = ();

    #[inline(always)]
    fn from_value(_value: T) -> Self::Inner {}

    #[inline(always)]
    fn to_option(_inner: &()) -> Option<T> {
        None
    }
}

impl<T: Clone> StoragePolicy<T> for Cond<true> {
    type Inner = T;

    #[inline(always)]
    fn from_value(value: T) -> Self::Inner {
        value
    }

    #[inline(always)]
    fn to_option(inner: &T) -> Option<T> {
        Some(inner.clone())
    }
}

/// A compile-time optional value that stores a value of type `T` based `STORE`.
///
/// `T` is assumed to support inexpensive cloning, like a reference or `Rc`.
///
/// When `STORE` is `true`, the value is stored and can be retrieved via `get()`.
/// When `STORE` is `false`, the value is discarded and the struct is zero-sized.
///
/// # Examples
///
/// ```
/// // Enabled: stores the value
/// let opt: CompileTimeOption<true, i32> = CompileTimeOption::new(42);
/// assert_eq!(opt.get(), Some(42));
///
/// // Disabled: zero-sized, returns None
/// let opt: CompileTimeOption<false, i32> = CompileTimeOption::new(42);
/// assert_eq!(opt.get(), None);
/// ```
#[derive(Clone)]
#[allow(private_bounds)]
pub struct CompileTimeOption<const STORE: bool, T: Clone>
where
    Cond<STORE>: StoragePolicy<T>,
{
    inner: <Cond<STORE> as StoragePolicy<T>>::Inner,
}

#[allow(private_bounds)]
impl<const STORE: bool, T: Clone> CompileTimeOption<STORE, T>
where
    Cond<STORE>: StoragePolicy<T>,
{
    #[inline(always)]
    pub fn new(value: T) -> Self {
        Self {
            inner: <Cond<STORE> as StoragePolicy<T>>::from_value(value),
        }
    }

    #[inline(always)]
    pub fn get(&self) -> Option<T> {
        <Cond<STORE> as StoragePolicy<T>>::to_option(&self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn enabled_stores_and_returns_value() {
        let opt: CompileTimeOption<true, i32> = CompileTimeOption::new(42);
        assert_eq!(opt.get(), Some(42));
    }

    #[test]
    fn disabled_returns_none() {
        let opt: CompileTimeOption<false, i32> = CompileTimeOption::new(42);
        assert_eq!(opt.get(), None);
    }

    #[test]
    fn disabled_is_zero_sized() {
        assert_eq!(size_of::<CompileTimeOption<false, i32>>(), 0);
        assert_eq!(size_of::<CompileTimeOption<false, String>>(), 0);
    }

    #[test]
    fn enabled_has_same_size_as_inner() {
        assert_eq!(size_of::<CompileTimeOption<true, i32>>(), size_of::<i32>());
        assert_eq!(
            size_of::<CompileTimeOption<true, String>>(),
            size_of::<String>()
        );
    }

    #[test]
    fn clone_works() {
        let opt: CompileTimeOption<true, i32> = CompileTimeOption::new(42);
        let cloned = opt.clone();
        assert_eq!(cloned.get(), Some(42));

        let opt: CompileTimeOption<false, i32> = CompileTimeOption::new(42);
        let cloned = opt.clone();
        assert_eq!(cloned.get(), None);
    }
}
