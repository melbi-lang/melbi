use super::Type;

/// A type scheme representing a polymorphic type with universally quantified type variables.
///
/// For example, the identity function has type scheme `∀a. a → a`, which is represented as:
/// ```text
/// TypeScheme {
///     quantified: &[0],  // Type variable 'a' with ID 0
///     ty: Function { params: [TypeVar(0)], ret: TypeVar(0) },
///     lambda_expr: None,
/// }
/// ```
///
/// Monomorphic types are represented with an empty quantified list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeScheme<'types, 'arena> {
    /// List of quantified type variable IDs (e.g., [0, 1] for ∀a,b. ...)
    /// Should be sorted and deduplicated.
    pub quantified: &'types [u16],

    /// The type containing the quantified variables
    pub ty: &'types Type<'types>,

    /// Optional pointer to the lambda expression if this scheme represents a polymorphic lambda
    /// This allows tracking instantiations without a separate lookup table
    pub lambda_expr: Option<*const crate::analyzer::typed_expr::Expr<'types, 'arena>>,
}

impl<'types, 'arena> TypeScheme<'types, 'arena> {
    /// Create a new type scheme without a lambda expression.
    pub fn new(quantified: &'types [u16], ty: &'types Type<'types>) -> Self {
        TypeScheme {
            quantified,
            ty,
            lambda_expr: None,
        }
    }

    /// Create a new type scheme with an associated lambda expression.
    pub fn new_with_lambda(
        quantified: &'types [u16],
        ty: &'types Type<'types>,
        lambda_expr: *const crate::analyzer::typed_expr::Expr<'types, 'arena>,
    ) -> Self {
        TypeScheme {
            quantified,
            ty,
            lambda_expr: Some(lambda_expr),
        }
    }

    /// Returns true if this is a monomorphic type (no quantified variables).
    pub fn is_monomorphic(&self) -> bool {
        self.quantified.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use bumpalo::Bump;

    use super::*;
    use crate::types::manager::TypeManager;

    #[test]
    fn monomorphic_scheme() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        let int_ty = mgr.int();
        let scheme = TypeScheme::new(&[], int_ty);

        assert!(scheme.is_monomorphic());
        assert_eq!(scheme.quantified.len(), 0);
        assert!(core::ptr::eq(scheme.ty, int_ty));
    }

    #[test]
    fn polymorphic_scheme() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        let type_var = mgr.type_var(0);
        let quantified = bump.alloc_slice_copy(&[0u16]);
        let scheme = TypeScheme::new(quantified, type_var);

        assert!(!scheme.is_monomorphic());
        assert_eq!(scheme.quantified.len(), 1);
        assert_eq!(scheme.quantified[0], 0);
    }

    #[test]
    fn polymorphic_function_scheme() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        // ∀a. a → a (identity function)
        let type_var = mgr.type_var(0);
        let func_ty = mgr.function(&[type_var], type_var);
        let quantified = bump.alloc_slice_copy(&[0u16]);
        let scheme = TypeScheme::new(quantified, func_ty);

        assert!(!scheme.is_monomorphic());
        assert_eq!(scheme.quantified, &[0]);
    }
}
