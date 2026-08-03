//! Generic scope stack for variable bindings.
//!
//! Used by both the analyzer (binds types) and evaluator (binds values).
//! Supports two kinds of scopes through a unified `Scope` trait:
//! - **Complete scopes**: Immutable, pre-populated (globals, variables)
//! - **Incomplete scopes**: Mutable, filled incrementally (where, lambda)
//!
//! The incomplete scope design enables sequential binding semantics where
//! later bindings can reference earlier ones:
//! ```melbi
//! b where { a = 1, b = a + 1 }  // `b` can see `a`
//! ```

use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt;

use bumpalo::Bump;

/// Trait for scopes that can be pushed onto the ScopeStack.
///
/// All scopes must implement lookup and bind operations.
/// Complete scopes return an error when bind() is called.
///
/// The lifetime parameter `'a` is for the arena where scope data is allocated.
/// The type parameter `T` can itself contain additional lifetimes (e.g., `Value<'types, 'arena>`).
pub trait Scope<'a, T> {
    /// Look up a name in this scope.
    ///
    /// Returns Some(&value) if the name is bound, None otherwise.
    /// The name must have the same lifetime as the scope data.
    fn lookup(&self, name: &'a str) -> Option<&T>;

    /// Bind a value to a name in this scope.
    ///
    /// Complete scopes return `BindError::ScopeIsImmutable`.
    /// Incomplete scopes fill in the value if the name was pre-declared.
    /// The name must have the same lifetime as the scope data.
    fn bind(&mut self, name: &'a str, value: T) -> Result<(), BindError>;
}

/// A complete, immutable scope.
///
/// Bindings are pre-populated and sorted for binary search.
/// Used for globals, captured variables, and function parameters.
pub struct CompleteScope<'a, T>(&'a [(&'a str, T)]);

impl<'a, T> CompleteScope<'a, T> {
    /// Create a new complete scope from sorted bindings.
    ///
    /// The bindings slice must be sorted by name for binary search to work.
    pub fn from_sorted(bindings: &'a [(&'a str, T)]) -> CompleteScope<'a, T> {
        debug_assert!(is_sorted(bindings), "Bindings must be sorted by name");
        CompleteScope(bindings)
    }
}

impl<'a, T> Scope<'a, T> for CompleteScope<'a, T> {
    fn lookup(&self, name: &'a str) -> Option<&T> {
        self.0
            .binary_search_by_key(&name, |(n, _)| *n)
            .ok()
            .map(|idx| &self.0[idx].1)
    }

    fn bind(&mut self, _name: &'a str, _value: T) -> Result<(), BindError> {
        Err(BindError::ScopeIsImmutable)
    }
}

/// An incomplete, mutable scope being built.
///
/// Names are pre-declared and sorted. Values start as None and are filled incrementally.
/// Used for `where` bindings and lambda parameter type inference.
pub struct IncompleteScope<'a, T>(&'a mut [(&'a str, Option<T>)]);

impl<'a, T> IncompleteScope<'a, T> {
    /// Create a new incomplete scope with pre-declared names.
    ///
    /// Names are sorted and allocated in the arena.
    /// Returns an error if there are duplicate names.
    pub fn new(arena: &'a Bump, names: &[&'a str]) -> Result<Self, DuplicateError> {
        let mut sorted_names = Vec::from(names);
        sorted_names.sort_unstable();

        // Check for duplicates
        for window in sorted_names.windows(2) {
            if window[0] == window[1] {
                return Err(DuplicateError(window[0].to_string()));
            }
        }

        // Allocate mutable slice with None values
        let slice = arena.alloc_slice_fill_iter(sorted_names.iter().map(|name| (*name, None)));
        Ok(Self(slice))
    }
}

impl<'a, T> Scope<'a, T> for IncompleteScope<'a, T> {
    fn lookup(&self, name: &'a str) -> Option<&T> {
        self.0
            .binary_search_by_key(&name, |(n, _)| *n)
            .ok()
            .and_then(|idx| self.0[idx].1.as_ref())
    }

    fn bind(&mut self, name: &'a str, value: T) -> Result<(), BindError> {
        match self.0.binary_search_by_key(&name, |(n, _)| *n) {
            Ok(idx) => {
                if self.0[idx].1.is_some() {
                    Err(BindError::AlreadyBound(name.to_string()))
                } else {
                    self.0[idx].1 = Some(value);
                    Ok(())
                }
            }
            Err(_) => Err(BindError::NameNotDeclared(name.to_string())),
        }
    }
}

