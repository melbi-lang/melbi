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
//! stored in collections, only the raw [`Val`] storage is kept. The type is
//! re-attached when elements are accessed.
//!
//! # Storage is inline, never borrowed
//!
//! A [`Val`] is the storage cell for a single value: a small, cheaply cloned bag
//! of bytes (one storage cell — a single word on 64-bit targets — for the arena
//! builder). Collections hold their elements *inline* as `[Val<B>]`, so an
//! `Array[Int]` with three elements is one allocation of three cells — not three
//! separate allocations plus a slice of pointers to them. [`Value`] holds its
//! `Val` inline for the same reason, so a scalar needs no value-storage
//! allocation at all (its [`Ty`](melbi_types::Ty) is still built by the type
//! builder).
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
use core::ops::Deref;

use allocator_api2::alloc::Allocator;
use melbi_types::TyBuilder;

use super::MapStorage;

// =============================================================================
// RawValue - Trait for builder-specific raw storage
// =============================================================================

/// Trait for builder-specific raw value storage.
///
/// Each concrete builder provides its own raw storage type:
/// - `BoxValueBuilder` uses an enum (`BoxRaw`) with proper Clone/Drop
/// - `ArenaValueBuilder` uses a union (`ArenaRaw`) where the arena handles cleanup
///
/// The accessor methods are unchecked — the caller must verify the type first
/// (via [`Value`]'s type field) before calling these.
pub trait RawValue: Clone + Debug {
    /// The handle type for arrays of values.
    type ArrayHandle;

    /// The handle type for strings.
    type StrHandle;

    /// The handle type for byte strings.
    type BytesHandle;

    /// The handle type for maps.
    type MapHandle;

    // --- Constructors ---

    /// Create raw storage for an integer.
    fn from_int(value: i64) -> Self;

    /// Create raw storage for a boolean.
    fn from_bool(value: bool) -> Self;

    /// Create raw storage for a float.
    fn from_float(value: f64) -> Self;

    /// Create raw storage for an array handle.
    fn from_array(handle: Self::ArrayHandle) -> Self;

    /// Create raw storage for a string handle.
    fn from_str(handle: Self::StrHandle) -> Self;

    /// Create raw storage for a byte string handle.
    fn from_bytes(handle: Self::BytesHandle) -> Self;

    /// Create raw storage for a map handle.
    fn from_map(handle: Self::MapHandle) -> Self;

    // --- Accessors (unchecked) ---

    /// Access the raw integer value. Only valid when the type is `Int`.
    fn as_int_unchecked(&self) -> i64;

    /// Access the raw boolean value. Only valid when the type is `Bool`.
    fn as_bool_unchecked(&self) -> bool;

    /// Access the raw float value. Only valid when the type is `Float`.
    fn as_float_unchecked(&self) -> f64;

    /// Access the raw array handle. Only valid when the type is `Array[T]`.
    fn as_array_unchecked(&self) -> &Self::ArrayHandle;

    /// Access the raw string handle. Only valid when the type is `Str`.
    fn as_str_unchecked(&self) -> &Self::StrHandle;

    /// Access the raw byte string handle. Only valid when the type is `Bytes`.
    fn as_bytes_unchecked(&self) -> &Self::BytesHandle;

    /// Access the raw map handle. Only valid when the type is `Map[K, V]`.
    fn as_map_unchecked(&self) -> &Self::MapHandle;
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
    /// - For `ArenaValueBuilder`: a union (`ArenaRaw`) where the arena handles cleanup
    type Raw: RawValue<
            ArrayHandle = Self::ArrayHandle,
            StrHandle = Self::StrHandle,
            BytesHandle = Self::BytesHandle,
            MapHandle = Self::MapHandle,
        >;

    /// Handle to an array of values, stored inline (no per-element types and no
    /// per-element indirection).
    /// Examples: `Rc<[Val<Self>]>`, `ThinRef<'a, [Val<Self>]>`.
    type ArrayHandle: AsRef<[Val<Self>]> + Clone + Debug;

    /// Handle to a UTF-8 string.
    /// Examples: `Rc<str>`, `ThinRef<'a, str>`.
    type StrHandle: Deref<Target = str> + Clone + Debug;

    /// Handle to a byte string.
    /// Examples: `Rc<[u8]>`, `ThinRef<'a, [u8]>`.
    type BytesHandle: Deref<Target = [u8]> + Clone + Debug;

    // TODO: RecordHandle, OptionHandle, etc.

    /// Get the type builder.
    fn ty_builder(&self) -> &Self::TB;

