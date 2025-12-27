use core::num::NonZeroU8;
use core::{hash::Hash, marker::PhantomData, mem, ptr, slice};

// === SimpleSmallStr ===

#[cfg(target_pointer_width = "32")]
type SimpleSliceSize = u16;

#[cfg(target_pointer_width = "64")]
type SimpleSliceSize = u32;

const SIMPLE_INLINE_SIZE: usize = 2 * mem::size_of::<usize>() - 2;

pub enum SimpleSmallStr<'a> {
    Slice(SimpleSliceSize, ptr::NonNull<u8>, PhantomData<&'a str>),
    Inline(u8, [u8; SIMPLE_INLINE_SIZE]),
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static_assertions::assert_eq_size!(SimpleSmallStr, (usize, usize));

// === StrSliceRepr ===

#[repr(C)]
#[derive(Clone, Copy)]
#[cfg(target_endian = "big")]
struct StrSliceRepr {
    len: usize,
    ptr: ptr::NonNull<u8>,
}

#[repr(C)]
#[derive(Clone, Copy)]
#[cfg(target_endian = "little")]
struct StrSliceRepr {
    ptr: ptr::NonNull<u8>,
    len: usize,
}

impl From<&str> for StrSliceRepr {
    fn from(s: &str) -> Self {
        let len = s.len();
        let ptr = ptr::NonNull::new(s.as_ptr() as *mut u8).unwrap();
        Self { len, ptr }
    }
}

impl AsRef<str> for StrSliceRepr {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(slice::from_raw_parts(self.ptr.as_ptr(), self.len)) }
    }
}

// === StrInlineRepr ===

const INLINE_SIZE: usize = mem::size_of::<StrSliceRepr>() - 1;

#[repr(C)]
#[derive(Clone, Copy)]
#[cfg(target_endian = "big")]
struct StrInlineRepr {
    tag_len: NonZeroU8, // Actually: (len << 1) | (1 << 7)
    data: [u8; INLINE_SIZE],
}

#[repr(C)]
#[derive(Clone, Copy)]
#[cfg(target_endian = "little")]
struct StrInlineRepr {
    data: [u8; INLINE_SIZE],
    tag_len: NonZeroU8, // Actually: (len << 1) | (1 << 7)
}

impl StrInlineRepr {
    const TAG_MASK: u8 = 0x80;

    fn is_inline(&self) -> bool {
        self.tag_len.get() & 0x80 != 0
    }
}

impl AsRef<str> for StrInlineRepr {
    #[allow(unsafe_code)]
    fn as_ref(&self) -> &str {
        let len: usize = (self.tag_len.get() & !Self::TAG_MASK) as usize;
        unsafe { str::from_utf8_unchecked(&self.data[..len]) }
    }
}

impl From<&str> for StrInlineRepr {
    fn from(s: &str) -> Self {
        let len: u8 = s
            .len()
            .try_into()
            .expect("str is too long for inline variant");
        let mut data = [0u8; _];
        data[..s.len()].copy_from_slice(s.as_bytes());
        Self {
            tag_len: NonZeroU8::new(len | Self::TAG_MASK).expect("should be non-zero"),
            data,
        }
    }
}

// === SmallStrRepr ===

#[repr(C)]
#[derive(Clone, Copy)]
union SmallStrRepr {
    slice: StrSliceRepr,
    inline: StrInlineRepr,
}

// === SmallStr ===

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SmallStr<'a> {
    repr: SmallStrRepr,
    phantom: PhantomData<&'a str>,
}

impl<'a> SmallStr<'a> {
    pub const INLINE_CAPACITY: usize = INLINE_SIZE;

    pub fn can_be_inlined(s: &str) -> bool {
        s.len() <= INLINE_SIZE
    }

