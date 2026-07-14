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

//! IN-SIMULATION VOLCANISM on the derived planet map: the melt phase (`civsim_physics::melting`) wired to the
//! plate map so the volcanic provinces emerge from the geodynamics rather than being painted. The geology
//! (crustal density, elevation, relief) derives from composition exactly as `planet_relief` proves; this
//! adds the melting.
//!
//! What DERIVES, end to end, from the mantle mineral signatures and the interior temperature:
//! - The mantle peridotite SOLIDUS is the multi-saturation point of the four lherzolite minerals (olivine,
//!   orthopyroxene, clinopyroxene, plagioclase), `civsim_physics::melting::multicomponent_solidus`, not an
//!   authored curve; its pressure dependence is the Clapeyron shift.
//! - Where mantle rises and crosses that solidus it MELTS, and the melt pools into crust, the adiabatic
//!   decompression column `civsim_physics::melting::adiabatic_melt_column`. The crust thickness and the peak
//!   melt fraction are outputs of the column, read as the volcanic productivity.
//! - The three volcanic settings emerge from the plate geometry, not a tag: a DIVERGENT boundary between two
//!   dense (oceanic) plates is a spreading RIDGE (decompression melting of upwelling mantle); a CONVERGENT
//!   boundary between a light and a dense plate is a subduction ARC; a hot interior anomaly is a PLUME
//!   (intraplate). Arc volcanism is FLUX melting: subducted water lowers the wedge solidus (the cryoscopic
//!   depression, the dilute limit of the same liquidus law), so on a DRY world there is no arc volcanism at
//!   all, a derived exclusivity. A hotter mantle melts more (the Archean komatiite regime), so the plume and
//!   ridge productivity rise with the interior temperature unprompted.
//!
//! The remaining labelled stand-ins, retiring when their chains derive: the per-plate composition and the
//! plate scatter (accretion and differentiation will derive them, as `planet_relief` notes), the interior
//! potential temperature and its plume anomalies (the 3D convection field will derive them), and the mantle
//! column parameters (adiabat gradient, productivity, densities), which are the mantle floor read here as
//! caller inputs. The MELTING is not a stand-in: it is the owner's derived melt phase, run on the map. The
//! honest grade is ideal-solution (the solidus lands about 150 K high but its slope shallower, the two errors
//! partly cancelling, so the ridge crust is a sane few kilometres), the rung-1 Margules calibration the next
//! rung. Fully deterministic (a fixed hash, no RNG), so the map replays byte for byte.
//!
//! Run: `cargo run -p civsim-sim --example planet_volcanism -- wet` (or `dry`). It writes the derived
//! elevation, province, relief, and volcanism grids plus a vent list to stdout for the renderer.

use civsim_core::Fixed;
use civsim_physics::geodynamics::airy_isostatic_elevation;
use civsim_physics::melting::{adiabatic_melt_column, multicomponent_solidus, Endmember};
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::petrology::crustal_density;
use civsim_physics::petrology_data::PhaseRegistry;
use civsim_units::bignum::BigRat;
use civsim_world::terrain::{classify_relief, relief_datum, TerrainRelief};

const WIDTH: usize = 220;
const HEIGHT: usize = 110;
const PLATES: usize = 18;
const HOTSPOTS: usize = 7;
const SEED: u32 = 0x5EED_1234;

/// A deterministic integer finalizer (Murmur3-style), the reproducible stand-in for the plate scatter,
/// province assignment, and hotspot placement. A pure function of its input, so the whole field replays.
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

/// The molar gas constant `R = N_A * k_B` (J/mol/K), derived from the two CODATA fundamentals, for the
/// cryoscopic (water-flux) solidus depression. The same derivation the melting module uses internally.
fn molar_gas_constant() -> Fixed {
    let n_a = BigRat::from_decimal_str(
        civsim_units::fundamentals::fundamental("N_A")
            .expect("Avogadro")
            .value,
    )
    .expect("N_A parses");
    let k_b = BigRat::from_decimal_str(
        civsim_units::fundamentals::fundamental("k_B")
            .expect("Boltzmann")
            .value,
    )
    .expect("k_B parses");
    Fixed::from_bits_i128(
        n_a.mul(&k_b)
            .round_to_scale(Fixed::FRAC_BITS)
            .expect("R fits Q32.32"),
    )
    .expect("R projects")
}

