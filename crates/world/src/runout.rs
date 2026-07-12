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

//! The runout integrator, the ONE general transfer law that places where a shed parcel of mass comes to
//! rest, so the non-local redistribution operator ([`crate::redistribute`]) has a distribution to apply.
//!
//! The design is the gate-ruled, panel-corrected form (PR #172): NOT a registry of per-process kernels (an
//! event naming which one to apply would read a process KIND to select a behaviour, the template case in
//! physics clothing, and a new process would be new code, failing admit-the-alien). Instead ONE fixed law
//! sums the floor FORCE primitives over the source's own physical state, and the named processes (ballistic
//! ejecta, granular runout, creep) EMERGE as regimes of that one law across the parameter space, never a
//! selected code path.
//!
//! The physics, in SPECIFIC energy (energy per unit mass), so the runout is mass-independent as it should
//! be (a parcel's reach depends on its launch speed and the friction, not on how much mass moves). A parcel
//! carries a specific-energy budget `e` (its launch kinetic energy per unit mass, `e0 = v^2 / 2`, the
//! `kinetic_energy` floor law per unit mass) and a directional momentum bias `heading`. Each step it moves
//! to the neighbour its momentum and gravity together favour, and pays the step's energy change
//! `de = g * (drop - mu * cell)`: gravity returns `g * drop` going downhill (a cost going up), and Coulomb
//! friction always dissipates `mu * g * cell` (the `friction` floor law's kinetic term `mu * normal` per
//! unit mass over the step, the `cos(theta)` of the slope cancelling between the normal force and the
//! along-slope distance). When the budget cannot cover the next step the parcel rests and deposits.
//!
//! The regimes fall out of the one law, none authored. The ANGLE OF REPOSE is the break-even slope
//! `drop = mu * cell`: a static parcel (`e0 = 0`) on a slope steeper than `mu` gains energy and flows, on a
//! gentler slope loses it and stays, so `arctan(mu)` (the floor `mech.static_friction`) is the stability
//! angle without a repose constant ever being written. A high-`e0` parcel climbs over sills (`drop < 0`
//! steps its budget can still pay), the NON-LOCAL reach neither local drainage nor local relaxation gives.
//! The runout distance scales with the dimensionless `e0 / (mu * g * cell)`, so a ballistic ejecta throw
//! (high `e0`) reaches far and a slow slump (low `e0`) stops near, the same law under different data.
//!
//! Determinism (Principle 3, Principle 10): every quantity is fixed-point, the neighbour choice is a total
//! order (the force score, then the lowest cell index on a tie), the march is bounded by a step cap (never
//! an unbounded until-rest spin), and energy strictly falls around any closed loop (friction is paid every
//! step, a zero-drop loop nets `-mu * g * cell` per step), so the path terminates and is a pure function of
//! the inputs, worker-invariant.

use civsim_core::Fixed;

/// Saturating fixed-point add on the raw bits, so an out-of-range intermediate clamps rather than panics or
/// wraps (determinism over an adversarial input; the physical inputs never reach the rails).
fn sadd(a: Fixed, b: Fixed) -> Fixed {
    Fixed::from_bits(a.to_bits().saturating_add(b.to_bits()))
}

/// Saturating fixed-point subtract on the raw bits.
fn ssub(a: Fixed, b: Fixed) -> Fixed {
    Fixed::from_bits(a.to_bits().saturating_sub(b.to_bits()))
}

/// Saturating fixed-point multiply: the checked product, or the correctly-signed rail on overflow.
fn smul(a: Fixed, b: Fixed) -> Fixed {
    a.checked_mul(b).unwrap_or_else(|| {
        if (a.to_bits() >= 0) == (b.to_bits() >= 0) {
            Fixed::MAX
        } else {
            Fixed::MIN
        }
    })
}

/// A shed parcel's launch state, the physical initial conditions an event supplies as DATA (never a named
/// kernel): its specific-energy budget and its directional momentum bias. Both are per-event physical
/// values; the law reads them, it does not read a process category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parcel {
    /// The launch specific energy `e0 = v^2 / 2` (energy per unit mass), the `kinetic_energy` floor law per
    /// unit mass. Zero is a static release (a slump that only gravity can move).
    pub energy: Fixed,
    /// The momentum bias along the grid axes, in specific-energy-per-step units (so it competes with the
    /// gravity term in the neighbour score on the same scale). The zero vector is a pure gravity release;
    /// a large bias is a directed throw that flies its heading until the budget runs down.
    pub heading: (Fixed, Fixed),
}

