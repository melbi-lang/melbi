//! Test: Invalid attribute format should produce an error.

use melbi_macros::melbi_fn_new;

#[melbi_fn_new(invalid_key = "value")]
fn bad_attribute(x: i64) -> i64 {
    x
}

fn main() {}
