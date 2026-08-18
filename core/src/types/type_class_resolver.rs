use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::parser::Span;
use crate::types::Type;
use crate::types::constraint_set::{ConstraintSet, TypeClassConstraint};
use crate::types::traits::TypeView;
use crate::types::type_class::{TypeClassId, has_instance};
use crate::types::unification::Unification;

/// Error type for constraint resolution failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintError {
    /// The type that failed to satisfy the constraint
    pub ty: String,

    /// The type class constraint that was not satisfied
    pub type_class: TypeClassId,

    /// Additional details about why the constraint failed.
    /// When empty, indicates the type simply doesn't implement the type class.
    /// When non-empty, provides specific information about the mismatch
    /// (e.g., "index must be Int, found Bool").
    pub details: String,

    /// Chain of source locations:
    /// - `spans[0]` is the original constraint location
    /// - `spans[1..]` are instantiation sites (call sites of polymorphic functions)
    pub spans: Vec<Span>,
}

impl ConstraintError {
    /// Creates a user-friendly error message.
    #[must_use]
    pub fn message(&self) -> String {
        format!(
            "Type '{}' does not implement {}\n\
             note: {} requires {}\n\
             help: {} is implemented for: {}",
            self.ty,
            self.type_class.name(),
            self.type_class.name(),
            self.type_class.description(),
            self.type_class.name(),
            self.type_class.instances()
        )
    }
}

/// Type class constraint resolver with associated types.
///
/// Resolves relational constraints like:
/// - Indexable(container, index, result): container[index] => result
/// - Numeric(left, right, result): left + right => result
///
/// Resolution involves:
/// 1. Resolving all types in the constraint through substitution
/// 2. Looking up the type class instance for the resolved types
/// 3. Unifying associated types based on the instance
///
/// For example, `Indexable(Array[Int], _idx, _result)` should unify:
/// - _idx with Int (arrays are indexed by Int)
/// - _result with Int (the element type)
pub struct TypeClassResolver<'types> {
    /// The constraint set with relational constraints
    constraints: ConstraintSet<'types>,
}

