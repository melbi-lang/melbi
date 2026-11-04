//! A compact, span-free text form of a parsed tree, for tests and debugging.
//!
//! `Sexpr(&tree).to_string()` prints the tree as an S-expression, so two trees
//! can be compared by shape with a readable diff:
//!
//! ```text
//! 1 + f(x)  =>  (Binary Add (Lit Int 1) (Call (Ident "f") (Ident "x")))
//! ```
//!
//! Spans are left out on purpose: they are tested separately, and including
//! them would make every expectation depend on whitespace.
//!
//! Printing recurses once per level of nesting, like the derived `Debug`, so
//! it is not meant for trees near the parser's depth limit.

use core::fmt::{self, Display, Formatter, Write as _};

use super::{Expr, ExprKind, LiteralKind, Pattern, PatternKind, TypeExpr, TypeExprKind};
use crate::{Tree, TreeBuilder};

/// Displays a parsed expression as a span-free S-expression.
pub struct Sexpr<'a, B: TreeBuilder>(pub &'a Tree<B, Expr>);

impl<B: TreeBuilder> Display for Sexpr<'_, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write_expr(self.0, f)
    }
}

fn write_expr<B: TreeBuilder>(tree: &Tree<B, Expr>, f: &mut Formatter<'_>) -> fmt::Result {
    use ExprKind::*;
    match tree.kind() {
        Binary { op, left, right } => write_node(f, format_args!("Binary {op:?}"), &[left, right]),
        Boolean { op, left, right } => {
            write_node(f, format_args!("Boolean {op:?}"), &[left, right])
        }
        Comparison { op, left, right } => {
            write_node(f, format_args!("Comparison {op:?}"), &[left, right])
        }
        Unary { op, expr } => write_node(f, format_args!("Unary {op:?}"), &[expr]),
        Call { callable, args } => {
            f.write_str("(Call ")?;
            write_expr(callable, f)?;
            for arg in args.iter() {
                f.write_char(' ')?;
                write_expr(arg, f)?;
            }
            f.write_char(')')
        }
        Index { value, index } => write_node(f, format_args!("Index"), &[value, index]),
        Field { value, field } => {
            write_node(f, format_args!("Field {:?}", field.as_ref()), &[value])
        }
        Cast { expr, ty } => {
            f.write_str("(Cast ")?;
            write_expr(expr, f)?;
            f.write_char(' ')?;
            write_type(ty, f)?;
            f.write_char(')')
        }
        Lambda { params, body } => {
            f.write_str("(Lambda ")?;
            write_str_list(params, f)?;
            f.write_char(' ')?;
            write_expr(body, f)?;
            f.write_char(')')
        }
        If {
            cond,
            then_branch,
            else_branch,
        } => write_node(f, format_args!("If"), &[cond, then_branch, else_branch]),
        Where { expr, bindings } => {
            f.write_str("(Where ")?;
            write_expr(expr, f)?;
            for binding in bindings.iter() {
                write!(f, " (bind {:?} ", binding.kind().name.as_ref())?;
                write_expr(&binding.kind().value, f)?;
                f.write_char(')')?;
            }
            f.write_char(')')
        }
        Otherwise { primary, fallback } => {
            write_node(f, format_args!("Otherwise"), &[primary, fallback])
        }
        Some(inner) => write_node(f, format_args!("Some"), &[inner]),
        None => f.write_str("(None)"),
        Match { scrutinee, arms } => {
            f.write_str("(Match ")?;
            write_expr(scrutinee, f)?;
            for arm in arms.iter() {
                f.write_str(" (arm ")?;
                write_pattern(&arm.kind().pattern, f)?;
                f.write_char(' ')?;
                write_expr(&arm.kind().body, f)?;
                f.write_char(')')?;
            }
            f.write_char(')')
        }
        Record(fields) => {
            f.write_str("(Record")?;
            for field in fields.iter() {
                write!(f, " (bind {:?} ", field.kind().name.as_ref())?;
                write_expr(&field.kind().value, f)?;
                f.write_char(')')?;
            }
            f.write_char(')')
        }
        Map(entries) => {
            f.write_str("(Map")?;
            for entry in entries.iter() {
                f.write_str(" (entry ")?;
                write_expr(&entry.kind().key, f)?;
                f.write_char(' ')?;
                write_expr(&entry.kind().value, f)?;
                f.write_char(')')?;
            }
            f.write_char(')')
        }
        Array(items) => {
            f.write_str("(Array")?;
            for item in items.iter() {
                f.write_char(' ')?;
                write_expr(item, f)?;
            }
            f.write_char(')')
        }
        FormatStr { strs, exprs } => {
            f.write_str("(FormatStr ")?;
            write_str_list(strs, f)?;
            for expr in exprs.iter() {
                f.write_char(' ')?;
                write_expr(expr, f)?;
            }
            f.write_char(')')
        }
        Literal(literal) => {
            f.write_str("(Lit ")?;
            write_literal(literal, f)?;
            f.write_char(')')
        }
        Ident(name) => write!(f, "(Ident {:?})", name.as_ref()),
    }
}

