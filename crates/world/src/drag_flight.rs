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

//! The ATMOSPHERE-COUPLED flight integrator: the numerical generalization of the vacuum ballistic arc
//! ([`crate::ballistic`]) that carries the drag of the air the parcel flies through. Where the ballistic
//! integrator flies a parcel on the exact projectile parabola (gravity alone, a closed arc), a parcel moving
//! through an atmosphere also feels aerodynamic DRAG, which is not conservative and has no closed arc, so the
//! trajectory is integrated step by step in the launch vertical plane. At zero drag the two coincide: this
//! law REDUCES to the ballistic parabola, the reduction the test measures against the kinematic range
//! `R = v^2 sin(2 alpha)/g` (the same way the momentum integrator's exactness against the ballistic oracle is
//! measured, not asserted).
//!
//! The physics, from Newton with no authored constant. A parcel of velocity `v = (v_x, v_z)` feels gravity
//! `-g` on the vertical and a drag acceleration opposing its motion, `a_drag = -1/2 * rho(z) * beta * |v| *
//! v`, where `rho(z)` is the local air density and `beta = C_d * A / m` is the parcel's inverse ballistic
//! coefficient (its drag area per unit mass). `beta` is per-parcel DATA: a dense boulder has a small `beta`
//! and flies nearly ballistic and far, a fine ash grain has a large `beta` and is braked to its terminal fall
//! near the vent, so a single launch spread deposits a SIZE-SORTED blanket, the sorting emerging from the
//! physics rather than an authored fall table. The admit-the-alien seam is the same `beta`: a mineral, an
//! ice, or an exotic ejecta is a different `beta` row and a different `rho(z)`, never a rewrite.
//!
//! The atmosphere is the barometric (isothermal-hydrostatic) profile `rho(z) = rho_0 * exp(-z / H)`, with the
//! surface density `rho_0` and the scale height `H` supplied as DATA (both derived upstream: `rho_0` from the
//! ideal-gas law at the surface pressure and temperature, `H = k_B T / (mu m_u g)` from hydrostatic balance).
//! So a thicker or thinner atmosphere, or a taller scale height, is a data change: a dense-air world brakes
//! its ejecta hard and a near-vacuum world lets them fly the bare parabola, with no code change.
//!
//! Gravity `g` is a PARAMETER (the floor `mech.gravitational_acceleration`), so the range reads the world's
//! own gravity. Determinism (Principle 3, Principle 10): fixed-point throughout (the launch angle through the
//! integer CORDIC [`Fixed::sin_cos`], the speed through the exact [`Fixed::sqrt`], the density through the
//! pinned [`Fixed::exp`], no float), a fixed timestep, and a bounded march (at most `step_cap` steps), so the
//! whole trajectory is a pure function of the launch, the atmosphere, and the forces, worker-invariant.

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

/// The barometric atmosphere the parcel flies through: an isothermal-hydrostatic profile whose only two
/// parameters are the surface density and the scale height, both derived upstream and passed as DATA.
#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    /// The air density at the surface `rho_0` (mass per volume, world units), from the ideal-gas law at the
    /// surface pressure and temperature. Zero is a vacuum (no drag).
    pub surface_density: Fixed,
    /// The atmospheric scale height `H` (length), from hydrostatic balance `H = k_B T / (mu m_u g)`. The
    /// density falls by `1/e` over this height. Must be positive.
    pub scale_height: Fixed,
}

/// A parcel's launch in its vertical plane plus its drag coupling. The azimuth is not carried here: this is
/// the in-plane flight, and a fan over azimuths lays the plane down in each compass direction.
#[derive(Clone, Copy, Debug)]
pub struct DragLaunch {
    /// The launch speed `v` (velocity-vector magnitude). Zero is no launch.
    pub speed: Fixed,
    /// The elevation angle `alpha` above the horizontal, in radians.
    pub elevation_angle: Fixed,
    /// The inverse ballistic coefficient `beta = C_d A / m` (drag area per unit mass), per-parcel DATA. Zero
    /// recovers the vacuum parabola; larger values brake the parcel sooner (finer or lighter ejecta).
    pub ballistic_beta: Fixed,
}

/// The world forces and the integration controls.
#[derive(Clone, Copy, Debug)]
pub struct DragForces {
    /// Gravity `g` (`mech.gravitational_acceleration`), a PARAMETER; a world's derived `g = G M / R^2` flows
    /// in with no code change.
    pub gravity: Fixed,
    /// The integration timestep `dt` (time). Smaller is more accurate and more steps; the reduction to the
    /// vacuum arc tightens as `dt` shrinks.
    pub dt: Fixed,
    /// The maximum number of steps the march may take, a determinism and performance bound, never a physical
    /// duration.
    pub step_cap: u32,
}

