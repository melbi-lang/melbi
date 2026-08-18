#![allow(
    unsafe_code,
    reason = "low-level FFI type-casting and pointer conversions from raw representation"
)]
use core::marker::PhantomData;

use crate::types::Type;
use crate::types::manager::TypeManager;
use crate::values::raw::{ArrayData, RawValue};

#[derive(Debug)]
pub enum TypeError {
    Mismatch,
    IndexOutOfBounds,
}

impl core::fmt::Display for TypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for TypeError {}

pub trait FromRawValue<'a>: Sized {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a>;

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: RawValue,
    ) -> Result<Self, TypeError>;
}

// i64 implementation
impl<'a> FromRawValue<'a> for i64 {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.int()
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        Ok(raw.as_int_unchecked())
    }
}

// f64 implementation
impl<'a> FromRawValue<'a> for f64 {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.float()
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        Ok(raw.as_float_unchecked())
    }
}

// bool implementation
impl<'a> FromRawValue<'a> for bool {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.bool()
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        Ok(raw.as_bool_unchecked())
    }
}

// &str implementation
impl<'a> FromRawValue<'a> for &'a str {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.str()
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: crate::values::raw::RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        Ok(raw.as_str_unchecked())
    }
}

// &[u8] implementation
impl<'a> FromRawValue<'a> for &'a [u8] {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.bytes()
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: crate::values::raw::RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        Ok(raw.as_bytes_unchecked())
    }
}

// Array implementation
// XXX: Move this to dynamic.rs.
pub struct Array<'a, T> {
    elem_ty: &'a Type<'a>,
    array_data: ArrayData<'a>,
    _phantom: PhantomData<(&'a (), T)>,
}

impl<'a, T: FromRawValue<'a>> FromRawValue<'a> for Array<'a, T> {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        let elem_ty = T::type_descr(type_mgr);
        type_mgr.array(elem_ty)
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }

        let Type::Array(elem_ty) = ty else {
            unreachable!()
        };

        let array_data = ArrayData::from_raw_value(raw);
        Ok(Array {
            elem_ty,
            array_data,
            _phantom: PhantomData,
        })
    }
}

impl<'a, T: FromRawValue<'a>> Array<'a, T> {
    pub fn get(&self, type_mgr: &'a TypeManager<'a>, index: usize) -> Result<T, TypeError> {
        if index >= self.array_data.length() {
            return Err(TypeError::IndexOutOfBounds);
        }

        let raw = unsafe { self.array_data.get_unchecked(index) };
        T::from_raw(type_mgr, self.elem_ty, raw)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.array_data.length()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn as_raw_value(&self) -> RawValue {
        self.array_data.as_raw_value()
    }
}
