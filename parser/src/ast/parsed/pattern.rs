//! The parsed pattern tree, and the match arm that reaches it.

use crate::{Tree, TreeBuilder};

use super::{Expr, LiteralKind, Pattern};

/// A node of the parsed pattern tree.
///
/// Patterns are a tree of their own rather than a corner of [`ExprKind`] for the
/// usual reason: they *bind and match* where an expression *produces a value*,
/// and they can appear only in match arms. Making that a type distinction means
/// the exhaustiveness checker takes `Tree<B, Pattern>` and cannot be handed an
/// arbitrary expression.
///
/// [`ExprKind`]: super::ExprKind
#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind<B: TreeBuilder> {
    /// `_` — matches anything, binds nothing.
    Wildcard,
    /// `x` — matches anything and binds it.
    Binding(B::Str),
    /// `1`, `"hi"`, `true` — matches one literal value.
    ///
    /// The very same enum [`ExprKind::Literal`] holds, so parsing and printing a
    /// literal is written once and a pattern literal cannot drift from an
    /// expression one.
    ///
    /// Note this admits a suffixed numeric literal, and so a pattern can reach
    /// back into the expression tree through the unit suffix. Whether
    /// ``100`seconds` `` is a legal pattern is a later pass's decision, not the
    /// grammar's.
    ///
    /// [`ExprKind::Literal`]: super::ExprKind::Literal
    Literal(LiteralKind<B>),
    /// `some p`
    Some(Tree<B, Pattern>),
    /// `none`
    None,
}

/// `pattern -> body` — one arm of a `match`.
///
/// A node rather than a pair in a list, because an arm spans two trees and wants
/// a span of its own: "unreachable arm" points here.
//
// TODO: no guard (`pattern if cond -> body`). The grammar has none today; adding
// one is `guard: Option<Tree<B, Expr>>`.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArmKind<B: TreeBuilder> {
    pub pattern: Tree<B, Pattern>,
    pub body: Tree<B, Expr>,
}
