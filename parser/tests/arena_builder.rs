//! Tests for the bump-arena builder.
//!
//! The point of interest here is that `Tree<ArenaBuilder<'_>, Sample>` is
//! `Copy`, and that rebuilding from one arena into another is the same generic
//! pass used by the heap builder — neither arena's lifetime appears anywhere in
//! it.

mod common;

use std::ptr;

use bumpalo::Bump;

use common::builders::HeapBuilder;
use common::sample_tree::{Expr, Rebuild, SumLiterals, sample, sample_with_literals};
use melbi_parser::{ArenaBuilder, Span};

#[test]
fn builds_and_inspects_a_tree() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    let root = sample(&builder);

    assert_eq!(*root.data(), Span(0, 12));

    let Expr::Sum(items) = root.kind() else {
        panic!("expected Sum, got {:?}", root.kind());
    };
    assert_eq!(items.len(), 2);
    assert_eq!(*items[0].kind(), Expr::Lit(1));
}

#[test]
fn trees_are_copy() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    let root = sample(&builder);

    // No clone: an arena handle is a plain reference, so `Tree` is `Copy`.
    let alias = root;
    assert_eq!(*alias.data(), *root.data());
}

#[test]
fn visit_threads_mutable_state() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    let root = sample(&builder);

    let mut ctx = SumLiterals::default();
    root.visit(&mut ctx);

    assert_eq!(ctx.total, 4);
    assert_eq!(ctx.nodes_seen, 7);
}

#[test]
fn visit_rebuilds_from_one_arena_into_another() {
    let source_arena = Bump::new();
    let source = ArenaBuilder::new(&source_arena);
    let root = sample(&source);

    let dest_arena = Bump::new();
    let mut ctx = Rebuild {
        out: ArenaBuilder::new(&dest_arena),
    };
    let rebuilt = root.visit(&mut ctx);

    // Structural equality across two unrelated arenas. Note that neither
    // lifetime was named to get here.
    assert_eq!(root, rebuilt);
    assert!(!ptr::eq(root.node(), rebuilt.node()));
}

#[test]
fn visit_rebuilds_from_an_arena_into_the_heap() {
    let source_arena = Bump::new();
    let source = ArenaBuilder::new(&source_arena);
    let root = sample(&source);

    let mut ctx = Rebuild { out: HeapBuilder };
    let rebuilt = root.visit(&mut ctx);

    // The rebuilt tree outlives nothing in particular: it is reference counted,
    // while the source is arena allocated.
    let mut ctx = SumLiterals::default();
    rebuilt.visit(&mut ctx);
    assert_eq!(ctx.total, 4);
    assert_eq!(ctx.nodes_seen, 7);
}

#[test]
fn covers_bytes_and_format_strings() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    let root = sample_with_literals(&builder);

    let mut ctx = SumLiterals::default();
    root.visit(&mut ctx);
    assert_eq!(ctx.total, 7);

    let dest_arena = Bump::new();
    let mut ctx = Rebuild {
        out: ArenaBuilder::new(&dest_arena),
    };
    assert_eq!(root, root.visit(&mut ctx));
}
