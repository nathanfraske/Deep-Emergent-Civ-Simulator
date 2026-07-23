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

//! GRAVITATIONAL DIFFERENTIATION (identity link): the piece between the condensed bulk and the SURFACE (crust)
//! composition that `slice0_demo_field` named as its authored stand-in ("a stand-in for what accretion and
//! differentiation will derive"). Under gravity the condensed phases separate into two immiscible fractions that
//! settle by density: the dense, oxygen-free METAL and SULFIDE melt sinks toward the core, and the light,
//! oxygen-bearing SILICATE and OXIDE fraction floats as the mantle and crust. So the SURFACE the tile chain reads is
//! the floating silicate crust, DERIVED from the condensation, not an authored composition banded for visual variety.
//! This is the banked Stokes-settling differentiation run on the disposer's own output; the anion chemistry is the
//! proxy for which immiscible melt a phase joins, and the melt densities (metal and sulfide ~5 to 8 against silicate
//! ~3) do the settling.
//!
//! NO AUTHORED GOLDSCHMIDT LIST (Principle 4), and this IS how it admits the alien (Principle 7): the rule reads
//! whether each condensed phase carries oxygen, which is the fO2 ladder's OWN readout. Siderophile-versus-lithophile
//! was always downstream physics: "did the oxygen fugacity at this orbit give the element's stable phase an oxygen".
//! So the SAME rule partitions a world by its OWN redox, never a fixed element table: at oxidizing fugacities calcium
//! and magnesium condense as silicates (oxygen-bearing) and FLOAT; at reducing, enstatite-chondrite fugacities they
//! condense as the sulfides oldhamite (CaS) and niningerite (MgS) (oxygen-free) and DEFECT to the sinking set. That
//! Ca-Mg defection under reduction is the test that proves the link reads chemistry, not a hidden list. The
//! oxygen-lithophile criterion is the pinned row for oxygen-chemistry worlds; a non-oxygen lithophile chemistry (a
//! carbon world's carbides and graphite, an icy world's ices) is a NAMED data extension (non-exhaustive), until which
//! a world with no oxygen-bearing condensate yields no derived crust (fail-loud), never a mis-sorted one.
//!
//! Honest grade: the IDENTITY of each fraction is derived here (which phases and elements sink or float), the part
//! that made the authored arrangement arbitrary. The ASSEMBLAGE composition ([`phase_set_composition`], seam 5) is
//! the rock the phases form: each element carried at the sum over the phases of the phase's modal amount (from the
//! VCS redistribution, `equilibrium_condensation::condensed_amounts`) times its stoichiometric count, so a crust of
//! enstatite reads `Mg:Si:O = 1:1:3` and not the oxygen-heavy solar element budget it condensed from. The
//! `DifferentiatedPlanet::surface_composition` field below is a coarser AUDIT view (the floating-fraction elements
//! at their bulk abundance, an at-a-glance which-elements-floated, distinct from the crust assemblage the tiles
//! read). The crust-versus-mantle split within the floating fraction is now the PARTIAL-MELT MECHANISM (seam 6,
//! [`partial_melt_crust_and_mantle`]): the crust is the low-melting FIRST MELT the banked Schroeder-van Laar melt
//! rung ([`civsim_physics::melting`]) extracts, enriched in the fusible phases, and the mantle is the refractory
//! residue it leaves; the earlier pure-buoyancy split ([`crust_and_mantle`]) remains as the named fail-soft
//! fallback for a floating set with no melting data or a sub-solidus mantle.

use civsim_core::Fixed;
use civsim_physics::melting::{
    adiabatic_melt_column, eutectic_liquid_composition, multicomponent_solidus, Endmember,
    PressureMeltingRefusal,
};
use std::collections::BTreeMap;

/// Parse a condensed-species name (its phase suffix stripped) into element counts. General over any formula, so an
/// alien condensate is parsed, not special-cased. `None` on a malformed formula.
fn parse_formula(formula: &str) -> Option<BTreeMap<String, i32>> {
    let chars: Vec<char> = formula.chars().collect();
    let mut atoms: BTreeMap<String, i32> = BTreeMap::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_uppercase() {
            return None;
        }
        let mut symbol = String::new();
        symbol.push(chars[i]);
        i += 1;
        while i < chars.len() && chars[i].is_ascii_lowercase() {
            symbol.push(chars[i]);
            i += 1;
        }
        let mut digits = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() {
            digits.push(chars[i]);
            i += 1;
        }
        let count: i32 = if digits.is_empty() {
            1
        } else {
            digits.parse().ok()?
        };
        *atoms.entry(symbol).or_insert(0) += count;
    }
    if atoms.is_empty() {
        None
    } else {
        Some(atoms)
    }
}

/// The elements of a condensed-species name (the formula before its `(cr,...)` or `(l)` phase suffix).
fn species_elements(name: &str) -> Option<BTreeMap<String, i32>> {
    let formula = name.split('(').next()?;
    parse_formula(formula)
}

/// Whether a condensed phase floats to the crust: it carries a LITHOPHILE anion. Oxygen is the pinned lithophile
/// anion for oxygen-chemistry worlds (a silicate or oxide floats); an oxygen-free phase (metal or sulfide) sinks.
/// This reads the fO2 ladder's readout, not an element tag: the SAME check floats a Ca-silicate at oxidizing
/// fugacity and sinks oldhamite (CaS) at reducing. Non-oxygen lithophile chemistries are the named data extension
/// (see the module note); until then their phases do not satisfy this and a non-oxygen world yields no crust.
fn floats_to_crust(name: &str) -> bool {
    species_elements(name)
        .map(|atoms| atoms.contains_key("O"))
        .unwrap_or(false)
}

