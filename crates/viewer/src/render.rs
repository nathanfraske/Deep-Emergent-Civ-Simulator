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
use civsim_sim::genesis::LivingWorld;
use civsim_sim::geodynamics::DerivedTile;
use civsim_world::terrain::TerrainRelief;
use civsim_world::{BiomeSet, Coord3, Rgb, TopologySpace};

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

/// The DERIVED tile relief at surface coordinate (u, v) (each in `[0, 1)`), an orthographic read of the derived relief
/// field wrapped onto the globe (the same (u, v) -> cell mapping [`pick_surface_tile`] inverts). `None` for an empty
/// field, so the caller falls back to a stand-in. Display-only.
fn sample_derived_relief(
    tiles: &[DerivedTile],
    cols: usize,
    u: f32,
    v: f32,
) -> Option<TerrainRelief> {
    if tiles.is_empty() || cols == 0 {
        return None;
    }
    let rows = tiles.len().div_ceil(cols);
    let cu = ((u.clamp(0.0, 0.999_9) * cols as f32) as usize).min(cols - 1);
    let cv = ((v.clamp(0.0, 0.999_9) * rows as f32) as usize).min(rows.saturating_sub(1));
    let idx = (cv * cols + cu).min(tiles.len() - 1);
    Some(tiles[idx].relief)
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

/// Draw the planet as a lit sphere: a filled disk of on-screen radius `radius_px` centred at `(cx, cy)`, its
/// surface textured from the DERIVED tiles (an orthographic sphere map of the relief field, sampled at the surface
/// coordinate the globe `orient`ation has rotated under each pixel) and shaded by a Lambert diffuse term against the
/// star direction `star_dir`. The sunlit hemisphere is bright and tinted by `light_tint` (the star's
/// [`blackbody_rgb`]); the night side falls to a faint neutral ambient; the cosine falloff between them is the soft
/// day/night terminator. The lighting rotates WITH `orient` (camera-orbit semantics), so panning sweeps the terminator
/// across the surface and the lit part visibly changes as the globe turns, even on a uniform crust. `style.tint`, if
/// given, is the crust's DERIVED perceived colour under the star ([`material_surface_rgb`]): each tile takes that colour
/// scaled by its relief shading, so the sphere wears the derived material colour rather than the relief swatch.
/// `style.grid`, if given, overlays a lat/lon tile grid as thin darkened seams, so the surface reads as an array of
/// tiles (the caller refines the grid with zoom so a tile opens into finer tiles). Pixels outside the disk are left
/// untouched (the caller paints space and the atmosphere limb). A pure, deterministic read of the derived radius,
/// tiles, star direction, style, and orientation, one-way canon -> pixels (Principle 10).
#[allow(clippy::too_many_arguments)]
pub fn draw_globe(
    buf: &mut [u32],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    radius_px: usize,
    tiles: &[DerivedTile],
    tile_cols: usize,
    star_dir: [f32; 3],
    light_tint: Rgb,
    style: SurfaceStyle,
    orient: GlobeOrientation,
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
    // A faint neutral ambient so the night hemisphere reads dark but not pure black (skyglow and starlight).
    const AMBIENT: f32 = 0.10;
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
            let (u, v) = body_to_uv(view_to_body([nx, -ny, nz], orient));
            // The surface base colour: when `surface_tint` is given (the derived crust's perceived colour under the
            // star, from `material_surface_rgb`), each tile is that colour scaled by its relief shading, so the sphere
            // wears the DERIVED material colour; otherwise the relief swatch ([`derived_tile_color`]). A uniform crust
            // reads a single shade (the honest look until lateral composition variation lands, a geodynamics
            // follow-on); an empty field falls back to the tint or a deep-ocean stand-in.
            let base = match sample_derived_relief(tiles, tile_cols, u, v) {
                Some(relief) => match style.tint {
                    Some(m) => {
                        let s = relief_shade(relief);
                        let scale = |c: u8| (c as f32 * s).clamp(0.0, 255.0) as u8;
                        Rgb::new(scale(m.r), scale(m.g), scale(m.b))
                    }
                    None => derived_tile_color(relief),
                },
                None => style.tint.unwrap_or(Rgb::new(40, 72, 120)),
            };
            // Lambert diffuse: dot of the surface normal with the star direction, clamped at the terminator. The
            // normal uses WORLD-UP y (-ny, since screen y points down), the SAME frame the tile sample above uses
            // ([nx, -ny, nz]) and the frame `l` is carried into; without this the brightness was computed in
            // screen-down y while the tiles were placed in world-up y, so the terminator did not line up with the
            // tiles (an inverted-vertical mismatch).
            let lambert = (nx * l[0] - ny * l[1] + nz * l[2]).max(0.0);
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
    tile_cols: usize,
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
        tile_cols,
        star_dir,
        star_color,
        style,
        orient,
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
    tile_cols: usize,
    t_eff_k: Fixed,
    star_dir_body: [f32; 3],
    star: Option<(i32, i32, usize)>,
    sky: Rgb,
    style: SurfaceStyle,
    orient: GlobeOrientation,
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
        tile_cols,
        star_dir_body,
        light_tint,
        style,
        orient,
    );
    // The limb wants the VIEW-space sun direction (screen x, y), the projection of the derived body-frame vector.
    let limb_dir = normalize3(body_to_view(star_dir_body, orient));
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

/// The derived-surface tile (column, row) under a screen pixel, inverting the orthographic sphere map and the globe
/// orientation: the pixel offset from the globe centre `(cx, cy)` normalized by the on-screen `radius_px` gives the
/// front-hemisphere view point (`z = sqrt(1 - x^2 - y^2)`), the inverse rotation ([`view_to_body`]) carries it to the
/// body frame, and its (u, v) selects the tile the same way [`sample_derived_surface`] does (a `cols` by `rows`
/// field). `None` if the pixel is off the sphere's disk (fail-soft: the caller draws no highlight). A pure inverse of
/// the sphere map, display-only (Principle 10).
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
            cols,
            [1.0, 0.0, 0.0],
            Rgb::new(255, 255, 255),
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
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
            cols,
            [1.0, 0.0, 0.0],
            Rgb::new(255, 255, 255),
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
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
            cols,
            s,
            Rgb::new(255, 255, 255),
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
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
            cols,
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
            cols,
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
                cols,
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
            cols,
            [0.3, -0.2, 0.9],
            Rgb::new(255, 250, 240),
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
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
            cols,
            [0.3, -0.2, 0.9],
            Rgb::new(255, 250, 240),
            SurfaceStyle::default(),
            GlobeOrientation {
                rot_lon: 0.0,
                rot_lat: 0.8,
            },
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
            cols,
            star,
            white,
            SurfaceStyle::default(),
            GlobeOrientation::IDENTITY,
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
            cols,
            star,
            white,
            SurfaceStyle::default(),
            GlobeOrientation {
                rot_lon: 1.2,
                rot_lat: 0.0,
            },
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
                cols,
                star,
                white,
                SurfaceStyle {
                    tint: Some(tint),
                    grid: None,
                },
                GlobeOrientation::IDENTITY,
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
                cols,
                star,
                white,
                SurfaceStyle { tint, grid },
                GlobeOrientation::IDENTITY,
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
}
