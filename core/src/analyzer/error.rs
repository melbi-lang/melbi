use alloc::string::ToString;

use crate::api::{Diagnostic, Severity};
use crate::diagnostics::context::Context;
use crate::parser::Span;
use crate::types::{Type, TypeClassId};
use crate::{Box, String, Vec, format, vec};

/// Inner data payload for type errors
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct TypeErrorData {
    pub kind: TypeErrorKind,
    pub source: String,
    pub span: Span,
    pub context: Vec<Context>,
}

/// Type error with context (boxed for 8-byte stack size)
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct TypeError(pub Box<TypeErrorData>);

impl core::ops::Deref for TypeError {
    type Target = TypeErrorData;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for TypeError {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl core::fmt::Display for TypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let diagnostic = self.to_diagnostic();
        write!(f, "{}: {}", diagnostic.severity, diagnostic.message)?;

        if let Some(ref code) = diagnostic.code {
            write!(f, " [{}]", code)?;
        }

        for help_msg in &diagnostic.help {
            write!(f, "\nhelp: {}", help_msg)?;
        }

        Ok(())
    }
}

/// Specific kinds of type errors
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum TypeErrorKind {
    /// Type mismatch between expected and found types.
    /// `context` provides additional help about what the expected type is for.
    TypeMismatch {
        expected: String,
        found: String,
        context: Option<String>,
    },
    /// Unbound/undefined variable
    UnboundVariable { name: String },
    /// Unhandled error type
    UnhandledError,
    /// Occurs check failed (infinite type)
    OccursCheck { type_var: String, ty: String },
    /// Type class constraint violation
    ConstraintViolation {
        ty: String,
        type_class: TypeClassId,
        details: String,
    },
    /// Field count mismatch in records
    FieldCountMismatch { expected: usize, found: usize },
    /// Field name mismatch in records
    FieldNameMismatch { expected: String, found: String },
    /// Function parameter count mismatch
    FunctionParamCountMismatch { expected: usize, found: usize },
    /// Cannot index into a non-indexable type
    NotIndexable { ty: String },
    /// Field does not exist on record
    UnknownField {
        field: String,
        available_fields: Vec<String>,
    },
    /// Cannot infer record type for field access
    CannotInferRecordType { field: String },
    /// Tried to access field on non-record type
    NotARecord { ty: String, field: String },
    /// Invalid type expression in cast
    InvalidTypeExpression { message: String },
    /// Invalid cast between types
    InvalidCast {
        from: String,
        to: String,
        reason: String,
    },
    /// Cast operation on polymorphic type (not yet supported)
    PolymorphicCast { target_type: String },
    /// Duplicate parameter name in lambda
    DuplicateParameter { name: String },
    /// Duplicate binding name in where clause
    DuplicateBinding { name: String },
    /// Type is not formattable in format string
    NotFormattable { ty: String },
    /// Unsupported language feature
    UnsupportedFeature { feature: String, suggestion: String },
    /// Non-exhaustive pattern matching
    NonExhaustivePatterns {
        ty: String,
        missing_cases: Vec<String>,
    },
    /// Generic type error (catch-all for other errors)
    Other { message: String },
}

impl TypeError {
    /// Create a new TypeError with no context
    pub fn new(kind: TypeErrorKind, source: String, span: Span) -> Self {
        Self(Box::new(TypeErrorData {
            kind,
            source,
            span,
            context: Vec::new(),
        }))
    }

