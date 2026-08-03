use core::cell::{Cell, Ref, RefCell};

use bumpalo::Bump;
use hashbrown::{DefaultHashBuilder, HashMap};

use crate::Vec;
use crate::types::traits::{TypeKind, TypeView};
use crate::types::types::{CompareTypeArgs, Type};

pub struct TypeManager<'a> {
    // Arena holding all types from this TypeManager.
    arena: &'a Bump,
    interned_strs: RefCell<HashMap<&'a str, &'a str, DefaultHashBuilder, &'a Bump>>,
    interned: RefCell<HashMap<CompareTypeArgs<'a>, &'a Type<'a>, DefaultHashBuilder, &'a Bump>>,
    next_type_var: Cell<u16>,
}

impl<'a> TypeManager<'a> {
    pub fn new(arena: &'a Bump) -> &'a Self {
        arena.alloc(Self {
            arena,
            interned_strs: RefCell::new(HashMap::new_in(arena)),
            interned: RefCell::new(HashMap::new_in(arena)),
            next_type_var: Cell::new(0),
        })
    }

    pub(super) fn intern_str(&self, s: &str) -> &'a str {
        if let Some(&interned_str) = self.interned_strs.borrow().get(s) {
            return interned_str;
        }
        let arena_str = self.arena.alloc_str(s);
        self.interned_strs.borrow_mut().insert(arena_str, arena_str);
        arena_str
    }