/// The four crust types, named by COMPOSITION not by any Earth tectonic setting (identical to `planet_relief`).
/// Silica is the light continental crust that floats high; the olivines and iron oxide are the dense oceanic
/// crust that founders. The dense/light split is what drives the tectonic setting at a boundary.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Crust {
    Silica,
    Olivine,
    FerroanOlivine,
    IronOxide,
}

impl Crust {
    fn from_plate(plate: usize) -> Self {
        match hash32(plate as u32 ^ 0xA11CE) % 100 {
            0..=33 => Crust::Silica,
            34..=66 => Crust::Olivine,
            67..=90 => Crust::FerroanOlivine,
            _ => Crust::IronOxide,
        }
    }
    fn is_light(self) -> bool {
        matches!(self, Crust::Silica)
    }
    fn code(self) -> u8 {
        match self {
            Crust::Silica => 0,
            Crust::Olivine => 1,
            Crust::FerroanOlivine => 2,
            Crust::IronOxide => 3,
        }
    }
    fn composition(self) -> Vec<(String, Fixed)> {
        match self {
            Crust::Silica => vec![
                ("Si".to_string(), Fixed::from_int(1)),
                ("O".to_string(), Fixed::from_int(2)),
            ],
            Crust::Olivine => vec![
                ("Mg".to_string(), Fixed::from_int(2)),
                ("Si".to_string(), Fixed::from_int(1)),
                ("O".to_string(), Fixed::from_int(4)),
            ],
            Crust::FerroanOlivine => vec![
                ("Mg".to_string(), Fixed::from_int(1)),
                ("Fe".to_string(), Fixed::from_int(1)),
                ("Si".to_string(), Fixed::from_int(1)),
                ("O".to_string(), Fixed::from_int(4)),
            ],
            Crust::IronOxide => vec![
                ("Fe".to_string(), Fixed::from_int(2)),
                ("O".to_string(), Fixed::from_int(3)),
            ],
        }
    }
}

/// The volcanic setting at a tile, emergent from the plate geometry and compositions.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Volcanism {
    None,
    Ridge,
    Arc,
    Plume,
}

impl Volcanism {
    fn code(self) -> u8 {
        match self {
            Volcanism::None => 0,
            Volcanism::Ridge => 1,
            Volcanism::Arc => 2,
            Volcanism::Plume => 3,
        }
    }
}