/// A differentiated planet: the condensed bulk separated into the fraction that sinks and the fraction that floats.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DifferentiatedPlanet {
    /// The SINKING fraction (core and metal-sulfide interior): the oxygen-free metal and sulfide condensates,
    /// (species, presence).
    pub sinking: Vec<(String, Fixed)>,
    /// The FLOATING fraction (mantle and crust): the oxygen-bearing silicate and oxide condensates, (species,
    /// presence).
    pub floating: Vec<(String, Fixed)>,
    /// An AUDIT view of the floating fraction: its elements at their bulk abundance, an at-a-glance which-elements-
    /// floated. This is NOT the surface the tile chain reads (that is the crust ASSEMBLAGE, `SurfaceComposition::
    /// surface` via [`phase_set_composition`], seam 5); it is the coarse floating-fraction summary. Deterministic
    /// element order.
    pub surface_composition: Vec<(String, Fixed)>,
}

/// Differentiate the condensed bulk, deriving the surface composition. `condensed_bulk` is the condensed phases at
/// the orbit (`equilibrium_condensation::condensed_active_set`), each a (species name, presence) pair. `bulk_
/// abundances` maps each element to its bulk amount, the source of the surface element amounts (identity-derived,
/// amounts-pending, see the module note). The oxygen-free metal and sulfide phases sink; the oxygen-bearing silicate
/// and oxide phases float; the surface carries each element the floating phases claim, at its bulk abundance.
/// `None` if the bulk is empty or no phase floats (a world with no oxygen-bearing condensate has no derived crust:
/// fail-loud, never a mis-sorted or fabricated surface).
pub fn differentiate(
    condensed_bulk: &[(String, Fixed)],
    bulk_abundances: &BTreeMap<String, Fixed>,
) -> Option<DifferentiatedPlanet> {
    if condensed_bulk.is_empty() {
        return None;
    }
    let mut sinking = Vec::new();
    let mut floating = Vec::new();
    let mut surface_elements: BTreeMap<String, Fixed> = BTreeMap::new();
    for (name, presence) in condensed_bulk {
        if floats_to_crust(name) {
            floating.push((name.clone(), *presence));
            if let Some(atoms) = species_elements(name) {
                for element in atoms.keys() {
                    if let Some(abundance) = bulk_abundances.get(element) {
                        surface_elements.insert(element.clone(), *abundance);
                    }
                }
            }
        } else {
            sinking.push((name.clone(), *presence));
        }
    }
    if floating.is_empty() || surface_elements.is_empty() {
        return None;
    }
    Some(DifferentiatedPlanet {
        sinking,
        floating,
        surface_composition: surface_elements.into_iter().collect(),
    })
}

/// THE PARTIAL-MELT CRUST EXTRACTION: split the floating silicate fraction into the CRUST (the buoyant partial melt
/// that floats to the surface) and the MANTLE (the dense refractory residue it leaves). Within the silicate fraction
/// the split is BUOYANCY, not chemistry: the low-density melt (feldspar ~2.76, silica ~2.65) rises over the dense
/// refractory (forsterite ~3.27), so the crust is the LEAST-DENSE floating phase and the mantle is the denser rest.
/// This is the density contrast that gives the crust its isostatic relief (crust ~2.8 floating on mantle ~3.3),
/// which a crust identical to its mantle cannot. Distinct from and downstream of [`differentiate`]: that set the
/// metal-sulfide/silicate split by chemistry (immiscibility); this sets the crust/mantle split by density (buoyancy),
/// the two physical drivers each in their own place.
///
/// Admit the alien: the primitive is density, so WHAT floats as crust is a data outcome (feldspathic on a Terran
/// silicate world, a different light phase elsewhere), never a mineral table. `density_of` returns each floating
/// phase's DERIVED density (the caller injects it from the petrology substrate). Returns (crust, mantle) as
/// (species, presence) lists; `None` if a floating-phase density fails to resolve (fail-loud). A single floating
/// phase is all crust (nothing denser to reject to the mantle).
// The (crust, mantle) pair of phase lists is the natural return; a named type would not read clearer than the pair.
#[allow(clippy::type_complexity)]
pub fn crust_and_mantle<F>(
    floating: &[(String, Fixed)],
    density_of: F,
) -> Option<(Vec<(String, Fixed)>, Vec<(String, Fixed)>)>
where
    F: Fn(&str) -> Option<Fixed>,
{
    if floating.is_empty() {
        return None;
    }
    // Each phase's density, when the petrology can resolve it. A phase the substrate has no molar volume for (an
    // exotic refractory like corundum or spinel the density kernel does not yet cover) is placed in the MANTLE as a
    // refractory rather than assigned a fabricated density: unresolved-density phases are not buoyancy candidates.
    // Preserve the input order (the condensation precedence) for determinism.
    let mut resolved: Vec<Option<Fixed>> = Vec::with_capacity(floating.len());
    let mut min_density: Option<Fixed> = None;
    for (name, _) in floating {
        let d = density_of(name);
        if let Some(d) = d {
            min_density = Some(match min_density {
                Some(m) if m.to_bits() <= d.to_bits() => m,
                _ => d,
            });
        }
        resolved.push(d);
    }
    let min_density = min_density?; // no floating phase has a resolvable density: no crust to float
    let mut crust = Vec::new();
    let mut mantle = Vec::new();
    for ((name, presence), density) in floating.iter().zip(resolved.iter()) {
        match density {
            Some(d) if *d <= min_density => crust.push((name.clone(), *presence)),
            _ => mantle.push((name.clone(), *presence)),
        }
    }
    Some((crust, mantle))
}

/// One gigapascal in bars, the petrology pressure unit the melt rung reads (the solidus slope is taken as the
/// solidus at 1 GPa minus at the surface). An engine unit bridge, not world content.
const ONE_GPA_IN_BAR: i32 = 10_000;