    fn intern_map(
        &self,
    ) -> Ref<'_, HashMap<CompareTypeArgs<'a>, &'a Type<'a>, DefaultHashBuilder, &'a Bump>> {
        self.interned.borrow()
    }

    fn alloc_and_intern(&self, ty: Type<'a>) -> &'a Type<'a> {
        let arena_ty = self.arena.alloc(ty.clone());
        self.interned
            .borrow_mut()
            .insert(CompareTypeArgs(ty), arena_ty);
        arena_ty
    }

    // Generate fresh type variable
    pub fn fresh_type_var(&self) -> &'a Type<'a> {
        let var_id = self.next_type_var.get();
        tracing::trace!(var_id, "Creating fresh type variable");

        self.next_type_var
            .set(var_id.checked_add(1).expect("TypeVar id overflowed"));
        let ty = Type::TypeVar(var_id);
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(ty.clone())) {
            return interned_ty;
        }
        self.alloc_and_intern(ty)
    }

    /// Get or create a type variable with a specific id
    /// This is useful for deserialization and resolving instantiations
    pub(crate) fn type_var(&self, id: u16) -> &'a Type<'a> {
        let ty = Type::TypeVar(id);
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(ty.clone())) {
            return interned_ty;
        }
        self.alloc_and_intern(ty)
    }

    // Allocate a slice of u16 in the arena (for TypeScheme quantified variables)
    pub fn alloc_u16_slice(&self, slice: &[u16]) -> &'a [u16] {
        self.arena.alloc_slice_copy(slice)
    }

    // Factory methods for types.
    pub fn int(&self) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(Type::Int)) {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Int)
    }
    pub fn float(&self) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(Type::Float)) {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Float)
    }
    pub fn bool(&self) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(Type::Bool)) {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Bool)
    }
    pub fn str(&self) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(Type::Str)) {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Str)
    }
    pub fn bytes(&self) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(Type::Bytes)) {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Bytes)
    }
    pub fn array(&self, elem_ty: &'a Type<'a>) -> &'a Type<'a> {
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Array(elem_ty)))
        {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Array(elem_ty))
    }

    pub fn map(&self, key_ty: &'a Type<'a>, val_ty: &'a Type<'a>) -> &'a Type<'a> {
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Map(key_ty, val_ty)))
        {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Map(key_ty, val_ty))
    }

    pub fn option(&self, inner_ty: &'a Type<'a>) -> &'a Type<'a> {
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Option(inner_ty)))
        {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Option(inner_ty))
    }

    pub fn record(&self, fields: Vec<(&str, &'a Type<'a>)>) -> &'a Type<'a> {
        // SAFETY: We own the data in the Vec, which was moved. Also, we immediately change
        // the lifetime of the &str field to 'a.
        let mut fields: Vec<(&str, &'a Type<'a>)> = unsafe { core::mem::transmute(fields) };
        // Intern all field names in-place to ensure pointer equality works
        for (name, _) in fields.iter_mut() {
            *name = self.intern_str(name);
        }

        // Sort by interned field names in-place
        fields.sort_by_key(|(name, _)| *name);

        // Lookup using the Vec as a slice
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Record(&fields)))
        {
            return interned_ty;
        }

        // Not found - allocate directly from Vec into arena (zero-copy move)
        let arena_fields = self.arena.alloc_slice_fill_iter(fields);
        self.alloc_and_intern(Type::Record(arena_fields))
    }

    pub fn function(&self, params: &[&'a Type<'a>], ret: &'a Type<'a>) -> &'a Type<'a> {
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Function { params, ret }))
        {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Function {
            params: self.arena.alloc_slice_copy(params),
            ret,
        })
    }

    pub fn symbol(&self, parts: Vec<&str>) -> &'a Type<'a> {
        // SAFETY: We own the data in the Vec, which was moved. Also, we immediately change
        // the lifetime of the &str field to 'a.
        let mut parts: Vec<&str> = unsafe { core::mem::transmute(parts) };

        // Intern all symbol parts in-place to ensure pointer equality works
        for part in parts.iter_mut() {
            *part = self.intern_str(part);
        }

        // Sort by interned parts in-place
        parts.sort();

        // Lookup using the Vec as a slice
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Symbol(&parts)))
        {
            return interned_ty;
        }

        // Not found - allocate directly from Vec into arena (zero-copy move)
        let arena_parts = self.arena.alloc_slice_fill_iter(parts);
        self.alloc_and_intern(Type::Symbol(arena_parts))
    }

    // TODO: Implement custom types and their capabilities.
    // pub fn custom(&mut self, name: String) -> &'a Type<'a> {
    //     self.intern(Type::Custom { name })
    // }

    // // Register a custom type's capabilities
    // pub fn register_custom_type<T: TypeCapabilities + 'static>(&mut self, capabilities: T) {
    //     self.type_registry.register_type(capabilities);
    // }

    // // Check if a custom type supports an operation
    // pub fn custom_type_supports(&self, type_name: &str, operation: &str) -> bool {
    //     self.type_registry.supports_operation(type_name, operation)
    // }

    // // Get capabilities for a custom type
    // pub fn get_custom_capabilities(&self, type_name: &str) -> Option<&dyn TypeCapabilities> {
    //     self.type_registry.get_capabilities(type_name)
    // }

    /// Recursively copies a type from another TypeManager into this TypeManager's arena,
    /// returning the interned equivalent in this manager.
    pub fn adopt<'b>(&self, other: &TypeManager<'b>, ty: &'b Type<'b>) -> &'a Type<'a> {
        fn inner<'a, 'b>(
            this: &TypeManager<'a>,
            _other: &TypeManager<'b>,
            ty: &'b Type<'b>,
            var_map: &mut HashMap<*const Type<'b>, &'a Type<'a>>,
        ) -> &'a Type<'a> {
            match ty {
                Type::Int => this.int(),
                Type::Float => this.float(),
                Type::Bool => this.bool(),
                Type::Str => this.str(),
                Type::Bytes => this.bytes(),
                Type::TypeVar(_id) => {
                    let ptr = ty as *const Type<'b>;
                    if let Some(&mapped) = var_map.get(&ptr) {
                        mapped
                    } else {
                        // Use fresh_type_var to generate a new variable in this manager
                        let fresh = this.fresh_type_var();
                        var_map.insert(ptr, fresh);
                        fresh
                    }
                }
                Type::Array(elem_ty) => {
                    let elem = inner(this, _other, elem_ty, var_map);
                    this.array(elem)
                }
                Type::Map(key_ty, val_ty) => {
                    let key = inner(this, _other, key_ty, var_map);
                    let val = inner(this, _other, val_ty, var_map);
                    this.map(key, val)
                }
                Type::Option(inner_ty) => {
                    let inner_adopted = inner(this, _other, inner_ty, var_map);
                    this.option(inner_adopted)
                }
                Type::Record(fields) => {
                    let adopted_fields: Vec<(&str, &'a Type<'a>)> = fields
                        .iter()
                        .map(|(name, t)| {
                            let t = inner(this, _other, t, var_map);
                            (*name, t)
                        })
                        .collect();
                    this.record(adopted_fields)
                }
                Type::Function { params, ret } => {
                    let adopted_params: Vec<&'a Type<'a>> = params
                        .iter()
                        .map(|p| inner(this, _other, p, var_map))
                        .collect();
                    let adopted_ret = inner(this, _other, ret, var_map);
                    this.function(&adopted_params, adopted_ret)
                }
                Type::Symbol(parts) => {
                    let adopted_parts: Vec<&str> = parts.to_vec();
                    this.symbol(adopted_parts)
                }
            }
        }
        let mut var_map = HashMap::new();
        inner(self, other, ty, &mut var_map)
    }

    /// Performs alpha conversion (renaming) of type variables in a type.
    /// Takes a mapping from old variable names to new variable names.
    pub fn alpha_convert(&self, ty: &'a Type<'a>) -> &'a Type<'a> {
        pub fn inner<'a>(
            this: &TypeManager<'a>,
            ty: &'a Type<'a>,
            var_map: &mut HashMap<*const Type<'a>, &'a Type<'a>>,
        ) -> &'a Type<'a> {
            match ty {
                Type::Int | Type::Float | Type::Bool | Type::Str | Type::Bytes => ty,
                Type::TypeVar(_) => {
                    let ptr = ty as *const Type<'a>;
                    if let Some(&mapped) = var_map.get(&ptr) {
                        mapped
                    } else {
                        let fresh = this.fresh_type_var();
                        var_map.insert(ptr, fresh);
                        fresh
                    }
                }
                Type::Array(elem_ty) => {
                    let elem = inner(this, elem_ty, var_map);
                    this.array(elem)
                }
                Type::Map(key_ty, val_ty) => {
                    let key = inner(this, key_ty, var_map);
                    let val = inner(this, val_ty, var_map);
                    this.map(key, val)
                }
                Type::Option(inner_ty) => {
                    let inner_converted = inner(this, inner_ty, var_map);
                    this.option(inner_converted)
                }
                Type::Record(fields) => {
                    let converted_fields: Vec<(&'a str, &'a Type<'a>)> = fields
                        .iter()
                        .map(|(name, t)| {
                            let t = inner(this, t, var_map);
                            (*name, t)
                        })
                        .collect();
                    this.record(converted_fields)
                }
                Type::Function { params, ret } => {
                    let converted_params: Vec<&'a Type<'a>> =
                        params.iter().map(|p| inner(this, p, var_map)).collect();
                    let converted_ret = inner(this, ret, var_map);
                    this.function(&converted_params, converted_ret)
                }
                Type::Symbol(_parts) => ty, // Symbols don't contain type variables, return as-is
            }
        }
        let mut var_map = HashMap::new();
        inner(self, ty, &mut var_map)
    }
}

