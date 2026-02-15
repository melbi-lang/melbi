#![allow(unsafe_code)] // Union field access requires unsafe

use bumpalo::Bump;
use core::fmt;

use melbi_thin_ref::ThinRef;
use melbi_types::ArenaBuilder;

use crate::traits::{RawValue, Val, ValueBuilder};

// =============================================================================
// ArenaRaw - Union-based raw storage for arena allocation
// =============================================================================

/// Raw value storage for [`ArenaValueBuilder`].
///
/// Uses a union because the arena handles cleanup (no Drop needed).
/// All fields are Copy, so the union itself is Copy. The caller (via
/// [`Value`]'s type field) ensures only the correct field is accessed.
///
/// # Safety invariant
///
/// Only the field matching the value's type (tracked externally by [`Value`])
/// may be read. This is enforced by the typed API — users never access
/// `ArenaRaw` directly.
#[repr(C)]
pub union ArenaRaw<'arena> {
    int: i64,
    bool: bool,
    float: f64,
    array: ThinRef<'arena, [&'arena Val<ArenaValueBuilder<'arena>>]>,
}

// 8 bytes on 64-bit: ThinRef is pointer-sized (length stored inline before data).
#[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
static_assertions::assert_eq_size!(ArenaRaw<'static>, usize);

// Union can't derive Copy/Clone — implement manually.
// SAFETY: All fields are Copy types, so bitwise copy is always valid
// regardless of which variant is active.
impl Copy for ArenaRaw<'_> {}

impl Clone for ArenaRaw<'_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl fmt::Debug for ArenaRaw<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ArenaRaw(..)")
    }
}

impl<'arena> RawValue for ArenaRaw<'arena> {
    type ArrayHandle = ThinRef<'arena, [&'arena Val<ArenaValueBuilder<'arena>>]>;

    fn from_int(value: i64) -> Self {
        ArenaRaw { int: value }
    }

    fn from_bool(value: bool) -> Self {
        ArenaRaw { bool: value }
    }

    fn from_float(value: f64) -> Self {
        ArenaRaw { float: value }
    }

    fn from_array(handle: Self::ArrayHandle) -> Self {
        ArenaRaw { array: handle }
    }

    fn as_int_unchecked(&self) -> i64 {
        // SAFETY: Caller guarantees this is an Int value (checked via Value's type field).
        unsafe { self.int }
    }

    fn as_bool_unchecked(&self) -> bool {
        // SAFETY: Caller guarantees this is a Bool value (checked via Value's type field).
        unsafe { self.bool }
    }

    fn as_float_unchecked(&self) -> f64 {
        // SAFETY: Caller guarantees this is a Float value (checked via Value's type field).
        unsafe { self.float }
    }

    fn as_array_unchecked(&self) -> &Self::ArrayHandle {
        // SAFETY: Caller guarantees this is an Array value (checked via Value's type field).
        unsafe { &self.array }
    }
}

// =============================================================================
// ArenaValueBuilder - Arena-based value builder (no Rc, no interning)
// =============================================================================

/// Value builder that uses arena allocation.
///
/// Values are allocated in a [`Bump`] arena — no reference counting needed.
/// Handles are plain references (`&'arena`), making them `Copy`.
///
/// Mirrors [`ArenaBuilder`] from the types crate.
///
/// # Example
///
/// ```ignore
/// use bumpalo::Bump;
/// use melbi_values::builders::ArenaValueBuilder;
/// use melbi_values::dynamic::Value;
/// use melbi_values::traits::ValueView;
///
/// let arena = Bump::new();
/// let builder = ArenaValueBuilder::new(&arena);
/// let v = Value::int(&builder, 42);
/// assert_eq!(v.as_int(), Some(42));
/// ```
#[derive(Copy, Clone, Debug)]
pub struct ArenaValueBuilder<'arena> {
    arena: &'arena Bump,
    type_builder: ArenaBuilder<'arena>,
}

impl<'arena> ArenaValueBuilder<'arena> {
    /// Create a new arena value builder.
    pub fn new(arena: &'arena Bump) -> Self {
        Self {
            arena,
            type_builder: ArenaBuilder::new(arena),
        }
    }
}

impl<'arena> ValueBuilder for ArenaValueBuilder<'arena> {
    type TB = ArenaBuilder<'arena>;
    type Raw = ArenaRaw<'arena>;
    type ValHandle = &'arena Val<Self>;
    type ArrayHandle = ThinRef<'arena, [Self::ValHandle]>;

    fn ty_builder(&self) -> &ArenaBuilder<'arena> {
        &self.type_builder
    }

    fn alloc_val(&self, raw: ArenaRaw<'arena>) -> Self::ValHandle {
        self.arena.alloc(Val::new(raw))
    }

    fn alloc_array(
        &self,
        elements: impl IntoIterator<Item = Self::ValHandle, IntoIter: ExactSizeIterator>,
    ) -> Self::ArrayHandle {
        ThinRef::from_slice(self.arena, elements)
    }
}
