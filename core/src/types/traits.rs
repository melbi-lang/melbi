/// `TypeView` trait enables zero-copy pattern matching over type representations.
///
/// This trait abstracts over different type representations (arena-allocated,
/// encoded bytes, indexed database) allowing generic algorithms to work with
/// any representation.
///
/// # Bounds
///
/// - `Sized`: Required for return types and avoiding trait objects
/// - `Copy`: All implementations are lightweight handles (references, indices)
/// - `Eq`: Enables equality checks (pointer equality for `&Type`, byte equality for encoded)
pub trait TypeView<'a>: Sized + Copy + Eq {
    type Iter: Iterator<Item = Self>;
    type NamedIter: Iterator<Item = (&'a str, Self)>;
    type StrIter: Iterator<Item = &'a str>;

    fn view(self) -> TypeKind<'a, Self>;
}

// Note that `repr(C, u8)` is the C equivalent of:
// `struct { uint8_t tag; Payload payload; }`
// See: https://github.com/rust-lang/rfcs/blob/master/text/2195-really-tagged-unions.md
#[repr(C, u8)]
pub enum TypeKind<'a, T: TypeView<'a>> {
    TypeVar(u16) = 0,
    Int = 1,
    Float = 2,
    Bool = 3,
    Str = 4,
    Bytes = 5,
    Array(T) = 6,
    Map(T, T) = 7,
    Record(T::NamedIter) = 8, // Must be sorted by field name.
    Function { params: T::Iter, ret: T } = 9,
    Symbol(T::StrIter) = 10, // Must be sorted.
    Option(T) = 11,
}

impl<'a, T: TypeView<'a>> TypeKind<'a, T> {
    /// Get the type tag for this type kind.
    ///
    /// This provides a stable ordering across type kinds that can be used
    /// for comparison and sorting. Returns a `TypeTag` which is Ord-comparable.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::types::traits::{TypeKind, TypeTag};
    ///
    /// let type_view = some_type.view();
    /// let tag = type_view.discriminant();
    /// // tag will be TypeTag::Int for Int, TypeTag::Float for Float, etc.
    /// ```
    pub fn discriminant(&self) -> TypeTag {
        match self {
            TypeKind::TypeVar(_) => TypeTag::TypeVar,
            TypeKind::Int => TypeTag::Int,
            TypeKind::Float => TypeTag::Float,
            TypeKind::Bool => TypeTag::Bool,
            TypeKind::Str => TypeTag::Str,
            TypeKind::Bytes => TypeTag::Bytes,
            TypeKind::Array(_) => TypeTag::Array,
            TypeKind::Map(_, _) => TypeTag::Map,
            TypeKind::Record(_) => TypeTag::Record,
            TypeKind::Function { .. } => TypeTag::Function,
            TypeKind::Symbol(_) => TypeTag::Symbol,
            TypeKind::Option(_) => TypeTag::Option,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TypeTag {
    TypeVar = 0,
    Int = 1,
    Float = 2,
    Bool = 3,
    Str = 4,
    Bytes = 5,
    Array = 6,
    Map = 7,
    Record = 8,
    Function = 9,
    Symbol = 10,
    Option = 11,
}

impl TryFrom<u8> for TypeTag {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::TypeVar),
            1 => Ok(Self::Int),
            2 => Ok(Self::Float),
            3 => Ok(Self::Bool),
            4 => Ok(Self::Str),
            5 => Ok(Self::Bytes),
            6 => Ok(Self::Array),
            7 => Ok(Self::Map),
            8 => Ok(Self::Record),
            9 => Ok(Self::Function),
            10 => Ok(Self::Symbol),
            11 => Ok(Self::Option),
            _ => Err(()),
        }
    }
}