// ============================================================================
// TypeBuilder implementation for TypeManager<'a>
// ============================================================================

use crate::types::traits::TypeBuilder;

impl<'a> TypeBuilder<'a> for &'a TypeManager<'a> {
    type Repr = &'a Type<'a>;

    fn int(&self) -> Self::Repr {
        TypeManager::int(self)
    }

    fn float(&self) -> Self::Repr {
        TypeManager::float(self)
    }

    fn bool(&self) -> Self::Repr {
        TypeManager::bool(self)
    }

    fn str(&self) -> Self::Repr {
        TypeManager::str(self)
    }

    fn bytes(&self) -> Self::Repr {
        TypeManager::bytes(self)
    }

    fn type_var(&self, id: u16) -> Self::Repr {
        TypeManager::type_var(self, id)
    }

    fn fresh_type_var(&self) -> Self::Repr {
        TypeManager::fresh_type_var(self)
    }

    fn array(&self, elem: Self::Repr) -> Self::Repr {
        TypeManager::array(self, elem)
    }

    fn map(&self, key: Self::Repr, val: Self::Repr) -> Self::Repr {
        TypeManager::map(self, key, val)
    }

    fn option(&self, inner: Self::Repr) -> Self::Repr {
        TypeManager::option(self, inner)
    }

    fn record(&self, fields: impl Iterator<Item = (&'a str, Self::Repr)>) -> Self::Repr {
        let fields_vec: Vec<_> = fields.collect();
        TypeManager::record(self, fields_vec)
    }

    fn function(&self, params: impl Iterator<Item = Self::Repr>, ret: Self::Repr) -> Self::Repr {
        let params_vec: Vec<_> = params.collect();
        TypeManager::function(self, params_vec.as_slice(), ret)
    }

    fn symbol(&self, parts: impl Iterator<Item = &'a str>) -> Self::Repr {
        let parts_vec: Vec<_> = parts.collect();
        TypeManager::symbol(self, parts_vec)
    }
}

// ============================================================================
// TypeView implementation for &'a Type<'a>
// ============================================================================

impl<'a> TypeView<'a> for &'a Type<'a> {
    // Use standard library iterators instead of custom implementations
    // since slice elements are Copy, we use .copied() to get the values directly
    type Iter = core::iter::Copied<core::slice::Iter<'a, &'a Type<'a>>>;
    type NamedIter = core::iter::Copied<core::slice::Iter<'a, (&'a str, &'a Type<'a>)>>;
    type StrIter = core::iter::Copied<core::slice::Iter<'a, &'a str>>;

    fn view(self) -> TypeKind<'a, Self> {
        match self {
            Type::TypeVar(id) => TypeKind::TypeVar(*id),
            Type::Int => TypeKind::Int,
            Type::Float => TypeKind::Float,
            Type::Bool => TypeKind::Bool,
            Type::Str => TypeKind::Str,
            Type::Bytes => TypeKind::Bytes,
            Type::Array(elem) => TypeKind::Array(elem),
            Type::Map(key, val) => TypeKind::Map(key, val),
            Type::Record(fields) => TypeKind::Record(fields.iter().copied()),
            Type::Function { params, ret } => TypeKind::Function {
                params: params.iter().copied(),
                ret,
            },
            Type::Symbol(parts) => TypeKind::Symbol(parts.iter().copied()),
            Type::Option(inner) => TypeKind::Option(inner),
        }
    }
}

#[cfg(test)]
mod type_view_tests {
    use bumpalo::Bump;

    use super::*;
    use crate::types::manager::TypeManager;

    #[test]
    fn test_primitives() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test Int
        let ty = mgr.int();
        match ty.view() {
            TypeKind::Int => {}
            _ => panic!("Expected Int"),
        }

