//! Melbi's single rule for comparing floats.
//!
//! It is used by `==`, `<`, sorting and map keys alike, so they can never
//! disagree. It matches IEEE except for `NaN`: every `NaN` equals every other
//! `NaN`, and `NaN` is greater than every number. `-0.0` and `0.0` are equal, as
//! in IEEE. See `docs/design/maps.md` for the alternatives that were rejected.

use core::cmp::Ordering;
use core::hash::Hasher;

/// Map `-0.0` to `0.0` and every `NaN` to one positive `NaN`.
///
/// After this, `total_cmp` agrees with IEEE on every pair of numbers, and the
/// sign bit of a `NaN` (which differs between x86 and ARM for `0.0 / 0.0`) no
/// longer matters.
fn canonical(x: f64) -> f64 {
    if x.is_nan() {
        f64::NAN
    } else if x == 0.0 {
        0.0
    } else {
        x
    }
}

pub(crate) fn cmp(a: f64, b: f64) -> Ordering {
    canonical(a).total_cmp(&canonical(b))
}

pub(crate) fn eq(a: f64, b: f64) -> bool {
    cmp(a, b) == Ordering::Equal
}

pub(crate) fn hash(x: f64, state: &mut impl Hasher) {
    state.write_u64(canonical(x).to_bits());
}
