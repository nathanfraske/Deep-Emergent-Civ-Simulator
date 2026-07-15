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

//! The impact-event composer: the ONE function that turns a single drawn impact into a conservative change of
//! the terrain elevation field, by composing the three already-built primitives of the impact chain and adding
//! nothing but the bowl geometry and the wiring. It excavates the crater the crater-scaling law
//! ([`crate::crater`]) derives, launches that excavated material as the isotropic ballistic ejecta fan
//! ([`crate::ballistic::ejecta_fan`]) computes, and deposits the blanket through the conservative
//! redistribution operator ([`crate::redistribute`]). No primitive is reimplemented here; this module is the
//! seam that joins them.
//!
//! The composition, in causal order. The crater law reads the impactor's and the target's own physical state
//! and returns the transient bowl's rim diameter `D` and depth `h`. The bowl is a paraboloid depression,
//! `z(r) = h (1 - (r/R)^2)` with `R = D/2` the rim radius, which is the SAME shape the crater law's own volume
//! `V = (pi/8)(h/D)D^3` integrates to (a paraboloid of depth `h` and radius `R` has volume `(pi/2) h R^2 =
//! (pi/8) h D^2`), so the per-cell depression is derived from the law's geometry, never an authored profile.
//! Each cell inside the rim drops by its bowl depth; the raw-bit drop is the field mass leaving the bowl. That
//! excavated mass is the source term the ejecta fan launches and the redistribution operator deposits, so the
//! blanket is laid around the crater and the whole excavated bowl is redeposited: excavated equals deposited by
//! the operator's construction, and the net delta field sums to exactly zero.
//!
//! Conservation (Principle 3, Principle 10). The net delta is a single call to the redistribution operator over
//! a move set whose sources are the excavated bowl cells (each shedding its own depth's worth of field units)
//! and whose destinations are the fan's landing distribution. The operator debits each source and credits the
//! destinations the identical apportioned total by the exact largest-remainder method, so the returned delta
//! field sums to exactly zero: the elevation material is moved, never created or lost. The composition is a
//! pure function of the impact, the material, the launch, and the terrain, worker-invariant, because every
//! primitive it calls is.
//!
//! Admit-the-alien (a prime directive) and the value line (Principle 11). Every input is a datum: the impactor
//! and target state and the crater coupling flow straight into the crater law, the ejecta launch is the
//! caller's per-event [`EjectaFan`] (the ejecta speed and angle are physical launch data, not authored here),
//! and the world's gravity and cell size arrive in the [`BallisticForces`]. An iron bolide into an ice shell on
//! a low-gravity world is a different set of numbers through the same composer, not a new code path. The
//! composer authors no value: the only new arithmetic is the paraboloid bowl, whose shape is the crater law's
//! own. A crater that does not resolve, a degenerate call, or a fan with nowhere to land falls SOFT to a zero
//! delta field (no change), never a fabricated crater or a silent loss of mass.
//!
//! What this slice does not yet model: the transient bowl's collapse. The crater law's escaping-ejecta fraction
//! ([`Crater::ejecta_mass_ratio`], the part that clears the rim as opposed to the breccia that slumps back)
//! would split the blanket outside the rim from the fallback inside it in a later crater-collapse slice; here
//! the whole excavated bowl is redeposited so the field conserves exactly. This composer is dormant until the
//! deep-time run loop draws impacts into it (a later wiring slice), so it does not move the run-loop byte pins.

use crate::ballistic::{ejecta_fan, BallisticForces, EjectaFan};
use crate::crater::{crater, CraterCoupling, Impactor, Target};
use crate::redistribute::{redistribute, Redistribution};
use civsim_core::Fixed;

