//! Statically-typed map view for zero-cost interop.
//!
//! `Map<B, K, V>` wraps the builder's `MapHandle` directly. It shares storage
//! with [`dynamic::Map`](crate::dynamic::Map): a map built here can be read
//! dynamically, and vice versa, because typed keys hash and compare exactly
//! like their dynamic forms.
//!
//! # Example
//!
//! ```
//! use melbi_values::builders::BoxValueBuilder;
//! use melbi_values::traits::MapView;
//! use melbi_values::typed::{Int, Map, Str};
//!
//! let builder = BoxValueBuilder::new();
//! let map = Map::<_, Str<_>, Int>::new(
//!     &builder,
//!     [(Str::new(&builder, "one"), 1), (Str::new(&builder, "two"), 2)],
//! );
//! assert_eq!(map.get(&Str::new(&builder, "two")), Some(Int(2)));
//! ```

use core::fmt::{self, Debug};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;

use melbi_types::{Ty, TyKind};

use super::marshal::Sealed;
use super::{Hashable, Marshal};
use crate::traits::{MapStorageMaker, MapView, Val, ValueBuilder, map_hasher};

/// A statically-typed map with keys `K` and values `V`.
///
/// For the arena builder this is a single thin pointer, like [`Array`](super::Array).
pub struct Map<B: ValueBuilder, K, V> {
    handle: B::MapHandle,
    _marker: PhantomData<(K, V)>,
}

// --- Manual Clone/Copy/Debug to avoid bounds on B, K and V ---

impl<B: ValueBuilder, K, V> Clone for Map<B, K, V> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            _marker: PhantomData,
        }
    }
}

impl<B: ValueBuilder, K, V> Copy for Map<B, K, V> where B::MapHandle: Copy {}

impl<B: ValueBuilder, K, V> Debug for Map<B, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Map")
            .field("handle", &self.handle)
            .field("key_type", &core::any::type_name::<K>())
            .field("value_type", &core::any::type_name::<V>())
            .finish()
    }
}

/// The hash a map stores `key` under. Matches the dynamic key hash.
fn key_hash<B: ValueBuilder, K: Hashable<B>>(key: &K) -> u64 {
    let mut state = map_hasher();
    key.hash(&mut state);
    state.finish()
}

// --- Construction ---

impl<B: ValueBuilder, K: Hashable<B>, V: Marshal<B>> Map<B, K, V> {
    /// Create a typed map from a list of entries. Later entries win over
    /// earlier ones with an equal key.
    pub fn new<KK: Into<K>, VV: Into<V>>(
        builder: &B,
        entries: impl IntoIterator<Item = (KK, VV)>,
    ) -> Self {
        let mut storage = MapStorageMaker::new(builder);
        for (key, value) in entries {
            let key: K = key.into();
            let hash = key_hash(&key);
            storage.insert(
                hash,
                key.into_val(builder),
                value.into().into_val(builder),
                |stored: &Val<B>, new: &Val<B>| {
                    K::from_val_unchecked(stored) == K::from_val_unchecked(new)
                },
                |stored: &Val<B>| key_hash(&K::from_val_unchecked(stored)),
            );
        }
        Self {
            handle: storage.build(builder),
            _marker: PhantomData,
        }
    }

    /// Create an empty typed map.
    pub fn empty(builder: &B) -> Self {
        Self::new(builder, core::iter::empty::<(K, V)>())
    }
}

// --- MapView ---

impl<B: ValueBuilder, K: Hashable<B>, V: Marshal<B>> MapView<K, V> for Map<B, K, V> {
    fn len(&self) -> usize {
        self.handle.len()
    }

    fn get(&self, key: &K) -> Option<V> {
        self.handle
            .get(key_hash(key), |stored| {
                K::from_val_unchecked(stored) == *key
            })
            .map(V::from_val_unchecked)
    }

    fn iter(&self) -> impl Iterator<Item = (K, V)> + '_ {
        self.handle
            .iter()
            .map(|(key, value)| (K::from_val_unchecked(key), V::from_val_unchecked(value)))
    }
}

// --- Equality and hashing: must match `dynamic::equality` for maps ---

impl<B: ValueBuilder, K: Hashable<B>, V: Marshal<B> + PartialEq> PartialEq for Map<B, K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .all(|(key, value)| other.get(&key).is_some_and(|other| other == value))
    }
}

impl<B: ValueBuilder, K: Hashable<B>, V: Marshal<B> + Eq> Eq for Map<B, K, V> {}

/// Order-independent: the length, then the sum of each entry's own hash.
impl<B: ValueBuilder, K: Hashable<B>, V: Marshal<B> + Hash> Hash for Map<B, K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.len());
        let sum = self.iter().fold(0u64, |sum, (key, value)| {
            let mut entry = map_hasher();
            key.hash(&mut entry);
            value.hash(&mut entry);
            sum.wrapping_add(entry.finish())
        });
        state.write_u64(sum);
    }
}

// --- Marshal (enables nesting, e.g. Map<B, Str<B>, Array<B, Int>>) ---

impl<B: ValueBuilder, K: Hashable<B>, V: Marshal<B>> Sealed for Map<B, K, V> {}

impl<B: ValueBuilder, K: Hashable<B>, V: Marshal<B>> Marshal<B> for Map<B, K, V> {
    fn ty(tb: &B::TB) -> Ty<B::TB> {
        TyKind::Map(K::ty(tb), V::ty(tb)).alloc(tb)
    }

    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool {
        match kind {
            TyKind::Map(key_ty, value_ty) => {
                K::matches_ty_kind(key_ty.kind()) && V::matches_ty_kind(value_ty.kind())
            }
            _ => false,
        }
    }

    fn from_val_unchecked(val: &Val<B>) -> Self {
        Self {
            handle: val.as_map_unchecked().clone(),
            _marker: PhantomData,
        }
    }

    fn into_val(self, _builder: &B) -> Val<B> {
        Val::map(self.handle)
    }
}

impl<B: ValueBuilder, K: Hashable<B>, V: Hashable<B>> Hashable<B> for Map<B, K, V> {}
