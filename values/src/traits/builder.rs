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
//! as internal - users should use the static constructors on [`Value`]:
//! ```ignore
//! let v = Value::int(&builder, 42);
//! ```

use core::fmt::Debug;

use melbi_types::TyBuilder;

// =============================================================================
// RawValue - Trait for builder-specific raw storage
// =============================================================================

/// Trait for builder-specific raw value storage.
///
/// Each concrete builder provides its own raw storage type:
/// - `BoxValueBuilder` uses an enum (`BoxRaw`) with proper Clone/Drop
/// - `ArenaValueBuilder` (future) uses a union where the arena handles cleanup
///
/// The accessor methods are unchecked — the caller must verify the type first
/// (via [`Value`]'s type field) before calling these.
pub trait RawValue: Clone + Debug {
    /// The handle type for arrays of value handles.
    type ArrayHandle;

    // --- Constructors ---

    /// Create raw storage for an integer.
    fn from_int(value: i64) -> Self;

    /// Create raw storage for a boolean.
    fn from_bool(value: bool) -> Self;

    /// Create raw storage for a float.
    fn from_float(value: f64) -> Self;

    /// Create raw storage for an array handle.
    fn from_array(handle: Self::ArrayHandle) -> Self;

    // --- Accessors (unchecked) ---

    /// Access the raw integer value. Only valid when the type is `Int`.
    fn as_int_unchecked(&self) -> i64;

    /// Access the raw boolean value. Only valid when the type is `Bool`.
    fn as_bool_unchecked(&self) -> bool;

    /// Access the raw float value. Only valid when the type is `Float`.
    fn as_float_unchecked(&self) -> f64;

    /// Access the raw array handle. Only valid when the type is `Array[T]`.
    fn as_array_unchecked(&self) -> &Self::ArrayHandle;
}

// =============================================================================
// ValueBuilder - Pluggable allocation strategy
// =============================================================================

/// Builder for allocating values with pluggable storage strategies.
///
/// Similar to [`TyBuilder`] for types, this trait abstracts over how values
/// are stored (heap with Rc, arena allocation, etc.).
///
/// # Internal Methods
///
/// The `alloc_*` methods are internal. Users should use [`Value`] constructors:
/// ```ignore
/// let v = Value::int(&builder, 42);
/// let arr = Value::array(&builder, element_ty, vec![v1, v2]);
/// ```
pub trait ValueBuilder: Sized + Clone + Debug {
    /// The type builder used for type representation.
    type TB: TyBuilder;

    /// Builder-specific raw value storage.
    ///
    /// - For `BoxValueBuilder`: an enum (`BoxRaw`) that can properly Clone/Drop
    /// - For `ArenaValueBuilder` (future): a union where the arena handles cleanup
    type Raw: RawValue<ArrayHandle = Self::ArrayHandle>;

    /// Handle to allocated raw value storage.
    /// Examples: `Rc<Val<Self>>`, `&'a Val<Self>`.
    type ValHandle: AsRef<Val<Self>> + Clone + Debug;

    /// Handle to an array of value handles (no per-element types).
    /// Examples: `Rc<[Self::ValueHandle]>`, `&'a [Self::ValueHandle]`.
    type ArrayHandle: AsRef<[Self::ValHandle]> + Clone + Debug;

    // TODO: StringHandle, BytesHandle, MapHandle, RecordHandle, etc.

    /// Get the type builder.
    fn ty_builder(&self) -> &Self::TB;

    /// Internal: Allocate a raw value and return a handle to it.
    ///
    /// This is the core allocation method. The convenience methods (`alloc_int`,
    /// `alloc_bool`, etc.) delegate to this by default.
    fn alloc_val(&self, raw: Self::Raw) -> Self::ValHandle;

    /// Internal: Allocate storage for an array of value handles.
    ///
    /// Returns an `ArrayHandle`, not a `ValueHandle`. To create a full array value,
    /// use [`Value::array()`] which calls this internally.
    fn alloc_array(
        &self,
        elements: impl IntoIterator<Item = Self::ValHandle, IntoIter: ExactSizeIterator>,
    ) -> Self::ArrayHandle;

    /// Internal: Allocate storage for an integer value.
    fn alloc_int(&self, value: i64) -> Self::ValHandle {
        self.alloc_val(Self::Raw::from_int(value))
    }

    /// Internal: Allocate storage for a boolean value.
    fn alloc_bool(&self, value: bool) -> Self::ValHandle {
        self.alloc_val(Self::Raw::from_bool(value))
    }

    /// Internal: Allocate storage for a float value.
    fn alloc_float(&self, value: f64) -> Self::ValHandle {
        self.alloc_val(Self::Raw::from_float(value))
    }
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
    raw: B::Raw,
}

impl<B: ValueBuilder> Val<B> {
    /// Internal: Create a new Val from raw data.
    pub(crate) fn new(raw: B::Raw) -> Self {
        Self { raw }
    }

    /// Internal: Access the raw integer. Only valid when the type is `Int`.
    pub(crate) fn as_int_unchecked(&self) -> i64 {
        self.raw.as_int_unchecked()
    }

    /// Internal: Access the raw boolean. Only valid when the type is `Bool`.
    pub(crate) fn as_bool_unchecked(&self) -> bool {
        self.raw.as_bool_unchecked()
    }

    /// Internal: Access the raw float. Only valid when the type is `Float`.
    pub(crate) fn as_float_unchecked(&self) -> f64 {
        self.raw.as_float_unchecked()
    }

    /// Internal: Access the raw array handle. Only valid when the type is `Array[T]`.
    pub(crate) fn as_array_unchecked(&self) -> &B::ArrayHandle {
        self.raw.as_array_unchecked()
    }
}

impl<B: ValueBuilder> AsRef<Val<B>> for Val<B> {
    fn as_ref(&self) -> &Val<B> {
        self
    }
}
