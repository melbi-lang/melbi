//! The Melbi abstract syntax tree.
//!
//! Two mutually recursive trees: expressions and literals. See
//! [`builder`] for how they name each other and why there is no `System` trait
//! bundling them.

pub mod builder;
mod expr;
mod literal;
pub mod operators;

pub use builder::{ExprBuilder, ExprTree, LiteralBuilder, LiteralTree};
pub use expr::ExprKind;
pub use literal::LiteralKind;
pub use operators::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};

#[cfg(test)]
mod expr_test;
