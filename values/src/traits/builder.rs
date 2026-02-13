//! Value builder trait and core value types.
//!
//! # Design Philosophy
//!
//! Values in Melbi do not store their type internally. Like real computer memory,
//! raw bytes are just bytes - the type is external context used to interpret them.
//! This design:
//! - Saves memory (no per-element type tags in arrays)
//! - Matches how actual computers work
//! - Prevents invalid states (heterogeneous arrays are impossible)
//!
//! The type is only stored at the outermost level in [`Value`]. When values are
//! stored in collections, only the raw handles are kept. The type is re-attached
//! when elements are accessed.
//!
//! # No Footguns
//!
//! Users never access raw values directly. All access goes through typed [`Value`]
//! wrappers that ensure safety. The internal allocation methods are documented
//! as internal - users should use value descriptors like `IntVal(42).alloc(&builder)`.

use core::fmt::Debug;

use melbi_types::TyBuilder;

use crate::raw::RawValue;

/// Builder for allocating values with pluggable storage strategies.
///
/// Similar to [`TyBuilder`] for types, this trait abstracts over how values
/// are stored (heap with Rc, arena allocation, etc.).
///
/// # Internal Methods
///
/// The `alloc_*` methods are internal. Users should use value descriptors:
/// ```ignore
/// // Instead of calling builder methods directly:
/// let handle = builder.alloc_int(42);  // DON'T
///
/// // Use value descriptors:
/// let value = IntVal(42).alloc(&builder);  // DO
/// ```
pub trait ValueBuilder: Sized + Clone + Debug {
    /// The type builder used for type representation.
    type TB: TyBuilder;

    /// Handle to allocated raw value storage.
    /// Examples: `Rc<Val>`, `&'a Val`.
    type ValueHandle: AsRef<Val<Self>> + Clone + Debug;

    /// Handle to an array of value handles (no per-element types).
    /// Examples: `Rc<[Self::ValueHandle]>`, `&'a [Self::ValueHandle]`.
    type ArrayHandle: AsRef<[Self::ValueHandle]> + Clone + Debug;

    // TODO: StringHandle, BytesHandle, MapHandle, RecordHandle, etc.

    /// Internal: Allocate storage for an integer value.
    /// Call instead: `IntVal(n).alloc(builder)`.
    fn alloc_int(&self, value: i64) -> Self::ValueHandle;

    /// Internal: Allocate storage for a boolean value.
    /// Call instead: `BoolVal(b).alloc(builder)`.
    fn alloc_bool(&self, value: bool) -> Self::ValueHandle;

    /// Internal: Allocate storage for an array of values.
    /// Call instead: `ArrayVal([...]).alloc(builder)`.
    fn alloc_array(
        &self,
        elements: impl IntoIterator<Item = Self::ValueHandle, IntoIter: ExactSizeIterator>,
    ) -> Self::ArrayHandle;
}

// =============================================================================
// Val - Internal raw value storage
// =============================================================================

/// Internal: Raw value storage (what [`ValueHandle`](ValueBuilder::ValueHandle) points to).
///
/// Contains only raw data, no type information. The type is tracked externally
/// via [`Value`]. Users never interact with `Val` directly.
#[derive(Debug, Clone)]
pub struct Val<B: ValueBuilder> {
    raw: RawValue<B>,
}

impl<B: ValueBuilder> Val<B> {
    /// Internal: Create a new Val from raw data.
    pub(crate) fn new(raw: RawValue<B>) -> Self {
        Self { raw }
    }

    /// Internal: Access the raw value.
    pub(crate) fn raw(&self) -> &RawValue<B> {
        &self.raw
    }
}

impl<B: ValueBuilder> AsRef<Val<B>> for Val<B> {
    fn as_ref(&self) -> &Val<B> {
        self
    }
}
