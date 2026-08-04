//! Fixtures shared by the builder integration tests.
//!
//! A directory rather than a top-level file, so Cargo does not compile it as a
//! test binary of its own.

// Each test binary compiles this module separately and uses a different subset
// of it, so unused items here are expected rather than a smell.
#![allow(dead_code, reason = "Shared test helpers used across integration test binaries")]

pub mod builders;
pub mod sample_tree;
