use melbi_types::{Scalar, Ty, TyKind};

use crate::traits::{Val, ValueBuilder, ValueView};

use super::Array;

/// A dynamically typed value that provides a safe API for access.
///
/// This is what users work with. The type is only stored here at the outermost
/// level. When values are stored in arrays/maps, only the handles are kept
/// internally, and types are re-attached when elements are accessed.
///
/// # Example
///
/// ```ignore
/// let builder = BoxValueBuilder::new();
/// let tb = BoxBuilder;
///
/// // Create a typed value
/// let v = IntVal(42).alloc(&builder, &tb);
/// assert_eq!(v.as_int(), Some(42));
///
/// // Type is carried with the value
/// assert_eq!(v.ty().kind(), &TyKind::Scalar(Scalar::Int));
/// ```
#[derive(Debug, Clone)]
pub struct Value<B: ValueBuilder> {
    ty: Ty<B::TB>,
    handle: B::ValueHandle,
}

impl<B: ValueBuilder> Value<B> {
    /// Create a new typed value from a type and handle.
    ///
    /// This is typically called by value descriptors, not directly by users.
    pub fn new(ty: Ty<B::TB>, handle: B::ValueHandle) -> Self {
        Self { ty, handle }
    }

    /// Internal: Get the handle to the raw storage.
    pub(crate) fn handle(&self) -> &B::ValueHandle {
        &self.handle
    }

    /// Internal: Consume and return the handle.
    pub(crate) fn into_handle(self) -> B::ValueHandle {
        self.handle
    }

    /// Internal: Access the raw value storage.
    pub(crate) fn val(&self) -> &Val<B> {
        self.handle.as_ref()
    }
}

impl<B: ValueBuilder> ValueView<B> for Value<B> {
    fn ty(&self) -> Ty<B::TB> {
        self.ty.clone()
    }

    fn as_int(&self) -> Option<i64> {
        match self.ty.kind() {
            TyKind::Scalar(Scalar::Int) => Some(self.val().raw().as_int_unchecked()),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self.ty.kind() {
            TyKind::Scalar(Scalar::Bool) => Some(self.val().raw().as_bool_unchecked()),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<impl crate::traits::ArrayView<Value<B>>> {
        let TyKind::Array(element_ty) = self.ty.kind() else {
            return None;
        };

        // TODO: This doesn't work yet - we need to reconcile
        // ValueHandle (points to Val with RawValue) vs ArrayHandle
        let raw = self.handle.as_ref().raw();
        Some(Array::<B>::new(
            element_ty.clone(),
            raw.as_array_unchecked(),
        ))
    }
}