        // Test Float
        let ty = mgr.float();
        match ty.view() {
            TypeKind::Float => {}
            _ => panic!("Expected Float"),
        }

        // Test Bool
        let ty = mgr.bool();
        match ty.view() {
            TypeKind::Bool => {}
            _ => panic!("Expected Bool"),
        }

        // Test Str
        let ty = mgr.str();
        match ty.view() {
            TypeKind::Str => {}
            _ => panic!("Expected Str"),
        }

        // Test Bytes
        let ty = mgr.bytes();
        match ty.view() {
            TypeKind::Bytes => {}
            _ => panic!("Expected Bytes"),
        }
    }

    #[test]
    fn test_type_var() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.type_var(42);
        match ty.view() {
            TypeKind::TypeVar(id) => assert!(id == 42),
            _ => panic!("Expected TypeVar"),
        }
    }

    #[test]
    fn test_array() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.array(mgr.int());
        match ty.view() {
            TypeKind::Array(elem) => match elem.view() {
                TypeKind::Int => {}
                _ => panic!("Expected Int element"),
            },
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    fn test_map() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.map(mgr.str(), mgr.int());
        match ty.view() {
            TypeKind::Map(key, val) => {
                match key.view() {
                    TypeKind::Str => {}
                    _ => panic!("Expected Str key"),
                }
                match val.view() {
                    TypeKind::Int => {}
                    _ => panic!("Expected Int value"),
                }
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_record() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![("age", mgr.int()), ("name", mgr.str())]);
        match ty.view() {
            TypeKind::Record(fields) => {
                let fields: Vec<_> = fields.collect();
                assert!(fields.len() == 2);
                assert!(fields[0].0 == "age");
                match fields[0].1.view() {
                    TypeKind::Int => {}
                    _ => panic!("Expected Int for age"),
                }
                assert!(fields[1].0 == "name");
                match fields[1].1.view() {
                    TypeKind::Str => {}
                    _ => panic!("Expected Str for name"),
                }
            }
            _ => panic!("Expected Record"),
        }
    }

    #[test]
    fn test_function() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.int(), mgr.str()], mgr.bool());
        match ty.view() {
            TypeKind::Function { params, ret } => {
                let params: Vec<_> = params.collect();
                assert!(params.len() == 2);
                match params[0].view() {
                    TypeKind::Int => {}
                    _ => panic!("Expected Int param"),
                }
                match params[1].view() {
                    TypeKind::Str => {}
                    _ => panic!("Expected Str param"),
                }
                match ret.view() {
                    TypeKind::Bool => {}
                    _ => panic!("Expected Bool return"),
                }
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn test_symbol() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.symbol(vec!["error", "pending", "success"]);
        match ty.view() {
            TypeKind::Symbol(parts) => {
                let parts: Vec<_> = parts.collect();
                assert!(parts.len() == 3);
                // TypeManager sorts symbol parts
                assert!(parts[0] == "error");
                assert!(parts[1] == "pending");
                assert!(parts[2] == "success");
            }
            _ => panic!("Expected Symbol"),
        }
    }

    #[test]
    fn test_nested_types() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Array[Map[Str, Int]]
        let ty = mgr.array(mgr.map(mgr.str(), mgr.int()));
        match ty.view() {
            TypeKind::Array(elem) => match elem.view() {
                TypeKind::Map(key, val) => {
                    match key.view() {
                        TypeKind::Str => {}
                        _ => panic!("Expected Str key"),
                    }
                    match val.view() {
                        TypeKind::Int => {}
                        _ => panic!("Expected Int value"),
                    }
                }
                _ => panic!("Expected Map element"),
            },
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    fn test_iterator_exact_size() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test TypeIter (function params)
        let ty = mgr.function(&[mgr.int(), mgr.str(), mgr.bool()], mgr.int());
        match ty.view() {
            TypeKind::Function { params, .. } => {
                assert!(params.len() == 3);
                let params: Vec<_> = params.collect();
                assert!(params.len() == 3);
            }
            _ => panic!("Expected Function"),
        }

        // Test NamedTypeIter (record fields)
        let ty = mgr.record(vec![("a", mgr.int()), ("b", mgr.str())]);
        match ty.view() {
            TypeKind::Record(fields) => {
                assert!(fields.len() == 2);
                let fields: Vec<_> = fields.collect();
                assert!(fields.len() == 2);
            }
            _ => panic!("Expected Record"),
        }

        // Test StrIter (symbol parts)
        let ty = mgr.symbol(vec!["a", "b", "c"]);
        match ty.view() {
            TypeKind::Symbol(parts) => {
                assert!(parts.len() == 3);
                let parts: Vec<_> = parts.collect();
                assert!(parts.len() == 3);
            }
            _ => panic!("Expected Symbol"),
        }
    }

    #[test]
    fn test_view_with_temporaries() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test that .view() works on temporaries (returned directly from function)
        // This tests that `self` parameter (not `&self`) works well with Copy types
        match mgr.int().view() {
            TypeKind::Int => {}
            _ => panic!("Expected Int"),
        }

        // Test with more complex expression
        match mgr.array(mgr.int()).view() {
            TypeKind::Array(elem) => match elem.view() {
                TypeKind::Int => {}
                _ => panic!("Expected Int"),
            },
            _ => panic!("Expected Array"),
        }

        // Test that we can also call it on stored references
        let ty = mgr.map(mgr.str(), mgr.int());
        let ty_ref: &Type = ty; // Explicit reference
        match ty_ref.view() {
            TypeKind::Map(_, _) => {}
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_typeview_equality() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test pointer equality (same interned type)
        let ty1 = mgr.int();
        let ty2 = mgr.int();
        assert!(ty1 == ty2); // Should be equal (same interned instance)

        // Test inequality (different types)
        let ty3 = mgr.float();
        assert_ne!(ty1, ty3);

        // Test equality with complex types
        let arr1 = mgr.array(mgr.int());
        let arr2 = mgr.array(mgr.int());
        assert!(arr1 == arr2); // Same structure, interned to same instance

        // Test that TypeView trait bound works (requires Eq)
        fn takes_typeview<'a, V: TypeView<'a>>(_v: V) {}
        takes_typeview(ty1); // Should compile
    }
}