/// Apply one impact to a terrain elevation field and return the conservative net elevation delta it implies, in
/// the field's own raw integer units (the caller adds these deltas to the elevation bits). The `elevation` grid
/// is `width` by `height` in row-major order; `source` is the impact cell. The impact is described entirely by
/// data: the drawn [`Impactor`] and [`Target`] and the material [`CraterCoupling`] the crater law reads, the
/// per-event [`EjectaFan`] launch, and the world [`BallisticForces`] (gravity and cell size). The returned
/// delta sums to exactly zero (the excavated bowl equals the deposited blanket). A crater that does not resolve,
/// a degenerate call (a mismatched grid length, an out-of-range source, a non-positive crater radius or bowl
/// depth), or a fan with nowhere to land yields a zero delta field, never a fabricated crater or a lost gram of
/// mass.
///
/// The units couple through the caller's own data: the impactor radius, the crater diameter and depth, and the
/// elevation field are one length scale, and [`BallisticForces::cell_size`] is that same length per grid cell,
/// so the crater diameter maps to a cell radius and the bowl depth maps to a raw-bit elevation drop.
#[allow(clippy::too_many_arguments)]
pub fn apply_impact(
    width: usize,
    height: usize,
    elevation: &[Fixed],
    source: usize,
    impactor: Impactor,
    target: Target,
    coupling: CraterCoupling,
    fan: EjectaFan,
    forces: BallisticForces,
) -> Vec<i64> {
    let n_cells = width.saturating_mul(height);
    let zero_field = || vec![0i64; n_cells];

    // Construction guards: fail soft on a degenerate call rather than panic, so a bad draw cannot crash the run.
    if width == 0 || height == 0 || elevation.len() != n_cells || source >= n_cells {
        return zero_field();
    }

    // 1. The crater. A non-physical or unbounded impact yields no crater; fall soft to no change.
    let bowl = match crater(impactor, target, coupling) {
        Some(c) => c,
        None => return zero_field(),
    };
    let depth = bowl.depth;
    if depth <= Fixed::ZERO {
        return zero_field();
    }

    // The rim radius in cells: R = (D / 2) / cell_size. A non-positive radius (a sub-grid or degenerate crater)
    // is no change.
    let radius_cells = match bowl
        .diameter
        .checked_div(Fixed::from_int(2))
        .and_then(|r| r.checked_div(forces.cell_size))
    {
        Some(r) if r > Fixed::ZERO => r,
        _ => return zero_field(),
    };
    let radius_sq = match radius_cells.checked_mul(radius_cells) {
        Some(rsq) if rsq > Fixed::ZERO => rsq,
        _ => return zero_field(),
    };

    // 2. Excavate the paraboloid bowl. z(r) = h (1 - (r/R)^2): deepest h at the centre, zero at the rim, the
    // crater law's own volume shape. (r/R)^2 = (dx^2 + dy^2) / R^2 in cell units, the cell size cancelling top
    // and bottom, so the depression is pure integer-offset geometry. floor(R) bounds the footprint: no cell one
    // step beyond it can lie inside the rim.
    let sx = (source % width) as i32;
    let sy = (source / width) as i32;
    let rint = radius_cells.to_int();
    let mut excavation: Vec<(usize, i64)> = Vec::new();
    for dy in -rint..=rint {
        let cy = sy + dy;
        if cy < 0 || cy >= height as i32 {
            continue;
        }
        let dyf = Fixed::from_int(dy);
        for dx in -rint..=rint {
            let cx = sx + dx;
            if cx < 0 || cx >= width as i32 {
                continue;
            }
            let dxf = Fixed::from_int(dx);
            // dx^2 + dy^2 as a Fixed; a wildly large offset that would overflow the square is skipped, not
            // fabricated.
            let dist_sq = match dxf
                .checked_mul(dxf)
                .and_then(|xx| dyf.checked_mul(dyf).and_then(|yy| xx.checked_add(yy)))
            {
                Some(v) => v,
                None => continue,
            };
            if dist_sq > radius_sq {
                continue; // outside the rim
            }
            let frac = match dist_sq.checked_div(radius_sq) {
                Some(f) => f,
                None => continue,
            };
            let well = match Fixed::ONE
                .checked_sub(frac)
                .and_then(|one_minus| depth.checked_mul(one_minus))
            {
                Some(w) if w > Fixed::ZERO => w,
                _ => continue,
            };
            let cell = cy as usize * width + cx as usize;
            // The bowl depth in raw field bits is the field mass that leaves this cell.
            excavation.push((cell, well.to_bits()));
        }
    }
    if excavation.is_empty() {
        return zero_field(); // a wholly clipped or sub-cell bowl removed nothing
    }

    // 3. Launch the isotropic ejecta fan from the impact site over the PRE-impact terrain (the ejecta clears the
    // terrain as it was at the strike) and aggregate its landing distribution.
    let blanket = ejecta_fan(width, height, elevation, source, fan, forces);
    if blanket.is_empty() {
        return zero_field(); // the ejecta has nowhere to land; refuse rather than lose the excavated mass
    }

    // Every excavated bowl cell sheds its own bowl mass along that same landing distribution, so the whole
    // excavated bowl is redeposited as the blanket. The redistribution operator conserves it exactly: each
    // source is debited and the destinations credited the identical apportioned total.
    let moves: Vec<Redistribution> = excavation
        .iter()
        .map(|&(cell, mass)| Redistribution {
            source: cell,
            mass,
            dests: blanket.clone(),
        })
        .collect();

    // 4. The net delta: bowl excavation (negative) plus the ejecta blanket (positive), summing to exactly zero.
    // Any fail-loud refusal (an overflowed credit, an unplaceable mass) falls soft to a zero field.
    redistribute(n_cells, &moves).unwrap_or_else(|_| zero_field())
}