/// One sample along the flight: the elapsed time, the horizontal range and height (in the launch plane), and
/// the velocity components. The last sample is at or below the reference ground (the parcel's return).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlightSample {
    /// Elapsed time since launch.
    pub time: Fixed,
    /// Horizontal range from the launch point (along the launch azimuth).
    pub x: Fixed,
    /// Height above the reference (`z = 0`), starting at the launch height.
    pub z: Fixed,
    /// Horizontal velocity component.
    pub vx: Fixed,
    /// Vertical velocity component.
    pub vz: Fixed,
}

/// The air density at an altitude, the barometric profile `rho(z) = rho_0 * exp(-z / H)`. Below the surface
/// (`altitude < 0`) it clamps to the surface density (a parcel does not fly underground). `None` if the scale
/// height is not positive.
pub fn atmospheric_density(altitude: Fixed, atmosphere: Atmosphere) -> Option<Fixed> {
    if atmosphere.scale_height <= Fixed::ZERO {
        return None;
    }
    let z = if altitude < Fixed::ZERO {
        Fixed::ZERO
    } else {
        altitude
    };
    let ratio = z.checked_div(atmosphere.scale_height)?;
    let factor = ssub(Fixed::ZERO, ratio).exp(); // exp(-z/H)
    Some(smul(atmosphere.surface_density, factor))
}

/// Integrate a parcel's trajectory through the atmosphere from `launch_height` (its vertical starting height
/// above the reference), returning the samples of its flight until it returns to the reference ground
/// (`z <= 0` while descending) or the step cap is reached. Semi-implicit (symplectic) Euler: the velocity is
/// updated from the accelerations, then the position from the updated velocity. `None` if the scale height is
/// not positive; a non-positive speed returns the single launch sample (no flight).
pub fn drag_flight(
    launch: DragLaunch,
    atmosphere: Atmosphere,
    forces: DragForces,
    launch_height: Fixed,
) -> Option<Vec<FlightSample>> {
    if atmosphere.scale_height <= Fixed::ZERO {
        return None;
    }
    let (sin_a, cos_a) = launch.elevation_angle.sin_cos();
    let mut vx = smul(launch.speed, cos_a);
    let mut vz = smul(launch.speed, sin_a);
    let mut x = Fixed::ZERO;
    let mut z = launch_height;
    let mut t = Fixed::ZERO;
    let half = Fixed::from_ratio(1, 2);
    let dt = forces.dt;

    let mut samples = vec![FlightSample {
        time: t,
        x,
        z,
        vx,
        vz,
    }];
    if launch.speed <= Fixed::ZERO {
        return Some(samples);
    }

    for _ in 0..forces.step_cap {
        // Speed magnitude |v| = sqrt(v_x^2 + v_z^2).
        let v_mag = sadd(smul(vx, vx), smul(vz, vz)).sqrt();
        let rho = atmospheric_density(z, atmosphere)?;
        // The drag scalar 1/2 * rho * beta * |v|, so a_drag = -(this) * v.
        let drag = smul(smul(smul(half, rho), launch.ballistic_beta), v_mag);
        let ax = ssub(Fixed::ZERO, smul(drag, vx));
        let az = ssub(ssub(Fixed::ZERO, smul(drag, vz)), forces.gravity);
        // Semi-implicit Euler: velocity first, then position from the new velocity.
        vx = sadd(vx, smul(ax, dt));
        vz = sadd(vz, smul(az, dt));
        x = sadd(x, smul(vx, dt));
        z = sadd(z, smul(vz, dt));
        t = sadd(t, dt);
        samples.push(FlightSample {
            time: t,
            x,
            z,
            vx,
            vz,
        });
        // Returned to the reference ground while descending: the flight is over.
        if z <= Fixed::ZERO && vz < Fixed::ZERO {
            break;
        }
    }
    Some(samples)
}

