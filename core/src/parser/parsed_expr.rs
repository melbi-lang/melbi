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
        left: &'a Self,
        right: &'a Self,
    },
    Boolean {
        op: BoolOp,
        left: &'a Self,
        right: &'a Self,
    },
    Comparison {
        op: ComparisonOp,
        left: &'a Self,
        right: &'a Self,
    },
    Unary {
        op: UnaryOp,
        expr: &'a Self,
    },
    Call {
        callable: &'a Self,
        args: &'a [&'a Self],
    },
    Index {
        value: &'a Self,
        index: &'a Self,
    },
    Field {
        value: &'a Self,
        field: &'a str,
    },
    Cast {
        ty: TypeExpr<'a>,
        expr: &'a Self,
    },
    Lambda {
        params: &'a [&'a str],
        body: &'a Self,
    },
    If {
        cond: &'a Self,
        then_branch: &'a Self,
        else_branch: &'a Self,
    },
    Where {
        expr: &'a Self,
        bindings: &'a [(&'a str, &'a Self)],
    },
    Otherwise {
        primary: &'a Self,
        fallback: &'a Self,
    },
    /// Option constructor: `some expr` or `none`
    /// Inner is Some(expr) for `some expr`, None for `none`
    Option {
        inner: Option<&'a Self>,
    },
    /// Pattern matching: `expr match { pattern -> body, ... }`
    Match {
        expr: &'a Self,
        arms: &'a [MatchArm<'a>],
    },
    Record(&'a [(&'a str, &'a Self)]),
    Map(&'a [(&'a Self, &'a Self)]),
    Array(&'a [&'a Self]),
    FormatStr {
        // REQUIRES: strs.len() == exprs.len() + 1
        strs: &'a [&'a str],
        exprs: &'a [&'a Self],
    },
    Literal(Literal<'a>),
    Ident(&'a str),
}

impl Expr<'_> {
    #[must_use]
    pub fn as_ptr(&self) -> *const Self {
        core::ptr::from_ref(self)
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

impl core::fmt::Debug for Literal<'_> {
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
        params: &'a [Self],
    },
    Record(&'a [(&'a str, Self)]),
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
    /// Some pattern `some p` - matches `Option::Some` and destructures inner value
    Some(&'a Self),
    /// None pattern `none` - matches `Option::None`
    None,
}
