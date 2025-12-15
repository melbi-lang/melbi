//! Test: name attribute must be a string literal.

use melbi_macros::melbi_fn;

#[melbi_fn(name = 123)]
fn bad_name_type(x: i64) -> i64 {
    x
}

fn main() {}
