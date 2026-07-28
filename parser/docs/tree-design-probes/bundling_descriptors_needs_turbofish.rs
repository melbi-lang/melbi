//! **Expected: E0283.** Bundling descriptors behind a trait, so a pass can be
//! generic over "the whole AST" instead of naming each tree.
//!
//! This is tempting exactly once per person: `Tree<B, A::Expr>` reads fine, and
//! it means a pass takes one type parameter instead of naming `ParsedExpr`,
//! `ParsedLiteral`, … individually.
//!
//! It does not work. `A::Expr` is an associated-type *projection*, and
//! projections are not injective — two different `Ast` impls may pick the same
//! `Expr` — so `A` cannot be recovered from `Tree<B, A::Expr>`. Every call in
//! every pass then needs `::<B, A>`, forever.
//!
//! Name the descriptor directly instead: `Tree<B, ParsedExpr>` is injective in
//! both parameters, and inference works with no annotations even when a pass
//! crosses from one tree into another and back.
//!
//! An earlier version of this probe bundled *builders* rather than descriptors,
//! back when there was one builder trait per tree. Same error, same reason; the
//! shape it takes under the current design is the one below.

use std::fmt::Debug;

pub trait TreeDescriptor: Sized + 'static {
    type Kind<B: TreeBuilder>: Clone + Debug;
}

pub trait TreeBuilder: Sized {
    type Handle<D: TreeDescriptor>: AsRef<TreeNode<Self, D>> + Clone;
}

pub struct Tree<B: TreeBuilder, D: TreeDescriptor>(B::Handle<D>);
pub struct TreeNode<B: TreeBuilder, D: TreeDescriptor> {
    pub kind: D::Kind<B>,
}

/// The tempting bundle.
pub trait Ast {
    type Expr: TreeDescriptor;
    type Lit: TreeDescriptor;
}

/// `A` is unconstrained by the argument type, so the recursive call cannot infer
/// it: E0283.
pub fn count<B: TreeBuilder, A: Ast>(tree: &Tree<B, A::Expr>) -> usize {
    1 + count(tree)
}
