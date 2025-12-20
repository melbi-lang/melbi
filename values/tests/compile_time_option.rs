#[derive(Clone)]
struct Cond<const B: bool>;

trait StoragePolicy<T: Clone>: Clone {
    type Inner: Clone;
    fn from_value(value: T) -> Self::Inner;
    fn as_ref(inner: &T) -> Option<T>;
}

impl<T: Clone> StoragePolicy<T> for Cond<false> {
    type Inner = ();

    #[inline(always)]
    fn from_value(_value: T) -> Self::Inner {}

    #[inline(always)]
    fn as_ref(_inner: &T) -> Option<T> {
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
    fn as_ref(inner: &T) -> Option<T> {
        Some(inner.clone())
    }
}

//#[derive(Clone)]
pub struct CompileTimeOption<const STORE: bool, T: Clone> {
    inner: <Cond<STORE> as StoragePolicy<T>>::Inner,
}

impl<const STORE: bool, T: Clone> CompileTimeOption<STORE, T> {
    #[inline(always)]
    pub fn new(value: T) -> Self {
        Self {
            inner: <Cond<STORE> as StoragePolicy<T>>::from_value(value),
        }
    }

    #[inline(always)]
    pub fn get(&self) -> Option<T> {
        <Cond<STORE> as StoragePolicy<T>>::as_ref(&self.inner)
    }
}
