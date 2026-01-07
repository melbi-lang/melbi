#![no_std]

use core::{error::Error, fmt, marker::PhantomData, ops::Deref};

use bumpalo::Bump;

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
    pub fn new(arena: &'a Bump, value: T) -> Self {
        ThinRef {
            inner: arena.alloc(value),
            phantom: PhantomData,
        }
    }
}

impl<'a, T> ThinRef<'a, [T]>
where
    [T]: 'a,
{
    pub fn from_slice(
        arena: &'a Bump,
        values: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
    ) -> Self {
        ThinRef {
            inner: arena.alloc_slice_fill_iter(values),
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
    extern crate alloc;
    extern crate std;

    use alloc::format;
    use alloc::string::ToString;
    use alloc::vec;
    use bumpalo::Bump;
    use std::fmt;

    use super::ThinRef;

    // ===================
    // ThinRef::new() tests
    // ===================

    #[test]
    fn new_i32_deref() {
        let arena = Bump::new();
        let thin = ThinRef::new(&arena, 42i32);
        assert_eq!(*thin, 42);
    }

    #[test]
    fn new_struct_field_access() {
        #[derive(Debug, PartialEq)]
        struct Point {
            x: i32,
            y: i32,
        }

        let arena = Bump::new();
        let thin = ThinRef::new(&arena, Point { x: 10, y: 20 });
        assert_eq!(thin.x, 10);
        assert_eq!(thin.y, 20);
        assert_eq!(*thin, Point { x: 10, y: 20 });
    }

    // =========================
    // ThinRef::from_slice() tests
    // =========================

    #[test]
    fn from_slice_vec_iterator() {
        let arena = Bump::new();
        let thin: ThinRef<[i32]> = ThinRef::from_slice(&arena, vec![1, 2, 3]);
        assert_eq!(&*thin, &[1, 2, 3]);
    }

    #[test]
    fn from_slice_array_iterator() {
        let arena = Bump::new();
        let thin: ThinRef<[i32]> = ThinRef::from_slice(&arena, [10, 20, 30]);
        assert_eq!(&*thin, &[10, 20, 30]);
    }

    #[test]
    fn from_slice_empty() {
        let arena = Bump::new();
        let thin: ThinRef<[i32]> = ThinRef::from_slice(&arena, []);
        assert!(thin.is_empty());
        assert_eq!(thin.len(), 0);
    }

    #[test]
    fn from_slice_indexing() {
        let arena = Bump::new();
        let thin: ThinRef<[i32]> = ThinRef::from_slice(&arena, [100, 200, 300]);
        assert_eq!(thin[0], 100);
        assert_eq!(thin[1], 200);
        assert_eq!(thin[2], 300);
    }

    #[test]
    fn from_slice_len() {
        let arena = Bump::new();
        let thin: ThinRef<[i32]> = ThinRef::from_slice(&arena, [1, 2, 3, 4, 5]);
        assert_eq!(thin.len(), 5);
    }

    // ===================
    // Debug and Display tests
    // ===================

    #[test]
    fn debug_sized_type() {
        let arena = Bump::new();
        let thin = ThinRef::new(&arena, 42i32);
        assert_eq!(format!("{:?}", thin), "42");
    }

    #[test]
    fn debug_slice() {
        let arena = Bump::new();
        let thin: ThinRef<[i32]> = ThinRef::from_slice(&arena, [1, 2, 3]);
        assert_eq!(format!("{:?}", thin), "[1, 2, 3]");
    }

    #[test]
    fn display_i32() {
        let arena = Bump::new();
        let thin = ThinRef::new(&arena, 42i32);
        assert_eq!(thin.to_string(), "42");
    }

    #[test]
    fn display_custom_type() {
        struct Greeting(&'static str);
        impl fmt::Display for Greeting {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "Hello, {}!", self.0)
            }
        }

        let arena = Bump::new();
        let thin = ThinRef::new(&arena, Greeting("World"));
        assert_eq!(thin.to_string(), "Hello, World!");
    }

    // ================================
    // Multiple ThinRefs from same arena
    // ================================

    #[test]
    fn multiple_thinrefs_same_arena() {
        let arena = Bump::new();

        let a = ThinRef::new(&arena, 1i32);
        let b = ThinRef::new(&arena, 2i32);
        let c: ThinRef<[i32]> = ThinRef::from_slice(&arena, [10, 20]);

        // All should remain valid and independent
        assert_eq!(*a, 1);
        assert_eq!(*b, 2);
        assert_eq!(&*c, &[10, 20]);
    }

    // ==========================
    // Send + Sync static assertions
    // ==========================

    #[test]
    fn send_sync_sized() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<ThinRef<i32>>();
        assert_sync::<ThinRef<i32>>();
    }

    #[test]
    fn send_sync_slice() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<ThinRef<[i32]>>();
        assert_sync::<ThinRef<[i32]>>();
    }
}
