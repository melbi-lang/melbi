#![allow(unsafe_code)]

use core::fmt;
use core::mem::ManuallyDrop;

use crate::traits::ValueBuilder;

#[repr(C)]
pub union RawValue<B: ValueBuilder> {
    int: i64,
    bool: bool,
    array: ManuallyDrop<B::ArrayHandle>, // TODO: This is a fat pointer, but it must be thin.
}

// static_assertions::assert_eq_size!(RawValue, usize);

// impl<B: ValueBuilder> Copy for RawValue<B> {}

// impl<B: ValueBuilder> Clone for RawValue<B> {
//     fn clone(&self) -> Self {
//         *self
//     }
// }

impl<B: ValueBuilder> fmt::Debug for RawValue<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:p}", unsafe { self.int as *const () })
    }
}

impl<B: ValueBuilder> RawValue<B> {
    pub fn new_int(value: i64) -> Self {
        RawValue { int: value }
    }

    pub fn new_bool(value: bool) -> Self {
        RawValue { bool: value }
    }

    pub fn new_array(
        builder: &B,
        values: impl IntoIterator<Item = B::ValueHandle, IntoIter: ExactSizeIterator>,
    ) -> Self {
        let array = ManuallyDrop::new(builder.alloc_array(values));
        RawValue { array }
    }

    pub fn as_int_unchecked(&self) -> i64 {
        unsafe { self.int }
    }

    pub fn as_bool_unchecked(&self) -> bool {
        unsafe { self.bool }
    }

    pub fn as_array_unchecked(&self) -> &[B::ValueHandle] {
        unsafe { self.array.as_ref() }
    }
}
