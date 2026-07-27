//! Builds real Melbi expressions with [`ExprKind`], to check that every variant
//! is constructible through the builder and reachable from a traversal.

use bumpalo::Bump;

use crate::ast::{BinaryOp, BoolOp, ComparisonOp, ExprKind, UnaryOp};
use crate::test_utils::Span;
use crate::{Tree, TreeBuilder, TreeNode, Visit};

#[derive(Clone, Copy)]
struct Ast<'arena> {
    arena: &'arena Bump,
}

// `Bump` implements none of `Debug`, `PartialEq` or `Hash`, so these cannot be
// derived. A builder's identity is the arena it allocates from.
impl core::fmt::Debug for Ast<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Ast")
    }
}

impl PartialEq for Ast<'_> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.arena, other.arena)
    }
}

impl Eq for Ast<'_> {}

impl core::hash::Hash for Ast<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::ptr::hash(self.arena, state);
    }
}

impl<'arena> TreeBuilder for Ast<'arena> {
    type TreeData = Span;
    type TreeKind = ExprKind<Self>;
    type TreeHandle = &'arena TreeNode<Self>;
    type Str = &'arena str;
    type List = &'arena [Tree<Self>];
    type StrList = &'arena [&'arena str];
    type Bytes = &'arena [u8];

    fn alloc(&self, node: TreeNode<Self>) -> Self::TreeHandle {
        self.arena.alloc(node)
    }

    fn alloc_str(&self, s: &str) -> Self::Str {
        self.arena.alloc_str(s)
    }

    fn alloc_list(
        &self,
        items: impl IntoIterator<Item = Tree<Self>, IntoIter: ExactSizeIterator>,
    ) -> Self::List {
        self.arena.alloc_slice_fill_iter(items)
    }

    fn alloc_str_list(
        &self,
        items: impl IntoIterator<Item = Self::Str, IntoIter: ExactSizeIterator>,
    ) -> Self::StrList {
        self.arena.alloc_slice_fill_iter(items)
    }

    fn alloc_bytes(&self, bytes: &[u8]) -> Self::Bytes {
        self.arena.alloc_slice_copy(bytes)
    }
}

/// Spans are irrelevant to these tests, but the builder still demands one —
/// which is the point of requiring the data explicitly.
const S: Span = Span(0, 0);

fn node<'a>(builder: &Ast<'a>, kind: ExprKind<Ast<'a>>) -> Tree<Ast<'a>> {
    TreeNode::new(S, kind).alloc(builder)
}

// --- A traversal that reaches every child ------------------------------------

#[derive(Default)]
struct Census {
    nodes: usize,
    idents: usize,
    ints: i64,
}

impl<'a> Visit<Ast<'a>, Census> for TreeNode<Ast<'a>> {
    type Output = ();

    fn visit(&self, ctx: &mut Census) {
        ctx.nodes += 1;
        match self.kind() {
            // A unit suffix is an ordinary subtree and has to be traversed like
            // any other child.
            ExprKind::Int { value, suffix } => {
                ctx.ints += value;
                if let Some(suffix) = suffix {
                    suffix.visit(ctx);
                }
            }
            ExprKind::Float { suffix, .. } => {
                if let Some(suffix) = suffix {
                    suffix.visit(ctx);
                }
            }
            ExprKind::Ident(_) => ctx.idents += 1,
            ExprKind::Bool(_) | ExprKind::Str(_) | ExprKind::Bytes(_) | ExprKind::None => {}

            ExprKind::Unary { expr, .. } | ExprKind::Some(expr) | ExprKind::Cast { expr } => {
                expr.visit(ctx)
            }
            ExprKind::Field { value, .. } => value.visit(ctx),
            ExprKind::Lambda { body, .. } => body.visit(ctx),

            ExprKind::Binary { left, right, .. }
            | ExprKind::Boolean { left, right, .. }
            | ExprKind::Comparison { left, right, .. } => {
                left.visit(ctx);
                right.visit(ctx);
            }
            ExprKind::Index { value, index } => {
                value.visit(ctx);
                index.visit(ctx);
            }
            ExprKind::Otherwise { primary, fallback } => {
                primary.visit(ctx);
                fallback.visit(ctx);
            }

            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                cond.visit(ctx);
                then_branch.visit(ctx);
                else_branch.visit(ctx);
            }

            ExprKind::Call { callable, args } => {
                callable.visit(ctx);
                for arg in args.iter() {
                    arg.visit(ctx);
                }
            }
            ExprKind::Array(items) | ExprKind::FormatStr { exprs: items, .. } => {
                for item in items.iter() {
                    item.visit(ctx);
                }
            }
        }
    }
}

// --- Tests -------------------------------------------------------------------

#[test]
fn builds_a_conditional() {
    let arena = Bump::new();
    let b = Ast { arena: &arena };

    // if n > 0 then some [n, 2] else none
    let n = node(&b, ExprKind::Ident(b.alloc_str("n")));
    let zero = node(
        &b,
        ExprKind::Int {
            value: 0,
            suffix: None,
        },
    );
    let two = node(
        &b,
        ExprKind::Int {
            value: 2,
            suffix: None,
        },
    );
    let cond = node(
        &b,
        ExprKind::Comparison {
            op: ComparisonOp::Gt,
            left: n,
            right: zero,
        },
    );
    let array = node(&b, ExprKind::Array(b.alloc_list([n, two])));
    let root = node(
        &b,
        ExprKind::If {
            cond,
            then_branch: node(&b, ExprKind::Some(array)),
            else_branch: node(&b, ExprKind::None),
        },
    );

    let mut census = Census::default();
    root.visit(&mut census);

    // if, cond, n, 0, some, array, n, 2, none
    assert_eq!(census.nodes, 9);
    assert_eq!(census.idents, 2);
    assert_eq!(census.ints, 2);
}