/// A recording scope that tracks variable lookups without binding any values.
///
/// Used during lambda analysis to detect which variables need to be captured.
/// All lookups return None (delegating to outer scopes), but the names are recorded.
///
/// The recorded names are shared via `Rc<RefCell<>>` so the analyzer can access them
/// after the scope is pushed onto the stack.
pub struct RecordingScope<'a, T> {
    recorded: Rc<RefCell<BTreeSet<&'a str>>>,
    _phantom: core::marker::PhantomData<T>,
}

impl<'a, T> RecordingScope<'a, T> {
    /// Create a new recording scope with a shared recording vector.
    ///
    /// The caller retains a clone of the `Rc` to access recorded names later.
    pub fn new(recorded: Rc<RefCell<BTreeSet<&'a str>>>) -> Self {
        Self {
            recorded,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<'a, T> Scope<'a, T> for RecordingScope<'a, T> {
    fn lookup(&self, name: &'a str) -> Option<&T> {
        // Record the lookup but return None (delegate to outer scopes)
        self.recorded.borrow_mut().insert(name);
        None
    }

    fn bind(&mut self, _name: &'a str, _value: T) -> Result<(), BindError> {
        Err(BindError::ScopeIsImmutable)
    }
}

/// A stack of scopes for variable lookup.
///
/// Maintains a single stack of boxed trait objects, searched from innermost to outermost.
///
/// The `'outer` lifetime parameter is for types that may contain additional lifetimes
/// (e.g., `Value<'types, 'arena>`), while `'a` is the lifetime of the scope data itself.
pub struct ScopeStack<'a, T> {
    scopes: Vec<Box<dyn Scope<'a, T> + 'a>>,
}

impl<'a, T: Copy> Default for ScopeStack<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T: Copy> ScopeStack<'a, T> {
    /// Create a new empty scope stack.
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    /// Push a scope onto the stack.
    pub fn push<S: Scope<'a, T> + 'a>(&mut self, scope: S) {
        self.scopes.push(Box::new(scope));
    }

    /// Pop the topmost scope from the stack.
    ///
    /// Returns an error if the stack is empty.
    pub fn pop(&mut self) -> Result<(), PopError> {
        self.scopes.pop().ok_or(PopError::EmptyStack)?;
        Ok(())
    }

    /// Look up a name, searching scopes from innermost to outermost.
    ///
    /// Returns the first matching value found, or None if not found in any scope.
    /// The name must have the same lifetime as the scope data.
    pub fn lookup(&self, name: &'a str) -> Option<&T> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.lookup(name) {
                return Some(val);
            }
        }
        None
    }

    /// Bind a value in the topmost scope.
    ///
    /// Returns an error if:
    /// - The stack is empty
    /// - The topmost scope is immutable (complete scope)
    /// - The name was not pre-declared (incomplete scope)
    /// - The name is already bound (incomplete scope)
    ///
    /// The name must have the same lifetime as the scope data.
    pub fn bind_in_current(&mut self, name: &'a str, value: T) -> Result<(), BindError> {
        self.scopes
            .last_mut()
            .ok_or(BindError::NoScope)?
            .bind(name, value)
    }
}

/// Check if a slice is sorted by name (for debug assertions).
fn is_sorted<T>(slice: &[(&str, T)]) -> bool {
    slice.windows(2).all(|w| w[0].0 <= w[1].0)
}

/// Error when trying to bind a value in a scope.
#[derive(Debug, Clone)]
pub enum BindError {
    /// No scope exists to bind in.
    NoScope,
    /// The scope is immutable (complete scope).
    ScopeIsImmutable,
    /// The name has already been bound in the current scope.
    AlreadyBound(alloc::string::String),
    /// The name was not declared when the scope was created.
    NameNotDeclared(alloc::string::String),
}

