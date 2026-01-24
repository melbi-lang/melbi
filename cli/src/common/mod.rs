//! Common utilities shared across CLI commands.

pub mod engine;
pub mod error;
pub mod input;
pub mod panic;

pub use error::{CliError, CliResult};
