//! Tests for the folding traversal.

use bumpalo::Bump;

use super::fold::{self, Folder, Step};
use super::test_support::{Ast, ExprTree, every_tree};
use super::{Expr, ExprKind, LiteralKind};
use crate::{Tree, TreeBuilder};

/// The do-nothing folder: overrides nothing, so the driver rebuilds every node.
/// With `In = Out` this is the identity; with two different builders it is a
/// deep copy that retargets the tree.
struct Rebuild<'a, Out: TreeBuilder> {
    out: &'a Out,
}

impl<In: TreeBuilder, Out: TreeBuilder> Folder<In, Out> for Rebuild<'_, Out> {
    type Error = core::convert::Infallible;

    fn output_builder(&self) -> &Out {
        self.out
    }
}

#[test]
fn rebuilding_is_the_identity() {
    let arena = Bump::new();
    let a = Ast::new(&arena);
    let root = every_tree(&a);

    let rebuilt: ExprTree = fold::fold_expr(&root, &mut Rebuild { out: &a }).unwrap();

    // Structural equality reaches every tree, so this compares the match arms,
    // patterns, bindings, map entries and the type syntax too.
    assert_eq!(root, rebuilt);
    // ...but it really did rebuild rather than hand back the same handle.
    assert!(!core::ptr::eq(root.node(), rebuilt.node()));
}

#[test]
fn retargets_at_another_builder() {
    let source_arena = Bump::new();
    let dest_arena = Bump::new();
    let source = Ast::new(&source_arena);
    let dest = Ast::new(&dest_arena);

    let root = every_tree(&source);
    let copied: ExprTree = fold::fold_expr(&root, &mut Rebuild { out: &dest }).unwrap();

    // Equal by structure, allocated somewhere else entirely. Note that neither
    // arena's lifetime was named to get here.
    assert_eq!(root, copied);
    assert!(!core::ptr::eq(root.node(), copied.node()));
}

// --- Mode: rewrite in place, same builder ------------------------------------

/// Replaces every `Ident(from)` with a literal integer.
struct SubstituteIdent<'a, 'arena> {
    out: &'a Ast<'arena>,
    from: &'static str,
    to: i64,
    hits: usize,
}

impl<'arena> Folder<Ast<'arena>, Ast<'arena>> for SubstituteIdent<'_, 'arena> {
    type Error = core::convert::Infallible;

    fn output_builder(&self) -> &Ast<'arena> {
        self.out
    }

    fn fold_expr(
        &mut self,
        tree: &Tree<Ast<'arena>, Expr>,
    ) -> Result<Step<Ast<'arena>, Ast<'arena>, Expr>, Self::Error> {
        if let ExprKind::Ident(name) = tree.kind()
            && AsRef::<str>::as_ref(name) == self.from
        {
            self.hits += 1;
            return Ok(Step::Done(self.out.int(self.to)));
        }
        Ok(Step::Recurse)
    }
}

#[test]
fn replaces_nodes_in_place() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    // (x + x) * y
    let root = a.expr(ExprKind::Binary {
        op: super::super::BinaryOp::Mul,
        left: a.expr(ExprKind::Binary {
            op: super::super::BinaryOp::Add,
            left: a.ident("x"),
            right: a.ident("x"),
        }),
        right: a.ident("y"),
    });

    let mut folder = SubstituteIdent {
        out: &a,
        from: "x",
        to: 42,
        hits: 0,
    };
    let rewritten: ExprTree = fold::fold_expr(&root, &mut folder).unwrap();

    assert_eq!(folder.hits, 2);

    let expected = a.expr(ExprKind::Binary {
        op: super::super::BinaryOp::Mul,
        left: a.expr(ExprKind::Binary {
            op: super::super::BinaryOp::Add,
            left: a.int(42),
            right: a.int(42),
        }),
        right: a.ident("y"),
    });
    assert_eq!(rewritten, expected);
}

// --- Mode: `Done` prunes, `Replace` re-enters --------------------------------

/// Returns `Done` for the whole tree without looking at children.
struct PruneEverything<'a, 'arena> {
    out: &'a Ast<'arena>,
    calls: usize,
}

impl<'arena> Folder<Ast<'arena>, Ast<'arena>> for PruneEverything<'_, 'arena> {
    type Error = core::convert::Infallible;

    fn output_builder(&self) -> &Ast<'arena> {
        self.out
    }

    fn fold_expr(
        &mut self,
        _tree: &Tree<Ast<'arena>, Expr>,
    ) -> Result<Step<Ast<'arena>, Ast<'arena>, Expr>, Self::Error> {
        self.calls += 1;
        Ok(Step::Done(self.out.int(0)))
    }
}

#[test]
fn done_prunes_children() {
    let arena = Bump::new();
    let a = Ast::new(&arena);
    let root = every_tree(&a);

    let mut folder = PruneEverything { out: &a, calls: 0 };
    let folded: ExprTree = fold::fold_expr(&root, &mut folder).unwrap();

    // The root was pruned, so nothing below it was ever visited.
    assert_eq!(folder.calls, 1);
    assert_eq!(folded, a.int(0));
}

