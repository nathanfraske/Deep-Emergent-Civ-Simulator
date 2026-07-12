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

//! The ballistic-arc integrator: where a parcel launched on a ballistic arc lands, a sibling regime to the
//! surface-runout integrator ([`crate::runout`]). The surface integrator walks a parcel ALONG the ground;
//! this one flies it OVER the ground on a projectile arc, so an impact's ejecta clears intervening terrain
//! and lands at a distance set by its launch, the vertical-arc regime the surface-contact integrator cannot
//! express (gate-ruled, PR #177: this is the projectile regime, not a `BodyForce` surface term, which is a
//! horizontal surface-potential gradient and could only walk the parcel into a valley, never over it).
//!
//! The physics, from kinematics with no authored constant. A parcel launches at speed `v`, elevation angle
//! `alpha`, and azimuth `phi`. Along the azimuth its height above the launch point is the projectile parabola
//! `z(x) = x*tan(alpha) - g*x^2 / (2*v^2*cos^2(alpha))`, so on flat ground it lands at the kinematic range
//! `R = v^2*sin(2*alpha)/g`. It lands where the arc's absolute height (the launch elevation plus `z`) first
//! meets the terrain along the azimuth: a downhill slope holds the arc above the falling ground longer and
//! EXTENDS the range, an uphill rim meets it sooner and SHORTENS it. That terrain intersection is where the
//! blanket's shape emerges (the ejecta fan, the next slice), no authored spread.
//!
//! Gravity `g` is a PARAMETER (the floor `mech.gravitational_acceleration`), never hardcoded, so the range
//! reads the world's own gravity, including a derived `g = G*M/R^2` when a world supplies it (gate note, PR
//! #177): the range is `R ~ 1/g`, so a low-gravity world throws its ejecta correspondingly farther, with no
//! code change.
//!
//! Determinism (Principle 3, Principle 10): fixed-point throughout (the angles through the integer CORDIC
//! [`Fixed::sin_cos`], no float), a bounded march along the azimuth (at most `step_cap` steps, never an
//! unbounded spin), and the cell mapping a deterministic floor ([`Fixed::to_int`]), so the landing is a pure
//! function of the launch, the terrain, and the forces, worker-invariant.

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

/// A ballistic launch, the physical initial conditions an impact event supplies as DATA: the launch speed,
/// the elevation angle above the horizontal, and the azimuth (heading) in radians. The law reads these; it
/// does not read a process category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BallisticLaunch {
    /// The launch speed `v`.
    pub speed: Fixed,
    /// The elevation angle `alpha` above the horizontal, in radians. Zero is a grazing launch (no arc); at
    /// or above a right angle the launch is vertical or backward and the range is zero.
    pub elevation_angle: Fixed,
    /// The azimuth `phi` (compass heading) in radians, measured so the ground direction is `(cos, sin)`.
    pub azimuth: Fixed,
}

/// The world force parameters the ballistic law reads, each a floor axis or a per-world datum (never
/// authored here): gravity and the cell edge length. The step cap is a determinism and performance bound
/// (the march terminates in at most this many steps), not a physical value.
#[derive(Clone, Copy, Debug)]
pub struct BallisticForces {
    /// Gravity `g` (the floor `mech.gravitational_acceleration`), a PARAMETER; a world's derived
    /// `g = G*M/R^2` flows in here with no code change.
    pub gravity: Fixed,
    /// The cell edge length in world units (a per-world spatial datum), the ground distance of one step.
    pub cell_size: Fixed,
    /// The maximum number of steps the march may take, a determinism bound, never a physical range.
    pub step_cap: u32,
}

/// Where a ballistic arc lands: the cells the arc passed over along the azimuth (its ground track, the source
/// first, consecutive duplicates dropped) and the cell it deposits in. The landing is the single-parcel
/// deposit site the redistribution operator credits; the isotropic ejecta fan is the next slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BallisticLanding {
    /// The cells the arc's ground track crossed, in travel order, starting with the source.
    pub track: Vec<usize>,
    /// The cell the arc lands in (the deposit site).
    pub landing: usize,
}

