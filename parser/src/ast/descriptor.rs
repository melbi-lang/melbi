//! The descriptors of the parsed AST, and the data its nodes carry.
//!
//! Two trees at the parsed stage — expressions and literals — which are
//! mutually recursive: an expression contains literals, and a numeric literal
//! carries a unit suffix that is itself an expression (`` 9.81`m/s^2` ``).
//!
//! Under the descriptor design that recursion needs no ceremony at all. Both
//! trees are hosted by the same builder, so [`ParsedExprKind`] simply holds a
//! `Tree<B, ParsedLiteral>` and [`ParsedLiteralKind`] holds a
//! `Tree<B, ParsedExpr>`. There is no knot of paired associated types, and a
//! pass crossing between them writes nothing beyond `B: TreeBuilder`.

use crate::{Span, TreeBuilder, TreeDescriptor};

use super::{ParsedExprKind, ParsedLiteralKind};

/// The expression tree, as the parser produces it.
pub struct ParsedExpr;

/// The literal tree, as the parser produces it.
///
/// Literals live in their own tree purely as a matter of grouping — nothing
/// forces it. They are not a recursion point: a literal appears in exactly one
/// position and never nests inside itself. The unit suffix is a back-edge into
/// the expression tree, not self-nesting, so inlining these variants into
/// [`ParsedExprKind`] would work too.
///
/// What the separation buys is a name for the set of nodes that **fold away**.
/// A literal denotes a value, so constant folding replaces it outright, and the
/// typed stage will have no literal tree at all — lowering a `ParsedLiteral`
/// produces a value rather than a node. Interpolated format strings are
/// therefore *not* here: `f"a { x } b"` denotes no value until its holes are
/// filled, so it stays an expression.
pub struct ParsedLiteral;

/// Data on every node of the parsed AST.
///
/// A struct rather than a bare [`Span`] so that adding a field later — a
/// cached subtree summary, say — does not touch every construction site's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ParsedData {
    pub span: Span,
}

impl ParsedData {
    pub fn new(span: Span) -> Self {
        Self { span }
    }
}

impl From<Span> for ParsedData {
    fn from(span: Span) -> Self {
        Self::new(span)
    }
}

impl TreeDescriptor for ParsedExpr {
    type Data = ParsedData;
    type Kind<B: TreeBuilder> = ParsedExprKind<B>;
}

impl TreeDescriptor for ParsedLiteral {
    type Data = ParsedData;
    type Kind<B: TreeBuilder> = ParsedLiteralKind<B>;
}