#[cfg(test)]
mod manager_tests {
    use super::*;

    #[test]
    fn test_interning() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        let int_type = manager.int();
        let float_type = manager.float();

        // Verify that calling the factory methods again returns the same pointer
        assert!(core::ptr::eq(int_type, manager.int()));
        assert!(core::ptr::eq(float_type, manager.float()));
    }

    #[test]
    fn test_adopt_preserves_typevar_identity() {
        let bump1 = Bump::new();
        let bump2 = Bump::new();
        let mgr1 = TypeManager::new(&bump1);
        let mgr2 = TypeManager::new(&bump2);

        // Create a type with repeated typevars: (Map[k, v], k, v) -> v
        let k = mgr1.fresh_type_var();
        let v = mgr1.fresh_type_var();
        let map = mgr1.map(k, v);
        let fields = vec![("map", map), ("k", k), ("v", v)];
        let tuple = mgr1.record(fields);
        let fun = mgr1.function(&[tuple], v);

        // Adopt into mgr2
        let adopted = mgr2.adopt(&mgr1, fun);

        // Extract adopted typevars
        if let Type::Function { params, ret } = adopted {
            if let Type::Record(fields) = params[0] {
                let adopted_map = fields.iter().find(|(n, _)| *n == "map").unwrap().1;
                let adopted_k = fields.iter().find(|(n, _)| *n == "k").unwrap().1;
                let adopted_v = fields.iter().find(|(n, _)| *n == "v").unwrap().1;

                // In the adopted type, the k in map and the k field must be the same pointer
                if let Type::Map(map_k, map_v) = adopted_map {
                    assert!(
                        core::ptr::eq(*map_k, adopted_k),
                        "k typevar identity not preserved"
                    );
                    assert!(
                        core::ptr::eq(*map_v, adopted_v),
                        "v typevar identity not preserved"
                    );
                } else {
                    panic!("Expected Map type in adopted record");
                }

                // The return type must be the same as the v field
                assert!(
                    core::ptr::eq(*ret, adopted_v),
                    "return typevar identity not preserved"
                );
            } else {
                panic!("Expected Record type in adopted function parameter");
            }
        } else {
            panic!("Expected Function type at top level");
        }
    }

    #[test]
    fn test_alpha_convert_simple_typevar() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create a type variable
        let var_a = manager.type_var(42);

        // Alpha convert
        let converted = manager.alpha_convert(var_a);

        // Should be a fresh type variable (not the same as input)
        assert!(!core::ptr::eq(converted, var_a));
        // Should still be a TypeVar
        if let Type::TypeVar(_) = converted {
            // ok
        } else {
            panic!("Expected TypeVar after alpha_convert");
        }
    }

    #[test]
    fn test_alpha_convert_function_type_same_var() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create function type: a -> a
        let var_a = manager.type_var(42);
        let func = manager.function(&[var_a], var_a);

        // Alpha convert
        let converted = manager.alpha_convert(func);

        // Should be a function type
        if let Type::Function { params, ret } = converted {
            assert!(params.len() == 1);
            // Both param and ret should be the same pointer (same fresh var)
            assert!(
                core::ptr::eq(params[0], *ret),
                "Param and ret should be the same fresh typevar"
            );
            // Should not be the same as the original var_a
            assert!(!core::ptr::eq(params[0], var_a));
        } else {
            panic!("Expected Function type after alpha_convert");
        }
    }

    #[test]
    fn test_alpha_convert_function_type_different_vars() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create function type: a -> b
        let var_a = manager.type_var(42);
        let var_b = manager.type_var(43);
        let func = manager.function(&[var_a], var_b);

        // Alpha convert
        let converted = manager.alpha_convert(func);

        // Should be a function type
        if let Type::Function { params, ret } = converted {
            assert!(params.len() == 1);
            // Param and ret should be different pointers (different fresh vars)
            assert!(
                !core::ptr::eq(params[0], *ret),
                "Param and ret should be different fresh typevars"
            );
            // Should not be the same as the original var_a or var_b
            assert!(!core::ptr::eq(params[0], var_a));
            assert!(!core::ptr::eq(*ret, var_b));
        } else {
            panic!("Expected Function type after alpha_convert");
        }
    }

    #[test]
    fn test_alpha_convert_complex_type() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create complex type: Map[a, Array[a]] -> a
        let var_a = manager.type_var(42);
        let array_a = manager.array(var_a);
        let map_a_array_a = manager.map(var_a, array_a);
        let func = manager.function(&[map_a_array_a], var_a);

        // Alpha convert
        let converted = manager.alpha_convert(func);

        // Should be a function type
        if let Type::Function { params, ret } = converted {
            assert!(params.len() == 1);
            // ret and the typevar inside param[0] should be the same pointer
            if let Type::Map(key, val) = params[0] {
                if let Type::Array(elem) = val {
                    // All three should be the same pointer
                    assert!(core::ptr::eq(*key, *elem));
                    assert!(core::ptr::eq(*key, *ret));
                } else {
                    panic!("Expected Array type in Map value");
                }
            } else {
                panic!("Expected Map type in function param");
            }
        } else {
            panic!("Expected Function type after alpha_convert");
        }
    }

    #[test]
    fn test_alpha_convert_no_typevars() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create function type: Int -> Float
        let int_ty = manager.int();
        let float_ty = manager.float();
        let func = manager.function(&[int_ty], float_ty);

        // Alpha convert
        let converted = manager.alpha_convert(func);

        // Should be unchanged
        assert!(core::ptr::eq(converted, func));
    }
}

