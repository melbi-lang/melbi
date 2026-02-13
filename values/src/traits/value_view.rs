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

    // Complex Types: Return the associated types from the System
    fn as_array(&self) -> Option<impl ArrayView<Value<VB>>>;

    // fn as_map(&self) -> Option<S::Map>;
    // fn as_string(&self) -> Option<S::String>;
}
