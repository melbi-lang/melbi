//! Test: Multiple lifetimes should produce an error.

use melbi_macros::melbi_fn_new;

#[melbi_fn_new]
fn bad_function<'a, 'b>(x: &'a str, y: &'b str) -> &'a str {
    x
}

fn main() {}
