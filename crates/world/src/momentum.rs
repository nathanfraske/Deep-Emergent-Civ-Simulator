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

//! The unified momentum integrator: ONE law that carries a parcel's velocity vector and lets the
//! surface-contact and airborne regimes EMERGE from the parcel's own state and the terrain, rather than a
//! driver naming which of two kernels to run (gate-ruled, the momentum-vector unification). The surface
//! integrator ([`crate::runout`]) walks a parcel ALONG the ground and the ballistic integrator
//! ([`crate::ballistic`]) flies it OVER the ground on a fixed arc; each is a regime of the one law here, and
//! WHICH regime a parcel is in is read from geometry each step (is it resting on the ground or above it),
//! never selected by the caller. A slow release skims and a fast throw flies, from the same law under
//! different launch data, and the transition between them (a parcel that runs to a break and launches, or one
//! that arcs and lands) falls out of the physics.
//!
//! The one law, in Newtonian terms. A parcel feels gravity always, the open body forces always (the alien
//! slot, [`crate::runout::BodyForce`]), and Coulomb friction only WHILE IN CONTACT, because friction is
//! `mu` times the normal force and the normal force is zero when the parcel is not pressing on the ground.
//! So the regime is not two mechanisms; it is one force law under an emergent normal force: airborne the
//! normal force is zero and the parcel flies the exact projectile arc (the [`crate::ballistic`] parabola),
//! in contact the normal force supports the parcel and friction dissipates along the surface (the
//! [`crate::runout`] specific-energy law). The parcel decides, each step, which it is: the arc it would fly
//! from its current velocity is compared to the next terrain, and if the ground has risen to meet it the
//! parcel is in contact, and if the ground has fallen away below the arc the parcel is airborne.
//!
//! The reductions are exact (to fixed-point rounding), which is how the unification is proven rather than
//! asserted. A launch that stays airborne reproduces [`crate::ballistic::ballistic_landing`] cell for cell
//! (the same anchored parabola sampled at the same per-cell steps). A launch that stays in contact reproduces
//! [`crate::runout::runout`] along the launch azimuth (the same `de = g*drop - mu*g*cell` budget). The tests
//! measure the first against the ballistic oracle directly, the gate's requirement that the exactness trade
//! be measured, not assumed.
//!
//! The honest boundaries, stated not hidden. The march is along a FIXED azimuth (the launch heading), so the
//! contact regime follows the terrain along that line rather than navigating freely downhill across the grid
//! (the surface integrator's 2-D steepest-descent turn); the isotropic fan over azimuths, the next slice,
//! recovers the blanket, and free 2-D contact navigation is a later refinement. The march step is one cell
//! (`cell_size`), the resolution of the `z = 0` per-cell heightfield (A's substrate boundary): a sub-cell
//! step buys nothing against a per-cell terrain until the heightfield is continuous, so it is reserved behind
//! that lift with its basis rather than added here where it is inert (a measured finding, not an assumption).
//! Airborne body forces (the floor `drag_force` on the arc) are deferred through the same slot, a refinement;
//! the slot acts in contact here, parity with the surface integrator, so an alien contact force is a data row.
//! A parcel that lands from flight deposits at the touch-down (the ballistic reduction); the post-landing
//! slide (a landed parcel continuing to run out) is a restitution refinement deferred to a later slice.
//!
//! Determinism (Principle 3, Principle 10): fixed-point throughout (angles through the integer CORDIC, the
//! velocity magnitude through the exact [`Fixed::sqrt`], no float), a bounded march (at most `step_cap`
//! steps, never an unbounded until-rest spin), and the cell map a deterministic floor, so the whole path is a
//! pure function of the launch, the terrain, and the forces, worker-invariant.

use crate::runout::BodyForce;
use civsim_core::Fixed;

/// Saturating fixed-point add on the raw bits (determinism over an adversarial input; the physical inputs
/// never reach the rails).
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

/// A parcel's launch, the physical initial conditions an event supplies as DATA (never a named kernel): the
/// launch speed, the elevation angle above the horizontal, and the azimuth. The vertical component of the
/// launch (`speed * sin(elevation_angle)`) is what decides whether the parcel leaves the ground, so the
/// regime is a reading of this launch datum and the terrain, not a category the caller picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MomentumLaunch {
    /// The launch speed `v` (the velocity-vector magnitude). Zero is no launch.
    pub speed: Fixed,
    /// The elevation angle `alpha` above the horizontal, in radians. Zero is a grazing (horizontal) launch,
    /// which stays in contact on flat or gentle ground and flies only where the ground falls away faster than
    /// free fall; at or above a right angle the horizontal speed is non-positive and the launch is degenerate.
    pub elevation_angle: Fixed,
    /// The azimuth `phi` (compass heading) in radians; the ground march direction is `(cos, sin)`.
    pub azimuth: Fixed,
}

