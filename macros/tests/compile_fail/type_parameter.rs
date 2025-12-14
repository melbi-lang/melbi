//! Test: Type parameters should produce an error (not yet supported).

use melbi_macros::melbi_fn_new;

#[melbi_fn_new]
fn generic_function<T>(x: T) -> T {
    x
}

fn main() {}
