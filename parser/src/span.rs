//! Source locations.

/// A half-open byte range `[start, end)` into the source text.
///
/// A tuple struct so that constructing one stays terse in tests and in the
/// parser, where spans are written constantly.
//
// TODO: `u32` caps a source file at 4 GiB, which is the right trade for an
// embedded expression language but should be revisited if Melbi ever grows a
// module system that concatenates sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span(pub u32, pub u32);

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self(start, end)
    }

    pub fn start(self) -> u32 {
        self.0
    }

    pub fn end(self) -> u32 {
        self.1
    }
}
