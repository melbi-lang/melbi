//! A small tree and two passes over it, shared by the builder tests.
//!
//! Everything here is generic over `B: TreeBuilder`, so the same descriptor and
//! the same passes are reused by every storage strategy. That is the property
//! being tested: an algorithm over the tree names no allocator and no lifetime.
//!
//! Note what is *absent* compared with the previous design: there is no
//! `SampleBuilder` shorthand trait pinning `TreeData` and `TreeKind` onto the
//! builder. Both now come from the descriptor, so a pass needs nothing but
//! `B: TreeBuilder`.

use crate::{Span, Tree, TreeBuilder, TreeDescriptor, TreeNode, Visit};

/// The descriptor for this test tree.
pub struct Sample;

impl TreeDescriptor for Sample {
    type Data = Span;
    type Kind<B: TreeBuilder> = Expr<B>;
}

// `Eq` is omitted only because `#[derive(Eq)]` on a generic kind cannot state
// the bound it needs. A kind that wants `Eq` writes it by hand; see the
// conditional impls in `tree_builder.rs`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<B: TreeBuilder> {
    Lit(i64),
    Ident(B::Str),
    Add(Tree<B, Sample>, Tree<B, Sample>),
    Sum(B::List<Sample>),
    Bytes(B::Bytes),
    Format {
        strs: B::StrList,
        exprs: B::List<Sample>,
    },
}

/// `[1, 1 + (2 + x)]` — covers a shared subtree, a list, and an interned string.
pub fn sample<B: TreeBuilder>(builder: &B) -> Tree<B, Sample> {
    let one = TreeNode::new(Span(0, 1), Expr::Lit(1)).alloc(builder);
    let two = TreeNode::new(Span(4, 5), Expr::Lit(2)).alloc(builder);
    let x = TreeNode::new(Span(8, 9), Expr::Ident(builder.alloc_str("x"))).alloc(builder);
    let inner = TreeNode::new(Span(4, 9), Expr::Add(two, x)).alloc(builder);
    let add = TreeNode::new(Span(0, 9), Expr::Add(one.clone(), inner)).alloc(builder);
    let list = builder.alloc_list([one, add]);
    TreeNode::new(Span(0, 12), Expr::Sum(list)).alloc(builder)
}

/// `f"a = {7}!"` next to `b"hi"`, so byte strings and string lists are covered.
pub fn sample_with_literals<B: TreeBuilder>(builder: &B) -> Tree<B, Sample> {
    let strs = builder.alloc_str_list([builder.alloc_str("a = "), builder.alloc_str("!")]);
    let exprs = builder.alloc_list([TreeNode::new(Span(4, 5), Expr::Lit(7)).alloc(builder)]);
    let format = TreeNode::new(Span(0, 8), Expr::Format { strs, exprs }).alloc(builder);
    let bytes = TreeNode::new(Span(9, 15), Expr::Bytes(builder.alloc_bytes(b"hi"))).alloc(builder);
    TreeNode::new(Span(0, 15), Expr::Sum(builder.alloc_list([format, bytes]))).alloc(builder)
}

// --- A stateful traversal: sum every literal ---------------------------------

#[derive(Default)]
pub struct SumLiterals {
    pub total: i64,
    pub nodes_seen: usize,
}

impl<B: TreeBuilder> Visit<B, Sample, SumLiterals> for Expr<B> {
    type Output = ();

    fn visit(&self, _data: &Span, ctx: &mut SumLiterals) {
        ctx.nodes_seen += 1;
        match self {
            Expr::Lit(value) => ctx.total += value,
            Expr::Ident(_) | Expr::Bytes(_) => {}
            Expr::Add(left, right) => {
                left.visit(ctx);
                right.visit(ctx);
            }
            Expr::Sum(items) => {
                for item in items.iter() {
                    item.visit(ctx);
                }
            }
            Expr::Format { exprs, .. } => {
                for expr in exprs.iter() {
                    expr.visit(ctx);
                }
            }
        }
    }
}

// --- A rebuilding traversal: copy into another builder ------------------------

/// The destination builder rides in the context, which is why the source and
/// destination builders never meet in a signature — and why neither of their
/// lifetimes appears in this pass.
pub struct Rebuild<Out: TreeBuilder> {
    pub out: Out,
}

impl<In: TreeBuilder, Out: TreeBuilder> Visit<In, Sample, Rebuild<Out>> for Expr<In> {
    type Output = Tree<Out, Sample>;

    fn visit(&self, data: &Span, ctx: &mut Rebuild<Out>) -> Tree<Out, Sample> {
        let kind = match self {
            Expr::Lit(value) => Expr::Lit(*value),
            Expr::Ident(name) => Expr::Ident(ctx.out.alloc_str(name.as_ref())),
            Expr::Add(left, right) => Expr::Add(left.visit(ctx), right.visit(ctx)),
            // The builder is cloned so the list can be allocated while `ctx` is
            // still borrowed mutably by the per-item visits.
            Expr::Sum(items) => {
                let out = ctx.out.clone();
                Expr::Sum(out.alloc_list(items.iter().map(|item| item.visit(ctx))))
            }
            Expr::Bytes(bytes) => Expr::Bytes(ctx.out.alloc_bytes(bytes)),
            Expr::Format { strs, exprs } => {
                let out = ctx.out.clone();
                Expr::Format {
                    strs: out.alloc_str_list(strs.iter().map(|s| out.alloc_str(s.as_ref()))),
                    exprs: out.alloc_list(exprs.iter().map(|expr| expr.visit(ctx))),
                }
            }
        };
        // TODO: the data is copied verbatim here because both ends use the same
        // descriptor. A pass that changes the descriptor (lowering: `Span` ->
        // `{ty, span}`) constructs it explicitly at every node. See the TODO on
        // `TreeNode::new`.
        TreeNode::new(*data, kind).alloc(&ctx.out)
    }
}
