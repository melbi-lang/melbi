//! Conversion from parser `TypeExpr` to type system Type.

use alloc::string::ToString;

use crate::parser::{Tree, TypeExpr, TypeExprKind};
use crate::types::Type;
use crate::types::manager::TypeManager;
use crate::{String, Vec};

/// Error returned when converting a `TypeExpr` to a Type.
#[derive(Debug)]
pub enum TypeConversionError {
    UnknownType {
        name: String,
    },
    WrongParameterCount {
        type_name: String,
        expected: usize,
        got: usize,
    },
}

impl core::fmt::Display for TypeConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownType { name } => {
                write!(f, "Unknown type: {name}")
            }
            Self::WrongParameterCount {
                type_name,
                expected,
                got,
            } => {
                write!(
                    f,
                    "{} type expects {} type parameter{}, got {}",
                    type_name,
                    expected,
                    if *expected == 1 { "" } else { "s" },
                    got
                )
            }
        }
    }
}

impl core::error::Error for TypeConversionError {}

/// Converts a parser `TypeExpr` into the type system's Type representation.
///
/// Returns a `TypeConversionError` without span information. The caller should
/// annotate this error with the appropriate source span using the Error type.
pub fn type_expr_to_type<'types>(
    type_manager: &'types TypeManager<'types>,
    type_expr: &Tree<'_, TypeExpr>,
) -> Result<&'types Type<'types>, TypeConversionError> {
    match type_expr.kind() {
        TypeExprKind::Path(path) => {
            // Map type path to built-in types
            match *path {
                "Int" => Ok(type_manager.int()),
                "Float" => Ok(type_manager.float()),
                "Bool" => Ok(type_manager.bool()),
                "String" => Ok(type_manager.str()),
                "Bytes" => Ok(type_manager.bytes()),
                _ => Err(TypeConversionError::UnknownType {
                    name: path.to_string(),
                }),
            }
        }
        TypeExprKind::Parametrized { path, params } => match *path {
            "Array" => {
                if params.len() != 1 {
                    return Err(TypeConversionError::WrongParameterCount {
                        type_name: "Array".to_string(),
                        expected: 1,
                        got: params.len(),
                    });
                }
                let element_ty = type_expr_to_type(type_manager, &params[0])?;
                Ok(type_manager.array(element_ty))
            }
            "Map" => {
                if params.len() != 2 {
                    return Err(TypeConversionError::WrongParameterCount {
                        type_name: "Map".to_string(),
                        expected: 2,
                        got: params.len(),
                    });
                }
                let key_ty = type_expr_to_type(type_manager, &params[0])?;
                let value_ty = type_expr_to_type(type_manager, &params[1])?;
                Ok(type_manager.map(key_ty, value_ty))
            }
            "Option" => {
                if params.len() != 1 {
                    return Err(TypeConversionError::WrongParameterCount {
                        type_name: "Option".to_string(),
                        expected: 1,
                        got: params.len(),
                    });
                }
                let inner_ty = type_expr_to_type(type_manager, &params[0])?;
                Ok(type_manager.option(inner_ty))
            }
            _ => Err(TypeConversionError::UnknownType {
                name: path.to_string(),
            }),
        },
        TypeExprKind::Record(fields) => {
            let field_types: Result<Vec<(&str, &'types Type<'types>)>, TypeConversionError> =
                fields
                    .iter()
                    .map(|field| {
                        let field_ty = type_expr_to_type(type_manager, &field.kind().ty)?;
                        Ok::<_, TypeConversionError>((field.kind().name, field_ty))
                    })
                    .collect();
            let field_types = field_types?;
            Ok(type_manager.record(&field_types))
        }
    }
}

#[cfg(test)]
mod tests {
    use bumpalo::Bump;

    use super::*;
    use crate::parser::{ExprKind, parse};

    /// Convert the type written in `type_source`, parsed as the target of a cast.
    fn convert<'types>(
        type_manager: &'types TypeManager<'types>,
        type_source: &str,
    ) -> Result<&'types Type<'types>, TypeConversionError> {
        let arena = Bump::new();
        let source = arena.alloc_str(&alloc::format!("x as {type_source}"));
        let parsed = parse(&arena, source).expect("type should parse");
        let ExprKind::Cast { ty, .. } = parsed.expr.kind() else {
            panic!("expected a cast, got {:?}", parsed.expr.kind());
        };
        type_expr_to_type(type_manager, ty)
    }

    #[test]
    fn simple_types() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        let test_cases = [
            ("Int", type_manager.int()),
            ("Float", type_manager.float()),
            ("Bool", type_manager.bool()),
            ("String", type_manager.str()),
            ("Bytes", type_manager.bytes()),
        ];

        for (type_source, expected) in test_cases {
            let result = convert(type_manager, type_source).unwrap();
            assert!(core::ptr::eq(result, expected), "{type_source}");
        }
    }

    #[test]
    fn unknown_type() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);
        assert!(convert(type_manager, "UnknownType").is_err());
    }

    #[test]
    fn parametrized_types() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        let test_cases = [
            ("Array[Int]", type_manager.array(type_manager.int())),
            (
                "Map[String, Int]",
                type_manager.map(type_manager.str(), type_manager.int()),
            ),
            (
                "Array[Array[Int]]",
                type_manager.array(type_manager.array(type_manager.int())),
            ),
            ("Option[Int]", type_manager.option(type_manager.int())),
            (
                "Option[Array[String]]",
                type_manager.option(type_manager.array(type_manager.str())),
            ),
            (
                "Option[Option[Bool]]",
                type_manager.option(type_manager.option(type_manager.bool())),
            ),
        ];

        for (type_source, expected) in test_cases {
            let result = convert(type_manager, type_source).unwrap();
            assert!(core::ptr::eq(result, expected), "{type_source}");
        }
    }

    #[test]
    fn record_type() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        let result = convert(type_manager, "Record[name: String, age: Int]").unwrap();
        let expected =
            type_manager.record(&[("name", type_manager.str()), ("age", type_manager.int())]);
        assert!(core::ptr::eq(result, expected));
    }

    #[test]
    fn wrong_parameter_count() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        // `Option[]` is a syntax error, so zero parameters cannot reach the conversion.
        for type_source in ["Array[Int, String]", "Map[Int]", "Option[Int, String]"] {
            assert!(
                matches!(
                    convert(type_manager, type_source),
                    Err(TypeConversionError::WrongParameterCount { .. })
                ),
                "{type_source}"
            );
        }
    }
}
