use melbi_types::Ty;

use crate::{
    dynamic::Value,
    traits::{ArrayView, ValueBuilder},
};

pub trait ValueView<VB: ValueBuilder>: Sized {
    fn ty(&self) -> Ty<VB::TB>;

    // Primitives: Return standard Rust types
    fn as_int(&self) -> Option<i64>;
    fn as_bool(&self) -> Option<bool>;
    fn as_float(&self) -> Option<f64>;

    // Complex Types
    fn as_array(&self) -> Option<impl ArrayView<Value<VB>>>;

    // TODO: fn as_map(&self) -> Option<...>;
    // TODO: fn as_string(&self) -> Option<...>;
}
