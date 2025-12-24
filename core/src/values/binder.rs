use crate::{String, Vec, values::dynamic::Value};
use core::fmt;

#[derive(Debug)]
pub enum Error {
    DuplicateBinding(Vec<String>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DuplicateBinding(names) => {
                write!(f, "Duplicate binding for ")?;
                for (i, name) in names.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "'{}'", name)?;
                }
                Ok(())
            }
        }
    }
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
