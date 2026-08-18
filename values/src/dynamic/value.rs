use alloc::vec::Vec;

use melbi_types::{Scalar, Ty, TyKind};

use super::Array;
use crate::traits::{Val, ValueBuilder, ValueView};

/// A dynamically typed value that provides a safe API for access.
///
/// This is what users work with. The type is only stored here at the outermost
/// level. When values are stored in arrays/maps, only the raw [`Val`] storage is
/// kept internally, and types are re-attached when elements are accessed.
///
/// The `Val` is held inline, so a scalar `Value` needs no allocation at all.
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
    val: Val<B>,
}

// Copy when both the type handle and the raw storage are Copy (e.g., for the
// arena builder, where a `Value` is two words and owns nothing).
impl<B: ValueBuilder> Copy for Value<B>
where
    Ty<B::TB>: Copy,
    Val<B>: Copy,
{
}

impl<B: ValueBuilder> Value<B> {
    /// Internal: Create a new typed value from a type and raw storage.
    ///
    /// Prefer using the static constructors (`Value::int`, `Value::bool`, etc.)
    /// which handle type creation automatically.
    pub(crate) fn new(ty: Ty<B::TB>, val: Val<B>) -> Self {
        Self { ty, val }
    }

    /// Create an integer value.
    pub fn int(builder: &B, value: i64) -> Self {
        let ty = TyKind::Scalar(Scalar::Int).alloc(builder.ty_builder());
        Self::new(ty, Val::int(value))
    }

    /// Create a boolean value.
    pub fn bool(builder: &B, value: bool) -> Self {
        let ty = TyKind::Scalar(Scalar::Bool).alloc(builder.ty_builder());
        Self::new(ty, Val::bool(value))
    }

    /// Create a float value.
    pub fn float(builder: &B, value: f64) -> Self {
        let ty = TyKind::Scalar(Scalar::Float).alloc(builder.ty_builder());
        Self::new(ty, Val::float(value))
    }

    /// Create an array value from a list of elements.
    ///
    /// All elements must have the same type (the given `element_ty`). Their raw
    /// storage is moved into the array; the per-element types are dropped and
    /// re-attached on access.
    pub fn array(builder: &B, element_ty: Ty<B::TB>, elements: Vec<Self>) -> Self {
        debug_assert!(
            elements.iter().all(|e| *e.ty() == element_ty),
            "all array elements must match element_ty",
        );
        let array_handle = builder.alloc_array(elements.into_iter().map(Self::into_val));
        let ty = TyKind::Array(element_ty).alloc(builder.ty_builder());
        Self::new(ty, Val::array(array_handle))
    }

    /// Internal: Access the raw value storage.
    pub(crate) fn val(&self) -> &Val<B> {
        &self.val
    }

    /// Internal: Consume and return the raw value storage.
    pub(crate) fn into_val(self) -> Val<B> {
        self.val
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

    fn as_array(&self) -> Option<impl crate::traits::ArrayView<Self>> {
        let TyKind::Array(element_ty) = self.ty.kind() else {
            return None;
        };

        let handle = self.val().as_array_unchecked().clone();
        Some(Array::<B>::new(element_ty.clone(), handle))
    }
}