impl fmt::Display for BindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BindError::NoScope => write!(f, "No scope to bind in"),
            BindError::ScopeIsImmutable => write!(f, "Cannot bind in immutable scope"),
            BindError::AlreadyBound(name) => {
                write!(f, "Name '{}' already bound in current scope", name)
            }
            BindError::NameNotDeclared(name) => {
                write!(f, "Name '{}' not declared in current scope", name)
            }
        }
    }
}

/// Error when trying to pop a scope.
#[derive(Debug, Clone)]
pub enum PopError {
    /// The stack is empty.
    EmptyStack,
}

impl fmt::Display for PopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PopError::EmptyStack => write!(f, "Cannot pop from empty scope stack"),
        }
    }
}

/// Error when duplicate names are found in a scope.
#[derive(Debug, Clone)]
pub struct DuplicateError(pub alloc::string::String);

impl fmt::Display for DuplicateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Duplicate name '{}' in scope", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_scope_lookup() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        let bindings = bump.alloc_slice_copy(&[("a", 1), ("b", 2), ("c", 3)]);
        stack.push(CompleteScope::from_sorted(bindings));

        assert_eq!(stack.lookup("a"), Some(&1));
        assert_eq!(stack.lookup("b"), Some(&2));
        assert_eq!(stack.lookup("c"), Some(&3));
        assert_eq!(stack.lookup("d"), None);
    }

    #[test]
    fn test_incomplete_scope_sequential_binding() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        // Push incomplete scope with names
        stack.push(IncompleteScope::new(&bump, &["a", "b"]).unwrap());

        // Before binding, lookup returns None
        assert_eq!(stack.lookup("a"), None);
        assert_eq!(stack.lookup("b"), None);

        // Bind 'a'
        stack.bind_in_current("a", 1).unwrap();
        assert_eq!(stack.lookup("a"), Some(&1));
        assert_eq!(stack.lookup("b"), None);

        // Bind 'b'
        stack.bind_in_current("b", 2).unwrap();
        assert_eq!(stack.lookup("a"), Some(&1));
        assert_eq!(stack.lookup("b"), Some(&2));
    }

    #[test]
    fn test_shadowing() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        // Push complete scope
        let bindings1 = bump.alloc_slice_copy(&[("a", 1), ("b", 2)]);
        stack.push(CompleteScope::from_sorted(bindings1));

        // Push incomplete scope that shadows 'a'
        stack.push(IncompleteScope::new(&bump, &["a"]).unwrap());
        stack.bind_in_current("a", 10).unwrap();

        // 'a' is shadowed, 'b' is not
        assert_eq!(stack.lookup("a"), Some(&10));
        assert_eq!(stack.lookup("b"), Some(&2));

        // Pop incomplete scope
        stack.pop().unwrap();

        // Original 'a' is visible again
        assert_eq!(stack.lookup("a"), Some(&1));
    }

    #[test]
    fn test_duplicate_names_error() {
        let bump = Bump::new();

        let result = IncompleteScope::<i32>::new(&bump, &["a", "b", "a"]);
        assert!(result.is_err());
        assert!(matches!(result, Err(DuplicateError(_))));
    }

    #[test]
    fn test_bind_already_bound_error() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        stack.push(IncompleteScope::new(&bump, &["a"]).unwrap());
        stack.bind_in_current("a", 1).unwrap();

        let result = stack.bind_in_current("a", 2);
        assert!(result.is_err());
        assert!(matches!(result, Err(BindError::AlreadyBound(_))));
    }

    #[test]
    fn test_bind_undeclared_name_error() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        let scope = IncompleteScope::new(&bump, &["a"]).unwrap();
        stack.push(scope);

        let result = stack.bind_in_current("b", 1);
        assert!(result.is_err());
        assert!(matches!(result, Err(BindError::NameNotDeclared(_))));
    }

    #[test]
    fn test_bind_immutable_scope_error() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        let bindings = bump.alloc_slice_copy(&[("a", 1)]);
        stack.push(CompleteScope::from_sorted(bindings));

        let result = stack.bind_in_current("a", 10);
        assert!(result.is_err());
        assert!(matches!(result, Err(BindError::ScopeIsImmutable)));
    }
}
