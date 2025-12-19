//! Type construction macros for ergonomic type building.
//!
//! # Example
//!
//! ```
//! use melbi_types::{ty, ArenaBuilder, TyKind, Scalar};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let b = ArenaBuilder::new(&arena);
//!
//! // Scalar types
//! let int_ty = ty!(b, Int);
//! let str_ty = ty!(b, Str);
//!
//! // Compound types
//! let array_int = ty!(b, Array[Int]);
//! let map_ty = ty!(b, Map[Str, Int]);
//!
//! // Function types
//! let func = ty!(b, (Int, Str) => Bool);
//!
//! // With type variables
//! let generic = ty!(b, [k, v] => (Map[k, v], k) => v);
//! ```

/// Macro for constructing types with a concise syntax.
///
/// # Syntax
///
/// | Pattern | Meaning |
/// |---------|---------|
/// | `Int`, `Str`, `Bool`, `Float`, `Bytes` | Scalar types |
/// | `Array[T]` | Array type |
/// | `Map[K, V]` | Map type |
/// | `(T1, T2) => R` | Function type |
/// | `Record[f1: T1, f2: T2]` | Record type |
/// | `[a, b] => T` | Introduce type variables |
#[macro_export]
macro_rules! ty {
    // === Entry points ===

    // With type variables: [a, b, ...] => Type
    ($b:expr, [$($var:ident),+ $(,)?] => $($rest:tt)+) => {{
        let __b = &$b;
        $crate::ty!(@vars __b, 0u16, [$($var),+] ; $($rest)+)
    }};

    // Without type variables
    ($b:expr, $($rest:tt)+) => {{
        let __b = &$b;
        $crate::ty!(@ty __b ; $($rest)+)
    }};

    // === Type variable binding ===

    // Last variable
    (@vars $b:expr, $n:expr, [$var:ident] ; $($rest:tt)+) => {{
        let $var = $crate::TyKind::TypeVar($n).alloc($b);
        $crate::ty!(@ty $b ; $($rest)+)
    }};

    // More variables to bind
    (@vars $b:expr, $n:expr, [$var:ident, $($tail:ident),+] ; $($rest:tt)+) => {{
        let $var = $crate::TyKind::TypeVar($n).alloc($b);
        $crate::ty!(@vars $b, $n + 1, [$($tail),+] ; $($rest)+)
    }};

    // === Scalar types ===

    (@ty $b:expr ; Int) => {
        $crate::TyKind::Scalar($crate::Scalar::Int).alloc($b)
    };
    (@ty $b:expr ; Float) => {
        $crate::TyKind::Scalar($crate::Scalar::Float).alloc($b)
    };
    (@ty $b:expr ; Bool) => {
        $crate::TyKind::Scalar($crate::Scalar::Bool).alloc($b)
    };
    (@ty $b:expr ; Str) => {
        $crate::TyKind::Scalar($crate::Scalar::Str).alloc($b)
    };
    (@ty $b:expr ; Bytes) => {
        $crate::TyKind::Scalar($crate::Scalar::Bytes).alloc($b)
    };

    // === Array[T] ===

    (@ty $b:expr ; Array[$($inner:tt)+]) => {{
        let elem = $crate::ty!(@ty $b ; $($inner)+);
        $crate::TyKind::Array(elem).alloc($b)
    }};

    // === Map[K, V] ===
    // Need to split on comma while handling nested brackets
    // Strategy: accumulate key tokens, when we see a bracketed group, include it whole

    (@ty $b:expr ; Map[$($args:tt)+]) => {{
        $crate::ty!(@map $b ; [] $($args)+)
    }};

    // Map parsing: accumulate key tokens until we hit a top-level comma
    // @map builder ; [key_acc] remaining_tokens

    // Found comma at depth 0 - key is complete, rest is value
    (@map $b:expr ; [$($key:tt)+] , $($val:tt)+) => {{
        let key = $crate::ty!(@ty $b ; $($key)+);
        let val = $crate::ty!(@ty $b ; $($val)+);
        $crate::TyKind::Map(key, val).alloc($b)
    }};

    // Bracketed group - include whole thing in key accumulator
    (@map $b:expr ; [$($key:tt)*] [$($inner:tt)*] $($rest:tt)*) => {
        $crate::ty!(@map $b ; [$($key)* [$($inner)*]] $($rest)*)
    };

    // Any other token - accumulate
    (@map $b:expr ; [$($key:tt)*] $tok:tt $($rest:tt)*) => {
        $crate::ty!(@map $b ; [$($key)* $tok] $($rest)*)
    };

    // === Function (params) => ret ===

    (@ty $b:expr ; ($($params:tt)*) => $($ret:tt)+) => {{
        let params = $crate::ty!(@params $b ; [] [] $($params)*);
        let ret = $crate::ty!(@ty $b ; $($ret)+);
        $crate::TyKind::Function { params, ret }.alloc($b)
    }};

    // === Param list parsing ===
    // @params builder ; [collected_types] [current_type_acc] remaining

    // Empty params
    (@params $b:expr ; [] []) => {{
        $crate::TyList::from_iter($b, core::iter::empty())
    }};

    // End of params - emit last accumulated type
    (@params $b:expr ; [$($collected:tt)*] [$($curr:tt)+]) => {{
        let last = $crate::ty!(@ty $b ; $($curr)+);
        $crate::TyList::from_iter($b, [$($collected)* last].into_iter())
    }};

    // Comma - emit current type, continue
    (@params $b:expr ; [$($collected:tt)*] [$($curr:tt)+] , $($rest:tt)*) => {{
        let item = $crate::ty!(@ty $b ; $($curr)+);
        $crate::ty!(@params $b ; [$($collected)* item,] [] $($rest)*)
    }};

    // Opening bracket in params - need depth tracking
    (@params $b:expr ; [$($collected:tt)*] [$($curr:tt)*] [$($inner:tt)*] $($rest:tt)*) => {{
        $crate::ty!(@params $b ; [$($collected)*] [$($curr)* [$($inner)*]] $($rest)*)
    }};

    // Any other token - accumulate
    (@params $b:expr ; [$($collected:tt)*] [$($curr:tt)*] $tok:tt $($rest:tt)*) => {{
        $crate::ty!(@params $b ; [$($collected)*] [$($curr)* $tok] $($rest)*)
    }};

    // === Record[f1: T1, f2: T2] ===

    (@ty $b:expr ; Record[$($fields:tt)+]) => {{
        $crate::ty!(@record $b ; [] [] $($fields)+)
    }};

    // Record field parsing
    // @record builder ; [collected_fields] [current_field_acc] remaining

    // End of fields - emit last field
    (@record $b:expr ; [$($collected:tt)*] [$name:ident : $($ty:tt)+]) => {{
        let last_ty = $crate::ty!(@ty $b ; $($ty)+);
        let last_field = ($crate::Ident::new($b, stringify!($name)), last_ty);
        $crate::TyKind::Record($crate::FieldList::from_iter($b, [$($collected)* last_field].into_iter())).alloc($b)
    }};

    // Comma - emit current field, continue
    (@record $b:expr ; [$($collected:tt)*] [$name:ident : $($ty:tt)+] , $($rest:tt)*) => {{
        let field_ty = $crate::ty!(@ty $b ; $($ty)+);
        let field = ($crate::Ident::new($b, stringify!($name)), field_ty);
        $crate::ty!(@record $b ; [$($collected)* field,] [] $($rest)*)
    }};

    // Accumulate field tokens (handles nested types in field values)
    (@record $b:expr ; [$($collected:tt)*] [$($curr:tt)*] [$($inner:tt)*] $($rest:tt)*) => {{
        $crate::ty!(@record $b ; [$($collected)*] [$($curr)* [$($inner)*]] $($rest)*)
    }};

    (@record $b:expr ; [$($collected:tt)*] [$($curr:tt)*] $tok:tt $($rest:tt)*) => {{
        $crate::ty!(@record $b ; [$($collected)*] [$($curr)* $tok] $($rest)*)
    }};

    // === Type variable reference (fallback for identifiers) ===

    (@ty $b:expr ; $var:ident) => { $var };
}

