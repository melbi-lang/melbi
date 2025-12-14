//! Test: Using `type_manager` instead of `type_mgr` should produce a helpful error.
//!
//! This catches a common typo/naming mistake.

use melbi_macros::melbi_fn_new;

struct TypeManager;

#[melbi_fn_new]
fn uses_wrong_name(type_manager: &TypeManager) -> i64 {
    let _ = type_manager;
    42
}

fn main() {}