/// The McKenzie-Bickle (1988) adiabatic-decompression-melting-column parameters the crustal-thickness closure
/// reads, gathered so the mechanism stays a pure function of data (nothing authored in the kernel). The solidus
/// (surface value and slope) is NOT here: it is DERIVED from the endmember signatures inside
/// [`partial_melt_crust_and_mantle`] (consuming the melt rung). These are the interior-thermostat and mantle-
/// floor inputs the caller supplies (each reserved-with-basis and cited, or itself derived), so an alien mantle
/// is a different set of numbers, never a code path.
#[derive(Clone, Copy, Debug)]
pub struct MeltColumnParams {
    /// The mantle POTENTIAL TEMPERATURE (kelvin): the adiabat projected to the surface, the interior thermostat's
    /// output. Below the derived solidus the mantle melts nothing (a sub-solidus mantle yields no partial-melt
    /// crust, the fail-soft to buoyancy).
    pub potential_temperature_k: Fixed,
    /// The mantle ADIABAT slope (kelvin per gigapascal), dT/dP along the isentrope.
    pub adiabat_slope_k_per_gpa: Fixed,
    /// The isentropic melting PRODUCTIVITY dF/dP (per gigapascal) near the solidus.
    pub productivity_per_gpa: Fixed,
    /// The mantle SOURCE DENSITY (kilograms per cubic metre) the pooled-melt thickness divides by.
    pub source_density_kg_per_m3: Fixed,
    /// The surface GRAVITY (metres per second squared).
    pub gravity_m_per_s2: Fixed,
}

/// WHY a partial-melt extraction produced no melt column, so a downstream reader can tell an honest sub-solidus
/// mantle from a degenerate near-failure input rather than painting both as "unprocessed". `Melted`: the column ran
/// and built crust. `SubSolidus`: a valid assemblage sat below its solidus (or the melt was not evaluated), the
/// honest unprocessed mantle. `Degenerate`: a near-failure reached the fallback (a phase with no melting datum, an
/// unsolvable eutectic, no source weight, or an empty crust-or-mantle split), which should NOT be read as a physical
/// sub-solidus outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeltStatus {
    Melted,
    SubSolidus,
    Degenerate,
}

/// The result of the partial-melt crust extraction: the CRUST (the extracted first melt) and the MANTLE (the
/// refractory residue) as weighted phase lists, plus the McKenzie-Bickle crustal thickness. When the melt rung
/// cannot run (a floating phase with no melting datum, an unsolvable eutectic, or a sub-solidus mantle),
/// `used_partial_melt` is false and the split is the pure-buoyancy fallback with no thickness (fail-soft, named);
/// `melt_status` says WHICH of those it was (a sub-solidus mantle versus a degenerate input).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialMeltCrust {
    /// The CRUST phases as `(name, liquid mole fraction x_i)`: the extracted first melt, enriched in the fusible
    /// phases (the incompatible-element enrichment falls out of this weighting, no separate model). Weights sum
    /// to one. On the buoyancy fallback these are the least-dense floating phases at their input presence.
    pub crust: Vec<(String, Fixed)>,
    /// The MANTLE phases as `(name, residue weight)`: the refractory residue the melt leaves (the source share
    /// minus the melt it gave up), enriched in the small-liquid-fraction refractory phases (olivine). On the
    /// buoyancy fallback these are the denser floating phases at their input presence.
    pub mantle: Vec<(String, Fixed)>,
    /// The McKenzie-Bickle crustal thickness (kilometres), `None` on the buoyancy fallback (no melt column).
    pub crust_thickness_km: Option<Fixed>,
    /// The peak melt fraction at the top of the column, for audit; `None` on the buoyancy fallback.
    pub max_melt_fraction: Option<Fixed>,
    /// The pressure (gigapascals) at which the rising mantle first crossed the solidus; `None` on the fallback.
    pub onset_pressure_gpa: Option<Fixed>,
    /// The DERIVED solidus surface temperature (K): the multi-saturation solidus of the floating assemblage's own
    /// endmember signatures at zero pressure ([`multicomponent_solidus`]), consumed here for the melt column and
    /// EXPOSED for a downstream deep-time volcanism that melts the same mantle. `Some` whenever the endmembers and
    /// their eutectic resolved (including the sub-solidus buoyancy fallback, where the solidus is still derived but
    /// the potential temperature does not cross it); `None` only when a missing melting datum or an unsolvable
    /// eutectic aborted before the solidus could be taken. This retires an authored peridotite solidus downstream:
    /// the solidus is the world's own, keyed on its endmembers, never Earth's 1373 K.
    pub solidus_surface_k: Option<Fixed>,
    /// The DERIVED solidus SLOPE (K per GPa): the Clausius-Clapeyron slope of the same assemblage's solidus (the
    /// derived one-gigapascal solidus minus the surface value). `Some`/`None` on the same condition as
    /// `solidus_surface_k`. Retires an authored ~130 K/GPa downstream: the slope is the world's own.
    pub solidus_slope_k_per_gpa: Option<Fixed>,
    /// Whether the PARTIAL-MELT mechanism ran (`true`) or the split fell back to buoyancy (`false`).
    pub used_partial_melt: bool,
    /// WHY there was no melt column (`Melted`, `SubSolidus`, or `Degenerate`), so a sub-solidus mantle is not
    /// confused with a near-failure input downstream.
    pub melt_status: MeltStatus,
}