/// The four lherzolite mantle mineral signatures `(T_m K, dH_fus J/mol, dV_fus cm^3/mol)`; measured validation
/// inputs, not floor rows. The mantle solidus is their multi-saturation point.
fn lherzolite() -> [Endmember; 4] {
    [
        Endmember {
            melting_point_k: Fixed::from_int(2163),
            fusion_enthalpy_j_per_mol: Fixed::from_int(114_000),
            fusion_volume_cm3_per_mol: Fixed::from_ratio(39, 10),
        }, // forsterite
        Endmember {
            melting_point_k: Fixed::from_int(1830),
            fusion_enthalpy_j_per_mol: Fixed::from_int(73_000),
            fusion_volume_cm3_per_mol: Fixed::from_int(5),
        }, // enstatite
        Endmember {
            melting_point_k: Fixed::from_int(1665),
            fusion_enthalpy_j_per_mol: Fixed::from_int(138_000),
            fusion_volume_cm3_per_mol: Fixed::from_ratio(52, 10),
        }, // diopside
        Endmember {
            melting_point_k: Fixed::from_int(1830),
            fusion_enthalpy_j_per_mol: Fixed::from_int(133_000),
            fusion_volume_cm3_per_mol: Fixed::from_int(6),
        }, // anorthite
    ]
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

fn main() {
    // The one command-line toggle: a WET world (subducted water fluxes the arc) or a DRY one (no arc
    // volcanism, the derived exclusivity). Default wet.
    let wet = !std::env::args().any(|a| a == "dry");

    let registry = PhaseRegistry::standard().expect("phase registry loads");
    let table = PeriodicTable::standard().expect("periodic table loads");
    let temperature_k = Fixed::from_int(300);
    let pressure_bar = Fixed::from_int(1);
    let mantle_density = Fixed::from_ratio(33, 10); // 3.3 g/cm^3, the interior fixture.
    let sea_level = Fixed::from_int(1500); // metres, the ocean/land datum fixture.

    // --- The derived mantle solidus and the melt-column mantle parameters (labelled interior inputs) ---
    let mantle = lherzolite();
    let sol0 = multicomponent_solidus(&mantle, Fixed::ZERO).expect("mantle solidus derives");
    let sol1 =
        multicomponent_solidus(&mantle, Fixed::from_int(10_000)).expect("mantle solidus at 1 GPa");
    let sol_slope = sol1 - sol0; // K/GPa, since 10000 bar = 1 GPa
    let adiabat_slope = Fixed::from_ratio(155, 10); // 15.5 K/GPa
    let productivity = Fixed::from_ratio(12, 100); // 0.12 /GPa
    let source_density = Fixed::from_int(3300);
    let gravity = Fixed::from_ratio(98, 10);
    let tp_normal = Fixed::from_int(1600); // K, the interior potential temperature (labelled)
    let tp_plume = Fixed::from_int(1750); // K, a hot plume anomaly (labelled)

    // The cryoscopic water-flux depression of the arc solidus: dT = (R * T_sol^2 / dH_bar) * x_water, the
    // dilute colligative limit of the liquidus law. The DIRECTION is derived (water lowers the solidus, so
    // arc melting exists only where there is water); the magnitude underestimates real water flux (water is
    // strongly non-ideal, the rung-1 target), so it is a modest but honest depression.
    let r = molar_gas_constant();
    let dh_bar = Fixed::from_int(114_500); // mean fusion enthalpy of the assemblage
    let x_water = Fixed::from_ratio(15, 100); // fluxed-wedge water fraction (labelled)
    let arc_depression = r
        .checked_mul(sol0)
        .and_then(|v| v.checked_mul(sol0))
        .and_then(|v| v.checked_div(dh_bar))
        .and_then(|v| v.checked_mul(x_water))
        .expect("cryoscopic depression");
    let arc_solidus = sol0 - arc_depression;

    // The melt-column productivity at a given interior temperature and solidus: the peak melt fraction (a
    // bounded 0..1 intensity) and the crust thickness (km). A sub-solidus mantle returns a zero column.
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
    let ridge_col = melt(tp_normal, sol0).expect("ridge column");
    let plume_col = melt(tp_plume, sol0).expect("plume column");
    let arc_col = melt(tp_normal, arc_solidus).expect("arc column");
    let dry_ref = ridge_col; // the dry reference the arc melt is measured against

    // Plate seeds and hotspot seeds, scattered deterministically.
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

    // Memoize the derived crust density per type.
    let densities: Vec<Fixed> = [
        Crust::Silica,
        Crust::Olivine,
        Crust::FerroanOlivine,
        Crust::IronOxide,
    ]
    .iter()
    .map(|c| {
        crustal_density(
            &c.composition(),
            temperature_k,
            pressure_bar,
            &registry,
            &table,
        )
        .expect("crust composition resolves")
    })
    .collect();
    let density_of = |c: Crust| densities[c.code() as usize];

    let mut elevations: Vec<Fixed> = Vec::with_capacity(WIDTH * HEIGHT);
    let mut provinces: Vec<u8> = Vec::with_capacity(WIDTH * HEIGHT);
    let mut volc_type: Vec<u8> = vec![0; WIDTH * HEIGHT];
    let mut volc_int: Vec<u8> = vec![0; WIDTH * HEIGHT];
    let mut vents: Vec<(usize, usize, u8, u8, f64)> = Vec::new(); // x, y, type, intensity 0..99, crust km

    for ty in 0..HEIGHT {
        for tx in 0..WIDTH {
            // Nearest and second-nearest plate.
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
            let crust = Crust::from_plate(p1);
            let neighbour = Crust::from_plate(p2);
            let gap = isqrt(d2) - isqrt(d1);

            // --- The derived elevation (identical chain to planet_relief) ---
            let g = gap.min(26) as i32;
            let thickness_m: i32 = match crust {
                Crust::Silica => {
                    if neighbour.is_light() && gap < 5 {
                        46_000 + (5 - gap as i32) * 1_400
                    } else {
                        20_000 + g * 1_000
                    }
                }
                Crust::Olivine => 8_000 - gap.min(16) as i32 * 90,
                Crust::FerroanOlivine => 7_000 - gap.min(16) as i32 * 90,
                Crust::IronOxide => 6_000,
            };
            let elevation = airy_isostatic_elevation(
                density_of(crust),
                mantle_density,
                Fixed::from_int(thickness_m),
            )
            .expect("isostasy resolves");
            elevations.push(elevation);
            provinces.push(crust.code());

            // --- The emergent volcanic setting ---
            let idx = ty * WIDTH + tx;
            // A hotspot anywhere: an intraplate plume (a hot interior column melts more).
            let on_hotspot = hotspots.iter().any(|&(hx, hy)| dist2(tx, ty, hx, hy) <= 3);
            let setting = if on_hotspot {
                Volcanism::Plume
            } else if gap <= 2 && p1 != p2 {
                // A plate boundary: the setting is read from the two crusts.
                if !crust.is_light() && !neighbour.is_light() {
                    Volcanism::Ridge // two dense (oceanic) plates diverge: a spreading ridge
                } else if crust.is_light() != neighbour.is_light() {
                    Volcanism::Arc // a dense plate subducts under a light one: a subduction arc
                } else {
                    Volcanism::None // two light plates collide: a mountain belt, no melting
                }
            } else {
                Volcanism::None
            };

            // The derived intensity (bounded melt fraction) and crust production per setting.
            let (intensity_f, crust_km) = match setting {
                Volcanism::Ridge => (ridge_col.max_melt_fraction, ridge_col.crust_thickness_km),
                Volcanism::Plume => (plume_col.max_melt_fraction, plume_col.crust_thickness_km),
                Volcanism::Arc => {
                    if wet {
                        // The flux-added melt above the dry reference: the water's contribution.
                        let extra = arc_col.crust_thickness_km - dry_ref.crust_thickness_km;
                        (arc_col.max_melt_fraction, extra.max(Fixed::ZERO))
                    } else {
                        (Fixed::ZERO, Fixed::ZERO) // no water, no arc melt: the derived exclusivity
                    }
                }
                Volcanism::None => (Fixed::ZERO, Fixed::ZERO),
            };

            let active = setting != Volcanism::None && intensity_f > Fixed::ZERO;
            if active {
                volc_type[idx] = setting.code();
                let scaled = (intensity_f.to_f64_lossy() * 99.0).clamp(0.0, 99.0) as u8;
                volc_int[idx] = scaled;
                // Record the strongest tile of each local vent as a discrete glyph, thinned to a grid so the
                // renderer draws cones rather than a solid band.
                if tx % 4 == 0 && ty % 4 == 0 {
                    vents.push((tx, ty, setting.code(), scaled, crust_km.to_f64_lossy()));
                }
            }
        }
    }

    let datum = relief_datum(&elevations).expect("field datum");
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for e in &elevations {
        let v = e.to_f64_lossy();
        lo = lo.min(v);
        hi = hi.max(v);
    }

    // --- Emit: header, then the grids and the vent list ---
    println!(
        "PLANET {WIDTH} {HEIGHT} datum={:.1} sea_level={:.1} min={:.1} max={:.1} world={}",
        datum.to_f64_lossy(),
        sea_level.to_f64_lossy(),
        lo,
        hi,
        if wet { "wet" } else { "dry" }
    );
    println!(
        "MELT solidus_surface={:.1} solidus_slope={:.1} tp_normal={:.0} tp_plume={:.0} arc_solidus={:.1} ridge_km={:.2} ridge_f={:.3} plume_km={:.2} plume_f={:.3} arc_extra_km={:.2}",
        sol0.to_f64_lossy(),
        sol_slope.to_f64_lossy(),
        tp_normal.to_f64_lossy(),
        tp_plume.to_f64_lossy(),
        arc_solidus.to_f64_lossy(),
        ridge_col.crust_thickness_km.to_f64_lossy(),
        ridge_col.max_melt_fraction.to_f64_lossy(),
        plume_col.crust_thickness_km.to_f64_lossy(),
        plume_col.max_melt_fraction.to_f64_lossy(),
        (arc_col.crust_thickness_km - dry_ref.crust_thickness_km).to_f64_lossy(),
    );
    let emit_grid = |name: &str, f: &dyn Fn(usize) -> String| {
        println!("{name}");
        for ty in 0..HEIGHT {
            let row: Vec<String> = (0..WIDTH).map(|tx| f(ty * WIDTH + tx)).collect();
            println!("{}", row.join(" "));
        }
    };
    emit_grid("ELEV", &|i| format!("{:.0}", elevations[i].to_f64_lossy()));
    emit_grid("PROVINCE", &|i| provinces[i].to_string());
    emit_grid("RELIEF", &|i| {
        match classify_relief(elevations[i], sea_level, datum) {
            TerrainRelief::Submarine => "0",
            TerrainRelief::Lowland => "1",
            TerrainRelief::Upland => "2",
        }
        .to_string()
    });
    emit_grid("VOLC", &|i| volc_type[i].to_string());
    emit_grid("VOLCINT", &|i| volc_int[i].to_string());
    println!("VENTS {}", vents.len());
    for (x, y, t, inten, km) in &vents {
        println!("{x} {y} {t} {inten} {km:.2}");
    }
}
