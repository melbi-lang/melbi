#![deny(unsafe_code)]

pub mod alpha_converter;
pub mod constraint_set;
pub mod encoding;
pub mod from_parser;
pub mod manager;
pub mod registry;
pub mod traits;
pub mod type_class;
pub mod type_class_resolver;
pub mod type_scheme;
mod types;
pub mod unification;

mod serialization;

#[cfg(test)]
mod manager_test;

pub use constraint_set::{ConstraintSet, TypeClassConstraint};
pub use from_parser::{TypeConversionError, type_expr_to_type};
pub use type_class::{TypeClassId, has_instance};
pub use type_class_resolver::{ConstraintError, TypeClassResolver};
pub use type_scheme::TypeScheme;
pub use types::Type;