    /// Convert to a Diagnostic for API boundary
    pub fn to_diagnostic(&self) -> Diagnostic {
        let (message, code, help) = match &self.kind {
            TypeErrorKind::TypeMismatch {
                expected,
                found,
                context,
            } => {
                let help_msg = context
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| "Types must match in this context".to_string());
                (
                    format!("Type mismatch: expected {}, found {}", expected, found),
                    Some("E001"),
                    vec![help_msg],
                )
            }
            TypeErrorKind::UnboundVariable { name, .. } => (
                format!("Undefined variable '{}'", name),
                Some("E002"),
                vec!["Make sure the variable is declared before use".to_string()],
            ),
            TypeErrorKind::UnhandledError => (
                "Unhandled error type".to_string(),
                Some("E003"),
                vec!["Use 'otherwise' to handle potential errors".to_string()],
            ),
            TypeErrorKind::OccursCheck { type_var, ty, .. } => (
                format!("Cannot construct infinite type: {} = {}", type_var, ty),
                Some("E004"),
                vec!["This usually indicates a recursive type definition".to_string()],
            ),
            TypeErrorKind::ConstraintViolation {
                ty,
                type_class,
                details,
            } => {
                let message = if details.is_empty() {
                    format!("Type '{}' does not implement {}", ty, type_class.name())
                } else {
                    format!("{} constraint not satisfied for '{}': {}", type_class.name(), ty, details)
                };
                (
                    message,
                    Some("E005"),
                    vec![
                        format!(
                            "{} is required for {}",
                            type_class.name(),
                            type_class.description()
                        ),
                        format!(
                            "{} is implemented for: {}",
                            type_class.name(),
                            type_class.instances()
                        ),
                    ],
                )
            }
            TypeErrorKind::FieldCountMismatch {
                expected, found, ..
            } => (
                format!(
                    "Record field count mismatch: expected {}, found {}",
                    expected, found
                ),
                Some("E006"),
                vec![],
            ),
            TypeErrorKind::FieldNameMismatch {
                expected, found, ..
            } => (
                format!(
                    "Record field name mismatch: expected '{}', found '{}'",
                    expected, found
                ),
                Some("E007"),
                vec![],
            ),
            TypeErrorKind::FunctionParamCountMismatch {
                expected, found, ..
            } => (
                format!(
                    "Function parameter count mismatch: expected {}, found {}",
                    expected, found
                ),
                Some("E008"),
                vec!["Check the number of arguments in the function call".to_string()],
            ),
            TypeErrorKind::NotIndexable { ty, .. } => (
                format!("Cannot index into non-indexable type '{}'", ty),
                Some("E009"),
                vec!["Only arrays, maps, and bytes can be indexed".to_string()],
            ),
            TypeErrorKind::UnknownField {
                field,
                available_fields,
                ..
            } => (
                format!(
                    "Record does not have field '{}'. Available fields: {}",
                    field,
                    available_fields.join(", ")
                ),
                Some("E010"),
                vec!["Check the field name for typos".to_string()],
            ),
            TypeErrorKind::CannotInferRecordType { field, .. } => (
                format!(
                    "Cannot infer record type for field access '.{}'. Row polymorphism not yet supported",
                    field
                ),
                Some("E011"),
                vec!["The value must have a concrete record type. Consider restructuring the code to avoid accessing fields on polymorphic values.".to_string()],
            ),
            TypeErrorKind::NotARecord { ty, field, .. } => (
                format!(
                    "Cannot access field '{}' on non-record type '{}'",
                    field, ty
                ),
                Some("E012"),
                vec!["Only record types support field access".to_string()],
            ),
            TypeErrorKind::InvalidTypeExpression { message, .. } => (
                format!("Invalid type expression: {}", message),
                Some("E013"),
                vec![],
            ),
            TypeErrorKind::InvalidCast {
                from, to, reason, ..
            } => (
                format!("Cannot cast from '{}' to '{}': {}", from, to, reason),
                Some("E014"),
                vec!["Only certain type conversions are allowed".to_string()],
            ),
            TypeErrorKind::PolymorphicCast { target_type, .. } => (
                format!("Cannot cast polymorphic value to '{}'", target_type),
                Some("E019"),
                vec![
                    "Casts on polymorphic types are not yet supported.".to_string(),
                    "The value must have a concrete type to be cast.".to_string(),
                ],
            ),
            TypeErrorKind::DuplicateParameter { name, .. } => (
                format!("Duplicate parameter name '{}'", name),
                Some("E015"),
                vec!["Each parameter must have a unique name".to_string()],
            ),
            TypeErrorKind::DuplicateBinding { name, .. } => (
                format!("Duplicate binding name '{}'", name),
                Some("E016"),
                vec!["Each binding in a where clause must have a unique name".to_string()],
            ),
            TypeErrorKind::NotFormattable { ty, .. } => (
                format!("Cannot format type '{}' in format string", ty),
                Some("E017"),
                vec!["Function types cannot be formatted".to_string()],
            ),
            TypeErrorKind::UnsupportedFeature {
                feature,
                suggestion,
                ..
            } => (
                feature.to_string(),
                Some("E018"),
                vec![suggestion.clone()],
            ),
            TypeErrorKind::NonExhaustivePatterns { ty, missing_cases, .. } => (
                format!(
                    "Non-exhaustive patterns: match on type '{}' does not cover all cases",
                    ty
                ),
                Some("E020"),
                vec![format!("Missing cases: {}", missing_cases.join(", "))],
            ),
            TypeErrorKind::Other { message, .. } => (message.clone(), Some("E999"), vec![]),
        };

