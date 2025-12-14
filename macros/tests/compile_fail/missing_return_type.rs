//! Test: Missing return type should produce an error.

use melbi_macros::melbi_fn;

#[melbi_fn]
fn no_return_type(x: i64) {
    let _ = x;
}

fn main() {}