    /// Create a new SmallStr, either by inlining the string data (if it fits)
    /// or by calling the provided closure to allocate it.
    ///
    /// If the string is short enough to be inlined, the data is copied and
    /// the closure is never called. Otherwise, the closure is called to
    /// allocate the string in an arena or return an existing interned string.
    pub fn new_or_alloc(s: &str, alloc_str: impl for<'b> FnOnce(&'b str) -> &'a str) -> Self {
        if Self::can_be_inlined(s) {
            Self {
                repr: SmallStrRepr {
                    inline: StrInlineRepr::from(s),
                },
                phantom: PhantomData,
            }
        } else {
            let allocated = alloc_str(s);
            Self {
                repr: SmallStrRepr {
                    slice: StrSliceRepr::from(allocated),
                },
                phantom: PhantomData,
            }
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            if self.repr.inline.is_inline() {
                self.repr.inline.as_ref()
            } else {
                self.repr.slice.as_ref()
            }
        }
    }

    pub fn is_inline(&self) -> bool {
        unsafe { self.repr.inline.is_inline() }
    }

    /// Smart equality comparison for interned strings.
    /// - If both are inline: compares by value (string content)
    /// - If both are arena-allocated: compares by pointer (fast interned comparison)
    /// - If mixed: compares by value (fallback)
    ///
    /// This is more efficient than regular equality when strings are properly interned,
    /// as it uses pointer comparison for long strings.
    pub fn interned_eq(&self, other: &Self) -> bool {
        match (self.is_inline(), other.is_inline()) {
            (true, true) => {
                // Both inline: compare by value
                self.as_str() == other.as_str()
            }
            (false, false) => {
                // Both arena-allocated: compare by pointer for speed
                core::ptr::eq(self.as_str().as_ptr(), other.as_str().as_ptr())
            }
            _ => {
                // One inline, one not: shouldn't happen with proper interning,
                // but fall back to value comparison
                self.as_str() == other.as_str()
            }
        }
    }

    /// Smart hash for interned strings.
    /// - If inline: hashes the string content
    /// - If arena-allocated: hashes the pointer for speed
    ///
    /// This should be used with `interned_eq` to maintain hash consistency.
    pub fn interned_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        if self.is_inline() {
            // Inline: hash the content
            self.as_str().hash(state);
        } else {
            // Arena-allocated: hash the pointer
            self.as_str().as_ptr().hash(state);
        }
    }
}

impl AsRef<str> for SmallStr<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl core::ops::Deref for SmallStr<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Default for SmallStr<'_> {
    fn default() -> Self {
        Self::new_or_alloc("", |_| unreachable!("empty string is always inline"))
    }
}

impl PartialEq for SmallStr<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for SmallStr<'_> {}

impl PartialEq<str> for SmallStr<'_> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SmallStr<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialOrd for SmallStr<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.as_str().cmp(other.as_str()))
    }
}

impl Ord for SmallStr<'_> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl core::hash::Hash for SmallStr<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl core::fmt::Debug for SmallStr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

impl core::fmt::Display for SmallStr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.as_str(), f)
    }
}

// === SimpleSmallStr Implementations ===

impl<'a> SimpleSmallStr<'a> {
    pub const INLINE_CAPACITY: usize = SIMPLE_INLINE_SIZE;

    pub fn can_be_inlined(s: &str) -> bool {
        s.len() <= SIMPLE_INLINE_SIZE
    }

