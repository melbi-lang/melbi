#![allow(unsafe_code)] // TODO: Disallow unsafe code.

use crate::{
    String, ToString, Vec,
    syntax::{
        bytes_literal::{QuoteStyle as BytesQuoteStyle, escape_bytes},
        string_literal::{QuoteStyle, escape_string},
    },
    types::{Type, manager::TypeManager, traits::TypeView},
    values::{
        binder::{self, Binder},
        from_raw::TypeError,
        function::Function,
        raw::{ArrayData, MapData, MapEntry, RawValue, RecordData, Slice},
    },
};

use alloc::collections::BTreeMap;
use bumpalo::Bump;

#[derive(Clone, Copy)]
pub struct Value<'ty_arena: 'value_arena, 'value_arena> {
    pub ty: &'ty_arena Type<'ty_arena>,
    // Keep these private - the abstraction should not leak!
    // Use constructors (int, float, str, etc.) and extractors (as_int, as_float, etc.)
    raw: RawValue,
    _phantom: core::marker::PhantomData<&'value_arena ()>,
}

impl<'ty_arena: 'value_arena, 'value_arena> PartialEq for Value<'ty_arena, 'value_arena> {
    fn eq(&self, other: &Self) -> bool {
        use crate::types::traits::TypeKind;

        // Use TypeView for type comparison (works for all type storage methods, not just interned)
        // Type implements TypeView which implements Eq
        if self.ty != other.ty {
            return false;
        }

        // Now compare values based on type using TypeView
        let self_type_view = self.ty.view();
        match self_type_view {
            TypeKind::Int => {
                // Use safe extraction methods instead of unsafe
                self.as_int().unwrap() == other.as_int().unwrap()
            }
            TypeKind::Float => {
                // Standard float equality: NaN != NaN
                self.as_float().unwrap() == other.as_float().unwrap()
            }
            TypeKind::Bool => self.as_bool().unwrap() == other.as_bool().unwrap(),
            TypeKind::Str => self.as_str().unwrap() == other.as_str().unwrap(),
            TypeKind::Bytes => self.as_bytes().unwrap() == other.as_bytes().unwrap(),
            TypeKind::Array(_) => {
                let a = self.as_array().unwrap();
                let b = other.as_array().unwrap();

                // Check length first
                if a.len() != b.len() {
                    return false;
                }

                // Compare elements recursively
                for i in 0..a.len() {
                    if a.get(i) != b.get(i) {
                        return false;
                    }
                }
                true
            }
            TypeKind::Record(_) => {
                let a = self.as_record().unwrap();
                let b = other.as_record().unwrap();

                // Check field count (redundant but fast early exit)
                if a.len() != b.len() {
                    return false;
                }

                // Compare field values only (same type implies same field names in same order)
                for (a_field, b_field) in a.iter().zip(b.iter()) {
                    if a_field.1 != b_field.1 {
                        return false;
                    }
                }
                true
            }
            TypeKind::Map(_, _) => {
                let a = self.as_map().unwrap();
                let b = other.as_map().unwrap();

                // Check length first
                if a.len() != b.len() {
                    return false;
                }

                // Compare all key-value pairs (maps are sorted, so iterate in order)
                for ((a_key, a_val), (b_key, b_val)) in a.iter().zip(b.iter()) {
                    if a_key != b_key || a_val != b_val {
                        return false;
                    }
                }
                true
            }
            TypeKind::Option(_) => {
                // Extract both options and compare
                let self_opt = self.as_option().unwrap();
                let other_opt = other.as_option().unwrap();

                match (self_opt, other_opt) {
                    (None, None) => true,
                    (Some(v1), Some(v2)) => v1 == v2,
                    _ => false,
                }
            }
            TypeKind::Function { .. } => {
                // Functions use reference equality
                core::ptr::eq(
                    self.raw.as_function_unchecked(),
                    other.raw.as_function_unchecked(),
                )
            }
            TypeKind::Symbol(_) => {
                // Symbols are interned, so compare by id.
                self.raw.id() == other.raw.id()
            }
            TypeKind::TypeVar(_) => {
                // TypeVars shouldn't appear at runtime, but compare by id
                self.raw.id() == other.raw.id()
            }
        }
    }
}

impl<'ty_arena: 'value_arena, 'value_arena> Eq for Value<'ty_arena, 'value_arena> {}

