//! Alpha-conversion for types - renaming type variables while preserving structure.
//!
//! This module provides `AlphaConverter`, a generic type transformer that renames
//! type variables using a consistent mapping. This is useful for:
//! - Normalizing types before comparison
//! - Avoiding variable name conflicts
//! - Generating fresh type variables
//!
//! # Example
//!
//! ```ignore
//! use crate::types::{TypeManager, alpha_converter::AlphaConverter};
//! use bumpalo::Bump;
//!
//! let bump = Bump::new();
//! let manager = TypeManager::new(&bump);
//!
//! // Type with variables: Array[_0] -> _0
//! let var_0 = manager.type_var(0);
//! let arr_ty = manager.array(var_0);
//! let func_ty = manager.function(&[arr_ty], var_0);
//!
//! // Alpha-convert to use fresh variables starting from 100
//! let mut converter = AlphaConverter::new(manager, 100);
//! let converted = converter.convert(func_ty);
//!
//! // Result: Array[_100] -> _100 (same structure, different variable IDs)
//! ```

use core::cell::{Cell, RefCell};

use hashbrown::HashMap;

use crate::types::traits::{TypeBuilder, TypeKind, TypeTransformer, TypeView};

/// Alpha-converter for type variable renaming.
///
/// Consistently renames type variables while preserving type structure.
/// Each unique variable ID is mapped to a fresh ID, with the same old ID
/// always mapping to the same new ID within a conversion.
///
/// Uses interior mutability (`RefCell`, Cell) to maintain mutable state while
/// working with the `&self` `TypeTransformer` API (needed for lazy iterators).
pub struct AlphaConverter<'a, B> {
    /// The type builder to build converted types
    builder: B,

    /// Mapping from old variable IDs to new variable IDs
    mapping: RefCell<HashMap<u16, u16>>,

    /// Next fresh variable ID to allocate
    next_var: Cell<u16>,

    /// Lifetime marker
    _phantom: core::marker::PhantomData<&'a ()>,
}

impl<B> AlphaConverter<'_, B> {
    /// Create a new alpha-converter with a given constructor and starting variable ID.
    ///
    /// # Arguments
    ///
    /// * `constructor` - Type constructor to build converted types
    /// * `start_var` - First variable ID to use for renaming (default: 0)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let converter = AlphaConverter::new(manager, 100);  // Start at _100
    /// ```
    pub fn new(builder: B, start_var: u16) -> Self {
        Self {
            builder,
            mapping: RefCell::new(HashMap::new()),
            next_var: Cell::new(start_var),
            _phantom: core::marker::PhantomData,
        }
    }

    /// Get a fresh variable ID.
    ///
    /// If this `old_id` has been seen before, return its existing mapping.
    /// Otherwise, allocate a fresh ID and record the mapping.
    fn fresh_var(&self, old_id: u16) -> u16 {
        if let Some(&new_id) = self.mapping.borrow().get(&old_id) {
            return new_id;
        }

        let new_id = self.next_var.get();
        self.next_var
            .set(new_id.checked_add(1).expect("Variable ID overflow"));
        self.mapping.borrow_mut().insert(old_id, new_id);
        new_id
    }
}

impl<'a, B: TypeBuilder<'a>> AlphaConverter<'a, B>
where
    B::Repr: TypeView<'a>,
{
    /// Convert a type, renaming all type variables.
    ///
    /// This is a convenience method that calls the `transform` method from
    /// the `TypeTransformer` trait.
    pub fn convert(&self, ty: B::Repr) -> B::Repr {
        self.transform(ty)
    }
}

impl<'a, B: TypeBuilder<'a>> TypeTransformer<'a, B> for AlphaConverter<'a, B>
where
    B::Repr: TypeView<'a>,
{
    type Input = B::Repr;

    fn builder(&self) -> &B {
        &self.builder
    }

    fn transform(&self, ty: Self::Input) -> B::Repr {
        match ty.view() {
            // The only case we override: rename type variables
            TypeKind::TypeVar(old_id) => {
                let new_id = self.fresh_var(old_id);
                self.builder().type_var(new_id)
            }

            // All other cases: delegate to default implementation
            _ => self.transform_default(ty),
        }
    }
}

#[cfg(test)]
mod tests {
    use bumpalo::Bump;