#[cfg(test)]
mod type_builder_tests {
    use super::*;
    use crate::types::traits::TypeBuilder;

    #[test]
    fn test_type_builder_primitives() {
        fn test_with_builder<'a, B: TypeBuilder<'a>>(builder: &B) {
            // All calls go through the TypeBuilder trait
            let int_ty = builder.int();
            let float_ty = builder.float();
            let bool_ty = builder.bool();
            let str_ty = builder.str();
            let bytes_ty = builder.bytes();
            let typevar = builder.type_var(42);

            // Test that calling again returns equal types (interning)
            assert!(int_ty == builder.int());
            assert!(float_ty == builder.float());
            assert!(bool_ty == builder.bool());
            assert!(str_ty == builder.str());
            assert!(bytes_ty == builder.bytes());
            assert!(typevar == builder.type_var(42));
        }

        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        test_with_builder(&manager);
    }

    #[test]
    fn test_type_builder_collections() {
        fn test_with_builder<'a, B: TypeBuilder<'a>>(builder: &B) {
            let int_ty = builder.int();
            let str_ty = builder.str();

            // All calls go through the TypeBuilder trait
            let arr_ty = builder.array(int_ty);
            let map_ty = builder.map(str_ty, int_ty);

            // Test that calling again returns equal types (interning)
            assert!(arr_ty == builder.array(int_ty));
            assert!(map_ty == builder.map(str_ty, int_ty));
        }

        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        test_with_builder(&manager);
    }

    #[test]
    fn test_type_builder_structural() {
        fn test_with_builder<'a, B: TypeBuilder<'a>>(builder: &B) {
            let int_ty = builder.int();
            let float_ty = builder.float();
            let str_ty = builder.str();

            // Test record with iterator (goes through TypeBuilder trait)
            let fields1 = vec![("x", int_ty), ("y", float_ty)];
            let record_ty = builder.record(fields1.into_iter());

            // Test interning - same fields should produce equal type
            let fields2 = vec![("x", int_ty), ("y", float_ty)];
            assert!(record_ty == builder.record(fields2.into_iter()));

            // Test function with iterator (goes through TypeBuilder trait)
            let params1 = vec![int_ty, str_ty];
            let func_ty = builder.function(params1.into_iter(), float_ty);

            // Test interning - same signature should produce equal type
            let params2 = vec![int_ty, str_ty];
            assert!(func_ty == builder.function(params2.into_iter(), float_ty));

            // Test symbol with iterator (goes through TypeBuilder trait)
            let parts1 = vec!["foo", "bar", "baz"];
            let sym_ty = builder.symbol(parts1.into_iter());

            // Test interning - same parts should produce equal type
            let parts2 = vec!["foo", "bar", "baz"];
            assert!(sym_ty == builder.symbol(parts2.into_iter()));
        }

        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        test_with_builder(&manager);
    }
}

#[cfg(test)]
mod type_transformer_tests {
    use super::*;
    use crate::types::traits::TypeTransformer;

