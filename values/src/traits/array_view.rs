/// A read-only, indexed view over an array of elements.
///
/// # Contract
///
/// Implementors **must** guarantee that `get(i)` returns `Some` for every
/// `i` in `0..len()`. The default [`iter`](Self::iter) implementation relies
/// on this invariant and will panic otherwise.
pub trait ArrayView<E> {
    /// Returns the number of elements in the array.
    fn len(&self) -> usize;

    /// Returns the element at `index`, or `None` if out of bounds.
    fn get(&self, index: usize) -> Option<E>;

    /// Returns `true` if the array contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over all elements.
    ///
    /// The default implementation calls `get(i)` for each index and unwraps.
    /// Concrete types may override this for better performance (e.g., avoiding
    /// repeated index lookups).
    fn iter(&self) -> impl Iterator<Item = E> + '_
    where
        Self: Sized,
    {
        (0..self.len()).map(move |i| self.get(i).unwrap())
    }
}
