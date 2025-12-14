//! Test: Const generics should produce an error.

use melbi_macros::melbi_fn_new;

#[melbi_fn_new]
fn const_generic_function<const N: usize>() -> i64 {
    N as i64
}

fn main() {}
