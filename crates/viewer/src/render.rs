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

//! The superfine render: the zoom level where a tile is large enough to show the individual
//! organisms standing on it (design Parts 14, 1; R-VIEW-ELAB done as a pure read of canon).
//!
//! At the overview levels the viewer draws the biome quadtree; here, once the camera has
//! zoomed past per-tile, each tile is painted as a block of its biome colour with the located
//! organisms drawn on it as marks, coloured by trophic layer (plants green, herbivores amber,
//! carnivores red) and individualised per species. This reads the [`LivingWorld`]'s tile map
//! and located occupants and never writes them, so the superfine view is an observer of the
//! world, not an author of it (Principle 10).

use civsim_core::{splitmix64, Fixed};
use civsim_materials::band_gap::conduction_class_from_column;
use civsim_materials::optics::{
    feature_response_at, marcus_hush_width_ev, optical_energies, thermal_broadening_width_ev,
    OpticalFeature,
};
use civsim_physics::band_gap::BandGapColumn;
use civsim_physics::crystal_field::{cm_to_ev, CrystalFieldTables};
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::polarizability::element_electronic_polarizability_a0_cubed;
use civsim_sim::deeptime::CraterRow;
use civsim_sim::genesis::LivingWorld;
use civsim_sim::geodynamics::DerivedTile;
use civsim_world::terrain::TerrainRelief;
use civsim_world::{BiomeSet, Coord3, Rgb, TopologySpace};
#[cfg(feature = "gpu")]
use rayon::prelude::*;

/// The colour of an organism mark: a base hue by trophic layer, jittered per species so each
/// kind is a distinct individual form. Presentation only, never canonical state.
pub fn organism_color(layer: u16, species_id: u32) -> Rgb {
    let (br, bg, bb) = match layer {
        0 => (46, 176, 74),  // producers: the plants, green
        1 => (214, 176, 58), // first consumers: herbivores, amber
        _ => (206, 74, 58),  // higher consumers: carnivores, red
    };
    let h = splitmix64(species_id as u64 ^ 0x9E37_79B9_7F4A_7C15);
    let jitter = |base: i32, shift: u32| -> u8 {
        let d = ((h >> shift) & 0x3f) as i32 - 32; // -32..31
        (base + d).clamp(0, 255) as u8
    };
    Rgb::new(jitter(br, 0), jitter(bg, 8), jitter(bb, 16))
}

/// The approximate DISPLAY colour of a blackbody at effective temperature `t_eff_k` (kelvin): the observability
/// non-canon projection of a star's DERIVED `T_eff` (from [`civsim_sim::astro::stellar_effective_temperature`])
/// onto a screen colour. A cool ~3000 K star reads orange-red, the Sun (~5772 K) a warm near-white, a hot
/// ~10000 K star blue-white, tracking the Planckian locus. The mapping is the piecewise fit of Tanner Helland
/// ("How to Convert Temperature (K) to RGB", 2012), itself a regression to Mitchell Charity's blackbody-colour
/// datafile (`bbr_color.txt`, computed from the CIE 1931 colour-matching functions). Display-only: it reads a
/// derived scalar and returns pixels, writes no canonical state (Principle 10), and uses `f64` because a screen
/// colour needs no fixed-point rigour past per-run determinism.
pub fn blackbody_rgb(t_eff_k: Fixed) -> Rgb {
    // The fit is defined on temperature/100, valid roughly 1000..40000 K; clamp into that band so a derived T_eff
    // past the fit returns its nearest sensible colour rather than a wild extrapolation.
    let temp = (t_eff_k.to_f64_lossy() / 100.0).clamp(10.0, 400.0);
    let clamp255 = |v: f64| v.clamp(0.0, 255.0) as u8;
    let red = if temp <= 66.0 {
        255.0
    } else {
        329.698_727_446 * (temp - 60.0).powf(-0.133_204_759_2)
    };
    let green = if temp <= 66.0 {
        99.470_802_586_1 * temp.ln() - 161.119_568_166_1
    } else {
        288.122_169_528_3 * (temp - 60.0).powf(-0.075_514_849_2)
    };
    let blue = if temp >= 66.0 {
        255.0
    } else if temp <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (temp - 10.0).ln() - 305.044_792_730_7
    };
    Rgb::new(clamp255(red), clamp255(green), clamp255(blue))
}

/// The observability-non-canon DISPLAY colour of a daytime sky, DERIVED from the atmosphere's molecular
/// polarizability by Rayleigh scattering: the scattered-light colour of a star of effective temperature
/// `star_t_eff_k` filtered through an atmosphere of `gas_mix` (each entry a chemical formula and its mole
/// fraction, for example `[("N2", 0.78), ("O2", 0.21), ("Ar", 0.01)]` for modern Earth air). Short wavelengths
/// scatter more strongly (the Rayleigh cross-section goes as `alpha^2 / lambda^4`), so a thin N2/O2 atmosphere
/// paints a blue sky; a more polarizable, denser CO2 atmosphere drives the short bands toward saturation and
/// desaturates the sky, the qualitative Hadean/Venusian shift.
///
/// The derivation chain, every physical step DERIVED from the banked polarizability substrate, never authored
/// RGB: a formula string parses to `(element, count)` pairs (a general parser, no hardcoded gas list) ->
/// each atom's static electronic polarizability comes from
/// [`civsim_physics::polarizability::element_electronic_polarizability_a0_cubed`] (the cited-ionization-energy
/// Unsold single-oscillator estimate, in Bohr-volume units `a_0^3`) -> the molecular polarizability is their
/// additive sum `alpha_mol = sum(count * alpha_atom)` -> each band's Rayleigh weight is
/// `w(lambda) = sum_gas moleFraction * alpha_mol^2 / lambda^4` -> the scattered sky per band is the star's
/// Planck spectral radiance times the Rayleigh transmittance `1 - exp(-tau(lambda))` -> the three bands are
/// normalized so the brightest is 255, preserving hue. The absolute Rayleigh prefactor `128 pi^5 / 3` and the
/// unit conversions cancel from that normalized ratio, so only the RELATIVE `alpha^2 / lambda^4` across bands
/// and gases matters.
///
/// Admit-the-alien: the mix is keyed on the formula string and per-element cited data, so a new gas is a data
/// row (a formula), never a code change. An element the polarizability substrate cannot resolve (for example a
/// transition metal with no main-group valence count) sinks its gas (fail-soft, no guessed value); if the whole
/// mix resolves to nothing, this returns `None` so the caller falls back to no atmosphere tint.
///
/// VALIDITY CEILING: this is FACTOR-GRADE and QUALITATIVE, not a calibrated radiance. It distinguishes a blue
/// N2/O2 sky from a desaturated CO2 sky, no finer. Its cited top rung is the ionization energy (the
/// polarizability's input); the Unsold estimate runs ~11% low, and the additivity approximation for molecular
/// `alpha` is itself factor-grade. This is acceptable ONLY because the render is observability-non-canon:
/// display-only, one-way (canon physics -> pixels), with zero effect on simulation state (Principle 10). The
/// model carries no absorption and a wavelength-independent polarizability, so it spans blue (thin) to
/// desaturated near-white (thick) and cannot render a true red/butterscotch sky; "less blue" is the honest
/// limit of the qualitative shift. `f64` throughout, mirroring [`blackbody_rgb`]: a screen colour needs no
/// fixed-point rigour past per-run determinism.
///
/// The observability-layer display choices this non-canon render is allowed (like [`blackbody_rgb`]'s fit): the
/// three sampling wavelengths (`BANDS_NM`, the R/G/B band centres, definitional) and one opacity scale
/// (`DISPLAY_OPACITY_UNIT`, documented at its definition). Every other quantity is derived or a cited fundamental.
pub fn rayleigh_sky_rgb(
    gas_mix: &[(&str, f64)],
    star_t_eff_k: Fixed,
    table: &PeriodicTable,
) -> Option<Rgb> {
    // The three display sampling wavelengths in nm: the R, G, B band centres. Definitional display choices for a
    // non-canon render (the observability layer's allowance), the only colours the sky is sampled at.
    const BANDS_NM: [f64; 3] = [630.0, 532.0, 465.0];
    // The reference wavelength for the dimensionless lambda^-4 factor: the mid (green) band. A relative anchor
    // that cancels from the normalized ratio; it only sets what "unit optical depth" is measured against, so the
    // band weights stay order-one rather than the ~1e-8 raw `alpha^2 / lambda_nm^4` would give.
    const LAMBDA_REF_NM: f64 = BANDS_NM[1];
    // The one opacity display knob: the optical depth per `a_0^6` of mole-fraction-weighted molecular
    // polarizability-squared at the reference (green) band. It stands in for the atmospheric column density times
    // the Rayleigh prefactor (128 pi^5 / 3) times the `a_0^3`-to-length unit conversion, none of which the viewer
    // has (the Stage-8 atmospheric column is unwired) and all of which cancel from a normalized band ratio. Its
    // basis: set so modern Earth air (weighted `alpha^2` on the order of a thousand `a_0^6` from the substrate)
    // lands near unit optical depth in the green band, moderate, so the short band is blue-dominant while no band
    // fully saturates. The single definitional display opacity, surfaced not hidden.
    const DISPLAY_OPACITY_UNIT: f64 = 1.0e-3;

    // Per gas: additive molecular polarizability from the formula, then the mole-fraction-weighted `alpha^2` that
    // scales every band's Rayleigh weight. An unresolvable gas contributes nothing (fail-soft, no guess).
    let mut weighted_alpha_sq = 0.0f64;
    for &(formula, mole_fraction) in gas_mix {
        if mole_fraction <= 0.0 {
            continue;
        }
        let Some(alpha_mol) = molecular_polarizability_a0_cubed(formula, table) else {
            continue;
        };
        weighted_alpha_sq += mole_fraction * alpha_mol * alpha_mol;
    }
    if weighted_alpha_sq <= 0.0 {
        return None; // nothing resolved: the caller falls back to no atmosphere tint
    }

    // The Planck exponent constant, DERIVED from the register (never authored): the incident starlight colour is
    // the star's Planck spectrum, and the Rayleigh weighting filters it.
    let c2_m_k = second_radiation_constant_m_k()?;
    let t_eff = star_t_eff_k.to_f64_lossy();
    let mut band = [0.0f64; 3];
    for (i, &lambda) in BANDS_NM.iter().enumerate() {
        let lambda_factor = (LAMBDA_REF_NM / lambda).powi(4); // the dimensionless lambda^-4 Rayleigh weighting
        let tau = DISPLAY_OPACITY_UNIT * weighted_alpha_sq * lambda_factor;
        let scattered = 1.0 - (-tau).exp(); // the Rayleigh single-scatter transmittance factor
        band[i] = planck_relative(lambda, t_eff, c2_m_k) * scattered;
    }

    // Normalize to the brightest band so the hue is preserved and the sky reads at full intensity.
    let max = band[0].max(band[1]).max(band[2]);
    if max <= 0.0 {
        return None;
    }
    let to_u8 = |v: f64| (v / max * 255.0).round().clamp(0.0, 255.0) as u8;
    Some(Rgb::new(to_u8(band[0]), to_u8(band[1]), to_u8(band[2])))
}

/// The additive molecular polarizability (Bohr-volume units `a_0^3`) of a gas from its chemical formula:
/// `alpha_mol = sum over atoms alpha_atom`, the standard atomic-additivity approximation summed over the
/// formula's `(element, count)` pairs. Returns `None` if the formula parses to no atoms or if any element's
/// polarizability is unavailable (fail-soft: an unresolvable element sinks the whole molecule rather than
/// contributing a guessed zero). Keyed on the formula string and per-element cited data, so a new gas is a data
/// row (admit-the-alien), never a code change.
fn molecular_polarizability_a0_cubed(formula: &str, table: &PeriodicTable) -> Option<f64> {
    let atoms = parse_formula(formula);
    if atoms.is_empty() {
        return None;
    }
    let mut alpha = 0.0f64;
    for (symbol, count) in atoms {
        let a = element_electronic_polarizability_a0_cubed(&symbol, table)?.to_f64_lossy();
        alpha += a * count as f64;
    }
    Some(alpha)
}

/// Parse a chemical formula into `(element symbol, count)` pairs by a GENERAL rule (no hardcoded gas list,
/// admit-the-alien): an uppercase ASCII letter opens a symbol, following lowercase ASCII letters continue it,
/// and an optional run of ASCII digits is the count (default 1). "CO2" -> `[("C",1),("O",2)]`, "H2O" ->
/// `[("H",2),("O",1)]`, "CH4" -> `[("C",1),("H",4)]`. A character that opens no symbol (a leading digit,
/// whitespace, punctuation) is skipped, so a malformed fragment yields no atoms for that fragment, never a panic.
fn parse_formula(formula: &str) -> Vec<(String, u32)> {
    let chars: Vec<char> = formula.chars().collect();
    let mut atoms = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_uppercase() {
            let mut symbol = String::new();
            symbol.push(chars[i]);
            i += 1;
            while i < chars.len() && chars[i].is_ascii_lowercase() {
                symbol.push(chars[i]);
                i += 1;
            }
            let mut count: u32 = 0;
            let mut has_digit = false;
            while i < chars.len() && chars[i].is_ascii_digit() {
                count = count
                    .saturating_mul(10)
                    .saturating_add(chars[i].to_digit(10).unwrap_or(0));
                has_digit = true;
                i += 1;
            }
            atoms.push((symbol, if has_digit { count } else { 1 }));
        } else {
            i += 1;
        }
    }
    atoms
}

/// The second radiation constant `c_2 = h c / k_B` in metre-kelvin, DERIVED from the fundamentals register
/// (`civsim_units::fundamentals`), never authored: it sets the Planck exponent `hc / (lambda k T)`. Parsed to
/// `f64` because this is a non-canon display value that needs no fixed-point rigour. `None` on a register miss.
fn second_radiation_constant_m_k() -> Option<f64> {
    let h: f64 = civsim_units::fundamentals::fundamental("h")?
        .value
        .parse()
        .ok()?;
    let c: f64 = civsim_units::fundamentals::fundamental("c")?
        .value
        .parse()
        .ok()?;
    let k_b: f64 = civsim_units::fundamentals::fundamental("k_B")?
        .value
        .parse()
        .ok()?;
    Some(h * c / k_b)
}

/// The star's RELATIVE Planck spectral radiance at wavelength `lambda_nm` and temperature `t_k`: Planck's law
/// `B(lambda, T) proportional to lambda^-5 / (exp(c_2 / (lambda T)) - 1)`, with the second radiation constant
/// `c_2 = h c / k_B` passed in (`c2_m_k`, in metre-kelvin). Only the RATIO across the three bands is used, so the
/// leading `2 h c^2` prefactor drops and the result carries no absolute unit. This is the incident starlight
/// colour the Rayleigh weighting then filters. Falls back to a flat spectrum on a non-physical temperature.
fn planck_relative(lambda_nm: f64, t_k: f64, c2_m_k: f64) -> f64 {
    if t_k <= 0.0 {
        return 1.0;
    }
    let lambda_m = lambda_nm * 1.0e-9;
    let x = c2_m_k / (lambda_m * t_k);
    let denom = x.exp() - 1.0;
    if denom <= 0.0 {
        return 0.0;
    }
    lambda_m.powi(-5) / denom
}

/// A physics-derived terrain colour: the tile's own `elevation`, `moisture`, and `temperature`
/// fields (the physical quantities worldgen computed, each in `[0, 1]`) mapped to colour, so terrain
/// looks like its physics rather than an authored biome swatch. Presentation only, a pure read of
/// canon (Principle 10). This is the first slice of the visual-projection substrate: the palette
/// anchors below (sea level, the tan/green/snow/ochre/rock endpoints) are the tunable projection, an
/// aesthetic call the owner reserves, not physics.
pub fn physics_terrain_color(elevation: Fixed, moisture: Fixed, temperature: Fixed) -> Rgb {
    fn unit_to_255(f: Fixed) -> i32 {
        f.checked_mul(Fixed::from_int(255))
            .map(|v| v.to_int())
            .unwrap_or(0)
            .clamp(0, 255)
    }
    fn mix(a: i32, b: i32, num: i32, den: i32) -> i32 {
        a + (b - a) * num.clamp(0, den) / den.max(1)
    }
    let (e, m, t) = (
        unit_to_255(elevation),
        unit_to_255(moisture),
        unit_to_255(temperature),
    );
    const SEA: i32 = 77; // elevation 0.30: below it the cell is water
    if e < SEA {
        // Water: teal at the warm shallows deepening to cold abyssal blue.
        let d = SEA - e;
        return Rgb::new(
            mix(22, 6, d, SEA) as u8,
            mix(104, 44, d, SEA) as u8,
            mix(176, 92, d, SEA) as u8,
        );
    }
    // Land base: dry tan to wet green by moisture.
    let mut r = mix(196, 58, m, 255);
    let mut g = mix(176, 132, m, 255);
    let mut b = mix(120, 58, m, 255);
    // Cold tints toward snow; heat tints toward arid ochre.
    if t < SEA {
        let c = SEA - t;
        r = mix(r, 236, c, SEA);
        g = mix(g, 240, c, SEA);
        b = mix(b, 246, c, SEA);
    } else if t > 255 - SEA {
        let hh = t - (255 - SEA);
        r = mix(r, 206, hh, SEA);
        g = mix(g, 150, hh, SEA);
        b = mix(b, 92, hh, SEA);
    }
    // High ground lightens toward rock, quadratic so lowlands keep their colour.
    let land = e - SEA;
    let hl = (land * land) / (255 - SEA);
    r = mix(r, 206, hl, 255 - SEA);
    g = mix(g, 210, hl, 255 - SEA);
    b = mix(b, 214, hl, 255 - SEA);
    Rgb::new(
        r.clamp(0, 255) as u8,
        g.clamp(0, 255) as u8,
        b.clamp(0, 255) as u8,
    )
}

/// Blend colour `a` toward `b` by `num/den`. Presentation only.
fn blend(a: Rgb, b: Rgb, num: i32, den: i32) -> Rgb {
    let mix = |x: u8, y: u8| -> u8 {
        (x as i32 + (y as i32 - x as i32) * num.clamp(0, den) / den.max(1)).clamp(0, 255) as u8
    };
    Rgb::new(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b))
}

#[inline]
fn fill_rect(buf: &mut [u32], w: usize, x0: usize, y0: usize, rw: usize, rh: usize, color: u32) {
    for y in y0..(y0 + rh).min(buf.len() / w.max(1)) {
        let row = y * w;
        for x in x0..(x0 + rw).min(w) {
            buf[row + x] = color;
        }
    }
}

/// The 5x7 rows of a glyph (low 5 bits per row, leftmost column is bit 4). A compact bitmap
/// font covering the characters the selector readout uses; lowercase maps to uppercase.
fn glyph_rows(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        ' ' => [0; 7],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        '#' => [
            0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010,
        ],
        '(' => [
            0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
        ],
        ')' => [
            0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
        ],
        ',' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100, 0b01000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ':' => [
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '|' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        '^' => [
            0b00100, 0b01010, 0b10001, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '~' => [
            0b00000, 0b00000, 0b01101, 0b10110, 0b00000, 0b00000, 0b00000,
        ],
        _ => [
            0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
        ], // '?'
    }
}

/// Draw a text label with a filled backing panel, so the cursor readout is legible on the map
/// (the on-canvas names of what the selector points at). `scale` is pixels per font pixel.
/// The panel is clamped to stay on screen.
#[allow(clippy::too_many_arguments)]
pub fn draw_label(
    buf: &mut [u32],
    w: usize,
    h: usize,
    x: i32,
    y: i32,
    text: &str,
    scale: usize,
    fg: Rgb,
    bg: Rgb,
) {
    let scale = scale.max(1);
    let cw = (5 + 1) * scale; // glyph width plus one-column gap
    let pad = scale * 2;
    let panel_w = text.chars().count() * cw + pad * 2;
    let panel_h = 7 * scale + pad * 2;
    // Clamp the panel onto the screen.
    let px = x.clamp(0, (w as i32 - panel_w as i32).max(0)) as usize;
    let py = y.clamp(0, (h as i32 - panel_h as i32).max(0)) as usize;
    fill_rect(buf, w, px, py, panel_w, panel_h, bg.pack());
    let fgp = fg.pack();
    let mut cx = px + pad;
    let ty = py + pad;
    for ch in text.chars() {
        let rows = glyph_rows(ch);
        for (r, bits) in rows.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    fill_rect(buf, w, cx + col * scale, ty + r * scale, scale, scale, fgp);
                }
            }
        }
        cx += cw;
    }
}

/// Draw a one-pixel outline around a cell rectangle, the cursor the tile selector uses to
/// indicate the hovered cell. Presentation only.
pub fn draw_outline(
    buf: &mut [u32],
    w: usize,
    x0: usize,
    y0: usize,
    rw: usize,
    rh: usize,
    color: Rgb,
) {
    let h = buf.len() / w.max(1);
    let c = color.pack();
    let x1 = (x0 + rw).min(w);
    let y1 = (y0 + rh).min(h);
    if x0 >= w || y0 >= h || x1 == 0 || y1 == 0 {
        return;
    }
    for x in x0..x1 {
        buf[y0 * w + x] = c;
        buf[(y1 - 1) * w + x] = c;
    }
    for y in y0..y1 {
        buf[y * w + x0] = c;
        buf[y * w + (x1 - 1)] = c;
    }
}

/// Paint the superfine view centred on `center`, each tile drawn as a `tile_px` square: its
/// biome colour, then the organisms on it as centred marks. Returns a `w` by `h` RGB buffer.
pub fn superfine(
    living: &LivingWorld,
    biomes: &BiomeSet,
    center: Coord3,
    tile_px: usize,
    w: usize,
    h: usize,
    bg: Rgb,
) -> Vec<u32> {
    let tile_px = tile_px.max(3);
    let cols = (w / tile_px).max(1) as i32;
    let rows = (h / tile_px).max(1) as i32;
    let ox = center.x - cols / 2;
    let oy = center.y - rows / 2;
    let topo = living.map.topo();
    let mut buf = vec![bg.pack(); w * h];

    for r in 0..rows {
        for c in 0..cols {
            let coord = Coord3::ground(ox + c, oy + r);
            let px0 = c as usize * tile_px;
            let py0 = r as usize * tile_px;
            // Physics-derived terrain colour (the tile's own elevation, moisture, and temperature),
            // with a light accent of the biome swatch for identity, or the empty-space colour off
            // the map. Terrain looks like its physics rather than an authored swatch.
            let tile_color = if topo.contains(coord) {
                living
                    .map
                    .tile(coord)
                    .map(|t| {
                        let physics =
                            physics_terrain_color(t.elevation(), t.moisture(), t.temperature());
                        blend(physics, biomes.color(t.biome), 46, 255) // about 18% biome accent
                    })
                    .unwrap_or(bg)
            } else {
                bg
            };
            fill_rect(&mut buf, w, px0, py0, tile_px, tile_px, tile_color.pack());

            // The organisms on this tile, drawn as marks sized by body mass and coloured by
            // kind, so anatomy shows: a big carnivore is a large red mark, a small plant a
            // small green one.
            let occ = living.occupants.occupants(coord);
            if occ.is_empty() {
                continue;
            }
            for (i, o) in occ.iter().enumerate().take(4) {
                let info = living.occupant_info.get(o);
                let color = info
                    .map(|inf| organism_color(inf.layer, inf.species.0))
                    .unwrap_or(Rgb::new(240, 240, 240));
                // Mark size scales with body mass: a quarter-tile at the smallest up to about
                // eight-tenths of a tile at the largest (integer, via the Fixed body-mass value).
                let bm = info
                    .map(|inf| inf.body_mass)
                    .unwrap_or(Fixed::from_ratio(1, 2));
                let span = Fixed::from_int((tile_px * 3 / 5) as i32);
                let extra = bm
                    .checked_mul(span)
                    .map(|v| v.to_int().max(0) as usize)
                    .unwrap_or(0);
                let mark = (tile_px / 4 + extra).clamp(2, tile_px);
                // Centre a lone occupant; nudge several so they stay distinct.
                let base = (tile_px.saturating_sub(mark)) / 2;
                let nudge = tile_px / 6;
                let (ox, oy) = match i {
                    0 => (base, base),
                    1 => (base.saturating_sub(nudge), base.saturating_sub(nudge)),
                    2 => (base + nudge, base + nudge),
                    _ => (base + nudge, base.saturating_sub(nudge)),
                };
                fill_rect(&mut buf, w, px0 + ox, py0 + oy, mark, mark, color.pack());
            }
        }
    }
    buf
}

/// The three photoreceptor band centres (nm) the non-canon material projection samples: red, green, blue. AUTHORED
/// display choices, the observability-layer allowance the optics substrate reserves for the downstream observer
/// projection (`optics.rs`: the perceived colour is the observer's projection, not in the substrate). They are the
/// only wavelengths the reflectance is sampled at, and they match the sky's `BANDS_NM` so the two non-canon
/// projections read the same spectrum. Each lies inside a human's `~1.6-3.1 eV` visible window (630 nm ~ 1.97 eV,
/// 532 nm ~ 2.33 eV, 465 nm ~ 2.67 eV).
const MATERIAL_BANDS_NM: [f64; 3] = [630.0, 532.0, 465.0];

/// The class-grade intensity weight of an ALLOWED charge-transfer band relative to the unit-weight sharp features
/// (the forbidden d-d line and the band-gap / plasma edges), applied in this non-canon projection only. It stands in
/// for the per-feature oscillator STRENGTH the optics substrate does not yet carry: a symmetry-allowed charge-transfer
/// transition is orders of magnitude more intense than a Laporte-forbidden d-d line (the Laporte rule), so its huge
/// absorption coefficient renders a mineral opaque even where its broad Lorentzian / step tail is only a few percent
/// of the peak. Without it a ferric oxide reads light (the tail alone tops out near forty percent absorption in the
/// visible); with it a ferric oxide reads dark-red and a mixed-valence oxide near-black. RESERVED, surfaced with its
/// basis to `docs/working/MORNING_REVIEW.md` (the allowed-to-forbidden oscillator-strength ratio, class-grade, bounded
/// below where a ferric oxide would stay light and above where hematite would over-darken to black); byte-neutral (a
/// non-canon presentation weight, like `MATERIAL_BANDS_NM` and the relief palette, with ZERO effect on canon). The
/// canon fix is a per-feature oscillator-strength column, the named follow-on. NOT authored in the canon.
const CHARGE_TRANSFER_INTENSITY_WEIGHT: f64 = 3.0;

/// The photon-energy constant `h c` in `eV * nm` (`~1239.8`), DERIVED from the fundamentals register (`h`, `c`, `e`),
/// never authored: the photon energy at wavelength `lambda` in nm is `E[eV] = hc_ev_nm / lambda`. Parsed to `f64`
/// because this is a non-canon display value that needs no fixed-point rigour past per-run determinism. `None` on a
/// register miss.
fn hc_ev_nm() -> Option<f64> {
    let h: f64 = civsim_units::fundamentals::fundamental("h")?
        .value
        .parse()
        .ok()?;
    let c: f64 = civsim_units::fundamentals::fundamental("c")?
        .value
        .parse()
        .ok()?;
    let e: f64 = civsim_units::fundamentals::fundamental("e")?
        .value
        .parse()
        .ok()?;
    Some(h * c / e * 1.0e9)
}

/// Reduce a composition's `(element, amount)` pairs to integer `(element, count)` pairs for the substance lookups (the
/// band-gap column, the crystal-field oxide table, and the iron oxidation-state derivation key on integer
/// stoichiometry). When every positive amount is already a whole number the ratio is reduced by its greatest common
/// divisor, so an EXACT stoichiometry survives intact and distinct (`Fe3O4` stays `{Fe:3, O:4}` rather than rounding
/// to `{Fe:1, O:1}`, which is what the ferric-versus-mixed-versus-ferrous distinction turns on; `SiO2` stays
/// `{Si:1, O:2}`). A fractional, solar-abundance-scaled crust instead scales to the smallest positive amount and
/// rounds, so it still reduces to a non-empty integer ratio rather than rounding every trace amount to zero; a mixed,
/// non-stoichiometric crust reduces to counts that match no seeded phase and resolves to no optical feature (a pale,
/// featureless read), the honest outcome for a fresh silicate crust.
fn composition_counts(composition: &[(String, Fixed)]) -> Vec<(String, u32)> {
    let positives: Vec<(&String, Fixed)> = composition
        .iter()
        .filter(|(_, a)| *a > Fixed::ZERO)
        .map(|(el, a)| (el, *a))
        .collect();
    if positives.is_empty() {
        return Vec::new();
    }
    // The exact-integer path: reduce by the greatest common divisor so a whole-number stoichiometry is preserved.
    let all_integer = positives
        .iter()
        .all(|(_, a)| Fixed::from_int(a.to_int()) == *a);
    if all_integer {
        let mut g: u64 = 0;
        for (_, a) in &positives {
            g = gcd_u64(g, a.to_int() as u64);
        }
        let g = g.max(1);
        return positives
            .iter()
            .filter_map(|(el, a)| {
                let n = (a.to_int() as u64) / g;
                (n > 0).then(|| ((*el).clone(), n as u32))
            })
            .collect();
    }
    // The fractional path: scale to the smallest positive amount and round to the nearest integer ratio.
    let scale = positives
        .iter()
        .map(|(_, a)| *a)
        .min()
        .unwrap_or(Fixed::ONE);
    positives
        .iter()
        .filter_map(|(el, amount)| {
            let ratio = amount.checked_div(scale)?;
            let n = ratio
                .checked_add(Fixed::from_ratio(1, 2))
                .map(|v| v.to_int())
                .unwrap_or(0);
            (n > 0).then(|| ((*el).clone(), n as u32))
        })
        .collect()
}

/// The greatest common divisor (Euclid), the stoichiometry reducer for [`composition_counts`].
fn gcd_u64(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd_u64(b, a % b)
    }
}

/// The observability-non-canon perceived colour of a material under a star's light, DERIVED from the material's own
/// absorption spectrum and the star's Planck spectrum, never an authored per-mineral swatch. This is exactly the
/// downstream observer projection the optics substrate (`civsim_materials::optics`) reserves and refuses to author:
/// the substrate produces the observer-INDEPENDENT characteristic energies, and this renderer projects them against a
/// human-baseline visible window and three photoreceptor bands into a screen colour, one-way (canon physics ->
/// pixels), writing no canonical state (Principle 10).
///
/// The derivation chain, each step read from the material's own data, never a colour lookup:
///   1. The composition reduces to integer counts, and the material's electronic classification and optical
///      characteristic energies come from the banked substrate: the band gap (an INTERBAND ONSET) from the band-gap
///      column ([`conduction_class_from_column`] / [`BandGapColumn::gap`]) and the ligand-field d-d line from the
///      crystal-field oxide table ([`CrystalFieldTables::oxide_delta_cm`] converted to eV by [`cm_to_ev`]). No plasma
///      edge is emitted: a carrier density is not derivable for an arbitrary composition here, so a metal carries no
///      plasma feature rather than a fabricated one (a stated limit, not a guessed value).
///   2. At each of the three photoreceptor bands the ABSORPTION is the summed feature response
///      ([`feature_response_at`]) of the material's optical energies, broadened by the grounded thermal width
///      ([`thermal_broadening_width_ev`], `~ k_B T`, never an authored linewidth), capped at full absorption. The
///      REFLECTANCE is its complement, `1 - absorption`: a feature that reaches the band darkens it.
///   3. The reflected radiance per band is the star's relative Planck spectral radiance ([`planck_relative`], the
///      incident starlight colour, derived from the second radiation constant) times the reflectance, and the three
///      bands normalize against the ILLUMINANT reference (the brightest band at full reflectance), so absorption reads
///      as DARKNESS: a fully-absorbing material is dark, a fully-reflecting one takes the star's colour at full
///      brightness, never renormalized back to full intensity per material.
///
/// Admit-the-alien: the mix is keyed on the composition and per-substance banked data (the gap column, the oxide d-d
/// table), so a new material is a data row, and the visible window and band centres are the OBSERVER's property
/// (`MATERIAL_BANDS_NM`), never the material's. A being with a different eye would read the same spectrum a different
/// colour, a data-row difference.
///
/// VALIDITY CEILING: this is FACTOR-GRADE and QUALITATIVE (dark versus light, warm versus cool), not a calibrated
/// reflectance. A substance whose absorption onset lies within or below the visible window reads dark; a wide-gap or
/// feature-free substance reads light; and the reflected colour warms under a cooler star. It carries NO Fresnel
/// surface reflectance (so a small-gap absorber reads pure-dark rather than dark-grey), and only the LEADING
/// crystal-field d-d line at `Delta_o` (the higher visible-range multiplets are a named optics follow-on). Seam 2
/// adds the iron charge-transfer darkening: a FERRIC oxide (`Fe3+`, hematite) carries the intense `O2- -> Fe3+`
/// charge-transfer edge whose broad tail reddens and darkens it, and a MIXED-valence oxide (magnetite) adds the
/// `Fe2+ -> Fe3+` intervalence band and reads near-black; keyed on the DERIVED iron oxidation state, so a FERROUS
/// oxide (`FeO`, whose only feature is the near-infrared `~0.93 eV` d-d line) correctly stays light, the honest
/// per-valence outcome. The remaining class-grade limit is the oscillator STRENGTH: the substrate carries the band
/// energy and its broad width but not a per-feature oscillator strength, so the charge-transfer intensity that makes
/// the tail opaque in the visible is a class-grade weight applied HERE ([`CHARGE_TRANSFER_INTENSITY_WEIGHT`], the
/// non-canon projection), never in the canon. `f64` throughout, mirroring [`rayleigh_sky_rgb`] and [`blackbody_rgb`].
/// `None` when the composition reduces to nothing or the illuminant does not resolve (fail-soft: the caller keeps the
/// relief swatch).
pub fn material_surface_rgb(
    composition: &[(String, Fixed)],
    star_t_eff_k: Fixed,
    temperature_k: Fixed,
    gaps: &BandGapColumn,
    crystal: &CrystalFieldTables,
    table: &PeriodicTable,
) -> Option<Rgb> {
    let counts = composition_counts(composition);
    if counts.is_empty() {
        return None;
    }
    // The material's own electronic classification and observer-independent optical characteristic energies: the
    // interband onset from the banked gap column, the d-d ligand-field line from the crystal-field oxide table, and
    // the iron charge-transfer / intervalence bands whose presence keys on the composition's DERIVED iron oxidation
    // state (a ferric phase carries the charge-transfer edge, a mixed-valence phase adds the intervalence band). No
    // plasma edge (no derivable carrier density here, so no fabricated feature). Keyed on the substance's own data.
    let class = conduction_class_from_column(gaps, &counts, temperature_k);
    let band_gap_ev = gaps.gap(&counts).map(|bg| bg.gap_ev);
    let dd_transition_ev = crystal.oxide_delta_cm(&counts).and_then(cm_to_ev);
    let (charge_transfer_ev, intervalence_ev) =
        crystal.iron_charge_transfer_energies(&counts, table);
    let features = optical_energies(
        &class,
        band_gap_ev,
        None,
        dd_transition_ev,
        charge_transfer_ev,
        intervalence_ev,
    );
    // The grounded broadening widths (thermal ~ k_B T for the sharp features, the broad Marcus-Hush vibronic width for
    // an allowed charge-transfer band), never an authored linewidth.
    let thermal = thermal_broadening_width_ev(temperature_k);
    let c2_m_k = second_radiation_constant_m_k()?;
    let hc = hc_ev_nm()?;
    let t_eff = star_t_eff_k.to_f64_lossy();
    let mut band_radiance = [0.0f64; 3];
    let mut illum_ref = 0.0f64;
    for (i, &lambda_nm) in MATERIAL_BANDS_NM.iter().enumerate() {
        let probe_ev_f = hc / lambda_nm;
        // The probe energy back into fixed-point (micro-eV resolution) for the observer-independent feature response.
        let probe_ev = Fixed::from_ratio((probe_ev_f * 1.0e6).round() as i64, 1_000_000);
        // Absorption at this band: the summed feature response of the material's optical energies, capped at full
        // absorption. Reflectance is its complement (a feature reaching the band darkens it). Each feature carries its
        // own grounded width and a class-grade intensity: the forbidden d-d line and the sharp edges take the near-
        // thermal width at unit weight; the ALLOWED charge-transfer and intervalence bands take the broad Marcus-Hush
        // vibronic width and the high oscillator-strength weight, so their intense tails flood the visible.
        let mut absorption = 0.0f64;
        for feature in &features {
            let (feature_width, weight) = match feature.feature {
                OpticalFeature::ChargeTransferBand | OpticalFeature::IntervalenceBand => (
                    marcus_hush_width_ev(feature.energy_ev, temperature_k),
                    CHARGE_TRANSFER_INTENSITY_WEIGHT,
                ),
                _ => (thermal, 1.0),
            };
            if let Some(r) = feature_response_at(probe_ev, feature, feature_width) {
                absorption += weight * r.to_f64_lossy();
            }
        }
        let reflectance = (1.0 - absorption).clamp(0.0, 1.0);
        let illum = planck_relative(lambda_nm, t_eff, c2_m_k);
        band_radiance[i] = illum * reflectance;
        if illum > illum_ref {
            illum_ref = illum;
        }
    }
    if illum_ref <= 0.0 {
        return None;
    }
    let to_u8 = |v: f64| (v / illum_ref * 255.0).round().clamp(0.0, 255.0) as u8;
    Some(Rgb::new(
        to_u8(band_radiance[0]),
        to_u8(band_radiance[1]),
        to_u8(band_radiance[2]),
    ))
}