/// THE PARTIAL-MELT CRUST EXTRACTION (seam 6): split the floating silicate fraction into the CRUST (the low-
/// melting first melt) and the MANTLE (the refractory residue) by the banked Schroeder-van Laar melt rung
/// ([`civsim_physics::melting`]), the physically-correct partial-melt mechanism that replaces "least-dense whole
/// phase = crust" with "the melt = crust". The crust is the eutectic FIRST-MELT liquid composition at the
/// crustal reference pressure: enriched in the fusible phases (clinopyroxene, plagioclase: a basalt) and
/// depleted in the refractory olivine, so the incompatible-element enrichment is an OUTPUT of the eutectic
/// weighting, never a separate model. The mantle is the batch-melt residue, `residue_i = source_i - F * x_i`,
/// enriched in the refractory phases the melt left behind. The crustal THICKNESS is the McKenzie-Bickle
/// decompression-melting column, `crust = (dF/dP) P0^2 / (2 rho g)`, whose solidus (surface value and slope) is
/// DERIVED from the endmember signatures here (consuming the rung), the rest of its inputs the caller's data.
///
/// FAIL-SOFT (named): a floating phase with no `Endmember` (no surface melting signature), or a SUB-SOLIDUS
/// mantle (the potential temperature does not cross the derived solidus, so no melt forms) falls back to the
/// pure-buoyancy split ([`crust_and_mantle`]) with `used_partial_melt = false` and no thickness. Missing
/// fusion-volume evidence is different: the surface signature exists, but the pressure slope does not, so the
/// function returns [`PressureMeltingRefusal`] and cannot select a crust outcome. Admit-the-alien: the mechanism
/// keys off each phase's own `Endmember` signature (via `endmember_of`), so any endmember set is handled by the
/// same law or the same refusal.
///
/// `floating` is the oxygen-bearing silicate fraction ([`DifferentiatedPlanet::floating`]); `source_amounts`
/// gives each phase's VCS modal amount (the saturation presence otherwise), the residue mass balance's source
/// weights; `endmember_of` reads a phase's melting signature by species name (the melting registry);
/// `density_of` is the buoyancy fallback's density read; `reference_pressure_bar` is where the first-melt
/// composition is taken (the surface, where the pooled melt sits); `params` are the McKenzie-Bickle inputs.
/// `Ok(None)` means the buoyancy fallback itself could not resolve. `Err` preserves a pressure-melting refusal.
pub fn partial_melt_crust_and_mantle<E, D>(
    floating: &[(String, Fixed)],
    source_amounts: &BTreeMap<String, Fixed>,
    endmember_of: E,
    density_of: D,
    reference_pressure_bar: Fixed,
    params: &MeltColumnParams,
) -> Result<Option<PartialMeltCrust>, PressureMeltingRefusal>
where
    E: Fn(&str) -> Option<Endmember>,
    D: Fn(&str) -> Option<Fixed>,
{
    macro_rules! or_unavailable {
        ($value:expr) => {
            match $value {
                Some(value) => value,
                None => return Ok(None),
            }
        };
    }

    if floating.is_empty() {
        return Ok(None);
    }
    // Gather each floating phase's endmember. A single missing melting datum trips the fail-soft buoyancy
    // fallback (named), so the chain never aborts on incomplete data. Input order preserved (determinism).
    let mut endmembers: Vec<Endmember> = Vec::with_capacity(floating.len());
    for (name, _) in floating {
        match endmember_of(name) {
            Some(em) => endmembers.push(em),
            // No melting datum for this phase: the solidus cannot be derived, so the fallback carries no solidus.
            None => {
                return Ok(buoyancy_fallback(
                    floating,
                    density_of,
                    None,
                    MeltStatus::Degenerate,
                ));
            }
        }
    }
    // The FIRST-MELT (eutectic) liquid composition at the crustal reference pressure = the CRUST (the extracted
    // partial melt, enriched in the fusible phases). The mole fractions come back in the input order.
    let (_reference_solidus, x_liq) =
        eutectic_liquid_composition(&endmembers, reference_pressure_bar)?;
    // The crustal THICKNESS via the McKenzie-Bickle column. The solidus surface value and slope are DERIVED from
    // the endmember signatures (consuming the rung), never authored: the multi-saturation solidus at the surface
    // and at one gigapascal.
    let solidus_surface = multicomponent_solidus(&endmembers, Fixed::ZERO)?;
    let solidus_deep = multicomponent_solidus(&endmembers, Fixed::from_int(ONE_GPA_IN_BAR))?;
    let solidus_slope = or_unavailable!(solidus_deep.checked_sub(solidus_surface));
    // The derived solidus travels to every downstream return, INCLUDING the sub-solidus buoyancy fallback: the
    // solidus is a property of the assemblage, computed whether or not the potential temperature crosses it, so a
    // deep-time volcanism melting the same mantle reads the world's own solidus rather than an authored one.
    let solidus = Some((solidus_surface, solidus_slope));
    let column = adiabatic_melt_column(
        params.potential_temperature_k,
        solidus_surface,
        solidus_slope,
        params.adiabat_slope_k_per_gpa,
        params.productivity_per_gpa,
        params.source_density_kg_per_m3,
        params.gravity_m_per_s2,
    );
    // A sub-solidus mantle (no melt) has no partial-melt crust: fall back to buoyancy (the pre-melt density
    // sorting, the honest state when the mantle is too cold to melt).
    let column = match column {
        Some(c) if c.crust_thickness_km > Fixed::ZERO && c.max_melt_fraction > Fixed::ZERO => c,
        _ => {
            return Ok(buoyancy_fallback(
                floating,
                density_of,
                solidus,
                MeltStatus::SubSolidus,
            ));
        }
    };
    let f = column.max_melt_fraction;
    // The normalized source weights (the VCS modal amount, the saturation presence otherwise), for the batch-
    // melt residue.
    let mut source_weights: Vec<Fixed> = Vec::with_capacity(floating.len());
    let mut total = Fixed::ZERO;
    for (name, presence) in floating {
        let w = source_amounts.get(name).copied().unwrap_or(*presence);
        let w = if w < Fixed::ZERO { Fixed::ZERO } else { w };
        total = or_unavailable!(total.checked_add(w));
        source_weights.push(w);
    }
    if total <= Fixed::ZERO {
        return Ok(buoyancy_fallback(
            floating,
            density_of,
            solidus,
            MeltStatus::Degenerate,
        ));
    }
    // The crust is the melt (weight = liquid mole fraction); the mantle is the residue,
    // residue_i = source_share_i - F * x_i, clamped at zero (a fusible phase can be fully consumed into the melt).
    let mut crust: Vec<(String, Fixed)> = Vec::with_capacity(floating.len());
    let mut mantle: Vec<(String, Fixed)> = Vec::with_capacity(floating.len());
    for (i, (name, _)) in floating.iter().enumerate() {
        let x = x_liq[i];
        if x > Fixed::ZERO {
            crust.push((name.clone(), x));
        }
        let share = or_unavailable!(source_weights[i].checked_div(total));
        let removed = or_unavailable!(f.checked_mul(x));
        let residue = share.checked_sub(removed).unwrap_or(Fixed::ZERO);
        if residue > Fixed::ZERO {
            mantle.push((name.clone(), residue));
        }
    }
    if crust.is_empty() || mantle.is_empty() {
        return Ok(buoyancy_fallback(
            floating,
            density_of,
            solidus,
            MeltStatus::Degenerate,
        ));
    }
    Ok(Some(PartialMeltCrust {
        crust,
        mantle,
        crust_thickness_km: Some(column.crust_thickness_km),
        max_melt_fraction: Some(column.max_melt_fraction),
        onset_pressure_gpa: Some(column.onset_pressure_gpa),
        solidus_surface_k: Some(solidus_surface),
        solidus_slope_k_per_gpa: Some(solidus_slope),
        used_partial_melt: true,
        melt_status: MeltStatus::Melted,
    }))
}

