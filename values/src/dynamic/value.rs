use alloc::vec::Vec;

use melbi_types::{Scalar, Ty, TyKind};

use crate::traits::{RawValue, Val, ValueBuilder, ValueView};

use super::Array;

/// A dynamically typed value that provides a safe API for access.
///
/// This is what users work with. The type is only stored here at the outermost
/// level. When values are stored in arrays/maps, only the handles are kept
/// internally, and types are re-attached when elements are accessed.
///
/// # Example
///
/// ```
/// use melbi_values::builders::BoxValueBuilder;
/// use melbi_values::dynamic::Value;
/// use melbi_values::traits::ValueView;
///
/// let builder = BoxValueBuilder::new();
/// let v = Value::int(&builder, 42);
/// assert_eq!(v.as_int(), Some(42));
/// ```
#[derive(Debug, Clone)]
pub struct Value<B: ValueBuilder> {
    ty: Ty<B::TB>,
    handle: B::ValHandle,
}

impl<B: ValueBuilder> Value<B> {
    /// Internal: Create a new typed value from a type and handle.
    ///
    /// Prefer using the static constructors (`Value::int`, `Value::bool`, etc.)
    /// which handle type creation automatically.
    pub(crate) fn new(ty: Ty<B::TB>, handle: B::ValHandle) -> Self {
        Self { ty, handle }
    }

    /// Create an integer value.
    pub fn int(builder: &B, value: i64) -> Self {
        let handle = builder.alloc_int(value);
        let ty = TyKind::Scalar(Scalar::Int).alloc(builder.ty_builder());
        Self::new(ty, handle)
    }

    /// Create a boolean value.
    pub fn bool(builder: &B, value: bool) -> Self {
        let handle = builder.alloc_bool(value);
        let ty = TyKind::Scalar(Scalar::Bool).alloc(builder.ty_builder());
        Self::new(ty, handle)
    }

    /// Create a float value.
    pub fn float(builder: &B, value: f64) -> Self {
        let handle = builder.alloc_float(value);
        let ty = TyKind::Scalar(Scalar::Float).alloc(builder.ty_builder());
        Self::new(ty, handle)
    }

    /// Create an array value from a list of elements.
    ///
    /// All elements must have the same type (the given `element_ty`).
    pub fn array(builder: &B, element_ty: Ty<B::TB>, elements: Vec<Self>) -> Self {
        debug_assert!(
            elements.iter().all(|e| *e.ty() == element_ty),
            "all array elements must match element_ty",
        );
        let handles = elements.into_iter().map(|e| e.into_handle());
        let array_handle = builder.alloc_array(handles);
        let val_handle = builder.alloc_val(B::Raw::from_array(array_handle));
        let ty = TyKind::Array(element_ty).alloc(builder.ty_builder());
        Self::new(ty, val_handle)
    }

    /// Internal: Get the handle to the raw storage.
    pub(crate) fn handle(&self) -> &B::ValHandle {
        &self.handle
    }

    /// Internal: Consume and return the handle.
    pub(crate) fn into_handle(self) -> B::ValHandle {
        self.handle
    }

    /// Internal: Access the raw value storage.
    pub(crate) fn val(&self) -> &Val<B> {
        self.handle.as_ref()
    }
}

impl<B: ValueBuilder> ValueView<B> for Value<B> {
    fn ty(&self) -> &Ty<B::TB> {
        &self.ty
    }

    fn as_int(&self) -> Option<i64> {
        match self.ty.kind() {
            TyKind::Scalar(Scalar::Int) => Some(self.val().as_int_unchecked()),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self.ty.kind() {
            TyKind::Scalar(Scalar::Bool) => Some(self.val().as_bool_unchecked()),
            _ => None,
        }
    }

    fn as_float(&self) -> Option<f64> {
        match self.ty.kind() {
            TyKind::Scalar(Scalar::Float) => Some(self.val().as_float_unchecked()),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<impl crate::traits::ArrayView<Value<B>>> {
        let TyKind::Array(element_ty) = self.ty.kind() else {
            return None;
        };

        let handle = self.val().as_array_unchecked().clone();
        Some(Array::<B>::new(element_ty.clone(), handle))
    }
}