        Diagnostic {
            severity: Severity::Error,
            message,
            span: self.span.clone(),
            related: self
                .context
                .iter()
                .map(|ctx| ctx.to_related_info())
                .collect(),
            help,
            code: code.map(|s| s.to_string()),
        }
    }

    /// Create a TypeError from a unification error
    pub fn from_unification_error(
        err: crate::types::unification::Error,
        span: Span,
        source: String,
    ) -> Self {
        use crate::types::unification::Error;

        let kind = match err {
            Error::OccursCheckFailed { type_var, ty } => {
                TypeErrorKind::OccursCheck { type_var, ty }
            }
            Error::FieldCountMismatch { expected, found } => {
                TypeErrorKind::FieldCountMismatch { expected, found }
            }
            Error::FieldNameMismatch { expected, found } => {
                TypeErrorKind::FieldNameMismatch { expected, found }
            }
            Error::FunctionParamCountMismatch { expected, found } => {
                TypeErrorKind::FunctionParamCountMismatch { expected, found }
            }
            Error::TypeMismatch { left, right } => TypeErrorKind::TypeMismatch {
                expected: right,
                found: left,
                context: None,
            },
        };

        Self::new(kind, source, span)
    }

    /// Create a TypeError from a type class constraint error
    ///
    /// The error's spans are used as follows:
    /// - `spans[0]` is the primary span (original constraint location)
    /// - `spans[1..]` are instantiation sites added as related context
    pub fn from_constraint_error(
        err: crate::types::type_class_resolver::ConstraintError,
        source: String,
    ) -> Self {
        // Primary span is the original constraint location (spans[0])
        let primary_span = err.spans.first().cloned().unwrap_or(Span(0..0));
        let mut error = Self::new(
            TypeErrorKind::ConstraintViolation {
                ty: err.ty,
                type_class: err.type_class,
                details: err.details,
            },
            source,
            primary_span,
        );

        // Add call site spans as related context (spans[1..] are instantiation sites)
        for span in err.spans.iter().skip(1) {
            error
                .context
                .push(Context::InstantiatedHere { span: span.clone() });
        }

        error
    }
}

/// Helper function to format types for error messages
pub fn format_type<'a>(ty: &'a Type<'a>) -> String {
    format!("{}", ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_error_to_diagnostic() {
        let error = TypeError::new(
            TypeErrorKind::UnboundVariable {
                name: "x".to_string(),
            },
            "test source".to_string(),
            Span(10..20),
        );

        let diagnostic = error.to_diagnostic();
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains("Undefined variable 'x'"));
        assert_eq!(diagnostic.code, Some("E002".to_string()));
    }

    #[test]
    fn type_mismatch_diagnostic() {
        let error = TypeError::new(
            TypeErrorKind::TypeMismatch {
                expected: "Int".to_string(),
                found: "String".to_string(),
                context: None,
            },
            "test source".to_string(),
            Span(5..10),
        );

        let diagnostic = error.to_diagnostic();
        assert!(diagnostic.message.contains("Type mismatch"));
        assert!(diagnostic.message.contains("expected Int"));
        assert!(diagnostic.message.contains("found String"));
        assert_eq!(diagnostic.code, Some("E001".to_string()));
    }

    #[test]
    fn non_exhaustive_patterns_diagnostic() {
        let error = TypeError::new(
            TypeErrorKind::NonExhaustivePatterns {
                ty: "Option[Int]".to_string(),
                missing_cases: vec!["none".to_string()],
            },
            "test source".to_string(),
            Span(15..30),
        );

        let diagnostic = error.to_diagnostic();
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains("Non-exhaustive patterns"));
        assert!(diagnostic.message.contains("Option[Int]"));
        assert!(diagnostic.message.contains("does not cover all cases"));
        assert_eq!(diagnostic.code, Some("E020".to_string()));
        assert_eq!(diagnostic.help.len(), 1);
        assert!(diagnostic.help[0].contains("Missing cases: none"));
    }
}