#[test]
fn every_variant_is_constructible() {
    let arena = Bump::new();
    let b = Ast { arena: &arena };

    let x = node(&b, ExprKind::Ident(b.alloc_str("x")));

    // 9.81`m/s^2` — the suffix is a real expression tree.
    let metres = node(&b, ExprKind::Ident(b.alloc_str("m")));
    let seconds = node(&b, ExprKind::Ident(b.alloc_str("s")));
    let two = node(
        &b,
        ExprKind::Int {
            value: 2,
            suffix: None,
        },
    );
    let per_second_squared = node(
        &b,
        ExprKind::Binary {
            op: BinaryOp::Div,
            left: metres,
            right: node(
                &b,
                ExprKind::Binary {
                    op: BinaryOp::Pow,
                    left: seconds,
                    right: two,
                },
            ),
        },
    );

    let one = node(
        &b,
        ExprKind::Int {
            value: 1,
            suffix: Some(per_second_squared),
        },
    );

    let variants = [
        one,
        node(
            &b,
            ExprKind::Float {
                value: 1.5,
                suffix: None,
            },
        ),
        node(&b, ExprKind::Bool(true)),
        node(&b, ExprKind::Str(b.alloc_str("hello"))),
        node(&b, ExprKind::Bytes(b.alloc_bytes(b"hi"))),
        node(
            &b,
            ExprKind::FormatStr {
                strs: b.alloc_str_list([b.alloc_str("n = "), b.alloc_str("!")]),
                exprs: b.alloc_list([x]),
            },
        ),
        x,
        node(
            &b,
            ExprKind::Binary {
                op: BinaryOp::Pow,
                left: x,
                right: one,
            },
        ),
        node(
            &b,
            ExprKind::Boolean {
                op: BoolOp::And,
                left: x,
                right: x,
            },
        ),
        node(
            &b,
            ExprKind::Comparison {
                op: ComparisonOp::NotIn,
                left: x,
                right: x,
            },
        ),
        node(
            &b,
            ExprKind::Unary {
                op: UnaryOp::Not,
                expr: x,
            },
        ),
        node(
            &b,
            ExprKind::Call {
                callable: x,
                args: b.alloc_list([one]),
            },
        ),
        node(
            &b,
            ExprKind::Index {
                value: x,
                index: one,
            },
        ),
        node(
            &b,
            ExprKind::Field {
                value: x,
                field: b.alloc_str("field"),
            },
        ),
        node(&b, ExprKind::Cast { expr: x }),
        node(
            &b,
            ExprKind::If {
                cond: x,
                then_branch: one,
                else_branch: one,
            },
        ),
        node(
            &b,
            ExprKind::Otherwise {
                primary: x,
                fallback: one,
            },
        ),
        node(&b, ExprKind::Some(x)),
        node(&b, ExprKind::None),
        node(&b, ExprKind::Array(b.alloc_list([x, one]))),
        node(
            &b,
            ExprKind::Lambda {
                params: b.alloc_str_list([b.alloc_str("a"), b.alloc_str("b")]),
                body: x,
            },
        ),
    ];

    // Every variant of `ExprKind` is built above. Adding a variant breaks the
    // exhaustive match in `Census`, and this count is the reminder to cover the
    // new variant here as well.
    assert_eq!(variants.len(), 21);

    let mut census = Census::default();
    for variant in variants {
        variant.visit(&mut census);
    }
    assert!(census.nodes > variants.len());
}

#[test]
fn unit_suffixes_are_traversed_as_subtrees() {
    let arena = Bump::new();
    let b = Ast { arena: &arena };

    // 100`seconds`
    let simple = node(
        &b,
        ExprKind::Int {
            value: 100,
            suffix: Some(node(&b, ExprKind::Ident(b.alloc_str("seconds")))),
        },
    );

    let mut census = Census::default();
    simple.visit(&mut census);
    assert_eq!(census.nodes, 2);
    assert_eq!(census.idents, 1);

    // 0b1010`K` and a bare 42 differ only by the suffix, so they must not
    // compare equal.
    let ten_kelvin = node(
        &b,
        ExprKind::Int {
            value: 10,
            suffix: Some(node(&b, ExprKind::Ident(b.alloc_str("K")))),
        },
    );
    let ten = node(
        &b,
        ExprKind::Int {
            value: 10,
            suffix: None,
        },
    );
    assert_ne!(ten_kelvin, ten);
}

#[test]
fn trees_compare_structurally_across_arenas() {
    fn build<'a>(b: &Ast<'a>) -> Tree<Ast<'a>> {
        let x = node(b, ExprKind::Ident(b.alloc_str("x")));
        let one = node(
            b,
            ExprKind::Int {
                value: 1,
                suffix: None,
            },
        );
        node(
            b,
            ExprKind::Binary {
                op: BinaryOp::Add,
                left: x,
                right: one,
            },
        )
    }

    let left_arena = Bump::new();
    let right_arena = Bump::new();

    // The same expression in two unrelated arenas: equal by structure, distinct
    // in memory.
    assert_eq!(
        build(&Ast { arena: &left_arena }),
        build(&Ast {
            arena: &right_arena
        })
    );
}