/// The glyph a DERIVED tile shows, keyed by its relief class (the R1-override terrain projected to a mark in the
/// Dwarf-Fortress-spirit glyph view): submarine reads as water `~`, lowland as flat ground `.`, upland as raised
/// `^`. Presentation only, a one-way read of the derived relief, never canonical state (Principle 10).
pub fn derived_tile_glyph(relief: TerrainRelief) -> char {
    match relief {
        TerrainRelief::Submarine => '~',
        TerrainRelief::Lowland => '.',
        TerrainRelief::Upland => '^',
    }
}

/// The colour a DERIVED tile paints in the window, keyed by its relief class. The palette (deep water blue,
/// basaltic lowland grey, a lighter upland grey) is the tunable visual projection, an aesthetic call the owner
/// reserves: authored ONLY here in the non-canon renderer, byte-neutral on canon (Principle 10). The relief it
/// keys off is derived (the substrate's elevation crossing the derived references), so what varies across the
/// frame is physics; only the swatch is authored.
pub fn derived_tile_color(relief: TerrainRelief) -> Rgb {
    match relief {
        TerrainRelief::Submarine => Rgb::new(28, 78, 156), // deep water
        TerrainRelief::Lowland => Rgb::new(74, 68, 62),    // basaltic lowland
        TerrainRelief::Upland => Rgb::new(124, 113, 102),  // lighter raised rock
    }
}

/// Draw one glyph centred in a `size` by `size` cell at `(x0, y0)`, the font scaled to the cell. Presentation only.
fn draw_glyph_centered(
    buf: &mut [u32],
    w: usize,
    x0: usize,
    y0: usize,
    size: usize,
    ch: char,
    fg: Rgb,
) {
    let scale = (size / 8).max(1);
    let gw = 5 * scale;
    let gh = 7 * scale;
    let ix = x0 + size.saturating_sub(gw) / 2;
    let iy = y0 + size.saturating_sub(gh) / 2;
    let fgp = fg.pack();
    for (r, bits) in glyph_rows(ch).iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) != 0 {
                fill_rect(buf, w, ix + col * scale, iy + r * scale, scale, scale, fgp);
            }
        }
    }
}

/// Paint a field of DERIVED tiles as a `w` by `h` frame: each tile a `tile_px` block of its relief colour
/// ([`derived_tile_color`]) with its relief glyph ([`derived_tile_glyph`]) centred, laid out `cols` to a row in
/// generation order. This is the capstone's visible spine reaching the window: the terrain in the frame is what
/// the substrate derived (composition -> elevation -> relief), never fractal noise or an authored biome swatch
/// (the R1 override, end to end). A pure, deterministic read of the derived field, one-way canon -> view, so it
/// writes no canonical state and adds nothing to the canon hash (Principle 10).
pub fn paint_derived_tiles(
    tiles: &[DerivedTile],
    cols: usize,
    tile_px: usize,
    w: usize,
    h: usize,
    bg: Rgb,
) -> Vec<u32> {
    let tile_px = tile_px.max(3);
    let cols = cols.max(1);
    let mut buf = vec![bg.pack(); w * h];
    for (i, t) in tiles.iter().enumerate() {
        let cx = (i % cols) * tile_px;
        let cy = (i / cols) * tile_px;
        if cx >= w || cy >= h {
            continue;
        }
        let color = derived_tile_color(t.relief);
        fill_rect(&mut buf, w, cx, cy, tile_px, tile_px, color.pack());
        // A readable glyph over the block: dark on a light tile, light on a dark one.
        let fg = if color.luminance() > 128 {
            Rgb::new(20, 20, 24)
        } else {
            Rgb::new(228, 232, 236)
        };
        draw_glyph_centered(
            &mut buf,
            w,
            cx,
            cy,
            tile_px,
            derived_tile_glyph(t.relief),
            fg,
        );
    }
    buf
}

/// A relief-shading brightness for the material-tile paint: upland reads full, lowland a shade dimmer, submarine
/// dimmest (as if the surface sits deeper / in shadow). The ONE authored display relief scale the non-canon layer is
/// allowed (documented here at its site), byte-neutral on canon: it modulates the DERIVED material colour's brightness
/// so relief structure is legible, never inventing a per-relief hue. The relief it keys off is derived. Used by the
/// sphere's material tint ([`draw_globe`]) and the flat material-tile paint.
fn relief_shade(relief: TerrainRelief) -> f32 {
    match relief {
        TerrainRelief::Upland => 1.0,
        TerrainRelief::Lowland => 0.82,
        TerrainRelief::Submarine => 0.55,
    }
}

/// Paint a field of DERIVED tiles coloured by their MATERIAL's perceived colour under the star
/// ([`material_surface_rgb`]), each tile a `tile_px` block of that colour scaled by the relief shading
/// ([`relief_shade`]) with the relief glyph centred. This is the zoom-in surface of the derived planet: the crust the
/// physics condensed and differentiated, painted its own optical colour under the star, never an authored swatch. For
/// a fresh, uniform crust every tile shares the material colour and the visible variation is the relief shading, the
/// honest look until lateral composition variation lands (a named geodynamics follow-on). A pure, deterministic read,
/// one-way canon -> pixels (Principle 10).
///
/// Retained for the flat material-tile view, but the derived viewer no longer calls it: that mode now zooms ONTO the
/// sphere's surface (a continuous globe zoom) rather than a disconnected flat tile field. Kept available (covered by a
/// unit test) so the material-tile path is not lost.
#[allow(dead_code)]
pub fn paint_material_tiles(
    tiles: &[DerivedTile],
    material: Rgb,
    cols: usize,
    tile_px: usize,
    w: usize,
    h: usize,
    bg: Rgb,
) -> Vec<u32> {
    let tile_px = tile_px.max(3);
    let cols = cols.max(1);
    let mut buf = vec![bg.pack(); w * h];
    for (i, t) in tiles.iter().enumerate() {
        let cx = (i % cols) * tile_px;
        let cy = (i / cols) * tile_px;
        if cx >= w || cy >= h {
            continue;
        }
        let s = relief_shade(t.relief);
        let scale = |c: u8| (c as f32 * s).clamp(0.0, 255.0) as u8;
        let color = Rgb::new(scale(material.r), scale(material.g), scale(material.b));
        fill_rect(&mut buf, w, cx, cy, tile_px, tile_px, color.pack());
        let fg = if color.luminance() > 128 {
            Rgb::new(20, 20, 24)
        } else {
            Rgb::new(228, 232, 236)
        };
        draw_glyph_centered(
            &mut buf,
            w,
            cx,
            cy,
            tile_px,
            derived_tile_glyph(t.relief),
            fg,
        );
    }
    buf
}

/// The on-screen radius (pixels) a planet's DERIVED radius projects to at a view scale of `m_per_px` metres per
/// pixel: `radius_px = radius_m / m_per_px`. This is the seeable-world size law, so a denser (smaller-radius)
/// planet draws a smaller globe and a larger one draws bigger, straight from [`civsim_sim::astro::planet_radius_m`].
/// All fixed-point, deterministic; a non-positive or overflowing input yields `0` (nothing to draw). Display-only,
/// a one-way read of the derived radius (Principle 10).
pub fn globe_radius_px(radius_m: Fixed, m_per_px: Fixed) -> usize {
    if radius_m <= Fixed::ZERO || m_per_px <= Fixed::ZERO {
        return 0;
    }
    radius_m
        .checked_div(m_per_px)
        .map(|v| v.to_int().max(0) as usize)
        .unwrap_or(0)
}

/// Normalise a 3-vector for display lighting, returning the +z unit vector for a zero input (a safe default facing
/// the viewer). Non-canon display math, `f32` is fine.
fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let m = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if m <= 0.0 {
        [0.0, 0.0, 1.0]
    } else {
        [v[0] / m, v[1] / m, v[2] / m]
    }
}

/// The radius, in rim-radii, out to which the ejecta blanket is stamped: a display COMPUTE bound (how far the
/// `x^-3` blanket is worth evaluating: by three rim-radii it is `1/27` of the rim height, under one percent of
/// the bowl depth), never a physical range. Its sibling is the crater law's ballistic step cap, a like bound.
const CRATER_STAMP_REACH: Fixed = Fixed::from_int(3);

/// The BODY-frame unit point of a crater/sample normalized surface coordinate `(u, v)` (longitude fraction in
/// `[0, 1)`, latitude fraction in `[0, 1]`), the same `lon = u*2pi - pi`, `lat = (0.5 - v)*pi` sphere map
/// [`uv_to_body`] uses, so a crater centre and a sampled surface point land in one frame. Fixed-point (Principle
/// 3), display-only math (Principle 10).
fn crater_uv_unit(u: Fixed, v: Fixed) -> [Fixed; 3] {
    let tau = Fixed::PI.mul(Fixed::from_int(2));
    let lon = u.mul(tau) - Fixed::PI;
    let lat = (Fixed::from_ratio(1, 2) - v).mul(Fixed::PI);
    let (sin_lat, cos_lat) = lat.sin_cos();
    let (sin_lon, cos_lon) = lon.sin_cos();
    [cos_lat.mul(sin_lon), sin_lat, cos_lat.mul(cos_lon)]
}

/// A crater ROW prepared for the analytic surface stamp: its centre as a unit vector on the globe, its angular
/// rim radius (radians), its bowl depth (kilometres), and the cosine of its reach cone (the bound a distant
/// sample fails cheaply, without the great-circle angle). Precomputed once per crater from a [`CraterRow`] and
/// the body radius, so the per-sample stamp is a dot product for the far majority and one great-circle angle
/// near the crater. Display-only (Principle 10): the canon is the row (position, diameter, depth, age); this is
/// its rendering form.
#[derive(Clone, Copy, Debug)]
pub struct CraterStamp {
    center: [Fixed; 3],
    /// alpha = (diameter / 2) / R_planet, the crater's rim radius as an angle on the sphere (radians).
    angular_radius: Fixed,
    /// The crater law's transient bowl depth, in kilometres (the tile field's unit).
    depth_km: Fixed,
    /// cos(CRATER_STAMP_REACH * alpha): a sample whose centre-dot is below this lies outside the reach cone.
    cos_reach: Fixed,
}

impl CraterStamp {
    /// Prepare a crater row for the analytic stamp against a body of radius `radius_m` (metres). `None` on a
    /// degenerate row (a non-positive radius or diameter), so a bad row is skipped rather than fabricated.
    pub fn from_row(row: &CraterRow, radius_m: Fixed) -> Option<CraterStamp> {
        if radius_m <= Fixed::ZERO || row.diameter_m <= Fixed::ZERO {
            return None;
        }
        // alpha = (D/2) / R: the rim radius as an angle on the sphere. A low-g or weak-target world's larger
        // crater subtends a larger angle by its own crater-law diameter, so the morphology conditions on the world.
        let angular_radius = row
            .diameter_m
            .checked_div(Fixed::from_int(2))?
            .checked_div(radius_m)?;
        if angular_radius <= Fixed::ZERO {
            return None;
        }
        let depth_km = row.depth_m.checked_div(Fixed::from_int(1000))?;
        let cos_reach = angular_radius.checked_mul(CRATER_STAMP_REACH)?.cos();
        Some(CraterStamp {
            center: crater_uv_unit(row.u, row.v),
            angular_radius,
            depth_km,
            cos_reach,
        })
    }
}

/// Prepare the crater rows of a deep-time state as analytic stamps against a body of radius `radius_m` (metres),
/// dropping any degenerate row. The caller builds this once, then samples it across the display tile grid (the
/// sample cache) through [`crater_relief_km`]. Display-only (Principle 10).
pub fn crater_stamps(craters: &[CraterRow], radius_m: Fixed) -> Vec<CraterStamp> {
    craters
        .iter()
        .filter_map(|row| CraterStamp::from_row(row, radius_m))
        .collect()
}

/// THE ANALYTIC CRATER STAMP: the summed elevation offset (kilometres) of every crater row covering the surface
/// sample point `p` (its unit vector on the globe, from [`crater_uv_unit`] on the sample's `(u, v)`). Each crater
/// contributes the crater law's OWN shape at the angular distance `d` from its centre, in rim-radii `x = d /
/// alpha`: inside the rim (`x <= 1`) the paraboloid excavation bowl `-h (1 - x^2)` (the same shape the built
/// impact composer digs, deepest `h` at the centre, zero at the rim), and outside the rim (`1 < x <= reach`) the
/// ejecta rim/blanket `(h/4) x^-3`, whose `x^-3` falloff is the cited lunar ejecta-thickness law (McGetchin 1973)
/// and whose amplitude `h/4` is DERIVED (no authored value) by conserving the excavated bowl volume `(pi/2) h R^2`
/// against the blanket integral `2 pi (h/4) R^2`. This is the stamp the ruling prescribes: at a coarse sample the
/// big craters show, at a fine sample the small ones resolve, from ONE row list; the existing hillshade shades the
/// stamped relief. `h` is the crater law's transient bowl depth, so a large fresh crater reads deep (the
/// simple-crater `bowl_aspect` with no complex-crater depth flattening is the crater law's own honest limit, off
/// this display path). The great-circle angle uses the small-crater domain (a sample inside the reach cone, so
/// `d <= reach*alpha`); a basin-scale row's rim stays within a hemisphere. The caller passes `p` precomputed (a
/// grid sampler builds it separably, one latitude sin/cos per row and one longitude sin/cos per column, so the
/// per-sample cost is a dot product, not trig). Deterministic fixed-point display math (Principle 3, Principle 10).
pub fn crater_relief_km(stamps: &[CraterStamp], p: [Fixed; 3]) -> Fixed {
    if stamps.is_empty() {
        return Fixed::ZERO;
    }
    let mut z = Fixed::ZERO;
    for s in stamps {
        // The reach-cone bound: cos(great-circle angle) is the dot product, so a sample below cos_reach lies
        // outside the crater's blanket and is skipped without the great-circle angle (the cheap far-majority path).
        let dot = p[0].mul(s.center[0]) + p[1].mul(s.center[1]) + p[2].mul(s.center[2]);
        if dot < s.cos_reach {
            continue;
        }
        // The great-circle central angle from the cross-product magnitude (|p x c| = sin(angle)), precise for the
        // small crater angles here (dot >= cos_reach >= 0, so the angle is at most the reach, within a hemisphere).
        let cx = p[1].mul(s.center[2]) - p[2].mul(s.center[1]);
        let cy = p[2].mul(s.center[0]) - p[0].mul(s.center[2]);
        let cz = p[0].mul(s.center[1]) - p[1].mul(s.center[0]);
        let cross_mag = (cx.mul(cx) + cy.mul(cy) + cz.mul(cz)).sqrt();
        let angle = cross_mag.asin();
        let x = match angle.checked_div(s.angular_radius) {
            Some(x) => x,
            None => continue,
        };
        let contribution = if x <= Fixed::ONE {
            // The paraboloid excavation bowl: -h (1 - x^2), the built impact composer's own dig shape.
            Fixed::ZERO - s.depth_km.mul(Fixed::ONE - x.mul(x))
        } else {
            // The ejecta rim/blanket: (h/4) x^-3, the McGetchin falloff at the bowl-conserving amplitude h/4.
            let x3 = x.mul(x).mul(x);
            match s
                .depth_km
                .checked_div(Fixed::from_int(4))
                .and_then(|q| q.checked_div(x3))
            {
                Some(e) => e,
                None => continue,
            }
        };
        z += contribution;
    }
    z
}

/// NON-CANON DISPLAY (Principle 10): one fresh crater's transient IMPACT-EVENT flash, the watchable incandescent
/// bloom a strike paints over its settled static relief and that RELAXES back into that relief as the deep-time
/// clock advances past its formation. Where [`CraterStamp`] is the crater's permanent shape, this is the brief
/// event at its birth: a viewer watching deep time SEES the impact land, then sees the flash fade to the quiet
/// crater. Every physical property is DERIVED from the crater's own row: WHEN it flashes is the crater's own
/// formation tick ([`CraterRow::age_myr`], the clock reading at the strike), WHERE it sits and how WIDE the bloom
/// spreads is the crater's own ejecta footprint (the same `center`, `angular_radius`, and reach cone the relief
/// stamp derives), and how it FADES is the crater's own time-decay from that formation tick. Only the emission HUE
/// and the brightness GAIN are display constants (siblings of the lava glow's), because the state carries no derived
/// shock temperature: the sim never forms the impact's kinetic energy (it overflows the fixed-point range, see
/// [`civsim_sim::deeptime::bombard_tick`]). Carried in the display f32 tier the globe pixel loop emits in
/// ([`draw_globe`], the tier the lava glow and hillshade already run in), with the load-bearing timing derived in
/// fixed-point so the same clock step yields the same flash to the bit (Principle 3).
#[derive(Clone, Copy, Debug)]
pub struct ImpactFlash {
    /// The crater centre as a BODY-frame unit vector (the globe pixel loop's f32 tier).
    center: [f32; 3],
    /// The crater's rim radius as an angle on the sphere (radians), its own `(D/2)/R` footprint.
    angular_radius: f32,
    /// cos(reach cone): a pixel whose centre-dot is below this lies outside the bloom and is skipped cheaply.
    cos_reach: f32,
    /// The flash brightness in `[0, 1]` at the render's current epoch: the crater's own time-decay, peak `1` at its
    /// formation tick and `0` once a full relaxation window has passed (after which the crater carries no flash).
    intensity: f32,
}

impl ImpactFlash {
    /// Prepare a crater row as a transient impact flash at the render's current deep-time `epoch_myr`, over a
    /// `window_myr` relaxation window. `None` when the crater is degenerate (the same guard [`CraterStamp::from_row`]
    /// applies) OR when the epoch lies outside the crater's flash window: before it formed (`epoch < formation`) or
    /// after it has settled (`epoch >= formation + window`). So a caller mapping this over every crater keeps only
    /// the few currently flashing, and the emission is a transient the viewer catches at each impact's own birth.
    /// The brightness is the DECLARED decay shape below; the geometry is the relief stamp's, carried to f32.
    pub fn from_row(
        row: &CraterRow,
        radius_m: Fixed,
        epoch_myr: Fixed,
        window_myr: Fixed,
    ) -> Option<ImpactFlash> {
        if window_myr <= Fixed::ZERO {
            return None;
        }
        // The crater's age since formation: [`CraterRow::age_myr`] is the elapsed clock reading AT the strike (the
        // formation tick), so the age is the current epoch minus it. Outside `[0, window)` the crater is not flashing
        // (not yet formed, or already relaxed to its static relief), so it contributes no emission.
        let since = epoch_myr.checked_sub(row.age_myr)?;
        if since < Fixed::ZERO || since >= window_myr {
            return None;
        }
        // `phase` in [0, 1): 0 at formation, -> 1 as it settles.
        let phase = since.checked_div(window_myr)?;
        // THE DECLARED DECAY SHAPE (the one display curve this effect is allowed): a quadratic ease-out
        // `(1 - phase)^2`, brightest (1) at the formation tick and easing smoothly to 0 a full window later. It is a
        // monotone fade (the flash peaks at the strike and only relaxes, never re-brightens), documented rather than
        // derived because no physical shock-cooling timescale is in the state; the relaxation WINDOW it runs over is
        // the caller's reserved-with-basis display duration.
        let one_minus = Fixed::ONE.checked_sub(phase)?;
        let intensity = one_minus.mul(one_minus);
        // Reuse the relief stamp's geometry AND its degenerate-row guard (a non-positive radius or diameter yields
        // `None`), then carry the few numbers the pixel loop needs into its f32 display tier.
        let stamp = CraterStamp::from_row(row, radius_m)?;
        Some(ImpactFlash {
            center: [
                stamp.center[0].to_f64_lossy() as f32,
                stamp.center[1].to_f64_lossy() as f32,
                stamp.center[2].to_f64_lossy() as f32,
            ],
            angular_radius: stamp.angular_radius.to_f64_lossy() as f32,
            cos_reach: stamp.cos_reach.to_f64_lossy() as f32,
            intensity: intensity.to_f64_lossy() as f32,
        })
    }

    /// Prepare a crater row as a HELD impact flash at a caller-supplied display `intensity` in `[0, 1]`, for the
    /// interactive FRAME-HOLD path. The observer's deep-time clock can jump many ticks per rendered frame at high
    /// playback speed, so a flash keyed only to the clock epoch (the headless single-epoch [`active_flash_stamps`])
    /// can be skipped between frames; the interactive viewer instead holds each fresh crater's bloom for a floor
    /// number of RENDERED frames and supplies the fade intensity from that frame count. The GEOMETRY is the SAME
    /// derived crater footprint the epoch-window flash uses ([`CraterStamp::from_row`], so an alien or low-gravity
    /// world's larger crater blooms wider by its own scaling); only the intensity comes from the frame-hold, not
    /// from a clock epoch. `None` on a degenerate row (the same guard [`CraterStamp::from_row`] applies).
    /// Display-only (Principle 10); the geometry is fixed-point, the intensity the display f32 tier.
    pub fn held(row: &CraterRow, radius_m: Fixed, intensity: f32) -> Option<ImpactFlash> {
        let stamp = CraterStamp::from_row(row, radius_m)?;
        Some(ImpactFlash {
            center: [
                stamp.center[0].to_f64_lossy() as f32,
                stamp.center[1].to_f64_lossy() as f32,
                stamp.center[2].to_f64_lossy() as f32,
            ],
            angular_radius: stamp.angular_radius.to_f64_lossy() as f32,
            cos_reach: stamp.cos_reach.to_f64_lossy() as f32,
            intensity: intensity.clamp(0.0, 1.0),
        })
    }
}

/// Prepare the fresh-impact FLASHES of a crater set at the render's current deep-time `epoch_myr`: map every crater
/// through [`ImpactFlash::from_row`] and keep only those currently flashing (formed within the last `window_myr`),
/// so the returned list is small (the quiescent majority of a heavily-cratered world drops out). The globe pixel
/// loop emits these over the shaded crust ([`crater_flash_emission`]), so as the clock steps past each crater's
/// formation the viewer SEES it land and fade. Empty when no crater is fresh, so the render adds nothing there
/// (byte-identical). Display-only (Principle 10); deterministic in the crater rows and the clock (Principle 3).
pub fn active_flash_stamps(
    craters: &[CraterRow],
    radius_m: Fixed,
    epoch_myr: Fixed,
    window_myr: Fixed,
) -> Vec<ImpactFlash> {
    craters
        .iter()
        .filter_map(|row| ImpactFlash::from_row(row, radius_m, epoch_myr, window_myr))
        .collect()
}

/// THE ANALYTIC IMPACT-FLASH EMISSION at a globe sample direction `b` (a display f32 unit vector): the summed
/// brightness in `[0, ~]` every fresh crater covering `b` contributes, its time-decayed intensity shaped by the
/// crater's OWN footprint, the SAME shape the static relief stamps: full over the excavation bowl (`x <= 1`, the
/// incandescent crater interior) and the `x^-3` ejecta falloff beyond the rim (`x > 1`, the glowing blanket),
/// continuous at the rim. Beyond a crater's reach cone it contributes nothing (the cheap far-majority reject, the
/// same bound the relief stamp uses). Zero for an empty list or a sample no fresh crater covers. Display-only f32
/// (Principle 10), the tier the lava glow and hillshade run in; [`draw_globe`] scales this by the flash hue and gain
/// and ADDS it, so a fresh impact glows on the night side too (it emits, it does not reflect).
fn crater_flash_emission(flashes: &[ImpactFlash], b: [f32; 3]) -> f32 {
    let mut e = 0.0f32;
    for f in flashes {
        let dot = b[0] * f.center[0] + b[1] * f.center[1] + b[2] * f.center[2];
        if dot < f.cos_reach || f.angular_radius <= 0.0 {
            continue;
        }
        // The great-circle angle from the cross-product magnitude (|b x c| = sin angle), precise for these small
        // crater angles (dot >= cos_reach >= 0, so the angle is at most the reach, within a hemisphere).
        let cx = b[1] * f.center[2] - b[2] * f.center[1];
        let cy = b[2] * f.center[0] - b[0] * f.center[2];
        let cz = b[0] * f.center[1] - b[1] * f.center[0];
        let angle = (cx * cx + cy * cy + cz * cz).sqrt().clamp(0.0, 1.0).asin();
        let x = angle / f.angular_radius;
        let profile = if x <= 1.0 { 1.0 } else { 1.0 / (x * x * x) };
        e += f.intensity * profile;
    }
    e
}

/// The lat-lon surface fraction `(fu, fv)` of a BODY-frame direction: `fu` the longitude fraction in `[0, 1)`
/// (wrapping the meridian) and `fv` the latitude fraction in `[0, 1]`, the same `lon = u*2pi - pi`,
/// `lat = (0.5 - v)*pi` sphere map [`crater_uv_unit`] inverts. This is the coordinate the COARSE province field is
/// indexed in, so the Sample reads that field at the direction it is sampling. Display-only (Principle 10).
pub fn dir_to_latlon_fraction(dir: [Fixed; 3]) -> (f32, f32) {
    use std::f32::consts::PI;
    let x = dir[0].to_f64_lossy() as f32;
    let y = dir[1].to_f64_lossy() as f32;
    let z = dir[2].to_f64_lossy() as f32;
    let lat = y.clamp(-1.0, 1.0).asin();
    let lon = x.atan2(z);
    let u = ((lon + PI) / (2.0 * PI)).rem_euclid(1.0);
    let v = (0.5 - lat / PI).clamp(0.0, 1.0);
    (u, v)
}

/// Bilinearly sample a coarse per-province `field` (one value per province, row-major `pcols` by `prows`) at a
/// normalized surface coordinate `(fu, fv)`, treating each province as a sample at its cell centre. Longitude
/// WRAPS (the globe is periodic east to west) and latitude CLAMPS (the poles), so the coarse DERIVED province
/// field reads as a smooth heightfield across province boundaries. A display resampling of the derived field
/// (Principle 10), never fabricated content.
///
/// The interpolation weight is carried at a 1/4096 quantization ([`PROVINCE_LERP_STEPS`]), so the implemented
/// surface is a fine staircase about the mathematical bilinear one. That step is far below any cache cell the
/// render samples at, so it is invisible in the picture, but it is the floor the analytic gradient's numerical
/// twin measures against (see [`SurfaceField::gradient`]).
pub fn sample_province_field(
    field: &[Fixed],
    pcols: usize,
    prows: usize,
    fu: f32,
    fv: f32,
) -> Fixed {
    let pc = pcols as i64;
    let pr = prows as i64;
    let gx = fu * pc as f32 - 0.5;
    let gy = fv * pr as f32 - 0.5;
    let x0 = gx.floor();
    let y0 = gy.floor();
    let tx = gx - x0;
    let ty = gy - y0;
    let x0i = x0 as i64;
    let y0i = y0 as i64;
    let at = |xi: i64, yi: i64| -> Fixed {
        let x = xi.rem_euclid(pc) as usize; // wrap longitude
        let y = yi.clamp(0, pr - 1) as usize; // clamp latitude
        field.get(y * pcols + x).copied().unwrap_or(Fixed::ZERO)
    };
    let lerp_fx = |a: Fixed, b: Fixed, t: f32| -> Fixed {
        let tf = Fixed::from_ratio(
            (t.clamp(0.0, 1.0) * PROVINCE_LERP_STEPS as f32) as i64,
            PROVINCE_LERP_STEPS,
        );
        b.checked_sub(a)
            .and_then(|d| d.checked_mul(tf))
            .and_then(|d| a.checked_add(d))
            .unwrap_or(a)
    };
    let top = lerp_fx(at(x0i, y0i), at(x0i + 1, y0i), tx);
    let bot = lerp_fx(at(x0i, y0i + 1), at(x0i + 1, y0i + 1), tx);
    lerp_fx(top, bot, ty)
}

