//! The parsed expression tree, plus the two small nodes reached only from it.

use super::super::operators::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};
use super::{Binding, Expr, LiteralKind, MapEntry, MatchArm, TypeExpr};
use crate::{Tree, TreeBuilder};

/// A node of the parsed expression tree.
///
/// `Eq` and `Hash` are absent by necessity, not choice: a literal may hold an
/// `f64`. `PartialEq` is derived, which works because `Tree`'s own `PartialEq`
/// is unconditional.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind<B: TreeBuilder> {
    // --- Literals ---
    /// ``42``, ``"hello"``, ``9.81`m/s^2` ``.
    ///
    /// Inline rather than a tree of its own: a literal's span is always exactly
    /// this node's span, so a separate node would only duplicate it. The same
    /// enum appears in [`PatternKind::Literal`], which is the reuse a shared
    /// tree would have bought.
    ///
    /// The literal's unit suffix is an edge back into this tree, which is what
    /// makes expressions and literals mutually recursive.
    ///
    /// [`PatternKind::Literal`]: super::PatternKind::Literal
    Literal(LiteralKind<B>),
    /// `f"a { x } b"`.
    ///
    /// REQUIRES: `strs.len() == exprs.len() + 1` — the literal pieces surround
    /// the interpolated ones.
    //
    // TODO: parallel lists are the one place a child still has no span of its
    // own. Unlike a binding or a map entry, a format-string hole is not a
    // construct the user names, so it is left alone for now.
    FormatStr {
        strs: B::StrList,
        exprs: B::List<Expr>,
    },

    // --- Names ---
    /// `x` — a name, still unresolved. The typed stage replaces this with a
    /// resolved reference, which is one reason the two stages do not share an
    /// enum.
    Ident(B::Str),

    // --- Operators ---
    /// `a + b`, `a ^ b`
    Binary {
        op: BinaryOp,
        left: Tree<B, Expr>,
        right: Tree<B, Expr>,
    },
    /// `a and b`, `a or b`
    Boolean {
        op: BoolOp,
        left: Tree<B, Expr>,
        right: Tree<B, Expr>,
    },
    /// `a == b`, `a in b`
    Comparison {
        op: ComparisonOp,
        left: Tree<B, Expr>,
        right: Tree<B, Expr>,
    },
    /// `-a`, `not a`
    Unary { op: UnaryOp, expr: Tree<B, Expr> },

    // --- Postfix ---
    /// `f(a, b)`
    Call {
        callable: Tree<B, Expr>,
        args: B::List<Expr>,
    },
    /// `a[i]`
    Index {
        value: Tree<B, Expr>,
        index: Tree<B, Expr>,
    },
    /// `a.b`
    Field { value: Tree<B, Expr>, field: B::Str },
    /// `x as T`, with `T` still unresolved type *syntax*.
    ///
    /// Distinct from the typed stage's cast, which carries only its operand
    /// because by then the target type is the node's own type and lives in the
    /// node's data. Naming both `Cast` is fine: they are variants of different
    /// enums in different modules.
    Cast {
        expr: Tree<B, Expr>,
        ty: Tree<B, TypeExpr>,
    },

    // --- Control flow ---
    /// `if cond then a else b`
    If {
        cond: Tree<B, Expr>,
        then_branch: Tree<B, Expr>,
        else_branch: Tree<B, Expr>,
    },
    /// `primary otherwise fallback`
    Otherwise {
        primary: Tree<B, Expr>,
        fallback: Tree<B, Expr>,
    },
    /// `expr match { pat -> body, … }`
    Match {
        scrutinee: Tree<B, Expr>,
        arms: B::List<MatchArm>,
    },
    /// `expr where { a = 1, b = 2 }`
    Where {
        expr: Tree<B, Expr>,
        bindings: B::List<Binding>,
    },

    // --- Options ---
    /// `some expr`
    Some(Tree<B, Expr>),
    /// `none`
    None,

    // --- Collections ---
    /// `[1, 2, 3]`
    Array(B::List<Expr>),
    /// `{ a = 1, b = 2 }` — a record literal.
    ///
    /// Shares [`Binding`] with `where`: both are `name = expr`. Nothing in the
    /// types stops a pass confusing the two, which is the price of sharing.
    Record(B::List<Binding>),
    /// `{ 1: "one", k: v }` — a map literal.
    Map(B::List<MapEntry>),

    // --- Functions ---
    /// `(x, y) => body`
    //
    // TODO: parameters are plain names today. Allowing patterns here
    // (`(some x) => …`) makes each parameter a `Tree<B, Pattern>`, which is also
    // what would give a parameter its own span. The pattern tree now exists, so
    // this is a small change whenever the grammar wants it.
    Lambda {
        params: B::StrList,
        body: Tree<B, Expr>,
    },
}

/// `name = expr` — one binding of a `where` block, or one field of a record
/// literal.
///
/// A node rather than a `(B::Str, Tree<B, Expr>)` pair in a list, so that it has
/// a span: "unused binding `x`" wants to point at the binding, not the block.
#[derive(Debug, Clone, PartialEq)]
pub struct BindingKind<B: TreeBuilder> {
    pub name: B::Str,
    pub value: Tree<B, Expr>,
}

/// `key: value` — one entry of a map literal.
///
/// The key is a whole expression: `{1 + 2: 3}` is legal.
#[derive(Debug, Clone, PartialEq)]
pub struct MapEntryKind<B: TreeBuilder> {
    pub key: Tree<B, Expr>,
    pub value: Tree<B, Expr>,
}
