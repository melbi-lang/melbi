//! Compile-fail tests for `#[melbi_fn_new]` error detection.
//!
//! These tests verify that the macro produces helpful error messages
//! for invalid input.

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
