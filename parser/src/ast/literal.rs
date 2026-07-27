//! The literal tree.
//!
//! Literals live in their own tree purely as a matter of grouping — nothing
//! forces it. They are not a recursion point: a literal appears in exactly one
//! position and never nests inside itself. The unit suffix is a back-edge into
//! the expression tree, not self-nesting, so inlining these variants into
//! [`ExprKind`](super::ExprKind) would work too.
//!
//! What the separation buys is a name for the set of nodes that **fold away**.
//! A literal denotes a value, so constant folding replaces it outright: the
//! typed AST in `core/src/analyzer/typed_expr.rs` has no literals at all, only
//! `Constant(Value)`. Interpolated format strings are therefore *not* here —
//! `f"a { x } b"` denotes no value until its holes are filled, so it stays an
//! expression.

use crate::Tree;

use super::builder::{ExprTree, LiteralBuilder};

/// A node of the literal tree.
///
/// `Eq` and `Hash` are absent by necessity: [`Float`] holds an `f64`.
///
/// [`Float`]: LiteralKind::Float
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralKind<B: LiteralBuilder> {
    /// ``42``, ``0x2a``, ``0b1010``, or with a unit: ``42`m` ``, ``0o755`B/s` ``.
    ///
    /// The suffix is a whole expression, not a name: a unit may be a product,
    /// quotient or power, as in ``9.81`m/s^2` ``. The grammar admits any
    /// expression there and a later pass rejects everything outside
    /// identifiers, integers, `*`, `/` and `^`.
    ///
    /// This is the edge that makes the two trees mutually recursive. See
    /// `docs/design/units-of-measurement.md`.
    Int {
        value: i64,
        suffix: Option<ExprTree<B>>,
    },
    /// ``3.14``, ``1.5e-10``, ``9.81`m/s^2` `` — see [`Int`](LiteralKind::Int)
    /// for the suffix.
    Float {
        value: f64,
        suffix: Option<ExprTree<B>>,
    },
    /// `true`, `false`
    Bool(bool),
    /// `"hello"`
    Str(B::Str),
    /// `b"hello"`
    Bytes(B::Bytes),
}

impl<B: LiteralBuilder> LiteralKind<B> {
    /// The unit suffix, if this is a numeric literal carrying one.
    pub fn suffix(&self) -> Option<&Tree<B::Expr>> {
        match self {
            LiteralKind::Int { suffix, .. } | LiteralKind::Float { suffix, .. } => suffix.as_ref(),
            _ => None,
        }
    }

    /// Allocate this literal as a node of `builder`'s tree.
    pub fn alloc(self, builder: &B, data: B::TreeData) -> Tree<B> {
        crate::TreeNode::new(data, self).alloc(builder)
    }
}
