/// A read-only view over a map.
///
/// Iteration order is not part of the language: it is unspecified, but the
/// same on every run for the same entries inserted in the same order.
pub trait MapView<K, V> {
    /// Returns the number of entries.
    fn len(&self) -> usize;

    /// Returns the value for `key`, or `None` if the map has no such key.
    fn get(&self, key: &K) -> Option<V>;

    /// Returns `true` if the map has no entries.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over all entries.
    fn iter(&self) -> impl Iterator<Item = (K, V)> + '_
    where
        Self: Sized;
}
