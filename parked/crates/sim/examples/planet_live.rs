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

//! A WATCHABLE derived planet: the geology and volcanism run in the terminal, evolving over deep time. This
//! is the running counterpart to the static `planet_volcanism` map: it renders an ANSI truecolor frame each
//! era and steps the interior temperature down, so the volcanism visibly cools from the hot Archean to the
//! present.
//!
//! What is DERIVED (not authored):
//! - The OCEANIC crust composition is the mantle's own first melt: `eutectic_liquid_composition` of the four
//!   lherzolite minerals gives an olivine-poor, clinopyroxene-and-plagioclase-rich basalt (the crust IS the
//!   derived melt), whose density falls out of `crustal_density`. This retires the arbitrary per-plate rock
//!   types the earlier map assigned by hash: the ocean floor is the derived melt, not a lookup.
//! - The ELEVATION is the Airy flotation of that derived density (a lighter continent floats high, the denser
//!   basaltic floor sits low), exactly the `planet_relief` chain.
//! - The VOLCANISM is the derived melt column (`adiabatic_melt_column`) over the derived peridotite solidus,
//!   with the three settings (ridge, arc, plume) emergent from the plate geometry, as `planet_volcanism`.
//! - The DEEP-TIME evolution: as the interior temperature falls, the melt column productivity falls with it,
//!   so the plume and ridge volcanism wane and the komatiite-hot early world gives way to a quieter one. The
//!   melt RESPONSE to temperature is fully derived; the cooling curve itself is a labelled interior driver
//!   (the calibrated secular thermal history is its own arc).
//!
//! Still a labelled stand-in, retiring with its chain: the plate scatter and which plates are continental
//! (accretion, differentiation, and the contingent plate map, which nature seeds too), the CONTINENTAL felsic
//! crust (its derivation needs the alkali-feldspar and water differentiation chain), and the interior
//! temperature curve. The melting and the oceanic composition are the derived phase, run on the map.
//!
//! Run: `cargo run -p civsim-sim --example planet_live` (live animation, cursor-home each frame). Add
//! `-- dump` to print every era's frame in sequence (for capture), `-- dry` for a waterless world (no arc
//! volcanism), and a trailing integer to set the frame delay in milliseconds (live mode).

use civsim_core::Fixed;
use civsim_physics::geodynamics::airy_isostatic_elevation;
use civsim_physics::melting::{
    adiabatic_melt_column, eutectic_liquid_composition, multicomponent_solidus, Endmember,
};
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::petrology::crustal_density;
use civsim_physics::petrology_data::PhaseRegistry;
use std::io::Write;

const WIDTH: usize = 120;
const HEIGHT: usize = 72; // even, so the half-block render packs two rows per line
const PLATES: usize = 14;
const HOTSPOTS: usize = 6;
const SEED: u32 = 0x5EED_1234;
const ERAS: usize = 9;

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

/// The four lherzolite mantle mineral signatures and their molecular formulas (element counts), so the derived
/// first melt can be turned into an element vector the density derivation reads.
struct Mineral {
    end: Endmember,
    formula: &'static [(&'static str, i32)],
}
fn lherzolite() -> [Mineral; 4] {
    [
        Mineral {
            end: Endmember {
                melting_point_k: Fixed::from_int(2163),
                fusion_enthalpy_j_per_mol: Fixed::from_int(114_000),
                fusion_volume_cm3_per_mol: Some(Fixed::from_ratio(39, 10)),
            },
            formula: &[("Mg", 2), ("Si", 1), ("O", 4)], // forsterite
        },
        Mineral {
            end: Endmember {
                melting_point_k: Fixed::from_int(1830),
                fusion_enthalpy_j_per_mol: Fixed::from_int(73_000),
                fusion_volume_cm3_per_mol: Some(Fixed::from_int(5)),
            },
            formula: &[("Mg", 1), ("Si", 1), ("O", 3)], // enstatite
        },
        Mineral {
            end: Endmember {
                melting_point_k: Fixed::from_int(1665),
                fusion_enthalpy_j_per_mol: Fixed::from_int(138_000),
                fusion_volume_cm3_per_mol: Some(Fixed::from_ratio(52, 10)),
            },
            formula: &[("Ca", 1), ("Mg", 1), ("Si", 2), ("O", 6)], // diopside
        },
        Mineral {
            end: Endmember {
                melting_point_k: Fixed::from_int(1830),
                fusion_enthalpy_j_per_mol: Fixed::from_int(133_000),
                fusion_volume_cm3_per_mol: Some(Fixed::from_int(6)),
            },
            formula: &[("Ca", 1), ("Al", 2), ("Si", 2), ("O", 8)], // anorthite
        },
    ]
}

