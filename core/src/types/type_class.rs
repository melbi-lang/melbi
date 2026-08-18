/// Type classes (traits) for Melbi's type system.
///
/// Type classes enable ad-hoc polymorphism by constraining type variables to types
/// that support specific operations. For example:
///   - `Numeric` for arithmetic operations (+, -, *, /, ^)
///   - `Indexable` for index operations (arr[i])
///   - `Hashable` for use as Map keys
///
/// # Design
///
/// Type classes are represented as constraints on type variables. During type
/// inference, operations add constraints (e.g., + adds Numeric constraint).
/// After unification, constraints are checked against the resolved types.
///
/// # Universal Capabilities (No Trait Required)
///
/// Some operations are available on all types without explicit constraints:
///   - **Eq**: All types support == and != (even Functions use reference equality)
///   - **Show**: All types can be converted to strings (including Functions)
///
/// These are built into the language and don't require type class constraints.
use crate::types::Type;
use crate::types::traits::TypeView;

/// Type class identifiers for Melbi's constraint system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeClassId {
    /// Numeric operations: +, -, *, /, ^
    /// Instances: Int, Float
    Numeric,

    /// Indexing operations: value[index]
    /// Instances: Array[e], Map[k,v], Bytes
    Indexable,

    /// Hashable types can be used as Map keys
    /// Instances: Int, Float, Bool, Str, Bytes, Symbol, Array[e] where e: Hashable
    Hashable,

    /// Ordering operations: <, >, <=, >=
    /// Instances: Int, Float, Str, Bytes
    Ord,

    /// Containment operations: in, not in
    /// Instances: (Str, Str), (Bytes, Bytes), (element, Array), (key, Map)
    /// Note: This is a relational constraint between two types (`has_instance` doesn't apply)
    Containable,
}

impl TypeClassId {
    /// Returns a human-readable name for this type class.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Numeric => "Numeric",
            Self::Indexable => "Indexable",
            Self::Hashable => "Hashable",
            Self::Ord => "Ord",
            Self::Containable => "Containable",
        }
    }

    /// Returns a description of what operations this type class enables.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::Numeric => "arithmetic operations (+, -, *, /, ^)",
            Self::Indexable => "indexing operations (value[index])",
            Self::Hashable => "use as Map keys",
            Self::Ord => "comparison operations (<, >, <=, >=)",
            Self::Containable => "containment operations (in, not in)",
        }
    }

    /// Returns which types implement this type class.
    #[must_use]
    pub fn instances(self) -> &'static str {
        match self {
            Self::Numeric => "Int, Float",
            Self::Indexable => "Array, Map, Bytes",
            Self::Hashable => {
                "Int, Float, Bool, Str, Bytes, Symbol, Array (if elements are Hashable)"
            }
            Self::Ord => "Int, Float, Str, Bytes",
            Self::Containable => "(Str, Str), (Bytes, Bytes), (element, Array), (key, Map)",
        }
    }
}

/// Checks if a type has an instance of a type class.
///
/// This is the core of the type class system. It determines whether a given
/// type satisfies a type class constraint.
///
/// # Returns
///
/// - `true` if the type definitely has an instance
/// - `false` if the type definitely does not have an instance
///
/// Note: Type variables should be resolved before calling this function.
/// If a type variable is passed, it will return `false`.
#[must_use]
pub fn has_instance<'a>(ty: &'a Type<'a>, class: TypeClassId) -> bool {
    use crate::types::traits::TypeKind;

    match (ty.view(), class) {
        // Numeric: Int, Float
        (TypeKind::Int | TypeKind::Float, TypeClassId::Numeric)
        // Indexable: Array, Map, Bytes
        | (TypeKind::Array(_) | TypeKind::Map(_, _) | TypeKind::Bytes, TypeClassId::Indexable)
        // Hashable: Most types except Function, Record, Map
        | (
            TypeKind::Int
            | TypeKind::Float
            | TypeKind::Bool
            | TypeKind::Str
            | TypeKind::Bytes
            | TypeKind::Symbol(_),
            TypeClassId::Hashable,
        )
        // Ord: Int, Float, Str, Bytes
        | (
            TypeKind::Int | TypeKind::Float | TypeKind::Str | TypeKind::Bytes,
            TypeClassId::Ord,
        ) => true,

        // Array[e] is Hashable if e is Hashable (recursive check)
        (TypeKind::Array(elem_ty), TypeClassId::Hashable) => {
            has_instance(elem_ty, TypeClassId::Hashable)
        }

        // All other combinations (including unresolved TypeVars) don't have instances
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use bumpalo::Bump;

    use super::*;
    use crate::types::manager::TypeManager;

    #[test]
    fn numeric_instances() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        assert!(has_instance(tm.int(), TypeClassId::Numeric));
        assert!(has_instance(tm.float(), TypeClassId::Numeric));
        assert!(!has_instance(tm.bool(), TypeClassId::Numeric));
        assert!(!has_instance(tm.str(), TypeClassId::Numeric));
    }

    #[test]
    fn indexable_instances() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        let int_array = tm.array(tm.int());
        assert!(has_instance(int_array, TypeClassId::Indexable));

        let map = tm.map(tm.int(), tm.str());
        assert!(has_instance(map, TypeClassId::Indexable));

        assert!(has_instance(tm.bytes(), TypeClassId::Indexable));
        assert!(!has_instance(tm.int(), TypeClassId::Indexable));
    }

    #[test]
    fn hashable_instances() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        // Primitives
        assert!(has_instance(tm.int(), TypeClassId::Hashable));
        assert!(has_instance(tm.float(), TypeClassId::Hashable));
        assert!(has_instance(tm.bool(), TypeClassId::Hashable));
        assert!(has_instance(tm.str(), TypeClassId::Hashable));
        assert!(has_instance(tm.bytes(), TypeClassId::Hashable));

        // Arrays: recursive check
        let int_array = tm.array(tm.int());
        assert!(has_instance(int_array, TypeClassId::Hashable));

        let func = tm.function(&[tm.int()], tm.int());
        let func_array = tm.array(func);
        assert!(!has_instance(func_array, TypeClassId::Hashable));

        // Functions and Records are not hashable
        assert!(!has_instance(func, TypeClassId::Hashable));
        let record = tm.record(&[("x", tm.int())]);
        assert!(!has_instance(record, TypeClassId::Hashable));

        // Maps are not hashable (for now)
        let map = tm.map(tm.int(), tm.str());
        assert!(!has_instance(map, TypeClassId::Hashable));
    }

    #[test]
    fn ord_instances() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        assert!(has_instance(tm.int(), TypeClassId::Ord));
        assert!(has_instance(tm.float(), TypeClassId::Ord));
        assert!(has_instance(tm.str(), TypeClassId::Ord));
        assert!(has_instance(tm.bytes(), TypeClassId::Ord));
        assert!(!has_instance(tm.bool(), TypeClassId::Ord));
    }

    #[test]
    fn type_class_names() {
        assert_eq!(TypeClassId::Numeric.name(), "Numeric");
        assert_eq!(TypeClassId::Indexable.name(), "Indexable");
        assert_eq!(TypeClassId::Hashable.name(), "Hashable");
        assert_eq!(TypeClassId::Ord.name(), "Ord");
        assert_eq!(TypeClassId::Containable.name(), "Containable");
    }
}
