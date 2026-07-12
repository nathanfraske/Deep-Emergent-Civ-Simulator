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

/// An OPEN body-force term the runout law sums beside gravity: a per-world force that does work on the
/// parcel along the gradient of its own potential field, keyed on that field, so an alien force is a DATA
/// ROW rather than a code path (admit-the-alien). Gravity is the built-in body force (its potential is the
/// terrain elevation, its coefficient `g`); a magical world adds a mana-pressure gradient as one of these
/// (its potential the mana field, its coefficient the mana-force axis), a photosynthetic lofting or an
/// electrostatic hop likewise, and the one integrator sums whichever are present. The coefficient is a
/// floor axis (Principle 9); nothing here is authored, the force is read from the world's own data.
#[derive(Clone, Copy, Debug)]
pub struct BodyForce<'a> {
    /// The force coefficient (a floor axis), the work per unit potential drop per unit mass.
    pub coefficient: Fixed,
    /// The potential field over the grid whose downhill gradient the force drives the parcel along (one
    /// entry per cell, `width * height` long). A drop in this potential returns energy, a rise costs it.
    pub potential: &'a [Fixed],
}

/// The dimensionless runout-scale ratio, the launch specific energy over the per-step friction dissipation
/// `mu * g * cell`: the number of flat-ground steps a parcel's budget carries it before friction spends it,
/// the KE-to-dissipation ratio the regimes read out from. A small ratio is a slump that stops near, a large
/// ratio a throw that reaches far; the regime is this DERIVED value, never a named category. Zero when the
/// dissipation is non-positive (a frictionless or zero-gravity degenerate world), the caller's signal that
/// the scale is unbounded rather than a fabricated number.
pub fn regime_ratio(energy: Fixed, forces: RunoutForces) -> Fixed {
    let dissipation = smul(smul(forces.friction, forces.gravity), forces.cell_size);
    if dissipation <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    energy.checked_div(dissipation).unwrap_or(Fixed::MAX)
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

/// The energy an extra body force set returns for a step from `cur` to `nbr`: the sum over the forces of the
/// coefficient times the drop in that force's potential. The caller ([`runout_with_forces`]) validates every
/// potential to the grid length, so the bounds check here is a safe fallback that never fires in practice.
fn body_gain(extra: &[BodyForce], cur: usize, nbr: usize) -> Fixed {
    let mut gain = Fixed::ZERO;
    for f in extra {
        if let (Some(&pc), Some(&pn)) = (f.potential.get(cur), f.potential.get(nbr)) {
            gain = sadd(gain, smul(f.coefficient, ssub(pc, pn)));
        }
    }
    gain
}

/// Integrate a shed parcel's runout over a `width` by `height` fixed-point elevation grid under gravity
/// alone, the common case. A convenience wrapper over [`runout_with_forces`] with no extra body forces; see
/// it and the module docs for the physics.
pub fn runout(
    width: usize,
    height: usize,
    elevation: &[Fixed],
    source: usize,
    parcel: Parcel,
    forces: RunoutForces,
) -> RunoutPath {
    runout_with_forces(width, height, elevation, source, parcel, forces, &[])
}

/// Integrate a shed parcel's runout over a `width` by `height` fixed-point elevation grid from `source`,
/// returning the path it takes and the cell it deposits in. Beside gravity (the terrain-potential body
/// force) the parcel feels the `extra` open body forces, each summed along the gradient of its own field, so
/// an alien force (a mana-pressure gradient) drives transport as a data row (admit-the-alien). Deterministic
/// and worker-invariant: fixed-point throughout, a total-order neighbour choice (force score then lowest
/// index), and a bounded march (at most `forces.step_cap` steps). Panics if `elevation.len()` is not
/// `width * height` or `source` is out of range (construction invariants). See the module docs for the
/// physics and the regimes that emerge from it.
pub fn runout_with_forces(
    width: usize,
    height: usize,
    elevation: &[Fixed],
    source: usize,
    parcel: Parcel,
    forces: RunoutForces,
    extra: &[BodyForce],
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
    for f in extra {
        assert_eq!(
            f.potential.len(),
            width * height,
            "an extra body-force potential of length {} must equal width*height {}",
            f.potential.len(),
            width * height
        );
    }
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
        // The total body-force gain toward a neighbour: gravity (g times the elevation drop) plus the open
        // extra forces (each summed along its own potential gradient). The parcel is drawn where this total,
        // plus its momentum bias, is largest.
        let mut best: Option<(usize, Fixed, Fixed)> = None; // (neighbour, score, total force gain)
        for (nbr, dx, dy) in neighbors4(cur, width, height) {
            let drop = ssub(elevation[cur], elevation[nbr]);
            let force_gain = sadd(smul(forces.gravity, drop), body_gain(extra, cur, nbr));
            let score = sadd(force_gain, heading_bias(parcel.heading, dx, dy));
            let take = match best {
                None => true,
                Some((bn, bscore, _)) => score > bscore || (score == bscore && nbr < bn),
            };
            if take {
                best = Some((nbr, score, force_gain));
            }
        }
        let (nbr, _, force_gain) = match best {
            Some(b) => b,
            None => break, // a zero-area interior with no neighbour (only the degenerate 1-cell grid)
        };
        // The step's specific-energy change: the summed body forces return `force_gain` (a cost when it is
        // negative, climbing against them), and friction always dissipates its fixed cost. If the budget
        // cannot cover it, the parcel rests here.
        let next_energy = ssub(sadd(energy, force_gain), friction_cost);
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

/// An isotropic launch fan: a source sheds its mass as `directions` equal parcels launched at evenly spaced
/// angles, the maximum-entropy unbiased direction prior. Each parcel carries the same launch state (an event
/// datum), and the blanket that results is NOT authored: it EMERGES from the terrain shaping each parcel's
/// runout, so a flat ground gives a symmetric spread and a slope funnels the deposit downhill, both falling
/// out of the physics rather than a coded spread shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaunchFan {
    /// Each parcel's launch specific energy (its range budget).
    pub energy: Fixed,
    /// The launch speed, the magnitude of the directional momentum bias applied to each `(cos, sin)`
    /// direction: a large speed is a ballistic throw, a small speed a gravity-directed release.
    pub speed: Fixed,
    /// The number of evenly spaced launch directions. A RESOLUTION and determinism bound, not a physical
    /// value: reserved-with-basis as the fewest directions at which the blanket's shape stops changing
    /// beyond the grid resolution (a resolution-versus-cost bound), never fabricated.
    pub directions: u32,
}

/// Fire an isotropic launch fan from `source` and aggregate where the parcels come to rest into a canonical
/// destination distribution the conservation operator ([`crate::redistribute`]) credits. The directions are
/// the evenly spaced angles `i * 2*pi / directions` from a fixed origin (deterministic fixed-point CORDIC,
/// never an RNG), each parcel runs through [`runout_with_forces`], and the rest cells are gathered in cell
/// order into `(cell, count)` weights, so the whole fan is a pure function of its inputs, worker-invariant.
/// A parcel that rests at the source contributes there (a fraction of the mass stays), which the operator
/// nets correctly. Returns an empty distribution for a zero-direction fan.
///
/// The honest boundary (the integrator is SURFACE-CONTACT, friction-dissipated along the ground): this fan
/// is correct for the surface-flowing mass movements (debris flows, lahars, pyroclastic and turbidity
/// currents, lava runout) and APPROXIMATE for a true impact ejecta blanket, whose parcels launch on a
/// VERTICAL ballistic arc. The vertical-arc regime (airborne drag in flight versus contact friction on the
/// ground) is a deeper force-term extension through the same open [`BodyForce`] slot, deferred behind this
/// boundary, not built here.
///
/// A second honest limit, the ANGULAR resolution: the integrator takes greedy single-cell 4-connected
/// steps, so on flat ground a diagonal launch snaps to its nearest axis (the deposit is SYMMETRIC but
/// effectively four-axis, and directions beyond the four re-weight the same axis endpoints rather than
/// filling in angles); the terrain breaks this on real relief, and true angular resolution is the deferred
/// momentum-vector (sub-cell velocity) integrator, a follow-on refinement of the same law.
pub fn launch_fan(
    width: usize,
    height: usize,
    elevation: &[Fixed],
    source: usize,
    fan: LaunchFan,
    forces: RunoutForces,
    extra: &[BodyForce],
) -> Vec<crate::redistribute::Weighted> {
    use crate::redistribute::Weighted;
    use std::collections::BTreeMap;
    if fan.directions == 0 {
        return Vec::new();
    }
    // 2*pi as the full turn the directions evenly divide; HALF_PI times four keeps it exact-in-Fixed.
    let full_turn = Fixed::HALF_PI.mul(Fixed::from_int(4));
    let count = Fixed::from_int(fan.directions as i32);
    // An ordered map so the aggregation is canonical (cell order), never a hash-iteration leak.
    let mut rests: BTreeMap<usize, u64> = BTreeMap::new();
    for i in 0..fan.directions {
        // The evenly spaced launch angle and its unit direction (cos, sin), scaled by the launch speed.
        let theta = full_turn.mul(Fixed::from_int(i as i32)).div(count);
        let (sin, cos) = theta.sin_cos();
        let heading = (fan.speed.mul(cos), fan.speed.mul(sin));
        let parcel = Parcel {
            energy: fan.energy,
            heading,
        };
        let path = runout_with_forces(width, height, elevation, source, parcel, forces, extra);
        *rests.entry(path.rest).or_insert(0) += 1;
    }
    rests
        .into_iter()
        .map(|(dest, weight)| Weighted { dest, weight })
        .collect()
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

    // --- Slice 3: the ballistic launch-dominated case, the open body-force slot, the regime ratio ---

    #[test]
    fn a_ballistic_launch_flies_its_heading_across_flat_ground() {
        // Flat ground (gravity returns nothing on any step), so only the launch momentum drives the parcel:
        // a strong east heading carries it east, friction spending the budget over the range e0/(mu*g*cell).
        // The gravity-driven regime (a static parcel) would not move here; the ballistic regime does, the
        // same law under a different parcel state.
        let e = vec![Fixed::ZERO; 10];
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 100);
        // friction cost per step = 1/2; a budget of 2 carries 4 steps east.
        let path = runout(
            10,
            1,
            &e,
            0,
            Parcel {
                energy: Fixed::from_int(2),
                heading: (Fixed::from_int(10), Fixed::ZERO),
            },
            f,
        );
        assert_eq!(
            path.rest, 4,
            "the ballistic parcel flies its heading the friction-limited range"
        );
        // A static parcel on the same flat ground does not move (no launch energy, gravity flat).
        let stuck = runout(
            10,
            1,
            &e,
            0,
            Parcel {
                energy: Fixed::ZERO,
                heading: still(),
            },
            f,
        );
        assert_eq!(
            stuck.rest, 0,
            "with no launch energy the flat-ground parcel stays put"
        );
    }

    #[test]
    fn a_larger_ballistic_budget_flies_farther() {
        let e = vec![Fixed::ZERO; 20];
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 100);
        let head = (Fixed::from_int(10), Fixed::ZERO);
        let near = runout(
            20,
            1,
            &e,
            0,
            Parcel {
                energy: Fixed::from_int(2),
                heading: head,
            },
            f,
        );
        let far = runout(
            20,
            1,
            &e,
            0,
            Parcel {
                energy: Fixed::from_int(4),
                heading: head,
            },
            f,
        );
        assert!(
            far.rest > near.rest,
            "a larger launch budget reaches farther ({} vs {})",
            far.rest,
            near.rest
        );
    }

    #[test]
    fn an_alien_body_force_drives_transport_as_a_data_row() {
        // Flat terrain, no launch energy, no momentum: gravity alone moves nothing. A world-declared mana
        // field falling toward the east adds an open body force whose gradient drives the parcel east, all
        // the way to the far edge, with no code path for "mana": the alien force is a data row keyed on its
        // own field (admit-the-alien). The SAME parcel under gravity alone stays put.
        let width = 8;
        let flat = vec![Fixed::ZERO; width];
        // A mana potential that falls by one per cell toward the east, so moving east is a mana drop.
        let mana: Vec<Fixed> = (0..width)
            .map(|x| Fixed::from_int((width - 1 - x) as i32))
            .collect();
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 100);
        let parcel = Parcel {
            energy: Fixed::ZERO,
            heading: still(),
        };
        // Gravity alone: the flat-ground static parcel does not move.
        let gravity_only = runout(width, 1, &flat, 0, parcel, f);
        assert_eq!(
            gravity_only.rest, 0,
            "gravity alone leaves the flat-ground parcel still"
        );
        // With the mana body force (coefficient 1, above the friction cost 1/2), it flows east to the edge.
        let mana_force = [BodyForce {
            coefficient: Fixed::ONE,
            potential: &mana,
        }];
        let driven = runout_with_forces(width, 1, &flat, 0, parcel, f, &mana_force);
        assert_eq!(
            driven.rest,
            width - 1,
            "the mana gradient drives the parcel to the far edge as a data row"
        );
    }

    // The four axis reaches of a fan's deposit around a source, for the symmetry and terrain-broken tests.
    fn axis_reaches(
        dist: &[crate::redistribute::Weighted],
        w: usize,
        sx: usize,
        sy: usize,
    ) -> (u64, u64, u64, u64) {
        let (mut east, mut west, mut north, mut south) = (0u64, 0u64, 0u64, 0u64);
        for wt in dist {
            let x = wt.dest % w;
            let y = wt.dest / w;
            if x > sx {
                east += wt.weight;
            }
            if x < sx {
                west += wt.weight;
            }
            if y > sy {
                south += wt.weight;
            }
            if y < sy {
                north += wt.weight;
            }
        }
        (east, west, north, south)
    }

    #[test]
    fn an_isotropic_fan_spreads_symmetrically_on_flat_ground() {
        // Flat ground, source at the centre, an 8-direction ballistic fan: the deposit is balanced
        // east-west and north-south (the maximum-entropy prior with no terrain to break it), spreads to
        // several cells (a blanket, not a point), and conserves the parcel count (total weight = directions).
        let (w, h) = (41usize, 41usize);
        let flat = vec![Fixed::ZERO; w * h];
        let (sx, sy) = (20usize, 20usize);
        let source = sy * w + sx;
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 200);
        let fan = LaunchFan {
            energy: Fixed::from_int(4),
            speed: Fixed::from_int(8),
            directions: 8,
        };
        let dist = launch_fan(w, h, &flat, source, fan, f, &[]);
        let total: u64 = dist.iter().map(|wt| wt.weight).sum();
        assert_eq!(total, 8, "every launched parcel is accounted for");
        assert!(
            dist.len() > 1,
            "the fan spreads into a blanket, not a point"
        );
        let (east, west, north, south) = axis_reaches(&dist, w, sx, sy);
        assert_eq!(east, west, "flat-ground symmetry east-west");
        assert_eq!(north, south, "flat-ground symmetry north-south");
    }

    #[test]
    fn a_fan_on_a_slope_funnels_the_deposit_downhill() {
        // A ramp falling toward the west (lower x) breaks the symmetry: downhill (west) parcels gain energy
        // and reach far, uphill (east) parcels lose it and stop near, so the deposit is west-heavy. The
        // asymmetry EMERGES from the terrain, not an authored bias.
        let (w, h) = (41usize, 21usize);
        let e = ramp(w, h, Fixed::from_ratio(3, 4));
        let (sx, sy) = (20usize, 10usize);
        let source = sy * w + sx;
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 400);
        let fan = LaunchFan {
            energy: Fixed::from_int(2),
            speed: Fixed::from_int(4),
            directions: 8,
        };
        let dist = launch_fan(w, h, &e, source, fan, f, &[]);
        let (east, west, _north, _south) = axis_reaches(&dist, w, sx, sy);
        assert!(
            west > east,
            "the deposit funnels downhill (west {west} > east {east})"
        );
    }

    #[test]
    fn the_fan_is_deterministic() {
        let (w, h) = (21usize, 21usize);
        let e = ramp(w, h, Fixed::from_ratio(2, 5));
        let source = 10 * w + 10;
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 3), Fixed::ONE, 200);
        let fan = LaunchFan {
            energy: Fixed::from_int(3),
            speed: Fixed::from_int(5),
            directions: 12,
        };
        let a = launch_fan(w, h, &e, source, fan, f, &[]);
        let b = launch_fan(w, h, &e, source, fan, f, &[]);
        assert_eq!(a, b, "the same fan reproduces the same distribution");
    }

    #[test]
    fn the_fan_feeds_the_operator_a_conservative_blanket() {
        // The whole surface transport: the fan places the blanket, the operator moves the source's mass into
        // it conservatively, so the delta field sums to zero and the source is debited its whole mass.
        let (w, h) = (31usize, 31usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = 15 * w + 15;
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 200);
        let fan = LaunchFan {
            energy: Fixed::from_int(3),
            speed: Fixed::from_int(6),
            directions: 8,
        };
        let dist = launch_fan(w, h, &flat, source, fan, f, &[]);
        let mass = 4000;
        let delta = redistribute(
            w * h,
            &[Redistribution {
                source,
                mass,
                dests: dist,
            }],
        )
        .unwrap();
        assert_eq!(delta.iter().sum::<i64>(), 0, "the blanket conserves mass");
        assert_eq!(delta[source], -mass, "the source shed its whole mass");
    }

    #[test]
    fn a_zero_direction_fan_is_empty() {
        let (w, h) = (5usize, 5usize);
        let flat = vec![Fixed::ZERO; w * h];
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 50);
        let fan = LaunchFan {
            energy: Fixed::from_int(2),
            speed: Fixed::from_int(3),
            directions: 0,
        };
        assert!(launch_fan(w, h, &flat, 12, fan, f, &[]).is_empty());
    }

    #[test]
    fn the_regime_ratio_reads_the_launch_energy_over_the_dissipation() {
        // The dimensionless runout-scale ratio is e0 / (mu*g*cell): with mu*g*cell = 1/2, a budget of 2
        // reads a ratio of 4 (four flat-ground steps of reach). It is a derived readout, never a category.
        let f = forces(Fixed::ONE, Fixed::from_ratio(1, 2), Fixed::ONE, 100);
        assert_eq!(regime_ratio(Fixed::from_int(2), f), Fixed::from_int(4));
        // A frictionless or zero-gravity degenerate world has non-positive dissipation, so the ratio reads
        // zero (the unbounded-scale signal), never a fabricated number.
        let frictionless = forces(Fixed::ONE, Fixed::ZERO, Fixed::ONE, 100);
        assert_eq!(regime_ratio(Fixed::from_int(5), frictionless), Fixed::ZERO);
    }
}