impl<'ty_arena: 'value_arena, 'value_arena> PartialOrd for Value<'ty_arena, 'value_arena> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'ty_arena: 'value_arena, 'value_arena> Ord for Value<'ty_arena, 'value_arena> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        use crate::types::traits::{TypeKind, TypeView};
        use core::cmp::Ordering;

        // Compare types first using TypeView
        let self_view = self.ty.view();
        let other_view = other.ty.view();

        // Compare type ordering using centralized discriminant method
        match self_view.discriminant().cmp(&other_view.discriminant()) {
            Ordering::Equal => {} // Same type, compare values
            ord => return ord,
        }

        // Same type, compare values
        match self_view {
            TypeKind::Int => self.as_int().unwrap().cmp(&other.as_int().unwrap()),
            TypeKind::Float => {
                // Use total_cmp for NaN-safe total ordering
                // NaN sorts greater than all other values
                self.as_float()
                    .unwrap()
                    .total_cmp(&other.as_float().unwrap())
            }
            TypeKind::Bool => self.as_bool().unwrap().cmp(&other.as_bool().unwrap()),
            TypeKind::Str => self.as_str().unwrap().cmp(other.as_str().unwrap()),
            TypeKind::Bytes => self.as_bytes().unwrap().cmp(other.as_bytes().unwrap()),
            TypeKind::Array(_) => {
                // Lexicographic comparison
                let a = self.as_array().unwrap();
                let b = other.as_array().unwrap();

                for i in 0..a.len().min(b.len()) {
                    match a.get(i).unwrap().cmp(&b.get(i).unwrap()) {
                        Ordering::Equal => continue,
                        ord => return ord,
                    }
                }
                // If all compared elements are equal, shorter array is less
                a.len().cmp(&b.len())
            }
            TypeKind::Record(_) => {
                // Lexicographic comparison of field values
                let a = self.as_record().unwrap();
                let b = other.as_record().unwrap();

                for (a_field, b_field) in a.iter().zip(b.iter()) {
                    match a_field.1.cmp(&b_field.1) {
                        Ordering::Equal => continue,
                        ord => return ord,
                    }
                }
                // If all compared fields are equal, compare length
                a.len().cmp(&b.len())
            }
            TypeKind::Map(_, _) => {
                // Lexicographic comparison of key-value pairs
                let a = self.as_map().unwrap();
                let b = other.as_map().unwrap();

                for ((a_key, a_val), (b_key, b_val)) in a.iter().zip(b.iter()) {
                    // Compare keys first
                    match a_key.cmp(&b_key) {
                        Ordering::Equal => {
                            // If keys equal, compare values
                            match a_val.cmp(&b_val) {
                                Ordering::Equal => continue,
                                ord => return ord,
                            }
                        }
                        ord => return ord,
                    }
                }
                // If all compared pairs are equal, compare length
                a.len().cmp(&b.len())
            }
            TypeKind::Option(_) => {
                // Extract both options and compare
                let self_opt = self.as_option().unwrap();
                let other_opt = other.as_option().unwrap();

                match (self_opt, other_opt) {
                    (None, None) => Ordering::Equal,
                    (None, Some(_)) => Ordering::Less, // None < Some
                    (Some(_), None) => Ordering::Greater, // Some > None
                    (Some(v1), Some(v2)) => v1.cmp(&v2),
                }
            }
            TypeKind::Symbol(_) | TypeKind::Function { .. } | TypeKind::TypeVar(_) => {
                // For these types, use pointer ordering as fallback
                // This gives a consistent (but arbitrary) ordering
                self.raw.id().cmp(&other.raw.id())
            }
        }
    }
}

/// Canonicalize a float for hashing to maintain Hash/Eq invariant.
///
/// - Maps -0.0 to +0.0 (since -0.0 == +0.0)
/// - Maps all NaN representations to a single canonical NaN
fn canonical_f64(value: f64) -> u64 {
    if value.is_nan() {
        // Use a canonical NaN representation
        f64::NAN.to_bits()
    } else if value == 0.0 {
        // Map both +0.0 and -0.0 to +0.0
        0.0_f64.to_bits()
    } else {
        value.to_bits()
    }
}

impl<'ty_arena: 'value_arena, 'value_arena> core::hash::Hash for Value<'ty_arena, 'value_arena> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        use crate::types::traits::TypeKind;

        // Get type view once
        let type_view = self.ty.view();

        // Use discriminant from TypeView for consistency across type storage methods
        core::mem::discriminant(&type_view).hash(state);

        match type_view {
            TypeKind::Int => {
                // Use safe extraction method
                self.as_int().unwrap().hash(state);
            }
            TypeKind::Float => {
                // Use canonical representation to maintain Hash/Eq invariant:
                // - +0.0 and -0.0 must hash the same (since +0.0 == -0.0)
                // - All NaN values should hash the same
                let value = self.as_float().unwrap();
                canonical_f64(value).hash(state);
            }
            TypeKind::Bool => {
                self.as_bool().unwrap().hash(state);
            }
            TypeKind::Str => {
                self.as_str().unwrap().hash(state);
            }
            TypeKind::Bytes => {
                self.as_bytes().unwrap().hash(state);
            }
            TypeKind::Array(_) => {
                let array = self.as_array().unwrap();
                // Hash length first
                array.len().hash(state);
                // Then hash each element recursively
                for elem in array.iter() {
                    elem.hash(state);
                }
            }
            TypeKind::Symbol(_) => {
                // Symbols are interned, so hash the ID
                self.raw.id().hash(state);
            }
            TypeKind::Record(_) => {
                // Records must use structural hashing to maintain Hash/Eq invariant
                // Even though Record is not Hashable per type class design, we need
                // to maintain the invariant: if a == b then hash(a) == hash(b)
                let record = self.as_record().unwrap();
                // Hash length
                record.len().hash(state);
                // Hash field values only (same type implies same field names in same order)
                for (_field_name, field_value) in record.iter() {
                    field_value.hash(state);
                }
            }
            TypeKind::Map(_, _) => {
                // Maps use structural hashing: hash length and all key-value pairs in order
                // Since maps are sorted, equal maps will hash identically
                let map = self.as_map().unwrap();
                map.len().hash(state);
                for (key, value) in map.iter() {
                    key.hash(state);
                    value.hash(state);
                }
            }
            TypeKind::Option(_) => {
                // Hash Option value structurally
                let opt = self.as_option().unwrap();
                match opt {
                    None => {
                        // Hash a marker for None
                        0u8.hash(state);
                    }
                    Some(value) => {
                        // Hash a marker for Some, then the value
                        1u8.hash(state);
                        value.hash(state);
                    }
                }
            }
            TypeKind::Function { .. } | TypeKind::TypeVar(_) => {
                // These types are not Hashable according to our type class design
                self.raw.id().hash(state);
            }
        }
    }
}