    /// Identity transformer - transforms a type to itself using the same TypeManager
    struct IdentityTransformer<'a> {
        builder: &'a TypeManager<'a>,
    }

    impl<'a> TypeTransformer<'a, &'a TypeManager<'a>> for IdentityTransformer<'a> {
        type Input = &'a Type<'a>;

        fn builder(&self) -> &&'a TypeManager<'a> {
            &self.builder
        }
    }

    #[test]
    fn test_identity_transform_primitives() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let transformer = IdentityTransformer { builder: manager };

        // Test all primitives
        let int_ty = manager.int();
        let float_ty = manager.float();
        let bool_ty = manager.bool();
        let str_ty = manager.str();
        let bytes_ty = manager.bytes();

        // Identity transform should return pointer-equal types
        assert!(core::ptr::eq(transformer.transform(int_ty), int_ty));
        assert!(core::ptr::eq(transformer.transform(float_ty), float_ty));
        assert!(core::ptr::eq(transformer.transform(bool_ty), bool_ty));
        assert!(core::ptr::eq(transformer.transform(str_ty), str_ty));
        assert!(core::ptr::eq(transformer.transform(bytes_ty), bytes_ty));
    }

    #[test]
    fn test_identity_transform_typevar() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let transformer = IdentityTransformer { builder: manager };

        let var_a = manager.type_var(42);
        let var_b = manager.type_var(100);

        // Identity transform preserves type variables
        assert!(core::ptr::eq(transformer.transform(var_a), var_a));
        assert!(core::ptr::eq(transformer.transform(var_b), var_b));
    }

    #[test]
    fn test_identity_transform_collections() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let transformer = IdentityTransformer { builder: manager };

        let int_ty = manager.int();
        let str_ty = manager.str();

        // Array[Int]
        let arr_ty = manager.array(int_ty);
        let arr_transformed = transformer.transform(arr_ty);
        assert!(core::ptr::eq(arr_transformed, arr_ty));

        // Map[String, Int]
        let map_ty = manager.map(str_ty, int_ty);
        let map_transformed = transformer.transform(map_ty);
        assert!(core::ptr::eq(map_transformed, map_ty));
    }

    #[test]
    fn test_identity_transform_record() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let transformer = IdentityTransformer { builder: manager };

        // Record { x: Int, y: Float }
        let int_ty = manager.int();
        let float_ty = manager.float();
        let record_ty = manager.record(vec![("x", int_ty), ("y", float_ty)]);

        let record_transformed = transformer.transform(record_ty);

        // Should be pointer-equal due to interning
        assert!(core::ptr::eq(record_transformed, record_ty));
    }

    #[test]
    fn test_identity_transform_function() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let transformer = IdentityTransformer { builder: manager };

        // (Int, Float) -> String
        let int_ty = manager.int();
        let float_ty = manager.float();
        let str_ty = manager.str();
        let func_ty = manager.function(&[int_ty, float_ty], str_ty);

        let func_transformed = transformer.transform(func_ty);

        // Should be pointer-equal due to interning
        assert!(core::ptr::eq(func_transformed, func_ty));
    }

    #[test]
    fn test_identity_transform_symbol() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let transformer = IdentityTransformer { builder: manager };

        // Symbol `foo.bar.baz`
        let sym_ty = manager.symbol(vec!["foo", "bar", "baz"]);

        let sym_transformed = transformer.transform(sym_ty);

        // Should be pointer-equal due to interning
        assert!(core::ptr::eq(sym_transformed, sym_ty));
    }

    #[test]
    fn test_identity_transform_nested() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let transformer = IdentityTransformer { builder: manager };

        // Complex nested type: Map[String, Array[Record { x: Int, y: Float }]] -> Int
        let int_ty = manager.int();
        let float_ty = manager.float();
        let str_ty = manager.str();

        let record_ty = manager.record(vec![("x", int_ty), ("y", float_ty)]);
        let array_ty = manager.array(record_ty);
        let map_ty = manager.map(str_ty, array_ty);
        let func_ty = manager.function(&[map_ty], int_ty);

        let func_transformed = transformer.transform(func_ty);

        // Should be pointer-equal due to interning
        assert!(core::ptr::eq(func_transformed, func_ty));
    }
}

#[cfg(test)]
mod display_type_tests {
    use super::*;
    use crate::types::traits::display_type;

