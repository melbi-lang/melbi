use smallvec::SmallVec;

use super::builder::TyBuilder;
use super::flags::TyFlags;
use super::ty::Ty;
use crate::core::IdentList;
use crate::{FieldList, Ident, TyList};

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
    /// Internal: Do not call, this is called automatically when allocating a [`Ty`].
    /// To access flags use the precomputed value [`Ty::flags`] instead.
    pub(super) fn compute_flags(&self) -> TyFlags {
        match self {
            Self::TypeVar(_) => TyFlags::HAS_TYPE_VARS,
            Self::Scalar(_) => TyFlags::empty(),
            Self::Array(elem) => B::resolve_ty_node(elem).flags(),
            Self::Map(k, v) => B::resolve_ty_node(k).flags() | B::resolve_ty_node(v).flags(),
            Self::Record(fields) => fields.iter().fold(TyFlags::empty(), |acc, (_, ty)| {
                acc | B::resolve_ty_node(ty).flags()
            }),
            Self::Function { params, ret } => {
                let param_flags = params.iter().fold(TyFlags::empty(), |acc, ty| {
                    acc | B::resolve_ty_node(ty).flags()
                });
                param_flags | B::resolve_ty_node(ret).flags()
            }
            Self::Symbol(_) => TyFlags::empty(),
        }
    }

    pub fn alloc(self, builder: &B) -> Ty<B> {
        Ty::new(builder, self)
    }

    pub fn iter_children(
        &self,
    ) -> impl ExactSizeIterator<Item = &Ty<B>> + DoubleEndedIterator + '_ {
        // TODO: Consider using a custom iterator to avoid copying a few references?
        type VecType<'a, T> = SmallVec<[&'a Ty<T>; 6]>;
        match self {
            Self::TypeVar(_) | Self::Scalar(_) | Self::Symbol(_) => {
                VecType::new().into_iter()
            }
            Self::Array(e) => VecType::from_slice(&[e]).into_iter(),
            Self::Map(k, v) => VecType::from_slice(&[k, v]).into_iter(),
            Self::Record(fields) => fields
                .iter()
                .map(|(_, ty)| ty)
                .collect::<VecType<_>>()
                .into_iter(),
            Self::Function { params, ret } => {
                let mut v = params.iter().collect::<VecType<_>>();
                v.push(ret);
                v.into_iter()
            }
        }
    }

    pub fn from_iter_children<Other: TyBuilder>(
        &self,
        builder: &Other,
        mut children: impl ExactSizeIterator<Item = Ty<Other>> + DoubleEndedIterator,
    ) -> TyKind<Other> {
        match self {
            // Leafs: just copy the data
            Self::TypeVar(id) => TyKind::TypeVar(*id),
            Self::Scalar(scalar) => TyKind::Scalar(*scalar),
            Self::Symbol(parts) => {
                let s = parts
                    .iter()
                    .map(|symbol| Ident::new(builder, symbol.as_str()));
                TyKind::Symbol(IdentList::from_iter(builder, s))
            }

            // Recursive: consume from iterator
            Self::Array(_) => TyKind::Array(children.next().unwrap()),

            Self::Map(_, _) => {
                let k = children.next().unwrap();
                let v = children.next().unwrap();
                TyKind::Map(k, v)
            }

            Self::Record(fields) => {
                // Reconstruct record: Keep original names, take new types
                // We assume `Other::Ty` is compatible with the storage in `TyKind::Record`
                // (e.g., standard Vec or generic list handle)
                let new_fields = fields.iter().map(|(name, _)| {
                    (Ident::new(builder, name.as_str()), children.next().unwrap())
                });
                TyKind::Record(FieldList::from_iter(builder, new_fields))
            }

            Self::Function { params, .. } => {
                // Invariant: `iter_children` yields params first, then ret last.
                // We use `next_back()` to pop the return type before consuming params.
                debug_assert!(
                    children.len() >= 1 && children.len() == params.len() + 1,
                    "Function children count mismatch: expected {} params + 1 ret, got {}",
                    params.len(),
                    children.len()
                );
                let new_ret = children.next_back().unwrap();

                TyKind::Function {
                    params: TyList::from_iter(builder, children),
                    ret: new_ret,
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_ord() {
        assert!(Scalar::Bool < Scalar::Int);
        assert!(Scalar::Int < Scalar::Float);
        assert!(Scalar::Float < Scalar::Str);
        assert!(Scalar::Str < Scalar::Bytes);
    }
}
