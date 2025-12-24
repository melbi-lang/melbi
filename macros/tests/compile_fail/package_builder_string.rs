//! Test: builder attribute must be an identifier.

use melbi_macros::melbi_package;

#[melbi_package(builder = "good_pkg")]
mod bad_pkg {
    pub fn placeholder() {}
}

fn main() {}