/// Write `(label child child ...)`.
fn write_node<B: TreeBuilder>(
    f: &mut Formatter<'_>,
    label: fmt::Arguments<'_>,
    children: &[&Tree<B, Expr>],
) -> fmt::Result {
    write!(f, "({label}")?;
    for child in children {
        f.write_char(' ')?;
        write_expr(child, f)?;
    }
    f.write_char(')')
}

/// Write a list of names as `["a", "b"]`.
fn write_str_list<S: AsRef<str>>(names: &[S], f: &mut Formatter<'_>) -> fmt::Result {
    f.debug_list()
        .entries(names.iter().map(AsRef::<str>::as_ref))
        .finish()
}

fn write_literal<B: TreeBuilder>(literal: &LiteralKind<B>, f: &mut Formatter<'_>) -> fmt::Result {
    match literal {
        LiteralKind::Int { value, suffix } => {
            write!(f, "Int {value}")?;
            if let Some(suffix) = suffix {
                f.write_str(" suffix=")?;
                write_expr(suffix, f)?;
            }
            Ok(())
        }
        LiteralKind::Float { value, suffix } => {
            write!(f, "Float {value:?}")?;
            if let Some(suffix) = suffix {
                f.write_str(" suffix=")?;
                write_expr(suffix, f)?;
            }
            Ok(())
        }
        LiteralKind::Bool(value) => write!(f, "Bool {value}"),
        LiteralKind::Str(value) => write!(f, "Str {:?}", value.as_ref()),
        LiteralKind::Bytes(value) => write!(f, "Bytes {:?}", &**value),
    }
}

fn write_pattern<B: TreeBuilder>(pattern: &Tree<B, Pattern>, f: &mut Formatter<'_>) -> fmt::Result {
    match pattern.kind() {
        PatternKind::Wildcard => f.write_char('_'),
        PatternKind::Binding(name) => write!(f, "(PVar {:?})", name.as_ref()),
        PatternKind::Literal(literal) => {
            f.write_str("(PLit ")?;
            write_literal(literal, f)?;
            f.write_char(')')
        }
        PatternKind::Some(inner) => {
            f.write_str("(PSome ")?;
            write_pattern(inner, f)?;
            f.write_char(')')
        }
        PatternKind::None => f.write_str("PNone"),
    }
}

fn write_type<B: TreeBuilder>(ty: &Tree<B, TypeExpr>, f: &mut Formatter<'_>) -> fmt::Result {
    match ty.kind() {
        TypeExprKind::Path(path) => write!(f, "(TPath {:?})", path.as_ref()),
        TypeExprKind::Parametrized { path, params } => {
            write!(f, "(TParam {:?}", path.as_ref())?;
            for param in params.iter() {
                f.write_char(' ')?;
                write_type(param, f)?;
            }
            f.write_char(')')
        }
        TypeExprKind::Record(fields) => {
            f.write_str("(TRecord")?;
            for field in fields.iter() {
                write!(f, " ({:?} ", field.kind().name.as_ref())?;
                write_type(&field.kind().ty, f)?;
                f.write_char(')')?;
            }
            f.write_char(')')
        }
    }
}
