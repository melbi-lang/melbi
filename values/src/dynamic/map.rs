use melbi_types::{Ty, TyKind};

use super::Value;
use super::equality::{key_hash, lookup, values_equal};
use crate::traits::{MapStorageMaker, MapView, Val, ValueBuilder, ValueView};

/// A view into a map value.
///
/// Stores the key and value types so that entries can be returned as properly
/// typed [`Value`]s, and so that keys can be hashed and compared.
#[derive(Debug, Clone)]
pub struct Map<B: ValueBuilder> {
    key_ty: Ty<B::TB>,
    value_ty: Ty<B::TB>,
    handle: B::MapHandle,
}

impl<B: ValueBuilder> Map<B> {
    pub(crate) fn new(key_ty: Ty<B::TB>, value_ty: Ty<B::TB>, handle: B::MapHandle) -> Self {
        Self {
            key_ty,
            value_ty,
            handle,
        }
    }
}

impl<B: ValueBuilder> MapView<Value<B>, Value<B>> for Map<B> {
    fn len(&self) -> usize {
        self.handle.len()
    }

    /// Returns `None` if `key` is missing or is not of the map's key type.
    fn get(&self, key: &Value<B>) -> Option<Value<B>> {
        if key.ty() != &self.key_ty {
            return None;
        }
        let value = lookup(&self.handle, &self.key_ty, key.val())?;
        Some(Value::new(self.value_ty.clone(), value.clone()))
    }

    fn iter(&self) -> impl Iterator<Item = (Value<B>, Value<B>)> + '_ {
        self.handle.iter().map(|(key, value)| {
            (
                Value::new(self.key_ty.clone(), key.clone()),
                Value::new(self.value_ty.clone(), value.clone()),
            )
        })
    }
}

/// Builds a map value one entry at a time.
///
/// The key and value types are given up front, so an empty map is correctly
/// typed, and code that does not know its entry count in advance (an FFI
/// function tallying an array, say) can accumulate entries. Inserting a key
/// that is already present replaces its value: the last insertion wins.
///
/// ```
/// use melbi_types::{Scalar, TyKind};
/// use melbi_values::builders::BoxValueBuilder;
/// use melbi_values::dynamic::{MapMaker, Value};
/// use melbi_values::traits::{MapView, ValueBuilder, ValueView};
///
/// let builder = BoxValueBuilder::new();
/// let str_ty = TyKind::Scalar(Scalar::Str).alloc(builder.ty_builder());
/// let int_ty = TyKind::Scalar(Scalar::Int).alloc(builder.ty_builder());
///
/// let mut maker = MapMaker::new(&builder, str_ty, int_ty);
/// maker.insert(Value::str(&builder, "a"), Value::int(&builder, 1));
/// maker.insert(Value::str(&builder, "a"), Value::int(&builder, 2));
/// let map = maker.build();
///
/// let entries = map.as_map().unwrap();
/// assert_eq!(entries.len(), 1);
/// assert_eq!(entries.get(&Value::str(&builder, "a")).unwrap().as_int(), Some(2));
/// ```
pub struct MapMaker<B: ValueBuilder> {
    builder: B,
    key_ty: Ty<B::TB>,
    value_ty: Ty<B::TB>,
    storage: MapStorageMaker<B>,
}

impl<B: ValueBuilder> MapMaker<B> {
    /// Start a map with keys of type `key_ty` and values of type `value_ty`.
    pub fn new(builder: &B, key_ty: Ty<B::TB>, value_ty: Ty<B::TB>) -> Self {
        Self {
            builder: builder.clone(),
            key_ty,
            value_ty,
            storage: MapStorageMaker::new(builder),
        }
    }

    /// Add an entry, replacing the value if the key is already present.
    pub fn insert(&mut self, key: Value<B>, value: Value<B>) {
        debug_assert!(
            *key.ty() == self.key_ty,
            "key must match the map's key type"
        );
        debug_assert!(
            *value.ty() == self.value_ty,
            "value must match the map's value type"
        );
        let key_ty = &self.key_ty;
        let key = key.into_val();
        self.storage.insert(
            key_hash(key_ty, &key),
            key,
            value.into_val(),
            |stored: &Val<B>, new: &Val<B>| values_equal(key_ty, stored, new),
            |stored: &Val<B>| key_hash(key_ty, stored),
        );
    }

    /// Finish the map. Takes `self`, so a map cannot change once built.
    pub fn build(self) -> Value<B> {
        let handle = self.storage.build(&self.builder);
        let ty = TyKind::Map(self.key_ty, self.value_ty).alloc(self.builder.ty_builder());
        Value::new(ty, Val::map(handle))
    }
}