#[cfg(test)]
mod tests {
    use crate::{ArenaBuilder, Scalar, TyKind};
    use bumpalo::Bump;

    #[test]
    fn test_scalar_int() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Int);
        assert_eq!(t.kind(), &TyKind::Scalar(Scalar::Int));
    }

    #[test]
    fn test_scalar_str() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Str);
        assert_eq!(t.kind(), &TyKind::Scalar(Scalar::Str));
    }

    #[test]
    fn test_scalar_bool() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Bool);
        assert_eq!(t.kind(), &TyKind::Scalar(Scalar::Bool));
    }

    #[test]
    fn test_scalar_float() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Float);
        assert_eq!(t.kind(), &TyKind::Scalar(Scalar::Float));
    }

    #[test]
    fn test_scalar_bytes() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Bytes);
        assert_eq!(t.kind(), &TyKind::Scalar(Scalar::Bytes));
    }

    #[test]
    fn test_array_simple() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Array[Int]);
        match t.kind() {
            TyKind::Array(elem) => {
                assert_eq!(elem.kind(), &TyKind::Scalar(Scalar::Int));
            }
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    fn test_array_nested() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Array[Array[Int]]);
        match t.kind() {
            TyKind::Array(elem) => match elem.kind() {
                TyKind::Array(inner) => {
                    assert_eq!(inner.kind(), &TyKind::Scalar(Scalar::Int));
                }
                _ => panic!("Expected nested Array"),
            },
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    fn test_map_simple() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Map[Str, Int]);
        match t.kind() {
            TyKind::Map(k, v) => {
                assert_eq!(k.kind(), &TyKind::Scalar(Scalar::Str));
                assert_eq!(v.kind(), &TyKind::Scalar(Scalar::Int));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_map_nested_value() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Map[Str, Array[Int]]);
        match t.kind() {
            TyKind::Map(k, v) => {
                assert_eq!(k.kind(), &TyKind::Scalar(Scalar::Str));
                match v.kind() {
                    TyKind::Array(elem) => {
                        assert_eq!(elem.kind(), &TyKind::Scalar(Scalar::Int));
                    }
                    _ => panic!("Expected Array value"),
                }
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_function_simple() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, (Int) => Bool);
        match t.kind() {
            TyKind::Function { params, ret } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].kind(), &TyKind::Scalar(Scalar::Int));
                assert_eq!(ret.kind(), &TyKind::Scalar(Scalar::Bool));
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn test_function_multiple_params() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, (Int, Str, Float) => Bool);
        match t.kind() {
            TyKind::Function { params, ret } => {
                assert_eq!(params.len(), 3);
                assert_eq!(params[0].kind(), &TyKind::Scalar(Scalar::Int));
                assert_eq!(params[1].kind(), &TyKind::Scalar(Scalar::Str));
                assert_eq!(params[2].kind(), &TyKind::Scalar(Scalar::Float));
                assert_eq!(ret.kind(), &TyKind::Scalar(Scalar::Bool));
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn test_function_no_params() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, () => Int);
        match t.kind() {
            TyKind::Function { params, ret } => {
                assert_eq!(params.len(), 0);
                assert_eq!(ret.kind(), &TyKind::Scalar(Scalar::Int));
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn test_type_var_simple() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, [a] => Array[a]);
        match t.kind() {
            TyKind::Array(elem) => {
                assert_eq!(elem.kind(), &TyKind::TypeVar(0));
            }
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    fn test_type_var_multiple() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, [k, v] => Map[k, v]);
        match t.kind() {
            TyKind::Map(k, v) => {
                assert_eq!(k.kind(), &TyKind::TypeVar(0));
                assert_eq!(v.kind(), &TyKind::TypeVar(1));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_type_var_function() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        // [k, v] => (Map[k, v], k) => v
        let t = ty!(b, [k, v] => (Map[k, v], k) => v);
        match t.kind() {
            TyKind::Function { params, ret } => {
                assert_eq!(params.len(), 2);
                // First param: Map[k, v]
                match params[0].kind() {
                    TyKind::Map(k, v) => {
                        assert_eq!(k.kind(), &TyKind::TypeVar(0));
                        assert_eq!(v.kind(), &TyKind::TypeVar(1));
                    }
                    _ => panic!("Expected Map param"),
                }
                // Second param: k
                assert_eq!(params[1].kind(), &TyKind::TypeVar(0));
                // Return: v
                assert_eq!(ret.kind(), &TyKind::TypeVar(1));
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn test_record_simple() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Record[name: Str, age: Int]);
        match t.kind() {
            TyKind::Record(fields) => {
                assert_eq!(fields.len(), 2);
                // Check fields exist (order may vary due to sorting)
                let has_name = fields.iter().any(|(id, _)| id.as_str() == "name");
                let has_age = fields.iter().any(|(id, _)| id.as_str() == "age");
                assert!(has_name, "Expected 'name' field");
                assert!(has_age, "Expected 'age' field");
            }
            _ => panic!("Expected Record"),
        }
    }

    #[test]
    fn test_record_nested() {
        let arena = Bump::new();
        let b = ArenaBuilder::new(&arena);
        let t = ty!(b, Record[data: Array[Int], lookup: Map[Str, Float]]);
        match t.kind() {
            TyKind::Record(fields) => {
                assert_eq!(fields.len(), 2);
            }
            _ => panic!("Expected Record"),
        }
    }
}
