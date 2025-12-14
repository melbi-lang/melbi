//! Test: Pattern matching in parameters is not supported.

use melbi_macros::melbi_fn;

#[melbi_fn]
fn destructure_tuple((a, b): (i64, i64)) -> i64 {
    a + b
}

fn main() {}