/// The derived basaltic OCEANIC crust: the mantle's first melt, as an element vector. Sums each mineral's
/// formula weighted by its share of the eutectic liquid, so the composition is what the mantle yields on
/// melting, never authored.
fn derived_basalt(minerals: &[Mineral]) -> Vec<(String, Fixed)> {
    let ends: Vec<Endmember> = minerals.iter().map(|m| m.end).collect();
    let (_t, comp) = eutectic_liquid_composition(&ends, Fixed::ZERO).expect("first melt derives");
    let mut acc: std::collections::BTreeMap<String, Fixed> = std::collections::BTreeMap::new();
    for (m, x) in minerals.iter().zip(comp.iter()) {
        for (el, n) in m.formula {
            let add = x.checked_mul(Fixed::from_int(*n)).expect("element count");
            let e = acc.entry((*el).to_string()).or_insert(Fixed::ZERO);
            *e = e.checked_add(add).expect("element sum");
        }
    }
    acc.into_iter().collect()
}

/// The labelled continental felsic crust (silica), retiring when the differentiation chain derives it. Light,
/// so it floats high as the standing masses (the `planet_relief` stand-in).
fn silica() -> Vec<(String, Fixed)> {
    vec![
        ("Si".to_string(), Fixed::from_int(1)),
        ("O".to_string(), Fixed::from_int(2)),
    ]
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Continent, // light felsic (labelled)
    Ocean,     // derived basalt
}
impl Kind {
    fn from_plate(plate: usize) -> Self {
        // A third of plates are continental (contingent layout, seeded like nature's plate map).
        if hash32(plate as u32 ^ 0xA11CE) % 100 < 34 {
            Kind::Continent
        } else {
            Kind::Ocean
        }
    }
    fn is_light(self) -> bool {
        matches!(self, Kind::Continent)
    }
}