    /// Create a new SimpleSmallStr, either by inlining the string data (if it fits)
    /// or by calling the provided closure to allocate it.
    ///
    /// If the string is short enough to be inlined, the data is copied and
    /// the closure is never called. Otherwise, the closure is called to
    /// allocate the string in an arena or return an existing interned string.
    pub fn new_or_alloc(s: &str, alloc_str: impl for<'b> FnOnce(&'b str) -> &'a str) -> Self {
        if Self::can_be_inlined(s) {
            let len: u8 = s
                .len()
                .try_into()
                .expect("str is too long for inline variant");
            let mut data = [0u8; SIMPLE_INLINE_SIZE];
            data[..s.len()].copy_from_slice(s.as_bytes());
            SimpleSmallStr::Inline(len, data)
        } else {
            let allocated = alloc_str(s);
            let len: SimpleSliceSize = allocated
                .len()
                .try_into()
                .expect("str is too long for slice variant");
            let ptr = ptr::NonNull::new(allocated.as_ptr() as *mut u8).unwrap();
            SimpleSmallStr::Slice(len, ptr, PhantomData)
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            SimpleSmallStr::Slice(len, ptr, _) => unsafe {
                str::from_utf8_unchecked(slice::from_raw_parts(ptr.as_ptr(), *len as usize))
            },
            SimpleSmallStr::Inline(len, data) => unsafe {
                str::from_utf8_unchecked(&data[..*len as usize])
            },
        }
    }

    pub fn is_inline(&self) -> bool {
        matches!(self, SimpleSmallStr::Inline(_, _))
    }

    /// Smart equality comparison for interned strings.
    /// - If both are inline: compares by value (string content)
    /// - If both are arena-allocated: compares by pointer (fast interned comparison)
    /// - If mixed: compares by value (fallback)
    ///
    /// This is more efficient than regular equality when strings are properly interned,
    /// as it uses pointer comparison for long strings.
    pub fn interned_eq(&self, other: &Self) -> bool {
        match (self.is_inline(), other.is_inline()) {
            (true, true) => {
                // Both inline: compare by value
                self.as_str() == other.as_str()
            }
            (false, false) => {
                // Both arena-allocated: compare by pointer for speed
                core::ptr::eq(self.as_str().as_ptr(), other.as_str().as_ptr())
            }
            _ => {
                // One inline, one not: shouldn't happen with proper interning,
                // but fall back to value comparison
                self.as_str() == other.as_str()
            }
        }
    }

    /// Smart hash for interned strings.
    /// - If inline: hashes the string content
    /// - If arena-allocated: hashes the pointer for speed
    ///
    /// This should be used with `interned_eq` to maintain hash consistency.
    pub fn interned_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        if self.is_inline() {
            // Inline: hash the content
            self.as_str().hash(state);
        } else {
            // Arena-allocated: hash the pointer
            self.as_str().as_ptr().hash(state);
        }
    }
}

impl AsRef<str> for SimpleSmallStr<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl core::ops::Deref for SimpleSmallStr<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Default for SimpleSmallStr<'_> {
    fn default() -> Self {
        Self::new_or_alloc("", |_| unreachable!("empty string is always inline"))
    }
}

impl PartialEq for SimpleSmallStr<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for SimpleSmallStr<'_> {}

impl PartialEq<str> for SimpleSmallStr<'_> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SimpleSmallStr<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialOrd for SimpleSmallStr<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.as_str().cmp(other))
    }
}

impl Ord for SimpleSmallStr<'_> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(&**other)
    }
}

impl core::hash::Hash for SimpleSmallStr<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl core::fmt::Debug for SimpleSmallStr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl core::fmt::Display for SimpleSmallStr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.as_str(), f)
    }
}

impl Clone for SimpleSmallStr<'_> {
    fn clone(&self) -> Self {
        match self {
            SimpleSmallStr::Slice(len, ptr, phantom) => SimpleSmallStr::Slice(*len, *ptr, *phantom),
            SimpleSmallStr::Inline(len, data) => SimpleSmallStr::Inline(*len, *data),
        }
    }
}