impl<'types> TypeClassResolver<'types> {
    /// Creates a new resolver with an empty constraint set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            constraints: ConstraintSet::new(),
        }
    }

    /// Adds an indexable constraint: container[index] => result
    pub fn add_indexable_constraint(
        &mut self,
        container: &'types Type<'types>,
        index: &'types Type<'types>,
        result: &'types Type<'types>,
        span: Span,
    ) {
        self.constraints
            .add_indexable(container, index, result, span);
    }

    /// Adds a numeric constraint: left op right => result
    pub fn add_numeric_constraint(
        &mut self,
        left: &'types Type<'types>,
        right: &'types Type<'types>,
        result: &'types Type<'types>,
        span: Span,
    ) {
        self.constraints.add_numeric(left, right, result, span);
    }

    /// Adds a hashable constraint: ty must be hashable
    pub fn add_hashable_constraint(&mut self, ty: &'types Type<'types>, span: Span) {
        self.constraints.add_hashable(ty, span);
    }

    /// Adds an ord constraint: ty must support ordering
    pub fn add_ord_constraint(&mut self, ty: &'types Type<'types>, span: Span) {
        self.constraints.add_ord(ty, span);
    }

    /// Adds a containable constraint: needle in haystack
    pub fn add_containable_constraint(
        &mut self,
        needle: &'types Type<'types>,
        haystack: &'types Type<'types>,
        span: Span,
    ) {
        self.constraints.add_containable(needle, haystack, span);
    }

    /// Resolves all constraints with unification.
    ///
    /// This is called after type inference is complete. It:
    /// 1. Resolves all types in constraints through substitution
    /// 2. Checks type class instances for resolved concrete types
    /// 3. Performs additional unification for associated types
    ///
    /// # Arguments
    ///
    /// * `unification` - The unification instance for resolving and unifying types
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all constraints are satisfied
    /// * `Err(errors)` with all unsatisfied constraints
    pub fn resolve_all<B>(
        &self,
        unification: &mut Unification<'types, B>,
    ) -> Result<(), Vec<ConstraintError>>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        let mut errors = Vec::new();

        for constraint in self.constraints.iter() {
            if let Err(err) = self.resolve_constraint(constraint, unification) {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Resolves a single constraint, performing unification if needed.
    fn resolve_constraint<B>(
        &self,
        constraint: &TypeClassConstraint<'types>,
        unification: &mut Unification<'types, B>,
    ) -> Result<(), ConstraintError>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        match constraint {
            TypeClassConstraint::Indexable {
                container,
                index,
                result,
                spans,
            } => self.resolve_indexable(container, index, result, unification, spans),
            TypeClassConstraint::Numeric {
                left,
                right,
                result,
                spans,
            } => self.resolve_numeric(left, right, result, unification, spans),
            TypeClassConstraint::Hashable { ty, spans } => {
                self.resolve_hashable(ty, unification, spans)
            }
            TypeClassConstraint::Ord { ty, spans } => self.resolve_ord(ty, unification, spans),
            TypeClassConstraint::Containable {
                needle,
                haystack,
                spans,
            } => self.resolve_containable(needle, haystack, unification, spans),
        }
    }

    /// Resolves an indexable constraint: container[index] => result
    ///
    /// Based on the container type, unifies index and result with the appropriate types:
    /// - Array[E]: index=Int, result=E
    /// - Map[K,V]: index=K, result=V
    /// - Bytes: index=Int, result=Int
    fn resolve_indexable<B>(
        &self,
        container: &'types Type<'types>,
        index: &'types Type<'types>,
        result: &'types Type<'types>,
        unification: &mut Unification<'types, B>,
        spans: &[Span],
    ) -> Result<(), ConstraintError>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        use crate::types::traits::TypeKind;

        tracing::debug!(
            container = %container,
            index = %index,
            result = %result,
            "Resolving Indexable constraint"
        );

        // Resolve all types first
        let container_resolved = unification.resolve(container);
        let index_resolved = unification.resolve(index);
        let result_resolved = unification.resolve(result);

        tracing::trace!(
            container_resolved = %container_resolved,
            index_resolved = %index_resolved,
            result_resolved = %result_resolved,
            "After resolve"
        );

        match container_resolved.view() {
            TypeKind::Array(elem_ty) => {
                // Array[E]: index must be Int, result must be E
                let int_ty = unification.builder().int();

                // Unify index with Int
                unification
                    .unifies_to(index_resolved, int_ty)
                    .map_err(|_| ConstraintError {
                        ty: format!("{container_resolved}"),
                        type_class: TypeClassId::Indexable,
                        details: format!(
                            "array indexing requires Int index, found {index_resolved}"
                        ),
                        spans: spans.to_vec(),
                    })?;

                // Unify result with element type
                unification
                    .unifies_to(result_resolved, elem_ty)
                    .map_err(|_| ConstraintError {
                        ty: format!("{container_resolved}"),
                        type_class: TypeClassId::Indexable,
                        details: format!(
                            "array indexing returns {elem_ty}, but expected {result_resolved}"
                        ),
                        spans: spans.to_vec(),
                    })?;

                Ok(())
            }
            TypeKind::Map(key_ty, value_ty) => {
                // Map[K,V]: index must be K, result must be V

                tracing::trace!(
                    key_ty = %key_ty,
                    value_ty = %value_ty,
                    "Map indexing constraint"
                );

                // Unify index with key type
                unification
                    .unifies_to(index_resolved, key_ty)
                    .map_err(|_| ConstraintError {
                        ty: format!("{container_resolved}"),
                        type_class: TypeClassId::Indexable,
                        details: format!(
                            "map indexing requires {key_ty} key, found {index_resolved}"
                        ),
                        spans: spans.to_vec(),
                    })?;

                tracing::trace!(
                    index_resolved = %index_resolved,
                    key_ty = %key_ty,
                    "Unified index with key type"
                );

                // Unify result with value type
                unification
                    .unifies_to(result_resolved, value_ty)
                    .map_err(|_| ConstraintError {
                        ty: format!("{container_resolved}"),
                        type_class: TypeClassId::Indexable,
                        details: format!(
                            "map indexing returns {value_ty}, but expected {result_resolved}"
                        ),
                        spans: spans.to_vec(),
                    })?;

                tracing::trace!(
                    result_resolved = %result_resolved,
                    value_ty = %value_ty,
                    "Unified result with value type"
                );

                Ok(())
            }
            TypeKind::Bytes => {
                // Bytes: index must be Int, result must be Int
                let int_ty = unification.builder().int();

                unification
                    .unifies_to(index_resolved, int_ty)
                    .map_err(|_| ConstraintError {
                        ty: format!("{container_resolved}"),
                        type_class: TypeClassId::Indexable,
                        details: format!(
                            "bytes indexing requires Int index, found {index_resolved}"
                        ),
                        spans: spans.to_vec(),
                    })?;

                unification
                    .unifies_to(result_resolved, int_ty)
                    .map_err(|_| ConstraintError {
                        ty: format!("{container_resolved}"),
                        type_class: TypeClassId::Indexable,
                        details: format!(
                            "bytes indexing returns Int, but expected {result_resolved}"
                        ),
                        spans: spans.to_vec(),
                    })?;

                Ok(())
            }
            TypeKind::TypeVar(_) => {
                // Still unresolved - this is OK, constraint will be checked later
                // This can happen in polymorphic contexts
                Ok(())
            }
            _ => Err(ConstraintError {
                ty: format!("{container_resolved}"),
                type_class: TypeClassId::Indexable,
                details: String::new(),
                spans: spans.to_vec(),
            }),
        }
    }

    /// Resolves a numeric constraint: left op right => result
    ///
    /// All three types must be the same numeric type (Int or Float).
    fn resolve_numeric<B>(
        &self,
        left: &'types Type<'types>,
        right: &'types Type<'types>,
        result: &'types Type<'types>,
        unification: &mut Unification<'types, B>,
        spans: &[Span],
    ) -> Result<(), ConstraintError>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        use crate::types::traits::TypeKind;

        // Resolve all types
        let left_resolved = unification.resolve(left);
        let right_resolved = unification.resolve(right);
        let result_resolved = unification.resolve(result);

        // Unify left with right
        unification
            .unifies_to(left_resolved, right_resolved)
            .map_err(|_| ConstraintError {
                ty: format!("{left_resolved}"),
                type_class: TypeClassId::Numeric,
                details: format!(
                    "operands must have the same numeric type, found {left_resolved} and {right_resolved}"
                ),
                spans: spans.to_vec(),
            })?;

        // Unify result with left (which is now unified with right)
        let unified_operand = unification.resolve(left_resolved);
        unification
            .unifies_to(result_resolved, unified_operand)
            .map_err(|_| ConstraintError {
                ty: format!("{unified_operand}"),
                type_class: TypeClassId::Numeric,
                details: format!(
                    "operation returns {unified_operand}, but expected {result_resolved}"
                ),
                spans: spans.to_vec(),
            })?;

        // Check that the final type is numeric (if resolved to concrete type)
        let final_ty = unification.resolve(unified_operand);
        match final_ty.view() {
            TypeKind::Int | TypeKind::Float | TypeKind::TypeVar(_) => Ok(()),
            _ => Err(ConstraintError {
                ty: format!("{final_ty}"),
                type_class: TypeClassId::Numeric,
                details: String::new(),
                spans: spans.to_vec(),
            }),
        }
    }

    /// Resolves a hashable constraint: ty must be hashable
    fn resolve_hashable<B>(
        &self,
        ty: &'types Type<'types>,
        unification: &mut Unification<'types, B>,
        spans: &[Span],
    ) -> Result<(), ConstraintError>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        use crate::types::traits::TypeKind;
        use crate::types::type_class::TypeClassId;

        // Resolve the type through substitution
        let resolved = unification.resolve(ty);

        // Check if it's still a type variable (polymorphic)
        match resolved.view() {
            TypeKind::TypeVar(_) => Ok(()), // Polymorphic, constraint will be checked at instantiation
            _ => {
                // Check if the concrete type has the Hashable instance
                if has_instance(resolved, TypeClassId::Hashable) {
                    Ok(())
                } else {
                    Err(ConstraintError {
                        ty: format!("{resolved}"),
                        type_class: TypeClassId::Hashable,
                        details: String::new(),
                        spans: spans.to_vec(),
                    })
                }
            }
        }
    }

    /// Resolves an ord constraint: ty must support ordering
    fn resolve_ord<B>(
        &self,
        ty: &'types Type<'types>,
        unification: &mut Unification<'types, B>,
        spans: &[Span],
    ) -> Result<(), ConstraintError>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        use crate::types::traits::TypeKind;
        use crate::types::type_class::TypeClassId;

        // Resolve the type through substitution
        let resolved = unification.resolve(ty);

        // Check if it's still a type variable (polymorphic)
        match resolved.view() {
            TypeKind::TypeVar(_) => Ok(()), // Polymorphic, constraint will be checked at instantiation
            _ => {
                // Check if the concrete type has the Ord instance
                if has_instance(resolved, TypeClassId::Ord) {
                    Ok(())
                } else {
                    Err(ConstraintError {
                        ty: format!("{resolved}"),
                        type_class: TypeClassId::Ord,
                        details: String::new(),
                        spans: spans.to_vec(),
                    })
                }
            }
        }
    }

    /// Resolves a containable constraint: needle in haystack
    ///
    /// Based on the haystack type, unifies needle with the appropriate type:
    /// - Str: needle must be Str
    /// - Bytes: needle must be Bytes
    /// - Array[E]: needle must be E
    /// - Map[K,V]: needle must be K
    fn resolve_containable<B>(
        &self,
        needle: &'types Type<'types>,
        haystack: &'types Type<'types>,
        unification: &mut Unification<'types, B>,
        spans: &[Span],
    ) -> Result<(), ConstraintError>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        use crate::types::traits::TypeKind;

        tracing::debug!(
            needle = %needle,
            haystack = %haystack,
            "Resolving Containable constraint"
        );

        // Resolve both types first
        let needle_resolved = unification.resolve(needle);
        let haystack_resolved = unification.resolve(haystack);

        tracing::trace!(
            needle_resolved = %needle_resolved,
            haystack_resolved = %haystack_resolved,
            "After resolve"
        );

        match haystack_resolved.view() {
            TypeKind::Str => {
                // Str in Str: needle must be Str
                let str_ty = unification.builder().str();
                unification
                    .unifies_to(needle_resolved, str_ty)
                    .map_err(|_| ConstraintError {
                        ty: format!("{haystack_resolved}"),
                        type_class: TypeClassId::Containable,
                        details: format!(
                            "string containment requires Str needle, found {needle_resolved}"
                        ),
                        spans: spans.to_vec(),
                    })?;
                Ok(())
            }
            TypeKind::Bytes => {
                // Bytes in Bytes: needle must be Bytes
                let bytes_ty = unification.builder().bytes();
                unification
                    .unifies_to(needle_resolved, bytes_ty)
                    .map_err(|_| ConstraintError {
                        ty: format!("{haystack_resolved}"),
                        type_class: TypeClassId::Containable,
                        details: format!(
                            "bytes containment requires Bytes needle, found {needle_resolved}"
                        ),
                        spans: spans.to_vec(),
                    })?;
                Ok(())
            }
            TypeKind::Array(elem_ty) => {
                // element in Array[E]: needle must be E
                unification
                    .unifies_to(needle_resolved, elem_ty)
                    .map_err(|_| ConstraintError {
                        ty: format!("{haystack_resolved}"),
                        type_class: TypeClassId::Containable,
                        details: format!(
                            "array containment requires {elem_ty} element, found {needle_resolved}"
                        ),
                        spans: spans.to_vec(),
                    })?;
                Ok(())
            }
            TypeKind::Map(key_ty, _value_ty) => {
                // key in Map[K,V]: needle must be K
                unification
                    .unifies_to(needle_resolved, key_ty)
                    .map_err(|_| ConstraintError {
                        ty: format!("{haystack_resolved}"),
                        type_class: TypeClassId::Containable,
                        details: format!(
                            "map containment requires {key_ty} key, found {needle_resolved}"
                        ),
                        spans: spans.to_vec(),
                    })?;
                Ok(())
            }
            TypeKind::TypeVar(_) => {
                // Still unresolved - this is OK, constraint will be checked later
                // This can happen in polymorphic contexts
                Ok(())
            }
            _ => {
                // Other types don't support containment
                Err(ConstraintError {
                    ty: format!("{haystack_resolved}"),
                    type_class: TypeClassId::Containable,
                    details: String::new(),
                    spans: spans.to_vec(),
                })
            }
        }
    }

    /// Returns a reference to the constraint set.
    #[must_use]
    pub fn constraint_set(&self) -> &ConstraintSet<'types> {
        &self.constraints
    }

    /// Copies constraints by applying a substitution map.
    ///
    /// When instantiating a polymorphic type scheme, we need to copy constraints
    /// from the quantified variables to the fresh variables. This method finds all
    /// constraints that mention ANY of the quantified variables and creates equivalent
    /// constraints with ALL substitutions applied at once.
    ///
    /// # Important
    ///
    /// Constraints may mention "internal" type variables that aren't part of the
    /// quantified set (e.g., intermediate results in `m[k1][k2]`). These internal
    /// variables must also be substituted with fresh variables to avoid sharing
    /// state between different instantiations.
    ///
    /// # Arguments
    ///
    /// * `subst` - Substitution map from old type variables to fresh types
    /// * `unification` - Unification context for resolving types before checking
    /// * `instantiation_span` - The span of the call site where instantiation occurs
    pub fn copy_constraints_with_subst<B>(
        &mut self,
        subst: &hashbrown::HashMap<u16, &'types Type<'types>>,
        unification: &crate::types::unification::Unification<'types, B>,
        instantiation_span: Span,
    ) where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        // Collect constraints that mention any of the quantified variables.
        // We must resolve types through unification before checking, because
        // unified variables (e.g., _1 = _2) may have been generalized under
        // the "canonical" variable ID (e.g., only _2 is in quantified).
        let constraints_to_copy: Vec<_> = self
            .constraints
            .iter()
            .filter(|c| {
                // Check if constraint mentions any variable in the substitution map
                subst
                    .keys()
                    .any(|&var_id| self.constraint_mentions_var_resolved(c, var_id, unification))
            })
            .cloned()
            .collect();

        // Build an extended substitution map that includes fresh variables for
        // any "internal" type variables mentioned in constraints but not in the
        // original quantified set. This ensures each instantiation gets its own
        // independent internal variables.
        let mut extended_subst = subst.clone();
        for constraint in &constraints_to_copy {
            self.collect_unsubstituted_vars(constraint, unification, &mut extended_subst);
        }

        // Create new constraints with full substitution applied
        for constraint in constraints_to_copy {
            // Build the new spans: original spans + instantiation site
            let mut new_spans = constraint.spans().to_vec();
            new_spans.push(instantiation_span.clone());

            match constraint {
                TypeClassConstraint::Numeric {
                    left,
                    right,
                    result,
                    ..
                } => {
                    self.constraints.push(TypeClassConstraint::Numeric {
                        left: unification.substitute(left, &extended_subst),
                        right: unification.substitute(right, &extended_subst),
                        result: unification.substitute(result, &extended_subst),
                        spans: new_spans,
                    });
                }
                TypeClassConstraint::Indexable {
                    container,
                    index,
                    result,
                    ..
                } => {
                    self.constraints.push(TypeClassConstraint::Indexable {
                        container: unification.substitute(container, &extended_subst),
                        index: unification.substitute(index, &extended_subst),
                        result: unification.substitute(result, &extended_subst),
                        spans: new_spans,
                    });
                }
                TypeClassConstraint::Hashable { ty, .. } => {
                    self.constraints.push(TypeClassConstraint::Hashable {
                        ty: unification.substitute(ty, &extended_subst),
                        spans: new_spans,
                    });
                }
                TypeClassConstraint::Ord { ty, .. } => {
                    self.constraints.push(TypeClassConstraint::Ord {
                        ty: unification.substitute(ty, &extended_subst),
                        spans: new_spans,
                    });
                }
                TypeClassConstraint::Containable {
                    needle, haystack, ..
                } => {
                    self.constraints.push(TypeClassConstraint::Containable {
                        needle: unification.substitute(needle, &extended_subst),
                        haystack: unification.substitute(haystack, &extended_subst),
                        spans: new_spans,
                    });
                }
            }
        }
    }

    /// Collect type variables from a constraint that aren't in the substitution map,
    /// and add fresh variables for them to the map.
    fn collect_unsubstituted_vars<B>(
        &self,
        constraint: &TypeClassConstraint<'types>,
        unification: &crate::types::unification::Unification<'types, B>,
        subst: &mut hashbrown::HashMap<u16, &'types Type<'types>>,
    ) where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        match constraint {
            TypeClassConstraint::Numeric {
                left,
                right,
                result,
                ..
            } => {
                self.collect_vars_from_type(left, unification, subst);
                self.collect_vars_from_type(right, unification, subst);
                self.collect_vars_from_type(result, unification, subst);
            }
            TypeClassConstraint::Indexable {
                container,
                index,
                result,
                ..
            } => {
                self.collect_vars_from_type(container, unification, subst);
                self.collect_vars_from_type(index, unification, subst);
                self.collect_vars_from_type(result, unification, subst);
            }
            TypeClassConstraint::Hashable { ty, .. } | TypeClassConstraint::Ord { ty, .. } => {
                self.collect_vars_from_type(ty, unification, subst);
            }
            TypeClassConstraint::Containable {
                needle, haystack, ..
            } => {
                self.collect_vars_from_type(needle, unification, subst);
                self.collect_vars_from_type(haystack, unification, subst);
            }
        }
    }

    /// Recursively find type variables in a type and add fresh variables to the
    /// substitution map for any that aren't already present.
    fn collect_vars_from_type<B>(
        &self,
        ty: &'types Type<'types>,
        unification: &crate::types::unification::Unification<'types, B>,
        subst: &mut hashbrown::HashMap<u16, &'types Type<'types>>,
    ) where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        use crate::types::traits::TypeKind;

        let resolved = unification.resolve(ty);

        match resolved.view() {
            TypeKind::TypeVar(id) => {
                // If not already in the substitution map, add a fresh variable
                if !subst.contains_key(&id) {
                    let fresh = unification.builder().fresh_type_var();
                    subst.insert(id, fresh);
                }
            }
            TypeKind::Array(elem) => {
                self.collect_vars_from_type(elem, unification, subst);
            }
            TypeKind::Map(key, val) => {
                self.collect_vars_from_type(key, unification, subst);
                self.collect_vars_from_type(val, unification, subst);
            }
            TypeKind::Option(inner) => {
                self.collect_vars_from_type(inner, unification, subst);
            }
            TypeKind::Record(fields) => {
                for (_, field_ty) in fields {
                    self.collect_vars_from_type(field_ty, unification, subst);
                }
            }
            TypeKind::Function { params, ret } => {
                for p in params {
                    self.collect_vars_from_type(p, unification, subst);
                }
                self.collect_vars_from_type(ret, unification, subst);
            }
            _ => {} // Primitives and symbols don't contain variables
        }
    }

    /// Checks if a constraint mentions a specific type variable (resolving through unification).
    ///
    /// This is used when copying constraints during instantiation. We must resolve types
    /// through unification because unified variables (e.g., _1 = _2 after `expect_types_match`)
    /// may only have one of the IDs in the quantified set.
    fn constraint_mentions_var_resolved<B>(
        &self,
        constraint: &TypeClassConstraint<'types>,
        var_id: u16,
        unification: &crate::types::unification::Unification<'types, B>,
    ) -> bool
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        match constraint {
            TypeClassConstraint::Numeric {
                left,
                right,
                result,
                ..
            } => {
                self.type_mentions_var_resolved(left, var_id, unification)
                    || self.type_mentions_var_resolved(right, var_id, unification)
                    || self.type_mentions_var_resolved(result, var_id, unification)
            }
            TypeClassConstraint::Indexable {
                container,
                index,
                result,
                ..
            } => {
                self.type_mentions_var_resolved(container, var_id, unification)
                    || self.type_mentions_var_resolved(index, var_id, unification)
                    || self.type_mentions_var_resolved(result, var_id, unification)
            }
            TypeClassConstraint::Hashable { ty, .. } | TypeClassConstraint::Ord { ty, .. } => {
                self.type_mentions_var_resolved(ty, var_id, unification)
            }
            TypeClassConstraint::Containable {
                needle, haystack, ..
            } => {
                self.type_mentions_var_resolved(needle, var_id, unification)
                    || self.type_mentions_var_resolved(haystack, var_id, unification)
            }
        }
    }

    /// Checks if a type mentions a specific type variable (resolving through unification).
    fn type_mentions_var_resolved<B>(
        &self,
        ty: &'types Type<'types>,
        var_id: u16,
        unification: &crate::types::unification::Unification<'types, B>,
    ) -> bool
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        use crate::types::traits::TypeKind;

        // Resolve through unification substitutions first
        let resolved = unification.resolve(ty);

        match resolved.view() {
            TypeKind::TypeVar(id) => id == var_id,
            TypeKind::Array(elem) => self.type_mentions_var_resolved(elem, var_id, unification),
            TypeKind::Map(key, val) => {
                self.type_mentions_var_resolved(key, var_id, unification)
                    || self.type_mentions_var_resolved(val, var_id, unification)
            }
            TypeKind::Record(mut fields) => fields.any(|(_, field_ty)| {
                self.type_mentions_var_resolved(field_ty, var_id, unification)
            }),
            TypeKind::Function { mut params, ret } => {
                params.any(|p| self.type_mentions_var_resolved(p, var_id, unification))
                    || self.type_mentions_var_resolved(ret, var_id, unification)
            }
            TypeKind::Option(inner) => self.type_mentions_var_resolved(inner, var_id, unification),
            _ => false, // Primitives don't mention variables
        }
    }

    /// Finds all type classes that constrain any of the given type variables.
    ///
    /// This is used to determine which type classes a polymorphic lambda uses,
    /// which helps decide if monomorphization is needed.
    pub fn type_classes_for_vars<B>(
        &self,
        var_ids: &[u16],
        unification: &crate::types::unification::Unification<'types, B>,
    ) -> hashbrown::HashSet<TypeClassId>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        let mut result = hashbrown::HashSet::new();

        for constraint in self.constraints.iter() {
            // Check if this constraint mentions any of the given variables
            let mentions_any = var_ids.iter().any(|&var_id| {
                self.constraint_mentions_var_resolved(constraint, var_id, unification)
            });

            if mentions_any {
                result.insert(constraint.type_class_id());
            }
        }

        result
    }

    /// Clears all constraints.
    pub fn clear(&mut self) {
        self.constraints.clear();
    }
}