#[cfg(test)]
mod tests {
    use super::*;

    // An illustrative competent-silicate coupling, standing in for one material's reserved row (a TEST fixture,
    // not authored floor values; the law reads it). Mirrors the crater-law tests.
    fn rock_coupling() -> CraterCoupling {
        CraterCoupling {
            velocity_exponent: Fixed::from_ratio(55, 100),
            density_exponent: Fixed::from_ratio(4, 10),
            efficiency_coefficient: Fixed::from_ratio(2, 10),
            strength_coefficient: Fixed::ONE,
            bowl_aspect: Fixed::from_ratio(2, 10),
            eject_fraction: Fixed::from_ratio(5, 10),
        }
    }

    fn moon_target() -> Target {
        Target {
            gravity: Fixed::from_ratio(162, 100),
            strength: Fixed::from_int(10_000_000),
            density: Fixed::from_int(2500),
        }
    }

    fn km_impactor() -> Impactor {
        Impactor {
            radius: Fixed::from_int(500),
            velocity: Fixed::from_int(17_000),
            density: Fixed::from_int(3000),
        }
    }

    // A world with kilometre cells, so the kilometre-class crater spans a handful of cells on a small grid.
    fn forces() -> BallisticForces {
        BallisticForces {
            gravity: Fixed::from_ratio(162, 100), // the moon target's own gravity
            cell_size: Fixed::from_int(1500),     // 1.5 km cells
            step_cap: 200,
        }
    }

    // An ejecta launch whose flat-ground range lands the blanket well outside the crater footprint, so on flat
    // terrain the bowl cells are pure sources and the ring cells are pure deposits.
    fn ejecta() -> EjectaFan {
        EjectaFan {
            speed: Fixed::from_int(270), // v^2 sin(2a)/g ~ 270^2 / 1.62 / 1500 ~ 30 cells on flat ground
            elevation_angle: Fixed::HALF_PI.div(Fixed::from_int(2)), // 45 degrees, the max-range angle
            azimuths: 24,
        }
    }

    // The cell distance from the source, squared (integer), for classifying a cell as bowl or blanket.
    fn dist_sq(cell: usize, source: usize, width: usize) -> i64 {
        let (cx, cy) = ((cell % width) as i64, (cell / width) as i64);
        let (sx, sy) = ((source % width) as i64, (source / width) as i64);
        (cx - sx) * (cx - sx) + (cy - sy) * (cy - sy)
    }

