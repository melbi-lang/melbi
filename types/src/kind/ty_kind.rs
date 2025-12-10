use crate::traits::{FieldList, IdentList, Ty, TyBuilder, TyFlags, TyList, TyNode};
use core::{fmt::Debug, hash::Hash};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyKind<B: TyBuilder> {
    /// Type variable for unification (Hindley-Milner style).
    ///
    /// Unlike Chalk's InferenceVar/Placeholder, Melbi uses simple
    /// numeric IDs for unification variables.
    TypeVar(u16),

    /// Scalar types (Bool, Int, Float, Str, Bytes)
    Scalar(Scalar),

    /// Array type with element type
    Array(Ty<B>),

    /// Map type with key and value types
    Map(Ty<B>, Ty<B>),

    /// Record (struct) with named fields.
    ///
    /// Fields are stored sorted by name for canonical representation.
    /// Field names are interned strings for efficient comparison.
    Record(FieldList<B>),

    /// Function type with parameters and return type.
    ///
    /// Parameters are stored as an interned list of types.
    Function { params: TyList<B>, ret: Ty<B> },

    /// Symbol (tagged union) with sorted parts.
    ///
    /// Parts are interned strings stored in sorted order.
    /// Example: Symbol["error", "pending", "success"]
    Symbol(IdentList<B>),
}

impl<B: TyBuilder> TyKind<B> {
    pub fn compute_flags(&self) -> TyFlags {
        TyFlags::empty() // TODO: Implement this.
    }

    pub fn alloc(self, builder: &B) -> Ty<B> {
        Ty::new(builder, TyNode::new(self))
    }
}

/// Scalar type variants
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scalar {
    /// Boolean type
    Bool,

    /// Integer type
    Int,

    /// Floating-point type
    Float,

    /// String type
    Str,

    /// Bytes type
    Bytes,
}

#[cfg(feature = "experimental")]
mod tests {
    use super::*;

    #[derive(Copy, Clone, Debug)]
    struct TestBuilder<'a> {
        arena: &'a bumpalo::Bump,
    }

    impl<'arena> PartialEq for TestBuilder<'arena> {
        fn eq(&self, other: &Self) -> bool {
            core::ptr::eq(self.arena, other.arena)
        }
    }

    impl<'arena> Eq for TestBuilder<'arena> {}

    impl<'arena> core::hash::Hash for TestBuilder<'arena> {
        fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
            core::ptr::hash(self.arena, state)
        }
    }

    impl<'a> TyBuilder for TestBuilder<'a> {
        type Ty = (); // Ty<Self>;
        type TyHandle = &'a TyKind<Self>;
        type Str = &'a str;
        type List<T> = &'a [T];

        fn alloc(&self, node: TyNode<Self>) -> Self::TyHandle {
            self.arena.alloc(*node.kind())
        }

        fn alloc_list<T>(&self, iter: impl IntoIterator<Item = T>) -> Self::List<T>
        where
            T: Debug + PartialEq + Eq + Clone + Hash,
        {
            self.arena.alloc_slice_fill_iter(iter)
        }
    }

    #[test]
    fn test_scalar_ord() {
        assert!(Scalar::Bool < Scalar::Int);
        assert!(Scalar::Int < Scalar::Float);
        assert!(Scalar::Float < Scalar::Str);
        assert!(Scalar::Str < Scalar::Bytes);
    }

    #[test]
    fn test_something() {
        assert!(true);
    }
}