/// The world force parameters the law reads, each a floor axis or a per-world datum (never authored here):
/// gravity, the material's Coulomb friction coefficient, and the cell edge length. The step cap is a
/// determinism and performance bound (the march terminates in at most this many steps), not a physical value.
#[derive(Clone, Copy, Debug)]
pub struct MomentumForces {
    /// Gravity `g` (`mech.gravitational_acceleration`), a PARAMETER; a world's derived `g = G*M/R^2` flows in
    /// with no code change.
    pub gravity: Fixed,
    /// The material's kinetic Coulomb friction coefficient `mu` (`mech.kinetic_friction`); its arctangent is
    /// the emergent angle of repose in the contact regime, exactly as in the surface integrator.
    pub friction: Fixed,
    /// The cell edge length in world units (a per-world spatial datum), the horizontal distance of one step.
    /// This is the march step: the resolution of the per-cell heightfield, so a sub-cell step is reserved
    /// behind a continuous terrain rather than a value here.
    pub cell_size: Fixed,
    /// The maximum number of steps the march may take, a determinism and performance bound (the path is a pure
    /// function up to this cap), never a physical length.
    pub step_cap: u32,
}

/// The path a parcel takes and where it deposits under the unified law: the cells it passed through in order
/// (the source first, consecutive duplicates dropped), the cell it came to rest in, and whether it rested by
/// FLYING to a landing (an airborne touch-down) or by RUNNING OUT in contact (friction spent its budget). The
/// rest cell is the single-parcel deposit site the redistribution operator ([`crate::redistribute`]) credits;
/// the isotropic fan that spreads a source's mass into a blanket is the next slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MomentumPath {
    /// The cells the parcel occupied, in travel order, starting with the source.
    pub track: Vec<usize>,
    /// The cell the parcel came to rest in (the deposit site).
    pub rest: usize,
    /// True if the parcel rested by an airborne touch-down (it flew an arc and landed), false if it ran out in
    /// contact (or never moved). The regime is emergent, so this reports which one the physics produced.
    pub landed_from_flight: bool,
}

/// Integrate a parcel's motion under the unified law from `source` over a `width` by `height` fixed-point
/// elevation grid, under gravity and Coulomb friction alone (the common case). A convenience wrapper over
/// [`momentum_integrate_with_forces`] with no extra body forces.
pub fn momentum_integrate(
    width: usize,
    height: usize,
    elevation: &[Fixed],
    source: usize,
    launch: MomentumLaunch,
    forces: MomentumForces,
) -> MomentumPath {
    momentum_integrate_with_forces(width, height, elevation, source, launch, forces, &[])
}

