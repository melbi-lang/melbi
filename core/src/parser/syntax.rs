// Source annotations shared by the typed tree and diagnostics.

use core::cell::RefCell;
use core::ops::Range;

use bumpalo::Bump;
use hashbrown::{DefaultHashBuilder, HashMap};

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

impl From<melbi_parser::Span> for Span {
    fn from(span: melbi_parser::Span) -> Self {
        Self(span.start() as usize..span.end() as usize)
    }
}