/// A POLE-AWARE, SMOOTHER display resample of a coarse per-province `field` (one value per province, row-major
/// `pcols` by `prows`), the sibling of [`sample_province_field`] for the CACHE-BUILD / derivation callers that want
/// a smooth field rather than the analytic-gradient-matched bilinear one. It differs from the plain sampler in two
/// display-only ways, each aimed at one of the coarse lat-lon field's two visible artifacts:
///
/// - SMOOTHER MESH. The interpolation weights pass through a quintic smootherstep (Hermite, zero-sloped at the cell
///   edges, so the interpolant is C1 there) rather than staying linear (C0), so the boxy province-cell mesh softens
///   instead of showing its facets.
/// - NO POLE SPOKES. Near a pole the lat-lon columns converge to a point, so the plain bilinear reads radial spokes
///   there; this blends the sample toward the poleward-most province row's LONGITUDE MEAN as the pole is approached
///   (over the width of that one row), collapsing the converging columns into a smooth cap.
///
/// It is NOT the analytic partner of [`SurfaceField::gradient`] (the smootherstep is not the derivative the gradient
/// takes), so it is used ONLY where no surface NORMAL is read off the same field: the lava glow
/// ([`super::derive_province_lava`]), a self-emitted colour field with no gradient. The height cache keeps the plain
/// [`sample_province_field`] so its analytic gradient (the pixel-shader normal) stays that field's exact derivative,
/// the standing numerical twin. Deterministic fixed-point with an f32 weight (the same 1/[`PROVINCE_LERP_STEPS`]
/// quantization the plain sampler carries), so a parallel cache build is thread-count-independent. Display-only
/// (Principle 10), a resampling of the DERIVED field, never fabricated content.
pub fn sample_province_field_smooth(
    field: &[Fixed],
    pcols: usize,
    prows: usize,
    fu: f32,
    fv: f32,
) -> Fixed {
    if pcols == 0 || prows == 0 || field.len() < pcols * prows {
        return Fixed::ZERO;
    }
    let pc = pcols as i64;
    let pr = prows as i64;
    let gx = fu * pc as f32 - 0.5;
    let gy = fv * pr as f32 - 0.5;
    let x0 = gx.floor();
    let y0 = gy.floor();
    // The quintic smootherstep 6t^5 - 15t^4 + 10t^3 of the cell-local fraction: zero-sloped at both cell edges, so
    // the interpolant meets its neighbours smoothly (the mesh softens). A display weight, quantized by the lerp below.
    let smootherstep = |t: f32| -> f32 {
        let t = t.clamp(0.0, 1.0);
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    };
    let tx = smootherstep(gx - x0);
    let ty = smootherstep(gy - y0);
    let x0i = x0 as i64;
    let y0i = y0 as i64;
    let at = |xi: i64, yi: i64| -> Fixed {
        let x = xi.rem_euclid(pc) as usize; // wrap longitude
        let y = yi.clamp(0, pr - 1) as usize; // clamp latitude
        field.get(y * pcols + x).copied().unwrap_or(Fixed::ZERO)
    };
    let lerp_fx = |a: Fixed, b: Fixed, t: f32| -> Fixed {
        let tf = Fixed::from_ratio(
            (t.clamp(0.0, 1.0) * PROVINCE_LERP_STEPS as f32) as i64,
            PROVINCE_LERP_STEPS,
        );
        b.checked_sub(a)
            .and_then(|d| d.checked_mul(tf))
            .and_then(|d| a.checked_add(d))
            .unwrap_or(a)
    };
    let top = lerp_fx(at(x0i, y0i), at(x0i + 1, y0i), tx);
    let bot = lerp_fx(at(x0i, y0i + 1), at(x0i + 1, y0i + 1), tx);
    let value = lerp_fx(top, bot, ty);
    // THE POLE CAP: within the poleward-most province row, blend toward that row's longitude MEAN so the converging
    // lat-lon columns read as a smooth cap rather than radial spokes. The blend ramps (smootherstep) from the full
    // mean AT the pole to none one province row in. `row_mean` averages a pole row across its longitudes.
    let row_mean = |row: i64| -> Fixed {
        let base = (row.clamp(0, pr - 1) as usize) * pcols;
        let mut sum = Fixed::ZERO;
        for c in 0..pcols {
            sum = sum
                .checked_add(field.get(base + c).copied().unwrap_or(Fixed::ZERO))
                .unwrap_or(sum);
        }
        sum.checked_div(Fixed::from_int(pcols as i32))
            .unwrap_or(sum)
    };
    let pole_band = 1.0 / prows as f32; // one province row, in fv units
    if fv < pole_band {
        let w = smootherstep(1.0 - fv / pole_band); // 1 at the north pole, 0 one row in
        lerp_fx(value, row_mean(0), w)
    } else if fv > 1.0 - pole_band {
        let w = smootherstep((fv - (1.0 - pole_band)) / pole_band); // 0 one row in, 1 at the south pole
        lerp_fx(value, row_mean(pr - 1), w)
    } else {
        value
    }
}

/// The quantization of the province-field interpolation weight (1/4096 of a cell): a display REPRESENTATION step
/// carried over from the pre-existing sample path, not a physical value. It sets the floor below which the
/// implemented height stops varying smoothly, and hence the smallest finite-difference step the analytic
/// gradient's numerical twin can resolve before it reads staircase noise rather than truncation error.
const PROVINCE_LERP_STEPS: i64 = 4096;

/// THE SAMPLE FUNCTION, in the analytic form both the cache build and the shading read it: the surface is the
/// SUPERPOSITION `Sample(dir) = airy(province crust) + sum(crater-row stamps)`, each layer at its own derived
/// scale (`docs/working/CONSOLIDATED_SURFACE_PIPELINE.md`, stages 4 and 5). It carries no raster of its own: the
/// display tile grid is only the memoized cache of [`SurfaceField::height_km`], so a finer cache resolves finer
/// craters from the SAME row list.
///
/// Its reason for existing beyond the height is [`SurfaceField::gradient`]: because every layer is ANALYTIC, the
/// surface normal is the ANALYTIC GRADIENT of the same superposition, evaluated at query time with no baking and
/// no finite differencing. The shading is then a derivative of the canonical field rather than of a resampling of
/// it, which is derive-first in the renderer.
///
/// THE FLEXURAL MIDDLE IS ABSENT, and its absence is loud rather than papered over. The band between the coarse
/// provinces and the fine crater rows (the moats, peripheral bulges, and terrain-scale swells a human reads as
/// terrain) is the elastic plate's flexure under its load list, and the kernel for it is BUILT and dormant
/// ([`civsim_physics::flexure`]). It is not wired here because its rigidity `D = E T_e^3 / (12 (1 - nu^2))` needs
/// the elastic lid thickness `T_e`, and NOTHING BUILT DERIVES `T_e` today: the deep-time thermal state carries one
/// lumped temperature per column ([`civsim_sim::geodynamics::ColumnState`]) and no depth-resolved geotherm, so
/// there is no profile against which a mechanical lid base can be located. Authoring a lid thickness, or pinning
/// it to the thermal boundary layer through an invented ratio, would cross the value-authoring line, so the layer
/// is left out and the gap is surfaced. When `T_e` derives, the flexure enters here as one more analytic layer:
/// its deflection adds to [`SurfaceField::height_km`] and its Green's function's analytic derivative adds to
/// [`SurfaceField::gradient`], with no change to either function's shape.
///
/// Display-only, one-way canon -> pixels (Principle 10): the canon is the province field and the crater rows; this
/// is their rendering form.
pub struct SurfaceField<'a> {
    /// The coarse DERIVED province crust-thickness field (kilometres), row-major `pcols` by `prows`, the field the
    /// Airy flotation floats. The PHYSICAL grid is the derived one; the cache samples it.
    pub thickness_km: &'a [Fixed],
    /// The province field's column count.
    pub pcols: usize,
    /// The province field's row count.
    pub prows: usize,
    /// The DERIVED crust density the province crust floats at.
    pub crust_density: Fixed,
    /// The DERIVED mantle density the crust floats ON.
    pub mantle_density: Fixed,
    /// The crater rows prepared as analytic stamps ([`crater_stamps`]); empty for a crust-only field.
    pub stamps: &'a [CraterStamp],
    /// The body's DERIVED radius in KILOMETRES, the arc length the gradient's rise is taken over (the tile
    /// elevations are in kilometres, so the radius is in the same unit and the slope comes out dimensionless).
    pub radius_km: f32,
}

impl SurfaceField<'_> {
    /// THE SAMPLE at a BODY-frame direction: `airy(province crust) + crater-row stamps`, in kilometres. The
    /// province crust is bilinearly resampled at this direction and floated by Airy isostasy
    /// ([`civsim_physics::geodynamics::airy_isostatic_elevation`], so a thicker-crust province stands higher),
    /// then the analytic crater stamp adds the bowls and ejecta rims covering the point ([`crater_relief_km`],
    /// surface topography rather than isostatically-compensated crust). `None` if the flotation does not resolve
    /// (a non-positive mantle density) or a fixed-point intermediate leaves the window, never a fabricated height.
    /// Deterministic fixed-point (Principle 3); pure over immutable inputs, so a parallel cache build is
    /// thread-count-independent.
    pub fn height_km(&self, dir: [Fixed; 3]) -> Option<Fixed> {
        let (fu, fv) = dir_to_latlon_fraction(dir);
        let thickness = sample_province_field(self.thickness_km, self.pcols, self.prows, fu, fv);
        let airy = civsim_physics::geodynamics::airy_isostatic_elevation(
            self.crust_density,
            self.mantle_density,
            thickness,
        )?;
        airy.checked_add(crater_relief_km(self.stamps, dir))
    }

    /// THE ANALYTIC GRADIENT of the Sample superposition at a BODY-frame direction `b`, returned as the
    /// TANGENT-PLANE gradient vector: a 3-vector lying in the tangent plane at `b` whose components are
    /// DIMENSIONLESS slope (kilometres of rise per kilometre of arc, at the field's own amplitude, no vertical
    /// exaggeration). The terrain normal is then `normalize(b - gradient(b))`, the heightfield normal
    /// `(-grad(h), 1)` carried into body coordinates ([`hillshade_normal`]).
    ///
    /// Each layer contributes its OWN derivative, summed, so no finite differencing and no baking is involved:
    ///
    /// - THE PROVINCE LAYER. The Airy elevation is LINEAR in the crust thickness
    ///   (`airy = T * (rho_m - rho_c) / rho_m`), so its gradient is the buoyant fraction times the gradient of the
    ///   bilinear interpolant through the province thicknesses ([`province_grad_uv`]), carried from per-unit-`u`
    ///   and per-unit-`v` into the east/north frame through the body radius (east arc `2 pi R cos(lat)`, south arc
    ///   `pi R`). The `cos(lat)` is floored at the poleward-most PROVINCE row's centre, the grid's own resolution
    ///   limit, never an authored constant.
    /// - THE CRATER LAYER. Each stamp within its reach cone contributes the derivative of the crater law's own
    ///   shape in rim-radii `x`: inside the rim the excavation paraboloid `-h (1 - x^2)` differentiates to
    ///   `df/dx = 2 h x`, outside it the McGetchin ejecta blanket `(h/4) x^-3` differentiates to
    ///   `df/dx = -3 h / (4 x^4)`. The chain rule carries `x` to arc length through the crater's angular radius
    ///   and the body radius (`df/ds = (df/dx) / (alpha_c R)`), and the gradient points along the great circle
    ///   AWAY from the crater centre, which is regular everywhere (at the centre `df/dx = 0`, so the vanishing
    ///   direction carries a vanishing slope).
    ///
    /// HONEST LIMITS, both of them real and neither papered over. The composed surface is only PIECEWISE smooth,
    /// so at two kinds of place no gradient exists and this returns the one-sided value the layer's own form
    /// gives: at a crater RIM (`x = 1`, where the bowl's `+2h` slope meets the blanket's `-3h/4`, which is what
    /// makes it a rim) and across a province CELL EDGE (where the bilinear interpolant's gradient steps). A
    /// finite-difference twin straddling either reads a chord and cannot converge; that is a property of the
    /// surface, not an error in the derivative. And the east gradient inherits the province field's own POLE
    /// SINGULARITY, because that field is still a lat-lon grid whose columns converge at the pole; the guard
    /// bounds it, and the field's migration off lat-lon is the chartered slice 9.
    ///
    /// Display-only `f32` math (Principle 10), the same tier the Lambert term it feeds already uses.
    pub fn gradient(&self, b: [f32; 3]) -> [f32; 3] {
        use std::f32::consts::PI;
        if self.radius_km <= 0.0 {
            return [0.0, 0.0, 0.0];
        }
        let mut g = [0.0f32; 3];
        // THE PROVINCE LAYER: the buoyant fraction times the bilinear thickness gradient, in the east/north frame.
        if self.pcols > 0 && self.prows > 0 && !self.thickness_km.is_empty() {
            let mantle = self.mantle_density.to_f64_lossy() as f32;
            if mantle > 0.0 {
                let buoyant = (mantle - self.crust_density.to_f64_lossy() as f32) / mantle;
                let lat = b[1].clamp(-1.0, 1.0).asin();
                let lon = b[0].atan2(b[2]);
                let (sin_lon, cos_lon) = lon.sin_cos();
                let sin_lat = lat.sin();
                let cos_lat = lat.cos();
                let east = [cos_lon, 0.0, -sin_lon];
                let north = [-sin_lat * sin_lon, cos_lat, -sin_lat * cos_lon];
                let (u, v) = body_to_uv(b);
                let (dt_du, dt_dv) =
                    province_grad_uv(self.thickness_km, self.pcols, self.prows, u, v);
                // The province grid's OWN poleward-most row centre: below that latitude it resolves no distinct
                // columns, so the east gradient there is degenerate. Grid-derived, not an authored constant.
                let pole_floor = (PI / (2.0 * self.prows as f32)).sin();
                let cos_lat_denom = cos_lat.abs().max(pole_floor);
                let g_east = buoyant * dt_du / (2.0 * PI * self.radius_km * cos_lat_denom);
                // v increases SOUTHWARD, so the northward slope negates the per-unit-v gradient over the meridian.
                let g_north = -buoyant * dt_dv / (PI * self.radius_km);
                for i in 0..3 {
                    g[i] += g_east * east[i] + g_north * north[i];
                }
            }
        }
        // THE CRATER LAYER: each stamp's own analytic profile derivative, along the great circle away from it.
        for s in self.stamps {
            let c = [
                s.center[0].to_f64_lossy() as f32,
                s.center[1].to_f64_lossy() as f32,
                s.center[2].to_f64_lossy() as f32,
            ];
            let dot = b[0] * c[0] + b[1] * c[1] + b[2] * c[2];
            if dot < s.cos_reach.to_f64_lossy() as f32 {
                continue; // outside the reach cone, the same cheap bound the stamp uses
            }
            let alpha_c = s.angular_radius.to_f64_lossy() as f32;
            if alpha_c <= 0.0 {
                continue;
            }
            let angle = dot.clamp(-1.0, 1.0).acos(); // the great-circle central angle
            let x = angle / alpha_c;
            let h = s.depth_km.to_f64_lossy() as f32;
            // The crater law's own shape, differentiated in rim-radii x.
            let df_dx = if x <= 1.0 {
                2.0 * h * x // the excavation paraboloid -h(1 - x^2)
            } else {
                -3.0 * h / (4.0 * x * x * x * x) // the McGetchin blanket (h/4) x^-3
            };
            // Chain rule to arc length: x = angle / alpha_c, s = R * angle.
            let df_ds = df_dx / (alpha_c * self.radius_km);
            // The unit tangent at b pointing AWAY from the crater centre (s increases away from it).
            let toward = [c[0] - dot * b[0], c[1] - dot * b[1], c[2] - dot * b[2]];
            let m = (toward[0] * toward[0] + toward[1] * toward[1] + toward[2] * toward[2]).sqrt();
            if m <= 0.0 {
                continue; // at the crater centre the profile is flat (df/dx = 0), so no contribution
            }
            for i in 0..3 {
                g[i] += df_ds * (-toward[i] / m);
            }
        }
        g
    }
}

/// The ANALYTIC gradient of the bilinear interpolant through a coarse province `field`, at surface coordinate
/// `(u, v)`, returned as the field's unit per-unit-`u` (eastward) and per-unit-`v` (southward) rate. The
/// interpolant is the straight ramp between province-cell centres (longitude wraps, latitude clamps at the poles),
/// the least-committed surface through the DERIVED values, and this is its exact derivative, so it invents no
/// relief. Its gradient STEPS across a cell edge (the interpolant is only piecewise smooth), which the caller's
/// honest limits name. Display-only (Principle 10).
fn province_grad_uv(field: &[Fixed], pcols: usize, prows: usize, u: f32, v: f32) -> (f32, f32) {
    if pcols == 0 || prows == 0 || field.is_empty() {
        return (0.0, 0.0);
    }
    let pc = pcols as i64;
    let pr = prows as i64;
    // Cell-centre-aligned fractional coordinates: province (r, c) is centred at ((c+0.5)/pcols, (r+0.5)/prows).
    let gx = u.rem_euclid(1.0) * pc as f32 - 0.5;
    let gy = v.clamp(0.0, 1.0) * pr as f32 - 0.5;
    let x0 = gx.floor();
    let y0 = gy.floor();
    let tx = (gx - x0).clamp(0.0, 1.0);
    let ty = (gy - y0).clamp(0.0, 1.0);
    let x0i = x0 as i64;
    let y0i = y0 as i64;
    let at = |xi: i64, yi: i64| -> f32 {
        let x = xi.rem_euclid(pc) as usize; // wrap longitude
        let y = yi.clamp(0, pr - 1) as usize; // clamp latitude
        field
            .get(y * pcols + x)
            .copied()
            .unwrap_or(Fixed::ZERO)
            .to_f64_lossy() as f32
    };
    let e00 = at(x0i, y0i);
    let e10 = at(x0i + 1, y0i);
    let e01 = at(x0i, y0i + 1);
    let e11 = at(x0i + 1, y0i + 1);
    // The bilinear partials in cell-fraction units, scaled to per-unit-u and per-unit-v.
    let df_dtx = (e10 - e00) * (1.0 - ty) + (e11 - e01) * ty;
    let df_dty = (e01 - e00) * (1.0 - tx) + (e11 - e10) * tx;
    (df_dtx * pc as f32, df_dty * pr as f32)
}

/// The DERIVED tile relief at BODY-frame direction `b` (its lat-lon coordinate `(u, v)` precomputed for the LatLon
/// path), under the cache parameterization `param`: the cube-sphere cell for a `CubeSphere` cache, the equirectangular
/// cell for a `LatLon` cache (the same pick [`pick_surface_tile`] inverts). `None` for an empty field, so the caller
/// falls back to a stand-in. Display-only.
fn sample_derived_tile(
    tiles: &[DerivedTile],
    param: SurfaceParam,
    b: [f32; 3],
    u: f32,
    v: f32,
) -> Option<DerivedTile> {
    surface_cell_index(param, b, u, v, tiles.len()).map(|i| tiles[i])
}

/// The self-emitted LAVA GLOW of one surface tile: the incandescent colour of the tile's DERIVED interior
/// temperature ([`blackbody_rgb`], cooler melt deep-red, hotter melt orange-to-yellow) paired with a melt-glow
/// INTENSITY in `[0, 1]` (the DERIVED melt fraction, zero for solid crust below the world's own solidus, rising as
/// the interior climbs above it). Unlike the sun-lit crust albedo, this EMITS: [`draw_globe`] ADDS `emission *
/// intensity` to the shaded tile, so a molten tile is bright on the NIGHT side too (lava glows in the dark, the
/// giveaway that it radiates rather than reflects). A per-display-tile field the caller derives, laid out to match
/// the tiles ([`sample_glow`] wraps it onto the sphere the same way [`sample_derived_tile`] wraps the relief).
/// Display-only, one-way canon -> pixels (Principle 10).
#[derive(Clone, Copy, Default)]
pub struct LavaGlow {
    /// The incandescent emission colour: the blackbody of the tile's DERIVED interior temperature.
    pub emission: Rgb,
    /// The melt-glow intensity in `[0, 1]`: the DERIVED melt fraction (zero below the world's solidus, no glow).
    pub intensity: f32,
}

/// Sample the per-cell [`LavaGlow`] field at BODY-frame direction `b` (its `(u, v)` precomputed for the LatLon path),
/// the SAME cache index [`sample_derived_tile`] uses, so the glow registers with the crust cell it rides on. `None`
/// for an empty field (a world with no molten record, so the caller adds no emission and the render is byte-identical
/// there). Display-only.
fn sample_glow(
    glow: &[LavaGlow],
    param: SurfaceParam,
    b: [f32; 3],
    u: f32,
    v: f32,
) -> Option<LavaGlow> {
    surface_cell_index(param, b, u, v, glow.len()).map(|i| glow[i])
}

/// The DERIVED elevation field's finite-difference gradient at surface coordinate (u, v), returned as metres of
/// elevation per unit-u (eastward) and per unit-v (southward) so the caller can convert it to a true physical slope
/// with the body radius. It is the analytic gradient of the BILINEAR interpolant through the four surrounding
/// tile-centre elevations (longitude wraps, latitude clamps at the poles), so the shading grades smoothly across a
/// cell rather than jumping at each tile edge; it invents no relief, since between two samples the interpolant is the
/// straight ramp, the least-committed surface through the DERIVED heights. Display-only (Principle 10).
fn elevation_grad_uv(
    tiles: &[DerivedTile],
    cols: usize,
    rows: usize,
    u: f32,
    v: f32,
) -> (f32, f32) {
    if cols == 0 || rows == 0 || tiles.is_empty() {
        return (0.0, 0.0);
    }
    // Cell-centre-aligned fractional coordinates: tile (r, c) is centred at ((c + 0.5)/cols, (r + 0.5)/rows).
    let fu = u.rem_euclid(1.0) * cols as f32 - 0.5;
    let fv = v.clamp(0.0, 1.0) * rows as f32 - 0.5;
    let u0 = fu.floor();
    let v0 = fv.floor();
    let tu = (fu - u0).clamp(0.0, 1.0);
    let tv = (fv - v0).clamp(0.0, 1.0);
    let c0 = (u0 as i64).rem_euclid(cols as i64) as usize; // longitude wraps
    let c1 = (u0 as i64 + 1).rem_euclid(cols as i64) as usize;
    let r0 = (v0 as i64).clamp(0, rows as i64 - 1) as usize; // latitude clamps at the poles
    let r1 = (v0 as i64 + 1).clamp(0, rows as i64 - 1) as usize;
    let e = |r: usize, c: usize| -> f32 {
        let idx = (r * cols + c).min(tiles.len() - 1);
        tiles[idx].elevation.to_f64_lossy() as f32
    };
    let e00 = e(r0, c0);
    let e10 = e(r0, c1);
    let e01 = e(r1, c0);
    let e11 = e(r1, c1);
    // The bilinear partials in cell-fraction units, scaled to per-unit-u and per-unit-v.
    let dh_dtu = (e10 - e00) * (1.0 - tv) + (e11 - e01) * tv;
    let dh_dtv = (e01 - e00) * (1.0 - tu) + (e11 - e10) * tu;
    (dh_dtu * cols as f32, dh_dtv * rows as f32)
}

/// The sun-direction HILLSHADE normal at a surface point: the sphere normal `b` (body frame) tilted by the DERIVED
/// terrain slope, so slopes facing the star are bright and slopes facing away are dark. The slope is dimensionless
/// (km rise over arc length at the field's OWN amplitude, no vertical exaggeration) and perturbs the normal as
/// `Up - grad(h)` (the heightfield normal `(-grad(h), 1)` in the local tangent frame), carried into body
/// coordinates and renormalised.
///
/// The slope comes from one of two sources, in order of fidelity:
///
/// - THE ANALYTIC GRADIENT of the Sample superposition ([`SurfaceField::gradient`]), when the caller supplies the
///   `field`. Each layer contributes its own derivative, evaluated at query time, so the normal is a derivative of
///   the CANONICAL field rather than of a resampling of it: no baking, no finite differencing, and the shading
///   carries relief the cache's own cell spacing cannot resolve. This is the derived-planet globe's path.
/// - THE CACHE'S FINITE DIFFERENCE, when no field is supplied (the living-world / fixture globe, byte-identical to
///   before): for a `LatLon` cache the gradient of the bilinear interpolant through the cached heights
///   ([`elevation_grad_uv`]) in the east/north frame, converted through the body radius (east arc
///   `2*pi*R*cos(lat)`, south arc `pi*R`, `cos(lat)` floored at the poleward row to guard the polar singularity);
///   for a `CubeSphere` cache the central difference of the cached height over one cell's angle along a tangent
///   pair REGULAR everywhere (the east/north frame is not: it collapses at the poles, and using it would put the
///   pole singularity back into the shading of a cache built to have none).
///
/// The tilt is basis-independent (the tangent slopes are the components of ONE gradient vector in the tangent
/// plane), so every path computes the same physical quantity: the real slope tilts the normal, at the field's own
/// amplitude. Display-only math (Principle 10).
fn hillshade_normal(
    b: [f32; 3],
    tiles: &[DerivedTile],
    param: SurfaceParam,
    u: f32,
    v: f32,
    radius_m: f32,
    field: Option<&SurfaceField>,
) -> [f32; 3] {
    use std::f32::consts::{FRAC_PI_2, PI};
    if radius_m <= 0.0 {
        return b;
    }
    // THE ANALYTIC PATH: the gradient of the Sample superposition itself, each layer's own derivative summed.
    if let Some(f) = field {
        let g = f.gradient(b);
        return normalize3([b[0] - g[0], b[1] - g[1], b[2] - g[2]]);
    }
    // The tile elevation is in KILOMETRES (the Airy isostatic derivation's unit), while the body radius arrives in
    // metres, so the radius is taken in the SAME km unit to form a dimensionless slope. The 1000 is the km-to-m unit
    // factor, not a physics tuneable; getting it wrong makes the relief read 1000x too flat.
    let radius_km = radius_m / 1000.0;
    if radius_km <= 0.0 {
        return b;
    }
    match param {
        SurfaceParam::LatLon { cols, rows } => {
            if cols == 0 || rows == 0 {
                return b;
            }
            // The lat-lon frame: east and north from the sphere coordinate, with the cos(lat) pole guard the
            // equirectangular grid needs. Unchanged from before the cube-sphere migration (byte-identical).
            let lat = b[1].clamp(-1.0, 1.0).asin();
            let lon = b[0].atan2(b[2]);
            let (sin_lon, cos_lon) = lon.sin_cos();
            let sin_lat = lat.sin();
            let cos_lat = lat.cos();
            let east = [cos_lon, 0.0, -sin_lon];
            let north = [-sin_lat * sin_lon, cos_lat, -sin_lat * cos_lon];
            let (dz_du, dz_dv) = elevation_grad_uv(tiles, cols, rows, u, v);
            // Floor cos(lat) at the poleward-most tile-row centre: below that latitude the grid resolves no distinct
            // cells, so the east gradient there is degenerate. Grid-derived, not an authored constant.
            let pole_floor = (PI / (2.0 * rows as f32)).sin();
            let cos_lat_denom = cos_lat.abs().max(pole_floor);
            let g_east = dz_du / (2.0 * PI * radius_km * cos_lat_denom);
            // v increases southward, so the northward slope negates the per-unit-v gradient over the meridian arc.
            let g_north = -dz_dv / (PI * radius_km);
            normalize3([
                b[0] - g_east * east[0] - g_north * north[0],
                b[1] - g_east * east[1] - g_north * north[1],
                b[2] - g_east * east[2] - g_north * north[2],
            ])
        }
        SurfaceParam::CubeSphere { face_res } => {
            if face_res == 0 {
                return b;
            }
            // A tangent basis REGULAR at every direction. The east/north frame above collapses at the poles (there
            // every direction is south, and the frame spins), which is the very singularity the cube-sphere cache
            // exists to retire, so the cube path must not reintroduce it in the shading. Instead take the world axis
            // least aligned with `b` and build an orthonormal tangent pair from it: the tilt `Up - grad(h)` is
            // basis-INDEPENDENT (the two tangent slopes are the components of ONE gradient vector in the tangent
            // plane), so any orthonormal pair yields the same physical normal, and this one never degenerates.
            let (ax, ay, az) = (b[0].abs(), b[1].abs(), b[2].abs());
            let axis = if ax <= ay && ax <= az {
                [1.0, 0.0, 0.0]
            } else if ay <= az {
                [0.0, 1.0, 0.0]
            } else {
                [0.0, 0.0, 1.0]
            };
            let t1 = normalize3(cross3(axis, b));
            let t2 = cross3(b, t1);
            // Central-difference the cached elevation along the tangent pair by ~one cube cell's angle (a face spans
            // pi/2 across face_res cells), so the slope is the real cell-to-cell grade over the true arc length, with
            // no cos(lat) factor anywhere. Each sample reads the cell the SAME way the base colour does.
            let dtheta = FRAC_PI_2 / face_res as f32;
            let height_km = |dir: [f32; 3]| -> f32 {
                match surface_cell_index(param, normalize3(dir), 0.0, 0.0, tiles.len()) {
                    Some(i) => tiles[i].elevation.to_f64_lossy() as f32,
                    None => 0.0,
                }
            };
            let step = |sign: f32, axis: [f32; 3]| {
                [
                    b[0] + sign * dtheta * axis[0],
                    b[1] + sign * dtheta * axis[1],
                    b[2] + sign * dtheta * axis[2],
                ]
            };
            let denom = 2.0 * dtheta * radius_km;
            let g1 = (height_km(step(1.0, t1)) - height_km(step(-1.0, t1))) / denom;
            let g2 = (height_km(step(1.0, t2)) - height_km(step(-1.0, t2))) / denom;
            normalize3([
                b[0] - g1 * t1[0] - g2 * t2[0],
                b[1] - g1 * t1[1] - g2 * t2[1],
                b[2] - g1 * t1[2] - g2 * t2[2],
            ])
        }
    }
}

/// The cross product of two 3-vectors. Non-canon display math (the hillshade's tangent basis).
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The globe's viewing orientation for the derived-surface explorer: a longitude spin about the polar axis and a
/// latitude tilt, both in radians. Panning the derived viewer changes these so the far side of the sphere rotates
/// into view and the whole surface becomes reachable. A display-only orientation (observability non-canon): it steers
/// which surface coordinate each screen pixel samples and writes no canonical state (Principle 10).
#[derive(Clone, Copy, Debug)]
pub struct GlobeOrientation {
    /// Longitude offset (radians); the caller wraps it, so a full spin reaches every meridian.
    pub rot_lon: f32,
    /// Latitude offset (radians); the caller clamps it away from the poles to avoid the projection singularity.
    pub rot_lat: f32,
}

impl GlobeOrientation {
    /// The unrotated orientation: the sphere is sampled straight on, so the render is identical to the
    /// pre-rotation globe (the headless commands and the living-world globe use this).
    pub const IDENTITY: Self = Self {
        rot_lon: 0.0,
        rot_lat: 0.0,
    };
}

/// Rotate a 3-vector about the x axis (screen-horizontal) by `angle` radians. Non-canon display math.
fn rot_x(p: [f32; 3], angle: f32) -> [f32; 3] {
    let (s, c) = angle.sin_cos();
    [p[0], c * p[1] - s * p[2], s * p[1] + c * p[2]]
}

/// Rotate a 3-vector about the y axis (the polar / up axis) by `angle` radians. Non-canon display math.
fn rot_y(p: [f32; 3], angle: f32) -> [f32; 3] {
    let (s, c) = angle.sin_cos();
    [c * p[0] + s * p[2], p[1], -s * p[0] + c * p[2]]
}

/// Carry a view-space point (x right, y up, z toward the viewer) on the unit sphere into the globe's BODY frame,
/// undoing the orientation: `B = R_y(-rot_lon) * R_x(-rot_lat) * P`. So a screen pixel maps to the surface point the
/// current orientation has rotated under it. At [`GlobeOrientation::IDENTITY`] this is the identity (both rotations
/// are by 0, so the body point equals the view point exactly). Non-canon display math.
fn view_to_body(p: [f32; 3], o: GlobeOrientation) -> [f32; 3] {
    rot_y(rot_x(p, -o.rot_lat), -o.rot_lon)
}

/// Carry a globe BODY-frame point into view space under the orientation: `P = R_x(rot_lat) * R_y(rot_lon) * B`, the
/// forward of [`view_to_body`]. Used to project a surface (u, v) back onto the screen for the highlight outline. Non-canon.
fn body_to_view(b: [f32; 3], o: GlobeOrientation) -> [f32; 3] {
    rot_x(rot_y(b, o.rot_lon), o.rot_lat)
}

/// The surface coordinate (u, v) of a BODY-frame point: longitude about the polar (y) axis mapped to `u` in [0, 1)
/// (wrapping the meridian) and latitude to `v` in [0, 1], matching [`draw_globe`]'s sphere map so the pick and the
/// paint agree. Non-canon display math.
fn body_to_uv(b: [f32; 3]) -> (f32, f32) {
    use std::f32::consts::PI;
    let lat = b[1].clamp(-1.0, 1.0).asin(); // -pi/2..pi/2
    let lon = b[0].atan2(b[2]); // -pi..pi
    let u = ((lon + PI) / (2.0 * PI)).rem_euclid(1.0);
    let v = (0.5 - lat / PI).clamp(0.0, 1.0);
    (u, v)
}

/// The BODY-frame unit point of a surface coordinate (u, v), the inverse of [`body_to_uv`]. Used to project a tile's
/// (u, v) corners forward onto the screen for the highlight box. Non-canon display math.
fn uv_to_body(u: f32, v: f32) -> [f32; 3] {
    use std::f32::consts::PI;
    let lon = u * 2.0 * PI - PI;
    let lat = (0.5 - v) * PI;
    [lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos()]
}

/// The parameterization of the surface SAMPLE CACHE: how the flat `tiles` slice (and the aligned lava-glow field)
/// maps to directions on the sphere. `LatLon` is the legacy equirectangular grid (`cols` by `rows`), whose cells
/// PINCH at the poles (their solid angle collapses to zero as `cos(lat) -> 0`), wasting the budget there and under-
/// resolving the equator. `CubeSphere` is the six-face equi-angular cube projection: each face is a `face_res` by
/// `face_res` grid, the cells near-uniform in solid angle (they vary by only about a factor of `sqrt(2)` across a
/// face and there is NO pole singularity). The living-world / fixture globe keeps `LatLon` (byte-identical); the
/// DERIVED-planet globe uses `CubeSphere`. Display-only: this re-parameterizes WHERE the Sample height function is
/// read, never the physics it reads (Principle 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceParam {
    /// The equirectangular grid: `cols` columns of longitude by `rows` rows of latitude (row-major). Pole-pinched.
    LatLon { cols: usize, rows: usize },
    /// The equi-angular cube-sphere: six faces, each `face_res` by `face_res`, face-major
    /// (`index = face * face_res * face_res + t_row * face_res + s_col`). No pole pinch.
    CubeSphere { face_res: usize },
}

/// The per-face basis of the cube-sphere: for face `f`, carry a FACE-LOCAL direction `d = (dx, dy, dz)` (with `dz`
/// the outward-normal component) into the BODY frame. The six faces tile the sphere (dominant `+x, -x, +y, -y, +z,
/// -z`), and this rotation is the exact inverse of the axis selection in [`cube_dir_to_face_st`]. Public so the
/// cache builder can evaluate the Sample function at each cube cell's world direction. Fixed-point (Principle 3),
/// display-only re-parameterization (Principle 10).
pub fn cube_face_local_to_world_fixed(face: usize, d: [Fixed; 3]) -> [Fixed; 3] {
    let (dx, dy, dz) = (d[0], d[1], d[2]);
    match face {
        0 => [dz, dy, -dx],  // +x dominant
        1 => [-dz, dy, dx],  // -x
        2 => [dx, dz, -dy],  // +y
        3 => [dx, -dz, dy],  // -y
        4 => [dx, dy, dz],   // +z
        _ => [-dx, dy, -dz], // -z
    }
}