    use super::*;
    use crate::types::manager::TypeManager;

    #[test]
    fn alpha_convert_primitives() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let converter = AlphaConverter::new(manager, 0);

        // Primitives should be unchanged
        let int_ty = manager.int();
        let converted = converter.convert(int_ty);
        assert!(core::ptr::eq(converted, int_ty));
    }

    #[test]
    fn alpha_convert_single_var() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let converter = AlphaConverter::new(manager, 100);

        // _0 should become _100
        let var_0 = manager.type_var(0);
        let converted = converter.convert(var_0);

        if let crate::types::Type::TypeVar(id) = converted {
            assert!(id == &100);
        } else {
            panic!("Expected TypeVar, got {converted:?}");
        }
    }

    #[test]
    fn alpha_convert_consistent_mapping() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let converter = AlphaConverter::new(manager, 100);

        // Same variable used multiple times should map to same new ID
        let var_0 = manager.type_var(0);
        let converted1 = converter.convert(var_0);
        let converted2 = converter.convert(var_0);

        assert!(core::ptr::eq(converted1, converted2));
    }

    #[test]
    fn alpha_convert_array() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let converter = AlphaConverter::new(manager, 100);

        // Array[_0] should become Array[_100]
        let var_0 = manager.type_var(0);
        let arr_ty = manager.array(var_0);
        let converted = converter.convert(arr_ty);

        if let crate::types::Type::Array(elem) = converted {
            if let crate::types::Type::TypeVar(id) = elem {
                assert!(id == &100);
            } else {
                panic!("Expected TypeVar in array elem");
            }
        } else {
            panic!("Expected Array type");
        }
    }

    #[test]
    fn alpha_convert_function() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let converter = AlphaConverter::new(manager, 100);

        // (_0, _1) => _0 should become (_100, _101) => _100
        let var_0 = manager.type_var(0);
        let var_1 = manager.type_var(1);
        let func_ty = manager.function(&[var_0, var_1], var_0);
        let converted = converter.convert(func_ty);

        if let crate::types::Type::Function { params, ret } = converted {
            assert!(params.len() == 2);

            // First param should be _100
            if let crate::types::Type::TypeVar(id) = params[0] {
                assert!(id == &100);
            } else {
                panic!("Expected TypeVar in param 0");
            }

            // Second param should be _101
            if let crate::types::Type::TypeVar(id) = params[1] {
                assert!(id == &101);
            } else {
                panic!("Expected TypeVar in param 1");
            }

            // Return should be _100 (same as first param)
            if let crate::types::Type::TypeVar(id) = ret {
                assert!(id == &100);
            } else {
                panic!("Expected TypeVar in return");
            }

            // First param and return should be pointer-equal (same interned type)
            assert!(core::ptr::eq(params[0], *ret));
        } else {
            panic!("Expected Function type");
        }
    }

    #[test]
    fn alpha_convert_nested() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);
        let converter = AlphaConverter::new(manager, 200);

        // Map[_0, Array[_1]] -> _0 should become Map[_200, Array[_201]] -> _200
        let var_0 = manager.type_var(0);
        let var_1 = manager.type_var(1);
        let arr_ty = manager.array(var_1);
        let map_ty = manager.map(var_0, arr_ty);
        let func_ty = manager.function(&[map_ty], var_0);
        let converted = converter.convert(func_ty);

        if let crate::types::Type::Function { params, ret } = converted {
            if let crate::types::Type::Map(key, val) = params[0] {
                // Key should be _200
                if let crate::types::Type::TypeVar(id) = key {
                    assert!(id == &200);
                } else {
                    panic!("Expected TypeVar in map key");
                }

                // Val should be Array[_201]
                if let crate::types::Type::Array(elem) = val {
                    if let crate::types::Type::TypeVar(id) = elem {
                        assert!(id == &201);
                    } else {
                        panic!("Expected TypeVar in array elem");
                    }
                } else {
                    panic!("Expected Array in map val");
                }
            } else {
                panic!("Expected Map in function param");
            }

            // Return should be _200 (same as map key)
            if let crate::types::Type::TypeVar(id) = ret {
                assert!(id == &200);
            } else {
                panic!("Expected TypeVar in return");
            }
        } else {
            panic!("Expected Function type");
        }
    }
}