fn dist2(tx: usize, ty: usize, sx: usize, sy: usize) -> i64 {
    let raw = (tx as i64 - sx as i64).abs();
    let dx = raw.min(WIDTH as i64 - raw);
    let dy = ty as i64 - sy as i64;
    dx * dx + dy * dy
}
fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut r = (v as f64).sqrt() as i64;
    while r * r > v {
        r -= 1;
    }
    while (r + 1) * (r + 1) <= v {
        r += 1;
    }
    r
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
fn ramp(stops: &[(f64, (u8, u8, u8))], t: f64) -> (u8, u8, u8) {
    for w in stops.windows(2) {
        let (p0, c0) = w[0];
        let (p1, c1) = w[1];
        if t <= p1 {
            let u = ((t - p0) / (p1 - p0).max(1e-9)).clamp(0.0, 1.0);
            return (
                lerp(c0.0 as f64, c1.0 as f64, u) as u8,
                lerp(c0.1 as f64, c1.1 as f64, u) as u8,
                lerp(c0.2 as f64, c1.2 as f64, u) as u8,
            );
        }
    }
    stops[stops.len() - 1].1
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dump = args.iter().any(|a| a == "dump");
    let wet = !args.iter().any(|a| a == "dry");
    let delay_ms: u64 = args
        .iter()
        .find_map(|a| a.parse::<u64>().ok())
        .unwrap_or(650);

    let registry = PhaseRegistry::standard().expect("phase registry");
    let table = PeriodicTable::standard().expect("periodic table");
    let t_surface = Fixed::from_int(300);
    let p_surface = Fixed::from_int(1);
    let mantle_density = Fixed::from_ratio(33, 10);
    let sea_level = Fixed::from_int(1500);

    // --- Derived crust compositions and densities ---
    let minerals = lherzolite();
    // The DERIVED first-melt composition (a basalt), computed for display: the crust IS this melt. Pricing its
    // density needs the Ca-Al aluminosilicate phases (plagioclase, pyroxene) in the density registry, which
    // currently holds only Si/O/Mg/Fe phases, so that is a flagged floor extension. The oceanic lithosphere's
    // buoyancy is dominated by the depleted olivine RESIDUE the melting leaves behind (registry-priceable), so
    // the isostasy floats that residue; a lighter basalt veneer sits on it.
    let basalt = derived_basalt(&minerals);
    let residue = vec![
        ("Mg".to_string(), Fixed::from_int(2)),
        ("Si".to_string(), Fixed::from_int(1)),
        ("O".to_string(), Fixed::from_int(4)),
    ]; // forsterite: the olivine-enriched harzburgite residue of partial melting
    let ocean_density = crustal_density(&residue, t_surface, p_surface, &registry, &table)
        .expect("residue density derives");
    let cont_density = crustal_density(&silica(), t_surface, p_surface, &registry, &table)
        .expect("silica density derives");

    // --- Derived mantle solidus and melt-column parameters ---
    let ends: Vec<Endmember> = minerals.iter().map(|m| m.end).collect();
    let sol0 = multicomponent_solidus(&ends, Fixed::ZERO).expect("solidus");
    let sol1 = multicomponent_solidus(&ends, Fixed::from_int(10_000)).expect("solidus 1 GPa");
    let sol_slope = sol1 - sol0;
    let adiabat_slope = Fixed::from_ratio(155, 10);
    let productivity = Fixed::from_ratio(12, 100);
    let source_density = Fixed::from_int(3300);
    let gravity = Fixed::from_ratio(98, 10);
    // Water-flux depression of the arc solidus (a modest labelled colligative drop; the direction is the point).
    let arc_solidus = sol0 - Fixed::from_int(25);

    let melt = |tp: Fixed, surface: Fixed| {
        adiabatic_melt_column(
            tp,
            surface,
            sol_slope,
            adiabat_slope,
            productivity,
            source_density,
            gravity,
        )
    };

    // --- Plate field (seeded contingent layout) and derived static elevation ---
    let seeds: Vec<(usize, usize)> = (0..PLATES)
        .map(|i| {
            (
                (hash32(i as u32 * 2) as usize) % WIDTH,
                (hash32(i as u32 * 2 + 1) as usize) % HEIGHT,
            )
        })
        .collect();
    let hotspots: Vec<(usize, usize)> = (0..HOTSPOTS)
        .map(|i| {
            (
                (hash32(i as u32 * 7 + 101) as usize) % WIDTH,
                (hash32(i as u32 * 7 + 103) as usize) % HEIGHT,
            )
        })
        .collect();

    // Per-tile: nearest plate, its kind, the boundary gap, and the volcanic setting (0 none,1 ridge,2 arc,3 plume).
    let mut kind_of = vec![Kind::Ocean; WIDTH * HEIGHT];
    let mut elevation = vec![0.0f64; WIDTH * HEIGHT];
    let mut relief_sea = vec![false; WIDTH * HEIGHT];
    let mut setting = vec![0u8; WIDTH * HEIGHT];
    let mut emin = f64::MAX;
    let mut emax = f64::MIN;
    for ty in 0..HEIGHT {
        for tx in 0..WIDTH {
            let (mut d1, mut d2) = (i64::MAX, i64::MAX);
            let (mut p1, mut p2) = (0usize, 0usize);
            for (i, &(sx, sy)) in seeds.iter().enumerate() {
                let d = dist2(tx, ty, sx, sy);
                if d < d1 {
                    d2 = d1;
                    p2 = p1;
                    d1 = d;
                    p1 = i;
                } else if d < d2 {
                    d2 = d;
                    p2 = i;
                }
            }
            let kind = Kind::from_plate(p1);
            let neighbour = Kind::from_plate(p2);
            let gap = isqrt(d2) - isqrt(d1);
            let idx = ty * WIDTH + tx;
            kind_of[idx] = kind;

            // Derived elevation: Airy flotation of the derived density, with a tectonic thickness stand-in
            // (thick stable continents, thinner ocean floor subsiding with age away from the ridge).
            let g = gap.min(22) as i32;
            let (density, thick_m) = match kind {
                Kind::Continent => {
                    let t = if neighbour.is_light() && gap < 5 {
                        42_000 + (5 - gap as i32) * 1_300
                    } else {
                        22_000 + g * 900
                    };
                    (cont_density, t)
                }
                Kind::Ocean => (ocean_density, 8_000 - gap.min(16) as i32 * 95),
            };
            let e = airy_isostatic_elevation(density, mantle_density, Fixed::from_int(thick_m))
                .expect("isostasy")
                .to_f64_lossy();
            elevation[idx] = e;
            emin = emin.min(e);
            emax = emax.max(e);
            relief_sea[idx] = e < sea_level.to_f64_lossy();

            // Volcanic setting from the plate geometry.
            let on_hotspot = hotspots.iter().any(|&(hx, hy)| dist2(tx, ty, hx, hy) <= 2);
            setting[idx] = if on_hotspot {
                3 // plume
            } else if gap <= 2 && p1 != p2 {
                if !kind.is_light() && !neighbour.is_light() {
                    1 // ridge (ocean-ocean divergence)
                } else if kind.is_light() != neighbour.is_light() {
                    2 // arc (subduction)
                } else {
                    0 // continent-continent collision, no melt
                }
            } else {
                0
            };
        }
    }
    let espan = (emax - emin).max(1.0);

    // Terrain hypsometric ramps.
    let sea_ramp = [
        (0.0, (7u8, 17, 34)),
        (0.55, (17, 52, 99)),
        (0.82, (31, 96, 138)),
        (1.0, (64, 140, 150)),
    ];
    let land_ramp = [
        (0.0, (47u8, 86, 52)),
        (0.4, (95, 120, 66)),
        (0.65, (140, 116, 70)),
        (0.85, (176, 150, 112)),
        (1.0, (233, 228, 214)),
    ];
    let sea_lo = sea_level.to_f64_lossy() - emin;

    // The deep-time interior temperature curve: a hot Archean cooling to the present (a labelled driver; the
    // melt response below is derived). Linear from tp_hot to tp_now over the eras.
    let tp_hot = 1780.0;
    let tp_now = 1600.0;

    let mut out = std::io::stdout().lock();
    for era in 0..ERAS {
        let frac = era as f64 / (ERAS - 1) as f64;
        let tp = tp_hot + (tp_now - tp_hot) * frac; // K, falling
        let tp_fx = Fixed::from_int(tp as i32);
        let tp_plume = Fixed::from_int((tp + 150.0) as i32); // plumes run hotter than the ambient

        // Derived melt columns at this era's temperature.
        let ridge = melt(tp_fx, sol0).expect("ridge");
        let plume = melt(tp_plume, sol0).expect("plume");
        let arc = melt(tp_fx, arc_solidus).expect("arc");
        let inten_of = |s: u8| -> f64 {
            match s {
                1 => ridge.max_melt_fraction.to_f64_lossy(),
                3 => plume.max_melt_fraction.to_f64_lossy(),
                2 => {
                    if wet && arc.crust_thickness_km > ridge.crust_thickness_km {
                        arc.max_melt_fraction.to_f64_lossy()
                    } else {
                        0.0
                    }
                }
                _ => 0.0,
            }
        };

        // Build the frame.
        let mut vents = 0usize;
        let mut frame = String::with_capacity(WIDTH * HEIGHT * 20);
        if !dump {
            frame.push_str("\x1b[H"); // cursor home (overwrite in place)
        }
        // Two map rows per text line (upper half-block).
        for ty in (0..HEIGHT).step_by(2) {
            for tx in 0..WIDTH {
                let mut cells = [(0u8, 0u8, 0u8); 2];
                for (half, cell) in cells.iter_mut().enumerate() {
                    let idx = (ty + half) * WIDTH + tx;
                    // terrain color
                    let e = elevation[idx];
                    let mut c = if relief_sea[idx] {
                        let d = sea_level.to_f64_lossy() - e;
                        ramp(&sea_ramp, 1.0 - (d / sea_lo.max(1.0)).clamp(0.0, 1.0))
                    } else {
                        let h = e - sea_level.to_f64_lossy();
                        ramp(
                            &land_ramp,
                            (h / (emax - sea_level.to_f64_lossy()).max(1.0)).clamp(0.0, 1.0),
                        )
                    };
                    let _ = espan;
                    // volcanic glow
                    let s = setting[idx];
                    let inten = inten_of(s);
                    if s != 0 && inten > 0.0 {
                        if half == 0 {
                            vents += 1;
                        }
                        let vcol = match s {
                            1 => (55.0, 211.0, 194.0),
                            2 => (255.0, 90.0, 60.0),
                            _ => (193.0, 99.0, 255.0),
                        };
                        let g = (0.35 + 0.55 * inten).clamp(0.0, 0.95);
                        c = (
                            lerp(c.0 as f64, vcol.0, g) as u8,
                            lerp(c.1 as f64, vcol.1, g) as u8,
                            lerp(c.2 as f64, vcol.2, g) as u8,
                        );
                    }
                    *cell = c;
                }
                let (t, b) = (cells[0], cells[1]);
                frame.push_str(&format!(
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m\u{2580}",
                    t.0, t.1, t.2, b.0, b.1, b.2
                ));
            }
            frame.push_str("\x1b[0m\n");
        }
        // Status line.
        frame.push_str(&format!(
            "\x1b[0m era {}/{}  interior T_p {:.0} K   ridge {:.1} km  plume F {:.2}  {} active vents   [{}]\x1b[K\n",
            era + 1,
            ERAS,
            tp,
            ridge.crust_thickness_km.to_f64_lossy(),
            plume.max_melt_fraction.to_f64_lossy(),
            vents,
            if wet { "wet: ridge+arc+plume" } else { "dry: no arc" },
        ));
        let _ = vents;
        let _ = write!(out, "{frame}");
        let _ = out.flush();

        if !dump {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
    }
    // The derived first-melt (basalt) composition, shown as the crust the mantle yields on melting.
    let basalt_str: Vec<String> = basalt
        .iter()
        .map(|(el, n)| format!("{el}{:.2}", n.to_f64_lossy()))
        .collect();
    let _ = writeln!(
        out,
        "derived first melt (the crust): {} | oceanic lithosphere = olivine residue {:.2} g/cm^3, continent = felsic (labelled) {:.2} | solidus {:.0} K",
        basalt_str.join(" "),
        ocean_density.to_f64_lossy(),
        cont_density.to_f64_lossy(),
        sol0.to_f64_lossy()
    );
}