/// Rewrites `a` to `b`, then `b` to `c`, by re-entering with `Replace`.
struct ChainedSubstitution<'a, 'arena> {
    out: &'a Ast<'arena>,
    steps: usize,
}

impl<'arena> Folder<Ast<'arena>, Ast<'arena>> for ChainedSubstitution<'_, 'arena> {
    type Error = core::convert::Infallible;

    fn output_builder(&self) -> &Ast<'arena> {
        self.out
    }

    fn fold_expr(
        &mut self,
        tree: &Tree<Ast<'arena>, Expr>,
    ) -> Result<Step<Ast<'arena>, Ast<'arena>, Expr>, Self::Error> {
        if let ExprKind::Ident(name) = tree.kind() {
            match AsRef::<str>::as_ref(name) {
                "a" => {
                    self.steps += 1;
                    return Ok(Step::Replace(self.out.ident("b")));
                }
                "b" => {
                    self.steps += 1;
                    return Ok(Step::Replace(self.out.ident("c")));
                }
                _ => {}
            }
        }
        Ok(Step::Recurse)
    }
}

#[test]
fn replace_refolds_the_substitute() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    let mut folder = ChainedSubstitution { out: &a, steps: 0 };
    let folded: ExprTree = fold::fold_expr(&a.ident("a"), &mut folder).unwrap();

    // `a` -> `b` -> `c`: the substitute is folded again rather than accepted.
    assert_eq!(folder.steps, 2);
    assert_eq!(folded, a.ident("c"));
}

// --- The reason the driver is iterative --------------------------------------

#[test]
fn deep_nesting_does_not_overflow_the_native_stack() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    // 100_000 nested `some`, far past what a recursive traversal survives.
    let depth = 100_000;
    let mut root = a.int(1);
    for _ in 0..depth {
        root = a.expr(ExprKind::Some(root));
    }

    let rebuilt: ExprTree = fold::fold_expr(&root, &mut Rebuild { out: &a }).unwrap();

    // Deliberately *not* `assert_eq!(root, rebuilt)`: `PartialEq` on a tree is
    // recursive, so comparing this would overflow the stack even though the
    // fold did not. See the note on `Tree`'s `PartialEq` in `tree_builder.rs`.
    // Counting the depth iteratively checks the same thing without recursing.
    let mut measured = 0;
    let mut cursor = rebuilt;
    while let ExprKind::Some(inner) = cursor.kind() {
        measured += 1;
        cursor = *inner;
    }
    assert_eq!(measured, depth);
    assert!(matches!(cursor.kind(), ExprKind::Literal(_)));
}

// --- Errors abort the traversal ----------------------------------------------

struct FailOnIdent<'a, 'arena> {
    out: &'a Ast<'arena>,
}

impl<'arena> Folder<Ast<'arena>, Ast<'arena>> for FailOnIdent<'_, 'arena> {
    type Error = &'static str;

    fn output_builder(&self) -> &Ast<'arena> {
        self.out
    }

    fn fold_expr(
        &mut self,
        tree: &Tree<Ast<'arena>, Expr>,
    ) -> Result<Step<Ast<'arena>, Ast<'arena>, Expr>, Self::Error> {
        match tree.kind() {
            ExprKind::Ident(_) => Err("unresolved name"),
            _ => Ok(Step::Recurse),
        }
    }
}

#[test]
fn an_error_stops_the_fold() {
    let arena = Bump::new();
    let a = Ast::new(&arena);
    let root = a.expr(ExprKind::Some(a.ident("x")));

    let result = fold::fold_expr(&root, &mut FailOnIdent { out: &a });
    assert_eq!(result.err(), Some("unresolved name"));
}

// --- Entry points for the other trees ----------------------------------------

#[test]
fn every_tree_has_its_own_entry_point() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    // Folding can start at any descriptor, not only at an expression.
    let pattern = a.pattern(super::PatternKind::Literal(LiteralKind::Int {
        value: 5,
        suffix: None,
    }));
    assert_eq!(
        fold::fold_pattern(&pattern, &mut Rebuild { out: &a }).unwrap(),
        pattern
    );

    let binding = a.binding("k", a.int(1));
    assert_eq!(
        fold::fold_binding(&binding, &mut Rebuild { out: &a }).unwrap(),
        binding
    );

    let entry = a.entry(a.int(1), a.int(2));
    assert_eq!(
        fold::fold_map_entry(&entry, &mut Rebuild { out: &a }).unwrap(),
        entry
    );

    let arm = a.arm(a.pattern(super::PatternKind::Wildcard), a.int(3));
    assert_eq!(
        fold::fold_match_arm(&arm, &mut Rebuild { out: &a }).unwrap(),
        arm
    );

    let ty = a.ty(super::TypeExprKind::Path(a.alloc_str("Int")));
    assert_eq!(
        fold::fold_type_expr(&ty, &mut Rebuild { out: &a }).unwrap(),
        ty
    );

    let field = a.field("f", ty);
    assert_eq!(
        fold::fold_type_field(&field, &mut Rebuild { out: &a }).unwrap(),
        field
    );
}
