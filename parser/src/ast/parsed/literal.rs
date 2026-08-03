//! Literals, shared by expressions and patterns.

use super::Expr;
use crate::{Tree, TreeBuilder};

/// A literal, inlined into [`ExprKind::Literal`] and [`PatternKind::Literal`].
///
/// Deliberately *not* a tree of its own. A literal's span is always exactly the
/// span of the node holding it, so allocating a second node would buy nothing
/// but a duplicate span; sharing this enum between the two positions already
/// gives the reuse a shared tree would have.
///
/// It is still a recursion point, through the unit suffix: a literal reaches
/// back into the expression tree, from both the expression *and* the pattern
/// side.
///
/// `Eq` and `Hash` are absent by necessity: [`Float`] holds an `f64`.
///
/// [`Float`]: LiteralKind::Float
/// [`ExprKind::Literal`]: super::ExprKind::Literal
/// [`PatternKind::Literal`]: super::PatternKind::Literal
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralKind<B: TreeBuilder> {
    /// ``42``, ``0x2a``, ``0b1010``, or with a unit: ``42`m` ``, ``0o755`B/s` ``.
    ///
    /// The suffix is a whole expression, not a name: a unit may be a product,
    /// quotient or power, as in ``9.81`m/s^2` ``. The grammar admits any
    /// expression there and a later pass rejects everything outside
    /// identifiers, integers, `*`, `/` and `^`.
    ///
    /// This is the edge that makes the literal and expression trees mutually
    /// recursive. See `docs/design/units-of-measurement.md`.
    Int {
        value: i64,
        suffix: Option<Tree<B, Expr>>,
    },
    /// ``3.14``, ``1.5e-10``, ``9.81`m/s^2` `` — see [`Int`](LiteralKind::Int)
    /// for the suffix.
    Float {
        value: f64,
        suffix: Option<Tree<B, Expr>>,
    },
    /// `true`, `false`
    Bool(bool),
    /// `"hello"`
    Str(B::Str),
    /// `b"hello"`
    Bytes(B::Bytes),
}

impl<B: TreeBuilder> LiteralKind<B> {
    /// The unit suffix, if this is a numeric literal carrying one.
    pub fn suffix(&self) -> Option<&Tree<B, Expr>> {
        match self {
            LiteralKind::Int { suffix, .. } | LiteralKind::Float { suffix, .. } => suffix.as_ref(),
            _ => None,
        }
    }
}
