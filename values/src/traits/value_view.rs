use melbi_types::Ty;

use crate::{
    dynamic::Value,
    traits::{ArrayView, ValueBuilder},
};

/// A read-only view over a dynamically typed value.
///
/// Each accessor returns `Some` only when the underlying value matches the
/// requested type, and `None` otherwise.
pub trait ValueView<VB: ValueBuilder>: Sized {
    /// Returns the value's type.
    fn ty(&self) -> &Ty<VB::TB>;

    /// Returns the integer value, or `None` if not an `Int`.
    fn as_int(&self) -> Option<i64>;

    /// Returns the boolean value, or `None` if not a `Bool`.
    fn as_bool(&self) -> Option<bool>;

    /// Returns the float value, or `None` if not a `Float`.
    fn as_float(&self) -> Option<f64>;

    /// Returns an array view, or `None` if not an `Array`.
    fn as_array(&self) -> Option<impl ArrayView<Value<VB>>>;

    // TODO: fn as_map(&self) -> Option<...>;
    // TODO: fn as_string(&self) -> Option<...>;
}
