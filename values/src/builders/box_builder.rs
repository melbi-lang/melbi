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
/// The arena builder (future) will use a union instead.
#[derive(Clone)]
pub enum BoxRaw {
    Int(i64),
    Bool(bool),
    Float(f64),
    Array(Rc<[Rc<Val<BoxValueBuilder>>]>),
}
static_assertions::assert_eq_size!(BoxRaw, (usize, usize, usize));

impl fmt::Debug for BoxRaw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoxRaw::Int(v) => write!(f, "Int({v})"),
            BoxRaw::Bool(v) => write!(f, "Bool({v})"),
            BoxRaw::Float(v) => write!(f, "Float({v})"),
            BoxRaw::Array(arr) => write!(f, "Array(len={})", arr.len()),
        }
    }
}

impl RawValue for BoxRaw {
    type ArrayHandle = Rc<[Rc<Val<BoxValueBuilder>>]>;

    fn from_int(value: i64) -> Self {
        BoxRaw::Int(value)
    }

    fn from_bool(value: bool) -> Self {
        BoxRaw::Bool(value)
    }

    fn from_float(value: f64) -> Self {
        BoxRaw::Float(value)
    }

    fn from_array(handle: Self::ArrayHandle) -> Self {
        BoxRaw::Array(handle)
    }

    fn as_int_unchecked(&self) -> i64 {
        match self {
            BoxRaw::Int(v) => *v,
            _ => unreachable!("as_int_unchecked called on non-int value"),
        }
    }

    fn as_bool_unchecked(&self) -> bool {
        match self {
            BoxRaw::Bool(v) => *v,
            _ => unreachable!("as_bool_unchecked called on non-bool value"),
        }
    }

    fn as_float_unchecked(&self) -> f64 {
        match self {
            BoxRaw::Float(v) => *v,
            _ => unreachable!("as_float_unchecked called on non-float value"),
        }
    }

    fn as_array_unchecked(&self) -> &Self::ArrayHandle {
        match self {
            BoxRaw::Array(v) => v,
            _ => unreachable!("as_array_unchecked called on non-array value"),
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
/// ```ignore
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
    tb: BoxBuilder,
}

impl BoxValueBuilder {
    /// Create a new box value builder.
    pub fn new() -> Self {
        Self {
            tb: BoxBuilder::new(),
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
    type ValueHandle = Rc<Val<Self>>;
    type ArrayHandle = Rc<[Self::ValueHandle]>;

    fn ty_builder(&self) -> &BoxBuilder {
        &self.tb
    }

    fn alloc_val(&self, raw: BoxRaw) -> Self::ValueHandle {
        Rc::new(Val::new(raw))
    }

    fn alloc_array(
        &self,
        elements: impl IntoIterator<Item = Self::ValueHandle, IntoIter: ExactSizeIterator>,
    ) -> Self::ArrayHandle {
        elements.into_iter().collect()
    }
}
