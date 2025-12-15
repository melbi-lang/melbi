//! Test: melbi_package on file module (non-inline) should produce an error.

use melbi_macros::melbi_package;

#[melbi_package]
mod file_module;

fn main() {}
