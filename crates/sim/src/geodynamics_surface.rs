// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The surface isostatic-relaxation consumer (genesis-forward Stage 3, the surface lane). This reads the
//! per-column [`GeodynamicColumn::isostatic_elevation`] the producer derives (the interior convection lane, or
//! the surface seed-crust pass, both through the Airy flotation law in
//! [`civsim_physics::geodynamics::airy_isostatic_elevation`]) and relaxes the EFFECTIVE elevation toward it by
//! writing the geological delta into the [`EarthworkField`]. It is the consumer end of the producer-consumer
//! boundary: it never derives the target itself, so the `mantle_density` and the seed `crustal_thickness` the
//! Airy law needs live at the producer (where the target is born and where their cited or derived basis
//! belongs), never here.
//!
//! The mechanism is a standard relaxation (a fractional Jacobi step toward the equilibrium each pass), chosen
//! over a one-shot snap so the same kernel serves the later LIVE case: when the interior lane advances the
//! `isostatic_elevation` each tick (mass redistributed by surface processes shifts the buoyancy target), the
//! surface relaxes toward the moving target one step at a time, exactly as it relaxes the static seed here. The
//! step is a fraction `lambda` of the residual `target - effective`; with `lambda == 1` a single pass snaps the
//! crust to isostatic equilibrium (the seed born in balance), with `lambda < 1` the crust approaches it
//! geometrically over passes (the residual falls as `(1 - lambda)^n`), which is the form the live drift needs.
//!
//! This module is the single relaxation PASS (the kernel). The drive-to-equilibrium OUTER loop (run the pass to
//! convergence under a fixed pass cap and an integer residual tolerance) is deferred to reuse C's shared
//! `crates/world/src/solve.rs` (the fixed-cap integer-residual iterative solve, generic over the kernel) on
//! rebase, rather than hand-rolling a second copy of that solve substrate. The surface-process balance picks up
//! the pass and the shared solve together.
//!
//! Deterministic fixed-point arithmetic in canonical [`civsim_world::Coord3`] key order (Principle 3). Off the
//! run path until a genesis scenario arms it, so declaring it leaves every scenario byte-identical: the
//! consumer walks only the columns [`GeodynamicField`] carries, and an unarmed geology carries none, so the
//! pass does no work and writes no delta (the opt-in empty-default pattern).

use civsim_core::Fixed;
use civsim_world::Coord3;

use crate::calibration::CalibrationError;
use crate::material::{EarthworkField, GeodynamicField};

/// One relaxation pass over the armed geodynamic columns: for each column carrying geodynamic state, read its
/// producer-derived `isostatic_elevation` target, measure the residual against the current effective elevation
/// (the worldgen base plus the accumulated earthwork, being and geological), and move the geological delta a
/// fraction `relaxation` of the residual toward the target. Returns the worst absolute residual OBSERVED this
/// pass (before this pass's adjustment), the disequilibrium the caller's convergence loop tests against its
/// tolerance.
///
/// `base_elevation` reads the frozen worldgen base at a column (the same base the physics reads under the
/// earthwork delta), passed as a closure so the consumer stays decoupled from the environment field that owns
/// the heightmap. `relaxation` is the fraction of the residual relaxed per pass, and must lie in the half-open
/// interval `(0, 1]`: a non-positive fraction never relaxes, and a fraction above one overshoots the target and
/// can oscillate or diverge, so either is refused fail-loud (a [`CalibrationError::BadValue`]) rather than
/// silently clamped. A fixed-point overflow in the step multiply is refused the same way, never a fabricated
/// delta.
///
/// The walk is in canonical [`Coord3`] key order (the `BTreeMap` order [`GeodynamicField::iter`] yields), so the
/// pass is reproducible and thread-invariant (Principle 3). An empty [`GeodynamicField`] yields no columns, so
/// the pass writes nothing and the field stays byte-identical (byte-neutral off the base pins).
pub fn relax_toward_isostasy<F>(
    earthwork: &mut EarthworkField,
    geodynamics: &GeodynamicField,
    base_elevation: F,
    relaxation: Fixed,
) -> Result<Fixed, CalibrationError>
where
    F: Fn(Coord3) -> Fixed,
{
    if relaxation <= Fixed::ZERO || relaxation > Fixed::ONE {
        return Err(CalibrationError::BadValue {
            id: "geodynamics.isostatic_relaxation_fraction".to_string(),
            detail: format!(
                "the relaxation fraction must lie in (0, 1]; got {}",
                relaxation.to_f64_lossy()
            ),
        });
    }
    let mut worst_residual = Fixed::ZERO;
    for (column, state) in geodynamics.iter() {
        let effective = base_elevation(column).saturating_add(earthwork.total_delta(column));
        // The residual the pass corrects a fraction of: how far the crust sits from its buoyancy target.
        // Elevations are bounded (metres), as in the sibling flotation law, so the difference is representable.
        let residual = state.isostatic_elevation - effective;
        let magnitude = residual.abs();
        if magnitude > worst_residual {
            worst_residual = magnitude;
        }
        let step = relaxation
            .checked_mul(residual)
            .ok_or_else(|| CalibrationError::BadValue {
                id: "geodynamics.isostatic_relaxation_fraction".to_string(),
                detail: "the relaxation step overflowed fixed-point; the isostatic target is out of range"
                    .to_string(),
            })?;
        earthwork.adjust_geological(column, step);
    }
    Ok(worst_residual)
}