    #[test]
    fn an_impact_carves_a_bowl_and_lays_a_blanket() {
        let (w, h) = (81usize, 81usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = 40 * w + 40;
        let delta = apply_impact(
            w,
            h,
            &flat,
            source,
            km_impactor(),
            moon_target(),
            rock_coupling(),
            ejecta(),
            forces(),
        );
        // The centre of the bowl drops (the deepest point of the excavation).
        assert!(
            delta[source] < 0,
            "the impact centre drops (delta {})",
            delta[source]
        );
        // The rim radius the composer used, so the test classifies cells the same way.
        let bowl = crater(km_impactor(), moon_target(), rock_coupling()).expect("resolves");
        let radius_cells = bowl
            .diameter
            .div(Fixed::from_int(2))
            .div(Fixed::from_int(1500));
        let rsq_int = radius_cells.mul(radius_cells).to_int() as i64; // floor(R^2) in integer cell units
                                                                      // Every bowl cell (inside the rim) is a pure source here (the ring lands outside), so none rose.
        let bowl_rose = (0..w * h).any(|c| delta[c] > 0 && dist_sq(c, source, w) <= rsq_int);
        assert!(
            !bowl_rose,
            "no cell inside the rim rose (the bowl is a clean dig)"
        );
        // A ring outside the rim rose: the ejecta blanket landed around the crater.
        let ring_rose = (0..w * h).any(|c| delta[c] > 0 && dist_sq(c, source, w) > rsq_int);
        assert!(
            ring_rose,
            "a ring outside the rim rose (the ejecta blanket)"
        );
    }

    #[test]
    fn the_net_delta_field_conserves_to_exactly_zero() {
        let (w, h) = (81usize, 81usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = 40 * w + 40;
        let delta = apply_impact(
            w,
            h,
            &flat,
            source,
            km_impactor(),
            moon_target(),
            rock_coupling(),
            ejecta(),
            forces(),
        );
        assert_eq!(
            delta.iter().sum::<i64>(),
            0,
            "the excavated bowl equals the deposited blanket (mass is moved, never created or lost)"
        );
        // And it is not a trivial all-zero field: material really moved.
        assert!(delta.iter().any(|&d| d != 0), "the impact moved material");
    }

    #[test]
    fn a_bigger_faster_impactor_makes_a_bigger_crater() {
        let (w, h) = (81usize, 81usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = 40 * w + 40;
        let slow = apply_impact(
            w,
            h,
            &flat,
            source,
            Impactor {
                velocity: Fixed::from_int(10_000),
                ..km_impactor()
            },
            moon_target(),
            rock_coupling(),
            ejecta(),
            forces(),
        );
        let fast = apply_impact(
            w,
            h,
            &flat,
            source,
            Impactor {
                radius: Fixed::from_int(700),
                velocity: Fixed::from_int(30_000),
                ..km_impactor()
            },
            moon_target(),
            rock_coupling(),
            ejecta(),
            forces(),
        );
        // The bigger, faster strike opens a wider bowl: more cells dropped (the footprint grew). On this flat
        // grid the ring lands outside both footprints, so the dropped cells are exactly the excavation.
        let slow_pits = slow.iter().filter(|&&d| d < 0).count();
        let fast_pits = fast.iter().filter(|&&d| d < 0).count();
        assert!(
            fast_pits > slow_pits,
            "the bigger, faster impactor drops more cells ({fast_pits} vs {slow_pits})"
        );
        // And the total excavated (the summed drop) is larger.
        let slow_dug: i128 = slow.iter().filter(|&&d| d < 0).map(|&d| -(d as i128)).sum();
        let fast_dug: i128 = fast.iter().filter(|&&d| d < 0).map(|&d| -(d as i128)).sum();
        assert!(
            fast_dug > slow_dug,
            "the bigger, faster impactor excavates more mass ({fast_dug} vs {slow_dug})"
        );
        assert_eq!(slow.iter().sum::<i64>(), 0);
        assert_eq!(fast.iter().sum::<i64>(), 0);
    }

    #[test]
    fn an_alien_impactor_and_target_are_a_data_row() {
        // An iron bolide into a soft ice shell on a low-gravity world: the same composer, different numbers, a
        // finite conservative crater. No Terran assumption blocks it.
        let (w, h) = (81usize, 81usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = 40 * w + 40;
        let delta = apply_impact(
            w,
            h,
            &flat,
            source,
            Impactor {
                radius: Fixed::from_int(300),
                velocity: Fixed::from_int(25_000),
                density: Fixed::from_int(7800), // iron
            },
            Target {
                gravity: Fixed::from_ratio(50, 100),  // a small icy moon
                strength: Fixed::from_int(1_000_000), // soft ice
                density: Fixed::from_int(920),        // water ice
            },
            CraterCoupling {
                bowl_aspect: Fixed::from_ratio(25, 100),
                eject_fraction: Fixed::from_ratio(4, 10),
                ..rock_coupling()
            },
            EjectaFan {
                speed: Fixed::from_int(130), // ~130^2 / 0.5 / 1000 ~ 34 cells on flat ground, on-grid
                elevation_angle: Fixed::HALF_PI.div(Fixed::from_int(2)),
                azimuths: 24,
            },
            BallisticForces {
                gravity: Fixed::from_ratio(50, 100),
                cell_size: Fixed::from_int(1000),
                step_cap: 200,
            },
        );
        assert!(delta[source] < 0, "the alien impact carves a bowl");
        assert!(
            delta.iter().any(|&d| d > 0),
            "the alien impact lays a blanket"
        );
        assert_eq!(
            delta.iter().sum::<i64>(),
            0,
            "the alien impact conserves elevation mass"
        );
    }

    #[test]
    fn the_impact_is_deterministic() {
        let (w, h) = (81usize, 81usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = 40 * w + 40;
        let a = apply_impact(
            w,
            h,
            &flat,
            source,
            km_impactor(),
            moon_target(),
            rock_coupling(),
            ejecta(),
            forces(),
        );
        let b = apply_impact(
            w,
            h,
            &flat,
            source,
            km_impactor(),
            moon_target(),
            rock_coupling(),
            ejecta(),
            forces(),
        );
        assert_eq!(a, b, "the same impact reproduces the same delta field");
    }

    #[test]
    fn degenerate_inputs_fail_soft_to_a_zero_field() {
        let (w, h) = (41usize, 41usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = 20 * w + 20;
        let z = vec![0i64; w * h];

        // A non-physical impactor (zero radius): no crater, so no change.
        assert_eq!(
            apply_impact(
                w,
                h,
                &flat,
                source,
                Impactor {
                    radius: Fixed::ZERO,
                    ..km_impactor()
                },
                moon_target(),
                rock_coupling(),
                ejecta(),
                forces(),
            ),
            z,
            "a zero-radius impactor is no change"
        );

        // A fan with no azimuths: the ejecta has nowhere to land, so the composer refuses (no change) rather
        // than lose the excavated mass.
        assert_eq!(
            apply_impact(
                w,
                h,
                &flat,
                source,
                km_impactor(),
                moon_target(),
                rock_coupling(),
                EjectaFan {
                    azimuths: 0,
                    ..ejecta()
                },
                forces(),
            ),
            z,
            "a fan with no azimuths is no change"
        );

        // A mismatched grid length is a degenerate call: no change (and no panic).
        assert_eq!(
            apply_impact(
                w,
                h,
                &flat[..w * h - 1],
                source,
                km_impactor(),
                moon_target(),
                rock_coupling(),
                ejecta(),
                forces(),
            ),
            z,
            "a mismatched grid length is no change"
        );

        // An out-of-range source is a degenerate call: no change.
        assert_eq!(
            apply_impact(
                w,
                h,
                &flat,
                w * h + 5,
                km_impactor(),
                moon_target(),
                rock_coupling(),
                ejecta(),
                forces(),
            ),
            z,
            "an out-of-range source is no change"
        );
    }
}