/// The f32 sibling of [`cube_face_local_to_world_fixed`], for the round-trip test.
#[cfg(test)]
fn cube_face_local_to_world_f32(face: usize, d: [f32; 3]) -> [f32; 3] {
    let (dx, dy, dz) = (d[0], d[1], d[2]);
    match face {
        0 => [dz, dy, -dx],
        1 => [-dz, dy, dx],
        2 => [dx, dz, -dy],
        3 => [dx, -dz, dy],
        4 => [dx, dy, dz],
        _ => [-dx, dy, -dz],
    }
}

/// The f32 EQUI-ANGULAR forward map (for the round-trip test; the render path needs only the inverse, and the cache
/// builder uses the fixed-point [`cube_face_local_to_world_fixed`] with the precomputed face-local directions). The
/// equi-angular map warps the face angle `alpha = (s - 1/2) * pi/2` (and `beta` from `t`) so equal steps in `(s, t)`
/// subtend near-equal solid angle (Ronchi, Iacono, Paolucci 1996, "The Cubed Sphere"), removing the gnomonic cube's
/// centre-to-corner area bias and the lat-lon grid's pole collapse. The face-local direction is
/// `(sin a cos b, cos a sin b, cos a cos b)` normalized (equal to `(tan a, tan b, 1)` normalized, but using only
/// sin/cos/sqrt so it needs no `tan`), then rotated into the body frame by the face basis.
#[cfg(test)]
fn cube_face_dir_f32(face: usize, s: f32, t: f32) -> [f32; 3] {
    use std::f32::consts::FRAC_PI_2;
    let (sa, ca) = ((s - 0.5) * FRAC_PI_2).sin_cos();
    let (sb, cb) = ((t - 0.5) * FRAC_PI_2).sin_cos();
    let lx = sa * cb;
    let ly = ca * sb;
    let lz = ca * cb;
    let n = (lx * lx + ly * ly + lz * lz).sqrt();
    let d = if n > 0.0 {
        [lx / n, ly / n, lz / n]
    } else {
        [0.0, 0.0, 1.0]
    };
    cube_face_local_to_world_f32(face, d)
}

/// The INVERSE cube-sphere map: the `(face, s, t)` of a BODY-frame direction `dir` (need not be unit), with `s, t` in
/// `[0, 1)`. The face is the axis of largest magnitude (with sign); the face-local `(dx, dy, dz)` is read off by the
/// exact inverse of `cube_face_local_to_world_f32`; and the equi-angular coordinates invert the warp,
/// `alpha = atan(dx / dz)`, `s = alpha / (pi/2) + 1/2`. A corner direction lands on a face by a deterministic first-
/// max tie-break. Display-only f32 math (Principle 10).
fn cube_dir_to_face_st(dir: [f32; 3]) -> (usize, f32, f32) {
    use std::f32::consts::FRAC_PI_2;
    let (x, y, z) = (dir[0], dir[1], dir[2]);
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    let (face, dx, dy, dz) = if ax >= ay && ax >= az {
        if x > 0.0 {
            (0usize, -z, y, x)
        } else {
            (1, z, y, -x)
        }
    } else if ay >= az {
        if y > 0.0 {
            (2, x, -z, y)
        } else {
            (3, x, z, -y)
        }
    } else if z > 0.0 {
        (4, x, y, z)
    } else {
        (5, -x, y, -z)
    };
    let inv = if dz != 0.0 { 1.0 / dz } else { 0.0 };
    let s = (dx * inv).atan() / FRAC_PI_2 + 0.5;
    let t = (dy * inv).atan() / FRAC_PI_2 + 0.5;
    (face, s.clamp(0.0, 0.999_9), t.clamp(0.0, 0.999_9))
}

/// The flat cache index of a BODY-frame direction `b` (with its lat-lon coordinate `(u, v)` precomputed by the
/// caller, used only by the [`SurfaceParam::LatLon`] path), under the cache parameterization `param`, for a field of
/// `field_len` cells. `None` for a degenerate cache (empty field, zero columns/rows or face resolution). For
/// `LatLon` this reproduces the former equirectangular cell pick exactly (byte-identical); for `CubeSphere` it maps
/// the direction to `(face, s, t)` and thence the face cell. Display-only (Principle 10).
fn surface_cell_index(
    param: SurfaceParam,
    b: [f32; 3],
    u: f32,
    v: f32,
    field_len: usize,
) -> Option<usize> {
    if field_len == 0 {
        return None;
    }
    match param {
        SurfaceParam::LatLon { cols, rows } => {
            if cols == 0 || rows == 0 {
                return None;
            }
            let cu = ((u.clamp(0.0, 0.999_9) * cols as f32) as usize).min(cols - 1);
            let cv = ((v.clamp(0.0, 0.999_9) * rows as f32) as usize).min(rows - 1);
            Some((cv * cols + cu).min(field_len - 1))
        }
        SurfaceParam::CubeSphere { face_res } => {
            if face_res == 0 {
                return None;
            }
            let (face, s, t) = cube_dir_to_face_st(b);
            let ci = ((s * face_res as f32) as usize).min(face_res - 1);
            let cj = ((t * face_res as f32) as usize).min(face_res - 1);
            Some((face * face_res * face_res + cj * face_res + ci).min(field_len - 1))
        }
    }
}

/// The display styling of the drawn sphere's surface: the DERIVED material tint (the crust's perceived colour under the
/// star, or `None` for the relief swatch) and an optional lat/lon TILE GRID `(cols, rows)` drawn onto the sphere as thin
/// seams, so the surface reads as an array of tiles the observer can drill into. Both are observability-non-canon
/// display choices (Principle 10); `default()` is no tint and no grid (the plain planet view).
#[derive(Clone, Copy, Default)]
pub struct SurfaceStyle {
    /// The DERIVED material colour to paint the surface (scaled per tile by relief shading), or `None` for the swatch.
    pub tint: Option<Rgb>,
    /// The display tile grid `(cols, rows)` to overlay as seams, or `None` for a smooth (un-gridded) sphere.
    pub grid: Option<(usize, usize)>,
    /// SUN-DIRECTION relief shading (a hillshade): when set, each surface point is lit by the dot of its terrain
    /// normal (the sphere normal tilted by the DERIVED elevation slope) with the star direction, so slopes facing the
    /// star are bright and slopes facing away are dark and a rich derived heightfield reads as lit topography rather
    /// than a two-tone swatch. `default()` is `false` (the discrete relief swatch, so the living-world globe is
    /// unchanged). The hillshade needs the physical body radius ([`SurfaceStyle::surface_radius_m`]) to turn the
    /// elevation gradient into a true slope; without it the lighting falls back to the bare sphere normal.
    pub relief_shading: bool,
    /// The physical body radius (metres) the hillshade uses to convert the elevation gradient into a true physical
    /// slope (no vertical exaggeration): it is READ from the caller's DERIVED scene, never authored. `default()` is
    /// `0` (no physical scale supplied), which skips the terrain tilt and keeps the bare-sphere lighting. Only the
    /// relief-shading derived paths set it; the living-world globe and the tests leave it `0`, byte-identical.
    pub surface_radius_m: Fixed,
}

/// The DERIVED body-frame sun direction for [`draw_globe`], the unit vector pointing from the body's centre toward the
/// SUB-SOLAR POINT, given the sub-solar latitude (`declination`, the seasons, from [`civsim_sim::orbit::solar_declination`])
/// and the sub-solar longitude (`subsolar_longitude`, the time of day, from [`civsim_sim::orbit::subsolar_longitude`]),
/// both in radians. Because [`draw_globe`]'s Lambert term is the dot of the surface normal with this vector, feeding it
/// the sub-solar direction makes the shading read out the physical solar elevation cosine
/// (`sin(lat)sin(decl) + cos(lat)cos(decl)cos(lon - subsolar_lon)`, [`civsim_sim::orbit::solar_elevation_cosine`])
/// automatically, so the lit hemisphere and the day/night terminator are DERIVED, never an authored light direction.
///
/// The vector is expressed in [`draw_globe`]'s BODY frame, whose axes come from [`body_to_uv`]: `y` is the north pole
/// (`lat = asin(b.y)`), and longitude is `atan2(b.x, b.z)`, so the prime meridian (`lon = 0`) is `+z` (facing the viewer
/// at [`GlobeOrientation::IDENTITY`]) and `lon = +pi/2` is `+x`. A surface point at geographic `(lat, lon)` is therefore
/// `(cos lat sin lon, sin lat, cos lat cos lon)` in this frame, and the sub-solar direction is the same with
/// `(decl, subsolar_lon)`: `(cos decl sin sslon, sin decl, cos decl cos sslon)`. At [`GlobeOrientation::IDENTITY`] a
/// positive declination lifts the lit pole toward screen-up and a positive sub-solar longitude lights the screen-right
/// (east) face, the expected faces. Display-only, one-way canon -> pixels (Principle 10).
pub fn sub_solar_body_dir(declination: f32, subsolar_longitude: f32) -> [f32; 3] {
    let (sin_decl, cos_decl) = declination.sin_cos();
    let (sin_sslon, cos_sslon) = subsolar_longitude.sin_cos();
    [cos_decl * sin_sslon, sin_decl, cos_decl * cos_sslon]
}

/// A faint neutral ambient so the night hemisphere reads dark but not pure black (skyglow and starlight). NON-CANON
/// display (Principle 10); hoisted to module scope so [`draw_globe`] and the GPU per-cell cache builders share it.
pub const AMBIENT: f32 = 0.10;
/// NON-CANON DISPLAY: the lava-glow emission brightness gain, the one display scale the self-emitted incandescence
/// is allowed (a sibling of `AMBIENT`, the relief palette, and the sky's `DISPLAY_OPACITY_UNIT`: the observability
/// layer's allowance, Principle 10). The glow add per channel is `emission_channel * intensity * GAIN`, where the
/// emission colour is the blackbody of the tile's DERIVED interior temperature (physics, the hue) and the intensity
/// is its DERIVED melt fraction (physics, zero below the world's own solidus). This scale maps that physical
/// partial-melt fraction, which for a silicate mantle saturates near the rheological lock-up (~0.4, the melt
/// fraction above which the surface behaves as a mobile magma ocean), onto the display's incandescent range: at
/// ~0.4 melt the tile emits at roughly full brightness (a mobile magma ocean glows fully), so `GAIN ~ 1 / 0.4`. It
/// scales BRIGHTNESS only; it never moves the threshold (the derived solidus) or the hue (the derived temperature),
/// and it has zero effect on canon (byte-neutral). Kept modest.
pub const LAVA_EMISSION_GAIN: f32 = 2.5;
/// NON-CANON DISPLAY: the IMPACT-FLASH emission brightness gain, a sibling of `LAVA_EMISSION_GAIN` and `AMBIENT`
/// (the observability layer's allowance, Principle 10). The flash add per channel is `FLASH_COLOR_channel *
/// emission * GAIN`, where `emission` is the fresh crater's DERIVED time-decay times its DERIVED footprint profile
/// (both physics, in `[0, 1]`); this scale only maps that onto the display's incandescent range so a strike reads
/// as a bright bloom that fades. At the flash's peak (a just-formed crater's bowl) this saturates to a white-hot
/// core, a fresh impact blindingly bright, and dims as the crater's own decay relaxes it to the settled relief. It
/// scales BRIGHTNESS only, never the timing (the derived formation tick) or the extent (the derived footprint),
/// and it has zero effect on canon (viewer-only, byte-neutral). Kept the same modest scale as the lava glow.
pub const FLASH_EMISSION_GAIN: f32 = 2.5;
/// NON-CANON DISPLAY: the impact-flash HUE, the incandescent white-hot of a fresh strike's shock-melt fireball.
/// Unlike the lava glow, whose hue is the blackbody of the DERIVED interior temperature, an impact carries no
/// derived shock temperature in the state (the sim never forms the impact's kinetic energy, which overflows the
/// fixed-point range: see `civsim_sim::deeptime::bombard_tick`), so the flash hue is a display choice on the same
/// incandescence family (a hot white, warm at the fading edge), stated plainly. A derived shock temperature would
/// let this derive like the lava hue; that is the honest limit and the future refinement.
pub const FLASH_COLOR: Rgb = Rgb::new(255, 246, 224);

/// Draw the planet as a lit sphere: a filled disk of on-screen radius `radius_px` centred at `(cx, cy)`, its
/// surface textured from the DERIVED tiles (an orthographic sphere map of the relief field, sampled at the surface
/// direction the globe `orient`ation has rotated under each pixel) and shaded by a Lambert diffuse term against the
/// star direction `star_dir`. `param` says how `tiles` (and the aligned `lava` field) map to sphere directions: the
/// DERIVED-planet globe passes [`SurfaceParam::CubeSphere`] (the six-face cache, so the poles carry the same cell
/// density as the equator and neither pinches nor seams), the living-world / fixture globe passes
/// [`SurfaceParam::LatLon`] (the equirectangular cache, byte-identical to before the cube-sphere migration). The
/// sunlit hemisphere is bright and tinted by `light_tint` (the star's
/// [`blackbody_rgb`]); the night side falls to a faint neutral ambient; the cosine falloff between them is the soft
/// day/night terminator. The lighting rotates WITH `orient` (camera-orbit semantics), so panning sweeps the terminator
/// across the surface and the lit part visibly changes as the globe turns, even on a uniform crust. `style.tint`, if
/// given, is the crust's DERIVED perceived colour under the star ([`material_surface_rgb`]): each tile takes that colour
/// scaled by its relief shading, so the sphere wears the derived material colour rather than the relief swatch.
/// `style.grid`, if given, overlays the drill-in SELECTION grid as thin darkened seams, so the surface reads as an
/// array of tiles the cursor can pick (the caller refines it with zoom). That overlay stays lat/lon to match
/// [`pick_surface_tile`], independent of how the sample cache underneath is parameterized; the level-of-detail
/// quadtree slice replaces both together. Pixels outside the disk are left untouched (the caller paints space and the
/// atmosphere limb). A pure, deterministic read of the derived radius, tiles, star direction, style, and orientation,
/// one-way canon -> pixels (Principle 10).
#[allow(clippy::too_many_arguments)]
pub fn draw_globe(
    buf: &mut [u32],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    radius_px: usize,
    tiles: &[DerivedTile],
    param: SurfaceParam,
    star_dir: [f32; 3],
    light_tint: Rgb,
    style: SurfaceStyle,
    orient: GlobeOrientation,
    lava: Option<&[LavaGlow]>,
    field: Option<&SurfaceField>,
    flash: Option<&[ImpactFlash]>,
) {
    if radius_px == 0 || w == 0 || h == 0 {
        return;
    }
    let r = radius_px as f32;
    // The star direction rotates WITH the orientation (camera-orbit semantics): as the pan spins the globe, the
    // day/night terminator sweeps across the surface, so panning visibly changes which part faces the star even on a
    // uniform crust. At GlobeOrientation::IDENTITY this reduces to `star_dir` exactly (rotation by 0), so the headless
    // and living-world globes render unchanged.
    let l = normalize3(body_to_view(star_dir, orient));
    let tint = [
        light_tint.r as f32 / 255.0,
        light_tint.g as f32 / 255.0,
        light_tint.b as f32 / 255.0,
    ];
    // AMBIENT, LAVA_EMISSION_GAIN, FLASH_EMISSION_GAIN, and FLASH_COLOR are hoisted to module scope (above), so the
    // NON-CANON GPU per-cell cache builders ([`globe_cell_base_rgb`] and siblings) share this one source of truth
    // with the CPU render; their reserved-with-basis rationale is documented at the definitions.
    // SUN-DIRECTION HILLSHADE (the seeable-relief payoff): when relief shading is on and the physical body radius is
    // supplied, the shading normal at each pixel is the sphere normal TILTED by the DERIVED terrain slope, so slopes
    // facing the star are bright and slopes facing away are dark. The relief is lit at its real (unexaggerated)
    // amplitude and reads where the light grazes (near the terminator and at surface zoom), the honest look. A zero
    // radius (the living-world globe, or a caller that supplies none) skips the tilt for the bare sphere, byte-identical.
    let surface_radius_m = style.surface_radius_m.to_f64_lossy() as f32;
    // The cache carries cells only if its parameterization resolves at least one (a `LatLon` grid needs both columns
    // and rows; a `CubeSphere` cache needs a positive face resolution). The hillshade needs that plus a physical body
    // radius; otherwise it falls back to the bare sphere normal (the living-world globe, byte-identical).
    let param_has_cells = match param {
        SurfaceParam::LatLon { cols, rows } => cols > 0 && rows > 0,
        SurfaceParam::CubeSphere { face_res } => face_res > 0,
    };
    let hillshade_on = style.relief_shading && surface_radius_m > 0.0 && param_has_cells;
    let rp = radius_px as i32;
    let x0 = (cx - rp).max(0);
    let x1 = (cx + rp).min(w as i32 - 1);
    let y0 = (cy - rp).max(0);
    let y1 = (cy + rp).min(h as i32 - 1);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let nx = (px - cx) as f32 / r;
            let ny = (py - cy) as f32 / r;
            let d2 = nx * nx + ny * ny;
            if d2 > 1.0 {
                continue; // outside the disk
            }
            let nz = (1.0 - d2).sqrt(); // the front-hemisphere normal, toward the viewer
                                        // Orthographic sphere map, rotated by the globe orientation so panning brings
                                        // the far side into view. Screen y points down, so world up is -ny. At
                                        // GlobeOrientation::IDENTITY this reduces to the straight-on map exactly.
            let b = view_to_body([nx, -ny, nz], orient);
            let (u, v) = body_to_uv(b);
            // The surface base colour: when `surface_tint` is given (the derived crust's perceived colour under the
            // star, from `material_surface_rgb`), each tile is that colour scaled by its relief shading, so the sphere
            // wears the DERIVED material colour; otherwise the relief swatch ([`derived_tile_color`]). A uniform crust
            // reads a single shade (the honest look until lateral composition variation lands, a geodynamics
            // follow-on); an empty field falls back to the tint or a deep-ocean stand-in.
            let base = match sample_derived_tile(tiles, param, b, u, v) {
                Some(tile) => match style.tint {
                    Some(m) => {
                        if style.relief_shading {
                            // The DERIVED material colour is the base albedo; the sun-direction hillshade (the
                            // perturbed normal in the Lambert term below) carries the relief, so a rich heightfield
                            // reads as lit topography rather than a flat swatch (no authored brightness ramp).
                            m
                        } else {
                            // The three discrete relief classes (the prior living-world behaviour, byte-identical).
                            let s = relief_shade(tile.relief);
                            let scale = |c: u8| (c as f32 * s).clamp(0.0, 255.0) as u8;
                            Rgb::new(scale(m.r), scale(m.g), scale(m.b))
                        }
                    }
                    None => derived_tile_color(tile.relief),
                },
                None => style.tint.unwrap_or(Rgb::new(40, 72, 120)),
            };
            // Lambert diffuse: dot of the surface normal with the star direction, clamped at the terminator. The
            // normal uses WORLD-UP y (-ny, since screen y points down), the SAME frame the tile sample above uses
            // ([nx, -ny, nz]) and the frame `l` is carried into; without this the brightness was computed in
            // screen-down y while the tiles were placed in world-up y, so the terminator did not line up with the
            // tiles (an inverted-vertical mismatch). When the hillshade is on, the shading normal is the sphere
            // normal TILTED by the DERIVED terrain slope, computed in the body frame so the dot is taken directly
            // against the body-frame `star_dir` (a rotation preserves the dot, so this matches the sphere term when
            // the ground is flat); the relief then lights only where the surface truly slopes.
            let lambert = if hillshade_on {
                let n = hillshade_normal(b, tiles, param, u, v, surface_radius_m, field);
                (n[0] * star_dir[0] + n[1] * star_dir[1] + n[2] * star_dir[2]).max(0.0)
            } else {
                (nx * l[0] - ny * l[1] + nz * l[2]).max(0.0)
            };
            let shade = |c: u8, t: f32| -> u8 {
                let day = AMBIENT + (1.0 - AMBIENT) * lambert * t;
                (c as f32 * day).clamp(0.0, 255.0) as u8
            };
            let mut color = Rgb::new(
                shade(base.r, tint[0]),
                shade(base.g, tint[1]),
                shade(base.b, tint[2]),
            );
            // The display TILE GRID: darken pixels that fall on a lat/lon cell boundary into a thin seam, so the
            // surface reads as an array of tiles. The seam half-width is set in cell-fraction units to render roughly
            // one pixel wide near the disk centre (it thickens toward the limb, where the sphere foreshortens); the
            // grid density is the caller's, refined with zoom so a tile opens into a finer array. Display-only.
            if let Some((gc, gr)) = style.grid {
                if gc > 0 && gr > 0 {
                    let gu = u * gc as f32;
                    let gv = v * gr as f32;
                    let du = (gu - gu.round()).abs();
                    let dv = (gv - gv.round()).abs();
                    let half_u = (gc as f32 / (2.0 * r) * 0.6).min(0.25);
                    let half_v = (gr as f32 / (2.0 * r) * 0.6).min(0.25);
                    if du < half_u || dv < half_v {
                        // Darken toward a seam; visible on the lit side (the night side is already dark).
                        color = Rgb::new(
                            (color.r as f32 * 0.45) as u8,
                            (color.g as f32 * 0.45) as u8,
                            (color.b as f32 * 0.5) as u8,
                        );
                    }
                }
            }
            // SELF-EMITTED LAVA GLOW (the visible-volcanism payoff): an actively-molten tile RADIATES on its own, so
            // its emission ADDS to the sun-lit albedo above and survives on the NIGHT side (lava glows in the dark,
            // the giveaway that it emits rather than reflects, unlike the shaded crust). The emission colour is the
            // blackbody of the tile's DERIVED interior temperature (the incandescence ramp: deep-red cool melt to
            // orange-yellow hot melt) and the intensity is its DERIVED melt fraction (zero below the world's own
            // solidus, so solid crust stays its shaded albedo), sampled at the SAME tile the albedo used so glow and
            // crust register. A young/hot world glows broadly; an aged world is dark crust with super-solidus hot-spots.
            if let Some(glow) = lava {
                if let Some(g) = sample_glow(glow, param, b, u, v) {
                    if g.intensity > 0.0 {
                        let add = |c: u8, e: u8| -> u8 {
                            (c as f32 + e as f32 * g.intensity * LAVA_EMISSION_GAIN)
                                .clamp(0.0, 255.0) as u8
                        };
                        color = Rgb::new(
                            add(color.r, g.emission.r),
                            add(color.g, g.emission.g),
                            add(color.b, g.emission.b),
                        );
                    }
                }
            }
            // SELF-EMITTED IMPACT FLASH (the watchable-impacts payoff): a crater whose formation the deep-time clock
            // has just passed RADIATES a brief incandescent bloom over its settled relief, peaking at its formation
            // tick and relaxing to the static crater as the clock advances (the flash/ejecta of a fresh strike). Like
            // the lava glow it EMITS (survives on the night side) and adds over the shaded crust; unlike it, it is a
            // TRANSIENT keyed on each crater's own formation time, so a viewer watching deep time SEES impacts land.
            // Empty (no fresh impact this epoch) leaves the render byte-identical; the few active flashes are already
            // filtered by the caller ([`active_flash_stamps`]), so this per-pixel sum is cheap.
            if let Some(flashes) = flash {
                let e = crater_flash_emission(flashes, b);
                if e > 0.0 {
                    let add = |c: u8, ch: u8| -> u8 {
                        (c as f32 + ch as f32 * e * FLASH_EMISSION_GAIN).clamp(0.0, 255.0) as u8
                    };
                    color = Rgb::new(
                        add(color.r, FLASH_COLOR.r),
                        add(color.g, FLASH_COLOR.g),
                        add(color.b, FLASH_COLOR.b),
                    );
                }
            }
            buf[py as usize * w + px as usize] = color.pack();
        }
    }
}

/// Draw the star: a solid disk of `color` (its [`blackbody_rgb`]) with a soft radial glow fading to the background
/// over about three radii, so the star's on-screen colour reads as its temperature. Display-only.
fn draw_star(buf: &mut [u32], w: usize, h: usize, sx: i32, sy: i32, radius_px: usize, color: Rgb) {
    if radius_px == 0 || w == 0 || h == 0 {
        return;
    }
    let core = radius_px as i32;
    let glow = core * 3;
    let cr = color.r as f32;
    let cg = color.g as f32;
    let cb = color.b as f32;
    let x0 = (sx - glow).max(0);
    let x1 = (sx + glow).min(w as i32 - 1);
    let y0 = (sy - glow).max(0);
    let y1 = (sy + glow).min(h as i32 - 1);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = (px - sx) as f32;
            let dy = (py - sy) as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = py as usize * w + px as usize;
            if dist <= core as f32 {
                buf[idx] = color.pack();
            } else if dist <= glow as f32 {
                // The glow falls off quadratically from the core edge and blends over whatever is already there.
                let t = 1.0 - (dist - core as f32) / (glow - core).max(1) as f32;
                let a = (t * t).clamp(0.0, 1.0) * 0.8;
                let word = buf[idx];
                let er = (word >> 16) as u8 as f32;
                let eg = (word >> 8) as u8 as f32;
                let eb = word as u8 as f32;
                let mix = |e: f32, c: f32| -> u8 { (e + (c - e) * a).clamp(0.0, 255.0) as u8 };
                buf[idx] = Rgb::new(mix(er, cr), mix(eg, cg), mix(eb, cb)).pack();
            }
        }
    }
}

/// A STAND-IN sky colour for the atmosphere limb: a pale blue placeholder, NOT a derived value.
// TODO(atmosphere): the real limb colour derives from the Stage-8 gas-mix Rayleigh scattering (the manager is
// building that substrate); until it lands this pale-blue fixture stands in, clearly labelled so it is not mistaken
// for physics. When the gas mix is available, replace this constant with a read of the scattered-sky spectrum.
pub const PLACEHOLDER_SKY: Rgb = Rgb::new(150, 190, 235);

/// Draw a soft atmosphere haze around the globe's limb: a thin glow just outside the disk (fading out over
/// `HALO_FRAC` of the radius) plus a faint rim just inside it, brighter on the day side (where the limb faces the
/// star) and dim on the night side. `sky` is the haze colour (a STAND-IN placeholder, see [`PLACEHOLDER_SKY`]; the
/// real colour derives from the Stage-8 gas-mix Rayleigh scattering when that substrate lands). Blends over whatever
/// is already drawn, so it tints the globe's edge and glows against space. Display-only, one-way canon -> pixels.
#[allow(clippy::too_many_arguments)]
fn draw_atmosphere_limb(
    buf: &mut [u32],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    radius_px: usize,
    star_dir: [f32; 3],
    sky: Rgb,
) {
    if radius_px == 0 || w == 0 || h == 0 {
        return;
    }
    // The haze extends this fraction of the radius beyond the limb, and tints this fraction just inside it.
    const HALO_FRAC: f32 = 0.14;
    const RIM_FRAC: f32 = 0.10;
    let r = radius_px as f32;
    let l = normalize3(star_dir);
    let sr = sky.r as f32;
    let sg = sky.g as f32;
    let sb = sky.b as f32;
    let outer = (r * (1.0 + HALO_FRAC)) as i32;
    let x0 = (cx - outer).max(0);
    let x1 = (cx + outer).min(w as i32 - 1);
    let y0 = (cy - outer).max(0);
    let y1 = (cy + outer).min(h as i32 - 1);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let nx = (px - cx) as f32 / r;
            let ny = (py - cy) as f32 / r;
            let d = (nx * nx + ny * ny).sqrt(); // radial distance in radius units
                                                // A band peaking at the limb (d = 1): ramps up over the inner rim, fades out over the outer halo.
            let profile = if d <= 1.0 {
                ((d - (1.0 - RIM_FRAC)) / RIM_FRAC).max(0.0)
            } else {
                (1.0 - (d - 1.0) / HALO_FRAC).max(0.0)
            };
            if profile <= 0.0 {
                continue;
            }
            // The limb point's outward direction, and how much it faces the star (day limb bright, night limb dim).
            let inv = if d > 0.0 { 1.0 / d } else { 0.0 };
            let facing = (nx * inv * l[0] + ny * inv * l[1]).max(0.0);
            let day = 0.15 + 0.85 * facing; // a faint glow survives on the night limb
            let a = (profile * day * 0.6).clamp(0.0, 1.0);
            let idx = py as usize * w + px as usize;
            let word = buf[idx];
            let er = (word >> 16) as u8 as f32;
            let eg = (word >> 8) as u8 as f32;
            let eb = word as u8 as f32;
            let mix = |e: f32, c: f32| -> u8 { (e + (c - e) * a).clamp(0.0, 255.0) as u8 };
            buf[idx] = Rgb::new(mix(er, sr), mix(eg, sg), mix(eb, sb)).pack();
        }
    }
}

/// Compose the zoomed-out solar-system / planet-object view: a `w` by `h` frame of the star and the lit planet
/// globe over space. The star draws as a [`blackbody_rgb`]-coloured disk at `star_px` (its on-screen position, the
/// caller's projection of the orbit, so the orbital phase sets which hemisphere is day). The planet sits at the view
/// centre, its on-screen size the DERIVED radius at this scale ([`globe_radius_px`]), lit from the star direction
/// with the sunlight tinted by the star's colour ([`draw_globe`]): the star-facing hemisphere bright, the far side
/// dark, a soft terminator between. This is the seeable-world payoff entry point: hand it the derived radius, the
/// star's derived `T_eff`, the derived tiles, and the star's projected position, and it draws the star-lit planet.
/// The atmosphere limb is tinted by `sky`, the DERIVED Rayleigh sky colour ([`rayleigh_sky_rgb`]) when the gas
/// mix resolves, or [`PLACEHOLDER_SKY`] as the fail-soft fallback. The globe texture is sampled at the `orient`ation
/// (pass [`GlobeOrientation::IDENTITY`] for the straight-on view), and `style` carries the surface display options (the
/// DERIVED material tint and the optional drill-in tile grid; [`SurfaceStyle::default`] for the plain planet view). A
/// pure, deterministic read of the derived planet and star (Principle 10); it writes no canonical state.
///
/// `derived_star_dir`, when `Some`, is the DERIVED body-frame sun direction ([`sub_solar_body_dir`], from the orbit and
/// the body's attitude): the globe is then lit by the physical sub-solar direction rather than the screen-space vector to
/// the star disk, so the lit hemisphere and terminator track the real sun. When `None`, the light falls back to the
/// on-screen star position (the living-world globe, which has no orbit plumbed), byte-identical to the pre-derivation
/// render.
#[allow(clippy::too_many_arguments)]
pub fn render_solar_system_view(
    radius_m: Fixed,
    t_eff_k: Fixed,
    tiles: &[DerivedTile],
    param: SurfaceParam,
    w: usize,
    h: usize,
    m_per_px: Fixed,
    star_px: (i32, i32),
    star_radius_px: usize,
    bg: Rgb,
    sky: Rgb,
    style: SurfaceStyle,
    orient: GlobeOrientation,
    derived_star_dir: Option<[f32; 3]>,
    lava: Option<&[LavaGlow]>,
    field: Option<&SurfaceField>,
    flash: Option<&[ImpactFlash]>,
) -> Vec<u32> {
    let mut buf = vec![bg.pack(); w.max(1) * h.max(1)];
    if w == 0 || h == 0 {
        return buf;
    }
    let star_color = blackbody_rgb(t_eff_k);
    let planet_cx = (w / 2) as i32;
    let planet_cy = (h / 2) as i32;
    let planet_radius_px = globe_radius_px(radius_m, m_per_px);
    // The star direction is the on-screen vector from the planet to the star, lifted out of the screen plane so the
    // lit hemisphere tilts toward the viewer (a readable terminator rather than an edge-on sliver). The in-plane
    // part carries the orbit's projected direction; the fixed z-lift is a display framing, not physics.
    let dx = (star_px.0 - planet_cx) as f32;
    let dy = (star_px.1 - planet_cy) as f32;
    let plane = (dx * dx + dy * dy).sqrt();
    // The y is WORLD-UP (-dy, since screen y points down), the same frame draw_globe's tile sample and Lambert
    // normal use, so the terminator tracks the tiles as you pan (at IDENTITY the two sign flips cancel, so the
    // straight-on globe is byte-identical to before).
    let fallback_star_dir = if plane <= 0.0 {
        [0.0, 0.0, 1.0]
    } else {
        [0.72 * dx / plane, -0.72 * dy / plane, 0.70]
    };
    // The DERIVED body-frame sun direction lights the globe when supplied; otherwise the screen-space fallback keeps
    // the living-world globe byte-identical.
    let star_dir = derived_star_dir.unwrap_or(fallback_star_dir);
    draw_star(
        &mut buf,
        w,
        h,
        star_px.0,
        star_px.1,
        star_radius_px,
        star_color,
    );
    draw_globe(
        &mut buf,
        w,
        h,
        planet_cx,
        planet_cy,
        planet_radius_px,
        tiles,
        param,
        star_dir,
        star_color,
        style,
        orient,
        lava,
        field,
        flash,
    );
    // The atmosphere haze around the limb, tinted by the caller's `sky` colour: the DERIVED Rayleigh sky from the
    // gas mix ([`rayleigh_sky_rgb`]) when it resolves, or [`PLACEHOLDER_SKY`] as the fail-soft fallback. The limb wants
    // the VIEW-space sun direction (it brightens the day-facing edge in screen x, y): for the derived body-frame vector
    // that is its projection through the orientation, and for the screen-space fallback it is the vector itself (at
    // IDENTITY the two coincide, so the fallback path stays byte-identical).
    let limb_dir = if derived_star_dir.is_some() {
        normalize3(body_to_view(star_dir, orient))
    } else {
        star_dir
    };
    draw_atmosphere_limb(
        &mut buf,
        w,
        h,
        planet_cx,
        planet_cy,
        planet_radius_px,
        limb_dir,
        sky,
    );
    buf
}