/// The pure-buoyancy fallback packaged as a [`PartialMeltCrust`] with no melt column, so the partial-melt
/// caller degrades to the earlier split without a special-case return type. `None` only if the buoyancy split
/// cannot resolve a single floating-phase density (the same fail-loud as [`crust_and_mantle`]). `solidus`
/// carries the DERIVED solidus (surface value, slope) when the endmembers and their eutectic resolved before the
/// fallback (a sub-solidus mantle, the common case), so the derived solidus survives the fallback for a
/// downstream consumer; it is `None` only when the fallback fired before the solidus could be taken (a missing
/// melting datum or an unsolvable eutectic).
fn buoyancy_fallback<D>(
    floating: &[(String, Fixed)],
    density_of: D,
    solidus: Option<(Fixed, Fixed)>,
    status: MeltStatus,
) -> Option<PartialMeltCrust>
where
    D: Fn(&str) -> Option<Fixed>,
{
    let (crust, mantle) = crust_and_mantle(floating, density_of)?;
    let (solidus_surface_k, solidus_slope_k_per_gpa) = match solidus {
        Some((surface, slope)) => (Some(surface), Some(slope)),
        None => (None, None),
    };
    Some(PartialMeltCrust {
        crust,
        mantle,
        crust_thickness_km: None,
        max_melt_fraction: None,
        onset_pressure_gpa: None,
        solidus_surface_k,
        solidus_slope_k_per_gpa,
        used_partial_melt: false,
        melt_status: status,
    })
}

