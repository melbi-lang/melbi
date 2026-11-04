//! Storage shared by dynamic and typed maps.
//!
//! A map built from typed values and one built from dynamic values have the
//! same storage, so either side can read a map the other built. That only works
//! because typed values hash and compare exactly like their dynamic forms; see
//! `tests/equality.rs`.

use core::fmt;
use core::hash::{BuildHasher, Hasher};

use hashbrown::HashTable;
use hashbrown::hash_table::Entry;

use crate::traits::{Val, ValueBuilder};

/// A map's entries, in a hash table built once and never mutated.
///
/// Keys and values are stored inline as [`Val`]s. Their types live outside, in
/// the map's type, like every other value's. So the table cannot hash or
/// compare keys by itself: whoever knows the key type supplies the hash and the
/// equality for each operation.
pub struct MapStorage<B: ValueBuilder> {
    table: HashTable<(Val<B>, Val<B>), B::MapAllocator>,
}

impl<B: ValueBuilder> MapStorage<B> {
    pub(crate) fn len(&self) -> usize {
        self.table.len()
    }

    /// The value stored for the key with this `hash` that `is_key` accepts.
    pub(crate) fn get(&self, hash: u64, is_key: impl FnMut(&Val<B>) -> bool) -> Option<&Val<B>> {
        let mut is_key = is_key;
        self.table
            .find(hash, |(key, _)| is_key(key))
            .map(|(_, value)| value)
    }

    /// All entries, in the table's order: unspecified, but the same on every
    /// run for the same entries inserted in the same order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &(Val<B>, Val<B>)> {
        self.table.iter()
    }
}

impl<B: ValueBuilder> fmt::Debug for MapStorage<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapStorage")
            .field("len", &self.table.len())
            .finish_non_exhaustive()
    }
}

/// The hasher for map keys.
///
/// The seed is fixed so that iteration order is reproducible from run to run,
/// which keeps display, serialization and test output stable.
pub(crate) fn map_hasher() -> impl Hasher {
    const SEED: u64 = 0x6d65_6c62_695f_6d61; // "melbi_ma"
    foldhash::fast::FixedState::with_seed(SEED).build_hasher()
}

/// Accumulates a map's entries. Inserting a key that is already present
/// replaces its value, so the last insertion wins.
pub(crate) struct MapStorageMaker<B: ValueBuilder> {
    table: HashTable<(Val<B>, Val<B>), B::MapAllocator>,
}

impl<B: ValueBuilder> MapStorageMaker<B> {
    pub(crate) fn new(builder: &B) -> Self {
        Self {
            table: HashTable::new_in(builder.map_allocator()),
        }
    }

    /// Insert an entry whose key hashes to `hash`.
    ///
    /// `same_key` decides whether a stored key equals `key`; `hash_of` hashes a
    /// stored key, for when the table grows.
    pub(crate) fn insert(
        &mut self,
        hash: u64,
        key: Val<B>,
        value: Val<B>,
        same_key: impl Fn(&Val<B>, &Val<B>) -> bool,
        hash_of: impl Fn(&Val<B>) -> u64,
    ) {
        match self.table.entry(
            hash,
            |(stored, _)| same_key(stored, &key),
            |(stored, _)| hash_of(stored),
        ) {
            // Keep the first key: it equals the new one.
            Entry::Occupied(mut entry) => entry.get_mut().1 = value,
            Entry::Vacant(entry) => {
                entry.insert((key, value));
            }
        }
    }

    pub(crate) fn build(self, builder: &B) -> B::MapHandle {
        builder.alloc_map(MapStorage { table: self.table })
    }
}