/// `TypeBuilder` trait enables building type representations.
///
/// This trait abstracts over type construction, allowing generic algorithms
/// to build types in any representation (arena-allocated `&Type`, encoded bytes,
/// AST nodes, etc.).
///
/// # Example
///
/// ```ignore
/// impl<'a> TypeBuilder<'a> for &'a TypeManager<'a> {
///     type Repr = &'a Type<'a>;
///
///     fn int(&mut self) -> Self::Repr {
///         self.intern(Type::Int)
///     }
///     // ... other methods
/// }
/// ```
///
/// Used with `TypeTransformer` to enable generic type transformations:
/// - Alpha-conversion (variable renaming)
/// - Type substitution
/// - Format conversion (`EncodedType` → &Type)
pub trait TypeBuilder<'a>: Copy {
    /// The type representation that is built by this builder.
    type Repr: TypeView<'a>;

    // Primitives
    fn int(&self) -> Self::Repr;
    fn float(&self) -> Self::Repr;
    fn bool(&self) -> Self::Repr;
    fn str(&self) -> Self::Repr;
    fn bytes(&self) -> Self::Repr;

    // Type variable
    fn type_var(&self, id: u16) -> Self::Repr;

    /// Create a fresh type variable with a unique ID.
    ///
    /// The returned type variable is guaranteed to have an ID that has not been
    /// previously returned by this builder instance. Implementations typically
    /// use a monotonically increasing counter.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let fresh1 = builder.fresh_type_var(); // e.g., TypeVar(0)
    /// let fresh2 = builder.fresh_type_var(); // e.g., TypeVar(1)
    /// // fresh1 and fresh2 are guaranteed to have different IDs
    /// ```
    fn fresh_type_var(&self) -> Self::Repr;

    // Collections
    fn array(&self, elem: Self::Repr) -> Self::Repr;
    fn map(&self, key: Self::Repr, val: Self::Repr) -> Self::Repr;
    fn option(&self, inner: Self::Repr) -> Self::Repr;

    // Structural types
    //
    // Note: These accept `impl Iterator` rather than the specific iterator types from TypeView
    // (e.g., `<Self::Repr as TypeView>::NamedIter`) because TypeTransformer needs to map over
    // the iterators, producing new iterator types. Using `impl Iterator` provides the flexibility
    // needed for transformations while maintaining type safety on the item types.
    fn record(&self, fields: impl Iterator<Item = (&'a str, Self::Repr)>) -> Self::Repr;
    fn function(&self, params: impl Iterator<Item = Self::Repr>, ret: Self::Repr) -> Self::Repr;
    fn symbol(&self, parts: impl Iterator<Item = &'a str>) -> Self::Repr;
}

/// `TypeTransformer` trait enables generic type transformations.
///
/// This trait provides a framework for walking type structures and rebuilding them,
/// optionally transforming parts along the way. The default `transform` method
/// recursively walks the type structure using `TypeView` and rebuilds it using
/// `TypeBuilder`.
///
/// # Customization
///
/// Implementations can override specific cases by implementing custom transformation
/// logic. For example:
///
/// ```ignore
/// struct AlphaConverter<'a, C> {
///     builder: C,
///     mapping: HashMap<u16, u16>,
///     _phantom: PhantomData<&'a ()>,
/// }
///
/// impl<'a, B: TypeBuilder<'a>> TypeTransformer<'a, B> for AlphaConverter<'a, B> {
///     fn builder(&self) -> &B {
///         &self.builder
///     }
///
///     fn transform<V: TypeView<'a>>(&self, ty: V) -> B::Repr {
///         match ty.view() {
///             TypeKind::TypeVar(id) => {
///                 // Custom handling: rename variable
///                 let new_id = self.mapping.get(&id).copied().unwrap_or(id);
///                 self.builder().typevar(new_id)
///             }
///             // All other cases handled by default implementation
///             _ => self.transform_default(ty),
///         }
///     }
/// }
/// ```
///
/// # Common Use Cases
///
/// - **Alpha-conversion**: Renaming type variables
/// - **Type substitution**: Replacing type variables with concrete types
/// - **Format conversion**: Converting between representations (e.g., `EncodedType → &Type`)
/// - **Type normalization**: Simplifying or canonicalizing types
pub trait TypeTransformer<'a, B: TypeBuilder<'a>> {
    /// The input type representation this transformer works with
    type Input: TypeView<'a>;

    /// Access the underlying type builder
    fn builder(&self) -> &B;

    /// Transform a type from the input representation to the builder's representation.
    ///
    /// The default implementation recursively walks the type structure:
    /// - Primitives are reconstructed as-is
    /// - Type variables are preserved (override to customize)
    /// - Collections and structural types are recursively transformed
    fn transform(&self, ty: Self::Input) -> B::Repr {
        self.transform_default(ty)
    }

    /// Default transformation logic (used by default `transform` and available for
    /// custom implementations that want to delegate some cases).
    fn transform_default(&self, ty: Self::Input) -> B::Repr {
        match ty.view() {
            // Primitives - reconstruct as-is
            TypeKind::Int => self.builder().int(),
            TypeKind::Float => self.builder().float(),
            TypeKind::Bool => self.builder().bool(),
            TypeKind::Str => self.builder().str(),
            TypeKind::Bytes => self.builder().bytes(),

            // Type variable - preserve ID (override transform() to customize)
            TypeKind::TypeVar(id) => self.builder().type_var(id),

            // Collections - recursively transform elements
            TypeKind::Array(elem) => {
                let elem_transformed = self.transform(elem);
                self.builder().array(elem_transformed)
            }
            TypeKind::Map(key, val) => {
                let key_transformed = self.transform(key);
                let val_transformed = self.transform(val);
                self.builder().map(key_transformed, val_transformed)
            }
            TypeKind::Option(inner) => {
                let inner_transformed = self.transform(inner);
                self.builder().option(inner_transformed)
            }

            // Structural types - recursively transform all parts
            TypeKind::Record(fields) => {
                let fields_transformed = fields.map(|(name, ty)| (name, self.transform(ty)));
                self.builder().record(fields_transformed)
            }
            TypeKind::Function { params, ret } => {
                let params_transformed = params.map(|ty| self.transform(ty));
                let ret_transformed = self.transform(ret);
                self.builder().function(params_transformed, ret_transformed)
            }
            TypeKind::Symbol(parts) => {
                // Symbol parts are just strings, no transformation needed
                self.builder().symbol(parts)
            }
        }
    }
}

/// `TypeVisitor` trait enables traversing type structures without building new types.
///
/// This trait is similar to `TypeTransformer` but for read-only traversals where you
/// don't need to construct new types. It's useful for algorithms that:
/// - Collect information (free variables, occurs check, depth calculation)
/// - Validate properties (well-formedness checks)
/// - Search for patterns
///
/// # Example
///
/// ```ignore
/// struct FreeVarsCollector {
///     vars: HashSet<u16>,
/// }
///
/// impl<'a> TypeVisitor<'a> for FreeVarsCollector {
///     type Input = &'a Type<'a>;
///
///     fn visit(&mut self, ty: Self::Input) {
///         match ty.view() {
///             TypeKind::TypeVar(id) => {
///                 self.vars.insert(id);
///             }
///             _ => self.visit_default(ty),
///         }
///     }
/// }
/// ```
///
/// # Common Use Cases
///
/// - **Free variable collection**: Finding all type variables in a type
/// - **Occurs check**: Detecting if a variable appears in a type
/// - **Depth calculation**: Computing nesting depth
/// - **Validation**: Checking invariants without mutation
pub trait TypeVisitor<'a> {
    /// The input type representation this visitor works with
    type Input: TypeView<'a>;

    /// Visit a type, traversing its structure.
    ///
    /// The default implementation recursively walks the type structure.
    /// Override this method to add custom logic before/after the default traversal.
    fn visit(&mut self, ty: Self::Input) {
        self.visit_default(ty);
    }

    /// Default visitation logic (used by default `visit` and available for
    /// custom implementations that want to delegate some cases).
    ///
    /// This recursively visits all sub-types in depth-first order.
    fn visit_default(&mut self, ty: Self::Input) {
        match ty.view() {
            // Primitives - nothing to visit
            TypeKind::Int
            | TypeKind::Float
            | TypeKind::Bool
            | TypeKind::Str
            | TypeKind::Bytes
            | TypeKind::TypeVar(_) => {}

            // Collections - recursively visit elements
            TypeKind::Array(elem) => {
                self.visit(elem);
            }
            TypeKind::Map(key, val) => {
                self.visit(key);
                self.visit(val);
            }
            TypeKind::Option(inner) => {
                self.visit(inner);
            }

            // Structural types - recursively visit all parts
            TypeKind::Record(fields) => {
                for (_, field_ty) in fields {
                    self.visit(field_ty);
                }
            }
            TypeKind::Function { params, ret } => {
                for param in params {
                    self.visit(param);
                }
                self.visit(ret);
            }
            TypeKind::Symbol(_) => {
                // Symbol parts are just strings, nothing to visit
            }
        }
    }
}

// ============================================================================
// Closure-based Transformer and Visitor
// ============================================================================

/// Closure-based transformer for convenient one-off transformations.
///
/// This provides a convenient way to create transformers without defining a new type.
/// The closure is called for each type node and can return `Some(result)` to replace
/// that node, or `None` to delegate to the default traversal.
///
/// Uses interior mutability (`RefCell`) to allow `FnMut` closures with the `&self` API.
///
/// # Example
///
/// ```ignore
/// // Simple variable remapping
/// let mut mapping = HashMap::new();
/// mapping.insert(0, 100);
/// mapping.insert(1, 101);
///
/// let renamed = ClosureTransformer::new(mgr, |ty| {
///     match ty.view() {
///         TypeKind::TypeVar(id) => mapping.get(&id).map(|&new_id| mgr.typevar(new_id)),
///         _ => None
///     }
/// }).transform(some_type);
/// ```
pub struct ClosureTransformer<'a, B, F>
where
    B: TypeBuilder<'a>,
    B::Repr: TypeView<'a>,
    F: FnMut(B::Repr) -> Option<B::Repr>,
{
    builder: B,
    closure: core::cell::RefCell<F>,
    _phantom: core::marker::PhantomData<&'a ()>,
}

impl<'a, B, F> ClosureTransformer<'a, B, F>
where
    B: TypeBuilder<'a>,
    B::Repr: TypeView<'a>,
    F: FnMut(B::Repr) -> Option<B::Repr>,
{
    /// Create a new closure-based transformer.
    ///
    /// # Arguments
    ///
    /// * `builder` - The type builder to use for constructing types
    /// * `closure` - A closure that returns `Some(type)` to replace a node, or `None` to use default traversal
    pub fn new(builder: B, closure: F) -> Self {
        Self {
            builder,
            closure: core::cell::RefCell::new(closure),
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<'a, B, F> TypeTransformer<'a, B> for ClosureTransformer<'a, B, F>
where
    B: TypeBuilder<'a>,
    B::Repr: TypeView<'a>,
    F: FnMut(B::Repr) -> Option<B::Repr>,
{
    type Input = B::Repr;

    fn builder(&self) -> &B {
        &self.builder
    }

    fn transform(&self, ty: Self::Input) -> B::Repr {
        // Call the closure - if it returns Some, use that; otherwise delegate to default
        if let Some(result) = (self.closure.borrow_mut())(ty) {
            result
        } else {
            self.transform_default(ty)
        }
    }
}

/// Closure-based visitor for convenient one-off traversals.
///
/// This provides a convenient way to create visitors without defining a new type.
/// The closure is called for each type node and should return `true` if it handled
/// the node completely, or `false` to delegate to the default traversal.
///
/// # Example
///
/// ```ignore
/// // Collect all type variables
/// let mut vars = HashSet::new();
/// ClosureVisitor::new(|ty| {
///     match ty.view() {
///         TypeKind::TypeVar(id) => {
///             vars.insert(id);
///             true  // We handled this node
///         }
///         _ => false  // Let default traversal handle it
///     }
/// }).visit(some_type);
/// ```
pub struct ClosureVisitor<'a, V, F>
where
    V: TypeView<'a>,
    F: FnMut(V) -> bool,
{
    closure: F,
    _phantom: core::marker::PhantomData<&'a V>,
}

impl<'a, V, F> ClosureVisitor<'a, V, F>
where
    V: TypeView<'a>,
    F: FnMut(V) -> bool,
{
    /// Create a new closure-based visitor.
    ///
    /// # Arguments
    ///
    /// * `closure` - A closure that returns `true` if it handled the node, `false` to delegate
    pub fn new(closure: F) -> Self {
        Self {
            closure,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<'a, V, F> TypeVisitor<'a> for ClosureVisitor<'a, V, F>
where
    V: TypeView<'a>,
    F: FnMut(V) -> bool,
{
    type Input = V;

    fn visit(&mut self, ty: Self::Input) {
        if !(self.closure)(ty) {
            // If closure returns false, delegate to default
            self.visit_default(ty);
        }
    }
}

// ============================================================================
// Generic Type Display
// ============================================================================

/// Format a type for display, works with any `TypeView` implementation.
///
/// This function produces the same output as `Display for Type`, but works
/// generically over any type representation that implements `TypeView`.
///
/// # Format
///
/// - Primitives: `Int`, `Float`, `Bool`, `Str`, `Bytes`
/// - Type variables: `_0`, `_42`, etc.
/// - Collections: `Array[Int]`, `Map[Str, Int]`, `Option[Int]`
/// - Records: `Record[x: Int, y: Float]`
/// - Functions: `(Int, Float) => Str`
/// - Symbols: `Symbol[foo|bar|baz]`
///
/// # Example
///
/// ```ignore
/// use crate::types::{TypeManager, type_traits::display_type};
/// use bumpalo::Bump;
///
/// let bump = Bump::new();
/// let mgr = TypeManager::new(&bump);
/// let int_ty = mgr.int();
/// let arr_ty = mgr.array(int_ty);
///
/// assert_eq!(display_type(arr_ty), "Array[Int]");
/// ```
pub(super) fn display_type<'a, V: TypeView<'a>>(ty: V) -> alloc::string::String {
    use alloc::string::ToString;

    match ty.view() {
        TypeKind::Int => "Int".to_string(),
        TypeKind::Float => "Float".to_string(),
        TypeKind::Bool => "Bool".to_string(),
        TypeKind::Str => "Str".to_string(),
        TypeKind::Bytes => "Bytes".to_string(),

        TypeKind::TypeVar(id) => alloc::format!("_{id}"),

        TypeKind::Array(elem) => {
            alloc::format!("Array[{}]", display_type(elem))
        }

        TypeKind::Map(key, val) => {
            alloc::format!("Map[{}, {}]", display_type(key), display_type(val))
        }

        TypeKind::Option(inner) => {
            alloc::format!("Option[{}]", display_type(inner))
        }

        TypeKind::Record(fields) => {
            let field_strs: alloc::vec::Vec<alloc::string::String> = fields
                .map(|(name, field_ty)| alloc::format!("{}: {}", name, display_type(field_ty)))
                .collect();
            alloc::format!("Record[{}]", field_strs.join(", "))
        }

        TypeKind::Function { params, ret } => {
            let param_strs: alloc::vec::Vec<alloc::string::String> =
                params.map(|param_ty| display_type(param_ty)).collect();
            alloc::format!("({}) => {}", param_strs.join(", "), display_type(ret))
        }

        TypeKind::Symbol(parts) => {
            let part_strs: alloc::vec::Vec<alloc::string::String> =
                parts.map(alloc::string::ToString::to_string).collect();
            alloc::format!("Symbol[{}]", part_strs.join("|"))
        }
    }
}

#[cfg(test)]
mod tests {
    use bumpalo::Bump;
    use hashbrown::{HashMap, HashSet};

    use super::*;
    use crate::types::Type;
    use crate::types::manager::TypeManager;

    #[test]
    fn closure_transformer_simple_remap() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        // Create a mapping: _0 -> _100, _1 -> _101
        let mut mapping = HashMap::new();
        mapping.insert(0, 100);
        mapping.insert(1, 101);

        // Test remapping a simple type variable
        let var_0 = mgr.type_var(0);
        let transformer = ClosureTransformer::new(mgr, |ty| match ty.view() {
            TypeKind::TypeVar(id) => mapping.get(&id).map(|&new_id| mgr.type_var(new_id)),
            _ => None,
        });

        let result = transformer.transform(var_0);

        if let Type::TypeVar(id) = result {
            assert_eq!(*id, 100);
        } else {
            panic!("Expected TypeVar(100), got {result:?}");
        }
    }

    #[test]
    fn closure_transformer_function() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        // Create mapping: _0 -> _100, _1 -> _101
        let mut mapping = HashMap::new();
        mapping.insert(0, 100);
        mapping.insert(1, 101);

        // Create function type: (_0, _1) => _0
        let var_0 = mgr.type_var(0);
        let var_1 = mgr.type_var(1);
        let func = mgr.function(&[var_0, var_1], var_0);

        let transformer = ClosureTransformer::new(mgr, |ty| match ty.view() {
            TypeKind::TypeVar(id) => mapping.get(&id).map(|&new_id| mgr.type_var(new_id)),
            _ => None,
        });

        let result = transformer.transform(func);

        // Should be: (_100, _101) => _100
        if let Type::Function { params, ret } = result {
            assert_eq!(params.len(), 2);

            if let Type::TypeVar(id) = params[0] {
                assert_eq!(*id, 100);
            } else {
                panic!("Expected TypeVar(100) in params[0]");
            }

            if let Type::TypeVar(id) = params[1] {
                assert_eq!(*id, 101);
            } else {
                panic!("Expected TypeVar(101) in params[1]");
            }

            if let Type::TypeVar(id) = ret {
                assert_eq!(*id, 100);
            } else {
                panic!("Expected TypeVar(100) in ret");
            }

            // Verify pointer equality: params[0] and ret should be the same interned type
            assert!(core::ptr::eq(params[0], *ret));
        } else {
            panic!("Expected Function type, got {result:?}");
        }
    }

    #[test]
    fn closure_transformer_nested() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        // Create mapping: _0 -> _100
        let mut mapping = HashMap::new();
        mapping.insert(0, 100);

        // Create nested type: Map[_0, Array[_0]]
        let var_0 = mgr.type_var(0);
        let arr = mgr.array(var_0);
        let map = mgr.map(var_0, arr);

        let transformer = ClosureTransformer::new(mgr, |ty| match ty.view() {
            TypeKind::TypeVar(id) => mapping.get(&id).map(|&new_id| mgr.type_var(new_id)),
            _ => None,
        });

        let result = transformer.transform(map);

        // Should be: Map[_100, Array[_100]]
        if let Type::Map(key, val) = result {
            if let Type::TypeVar(id) = key {
                assert_eq!(*id, 100);
            } else {
                panic!("Expected TypeVar(100) in key");
            }

            if let Type::Array(elem) = val {
                if let Type::TypeVar(id) = elem {
                    assert_eq!(*id, 100);
                } else {
                    panic!("Expected TypeVar(100) in array elem");
                }

                // Verify pointer equality: key and array elem should be the same
                assert!(core::ptr::eq(*key, *elem));
            } else {
                panic!("Expected Array in val");
            }
        } else {
            panic!("Expected Map type, got {result:?}");
        }
    }

    #[test]
    fn closure_visitor_collect_vars() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        // Create type: (_0, _1) => Map[_2, _0]
        let var_0 = mgr.type_var(0);
        let var_1 = mgr.type_var(1);
        let var_2 = mgr.type_var(2);
        let map = mgr.map(var_2, var_0);
        let func = mgr.function(&[var_0, var_1], map);

        let mut vars = HashSet::new();
        let mut visitor = ClosureVisitor::new(|ty: &Type| match ty.view() {
            TypeKind::TypeVar(id) => {
                vars.insert(id);
                true // We handled this node
            }
            _ => false, // Let default traversal handle it
        });

        visitor.visit(func);

        // Should have collected all three variables
        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&0));
        assert!(vars.contains(&1));
        assert!(vars.contains(&2));
    }

    #[test]
    fn closure_visitor_count_nodes() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        // Create type: Array[Map[Int, Float]]
        let int_ty = mgr.int();
        let float_ty = mgr.float();
        let map = mgr.map(int_ty, float_ty);
        let arr = mgr.array(map);

        let mut count = 0;
        let mut visitor = ClosureVisitor::new(|_ty: &Type| {
            count += 1;
            false // Always traverse
        });

        visitor.visit(arr);

        // Should visit: Array, Map, Int, Float = 4 nodes
        assert_eq!(count, 4);
    }

    #[test]
    fn closure_visitor_early_stop() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        // Create type: Map[Int, Array[Float]]
        let int_ty = mgr.int();
        let float_ty = mgr.float();
        let arr = mgr.array(float_ty);
        let map = mgr.map(int_ty, arr);

        let mut found_array = false;
        let mut visitor = ClosureVisitor::new(|ty: &Type| match ty.view() {
            TypeKind::Array(_) => {
                found_array = true;
                true // Stop traversal at array (don't visit Float)
            }
            _ => false,
        });

        visitor.visit(map);

        assert!(found_array);
        // The visitor should have stopped at Array without visiting Float
        // We can verify this by counting visits
    }

    #[test]
    fn closure_transformer_vs_manual_implementation() {
        // This test demonstrates the ergonomic improvement of ClosureTransformer
        // over manual trait implementation
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        let var_0 = mgr.type_var(0);
        let var_1 = mgr.type_var(1);
        let func = mgr.function(&[var_0, var_1], var_0);

        // Using ClosureTransformer - clean and concise
        let result1 = ClosureTransformer::new(mgr, |ty| match ty.view() {
            TypeKind::TypeVar(0) => Some(mgr.type_var(100)),
            TypeKind::TypeVar(1) => Some(mgr.type_var(101)),
            _ => None,
        })
        .transform(func);

        // Manual implementation would require:
        // 1. Define a struct with builder and mapping
        // 2. Implement TypeTransformer trait
        // 3. Override transform method
        // 4. Create instance and call transform
        // ClosureTransformer does all this in one expression!

        if let Type::Function { params, ret } = result1 {
            if let Type::TypeVar(id) = params[0] {
                assert_eq!(*id, 100);
            }
            if let Type::TypeVar(id) = params[1] {
                assert_eq!(*id, 101);
            }
            if let Type::TypeVar(id) = ret {
                assert_eq!(*id, 100);
            }
        } else {
            panic!("Expected Function type");
        }
    }
}