    /// Internal: Allocate storage for an array of values.
    ///
    /// The elements are stored inline in the returned handle. To create a full
    /// array value, use [`Value::array()`] which calls this internally.
    fn alloc_array(
        &self,
        elements: impl IntoIterator<Item = Val<Self>, IntoIter: ExactSizeIterator>,
    ) -> Self::ArrayHandle;

    /// Internal: Allocate storage for a string.
    ///
    /// To create a full string value, use [`Value::str()`] which calls this
    /// internally.
    fn alloc_str(&self, value: &str) -> Self::StrHandle;

    /// Internal: Allocate storage for a byte string.
    ///
    /// To create a full bytes value, use [`Value::bytes()`] which calls this
    /// internally.
    fn alloc_bytes(&self, value: &[u8]) -> Self::BytesHandle;

    /// Allocator for map storage while a map is being built.
    /// Examples: `&'arena Bump`, `Global`.
    type MapAllocator: Allocator + Clone;

    /// Handle to a map's storage.
    /// Examples: `ThinRef<'a, MapStorage<Self>>`, `Rc<MapStorage<Self>>`.
    type MapHandle: Deref<Target = MapStorage<Self>> + Clone + Debug;

    /// Internal: The allocator for a map under construction.
    fn map_allocator(&self) -> Self::MapAllocator;

    /// Internal: Allocate a finished map. Use `MapMaker` or `typed::Map::new`.
    fn alloc_map(&self, storage: MapStorage<Self>) -> Self::MapHandle;
}

// =============================================================================
// Val - Internal raw value storage
// =============================================================================

/// Internal: Raw storage for a single value.
///
/// Contains only raw data, no type information. The type is tracked externally
/// via [`Value`]. Users never interact with `Val` directly.
///
/// `Val` is what collections store, so it is deliberately small and stored
/// inline: `[Val<B>]` for arrays, and the payload of [`Value`] itself.
#[derive(Debug, Clone)]
pub struct Val<B: ValueBuilder> {
    raw: B::Raw,
}

// Copy when the raw storage is Copy (e.g., `ArenaRaw`, where the arena owns the
// data and nothing needs cleanup).
impl<B: ValueBuilder> Copy for Val<B> where B::Raw: Copy {}

impl<B: ValueBuilder> Val<B> {
    /// Internal: Create a new Val from raw data.
    fn new(raw: B::Raw) -> Self {
        Self { raw }
    }

    /// Internal: Storage for an integer value.
    pub(crate) fn int(value: i64) -> Self {
        Self::new(B::Raw::from_int(value))
    }

    /// Internal: Storage for a boolean value.
    pub(crate) fn bool(value: bool) -> Self {
        Self::new(B::Raw::from_bool(value))
    }

    /// Internal: Storage for a float value.
    pub(crate) fn float(value: f64) -> Self {
        Self::new(B::Raw::from_float(value))
    }

    /// Internal: Storage for an array value, from a handle built by
    /// [`ValueBuilder::alloc_array`].
    pub(crate) fn array(handle: B::ArrayHandle) -> Self {
        Self::new(B::Raw::from_array(handle))
    }

    /// Internal: Storage for a string value, from a handle built by
    /// [`ValueBuilder::alloc_str`].
    pub(crate) fn str(handle: B::StrHandle) -> Self {
        Self::new(B::Raw::from_str(handle))
    }

    /// Internal: Storage for a byte string value, from a handle built by
    /// [`ValueBuilder::alloc_bytes`].
    pub(crate) fn bytes(handle: B::BytesHandle) -> Self {
        Self::new(B::Raw::from_bytes(handle))
    }

    /// Internal: Storage for a map value, from a handle built by
    /// [`ValueBuilder::alloc_map`].
    pub(crate) fn map(handle: B::MapHandle) -> Self {
        Self::new(B::Raw::from_map(handle))
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

    /// Internal: Access the raw string handle. Only valid when the type is `Str`.
    pub(crate) fn as_str_unchecked(&self) -> &B::StrHandle {
        self.raw.as_str_unchecked()
    }

    /// Internal: Access the raw byte string handle. Only valid when the type is `Bytes`.
    pub(crate) fn as_bytes_unchecked(&self) -> &B::BytesHandle {
        self.raw.as_bytes_unchecked()
    }

    /// Internal: Access the raw map handle. Only valid when the type is `Map[K, V]`.
    pub(crate) fn as_map_unchecked(&self) -> &B::MapHandle {
        self.raw.as_map_unchecked()
    }
}
