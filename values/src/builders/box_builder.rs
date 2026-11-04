use alloc::rc::Rc;
use core::fmt;

use melbi_types::BoxBuilder;

use crate::traits::{RawValue, Val, ValueBuilder};

// =============================================================================
// BoxRaw - Enum-based raw storage for heap allocation
// =============================================================================

/// Raw value storage for [`BoxValueBuilder`].
///
/// Uses an enum so that Clone and Drop work naturally (no unsafe needed).
/// The arena builder uses a union instead (see [`ArenaRaw`](super::arena_builder::ArenaRaw)).
#[derive(Clone)]
pub enum BoxRaw {
    Int(i64),
    Bool(bool),
    Float(f64),
    Str(Rc<str>),
    Bytes(Rc<[u8]>),
    // Elements are stored inline in the `Rc` allocation, not as a slice of
    // pointers to individually allocated elements.
    Array(Rc<[Val<BoxValueBuilder>]>),
}
// A tag plus the widest payload, which is a fat `Rc` (two words).
static_assertions::assert_eq_size!(BoxRaw, [usize; 3]);

impl fmt::Debug for BoxRaw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(v) => write!(f, "Int({v})"),
            Self::Bool(v) => write!(f, "Bool({v})"),
            Self::Float(v) => write!(f, "Float({v})"),
            Self::Str(s) => write!(f, "Str({s:?})"),
            Self::Bytes(b) => write!(f, "Bytes(len={})", b.len()),
            Self::Array(arr) => write!(f, "Array(len={})", arr.len()),
        }
    }
}

impl RawValue for BoxRaw {
    type ArrayHandle = Rc<[Val<BoxValueBuilder>]>;
    type StrHandle = Rc<str>;
    type BytesHandle = Rc<[u8]>;

    fn from_int(value: i64) -> Self {
        Self::Int(value)
    }

    fn from_bool(value: bool) -> Self {
        Self::Bool(value)
    }

    fn from_float(value: f64) -> Self {
        Self::Float(value)
    }

    fn from_array(handle: Self::ArrayHandle) -> Self {
        Self::Array(handle)
    }

    fn from_str(handle: Self::StrHandle) -> Self {
        Self::Str(handle)
    }

    fn from_bytes(handle: Self::BytesHandle) -> Self {
        Self::Bytes(handle)
    }

    fn as_int_unchecked(&self) -> i64 {
        match self {
            Self::Int(v) => *v,
            _ => unreachable!("as_int_unchecked called on non-int value"),
        }
    }

    fn as_bool_unchecked(&self) -> bool {
        match self {
            Self::Bool(v) => *v,
            _ => unreachable!("as_bool_unchecked called on non-bool value"),
        }
    }

    fn as_float_unchecked(&self) -> f64 {
        match self {
            Self::Float(v) => *v,
            _ => unreachable!("as_float_unchecked called on non-float value"),
        }
    }

    fn as_array_unchecked(&self) -> &Self::ArrayHandle {
        match self {
            Self::Array(v) => v,
            _ => unreachable!("as_array_unchecked called on non-array value"),
        }
    }

    fn as_str_unchecked(&self) -> &Self::StrHandle {
        match self {
            Self::Str(v) => v,
            _ => unreachable!("as_str_unchecked called on non-str value"),
        }
    }

    fn as_bytes_unchecked(&self) -> &Self::BytesHandle {
        match self {
            Self::Bytes(v) => v,
            _ => unreachable!("as_bytes_unchecked called on non-bytes value"),
        }
    }
}

// =============================================================================
// BoxValueBuilder - Heap-based value builder (Rc, no interning)
// =============================================================================

/// Value builder that uses reference counting (`Rc`) for allocation.
///
/// Simple heap-based builder with no deduplication. Useful for testing
/// and cases where arena allocation isn't needed.
///
/// Mirrors [`BoxBuilder`] from the types crate.
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
#[derive(Clone, Debug)]
pub struct BoxValueBuilder {
    type_builder: BoxBuilder,
}

impl BoxValueBuilder {
    /// Create a new box value builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            type_builder: BoxBuilder::new(),
        }
    }
}

impl Default for BoxValueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueBuilder for BoxValueBuilder {
    type TB = BoxBuilder;
    type Raw = BoxRaw;
    type ArrayHandle = Rc<[Val<Self>]>;
    type StrHandle = Rc<str>;
    type BytesHandle = Rc<[u8]>;

    fn ty_builder(&self) -> &BoxBuilder {
        &self.type_builder
    }

    fn alloc_array(
        &self,
        elements: impl IntoIterator<Item = Val<Self>, IntoIter: ExactSizeIterator>,
    ) -> Self::ArrayHandle {
        elements.into_iter().collect()
    }

    fn alloc_str(&self, value: &str) -> Self::StrHandle {
        Rc::from(value)
    }

    fn alloc_bytes(&self, value: &[u8]) -> Self::BytesHandle {
        Rc::from(value)
    }
}
