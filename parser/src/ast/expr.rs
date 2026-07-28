//! The parsed expression tree.
//!
//! # What is missing
//!
//! Four surface constructs are deliberately absent, each blocked on a design
//! decision rather than on effort. They are listed here so the gap is visible
//! from the type itself:
//!
//! | Construct | Blocked on |
//! |---|---|
//! | `expr match { pat -> body, … }` | the pattern tree, and how a match arm is represented |
//! | `expr where { a = 1, b = 2 }` | how a binding is represented |
//! | `{ a = 1, b = 2 }` (record) | same as `where` — a record field and a binding have the same shape |
//! | `{ 1: "one" }` (map) | how a map entry is represented |
//!
//! Each needs a node for something that would otherwise be a tuple in a list —
//! a match arm, a binding, a map entry — because a tuple in a list has no span.
//! Under the descriptor design that is now a cheap answer rather than an
//! expensive one: each becomes another `TreeDescriptor` with its own `Data` and
//! `Kind`, sharing all of the tree machinery and adding nothing to any pass's
//! bounds.

use crate::{Tree, TreeBuilder};

use super::descriptor::{ParsedExpr, ParsedLiteral};
use super::operators::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};

/// A node of the parsed expression tree.
///
/// Apart from [`Literal`], every child is a [`Tree<B, ParsedExpr>`] in this same
/// tree.
///
/// `Eq` and `Hash` are absent by necessity, not choice: a literal may hold an
/// `f64`. `PartialEq` is derived, which works because `Tree`'s own `PartialEq`
/// is unconditional.
///
/// [`Literal`]: ParsedExprKind::Literal
/// [`Tree<B, ParsedExpr>`]: crate::Tree
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedExprKind<B: TreeBuilder> {
    // --- Literals ---
    /// ``42``, ``"hello"``, ``9.81`m/s^2` `` — a node of the literal tree.
    ///
    /// This is the edge into [`ParsedLiteralKind`](super::ParsedLiteralKind),
    /// and the literal's suffix is the edge back, which is what makes the two
    /// trees mutually recursive.
    Literal(Tree<B, ParsedLiteral>),
    /// `f"a { x } b"`.
    ///
    /// REQUIRES: `strs.len() == exprs.len() + 1` — the literal pieces surround
    /// the interpolated ones.
    FormatStr {
        strs: B::StrList,
        exprs: B::List<ParsedExpr>,
    },

    // --- Names ---
    /// `x` — a name, still unresolved. The typed stage replaces this with a
    /// resolved reference, which is one of the reasons the two stages do not
    /// share an enum.
    Ident(B::Str),

    // --- Operators ---
    /// `a + b`, `a ^ b`
    Binary {
        op: BinaryOp,
        left: Tree<B, ParsedExpr>,
        right: Tree<B, ParsedExpr>,
    },
    /// `a and b`, `a or b`
    Boolean {
        op: BoolOp,
        left: Tree<B, ParsedExpr>,
        right: Tree<B, ParsedExpr>,
    },
    /// `a == b`, `a in b`
    Comparison {
        op: ComparisonOp,
        left: Tree<B, ParsedExpr>,
        right: Tree<B, ParsedExpr>,
    },
    /// `-a`, `not a`
    Unary {
        op: UnaryOp,
        expr: Tree<B, ParsedExpr>,
    },

    // --- Postfix ---
    /// `f(a, b)`
    Call {
        callable: Tree<B, ParsedExpr>,
        args: B::List<ParsedExpr>,
    },
    /// `a[i]`
    Index {
        value: Tree<B, ParsedExpr>,
        index: Tree<B, ParsedExpr>,
    },
    /// `a.b`
    Field {
        value: Tree<B, ParsedExpr>,
        field: B::Str,
    },
    /// `x as T`, with `T` still unresolved.
    ///
    /// Distinct from the typed stage's cast, which carries only its operand
    /// because by then the target type is the node's own type and lives in the
    /// node's data. Naming both `Cast` is fine: they are variants of different
    /// enums.
    //
    // TODO: `ty_name` is a placeholder. It covers `x as Int` but not
    // `x as Array[Int]`, which needs a tree for type syntax — another
    // descriptor, once the shape of type expressions is settled.
    Cast {
        expr: Tree<B, ParsedExpr>,
        ty_name: B::Str,
    },

    // --- Control flow ---
    /// `if cond then a else b`
    If {
        cond: Tree<B, ParsedExpr>,
        then_branch: Tree<B, ParsedExpr>,
        else_branch: Tree<B, ParsedExpr>,
    },
    /// `primary otherwise fallback`
    Otherwise {
        primary: Tree<B, ParsedExpr>,
        fallback: Tree<B, ParsedExpr>,
    },

    // --- Options ---
    /// `some expr`
    Some(Tree<B, ParsedExpr>),
    /// `none`
    None,

    // --- Collections ---
    /// `[1, 2, 3]`
    Array(B::List<ParsedExpr>),

    // --- Functions ---
    /// `(x, y) => body`
    //
    // TODO: parameters are plain names today. Allowing patterns here
    // (`(some x) => …`) makes each parameter a node in the pattern tree, which
    // is also what would give a parameter its own span.
    Lambda {
        params: B::StrList,
        body: Tree<B, ParsedExpr>,
    },
}

// `#[path]` keeps the test beside this file instead of forcing an `expr/`
// directory for a single module, matching `core/src/stdlib/string.rs`.
#[cfg(test)]
#[path = "expr_test.rs"]
mod expr_test;