impl Copy for SimpleSmallStr<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_str_inline() {
        let s = "hello";
        let small = SmallStr::new_or_alloc(s, |_| panic!("should not allocate"));
        assert_eq!(small.as_str(), "hello");
        assert!(small.is_inline());
        assert_eq!(small.len(), 5);
        assert!(!small.is_empty());
    }

    #[test]
    fn test_small_str_slice() {
        let arena_str = String::from("this is a very long string that exceeds inline capacity");
        let small = SmallStr::new_or_alloc(&arena_str, |s| {
            // Simulate arena allocation by leaking (just for test)
            Box::leak(s.to_string().into_boxed_str())
        });
        assert_eq!(small.as_str(), arena_str.as_str());
        assert!(!small.is_inline());
    }

    #[test]
    fn test_small_str_equality() {
        let s1 = SmallStr::new_or_alloc("hello", |_| panic!("should not allocate"));
        let s2 = SmallStr::new_or_alloc("hello", |_| panic!("should not allocate"));
        let s3 = SmallStr::new_or_alloc("world", |_| panic!("should not allocate"));

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
        assert_eq!(s1, "hello");
        assert_ne!(s1, "world");
    }

    #[test]
    fn test_small_str_ordering() {
        let s1 = SmallStr::new_or_alloc("apple", |_| panic!("should not allocate"));
        let s2 = SmallStr::new_or_alloc("banana", |_| panic!("should not allocate"));
        let s3 = SmallStr::new_or_alloc("apple", |_| panic!("should not allocate"));

        assert!(s1 < s2);
        assert!(s2 > s1);
        assert_eq!(s1.cmp(&s3), core::cmp::Ordering::Equal);
    }

    #[test]
    fn test_small_str_hash() {
        use core::hash::{Hash, Hasher};

        // Simple hasher for testing
        struct SimpleHasher(u64);
        impl Hasher for SimpleHasher {
            fn finish(&self) -> u64 {
                self.0
            }
            fn write(&mut self, bytes: &[u8]) {
                for &b in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(b as u64);
                }
            }
        }

        let s1 = SmallStr::new_or_alloc("test", |_| panic!("should not allocate"));
        let s2 = SmallStr::new_or_alloc("test", |_| panic!("should not allocate"));

        let mut hasher1 = SimpleHasher(0);
        let mut hasher2 = SimpleHasher(0);

        s1.hash(&mut hasher1);
        s2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_small_str_debug_display() {
        let small = SmallStr::new_or_alloc("hello", |_| panic!("should not allocate"));
        assert_eq!(format!("{}", small), "hello");
        assert_eq!(format!("{:?}", small), "\"hello\"");
    }

    #[test]
    fn test_small_str_default() {
        let small = SmallStr::default();
        assert_eq!(small.as_str(), "");
        assert!(small.is_empty());
        assert!(small.is_inline());
    }

    #[test]
    fn test_small_str_deref() {
        let small = SmallStr::new_or_alloc("hello world", |_| panic!("should not allocate"));
        assert_eq!(small.chars().count(), 11);
        assert!(small.starts_with("hello"));
        assert!(small.ends_with("world"));
    }

    #[test]
    fn test_simple_small_str_inline() {
        let s = "hello";
        let simple = SimpleSmallStr::new_or_alloc(s, |_| panic!("should not allocate"));
        assert_eq!(simple.as_str(), "hello");
        assert!(simple.is_inline());
        assert_eq!(simple.len(), 5);
        assert!(!simple.is_empty());
    }

    #[test]
    fn test_simple_small_str_slice() {
        let arena_str = String::from("this is a very long string that exceeds inline capacity");
        let simple = SimpleSmallStr::new_or_alloc(&arena_str, |s| {
            // Simulate arena allocation by leaking (just for test)
            Box::leak(s.to_string().into_boxed_str())
        });
        assert_eq!(simple.as_str(), arena_str.as_str());
        assert!(!simple.is_inline());
    }

    #[test]
    fn test_simple_small_str_equality() {
        let s1 = SimpleSmallStr::new_or_alloc("hello", |_| panic!("should not allocate"));
        let s2 = SimpleSmallStr::new_or_alloc("hello", |_| panic!("should not allocate"));
        let s3 = SimpleSmallStr::new_or_alloc("world", |_| panic!("should not allocate"));

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
        assert_eq!(s1, "hello");
        assert_ne!(s1, "world");
    }

    #[test]
    fn test_simple_small_str_ordering() {
        let s1 = SimpleSmallStr::new_or_alloc("apple", |_| panic!("should not allocate"));
        let s2 = SimpleSmallStr::new_or_alloc("banana", |_| panic!("should not allocate"));
        let s3 = SimpleSmallStr::new_or_alloc("apple", |_| panic!("should not allocate"));

        assert!(s1 < s2);
        assert!(s2 > s1);
        assert_eq!(s1.cmp(&s3), core::cmp::Ordering::Equal);
    }

    #[test]
    fn test_simple_small_str_default() {
        let simple = SimpleSmallStr::default();
        assert_eq!(simple.as_str(), "");
        assert!(simple.is_empty());
        assert!(simple.is_inline());
    }

    #[test]
    fn test_simple_small_str_copy() {
        let s1 = SimpleSmallStr::new_or_alloc("hello", |_| panic!("should not allocate"));
        let s2 = s1;
        assert_eq!(s1, s2);
        assert_eq!(s1.as_str(), "hello");
    }

    #[test]
    fn test_inline_capacity_constants() {
        assert_eq!(SmallStr::INLINE_CAPACITY, INLINE_SIZE);
        assert_eq!(SimpleSmallStr::INLINE_CAPACITY, SIMPLE_INLINE_SIZE);
    }

    #[test]
    fn test_can_be_inlined() {
        assert!(SmallStr::can_be_inlined("short"));
        assert!(!SmallStr::can_be_inlined(
            "this is a very long string that exceeds capacity"
        ));

        assert!(SimpleSmallStr::can_be_inlined("short"));
        assert!(!SimpleSmallStr::can_be_inlined(
            "this is a very long string that exceeds capacity"
        ));
    }

    #[test]
    fn test_small_str_new_or_alloc_with_closure() {
        let long_str = String::from("this is a very long string that exceeds inline capacity");
        let mut called = false;
        let small = SmallStr::new_or_alloc(&long_str, |s| {
            called = true;
            // Simulate arena allocation by leaking (just for test)
            Box::leak(s.to_string().into_boxed_str())
        });
        assert!(called, "closure should be called for long strings");
        assert_eq!(small.as_str(), long_str.as_str());
        assert!(!small.is_inline());
    }

    #[test]
    fn test_simple_small_str_new_or_alloc_with_closure() {
        let long_str = String::from("this is a very long string that exceeds inline capacity");
        let mut called = false;
        let simple = SimpleSmallStr::new_or_alloc(&long_str, |s| {
            called = true;
            // Simulate arena allocation by leaking (just for test)
            Box::leak(s.to_string().into_boxed_str())
        });
        assert!(called, "closure should be called for long strings");
        assert_eq!(simple.as_str(), long_str.as_str());
        assert!(!simple.is_inline());
    }

    #[test]
    fn test_small_str_interned_eq_both_inline() {
        let s1 = SmallStr::new_or_alloc("hello", |_| panic!("should not allocate"));
        let s2 = SmallStr::new_or_alloc("hello", |_| panic!("should not allocate"));
        let s3 = SmallStr::new_or_alloc("world", |_| panic!("should not allocate"));

        assert!(s1.is_inline());
        assert!(s2.is_inline());
        assert!(s3.is_inline());

        // Same content should be equal
        assert!(s1.interned_eq(&s2));
        assert!(s2.interned_eq(&s1));

        // Different content should not be equal
        assert!(!s1.interned_eq(&s3));
        assert!(!s3.interned_eq(&s1));
    }

    #[test]
    fn test_small_str_interned_eq_both_arena() {
        // Create a shared arena-allocated string
        let arena_str: &'static str = Box::leak(
            String::from("this is a long string that exceeds inline capacity").into_boxed_str(),
        );

        let s1 = SmallStr::new_or_alloc(arena_str, |_| arena_str);
        let s2 = SmallStr::new_or_alloc(arena_str, |_| arena_str);

        assert!(!s1.is_inline());
        assert!(!s2.is_inline());

        // Same pointer should be equal
        assert!(s1.interned_eq(&s2));
        assert!(s2.interned_eq(&s1));

        // Different arena-allocated strings with same content
        let arena_str2: &'static str = Box::leak(
            String::from("this is a long string that exceeds inline capacity").into_boxed_str(),
        );
        let s3 = SmallStr::new_or_alloc(arena_str2, |_| arena_str2);

        // Different pointers should not be equal (pointer comparison)
        assert!(!s1.interned_eq(&s3));
    }

    #[test]
    fn test_small_str_interned_hash_consistency() {
        use core::hash::Hasher;

        struct SimpleHasher(u64);
        impl Hasher for SimpleHasher {
            fn finish(&self) -> u64 {
                self.0
            }
            fn write(&mut self, bytes: &[u8]) {
                for &b in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(b as u64);
                }
            }
        }

        // Two inline strings with same content should hash the same
        let s1 = SmallStr::new_or_alloc("test", |_| panic!("should not allocate"));
        let s2 = SmallStr::new_or_alloc("test", |_| panic!("should not allocate"));

        let mut h1 = SimpleHasher(0);
        let mut h2 = SimpleHasher(0);
        s1.interned_hash(&mut h1);
        s2.interned_hash(&mut h2);

        assert_eq!(h1.finish(), h2.finish());

        // Arena-allocated strings pointing to same location should hash the same
        let arena_str: &'static str = Box::leak(
            String::from("this is a long string that exceeds inline capacity").into_boxed_str(),
        );
        let s3 = SmallStr::new_or_alloc(arena_str, |_| arena_str);
        let s4 = SmallStr::new_or_alloc(arena_str, |_| arena_str);

        let mut h3 = SimpleHasher(0);
        let mut h4 = SimpleHasher(0);
        s3.interned_hash(&mut h3);
        s4.interned_hash(&mut h4);

        assert_eq!(h3.finish(), h4.finish());
    }

    #[test]
    fn test_simple_small_str_interned_eq_both_inline() {
        let s1 = SimpleSmallStr::new_or_alloc("hello", |_| panic!("should not allocate"));
        let s2 = SimpleSmallStr::new_or_alloc("hello", |_| panic!("should not allocate"));
        let s3 = SimpleSmallStr::new_or_alloc("world", |_| panic!("should not allocate"));

        assert!(s1.is_inline());
        assert!(s2.is_inline());
        assert!(s3.is_inline());

        // Same content should be equal
        assert!(s1.interned_eq(&s2));
        assert!(s2.interned_eq(&s1));

        // Different content should not be equal
        assert!(!s1.interned_eq(&s3));
        assert!(!s3.interned_eq(&s1));
    }

    #[test]
    fn test_simple_small_str_interned_eq_both_arena() {
        // Create a shared arena-allocated string
        let arena_str: &'static str = Box::leak(
            String::from("this is a long string that exceeds inline capacity").into_boxed_str(),
        );

        let s1 = SimpleSmallStr::new_or_alloc(arena_str, |_| arena_str);
        let s2 = SimpleSmallStr::new_or_alloc(arena_str, |_| arena_str);

        assert!(!s1.is_inline());
        assert!(!s2.is_inline());

        // Same pointer should be equal
        assert!(s1.interned_eq(&s2));
        assert!(s2.interned_eq(&s1));

        // Different arena-allocated strings with same content
        let arena_str2: &'static str = Box::leak(
            String::from("this is a long string that exceeds inline capacity").into_boxed_str(),
        );
        let s3 = SimpleSmallStr::new_or_alloc(arena_str2, |_| arena_str2);

        // Different pointers should not be equal (pointer comparison)
        assert!(!s1.interned_eq(&s3));
    }
}