/// Compose one planet-globe scene at an ARBITRARY on-screen centre `(cx, cy)` and `radius_px` over the caller's
/// already-filled `buf` (unlike [`render_solar_system_view`], which allocates and centres the globe itself). This is the
/// primitive the interactive derived viewer and its woosh transition share: the woosh grows the globe from a map dot to
/// the frame centre by animating `(cx, cy, radius_px)` each frame, and the settled globe view is the same call at the
/// centre. It draws the optional `star` disk (`(sx, sy, radius)`, or `None` to omit it), the derived-lit globe
/// (`star_dir_body` is the DERIVED body-frame sun direction, [`sub_solar_body_dir`]), and the atmosphere limb (lit from
/// the view-space projection of the sun so the day edge glows). Display-only, one-way canon -> pixels (Principle 10).
#[allow(clippy::too_many_arguments)]
pub fn draw_globe_scene(
    buf: &mut [u32],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    radius_px: usize,
    tiles: &[DerivedTile],
    param: SurfaceParam,
    t_eff_k: Fixed,
    star_dir_body: [f32; 3],
    star: Option<(i32, i32, usize)>,
    sky: Rgb,
    style: SurfaceStyle,
    orient: GlobeOrientation,
    lava: Option<&[LavaGlow]>,
    field: Option<&SurfaceField>,
    flash: Option<&[ImpactFlash]>,
) {
    if w == 0 || h == 0 {
        return;
    }
    let light_tint = blackbody_rgb(t_eff_k);
    if let Some((sx, sy, sr)) = star {
        draw_star(buf, w, h, sx, sy, sr, light_tint);
    }
    draw_globe(
        buf,
        w,
        h,
        cx,
        cy,
        radius_px,
        tiles,
        param,
        star_dir_body,
        light_tint,
        style,
        orient,
        lava,
        field,
        flash,
    );
    // The limb wants the VIEW-space sun direction (screen x, y), the projection of the derived body-frame vector.
    let limb_dir = normalize3(body_to_view(star_dir_body, orient));
    draw_atmosphere_limb(buf, w, h, cx, cy, radius_px, limb_dir, sky);
}

// ============================================================================================================
// NON-CANON GPU shading cache builders (Principle 10). These pre-reduce, ONCE per epoch, exactly the DERIVED
// per-cell shading inputs the GPU globe kernel (`civsim_gpu::globe`) samples: the base albedo, the terrain hillshade
// normal, and the self-emitted lava and impact-flash adds, each evaluated at the cell CENTRE with the SAME verified
// helpers `draw_globe` uses per pixel. Moving these off the per-pixel path is what lets the GPU shade run without the
// O(pixels x craters) analytic gradient the CPU renderer pays; the kernel then samples the cell a pixel lands in.
// The per-cell (rather than per-pixel) evaluation is the one approximation of the GPU path: within a cell the normal
// and flash are held at the centre value, so the GPU frame is VISUALLY equal to `draw_globe`, not byte-equal (a
// non-canon display allowance, Principle 10). All display f32 / packed RGB; they write no canonical state.
// ============================================================================================================

/// The BODY-frame direction at the CENTRE of surface cell `i` under the cache parameterization `param`, the
/// representative direction the per-cell shading cache is evaluated at. A pixel that samples cell `i` (via
/// [`surface_cell_index`]) reads the value built here; the centre round-trips back to `i` through that index by
/// construction. Display-only f32 (Principle 10).
#[cfg(feature = "gpu")]
pub fn surface_cell_center_dir(param: SurfaceParam, i: usize) -> [f32; 3] {
    use std::f32::consts::FRAC_PI_2;
    match param {
        SurfaceParam::LatLon { cols, rows } => {
            if cols == 0 || rows == 0 {
                return [0.0, 0.0, 1.0];
            }
            let cu = i % cols;
            let cv = (i / cols).min(rows - 1);
            let u = (cu as f32 + 0.5) / cols as f32;
            let v = (cv as f32 + 0.5) / rows as f32;
            uv_to_body(u, v)
        }
        SurfaceParam::CubeSphere { face_res } => {
            if face_res == 0 {
                return [0.0, 0.0, 1.0];
            }
            let fc = face_res * face_res;
            let face = (i / fc).min(5);
            let rem = i % fc;
            let cj = rem / face_res; // t row
            let ci = rem % face_res; // s col
            let s = (ci as f32 + 0.5) / face_res as f32;
            let t = (cj as f32 + 0.5) / face_res as f32;
            // The equi-angular face-local direction (the same warp `cube_dir_to_face_st` inverts), then the face
            // basis into the body frame (the exact inverse of the cube cell index the kernel computes).
            let (sa, ca) = ((s - 0.5) * FRAC_PI_2).sin_cos();
            let (sb, cb) = ((t - 0.5) * FRAC_PI_2).sin_cos();
            let lx = sa * cb;
            let ly = ca * sb;
            let lz = ca * cb;
            let n = (lx * lx + ly * ly + lz * lz).sqrt();
            let d = if n > 0.0 {
                [lx / n, ly / n, lz / n]
            } else {
                [0.0, 0.0, 1.0]
            };
            let (dx, dy, dz) = (d[0], d[1], d[2]);
            match face {
                0 => [dz, dy, -dx],  // +x dominant
                1 => [-dz, dy, dx],  // -x
                2 => [dx, dz, -dy],  // +y
                3 => [dx, -dz, dy],  // -y
                4 => [dx, dy, dz],   // +z
                _ => [-dx, dy, -dz], // -z
            }
        }
    }
}

/// The per-cell BASE ALBEDO (packed `0x00RRGGBB`), the colour `draw_globe` paints a cell before lighting: the
/// DERIVED material tint (`style.tint`) when relief shading is on, that tint scaled by the discrete relief shade
/// when it is off, or the relief swatch when there is no tint. One entry per tile, in cache order. Display-only.
#[cfg(feature = "gpu")]
pub fn globe_cell_base_rgb(tiles: &[DerivedTile], style: SurfaceStyle) -> Vec<u32> {
    tiles
        .iter()
        .map(|tile| {
            let rgb = match style.tint {
                Some(m) => {
                    if style.relief_shading {
                        m
                    } else {
                        let s = relief_shade(tile.relief);
                        let sc = |c: u8| (c as f32 * s).clamp(0.0, 255.0) as u8;
                        Rgb::new(sc(m.r), sc(m.g), sc(m.b))
                    }
                }
                None => derived_tile_color(tile.relief),
            };
            rgb.pack()
        })
        .collect()
}

/// The per-cell TERRAIN NORMAL (interleaved `nx, ny, nz` per cell, `3 * tiles.len()` long), the sphere normal
/// tilted by the DERIVED slope, evaluated at each cell centre with the SAME [`hillshade_normal`] `draw_globe` uses
/// per pixel (the analytic Sample gradient when `field` is supplied, the cache finite difference otherwise). Built
/// in PARALLEL (each cell independent), the once-per-epoch reduction that moves the analytic gradient off the
/// per-frame per-pixel path. Display-only f32 (Principle 10).
#[cfg(feature = "gpu")]
pub fn globe_cell_normals(
    tiles: &[DerivedTile],
    param: SurfaceParam,
    surface_radius_m: Fixed,
    field: Option<&SurfaceField>,
) -> Vec<f32> {
    let radius_m = surface_radius_m.to_f64_lossy() as f32;
    let per_cell: Vec<[f32; 3]> = (0..tiles.len())
        .into_par_iter()
        .map(|i| {
            let b = surface_cell_center_dir(param, i);
            let (u, v) = body_to_uv(b);
            hillshade_normal(b, tiles, param, u, v, radius_m, field)
        })
        .collect();
    per_cell.into_iter().flatten().collect()
}

/// The per-cell SELF-EMITTED LAVA add (interleaved `r, g, b` per cell, `3 * ncells` long), already
/// `emission * intensity * LAVA_EMISSION_GAIN`, the incandescent glow `draw_globe` adds over the shaded crust. Zero
/// where a cell is below the world's solidus. Sampled at each cell centre through the same [`sample_glow`]
/// `draw_globe` uses, so it registers with the crust cell it rides on. Display-only.
#[cfg(feature = "gpu")]
pub fn globe_cell_lava_add(lava: &[LavaGlow], param: SurfaceParam, ncells: usize) -> Vec<f32> {
    (0..ncells)
        .flat_map(|i| {
            let b = surface_cell_center_dir(param, i);
            let (u, v) = body_to_uv(b);
            match sample_glow(lava, param, b, u, v) {
                Some(g) if g.intensity > 0.0 => {
                    let add = |e: u8| e as f32 * g.intensity * LAVA_EMISSION_GAIN;
                    [add(g.emission.r), add(g.emission.g), add(g.emission.b)]
                }
                _ => [0.0, 0.0, 0.0],
            }
        })
        .collect()
}

/// The active IMPACT FLASHES as the plain [`civsim_gpu::globe::GlobeFlash`] the kernel sums PER PIXEL (unlike the
/// per-cell base / normal / lava, the flash has a steep `1/x^3` falloff and is the effect the owner watches land,
/// so it is kept pixel-accurate). A direct carry of each [`ImpactFlash`]'s already-f32 geometry and decayed
/// intensity; the kernel reproduces [`crater_flash_emission`] over them. Display-only (Principle 10).
#[cfg(feature = "gpu")]
pub fn globe_gpu_flashes(flash: &[ImpactFlash]) -> Vec<civsim_gpu::globe::GlobeFlash> {
    flash
        .iter()
        .map(|f| civsim_gpu::globe::GlobeFlash {
            center: f.center,
            angular_radius: f.angular_radius,
            cos_reach: f.cos_reach,
            intensity: f.intensity,
        })
        .collect()
}

/// The per-frame [`civsim_gpu::globe::GlobeFrame`] scalars for the GPU globe shade: the camera, the body-frame and
/// view-frame sun directions, the sunlight tint, the ambient floor, the tile-grid seam, and the hillshade-on
/// decision, all derived exactly as [`draw_globe`] derives them per pixel. Shared by [`draw_globe_scene_gpu`] and
/// the parity test so the two cannot drift. Cheap (a handful of scalars), rebuilt every frame. Display-only.
#[cfg(feature = "gpu")]
#[allow(clippy::too_many_arguments)]
fn globe_gpu_frame(
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    radius_px: usize,
    param: SurfaceParam,
    star_dir_body: [f32; 3],
    light_tint: Rgb,
    style: SurfaceStyle,
    orient: GlobeOrientation,
) -> civsim_gpu::globe::GlobeFrame {
    let surface_radius_m = style.surface_radius_m.to_f64_lossy() as f32;
    let param_has_cells = match param {
        SurfaceParam::LatLon { cols, rows } => cols > 0 && rows > 0,
        SurfaceParam::CubeSphere { face_res } => face_res > 0,
    };
    let hillshade_on = style.relief_shading && surface_radius_m > 0.0 && param_has_cells;
    civsim_gpu::globe::GlobeFrame {
        w,
        h,
        cx,
        cy,
        radius_px,
        rot_lon: orient.rot_lon,
        rot_lat: orient.rot_lat,
        star_dir_body,
        light_view: normalize3(body_to_view(star_dir_body, orient)),
        tint: [
            light_tint.r as f32 / 255.0,
            light_tint.g as f32 / 255.0,
            light_tint.b as f32 / 255.0,
        ],
        ambient: AMBIENT,
        flash_color: [
            FLASH_COLOR.r as f32,
            FLASH_COLOR.g as f32,
            FLASH_COLOR.b as f32,
        ],
        flash_gain: FLASH_EMISSION_GAIN,
        grid: style
            .grid
            .filter(|&(c, r)| c > 0 && r > 0)
            .unwrap_or((0, 0)),
        hillshade_on,
    }
}

/// The GPU counterpart of [`draw_globe_scene`]: it composites the star (CPU), the globe DISK (the GPU shade in
/// `civsim_gpu::globe`, the per-pixel port of [`draw_globe`]), and the atmosphere limb (CPU) into `buf`, in the same
/// order and from the same inputs as [`draw_globe_scene`]. The heavy DERIVED per-cell shading cache (base albedo,
/// hillshade normal, lava and flash adds) is (re)built and uploaded only when `scene_epoch` differs from the
/// renderer's resident tag: a deep-time step or a scene switch rebuilds it, a rotate / zoom / sweep reuses the
/// resident cache and pays only the small per-frame scalar upload plus the GPU shade. Fails soft to the CPU
/// [`draw_globe`] for a degenerate cache. NON-CANON (Principle 10): writes pixels only, adds nothing to canon.
#[cfg(feature = "gpu")]
#[allow(clippy::too_many_arguments)]
pub fn draw_globe_scene_gpu(
    gpu: &mut civsim_gpu::globe::CudaGlobeRenderer,
    scene_epoch: u64,
    buf: &mut [u32],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    radius_px: usize,
    tiles: &[DerivedTile],
    param: SurfaceParam,
    t_eff_k: Fixed,
    star_dir_body: [f32; 3],
    star: Option<(i32, i32, usize)>,
    sky: Rgb,
    style: SurfaceStyle,
    orient: GlobeOrientation,
    lava: Option<&[LavaGlow]>,
    field: Option<&SurfaceField>,
    flash: Option<&[ImpactFlash]>,
) {
    use civsim_gpu::globe::{GlobeCells, GlobeParam};
    if w == 0 || h == 0 {
        return;
    }
    let light_tint = blackbody_rgb(t_eff_k);
    if let Some((sx, sy, sr)) = star {
        draw_star(buf, w, h, sx, sy, sr, light_tint);
    }

    let gparam = match param {
        SurfaceParam::LatLon { cols, rows } => GlobeParam::LatLon { cols, rows },
        SurfaceParam::CubeSphere { face_res } => GlobeParam::CubeSphere { face_res },
    };
    let cells_ok = gparam.cells() == tiles.len() && !tiles.is_empty() && radius_px > 0;

    // (Re)build the per-cell cache on an epoch change; a matching tag reuses the resident device cache. The flash
    // is NOT part of the resident cache: it is a small per-frame array summed per pixel in the kernel.
    if cells_ok && (gpu.tag() != Some(scene_epoch) || !gpu.has_cells()) {
        let base_rgb = globe_cell_base_rgb(tiles, style);
        let normals = globe_cell_normals(tiles, param, style.surface_radius_m, field);
        let lava_add = match lava {
            Some(l) => globe_cell_lava_add(l, param, tiles.len()),
            None => Vec::new(),
        };
        gpu.upload_cells(
            gparam,
            GlobeCells {
                base_rgb: &base_rgb,
                normal: &normals,
                lava_add: &lava_add,
            },
            scene_epoch,
        );
    }

    let limb_dir = normalize3(body_to_view(star_dir_body, orient));
    if cells_ok && gpu.has_cells() {
        let frame = globe_gpu_frame(
            w,
            h,
            cx,
            cy,
            radius_px,
            param,
            star_dir_body,
            light_tint,
            style,
            orient,
        );
        let flashes = flash.map(globe_gpu_flashes).unwrap_or_default();
        gpu.render(buf, &frame, &flashes);
    } else {
        // Degenerate cache: keep drawing rather than blanking, via the CPU renderer.
        draw_globe(
            buf,
            w,
            h,
            cx,
            cy,
            radius_px,
            tiles,
            param,
            star_dir_body,
            light_tint,
            style,
            orient,
            lava,
            field,
            flash,
        );
    }

    draw_atmosphere_limb(buf, w, h, cx, cy, radius_px, limb_dir, sky);
}

/// One body on the zoomed-out SYSTEM MAP: its orbit's semi-major axis (AU) and eccentricity (which trace the ellipse
/// through [`civsim_sim::orbit::orbital_state`]), its current mean anomaly (the phase that places the dot on the ellipse),
/// and the display colour and pixel size of its dot. The orbit geometry is DERIVED from `orbit.rs`; the dot colour is the
/// planet's derived material colour and the size is a non-canon display choice the caller sets.
#[derive(Clone, Copy)]
pub struct MapBody {
    /// The orbit semi-major axis in AU (scales the perifocal `orbital_state` position to a real distance).
    pub semi_major_au: Fixed,
    /// The orbit eccentricity (the ellipse's shape; `0` is a circle).
    pub eccentricity: Fixed,
    /// The current mean anomaly (radians) placing the planet dot on its ellipse.
    pub mean_anomaly: Fixed,
    /// The dot's display colour (the planet's DERIVED material colour).
    pub dot_color: Rgb,
    /// The dot's on-screen radius in pixels (a non-canon display size).
    pub dot_px: usize,
}

/// The system-map projection scale (AU per pixel) that fits every body's aphelion inside `fit_frac` of the frame's
/// smaller dimension, with the star at the centre. A pure display projection (Principle 10).
fn system_map_au_per_px(w: usize, h: usize, bodies: &[MapBody], fit_frac: f64) -> f64 {
    let min_dim = w.min(h) as f64;
    let max_extent_au = bodies
        .iter()
        .map(|b| {
            let a = b.semi_major_au.to_f64_lossy();
            let e = b.eccentricity.to_f64_lossy();
            a * (1.0 + e.max(0.0))
        })
        .fold(0.0_f64, f64::max)
        .max(1e-6);
    let fit_px = (min_dim * fit_frac).max(1.0);
    max_extent_au / fit_px
}

/// Project a perifocal position (in units of the semi-major axis) at semi-major axis `a_au` to a screen pixel, with the
/// star (the orbit focus) at the frame centre `(cx, cy)` and screen y pointing down. A pure display projection.
fn system_map_project(
    px_over_a: f64,
    py_over_a: f64,
    a_au: f64,
    cx: i32,
    cy: i32,
    au_per_px: f64,
) -> (i32, i32) {
    let au_x = px_over_a * a_au;
    let au_y = py_over_a * a_au;
    let sx = cx + (au_x / au_per_px).round() as i32;
    let sy = cy - (au_y / au_per_px).round() as i32; // screen y points down, so orbit +y is up
    (sx, sy)
}

/// Fill a small filled disk of `color` (an opaque dot) centred at `(cx, cy)`. Display-only.
fn fill_disk(buf: &mut [u32], w: usize, h: usize, cx: i32, cy: i32, radius_px: usize, color: u32) {
    let r = radius_px as i32;
    let x0 = (cx - r).max(0);
    let x1 = (cx + r).min(w as i32 - 1);
    let y0 = (cy - r).max(0);
    let y1 = (cy + r).min(h as i32 - 1);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = px - cx;
            let dy = py - cy;
            if dx * dx + dy * dy <= r * r {
                buf[py as usize * w + px as usize] = color;
            }
        }
    }
}

/// Render the zoomed-out SYSTEM MAP: the star (its [`blackbody_rgb`] colour) at the frame centre, each body's ORBIT
/// ELLIPSE traced from `orbit.rs` (`orbital_state` swept over a full turn of mean anomaly, scaled by the semi-major axis
/// and projected with the star at one focus), and each body's dot at its current phase. Returns the frame and the
/// on-screen dot centres (so the caller can hit-test clicks and place labels). The map is a display projection of DERIVED
/// orbit geometry; it does NOT model a gravitationally-assembled multi-body system (that is the solar-system generator,
/// task #72). The set of orbits is a viewer input, not a canonical layout. Display-only (Principle 10).
pub fn render_system_map(
    w: usize,
    h: usize,
    bg: Rgb,
    star_t_eff: Fixed,
    star_radius_px: usize,
    bodies: &[MapBody],
) -> (Vec<u32>, Vec<(i32, i32)>) {
    let mut buf = vec![bg.pack(); w.max(1) * h.max(1)];
    let mut dots = Vec::with_capacity(bodies.len());
    if w == 0 || h == 0 {
        return (buf, dots);
    }
    let cx = (w / 2) as i32;
    let cy = (h / 2) as i32;
    // Fit every aphelion inside ~0.42 of the smaller dimension (a non-canon display framing).
    let au_per_px = system_map_au_per_px(w, h, bodies, 0.42);
    // A dim graticule colour for the orbit lines, and the number of samples per ellipse (a display resolution).
    let orbit_color = Rgb::new(70, 78, 96).pack();
    const ORBIT_SAMPLES: usize = 160;
    for b in bodies {
        let a_au = b.semi_major_au.to_f64_lossy();
        // Trace the ellipse: sweep the mean anomaly over a full turn, projecting each solved position; connect
        // consecutive points, closing the loop. The star sits at the focus (the perifocal origin), so the ellipse is
        // offset the way a real orbit is, not centred on the star.
        let mut prev: Option<(i32, i32)> = None;
        let mut first: Option<(i32, i32)> = None;
        for k in 0..=ORBIT_SAMPLES {
            let frac = k as f64 / ORBIT_SAMPLES as f64;
            let m = Fixed::from_ratio((frac * 1_000_000.0) as i64, 1_000_000)
                .checked_mul(Fixed::PI.checked_add(Fixed::PI).unwrap_or(Fixed::PI));
            let point = m.and_then(|m| {
                civsim_sim::orbit::orbital_state(m, b.eccentricity).map(|s| {
                    system_map_project(
                        s.position_x_over_a.to_f64_lossy(),
                        s.position_y_over_a.to_f64_lossy(),
                        a_au,
                        cx,
                        cy,
                        au_per_px,
                    )
                })
            });
            if let Some((sx, sy)) = point {
                if let Some((psx, psy)) = prev {
                    draw_line(&mut buf, w, h, psx, psy, sx, sy, orbit_color);
                }
                prev = Some((sx, sy));
                first.get_or_insert((sx, sy));
            }
        }
        // The planet dot at its current phase; fail-soft to the frame centre if the state does not resolve.
        let dot = civsim_sim::orbit::orbital_state(b.mean_anomaly, b.eccentricity)
            .map(|s| {
                system_map_project(
                    s.position_x_over_a.to_f64_lossy(),
                    s.position_y_over_a.to_f64_lossy(),
                    a_au,
                    cx,
                    cy,
                    au_per_px,
                )
            })
            .unwrap_or((cx, cy));
        fill_disk(&mut buf, w, h, dot.0, dot.1, b.dot_px, b.dot_color.pack());
        dots.push(dot);
    }
    // The star last, so its glow sits over the orbit lines near the centre.
    draw_star(
        &mut buf,
        w,
        h,
        cx,
        cy,
        star_radius_px,
        blackbody_rgb(star_t_eff),
    );
    (buf, dots)
}

/// The DISPLAY tile (column, row) under a screen pixel, inverting the orthographic sphere map and the globe
/// orientation: the pixel offset from the globe centre `(cx, cy)` normalized by the on-screen `radius_px` gives the
/// front-hemisphere view point (`z = sqrt(1 - x^2 - y^2)`), the inverse rotation ([`view_to_body`]) carries it to the
/// body frame, and its (u, v) selects the cell of the `cols` by `rows` equirectangular display grid, by the same cell
/// arithmetic [`sample_derived_tile`] applies to a [`SurfaceParam::LatLon`] cache. That grid is the OBSERVER's tile
/// addressing (the seams [`SurfaceStyle::grid`] overlays, the cell [`draw_surface_highlight`] outlines, the
/// `tile (col,row)` the provenance readout names), NOT the sample cache index: a [`SurfaceParam::CubeSphere`] cache is
/// parameterized by `face_res` over six faces and this pick does not address it. `None` if the pixel is off the
/// sphere's disk (fail-soft: the caller draws no highlight). A pure inverse of the sphere map, display-only
/// (Principle 10).
#[allow(clippy::too_many_arguments)]
pub fn pick_surface_tile(
    px: i32,
    py: i32,
    cx: i32,
    cy: i32,
    radius_px: usize,
    orient: GlobeOrientation,
    cols: usize,
    rows: usize,
) -> Option<(usize, usize)> {
    if radius_px == 0 || cols == 0 || rows == 0 {
        return None;
    }
    let r = radius_px as f32;
    let nx = (px - cx) as f32 / r;
    let ny = (py - cy) as f32 / r;
    let d2 = nx * nx + ny * ny;
    if d2 > 1.0 {
        return None; // off the disk
    }
    let nz = (1.0 - d2).sqrt();
    let (u, v) = body_to_uv(view_to_body([nx, -ny, nz], orient));
    let cu = ((u.clamp(0.0, 0.999_9) * cols as f32) as usize).min(cols - 1);
    let cv = ((v.clamp(0.0, 0.999_9) * rows as f32) as usize).min(rows - 1);
    Some((cu, cv))
}

/// Draw a highlight outline around the derived-surface tile `(cu, cv)` of a `cols` by `rows` field: project the four
/// corners of the tile's (u, v) cell forward onto the sphere ([`uv_to_body`] then [`body_to_view`]) at the given
/// orientation and connect them, so the marked tile curves with the globe and stays put as it rotates. A corner on
/// the far hemisphere (view z < 0) drops its segments (fail-soft), so a tile at the limb shows a partial outline
/// rather than a line flung across the frame. Display-only, one-way canon -> pixels (Principle 10).
#[allow(clippy::too_many_arguments)]
pub fn draw_surface_highlight(
    buf: &mut [u32],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    radius_px: usize,
    orient: GlobeOrientation,
    cols: usize,
    rows: usize,
    cu: usize,
    cv: usize,
    color: Rgb,
) {
    if radius_px == 0 || cols == 0 || rows == 0 || w == 0 || h == 0 {
        return;
    }
    let r = radius_px as f32;
    let u0 = cu as f32 / cols as f32;
    let u1 = (cu + 1) as f32 / cols as f32;
    let v0 = cv as f32 / rows as f32;
    let v1 = (cv + 1) as f32 / rows as f32;
    let corners = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)];
    // Forward-project each corner; a back-facing corner (view z < 0) yields None so its segments are skipped.
    let project = |u: f32, v: f32| -> Option<(i32, i32)> {
        let pv = body_to_view(uv_to_body(u, v), orient);
        if pv[2] < 0.0 {
            return None; // far hemisphere
        }
        let sx = cx + (pv[0] * r).round() as i32;
        let sy = cy - (pv[1] * r).round() as i32; // screen y points down, world up is +view.y
        Some((sx, sy))
    };
    let pts: [Option<(i32, i32)>; 4] = [
        project(corners[0].0, corners[0].1),
        project(corners[1].0, corners[1].1),
        project(corners[2].0, corners[2].1),
        project(corners[3].0, corners[3].1),
    ];
    let c = color.pack();
    for i in 0..4 {
        if let (Some(a), Some(b)) = (pts[i], pts[(i + 1) % 4]) {
            draw_line(buf, w, h, a.0, a.1, b.0, b.1, c);
        }
    }
}

