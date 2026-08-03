//! Core evaluation logic.

use alloc::string::ToString;

use bumpalo::Bump;

use crate::Vec;
use crate::analyzer::typed_expr::{Expr, ExprInner, TypedExpr, TypedPattern};
use crate::evaluator::InternalError::*;
use crate::evaluator::ResourceExceededError::*;
use crate::evaluator::RuntimeError::*;
use crate::evaluator::{EvaluatorOptions, ExecutionError, ExecutionErrorKind};
use crate::parser::{BoolOp, ComparisonOp};
use crate::scope_stack::{self, ScopeStack};
use crate::types::Type;
use crate::types::manager::TypeManager;
use crate::types::unification::Unification;
use crate::values::EvalLambda;
use crate::values::dynamic::Value;
use crate::values::function::FfiContext;

/// Evaluator for type-checked expressions.
pub struct Evaluator<'types, 'arena> {
    options: EvaluatorOptions,
    arena: &'arena Bump,
    type_manager: &'types TypeManager<'types>,
    /// The typed expression being evaluated (used for error context).
    expr: &'arena TypedExpr<'types, 'arena>,
    scope_stack: ScopeStack<'arena, Value<'types, 'arena>>,
    depth: usize,
    /// Type unification for monomorphizing polymorphic lambda bodies.
    /// When evaluating a polymorphic lambda, this contains the unification
    /// of the lambda's parameter types with the concrete argument types.
    monomorphism: Option<Unification<'types, &'types TypeManager<'types>>>,
}

