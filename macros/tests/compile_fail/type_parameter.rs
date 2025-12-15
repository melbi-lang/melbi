//! Test: Type parameters should produce an error.

use melbi_macros::melbi_fn;

#[melbi_fn]
fn generic_function<T>(x: T) -> T {
    x
}

fn main() {}
