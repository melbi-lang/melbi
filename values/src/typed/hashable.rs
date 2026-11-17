use core::hash::Hash;

use super::marshal::Marshal;
use super::{Array, Bool, Bytes, Float, Int, Str};
use crate::traits::ValueBuilder;

/// Types that can be map keys: every Melbi type except functions.
///
/// Typed and dynamic maps share one layout, so a typed key must compare and
/// hash exactly like the same value held in a dynamic
/// [`Value`](crate::dynamic::Value). `tests/equality.rs` checks every
/// implementation against the dynamic one. Sealed through [`Marshal`].
pub trait Hashable<B: ValueBuilder>: Marshal<B> + Eq + Hash {}

impl<B: ValueBuilder> Hashable<B> for Int {}
impl<B: ValueBuilder> Hashable<B> for Float {}
impl<B: ValueBuilder> Hashable<B> for Bool {}
impl<B: ValueBuilder> Hashable<B> for Str<B> {}
impl<B: ValueBuilder> Hashable<B> for Bytes<B> {}
impl<B: ValueBuilder, E: Hashable<B>> Hashable<B> for Array<B, E> {}