/// Integrate a parcel's motion under the unified law from `source`, returning the path it takes and the cell
/// it deposits in. The regime (airborne flight versus surface contact) emerges each step from the parcel's arc
/// against the terrain: while the ground stays below the arc the parcel flies the exact projectile parabola
/// (gravity only), and where the ground rises to meet the arc the parcel is in contact and the surface law
/// applies (gravity along the slope, Coulomb friction, and the `extra` open body forces summed along their
/// own potential gradients, so an alien contact force is a data row). A parcel that launches airborne and
/// touches down deposits at the landing; a parcel in contact runs out and deposits where its budget is spent;
/// a contact parcel that reaches ground falling away faster than free fall launches, all emergent. Panics if
/// `elevation.len()` is not `width * height`, if `source` is out of range, or if an `extra` potential is not
/// the grid length (construction invariants). A degenerate launch (non-positive speed, or an elevation angle
/// at or beyond the vertical so the horizontal speed is non-positive) rests at the source.
pub fn momentum_integrate_with_forces(
    width: usize,
    height: usize,
    elevation: &[Fixed],
    source: usize,
    launch: MomentumLaunch,
    forces: MomentumForces,
    extra: &[BodyForce],
) -> MomentumPath {
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

    let at_source = || MomentumPath {
        track: vec![source],
        rest: source,
        landed_from_flight: false,
    };

    let (sin_a, cos_a) = launch.elevation_angle.sin_cos();
    // The launch velocity vector, split into its horizontal magnitude (constant while airborne, dissipated by
    // friction in contact) and its vertical component (which decides whether the parcel leaves the ground).
    let vh0 = smul(launch.speed, cos_a);
    let vz0 = smul(launch.speed, sin_a);
    // A non-positive speed, or a launch at or beyond the vertical (cos_a <= 0), has no forward horizontal
    // motion, so there is no path: the parcel falls back to the source.
    if launch.speed <= Fixed::ZERO || vh0 <= Fixed::ZERO {
        return at_source();
    }
    let (sin_p, cos_p) = launch.azimuth.sin_cos();

    let half = Fixed::from_ratio(1, 2);
    let two = Fixed::from_int(2);
    let g = forces.gravity;
    let cell = forces.cell_size;
    // The friction cost of one step, mu * g * cell (the Coulomb dissipation per unit mass over the horizontal
    // step), the same everywhere, exactly the surface integrator's fixed step cost.
    let friction_cost = smul(smul(forces.friction, g), cell);

    let sx = (source % width) as i32;
    let sy = (source / width) as i32;

    // The parcel starts in contact resting on the source; whether it leaves the ground is read each step from
    // the arc its velocity would fly against the terrain, so a steep launch goes airborne on the first step and
    // a grazing one skims. The airborne anchor is the launch point of the current arc, set when it launches.
    let mut airborne = false;
    let mut anchor_x = Fixed::ZERO; // horizontal distance travelled at the current arc's launch
    let mut anchor_h = elevation[source]; // launch height of the current arc
    let mut anchor_vh = vh0; // horizontal speed of the current arc (constant while airborne)
    let mut anchor_vz = vz0; // vertical velocity at the current arc's launch

    // The contact specific-energy budget (horizontal kinetic energy per unit mass, e = v_h^2 / 2) and the
    // parcel's vertical velocity (the launch value, then the surface-following rate as it skims). The vertical
    // velocity feeds the launch test, so a parcel already descending does not launch unless the ground drops
    // faster still.
    let mut energy = smul(half, smul(vh0, vh0));
    let mut vz = vz0;

    let mut track = vec![source];
    let mut cur = source;
    let mut cur_elev = elevation[source];
    let mut landed_from_flight = false;

    for i in 1..=forces.step_cap {
        let step = Fixed::from_int(i as i32);
        // The ground position along the azimuth (cell units), floored to a cell, exactly the ballistic march.
        let px = sadd(Fixed::from_int(sx), smul(step, cos_p));
        let py = sadd(Fixed::from_int(sy), smul(step, sin_p));
        let cx = px.to_int();
        let cy = py.to_int();
        if cx < 0 || cx >= width as i32 || cy < 0 || cy >= height as i32 {
            break; // the path left the grid; the parcel rests at the last in-grid cell
        }
        let nxt = cy as usize * width + cx as usize;
        let nxt_elev = elevation[nxt];
        let x = smul(step, cell); // the horizontal distance of this step's ground position from the source

        if airborne {
            // The exact arc height at x: h = anchor_h + anchor_vz * T - g/2 * T^2, T = (x - anchor_x)/anchor_vh.
            let t = ssub(x, anchor_x)
                .checked_div(anchor_vh)
                .unwrap_or(Fixed::MAX);
            let arc_h = ssub(
                sadd(anchor_h, smul(anchor_vz, t)),
                smul(smul(half, g), smul(t, t)),
            );
            if *track.last().expect("track is seeded with the source") != nxt {
                track.push(nxt);
            }
            cur = nxt;
            if arc_h > nxt_elev {
                cur_elev = nxt_elev; // still flying: the ground is below the arc
                continue;
            }
            landed_from_flight = true; // the ground rose to meet the arc: touch down and deposit
            break;
        }

        // In contact. First the launch test: from the current cell, moving horizontally at v_h = sqrt(2 e) with
        // the current vertical velocity, the arc reaches height cur_elev + vz*dt - g/2*dt^2 at the next cell
        // (dt = cell / v_h). If that clears the next terrain the ground has fallen away and the parcel launches.
        let vh = smul(two, energy).max(Fixed::ZERO).sqrt();
        if vh > Fixed::ZERO {
            let dt = cell.checked_div(vh).unwrap_or(Fixed::MAX);
            let arc_h = ssub(
                sadd(cur_elev, smul(vz, dt)),
                smul(smul(half, g), smul(dt, dt)),
            );
            if arc_h > nxt_elev {
                // Launches off the break: anchor a fresh arc at the current cell (it leaves with its horizontal
                // speed and current vertical velocity), then fly this step to the next cell.
                airborne = true;
                anchor_x = ssub(x, cell); // the launch is at the current cell's ground position
                anchor_h = cur_elev;
                anchor_vh = vh;
                anchor_vz = vz;
                if *track.last().expect("track is seeded with the source") != nxt {
                    track.push(nxt);
                }
                // The arc clears the next terrain (that is why it launched), so it flies this step; whether it
                // lands is decided at the following step by the airborne branch above.
                cur = nxt;
                cur_elev = nxt_elev;
                continue;
            }
        }

        // Stays in contact. The step's specific-energy change: gravity returns g * drop over the elevation
        // drop, the open body forces return their summed potential drop, and friction always dissipates its
        // fixed cost. If the budget cannot cover it, the parcel rests here.
        let drop = ssub(cur_elev, nxt_elev);
        let force_gain = sadd(smul(g, drop), body_gain(extra, cur, nxt));
        let next_energy = ssub(sadd(energy, force_gain), friction_cost);
        if next_energy < Fixed::ZERO {
            break;
        }
        energy = next_energy;
        // The vertical velocity moving along the surface to the next cell: the elevation change over the step
        // time (dt = cell / v_h_new), so vz = (nxt_elev - cur_elev) * v_h_new / cell. This feeds the next
        // launch test, so a steepening surface builds a downward vz until the ground outruns the parcel.
        let vh_new = smul(two, energy).max(Fixed::ZERO).sqrt();
        vz = smul(ssub(nxt_elev, cur_elev), vh_new)
            .checked_div(cell)
            .unwrap_or(Fixed::ZERO);
        cur = nxt;
        cur_elev = nxt_elev;
        if *track.last().expect("track is seeded with the source") != nxt {
            track.push(nxt);
        }
    }

    MomentumPath {
        track,
        rest: cur,
        landed_from_flight,
    }
}

