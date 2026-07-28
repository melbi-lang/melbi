//! The parsed literal tree.
//!
//! See [`ParsedLiteral`](super::ParsedLiteral) for why literals get a tree of
//! their own.

use crate::{Tree, TreeBuilder};

use super::descriptor::ParsedExpr;

/// A node of the parsed literal tree.
///
/// `Eq` and `Hash` are absent by necessity: [`Float`] holds an `f64`.
///
/// [`Float`]: ParsedLiteralKind::Float
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedLiteralKind<B: TreeBuilder> {
    /// ``42``, ``0x2a``, ``0b1010``, or with a unit: ``42`m` ``, ``0o755`B/s` ``.
    ///
    /// The suffix is a whole expression, not a name: a unit may be a product,
    /// quotient or power, as in ``9.81`m/s^2` ``. The grammar admits any
    /// expression there and a later pass rejects everything outside
    /// identifiers, integers, `*`, `/` and `^`.
    ///
    /// This is the edge that makes the two trees mutually recursive. See
    /// `docs/design/units-of-measurement.md`.
    Int {
        value: i64,
        suffix: Option<Tree<B, ParsedExpr>>,
    },
    /// ``3.14``, ``1.5e-10``, ``9.81`m/s^2` `` — see [`Int`](ParsedLiteralKind::Int)
    /// for the suffix.
    Float {
        value: f64,
        suffix: Option<Tree<B, ParsedExpr>>,
    },
    /// `true`, `false`
    Bool(bool),
    /// `"hello"`
    Str(B::Str),
    /// `b"hello"`
    Bytes(B::Bytes),
}

impl<B: TreeBuilder> ParsedLiteralKind<B> {
    /// The unit suffix, if this is a numeric literal carrying one.
    pub fn suffix(&self) -> Option<&Tree<B, ParsedExpr>> {
        match self {
            ParsedLiteralKind::Int { suffix, .. } | ParsedLiteralKind::Float { suffix, .. } => {
                suffix.as_ref()
            }
            _ => None,
        }
    }
}
