#![allow(unsafe_code)]

use core::ptr::NonNull;
use core::{fmt, slice};

use alloc::vec::Vec;

#[repr(C)]
pub union RawValue {
    int: i64,
    bool: bool,
    // Do not use NonNull<[RawValue]> to keep the pointer thin.
    array: NonNull<RawValue>,
}

static_assertions::assert_eq_size!(RawValue, usize);

impl Copy for RawValue {}

impl Clone for RawValue {
    fn clone(&self) -> Self {
        *self
    }
}

impl fmt::Debug for RawValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:p}", unsafe { self.array })
    }
}

impl RawValue {
    pub fn new_int(value: i64) -> Self {
        RawValue { int: value }
    }

    pub fn new_bool(value: bool) -> Self {
        RawValue { bool: value }
    }

    pub fn new_array(values: &[RawValue]) -> Self {
        // TODO: This is just a proof of concept (and it leaks memory!).
        let mut data = Vec::with_capacity(1 + values.len());
        data.push(RawValue {
            int: values.len() as i64,
        });
        data.extend_from_slice(values);
        let array = NonNull::from_ref(data.leak()).cast();
        RawValue { array }
    }

    pub fn as_int_unchecked(&self) -> i64 {
        unsafe { self.int }
    }

    pub fn as_bool_unchecked(&self) -> bool {
        unsafe { self.bool }
    }

    pub fn as_array_unchecked(&self) -> &[RawValue] {
        unsafe {
            let len: usize = self.array.as_ref().int as usize;
            slice::from_raw_parts(self.array.add(1).as_ref(), len)
        }
    }
}
