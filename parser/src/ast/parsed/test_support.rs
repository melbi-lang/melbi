//! An arena builder and construction helpers, shared by the parsed-AST tests.

use bumpalo::Bump;

use crate::ast::parsed::{
    self, BindingKind, Data, ExprKind, LiteralKind, MapEntryKind, MatchArmKind, PatternKind,
    TypeExprKind, TypeFieldKind,
};
use crate::{Span, Tree, TreeBuilder, TreeDescriptor, TreeNode};

/// One builder hosting every parsed tree.
#[derive(Clone, Copy)]
pub struct Ast<'arena> {
    arena: &'arena Bump,
}

impl<'arena> Ast<'arena> {
    pub fn new(arena: &'arena Bump) -> Self {
        Self { arena }
    }
}

/// `Bump` implements none of `Debug`, `PartialEq` or `Hash`, so a builder
/// holding one cannot derive them. Identity is the arena it allocates from.
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
    type Handle<D: TreeDescriptor> = &'arena TreeNode<Self, D>;
    type List<D: TreeDescriptor> = &'arena [Tree<Self, D>];
    type Str = &'arena str;
    type StrList = &'arena [&'arena str];
    type Bytes = &'arena [u8];

    fn alloc<D: TreeDescriptor>(&self, node: TreeNode<Self, D>) -> Self::Handle<D> {
        self.arena.alloc(node)
    }
    fn alloc_list<D: TreeDescriptor>(
        &self,
        items: impl IntoIterator<Item = Tree<Self, D>, IntoIter: ExactSizeIterator>,
    ) -> Self::List<D> {
        self.arena.alloc_slice_fill_iter(items)
    }
    fn alloc_str(&self, s: &str) -> Self::Str {
        self.arena.alloc_str(s)
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

pub type ExprTree<'arena> = Tree<Ast<'arena>, parsed::Expr>;
pub type PatternTree<'arena> = Tree<Ast<'arena>, parsed::Pattern>;
pub type TypeExprTree<'arena> = Tree<Ast<'arena>, parsed::TypeExpr>;

impl<'arena> Ast<'arena> {
    /// Every parsed descriptor shares [`Data`], so one helper allocates into any
    /// of the seven trees. The descriptor comes from the return type, since
    /// `D::Kind<B>` is a projection and cannot determine `D` on its own.
    ///
    /// Spans are irrelevant to these tests, but a node still demands data —
    /// which is the point of requiring it explicitly.
    pub fn node<D>(&self, kind: D::Kind<Self>) -> Tree<Self, D>
    where
        D: TreeDescriptor<Data = Data>,
    {
        TreeNode::new(Data::new(Span(0, 0)), kind).alloc(self)
    }

    pub fn expr(&self, kind: ExprKind<Self>) -> ExprTree<'arena> {
        self.node(kind)
    }

    pub fn pattern(&self, kind: PatternKind<Self>) -> PatternTree<'arena> {
        self.node(kind)
    }

    pub fn ty(&self, kind: TypeExprKind<Self>) -> TypeExprTree<'arena> {
        self.node(kind)
    }

    /// A literal, in the expression node that holds it. One node, not two: the
    /// literal is inline.
    pub fn lit(&self, kind: LiteralKind<Self>) -> ExprTree<'arena> {
        self.expr(ExprKind::Literal(kind))
    }

    pub fn int(&self, value: i64) -> ExprTree<'arena> {
        self.lit(LiteralKind::Int {
            value,
            suffix: None,
        })
    }

    pub fn ident(&self, name: &str) -> ExprTree<'arena> {
        self.expr(ExprKind::Ident(self.alloc_str(name)))
    }

    pub fn arm(
        &self,
        pattern: PatternTree<'arena>,
        body: ExprTree<'arena>,
    ) -> Tree<Self, parsed::MatchArm> {
        self.node(MatchArmKind { pattern, body })
    }

    pub fn binding(&self, name: &str, value: ExprTree<'arena>) -> Tree<Self, parsed::Binding> {
        self.node(BindingKind {
            name: self.alloc_str(name),
            value,
        })
    }

    pub fn entry(
        &self,
        key: ExprTree<'arena>,
        value: ExprTree<'arena>,
    ) -> Tree<Self, parsed::MapEntry> {
        self.node(MapEntryKind { key, value })
    }

    pub fn field(&self, name: &str, ty: TypeExprTree<'arena>) -> Tree<Self, parsed::TypeField> {
        self.node(TypeFieldKind {
            name: self.alloc_str(name),
            ty,
        })
    }
}

/// An expression touching every tree: `(x as Record[a: Int]) match { … }` with a
/// `where`, a record, a map and a suffixed literal inside.
///
/// Used to check that a traversal reaches all seven trees, not just the two easy
/// ones.
pub fn every_tree<'a>(a: &Ast<'a>) -> ExprTree<'a> {
    let cast = a.expr(ExprKind::Cast {
        expr: a.ident("x"),
        ty: a.ty(TypeExprKind::Record(a.alloc_list([a.field(
            "a",
            a.ty(TypeExprKind::Parametrized {
                path: a.alloc_str("Array"),
                params: a.alloc_list([a.ty(TypeExprKind::Path(a.alloc_str("Int")))]),
            }),
        )]))),
    });

    let body = a.expr(ExprKind::Where {
        expr: a.expr(ExprKind::Record(
            a.alloc_list([a.binding("r", a.ident("y"))]),
        )),
        bindings: a.alloc_list([a.binding(
            "y",
            a.expr(ExprKind::Map(a.alloc_list([a.entry(
                a.int(1),
                // A suffixed literal: the literal reaches back into expressions.
                a.lit(LiteralKind::Float {
                    value: 9.81,
                    suffix: Some(a.ident("m")),
                }),
            )]))),
        )]),
    });

    a.expr(ExprKind::Match {
        scrutinee: cast,
        arms: a.alloc_list([
            a.arm(
                a.pattern(PatternKind::Some(a.pattern(PatternKind::Literal(
                    LiteralKind::Int {
                        value: 7,
                        suffix: None,
                    },
                )))),
                body,
            ),
            a.arm(a.pattern(PatternKind::Wildcard), a.int(0)),
        ]),
    })
}
