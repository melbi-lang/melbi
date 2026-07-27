//! The two builder traits of the AST, and how they name each other.
//!
//! The expression tree and the literal tree are mutually recursive: an
//! expression contains literals, and a numeric literal carries a unit suffix
//! that is itself an expression (`` 9.81`m/s^2` ``).
//!
//! Each trait names the other as an associated type, with an equality
//! constraint pointing back at `Self`. That mutual `<Expr = Self>` /
//! `<Lit = Self>` pair is what makes the two definitions close over each other,
//! and it is what lets a pass be generic over one builder while still reaching
//! the other.
//!
//! # Why there is no `System` trait bundling the builders
//!
//! The obvious alternative is a supertrait holding both builders, with passes
//! generic over it. It does not work. `S::Expr` is an associated-type
//! projection, and projections are not injective, so `S` cannot be recovered
//! from `Tree<S::Expr>`: every call in every pass needs a turbofish (E0283), and
//! the impl forwarding `Visit` from `Tree` cannot even be written (E0207,
//! unconstrained type parameter).
//!
//! Parameterizing passes over `B` directly, as below, avoids all of it —
//! `Tree<B>` *is* injective in `B`, so inference works with no annotations, even
//! when a pass crosses from one tree into the other and back.
//!
//! Probes for every claim above, with the exact error codes, are in
//! `parser/docs/poc-mutually-recursive-trees/`.

use crate::TreeBuilder;

use super::{ExprKind, LiteralKind};

/// A builder whose tree is the expression tree.
pub trait ExprBuilder: TreeBuilder<TreeKind = ExprKind<Self>> {
    /// The builder for the literal tree this expression tree pairs with.
    type Lit: LiteralBuilder<Expr = Self>;
}

/// A builder whose tree is the literal tree.
pub trait LiteralBuilder: TreeBuilder<TreeKind = LiteralKind<Self>> {
    /// The builder for the expression tree this literal tree pairs with.
    type Expr: ExprBuilder<Lit = Self>;
}

/// The expression tree paired with a literal builder — `Tree<Expr<B>>`.
pub type ExprTree<B> = crate::Tree<<B as LiteralBuilder>::Expr>;

/// The literal tree paired with an expression builder — `Tree<Lit<B>>`.
pub type LiteralTree<B> = crate::Tree<<B as ExprBuilder>::Lit>;
