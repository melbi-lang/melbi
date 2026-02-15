//! Thin pointers for arena-allocated unsized types.
//!
//! A `ThinRef<[T]>` is pointer-sized (`usize`), unlike `&[T]` which is two words
//! (pointer + length). This matters when you're storing many references to slices
//! or strings in memory-constrained or cache-sensitive contexts.
//!
//! # Why Thin?
//!
//! Standard Rust slice and string references are "fat pointers" - they carry both
//! a pointer and a length, making them 16 bytes on 64-bit systems. `ThinRef` stores
//! the length inline *before* the data in a single contiguous allocation:
//!
//! ```text
//! ThinRef<[T]>:  ptr ──▶ [len | data...]
//!                        └─────────────┘
//!                        single allocation
//! ```
//!
//! Benefits:
//!
//! - **Half the size**: 8 bytes instead of 16 for slice/string references
//! - **Single allocation**: length and data are allocated together, no indirection
//! - **Cache-friendly**: reading the length prefetches the data into the same cache line
//! - **Simpler data structures**: arrays of `ThinRef` are more compact
//!
//! # Example
//!
//! ```
//! use bumpalo::Bump;
//! use melbi_thin_ref::ThinRef;
//!
//! let arena = Bump::new();
//!
//! // Sized types - just a pointer
//! let num: ThinRef<i32> = ThinRef::new(&arena, 42);
//! assert_eq!(*num, 42);
//!
//! // Slices - thin pointer with inline length
//! let slice: ThinRef<[i32]> = ThinRef::from_slice(&arena, [1, 2, 3]);
//! assert_eq!(slice.len(), 3);
//!
//! // Strings - same as slices
//! let s: ThinRef<str> = ThinRef::from_str(&arena, "hello");
//! assert_eq!(&*s, "hello");
//! ```
//!
//! # Memory Layout
//!
//! | Type | Layout |
//! |------|--------|
//! | Sized `T` | `ptr → T` |
//! | `[T]` | `ptr → [len: usize][data: T...]` |
//! | `str` | `ptr → [len: usize][utf8 bytes...]` |
//!
//! # Gotchas
//!
//! - **No drop**: `Drop` is not called on contained values. Bumpalo arenas don't
//!   run destructors by default. Don't store types that require cleanup.
//! - **Immutable**: No `DerefMut` - values are read-only after creation.

#![no_std]

use core::{alloc::Layout, error::Error, fmt, marker::PhantomData, ops::Deref, ptr::NonNull};

use bumpalo::Bump;

/// A thin pointer to an arena-allocated value.
///
/// Unlike `&T`, `ThinRef<[T]>` and `ThinRef<str>` are pointer-sized (no fat pointer).
/// The length is stored inline before the data.
///
/// See [crate-level docs](crate) for examples.
pub struct ThinRef<'a, T>
where
    T: 'a + ?Sized,
{
    ptr: NonNull<u8>,
    phantom: PhantomData<&'a T>,
}

static_assertions::assert_eq_size!(ThinRef<u128>, usize);
static_assertions::assert_eq_size!(ThinRef<[i32; 3]>, usize);
static_assertions::assert_eq_size!(ThinRef<[i32]>, usize);
static_assertions::assert_eq_size!(ThinRef<str>, usize);

mod private {
    pub trait Sealed {}
}

/// Trait that enables ThinRef to work with different types.
/// This is a sealed trait - it cannot be implemented outside this crate.
pub trait ThinRefTarget: private::Sealed {
    #[doc(hidden)]
    fn deref_inner<'a>(thin: &ThinRef<'a, Self>) -> &'a Self
    where
        Self: 'a;
}

impl<T> private::Sealed for T {}

impl<T> ThinRefTarget for T {
    fn deref_inner<'a>(thin: &ThinRef<'a, Self>) -> &'a Self
    where
        Self: 'a,
    {
        // SAFETY: ptr was created from a valid &T allocated in the arena.
        // The lifetime 'a ensures the arena (and thus the allocation) outlives this reference.
        unsafe { thin.ptr.cast().as_ref() }
    }
}

impl<'a, T> ThinRef<'a, T>
where
    T: 'a,
{
    /// Allocates a sized value in the arena and returns a thin reference to it.
    ///
    /// # Example
    ///
    /// ```
    /// use bumpalo::Bump;
    /// use melbi_thin_ref::ThinRef;
    ///
    /// let arena = Bump::new();
    /// let value: ThinRef<i32> = ThinRef::new(&arena, 42);
    /// assert_eq!(*value, 42);
    /// ```
    pub fn new(arena: &'a Bump, value: T) -> Self {
        let ptr = NonNull::from_ref(arena.alloc(value)).cast();
        ThinRef {
            ptr,
            phantom: PhantomData,
        }
    }
}

impl<T> private::Sealed for [T] {}

