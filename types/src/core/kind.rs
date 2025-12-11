use super::builder::TyBuilder;
use super::flags::TyFlags;
use super::ty::Ty;

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
    Array(B::Ty),

    /// Map type with key and value types
    Map(B::Ty, B::Ty),

    /// Record (struct) with named fields.
    ///
    /// Fields are stored sorted by name for canonical representation.
    /// Field names are interned strings for efficient comparison.
    Record(B::FieldList),

    /// Function type with parameters and return type.
    ///
    /// Parameters are stored as an interned list of types.
    Function { params: B::TyList, ret: B::Ty },

    /// Symbol (tagged union) with sorted parts.
    ///
    /// Parts are interned strings stored in sorted order.
    /// Example: Symbol["error", "pending", "success"]
    Symbol(B::IdentList),
}

impl<B: TyBuilder> TyKind<B> {
    pub fn compute_flags(&self) -> TyFlags {
        match self {
            TyKind::TypeVar(_) => TyFlags::HAS_TYPE_VARS,
            TyKind::Scalar(_) => TyFlags::empty(),
            TyKind::Array(elem) => B::resolve_ty_node(elem).flags(),
            TyKind::Map(k, v) => B::resolve_ty_node(k).flags() | B::resolve_ty_node(v).flags(),
            TyKind::Record(fields) => fields.iter().fold(TyFlags::empty(), |acc, (_, ty)| {
                acc | B::resolve_ty_node(ty).flags()
            }),
            TyKind::Function { params, ret } => {
                let param_flags = params.iter().fold(TyFlags::empty(), |acc, ty| {
                    acc | B::resolve_ty_node(ty).flags()
                });
                param_flags | B::resolve_ty_node(ret).flags()
            }
            TyKind::Symbol(_) => TyFlags::empty(),
        }
    }

    pub fn alloc(self, builder: &B) -> Ty<B> {
        Ty::new(builder, self)
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
    fn test_scalar_ord() {
        assert!(Scalar::Bool < Scalar::Int);
        assert!(Scalar::Int < Scalar::Float);
        assert!(Scalar::Float < Scalar::Str);
        assert!(Scalar::Str < Scalar::Bytes);
    }
}
