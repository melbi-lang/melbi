use crate::{String, Vec, values::dynamic::Value};

#[derive(Debug)]
pub enum Error {
    DuplicateBinding(Vec<String>),
}

/// A trait for types that can be built by binding names to values.
///
/// This provides a unified, fluent interface for constructing complex,
/// field-based values like records and global environments.
pub trait Binder<'ty: 'val, 'val>: Sized {
    /// The final, successfully built output type.
    type Output;

    /// Binds a name to a value in the builder.
    ///
    /// This method uses a fluent API, returning the builder to allow for chained calls.
    fn bind(self, name: &str, value: Value<'ty, 'val>) -> Self;

    /// Finalizes the build process.
    ///
    /// This method consumes the builder and returns the final constructed
    /// output or an error if the build fails.
    fn build(self) -> Result<Self::Output, Error>;
}
