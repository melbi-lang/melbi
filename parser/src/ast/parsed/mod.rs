//! The AST as the parser produces it.
//!
//! Every tree of this stage is declared here as a descriptor — an empty marker
//! struct naming its kind — so that `Tree<B, parsed::Expr>` and
//! `Tree<B, parsed::Pattern>` are distinct, injective types. The stage is part
//! of the module path rather than the type name, so a pass working within one
//! stage writes `Expr`, and only a pass spanning stages pays for
//! `parsed::Expr` / `typed::Expr`.
//!
//! # The trees, and why each one exists
//!
//! | Descriptor | Holds | Exists because |
//! |---|---|---|
//! | [`Expr`] | [`ExprKind`] | the root of the language |
//! | [`Pattern`] | [`PatternKind`] | patterns *bind* rather than produce values |
//! | [`MatchArm`] | [`MatchArmKind`] | an arm spans two trees and needs a span of its own |
//! | [`Binding`] | [`BindingKind`] | one `name = expr`, shared by `where` and record literals |
//! | [`MapEntry`] | [`MapEntryKind`] | one `key: value` |
//! | [`TypeExpr`] | [`TypeExprKind`] | type *syntax*, before it resolves to a type |
//! | [`TypeField`] | [`TypeFieldKind`] | one `name: Type` inside `Record[…]` |
//!
//! The last five exist for one reason each: they would otherwise be a tuple in a
//! list, and a tuple in a list has no span. The prototype in
//! `core/src/parser/parsed_expr.rs` stores them exactly that way — see
//! `bindings: &'a [(&'a str, &'a Expr<'a>)]` — which is the gap this closes.
//!
//! # What is deliberately *not* a tree
//!
//! [`LiteralKind`] is inlined into [`ExprKind::Literal`] and
//! [`PatternKind::Literal`] rather than being a tree of its own, because a
//! literal's span is always exactly its parent's — a literal tree would allocate
//! a second node per literal carrying a duplicate span. Sharing the enum between
//! the two positions still gets the reuse a shared tree would have.
//!
//! # Mutual recursion
//!
//! Four cycles run through these trees, and none needs any ceremony: every tree
//! is hosted by the same builder, so a kind simply holds a `Tree<B, D>` for
//! whichever `D` it wants.
//!
//! - expression → (inline literal) → unit suffix → expression
//! - expression → match arm → pattern → (inline literal) → unit suffix → expression
//! - expression → binding → expression
//! - expression → type expression → type field → type expression

use crate::{Span, TreeBuilder, TreeDescriptor};

mod expr;
pub mod fold;
mod literal;
mod pattern;
mod type_expr;

pub use expr::{BindingKind, ExprKind, MapEntryKind};
pub use literal::LiteralKind;
pub use pattern::{MatchArmKind, PatternKind};
pub use type_expr::{TypeExprKind, TypeFieldKind};

/// Data on every node of the parsed AST.
///
/// A struct rather than a bare [`Span`] so that adding a field later — a cached
/// subtree summary, say — does not change every construction site's shape.
///
/// Every parsed tree shares this. They diverge at the typed stage, where an
/// expression carries a type and a pattern additionally carries reachability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Data {
    pub span: Span,
}

impl Data {
    #[must_use]
    pub fn new(span: Span) -> Self {
        Self { span }
    }
}

impl From<Span> for Data {
    fn from(span: Span) -> Self {
        Self::new(span)
    }
}

/// Declares a descriptor whose data is [`Data`] and whose kind is `$kind`.
macro_rules! parsed_trees {
    ($($(#[$meta:meta])* $name:ident => $kind:ident),+ $(,)?) => {
        $(
            $(#[$meta])*
            pub struct $name;

            impl TreeDescriptor for $name {
                type Data = Data;
                type Kind<B: TreeBuilder> = $kind<B>;
            }
        )+
    };
}

parsed_trees! {
    /// `1 + f(x)` — the expression tree.
    Expr => ExprKind,

    /// `some x`, `_`, `1` — the pattern tree.
    Pattern => PatternKind,

    /// `pattern -> body`, one arm of a `match`.
    MatchArm => MatchArmKind,

    /// `name = expr`, in a `where` block or a record literal.
    Binding => BindingKind,

    /// `key: value`, one entry of a map literal.
    MapEntry => MapEntryKind,

    /// `Int`, `Array[Int]`, `Record[a: Int]` — type *syntax*.
    ///
    /// Resolving this to an actual type is `core/src/types/from_parser.rs`,
    /// which will move to `melbi-types`.
    TypeExpr => TypeExprKind,

    /// `name: Type`, one field of a `Record[…]` type.
    TypeField => TypeFieldKind,
}

// `#[path]` keeps the tests beside this module instead of forcing a directory
// for each one, matching `core/src/stdlib/string.rs`.
#[cfg(test)]
#[path = "test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "expr_test.rs"]
mod expr_test;

#[cfg(test)]
#[path = "fold_test.rs"]
mod fold_test;