/// The horizontal range at which a flight returns to its launch height (the ground-level landing distance), a
/// convenience over [`drag_flight`] for the reduction test and the range comparisons. `None` if the flight
/// never descends back to the launch height within the step cap.
pub fn drag_range(launch: DragLaunch, atmosphere: Atmosphere, forces: DragForces) -> Option<Fixed> {
    let samples = drag_flight(launch, atmosphere, forces, Fixed::ZERO)?;
    // Find the first descending crossing of z = 0 and interpolate the range there.
    for pair in samples.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a.z >= Fixed::ZERO && b.z < Fixed::ZERO {
            let dz = ssub(a.z, b.z);
            if dz <= Fixed::ZERO {
                return Some(b.x);
            }
            let frac = a.z.checked_div(dz)?; // a.z / (a.z - b.z)
            return Some(sadd(a.x, smul(ssub(b.x, a.x), frac)));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deg(d: i64) -> Fixed {
        Fixed::PI.checked_mul(Fixed::from_ratio(d, 180)).unwrap()
    }
    fn earth_air() -> Atmosphere {
        Atmosphere {
            surface_density: Fixed::from_ratio(12, 10), // 1.2 kg/m^3
            scale_height: Fixed::from_int(8000),        // 8 km
        }
    }
    fn forces() -> DragForces {
        DragForces {
            gravity: Fixed::from_ratio(981, 100),
            dt: Fixed::from_ratio(1, 100),
            step_cap: 100_000,
        }
    }

    #[test]
    fn zero_drag_reduces_to_the_vacuum_parabola() {
        // With beta = 0 the trajectory is the exact projectile parabola, so the range must match the kinematic
        // range R = v^2 sin(2 alpha)/g. For v = 100 m/s at 45 degrees, R = 100^2 * 1 / 9.81 = 1019.4 m.
        let launch = DragLaunch {
            speed: Fixed::from_int(100),
            elevation_angle: deg(45),
            ballistic_beta: Fixed::ZERO,
        };
        let r = drag_range(launch, earth_air(), forces())
            .expect("a launched parcel returns to the ground")
            .to_f64_lossy();
        assert!(
            (r - 1019.4).abs() < 6.0,
            "zero-drag range reduces to the kinematic 1019 m, got {r}"
        );
    }

    #[test]
    fn drag_shortens_the_range_monotonically() {
        // A larger drag coupling brakes the parcel sooner, so the range falls as beta grows: the size-sorting
        // that lays a fine-near, coarse-far ejecta blanket. v = 150 m/s at 45 degrees.
        let base = |b: i64| DragLaunch {
            speed: Fixed::from_int(150),
            elevation_angle: deg(45),
            ballistic_beta: Fixed::from_ratio(b, 1000),
        };
        let mut prev = f64::INFINITY;
        for b in [0, 1, 5, 20, 100] {
            let r = drag_range(base(b), earth_air(), forces())
                .expect("returns to ground")
                .to_f64_lossy();
            assert!(
                r < prev,
                "range must fall as the drag coupling grows: beta={b} gave {r}, previous {prev}"
            );
            prev = r;
        }
    }

    #[test]
    fn a_dense_boulder_outflies_a_light_ash_grain() {
        // Same launch, two drag couplings: the low-beta boulder flies far, the high-beta ash falls near.
        let air = earth_air();
        let f = forces();
        let boulder = DragLaunch {
            speed: Fixed::from_int(200),
            elevation_angle: deg(50),
            ballistic_beta: Fixed::from_ratio(2, 10000), // dense, small drag area per mass
        };
        let ash = DragLaunch {
            ballistic_beta: Fixed::from_ratio(5, 100), // light, large drag area per mass
            ..boulder
        };
        let rb = drag_range(boulder, air, f).unwrap().to_f64_lossy();
        let ra = drag_range(ash, air, f).unwrap().to_f64_lossy();
        assert!(
            rb > ra * 5.0,
            "the boulder ({rb} m) flies far beyond the ash ({ra} m)"
        );
    }

    #[test]
    fn the_barometric_density_falls_by_one_over_e_per_scale_height() {
        let air = earth_air();
        let rho0 = atmospheric_density(Fixed::ZERO, air)
            .unwrap()
            .to_f64_lossy();
        let rho_h = atmospheric_density(air.scale_height, air)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (rho0 - 1.2).abs() < 1e-3,
            "surface density is rho_0, got {rho0}"
        );
        assert!(
            (rho_h - 1.2 / std::f64::consts::E).abs() < 1e-2,
            "density falls to rho_0/e at one scale height, got {rho_h}"
        );
    }

    #[test]
    fn the_flight_is_deterministic_and_guards_the_atmosphere() {
        let launch = DragLaunch {
            speed: Fixed::from_int(120),
            elevation_angle: deg(40),
            ballistic_beta: Fixed::from_ratio(3, 1000),
        };
        assert_eq!(
            drag_flight(launch, earth_air(), forces(), Fixed::ZERO),
            drag_flight(launch, earth_air(), forces(), Fixed::ZERO),
            "the trajectory replays byte for byte"
        );
        let vacuum = Atmosphere {
            surface_density: Fixed::from_ratio(12, 10),
            scale_height: Fixed::ZERO,
        };
        assert_eq!(
            drag_flight(launch, vacuum, forces(), Fixed::ZERO),
            None,
            "a non-positive scale height is rejected"
        );
        assert_eq!(atmospheric_density(Fixed::from_int(100), vacuum), None);
    }
}
