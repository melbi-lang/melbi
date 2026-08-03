//! The parsed type-expression tree — type *syntax*, before it means anything.
//!
//! Ported from `TypeExpr` in `core/src/parser/parsed_expr.rs`. Resolving one of
//! these into an actual type is `core/src/types/from_parser.rs`, which will move
//! to `melbi-types`.

use super::{TypeExpr, TypeField};
use crate::{Tree, TreeBuilder};

/// A node of the parsed type-expression tree.
///
/// This is deliberately *syntax*: `Path("Frobnicate")` is representable and only
/// rejected when resolved. Keeping the grammar permissive and the checking in
/// one place is the same trade the unit suffix makes.
///
/// Unlike the prototype's `TypeExpr`, every node here is allocated, so each
/// carries a span — which is what lets "unknown type `Frobnicate`" underline the
/// name rather than the whole cast.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExprKind<B: TreeBuilder> {
    /// `Int`, `Str`, `MyType` — a bare name.
    Path(B::Str),
    /// `Array[Int]`, `Map[Str, Int]`, `Option[T]`.
    Parametrized {
        path: B::Str,
        params: B::List<TypeExpr>,
    },
    /// `Record[a: Int, b: Str]`.
    Record(B::List<TypeField>),
    //
    // There is deliberately no function type. `(Int, Str) => Bool` is valid
    // Melbi type *notation*, but nothing has ever needed to write one: a
    // function's type is inferred, never annotated. The only use anyone has
    // identified is selecting between overloads, and that need has not arisen.
    //
    // If it does, the variant is
    // `Function { params: B::List<TypeExpr>, result: Tree<B, TypeExpr> }`,
    // and it has to land together with the matching arm in the resolver.
}

/// `name: Type` — one field of a `Record[…]` type.
///
/// A node rather than a `(B::Str, TypeExpr)` pair in a list, for the same reason
/// as a binding: a field is something the user writes and an error can point at.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeFieldKind<B: TreeBuilder> {
    pub name: B::Str,
    pub ty: Tree<B, TypeExpr>,
}