/// Draw a 1-pixel line (Bresenham) clipped to the buffer, for the surface highlight outline. Presentation only.
#[allow(clippy::too_many_arguments)]
fn draw_line(buf: &mut [u32], w: usize, h: usize, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);
    loop {
        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
            buf[y as usize * w + x as usize] = color;
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_sim::genesis::{genesis, GenesisParams};

    // A crater row centred at (u, v) = (0.5, 0.5) (the sub-solar-frame equator, prime facing), with a given rim
    // diameter and bowl depth (metres), for the analytic-stamp shape tests.
    fn crater_row(diameter_m: i32, depth_m: i32) -> CraterRow {
        CraterRow {
            u: Fixed::from_ratio(1, 2),
            v: Fixed::from_ratio(1, 2),
            diameter_m: Fixed::from_int(diameter_m),
            depth_m: Fixed::from_int(depth_m),
            age_myr: Fixed::ZERO,
        }
    }

    // A crater row at (u, v) = (0.5, 0.5) formed at a given clock reading `age_myr` (megayears), for the
    // impact-FLASH timing tests: the flash keys this formation reading against the render's current epoch.
    fn crater_row_formed_at(diameter_m: i32, depth_m: i32, age_myr: i32) -> CraterRow {
        CraterRow {
            u: Fixed::from_ratio(1, 2),
            v: Fixed::from_ratio(1, 2),
            diameter_m: Fixed::from_int(diameter_m),
            depth_m: Fixed::from_int(depth_m),
            age_myr: Fixed::from_int(age_myr),
        }
    }

    // The crater-centre direction (u, v) = (0.5, 0.5) as the display f32 unit vector the flash emission samples.
    fn flash_center_dir() -> [f32; 3] {
        let c = crater_uv_unit(Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2));
        [
            c[0].to_f64_lossy() as f32,
            c[1].to_f64_lossy() as f32,
            c[2].to_f64_lossy() as f32,
        ]
    }

    #[test]
    fn an_impact_flashes_at_its_formation_and_settles_after_the_window() {
        // A crater formed at 1000 Myr on a 3000 km body, a 60-Myr (3-tick x 20-Myr) relaxation window. The flash is
        // present and PEAK at the formation epoch, dimmer halfway through the window, and GONE (no flash object) once
        // a full window has passed, so a viewer watching deep time sees it land and relax to the static crater.
        let radius_m = Fixed::from_int(3_000_000);
        let window = Fixed::from_int(60);
        let formed = 1000;
        let rows = vec![crater_row_formed_at(60_000, 6_000, formed)];

        // Before it forms: no flash (the crater does not exist yet on the clock).
        let before = active_flash_stamps(&rows, radius_m, Fixed::from_int(formed - 20), window);
        assert!(
            before.is_empty(),
            "a crater flashes only from its formation onward"
        );

        // At formation: exactly one flash, at peak intensity (1.0).
        let at = active_flash_stamps(&rows, radius_m, Fixed::from_int(formed), window);
        assert_eq!(
            at.len(),
            1,
            "the fresh crater flashes at its formation epoch"
        );
        let peak = at[0].intensity;
        assert!(
            (peak - 1.0).abs() < 1e-3,
            "peak flash at formation, got {peak}"
        );

        // Halfway through the window (phase 0.5): the declared (1-phase)^2 decay = 0.25 of peak.
        let mid = active_flash_stamps(&rows, radius_m, Fixed::from_int(formed + 30), window);
        assert_eq!(mid.len(), 1);
        let midi = mid[0].intensity;
        assert!(
            (midi - 0.25).abs() < 1e-2,
            "quadratic ease-out at half window ~0.25, got {midi}"
        );
        assert!(midi < peak, "the flash decays from its formation peak");

        // A full window later: the crater has settled, no flash object at all (it is now just static relief).
        let after = active_flash_stamps(&rows, radius_m, Fixed::from_int(formed + 60), window);
        assert!(
            after.is_empty(),
            "the flash is gone once the window passes (settled to static relief)"
        );
    }

    #[test]
    fn the_flash_emission_is_bright_at_the_crater_and_fades_to_nothing() {
        // The emission the pixel loop adds: bright over the crater at formation, gone once it settles, and zero for a
        // sample far from the crater. Sampled at the crater-centre direction (u, v) = (0.5, 0.5).
        let radius_m = Fixed::from_int(3_000_000);
        let window = Fixed::from_int(60);
        let rows = vec![crater_row_formed_at(60_000, 6_000, 1000)];
        let c = flash_center_dir();

        let at = active_flash_stamps(&rows, radius_m, Fixed::from_int(1000), window);
        let e_at = crater_flash_emission(&at, c);
        assert!(
            e_at > 0.9,
            "the crater centre is bright at formation, got {e_at}"
        );

        // The far side of the globe is outside the reach cone: no emission.
        let far = crater_flash_emission(&at, [-c[0], -c[1], -c[2]]);
        assert_eq!(far, 0.0, "a sample far from the crater gets no flash");

        // After the window: no flash object, so no emission (the crater is now only static relief).
        let after = active_flash_stamps(&rows, radius_m, Fixed::from_int(1060), window);
        assert_eq!(
            crater_flash_emission(&after, c),
            0.0,
            "no emission once the flash has settled"
        );
    }

    #[test]
    fn the_flash_is_deterministic_in_the_rows_and_the_clock() {
        // Same rows + same epoch + same window => the same flashes and the same emission, to the bit (Principle 3).
        let radius_m = Fixed::from_int(3_000_000);
        let window = Fixed::from_int(60);
        let rows = vec![
            crater_row_formed_at(60_000, 6_000, 1000),
            crater_row_formed_at(30_000, 3_000, 1010),
        ];
        let a = active_flash_stamps(&rows, radius_m, Fixed::from_int(1015), window);
        let b = active_flash_stamps(&rows, radius_m, Fixed::from_int(1015), window);
        assert_eq!(
            a.len(),
            b.len(),
            "the same clock step draws the same flashes"
        );
        assert_eq!(
            a.len(),
            2,
            "both craters are within the window at epoch 1015"
        );
        for (fa, fb) in a.iter().zip(b.iter()) {
            assert_eq!(fa.intensity.to_bits(), fb.intensity.to_bits());
        }
        let c = flash_center_dir();
        assert_eq!(
            crater_flash_emission(&a, c).to_bits(),
            crater_flash_emission(&b, c).to_bits(),
            "same seed + same clock step => same rendered flash, to the bit"
        );
    }

    #[test]
    fn draw_globe_paints_a_fresh_impact_flash_over_the_dark_crust() {
        // The pixel-level wiring: a fresh impact centred on the SUB-OBSERVER point radiates over the crust the globe
        // draws, so the pixel there is brighter WITH the flash than without. The star lights the +x limb, so the
        // sub-observer point [0, 0, 1] sits in shadow (Lambert 0, ambient only): a brighter pixel there proves the
        // flash EMITS (like the lava glow, it survives on the dark side) rather than merely reflecting sunlight. A
        // pixel out at the lit limb, far from the crater, is unchanged, so the flash is a local bloom, not a tint.
        let (w, h) = (160usize, 120usize);
        let bg = Rgb::new(8, 9, 14);
        let cols = 8usize;
        let uniform: Vec<DerivedTile> = (0..cols * 8)
            .map(|_| DerivedTile {
                elevation: Fixed::from_int(1),
                relief: TerrainRelief::Lowland,
            })
            .collect();
        let (cx, cy, radius) = (80i32, 60i32, 48usize);
        let star = [1.0f32, 0.0, 0.0]; // lights the +x limb; the sub-observer point [0,0,1] is dark
        let white = Rgb::new(255, 255, 255);

        // One fresh impact at the sub-observer point (u, v) = (0.5, 0.5) -> body [0, 0, 1], at peak intensity
        // (epoch == formation), 200 km across on a 3000 km body so its bloom covers several pixels around centre.
        let radius_m = Fixed::from_int(3_000_000);
        let window = Fixed::from_int(60);
        let rows = vec![crater_row_formed_at(200_000, 20_000, 0)];
        let flashes = active_flash_stamps(&rows, radius_m, Fixed::ZERO, window);
        assert_eq!(
            flashes.len(),
            1,
            "the fresh impact is flashing at its formation"
        );

        let render = |flash: Option<&[ImpactFlash]>| -> Vec<u32> {
            let mut buf = vec![bg.pack(); w * h];
            draw_globe(
                &mut buf,
                w,
                h,
                cx,
                cy,
                radius,
                &uniform,
                latlon(&uniform, cols),
                star,
                white,
                SurfaceStyle::default(),
                GlobeOrientation::IDENTITY,
                None,
                None,
                flash,
            );
            buf
        };
        let lum = |px: u32| -> u32 { ((px >> 16) & 0xff) + ((px >> 8) & 0xff) + (px & 0xff) };

        let without = render(None);
        let with = render(Some(&flashes));
        let center = cy as usize * w + cx as usize;
        assert!(
            lum(with[center]) > lum(without[center]) + 100,
            "the fresh impact flashes bright over the dark sub-observer crust: without {} vs with {}",
            lum(without[center]),
            lum(with[center])
        );

        // A pixel out toward the lit +x limb (far from the crater's reach cone) is unchanged: the flash is local.
        let limb = cy as usize * w + (cx + 40) as usize;
        assert_eq!(
            with[limb], without[limb],
            "a sample far from the impact is untouched (the flash is a local bloom, not a global tint)"
        );
    }

    #[test]
    fn the_crater_stamp_is_a_bowl_inside_the_rim_and_an_ejecta_rim_outside() {
        // A 60 km crater 6 km deep on a 3000 km body: the stamp is the crater law's own shape. At the centre it is
        // the full bowl depth below the surface; at the rim it returns to the surface; just outside it rises as the
        // ejecta rim; far away it is zero. This proves the analytic stamp the renderer composes (rows not rasters).
        let radius_m = Fixed::from_int(3_000_000);
        let stamps = crater_stamps(&[crater_row(60_000, 6_000)], radius_m);
        assert_eq!(stamps.len(), 1, "the crater row prepares one stamp");
        let center = crater_uv_unit(Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2));
        let bottom = crater_relief_km(&stamps, center).to_f64_lossy();
        assert!(
            (bottom - (-6.0)).abs() < 0.2,
            "the bowl centre sits at minus the crater depth (~-6 km), got {bottom:.3} km"
        );
        // A point one rim-radius away (alpha = (30 km)/(3000 km) = 0.01 rad in longitude, at the equator lon =
        // u*2pi - pi so du = alpha/(2pi)): the paraboloid returns to zero at the rim.
        let alpha = 30_000.0 / 3_000_000.0; // 0.01 rad
        let du_rim = alpha / (2.0 * std::f64::consts::PI);
        let u_rim = Fixed::from_ratio(((0.5 + du_rim) * 1_000_000.0) as i64, 1_000_000);
        let at_rim = crater_relief_km(&stamps, crater_uv_unit(u_rim, Fixed::from_ratio(1, 2)))
            .to_f64_lossy();
        assert!(
            at_rim.abs() < 0.6,
            "the paraboloid bowl returns to the surface at the rim, got {at_rim:.3} km"
        );
        // Just outside the rim (1.5 rim-radii): the ejecta rim/blanket is positive (raised).
        let du_out = 1.5 * alpha / (2.0 * std::f64::consts::PI);
        let u_out = Fixed::from_ratio(((0.5 + du_out) * 1_000_000.0) as i64, 1_000_000);
        let outside = crater_relief_km(&stamps, crater_uv_unit(u_out, Fixed::from_ratio(1, 2)))
            .to_f64_lossy();
        assert!(
            outside > 0.0,
            "the ejecta rim outside the crater is raised (positive), got {outside:.3} km"
        );
        // Far away (the opposite side of the globe): no contribution.
        let far = crater_relief_km(
            &stamps,
            crater_uv_unit(Fixed::ZERO, Fixed::from_ratio(1, 2)),
        )
        .to_f64_lossy();
        assert_eq!(
            far, 0.0,
            "a crater contributes nothing to the far side of the globe"
        );
    }

    #[test]
    fn a_bigger_crater_stamps_a_deeper_wider_bowl() {
        // The morphology conditions on the crater law's outputs: a larger, deeper crater stamps a deeper bowl over a
        // wider footprint, from the SAME analytic function (rows carry the derived size).
        let radius_m = Fixed::from_int(3_000_000);
        let small = crater_stamps(&[crater_row(30_000, 3_000)], radius_m);
        let big = crater_stamps(&[crater_row(90_000, 9_000)], radius_m);
        let center = crater_uv_unit(Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2));
        let ds = crater_relief_km(&small, center).to_f64_lossy();
        let db = crater_relief_km(&big, center).to_f64_lossy();
        assert!(
            db < ds && db < -8.0 && ds > -4.0,
            "the bigger crater stamps a deeper bowl ({db:.2} km vs {ds:.2} km)"
        );
        // A point at a fixed small angular distance sits inside the big crater but outside the small one, so the big
        // crater still digs there while the small one is already past its rim (a wider footprint).
        let du = 20_000.0 / 3_000_000.0 / (2.0 * std::f64::consts::PI); // ~0.67 small-rim-radii, 0.44 big
        let off = crater_uv_unit(
            Fixed::from_ratio(((0.5 + du) * 1_000_000.0) as i64, 1_000_000),
            Fixed::from_ratio(1, 2),
        );
        assert!(
            crater_relief_km(&big, off).to_f64_lossy()
                < crater_relief_km(&small, off).to_f64_lossy(),
            "the wider crater digs at a distance where the narrow one has ended"
        );
    }

    #[test]
    fn a_degenerate_crater_row_prepares_no_stamp() {
        let radius_m = Fixed::from_int(3_000_000);
        assert!(
            crater_stamps(&[crater_row(0, 6_000)], radius_m).is_empty(),
            "a zero-diameter row prepares no stamp (skipped, not fabricated)"
        );
        assert!(
            crater_stamps(&[crater_row(60_000, 6_000)], Fixed::ZERO).is_empty(),
            "a zero-radius body prepares no stamp"
        );
    }

    #[test]
    fn organism_colour_is_deterministic_and_layer_keyed() {
        assert_eq!(
            organism_color(0, 7),
            organism_color(0, 7),
            "same inputs, same colour"
        );
        // Plants (layer 0) are greener than carnivores (layer 2): more green, less red.
        let plant = organism_color(0, 1);
        let carnivore = organism_color(2, 1);
        assert!(plant.g > plant.r, "a plant is green-dominant");
        assert!(carnivore.r > carnivore.g, "a carnivore is red-dominant");
    }

    #[test]
    fn superfine_paints_the_requested_size_and_marks_occupants() {
        let mut params = GenesisParams::dev_default();
        params.width = 48;
        params.height = 32;
        let living = genesis(
            0xEA27,
            &params,
            &civsim_sim::environ::AbioticSourceRegistry::earth_dev(),
            None,
        );
        let (w, h, tile_px) = (240usize, 160usize, 18usize);
        // Centre on an occupied tile so at least one organism mark is drawn.
        let center = living
            .occupants
            .occupied()
            .next()
            .expect("an occupied tile");
        let buf = super::superfine(
            &living,
            &BiomeSet::dev_default(),
            center,
            tile_px,
            w,
            h,
            Rgb::new(8, 9, 14),
        );
        assert_eq!(buf.len(), w * h, "one word per pixel");
        assert_eq!(
            buf,
            super::superfine(
                &living,
                &BiomeSet::dev_default(),
                center,
                tile_px,
                w,
                h,
                Rgb::new(8, 9, 14)
            ),
            "a pure read replays"
        );
        // The centre tile's block carries a mark distinct from the background colour.
        assert!(
            buf.iter().any(|&p| p != Rgb::new(8, 9, 14).pack()),
            "something is drawn"
        );
    }
    #[test]
    fn blackbody_colour_tracks_effective_temperature() {
        // The star colour is a deterministic pure read of the derived T_eff: the same temperature replays the same
        // colour, and the chromaticity walks the Planckian locus from red through white to blue.
        let sun = blackbody_rgb(Fixed::from_int(5772));
        assert_eq!(
            blackbody_rgb(Fixed::from_int(5772)),
            sun,
            "a pure read replays"
        );
        // The Sun (~5772 K) reads a warm near-white: every channel high, red the strongest, blue a shade lower.
        assert!(
            sun.r > 240 && sun.g > 220 && sun.b > 200,
            "the Sun is near-white, got {sun:?}"
        );
        assert!(sun.r >= sun.g && sun.g >= sun.b, "the Sun leans warm");
        // A cool M dwarf (~3000 K) reads reddish: red dominant, little blue.
        let m_dwarf = blackbody_rgb(Fixed::from_int(3000));
        assert!(
            m_dwarf.r > m_dwarf.b && m_dwarf.r >= m_dwarf.g,
            "an M dwarf is reddish, got {m_dwarf:?}"
        );
        assert!(m_dwarf.b < 160, "an M dwarf carries little blue");
        // A hot early-type star (~10000 K) reads blue-white: blue overtakes red.
        let hot = blackbody_rgb(Fixed::from_int(10000));
        assert!(hot.b > hot.r, "a hot star is bluish, got {hot:?}");
    }

    /// A small hand-built DERIVED-tile field for the globe-texture tests: a 6-wide grid banded by relief, so the
    /// sphere has a surface to wrap without loading the petrology registry.
    fn demo_globe_tiles() -> (Vec<DerivedTile>, usize) {
        let cols = 6usize;
        let rows = 6usize;
        let mut tiles = Vec::with_capacity(cols * rows);
        for r in 0..rows {
            let relief = match r {
                0 | 1 => TerrainRelief::Upland,
                2 | 3 => TerrainRelief::Lowland,
                _ => TerrainRelief::Submarine,
            };
            for _ in 0..cols {
                tiles.push(DerivedTile {
                    elevation: Fixed::from_int(r as i32),
                    relief,
                });
            }
        }
        (tiles, cols)
    }

    /// Wrap a lat-lon demo tile field as a [`SurfaceParam`] (rows derived from the field), so the globe-render tests
    /// drive the equirectangular sampling path exactly as before the cube-sphere migration (byte-identical).
    fn latlon(tiles: &[DerivedTile], cols: usize) -> SurfaceParam {
        SurfaceParam::LatLon {
            cols,
            rows: tiles.len() / cols,
        }
    }

    #[test]
    fn the_cube_sphere_map_round_trips_and_selects_the_expected_face() {
        // The forward map (face, s, t) -> direction and the inverse direction -> (face, s, t) are consistent: a cell
        // centre sampled forward and inverted returns its own face and coordinate. This is the load-bearing identity:
        // the cache writes a cell at forward(face, s, t) and the render reads it back by inverting the pixel direction,
        // so they MUST agree for the surface to sample correctly.
        let res = 32usize;
        for face in 0..6usize {
            for &(i, j) in &[(0usize, 0usize), (5, 9), (16, 16), (31, 31), (7, 24)] {
                let s = (i as f32 + 0.5) / res as f32;
                let t = (j as f32 + 0.5) / res as f32;
                let dir = cube_face_dir_f32(face, s, t);
                // The forward direction is a unit vector.
                let n = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
                assert!(
                    (n - 1.0).abs() < 1e-4,
                    "the cube direction is unit, got {n}"
                );
                let (f2, s2, t2) = cube_dir_to_face_st(dir);
                assert_eq!(
                    f2, face,
                    "the inverse recovers the face (face {face}, cell {i},{j})"
                );
                assert!(
                    (s2 - s).abs() < 1e-4 && (t2 - t).abs() < 1e-4,
                    "the inverse recovers (s, t): ({s},{t}) vs ({s2},{t2}) on face {face}"
                );
            }
        }
    }

    #[test]
    fn the_cube_sphere_spreads_the_budget_where_the_lat_lon_grid_pinches_at_the_pole() {
        // THE POLE-PINCH FIX, MEASURED rather than asserted. Count how many CACHE CELLS fall inside two caps of the
        // SAME solid angle, one centred on the north pole and one on the equator, at the two comparable budgets: the
        // lat-lon 1440x720 = 1,036,800 grid the cache used before, and the 6 x 416^2 = 1,038,336 cube-sphere it uses
        // now. The lat-lon grid CROWDS the pole (its cell area collapses as cos(lat)), so the polar cap swallows
        // several times its fair share of the budget while the equator is under-resolved; the cube-sphere spreads the
        // cells evenly, so the two caps hold comparable counts. One measurement, both parameterizations.
        const CAP_COS: f32 = 0.984_807_7; // cos(10 degrees)
        let pole = [0.0f32, 1.0, 0.0];
        let equator = [0.0f32, 0.0, 1.0];
        let in_cap = |d: [f32; 3], axis: [f32; 3]| {
            d[0] * axis[0] + d[1] * axis[1] + d[2] * axis[2] >= CAP_COS
        };
        // The lat-lon cache: each cell's centre direction is the sphere map at the cell centre.
        let (cols, rows) = (1440usize, 720usize);
        let (mut ll_pole, mut ll_eq) = (0usize, 0usize);
        for r in 0..rows {
            let v = (r as f32 + 0.5) / rows as f32;
            for c in 0..cols {
                let u = (c as f32 + 0.5) / cols as f32;
                let d = uv_to_body(u, v);
                ll_pole += in_cap(d, pole) as usize;
                ll_eq += in_cap(d, equator) as usize;
            }
        }
        // The cube-sphere cache: each cell's centre direction is the equi-angular forward map at the cell centre.
        let res = 416usize;
        let (mut cs_pole, mut cs_eq) = (0usize, 0usize);
        for face in 0..6usize {
            for j in 0..res {
                let t = (j as f32 + 0.5) / res as f32;
                for i in 0..res {
                    let s = (i as f32 + 0.5) / res as f32;
                    let d = cube_face_dir_f32(face, s, t);
                    cs_pole += in_cap(d, pole) as usize;
                    cs_eq += in_cap(d, equator) as usize;
                }
            }
        }
        let ll_ratio = ll_pole as f64 / ll_eq as f64;
        let cs_ratio = cs_pole as f64 / cs_eq as f64;
        eprintln!(
            "budget in a 10-degree cap: lat-lon pole {ll_pole} equator {ll_eq} (ratio {ll_ratio:.2}); \
             cube-sphere pole {cs_pole} equator {cs_eq} (ratio {cs_ratio:.2})"
        );
        assert!(
            ll_ratio > 5.0,
            "the lat-lon grid pinches: its polar cap swallows {ll_ratio:.1}x the equatorial cap's cells"
        );
        assert!(
            (0.6..1.6).contains(&cs_ratio),
            "the cube-sphere budget is uniform from pole to equator, got ratio {cs_ratio:.2}"
        );
    }

    #[test]
    fn the_cube_sphere_covers_the_poles_without_a_singularity() {
        // The lat-lon grid PINCHES at the poles (its cells collapse to zero area there). The cube-sphere does not: the
        // north pole (+y) and south pole (-y) map cleanly onto the +y / -y faces at their centres, and the six faces
        // tile the whole sphere. Sample a spread of directions (including both poles) and confirm each lands on a valid
        // face cell of a res x res x 6 field, so no direction is unmapped or pinched.
        let res = 16usize;
        let field_len = 6 * res * res;
        // Both poles land at the centre of their polar face (s ~ t ~ 0.5), never crowded onto a seam.
        let (fnorth, sn, tn) = cube_dir_to_face_st([0.0, 1.0, 0.0]);
        assert_eq!(fnorth, 2, "the north pole is the +y face");
        assert!(
            (sn - 0.5).abs() < 1e-3 && (tn - 0.5).abs() < 1e-3,
            "the north pole sits at the +y face centre, got ({sn},{tn})"
        );
        let (fsouth, _, _) = cube_dir_to_face_st([0.0, -1.0, 0.0]);
        assert_eq!(fsouth, 3, "the south pole is the -y face");
        // Every direction on a lattice over the sphere resolves to a valid, in-range cell (all six faces are exercised).
        let mut faces_hit = [false; 6];
        for a in 0..24 {
            for b in 0..12 {
                let lon = (a as f32 + 0.5) / 24.0 * std::f32::consts::TAU - std::f32::consts::PI;
                let lat = (0.5 - (b as f32 + 0.5) / 12.0) * std::f32::consts::PI;
                let dir = [lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos()];
                let (face, s, t) = cube_dir_to_face_st(dir);
                assert!(face < 6, "the direction lands on a valid face");
                faces_hit[face] = true;
                let idx = surface_cell_index(
                    SurfaceParam::CubeSphere { face_res: res },
                    dir,
                    0.0,
                    0.0,
                    field_len,
                )
                .expect("a direction maps to a cube cell");
                assert!(idx < field_len, "the cube cell index is in range");
                assert!((0.0..1.0).contains(&s) && (0.0..1.0).contains(&t));
            }
        }
        assert!(
            faces_hit.iter().all(|&h| h),
            "a full lattice exercises all six cube faces: {faces_hit:?}"
        );
    }

    /// The mean luminance of the disk pixels on one side of the vertical centre line at `cx`.
    fn half_luminance(buf: &[u32], w: usize, cx: i32, cy: i32, r: i32, right: bool) -> f64 {
        let mut sum = 0f64;
        let mut n = 0f64;
        for py in (cy - r).max(0)..=(cy + r) {
            for px in (cx - r).max(0)..=(cx + r) {
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                if right != (dx > 0) {
                    continue;
                }
                let word = buf[py as usize * w + px as usize];
                let rgb = Rgb::new((word >> 16) as u8, (word >> 8) as u8, word as u8);
                sum += rgb.luminance() as f64;
                n += 1.0;
            }
        }
        if n == 0.0 {
            0.0
        } else {
            sum / n
        }
    }

    #[test]
    fn the_globe_is_a_lit_sphere_sized_from_the_derived_radius() {
        use civsim_sim::astro;
        // The on-screen size scales from the DERIVED planet radius: a denser planet of the same mass has a smaller
        // derived radius and draws a smaller disk at the same view scale (a pure read of planet_radius_m).
        let m_per_px = Fixed::from_int(30_000);
        let earth = astro::planet_radius_m(Fixed::ONE, Fixed::from_ratio(5514, 1000))
            .expect("earth radius");
        let dense = astro::planet_radius_m(Fixed::ONE, Fixed::from_int(8)).expect("dense radius");
        let earth_px = globe_radius_px(earth, m_per_px);
        let dense_px = globe_radius_px(dense, m_per_px);
        assert!(
            earth_px > 0 && dense_px > 0,
            "both globes have an on-screen size"
        );
        assert!(
            earth_px > dense_px,
            "a denser, smaller planet draws a smaller globe"
        );
        assert_eq!(
            globe_radius_px(Fixed::ZERO, m_per_px),
            0,
            "no radius, no globe"
        );

        // Draw the Earth globe lit from the right (+x). The star-facing (right) hemisphere reads brighter than the
        // night (left) side, the terminator running down the middle, and the render replays byte for byte.
        let (w, h) = (200usize, 160usize);
        let bg = Rgb::new(8, 9, 14);
        let (tiles, cols) = demo_globe_tiles();
        let (cx, cy) = (100i32, 80i32);
        let radius = 64usize;
        let mut buf = vec![bg.pack(); w * h];
        draw_globe(
            &mut buf,
            w,
            h,
            cx,
            cy,
            radius,
            &tiles,
            latlon(&tiles, cols),
            [1.0, 0.0, 0.0],
            Rgb::new(255, 255, 255),
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
            None,
            None,
            None,
        );
        assert!(buf.iter().any(|&p| p != bg.pack()), "the globe is drawn");
        let right = half_luminance(&buf, w, cx, cy, radius as i32, true);
        let left = half_luminance(&buf, w, cx, cy, radius as i32, false);
        assert!(
            right > left * 1.5,
            "the sunlit hemisphere is brighter than the night side (right {right:.1} vs left {left:.1})"
        );
        let mut replay = vec![bg.pack(); w * h];
        draw_globe(
            &mut replay,
            w,
            h,
            cx,
            cy,
            radius,
            &tiles,
            latlon(&tiles, cols),
            [1.0, 0.0, 0.0],
            Rgb::new(255, 255, 255),
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
            None,
            None,
            None,
        );
        assert_eq!(buf, replay, "a pure read replays byte for byte");
    }

    #[test]
    fn the_derived_sun_direction_lights_the_sub_solar_face_and_reads_the_solar_elevation() {
        use civsim_sim::orbit;
        // The load-bearing identity: the body-frame sun vector dotted with a surface point's body-frame normal is the
        // physical solar-elevation cosine, so draw_globe's Lambert term yields the derived illumination for free. Check
        // it against orbit::solar_elevation_cosine at several surface points, for a tilted, off-meridian sub-solar point.
        let decl_f = 0.30f32;
        let sslon_f = 0.50f32;
        let s = sub_solar_body_dir(decl_f, sslon_f);
        let mag = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
        assert!(
            (mag - 1.0).abs() < 1e-4,
            "the derived sun direction is unit length, got {mag}"
        );
        let decl = Fixed::from_ratio(3, 10);
        let sslon = Fixed::from_ratio(1, 2);
        for &(lat_i, lon_i) in &[(1i64, 5i64), (-3, 2), (7, -4), (0, 0)] {
            let lat_f = lat_i as f32 / 10.0;
            let lon_f = lon_i as f32 / 5.0;
            // The surface normal in draw_globe's body frame: (cos lat sin lon, sin lat, cos lat cos lon).
            let (sl, cl) = lat_f.sin_cos();
            let (slon, clon) = lon_f.sin_cos();
            let n = [cl * slon, sl, cl * clon];
            let dot = n[0] * s[0] + n[1] * s[1] + n[2] * s[2];
            let expected = orbit::solar_elevation_cosine(
                Fixed::from_ratio(lat_i, 10),
                Fixed::from_ratio(lon_i, 5),
                decl,
                sslon,
            )
            .expect("solar elevation")
            .to_f64_lossy() as f32;
            assert!(
                (dot - expected).abs() < 2e-3,
                "N.S is the solar-elevation cosine at ({lat_f},{lon_f}): {dot} vs {expected}"
            );
        }
        // At IDENTITY the derived light lands on the expected face: a positive declination (northern sub-solar point)
        // and a positive sub-solar longitude (eastern) light the NORTH-EAST of the disk, so the brightest disk pixel
        // sits up (smaller screen y) and to the right (larger screen x) of the globe centre.
        let (w, h) = (160usize, 160usize);
        let bg = Rgb::new(6, 7, 12);
        let (tiles, cols) = demo_globe_tiles();
        let (cx, cy) = (80i32, 80i32);
        let radius = 60usize;
        let mut buf = vec![bg.pack(); w * h];
        draw_globe(
            &mut buf,
            w,
            h,
            cx,
            cy,
            radius,
            &tiles,
            latlon(&tiles, cols),
            s,
            Rgb::new(255, 255, 255),
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
            None,
            None,
            None,
        );
        let mut best = (-1.0f32, cx, cy);
        for py in (cy - radius as i32).max(0)..=(cy + radius as i32) {
            for px in (cx - radius as i32).max(0)..=(cx + radius as i32) {
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy > (radius as i32) * (radius as i32) {
                    continue;
                }
                let word = buf[py as usize * w + px as usize];
                let lum = Rgb::new((word >> 16) as u8, (word >> 8) as u8, word as u8).luminance();
                if (lum as f32) > best.0 {
                    best = (lum as f32, px, py);
                }
            }
        }
        assert!(
            best.1 > cx && best.2 < cy,
            "the derived sun lights the north-east face (brightest at ({},{}) vs centre ({cx},{cy}))",
            best.1,
            best.2
        );
    }

    #[test]
    fn the_system_map_traces_orbit_ellipses_and_places_the_planet_dots() {
        // The system map draws a dim orbit ellipse and a dot for each body, with the star at the centre. Two bodies at
        // different semi-major axes place their dots at different radii from the centre, and each dot reads its own
        // colour. A pure display read of orbit.rs geometry.
        let (w, h) = (240usize, 200usize);
        let bg = Rgb::new(8, 9, 14);
        let inner = Rgb::new(200, 120, 90);
        let outer = Rgb::new(120, 160, 220);
        let bodies = [
            MapBody {
                semi_major_au: Fixed::from_ratio(7, 10),
                eccentricity: Fixed::from_ratio(1, 10),
                mean_anomaly: Fixed::ZERO,
                dot_color: inner,
                dot_px: 4,
            },
            MapBody {
                semi_major_au: Fixed::from_ratio(15, 10),
                eccentricity: Fixed::from_ratio(1, 10),
                mean_anomaly: Fixed::ZERO,
                dot_color: outer,
                dot_px: 4,
            },
        ];
        let sun_t = Fixed::from_int(5772);
        let (buf, dots) = render_system_map(w, h, bg, sun_t, 8, &bodies);
        assert_eq!(buf.len(), w * h, "one word per pixel");
        assert_eq!(dots.len(), 2, "one dot centre per body");
        // The star sits at the centre in its blackbody colour.
        let (cx, cy) = ((w / 2) as i32, (h / 2) as i32);
        assert_eq!(
            buf[cy as usize * w + cx as usize],
            blackbody_rgb(sun_t).pack(),
            "the star reads its blackbody colour at the centre"
        );
        // Both dots are at perihelion (mean anomaly 0): the outer planet's dot is farther from the centre than the
        // inner planet's, along the same axis.
        let inner_dx = (dots[0].0 - cx).abs();
        let outer_dx = (dots[1].0 - cx).abs();
        assert!(
            outer_dx > inner_dx,
            "the wider orbit places its dot farther out ({outer_dx} vs {inner_dx})"
        );
        // The dot pixels carry their bodies' colours.
        assert_eq!(
            buf[dots[0].1 as usize * w + dots[0].0 as usize],
            inner.pack(),
            "the inner dot reads its colour"
        );
        assert_eq!(
            buf[dots[1].1 as usize * w + dots[1].0 as usize],
            outer.pack(),
            "the outer dot reads its colour"
        );
        // Some orbit-line pixels are drawn (the ellipse is visible against the background).
        let orbit_pixels = buf
            .iter()
            .filter(|&&p| p == Rgb::new(70, 78, 96).pack())
            .count();
        assert!(orbit_pixels > 50, "the orbit ellipses are traced");
        let (replay, _) = render_system_map(w, h, bg, sun_t, 8, &bodies);
        assert_eq!(buf, replay, "a pure display read replays byte for byte");
    }

    #[test]
    fn the_solar_view_lights_the_globe_from_the_star_and_tints_by_temperature() {
        use civsim_sim::astro;
        let (w, h) = (240usize, 180usize);
        let bg = Rgb::new(6, 7, 12);
        let (tiles, cols) = demo_globe_tiles();
        let radius_m = astro::planet_radius_m(Fixed::ONE, Fixed::from_ratio(5514, 1000))
            .expect("earth radius");
        // A view scale that draws Earth's globe at a legible size, the star off to the left of the planet.
        let m_per_px = Fixed::from_int(80_000);
        let sun_t = astro::stellar_effective_temperature(
            Fixed::ONE,
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(8, 10),
            Fixed::from_int(50_000),
        )
        .expect("sun T_eff");
        let star_px = (24i32, 40i32); // upper-left of the centred planet
        let frame = render_solar_system_view(
            radius_m,
            sun_t,
            &tiles,
            latlon(&tiles, cols),
            w,
            h,
            m_per_px,
            star_px,
            10,
            bg,
            PLACEHOLDER_SKY,
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
            None,
            None,
            None,
            None,
        );
        assert_eq!(frame.len(), w * h, "one word per pixel");
        // The star disk carries the derived blackbody colour at its core.
        let star_core = frame[star_px.1 as usize * w + star_px.0 as usize];
        assert_eq!(
            star_core,
            blackbody_rgb(sun_t).pack(),
            "the star reads its blackbody colour"
        );
        // The day side faces the star: with the star upper-left, the globe's LEFT hemisphere is brighter.
        let (pcx, pcy) = ((w / 2) as i32, (h / 2) as i32);
        let pr = globe_radius_px(radius_m, m_per_px) as i32;
        let left = half_luminance(&frame, w, pcx, pcy, pr, false);
        let right = half_luminance(&frame, w, pcx, pcy, pr, true);
        assert!(
            left > right * 1.3,
            "the star-facing (left) hemisphere is the day side (left {left:.1} vs right {right:.1})"
        );
        // Deterministic pure read.
        let replay = render_solar_system_view(
            radius_m,
            sun_t,
            &tiles,
            latlon(&tiles, cols),
            w,
            h,
            m_per_px,
            star_px,
            10,
            bg,
            PLACEHOLDER_SKY,
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
            None,
            None,
            None,
            None,
        );
        assert_eq!(frame, replay, "a pure read replays byte for byte");

        // The star's blackbody colour tints the sunlight: a cool ~3200 K star warms the day side (a higher
        // red-to-blue ratio) versus a hot ~9000 K star, at the same geometry.
        let cool = astro::stellar_effective_temperature(
            Fixed::from_ratio(6, 10),
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(8, 10),
            Fixed::from_int(50_000),
        )
        .expect("cool T_eff");
        let hot = astro::stellar_effective_temperature(
            Fixed::from_int(3),
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(8, 10),
            Fixed::from_int(50_000),
        )
        .expect("hot T_eff");
        let day_ratio = |t_eff: Fixed| -> f64 {
            let f = render_solar_system_view(
                radius_m,
                t_eff,
                &tiles,
                latlon(&tiles, cols),
                w,
                h,
                m_per_px,
                star_px,
                10,
                bg,
                PLACEHOLDER_SKY,
                SurfaceStyle::default(),
                GlobeOrientation::IDENTITY,
                None,
                None,
                None,
                None,
            );
            let mut sr = 0f64;
            let mut sb = 0f64;
            for py in (pcy - pr).max(0)..=(pcy + pr) {
                for px in (pcx - pr).max(0)..=(pcx + pr) {
                    let dx = px - pcx;
                    let dy = py - pcy;
                    if dx * dx + dy * dy > pr * pr || dx > 0 {
                        continue; // the lit (left) hemisphere
                    }
                    let word = f[py as usize * w + px as usize];
                    sr += (word >> 16) as u8 as f64;
                    sb += (word & 0xff) as f64;
                }
            }
            (sr + 1.0) / (sb + 1.0)
        };
        assert!(
            day_ratio(cool) > day_ratio(hot),
            "a cool star warms the day side more than a hot star"
        );
    }

    #[test]
    fn the_atmosphere_limb_is_a_day_bright_haze_ring() {
        let unpack = |word: u32| Rgb::new((word >> 16) as u8, (word >> 8) as u8, word as u8);
        let (w, h) = (200usize, 200usize);
        let bg = Rgb::new(6, 7, 12);
        let (cx, cy) = (100i32, 100i32);
        let radius = 60usize;
        let mut buf = vec![bg.pack(); w * h];
        // Star to the right (+x): the day limb is on the right, the night limb on the left.
        draw_atmosphere_limb(
            &mut buf,
            w,
            h,
            cx,
            cy,
            radius,
            [1.0, 0.0, 0.2],
            PLACEHOLDER_SKY,
        );
        // Just outside the day (right) limb the pixel is tinted toward the sky colour: bluer and brighter than space.
        let day = unpack(buf[cy as usize * w + (cx + radius as i32 + 3) as usize]);
        assert!(
            day.b > bg.b + 10 && day.b > day.r,
            "the day limb glows sky-blue, got {day:?}"
        );
        // The night (left) limb at the same offset is dimmer than the day limb.
        let night = unpack(buf[cy as usize * w + (cx - radius as i32 - 3) as usize]);
        assert!(
            day.b > night.b,
            "the day limb is brighter than the night limb"
        );
        // The far background is untouched: the haze is confined to the limb.
        assert_eq!(buf[5 * w + 5], bg.pack(), "the haze stays at the limb");
        // Deterministic pure read.
        let mut replay = vec![bg.pack(); w * h];
        draw_atmosphere_limb(
            &mut replay,
            w,
            h,
            cx,
            cy,
            radius,
            [1.0, 0.0, 0.2],
            PLACEHOLDER_SKY,
        );
        assert_eq!(buf, replay, "a pure read replays byte for byte");
    }

    #[test]
    fn physics_terrain_colour_reflects_the_fields() {
        let p = |n: i64| Fixed::from_ratio(n, 100);
        // A deterministic pure read of the tile's physical fields.
        assert_eq!(
            physics_terrain_color(p(60), p(50), p(50)),
            physics_terrain_color(p(60), p(50), p(50)),
        );
        // Low elevation is water: blue-dominant.
        let water = physics_terrain_color(p(10), p(50), p(50));
        assert!(water.b > water.r && water.b > water.g, "water reads blue");
        // Wet temperate land is green-dominant; drier ground at the same elevation is warmer.
        let meadow = physics_terrain_color(p(50), p(80), p(50));
        assert!(
            meadow.g > meadow.r && meadow.g > meadow.b,
            "wet temperate land reads green"
        );
        let dry = physics_terrain_color(p(50), p(10), p(50));
        assert!(dry.r > meadow.r, "drier ground is warmer than a meadow");
        // A cold high peak lightens toward snow and rock.
        let peak = physics_terrain_color(p(95), p(40), p(10));
        assert!(
            peak.r > 180 && peak.g > 180 && peak.b > 180,
            "a cold peak reads pale"
        );
    }

    #[test]
    fn the_derived_tile_glyph_and_colour_key_off_relief() {
        // The render mapping is a pure, distinct read of the relief class: three classes, three glyphs, three
        // colours. Water reads bluest; the upland reads lighter than the lowland (the raised-rock swatch), so the
        // frame's contrast tracks the derived relief.
        assert_eq!(derived_tile_glyph(TerrainRelief::Submarine), '~');
        assert_eq!(derived_tile_glyph(TerrainRelief::Lowland), '.');
        assert_eq!(derived_tile_glyph(TerrainRelief::Upland), '^');
        let sub = derived_tile_color(TerrainRelief::Submarine);
        let low = derived_tile_color(TerrainRelief::Lowland);
        let up = derived_tile_color(TerrainRelief::Upland);
        assert!(sub.b > sub.r && sub.b > sub.g, "submarine reads blue");
        assert!(
            up.luminance() > low.luminance(),
            "the upland reads lighter than the lowland"
        );
        assert!(
            sub != low && low != up && sub != up,
            "each relief has a distinct swatch"
        );
    }

    #[test]
    fn paint_derived_tiles_replays_and_shows_the_relief() {
        // The paint is a deterministic pure read: the same derived field paints the same frame, byte for byte. A
        // hand-built render-test field (labelled test-only) of one submarine and one upland tile shows both relief
        // swatches in the frame.
        let field = [
            DerivedTile {
                elevation: Fixed::from_int(-5),
                relief: TerrainRelief::Submarine,
            },
            DerivedTile {
                elevation: Fixed::from_int(9),
                relief: TerrainRelief::Upland,
            },
        ];
        let (cols, tile_px, w, h) = (2usize, 16usize, 32usize, 16usize);
        let bg = Rgb::new(8, 9, 14);
        let frame = paint_derived_tiles(&field, cols, tile_px, w, h, bg);
        assert_eq!(frame.len(), w * h, "one word per pixel");
        assert_eq!(
            frame,
            paint_derived_tiles(&field, cols, tile_px, w, h, bg),
            "a pure read replays byte for byte"
        );
        assert!(
            frame.contains(&derived_tile_color(TerrainRelief::Submarine).pack()),
            "the submarine swatch is in the frame"
        );
        assert!(
            frame.contains(&derived_tile_color(TerrainRelief::Upland).pack()),
            "the upland swatch is in the frame"
        );
    }

    #[test]
    fn an_authored_composition_yields_a_visible_frame_whose_terrain_is_derived() {
        // THE VISIBLE SPINE, END TO END: the labelled Slice-0 demo field (its per-tile composition the only authored
        // input) drives the real substrate to derived elevations, the field datum, and the relief by crossing it,
        // and that DERIVED field paints a frame. The light silica band floats to Upland, the forsterite to Lowland,
        // the dense periclase below the datum to Submarine, so the terrain in the window is what the material is,
        // never fractal noise (the R1 override reaching the viewer). Colour is authored only in the swatch; the
        // relief that selects it is derived. Generation lives in the sim lane; the viewer only reads and paints.
        let tiles =
            civsim_sim::geodynamics::slice0_demo_field(6, 6).expect("the derived demo field");
        // The frame carries all three DERIVED relief classes.
        let has = |r: TerrainRelief| tiles.iter().any(|t| t.relief == r);
        assert!(has(TerrainRelief::Upland), "a light band derives upland");
        assert!(has(TerrainRelief::Lowland), "a middle band derives lowland");
        assert!(
            has(TerrainRelief::Submarine),
            "a dense band derives submarine"
        );
        let bg = Rgb::new(8, 9, 14);
        let frame = paint_derived_tiles(&tiles, 6, 16, 96, 96, bg);
        assert_eq!(
            frame,
            paint_derived_tiles(&tiles, 6, 16, 96, 96, bg),
            "the derived-terrain frame is a deterministic pure read"
        );
        // The frame shows the three DERIVED relief swatches: the terrain reached the window from composition alone.
        assert!(frame.contains(&derived_tile_color(TerrainRelief::Upland).pack()));
        assert!(frame.contains(&derived_tile_color(TerrainRelief::Lowland).pack()));
        assert!(frame.contains(&derived_tile_color(TerrainRelief::Submarine).pack()));
        assert!(
            frame.iter().any(|&p| p != bg.pack()),
            "a derived frame is painted"
        );
    }

    #[test]
    fn the_formula_parser_is_general() {
        // No hardcoded gas list: an uppercase letter opens a symbol, lowercase continues it, trailing digits are
        // the count (default 1). A new gas is a data row (a formula string), never a code change.
        assert_eq!(
            parse_formula("CO2"),
            vec![("C".to_string(), 1), ("O".to_string(), 2)]
        );
        assert_eq!(
            parse_formula("H2O"),
            vec![("H".to_string(), 2), ("O".to_string(), 1)]
        );
        assert_eq!(
            parse_formula("CH4"),
            vec![("C".to_string(), 1), ("H".to_string(), 4)]
        );
        assert_eq!(parse_formula("N2"), vec![("N".to_string(), 2)]);
        // A two-letter symbol continues through its lowercase tail.
        assert_eq!(parse_formula("Ar"), vec![("Ar".to_string(), 1)]);
        // A malformed fragment yields no atoms rather than a panic.
        assert!(parse_formula("").is_empty());
        assert!(parse_formula("123").is_empty());
    }

    #[test]
    fn modern_earth_air_derives_a_blue_sky() {
        // Modern Earth air (N2/O2/Ar) at the Sun's effective temperature scatters into a blue sky: the DERIVED
        // Rayleigh weighting (alpha^2 / lambda^4 from the banked polarizability substrate) drives the blue band
        // above the red. Assert the RELATIVE relationship only, never an absolute RGB.
        let tbl = PeriodicTable::standard().expect("the periodic table loads");
        let sun = Fixed::from_int(5772);
        let sky = rayleigh_sky_rgb(&[("N2", 0.78), ("O2", 0.21), ("Ar", 0.01)], sun, &tbl)
            .expect("modern air resolves through N and O");
        assert!(
            sky.b > sky.r,
            "modern Earth air derives a blue sky (blue {} exceeds red {})",
            sky.b,
            sky.r
        );
        // Deterministic pure read: the same mix replays byte for byte.
        assert_eq!(
            sky,
            rayleigh_sky_rgb(&[("N2", 0.78), ("O2", 0.21), ("Ar", 0.01)], sun, &tbl).unwrap()
        );
    }

    #[test]
    fn a_co2_atmosphere_is_less_blue_than_earth_air() {
        // The qualitative Hadean/Venusian shift: a CO2-dominated atmosphere is more polarizable, so its short
        // bands push toward saturation and the sky DESATURATES, a lower blue-to-red ratio than modern air. Assert
        // the RATIO ORDERING, never absolute values.
        let tbl = PeriodicTable::standard().expect("the periodic table loads");
        let sun = Fixed::from_int(5772);
        let air = rayleigh_sky_rgb(&[("N2", 0.78), ("O2", 0.21), ("Ar", 0.01)], sun, &tbl)
            .expect("air resolves");
        let co2 =
            rayleigh_sky_rgb(&[("CO2", 0.95), ("N2", 0.05)], sun, &tbl).expect("CO2 resolves");
        let ratio = |c: Rgb| (c.b as f64 + 1.0) / (c.r as f64 + 1.0);
        assert!(
            ratio(co2) < ratio(air),
            "a CO2 sky is less blue-dominant than air (CO2 blue/red {:.2} below air {:.2})",
            ratio(co2),
            ratio(air)
        );
    }

    #[test]
    fn an_unresolvable_mix_returns_none() {
        // Fail-soft: an empty mix, or one whose every element the substrate cannot resolve (no cited ionization
        // energy), returns None so the caller falls back to no atmosphere tint rather than a fabricated colour.
        let tbl = PeriodicTable::standard().expect("the periodic table loads");
        let sun = Fixed::from_int(5772);
        assert_eq!(
            rayleigh_sky_rgb(&[], sun, &tbl),
            None,
            "an empty mix is None"
        );
        // Argon carries no cited ionization energy in the table, so an Ar-only atmosphere resolves to nothing.
        assert_eq!(
            rayleigh_sky_rgb(&[("Ar", 1.0)], sun, &tbl),
            None,
            "an all-unresolvable mix is None"
        );
    }

    fn mat_comp(pairs: &[(&str, i64)]) -> Vec<(String, Fixed)> {
        pairs
            .iter()
            .map(|(s, n)| (s.to_string(), Fixed::from_int(*n as i32)))
            .collect()
    }

    #[test]
    fn a_small_gap_absorber_reads_dark_and_a_wide_gap_solid_reads_light() {
        // THE DARK-VERSUS-LIGHT MECHANISM, from the material's OWN banked gap, no authored swatch: silicon's cited
        // 1.12 eV gap sits BELOW the human visible window (~1.6-3.1 eV), so its interband onset absorbs across the
        // whole window and it reads dark; magnesium oxide's cited 7.8 eV gap sits far ABOVE the window, so it has no
        // visible absorption and reads light (it takes the star's colour). The colour is a pure read of the DERIVED
        // absorption spectrum, deterministic and replaying.
        let gaps = BandGapColumn::standard().expect("the gap column loads");
        let crystal = CrystalFieldTables::standard().expect("the crystal-field table loads");
        let table = PeriodicTable::standard().expect("the periodic table loads");
        let sun = Fixed::from_int(5772);
        let t = Fixed::from_int(300);
        let si = material_surface_rgb(&mat_comp(&[("Si", 1)]), sun, t, &gaps, &crystal, &table)
            .expect("silicon resolves");
        let mgo = material_surface_rgb(
            &mat_comp(&[("Mg", 1), ("O", 1)]),
            sun,
            t,
            &gaps,
            &crystal,
            &table,
        )
        .expect("MgO resolves");
        assert!(
            si.luminance() < mgo.luminance(),
            "the small-gap absorber (Si) is darker than the wide-gap solid (MgO): {} vs {}",
            si.luminance(),
            mgo.luminance()
        );
        assert!(si.luminance() < 40, "silicon reads dark, got {si:?}");
        assert!(mgo.luminance() > 180, "MgO reads light, got {mgo:?}");
        // A pure read replays byte for byte.
        assert_eq!(
            si,
            material_surface_rgb(&mat_comp(&[("Si", 1)]), sun, t, &gaps, &crystal, &table).unwrap()
        );
    }

    #[test]
    fn quartz_reads_light() {
        // Quartz (SiO2) is not a seeded gap-column phase and carries no crystal-field d-d line (no d-block cation),
        // so it has NO optical feature in the visible window: its reflectance is unity across the bands and it takes
        // the star's colour at full brightness (a light, warm-white silicate under the Sun). The honest read for a
        // colourless insulator, whose intrinsic band structure gives it no visible-window absorption.
        let gaps = BandGapColumn::standard().expect("the gap column loads");
        let crystal = CrystalFieldTables::standard().expect("the crystal-field table loads");
        let table = PeriodicTable::standard().expect("the periodic table loads");
        let sun = Fixed::from_int(5772);
        let t = Fixed::from_int(300);
        let quartz = material_surface_rgb(
            &mat_comp(&[("Si", 1), ("O", 2)]),
            sun,
            t,
            &gaps,
            &crystal,
            &table,
        )
        .expect("quartz resolves");
        assert!(
            quartz.luminance() > 180,
            "quartz reads light, got {quartz:?}"
        );
    }

    #[test]
    fn the_same_material_warms_under_a_cooler_star() {
        // STAR WARMTH: the reflected colour is the material's reflectance times the star's Planck spectrum, so the
        // SAME material reflects a warmer (higher red-to-blue) colour under a cool star than under a hot one, tracking
        // the illuminant. A featureless silicate (quartz) reflects the star's own colour, the cleanest witness.
        let gaps = BandGapColumn::standard().expect("the gap column loads");
        let crystal = CrystalFieldTables::standard().expect("the crystal-field table loads");
        let table = PeriodicTable::standard().expect("the periodic table loads");
        let t = Fixed::from_int(300);
        let quartz = mat_comp(&[("Si", 1), ("O", 2)]);
        let cool = material_surface_rgb(&quartz, Fixed::from_int(3000), t, &gaps, &crystal, &table)
            .expect("cool-star quartz");
        let hot = material_surface_rgb(&quartz, Fixed::from_int(9000), t, &gaps, &crystal, &table)
            .expect("hot-star quartz");
        let warmth = |c: Rgb| (c.r as f64 + 1.0) / (c.b as f64 + 1.0);
        assert!(
            warmth(cool) > warmth(hot),
            "a cool star reflects warmer than a hot star (red/blue {:.2} vs {:.2})",
            warmth(cool),
            warmth(hot)
        );
    }

    #[test]
    fn the_iron_oxidation_state_sets_the_crust_colour_ferric_dark_red_mixed_near_black_ferrous_light(
    ) {
        // SEAM 2, the iron dark-crust optics, keyed on the DERIVED iron oxidation state (the phase is the state):
        //  - HEMATITE (Fe2O3, Fe3+): the intense O2- -> Fe3+ charge-transfer edge (3.1 eV) whose broad Marcus-Hush
        //    tail floods the visible reddens and darkens it (blue absorbed more than red).
        //  - MAGNETITE (Fe3O4, mixed Fe2+/Fe3+): that edge PLUS the Fe2+ -> Fe3+ intervalence band (0.6 eV) absorbs
        //    across the visible and reads near-black, darker than hematite.
        //  - WUSTITE (FeO, Fe2+/ferrous): NO charge-transfer band, only the near-IR d-d line (~0.93 eV) below the
        //    visible, so it correctly stays LIGHT (the honest per-valence outcome, the old LAW-3 exhibit preserved:
        //    the ferrous limit is pinned with a test, never tuned away).
        //  - ENSTATITE (MgSiO3, iron-free): no iron chromophore at all, stays LIGHT.
        // The colour is a one-way projection of the DERIVED absorption spectrum; the canon authors none of it.
        let gaps = BandGapColumn::standard().expect("the gap column loads");
        let crystal = CrystalFieldTables::standard().expect("the crystal-field table loads");
        let table = PeriodicTable::standard().expect("the periodic table loads");
        let sun = Fixed::from_int(5772);
        let t = Fixed::from_int(300);
        let surf = |c: &[(&str, i64)]| {
            material_surface_rgb(&mat_comp(c), sun, t, &gaps, &crystal, &table)
                .expect("the material resolves")
        };
        let hematite = surf(&[("Fe", 2), ("O", 3)]);
        let magnetite = surf(&[("Fe", 3), ("O", 4)]);
        let wustite = surf(&[("Fe", 1), ("O", 1)]);
        let enstatite = surf(&[("Mg", 1), ("Si", 1), ("O", 3)]);
        // Hematite reads dark and red-dominant (blue absorbed more than red by the charge-transfer tail).
        assert!(
            hematite.luminance() < 80,
            "hematite (ferric) reads dark, got {hematite:?}"
        );
        assert!(
            hematite.r > hematite.b,
            "hematite reddens (red reflected over blue), got {hematite:?}"
        );
        // Magnetite reads near-black, darker than hematite (the added intervalence band).
        assert!(
            magnetite.luminance() < hematite.luminance(),
            "magnetite (mixed valence) is darker than hematite: {} vs {}",
            magnetite.luminance(),
            hematite.luminance()
        );
        assert!(
            magnetite.luminance() < 40,
            "magnetite reads near-black, got {magnetite:?}"
        );
        // Ferrous wustite and iron-free enstatite stay light (no charge-transfer band).
        assert!(
            wustite.luminance() > 180,
            "wustite (ferrous) stays light: only the near-IR d-d line, no charge-transfer band, got {wustite:?}"
        );
        assert!(
            enstatite.luminance() > 180,
            "iron-free enstatite stays light, got {enstatite:?}"
        );
        // A pure read replays byte for byte (deterministic projection).
        assert_eq!(hematite, surf(&[("Fe", 2), ("O", 3)]));
    }

    #[test]
    fn paint_material_tiles_replays_and_shades_by_relief() {
        // The material-tile paint is a deterministic pure read: the same tiles and material paint the same frame, byte
        // for byte, and the relief shading dims the material colour (an upland tile is brighter than a lowland tile of
        // the same material). A hand-built two-tile field (labelled test-only) exercises both relief shadings.
        let field = [
            DerivedTile {
                elevation: Fixed::from_int(9),
                relief: TerrainRelief::Upland,
            },
            DerivedTile {
                elevation: Fixed::from_int(1),
                relief: TerrainRelief::Lowland,
            },
        ];
        let material = Rgb::new(200, 210, 220);
        let bg = Rgb::new(8, 9, 14);
        let (cols, tile_px, w, h) = (2usize, 16usize, 32usize, 16usize);
        let frame = paint_material_tiles(&field, material, cols, tile_px, w, h, bg);
        assert_eq!(frame.len(), w * h, "one word per pixel");
        assert_eq!(
            frame,
            paint_material_tiles(&field, material, cols, tile_px, w, h, bg),
            "a pure read replays byte for byte"
        );
        // The upland block (left) is brighter than the lowland block (right): read a pixel inside each block.
        let unpack = |word: u32| Rgb::new((word >> 16) as u8, (word >> 8) as u8, word as u8);
        let upland = unpack(frame[8 * w + 4]);
        let lowland = unpack(frame[8 * w + 20]);
        assert!(
            upland.luminance() > lowland.luminance(),
            "the upland tile is brighter than the lowland tile (relief shading): {} vs {}",
            upland.luminance(),
            lowland.luminance()
        );
    }

    #[test]
    fn surface_pick_inverts_the_sphere_map_and_fails_soft_off_the_disk() {
        let (cx, cy, r) = (100i32, 100i32, 60usize);
        let (cols, rows) = (6usize, 6usize);
        let o = GlobeOrientation::IDENTITY;
        // A pixel well outside the disk yields no tile (fail-soft: the caller then draws no highlight).
        assert_eq!(
            pick_surface_tile(cx + 200, cy, cx, cy, r, o, cols, rows),
            None,
            "off the disk, no pick"
        );
        // The globe centre maps to the middle of the surface (u ~ 0.5, v ~ 0.5): the centre column and row.
        let (cu, cv) =
            pick_surface_tile(cx, cy, cx, cy, r, o, cols, rows).expect("the centre is on the disk");
        assert_eq!(cu, cols / 2, "the centre pixel samples the middle meridian");
        assert_eq!(cv, rows / 2, "the centre pixel samples the equator row");
        // Deterministic pure inverse.
        assert_eq!(
            pick_surface_tile(cx, cy, cx, cy, r, o, cols, rows),
            pick_surface_tile(cx, cy, cx, cy, r, o, cols, rows),
            "a pure inverse replays"
        );
    }

    #[test]
    fn a_latitude_rotation_brings_the_far_surface_to_the_centre() {
        // Rotating in latitude pans toward the pole, so the centre pixel samples a higher-latitude row: the far side
        // of the sphere is reachable by rotation (the pan the derived viewer needs).
        let (cx, cy, r) = (100i32, 100i32, 60usize);
        let (cols, rows) = (6usize, 6usize);
        let straight =
            pick_surface_tile(cx, cy, cx, cy, r, GlobeOrientation::IDENTITY, cols, rows).unwrap();
        let tilted = pick_surface_tile(
            cx,
            cy,
            cx,
            cy,
            r,
            GlobeOrientation {
                rot_lon: 0.0,
                rot_lat: 1.2,
            },
            cols,
            rows,
        )
        .unwrap();
        assert_ne!(
            straight.1, tilted.1,
            "a latitude tilt samples a different row at the centre"
        );
        assert!(
            tilted.1 < straight.1,
            "tilting toward the north pole samples a lower row index (higher latitude): {} vs {}",
            tilted.1,
            straight.1
        );
    }

    #[test]
    fn the_surface_highlight_draws_a_bounded_outline_and_replays() {
        let (w, h) = (200usize, 200usize);
        let (cx, cy, r) = (100i32, 100i32, 60usize);
        let (cols, rows) = (6usize, 6usize);
        let o = GlobeOrientation::IDENTITY;
        let bg = Rgb::new(6, 7, 12);
        let hi = Rgb::new(255, 240, 90);
        let mut buf = vec![bg.pack(); w * h];
        // Highlight the centre tile: an outline appears, and the far corner is untouched (the box sits on the tile).
        draw_surface_highlight(
            &mut buf,
            w,
            h,
            cx,
            cy,
            r,
            o,
            cols,
            rows,
            cols / 2,
            rows / 2,
            hi,
        );
        assert!(
            buf.iter().any(|&p| p == hi.pack()),
            "the highlight outline is drawn"
        );
        assert_eq!(
            buf[5 * w + 5],
            bg.pack(),
            "the highlight stays on the tile, not the far corner"
        );
        // Deterministic pure read.
        let mut replay = vec![bg.pack(); w * h];
        draw_surface_highlight(
            &mut replay,
            w,
            h,
            cx,
            cy,
            r,
            o,
            cols,
            rows,
            cols / 2,
            rows / 2,
            hi,
        );
        assert_eq!(buf, replay, "a pure read replays byte for byte");
    }

    #[test]
    fn the_globe_texture_responds_to_orientation() {
        // Panning must rotate the surface: the same globe drawn at IDENTITY and at a small latitude tilt differs, so a
        // pan brings the far side of the sphere into view. (The IDENTITY path itself reduces to the pre-rotation map,
        // which the unchanged `the_globe_is_a_lit_sphere_sized_from_the_derived_radius` render test still pins.)
        let (w, h) = (160usize, 120usize);
        let bg = Rgb::new(8, 9, 14);
        let (tiles, cols) = demo_globe_tiles();
        let (cx, cy, radius) = (80i32, 60i32, 48usize);
        let mut a = vec![bg.pack(); w * h];
        draw_globe(
            &mut a,
            w,
            h,
            cx,
            cy,
            radius,
            &tiles,
            latlon(&tiles, cols),
            [0.3, -0.2, 0.9],
            Rgb::new(255, 250, 240),
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
            None,
            None,
            None,
        );
        // A tiny rotation moves at least one pixel: the texture responds to the orientation.
        let mut b = vec![bg.pack(); w * h];
        draw_globe(
            &mut b,
            w,
            h,
            cx,
            cy,
            radius,
            &tiles,
            latlon(&tiles, cols),
            [0.3, -0.2, 0.9],
            Rgb::new(255, 250, 240),
            SurfaceStyle::default(),
            GlobeOrientation {
                rot_lon: 0.0,
                rot_lat: 0.8,
            },
            None,
            None,
            None,
        );
        assert_ne!(a, b, "a latitude tilt changes the drawn surface");
    }

    #[test]
    fn panning_sweeps_the_terminator_and_the_tint_colours_a_uniform_sphere() {
        // A UNIFORM crust: every tile the same relief, so texture rotation alone is invisible. The frame must still
        // change when the globe is panned, which proves the lighting rotates with the orientation (the terminator
        // sweeps across the surface, the owner's "the light changes as I pan"). And the derived material tint must
        // reach the surface: a red-tinted sphere reads redder on the lit side than a blue-tinted one.
        let (w, h) = (160usize, 120usize);
        let bg = Rgb::new(8, 9, 14);
        let cols = 8usize;
        let uniform: Vec<DerivedTile> = (0..cols * 8)
            .map(|_| DerivedTile {
                elevation: Fixed::from_int(1),
                relief: TerrainRelief::Lowland,
            })
            .collect();
        let (cx, cy, radius) = (80i32, 60i32, 48usize);
        let star = [1.0f32, 0.0, 0.3];
        let white = Rgb::new(255, 255, 255);
        let mut straight = vec![bg.pack(); w * h];
        draw_globe(
            &mut straight,
            w,
            h,
            cx,
            cy,
            radius,
            &uniform,
            latlon(&uniform, cols),
            star,
            white,
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
            None,
            None,
            None,
        );
        let mut panned = vec![bg.pack(); w * h];
        draw_globe(
            &mut panned,
            w,
            h,
            cx,
            cy,
            radius,
            &uniform,
            latlon(&uniform, cols),
            star,
            white,
            SurfaceStyle::default(),
            GlobeOrientation {
                rot_lon: 1.2,
                rot_lat: 0.0,
            },
            None,
            None,
            None,
        );
        assert_ne!(
            straight, panned,
            "panning a uniform sphere still changes the frame (the terminator sweeps)"
        );

        // The material tint reaches the surface: a red tint reads redder on the lit (+x) side than a blue tint.
        let lit_red = |tint: Rgb| -> u64 {
            let mut buf = vec![bg.pack(); w * h];
            draw_globe(
                &mut buf,
                w,
                h,
                cx,
                cy,
                radius,
                &uniform,
                latlon(&uniform, cols),
                star,
                white,
                SurfaceStyle {
                    tint: Some(tint),
                    grid: None,
                    ..Default::default()
                },
                GlobeOrientation::IDENTITY,
                None,
                None,
                None,
            );
            let mut sum = 0u64;
            for py in (cy - radius as i32).max(0)..=(cy + radius as i32) {
                for px in cx..=(cx + radius as i32) {
                    let dx = px - cx;
                    let dy = py - cy;
                    if dx * dx + dy * dy > (radius as i32).pow(2) {
                        continue;
                    }
                    sum += ((buf[py as usize * w + px as usize] >> 16) & 0xff) as u64;
                }
            }
            sum
        };
        assert!(
            lit_red(Rgb::new(230, 40, 40)) > lit_red(Rgb::new(40, 40, 230)),
            "the derived material tint colours the sphere (a red tint reads redder than a blue tint)"
        );
    }

    #[test]
    fn the_surface_grid_overlays_seams_and_subdivides_with_density() {
        // The display tile grid overlays visible seams on the sphere, and a finer grid subdivides into more seams:
        // each tile opens into a finer array as the caller refines the grid with zoom (the owner's drill-in).
        let (w, h) = (200usize, 200usize);
        let bg = Rgb::new(8, 9, 14);
        let (tiles, cols) = demo_globe_tiles();
        let (cx, cy, radius) = (100i32, 100i32, 80usize);
        let star = [0.4f32, -0.2, 0.9];
        let white = Rgb::new(255, 255, 255);
        let tint = Some(Rgb::new(200, 210, 205));
        let draw = |grid: Option<(usize, usize)>| -> Vec<u32> {
            let mut buf = vec![bg.pack(); w * h];
            draw_globe(
                &mut buf,
                w,
                h,
                cx,
                cy,
                radius,
                &tiles,
                latlon(&tiles, cols),
                star,
                white,
                SurfaceStyle {
                    tint,
                    grid,
                    ..Default::default()
                },
                GlobeOrientation::IDENTITY,
                None,
                None,
                None,
            );
            buf
        };
        let plain = draw(None);
        let coarse = draw(Some((8, 6)));
        let fine = draw(Some((16, 12)));
        let seam_count = |g: &[u32]| g.iter().zip(plain.iter()).filter(|(a, b)| a != b).count();
        let coarse_seams = seam_count(&coarse);
        let fine_seams = seam_count(&fine);
        assert!(coarse_seams > 0, "the grid overlays visible seams");
        assert!(
            fine_seams > coarse_seams,
            "a finer grid subdivides into more seams (each tile opens into a finer array): {fine_seams} vs {coarse_seams}"
        );
        // A None grid leaves the sphere ungridded (the plain planet), and the read replays deterministically.
        assert_eq!(coarse, draw(Some((8, 6))), "a pure read replays");
    }

    #[test]
    fn the_hillshade_lights_sun_facing_slopes_and_needs_the_body_radius() {
        // A field whose elevation rises steadily eastward is a slope that FACES WEST: its surface normal leans west,
        // so it reads brighter lit from the west than from the east. The sun-direction hillshade must resolve that,
        // and only when the physical body radius is supplied (the slope is elevation over horizontal distance, so a
        // zero radius cannot form a slope and the directional signal collapses). This pins the hillshade direction and
        // the km-to-metre unit bridge in one test.
        let (w, h) = (120usize, 120usize);
        let (cx, cy, radius) = (60i32, 60i32, 50usize);
        let cols = 8usize;
        let rows = 4usize;
        // Elevation in km, rising with the column index (an eastward ramp); the relief class is not used here.
        let tiles: Vec<DerivedTile> = (0..cols * rows)
            .map(|i| DerivedTile {
                elevation: Fixed::from_int((i % cols) as i32 * 400),
                relief: TerrainRelief::Lowland,
            })
            .collect();
        let tint = Rgb::new(200, 200, 200);
        let white = Rgb::new(255, 255, 255);
        let radius_m = Fixed::from_int(1_000_000); // a 1000 km body
                                                   // Two near-zenith suns tilted slightly west and slightly east, so the sphere macro term is about equal at the
                                                   // disk centre and only the terrain slope distinguishes them.
        let west_sun = [-0.25f32, 0.0, 0.968]; // horizontal component toward -x (west at the prime meridian)
        let east_sun = [0.25f32, 0.0, 0.968];
        let centre_red = |sun: [f32; 3], radius_m: Fixed| -> u64 {
            let mut buf = vec![Rgb::new(0, 0, 0).pack(); w * h];
            draw_globe(
                &mut buf,
                w,
                h,
                cx,
                cy,
                radius,
                &tiles,
                latlon(&tiles, cols),
                sun,
                white,
                SurfaceStyle {
                    tint: Some(tint),
                    grid: None,
                    relief_shading: true,
                    surface_radius_m: radius_m,
                },
                GlobeOrientation::IDENTITY,
                None,
                None,
                None,
            );
            let mut s = 0u64;
            for py in (cy - 6)..=(cy + 6) {
                for px in (cx - 6)..=(cx + 6) {
                    s += ((buf[py as usize * w + px as usize] >> 16) & 0xff) as u64;
                }
            }
            s
        };
        let west = centre_red(west_sun, radius_m);
        let east = centre_red(east_sun, radius_m);
        assert!(
            west > east,
            "an eastward-rising ramp faces west, so it is brighter lit from the west: west {west} east {east}"
        );
        // With a zero body radius the hillshade cannot form a slope, so the two suns light the centre equally: the
        // directional relief is gone. This is the failure the km-to-metre unit bug caused (a 1000x-too-flat slope).
        let none_west = centre_red(west_sun, Fixed::ZERO);
        let none_east = centre_red(east_sun, Fixed::ZERO);
        let relief_diff = (west as i64 - east as i64).abs();
        let none_diff = (none_west as i64 - none_east as i64).abs();
        assert!(
            relief_diff > none_diff * 8 + 8,
            "the hillshade (driven by the body radius) creates the directional relief: with-radius diff {relief_diff} vs no-radius diff {none_diff}"
        );
    }

    // ---- THE ANALYTIC SAMPLE GRADIENT AND ITS NUMERICAL TWIN ----

    /// A varied DERIVED province crust-thickness field (kilometres), 4 by 2, for the gradient twins.
    fn twin_thickness() -> Vec<Fixed> {
        vec![
            Fixed::from_int(2),
            Fixed::from_int(60),
            Fixed::from_int(5),
            Fixed::from_int(40),
            Fixed::from_int(80),
            Fixed::from_int(3),
            Fixed::from_int(50),
            Fixed::from_int(8),
        ]
    }

    /// A Mars-class body radius for the twins: the derived default world's own scale (2990 km).
    const TWIN_RADIUS_KM: f32 = 2990.0;

    /// One crater row for the twins, big enough that its bowl and blanket span a resolvable angle.
    fn twin_crater_rows() -> Vec<CraterRow> {
        vec![CraterRow {
            u: Fixed::from_ratio(1, 5),
            v: Fixed::from_ratio(2, 5),
            diameter_m: Fixed::from_int(400_000),
            depth_m: Fixed::from_int(20_000),
            age_myr: Fixed::ZERO,
        }]
    }

    /// A great-circle step from `b` by angle `h` along the unit tangent `t`. The step stays ON the sphere (unit
    /// norm), so the finite difference is taken over a true ARC length `R*h`, never a chord: the twin measures the
    /// same slope the analytic gradient reports, rather than a secant through the ball.
    fn arc_step(b: [f32; 3], t: [f32; 3], h: f32) -> [Fixed; 3] {
        let (s, c) = h.sin_cos();
        let p = normalize3([
            c * b[0] + s * t[0],
            c * b[1] + s * t[1],
            c * b[2] + s * t[2],
        ]);
        [
            Fixed::from_ratio((p[0] as f64 * 1e9) as i64, 1_000_000_000),
            Fixed::from_ratio((p[1] as f64 * 1e9) as i64, 1_000_000_000),
            Fixed::from_ratio((p[2] as f64 * 1e9) as i64, 1_000_000_000),
        ]
    }

    /// An orthonormal tangent pair at `b`, regular at every direction (the same never-degenerate construction the
    /// cube-sphere hillshade uses).
    fn twin_tangents(b: [f32; 3]) -> ([f32; 3], [f32; 3]) {
        let (ax, ay, az) = (b[0].abs(), b[1].abs(), b[2].abs());
        let axis = if ax <= ay && ax <= az {
            [1.0, 0.0, 0.0]
        } else if ay <= az {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let t1 = normalize3(cross3(axis, b));
        let t2 = cross3(b, t1);
        (t1, t2)
    }

    /// The CENTRAL-DIFFERENCE slope of the Sample along tangent `t` at `b`, over an angular step `h`: the
    /// numerical twin of `gradient(b) . t`. Rise in kilometres over the arc `2 R h`, the same dimensionless slope
    /// the analytic gradient returns.
    fn twin_fd_slope(field: &SurfaceField, b: [f32; 3], t: [f32; 3], h: f32) -> Option<f64> {
        let hp = field.height_km(arc_step(b, t, h))?.to_f64_lossy();
        let hm = field.height_km(arc_step(b, t, -h))?.to_f64_lossy();
        Some((hp - hm) / (2.0 * TWIN_RADIUS_KM as f64 * h as f64))
    }

    /// A point at `x` rim-radii from the crater centre, along the crater's own tangent: the probe point for the
    /// bowl (`x < 1`) and blanket (`x > 1`) twins.
    fn twin_point_at_rim_radii(stamps: &[CraterStamp], x: f32) -> [f32; 3] {
        let c = [
            stamps[0].center[0].to_f64_lossy() as f32,
            stamps[0].center[1].to_f64_lossy() as f32,
            stamps[0].center[2].to_f64_lossy() as f32,
        ];
        let (t1, _) = twin_tangents(c);
        let ang = x * stamps[0].angular_radius.to_f64_lossy() as f32;
        let (s, co) = ang.sin_cos();
        normalize3([
            co * c[0] + s * t1[0],
            co * c[1] + s * t1[1],
            co * c[2] + s * t1[2],
        ])
    }

    #[test]
    fn the_analytic_crater_gradient_converges_second_order_against_its_numerical_twin() {
        // THE NUMERICAL TWIN (the standing rule): the ANALYTIC gradient of the Sample is checked against a
        // FINITE-DIFFERENCE gradient of the SAME Sample over a STEP SWEEP, and must show the central difference's
        // expected SECOND-ORDER convergence as h shrinks. An analytic-recovers-analytic check would be circular.
        //
        // The convergence ORDER is measured on the crater EJECTA BLANKET, and that choice is forced rather than
        // convenient: it is the only layer of the built Sample with a genuine non-vanishing third derivative. The
        // excavation bowl is EXACTLY quadratic in the rim-radius x, so a central difference of it carries no
        // truncation error to converge (its twin is checked at the representation floor below), and the province
        // layer's interpolation weight is quantized, which floors its twin before the truncation term is reached
        // (also below). The blanket is where a rate can be measured at all.
        let rows = twin_crater_rows();
        let stamps = crater_stamps(&rows, Fixed::from_int(2_990_000));
        let flat: Vec<Fixed> = vec![Fixed::ZERO; 8];
        // A FLAT crust isolates the crater layer: the province layer contributes an exactly zero gradient.
        let field = SurfaceField {
            thickness_km: &flat,
            pcols: 4,
            prows: 2,
            crust_density: Fixed::from_ratio(29, 10),
            mantle_density: Fixed::from_ratio(33, 10),
            stamps: &stamps,
            radius_km: TWIN_RADIUS_KM,
        };
        let b = twin_point_at_rim_radii(&stamps, 2.0); // out in the blanket, clear of the rim kink
        let (t1, t2) = twin_tangents(b);
        let g = field.gradient(b);
        for (which, t) in [("t1", t1), ("t2", t2)] {
            let analytic = (g[0] * t[0] + g[1] * t[1] + g[2] * t[2]) as f64;
            // The truncation-dominated window: coarse enough that the fixed-point floor is far below the
            // truncation term, fine enough to stay clear of the rim. Each halving must quarter the error.
            let mut orders = Vec::new();
            let mut h = 2.5e-2f32;
            let mut prev = (twin_fd_slope(&field, b, t, h).expect("fd") - analytic).abs();
            for _ in 0..4 {
                h *= 0.5;
                let err = (twin_fd_slope(&field, b, t, h).expect("fd") - analytic).abs();
                // order = log2(err(h) / err(h/2)); a second-order scheme gives 2.
                orders.push((prev / err).log2());
                prev = err;
            }
            let mean: f64 = orders.iter().sum::<f64>() / orders.len() as f64;
            assert!(
                (1.8..=2.2).contains(&mean),
                "the {which} twin must converge at the central difference's second order, got {mean} from {orders:?}"
            );
        }
    }

    #[test]
    fn the_analytic_gradient_matches_its_numerical_twin_across_every_sample_layer() {
        // The twin's other half: the analytic gradient must AGREE with the finite difference, layer by layer and
        // in superposition, not merely converge at the right rate. Each probe sits where the surface is SMOOTH
        // (the composed surface is only piecewise smooth: no gradient exists at a crater rim or across a province
        // cell edge, so a difference straddling either reads a chord and is not a fair twin).
        let rows = twin_crater_rows();
        let stamps = crater_stamps(&rows, Fixed::from_int(2_990_000));
        let no_stamps: Vec<CraterStamp> = Vec::new();
        let thickness = twin_thickness();
        let flat: Vec<Fixed> = vec![Fixed::ZERO; 8];
        let crater_only = SurfaceField {
            thickness_km: &flat,
            pcols: 4,
            prows: 2,
            crust_density: Fixed::from_ratio(29, 10),
            mantle_density: Fixed::from_ratio(33, 10),
            stamps: &stamps,
            radius_km: TWIN_RADIUS_KM,
        };
        let province_only = SurfaceField {
            thickness_km: &thickness,
            pcols: 4,
            prows: 2,
            crust_density: Fixed::from_ratio(29, 10),
            mantle_density: Fixed::from_ratio(33, 10),
            stamps: &no_stamps,
            radius_km: TWIN_RADIUS_KM,
        };
        let composed = SurfaceField {
            thickness_km: &thickness,
            pcols: 4,
            prows: 2,
            crust_density: Fixed::from_ratio(29, 10),
            mantle_density: Fixed::from_ratio(33, 10),
            stamps: &stamps,
            radius_km: TWIN_RADIUS_KM,
        };

        // Each probe carries the step at which its OWN twin is cleanest, and the tolerance measured there. The
        // steps differ by nearly two decades because the layers' resolvable windows really do differ, and each
        // window is a property of the layer rather than a tolerance chosen to pass:
        //   - the BOWL is exactly quadratic in the rim-radius x, so the central difference carries no truncation
        //     error and the residual is the fixed-point representation floor (~4e-7). The step must stay INSIDE
        //     the rim, which at x = 0.5 rim-radii is 0.033 rad away, so h = 5e-3 is safely clear of the kink.
        //   - the BLANKET has a genuine non-vanishing third derivative, so its error is the h^2 truncation term
        //     and it wants a FINE step.
        //   - the PROVINCE interior wants a COARSE step, the opposite of the usual story: the bilinear
        //     interpolant is near-linear along the probe path (so truncation is tiny even at h = 5e-2), while its
        //     interpolation weight is quantized to 1/PROVINCE_LERP_STEPS of a cell, so a FINE step reads the
        //     staircase rather than the slope. That floor is a measured property of the pre-existing sample path,
        //     named rather than hidden. The probe sits MID-CELL (tx = ty = 0.5): a point near an interpolation
        //     cell boundary would straddle the interpolant's gradient step, where no gradient exists.
        //   - the COMPOSED Sample must satisfy BOTH layers at once, so it sits in their overlap (h = 6.25e-3) and
        //     holds a tolerance that is the sum of the two floors there, wider than either alone. That is the
        //     honest cost of the two windows, not a slackened gate.
        let bowl = twin_point_at_rim_radii(&stamps, 0.5);
        let blanket = twin_point_at_rim_radii(&stamps, 2.0);
        let interior = uv_to_body(0.25, 0.5);
        for (name, field, b, h, tol) in [
            ("crater bowl", &crater_only, bowl, 5.0e-3f32, 1e-5f64),
            ("crater blanket", &crater_only, blanket, 1.0e-3f32, 1e-5f64),
            (
                "province interior",
                &province_only,
                interior,
                5.0e-2f32,
                1e-6f64,
            ),
            (
                "the composed Sample",
                &composed,
                blanket,
                6.25e-3f32,
                5e-5f64,
            ),
        ] {
            let (t1, t2) = twin_tangents(b);
            let g = field.gradient(b);
            for t in [t1, t2] {
                let analytic = (g[0] * t[0] + g[1] * t[1] + g[2] * t[2]) as f64;
                let fd = twin_fd_slope(field, b, t, h).expect("the twin resolves");
                assert!(
                    (fd - analytic).abs() < tol,
                    "{name}: analytic slope {analytic} vs numerical twin {fd} at h = {h}"
                );
            }
        }
    }

    #[test]
    fn a_flat_world_has_no_slope_and_a_crater_gradient_points_downhill() {
        // The gradient's own sanity: a flat crust with no craters has exactly zero gradient everywhere (the
        // analytic derivative invents no relief), and inside a crater bowl the surface rises AWAY from the centre
        // (the bowl is a depression), so the gradient points away from it.
        let no_stamps: Vec<CraterStamp> = Vec::new();
        let flat: Vec<Fixed> = vec![Fixed::from_int(30); 8]; // a UNIFORM crust: thick, but no lateral contrast
        let flat_field = SurfaceField {
            thickness_km: &flat,
            pcols: 4,
            prows: 2,
            crust_density: Fixed::from_ratio(29, 10),
            mantle_density: Fixed::from_ratio(33, 10),
            stamps: &no_stamps,
            radius_km: TWIN_RADIUS_KM,
        };
        for (u, v) in [(0.1f32, 0.2f32), (0.5, 0.5), (0.9, 0.8)] {
            let g = flat_field.gradient(uv_to_body(u, v));
            let m = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
            assert_eq!(m, 0.0, "a laterally uniform crust has no slope, got {g:?}");
        }
        // Inside the bowl the gradient points AWAY from the crater centre (uphill toward the rim).
        let rows = twin_crater_rows();
        let stamps = crater_stamps(&rows, Fixed::from_int(2_990_000));
        let zero: Vec<Fixed> = vec![Fixed::ZERO; 8];
        let crater_field = SurfaceField {
            thickness_km: &zero,
            pcols: 4,
            prows: 2,
            crust_density: Fixed::from_ratio(29, 10),
            mantle_density: Fixed::from_ratio(33, 10),
            stamps: &stamps,
            radius_km: TWIN_RADIUS_KM,
        };
        let b = twin_point_at_rim_radii(&stamps, 0.5);
        let c = [
            stamps[0].center[0].to_f64_lossy() as f32,
            stamps[0].center[1].to_f64_lossy() as f32,
            stamps[0].center[2].to_f64_lossy() as f32,
        ];
        let g = crater_field.gradient(b);
        let dot = b[0] * c[0] + b[1] * c[1] + b[2] * c[2];
        let toward = normalize3([c[0] - dot * b[0], c[1] - dot * b[1], c[2] - dot * b[2]]);
        let along_toward = g[0] * toward[0] + g[1] * toward[1] + g[2] * toward[2];
        assert!(
            along_toward < 0.0,
            "inside the bowl the surface rises away from the centre, so the gradient points away: got {along_toward}"
        );
    }
}

