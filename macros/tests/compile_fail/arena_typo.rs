//! Test: Using wrong name for Bump parameter should produce a helpful error.

use melbi_macros::melbi_fn_new;

struct Bump;

#[melbi_fn_new]
fn uses_wrong_arena_name(bump: &Bump) -> i64 {
    let _ = bump;
    42
}

fn main() {}
