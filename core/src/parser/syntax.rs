// These are common syntax structures used in ParsedExpr and TypedExpr.

use core::cell::RefCell;
use core::ops::Range;

use bumpalo::Bump;
use hashbrown::{DefaultHashBuilder, HashMap};
use serde::Serialize;

#[derive(Debug)]
pub struct AnnotatedSource<'a, T> {
    pub source: &'a str,
    spans: RefCell<HashMap<*const T, Span, DefaultHashBuilder, &'a Bump>>,
}

impl<'a, T> AnnotatedSource<'a, T> {
    pub fn new(arena: &'a Bump, source: &'a str) -> Self {
        Self {
            source,
            spans: RefCell::new(HashMap::new_in(arena)),
        }
    }
    pub fn add_span(&self, expr: &T, span: Span) {
        let p = core::ptr::from_ref(expr);
        self.spans.borrow_mut().insert(p, span);
    }
    pub fn span_of(&self, expr: &T) -> Option<Span> {
        let p = core::ptr::from_ref(expr);
        self.spans.borrow().get(&p).cloned()
    }
    pub fn snippet(&self, span: Span) -> &str {
        &self.source[span.0]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span(pub Range<usize>);

impl Span {
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Self(start..end)
    }
    #[must_use]
    pub fn combine(a: &Self, b: &Self) -> Self {
        Self::new(a.0.start, b.0.end)
    }
    #[must_use]
    pub fn str_of<'a>(&self, source: &'a str) -> &'a str {
        &source[self.0.start..self.0.end]
    }
}

impl From<pest::Span<'_>> for Span {
    fn from(s: pest::Span<'_>) -> Self {
        Self(s.start()..s.end())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
#[repr(u8)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
#[repr(u8)]
pub enum BoolOp {
    And,
    Or,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
#[repr(u8)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
#[repr(u8)]
pub enum ComparisonOp {
    Eq = 1,    // ==
    Neq = 2,   // !=
    Lt = 3,    // <
    Gt = 4,    // >
    Le = 5,    // <=
    Ge = 6,    // >=
    In = 7,    // in
    NotIn = 8, // not in
}