// The drive-to-equilibrium outer loop (run this single pass to convergence under a fixed pass cap and an
// integer residual tolerance) is DEFERRED to reuse C's shared `crates/world/src/solve.rs` (the fixed-cap
// integer-residual iterative solve, generic over the kernel) once this branch rebases onto it, rather than
// hand-rolling a second copy of that substrate here. This single pass is the kernel `solve` will drive; the
// surface-process balance picks up both together.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::GeodynamicColumn;
    use civsim_physics::geodynamics::airy_isostatic_elevation;

    fn flat_base(_: Coord3) -> Fixed {
        Fixed::ZERO
    }

    #[test]
    fn a_single_full_pass_snaps_the_crust_to_its_isostatic_target() {
        // lambda == 1: one relaxation pass drives the effective elevation exactly to the target, the crust born
        // in isostatic balance. The residual observed is the full initial disequilibrium; the delta written
        // closes it.
        let mut earthwork = EarthworkField::new();
        let mut geo = GeodynamicField::new();
        let a = Coord3::ground(1, 1);
        let target = Fixed::from_int(2000);
        geo.set(
            a,
            GeodynamicColumn {
                crustal_density: Fixed::from_ratio(27, 10),
                crustal_thickness: Fixed::from_int(35_000),
                isostatic_elevation: target,
                ..GeodynamicColumn::default()
            },
        );
        let observed = relax_toward_isostasy(&mut earthwork, &geo, flat_base, Fixed::ONE)
            .expect("valid fraction");
        assert_eq!(
            observed, target,
            "the pass observes the full initial residual"
        );
        // Effective = base (0) + geological delta; a full pass makes it equal the target.
        assert_eq!(
            earthwork.total_delta(a),
            target,
            "a full pass snaps the crust to its isostatic target"
        );
    }

    #[test]
    fn a_partial_pass_approaches_the_target_geometrically() {
        // lambda < 1: each pass closes the fraction lambda of the remaining residual, so the residual falls as
        // (1 - lambda)^n. Half-relaxation from base 0 toward 1000 gives 500, then 750, then 875.
        let mut earthwork = EarthworkField::new();
        let mut geo = GeodynamicField::new();
        let a = Coord3::ground(0, 0);
        geo.set(
            a,
            GeodynamicColumn {
                isostatic_elevation: Fixed::from_int(1000),
                ..GeodynamicColumn::default()
            },
        );
        let half = Fixed::from_ratio(1, 2);
        relax_toward_isostasy(&mut earthwork, &geo, flat_base, half).expect("valid fraction");
        assert_eq!(
            earthwork.total_delta(a),
            Fixed::from_int(500),
            "first half-step"
        );
        relax_toward_isostasy(&mut earthwork, &geo, flat_base, half).expect("valid fraction");
        assert_eq!(
            earthwork.total_delta(a),
            Fixed::from_int(750),
            "second half-step"
        );
        relax_toward_isostasy(&mut earthwork, &geo, flat_base, half).expect("valid fraction");
        assert_eq!(
            earthwork.total_delta(a),
            Fixed::from_int(875),
            "third half-step"
        );
    }

    #[test]
    fn a_felsic_and_a_mafic_column_relax_to_their_airy_targets_with_felsic_higher() {
        // The full producer-consumer chain against a synthetic composition: the Airy law derives each column's
        // isostatic target from a synthetic density and thickness (the producer), and the relaxation drives the
        // effective elevation to it (the consumer). A felsic (light) column relaxes to a HIGHER elevation than a
        // mafic (dense) one, the freeboard emerging from the density contrast, never an authored height. A full
        // pass (lambda == 1) snaps each column to its target, so the single kernel needs no outer loop to show
        // the emergent freeboard.
        let mantle = Fixed::from_ratio(33, 10);
        let thickness = Fixed::from_int(35_000);
        let felsic_target =
            airy_isostatic_elevation(Fixed::from_ratio(265, 100), mantle, thickness)
                .expect("felsic floats");
        let mafic_target = airy_isostatic_elevation(Fixed::from_ratio(300, 100), mantle, thickness)
            .expect("mafic floats");
        let mut earthwork = EarthworkField::new();
        let mut geo = GeodynamicField::new();
        let felsic = Coord3::ground(2, 3);
        let mafic = Coord3::ground(4, 5);
        geo.set(
            felsic,
            GeodynamicColumn {
                crustal_density: Fixed::from_ratio(265, 100),
                crustal_thickness: thickness,
                isostatic_elevation: felsic_target,
                ..GeodynamicColumn::default()
            },
        );
        geo.set(
            mafic,
            GeodynamicColumn {
                crustal_density: Fixed::from_ratio(300, 100),
                crustal_thickness: thickness,
                isostatic_elevation: mafic_target,
                ..GeodynamicColumn::default()
            },
        );
        relax_toward_isostasy(&mut earthwork, &geo, flat_base, Fixed::ONE).expect("valid fraction");
        // Each column snaps to its Airy target, and the felsic stands higher than the mafic from density alone.
        let felsic_eff = earthwork.total_delta(felsic);
        let mafic_eff = earthwork.total_delta(mafic);
        assert_eq!(felsic_eff, felsic_target, "felsic snapped to its target");
        assert_eq!(mafic_eff, mafic_target, "mafic snapped to its target");
        assert!(
            felsic_eff > mafic_eff,
            "the lighter crust stands higher, freeboard from density contrast"
        );
    }

    #[test]
    fn successive_partial_passes_drive_the_residual_toward_zero_the_solve_kernel_converges() {
        // The single pass is the kernel a fixed-cap solve will drive: successive partial passes reduce the worst
        // residual monotonically toward zero (the (1 - lambda)^n falloff), so an outer loop over it converges.
        // The outer loop itself reuses C's shared `solve.rs` on rebase rather than a hand-rolled copy here.
        let mut earthwork = EarthworkField::new();
        let mut geo = GeodynamicField::new();
        let a = Coord3::ground(0, 0);
        geo.set(
            a,
            GeodynamicColumn {
                isostatic_elevation: Fixed::from_int(5000),
                ..GeodynamicColumn::default()
            },
        );
        let tenth = Fixed::from_ratio(1, 10);
        // The first pass OBSERVES the full initial gap (5000) before adjusting, so seed the bound just above it;
        // each subsequent observed residual is 0.9 of the last, strictly decreasing.
        let mut prev = Fixed::from_int(5001);
        for _ in 0..8 {
            let residual = relax_toward_isostasy(&mut earthwork, &geo, flat_base, tenth)
                .expect("valid fraction");
            assert!(
                residual < prev,
                "each pass strictly reduces the residual (got {} after {})",
                residual.to_f64_lossy(),
                prev.to_f64_lossy()
            );
            prev = residual;
        }
        // After eight tenths-passes the crust is well on its way to the target (0.9^8 of the gap remains).
        assert!(
            earthwork.total_delta(a) > Fixed::from_int(2000),
            "the crust has relaxed most of the way toward the 5000 m target"
        );
    }

    #[test]
    fn an_out_of_range_relaxation_fraction_is_refused_fail_loud() {
        let mut earthwork = EarthworkField::new();
        let mut geo = GeodynamicField::new();
        geo.set(
            Coord3::ground(0, 0),
            GeodynamicColumn {
                isostatic_elevation: Fixed::from_int(100),
                ..GeodynamicColumn::default()
            },
        );
        // Zero never relaxes; above one overshoots. Both refused rather than clamped or fabricated.
        assert!(relax_toward_isostasy(&mut earthwork, &geo, flat_base, Fixed::ZERO).is_err());
        assert!(
            relax_toward_isostasy(&mut earthwork, &geo, flat_base, Fixed::from_ratio(3, 2))
                .is_err()
        );
    }

    #[test]
    fn an_unarmed_geology_relaxes_nothing_and_leaves_the_earthwork_untouched() {
        // The byte-neutral invariant: an empty geodynamic field yields no columns, so the consumer writes no
        // geological delta and the earthwork stays empty (its hash folds nothing, the base pins hold).
        let mut earthwork = EarthworkField::new();
        let geo = GeodynamicField::new();
        let observed = relax_toward_isostasy(&mut earthwork, &geo, flat_base, Fixed::ONE)
            .expect("valid fraction");
        assert_eq!(observed, Fixed::ZERO, "no column, no residual");
        assert!(earthwork.is_empty(), "an unarmed geology writes no delta");
    }

    #[test]
    fn the_relaxation_reads_the_worldgen_base_so_the_target_is_freeboard_above_it() {
        // The effective elevation is the base plus the delta, so relaxing toward a target from a non-zero base
        // writes only the freeboard the target stands above that base, not the whole target.
        let mut earthwork = EarthworkField::new();
        let mut geo = GeodynamicField::new();
        let a = Coord3::ground(7, 7);
        geo.set(
            a,
            GeodynamicColumn {
                isostatic_elevation: Fixed::from_int(1200),
                ..GeodynamicColumn::default()
            },
        );
        let base = |c: Coord3| {
            if c == a {
                Fixed::from_int(800)
            } else {
                Fixed::ZERO
            }
        };
        relax_toward_isostasy(&mut earthwork, &geo, base, Fixed::ONE).expect("valid fraction");
        // Base 800 + geological delta must equal target 1200, so the delta is the 400 m of freeboard.
        assert_eq!(
            earthwork.geological_delta(a),
            Fixed::from_int(400),
            "the delta lifts the base to the target, no more"
        );
    }
}