impl<'types, 'arena> Evaluator<'types, 'arena> {
    /// Create a new evaluator with the given options.
    pub fn new(
        options: EvaluatorOptions,
        arena: &'arena Bump,
        type_manager: &'types TypeManager<'types>,
        expr: &'arena TypedExpr<'types, 'arena>,
        globals: &[(&'arena str, Value<'types, 'arena>)],
        variables: &[(&'arena str, Value<'types, 'arena>)],
    ) -> Self {
        let mut scope_stack = ScopeStack::new();

        // Push globals scope (constants, packages, functions)
        if !globals.is_empty() {
            let bindings = arena.alloc_slice_copy(globals);
            scope_stack.push(scope_stack::CompleteScope::from_sorted(bindings));
        }

        // Push variables scope (client-provided runtime variables)
        if !variables.is_empty() {
            let bindings = arena.alloc_slice_copy(variables);
            scope_stack.push(scope_stack::CompleteScope::from_sorted(bindings));
        }

        Self {
            options,
            arena,
            type_manager,
            expr,
            scope_stack,
            depth: 0,
            monomorphism: None,
        }
    }

    /// Push a scope onto the scope stack.
    ///
    /// This is used internally by lambda functions to push captures and parameters.
    pub fn push_scope<S: scope_stack::Scope<'arena, Value<'types, 'arena>> + 'arena>(
        &mut self,
        scope: S,
    ) {
        self.scope_stack.push(scope);
    }

    /// Set the monomorphization unification for this evaluator.
    /// This should be called before evaluating polymorphic lambda bodies.
    pub fn set_monomorphism(
        &mut self,
        unification: Unification<'types, &'types TypeManager<'types>>,
    ) {
        self.monomorphism = Some(unification);
    }

    /// Resolve a type by applying monomorphization if present.
    /// This replaces type variables with concrete types when evaluating
    /// polymorphic lambda bodies.
    fn resolve_type(&self, ty: &'types Type<'types>) -> &'types Type<'types> {
        self.monomorphism
            .as_ref()
            .map(|m| m.fully_resolve(ty))
            .unwrap_or(ty)
    }

    fn error(
        &self,
        expr: &'arena Expr<'types, 'arena>,
        error: ExecutionErrorKind,
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        Err(self.add_error_context(expr, error))
    }

    fn add_error_context(
        &self,
        expr: &'arena Expr<'types, 'arena>,
        kind: ExecutionErrorKind,
    ) -> ExecutionError {
        // Set span from the expression
        let span = self.expr.ann.span_of(expr).expect("span not found");
        // Set source from the annotated source
        let source = self.expr.ann.source.to_string();
        ExecutionError { kind, span, source }
    }

    /// Evaluate a type-checked expression.
    pub fn eval(&mut self) -> Result<Value<'types, 'arena>, ExecutionError> {
        self.eval_expr(self.expr.expr)
    }

    /// Evaluate an expression node.
    pub(crate) fn eval_expr(
        &mut self,
        expr: &'arena Expr<'types, 'arena>,
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        // Check depth before recursing
        if self.depth >= self.options.max_depth {
            return self.error(
                expr,
                StackOverflow {
                    depth: self.depth,
                    max_depth: self.options.max_depth,
                }
                .into(),
            );
        }

        self.depth += 1;
        let result = self.eval_expr_inner(expr);
        self.depth -= 1;

        result
    }

    /// Inner evaluation logic (no depth tracking).
    fn eval_expr_inner(
        &mut self,
        expr: &'arena Expr<'types, 'arena>,
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        match &expr.1 {
            ExprInner::Constant(value) => {
                // Constants are already values, just return them
                Ok(*value)
            }

            ExprInner::Ident(name) => {
                // Look up variable in scope stack
                match self.scope_stack.lookup(name) {
                    Some(value) => Ok(*value),
                    None => {
                        // This should never happen if the expression was type-checked
                        debug_assert!(
                            false,
                            "Undefined variable '{}' - analyzer should have caught this",
                            name
                        );
                        // In release mode, we can't panic, so return a dummy value
                        // This is safe because the expression is guaranteed to be well-typed
                        unreachable!("Undefined variable '{}' in type-checked expression", name)
                    }
                }
            }

            ExprInner::Binary { op, left, right } => {
                use crate::types::Type;

                // Recursively evaluate operands (direct call to eval_expr, not eval)
                let left_val = self.eval_expr(left)?;
                let right_val = self.eval_expr(right)?;

                // Dispatch based on type (we know both operands have the same type after type-checking)
                match left_val.ty {
                    Type::Int => {
                        let l = left_val.as_int().expect("Type-checked as Int");
                        let r = right_val.as_int().expect("Type-checked as Int");
                        let result = super::operators::eval_binary_int(*op, l, r)
                            .map_err(|e| self.add_error_context(expr, e))?;
                        Ok(Value::int(self.type_manager, result))
                    }
                    Type::Float => {
                        let l = left_val.as_float().expect("Type-checked as Float");
                        let r = right_val.as_float().expect("Type-checked as Float");
                        let result = super::operators::eval_binary_float(*op, l, r);
                        Ok(Value::float(self.type_manager, result))
                    }
                    _ => {
                        // Type checker should have caught this
                        debug_assert!(false, "Binary operator on non-numeric type");
                        unreachable!("Binary operator on invalid type in type-checked expression")
                    }
                }
            }

            ExprInner::Boolean { op, left, right } => {
                // Evaluate left operand
                let left_val = self.eval_expr(left)?;
                let left_bool = left_val.as_bool().expect("Type-checked as Bool");

                // Short-circuit evaluation
                match op {
                    BoolOp::And => {
                        // If left is false, return false without evaluating right
                        if !left_bool {
                            return Ok(Value::bool(self.type_manager, false));
                        }
                        // Left is true, return right's value
                        let right_val = self.eval_expr(right)?;
                        let right_bool = right_val.as_bool().expect("Type-checked as Bool");
                        Ok(Value::bool(self.type_manager, right_bool))
                    }
                    BoolOp::Or => {
                        // If left is true, return true without evaluating right
                        if left_bool {
                            return Ok(Value::bool(self.type_manager, true));
                        }
                        // Left is false, return right's value
                        let right_val = self.eval_expr(right)?;
                        let right_bool = right_val.as_bool().expect("Type-checked as Bool");
                        Ok(Value::bool(self.type_manager, right_bool))
                    }
                }
            }

            ExprInner::Comparison { op, left, right } => {
                use crate::types::Type;

                // Recursively evaluate operands
                let left_val = self.eval_expr(left)?;
                let right_val = self.eval_expr(right)?;

                // For containment operations, dispatch based on haystack (right) type
                // For other comparisons, dispatch based on left type
                let result = if matches!(op, ComparisonOp::In | ComparisonOp::NotIn) {
                    // Containment: needle in haystack
                    match right_val.ty {
                        Type::Str => {
                            let needle = left_val.as_str().expect("Type-checked as Str");
                            let haystack = right_val.as_str().expect("Type-checked as Str");
                            super::operators::eval_comparison_string(*op, needle, haystack)
                        }
                        Type::Bytes => {
                            let needle = left_val.as_bytes().expect("Type-checked as Bytes");
                            let haystack = right_val.as_bytes().expect("Type-checked as Bytes");
                            super::operators::eval_comparison_bytes(*op, needle, haystack)
                        }
                        Type::Array(_) => {
                            let needle = left_val;
                            let haystack = right_val.as_array().expect("Type-checked as Array");
                            let found = haystack.iter().any(|elem| elem == needle);
                            match op {
                                ComparisonOp::In => found,
                                ComparisonOp::NotIn => !found,
                                _ => unreachable!(),
                            }
                        }
                        Type::Map(_, _) => {
                            let key = left_val;
                            let haystack = right_val.as_map().expect("Type-checked as Map");
                            let found = haystack.get(&key).is_some();
                            match op {
                                ComparisonOp::In => found,
                                ComparisonOp::NotIn => !found,
                                _ => unreachable!(),
                            }
                        }
                        _ => {
                            debug_assert!(false, "Containment operation on non-containable type");
                            unreachable!(
                                "Containment on invalid haystack type in type-checked expression"
                            )
                        }
                    }
                } else {
                    // Regular comparison: dispatch based on left type
                    match left_val.ty {
                        Type::Int => {
                            let l = left_val.as_int().expect("Type-checked as Int");
                            let r = right_val.as_int().expect("Type-checked as Int");
                            super::operators::eval_comparison_int(*op, l, r)
                        }
                        Type::Float => {
                            let l = left_val.as_float().expect("Type-checked as Float");
                            let r = right_val.as_float().expect("Type-checked as Float");
                            super::operators::eval_comparison_float(*op, l, r)
                        }
                        Type::Bool => {
                            let l = left_val.as_bool().expect("Type-checked as Bool");
                            let r = right_val.as_bool().expect("Type-checked as Bool");
                            super::operators::eval_comparison_bool(*op, l, r)
                        }
                        Type::Str => {
                            let l = left_val.as_str().expect("Type-checked as Str");
                            let r = right_val.as_str().expect("Type-checked as Str");
                            super::operators::eval_comparison_string(*op, l, r)
                        }
                        Type::Bytes => {
                            let l = left_val.as_bytes().expect("Type-checked as Bytes");
                            let r = right_val.as_bytes().expect("Type-checked as Bytes");
                            super::operators::eval_comparison_bytes(*op, l, r)
                        }
                        _ => {
                            // For other types, we only support equality operators
                            match op {
                                ComparisonOp::Eq => left_val == right_val,
                                ComparisonOp::Neq => left_val != right_val,
                                _ => {
                                    // Type checker should have caught this
                                    debug_assert!(
                                        false,
                                        "Ordering comparison on non-orderable type"
                                    );
                                    unreachable!(
                                        "Ordering comparison on invalid type in type-checked expression"
                                    )
                                }
                            }
                        }
                    }
                };

                Ok(Value::bool(self.type_manager, result))
            }

            ExprInner::Where { expr, bindings } => {
                // Extract binding names
                let names: crate::Vec<&'arena str> =
                    bindings.iter().map(|(name, _)| *name).collect();

                // Push incomplete scope with all binding names
                // This allows sequential binding (later bindings can reference earlier ones)
                self.scope_stack.push(
                    scope_stack::IncompleteScope::new(self.arena, &names)
                        .expect("Duplicate binding in where - analyzer should have caught this"),
                );

                // Evaluate and bind each expression sequentially
                for (name, value_expr) in bindings.iter() {
                    let value = self.eval_expr(value_expr)?;
                    self.scope_stack
                        .bind_in_current(name, value)
                        .expect("Failed to bind in where - analyzer should have caught this");
                }

                // Evaluate the body expression (has access to all bindings)
                let result = self.eval_expr(expr)?;

                // Pop the scope
                self.scope_stack
                    .pop()
                    .expect("Failed to pop where scope - internal error");

                Ok(result)
            }

            ExprInner::Record { fields } => {
                // Resolve type (replaces type variables if evaluating polymorphic lambda)
                let resolved_ty = self.resolve_type(expr.0);

                // Get field names from the type (which has the right lifetime 'types)
                let Type::Record(field_types) = resolved_ty else {
                    unreachable!("Record expression must have Record type")
                };

                // Evaluate fields in type order (sorted), not AST order
                // Build a map from field name to field expression for quick lookup
                let mut field_map = hashbrown::HashMap::new();
                for (name, expr) in fields.iter() {
                    field_map.insert(*name, expr);
                }

                // Evaluate in type order and collect values
                let mut field_values_temp: crate::Vec<(&'types str, Value<'types, 'arena>)> =
                    crate::Vec::new();

                for (field_name, _field_ty) in field_types.iter() {
                    // Look up the expression for this field
                    let field_expr = field_map
                        .get(field_name)
                        .expect("Field in type but not in AST - analyzer should have caught this");

                    // Evaluate the field expression
                    let field_value = self.eval_expr(field_expr)?;

                    // Use field name from type (has 'types lifetime)
                    field_values_temp.push((*field_name, field_value));
                }

                // Allocate in arena to get proper lifetime
                let field_values = self.arena.alloc_slice_copy(&field_values_temp);

                // Construct record value (fields are now in sorted order)
                Ok(Value::record(self.arena, resolved_ty, field_values)
                    .expect("Record construction failed - analyzer should have validated types"))
            }

            ExprInner::Field { value, field } => {
                // Evaluate the record expression
                let record_value = self.eval_expr(value)?;

                // Extract as record
                let record = record_value
                    .as_record()
                    .expect("Field access on non-record - analyzer should have caught this");

                // Look up field by name
                Ok(record
                    .get(field)
                    .expect("Field not found in record - analyzer should have caught this"))
            }

            ExprInner::Unary { op, expr: operand } => {
                use crate::types::Type;

                // Evaluate the operand
                let operand_val = self.eval_expr(operand)?;

                // Dispatch based on type
                match operand_val.ty {
                    Type::Int => {
                        let val = operand_val.as_int().expect("Type-checked as Int");
                        let result = super::operators::eval_unary_int(*op, val);
                        Ok(Value::int(self.type_manager, result))
                    }
                    Type::Float => {
                        let val = operand_val.as_float().expect("Type-checked as Float");
                        let result = super::operators::eval_unary_float(*op, val);
                        Ok(Value::float(self.type_manager, result))
                    }
                    Type::Bool => {
                        let val = operand_val.as_bool().expect("Type-checked as Bool");
                        let result = super::operators::eval_unary_bool(*op, val);
                        Ok(Value::bool(self.type_manager, result))
                    }
                    _ => {
                        // Type checker should have caught this
                        debug_assert!(false, "Unary operator on invalid type");
                        unreachable!("Unary operator on invalid type in type-checked expression")
                    }
                }
            }

            ExprInner::If {
                cond,
                then_branch,
                else_branch,
            } => {
                // Evaluate the condition
                let cond_val = self.eval_expr(cond)?;
                let cond_bool = cond_val.as_bool().expect("Type-checked as Bool");

                // Evaluate the appropriate branch (lazy evaluation)
                if cond_bool {
                    self.eval_expr(then_branch)
                } else {
                    self.eval_expr(else_branch)
                }
            }

            ExprInner::Array { elements } => {
                // Evaluate all element expressions
                let mut element_values: Vec<Value<'types, 'arena>> = Vec::new();
                for elem_expr in elements.iter() {
                    let elem_value = self.eval_expr(elem_expr)?;
                    element_values.push(elem_value);
                }

                // Resolve type (replaces type variables if evaluating polymorphic lambda)
                let resolved_ty = self.resolve_type(expr.0);

                // Construct array value
                // The analyzer ensures all elements have the same type, so this should never fail
                Ok(Value::array(self.arena, resolved_ty, &element_values)
                    .expect("Array construction failed - analyzer should have validated types"))
            }

            ExprInner::Index { value, index } => {
                // Evaluate the value being indexed
                let indexed_value = self.eval_expr(value)?;

                // Evaluate the index expression
                let index_value = self.eval_expr(index)?;

                // Handle array indexing
                if let Ok(array) = indexed_value.as_array() {
                    let original_index: i64 = index_value
                        .as_int()
                        .expect("Index with non-integer - analyzer should have caught this");

                    let index = usize::try_from(if original_index < 0 {
                        original_index + array.len() as i64
                    } else {
                        original_index
                    });
                    if index.is_err() || index.unwrap() >= array.len() {
                        return self.error(
                            expr,
                            IndexOutOfBounds {
                                index: original_index,
                                len: array.len(),
                            }
                            .into(),
                        );
                    }

                    // Get element (safe after bounds check)
                    Ok(array
                        .get(index.unwrap())
                        .expect("Index should be in bounds after check"))

                // Handle map indexing
                } else if let Ok(map) = indexed_value.as_map() {
                    // Look up the key in the map
                    match map.get(&index_value) {
                        Some(result) => Ok(result),
                        None => {
                            // Key not found - return error with formatted key
                            self.error(
                                expr,
                                KeyNotFound {
                                    key_display: alloc::format!("{}", index_value),
                                }
                                .into(),
                            )
                        }
                    }
                } else {
                    unreachable!(
                        "Index operation on non-indexable type - analyzer should have caught this"
                    )
                }
            }

            ExprInner::FormatStr { strs, exprs } => {
                // Invariant: strs.len() == exprs.len() + 1
                // Format: strs[0] + value(exprs[0]) + strs[1] + value(exprs[1]) + ... + strs[n]

                use core::fmt::Write;
                let mut result = crate::String::new();

                // Add first string part
                result.push_str(strs[0]);

                // Interleave evaluated expressions and string parts
                for (i, expr_item) in exprs.iter().enumerate() {
                    let value = self.eval_expr(expr_item)?;
                    // Use Display which outputs strings without quotes
                    write!(result, "{}", value).expect("Writing to String should not fail");
                    result.push_str(strs[i + 1]);
                }

                // Allocate string in arena
                let result_str = self.arena.alloc_str(&result);
                Ok(Value::str(self.arena, self.type_manager.str(), result_str))
            }

            ExprInner::Otherwise { primary, fallback } => {
                // Try to evaluate the primary expression
                match self.eval_expr(primary) {
                    Ok(value) => Ok(value),
                    // Runtime errors trigger the fallback. Resource exceeded errors and
                    // internal errors propagate without running the fallback.
                    Err(e) => match e.kind {
                        crate::evaluator::ExecutionErrorKind::Runtime(runtime_error) => {
                            tracing::debug!(error = %runtime_error, "Handled by `otherwise` block");
                            self.eval_expr(fallback)
                        }
                        crate::evaluator::ExecutionErrorKind::ResourceExceeded(_) => Err(e),
                        crate::evaluator::ExecutionErrorKind::Internal(_) => Err(e),
                    },
                }
            }

            ExprInner::Option { inner } => {
                // Resolve type (replaces type variables if evaluating polymorphic lambda)
                let resolved_ty = self.resolve_type(expr.0);

                // Evaluate Option constructor
                match inner {
                    Some(expr_inner) => {
                        // Evaluate the inner expression
                        let inner_value = self.eval_expr(expr_inner)?;

                        // Create Some(value) with resolved type
                        Value::optional(self.arena, resolved_ty, Some(inner_value))
                            .map_err(|_| {
                                self.add_error_context(
                                    expr,
                                    InvariantViolation {
                                        message: "Type resolution failed for Option value - this indicates a compiler bug".to_string(),
                                    }.into()
                                )
                            })
                    }
                    None => {
                        // Create None with resolved type
                        Value::optional(self.arena, resolved_ty, None)
                            .map_err(|_| {
                                self.add_error_context(
                                    expr,
                                    InvariantViolation {
                                        message: "Type resolution failed for Option value - this indicates a compiler bug".to_string(),
                                    }.into()
                                )
                            })
                    }
                }
            }

            ExprInner::Cast { expr: inner_expr } => {
                // Evaluate the expression being cast
                let value = self.eval_expr(inner_expr)?;

                // Resolve type (replaces type variables if evaluating polymorphic lambda)
                let resolved_ty = self.resolve_type(expr.0);

                // Perform the cast using the casting library
                // The target type is in expr.0 (the type of the Cast expression)
                crate::casting::perform_cast(self.arena, value, resolved_ty, self.type_manager)
                    .map_err(|e| self.add_error_context(expr, e.into()))
            }
            ExprInner::Call { callable, args } => {
                // Evaluate the callable expression
                let func_value = self.eval_expr(callable)?;

                // Extract function trait object
                let func = func_value
                    .as_function()
                    .expect("Type checker guarantees callable is a Function");

                // Evaluate all arguments
                let arg_values: alloc::vec::Vec<Value<'types, 'arena>> = args
                    .iter()
                    .map(|arg| self.eval_expr(arg))
                    .collect::<Result<_, _>>()?;

                // Call the function via trait method
                // SAFETY: The type checker guarantees the function type matches,
                // arguments have correct types, and arity is correct.
                let ctx = FfiContext::new(self.arena, self.type_manager);
                unsafe { func.call_unchecked(&ctx, &arg_values) }
            }
            ExprInner::Lambda {
                params,
                body,
                captures,
            } => {
                // Capture the values of free variables from the current scope
                let mut capture_values = Vec::new();
                for &name in captures.iter() {
                    if let Some(value) = self.scope_stack.lookup(name) {
                        // TODO: Filter out globals (they should be accessed during call, not captured)
                        capture_values.push((name, *value));
                    }
                }

                let captures_slice = self.arena.alloc_slice_copy(&capture_values);

                // Construct a TypedExpr for the lambda body so it can report errors with spans
                // We use the same annotation source as the parent expression
                let body_typed = self.arena.alloc(TypedExpr {
                    expr: body,
                    ann: self.expr.ann,
                    // Evaluator doesn't need instantiation info (just for error reporting)
                    lambda_instantiations: hashbrown::HashMap::new_in(self.arena),
                });

                let lambda = EvalLambda::new(expr.0, params, body_typed, captures_slice);

                // Value::function returns Result, but should never fail because
                // the type checker guarantees expr.0 is a Function type
                let fun = Value::function(self.arena, lambda)
                    .expect("Type checker guarantees Function type");
                Ok(fun)
            }
            ExprInner::Map { elements } => {
                // Evaluate all key-value pairs
                let mut pair_values: Vec<(Value<'types, 'arena>, Value<'types, 'arena>)> =
                    Vec::new();
                for (key_expr, value_expr) in elements.iter() {
                    let key_value = self.eval_expr(key_expr)?;
                    let value_value = self.eval_expr(value_expr)?;
                    pair_values.push((key_value, value_value));
                }

                // Resolve type to handle polymorphic lambda bodies
                let resolved_ty = self.resolve_type(expr.0);

                // Construct map value
                // The analyzer ensures all keys and values have consistent types
                Ok(Value::map(self.arena, resolved_ty, &pair_values)
                    .expect("Map construction failed - analyzer should have validated types"))
            }

            ExprInner::Match {
                expr: match_expr,
                arms,
            } => {
                // Evaluate the matched expression
                let matched_value = self.eval_expr(match_expr)?;

                // Try each pattern arm in order
                for arm in arms.iter() {
                    // Check if pattern matches, and if so, bind variables and evaluate body
                    if let Some(mut bindings) = self.match_pattern(arm.pattern, matched_value)? {
                        // Sort bindings by variable name (required by CompleteScope::from_sorted)
                        bindings.sort_by_key(|(name, _)| *name);

                        // Create a new scope with pattern bindings
                        self.scope_stack
                            .push(scope_stack::CompleteScope::from_sorted(
                                self.arena.alloc_slice_copy(&bindings),
                            ));

                        // Evaluate the arm body (don't use ? yet to ensure scope cleanup)
                        let result = self.eval_expr(arm.body);

                        // Always pop pattern binding scope, even on error
                        self.scope_stack
                            .pop()
                            .expect("Scope stack underflow - this is a bug");

                        // Now return the result (propagate error if any)
                        return result;
                    }
                }

                // This should never happen if exhaustiveness checking is working
                unreachable!("Non-exhaustive pattern match - analyzer should have caught this")
            }
        }
    }

    /// Check if a pattern matches a value, returning variable bindings if it matches.
    /// Returns Ok(Some(bindings)) if the pattern matches, Ok(None) if it doesn't match,
    /// or Err if a runtime error occurs.
    fn match_pattern(
        &self,
        pattern: &'arena TypedPattern<'types, 'arena>,
        value: Value<'types, 'arena>,
    ) -> Result<Option<Vec<(&'arena str, Value<'types, 'arena>)>>, ExecutionError> {
        match pattern {
            TypedPattern::Wildcard => {
                // Wildcard always matches, no bindings
                Ok(Some(Vec::new()))
            }
            TypedPattern::Var(name) => {
                // Variable always matches and binds the value
                Ok(Some(crate::vec![(*name, value)]))
            }
            TypedPattern::Literal(pattern_value) => {
                // Literal matches if values are equal
                if value == *pattern_value {
                    Ok(Some(Vec::new()))
                } else {
                    Ok(None)
                }
            }
            TypedPattern::Some(inner_pattern) => {
                // Some pattern matches Option::Some values
                // as_option() returns Result<Option<Value>, TypeError>
                let option_value = value.as_option().expect("Type-checked as Option");
                match option_value {
                    Some(inner_value) => {
                        // Recursively match the inner pattern
                        self.match_pattern(inner_pattern, inner_value)
                    }
                    None => Ok(None), // Value is None, pattern is Some - no match
                }
            }
            TypedPattern::None => {
                // None pattern matches Option::None values
                let option_value = value.as_option().expect("Type-checked as Option");
                match option_value {
                    None => Ok(Some(Vec::new())),
                    Some(_) => Ok(None), // Value is Some, pattern is None - no match
                }
            }
        }
    }
}