/// Integrate a ballistic launch from `source` over a `width` by `height` fixed-point elevation grid and
/// return where it lands. The arc flies over intervening terrain and deposits where its absolute height
/// (`elevation[source] + z(x)`) first meets the terrain along the azimuth. Deterministic and worker-invariant
/// (fixed-point CORDIC angles, a bounded march, a floor cell map). Panics if `elevation.len()` is not
/// `width * height` or `source` is out of range (construction invariants). A degenerate launch (a
/// non-positive speed, or an elevation angle at or beyond the vertical) lands at the source.
pub fn ballistic_landing(
    width: usize,
    height: usize,
    elevation: &[Fixed],
    source: usize,
    launch: BallisticLaunch,
    forces: BallisticForces,
) -> BallisticLanding {
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

    let at_source = || BallisticLanding {
        track: vec![source],
        landing: source,
    };

    let (sin_a, cos_a) = launch.elevation_angle.sin_cos();
    // A non-positive speed or a launch at or beyond the vertical (cos_a <= 0) has no forward arc.
    if launch.speed <= Fixed::ZERO || cos_a <= Fixed::ZERO {
        return at_source();
    }
    let (sin_p, cos_p) = launch.azimuth.sin_cos();

    // The parabola coefficients. tan(alpha) = sin/cos (cos > 0 here), and the curvature
    // k = g / (2 v^2 cos^2 alpha), so z(x) = x*tan - k*x^2. A tiny cos (near-vertical) makes k enormous, so
    // the arc drops at once and lands near the source, the physically correct short range.
    let tan_a = sin_a.checked_div(cos_a).unwrap_or(Fixed::MAX);
    let v2 = smul(launch.speed, launch.speed);
    let denom = smul(smul(Fixed::from_int(2), v2), smul(cos_a, cos_a));
    let k = if denom > Fixed::ZERO {
        forces.gravity.checked_div(denom).unwrap_or(Fixed::MAX)
    } else {
        Fixed::MAX
    };

    let e0 = elevation[source];
    let sx = (source % width) as i32;
    let sy = (source / width) as i32;

    let mut track = vec![source];
    let mut landing = source;
    for i in 1..=forces.step_cap {
        let step = Fixed::from_int(i as i32);
        // The horizontal distance and the arc height at this step.
        let x = smul(step, forces.cell_size);
        let z = ssub(smul(x, tan_a), smul(k, smul(x, x)));
        let h = sadd(e0, z);
        // The ground position along the azimuth (cell units), floored to a cell.
        let px = sadd(Fixed::from_int(sx), smul(step, cos_p));
        let py = sadd(Fixed::from_int(sy), smul(step, sin_p));
        let cx = px.to_int();
        let cy = py.to_int();
        if cx < 0 || cx >= width as i32 || cy < 0 || cy >= height as i32 {
            break; // the arc left the grid; it lands at the last in-grid cell of the track
        }
        let cell = cy as usize * width + cx as usize;
        if *track.last().expect("track is seeded with the source") != cell {
            track.push(cell);
        }
        landing = cell;
        if h <= elevation[cell] {
            break; // the arc met the ground here
        }
    }

    BallisticLanding { track, landing }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A right-angle-eighth helper: 45 degrees, where sin(2*alpha) = 1 and the flat-ground range is the
    // maximum v^2/g.
    fn deg45() -> Fixed {
        Fixed::HALF_PI.div(Fixed::from_int(2))
    }

    fn forces(gravity: Fixed, cell: Fixed, cap: u32) -> BallisticForces {
        BallisticForces {
            gravity,
            cell_size: cell,
            step_cap: cap,
        }
    }

    fn launch(speed: Fixed, alpha: Fixed, azimuth: Fixed) -> BallisticLaunch {
        BallisticLaunch {
            speed,
            elevation_angle: alpha,
            azimuth,
        }
    }

    #[test]
    fn the_flat_ground_range_matches_the_kinematic_formula() {
        // v = 10, g = 10, alpha = 45 (sin 2a = 1), cell = 1: R = v^2 sin(2a) / g = 100/10 = 10 cells. Launch
        // east from a source with room to fly; the landing is about 10 cells east (within the CORDIC and
        // floor rounding of the ideal, so a tolerance of a cell).
        let (w, h) = (40usize, 3usize);
        let flat = vec![Fixed::ZERO; w * h];
        let sx = 5usize;
        let source = w + sx; // row 1
        let land = ballistic_landing(
            w,
            h,
            &flat,
            source,
            launch(Fixed::from_int(10), deg45(), Fixed::ZERO), // azimuth 0 -> +x (east)
            forces(Fixed::from_int(10), Fixed::ONE, 200),
        );
        let landed_x = (land.landing % w) as i32;
        let range = landed_x - sx as i32;
        assert!(
            (9..=11).contains(&range),
            "the flat-ground range is about v^2 sin(2a)/g = 10 cells (got {range})"
        );
        assert_eq!(land.landing / w, 1, "an eastward launch stays on its row");
    }

    #[test]
    fn a_lower_gravity_throws_the_ejecta_farther() {
        // R ~ 1/g: halving gravity roughly doubles the range, and gravity is read as a parameter so a world's
        // own (or derived) gravity flows straight in.
        let (w, h) = (60usize, 3usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = w + 2;
        let strong = ballistic_landing(
            w,
            h,
            &flat,
            source,
            launch(Fixed::from_int(10), deg45(), Fixed::ZERO),
            forces(Fixed::from_int(10), Fixed::ONE, 400),
        );
        let weak = ballistic_landing(
            w,
            h,
            &flat,
            source,
            launch(Fixed::from_int(10), deg45(), Fixed::ZERO),
            forces(Fixed::from_int(5), Fixed::ONE, 400),
        );
        assert!(
            weak.landing % w > strong.landing % w,
            "lower gravity throws farther ({} vs {})",
            weak.landing % w,
            strong.landing % w
        );
    }

    // A ramp that falls toward the east (higher x is lower), grade per cell, so a downhill launch to the east
    // extends and an uphill launch to the west shortens.
    fn east_downhill(width: usize, height: usize, grade: Fixed) -> Vec<Fixed> {
        let mut e = vec![Fixed::ZERO; width * height];
        for y in 0..height {
            for x in 0..width {
                // Falls toward the east: elevation decreases with x.
                e[y * width + x] = smul(grade, Fixed::from_int((width - 1 - x) as i32));
            }
        }
        e
    }

    #[test]
    fn a_downhill_slope_extends_the_range_an_uphill_rim_shortens_it() {
        let (w, h) = (40usize, 3usize);
        let grade = Fixed::from_ratio(1, 5);
        let ramp = east_downhill(w, h, grade);
        let flat = vec![Fixed::ZERO; w * h];
        let sx = 20usize;
        let source = w + sx;
        let l = launch(Fixed::from_int(10), deg45(), Fixed::ZERO); // east, downhill on the ramp
        let f = forces(Fixed::from_int(10), Fixed::ONE, 200);
        let downhill = ballistic_landing(w, h, &ramp, source, l, f);
        let on_flat = ballistic_landing(w, h, &flat, source, l, f);
        assert!(
            downhill.landing % w > on_flat.landing % w,
            "the arc over a downhill slope reaches farther ({} vs flat {})",
            downhill.landing % w,
            on_flat.landing % w
        );
        // West is uphill on this ramp, so a westward launch lands nearer than the same launch on flat ground.
        let west = launch(
            Fixed::from_int(10),
            deg45(),
            Fixed::HALF_PI.mul(Fixed::from_int(2)),
        ); // phi = pi -> -x
        let uphill = ballistic_landing(w, h, &ramp, source, west, f);
        let flat_west = ballistic_landing(w, h, &flat, source, west, f);
        let uphill_reach = sx as i32 - (uphill.landing % w) as i32;
        let flat_reach = sx as i32 - (flat_west.landing % w) as i32;
        assert!(
            uphill_reach < flat_reach,
            "the arc into an uphill rim lands nearer ({uphill_reach} vs flat {flat_reach})"
        );
    }

    #[test]
    fn the_arc_clears_an_intervening_valley() {
        // A deep valley sits between the source and a far rim. A surface-contact parcel would descend into
        // the valley; the ballistic arc flies over it and lands beyond, on the far side, because the low
        // valley floor never rises to meet the arc.
        let (w, h) = (30usize, 3usize);
        let mut e = vec![Fixed::ZERO; w * h];
        for y in 0..h {
            for x in 10..20 {
                e[y * w + x] = Fixed::from_int(-50); // a deep valley in the flight path
            }
        }
        let sx = 5usize;
        let source = w + sx;
        let land = ballistic_landing(
            w,
            h,
            &e,
            source,
            launch(Fixed::from_int(10), deg45(), Fixed::ZERO),
            forces(Fixed::from_int(10), Fixed::ONE, 200),
        );
        let landed_x = land.landing % w;
        assert!(
            landed_x >= 15,
            "the arc cleared the valley rather than dropping into it (landed at x={landed_x})"
        );
        // No track cell rests on the valley floor below the launch height before the landing.
        assert_eq!(land.landing / w, 1, "the eastward arc stays on its row");
    }

    #[test]
    fn a_near_vertical_launch_lands_near_the_source() {
        // Alpha approaching a right angle: the range collapses toward zero, so the ejecta falls back near
        // where it launched.
        let (w, h) = (20usize, 3usize);
        let flat = vec![Fixed::ZERO; w * h];
        let sx = 10usize;
        let source = w + sx;
        // 89 degrees: HALF_PI * (89/90).
        let almost_vertical = smul(Fixed::HALF_PI, Fixed::from_ratio(89, 90));
        let land = ballistic_landing(
            w,
            h,
            &flat,
            source,
            launch(Fixed::from_int(10), almost_vertical, Fixed::ZERO),
            forces(Fixed::from_int(10), Fixed::ONE, 200),
        );
        let range = (land.landing % w) as i32 - sx as i32;
        assert!(
            (0..=2).contains(&range),
            "a near-vertical launch lands within a cell or two of the source (got {range})"
        );
    }

    #[test]
    fn a_degenerate_launch_lands_at_the_source() {
        let (w, h) = (10usize, 3usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = w + 5;
        let f = forces(Fixed::from_int(10), Fixed::ONE, 50);
        // Zero speed: no launch.
        let still = ballistic_landing(
            w,
            h,
            &flat,
            source,
            launch(Fixed::ZERO, deg45(), Fixed::ZERO),
            f,
        );
        assert_eq!(still.landing, source);
        assert_eq!(still.track, vec![source]);
        // Straight up (a right angle): no forward range.
        let up = ballistic_landing(
            w,
            h,
            &flat,
            source,
            launch(Fixed::from_int(10), Fixed::HALF_PI, Fixed::ZERO),
            f,
        );
        assert_eq!(up.landing, source);
    }

    #[test]
    fn the_landing_is_deterministic() {
        let (w, h) = (30usize, 30usize);
        let e: Vec<Fixed> = (0..w * h)
            .map(|i| Fixed::from_int(((i * 7) % 11) as i32))
            .collect();
        let l = launch(
            Fixed::from_int(12),
            Fixed::HALF_PI.div(Fixed::from_int(3)), // 30 degrees
            Fixed::HALF_PI.div(Fixed::from_int(2)), // azimuth 45
        );
        let f = forces(Fixed::from_int(9), Fixed::ONE, 300);
        let a = ballistic_landing(w, h, &e, 15 * w + 15, l, f);
        let b = ballistic_landing(w, h, &e, 15 * w + 15, l, f);
        assert_eq!(
            a, b,
            "the same launch and terrain reproduce the same landing"
        );
    }

    #[test]
    fn the_march_is_bounded_by_the_step_cap() {
        // A long flat run with a tiny gravity would fly far; the cap bounds the track length regardless.
        let (w, h) = (200usize, 3usize);
        let flat = vec![Fixed::ZERO; w * h];
        let source = w;
        let land = ballistic_landing(
            w,
            h,
            &flat,
            source,
            launch(Fixed::from_int(50), deg45(), Fixed::ZERO),
            forces(Fixed::from_ratio(1, 100), Fixed::ONE, 5),
        );
        assert!(
            land.track.len() <= 5 + 1,
            "the track is bounded by the step cap ({} cells)",
            land.track.len()
        );
    }
}
