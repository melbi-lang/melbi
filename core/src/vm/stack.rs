use alloc::fmt;

use crate::Vec;

/// A stack data structure with maximum size enforcement in debug mode.
///
/// This stack is used by the VM for value storage during execution. The maximum
/// size is only enforced in debug builds to catch stack overflow bugs during
/// development, while maintaining zero overhead in release builds.
pub struct Stack<T> {
    /// The underlying storage for stack elements.
    items: Vec<T>,
    /// Maximum allowed stack size (enforced in debug mode only).
    max_size: usize,
}

#[allow(
    dead_code,
    reason = "Preserve the full Stack API. All methods are tested."
)]
impl<T> Stack<T> {
    pub fn new(max_size: usize) -> Self {
        // Pre-allocate a reasonable amount (min of max_size or 256)
        // to avoid frequent reallocations during normal execution
        let initial_capacity = max_size.min(256);

        Self {
            items: Vec::with_capacity(initial_capacity),
            max_size,
        }
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        debug_assert!(
            self.items.len() < self.max_size,
            "Stack overflow: attempted to push beyond maximum size of {}",
            self.max_size
        );
        self.items.push(value);
    }

    #[inline]
    pub fn pop(&mut self) -> T {
        debug_assert!(!self.is_empty());
        self.items.pop().unwrap()
    }

    #[inline]
    pub fn peek(&self) -> Option<&T> {
        self.items.last()
    }

    #[inline]
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        self.items.last_mut()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
    }

    #[inline]
    pub fn pop_n(&mut self, n: usize) {
        let new_len = self.len().saturating_sub(n);
        self.items.truncate(new_len);
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    #[inline]
    pub fn top_n(&self, n: usize) -> &[T] {
        debug_assert!(n <= self.items.len());
        let len = self.items.len();
        &self.items[len - n..]
    }

    #[inline]
    pub fn top_n_mut(&mut self, n: usize) -> Option<&mut [T]> {
        let len = self.items.len();
        if n > len {
            None
        } else {
            Some(&mut self.items[len - n..])
        }
    }
}

#[allow(
    dead_code,
    reason = "Preserve the full Stack API. All methods are tested."
)]
impl<T: Clone> Stack<T> {
    #[inline]
    pub fn peek_at(&self, offset: usize) -> Option<&T> {
        let len = self.items.len();
        if offset >= len {
            None
        } else {
            Some(&self.items[len - 1 - offset])
        }
    }

    #[inline]
    pub fn dup(&mut self) -> bool {
        if let Some(value) = self.peek().cloned() {
            self.push(value);
            true
        } else {
            false
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Stack<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stack")
            .field("items", &self.items)
            .field("len", &self.items.len())
            .field("capacity", &self.max_size)
            .finish()
    }
}

impl<T> core::ops::Index<usize> for Stack<T> {
    type Output = T;

    /// Indexes the stack from the top.
    ///
    /// `stack[0]` returns the top element, `stack[1]` returns the element below it, etc.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(10);
    /// stack.push(20);
    /// stack.push(30);
    ///
    /// assert_eq!(stack[0], 30); // Top
    /// assert_eq!(stack[1], 20); // Below top
    /// assert_eq!(stack[2], 10); // Bottom
    /// ```
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        let len = self.items.len();
        assert!(
            index < len,
            "Stack index out of bounds: index {index} but stack has {len} elements"
        );
        &self.items[len - 1 - index]
    }
}

impl<T> core::ops::IndexMut<usize> for Stack<T> {
    /// Indexes the stack from the top (mutable).
    ///
    /// `stack[0]` returns the top element, `stack[1]` returns the element below it, etc.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(10);
    /// stack.push(20);
    /// stack.push(30);
    ///
    /// stack[0] = 99; // Modify top
    /// assert_eq!(stack[0], 99);
    /// ```
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let len = self.items.len();
        assert!(
            index < len,
            "Stack index out of bounds: index {index} but stack has {len} elements"
        );
        &mut self.items[len - 1 - index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stack() {
        let stack: Stack<i32> = Stack::new(100);
        assert_eq!(stack.len(), 0);
        assert_eq!(stack.capacity(), 100);
        assert!(stack.is_empty());
    }

    #[test]
    fn push_pop() {
        let mut stack = Stack::new(100);
        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.len(), 3);
        assert_eq!(stack.pop(), 3);
        assert_eq!(stack.pop(), 2);
        assert_eq!(stack.pop(), 1);
        assert!(stack.is_empty());
    }

    #[test]
    fn peek() {
        let mut stack = Stack::new(100);
        assert_eq!(stack.peek(), None);

        stack.push(42);
        assert_eq!(stack.peek(), Some(&42));
        assert_eq!(stack.len(), 1); // Peek doesn't remove

        stack.push(17);
        assert_eq!(stack.peek(), Some(&17));

        // Clean up
        stack.clear();
    }

    #[test]
    fn peek_mut() {
        let mut stack = Stack::new(100);
        stack.push(42);

        if let Some(top) = stack.peek_mut() {
            *top = 100;
        }

        assert_eq!(stack.pop(), 100);
    }

    #[test]
    fn peek_at() {
        let mut stack = Stack::new(100);
        stack.push(10);
        stack.push(20);
        stack.push(30);

        assert_eq!(stack.peek_at(0), Some(&30));
        assert_eq!(stack.peek_at(1), Some(&20));
        assert_eq!(stack.peek_at(2), Some(&10));
        assert_eq!(stack.peek_at(3), None);

        // Clean up
        stack.clear();
    }

    #[test]
    fn dup() {
        let mut stack = Stack::new(100);

        // Dup on empty stack
        assert!(!stack.dup());

        // Dup with value
        stack.push(42);
        assert!(stack.dup());
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.pop(), 42);
        assert_eq!(stack.pop(), 42);
    }

    #[test]
    fn clear() {
        let mut stack = Stack::new(100);
        stack.push(1);
        stack.push(2);
        stack.push(3);

        stack.clear();
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn iter() {
        let mut stack = Stack::new(100);
        stack.push(1);
        stack.push(2);
        stack.push(3);

        let items: Vec<_> = stack.iter().copied().collect();
        assert_eq!(items, vec![1, 2, 3]);

        // Clean up
        stack.clear();
    }

    #[test]
    #[should_panic(expected = "Stack overflow")]
    #[cfg(debug_assertions)]
    fn overflow_debug() {
        let mut stack = Stack::new(2);
        stack.push(1);
        stack.push(2);
        stack.push(3); // Should panic in debug mode
    }

    #[test]
    fn large_stack() {
        let mut stack = Stack::new(10000);
        for i in 0..1000 {
            stack.push(i);
        }

        assert_eq!(stack.len(), 1000);

        for i in (0..1000).rev() {
            assert_eq!(stack.pop(), i);
        }
    }
}
