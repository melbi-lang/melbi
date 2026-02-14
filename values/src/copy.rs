//! Cross-builder value copying.
//!
//! Copies a [`Value<Src>`] into a different builder, producing a [`Value<Dst>`].
//! This validates that the builder abstraction is truly generic — the same
//! function works across all builder combinations (Box↔Arena, Arena↔Arena, etc.).
//!
//! # Example
//!
//! ```ignore
//! use melbi_values::copy::copy_value;
//!
//! let box_builder = BoxValueBuilder::new();
//! let arena = Bump::new();
//! let arena_builder = ArenaValueBuilder::new(&arena);
//!
//! let v = Value::int(&box_builder, 42);
//! let copied = copy_value(&v, &arena_builder);
//! assert_eq!(copied.as_int(), Some(42));
//! ```

// TODO: Arena→Arena specialized copy. When both source and destination are
// ArenaValueBuilder, the copy can be much more efficient — e.g., bulk-copying
// raw Val data and array slices into the destination arena without going
// through the generic ValueView match/reconstruct path.

use melbi_types::{Scalar, Ty, TyBuilder, TyKind};

use crate::dynamic::Value;
use crate::traits::{ArrayView, RawValue, ValueBuilder, ValueView};

/// Copy a type from one builder to another.
///
/// Recursively reconstructs the type tree in the destination builder using
/// [`TyKind::from_iter_children`]. No intermediate allocations — the mapped
/// iterator is passed directly.
fn copy_type<Src: TyBuilder, Dst: TyBuilder>(ty: &Ty<Src>, dst: &Dst) -> Ty<Dst> {
    let kind = ty.kind();
    let children = kind.iter_children().map(|child| copy_type(child, dst));
    kind.from_iter_children(dst, children).alloc(dst)
}

/// Copy a value from one builder to another.
///
/// Reads the source value through the generic [`ValueView`] interface and
/// reconstructs it in the destination builder. Works across any combination
/// of builders (Box→Arena, Arena→Box, Arena→Arena, Box→Box).
///
/// For arrays, elements are streamed directly through `alloc_array` without
/// collecting into an intermediate `Vec`.
pub fn copy_value<Src: ValueBuilder, Dst: ValueBuilder>(
    value: &Value<Src>,
    dst: &Dst,
) -> Value<Dst> {
    match value.ty().kind() {
        TyKind::Scalar(Scalar::Int) => Value::int(dst, value.as_int().unwrap()),
        TyKind::Scalar(Scalar::Bool) => Value::bool(dst, value.as_bool().unwrap()),
        TyKind::Scalar(Scalar::Float) => Value::float(dst, value.as_float().unwrap()),
        TyKind::Array(elem_ty) => {
            let dst_elem_ty = copy_type(elem_ty, dst.ty_builder());
            let array = value.as_array().unwrap();
            // Stream elements directly through alloc_array (no Vec)
            let handles = (0..array.len())
                .map(|i| copy_value(&array.get(i).unwrap(), dst).into_handle());
            let array_handle = dst.alloc_array(handles);
            let val_handle = dst.alloc_val(Dst::Raw::from_array(array_handle));
            let ty = TyKind::Array(dst_elem_ty).alloc(dst.ty_builder());
            Value::new(ty, val_handle)
        }
        other => unreachable!("unsupported value type for copying: {other:?}"),
    }
}
