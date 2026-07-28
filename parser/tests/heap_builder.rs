//! Tests for the reference-counted heap builder.

mod common;

use std::rc::Rc;

use common::builders::HeapBuilder;
use common::sample_tree::{Expr, Rebuild, Sample, SumLiterals, sample, sample_with_literals};
use melbi_parser::{Span, TreeBuilder, TreeNode};

#[test]
fn builds_and_inspects_a_tree() {
    let builder = HeapBuilder;
    let root = sample(&builder);

    assert_eq!(*root.data(), Span(0, 12));

    let Expr::Sum(items) = root.kind() else {
        panic!("expected Sum, got {:?}", root.kind());
    };
    assert_eq!(items.len(), 2);
    assert_eq!(*items[0].kind(), Expr::Lit(1));

    // Reaching a grandchild goes through `kind()` only — no handle juggling.
    let Expr::Add(left, _) = items[1].kind() else {
        panic!("expected Add");
    };
    assert_eq!(*left.data(), Span(0, 1));
}

#[test]
fn interned_strings_resolve() {
    let builder = HeapBuilder;
    let name = builder.alloc_str("some_ident");
    // The descriptor has to be named here: `D::Kind<B>` is a projection, so
    // `Expr<HeapBuilder>` does not determine `Sample` on its own. Everywhere
    // else in these tests it is inferred from a `Tree<B, D>` in a signature or a
    // field, which is the usual case.
    let node = TreeNode::<HeapBuilder, Sample>::new(Span(0, 10), Expr::Ident(name)).alloc(&builder);

    let Expr::Ident(name) = node.kind() else {
        panic!("expected Ident");
    };
    assert_eq!(name.as_ref(), "some_ident");
}

#[test]
fn visit_threads_mutable_state() {
    let builder = HeapBuilder;
    let root = sample(&builder);

    let mut ctx = SumLiterals::default();
    root.visit(&mut ctx);

    // Literals 1, 1 and 2: the `1` is shared between two parents, and a shared
    // subtree is visited once per edge, not once per allocation — so it is
    // counted twice among the 7 visits of this 6-node tree.
    assert_eq!(ctx.total, 4);
    assert_eq!(ctx.nodes_seen, 7);
}

#[test]
fn visit_rebuilds_into_another_builder() {
    let source = HeapBuilder;
    let root = sample(&source);

    let mut ctx = Rebuild { out: HeapBuilder };
    let rebuilt = root.visit(&mut ctx);

    // Equality is structural, so a rebuilt tree compares equal to its source.
    assert_eq!(root, rebuilt);
}

#[test]
fn rebuilt_tree_does_not_share_storage_with_the_source() {
    let source = HeapBuilder;
    let root = sample(&source);
    let before = Rc::strong_count(root.handle());

    let mut ctx = Rebuild { out: HeapBuilder };
    let rebuilt = root.visit(&mut ctx);

    assert_eq!(Rc::strong_count(root.handle()), before);
    assert!(!Rc::ptr_eq(root.handle(), rebuilt.handle()));
}

#[test]
fn covers_bytes_and_format_strings() {
    let builder = HeapBuilder;
    let root = sample_with_literals(&builder);

    let mut ctx = SumLiterals::default();
    root.visit(&mut ctx);
    assert_eq!(ctx.total, 7);

    let mut ctx = Rebuild { out: HeapBuilder };
    assert_eq!(root, root.visit(&mut ctx));
}
