//! Test: Wrong name for TypeManager in legacy mode (second parameter).

use melbi_macros::melbi_fn_new;

struct Bump;
struct TypeManager;

#[melbi_fn_new]
fn legacy_wrong_type_mgr_name(_arena: &Bump, types: &TypeManager) -> i64 {
    let _ = types;
    42
}

fn main() {}
