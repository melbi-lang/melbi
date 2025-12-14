//! Test: Self parameter is not supported.

use melbi_macros::melbi_fn;

struct Foo;

impl Foo {
    #[melbi_fn]
    fn method(self) -> i64 {
        42
    }
}

fn main() {}
