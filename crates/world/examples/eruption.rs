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

//! A visible VOLCANIC ERUPTION, in cross-section, driven by the derived atmosphere-coupled flight integrator
//! ([`civsim_world::drag_flight`]). A vent at the summit of a cone throws a spread of fragments spanning a
//! range of sizes; each fragment flies through the atmosphere under gravity and drag, and where its arc meets
//! the flank or plain it impacts. The physics does the sorting: a dense boulder (small drag coupling) flies
//! far onto the plain, a fine ash grain (large drag coupling) is braked to a near-vent fall, so the eruption
//! lays a SIZE-SORTED blanket (coarse near, fine far) with no authored fall table. A lava flow runs down one
//! flank to a runout set by the energy budget.
//!
//! What is DERIVED: every fragment trajectory (the drag_flight integrator, the same one the tests pin against
//! the vacuum parabola), the impact point (arc meets terrain), the impact speed, and the size sorting. What
//! is an authored, labelled STAND-IN: the cone cross-section geometry, the launch spectrum (speeds, angles,
//! and the size classes' drag couplings), and the atmosphere parameters, all of which the accretion,
//! fragmentation, and atmosphere chains will derive later. It is fully deterministic (a fixed hash, no RNG),
//! so the eruption replays identically. Run: `cargo run -p civsim-world --example eruption`.

use civsim_core::Fixed;
use civsim_world::drag_flight::{drag_flight, Atmosphere, DragForces, DragLaunch, FlightSample};

const WIDTH_M: f64 = 12_000.0;
const VENT_X: f64 = 6_000.0;
const SUMMIT_M: f64 = 1_500.0;
const FLANK_SLOPE: f64 = 0.62; // tan(~32 deg), the angle of repose of loose tephra
const N_PARCELS: usize = 72;
const SEED: u32 = 0x_A5F0_1E7A;

fn hash32(mut x: u32) -> u32 {
    x ^= SEED;
    x = x.wrapping_mul(0x9E37_79B1);
    x ^= x >> 16;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    x = x.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 16;
    x
}

/// The cone cross-section: a symmetric edifice sloping from the summit to the plain (a labelled stand-in
/// geometry, retiring when the deposited ejecta build the edifice).
fn terrain(x: f64) -> f64 {
    (SUMMIT_M - FLANK_SLOPE * (x - VENT_X).abs()).max(0.0)
}

/// The five fragment size classes, each a representative inverse ballistic coefficient beta = C_d A / m
/// (drag area per unit mass). A dense boulder is small beta; fine ash is large beta.
const SIZE_CLASSES: [(&str, f64); 5] = [
    ("boulder", 2.0e-4),
    ("block", 1.0e-3),
    ("lapillus", 6.0e-3),
    ("coarse ash", 3.0e-2),
    ("fine ash", 1.2e-1),
];

fn fx(v: f64) -> Fixed {
    // A helper for building Fixed from an f64 launch parameter (example-only; the engine reads Fixed data).
    Fixed::from_bits((v * (1i64 << 32) as f64) as i64)
}

