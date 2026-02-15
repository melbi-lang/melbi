//! Statically-typed array view for zero-cost interop.
//!
//! `Array<B, E>` wraps the builder's `ArrayHandle` directly — no deep conversion
//! or runtime type tags. Element access returns `E` (e.g., `i64`) without runtime
//! type checking.
//!
//! # Example
//!
//! ```ignore
//! let builder = BoxValueBuilder::new();
//! let arr = Array::<_, i64>::new(&builder, vec![1, 2, 3]);
//!
//! assert_eq!(arr.get(0), Some(1));
//! assert_eq!(arr.len(), 3);
//!
//! // Convert to dynamic Value and back
//! let value = arr.clone().into_value(&builder);
//! let arr2 = Array::<_, i64>::from_value(&value).unwrap();
//! assert_eq!(arr2.get(0), Some(1));
//! ```

use core::fmt::Debug;
use core::marker::PhantomData;

use melbi_types::{Ty, TyKind};

use crate::dynamic::Value;
use crate::traits::{ArrayView, RawValue, Val, ValueBuilder, ValueView};

use super::Marshal;

/// A statically-typed array view.
///
/// Wraps the builder's `ArrayHandle` with a compile-time element type `E`.
/// No runtime type information is stored — `E` provides the type via [`Marshal`].
///
/// For the arena builder, this is as compact as the legacy `Array<'a, T>`:
/// a single `ThinRef` (8 bytes, `Copy`).
pub struct Array<B: ValueBuilder, E> {
    handle: B::ArrayHandle,
    _marker: PhantomData<E>,
}

// --- Manual Clone/Copy/Debug to avoid E: Clone/Copy/Debug bounds ---

impl<B: ValueBuilder, E> Clone for Array<B, E> {
    fn clone(&self) -> Self {
        Array {
            handle: self.handle.clone(),
            _marker: PhantomData,
        }
    }
}

impl<B: ValueBuilder, E> Copy for Array<B, E> where B::ArrayHandle: Copy {}

impl<B: ValueBuilder, E> Debug for Array<B, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Array")
            .field("handle", &self.handle)
            .field("element_type", &core::any::type_name::<E>())
            .finish()
    }
}

// --- Construction ---

impl<B: ValueBuilder, E: Marshal<B>> Array<B, E> {
    /// Create a typed array from a list of elements.
    ///
    /// Each element is marshalled into the builder's storage via
    /// [`Marshal::into_value_handle`].
    pub fn new(builder: &B, elements: impl IntoIterator<Item = E, IntoIter: ExactSizeIterator>) -> Self {
        let handles = elements.into_iter().map(|e| e.into_value_handle(builder));
        let handle = builder.alloc_array(handles);
        Array {
            handle,
            _marker: PhantomData,
        }
    }

    /// Try to extract a typed array from a dynamic [`Value`].
    ///
    /// Returns `None` if the value's type does not match `Array[E]`.
    /// Type checking is structural and allocation-free.
    pub fn from_value(value: &Value<B>) -> Option<Self> {
        if !Self::matches_ty_kind(&value.ty().kind()) {
            return None;
        }
        let handle = value.val().as_array_unchecked().clone();
        Some(Array {
            handle,
            _marker: PhantomData,
        })
    }

    /// Convert this typed array into a dynamic [`Value`].
    ///
    /// Wraps the array handle in a `Value` with the appropriate `Ty`.
    pub fn into_value(self, builder: &B) -> Value<B> {
        let elem_ty = E::ty(builder.ty_builder());
        let ty = TyKind::Array(elem_ty).alloc(builder.ty_builder());
        let val_handle = builder.alloc_val(B::Raw::from_array(self.handle));
        Value::new(ty, val_handle)
    }
}

// --- ArrayView ---

impl<B: ValueBuilder, E: Marshal<B>> ArrayView<E> for Array<B, E> {
    fn len(&self) -> usize {
        self.handle.as_ref().len()
    }

    fn get(&self, index: usize) -> Option<E> {
        let elem_handle = self.handle.as_ref().get(index)?;
        Some(E::from_val_unchecked(elem_handle.as_ref()))
    }
}

// --- Marshal (enables nesting: Array<B, Array<B, i64>>) ---

impl<B: ValueBuilder, E: Marshal<B>> Marshal<B> for Array<B, E> {
    fn ty(tb: &B::TB) -> Ty<B::TB> {
        TyKind::Array(E::ty(tb)).alloc(tb)
    }

    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool {
        match kind {
            TyKind::Array(elem_ty) => E::matches_ty_kind(&elem_ty.kind()),
            _ => false,
        }
    }

    fn from_val_unchecked(val: &Val<B>) -> Self {
        Array {
            handle: val.as_array_unchecked().clone(),
            _marker: PhantomData,
        }
    }

    fn into_value_handle(self, builder: &B) -> B::ValueHandle {
        builder.alloc_val(B::Raw::from_array(self.handle))
    }
}