/// The world force parameters the runout law reads, each a floor axis or a per-world datum (never authored
/// here): gravity, the material's Coulomb friction coefficient, and the cell edge length. The step cap is a
/// determinism bound (the march terminates in at most this many steps), not a physical value.
#[derive(Clone, Copy, Debug)]
pub struct RunoutForces {
    /// Gravity `g` (`mech.gravitational_acceleration`).
    pub gravity: Fixed,
    /// The material's kinetic Coulomb friction coefficient `mu` (`mech.kinetic_friction`); its arctangent is
    /// the emergent angle of repose.
    pub friction: Fixed,
    /// The cell edge length in world units (a per-world spatial datum), the horizontal distance of one step.
    pub cell_size: Fixed,
    /// The maximum number of steps the march may take, a determinism and performance bound (the path is a
    /// pure function up to this cap), never a physical runout length.
    pub step_cap: u32,
}

/// The path a parcel takes and where it deposits: the cells it passed through in order (the source first),
/// the cell it came to rest in, and the specific energy left when it stopped. The rest cell is the
/// single-parcel deposit site the redistribution operator credits; the multi-parcel fan that spreads a
/// source's mass into a blanket is the next slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunoutPath {
    /// The cells the parcel occupied, in travel order, starting with the source.
    pub cells: Vec<usize>,
    /// The cell the parcel came to rest in (the deposit site).
    pub rest: usize,
    /// The specific energy remaining at rest (non-negative; zero when friction exactly spent the budget).
    pub remaining_energy: Fixed,
}

/// The 4-connected orthogonal neighbours of a cell as `(flat index, dx, dy)`, in a fixed canonical order
/// (west, east, north, south), so the neighbour walk is deterministic.
fn neighbors4(cell: usize, width: usize, height: usize) -> Vec<(usize, i32, i32)> {
    let x = cell % width;
    let y = cell / width;
    let mut out = Vec::with_capacity(4);
    if x > 0 {
        out.push((cell - 1, -1, 0));
    }
    if x + 1 < width {
        out.push((cell + 1, 1, 0));
    }
    if y > 0 {
        out.push((cell - width, 0, -1));
    }
    if y + 1 < height {
        out.push((cell + width, 0, 1));
    }
    out
}

/// The momentum contribution to a neighbour's score: the heading dotted with the step offset. The offsets
/// are `-1`, `0`, or `1`, so this is a sign selection over the heading components, no multiply needed.
fn heading_bias(heading: (Fixed, Fixed), dx: i32, dy: i32) -> Fixed {
    let hx = match dx {
        1 => heading.0,
        -1 => ssub(Fixed::ZERO, heading.0),
        _ => Fixed::ZERO,
    };
    let hy = match dy {
        1 => heading.1,
        -1 => ssub(Fixed::ZERO, heading.1),
        _ => Fixed::ZERO,
    };
    sadd(hx, hy)
}

