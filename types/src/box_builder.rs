use crate::traits::{Ty, TyBuilder, TyNode};
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::{fmt::Debug, hash::Hash};

/// Interner that uses reference counting (no deduplication).
///
/// Types are allocated with `Rc` and no interning is performed.
/// This is useful for:
/// - Testing (simpler than arena)
/// - Situations where deduplication isn't needed
/// - Comparing performance with/without interning
///
/// Following Chalk's design, we compute type flags during interning and
/// wrap the TyKind in TyData.
///
/// # Example
///
/// ```ignore
/// let builder = BoxBuilder::new();
/// let int_ty = TyKind::Scalar(Scalar::Int).alloc(builder);
/// let arr_ty = TyKind::Array(int_ty).alloc(builder);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoxBuilder;

impl BoxBuilder {
    /// Create a new box builder.
    pub fn new() -> Self {
        Self
    }
}

impl TyBuilder for BoxBuilder {
    type Ty = self::Ty<Self>;
    type TyHandle = Rc<TyNode<Self>>;
    type Str = Rc<str>;
    type List<T>
        = Vec<T>
    where
        T: Debug + PartialEq + Eq + Clone + Hash;

    fn alloc(&self, node: TyNode<Self>) -> Self::TyHandle {
        Rc::new(node)
    }

    fn alloc_str(&self, s: impl AsRef<str>) -> Self::Str {
        Rc::from(s.as_ref())
    }

    fn alloc_list<T>(&self, iter: impl IntoIterator<Item = T>) -> Self::List<T>
    where
        T: Debug + PartialEq + Eq + Clone + Hash,
    {
        iter.into_iter().collect()
    }
}
