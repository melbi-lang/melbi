use serde::Serialize;

use crate::parser::syntax::AnnotatedSource;
use crate::parser::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};

#[derive(Debug)]
pub struct ParsedExpr<'a> {
    pub expr: &'a Expr<'a>,
    pub ann: &'a AnnotatedSource<'a, Expr<'a>>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum Expr<'a> {
    Binary {
        op: BinaryOp,
        left: &'a Expr<'a>,
        right: &'a Expr<'a>,
    },
    Boolean {
        op: BoolOp,
        left: &'a Expr<'a>,
        right: &'a Expr<'a>,
    },
    Comparison {
        op: ComparisonOp,
        left: &'a Expr<'a>,
        right: &'a Expr<'a>,
    },
    Unary {
        op: UnaryOp,
        expr: &'a Expr<'a>,
    },
    Call {
        callable: &'a Expr<'a>,
        args: &'a [&'a Expr<'a>],
    },
    Index {
        value: &'a Expr<'a>,
        index: &'a Expr<'a>,
    },
    Field {
        value: &'a Expr<'a>,
        field: &'a str,
    },
    Cast {
        ty: TypeExpr<'a>,
        expr: &'a Expr<'a>,
    },
    Lambda {
        params: &'a [&'a str],
        body: &'a Expr<'a>,
    },
    If {
        cond: &'a Expr<'a>,
        then_branch: &'a Expr<'a>,
        else_branch: &'a Expr<'a>,
    },
    Where {
        expr: &'a Expr<'a>,
        bindings: &'a [(&'a str, &'a Expr<'a>)],
    },
    Otherwise {
        primary: &'a Expr<'a>,
        fallback: &'a Expr<'a>,
    },
    /// Option constructor: `some expr` or `none`
    /// Inner is Some(expr) for `some expr`, None for `none`
    Option {
        inner: Option<&'a Expr<'a>>,
    },
    /// Pattern matching: `expr match { pattern -> body, ... }`
    Match {
        expr: &'a Expr<'a>,
        arms: &'a [MatchArm<'a>],
    },
    Record(&'a [(&'a str, &'a Expr<'a>)]),
    Map(&'a [(&'a Expr<'a>, &'a Expr<'a>)]),
    Array(&'a [&'a Expr<'a>]),
    FormatStr {
        // REQUIRES: strs.len() == exprs.len() + 1
        strs: &'a [&'a str],
        exprs: &'a [&'a Expr<'a>],
    },
    Literal(Literal<'a>),
    Ident(&'a str),
}

impl<'a> Expr<'a> {
    pub fn as_ptr(&self) -> *const Self {
        self as *const _
    }
}

#[derive(Clone, PartialEq, Serialize)]
pub enum Literal<'a> {
    Int {
        value: i64,
        suffix: Option<&'a Expr<'a>>,
    },
    Float {
        value: f64,
        suffix: Option<&'a Expr<'a>>,
    },
    Bool(bool),
    Str(&'a str),
    Bytes(&'a [u8]),
}

impl<'a> core::fmt::Debug for Literal<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Literal::Int {
                value,
                suffix: None,
            } => write!(f, "Int({value})"),
            Literal::Int {
                value,
                suffix: Some(s),
            } => write!(f, "Int({value}, suffix: {s:?})"),
            Literal::Float {
                value,
                suffix: None,
            } => write!(f, "Float({value})"),
            Literal::Float {
                value,
                suffix: Some(s),
            } => write!(f, "Float({value}, suffix: {s:?})"),
            Literal::Bool(b) => write!(f, "Bool({b})"),
            Literal::Str(s) => write!(f, "Str({s:?})"),
            Literal::Bytes(bytes) => write!(f, "Bytes({bytes:?})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum TypeExpr<'a> {
    Path(&'a str),
    Parametrized {
        path: &'a str,
        params: &'a [TypeExpr<'a>],
    },
    Record(&'a [(&'a str, TypeExpr<'a>)]),
}

/// A single arm in a match expression.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MatchArm<'a> {
    pub pattern: &'a Pattern<'a>,
    pub body: &'a Expr<'a>,
}

/// Patterns for destructuring and matching values.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Pattern<'a> {
    /// Wildcard pattern `_` - matches anything
    Wildcard,
    /// Variable pattern `x` - binds the value to a name
    Var(&'a str),
    /// Literal pattern - matches specific literal values
    Literal(Literal<'a>),
    /// Some pattern `some p` - matches Option::Some and destructures inner value
    Some(&'a Pattern<'a>),
    /// None pattern `none` - matches Option::None
    None,
}