// ============================================================================================================
// NON-CANON GPU globe PARITY tests (Principle 10). They render the same scene with the CPU `draw_globe` reference
// and with the GPU kernel (`civsim_gpu::globe`, run on the CubeCL CPU backend so no device is needed) and assert
// the frames are VISUALLY equal: the max per-channel difference stays within a small display tolerance. Non-canon
// means visual-equal, not byte-equal (two observers' framebuffers need not agree to the bit), so the tolerance
// covers f32 order-of-operation drift and the ONE approximation of the GPU path (per-cell rather than per-pixel
// evaluation of the hillshade normal and the flash bloom). Gated on the `gpu` feature.
// ============================================================================================================
#[cfg(all(test, feature = "gpu"))]
mod gpu_parity_tests {
    use super::*;

    /// Render the globe disk with the GPU kernel on the CubeCL CPU backend, into a `bg`-filled buffer, using the
    /// same per-cell cache builders and per-frame scalars `draw_globe_scene_gpu` uses.
    #[allow(clippy::too_many_arguments)]
    fn gpu_disk(
        w: usize,
        h: usize,
        cx: i32,
        cy: i32,
        radius_px: usize,
        tiles: &[DerivedTile],
        param: SurfaceParam,
        t_eff_k: Fixed,
        star_dir_body: [f32; 3],
        style: SurfaceStyle,
        orient: GlobeOrientation,
        lava: Option<&[LavaGlow]>,
        field: Option<&SurfaceField>,
        flash: Option<&[ImpactFlash]>,
        bg: u32,
    ) -> Vec<u32> {
        use civsim_gpu::globe::{GlobeCells, GlobeParam};
        let gparam = match param {
            SurfaceParam::LatLon { cols, rows } => GlobeParam::LatLon { cols, rows },
            SurfaceParam::CubeSphere { face_res } => GlobeParam::CubeSphere { face_res },
        };
        let base_rgb = globe_cell_base_rgb(tiles, style);
        let normals = globe_cell_normals(tiles, param, style.surface_radius_m, field);
        let lava_add = lava
            .map(|l| globe_cell_lava_add(l, param, tiles.len()))
            .unwrap_or_default();
        let flashes = flash.map(globe_gpu_flashes).unwrap_or_default();
        let light_tint = blackbody_rgb(t_eff_k);
        let frame = globe_gpu_frame(
            w,
            h,
            cx,
            cy,
            radius_px,
            param,
            star_dir_body,
            light_tint,
            style,
            orient,
        );
        let mut r = civsim_gpu::globe::cpu_renderer();
        r.upload_cells(
            gparam,
            GlobeCells {
                base_rgb: &base_rgb,
                normal: &normals,
                lava_add: &lava_add,
            },
            0,
        );
        let mut buf = vec![bg; w * h];
        r.render(&mut buf, &frame, &flashes);
        buf
    }