fn main() {
    // The atmosphere the fragments fly through (a labelled Earth-like stand-in: rho_0 from the ideal-gas law
    // at the surface, H from hydrostatic balance).
    let atmo = Atmosphere {
        surface_density: Fixed::from_ratio(12, 10), // 1.2 kg/m^3
        scale_height: Fixed::from_int(8000),        // 8 km
    };
    let forces = DragForces {
        gravity: Fixed::from_ratio(981, 100),
        dt: Fixed::from_ratio(2, 100),
        step_cap: 6000,
    };
    let launch_height = fx(SUMMIT_M);

    println!(
        "ERUPTION width={} vent_x={} summit={} slope={} rho0=1.2 H=8000 g=9.81",
        WIDTH_M, VENT_X, SUMMIT_M, FLANK_SLOPE
    );
    // The terrain profile, sampled every 40 m.
    let n_terr = (WIDTH_M / 40.0) as usize;
    print!("TERRAIN");
    for i in 0..=n_terr {
        print!(" {:.0}", terrain(i as f64 * 40.0));
    }
    println!();

    for i in 0..N_PARCELS {
        let h = hash32(i as u32);
        let sign = if h & 1 == 0 { 1.0 } else { -1.0 };
        let speed = 45.0 + (((h >> 1) % 1000) as f64 / 1000.0) * 195.0; // 45..240 m/s
        let angle_deg = 32.0 + (((h >> 11) % 1000) as f64 / 1000.0) * 55.0; // 32..87 deg
                                                                            // Bias the size mix toward finer fragments (a real fragmentation spectrum has far more ash than
                                                                            // boulders): 1 - (1-u)^2 pushes most parcels into the higher-beta (finer) classes, so the near-vent
                                                                            // ash plume dominates and the far-flung boulders are the rarer ballistic bombs.
        let u = ((h >> 20) % 1000) as f64 / 1000.0;
        let cls = (((1.0 - (1.0 - u) * (1.0 - u)) * SIZE_CLASSES.len() as f64) as usize)
            .min(SIZE_CLASSES.len() - 1);
        let (_cname, beta) = SIZE_CLASSES[cls];

        let launch = DragLaunch {
            speed: fx(speed),
            elevation_angle: fx(angle_deg.to_radians()),
            ballistic_beta: fx(beta),
        };
        let samples = drag_flight(launch, atmo, forces, launch_height).expect("valid atmosphere");

        // World-plane samples (x measured from the vent, signed by launch direction) and the impact: the
        // first sample whose height drops to or below the terrain under it.
        let mut impact: Option<(f64, f64, f64)> = None; // (world_x, z, impact_speed)
        let world = |s: &FlightSample| (VENT_X + sign * s.x.to_f64_lossy(), s.z.to_f64_lossy());
        for w in samples.windows(2) {
            let (wx1, z1) = world(&w[1]);
            if w[1].time.to_f64_lossy() > 0.0 && z1 <= terrain(wx1) {
                let sp = (w[1].vx.to_f64_lossy().powi(2) + w[1].vz.to_f64_lossy().powi(2)).sqrt();
                impact = Some((wx1, terrain(wx1).max(z1), sp));
                break;
            }
        }
        let (ix, iz, isp) = impact.unwrap_or_else(|| {
            let (wx, z) = world(samples.last().unwrap());
            (wx, z, 0.0)
        });

        // Downsample the trajectory to ~36 points (up to the impact) for the animation.
        let cut = samples
            .iter()
            .position(|s| {
                let (wx, z) = world(s);
                s.time.to_f64_lossy() > 0.0 && z <= terrain(wx)
            })
            .unwrap_or(samples.len() - 1);
        let keep = &samples[..=cut];
        let stride = (keep.len() / 36).max(1);
        print!("PARCEL {} {:.4} {:.1} {:.1} {:.1}", cls, beta, ix, iz, isp);
        for (j, s) in keep.iter().enumerate() {
            if j % stride == 0 || j == keep.len() - 1 {
                let (wx, z) = world(s);
                print!(" {:.1},{:.1},{:.2}", wx, z, s.time.to_f64_lossy());
            }
        }
        println!();

        // For a few coarse fragments, also emit the VACUUM arc (beta = 0) so the render can show what the
        // atmosphere took away.
        if cls <= 1 && i % 3 == 0 {
            let vac = DragLaunch {
                ballistic_beta: Fixed::ZERO,
                ..launch
            };
            let vs = drag_flight(vac, atmo, forces, launch_height).expect("valid");
            let cutv = vs
                .iter()
                .position(|s| {
                    let (wx, z) = world(s);
                    s.time.to_f64_lossy() > 0.0 && z <= terrain(wx)
                })
                .unwrap_or(vs.len() - 1);
            let keepv = &vs[..=cutv];
            let stridev = (keepv.len() / 36).max(1);
            print!("VACUUM {}", i);
            for (j, s) in keepv.iter().enumerate() {
                if j % stridev == 0 || j == keepv.len() - 1 {
                    let (wx, z) = world(s);
                    print!(" {:.1},{:.1}", wx, z);
                }
            }
            println!();
        }
    }

    // A lava flow down the right flank: the runout length from the energy budget (the flow descends the
    // summit height H against an effective friction mu, giving the runout ratio H/L = mu, the same
    // energy-budget relation the surface runout integrator uses). A hot, low-friction flow runs far.
    let mu_lava = 0.28; // effective friction of a channelized flow (a labelled stand-in)
    let runout_surface = SUMMIT_M / mu_lava; // total path length along the surface
                                             // Walk the runout distance down the right flank then onto the plain, emitting the surface path.
    let flank_surface = SUMMIT_M * (1.0 + FLANK_SLOPE * FLANK_SLOPE).sqrt() / FLANK_SLOPE; // slope length
    print!("LAVA {:.1}", runout_surface);
    let steps = 60;
    for j in 0..=steps {
        let s = runout_surface * j as f64 / steps as f64;
        let (wx, wz) = if s <= flank_surface {
            let horiz = s / (1.0 + FLANK_SLOPE * FLANK_SLOPE).sqrt();
            (VENT_X + horiz, terrain(VENT_X + horiz))
        } else {
            let horiz =
                (flank_surface / (1.0 + FLANK_SLOPE * FLANK_SLOPE).sqrt()) + (s - flank_surface);
            (VENT_X + horiz, 0.0)
        };
        print!(" {:.1},{:.1}", wx, wz);
    }
    println!();
}