    #[test]
    fn test_display_primitives() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        assert!(display_type(manager.int()) == "Int");
        assert!(display_type(manager.float()) == "Float");
        assert!(display_type(manager.bool()) == "Bool");
        assert!(display_type(manager.str()) == "Str");
        assert!(display_type(manager.bytes()) == "Bytes");
    }

    #[test]
    fn test_display_type_var() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        let var_0 = manager.type_var(0);
        let var_42 = manager.type_var(42);
        let var_999 = manager.type_var(999);

        assert!(display_type(var_0) == "_0");
        assert!(display_type(var_42) == "_42");
        assert!(display_type(var_999) == "_999");
    }

    #[test]
    fn test_display_array() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        let int_ty = manager.int();
        let arr_ty = manager.array(int_ty);

        assert!(display_type(arr_ty) == "Array[Int]");
    }

    #[test]
    fn test_display_map() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        let str_ty = manager.str();
        let int_ty = manager.int();
        let map_ty = manager.map(str_ty, int_ty);

        assert!(display_type(map_ty) == "Map[Str, Int]");
    }

    #[test]
    fn test_display_record() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        let int_ty = manager.int();
        let float_ty = manager.float();
        let record_ty = manager.record(vec![("x", int_ty), ("y", float_ty)]);

        assert!(display_type(record_ty) == "Record[x: Int, y: Float]");
    }

    #[test]
    fn test_display_function() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        let int_ty = manager.int();
        let float_ty = manager.float();
        let str_ty = manager.str();

        // (Int, Float) => Str
        let func_ty = manager.function(&[int_ty, float_ty], str_ty);

        assert!(display_type(func_ty) == "(Int, Float) => Str");
    }

    #[test]
    fn test_display_function_no_params() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        let int_ty = manager.int();

        // () => Int
        let func_ty = manager.function(&[], int_ty);

        assert!(display_type(func_ty) == "() => Int");
    }

    #[test]
    fn test_display_symbol() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Note: Symbol parts are stored sorted
        let sym_ty = manager.symbol(vec!["foo", "bar", "baz"]);

        // Output will be sorted: bar, baz, foo
        assert!(display_type(sym_ty) == "Symbol[bar|baz|foo]");
    }

    #[test]
    fn test_display_nested() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Array[Map[Str, Int]]
        let str_ty = manager.str();
        let int_ty = manager.int();
        let map_ty = manager.map(str_ty, int_ty);
        let arr_ty = manager.array(map_ty);

        assert!(display_type(arr_ty) == "Array[Map[Str, Int]]");
    }

    #[test]
    fn test_display_matches_display_impl() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Test that display_type produces same output as Display for Type
        let int_ty = manager.int();
        let float_ty = manager.float();
        let str_ty = manager.str();

        // Complex type: (Map[Str, Int], Array[Float]) => Record[x: Int, y: Float]
        let map_ty = manager.map(str_ty, int_ty);
        let arr_ty = manager.array(float_ty);
        let record_ty = manager.record(vec![("x", int_ty), ("y", float_ty)]);
        let func_ty = manager.function(&[map_ty, arr_ty], record_ty);

        // Both should produce identical output
        let display_output = alloc::format!("{}", func_ty);
        let generic_output = display_type(func_ty);

        assert!(
            display_output == generic_output,
            "Display impl output: {}\nGeneric display_type output: {}",
            display_output,
            generic_output
        );
    }

    #[test]
    fn test_display_option() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        let int_ty = manager.int();
        let option_ty = manager.option(int_ty);

        assert!(display_type(option_ty) == "Option[Int]");
    }

    #[test]
    fn test_display_option_complex_inner() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Option[Array[String]]
        let str_ty = manager.str();
        let arr_ty = manager.array(str_ty);
        let option_ty = manager.option(arr_ty);

        assert!(display_type(option_ty) == "Option[Array[Str]]");
    }

    #[test]
    fn test_display_nested_option() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Option[Option[Bool]]
        let bool_ty = manager.bool();
        let inner_option = manager.option(bool_ty);
        let outer_option = manager.option(inner_option);

        assert!(display_type(outer_option) == "Option[Option[Bool]]");
    }

    #[test]
    fn test_display_option_in_complex_type() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Map[Str, Option[Int]]
        let str_ty = manager.str();
        let int_ty = manager.int();
        let option_int = manager.option(int_ty);
        let map_ty = manager.map(str_ty, option_int);

        assert!(display_type(map_ty) == "Map[Str, Option[Int]]");
    }

    #[test]
    fn test_option_interning() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        let int_ty = manager.int();
        let opt1 = manager.option(int_ty);
        let opt2 = manager.option(int_ty);

        // Same Option[Int] should return the same pointer due to interning
        assert!(core::ptr::eq(opt1, opt2));
    }

    #[test]
    fn test_option_adopt() {
        let bump1 = Bump::new();
        let bump2 = Bump::new();
        let mgr1 = TypeManager::new(&bump1);
        let mgr2 = TypeManager::new(&bump2);

        // Create Option[_0] in mgr1
        let var0 = mgr1.type_var(0);
        let opt_var = mgr1.option(var0);

        // Adopt into mgr2
        let adopted = mgr2.adopt(mgr1, opt_var);

        // Should be Option type
        match adopted {
            Type::Option(inner) => {
                // Inner should be a type variable (possibly with different ID)
                assert!(matches!(inner, Type::TypeVar(_)));
            }
            _ => panic!("Expected Option type"),
        }
    }

    #[test]
    fn test_option_alpha_convert() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create Option[_42]
        let var42 = manager.type_var(42);
        let opt_var = manager.option(var42);

        // Alpha convert should create fresh type variable inside Option
        let converted = manager.alpha_convert(opt_var);

        match converted {
            Type::Option(inner) => {
                // Inner should be a TypeVar but not the same pointer as var42
                assert!(matches!(inner, Type::TypeVar(_)));
                assert!(
                    !core::ptr::eq(*inner, var42),
                    "Expected fresh type variable"
                );
            }
            _ => panic!("Expected Option type"),
        }
    }
}