impl<T> ThinRefTarget for [T] {
    fn deref_inner<'a>(thin: &ThinRef<'a, Self>) -> &'a Self
    where
        Self: 'a,
    {
        // SAFETY: ptr points to [len: usize][data: T * len] allocated in the arena.
        // The lifetime 'a ensures the arena (and thus the allocation) outlives this reference.
        unsafe {
            let len = *thin.ptr.cast::<usize>().as_ref();
            let (_, slice_offset) = ThinRef::<[T]>::layout(len);
            let data_ptr = thin.ptr.add(slice_offset).cast::<T>();
            core::slice::from_raw_parts(data_ptr.as_ptr(), len)
        }
    }
}

impl<'a, T> ThinRef<'a, [T]>
where
    [T]: 'a,
{
    /// Allocates a slice in the arena from an iterator.
    ///
    /// The iterator must be [`ExactSizeIterator`] so the length is known upfront.
    ///
    /// # Example
    ///
    /// ```
    /// use bumpalo::Bump;
    /// use melbi_thin_ref::ThinRef;
    ///
    /// let arena = Bump::new();
    /// let slice: ThinRef<[i32]> = ThinRef::from_slice(&arena, [1, 2, 3]);
    /// assert_eq!(&*slice, &[1, 2, 3]);
    /// ```
    pub fn from_slice(
        arena: &'a Bump,
        values: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
    ) -> Self {
        let mut iter = values.into_iter();
        let len = iter.len();
        let (layout, slice_offset) = Self::layout(len);

        // SAFETY: arena.alloc_layout returns a valid, non-null, properly aligned pointer.
        let ptr = arena.alloc_layout(layout);

        unsafe {
            // Write the length at the start
            ptr.cast::<usize>().write(len);

            // Write each element at the correct offset
            let data_ptr = ptr.add(slice_offset).cast::<T>();
            for i in 0..len {
                data_ptr
                    .add(i)
                    .write(iter.next().expect("iterator exhausted too early"));
            }
        }

        ThinRef {
            ptr,
            phantom: PhantomData,
        }
    }

    /// Returns the layout and the offset to the slice data.
    fn layout(n: usize) -> (Layout, usize) {
        let (layout, slice_offset) = Layout::new::<usize>()
            .extend(Layout::array::<T>(n).unwrap())
            .unwrap();
        (layout.pad_to_align(), slice_offset)
    }
}

impl private::Sealed for str {}

impl ThinRefTarget for str {
    fn deref_inner<'a>(thin: &ThinRef<'a, Self>) -> &'a Self
    where
        Self: 'a,
    {
        // SAFETY: ptr points to [len: usize][utf8 bytes] allocated in the arena.
        // The bytes are valid UTF-8 because they were copied from a valid &str.
        // The lifetime 'a ensures the arena (and thus the allocation) outlives this reference.
        unsafe {
            let len = *thin.ptr.cast::<usize>().as_ref();
            let (_, bytes_offset) = ThinRef::<str>::layout(len);
            let data_ptr = thin.ptr.add(bytes_offset);
            let bytes = core::slice::from_raw_parts(data_ptr.as_ptr(), len);
            core::str::from_utf8_unchecked(bytes)
        }
    }
}

impl<'a> ThinRef<'a, str> {
    /// Allocates a string in the arena by copying the given `&str`.
    ///
    /// # Example
    ///
    /// ```
    /// use bumpalo::Bump;
    /// use melbi_thin_ref::ThinRef;
    ///
    /// let arena = Bump::new();
    /// let s: ThinRef<str> = ThinRef::from_str(&arena, "hello");
    /// assert_eq!(&*s, "hello");
    /// ```
    pub fn from_str(arena: &'a Bump, value: &str) -> Self {
        let bytes = value.as_bytes();
        let len = bytes.len();
        let (layout, bytes_offset) = Self::layout(len);

        // SAFETY: arena.alloc_layout returns a valid, non-null, properly aligned pointer.
        let ptr = arena.alloc_layout(layout);

        unsafe {
            // Write the length at the start
            ptr.cast::<usize>().write(len);

            // Copy the UTF-8 bytes
            let data_ptr = ptr.add(bytes_offset);
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), data_ptr.as_ptr(), len);
        }

        ThinRef {
            ptr,
            phantom: PhantomData,
        }
    }

    /// Returns the layout and the offset to the string bytes.
    fn layout(n: usize) -> (Layout, usize) {
        let (layout, bytes_offset) = Layout::new::<usize>()
            .extend(Layout::array::<u8>(n).unwrap())
            .unwrap();
        (layout.pad_to_align(), bytes_offset)
    }
}

impl<T: ?Sized> Clone for ThinRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: ?Sized> Copy for ThinRef<'_, T> {}
impl<'a, T: ?Sized + ThinRefTarget + fmt::Debug> fmt::Debug for ThinRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
impl<'a, T: ?Sized + ThinRefTarget + fmt::Display> fmt::Display for ThinRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
impl<'a, T: ?Sized + ThinRefTarget + Error> Error for ThinRef<'a, T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (**self).source()
    }
}
impl<'a, T: ?Sized + ThinRefTarget> Deref for ThinRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        T::deref_inner(self)
    }
}