/// The energy an extra body force set returns for a step from `cur` to `nbr`: the sum over the forces of the
/// coefficient times the drop in that force's potential, exactly the surface integrator's body-force sum, so
/// an alien contact force stays a data row. The caller validates every potential to the grid length, so the
/// bounds check here is a safe fallback that never fires in practice.
fn body_gain(extra: &[BodyForce], cur: usize, nbr: usize) -> Fixed {
    let mut gain = Fixed::ZERO;
    for f in extra {
        if let (Some(&pc), Some(&pn)) = (f.potential.get(cur), f.potential.get(nbr)) {
            gain = sadd(gain, smul(f.coefficient, ssub(pc, pn)));
        }
    }
    gain
}

/// An isotropic fan under the unified law: a source sheds its mass as `azimuths` equal parcels launched at the
/// same speed and elevation angle in evenly spaced azimuths (the maximum-entropy, unbiased direction prior),
/// each integrated through the one law. Because the law is unified, ONE fan covers both regimes: a steep
/// elevation angle flies a ballistic blanket, a grazing one runs out a surface blanket, and a mixed relief
/// does both along different azimuths, from the same fan. The blanket is NOT authored; it EMERGES from the
/// terrain shaping each parcel's path, so flat ground gives a symmetric ring and real relief breaks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MomentumFan {
    /// The launch speed, the same for every azimuth (isotropic launch).
    pub speed: Fixed,
    /// The elevation angle above the horizontal, the same for every azimuth. It selects the regime the fan
    /// runs in without naming it: steep flies, grazing skims, and the terrain decides the rest.
    pub elevation_angle: Fixed,
    /// The number of evenly spaced launch azimuths. A RESOLUTION and determinism bound, not a physical value:
    /// reserved-with-basis as the fewest azimuths at which the blanket's shape stops changing beyond the grid
    /// resolution (a resolution-versus-cost bound), never fabricated.
    pub azimuths: u32,
}

/// Fire an isotropic fan from `source` under gravity and Coulomb friction alone (the common case). A
/// convenience wrapper over [`momentum_fan_with_forces`] with no extra body forces.
pub fn momentum_fan(
    width: usize,
    height: usize,
    elevation: &[Fixed],
    source: usize,
    fan: MomentumFan,
    forces: MomentumForces,
) -> Vec<crate::redistribute::Weighted> {
    momentum_fan_with_forces(width, height, elevation, source, fan, forces, &[])
}

