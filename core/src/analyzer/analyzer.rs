use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::ToString;
use core::cell::RefCell;

use bumpalo::Bump;
use hashbrown::DefaultHashBuilder;

use crate::analyzer::error::{TypeError, TypeErrorKind};
use crate::analyzer::typed_expr::{
    self as typed_expr, Expr, ExprInner, LambdaInstantiations, TypedExpr,
};
use crate::parser::{self, BinaryOp, ComparisonOp, Span, UnaryOp};
use crate::scope_stack::{self, ScopeStack};
use crate::types::manager::TypeManager;
use crate::types::traits::{TypeKind, TypeView};
use crate::types::unification::Unification;
use crate::types::{Type, TypeClassResolver, TypeScheme, type_expr_to_type};
use crate::values::dynamic::Value;
use crate::{String, Vec, casting, format};

// TODO: Create a temporary TypeManager for analysis only.
pub fn analyze<'types, 'arena>(
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    expr: &'arena parser::ParsedExpr<'arena>,
    globals: &[(&'arena str, &'types Type<'types>)],
    variables: &[(&'arena str, &'types Type<'types>)],
) -> Result<&'arena TypedExpr<'types, 'arena>, TypeError> {
    tracing::info!(
        globals_count = globals.len(),
        variables_count = variables.len(),
        "Starting type analysis"
    );

    // Create annotation map for typed expressions
    // We reuse the same source string since both ParsedExpr and TypedExpr are in the same arena
    let typed_ann = arena.alloc(parser::AnnotatedSource::new(arena, expr.ann.source));

    let mut analyzer = Analyzer {
        type_manager,
        arena,
        scope_stack: ScopeStack::new(),
        unification: Unification::new(type_manager),
        type_class_resolver: TypeClassResolver::new(),
        parsed_ann: expr.ann,
        typed_ann,
        current_span: None, // Initialize to None
        env_vars_stack: Vec::new(),
        polymorphic_lambdas: hashbrown::HashMap::new(),
        pending_instantiations: hashbrown::HashMap::new(),
    };

    // Push globals scope (constants, packages, functions)
    if !globals.is_empty() {
        // Wrap each type in a monomorphic TypeScheme
        // TODO: Accept TypeScheme as an argument.
        let bindings: Vec<(&'arena str, TypeScheme<'types, 'arena>)> = globals
            .iter()
            .map(|(name, ty)| {
                let empty_quantified = type_manager.alloc_u16_slice(&[]);
                (*name, TypeScheme::new(empty_quantified, ty))
            })
            .collect();
        let bindings_slice = arena.alloc_slice_fill_iter(bindings);
        analyzer
            .scope_stack
            .push(scope_stack::CompleteScope::from_sorted(bindings_slice));
    }

    // Push variables scope (client-provided runtime variables)
    if !variables.is_empty() {
        // Wrap each type in a monomorphic TypeScheme
        // TODO: Accept TypeScheme as an argument.
        let bindings: Vec<(&'arena str, TypeScheme<'types, 'arena>)> = variables
            .iter()
            .map(|(name, ty)| {
                let empty_quantified = type_manager.alloc_u16_slice(&[]);
                (*name, TypeScheme::new(empty_quantified, ty))
            })
            .collect();
        let bindings_slice = arena.alloc_slice_fill_iter(bindings);
        analyzer
            .scope_stack
            .push(scope_stack::CompleteScope::from_sorted(bindings_slice));
    }
    let result = analyzer.analyze_expr(expr)?;

    // Check all type class constraints after unification
    analyzer.finalize_constraints()?;

    // Resolve all type variables in the expression tree
    // This replaces type variables with their fully resolved types (e.g., _5 → Str)
    // Type variables that aren't unified (e.g., generalized lambda body vars) remain unchanged
    // We also track old→new pointer mappings to remap lambda_instantiations keys
    let mut ptr_remap = hashbrown::HashMap::new();
    let resolved_expr = analyzer.resolve_expr_types(result.expr, &mut ptr_remap);

    // Build final instantiation substitutions by resolving fresh vars to concrete types
    let old_lambda_instantiations = analyzer.build_lambda_instantiations(arena);

    // Remap the lambda_instantiations keys from old pointers to new pointers
    // This is necessary because resolve_expr_types allocates new Expr nodes
    let lambda_instantiations =
        Analyzer::remap_lambda_instantiations(old_lambda_instantiations, &ptr_remap, arena);

    // Create new TypedExpr with resolved expression and remapped instantiation info
    let resolved_result = analyzer.arena.alloc(TypedExpr {
        expr: resolved_expr,
        ann: result.ann,
        lambda_instantiations,
    });

    Ok(resolved_result)
}

struct Analyzer<'types, 'arena> {
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    scope_stack: ScopeStack<'arena, TypeScheme<'types, 'arena>>,
    unification: Unification<'types, &'types TypeManager<'types>>,
    type_class_resolver: TypeClassResolver<'types>,
    parsed_ann: &'arena parser::AnnotatedSource<'arena, parser::Expr<'arena>>,
    typed_ann: &'arena parser::AnnotatedSource<'arena, Expr<'types, 'arena>>,
    current_span: Option<Span>, // Track current expression span
    /// Stack of environment type variables from outer scopes.
    /// Each element is a set of type variables that should not be generalized.
    /// When entering a lambda or let-binding, we push the free vars from parameters.
    /// When exiting, we pop them.
    env_vars_stack: Vec<hashbrown::HashSet<u16>>,
    /// Track polymorphic lambdas (lambda pointer -> type scheme)
    polymorphic_lambdas:
        hashbrown::HashMap<*const Expr<'types, 'arena>, TypeScheme<'types, 'arena>>,
    /// Track instantiations as they occur (lambda pointer -> list of (fresh var ID -> generalized var ID) mappings)
    /// These will be resolved to concrete types after `finalize_constraints`
    pending_instantiations:
        hashbrown::HashMap<*const Expr<'types, 'arena>, Vec<hashbrown::HashMap<u16, u16>>>,
}

impl<'types, 'arena> Analyzer<'types, 'arena> {
    fn analyze_expr(
        &mut self,
        expr: &parser::ParsedExpr<'arena>,
    ) -> Result<&'arena mut TypedExpr<'types, 'arena>, TypeError> {
        let typed_expr = self.analyze(expr.expr)?;
        // This is used internally, instantiations will be added at the top level
        Ok(self.arena.alloc(TypedExpr {
            expr: typed_expr,
            ann: self.typed_ann,
            lambda_instantiations: hashbrown::HashMap::new_in(self.arena),
        }))
    }

    fn alloc(
        &mut self,
        ty: &'types Type<'types>,
        inner: ExprInner<'types, 'arena>,
    ) -> &'arena mut Expr<'types, 'arena> {
        let typed_expr = self
            .arena
            .alloc(Expr(self.unification.fully_resolve(ty), inner));
        // Copy span from current_span to typed annotation
        if let Some(ref span) = self.current_span {
            self.typed_ann.add_span(typed_expr, span.clone());
        }
        typed_expr
    }

    /// Helper to wrap unification errors with a specific expression's span
    fn with_context_for<T>(
        &self,
        expr: &'arena Expr<'types, 'arena>,
        result: Result<T, crate::types::unification::Error>,
    ) -> Result<T, TypeError> {
        result.map_err(|err| {
            let span = self.typed_ann.span_of(expr).unwrap_or(Span(0..0));
            TypeError::from_unification_error(err, span, self.get_source())
        })
    }

    // Get current span or default
    fn get_span(&self) -> Span {
        self.current_span.clone().unwrap_or(Span(0..0))
    }

    // Get source code as String
    fn get_source(&self) -> String {
        self.parsed_ann.source.to_string()
    }

    /// Helper to create a `TypeError` with the current span and source
    fn type_error(&self, kind: TypeErrorKind) -> TypeError {
        TypeError::new(kind, self.get_source(), self.get_span())
    }

    /// Helper to return a `TypeError` as an Err with the current span and source
    fn error<T>(&self, kind: TypeErrorKind) -> Result<T, TypeError> {
        Err(self.type_error(kind))
    }

    // Helper for internal/unexpected errors (invariant violations)
    fn internal_error(&self, message: impl Into<String>) -> TypeError {
        self.type_error(TypeErrorKind::Other {
            message: message.into(),
        })
    }

    /// Check that two expression types match (symmetric case).
    /// Points the error at `expr` with "Types must match in this context" help message.
    /// Use for cases like: if branches, match arms, array elements.
    fn expect_types_match(
        &mut self,
        expr: &'arena Expr<'types, 'arena>,
        got: &'types Type<'types>,
        expected: &'types Type<'types>,
    ) -> Result<&'types Type<'types>, TypeError> {
        let unification_result = self.unification.unifies_to(got, expected);
        self.with_context_for(expr, unification_result)
    }

    /// Check that an expression has a specific expected type (asymmetric case).
    /// Points the error at `expr` with a context-specific help message.
    /// Use for cases like: if condition must be Bool, index must be Int.
    fn expect_type_to_be(
        &mut self,
        expr: &'arena Expr<'types, 'arena>,
        got: &'types Type<'types>,
        expected: &'types Type<'types>,
        context: &str,
    ) -> Result<&'types Type<'types>, TypeError> {
        self.unification.unifies_to(got, expected).map_err(|err| {
            let span = self.typed_ann.span_of(expr).unwrap_or(Span(0..0));
            match err {
                crate::types::unification::Error::TypeMismatch { left, right } => TypeError::new(
                    TypeErrorKind::TypeMismatch {
                        expected: right,
                        found: left,
                        context: Some(context.to_string()),
                    },
                    self.get_source(),
                    span,
                ),
                other => TypeError::from_unification_error(other, span, self.get_source()),
            }
        })
    }

    // Finalize type checking by resolving all type class constraints
    fn finalize_constraints(&mut self) -> Result<(), TypeError> {
        self.type_class_resolver
            .resolve_all(&mut self.unification)
            .map_err(|errors| {
                // For now, just report the first error
                // In the future, we can report all errors
                if let Some(first_error) = errors.first() {
                    TypeError::from_constraint_error(first_error.clone(), self.get_source())
                } else {
                    self.internal_error("Type class constraint error".to_string())
                }
            })
    }

    /// Get the current environment type variables (union of all sets in the stack).
    /// These are type variables that should NOT be generalized in let-polymorphism.
    fn get_env_vars(&self) -> hashbrown::HashSet<u16> {
        let mut result = hashbrown::HashSet::new();
        for set in &self.env_vars_stack {
            result.extend(set);
        }
        result
    }

    fn analyze(
        &mut self,
        expr: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Set current span for this expression from parsed annotations
        let old_span = self.current_span.clone();
        self.current_span = self.parsed_ann.span_of(expr);

        let result = match expr {
            parser::Expr::Binary { op, left, right } => self.analyze_binary(*op, left, right),
            parser::Expr::Boolean { op, left, right } => self.analyze_boolean(*op, left, right),
            parser::Expr::Comparison { op, left, right } => {
                self.analyze_comparison(*op, left, right)
            }
            parser::Expr::Unary { op, expr } => self.analyze_unary(*op, expr),
            parser::Expr::Call { callable, args } => self.analyze_call(callable, args),
            parser::Expr::Index { value, index } => self.analyze_index(value, index),
            parser::Expr::Field { value, field } => self.analyze_field(value, field),
            parser::Expr::Cast { ty, expr } => self.analyze_cast(ty, expr),
            parser::Expr::Lambda { params, body } => self.analyze_lambda(params, body),
            parser::Expr::If {
                cond,
                then_branch,
                else_branch,
            } => self.analyze_if(cond, then_branch, else_branch),
            parser::Expr::Where { expr, bindings } => self.analyze_where(expr, bindings),
            parser::Expr::Otherwise { primary, fallback } => {
                self.analyze_otherwise(primary, fallback)
            }
            parser::Expr::Option { inner } => self.analyze_option(*inner),
            parser::Expr::Match { expr, arms } => self.analyze_match(expr, arms),
            parser::Expr::Record(items) => self.analyze_record(items),
            parser::Expr::Map(items) => self.analyze_map(items),
            parser::Expr::Array(exprs) => self.analyze_array(exprs),
            parser::Expr::FormatStr { strs, exprs } => self.analyze_format_str(strs, exprs),
            parser::Expr::Literal(literal) => self.analyze_literal(literal),
            parser::Expr::Ident(ident) => self.analyze_ident(ident),
        };

        // Restore previous span
        self.current_span = old_span;

        result
    }

    fn analyze_binary(
        &mut self,
        op: BinaryOp,
        left: &'arena parser::Expr<'arena>,
        right: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let left = self.analyze(left)?;
        let right = self.analyze(right)?;

        // Unify left and right to determine result type - point to right if mismatch
        let result_ty = self.expect_types_match(right, right.0, left.0)?;

        // Add relational Numeric constraint: Numeric(left, right, result)
        // The constraint resolver will verify and unify based on the numeric instance:
        //   - (Int, Int) => Int
        //   - (Float, Float) => Float
        self.type_class_resolver.add_numeric_constraint(
            left.0,
            right.0,
            result_ty,
            self.get_span(),
        );

        Ok(self.alloc(result_ty, ExprInner::Binary { op, left, right }))
    }

    fn analyze_boolean(
        &mut self,
        op: parser::BoolOp,
        left: &'arena parser::Expr<'arena>,
        right: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let left = self.analyze(left)?;
        let right = self.analyze(right)?;

        self.expect_type_to_be(
            left,
            left.0,
            self.type_manager.bool(),
            "Operand of 'and'/'or' must be Bool",
        )?;
        self.expect_type_to_be(
            right,
            right.0,
            self.type_manager.bool(),
            "Operand of 'and'/'or' must be Bool",
        )?;

        Ok(self.alloc(
            self.type_manager.bool(),
            ExprInner::Boolean { op, left, right },
        ))
    }

    fn analyze_comparison(
        &mut self,
        op: ComparisonOp,
        left: &'arena parser::Expr<'arena>,
        right: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let left = self.analyze(left)?;
        let right = self.analyze(right)?;

        // For equality operators (== and !=), any types can be compared
        // For ordering operators (<, >, <=, >=), operands must support Ord (Int, Float, Str, Bytes)
        // For containment operators (in, not in), we support:
        //     (Str, Str), (Bytes, Bytes), (element, Array), (key, Map)
        match op {
            ComparisonOp::Eq | ComparisonOp::Neq => {
                // Equality: just ensure both operands have the same type
                self.expect_types_match(right, right.0, left.0)?;
            }
            ComparisonOp::Lt | ComparisonOp::Gt | ComparisonOp::Le | ComparisonOp::Ge => {
                // Ordering: operands must support Ord and have the same type

                // Unify left and right - point to right if mismatch
                self.expect_types_match(right, right.0, left.0)?;

                // Add Ord constraints - Ord is a simple predicate, not relational
                let span = self.get_span();
                self.type_class_resolver
                    .add_ord_constraint(left.0, span.clone());
                self.type_class_resolver.add_ord_constraint(right.0, span);

                // Note: No need to check immediately - finalize_constraints will check
            }
            ComparisonOp::In | ComparisonOp::NotIn => {
                // Containment: needle in haystack
                // Supported: (Str, Str), (Bytes, Bytes), (element, Array[element]), (key, Map[key, value])
                let span = self.get_span();
                self.type_class_resolver.add_containable_constraint(
                    left.0,  // needle
                    right.0, // haystack
                    span,
                );

                // Note: No need to check immediately - finalize_constraints will check
            }
        }

        Ok(self.alloc(
            self.type_manager.bool(),
            ExprInner::Comparison { op, left, right },
        ))
    }

    fn analyze_unary(
        &mut self,
        op: UnaryOp,
        expr: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let expr = self.analyze(expr)?;

        match op {
            UnaryOp::Neg => {
                // Negation: operand must be numeric
                // Add relational Numeric constraint: Numeric(expr, expr, expr)
                // (negation preserves type: -Int => Int, -Float => Float)
                let span = self.get_span();
                self.type_class_resolver
                    .add_numeric_constraint(expr.0, expr.0, expr.0, span);

                Ok(self.alloc(expr.0, ExprInner::Unary { op, expr }))
            }
            UnaryOp::Not => {
                // Logical not: operand must be Bool
                self.expect_type_to_be(
                    expr,
                    expr.0,
                    self.type_manager.bool(),
                    "Operand of 'not' must be Bool",
                )?;

                Ok(self.alloc(self.type_manager.bool(), ExprInner::Unary { op, expr }))
            }
        }
    }

    fn analyze_call(
        &mut self,
        callable: &'arena parser::Expr<'arena>,
        args: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // 1. Analyze callable and its arguments.
        let callable = self.analyze(callable)?;
        let args_typed = self
            .arena
            .alloc_slice_try_fill_iter(args.iter().map(|arg| self.analyze(arg)))?;

        // 2. Extract actual argument types.
        let arg_types: Vec<_> = args_typed.iter().map(|arg| arg.0).collect();

        // 3. Create a fresh type variable for the return type.
        let ret_ty = self.type_manager.fresh_type_var();

        // 4. Construct the expected function type.
        let expected_fn_ty = self.type_manager.function(&arg_types, ret_ty);

        // 5. Unify callable's type with the expected function type.
        // TODO: Improve error reporting for function calls. Currently, when unification
        // fails (e.g., argument type mismatch, nested function arity mismatch), we point
        // to the whole call expression but can't pinpoint which argument caused the issue.
        // A proper solution would have unification return a trace structure showing where
        // it failed, allowing us to point to the specific problematic argument.
        let unified_fn_type = self
            .unification
            .unifies_to(callable.0, expected_fn_ty)
            .map_err(|err| {
                TypeError::from_unification_error(err, self.get_span(), self.get_source())
            })?;

        // 6. Extract return type from unified function type.
        let TypeKind::Function { ret: result_ty, .. } = unified_fn_type.view() else {
            return Err(self.internal_error(format!(
                "Internal error: Expected Function type after unification, got {unified_fn_type}"
            )));
        };

        // 7. Resolve the return type through substitution
        let resolved_ret_ty = self.unification.resolve(result_ty);

        // 8. Create the typed Call expression
        Ok(self.alloc(
            resolved_ret_ty,
            ExprInner::Call {
                callable,
                args: self
                    .arena
                    .alloc_slice_fill_iter(args_typed.iter_mut().map(|arg| &**arg)),
            },
        ))
    }

    fn analyze_index(
        &mut self,
        value: &'arena parser::Expr<'arena>,
        index: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let value = self.analyze(value)?;
        let index = self.analyze(index)?;

        // Determine the result type based on the value type
        let result_ty = match value.0.view() {
            TypeKind::Array(element_ty) => {
                // Arrays are indexed by integers
                self.expect_type_to_be(
                    index,
                    index.0,
                    self.type_manager.int(),
                    "Array index must be Int",
                )?;
                element_ty
            }
            TypeKind::Map(key_ty, value_ty) => {
                // Maps are indexed by their key type
                self.expect_types_match(index, index.0, key_ty)?;
                value_ty
            }
            TypeKind::Bytes => {
                // Bytes are indexed by integers, return Int
                self.expect_type_to_be(
                    index,
                    index.0,
                    self.type_manager.int(),
                    "Bytes index must be Int",
                )?;
                self.type_manager.int()
            }
            TypeKind::TypeVar(_) => {
                // Type variable not yet resolved - add relational Indexable constraint
                // The constraint tracks: Indexable(container, index, result)
                // When resolved, it will unify based on the container's concrete type:
                //   - Array[E]: index=Int, result=E
                //   - Map[K,V]: index=K, result=V
                //   - Bytes: index=Int, result=Int

                let result_ty = self.type_manager.fresh_type_var();

                // Add the relational constraint
                self.type_class_resolver.add_indexable_constraint(
                    value.0,
                    index.0,
                    result_ty,
                    self.get_span(),
                );

                result_ty
            }
            _ => {
                return self.error(TypeErrorKind::NotIndexable {
                    ty: format!("{}", value.0),
                });
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Index { value, index }))
    }

    fn analyze_field(
        &mut self,
        value: &'arena parser::Expr<'arena>,
        field: &'arena str,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let value = self.analyze(value)?;

        // Check that value is a record and get the field type
        let result_ty = match value.0.view() {
            TypeKind::Record(fields) => {
                // Clone the iterator to use it twice (once for search, once for error message)
                let fields_vec: Vec<_> = fields.collect();

                // Look for the field in the record
                fields_vec
                    .iter()
                    .find(|(name, _)| *name == field)
                    .map(|(_, ty)| *ty)
                    .ok_or_else(|| {
                        self.type_error(TypeErrorKind::UnknownField {
                            field: field.to_string(),
                            available_fields: fields_vec
                                .iter()
                                .map(|(n, _)| n.to_string())
                                .collect(),
                        })
                    })?
            }
            TypeKind::TypeVar(_) => {
                // Cannot infer record type from field access alone
                // TODO(row-polymorphism): With row polymorphism, we could infer
                // "any record with at least field 'x' of some type"
                return self.error(TypeErrorKind::CannotInferRecordType {
                    field: field.to_string(),
                });
            }
            _ => {
                return self.error(TypeErrorKind::NotARecord {
                    ty: format!("{}", value.0),
                    field: field.to_string(),
                });
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Field { value, field }))
    }

    fn analyze_cast(
        &mut self,
        ty_expr: &'arena parser::TypeExpr<'arena>,
        expr: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let analyzed_expr = self.analyze(expr)?;
        let source_type = analyzed_expr.0;
        let target_type = match type_expr_to_type(self.type_manager, ty_expr) {
            Ok(ty) => ty,
            Err(e) => {
                return self.error(TypeErrorKind::InvalidTypeExpression {
                    message: e.to_string(),
                });
            }
        };

        // Check if source type is a type variable (polymorphic)
        if matches!(source_type.view(), TypeKind::TypeVar(_)) {
            let mut err = self.type_error(TypeErrorKind::PolymorphicCast {
                target_type: format!("{target_type}"),
            });

            // Add context pointing to the expression with polymorphic type
            if let Some(expr_span) = self.typed_ann.span_of(analyzed_expr) {
                err.context
                    .push(crate::diagnostics::context::Context::InferredHere {
                        type_name: format!("{source_type}"),
                        span: expr_span,
                    });
            }

            return Err(err);
        }

        // Validate the cast using casting library
        casting::validate_cast(source_type, target_type).map_err(|err| {
            self.type_error(TypeErrorKind::InvalidCast {
                from: format!("{source_type}"),
                to: format!("{target_type}"),
                reason: err.to_string(),
            })
        })?;

        Ok(self.alloc(
            target_type,
            ExprInner::Cast {
                expr: analyzed_expr,
            },
        ))
    }

    fn analyze_lambda(
        &mut self,
        params: &'arena [&'arena str],
        body: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let ty = self.type_manager;

        // Create shared recording vector and push recording scope
        let recorded = Rc::new(RefCell::new(BTreeSet::new()));
        let recording_scope = scope_stack::RecordingScope::new(recorded.clone());
        self.scope_stack.push(recording_scope);

        // Push incomplete scope with parameter names
        self.scope_stack.push(
            scope_stack::IncompleteScope::new(self.arena, params).map_err(|e| {
                self.type_error(TypeErrorKind::DuplicateParameter {
                    name: e.0,
                })
            })?,
        );

        // Create fresh type variables for parameters
        let mut param_types: Vec<&'types Type<'types>> = Vec::new();
        for param in params {
            let param_ty = ty.fresh_type_var();

            // Wrap in monomorphic TypeScheme (lambda parameters are not polymorphic)
            let empty_quantified = self.type_manager.alloc_u16_slice(&[]);
            let scheme = TypeScheme::new(empty_quantified, param_ty);

            self.scope_stack
                .bind_in_current(param, scheme)
                .map_err(|e| self.internal_error(format!("Failed to bind parameter: {e:?}")))?;
            param_types.push(param_ty);
        }

        // Push parameter type vars to env_vars_stack so they won't be generalized
        // These should NOT be generalized in nested where clauses
        let mut param_env_vars = hashbrown::HashSet::new();
        for param_ty in &param_types {
            param_env_vars.extend(self.unification.free_type_vars(*param_ty));
        }
        self.env_vars_stack.push(param_env_vars);

        let body = self.analyze(body)?;

        // Pop environment variables
        self.env_vars_stack.pop();

        // Pop parameter scope
        self.scope_stack
            .pop()
            .map_err(|e| self.internal_error(format!("Failed to pop scope: {e:?}")))?;

        // Pop recording scope (we don't need the returned value)
        self.scope_stack
            .pop()
            .map_err(|e| self.internal_error(format!("Failed to pop recording scope: {e:?}")))?;

        // Get recorded names from our Rc clone
        let captures = self
            .arena
            .alloc_slice_fill_iter(recorded.borrow().iter().copied());

        let result_ty = ty.function(
            self.arena.alloc_slice_fill_iter(
                param_types.into_iter().map(|t| self.unification.resolve(t)),
            ),
            body.0,
        );
        Ok(self.alloc(
            result_ty,
            ExprInner::Lambda {
                params: self.arena.alloc_slice_copy(params),
                body,
                captures,
            },
        ))
    }

    fn analyze_if(
        &mut self,
        cond: &'arena parser::Expr<'arena>,
        then_branch: &'arena parser::Expr<'arena>,
        else_branch: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let cond = self.analyze(cond)?;
        let then_branch = self.analyze(then_branch)?;
        let else_branch = self.analyze(else_branch)?;

        // Condition must be a boolean - use condition's span for error
        self.expect_type_to_be(
            cond,
            cond.0,
            self.type_manager.bool(),
            "Condition of 'if' must be Bool",
        )?;

        // Both branches must have the same type - point to else branch if mismatch
        let result_ty = self.expect_types_match(else_branch, else_branch.0, then_branch.0)?;

        Ok(self.alloc(
            result_ty,
            ExprInner::If {
                cond,
                then_branch,
                else_branch,
            },
        ))
    }

    fn analyze_where(
        &mut self,
        expr: &'arena parser::Expr<'arena>,
        bindings: &'arena [(&'arena str, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Extract binding names
        let names: Vec<&'arena str> = bindings.iter().map(|(name, _)| *name).collect();

        // Push incomplete scope with all binding names
        self.scope_stack.push(
            scope_stack::IncompleteScope::new(self.arena, &names).map_err(|e| {
                self.type_error(TypeErrorKind::DuplicateBinding {
                    name: e.0,
                })
            })?,
        );

        // Analyze and bind each expression sequentially
        let mut analyzed_bindings: Vec<(&'arena str, &'arena mut Expr<'types, 'arena>)> =
            Vec::new();
        for (name, value_expr) in bindings {
            let analyzed = self.analyze(value_expr)?;

            // Generalize the type to a type scheme
            // Use current environment variables to prevent generalizing over lambda parameters
            let env_vars = self.get_env_vars();
            let mut scheme = self.unification.generalize(analyzed.0, &env_vars);

            // Track polymorphic lambdas for instantiation tracking
            // Store the lambda pointer directly in the TypeScheme
            if !scheme.is_monomorphic() && matches!(analyzed.1, ExprInner::Lambda { .. }) {
                let lambda_ptr = analyzed.as_ptr();
                scheme.lambda_expr = Some(lambda_ptr);
                self.polymorphic_lambdas.insert(lambda_ptr, scheme);
            }

            self.scope_stack
                .bind_in_current(name, scheme)
                .map_err(|e| self.internal_error(format!("Failed to bind in where: {e:?}")))?;
            analyzed_bindings.push((*name, analyzed));
        }

        let expr_typed = self.analyze(expr)?;

        self.scope_stack
            .pop()
            .map_err(|e| self.internal_error(format!("Failed to pop scope: {e:?}")))?;

        Ok(self.alloc(
            expr_typed.0,
            ExprInner::Where {
                expr: expr_typed,
                bindings: self
                    .arena
                    .alloc_slice_fill_iter(analyzed_bindings.into_iter().map(|(k, v)| (k, &*v))),
            },
        ))
    }

    fn analyze_otherwise(
        &mut self,
        primary: &'arena parser::Expr<'arena>,
        fallback: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let primary = self.analyze(primary)?;
        let fallback = self.analyze(fallback)?;

        // Both expressions must have the same type - point to fallback if mismatch
        let result_ty = self.expect_types_match(fallback, fallback.0, primary.0)?;

        Ok(self.alloc(result_ty, ExprInner::Otherwise { primary, fallback }))
    }

    fn analyze_option(
        &mut self,
        inner: Option<&'arena parser::Expr<'arena>>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        if let Some(expr) = inner {
            // Analyze inner expression
            let typed_expr = self.analyze(expr)?;
            let inner_ty = typed_expr.0;

            // Wrap in Option type
            let result_ty = self.type_manager.option(inner_ty);

            Ok(self.alloc(
                result_ty,
                ExprInner::Option {
                    inner: Some(typed_expr),
                },
            ))
        } else {
            // Polymorphic none: Option[fresh type variable]
            let fresh_var = self.type_manager.fresh_type_var();
            let result_ty = self.type_manager.option(fresh_var);

            Ok(self.alloc(result_ty, ExprInner::Option { inner: None }))
        }
    }

    fn analyze_match(
        &mut self,
        expr: &'arena parser::Expr<'arena>,
        arms: &'arena [parser::MatchArm<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Analyze the matched expression
        let typed_expr = self.analyze(expr)?;
        let matched_ty = typed_expr.0;

        if arms.is_empty() {
            return self.error(TypeErrorKind::UnsupportedFeature {
                feature: "match expression with no arms".to_string(),
                suggestion: "Add at least one pattern arm to the match expression".to_string(),
            });
        }

        // Analyze each arm
        let mut typed_arms = Vec::new();
        let mut result_ty: Option<&'types Type<'types>> = None;

        for arm in arms {
            // First pass: collect all variable names from the pattern
            let mut pattern_vars = Vec::new();
            self.collect_pattern_vars(arm.pattern, &mut pattern_vars);

            // Create a new scope with the pattern variables pre-declared
            self.scope_stack.push(
                scope_stack::IncompleteScope::new(self.arena, &pattern_vars).map_err(|e| {
                    self.type_error(TypeErrorKind::DuplicateBinding {
                        name: e.0,
                    })
                })?,
            );

            // Analyze pattern and bind variables
            let typed_pattern = self.analyze_pattern(arm.pattern, matched_ty)?;

            // Analyze the arm body with pattern bindings in scope
            let typed_body = self.analyze(arm.body)?;

            // Pop pattern binding scope
            self.scope_stack
                .pop()
                .map_err(|e| self.internal_error(format!("Failed to pop scope: {e:?}")))?;

            // Check that all arms have the same result type
            match result_ty {
                None => {
                    // First arm sets the expected result type
                    result_ty = Some(typed_body.0);
                }
                Some(expected_ty) => {
                    // Subsequent arms must match the first arm's type - point to mismatched arm body
                    self.expect_types_match(typed_body, typed_body.0, expected_ty)?;
                }
            }

            // Allocate pattern_vars in the arena for the typed arm
            let vars = self.arena.alloc_slice_copy(&pattern_vars);

            typed_arms.push(typed_expr::TypedMatchArm {
                pattern: typed_pattern,
                body: typed_body,
                vars,
            });
        }

        let result_ty = result_ty.unwrap(); // Safe because we checked arms is not empty

        // Check exhaustiveness for Bool and Option types
        self.check_exhaustiveness(matched_ty, &typed_arms)?;

        Ok(self.alloc(
            result_ty,
            ExprInner::Match {
                expr: typed_expr,
                arms: self.arena.alloc_slice_fill_iter(typed_arms),
            },
        ))
    }

    /// Check if a pattern is a catch-all (matches any value)
    fn is_catch_all_pattern(pattern: &typed_expr::TypedPattern<'_, '_>) -> bool {
        match pattern {
            typed_expr::TypedPattern::Wildcard | typed_expr::TypedPattern::Var(_) => true,
            // For nested patterns, recursively check if inner pattern is catch-all
            typed_expr::TypedPattern::Some(inner) => Self::is_catch_all_pattern(inner),
            _ => false,
        }
    }

    /// Check if the patterns in a match are exhaustive for Bool and Option types.
    /// For other types, we don't check exhaustiveness (would require wildcard).
    fn check_exhaustiveness(
        &self,
        matched_ty: &'types Type<'types>,
        arms: &[typed_expr::TypedMatchArm<'types, 'arena>],
    ) -> Result<(), TypeError> {
        use crate::types::traits::TypeKind;

        // Check if there's a wildcard or variable pattern (catches all)
        let has_catch_all = arms
            .iter()
            .any(|arm| Self::is_catch_all_pattern(arm.pattern));

        if has_catch_all {
            // Wildcard/variable pattern covers all cases
            return Ok(());
        }

        // Resolve through unification before checking exhaustiveness
        let resolved_ty = self.unification.fully_resolve(matched_ty);

        // Check exhaustiveness based on resolved type
        match resolved_ty.view() {
            TypeKind::Bool => {
                // For Bool, we need both true and false patterns
                let has_true = arms.iter().any(|arm| {
                    if let typed_expr::TypedPattern::Literal(value) = arm.pattern {
                        value.as_bool().unwrap_or(false)
                    } else {
                        false
                    }
                });
                let has_false = arms.iter().any(|arm| {
                    if let typed_expr::TypedPattern::Literal(value) = arm.pattern {
                        !value.as_bool().unwrap_or(true)
                    } else {
                        false
                    }
                });

                let mut missing = Vec::new();
                if !has_true {
                    missing.push("true".to_string());
                }
                if !has_false {
                    missing.push("false".to_string());
                }

                if !missing.is_empty() {
                    return self.error(TypeErrorKind::NonExhaustivePatterns {
                        ty: resolved_ty.to_string(),
                        missing_cases: missing,
                    });
                }
            }
            TypeKind::Option(_inner_ty) => {
                // For Option[T], we need both some and none patterns
                // The some pattern must have a catch-all inner pattern (some _ or some x)
                let has_some = arms.iter().any(|arm| {
                    matches!(arm.pattern, typed_expr::TypedPattern::Some(inner)
                        if Self::is_catch_all_pattern(inner))
                });
                let has_none = arms
                    .iter()
                    .any(|arm| matches!(arm.pattern, typed_expr::TypedPattern::None));

                let mut missing = Vec::new();
                if !has_some {
                    missing.push("some _".to_string());
                }
                if !has_none {
                    missing.push("none".to_string());
                }

                if !missing.is_empty() {
                    return self.error(TypeErrorKind::NonExhaustivePatterns {
                        ty: resolved_ty.to_string(),
                        missing_cases: missing,
                    });
                }
            }
            _ => {
                // For other types (Int, Str, etc.), we don't check exhaustiveness
                // They would require a wildcard/variable pattern
            }
        }

        Ok(())
    }

    fn collect_pattern_vars(
        &self,
        pattern: &'arena parser::Pattern<'arena>,
        vars: &mut Vec<&'arena str>,
    ) {
        match pattern {
            parser::Pattern::Wildcard => {}
            parser::Pattern::Var(name) => vars.push(name),
            parser::Pattern::Literal(_) => {}
            parser::Pattern::Some(inner) => self.collect_pattern_vars(inner, vars),
            parser::Pattern::None => {}
        }
    }

    fn analyze_pattern(
        &mut self,
        pattern: &'arena parser::Pattern<'arena>,
        expected_ty: &'types Type<'types>,
    ) -> Result<&'arena typed_expr::TypedPattern<'types, 'arena>, TypeError> {
        match pattern {
            parser::Pattern::Wildcard => {
                // Wildcard matches anything, no bindings
                Ok(self.arena.alloc(typed_expr::TypedPattern::Wildcard))
            }

            parser::Pattern::Var(name) => {
                // Variable pattern binds the matched value to a name
                // Create a monomorphic type scheme (no quantified variables)
                let type_scheme = TypeScheme::new(&[], expected_ty);

                // Add binding to current scope
                self.scope_stack
                    .bind_in_current(name, type_scheme)
                    .map_err(|_| {
                        self.type_error(TypeErrorKind::DuplicateBinding {
                            name: name.to_string(),
                        })
                    })?;

                Ok(self.arena.alloc(typed_expr::TypedPattern::Var(name)))
            }

            parser::Pattern::Literal(lit) => {
                // Convert literal to Value for pattern matching
                let value = match lit {
                    parser::Literal::Int {
                        value,
                        suffix: None,
                    } => Value::int(self.type_manager, *value),
                    parser::Literal::Float {
                        value,
                        suffix: None,
                    } => Value::float(self.type_manager, *value),
                    parser::Literal::Bool(b) => Value::bool(self.type_manager, *b),
                    parser::Literal::Str(s) => Value::str(self.arena, self.type_manager.str(), s),
                    parser::Literal::Bytes(b) => {
                        Value::bytes(self.arena, self.type_manager.bytes(), b)
                    }
                    _ => {
                        return self.error(TypeErrorKind::UnsupportedFeature {
                            feature: "Suffixes in pattern literals".to_string(),
                            suggestion: "Remove the suffix from the literal in the pattern"
                                .to_string(),
                        });
                    }
                };

                let literal_ty = value.ty;

                // Pattern literal type must match the type of the matched expression
                self.unification
                    .unifies_to(literal_ty, expected_ty)
                    .map_err(|err| match err {
                        crate::types::unification::Error::TypeMismatch { left, right } => {
                            TypeError::new(
                                TypeErrorKind::TypeMismatch {
                                    expected: right,
                                    found: left,
                                    context: Some(
                                        "Pattern literal must match the type of the matched expression"
                                            .to_string(),
                                    ),
                                },
                                self.get_source(),
                                self.get_span(),
                            )
                        }
                        other => TypeError::from_unification_error(
                            other,
                            self.get_span(),
                            self.get_source(),
                        ),
                    })?;

                Ok(self.arena.alloc(typed_expr::TypedPattern::Literal(value)))
            }

            parser::Pattern::Some(inner_pattern) => {
                // Create a fresh type variable for the inner type
                let inner_ty_var = self.type_manager.fresh_type_var();

                // Unify expected_ty with Option[inner_ty_var]
                let option_ty = self.type_manager.option(inner_ty_var);
                self.unification
                    .unifies_to(expected_ty, option_ty)
                    .map_err(|_e| {
                        self.type_error(TypeErrorKind::TypeMismatch {
                            expected: "Option[T]".to_string(),
                            found: format!("{expected_ty}"),
                            context: Some("'some' pattern requires an Option type".to_string()),
                        })
                    })?;

                // Get the resolved inner type after unification
                let resolved_inner_ty = self.unification.fully_resolve(inner_ty_var);

                // Recursively analyze the inner pattern
                let typed_inner = self.analyze_pattern(inner_pattern, resolved_inner_ty)?;
                Ok(self
                    .arena
                    .alloc(typed_expr::TypedPattern::Some(typed_inner)))
            }

            parser::Pattern::None => {
                // Create a fresh type variable for the inner type
                let inner_ty_var = self.type_manager.fresh_type_var();

                // Unify expected_ty with Option[inner_ty_var]
                let option_ty = self.type_manager.option(inner_ty_var);
                self.unification
                    .unifies_to(expected_ty, option_ty)
                    .map_err(|_e| {
                        self.type_error(TypeErrorKind::TypeMismatch {
                            expected: "Option[T]".to_string(),
                            found: format!("{expected_ty}"),
                            context: Some("'none' pattern requires an Option type".to_string()),
                        })
                    })?;

                Ok(self.arena.alloc(typed_expr::TypedPattern::None))
            }
        }
    }

    fn analyze_record(
        &mut self,
        items: &'arena [(&'arena str, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let mut fields: Vec<(&'arena str, &'arena mut Expr<'types, 'arena>)> = Vec::new();
        let mut field_types: Vec<(&'arena str, &'types Type<'types>)> = Vec::new();

        for (name, value_expr) in items {
            let value = self.analyze(value_expr)?;
            field_types.push((*name, value.0));
            fields.push((*name, value));
        }

        let record_ty = self.type_manager.record(field_types);

        Ok(self.alloc(
            record_ty,
            ExprInner::Record {
                fields: self
                    .arena
                    .alloc_slice_fill_iter(fields.into_iter().map(|(k, v)| (k, &*v))),
            },
        ))
    }

    fn analyze_map(
        &mut self,
        items: &'arena [(&'arena parser::Expr<'arena>, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        if items.is_empty() {
            // Empty map - use fresh type variables for key and value
            let key_ty = self.type_manager.fresh_type_var();
            let value_ty = self.type_manager.fresh_type_var();
            let map_ty = self.type_manager.map(key_ty, value_ty);

            return Ok(self.alloc(map_ty, ExprInner::Map { elements: &[] }));
        }

        // Analyze all keys and values
        let mut entries: Vec<(
            &'arena mut Expr<'types, 'arena>,
            &'arena mut Expr<'types, 'arena>,
        )> = Vec::new();

        for (key_expr, value_expr) in items {
            let key = self.analyze(key_expr)?;
            let value = self.analyze(value_expr)?;
            entries.push((key, value));
        }

        // Allocate in arena first, then do type checks on arena-allocated slice
        let entries_slice: &'arena [(
            &'arena Expr<'types, 'arena>,
            &'arena Expr<'types, 'arena>,
        )] = self
            .arena
            .alloc_slice_fill_iter(entries.into_iter().map(|(k, v)| (&*k, &*v)));

        // All keys must have the same type, all values must have the same type
        let key_ty = entries_slice[0].0.0;
        let value_ty = entries_slice[0].1.0;

        for &(key, value) in entries_slice.iter().skip(1) {
            self.expect_types_match(key, key.0, key_ty)?;
            self.expect_types_match(value, value.0, value_ty)?;
        }

        // Map keys must be hashable
        let span = self.get_span();
        self.type_class_resolver
            .add_hashable_constraint(key_ty, span);

        let map_ty = self.type_manager.map(key_ty, value_ty);

        Ok(self.alloc(
            map_ty,
            ExprInner::Map {
                elements: entries_slice,
            },
        ))
    }

    fn analyze_array(
        &mut self,
        exprs: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        if exprs.is_empty() {
            // Empty array - use a fresh type variable for the element type
            let element_ty = self.type_manager.fresh_type_var();
            let array_ty = self.type_manager.array(element_ty);

            return Ok(self.alloc(array_ty, ExprInner::Array { elements: &[] }));
        }

        // Analyze all elements
        let elements: Vec<&'arena mut Expr<'types, 'arena>> = exprs
            .iter()
            .map(|e| self.analyze(e))
            .collect::<Result<_, _>>()?;

        // Allocate in arena first, then do type checks on arena-allocated slice
        let elements_slice: &'arena [&'arena Expr<'types, 'arena>] = self
            .arena
            .alloc_slice_fill_iter(elements.into_iter().map(|e| &*e));

        // All elements must have the same type - point to mismatching element
        let element_ty = elements_slice[0].0;
        for &elem in elements_slice.iter().skip(1) {
            self.expect_types_match(elem, elem.0, element_ty)?;
        }

        let array_ty = self.type_manager.array(element_ty);

        Ok(self.alloc(
            array_ty,
            ExprInner::Array {
                elements: elements_slice,
            },
        ))
    }

    fn analyze_format_str(
        &mut self,
        _strs: &'arena [&'arena str],
        exprs: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Analyze all interpolated expressions
        let exprs_typed: Vec<&'arena mut Expr<'types, 'arena>> = exprs
            .iter()
            .map(|e| self.analyze(e))
            .collect::<Result<_, _>>()?;

        // Check that all expressions are formattable (not functions)
        for expr in &exprs_typed {
            if matches!(expr.type_view(), TypeKind::Function { .. }) {
                return self.error(TypeErrorKind::NotFormattable {
                    ty: format!("{}", expr.0),
                });
            }
        }

        Ok(self.alloc(
            self.type_manager.str(),
            ExprInner::FormatStr {
                strs: _strs,
                exprs: self
                    .arena
                    .alloc_slice_fill_iter(exprs_typed.into_iter().map(|e| &*e)),
            },
        ))
    }

    fn analyze_literal(
        &mut self,
        literal: &parser::Literal<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        match literal {
            parser::Literal::Int { value, suffix } => {
                if let Some(_suffix) = suffix {
                    return self.error(TypeErrorKind::UnsupportedFeature {
                        feature: "Integer suffixes are not yet supported".to_string(),
                        suggestion: "In the future, suffixes will support units of measurement (e.g., 10`MB`, 5`seconds`)".to_string(),
                    });
                }
                let ty = self.type_manager.int();
                let value = Value::int(self.type_manager, *value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
            parser::Literal::Float { value, suffix } => {
                if let Some(_suffix) = suffix {
                    return self.error(TypeErrorKind::UnsupportedFeature {
                        feature: "Float suffixes are not yet supported".to_string(),
                        suggestion: "In the future, suffixes will support units of measurement (e.g., 3.14`meters`, 2.5`kg`)".to_string(),
                    });
                }
                let ty = self.type_manager.float();
                let value = Value::float(self.type_manager, *value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
            parser::Literal::Bool(value) => {
                let ty = self.type_manager.bool();
                let value = Value::bool(self.type_manager, *value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
            parser::Literal::Str(value) => {
                let ty = self.type_manager.str();
                let value = Value::str(self.arena, ty, value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
            parser::Literal::Bytes(value) => {
                let ty = self.type_manager.bytes();
                let value = Value::bytes(self.arena, ty, value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
        }
    }

    fn analyze_ident(
        &mut self,
        ident: &'arena str,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Look up the identifier in the scope stack
        if let Some(scheme) = self.scope_stack.lookup(ident) {
            // Instantiate the type scheme with fresh type variables
            // Pass the current span as the instantiation site for constraint tracking
            let instantiation_span = self.get_span();
            let (ty, inst_subst) = self.unification.instantiate_with_subst(
                scheme,
                &mut self.type_class_resolver,
                instantiation_span,
            );

            // If this identifier refers to a polymorphic lambda, record the instantiation
            // The lambda pointer is stored in the TypeScheme itself
            if let Some(lambda_ptr) = scheme.lambda_expr
                && !inst_subst.is_empty()
            {
                // Build reverse mapping: fresh var ID -> generalized var ID
                let mut fresh_to_gen = hashbrown::HashMap::new();
                for (gen_var_id, fresh_ty) in &inst_subst {
                    // Extract fresh var ID from the type (should be TypeVar)
                    use crate::types::traits::{TypeKind, TypeView};
                    if let TypeKind::TypeVar(fresh_var_id) = fresh_ty.view() {
                        fresh_to_gen.insert(fresh_var_id, *gen_var_id);
                    }
                }

                // Add to pending instantiations
                self.pending_instantiations
                    .entry(lambda_ptr)
                    .or_insert_with(Vec::new)
                    .push(fresh_to_gen);
            }

            return Ok(self.alloc(ty, ExprInner::Ident(ident)));
        }

        self.error(TypeErrorKind::UnboundVariable {
            name: ident.to_string(),
        })
    }

    /// Build final lambda instantiation substitutions by resolving fresh vars to concrete types.
    /// This is called after `finalize_constraints()` so all type variables are resolved.
    fn build_lambda_instantiations(
        &self,
        arena: &'arena Bump,
    ) -> hashbrown::HashMap<
        *const Expr<'types, 'arena>,
        LambdaInstantiations<'types, 'arena>,
        DefaultHashBuilder,
        &'arena bumpalo::Bump,
    > {
        let mut result = hashbrown::HashMap::new_in(arena);

        for (lambda_ptr, inst_list) in &self.pending_instantiations {
            // Get the type scheme to access quantified variables
            let scheme = self
                .polymorphic_lambdas
                .get(lambda_ptr)
                .expect("Lambda should be tracked");

            // Find which type classes constrain the lambda's quantified variables
            let quantified_vars: alloc::vec::Vec<u16> = scheme.quantified.to_vec();
            let type_classes = self
                .type_class_resolver
                .type_classes_for_vars(&quantified_vars, &self.unification);

            let mut substitutions = alloc::vec::Vec::new();

            for fresh_to_gen_map in inst_list {
                // Build substitution from generalized var ID to concrete type
                let mut substitution = hashbrown::HashMap::new_in(arena);

                for (fresh_var_id, gen_var_id) in fresh_to_gen_map {
                    // Get the type variable and resolve it to its concrete type
                    let fresh_var = self.type_manager.type_var(*fresh_var_id);
                    let concrete_ty = self.unification.fully_resolve(fresh_var);

                    // Map: generalized var ID -> concrete type
                    substitution.insert(*gen_var_id, concrete_ty);
                }

                substitutions.push(substitution);
            }

            result.insert(
                *lambda_ptr,
                LambdaInstantiations {
                    substitutions,
                    type_classes,
                },
            );
        }

        result
    }

    /// Remap `lambda_instantiations` keys from old expression pointers to new ones.
    /// This is necessary because `resolve_expr_types` allocates new Expr nodes with
    /// different pointers, but `lambda_instantiations` uses pointers as keys.
    fn remap_lambda_instantiations(
        old_instantiations: hashbrown::HashMap<
            *const Expr<'types, 'arena>,
            LambdaInstantiations<'types, 'arena>,
            DefaultHashBuilder,
            &'arena bumpalo::Bump,
        >,
        ptr_remap: &hashbrown::HashMap<*const Expr<'types, 'arena>, *const Expr<'types, 'arena>>,
        arena: &'arena Bump,
    ) -> hashbrown::HashMap<
        *const Expr<'types, 'arena>,
        LambdaInstantiations<'types, 'arena>,
        DefaultHashBuilder,
        &'arena bumpalo::Bump,
    > {
        let mut result = hashbrown::HashMap::new_in(arena);

        for (old_ptr, instantiations) in old_instantiations {
            // Look up the new pointer for this lambda
            if let Some(&new_ptr) = ptr_remap.get(&old_ptr) {
                // Insert using the new pointer as key
                result.insert(new_ptr, instantiations);
            } else {
                // This shouldn't happen if resolve_expr_types visited all nodes
                // but we'll handle it gracefully by keeping the old pointer
                result.insert(old_ptr, instantiations);
            }
        }

        result
    }

    /// Recursively resolve all type variables in an expression tree.
    /// This is called after `finalize_constraints()` to replace type variables
    /// with their fully resolved types (e.g., _5 → Str).
    ///
    /// The `ptr_remap` parameter tracks old→new pointer mappings so that
    /// `lambda_instantiations` keys can be remapped after resolution.
    ///
    /// Note: Type variables that aren't unified (e.g., generalized lambda body vars)
    /// will remain unchanged, which is the correct behavior.
    fn resolve_expr_types(
        &self,
        expr: &'arena Expr<'types, 'arena>,
        ptr_remap: &mut hashbrown::HashMap<
            *const Expr<'types, 'arena>,
            *const Expr<'types, 'arena>,
        >,
    ) -> &'arena Expr<'types, 'arena> {
        let resolved_ty = self.unification.fully_resolve(expr.0);
        let old_ptr = core::ptr::from_ref(expr);
        let old_span = self.typed_ann.span_of(expr);

        let resolved_inner = match &expr.1 {
            ExprInner::Binary { op, left, right } => ExprInner::Binary {
                op: *op,
                left: self.resolve_expr_types(left, ptr_remap),
                right: self.resolve_expr_types(right, ptr_remap),
            },
            ExprInner::Boolean { op, left, right } => ExprInner::Boolean {
                op: *op,
                left: self.resolve_expr_types(left, ptr_remap),
                right: self.resolve_expr_types(right, ptr_remap),
            },
            ExprInner::Comparison { op, left, right } => ExprInner::Comparison {
                op: *op,
                left: self.resolve_expr_types(left, ptr_remap),
                right: self.resolve_expr_types(right, ptr_remap),
            },
            ExprInner::Unary { op, expr: inner } => ExprInner::Unary {
                op: *op,
                expr: self.resolve_expr_types(inner, ptr_remap),
            },
            ExprInner::Call { callable, args } => {
                let resolved_callable = self.resolve_expr_types(callable, ptr_remap);
                let resolved_args: Vec<_> = args
                    .iter()
                    .map(|arg| self.resolve_expr_types(arg, ptr_remap))
                    .collect();
                ExprInner::Call {
                    callable: resolved_callable,
                    args: self.arena.alloc_slice_fill_iter(resolved_args),
                }
            }
            ExprInner::Index { value, index } => ExprInner::Index {
                value: self.resolve_expr_types(value, ptr_remap),
                index: self.resolve_expr_types(index, ptr_remap),
            },
            ExprInner::Field { value, field } => ExprInner::Field {
                value: self.resolve_expr_types(value, ptr_remap),
                field,
            },
            ExprInner::Cast { expr: inner } => ExprInner::Cast {
                expr: self.resolve_expr_types(inner, ptr_remap),
            },
            ExprInner::Lambda {
                params,
                body,
                captures,
            } => ExprInner::Lambda {
                params,
                body: self.resolve_expr_types(body, ptr_remap),
                captures,
            },
            ExprInner::If {
                cond,
                then_branch,
                else_branch,
            } => ExprInner::If {
                cond: self.resolve_expr_types(cond, ptr_remap),
                then_branch: self.resolve_expr_types(then_branch, ptr_remap),
                else_branch: self.resolve_expr_types(else_branch, ptr_remap),
            },
            ExprInner::Where {
                expr: inner,
                bindings,
            } => {
                let resolved_expr = self.resolve_expr_types(inner, ptr_remap);
                let resolved_bindings: Vec<_> = bindings
                    .iter()
                    .map(|(name, value)| (*name, self.resolve_expr_types(value, ptr_remap)))
                    .collect();
                ExprInner::Where {
                    expr: resolved_expr,
                    bindings: self.arena.alloc_slice_fill_iter(resolved_bindings),
                }
            }
            ExprInner::Otherwise { primary, fallback } => ExprInner::Otherwise {
                primary: self.resolve_expr_types(primary, ptr_remap),
                fallback: self.resolve_expr_types(fallback, ptr_remap),
            },
            ExprInner::Option { inner } => ExprInner::Option {
                inner: inner.map(|expr| self.resolve_expr_types(expr, ptr_remap)),
            },
            ExprInner::Match { expr, arms } => {
                // Resolve types in matched expression
                let resolved_expr = self.resolve_expr_types(expr, ptr_remap);

                // Resolve types in each arm (vars don't need resolution - they're just names)
                let resolved_arms: Vec<_> = arms
                    .iter()
                    .map(|arm| typed_expr::TypedMatchArm {
                        pattern: self.resolve_pattern_types(arm.pattern, ptr_remap),
                        body: self.resolve_expr_types(arm.body, ptr_remap),
                        vars: arm.vars,
                    })
                    .collect();

                ExprInner::Match {
                    expr: resolved_expr,
                    arms: self.arena.alloc_slice_fill_iter(resolved_arms),
                }
            }
            ExprInner::Record { fields } => {
                let resolved_fields: Vec<_> = fields
                    .iter()
                    .map(|(name, value)| (*name, self.resolve_expr_types(value, ptr_remap)))
                    .collect();
                ExprInner::Record {
                    fields: self.arena.alloc_slice_fill_iter(resolved_fields),
                }
            }
            ExprInner::Map { elements } => {
                let resolved_elements: Vec<_> = elements
                    .iter()
                    .map(|(key, value)| {
                        (
                            self.resolve_expr_types(key, ptr_remap),
                            self.resolve_expr_types(value, ptr_remap),
                        )
                    })
                    .collect();
                ExprInner::Map {
                    elements: self.arena.alloc_slice_fill_iter(resolved_elements),
                }
            }
            ExprInner::Array { elements } => {
                let resolved_elements: Vec<_> = elements
                    .iter()
                    .map(|elem| self.resolve_expr_types(elem, ptr_remap))
                    .collect();
                ExprInner::Array {
                    elements: self.arena.alloc_slice_fill_iter(resolved_elements),
                }
            }
            ExprInner::FormatStr { strs, exprs } => {
                let resolved_exprs: Vec<_> = exprs
                    .iter()
                    .map(|expr| self.resolve_expr_types(expr, ptr_remap))
                    .collect();
                ExprInner::FormatStr {
                    strs,
                    exprs: self.arena.alloc_slice_fill_iter(resolved_exprs),
                }
            }
            ExprInner::Constant(value) => ExprInner::Constant(*value),
            ExprInner::Ident(name) => ExprInner::Ident(name),
        };

        // Allocate new expression with resolved type
        let new_expr = self.arena.alloc(Expr(resolved_ty, resolved_inner));

        // Copy span from old expression to new expression
        if let Some(span) = old_span {
            self.typed_ann.add_span(new_expr, span);
        }

        // Record old→new pointer mapping
        ptr_remap.insert(old_ptr, core::ptr::from_ref(new_expr));

        new_expr
    }

    fn resolve_pattern_types(
        &self,
        pattern: &'arena typed_expr::TypedPattern<'types, 'arena>,
        _ptr_remap: &mut hashbrown::HashMap<
            *const Expr<'types, 'arena>,
            *const Expr<'types, 'arena>,
        >,
    ) -> &'arena typed_expr::TypedPattern<'types, 'arena> {
        match pattern {
            typed_expr::TypedPattern::Wildcard => {
                self.arena.alloc(typed_expr::TypedPattern::Wildcard)
            }
            typed_expr::TypedPattern::Var(name) => {
                self.arena.alloc(typed_expr::TypedPattern::Var(name))
            }
            typed_expr::TypedPattern::Literal(value) => {
                // Literals have concrete types, no type variables to resolve
                self.arena.alloc(typed_expr::TypedPattern::Literal(*value))
            }
            typed_expr::TypedPattern::Some(inner) => {
                let resolved_inner = self.resolve_pattern_types(inner, _ptr_remap);
                self.arena
                    .alloc(typed_expr::TypedPattern::Some(resolved_inner))
            }
            typed_expr::TypedPattern::None => self.arena.alloc(typed_expr::TypedPattern::None),
        }
    }
}