impl Default for TypeClassResolver<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use bumpalo::Bump;

    use super::*;
    use crate::types::manager::TypeManager;
    use crate::types::unification::Unification;

    #[test]
    fn indexable_array() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: Array[Int][Int] => Int
        let arr = tm.array(tm.int());
        let result = tm.fresh_type_var();

        resolver.add_indexable_constraint(arr, tm.int(), result, Span(0..1));

        // Should resolve successfully and unify result with Int
        assert!(resolver.resolve_all(&mut unify).is_ok());
        assert_eq!(format!("{}", unify.resolve(result)), "Int");
    }

    #[test]
    fn indexable_map() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: Map[Int, Str][Int] => Str
        let map = tm.map(tm.int(), tm.str());
        let result = tm.fresh_type_var();

        resolver.add_indexable_constraint(map, tm.int(), result, Span(0..1));

        // Should resolve successfully and unify result with Str
        assert!(resolver.resolve_all(&mut unify).is_ok());
        assert_eq!(format!("{}", unify.resolve(result)), "Str");
    }

    #[test]
    fn numeric_constraint() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: Int + Int => Int
        let result = tm.fresh_type_var();

        resolver.add_numeric_constraint(tm.int(), tm.int(), result, Span(0..1));

        // Should resolve successfully and unify result with Int
        assert!(resolver.resolve_all(&mut unify).is_ok());
        assert_eq!(format!("{}", unify.resolve(result)), "Int");
    }

    #[test]
    fn containable_string() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: Str in Str
        let needle = tm.fresh_type_var();
        resolver.add_containable_constraint(needle, tm.str(), Span(0..1));

        // Should resolve successfully and unify needle with Str
        assert!(resolver.resolve_all(&mut unify).is_ok());
        assert_eq!(format!("{}", unify.resolve(needle)), "Str");
    }

    #[test]
    fn containable_bytes() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: Bytes in Bytes
        let needle = tm.fresh_type_var();
        resolver.add_containable_constraint(needle, tm.bytes(), Span(0..1));

        // Should resolve successfully and unify needle with Bytes
        assert!(resolver.resolve_all(&mut unify).is_ok());
        assert_eq!(format!("{}", unify.resolve(needle)), "Bytes");
    }

    #[test]
    fn containable_array() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: element in Array[Int]
        let needle = tm.fresh_type_var();
        let array = tm.array(tm.int());
        resolver.add_containable_constraint(needle, array, Span(0..1));

        // Should resolve successfully and unify needle with Int
        assert!(resolver.resolve_all(&mut unify).is_ok());
        assert_eq!(format!("{}", unify.resolve(needle)), "Int");
    }

    #[test]
    fn containable_map() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: key in Map[Str, Int]
        let needle = tm.fresh_type_var();
        let map = tm.map(tm.str(), tm.int());
        resolver.add_containable_constraint(needle, map, Span(0..1));

        // Should resolve successfully and unify needle with Str
        assert!(resolver.resolve_all(&mut unify).is_ok());
        assert_eq!(format!("{}", unify.resolve(needle)), "Str");
    }

    #[test]
    fn containable_invalid() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: Int in Int (invalid - Int doesn't support containment)
        resolver.add_containable_constraint(tm.int(), tm.int(), Span(0..1));

        // Should fail with a constraint error
        let result = resolver.resolve_all(&mut unify);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].type_class, TypeClassId::Containable);
    }

    #[test]
    fn containable_type_mismatch() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: Int in Str (invalid - needle type doesn't match)
        resolver.add_containable_constraint(tm.int(), tm.str(), Span(0..1));

        // Should fail with a constraint error
        let result = resolver.resolve_all(&mut unify);
        assert!(result.is_err());
    }
}
