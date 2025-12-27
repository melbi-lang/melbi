#[cfg(target_endian = "little")]
use core::num::NonZeroU8;
use core::{mem, ops::BitOrAssign, ptr, slice};

// === SimpleSmallStr ===

#[cfg(target_pointer_width = "32")]
type SimpleSliceSize = u16;

#[cfg(target_pointer_width = "64")]
type SimpleSliceSize = u32;

const SIMPLE_INLINE_SIZE: usize = 2 * mem::size_of::<usize>() - 2;

pub enum SimpleSmallStr {
    Slice(SimpleSliceSize, ptr::NonNull<u8>),
    Inline(u8, [u8; SIMPLE_INLINE_SIZE]),
}

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
pub struct SmallStr {
    repr: SmallStrRepr,
}

impl SmallStr {
    fn can_be_inlined(s: &str) -> bool {
        s.len() <= INLINE_SIZE
    }
}

impl From<&str> for SmallStr {
    fn from(s: &str) -> Self {
        if s.len() <= INLINE_SIZE {
            Self {
                repr: SmallStrRepr {
                    inline: StrInlineRepr::from(s),
                },
            }
        } else {
            Self {
                repr: SmallStrRepr {
                    slice: StrSliceRepr::from(s),
                },
            }
        }
    }
}

impl AsRef<str> for SmallStr {
    fn as_ref(&self) -> &str {
        unsafe {
            if self.repr.inline.is_inline() {
                self.repr.inline.as_ref()
            } else {
                self.repr.slice.as_ref()
            }
        }
    }
}

impl core::ops::Deref for SmallStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