impl<T> AsRef<[T]> for ThinRef<'_, [T]> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

// Important: use correct semantics for references.
unsafe impl<T: ?Sized + Sync> Send for ThinRef<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for ThinRef<'_, T> {}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;

    use alloc::boxed::Box;
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

    #[test]
    fn new_unit_type() {
        let arena = Bump::new();
        let thin: ThinRef<()> = ThinRef::new(&arena, ());
        assert_eq!(size_of_val(&thin), 8);
        assert_eq!(*thin, ());

        let b = Box::new(());
        assert_eq!(size_of_val(&b), 8);
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
    fn from_slice_array_align16_iterator() {
        let arena = Bump::new();
        let thin: ThinRef<[u128]> = ThinRef::from_slice(&arena, [10]);
        assert_eq!(&*thin, &[10]);

        // Layout: [usize:8|[u64:8, u64:8]]
        assert_eq!(ThinRef::<[u64]>::layout(2).0.size(), 24);

        // Layout: [usize:8|_:8|[u128:16]] where _ is padding.
        assert_eq!(ThinRef::<[u128]>::layout(1).0.size(), 32);
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
    // AsRef tests
    // ===================

    #[test]
    fn as_ref_slice() {
        let arena = Bump::new();
        let thin: ThinRef<[i32]> = ThinRef::from_slice(&arena, [10, 20, 30]);
        let slice: &[i32] = thin.as_ref();
        assert_eq!(slice, &[10, 20, 30]);
    }

    #[test]
    fn as_ref_empty_slice() {
        let arena = Bump::new();
        let thin: ThinRef<[i32]> = ThinRef::from_slice(&arena, []);
        let slice: &[i32] = thin.as_ref();
        assert!(slice.is_empty());
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

    // =========================
    // ThinRef::from_str() tests
    // =========================

    #[test]
    fn from_str_basic() {
        let arena = Bump::new();
        let thin: ThinRef<str> = ThinRef::from_str(&arena, "hello");
        assert_eq!(&*thin, "hello");
    }

    #[test]
    fn from_str_empty() {
        let arena = Bump::new();
        let thin: ThinRef<str> = ThinRef::from_str(&arena, "");
        assert!(thin.is_empty());
        assert_eq!(thin.len(), 0);
        assert_eq!(&*thin, "");
    }

    #[test]
    fn from_str_unicode_multibyte() {
        let arena = Bump::new();

        // Japanese hiragana
        let thin: ThinRef<str> = ThinRef::from_str(&arena, "こんにちは");
        assert_eq!(&*thin, "こんにちは");

        // Emoji
        let thin2: ThinRef<str> = ThinRef::from_str(&arena, "🦀🎉✨");
        assert_eq!(&*thin2, "🦀🎉✨");

        // Mixed ASCII and multi-byte
        let thin3: ThinRef<str> = ThinRef::from_str(&arena, "hello世界!");
        assert_eq!(&*thin3, "hello世界!");
    }

    #[test]
    fn from_str_len_vs_chars() {
        let arena = Bump::new();

        // ASCII: len == char count
        let ascii: ThinRef<str> = ThinRef::from_str(&arena, "abc");
        assert_eq!(ascii.len(), 3);
        assert_eq!(ascii.chars().count(), 3);

        // Multi-byte: len (bytes) > char count
        let unicode: ThinRef<str> = ThinRef::from_str(&arena, "日本");
        assert_eq!(unicode.len(), 6); // 2 chars * 3 bytes each
        assert_eq!(unicode.chars().count(), 2);
    }

    #[test]
    fn from_str_deref_methods() {
        let arena = Bump::new();
        let thin: ThinRef<str> = ThinRef::from_str(&arena, "  hello world  ");

        // Various str methods via Deref
        assert_eq!(thin.len(), 15);
        assert!(!thin.is_empty());
        assert!(thin.contains("hello"));
        assert!(thin.starts_with("  hello"));
        assert!(thin.ends_with("world  "));
        assert_eq!(thin.trim(), "hello world");

        let chars: alloc::vec::Vec<char> = thin.chars().take(5).collect();
        assert_eq!(chars, vec![' ', ' ', 'h', 'e', 'l']);
    }

    #[test]
    fn from_str_debug_format() {
        let arena = Bump::new();
        let thin: ThinRef<str> = ThinRef::from_str(&arena, "test");
        // Debug for str includes quotes
        assert_eq!(format!("{:?}", thin), "\"test\"");
    }

    #[test]
    fn from_str_display_format() {
        let arena = Bump::new();
        let thin: ThinRef<str> = ThinRef::from_str(&arena, "hello world");
        // Display for str does not include quotes
        assert_eq!(thin.to_string(), "hello world");
    }

    #[test]
    fn send_sync_str() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<ThinRef<str>>();
        assert_sync::<ThinRef<str>>();
    }
}
