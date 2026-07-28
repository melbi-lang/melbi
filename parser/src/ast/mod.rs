//! The Melbi abstract syntax tree.
//!
//! Two mutually recursive trees at the parsed stage: expressions and literals.
//! See [`descriptor`] for how they are declared and why the mutual recursion
//! needs no ceremony.
//
// TODO: the typed stage — `TypedExpr` and friends — is not here yet. It is a
// second set of descriptors with their own data (span *and* type) and their own
// kinds: no literals at all (they fold to constants), resolved slots in place of
// `Ident`, and a `Cast` carrying only its operand. Adding it needs decisions
// about value and slot representation that belong with the analyzer, not here.

pub mod descriptor;
mod expr;
mod literal;
pub mod operators;

pub use descriptor::{ParsedData, ParsedExpr, ParsedLiteral};
pub use expr::ParsedExprKind;
pub use literal::ParsedLiteralKind;
pub use operators::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};

#[cfg(test)]
mod expr_test;
