use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};

use serde::Serialize;

use crate::types::traits::display_type;

#[derive(Serialize, Clone, Hash)]
#[repr(C, u8)]
pub enum Type<'a> {
    // Type variables.
    TypeVar(u16) = 0,

    // Primitives.
    Int = 1,
    Float = 2,
    Bool = 3,
    Str = 4,
    Bytes = 5,

    // Collections.
    Array(&'a Self) = 6,
    Map(&'a Self, &'a Self) = 7,

    // Structural records.
    Record(&'a [(&'a str, &'a Self)]) = 8, // Must be sorted by field name.

    // Functions.
    Function {
        params: &'a [&'a Self],
        ret: &'a Self,
    } = 9,

    // Symbols.
    Symbol(&'a [&'a str]) = 10, // Must be sorted.

    // Option type.
    Option(&'a Self) = 11,
    // TODO: More types to add later:
    //   Custom(&'a str),
    //   Union(&'a [&'a Type<'a>]),  // Must be sorted.
}

impl Type<'_> {
    #[must_use]
    pub fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(C, u8)`, its layout is a `repr(C)` `struct`
        // with a `u8` discriminant and a union of `structs`, so we can read the discriminant
        // directly without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }
}

pub(super) struct CompareTypeArgs<'a>(pub(super) Type<'a>);

impl Hash for CompareTypeArgs<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(&self.0).hash(state);
        match &self.0 {
            // Primitives - just discriminant is enough (no additional data)
            Type::Int | Type::Float | Type::Bool | Type::Str | Type::Bytes => {}

            // TypeVar - hash the ID
            Type::TypeVar(id) => {
                id.hash(state);
            }

            Type::Array(elem) => {
                core::ptr::from_ref::<Type<'_>>(*elem).hash(state);
            }
            Type::Map(key, val) => {
                core::ptr::from_ref::<Type<'_>>(*key).hash(state);
                core::ptr::from_ref::<Type<'_>>(*val).hash(state);
            }
            Type::Option(inner) => {
                core::ptr::from_ref::<Type<'_>>(*inner).hash(state);
            }
            Type::Function { params, ret } => {
                for param in *params {
                    core::ptr::from_ref::<Type<'_>>(*param).hash(state);
                }
                core::ptr::from_ref::<Type<'_>>(*ret).hash(state);
            }
            Type::Symbol(parts) => {
                for part in *parts {
                    core::ptr::from_ref::<str>(*part).hash(state);
                }
            }
            Type::Record(fields) => {
                for (name, ty) in *fields {
                    core::ptr::from_ref::<str>(*name).hash(state);
                    core::ptr::from_ref::<Type<'_>>(*ty).hash(state);
                }
            }
        }
    }
}

impl PartialEq for CompareTypeArgs<'_> {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(&self.0) == core::mem::discriminant(&other.0)
            && match (&self.0, &other.0) {
                // Primitives - discriminant comparison is sufficient
                (Type::Int, Type::Int)
                | (Type::Float, Type::Float)
                | (Type::Bool, Type::Bool)
                | (Type::Str, Type::Str)
                | (Type::Bytes, Type::Bytes) => true,

                // TypeVar - compare IDs
                (Type::TypeVar(id1), Type::TypeVar(id2)) => id1 == id2,

                (Type::Array(elem1), Type::Array(elem2)) => core::ptr::eq(*elem1, *elem2),
                (Type::Map(key1, val1), Type::Map(key2, val2)) => {
                    core::ptr::eq(*key1, *key2) && core::ptr::eq(*val1, *val2)
                }
                (Type::Option(inner1), Type::Option(inner2)) => core::ptr::eq(*inner1, *inner2),
                (
                    Type::Function {
                        params: params1,
                        ret: ret1,
                    },
                    Type::Function {
                        params: params2,
                        ret: ret2,
                    },
                ) => {
                    params1.len() == params2.len()
                        && params1
                            .iter()
                            .zip(*params2)
                            .all(|(&a, &b)| core::ptr::eq(a, b))
                        && core::ptr::eq(*ret1, *ret2)
                }
                (Type::Symbol(parts1), Type::Symbol(parts2)) => {
                    parts1.len() == parts2.len()
                        && parts1.iter().zip(*parts2).all(|(a, b)| {
                            core::ptr::eq(
                                core::ptr::from_ref::<str>(*a),
                                core::ptr::from_ref::<str>(*b),
                            )
                        })
                }
                (Type::Record(fields1), Type::Record(fields2)) => {
                    fields1.len() == fields2.len()
                        && fields1
                            .iter()
                            .zip(*fields2)
                            .all(|((name1, ty1), (name2, ty2))| {
                                core::ptr::eq(
                                    core::ptr::from_ref::<str>(*name1),
                                    core::ptr::from_ref::<str>(*name2),
                                ) && core::ptr::eq(*ty1, *ty2)
                            })
                }
                _ => false,
            }
    }
}

impl Eq for CompareTypeArgs<'_> {}

// Pointer-based equality for &Type (used by TypeView trait)
// Two type references are equal if they point to the same arena-allocated type
// This enables fast O(1) equality checks via interning
impl<'a> PartialEq for &'a Type<'a> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(
            core::ptr::from_ref::<Type<'a>>(*self),
            core::ptr::from_ref::<Type<'a>>(*other),
        )
    }
}

impl<'a> Eq for &'a Type<'a> {}

impl Display for Type<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Delegate to the generic display_type function
        write!(f, "{}", display_type(self))
    }
}

impl Debug for Type<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to the generic display_type function
        write!(f, "{}", display_type(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_eq() {
        let ty1 = &Type::Int;
        let ty2 = &Type::Int;
        assert_eq!(ty1, ty2);
    }

    #[test]
    fn discriminant() {
        let ty = &Type::Int;
        assert_eq!(ty.discriminant(), 1);
    }
}