    /// The max per-channel absolute difference between two framebuffers, and the fraction of pixels that differ by
    /// more than one level (a picture of how localized the difference is).
    fn frame_diff(a: &[u32], b: &[u32]) -> (u8, f64) {
        let mut maxd = 0u8;
        let mut over = 0usize;
        for (&x, &y) in a.iter().zip(b.iter()) {
            let mut worst = 0u8;
            for sh in [16u32, 8, 0] {
                let cx = ((x >> sh) & 255) as i32;
                let cy = ((y >> sh) & 255) as i32;
                worst = worst.max((cx - cy).unsigned_abs() as u8);
            }
            maxd = maxd.max(worst);
            if worst > 1 {
                over += 1;
            }
        }
        (maxd, over as f64 / a.len() as f64)
    }

    fn tile(elev_km: f32, relief: TerrainRelief) -> DerivedTile {
        DerivedTile {
            elevation: Fixed::from_ratio((elev_km * 1000.0) as i64, 1000),
            relief,
        }
    }

    /// SCENE A, the CORE PIPELINE with no hillshade: rotate + cell-index + per-cell base albedo + bare-sphere
    /// Lambert + tile-grid seam. Every term is the SAME per-pixel math on both paths (no per-cell approximation),
    /// so the GPU frame must equal `draw_globe` to within f32 rounding.
    #[test]
    fn gpu_matches_draw_globe_core_pipeline() {
        let face_res = 64usize;
        let param = SurfaceParam::CubeSphere { face_res };
        let n = 6 * face_res * face_res;
        // A varied relief field (drives the per-cell base colour under relief-off shading and the swatch).
        let tiles: Vec<DerivedTile> = (0..n)
            .map(|i| {
                let d = surface_cell_center_dir(param, i);
                let e = 3.0 * (d[0] * 4.0).sin() + 2.0 * (d[1] * 3.0).cos();
                let relief = if e < -1.0 {
                    TerrainRelief::Submarine
                } else if e < 1.5 {
                    TerrainRelief::Lowland
                } else {
                    TerrainRelief::Upland
                };
                tile(e, relief)
            })
            .collect();
        let style = SurfaceStyle {
            tint: Some(Rgb::new(128, 116, 104)),
            grid: Some((16, 8)),
            relief_shading: false,
            surface_radius_m: Fixed::from_int(3_390_000),
        };
        let orient = GlobeOrientation {
            rot_lon: 0.7,
            rot_lat: 0.3,
        };
        let star = normalize3([0.5, 0.25, 0.8]);
        let t_eff = Fixed::from_int(5772);
        let (w, h) = (420usize, 340usize);
        let (cx, cy, rp) = (210i32, 170i32, 150usize);
        const BG: u32 = 0x00101018;

        let mut cpu = vec![BG; w * h];
        draw_globe(
            &mut cpu,
            w,
            h,
            cx,
            cy,
            rp,
            &tiles,
            param,
            star,
            blackbody_rgb(t_eff),
            style,
            orient,
            None,
            None,
            None,
        );
        let gpu = gpu_disk(
            w, h, cx, cy, rp, &tiles, param, t_eff, star, style, orient, None, None, None, BG,
        );
        let (maxd, frac) = frame_diff(&cpu, &gpu);
        eprintln!("scene A (core pipeline, CPU backend): max per-channel diff = {maxd}, frac>1 = {frac:.5}");
        assert!(
            maxd <= 2,
            "core pipeline must match draw_globe to f32 rounding: max diff {maxd}"
        );

        // The SHIPPING path: the SAME kernel on the actual CUDA device (when one is present, i.e. CIVSIM_GPU +
        // the CUDA env are set) must also match draw_globe. This proves the 5090 render, not only the portable
        // CPU-backend codegen. Skipped (a pass) when no device is present, so the default gate stays device-free.
        if let Some(mut r) = civsim_gpu::globe::try_cuda_renderer() {
            use civsim_gpu::globe::{GlobeCells, GlobeParam};
            let gparam = match param {
                SurfaceParam::LatLon { cols, rows } => GlobeParam::LatLon { cols, rows },
                SurfaceParam::CubeSphere { face_res } => GlobeParam::CubeSphere { face_res },
            };
            let base_rgb = globe_cell_base_rgb(&tiles, style);
            let normals = globe_cell_normals(&tiles, param, style.surface_radius_m, None);
            let frame = globe_gpu_frame(
                w,
                h,
                cx,
                cy,
                rp,
                param,
                star,
                blackbody_rgb(t_eff),
                style,
                orient,
            );
            r.upload_cells(
                gparam,
                GlobeCells {
                    base_rgb: &base_rgb,
                    normal: &normals,
                    lava_add: &[],
                },
                0,
            );
            let mut cbuf = vec![BG; w * h];
            r.render(&mut cbuf, &frame, &[]);
            let (cd, cf) = frame_diff(&cpu, &cbuf);
            eprintln!(
                "scene A (core pipeline, CUDA 5090): max per-channel diff = {cd}, frac>1 = {cf:.6}"
            );
            // Visual-equality is the claim (not byte-equality): essentially every pixel is within one level of
            // draw_globe. The isolated larger differences are TILE-BOUNDARY pixels where the 5090's hardware
            // atan/asin round differently from libm and flip the discrete cell lookup, so a boundary pixel takes
            // the neighbouring cell's colour (here up to the adjacent-relief contrast). It is a handful of pixels
            // (frac below), invisible in the picture, and in the interactive path (a uniform material tint) a
            // boundary flip changes no colour at all. A broken kernel would blow this fraction up.
            assert!(
                cf < 0.001,
                "the CUDA render must be visually equal to draw_globe (a tiny boundary sliver): frac>1 = {cf}, max = {cd}"
            );
        }
    }

    /// SCENE B, the FULL EFFECT STACK: the analytic hillshade normal, the self-emitted lava glow, and the fresh
    /// impact flash, plus the material tint. Here the GPU evaluates the hillshade normal and the flash bloom PER
    /// CELL (the once-per-epoch reduction) where `draw_globe` evaluates them per pixel, so the frames are visually
    /// equal within a small tolerance rather than exact. This asserts that tolerance and prints the measured value.
    #[test]
    fn gpu_matches_draw_globe_full_effects_within_tolerance() {
        let face_res = 96usize;
        let param = SurfaceParam::CubeSphere { face_res };
        let n = 6 * face_res * face_res;
        // Tiles are trivial here: with tint + relief shading the base is the material colour, so relief is unused.
        let tiles: Vec<DerivedTile> = vec![tile(0.0, TerrainRelief::Lowland); n];

        // A province crust field with lateral variation, so the analytic hillshade has real slope structure.
        let (pcols, prows) = (12usize, 6usize);
        let thickness: Vec<Fixed> = (0..pcols * prows)
            .map(|k| {
                let c = (k % pcols) as f32;
                let r = (k / pcols) as f32;
                let t = 30.0 + 8.0 * (c * 0.9).sin() + 6.0 * (r * 1.3).cos();
                Fixed::from_ratio((t * 1000.0) as i64, 1000)
            })
            .collect();
        let radius_m = Fixed::from_int(3_390_000);
        // A few craters for the analytic crater layer of the gradient.
        let craters = vec![
            CraterRow {
                u: Fixed::from_ratio(1, 4),
                v: Fixed::from_ratio(2, 5),
                diameter_m: Fixed::from_int(300_000),
                depth_m: Fixed::from_int(20_000),
                age_myr: Fixed::from_int(100),
            },
            CraterRow {
                u: Fixed::from_ratio(3, 5),
                v: Fixed::from_ratio(1, 2),
                diameter_m: Fixed::from_int(500_000),
                depth_m: Fixed::from_int(30_000),
                age_myr: Fixed::from_int(100),
            },
        ];
        let stamps = crater_stamps(&craters, radius_m);
        let field = SurfaceField {
            thickness_km: &thickness,
            pcols,
            prows,
            crust_density: Fixed::from_int(2800),
            mantle_density: Fixed::from_int(3300),
            stamps: &stamps,
            radius_km: radius_m.to_f64_lossy() as f32 / 1000.0,
        };

        // A lava patch on the front hemisphere (the same cell layout the tiles use).
        let lava: Vec<LavaGlow> = (0..n)
            .map(|i| {
                let d = surface_cell_center_dir(param, i);
                if d[2] > 0.3 {
                    LavaGlow {
                        emission: Rgb::new(255, 90, 20),
                        intensity: 0.5,
                    }
                } else {
                    LavaGlow::default()
                }
            })
            .collect();

        // Fresh impact flashes at their formation tick (peak intensity), from the crater rows.
        let flash = active_flash_stamps(
            &craters,
            radius_m,
            Fixed::from_int(100),
            Fixed::from_int(50),
        );
        assert!(!flash.is_empty(), "test setup: flashes should be active");

        let style = SurfaceStyle {
            tint: Some(Rgb::new(120, 110, 100)),
            grid: None,
            relief_shading: true,
            surface_radius_m: radius_m,
        };
        let orient = GlobeOrientation {
            rot_lon: 0.4,
            rot_lat: -0.2,
        };
        let star = normalize3([0.3, 0.4, 0.85]);
        let t_eff = Fixed::from_int(5772);
        let (w, h) = (420usize, 340usize);
        let (cx, cy, rp) = (210i32, 170i32, 150usize);
        const BG: u32 = 0x00101018;

        let mut cpu = vec![BG; w * h];
        draw_globe(
            &mut cpu,
            w,
            h,
            cx,
            cy,
            rp,
            &tiles,
            param,
            star,
            blackbody_rgb(t_eff),
            style,
            orient,
            Some(&lava),
            Some(&field),
            Some(&flash),
        );
        let gpu = gpu_disk(
            w,
            h,
            cx,
            cy,
            rp,
            &tiles,
            param,
            t_eff,
            star,
            style,
            orient,
            Some(&lava),
            Some(&field),
            Some(&flash),
            BG,
        );
        let (maxd, frac) = frame_diff(&cpu, &gpu);
        eprintln!("scene B (full stack): max per-channel diff = {maxd}, frac>1 = {frac:.5}");

        // Isolate the two effects that are NOT bit-shared per pixel, so the honest bound is asserted directly rather
        // than left to whether one effect happens to saturate over another (as the lava does over the crater rims in
        // the full stack above, which is why its max reads lower than the hillshade residual alone).
        //
        // FLASH is summed PER PIXEL in the kernel: it must reproduce crater_flash_emission to f32 rounding.
        let flat = SurfaceStyle {
            relief_shading: false,
            ..style
        };
        let mut flash_cpu = vec![BG; w * h];
        draw_globe(
            &mut flash_cpu,
            w,
            h,
            cx,
            cy,
            rp,
            &tiles,
            param,
            star,
            blackbody_rgb(t_eff),
            flat,
            orient,
            None,
            None,
            Some(&flash),
        );
        let flash_gpu = gpu_disk(
            w,
            h,
            cx,
            cy,
            rp,
            &tiles,
            param,
            t_eff,
            star,
            flat,
            orient,
            None,
            None,
            Some(&flash),
            BG,
        );
        let (fd, ff) = frame_diff(&flash_cpu, &flash_gpu);
        eprintln!("  flash-only (per-pixel): max = {fd}, frac>1 = {ff:.5}");
        assert!(
            fd <= 2,
            "the per-pixel flash must reproduce crater_flash_emission to f32 rounding: max diff {fd}"
        );

        // HILLSHADE is the one per-CELL approximation: the analytic normal is held at the cell centre, so the frame
        // differs from the per-pixel draw_globe only in a small sliver of crater-rim cells (where the analytic
        // gradient is discontinuous). This is the honest tolerance of the GPU path, measured here.
        let mut hs_cpu = vec![BG; w * h];
        draw_globe(
            &mut hs_cpu,
            w,
            h,
            cx,
            cy,
            rp,
            &tiles,
            param,
            star,
            blackbody_rgb(t_eff),
            style,
            orient,
            None,
            Some(&field),
            None,
        );
        let hs_gpu = gpu_disk(
            w,
            h,
            cx,
            cy,
            rp,
            &tiles,
            param,
            t_eff,
            star,
            style,
            orient,
            None,
            Some(&field),
            None,
            BG,
        );
        let (hd, hf) = frame_diff(&hs_cpu, &hs_gpu);
        eprintln!("  hillshade-only (per-cell normal): max = {hd}, frac>1 = {hf:.5}");
        assert!(
            hd <= 24,
            "the per-cell hillshade normal must stay a small display tolerance from draw_globe: max diff {hd}"
        );
        assert!(
            hf < 0.01,
            "the hillshade difference must be a small sliver (crater rims): frac>1 = {hf}"
        );
        // The full stack is bounded by the hillshade residual (the flash is pixel-accurate, the lava exact).
        assert!(maxd <= 24, "full-stack frame max diff {maxd}");
    }
}

// ============================================================================================================
// NON-CANON GPU globe BENCHMARK (opt-in, `--ignored`). It times the heavy interactive path (a ~1M-cell derived
// globe with an analytic hillshade over many craters, at surface-zoom radius) rendered by the serial CPU
// `draw_globe` versus the GPU kernel on the resident per-cell cache, so the win is quantified. Run with:
//   CIVSIM_GPU=1 CUDA_PATH=$HOME/.local/cuda LD_LIBRARY_PATH=$HOME/.local/cuda/lib:/usr/lib/wsl/lib \
//     cargo test -p civsim-viewer --features gpu --release gpu_globe_benchmark -- --ignored --nocapture
// Gated on the `gpu` feature; prints a note and skips the GPU timing when no CUDA device is present.
// ============================================================================================================
#[cfg(all(test, feature = "gpu"))]
mod gpu_bench {
    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore = "opt-in performance benchmark; run with --ignored --release"]
    fn gpu_globe_benchmark() {
        // A ~1M-cell derived globe (the cube-sphere render cache resolution) with an analytic Sample field over a
        // heavily-cratered surface (the analytic hillshade the interactive derived globe uses), at surface-zoom
        // framing (the globe fills the frame). This is the heavy path the goal targets.
        let face_res = 400usize;
        let param = SurfaceParam::CubeSphere { face_res };
        let n = 6 * face_res * face_res;
        let tiles: Vec<DerivedTile> = vec![
            DerivedTile {
                elevation: Fixed::ZERO,
                relief: TerrainRelief::Lowland,
            };
            n
        ];
        let (pcols, prows) = (24usize, 12usize);
        let thickness: Vec<Fixed> = (0..pcols * prows)
            .map(|k| {
                let c = (k % pcols) as f32;
                let r = (k / pcols) as f32;
                let t = 30.0 + 10.0 * (c * 0.7).sin() + 7.0 * (r * 1.1).cos();
                Fixed::from_ratio((t * 1000.0) as i64, 1000)
            })
            .collect();
        let radius_m = Fixed::from_int(3_390_000);
        // A bombarded surface: 200 craters, so the per-pixel analytic gradient loops over a real crater list.
        let ncr = 200usize;
        let craters: Vec<CraterRow> = (0..ncr)
            .map(|i| {
                let f = i as f32;
                CraterRow {
                    u: Fixed::from_ratio(((f * 0.6180339).fract() * 1000.0) as i64, 1000),
                    v: Fixed::from_ratio(((f * 0.7548776).fract() * 1000.0) as i64, 1000),
                    diameter_m: Fixed::from_int(80_000 + (i % 40) as i32 * 8_000),
                    depth_m: Fixed::from_int(6_000 + (i % 20) as i32 * 700),
                    age_myr: Fixed::from_int(100),
                }
            })
            .collect();
        let stamps = crater_stamps(&craters, radius_m);
        let field = SurfaceField {
            thickness_km: &thickness,
            pcols,
            prows,
            crust_density: Fixed::from_int(2800),
            mantle_density: Fixed::from_int(3300),
            stamps: &stamps,
            radius_km: radius_m.to_f64_lossy() as f32 / 1000.0,
        };
        let lava: Vec<LavaGlow> = (0..n)
            .map(|i| {
                let d = surface_cell_center_dir(param, i);
                if d[2] > 0.4 {
                    LavaGlow {
                        emission: Rgb::new(255, 90, 20),
                        intensity: 0.4,
                    }
                } else {
                    LavaGlow::default()
                }
            })
            .collect();
        let flash = active_flash_stamps(
            &craters,
            radius_m,
            Fixed::from_int(100),
            Fixed::from_int(50),
        );
        let style = SurfaceStyle {
            tint: Some(Rgb::new(120, 110, 100)),
            grid: Some((64, 32)),
            relief_shading: true,
            surface_radius_m: radius_m,
        };
        let t_eff = Fixed::from_int(5772);
        let star = normalize3([0.3, 0.4, 0.85]);
        // Surface-zoom framing: the globe fills the frame (radius ~ half the min dimension).
        let (w, h) = (1100usize, 850usize);
        let (cx, cy, rp) = (550i32, 425i32, 430usize);
        const BG: u32 = 0x00101018;
        let k = 4usize;

        eprintln!(
            "\n=== GPU globe benchmark: {n} cells, {ncr} craters, {w}x{h} frame, radius {rp}px ===",
        );

        // CPU: the serial per-pixel draw_globe (the current interactive render), K frames.
        let mut sink = 0u64;
        let t = Instant::now();
        for i in 0..k {
            let mut buf = vec![BG; w * h];
            let orient = GlobeOrientation {
                rot_lon: 0.4 + i as f32 * 0.05,
                rot_lat: -0.2,
            };
            draw_globe(
                &mut buf,
                w,
                h,
                cx,
                cy,
                rp,
                &tiles,
                param,
                star,
                blackbody_rgb(t_eff),
                style,
                orient,
                Some(&lava),
                Some(&field),
                Some(&flash),
            );
            sink ^= buf[buf.len() / 2] as u64;
        }
        let cpu_ms = t.elapsed().as_secs_f64() * 1000.0 / k as f64;
        eprintln!("CPU  draw_globe (serial per-pixel):   {cpu_ms:8.2} ms/frame");

        // GPU: the per-cell cache build (rayon, once per epoch) and the resident-cache render (per frame).
        match civsim_gpu::globe::try_cuda_renderer() {
            None => {
                eprintln!(
                    "GPU: no CUDA device (set CIVSIM_GPU + CUDA_PATH/LD_LIBRARY_PATH); skipping GPU timing"
                );
            }
            Some(mut gpu) => {
                use civsim_gpu::globe::{GlobeCells, GlobeParam};
                let gparam = match param {
                    SurfaceParam::LatLon { cols, rows } => GlobeParam::LatLon { cols, rows },
                    SurfaceParam::CubeSphere { face_res } => GlobeParam::CubeSphere { face_res },
                };
                // The once-per-epoch per-cell cache build (the analytic normals dominate; rayon-parallel).
                let tb = Instant::now();
                let base_rgb = globe_cell_base_rgb(&tiles, style);
                let normals =
                    globe_cell_normals(&tiles, param, style.surface_radius_m, Some(&field));
                let lava_add = globe_cell_lava_add(&lava, param, tiles.len());
                let build_ms = tb.elapsed().as_secs_f64() * 1000.0;
                gpu.upload_cells(
                    gparam,
                    GlobeCells {
                        base_rgb: &base_rgb,
                        normal: &normals,
                        lava_add: &lava_add,
                    },
                    0,
                );
                let flashes = globe_gpu_flashes(&flash);
                // Warm up (first launch JIT-compiles the kernel).
                {
                    let mut buf = vec![BG; w * h];
                    let frame = globe_gpu_frame(
                        w,
                        h,
                        cx,
                        cy,
                        rp,
                        param,
                        star,
                        blackbody_rgb(t_eff),
                        style,
                        GlobeOrientation {
                            rot_lon: 0.4,
                            rot_lat: -0.2,
                        },
                    );
                    gpu.render(&mut buf, &frame, &flashes);
                    sink ^= buf[buf.len() / 2] as u64;
                }
                let t = Instant::now();
                for i in 0..k {
                    let mut buf = vec![BG; w * h];
                    let frame = globe_gpu_frame(
                        w,
                        h,
                        cx,
                        cy,
                        rp,
                        param,
                        star,
                        blackbody_rgb(t_eff),
                        style,
                        GlobeOrientation {
                            rot_lon: 0.4 + i as f32 * 0.05,
                            rot_lat: -0.2,
                        },
                    );
                    gpu.render(&mut buf, &frame, &flashes);
                    sink ^= buf[buf.len() / 2] as u64;
                }
                let gpu_ms = t.elapsed().as_secs_f64() * 1000.0 / k as f64;
                eprintln!("GPU  render (resident cache):         {gpu_ms:8.2} ms/frame");
                eprintln!("GPU  per-cell cache build (once):     {build_ms:8.2} ms  (rayon; only on an epoch change)");
                eprintln!(
                    "\nSURFACE-ZOOM (epoch fixed, cache amortized): CPU {cpu_ms:.1} ms  ->  GPU {gpu_ms:.1} ms  = {:.1}x",
                    cpu_ms / gpu_ms.max(1e-6)
                );
                eprintln!(
                    "DEEP-TIME (epoch changes each frame): CPU {cpu_ms:.1} ms  ->  GPU {:.1} ms (build+render) = {:.1}x",
                    build_ms + gpu_ms,
                    cpu_ms / (build_ms + gpu_ms).max(1e-6)
                );
            }
        }
        eprintln!("(sink {sink})");
    }
}