impl<'ty_arena: 'value_arena, 'value_arena> core::fmt::Debug for Value<'ty_arena, 'value_arena> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.ty {
            Type::Int => {
                let value = self.raw.as_int_unchecked();
                write!(f, "{}", value)
            }
            Type::Float => {
                let value = self.raw.as_float_unchecked();
                format_float(f, value)
            }
            Type::Bool => {
                let value = self.raw.as_bool_unchecked();
                write!(f, "{}", value)
            }
            Type::Str => {
                let s = self.as_str().unwrap();
                escape_string(f, s, QuoteStyle::default())
            }
            Type::Bytes => {
                let bytes = self.as_bytes().unwrap();
                escape_bytes(f, bytes, BytesQuoteStyle::default())
            }
            Type::Array(_) => {
                let array = self.as_array().unwrap();
                write!(f, "[")?;
                for (i, elem) in array.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", elem)?;
                }
                write!(f, "]")
            }
            Type::Map(_, _) => {
                let map = self.as_map().unwrap();
                write!(f, "{{")?;
                for (i, (key, value)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}: {:?}", key, value)?;
                }
                write!(f, "}}")
            }
            Type::Record(_) => {
                let record = self.as_record().unwrap();
                write!(f, "{{")?;
                for (i, (field_name, field_value)) in record.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} = {:?}", field_name, field_value)?;
                }
                write!(f, "}}")
            }
            Type::Function { .. } => {
                let ptr = self.raw.as_function_unchecked() as *const _;
                write!(f, "<Function @ {:p}: {}>", ptr as *const (), self.ty)
            }
            Type::Symbol(_) => {
                // TODO: Implement proper Symbol display (e.g., show symbol name or value)
                // For now, print a placeholder with the pointer address
                let id = self.raw.id();
                write!(f, "<Symbol@{:p}>", id as *const ())
            }
            Type::TypeVar(_) => {
                // TODO: Implement proper TypeVar display (e.g., show type variable name)
                // For now, print a placeholder with the pointer address
                write!(f, "<TypeVar@{:p}>", self.raw.id() as *const ())
            }
            Type::Option(_) => {
                // Display Option value properly
                let opt = self.as_option().unwrap();
                match opt {
                    None => write!(f, "None"),
                    Some(value) => write!(f, "Some({:?})", value),
                }
            }
        }
    }
}

impl<'ty_arena: 'value_arena, 'value_arena> core::fmt::Display for Value<'ty_arena, 'value_arena> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.ty {
            // Primitives: use native Display (no quotes, respects format flags)
            Type::Int => {
                let value = self.raw.as_int_unchecked();
                write!(f, "{}", value)
            }
            Type::Float => {
                let value = self.raw.as_float_unchecked();
                write!(f, "{}", value)
            }
            Type::Bool => {
                let value = self.raw.as_bool_unchecked();
                write!(f, "{}", value)
            }
            Type::Str => {
                let s = self.as_str().unwrap();
                write!(f, "{}", s)
            }

            // Complex types and Bytes: delegate to Debug
            _ => write!(f, "{:?}", self),
        }
    }
}

/// Format a float ensuring it always has a decimal point (Melbi requirement)
fn format_float(f: &mut core::fmt::Formatter<'_>, value: f64) -> core::fmt::Result {
    if value.is_nan() {
        write!(f, "nan")
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            write!(f, "inf")
        } else {
            write!(f, "-inf")
        }
    } else {
        let s = value.to_string();
        if s.contains('.') || s.contains('e') || s.contains('E') {
            write!(f, "{}", s)
        } else {
            write!(f, "{}.", s)
        }
    }
}

