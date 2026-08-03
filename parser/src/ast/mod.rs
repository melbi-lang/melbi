//! The Melbi abstract syntax tree.
//!
//! One module per compiler stage — [`parsed`] today, `typed` once it exists —
//! each declaring its own descriptors. The stage lives in the module path rather
//! than in the type name, so code working within one stage says `Expr` and only
//! code spanning stages pays for `parsed::Expr` / `typed::Expr`.
//!
//! Operators sit outside any stage: `+` means the same thing before and after
//! type-checking, so both stages share [`BinaryOp`] and friends.

mod operators;
pub mod parsed;

pub use operators::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};

// TODO: the typed stage. A second set of descriptors with their own data (span
// *and* type) and their own kinds: no literal tree at all — literals fold to
// constants — resolved slots in place of `Ident`, and a `Cast` carrying only its
// operand. Adding it needs decisions about value and slot representation that
// belong with the analyzer, and it is what the `Folder` half of the visitor
// should be designed against.
