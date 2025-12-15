//! Test: builder attribute must be a string literal.

use melbi_macros::melbi_package;

#[melbi_package(builder = 123)]
mod bad_pkg {
    pub fn placeholder() {}
}

fn main() {}