impl<'ty_arena: 'value_arena, 'value_arena> Value<'ty_arena, 'value_arena> {
    // ============================================================================
    // Raw Value Access
    // ============================================================================

    /// Get the underlying RawValue.
    ///
    /// This is useful for the bytecode compiler which needs to convert Values
    /// (with type information) to RawValues (for VM execution).
    ///
    /// # Safety
    /// The returned RawValue is a union - accessing its fields requires knowing
    /// the correct type. Use the type information in `self.ty` or the type-safe
    /// extractors (`as_int()`, `as_float()`, etc.) instead when possible.
    pub fn as_raw(&self) -> RawValue {
        self.raw
    }

    /// Construct a Value from a type and raw value (for testing and VM results).
    ///
    /// # Safety
    /// The caller must ensure that the RawValue matches the given Type.
    /// This is primarily intended for converting VM execution results back to Values
    /// or macro-generated code where type safety is guaranteed by the type system.
    pub fn from_raw_unchecked(ty: &'ty_arena Type<'ty_arena>, raw: RawValue) -> Self {
        Self {
            ty,
            raw,
            _phantom: core::marker::PhantomData,
        }
    }

    // ============================================================================
    // Safe Construction API - Primitives (simple values, no allocation)
    // ============================================================================
    //
    // Simple values take TypeManager (not Type) and don't return Result.
    // They can't fail because the value always matches the type.

    /// Create an integer value.
    ///
    /// Type is inferred from TypeManager. No allocation needed.
    pub fn int(type_mgr: &'ty_arena TypeManager<'ty_arena>, value: i64) -> Self {
        Self {
            ty: type_mgr.int(),
            raw: RawValue::make_int(value),
            _phantom: core::marker::PhantomData,
        }
    }

    /// Create a float value.
    ///
    /// Type is inferred from TypeManager. No allocation needed.
    pub fn float(type_mgr: &'ty_arena TypeManager<'ty_arena>, value: f64) -> Self {
        Self {
            ty: type_mgr.float(),
            raw: RawValue::make_float(value),
            _phantom: core::marker::PhantomData,
        }
    }

    /// Create a boolean value.
    ///
    /// Type is inferred from TypeManager. No allocation needed.
    pub fn bool(type_mgr: &'ty_arena TypeManager<'ty_arena>, value: bool) -> Self {
        Self {
            ty: type_mgr.bool(),
            raw: RawValue::make_bool(value),
            _phantom: core::marker::PhantomData,
        }
    }

    // ============================================================================
    // Safe Construction API - Compound Values (require allocation and validation)
    // ============================================================================
    //
    // Compound values require explicit type and arena, and return Result for validation.

    /// Create a string value.
    ///
    /// Requires arena for allocation and explicit type.
    pub fn str(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        value: &str,
    ) -> Self {
        let data = arena.alloc_slice_copy(value.as_bytes());
        Self {
            ty,
            raw: arena.alloc(Slice::new(arena, data)).as_raw_value(),
            _phantom: core::marker::PhantomData,
        }
    }

    /// Create a bytes value.
    ///
    /// Requires arena for allocation and explicit type.
    pub fn bytes(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        value: &[u8],
    ) -> Self {
        let data = arena.alloc_slice_copy(value);
        Self {
            ty,
            raw: arena.alloc(Slice::new(arena, data)).as_raw_value(),
            _phantom: core::marker::PhantomData,
        }
    }

    /// Create an array value with runtime type validation.
    ///
    /// Type must be Array(elem_ty). All elements must match elem_ty.
    /// Returns error if type is not Array or if any element has wrong type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let arr = Value::array(
    ///     &arena,
    ///     type_mgr.array(type_mgr.int()),
    ///     &[
    ///         Value::int(type_mgr, 1),
    ///         Value::int(type_mgr, 2),
    ///     ]
    /// )?;
    /// ```
    pub fn array(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        elements: &[Value<'ty_arena, 'value_arena>],
    ) -> Result<Self, TypeError> {
        // Validate: ty must be Array(elem_ty)
        let Type::Array(elem_ty) = ty else {
            return Err(TypeError::Mismatch);
        };

        // Validate: all elements match elem_ty
        for elem in elements.iter() {
            if !core::ptr::eq(elem.ty, *elem_ty) {
                return Err(TypeError::Mismatch);
            }
        }

        // Extract raw values
        let raw_values: Vec<RawValue> = elements.iter().map(|v| v.raw).collect();

        // Allocate in arena
        let data = ArrayData::new_with(arena, &raw_values);

        Ok(Self {
            ty,
            raw: data.as_raw_value(),
            _phantom: core::marker::PhantomData,
        })
    }

    /// Create an Option value.
    ///
    /// Type must be Option(inner_ty).
    /// For None: inner should be None
    /// For Some(v): inner should be Some(v) where v.ty == inner_ty
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Create None
    /// let none_val = Value::optional(
    ///     &arena,
    ///     type_mgr.option(type_mgr.int()),
    ///     None
    /// )?;
    ///
    /// // Create Some(42)
    /// let some_val = Value::optional(
    ///     &arena,
    ///     type_mgr.option(type_mgr.int()),
    ///     Some(Value::int(type_mgr, 42))
    /// )?;
    /// ```
    pub fn optional(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        inner: Option<Value<'ty_arena, 'value_arena>>,
    ) -> Result<Self, TypeError> {
        // Validate: ty must be Option(inner_ty)
        let Type::Option(inner_ty) = ty else {
            return Err(TypeError::Mismatch);
        };

        // Validate inner value type if Some
        if let Some(ref value) = inner {
            if !core::ptr::eq(value.ty, *inner_ty) {
                return Err(TypeError::Mismatch);
            }
        }

        // Use RawValue::make_optional to encapsulate memory layout
        let raw = RawValue::make_optional(arena, inner.map(|v| v.raw));

        Ok(Self {
            ty,
            raw,
            _phantom: core::marker::PhantomData,
        })
    }

    /// Create a record value with runtime type validation.
    ///
    /// Type must be Record(fields). Field names and types must match.
    /// Fields must be provided in sorted order by name.
    /// Returns error if type is not Record or if fields don't match.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rec = Value::record(
    ///     &arena,
    ///     type_mgr.record(&[("x", type_mgr.int()), ("y", type_mgr.float())]),
    ///     &[
    ///         ("x", Value::int(type_mgr, 42)),
    ///         ("y", Value::float(type_mgr, 3.14)),
    ///     ]
    /// )?;
    /// ```
    pub fn record(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        fields: &[(&'ty_arena str, Value<'ty_arena, 'value_arena>)],
    ) -> Result<Self, TypeError> {
        // Validate: ty must be Record(field_types)
        let Type::Record(field_types) = ty else {
            return Err(TypeError::Mismatch);
        };

        // Validate: field count matches
        if fields.len() != field_types.len() {
            return Err(TypeError::Mismatch);
        }

        // Validate: field names and types match (both are sorted)
        for (i, (field_name, field_value)) in fields.iter().enumerate() {
            let (expected_name, expected_ty) = field_types[i];
            if *field_name != expected_name {
                return Err(TypeError::Mismatch);
            }
            if !core::ptr::eq(field_value.ty, expected_ty) {
                return Err(TypeError::Mismatch);
            }
        }

        // Extract raw values
        let raw_values: Vec<RawValue> = fields.iter().map(|(_, v)| v.raw).collect();

        // Allocate in arena
        let data = RecordData::new_with(arena, &raw_values);

        Ok(Self {
            ty,
            raw: data.as_raw_value(),
            _phantom: core::marker::PhantomData,
        })
    }

    /// Create a map value with runtime type validation.
    ///
    /// The map will store key-value pairs in sorted order by key for efficient
    /// binary search lookups. Keys will be sorted using Value::cmp.
    ///
    /// # Arguments
    ///
    /// * `arena` - Arena allocator for map storage
    /// * `ty` - Must be a Map type `Map(key_ty, value_ty)`
    /// * `pairs` - Key-value pairs (will be sorted by key)
    ///
    /// # Errors
    ///
    /// Returns TypeError::Mismatch if:
    /// - `ty` is not a Map type
    /// - Any key doesn't match the map's key type
    /// - Any value doesn't match the map's value type
    ///
    /// # Example
    ///
    /// ```ignore
    /// let key1 = Value::int(type_mgr, 1);
    /// let val1 = Value::str(arena, type_mgr, "one");
    /// let key2 = Value::int(type_mgr, 2);
    /// let val2 = Value::str(arena, type_mgr, "two");
    ///
    /// let map_ty = type_mgr.map(type_mgr.int(), type_mgr.str());
    /// let map = Value::map(arena, map_ty, &[(key1, val1), (key2, val2)])?;
    /// ```
    pub fn map(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        pairs: &[(
            Value<'ty_arena, 'value_arena>,
            Value<'ty_arena, 'value_arena>,
        )],
    ) -> Result<Self, TypeError> {
        // Validate: ty must be Map(key_ty, value_ty)
        let Type::Map(key_ty, value_ty) = ty else {
            return Err(TypeError::Mismatch);
        };

        // Validate: all keys match key_ty and all values match value_ty
        for (key, value) in pairs.iter() {
            if !core::ptr::eq(key.ty, *key_ty) {
                return Err(TypeError::Mismatch);
            }
            if !core::ptr::eq(value.ty, *value_ty) {
                return Err(TypeError::Mismatch);
            }
        }

        // Sort pairs by key using Value::cmp
        let mut sorted_pairs: Vec<(
            Value<'ty_arena, 'value_arena>,
            Value<'ty_arena, 'value_arena>,
        )> = pairs.to_vec();
        sorted_pairs.sort_by(|a, b| a.0.cmp(&b.0));

        // Deduplicate keys, keeping the last value for each key
        let mut deduplicated: Vec<(
            Value<'ty_arena, 'value_arena>,
            Value<'ty_arena, 'value_arena>,
        )> = Vec::new();
        for (key, value) in sorted_pairs {
            if let Some(last) = deduplicated.last() {
                if last.0 == key {
                    // Same key as previous - replace the value
                    deduplicated.pop();
                }
            }
            deduplicated.push((key, value));
        }

        // Convert to MapEntry structs
        let entries: Vec<MapEntry> = deduplicated
            .iter()
            .map(|(key, value)| MapEntry {
                key: key.raw,
                value: value.raw,
            })
            .collect();

        // Allocate in arena
        let data = MapData::new_with_sorted(arena, &entries);

        Ok(Self {
            ty,
            raw: data.as_raw_value(),
            _phantom: core::marker::PhantomData,
        })
    }

    /// Create a function value.
    ///
    /// The function's type is obtained from `func.ty()` and must be a Function type.
    /// The function is allocated in the arena and can be called through the evaluator.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::values::function::NativeFunction;
    ///
    /// let func_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
    /// let value = Value::function(&arena, NativeFunction::new(func_ty, add_function))?;
    /// ```
    pub fn function<T: Function<'ty_arena, 'value_arena> + 'value_arena>(
        arena: &'value_arena bumpalo::Bump,
        func: T,
    ) -> Result<Self, TypeError> {
        // Get the type from the allocated function
        let ty = func.ty();
        // Validate: ty must be Function
        let Type::Function { .. } = ty else {
            return Err(TypeError::Mismatch);
        };

        let raw = RawValue::make_function(arena, func);

        Ok(Self {
            ty,
            raw,
            _phantom: core::marker::PhantomData,
        })
    }

    // ============================================================================
    // Dynamic Extraction API
    // ============================================================================
    //
    // These methods extract values without requiring compile-time type knowledge.

    /// Get the raw value representation.
    ///
    /// This is useful for zero-cost conversions in macro-generated code.
    /// Users of this method must ensure type safety manually.
    #[inline]
    pub fn raw(&self) -> RawValue {
        self.raw
    }

    /// Extract integer value dynamically.
    ///
    /// Returns error if value is not an Int.
    pub fn as_int(&self) -> Result<i64, TypeError> {
        match self.ty {
            Type::Int => Ok(self.raw.as_int_unchecked()),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract float value dynamically.
    ///
    /// Returns error if value is not a Float.
    pub fn as_float(&self) -> Result<f64, TypeError> {
        match self.ty {
            Type::Float => Ok(self.raw.as_float_unchecked()),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract boolean value dynamically.
    ///
    /// Returns error if value is not a Bool.
    pub fn as_bool(&self) -> Result<bool, TypeError> {
        match self.ty {
            Type::Bool => Ok(self.raw.as_bool_unchecked()),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract string value dynamically.
    ///
    /// Returns error if value is not a Str.
    pub fn as_str(&self) -> Result<&str, TypeError> {
        match self.ty {
            Type::Str => Ok(self.raw.as_str_unchecked()),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract bytes value dynamically.
    ///
    /// Returns error if value is not Bytes.
    pub fn as_bytes(&self) -> Result<&[u8], TypeError> {
        match self.ty {
            Type::Bytes => Ok(self.raw.as_bytes_unchecked()),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Get dynamic array view.
    ///
    /// Returns Array wrapper that allows iteration and indexing
    /// without compile-time type knowledge.
    pub fn as_array(&self) -> Result<Array<'ty_arena, 'value_arena>, TypeError> {
        match self.ty {
            Type::Array(elem_ty) => Ok(Array {
                elem_ty,
                data: ArrayData::from_raw_value(self.raw),
                _phantom: core::marker::PhantomData,
            }),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Get dynamic record view.
    ///
    /// Returns Record wrapper that allows field access and iteration
    /// without compile-time type knowledge.
    pub fn as_record(&self) -> Result<Record<'ty_arena, 'value_arena>, TypeError> {
        match self.ty {
            Type::Record(field_types) => Ok(Record {
                field_types,
                data: RecordData::from_raw_value(self.raw),
                _phantom: core::marker::PhantomData,
            }),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract a Map from this value, or return a TypeError if not a map.
    pub fn as_map(&self) -> Result<Map<'ty_arena, 'value_arena>, TypeError> {
        match self.ty {
            Type::Map(key_ty, value_ty) => Ok(Map {
                key_ty,
                value_ty,
                data: MapData::from_raw_value(self.raw),
                _phantom: core::marker::PhantomData,
            }),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract an Option value dynamically.
    ///
    /// Returns None for none, or Some(inner_value) for some.
    /// Returns error if value is not an Option.
    pub fn as_option(&self) -> Result<Option<Value<'ty_arena, 'value_arena>>, TypeError> {
        match self.ty {
            Type::Option(inner_ty) => Ok(self.raw.as_optional_unchecked().map(|raw| Value {
                ty: inner_ty,
                raw: raw,
                _phantom: core::marker::PhantomData,
            })),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract function trait object dynamically.
    ///
    /// Returns reference to Function trait object if value is a Function.
    /// Returns error if value is not a Function.
    ///
    /// TODO: Consider adding a checked wrapper API that provides runtime validation.
    pub fn as_function(
        &self,
    ) -> Result<&'value_arena dyn Function<'ty_arena, 'value_arena>, TypeError> {
        match self.ty {
            Type::Function { .. } => Ok(self.raw.as_function_unchecked()),
            _ => Err(TypeError::Mismatch),
        }
    }
}

// ============================================================================
// Array - Runtime array access without compile-time type knowledge
// ============================================================================

/// Dynamic view of an array that doesn't require compile-time element type.
///
/// Allows iteration and indexing, returning elements as `Value`.
pub struct Array<'ty_arena, 'value_arena> {
    elem_ty: &'ty_arena Type<'ty_arena>,
    data: ArrayData<'value_arena>,
    _phantom: core::marker::PhantomData<&'value_arena ()>,
}

impl<'ty_arena, 'value_arena> Array<'ty_arena, 'value_arena> {
    /// Get the number of elements in the array.
    pub fn len(&self) -> usize {
        self.data.length()
    }

    /// Check if the array is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get element at index, returning it as a Value.
    ///
    /// Returns None if index is out of bounds.
    pub fn get(&self, index: usize) -> Option<Value<'ty_arena, 'value_arena>> {
        if index >= self.len() {
            return None;
        }

        let raw = unsafe { self.data.get_unchecked(index) };
        Some(Value {
            ty: self.elem_ty,
            raw,
            _phantom: core::marker::PhantomData,
        })
    }

    /// Iterate over elements as Values.
    pub fn iter(&self) -> ArrayIter<'_, 'ty_arena, 'value_arena> {
        let start = self.data.as_data_ptr();
        let end = unsafe { start.add(self.len()) };
        ArrayIter {
            elem_ty: self.elem_ty,
            current: start,
            end,
            _phantom: core::marker::PhantomData,
        }
    }
}

/// Iterator over Array elements.
///
/// Uses start/end pointer strategy like C++ iterators for efficient iteration
/// without repeated bounds checks.
pub struct ArrayIter<'a, 'ty_arena, 'value_arena> {
    elem_ty: &'ty_arena Type<'ty_arena>,
    current: *const RawValue,
    end: *const RawValue,
    _phantom: core::marker::PhantomData<&'a Array<'ty_arena, 'value_arena>>,
}

impl<'a, 'ty_arena: 'value_arena, 'value_arena> Iterator
    for ArrayIter<'a, 'ty_arena, 'value_arena>
{
    type Item = Value<'ty_arena, 'value_arena>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.end {
            return None;
        }

        let raw = unsafe { *self.current };
        self.current = unsafe { self.current.add(1) };

        Some(Value {
            ty: self.elem_ty,
            raw,
            _phantom: core::marker::PhantomData,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = unsafe { self.end.offset_from(self.current) as usize };
        (remaining, Some(remaining))
    }
}

impl<'a, 'ty_arena: 'value_arena, 'value_arena> ExactSizeIterator
    for ArrayIter<'a, 'ty_arena, 'value_arena>
{
    fn len(&self) -> usize {
        unsafe { self.end.offset_from(self.current) as usize }
    }
}

// ============================================================================
// Record - Runtime record access without compile-time type knowledge
// ============================================================================

/// Dynamic view of a record that doesn't require compile-time field types.
///
/// Allows field access by name and iteration over fields.
pub struct Record<'ty_arena, 'value_arena> {
    field_types: &'ty_arena [(&'ty_arena str, &'ty_arena Type<'ty_arena>)],
    data: RecordData<'value_arena>,
    _phantom: core::marker::PhantomData<&'value_arena ()>,
}

impl<'ty_arena, 'value_arena> Record<'ty_arena, 'value_arena> {
    /// Get the number of fields in the record.
    pub fn len(&self) -> usize {
        self.field_types.len()
    }

    /// Check if the record is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get field by name, returning it as a Value.
    ///
    /// Returns None if field name is not found.
    /// Uses binary search since fields are sorted by name.
    pub fn get(&self, field_name: &str) -> Option<Value<'ty_arena, 'value_arena>> {
        // Binary search for field name
        let index = self
            .field_types
            .binary_search_by_key(&field_name, |(name, _)| *name)
            .ok()?;

        let (_, field_ty) = self.field_types[index];
        let raw = unsafe { self.data.get(index) };

        Some(Value {
            ty: field_ty,
            raw,
            _phantom: core::marker::PhantomData,
        })
    }

    /// Iterate over fields as (name, Value) pairs.
    pub fn iter(&self) -> RecordIter<'_, 'ty_arena, 'value_arena> {
        RecordIter {
            field_types: self.field_types,
            data: self.data,
            index: 0,
            _phantom: core::marker::PhantomData,
        }
    }
}

/// Iterator over Record fields.
///
/// Yields (field_name, field_value) pairs in sorted order by field name.
pub struct RecordIter<'a, 'ty_arena, 'value_arena> {
    field_types: &'ty_arena [(&'ty_arena str, &'ty_arena Type<'ty_arena>)],
    data: RecordData<'value_arena>,
    index: usize,
    _phantom: core::marker::PhantomData<&'a Record<'ty_arena, 'value_arena>>,
}

impl<'a, 'ty_arena: 'value_arena, 'value_arena> Iterator
    for RecordIter<'a, 'ty_arena, 'value_arena>
{
    type Item = (&'ty_arena str, Value<'ty_arena, 'value_arena>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.field_types.len() {
            return None;
        }

        let (field_name, field_ty) = self.field_types[self.index];
        let raw = unsafe { self.data.get(self.index) };
        self.index += 1;

        Some((
            field_name,
            Value {
                ty: field_ty,
                raw,
                _phantom: core::marker::PhantomData,
            },
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.field_types.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a, 'ty_arena: 'value_arena, 'value_arena> ExactSizeIterator
    for RecordIter<'a, 'ty_arena, 'value_arena>
{
    fn len(&self) -> usize {
        self.field_types.len() - self.index
    }
}

// ============================================================================
// Map - Immutable sorted key-value mapping
// ============================================================================

/// A dynamically-typed immutable map with runtime type checking.
///
/// Maps store key-value pairs in a sorted array for efficient binary search.
/// All keys must have the same type, and all values must have the same type.
/// Maps are immutable once created.
pub struct Map<'ty_arena, 'value_arena> {
    key_ty: &'ty_arena Type<'ty_arena>,
    value_ty: &'ty_arena Type<'ty_arena>,
    data: MapData<'value_arena>,
    _phantom: core::marker::PhantomData<&'value_arena ()>,
}

impl<'ty_arena, 'value_arena> Map<'ty_arena, 'value_arena> {
    /// Get the number of key-value pairs in the map.
    pub fn len(&self) -> usize {
        self.data.length()
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Look up a value by key using binary search.
    ///
    /// Returns None if the key is not found or if the key has the wrong type.
    pub fn get(
        &self,
        key: &Value<'ty_arena, 'value_arena>,
    ) -> Option<Value<'ty_arena, 'value_arena>> {
        // Type check the key
        if key.ty != self.key_ty {
            return None;
        }

        // Binary search for the key
        let mut low = 0;
        let mut high = self.len();

        while low < high {
            let mid = low + (high - low) / 2;
            let mid_key_raw = unsafe { self.data.get_key(mid) };
            let mid_key = Value {
                ty: self.key_ty,
                raw: mid_key_raw,
                _phantom: core::marker::PhantomData,
            };

            match mid_key.cmp(key) {
                core::cmp::Ordering::Less => low = mid + 1,
                core::cmp::Ordering::Greater => high = mid,
                core::cmp::Ordering::Equal => {
                    // Found the key, return the value
                    let value_raw = unsafe { self.data.get_value(mid) };
                    return Some(Value {
                        ty: self.value_ty,
                        raw: value_raw,
                        _phantom: core::marker::PhantomData,
                    });
                }
            }
        }

        None
    }

    /// Get the key type of this map.
    pub fn key_type(&self) -> &'ty_arena Type<'ty_arena> {
        self.key_ty
    }

    /// Get the value type of this map.
    pub fn value_type(&self) -> &'ty_arena Type<'ty_arena> {
        self.value_ty
    }

    /// Iterate over all key-value pairs in the map.
    ///
    /// Pairs are returned in sorted order by key.
    pub fn iter(&self) -> MapIter<'_, 'ty_arena, 'value_arena> {
        MapIter {
            key_ty: self.key_ty,
            value_ty: self.value_ty,
            data: self.data,
            index: 0,
            _phantom: core::marker::PhantomData,
        }
    }
}

/// Iterator over map key-value pairs.
pub struct MapIter<'a, 'ty_arena, 'value_arena> {
    key_ty: &'ty_arena Type<'ty_arena>,
    value_ty: &'ty_arena Type<'ty_arena>,
    data: MapData<'value_arena>,
    index: usize,
    _phantom: core::marker::PhantomData<&'a Map<'ty_arena, 'value_arena>>,
}

impl<'a, 'ty_arena: 'value_arena, 'value_arena> Iterator for MapIter<'a, 'ty_arena, 'value_arena> {
    type Item = (
        Value<'ty_arena, 'value_arena>,
        Value<'ty_arena, 'value_arena>,
    );

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.data.length() {
            return None;
        }

        let key_raw = unsafe { self.data.get_key(self.index) };
        let value_raw = unsafe { self.data.get_value(self.index) };
        self.index += 1;

        Some((
            Value {
                ty: self.key_ty,
                raw: key_raw,
                _phantom: core::marker::PhantomData,
            },
            Value {
                ty: self.value_ty,
                raw: value_raw,
                _phantom: core::marker::PhantomData,
            },
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.data.length() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a, 'ty_arena: 'value_arena, 'value_arena> ExactSizeIterator
    for MapIter<'a, 'ty_arena, 'value_arena>
{
    fn len(&self) -> usize {
        self.data.length() - self.index
    }
}

// ============================================================================
// RecordBuilder - Ergonomic API for building records
// ============================================================================

/// Builder for constructing records with automatic field sorting.
///
/// Records require fields to be sorted alphabetically, which can be tedious
/// to manage manually. RecordBuilder handles sorting automatically and
/// infers the record type from the accumulated fields.
///
/// # Example
///
/// ```ignore
/// use melbi_core::values::binder::Binder;
///
/// let rec = RecordBuilder::new(type_mgr)
///     .bind("y", Value::float(type_mgr, 3.14))
///     .bind("x", Value::int(type_mgr, 42))  // Order doesn't matter
///     .build(&arena)?;  // Fields automatically sorted: x, y
/// ```
pub struct RecordBuilder<'ty_arena: 'value_arena, 'value_arena> {
    arena: &'value_arena Bump,
    type_mgr: &'ty_arena TypeManager<'ty_arena>,
    fields: BTreeMap<String, Value<'ty_arena, 'value_arena>>,
    duplicates: Vec<String>,
}

impl<'ty_arena: 'value_arena, 'value_arena> RecordBuilder<'ty_arena, 'value_arena> {
    /// Create a new RecordBuilder.
    pub fn new(arena: &'value_arena Bump, type_mgr: &'ty_arena TypeManager<'ty_arena>) -> Self {
        Self {
            arena,
            type_mgr,
            fields: BTreeMap::new(),
            duplicates: Vec::new(),
        }
    }
}

impl<'ty_arena: 'value_arena, 'value_arena> Binder<'ty_arena, 'value_arena>
    for RecordBuilder<'ty_arena, 'value_arena>
{
    type Output = Value<'ty_arena, 'value_arena>;

    fn bind(mut self, name: &str, value: Value<'ty_arena, 'value_arena>) -> Self {
        // TODO: Actually error if there is a duplicate key.
        // We could allocate `name` on the arena instead, but creating a record type
        // will already do that, so we store it as a owned String for now.
        if self.fields.insert(name.to_string(), value).is_some() {
            self.duplicates.push(name.to_string());
        }
        self
    }

    fn build(mut self) -> Result<Self::Output, binder::Error> {
        if !self.duplicates.is_empty() {
            return Err(binder::Error::DuplicateBinding(core::mem::take(
                &mut self.duplicates,
            )));
        }

        // Build the type from field names and types
        // The record() method will intern field names
        let field_types: Vec<(&str, &'ty_arena Type<'ty_arena>)> = self
            .fields
            .iter()
            .map(|(name, value)| (name.as_str(), value.ty))
            .collect();

        let record_ty = self.type_mgr.record(field_types);

        // Extract the interned field names from the type to use in Value::record
        let Type::Record(interned_fields) = record_ty else {
            panic!("Created a Record that is not a Record: {:?}", record_ty)
        };

        // Build field values array using interned names from the type
        let field_values: Vec<(&'ty_arena str, Value<'ty_arena, 'value_arena>)> = interned_fields
            .iter()
            .zip(self.fields.iter())
            .map(|((interned_name, _), (_, value))| (*interned_name, *value))
            .collect();

        // Create the record value
        Ok(Value::record(self.arena, record_ty, &field_values).expect("Record should be valid"))
    }
}

impl<'ty_arena: 'value_arena, 'value_arena> Value<'ty_arena, 'value_arena> {
    /// Create a RecordBuilder for constructing records ergonomically.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use melbi_core::values::binder::Binder;
    ///
    /// let rec = Value::record_builder(type_mgr)
    ///     .bind("y", Value::float(type_mgr, 3.14))
    ///     .bind("x", Value::int(type_mgr, 42))
    ///     .build(&arena)?;
    /// ```
    pub fn record_builder(
        arena: &'value_arena Bump,
        type_mgr: &'ty_arena TypeManager<'ty_arena>,
    ) -> RecordBuilder<'ty_arena, 'value_arena> {
        RecordBuilder::new(arena, type_mgr)
    }
}
