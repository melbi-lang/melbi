//! Test: Generic type parameter must be used in at least one input parameter
//! for runtime dispatch to work.

#[allow(unused_imports)]
use melbi_core::values::Numeric;
use melbi_macros::melbi_fn;

#[melbi_fn]
fn make_default<T: Numeric>() -> T {
    todo!()
}

fn main() {}