/// Fire an isotropic fan from `source` under the unified law and aggregate where the parcels come to rest into
/// a canonical destination distribution the redistribution operator ([`crate::redistribute`]) credits. The
/// azimuths are the evenly spaced angles `i * 2*pi / azimuths` from a fixed origin (deterministic fixed-point
/// CORDIC, never an RNG), each parcel integrated through [`momentum_integrate_with_forces`], and the rest cells
/// gathered in cell order into `(cell, count)` weights, so the whole fan is a pure function of its inputs,
/// worker-invariant. Returns an empty distribution for a zero-azimuth fan.
///
/// The angular resolution is the CORDIC's in BOTH regimes, the unification's bonus: the integrator holds a
/// sub-cell heading along the continuous azimuth and floors it to a cell (like the ballistic march), so a
/// contact-regime fan resolves distinct azimuths rather than collapsing to the four axes the surface
/// integrator's greedy 4-connected steps give ([`crate::runout::launch_fan`]'s stated limit). Gravity is the
/// parameter [`MomentumForces::gravity`], so a world's own or derived gravity sets the blanket radius with no
/// code change.
pub fn momentum_fan_with_forces(
    width: usize,
    height: usize,
    elevation: &[Fixed],
    source: usize,
    fan: MomentumFan,
    forces: MomentumForces,
    extra: &[BodyForce],
) -> Vec<crate::redistribute::Weighted> {
    use crate::redistribute::Weighted;
    use std::collections::BTreeMap;
    if fan.azimuths == 0 {
        return Vec::new();
    }
    // 2*pi as the full turn the azimuths evenly divide; HALF_PI times four keeps it exact-in-Fixed.
    let full_turn = Fixed::HALF_PI.mul(Fixed::from_int(4));
    let count = Fixed::from_int(fan.azimuths as i32);
    // An ordered map so the aggregation is canonical (cell order), never a hash-iteration leak.
    let mut rests: BTreeMap<usize, u64> = BTreeMap::new();
    for i in 0..fan.azimuths {
        let phi = full_turn.mul(Fixed::from_int(i as i32)).div(count);
        let launch = MomentumLaunch {
            speed: fan.speed,
            elevation_angle: fan.elevation_angle,
            azimuth: phi,
        };
        let path =
            momentum_integrate_with_forces(width, height, elevation, source, launch, forces, extra);
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
    use crate::ballistic::{ballistic_landing, BallisticForces, BallisticLaunch};
    use crate::runout::{runout, Parcel, RunoutForces};

    // 45 degrees, where the flat-ground ballistic range is the maximum v^2/g.
    fn deg45() -> Fixed {
        Fixed::HALF_PI.div(Fixed::from_int(2))
    }

    fn m_forces(gravity: Fixed, friction: Fixed, cell: Fixed, cap: u32) -> MomentumForces {
        MomentumForces {
            gravity,
            friction,
            cell_size: cell,
            step_cap: cap,
        }
    }

    fn m_launch(speed: Fixed, alpha: Fixed, azimuth: Fixed) -> MomentumLaunch {
        MomentumLaunch {
            speed,
            elevation_angle: alpha,
            azimuth,
        }
    }

    // A ramp that falls toward the east (higher x is lower) by `grade` per cell.
    fn east_downhill(width: usize, height: usize, grade: Fixed) -> Vec<Fixed> {
        let mut e = vec![Fixed::ZERO; width * height];
        for y in 0..height {
            for x in 0..width {
                e[y * width + x] = smul(grade, Fixed::from_int((width - 1 - x) as i32));
            }
        }
        e
    }

    #[test]
    fn the_airborne_regime_reproduces_the_ballistic_oracle_on_flat_ground() {
        // A steep upward launch flies an arc; the unified integrator uses the same anchored parabola and the
        // same per-cell march as the ballistic oracle, so it must land in the SAME cell. This is the gate's
        // required measurement of the exactness trade: on flat ground it is zero.
        let (w, h) = (40usize, 3usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = w + 5;
        let path = momentum_integrate(
            w,
            h,
            &flat,
            source,
            m_launch(Fixed::from_int(10), deg45(), Fixed::ZERO),
            m_forces(
                Fixed::from_int(10),
                Fixed::from_ratio(1, 2),
                Fixed::ONE,
                200,
            ),
        );
        let oracle = ballistic_landing(
            w,
            h,
            &flat,
            source,
            BallisticLaunch {
                speed: Fixed::from_int(10),
                elevation_angle: deg45(),
                azimuth: Fixed::ZERO,
            },
            BallisticForces {
                gravity: Fixed::from_int(10),
                cell_size: Fixed::ONE,
                step_cap: 200,
            },
        );
        assert!(path.landed_from_flight, "a steep launch flies and lands");
        assert_eq!(
            path.rest, oracle.landing,
            "the airborne regime lands in the ballistic oracle's cell exactly (unified {} vs oracle {})",
            path.rest, oracle.landing
        );
    }

    #[test]
    fn the_airborne_regime_matches_the_oracle_over_a_valley_and_a_slope() {
        // Two terrains the closed form and the unified march must agree on: a deep valley the arc clears, and a
        // downhill ramp that extends the range. The exactness trade is measured as the cell difference, zero
        // (the same parabola, the same per-cell sampling).
        let (w, h) = (30usize, 3usize);
        let f_m = m_forces(
            Fixed::from_int(10),
            Fixed::from_ratio(1, 2),
            Fixed::ONE,
            200,
        );
        let f_b = BallisticForces {
            gravity: Fixed::from_int(10),
            cell_size: Fixed::ONE,
            step_cap: 200,
        };
        let launch_m = m_launch(Fixed::from_int(10), deg45(), Fixed::ZERO);
        let launch_b = BallisticLaunch {
            speed: Fixed::from_int(10),
            elevation_angle: deg45(),
            azimuth: Fixed::ZERO,
        };
        // A deep valley between the source and the far rim.
        let mut valley = vec![Fixed::ZERO; w * h];
        for y in 0..h {
            for x in 10..20 {
                valley[y * w + x] = Fixed::from_int(-50);
            }
        }
        let source = w + 5;
        let p = momentum_integrate(w, h, &valley, source, launch_m, f_m);
        let o = ballistic_landing(w, h, &valley, source, launch_b, f_b);
        assert_eq!(
            p.rest, o.landing,
            "the arc clears the valley to the oracle's cell"
        );
        // A downhill ramp.
        let ramp = east_downhill(w, h, Fixed::from_ratio(1, 5));
        let src2 = w + 15;
        let p2 = momentum_integrate(w, h, &ramp, src2, launch_m, f_m);
        let o2 = ballistic_landing(w, h, &ramp, src2, launch_b, f_b);
        assert_eq!(
            p2.rest, o2.landing,
            "the arc over a slope lands at the oracle's cell"
        );
    }

    #[test]
    fn the_contact_regime_reproduces_the_surface_runout() {
        // A slow grazing release on a gentle ramp aligned with the launch azimuth: it never leaves the ground
        // (free fall over a cell exceeds the gentle terrain drop), so it runs out under the surface law and
        // must rest in the SAME cell as the surface integrator, which also marches straight down the aligned
        // ramp. The momentum speed maps to the runout energy budget by e0 = v^2 / 2 (a grazing launch).
        let (w, h) = (40usize, 3usize);
        let grade = Fixed::from_ratio(1, 20); // gentle: 0.05 drop per cell
        let ramp = east_downhill(w, h, grade);
        let source = w + 2; // near the high (west) end, launched east (downhill)
        let g = Fixed::from_int(10);
        // mu = 0.12 so the per-step budget change (g*grade - mu*g*cell = -0.7) does not evenly divide the
        // launch budget e0 = 2, keeping the rest cell off an energy-zero boundary where the sub-ULP CORDIC
        // representation of the grazing angle (cos 0 is 1 + 1.6e-9) could flip it: the reduction is then an
        // exact cell match, not a within-a-cell one.
        let mu = Fixed::from_ratio(3, 25);
        let speed = Fixed::from_int(2); // slow: e0 = 2, stays in contact
        let path = momentum_integrate(
            w,
            h,
            &ramp,
            source,
            m_launch(speed, Fixed::ZERO, Fixed::ZERO), // grazing, east
            m_forces(g, mu, Fixed::ONE, 200),
        );
        // The surface integrator with the matching budget e0 = v^2 / 2 and an eastward heading bias.
        let e0 = smul(Fixed::from_ratio(1, 2), smul(speed, speed));
        let run = runout(
            w,
            h,
            &ramp,
            source,
            Parcel {
                energy: e0,
                heading: (Fixed::ONE, Fixed::ZERO),
            },
            RunoutForces {
                gravity: g,
                friction: mu,
                cell_size: Fixed::ONE,
                step_cap: 200,
            },
        );
        assert!(
            !path.landed_from_flight,
            "the grazing release stays in contact"
        );
        assert_eq!(
            path.rest, run.rest,
            "the contact regime rests where the surface integrator does (unified {} vs runout {})",
            path.rest, run.rest
        );
    }

    #[test]
    fn a_break_launches_a_contact_parcel_over_it() {
        // The regime emerges, not chosen: a grazing parcel skims flat ground, then the ground drops sharply
        // away and the parcel LAUNCHES (goes airborne) with no kernel switch, clearing the break and landing
        // on the low ground beyond rather than descending the face as a pure surface parcel would.
        let (w, h) = (30usize, 3usize);
        let mut terr = vec![Fixed::ZERO; w * h];
        // A step down: cells at x >= 8 sit 5 below the flat top (a sharp break the parcel flies off).
        for y in 0..h {
            for x in 8..w {
                terr[y * w + x] = Fixed::from_int(-5);
            }
        }
        let source = w + 2;
        let path = momentum_integrate(
            w,
            h,
            &terr,
            source,
            m_launch(Fixed::from_int(8), Fixed::ZERO, Fixed::ZERO), // grazing, east
            m_forces(
                Fixed::from_int(10),
                Fixed::from_ratio(1, 10),
                Fixed::ONE,
                200,
            ),
        );
        assert!(
            path.landed_from_flight,
            "the parcel launched off the break and landed from flight (rest x = {})",
            path.rest % w
        );
        assert!(
            path.rest % w > 8,
            "it cleared the break and landed on the low ground beyond (x = {})",
            path.rest % w
        );
    }

    #[test]
    fn a_degenerate_launch_rests_at_the_source() {
        let (w, h) = (10usize, 3usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = w + 5;
        let f = m_forces(
            Fixed::from_int(10),
            Fixed::from_ratio(1, 10),
            Fixed::ONE,
            50,
        );
        // Zero speed.
        let still = momentum_integrate(
            w,
            h,
            &flat,
            source,
            m_launch(Fixed::ZERO, deg45(), Fixed::ZERO),
            f,
        );
        assert_eq!(still.rest, source);
        assert_eq!(still.track, vec![source]);
        // Straight up (a right angle): no horizontal motion.
        let up = momentum_integrate(
            w,
            h,
            &flat,
            source,
            m_launch(Fixed::from_int(10), Fixed::HALF_PI, Fixed::ZERO),
            f,
        );
        assert_eq!(up.rest, source);
    }

    #[test]
    fn the_body_force_slot_biases_the_contact_runout() {
        // An extra body force acting along its own potential gradient does work on the parcel in contact, so an
        // alien contact force is a data row (admit-the-alien). A potential that falls toward the east adds
        // energy to an eastward contact run, so the parcel reaches farther than under gravity and friction
        // alone. Flat ground, so gravity does no horizontal work and the effect is the body force's alone.
        let (w, h) = (40usize, 3usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = w + 2;
        let g = Fixed::from_int(10);
        let mu = Fixed::from_ratio(1, 10);
        let speed = Fixed::from_int(2);
        let f = m_forces(g, mu, Fixed::ONE, 200);
        let baseline = momentum_integrate(
            w,
            h,
            &flat,
            source,
            m_launch(speed, Fixed::ZERO, Fixed::ZERO),
            f,
        );
        // A potential decreasing toward the east (a downhill in the extra field), coefficient 1.
        let mut potential = vec![Fixed::ZERO; w * h];
        for y in 0..h {
            for x in 0..w {
                potential[y * w + x] =
                    smul(Fixed::from_ratio(1, 2), Fixed::from_int((w - 1 - x) as i32));
            }
        }
        let extra = [BodyForce {
            coefficient: Fixed::ONE,
            potential: &potential,
        }];
        let pushed = momentum_integrate_with_forces(
            w,
            h,
            &flat,
            source,
            m_launch(speed, Fixed::ZERO, Fixed::ZERO),
            f,
            &extra,
        );
        assert!(
            pushed.rest % w > baseline.rest % w,
            "the body force carries the parcel farther east ({} vs {})",
            pushed.rest % w,
            baseline.rest % w
        );
    }

    #[test]
    fn the_march_is_bounded_by_the_step_cap() {
        // A far-reaching flat launch with a tiny gravity would fly a long way; the cap bounds the track length.
        let (w, h) = (200usize, 3usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = w;
        let path = momentum_integrate(
            w,
            h,
            &flat,
            source,
            m_launch(Fixed::from_int(50), deg45(), Fixed::ZERO),
            m_forces(
                Fixed::from_ratio(1, 100),
                Fixed::from_ratio(1, 10),
                Fixed::ONE,
                5,
            ),
        );
        assert!(
            path.track.len() <= 5 + 1,
            "the track is bounded by the step cap ({} cells)",
            path.track.len()
        );
    }

    #[test]
    fn the_integration_is_deterministic() {
        let (w, h) = (30usize, 30usize);
        let e: Vec<Fixed> = (0..w * h)
            .map(|i| Fixed::from_int(((i * 7) % 11) as i32))
            .collect();
        let l = m_launch(
            Fixed::from_int(12),
            Fixed::HALF_PI.div(Fixed::from_int(3)), // 30 degrees
            Fixed::HALF_PI.div(Fixed::from_int(2)), // azimuth 45
        );
        let f = m_forces(
            Fixed::from_int(9),
            Fixed::from_ratio(1, 10),
            Fixed::ONE,
            300,
        );
        let a = momentum_integrate(w, h, &e, 15 * w + 15, l, f);
        let b = momentum_integrate(w, h, &e, 15 * w + 15, l, f);
        assert_eq!(a, b, "the same launch and terrain reproduce the same path");
    }

    // The east-west and north-south parcel counts of a fan's blanket around a source.
    fn axis_counts(
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
    fn an_airborne_fan_spreads_near_symmetrically_with_true_angular_resolution_on_flat_ground() {
        // Flat ground, source at the centre, a 16-azimuth fan at a steep angle (the airborne regime): every
        // azimuth lands at its own cell, the parcel count is conserved, and the blanket is symmetric east-west
        // and north-south to within one parcel (the residual a deterministic sub-cell floor-rounding artifact).
        let (w, h) = (61usize, 61usize);
        let flat = vec![Fixed::ZERO; w * h];
        let (sx, sy) = (30usize, 30usize);
        let source = sy * w + sx;
        let fan = MomentumFan {
            speed: Fixed::from_int(10),
            elevation_angle: deg45(),
            azimuths: 16,
        };
        let dist = momentum_fan(
            w,
            h,
            &flat,
            source,
            fan,
            m_forces(
                Fixed::from_int(10),
                Fixed::from_ratio(1, 2),
                Fixed::ONE,
                200,
            ),
        );
        let total: u64 = dist.iter().map(|wt| wt.weight).sum();
        assert_eq!(total, 16, "every launched parcel is accounted for");
        assert!(
            dist.len() >= 12,
            "the airborne fan resolves distinct azimuths, not four axes (got {} cells)",
            dist.len()
        );
        let (east, west, north, south) = axis_counts(&dist, w, sx, sy);
        assert!(
            east.abs_diff(west) <= 1 && north.abs_diff(south) <= 1,
            "near-symmetric on flat ground (E{east} W{west} N{north} S{south})"
        );
    }

    #[test]
    fn a_contact_fan_resolves_more_azimuths_than_the_surface_four_axis_fan() {
        // The unification's bonus, proven by contrast rather than asserted: a grazing (contact-regime) fan
        // through the unified integrator holds each azimuth's continuous heading and floors it to a cell, so it
        // resolves many distinct landing cells, where the surface integrator's greedy 4-connected launch fan
        // collapses the same isotropic launch onto the four axes. Both run the SAME isotropic launch on the
        // SAME flat ground, so the difference is the angular resolution alone.
        let (w, h) = (61usize, 61usize);
        let flat = vec![Fixed::ZERO; w * h];
        let (sx, sy) = (30usize, 30usize);
        let source = sy * w + sx;
        let g = Fixed::from_int(10);
        let mu = Fixed::from_ratio(1, 10);
        let speed = Fixed::from_int(4); // grazing and slow enough to skim, far enough to resolve
        let unified = momentum_fan(
            w,
            h,
            &flat,
            source,
            MomentumFan {
                speed,
                elevation_angle: Fixed::ZERO, // grazing: the contact regime
                azimuths: 16,
            },
            m_forces(g, mu, Fixed::ONE, 200),
        );
        // The surface launch fan with the matching isotropic launch (energy e0 = v^2/2, the same speed bias).
        let e0 = smul(Fixed::from_ratio(1, 2), smul(speed, speed));
        let surface = crate::runout::launch_fan(
            w,
            h,
            &flat,
            source,
            crate::runout::LaunchFan {
                energy: e0,
                speed,
                directions: 16,
            },
            RunoutForces {
                gravity: g,
                friction: mu,
                cell_size: Fixed::ONE,
                step_cap: 200,
            },
            &[],
        );
        let unified_total: u64 = unified.iter().map(|wt| wt.weight).sum();
        assert_eq!(unified_total, 16, "the unified fan conserves the parcels");
        assert!(
            unified.len() >= 12,
            "the contact fan resolves distinct azimuths (got {} cells)",
            unified.len()
        );
        assert!(
            unified.len() > surface.len(),
            "the unified contact fan resolves more cells than the surface four-axis fan (unified {} vs surface {})",
            unified.len(),
            surface.len()
        );
        let (east, west, north, south) = axis_counts(&unified, w, sx, sy);
        assert!(
            east.abs_diff(west) <= 1 && north.abs_diff(south) <= 1,
            "the contact blanket is near-symmetric on flat ground (E{east} W{west} N{north} S{south})"
        );
    }

    #[test]
    fn a_slope_breaks_the_fan_blanket() {
        // On a ramp falling to the east, the downhill (east) parcels reach farther than the uphill (west) ones,
        // so the blanket stretches east: the ring is broken by the terrain, not by an authored shape.
        let (w, h) = (61usize, 41usize);
        let ramp = east_downhill(w, h, Fixed::from_ratio(1, 4));
        let (sx, sy) = (30usize, 20usize);
        let source = sy * w + sx;
        let fan = MomentumFan {
            speed: Fixed::from_int(10),
            elevation_angle: deg45(),
            azimuths: 16,
        };
        let dist = momentum_fan(
            w,
            h,
            &ramp,
            source,
            fan,
            m_forces(
                Fixed::from_int(10),
                Fixed::from_ratio(1, 2),
                Fixed::ONE,
                300,
            ),
        );
        let max_east = dist
            .iter()
            .map(|wt| wt.dest % w)
            .filter(|&x| x > sx)
            .map(|x| x - sx)
            .max()
            .unwrap_or(0);
        let max_west = dist
            .iter()
            .map(|wt| wt.dest % w)
            .filter(|&x| x < sx)
            .map(|x| sx - x)
            .max()
            .unwrap_or(0);
        assert!(
            max_east > max_west,
            "the terrain stretches the blanket downhill (east reach {max_east} > west reach {max_west})"
        );
    }

    #[test]
    fn the_fan_feeds_the_operator_a_conservative_blanket() {
        use crate::redistribute::{redistribute, Redistribution};
        let (w, h) = (41usize, 41usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = 20 * w + 20;
        let fan = MomentumFan {
            speed: Fixed::from_int(9),
            elevation_angle: deg45(),
            azimuths: 16,
        };
        let dist = momentum_fan(
            w,
            h,
            &flat,
            source,
            fan,
            m_forces(
                Fixed::from_int(10),
                Fixed::from_ratio(1, 2),
                Fixed::ONE,
                200,
            ),
        );
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
    fn the_fan_is_deterministic() {
        let (w, h) = (41usize, 41usize);
        let e: Vec<Fixed> = (0..w * h)
            .map(|i| Fixed::from_int(((i * 3) % 7) as i32))
            .collect();
        let source = 20 * w + 20;
        let fan = MomentumFan {
            speed: Fixed::from_int(12),
            elevation_angle: deg45(),
            azimuths: 24,
        };
        let f = m_forces(
            Fixed::from_int(9),
            Fixed::from_ratio(1, 10),
            Fixed::ONE,
            300,
        );
        let a = momentum_fan(w, h, &e, source, fan, f);
        let b = momentum_fan(w, h, &e, source, fan, f);
        assert_eq!(a, b, "the same fan reproduces the same blanket");
    }

    #[test]
    fn a_zero_azimuth_fan_is_empty() {
        let (w, h) = (9usize, 9usize);
        let flat = vec![Fixed::ZERO; w * h];
        let fan = MomentumFan {
            speed: Fixed::from_int(10),
            elevation_angle: deg45(),
            azimuths: 0,
        };
        assert!(momentum_fan(
            w,
            h,
            &flat,
            40,
            fan,
            m_forces(
                Fixed::from_int(10),
                Fixed::from_ratio(1, 10),
                Fixed::ONE,
                50
            )
        )
        .is_empty());
    }
}