/// THE ASSEMBLAGE COMPOSITION (seam 5): the element amounts of a phase set as the ROCK it is, the sum over the
/// phases of each phase's modal amount times the element's stoichiometric count in that phase. A pure enstatite
/// (MgSiO3) crust comes out `Mg:Si:O = 1:1:3`, its real mineral ratio, not the oxygen-heavy solar element budget
/// the phases condensed from (which was never a rock). `amounts` gives each phase's modal amount from the VCS
/// redistribution ([`crate::equilibrium_condensation::condensed_amounts`]); a phase absent from `amounts` (a
/// degenerate VCS vertex where the moles did not resolve) falls back to its own saturation presence, the
/// best-available proxy, still a stoichiometric rock rather than the solar budget. Deterministic element order.
pub fn phase_set_composition(
    phases: &[(String, Fixed)],
    amounts: &BTreeMap<String, Fixed>,
) -> Vec<(String, Fixed)> {
    let mut elements: BTreeMap<String, Fixed> = BTreeMap::new();
    for (name, presence) in phases {
        let Some(atoms) = species_elements(name) else {
            continue;
        };
        // The phase's modal amount when the VCS vertex was well-posed, its saturation presence otherwise. Never the
        // solar element budget: the composition is the assemblage the phases form, weighted by how much of each.
        let weight = amounts.get(name).copied().unwrap_or(*presence);
        for (element, count) in &atoms {
            let contribution = match weight.checked_mul(Fixed::from_int(*count)) {
                Some(c) => c,
                None => continue,
            };
            let entry = elements.entry(element.clone()).or_insert(Fixed::ZERO);
            *entry = entry.checked_add(contribution).unwrap_or(*entry);
        }
    }
    elements.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bulk(entries: &[(&str, i64)]) -> BTreeMap<String, Fixed> {
        entries
            .iter()
            .map(|(el, n)| (el.to_string(), Fixed::from_int(*n as i32)))
            .collect()
    }

    fn one(name: &str) -> (String, Fixed) {
        (name.to_string(), Fixed::ONE)
    }

    fn sink_has(d: &DifferentiatedPlanet, name: &str) -> bool {
        d.sinking.iter().any(|(n, _)| n == name)
    }
    fn float_has(d: &DifferentiatedPlanet, name: &str) -> bool {
        d.floating.iter().any(|(n, _)| n == name)
    }
    fn surface_has(d: &DifferentiatedPlanet, element: &str) -> bool {
        d.surface_composition.iter().any(|(e, _)| e == element)
    }

    #[test]
    fn at_solar_composition_metal_and_sulfide_sink_and_silicates_float() {
        // The acceptance row, oxidizing (solar) fugacity: the sinking set contains Fe-Ni metal and FeS; the floating
        // set carries the Mg-silicates and the feldspathic (Ca-Al) component. The surface is the silicate elements.
        let condensed = vec![
            one("Fe(cr)"),
            one("Ni(cr)"),
            one("FeS(cr,troilite)"),
            one("Mg2SiO4(cr,forsterite)"),
            one("MgSiO3(cr,enstatite)"),
            one("CaAl2Si2O8(cr,anorthite)"),
        ];
        let abundances = bulk(&[
            ("Fe", 90),
            ("Ni", 5),
            ("Mg", 100),
            ("Si", 90),
            ("O", 340),
            ("S", 45),
            ("Ca", 6),
            ("Al", 9),
        ]);
        let d = differentiate(&condensed, &abundances).expect("solar bulk differentiates");
        assert!(
            sink_has(&d, "Fe(cr)") && sink_has(&d, "Ni(cr)") && sink_has(&d, "FeS(cr,troilite)")
        );
        assert!(
            float_has(&d, "Mg2SiO4(cr,forsterite)") && float_has(&d, "CaAl2Si2O8(cr,anorthite)"),
            "the Mg-silicates and feldspathic component float"
        );
        assert!(
            surface_has(&d, "Mg")
                && surface_has(&d, "Si")
                && surface_has(&d, "Ca")
                && surface_has(&d, "Al")
        );
        assert!(
            !surface_has(&d, "Ni") && !surface_has(&d, "S"),
            "the sinking metal and sulfur are not on the surface"
        );
    }

    #[test]
    fn under_reduction_calcium_and_magnesium_defect_to_the_sinking_set_as_sulfides() {
        // The counter-test that proves the link reads chemistry, not an element list: at IW-class reducing fugacity
        // the condensation makes calcium and magnesium as the sulfides oldhamite (CaS) and niningerite (MgS), which
        // are oxygen-free and so DEFECT to the sinking set, while the same elements floated as silicates at solar
        // fugacity above. The floating fraction is only the enstatite that stayed oxygen-bearing.
        let condensed = vec![
            one("Fe(cr)"),
            one("CaS(cr,oldhamite)"),
            one("MgS(cr,niningerite)"),
            one("MgSiO3(cr,enstatite)"),
        ];
        let abundances = bulk(&[
            ("Fe", 90),
            ("Ca", 6),
            ("Mg", 100),
            ("Si", 60),
            ("O", 180),
            ("S", 40),
        ]);
        let d = differentiate(&condensed, &abundances).expect("reduced bulk differentiates");
        assert!(
            sink_has(&d, "CaS(cr,oldhamite)") && sink_has(&d, "MgS(cr,niningerite)"),
            "under reduction the Ca and Mg sulfides sink"
        );
        // Ca defected entirely to the sink (only present as CaS), so it is not on the surface; Mg still floats in
        // the enstatite that stayed a silicate, so Mg remains a surface element.
        assert!(
            !surface_has(&d, "Ca"),
            "reduced calcium defected to the core, off the surface"
        );
        assert!(
            surface_has(&d, "Mg") && surface_has(&d, "Si"),
            "the silicate that stayed floats"
        );
    }

    #[test]
    fn a_world_with_no_oxygen_bearing_condensate_has_no_crust() {
        // A fully reduced or non-oxygen bulk floats nothing under the pinned oxygen-lithophile row: fail-loud (None),
        // the honest edge until the non-oxygen lithophile extension lands, never a mis-sorted crust.
        let condensed = vec![
            one("Fe(cr)"),
            one("FeS(cr,troilite)"),
            one("SiC(cr,moissanite)"),
        ];
        let abundances = bulk(&[("Fe", 90), ("Si", 40), ("S", 45), ("C", 60)]);
        assert!(differentiate(&condensed, &abundances).is_none());
    }

    #[test]
    fn the_surface_is_the_rock_assemblage_not_the_solar_budget() {
        // SEAM 5: a crust of pure enstatite MgSiO3 must read its mineral stoichiometry Mg:Si:O = 1:1:3, not the
        // oxygen-heavy solar element budget it condensed from. With the enstatite phase at unit modal amount the
        // element amounts are exactly (Mg 1, Si 1, O 3).
        let crust = vec![one("MgSiO3(cr,enstatite)")];
        let amounts: BTreeMap<String, Fixed> = [("MgSiO3(cr,enstatite)".to_string(), Fixed::ONE)]
            .into_iter()
            .collect();
        let comp: BTreeMap<String, Fixed> = phase_set_composition(&crust, &amounts)
            .into_iter()
            .collect();
        assert_eq!(comp.get("Mg"), Some(&Fixed::ONE));
        assert_eq!(comp.get("Si"), Some(&Fixed::ONE));
        assert_eq!(comp.get("O"), Some(&Fixed::from_int(3)));
        // The solar budget is oxygen-heavy (O ~ 340 against Si ~ 90); the assemblage must NOT reproduce that ratio.
        let o_over_si =
            comp.get("O").unwrap().to_bits() as f64 / comp.get("Si").unwrap().to_bits() as f64;
        assert!(
            (o_over_si - 3.0).abs() < 1e-6,
            "the crust reads its enstatite O:Si = 3, not the solar ~3.8+"
        );
    }

    #[test]
    fn a_two_phase_crust_weights_by_modal_amount() {
        // A crust of forsterite (Mg2SiO4) and enstatite (MgSiO3): the element amounts sum each phase's modal amount
        // times its stoichiometry. At forsterite 1, enstatite 2: Mg = 2*1 + 1*2 = 4, Si = 1*1 + 1*2 = 3,
        // O = 4*1 + 3*2 = 10. Doubling enstatite must shift the ratios, proving the modal weighting is live.
        let crust = vec![one("Mg2SiO4(cr,forsterite)"), one("MgSiO3(cr,enstatite)")];
        let amounts: BTreeMap<String, Fixed> = [
            ("Mg2SiO4(cr,forsterite)".to_string(), Fixed::ONE),
            ("MgSiO3(cr,enstatite)".to_string(), Fixed::from_int(2)),
        ]
        .into_iter()
        .collect();
        let comp: BTreeMap<String, Fixed> = phase_set_composition(&crust, &amounts)
            .into_iter()
            .collect();
        assert_eq!(comp.get("Mg"), Some(&Fixed::from_int(4)));
        assert_eq!(comp.get("Si"), Some(&Fixed::from_int(3)));
        assert_eq!(comp.get("O"), Some(&Fixed::from_int(10)));
    }

    #[test]
    fn a_degenerate_vertex_falls_back_to_the_phase_presence_not_the_budget() {
        // When the VCS moles did not resolve (the phase is absent from `amounts`), the assemblage falls back to the
        // phase's own saturation presence, still a stoichiometric rock. Enstatite at presence 2 reads Mg:Si:O =
        // 2:2:6, the enstatite ratio scaled by the presence, never the solar budget.
        let crust = vec![("MgSiO3(cr,enstatite)".to_string(), Fixed::from_int(2))];
        let empty: BTreeMap<String, Fixed> = BTreeMap::new();
        let comp: BTreeMap<String, Fixed> =
            phase_set_composition(&crust, &empty).into_iter().collect();
        assert_eq!(comp.get("Mg"), Some(&Fixed::from_int(2)));
        assert_eq!(comp.get("Si"), Some(&Fixed::from_int(2)));
        assert_eq!(comp.get("O"), Some(&Fixed::from_int(6)));
    }

    #[test]
    fn the_buoyant_silicate_floats_as_crust_over_the_dense_mantle() {
        // Within the floating silicate fraction the split is BUOYANCY: the lighter phase (enstatite ~3.20) floats as
        // the crust over the denser forsterite ~3.27 mantle residue, the density contrast the isostasy needs. A real
        // feldspathic crust (anorthite ~2.76) floats even more strongly when the data supplies it.
        let floating = vec![one("Mg2SiO4(cr,forsterite)"), one("MgSiO3(cr,enstatite)")];
        let density = |name: &str| -> Option<Fixed> {
            Some(match name.split('(').next()? {
                "Mg2SiO4" => Fixed::from_ratio(327, 100),
                "MgSiO3" => Fixed::from_ratio(320, 100),
                _ => return None,
            })
        };
        let (crust, mantle) = crust_and_mantle(&floating, density).expect("splits");
        assert!(
            crust.iter().any(|(n, _)| n.starts_with("MgSiO3")),
            "the lighter enstatite is the buoyant crust"
        );
        assert!(
            mantle.iter().any(|(n, _)| n.starts_with("Mg2SiO4")),
            "the denser forsterite is the mantle residue"
        );
    }

    // The fertile lherzolite endmember signatures (the cited data-file values, `data/melting_endmembers.toml`)
    // as a species-name lookup, so the partial-melt tests key off each phase's own melting data (the admit-the-
    // alien mechanism) and exercise the candidate-evidence signatures. Anorthite's fusion volume remains absent,
    // so any pressure-derived slope must refuse.
    fn fertile_endmember(name: &str) -> Option<Endmember> {
        let formula = name.split('(').next()?;
        Some(match formula {
            "Mg2SiO4" => Endmember {
                melting_point_k: Fixed::from_int(2163),
                fusion_enthalpy_j_per_mol: Fixed::from_int(142_000),
                fusion_volume_cm3_per_mol: Some(Fixed::from_ratio(384, 100)),
            },
            "MgSiO3" => Endmember {
                melting_point_k: Fixed::from_int(1830),
                fusion_enthalpy_j_per_mol: Fixed::from_int(73_000),
                fusion_volume_cm3_per_mol: Some(Fixed::from_ratio(481, 100)),
            },
            "CaMgSi2O6" => Endmember {
                melting_point_k: Fixed::from_int(1665),
                fusion_enthalpy_j_per_mol: Fixed::from_int(138_500),
                fusion_volume_cm3_per_mol: Some(Fixed::from_ratio(132, 10)),
            },
            "CaAl2Si2O8" => Endmember {
                melting_point_k: Fixed::from_int(1830),
                fusion_enthalpy_j_per_mol: Fixed::from_int(135_600),
                fusion_volume_cm3_per_mol: None,
            },
            _ => return None,
        })
    }

    // A normal-mantle McKenzie-Bickle parameter set (potential temperature 1588 K, adiabat 15.5 K/GPa,
    // productivity 0.12/GPa, source density 3300 kg/m3, gravity 9.8), the melt rung's validated inputs.
    fn normal_params() -> MeltColumnParams {
        MeltColumnParams {
            potential_temperature_k: Fixed::from_int(1588),
            adiabat_slope_k_per_gpa: Fixed::from_ratio(155, 10),
            productivity_per_gpa: Fixed::from_ratio(12, 100),
            source_density_kg_per_m3: Fixed::from_int(3300),
            gravity_m_per_s2: Fixed::from_ratio(98, 10),
        }
    }

    fn fertile_floating() -> (Vec<(String, Fixed)>, BTreeMap<String, Fixed>) {
        let floating = vec![
            one("Mg2SiO4(cr,forsterite)"),
            one("MgSiO3(cr,enstatite)"),
            one("CaMgSi2O6(cr,diopside)"),
            one("CaAl2Si2O8(cr,anorthite)"),
        ];
        let amounts: BTreeMap<String, Fixed> = floating
            .iter()
            .map(|(n, _)| (n.clone(), Fixed::ONE))
            .collect();
        (floating, amounts)
    }

    #[test]
    fn the_fertile_column_refuses_the_absent_anorthite_fusion_volume() {
        // The surface first-melt composition does not need dV_fus, but this function also derives a one-gigapascal
        // solidus slope for the decompression column. Anorthite has no evidenced fusion volume, so the combined
        // evaluation must refuse instead of assigning its pressure contribution a zero.
        let (floating, amounts) = fertile_floating();
        let density = |_: &str| Some(Fixed::from_int(3));
        let refusal = partial_melt_crust_and_mantle(
            &floating,
            &amounts,
            fertile_endmember,
            density,
            Fixed::ZERO,
            &normal_params(),
        )
        .unwrap_err();
        assert!(matches!(
            refusal,
            PressureMeltingRefusal::MissingFusionVolume { .. }
        ));
    }

    #[test]
    fn the_mckenzie_bickle_closure_reproduces_the_six_and_a_half_km_ocean_crust() {
        // The consumed closure, fed the MEASURED peridotite solidus (1373 K, 130 K/GPa) at a normal potential
        // temperature, reproduces McKenzie-Bickle's ~6.5 km of oceanic crust. This is the banked rung's own
        // validation, exercised here through the seam-6 consumer to prove the wiring reads the closure unchanged.
        let column = adiabatic_melt_column(
            Fixed::from_int(1588),
            Fixed::from_int(1373),
            Fixed::from_int(130),
            Fixed::from_ratio(155, 10),
            Fixed::from_ratio(12, 100),
            Fixed::from_int(3300),
            Fixed::from_ratio(98, 10),
        )
        .expect("the measured-solidus column resolves");
        assert!(
            (6.0..=7.0).contains(&column.crust_thickness_km.to_f64_lossy()),
            "the closure reproduces ~6.5 km, got {}",
            column.crust_thickness_km.to_f64_lossy()
        );
    }

    #[test]
    fn an_alien_endmember_set_is_a_data_row_not_a_code_path() {
        // Admit-the-alien: two fictional refractory endmembers (elements the silicate world never carries) run
        // through the SAME partial-melt mechanism, because it keys off each phase's own Endmember signature, not a
        // silicate mineral table. The alien crust is a data row.
        let floating = vec![one("Xx2O3(cr,alienite)"), one("YyO2(cr,alienon)")];
        let amounts: BTreeMap<String, Fixed> = floating
            .iter()
            .map(|(n, _)| (n.clone(), Fixed::ONE))
            .collect();
        let alien = |name: &str| -> Option<Endmember> {
            let formula = name.split('(').next()?;
            Some(match formula {
                "Xx2O3" => Endmember {
                    melting_point_k: Fixed::from_int(1600),
                    fusion_enthalpy_j_per_mol: Fixed::from_int(90_000),
                    fusion_volume_cm3_per_mol: Some(Fixed::from_int(4)),
                },
                "YyO2" => Endmember {
                    melting_point_k: Fixed::from_int(1400),
                    fusion_enthalpy_j_per_mol: Fixed::from_int(70_000),
                    fusion_volume_cm3_per_mol: Some(Fixed::from_int(5)),
                },
                _ => return None,
            })
        };
        let density = |_: &str| Some(Fixed::from_int(3));
        let params = MeltColumnParams {
            potential_temperature_k: Fixed::from_int(1450),
            ..normal_params()
        };
        let pm = partial_melt_crust_and_mantle(
            &floating,
            &amounts,
            alien,
            density,
            Fixed::ZERO,
            &params,
        )
        .expect("the alien pressure evidence is complete")
        .expect("the alien endmember set splits through the same mechanism");
        assert!(
            pm.used_partial_melt,
            "the alien endmember set runs the same partial-melt mechanism, no code path"
        );
        // The lower-melting YyO2 dominates the alien first melt (the fusible component), the same emergence.
        let crust_w = |formula: &str| {
            pm.crust
                .iter()
                .find(|(n, _)| n.starts_with(formula))
                .map(|(_, w)| w.to_f64_lossy())
                .unwrap_or(0.0)
        };
        assert!(
            crust_w("YyO2") > crust_w("Xx2O3"),
            "the lower-melting alien phase dominates its first melt, the same eutectic emergence"
        );
    }

    #[test]
    fn a_missing_melting_datum_falls_back_to_the_buoyancy_proxy() {
        // Fail-soft (named): an endmember lookup that knows forsterite but not enstatite (a missing melting datum)
        // falls back to the pure-buoyancy split, so the chain never aborts on incomplete data. The buoyancy crust
        // is the lighter enstatite, the mantle the denser forsterite, and no melt-column thickness is set.
        let floating = vec![one("Mg2SiO4(cr,forsterite)"), one("MgSiO3(cr,enstatite)")];
        let amounts: BTreeMap<String, Fixed> = floating
            .iter()
            .map(|(n, _)| (n.clone(), Fixed::ONE))
            .collect();
        let partial = |name: &str| -> Option<Endmember> {
            let formula = name.split('(').next()?;
            match formula {
                "Mg2SiO4" => Some(Endmember {
                    melting_point_k: Fixed::from_int(2163),
                    fusion_enthalpy_j_per_mol: Fixed::from_int(114_000),
                    fusion_volume_cm3_per_mol: Some(Fixed::from_ratio(39, 10)),
                }),
                _ => None, // enstatite's datum is missing
            }
        };
        let density = |name: &str| -> Option<Fixed> {
            Some(match name.split('(').next()? {
                "Mg2SiO4" => Fixed::from_ratio(327, 100),
                "MgSiO3" => Fixed::from_ratio(320, 100),
                _ => return None,
            })
        };
        let pm = partial_melt_crust_and_mantle(
            &floating,
            &amounts,
            partial,
            density,
            Fixed::ZERO,
            &normal_params(),
        )
        .expect("no pressure evaluation is needed after the missing datum")
        .expect("the fail-soft buoyancy split resolves");
        assert!(
            !pm.used_partial_melt,
            "a missing melting datum falls back to the buoyancy proxy"
        );
        assert!(
            pm.crust_thickness_km.is_none(),
            "the buoyancy fallback carries no melt-column thickness"
        );
        assert!(
            pm.crust.iter().any(|(n, _)| n.starts_with("MgSiO3")),
            "the lighter enstatite is the buoyancy crust"
        );
        assert!(
            pm.mantle.iter().any(|(n, _)| n.starts_with("Mg2SiO4")),
            "the denser forsterite is the buoyancy mantle"
        );
    }

    #[test]
    fn the_partial_melt_split_is_deterministic() {
        // The melt split replays byte-for-byte (fixed-point throughout, deterministic order), the canonical-path
        // discipline even though this chain is off the run path.
        let (floating, amounts) = fertile_floating();
        let density = |_: &str| Some(Fixed::from_int(3));
        let a = partial_melt_crust_and_mantle(
            &floating,
            &amounts,
            fertile_endmember,
            density,
            Fixed::ZERO,
            &normal_params(),
        );
        let b = partial_melt_crust_and_mantle(
            &floating,
            &amounts,
            fertile_endmember,
            density,
            Fixed::ZERO,
            &normal_params(),
        );
        assert_eq!(a, b, "the pressure refusal is deterministic");
    }
}