/// Integrate a shed parcel's runout over a `width` by `height` fixed-point elevation grid from `source`,
/// returning the path it takes and the cell it deposits in. Deterministic and worker-invariant: fixed-point
/// throughout, a total-order neighbour choice (force score then lowest index), and a bounded march (at most
/// `forces.step_cap` steps). Panics if `elevation.len()` is not `width * height` or `source` is out of range
/// (construction invariants). See the module docs for the physics and the regimes that emerge from it.
pub fn runout(
    width: usize,
    height: usize,
    elevation: &[Fixed],
    source: usize,
    parcel: Parcel,
    forces: RunoutForces,
) -> RunoutPath {
    assert_eq!(
        elevation.len(),
        width * height,
        "elevation grid length {} must equal width*height {}",
        elevation.len(),
        width * height
    );
    assert!(
        source < width * height,
        "source {source} is outside the {}-cell grid",
        width * height
    );
    // The friction cost of one step is the same everywhere: mu * g * cell (the Coulomb dissipation per unit
    // mass over the horizontal step). A parcel must have this much specific energy plus whatever the slope
    // costs to advance, so a shallow-enough slope with no launch energy never moves (the repose stability).
    let friction_cost = smul(smul(forces.friction, forces.gravity), forces.cell_size);

    let mut cells = vec![source];
    let mut cur = source;
    let mut energy = parcel.energy;

    for _ in 0..forces.step_cap {
        // Score each neighbour by the force the parcel feels toward it: gravity (returned energy for a drop)
        // plus the momentum bias. The best score, then the lowest cell index on a tie, is a total order.
        let mut best: Option<(usize, Fixed, Fixed)> = None; // (neighbour, score, drop)
        for (nbr, dx, dy) in neighbors4(cur, width, height) {
            let drop = ssub(elevation[cur], elevation[nbr]);
            let gravity_gain = smul(forces.gravity, drop);
            let score = sadd(gravity_gain, heading_bias(parcel.heading, dx, dy));
            let take = match best {
                None => true,
                Some((bn, bscore, _)) => score > bscore || (score == bscore && nbr < bn),
            };
            if take {
                best = Some((nbr, score, drop));
            }
        }
        let (nbr, _, drop) = match best {
            Some(b) => b,
            None => break, // a zero-area interior with no neighbour (only the degenerate 1-cell grid)
        };
        // The step's specific-energy change: gravity returns g*drop (a cost when climbing, drop < 0), and
        // friction always dissipates its fixed cost. If the budget cannot cover it, the parcel rests here.
        let gravity_gain = smul(forces.gravity, drop);
        let next_energy = ssub(sadd(energy, gravity_gain), friction_cost);
        if next_energy < Fixed::ZERO {
            break;
        }
        energy = next_energy;
        cur = nbr;
        cells.push(cur);
    }

    RunoutPath {
        cells,
        rest: cur,
        remaining_energy: energy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redistribute::{redistribute, Redistribution, Weighted};

    // A flat world tilted at a constant grade: elevation falls by `grade` per cell toward the west (lower x),
    // so every cell's steepest descent is toward x = 0. `grade` is the drop per cell, the tangent of the
    // slope in cell units, so comparing it to the friction coefficient is comparing the slope to the repose
    // angle directly.
    fn ramp(width: usize, height: usize, grade: Fixed) -> Vec<Fixed> {
        let mut e = vec![Fixed::ZERO; width * height];
        for y in 0..height {
            for x in 0..width {
                e[y * width + x] = grade.mul(Fixed::from_int(x as i32));
            }
        }
        e
    }

    fn forces(gravity: Fixed, friction: Fixed, cell: Fixed, cap: u32) -> RunoutForces {
        RunoutForces {
            gravity,
            friction,
            cell_size: cell,
            step_cap: cap,
        }
    }

    fn still() -> (Fixed, Fixed) {
        (Fixed::ZERO, Fixed::ZERO)
    }

    #[test]
    fn a_static_parcel_below_the_repose_angle_does_not_move() {
        // Slope grade 1/4, friction 1/2: the slope is gentler than the repose angle (grade < mu), so a
        // parcel with no launch energy loses energy on any step and stays put. The angle of repose emerges
        // from the friction coefficient, no repose constant authored.
        let e = ramp(5, 1, Fixed::from_ratio(1, 4));
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 100);
        let path = runout(
            5,
            1,
            &e,
            4,
            Parcel {
                energy: Fixed::ZERO,
                heading: still(),
            },
            f,
        );
        assert_eq!(path.rest, 4, "a sub-repose slope holds the parcel");
        assert_eq!(path.cells, vec![4]);
    }

    #[test]
    fn a_static_parcel_above_the_repose_angle_flows_to_the_foot() {
        // Slope grade 3/4, friction 1/2: steeper than the repose angle (grade > mu), so a static parcel
        // gains energy each step and flows all the way down to the western foot (cell 0).
        let e = ramp(5, 1, Fixed::from_ratio(3, 4));
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 100);
        let path = runout(
            5,
            1,
            &e,
            4,
            Parcel {
                energy: Fixed::ZERO,
                heading: still(),
            },
            f,
        );
        assert_eq!(path.rest, 0, "a supra-repose slope flows to the foot");
        assert_eq!(path.cells, vec![4, 3, 2, 1, 0]);
    }

    #[test]
    fn launch_energy_carries_a_parcel_farther_the_runout_scales_with_the_budget() {
        // On a sub-repose slope a static parcel is stuck, but a launch budget carries it downslope until
        // friction spends the budget; more budget reaches farther (the e0 / (mu*g*cell) regime knob).
        let e = ramp(9, 1, Fixed::from_ratio(1, 4));
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 100);
        // Net cost per downhill step = mu*g*cell - g*drop = 1/2 - 1/4 = 1/4 specific energy per step.
        let small = runout(
            9,
            1,
            &e,
            8,
            Parcel {
                energy: Fixed::from_ratio(1, 2),
                heading: still(),
            },
            f,
        );
        let large = runout(
            9,
            1,
            &e,
            8,
            Parcel {
                energy: Fixed::from_int(1),
                heading: still(),
            },
            f,
        );
        assert!(
            large.rest < small.rest,
            "a larger launch budget rests farther downslope ({} vs {})",
            large.rest,
            small.rest
        );
        assert!(small.rest < 8, "the launched parcel moves at all");
    }

    #[test]
    fn a_launched_parcel_climbs_over_a_sill_the_non_local_reach() {
        // A pit-sill-basin profile: cell 0 low, cell 1 a sill (high), cell 2 a basin beyond it. A parcel
        // launched east from cell 0 with enough budget climbs the sill (an uphill step its budget pays for)
        // and comes to rest beyond it, the non-local reach local downhill routing cannot express.
        let mut e = vec![Fixed::ZERO; 3];
        e[0] = Fixed::ZERO;
        e[1] = Fixed::from_int(2); // the sill
        e[2] = Fixed::ONE; // the basin floor beyond the sill
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 10), Fixed::ONE, 100);
        // Head east with a large budget: it must pay g*2 to climb the sill plus friction, then settle.
        let path = runout(
            3,
            1,
            &e,
            0,
            Parcel {
                energy: Fixed::from_int(5),
                heading: (Fixed::from_int(10), Fixed::ZERO),
            },
            f,
        );
        assert!(
            path.cells.contains(&2),
            "the launched parcel cleared the sill into the far basin (path {:?})",
            path.cells
        );
    }

    #[test]
    fn the_runout_is_deterministic() {
        let e = ramp(7, 3, Fixed::from_ratio(3, 5));
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 4), Fixed::ONE, 200);
        let p = Parcel {
            energy: Fixed::from_int(2),
            heading: (Fixed::from_ratio(1, 3), Fixed::from_ratio(-1, 7)),
        };
        let a = runout(7, 3, &e, 7 * 3 - 1, p, f);
        let b = runout(7, 3, &e, 7 * 3 - 1, p, f);
        assert_eq!(a, b, "the same parcel and world reproduce the same path");
    }

    #[test]
    fn the_march_is_bounded_by_the_step_cap() {
        // A steep endless-looking descent capped low: the path length can never exceed the cap plus the
        // source cell, so the march is bounded, never an unbounded until-rest spin.
        let e = ramp(50, 1, Fixed::from_int(1));
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 100), Fixed::ONE, 3);
        let path = runout(
            50,
            1,
            &e,
            49,
            Parcel {
                energy: Fixed::from_int(1),
                heading: still(),
            },
            f,
        );
        assert!(
            path.cells.len() <= 3 + 1,
            "the path is bounded by the step cap ({} cells)",
            path.cells.len()
        );
    }

    #[test]
    fn the_integrator_feeds_the_operator_a_conservative_move() {
        // The integrator places the rest cell; the operator moves the source's mass there conservatively.
        // Together they are one non-local, mass-conserving surface transport: the delta field sums to zero.
        let e = ramp(6, 1, Fixed::from_ratio(3, 4));
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 4), Fixed::ONE, 100);
        let source = 5;
        let path = runout(
            6,
            1,
            &e,
            source,
            Parcel {
                energy: Fixed::ZERO,
                heading: still(),
            },
            f,
        );
        let mass = 1000;
        let delta = redistribute(
            6,
            &[Redistribution {
                source,
                mass,
                dests: vec![Weighted {
                    dest: path.rest,
                    weight: 1,
                }],
            }],
        )
        .unwrap();
        assert_eq!(
            delta.iter().sum::<i64>(),
            0,
            "the surface transport conserves mass"
        );
        assert_eq!(delta[source], -mass, "the source shed its mass");
        assert_eq!(delta[path.rest], mass, "the runout toe received it");
        assert_ne!(path.rest, source, "the mass moved to a different cell");
    }

    #[test]
    fn a_single_cell_world_rests_in_place() {
        let e = vec![Fixed::ZERO];
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 10);
        let path = runout(
            1,
            1,
            &e,
            0,
            Parcel {
                energy: Fixed::from_int(5),
                heading: (Fixed::from_int(3), Fixed::ZERO),
            },
            f,
        );
        assert_eq!(path.rest, 0);
        assert_eq!(path.cells, vec![0]);
    }
}
