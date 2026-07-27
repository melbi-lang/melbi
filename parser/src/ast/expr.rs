//! The expression tree.
//!
//! # What is missing
//!
//! Five surface constructs are deliberately absent, each blocked on a design
//! decision rather than on effort. They are listed here so the gap is visible
//! from the type itself:
//!
//! | Construct | Blocked on |
//! |---|---|
//! | `expr match { pat -> body, … }` | how a match arm is represented, and the pattern tree |
//! | `expr where { a = 1, b = 2 }` | how a binding is represented |
//! | `{ a = 1, b = 2 }` (record) | same as `where` — a record field and a binding have the same shape |
//! | `{ 1: "one" }` (map) | how a map entry is represented |
//! | `x as T` before resolution | the type-syntax tree (`UnresolvedCast`) |
//!
//! The first four each need a node for something currently stored as a tuple in
//! a list — a match arm, a binding, a map entry — because a tuple in a list has
//! no span. Whether those become plain structs with a `span` field or nodes in
//! their own builders is undecided.
//!
//! The fifth is half present: [`ExprKind::Cast`] is the *resolved* form, whose
//! target type lives in the node's `TreeData` rather than in the kind. The
//! unresolved form the parser emits needs a separate tree for type syntax.

use crate::{Tree, TreeBuilder};

use super::operators::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};

/// A node of the expression tree.
///
/// Every child is a [`Tree<B>`], so this enum is self-recursive and needs no
/// other tree — which is exactly why the constructs listed in the module docs
/// are missing: each of them reaches outside this tree.
///
/// `Eq` and `Hash` are absent by necessity, not choice: [`Float`] holds an
/// `f64`. `PartialEq` is derived, which works because `Tree`'s own `PartialEq`
/// is unconditional.
///
/// [`Float`]: ExprKind::Float
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind<B: TreeBuilder> {
    // --- Literals ---
    /// ``42``, ``0x2a``, ``0b1010``, or with a unit of measurement:
    /// ``42`m` ``, ``0o755`B/s` ``.
    ///
    /// The suffix is a whole expression, not a name: a unit may be a product,
    /// quotient or power, as in ``9.81`m/s^2` ``. The grammar admits any
    /// expression there and a later pass rejects everything outside
    /// identifiers, integers, `*`, `/` and `^`.
    ///
    /// See `docs/design/units-of-measurement.md`.
    Int {
        value: i64,
        suffix: Option<Tree<B>>,
    },
    /// ``3.14``, ``1.5e-10``, ``9.81`m/s^2` `` — see [`Int`](ExprKind::Int) for
    /// the suffix.
    Float {
        value: f64,
        suffix: Option<Tree<B>>,
    },
    /// `true`, `false`
    Bool(bool),
    /// `"hello"`
    Str(B::Str),
    /// `b"hello"`
    Bytes(B::Bytes),
    /// `f"a { x } b"`.
    ///
    /// REQUIRES: `strs.len() == exprs.len() + 1` — the literal pieces surround
    /// the interpolated ones.
    FormatStr { strs: B::StrList, exprs: B::List },

    // --- Names ---
    /// `x`
    Ident(B::Str),

    // --- Operators ---
    /// `a + b`, `a ^ b`
    Binary {
        op: BinaryOp,
        left: Tree<B>,
        right: Tree<B>,
    },
    /// `a and b`, `a or b`
    Boolean {
        op: BoolOp,
        left: Tree<B>,
        right: Tree<B>,
    },
    /// `a == b`, `a in b`
    Comparison {
        op: ComparisonOp,
        left: Tree<B>,
        right: Tree<B>,
    },
    /// `-a`, `not a`
    Unary { op: UnaryOp, expr: Tree<B> },

    // --- Postfix ---
    /// `f(a, b)`
    Call { callable: Tree<B>, args: B::List },
    /// `a[i]`
    Index { value: Tree<B>, index: Tree<B> },
    /// `a.b`
    Field { value: Tree<B>, field: B::Str },
    /// `x as T`, *after* the annotation has been resolved.
    ///
    /// The target type is the node's own type, and so lives in `TreeData`; a
    /// cast carries nothing beyond its operand.
    Cast { expr: Tree<B> },

    // --- Control flow ---
    /// `if cond then a else b`
    If {
        cond: Tree<B>,
        then_branch: Tree<B>,
        else_branch: Tree<B>,
    },
    /// `primary otherwise fallback`
    Otherwise {
        primary: Tree<B>,
        fallback: Tree<B>,
    },

    // --- Options ---
    /// `some expr`
    Some(Tree<B>),
    /// `none`
    None,

    // --- Collections ---
    /// `[1, 2, 3]`
    Array(B::List),

    // --- Functions ---
    /// `(x, y) => body`
    //
    // TODO: parameters are plain names today. Allowing patterns here
    // (`(some x) => …`) makes each parameter a node in the pattern tree, which
    // is also what would give a parameter its own span.
    Lambda { params: B::StrList, body: Tree<B> },
}
