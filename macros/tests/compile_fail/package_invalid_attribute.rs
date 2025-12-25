//! Test: Invalid package attribute should produce an error.

use melbi_macros::melbi_package;

#[melbi_package(invalid = BadPkg)]
mod bad_pkg {
    pub fn placeholder() {}
}

fn main() {}
