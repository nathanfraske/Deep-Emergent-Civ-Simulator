//! Transitional accessors that keep pre-migration planetary mechanisms on the
//! same independently sealed physical floor as the canonical runner.
//!
//! Typed stage adapters will pass the execution capability directly. Until
//! those adapters land, old private mechanisms may reach `G` only through this
//! no-input constructor, which builds and verifies the repository floor first.

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;

pub(crate) fn gravitational_constant_bigrat() -> Option<BigRat> {
    let execution = civsim_units::constants::canonical_si_execution_magnitudes().ok()?;
    Some(execution.get("G")?.exact_rational())
}

pub(crate) fn ln_gravitational_constant() -> Option<Fixed> {
    let execution = civsim_units::constants::canonical_si_execution_magnitudes().ok()?;
    civsim_physics::saha::ln_fundamental(&execution, "G")
}
