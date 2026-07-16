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

//! THE YOUNG-THERMAL REGIME VERDICT (R-YOUNG-TEMPERATURE). A rocky world's formation either crosses its own
//! solidus or it does not, and the difference is the difference between a bumpy volcanic planet and a smooth
//! condensed ball. This module DERIVES that verdict per world and, for a melted world, the young potential
//! temperature the deep-time run starts from.
//!
//! THE ATTRACTOR LICENSE. The magma ocean is a fast attractor whose forgetting time (Myr-class beyond the
//! Hamano critical distance, up to the ~100 Myr desiccation window inside it) covers the whole impact series, so
//! any world that melts exits at the SAME solidus-pinned handoff regardless of whether its peak was 3000 K or
//! 6000 K. The handoff potential temperature is therefore PINNED at the world's own derived solidus plus the
//! rheological lock-up offset (the superheat at which the melt fraction reaches the critical lock-up fraction
//! phi_c), NOT integrated from an accretional energy budget. A binding-energy-first budget fails the alien test:
//! it assigns spuriously hot starts to slow-accreted small worlds where energy arrives in small packets and
//! radiates between deposits, so the deposit SPECTRUM matters, never the integral. The impact list, when it
//! lands, is the CHRONOMETER (the reset epoch t0) and the marginal-band resolver, never the source of the
//! handoff temperature.
//!
//! THREE BRANCHES. MELTED: the world crosses its solidus (short-lived-radionuclide heat on an early-formed body,
//! or giant impacts on an Earth-mass-and-up body), and the handoff is the solidus-pinned lock-up temperature.
//! NEVER-MELTED: a late-formed, slow-accreted, radionuclide-poor body whose cold-accretion budget stays
//! sub-solidus even at the top of the energy-retention band. MARGINAL: a near-degenerate world the retention
//! band can flip either way, carried and surfaced, never asserted, until the per-world impact list resolves it.
//!
//! THE MELT CRITERION is monotone in the retention efficiency eta (more retained energy is hotter), so a verdict
//! is GAPPED (decidable now) exactly when both ends of the reserved retention band agree, and MARGINAL when they
//! disagree. That makes the mandatory self-test structural: varying eta inside its band cannot flip a GAPPED
//! verdict ([`retention_band_cannot_flip_a_gapped_verdict`]).
//!
//! ALIEN-ADMISSIBLE BY DATA. The heat source is a birth-environment SLR-family DRAW (the isotope inventory, the
//! initial abundance ratios), the parent-element mass fractions are DERIVED from the world's own condensed
//! composition, the solidus is DERIVED from the world's own endmembers, and the rheology, heat capacity, and
//! retention band are reserved-with-basis per-world data. A photosynthetic mind's world, an iron world, a world
//! born in a supernova-enriched cloud with ten times the canonical 26Al: each is a different data row, never a
//! rewrite. The mechanism is fixed Rust; the numbers are the world's.

use civsim_core::Fixed;

/// Seconds in one Julian year (365.25 d), 31,556,952 s exactly. The megayear conversion the SLR budget needs is
/// applied as this factor times [`YEARS_PER_MYR`] so no single intermediate leaves the Q32.32 window: the
/// per-kilogram-of-isotope decay energy is ~10^12 J/kg and the bulk value ~10^4 J/kg, and only interleaving the
/// unit factors with the (tiny) isotope mass fractions keeps every running product inside the fixed-point range.
const SECONDS_PER_YEAR: Fixed = Fixed::from_int(31_556_952);

/// Years per megayear, the second half of the seconds-per-megayear conversion (see [`SECONDS_PER_YEAR`]).
const YEARS_PER_MYR: Fixed = Fixed::from_int(1_000_000);

/// The natural log of two, the half-life-to-mean-life divisor (`tau = t_half / ln 2`). Deterministic, from the
/// pinned integer-only [`Fixed::ln`].
fn ln_two() -> Fixed {
    Fixed::from_int(2).ln()
}

/// One short-lived radionuclide in the birth-environment SLR family. The heat source of a young rocky body is
/// this family DECAYED over the body's formation time. Every field is data: the specific power and half-life are
/// cited nuclear constants (Ruedas 2017), the initial abundance ratio is the birth-environment draw (canonical
/// solar-system value the default Mirror world uses, alien by a different draw), and the parent mass fraction is
/// DERIVED from the world's own condensed composition, so a metal-rich world carries more 60Fe fuel and an
/// aluminium-rich world more 26Al.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortLivedRadionuclide {
    /// The specific radiogenic heat production per kilogram of the ISOTOPE (W/kg). Cited: Ruedas 2017,
    /// Geochem. Geophys. Geosyst. 18, 3530, DOI 10.1002/2017GC006997, Table 2.
    pub specific_power_w_per_kg: Fixed,
    /// The isotope half-life (megayears). Cited: Ruedas 2017, DOI 10.1002/2017GC006997, Table 2.
    pub half_life_myr: Fixed,
    /// The mass fraction of the PARENT ELEMENT in the rock (kg of the element per kg of rock). DERIVED from the
    /// world's own condensed composition, never authored.
    pub parent_mass_fraction: Fixed,
    /// The initial atomic ratio of the radioisotope to its reference stable isotope (for example 26Al/27Al or
    /// 60Fe/56Fe). A BIRTH-ENVIRONMENT DRAW: the star-forming region's recent supernova/AGB enrichment sets it,
    /// so it is per-world data with the canonical solar-system value as the default, not a floor constant.
    pub isotope_number_ratio: Fixed,
    /// The ratio of the radioisotope mass number to the reference stable isotope's (26/27 for 26Al, 60/56 for
    /// 60Fe), converting the atomic ratio to a mass ratio.
    pub mass_number_ratio: Fixed,
}

/// The default solar-system SLR family (the Mirror-to-Earth birth-environment draw): 26Al and 60Fe, the two
/// radionuclides that melt early planetesimals and embryos. The specific powers and half-lives are the cited
/// Ruedas 2017 Table 2 values; the initial abundance ratios are the canonical solar-system values, reserved as a
/// per-world birth-environment draw with their basis. The parent (aluminium, iron) mass fractions are supplied
/// DERIVED from the world's condensed composition.
///
/// Reserved-with-basis (the birth-environment draw, defaulting to the canonical solar-system value, never
/// fabricated):
/// - `al26_over_al27` initial ratio, basis: the canonical CAI 26Al/27Al ~5.2e-5 (Lodders lineage / meteoritic
///   record); a per-cloud draw, higher in a freshly supernova-enriched region, lower in an evolved one.
/// - `fe60_over_fe56` initial ratio, basis: the solar-system initial 60Fe/56Fe, banded ~1e-8 (the fetch doc's
///   "60Fe subdominant and banded"); the least certain of the two and the most birth-environment-variable.
pub fn solar_system_slr_family(
    aluminium_mass_fraction: Fixed,
    iron_mass_fraction: Fixed,
    al26_over_al27: Fixed,
    fe60_over_fe56: Fixed,
) -> [ShortLivedRadionuclide; 2] {
    [
        // 26Al: specific power 0.3583 W/kg, half-life 0.717 Myr (Ruedas 2017, Table 2). Reference stable
        // isotope 27Al (aluminium is monoisotopic), mass-number ratio 26/27.
        ShortLivedRadionuclide {
            specific_power_w_per_kg: Fixed::from_ratio(3583, 10_000),
            half_life_myr: Fixed::from_ratio(717, 1000),
            parent_mass_fraction: aluminium_mass_fraction,
            isotope_number_ratio: al26_over_al27,
            mass_number_ratio: Fixed::from_ratio(26, 27),
        },
        // 60Fe: specific power 3.6579e-2 W/kg, half-life 2.62 Myr (Ruedas 2017, Table 2). Reference stable
        // isotope 56Fe (the dominant iron isotope), mass-number ratio 60/56; the 56Fe abundance fraction is
        // folded into the reserved ratio's basis (a first-grade approximation for a subdominant contributor).
        ShortLivedRadionuclide {
            specific_power_w_per_kg: Fixed::from_ratio(36_579, 1_000_000),
            half_life_myr: Fixed::from_ratio(262, 100),
            parent_mass_fraction: iron_mass_fraction,
            isotope_number_ratio: fe60_over_fe56,
            mass_number_ratio: Fixed::from_ratio(60, 56),
        },
    ]
}

/// The three-branch young-thermal regime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YoungThermalRegime {
    /// The formation crosses the solidus: a super-solidus start, a nonzero melt fraction, and volcanic relief.
    Melted,
    /// The cold-accretion budget stays sub-solidus even at the top of the retention band: a smooth condensed ball.
    NeverMelted,
    /// Near-degenerate by the Gap Law: the retention band can flip the outcome, carried until the impact list resolves it.
    Marginal,
}

/// Whether the young-thermal inputs are the world's own draw or a class-grade interim band, so a downstream
/// written-state row can carry the provenance and be selectively recomputed when the per-world impact list lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalProvenance {
    /// The reset epoch t0 and the formation time are class-grade interim bands (the assembly statistics), not the
    /// world's own draw. The SCOPE FENCE binds the interim to TEXTURE ONSET and THERMAL INITIAL CONDITIONS only:
    /// it must never synthesize impact-coupled archives (spin or obliquity resets, per-event atmosphere blow-off
    /// history, late-veneer chemistry, or event chronology), which the impact list uniquely provides.
    Interim,
    /// The world's own assembly draw resolved the reset epoch and formation time (the post-impact-list grade).
    WorldDraw,
}

/// The rheological lifetime class of the magma ocean, the per-volatile outgoing-radiation behaviour of the
/// outgassed blanket (Lichtenberg et al. 2021). It sets how long the world stays molten (the forgetting time /
/// reset epoch), never the handoff temperature. A data-driven classification: the volatile set grows with the
/// world, and the H2O 282.5 W/m^2 tropospheric limit is an Earth VALIDATION ANCHOR, not the mechanism.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MagmaOceanLifetime {
    /// A weak greenhouse (CO, O2, N2): the planet radiates freely and reaches the rheological transition in
    /// ~10^3 yr, fully solidifying in ~1 Myr.
    Fast,
    /// An intermediate blanket (H2O, CO2, CH4): the rheological transition in ~10^4 yr, full solidification ~1 Myr.
    Intermediate,
    /// A strong, non-condensable blanket (H2): the interior stays partially molten past 100 Myr.
    Extended,
}

/// The per-volatile magma-ocean lifetime class for the dominant outgassed volatile (Lichtenberg et al. 2021,
/// JGR Planets 126, e2020JE006711, their Figure 5 and surrounding text). Data-driven (a match the world's own
/// blanket set extends), fail-soft to [`MagmaOceanLifetime::Intermediate`] for an unlisted volatile (the H2O/CO2
/// grade). The H2O tropospheric limit of 282.5 W/m^2 is the reference anchor; the non-condensable blankets do
/// not saturate to a single flux, so the class reads their OLR behaviour, never one ceiling number.
pub fn magma_ocean_lifetime_for_blanket(dominant_volatile: &str) -> MagmaOceanLifetime {
    match dominant_volatile {
        "H2" => MagmaOceanLifetime::Extended,
        "H2O" | "CO2" | "CH4" => MagmaOceanLifetime::Intermediate,
        "CO" | "O2" | "N2" => MagmaOceanLifetime::Fast,
        _ => MagmaOceanLifetime::Intermediate,
    }
}

/// The magma-ocean handoff potential temperature: the world's own derived solidus plus the superheat at which
/// the adiabatic decompression melt column reaches the rheological lock-up fraction phi_c. This INVERTS the
/// banked [`adiabatic_melt_column`]: that column's peak melt fraction is
/// `F = (dF/dP) * (T_p - T_sol0) / (m_sol - m_ad)`, so pinning `F = phi_c` gives
/// `T_p = T_sol0 + phi_c * (m_sol - m_ad) / (dF/dP)`. Feeding this temperature back into the same column
/// reproduces `F = phi_c` by construction, so a melted world forms exactly at its own lock-up boundary: the
/// physical meaning of the magma ocean freezing to a crystal framework as it cools. `None` if the solidus does
/// not rise faster than the adiabat (no decompression column forms) or an input is non-physical.
pub fn magma_ocean_handoff_temperature(
    solidus_surface_k: Fixed,
    solidus_slope_k_per_gpa: Fixed,
    adiabat_slope_k_per_gpa: Fixed,
    productivity_per_gpa: Fixed,
    lockup_melt_fraction: Fixed,
) -> Option<Fixed> {
    if productivity_per_gpa <= Fixed::ZERO || lockup_melt_fraction <= Fixed::ZERO {
        return None;
    }
    let slope_diff = solidus_slope_k_per_gpa.checked_sub(adiabat_slope_k_per_gpa)?;
    if slope_diff <= Fixed::ZERO {
        return None;
    }
    let superheat = lockup_melt_fraction
        .checked_mul(slope_diff)?
        .checked_div(productivity_per_gpa)?;
    solidus_surface_k.checked_add(superheat)
}

/// The temperature rise from a short-lived-radionuclide family decayed over the body's formation time. For each
/// isotope the bulk decay energy per unit mass is `H0 * tau` (the specific power at CAI integrated to full decay
/// from the formation time onward), decayed by `exp(-t_form / tau)`, over the specific heat. The mean life is
/// `tau = t_half / ln 2`. Order of operations keeps every intermediate inside the Q32.32 window: the large
/// unit-conversion factors (seconds per year, then years per megayear) are interleaved with the small isotope
/// mass fractions so neither the per-isotope decay energy (~10^12 J/kg) nor the bulk power (~10^-7 W/kg) is ever
/// materialized alone. An isotope so decayed that `t_form / tau > 22` contributes zero ([`Fixed::exp`] saturates
/// to zero there), the honest Q32.32 read of a dead radionuclide. `None` on a non-physical heat capacity or an
/// arithmetic failure.
pub fn slr_family_temperature_rise(
    family: &[ShortLivedRadionuclide],
    formation_time_myr: Fixed,
    specific_heat_j_per_kg_k: Fixed,
) -> Option<Fixed> {
    if specific_heat_j_per_kg_k <= Fixed::ZERO {
        return None;
    }
    let ln2 = ln_two();
    let mut total = Fixed::ZERO;
    for iso in family {
        if iso.half_life_myr <= Fixed::ZERO {
            continue;
        }
        let tau_myr = iso.half_life_myr.checked_div(ln2)?;
        if tau_myr <= Fixed::ZERO {
            continue;
        }
        // The decay factor exp(-t_form / tau). A negative-past-range exponent saturates to zero: a dead isotope.
        let ratio = formation_time_myr.checked_div(tau_myr)?;
        let decay = Fixed::ZERO.checked_sub(ratio)?.exp();
        // The decay energy per kilogram of rock over the specific heat, interleaving the unit factors with the
        // mass fractions so no running product leaves the fixed-point range (see the function note).
        let dt = iso
            .specific_power_w_per_kg
            .checked_mul(SECONDS_PER_YEAR)?
            .checked_mul(tau_myr)?
            .checked_mul(iso.parent_mass_fraction)?
            .checked_mul(iso.isotope_number_ratio)?
            .checked_mul(iso.mass_number_ratio)?
            .checked_mul(YEARS_PER_MYR)?
            .checked_mul(decay)?
            .checked_div(specific_heat_j_per_kg_k)?;
        total = total.checked_add(dt)?;
    }
    Some(total)
}

/// The specific accretional energy retained as heat, `k_geom * g * R`, the gravitational binding energy per unit
/// mass of a body of surface gravity `g` and radius `R` scaled by the geometry factor `k_geom` (3/5 for a uniform
/// sphere, `(3/5) G M / R = (3/5) g R`). Expressed through the DERIVED surface gravity and radius so no
/// planet-mass-in-kilograms (which overflows Q32.32) is ever formed. The retained fraction (the energy-retention
/// efficiency eta) multiplies this in the cold-accretion budget. `None` on an arithmetic failure.
pub fn accretional_specific_energy(
    binding_energy_geometry: Fixed,
    surface_gravity_m_per_s2: Fixed,
    radius_m: Fixed,
) -> Option<Fixed> {
    binding_energy_geometry
        .checked_mul(surface_gravity_m_per_s2)?
        .checked_mul(radius_m)
}

/// The reserved-with-basis and derived inputs the young-thermal verdict reads. The solidus is derived from the
/// world's own endmembers; the gravity, radius, and parent mass fractions are derived from the world's own
/// structure and composition; the rheology, heat capacity, retention band, formation time, and reset epoch are
/// reserved-with-basis per-world data.
#[derive(Clone, Copy, Debug)]
pub struct YoungThermalInputs {
    /// The world's own DERIVED mantle solidus surface temperature (K).
    pub solidus_surface_k: Fixed,
    /// The world's own DERIVED mantle solidus slope (K/GPa).
    pub solidus_slope_k_per_gpa: Fixed,
    /// The mantle adiabat slope (K/GPa). Reserved-with-basis (derives from the assemblage's thermal expansion,
    /// density, and heat capacity once the petrology substrate supplies them).
    pub adiabat_slope_k_per_gpa: Fixed,
    /// The isentropic melting productivity dF/dP (per GPa). Reserved-with-basis (derives from the entropy of
    /// fusion and heat capacity once those land).
    pub productivity_per_gpa: Fixed,
    /// The rheological critical (lock-up) melt fraction phi_c. Reserved-with-basis, data-driven: ~0.4 default
    /// (Abe 1993; Solomatov 2015), shape- and polydispersity-dependent, alien-admissible.
    pub lockup_melt_fraction: Fixed,
    /// The specific heat capacity (J/(kg*K)). Reserved-with-basis (silicate-mantle value, derives from the
    /// assemblage's heat capacity once that lands).
    pub specific_heat_j_per_kg_k: Fixed,
    /// The DERIVED pre-heating reference temperature at the orbit (K), the cold-accretion baseline the budgets add to.
    pub reference_temperature_k: Fixed,
    /// The DERIVED surface gravity (m/s^2).
    pub surface_gravity_m_per_s2: Fixed,
    /// The DERIVED planet radius (m).
    pub radius_m: Fixed,
    /// The accretional-energy geometry factor `k_geom`. Reserved-with-basis: 3/5, the gravitational binding
    /// energy per unit mass of a uniform sphere.
    pub binding_energy_geometry: Fixed,
    /// The low edge of the reserved energy-retention efficiency band (dimensionless, a PREFACTOR entry). Basis:
    /// the cold-accretion retention literature (the fraction of accretional/impact energy retained as heat rather
    /// than radiated between deposits); reserved, never fabricated.
    pub retention_efficiency_lo: Fixed,
    /// The high edge of the reserved energy-retention efficiency band.
    pub retention_efficiency_hi: Fixed,
    /// The body's formation time relative to CAI (megayears), the SLR decay clock. Reserved-with-basis interim
    /// band: the oligarchic isolation-mass growth time at the orbit (sub-megayear for the inner disk), a birth /
    /// assembly draw, upgraded to the world's own draw when the impact list lands.
    pub formation_time_myr: Fixed,
    /// The DERIVED (or reserved-interim) planet mass in Earth masses, for the giant-impact class gate.
    pub planet_mass_earth: Fixed,
    /// The mass (Earth masses) at and above which giant impacts are class-generic. Reserved-with-basis: the
    /// Earth-mass terrestrial-embryo-merger regime where the assembly statistics guarantee ~two dozen giant
    /// impacts per system.
    pub giant_impact_mass_threshold_earth: Fixed,
    /// The interim reset epoch t0 (megayears), the young-clock zero. Reserved-with-basis: the last-giant-impact
    /// time in the standard assembly ensembles (73 +/- 74 Myr), tagged interim.
    pub reset_epoch_myr: Fixed,
    /// The half-width of the interim reset-epoch band (megayears).
    pub reset_epoch_half_band_myr: Fixed,
}

/// The young-thermal verdict: the regime, whether it is decidable now (GAPPED) or impact-list-pending (MARGINAL),
/// the young potential temperature the deep-time run starts from, and the diagnostics the reversibility discipline
/// and the self-test read.
#[derive(Clone, Copy, Debug)]
pub struct YoungThermalVerdict {
    /// The three-branch regime.
    pub regime: YoungThermalRegime,
    /// True when the regime cannot flip within the reserved retention band (or is decided by the class-generic
    /// giant-impact gate): decidable without the impact list. False for a MARGINAL world.
    pub gapped: bool,
    /// The young potential temperature (K) the deep-time province run starts from: the solidus-pinned handoff for
    /// a MELTED world, the cold peak for a NEVER-MELTED world, and the carried lower branch for a MARGINAL world
    /// (never asserted hot until the impact list resolves it).
    pub young_potential_temperature_k: Fixed,
    /// The solidus-pinned lock-up handoff (K), `Some` iff the regime is MELTED and the decompression column
    /// resolves.
    pub handoff_potential_temperature_k: Option<Fixed>,
    /// The SLR-family temperature rise (K), the eta-independent part of the budget.
    pub slr_temperature_rise_k: Fixed,
    /// The cold peak temperature (K) at the low edge of the retention band.
    pub cold_peak_temperature_k_lo: Fixed,
    /// The cold peak temperature (K) at the high edge of the retention band.
    pub cold_peak_temperature_k_hi: Fixed,
    /// The provenance of the interim inputs (the reset epoch and formation time), for the selective recompute.
    pub provenance: ThermalProvenance,
    /// The interim reset epoch t0 (megayears), carried for the deep-time clock zero.
    pub reset_epoch_myr: Fixed,
    /// The half-width of the interim reset-epoch band (megayears).
    pub reset_epoch_half_band_myr: Fixed,
}

/// The young-thermal peak temperature at a given retention efficiency: `T_ref + dT_slr + eta * u_acc / c_p`. It
/// is monotone non-decreasing in eta (more retained accretional energy is hotter), which is what makes the GAPPED
/// verdict structural: both ends of the retention band agreeing means no interior eta can disagree.
fn peak_temperature_at(
    reference_temperature_k: Fixed,
    slr_rise_k: Fixed,
    accretional_energy: Fixed,
    retention_efficiency: Fixed,
    specific_heat_j_per_kg_k: Fixed,
) -> Option<Fixed> {
    let cold_rise = retention_efficiency
        .checked_mul(accretional_energy)?
        .checked_div(specific_heat_j_per_kg_k)?;
    reference_temperature_k
        .checked_add(slr_rise_k)?
        .checked_add(cold_rise)
}

/// The young-thermal regime verdict for a world. Combines the SLR-family heat (decayed over the formation time),
/// the cold-accretion budget (over the reserved retention band), and the class-generic giant-impact gate, all
/// against the world's own derived solidus, and pins the melted handoff at the solidus-plus-lock-up temperature.
///
/// The decision is monotone in the retention efficiency eta, so: MELTED and GAPPED when the world melts even at
/// the low edge of the band (or is Earth-mass-and-up); NEVER-MELTED and GAPPED when it stays sub-solidus even at
/// the high edge; MARGINAL (not gapped) when the band straddles the solidus. `None` on an arithmetic failure or a
/// non-physical input.
pub fn young_thermal_verdict(
    inputs: &YoungThermalInputs,
    slr_family: &[ShortLivedRadionuclide],
) -> Option<YoungThermalVerdict> {
    let slr_rise = slr_family_temperature_rise(
        slr_family,
        inputs.formation_time_myr,
        inputs.specific_heat_j_per_kg_k,
    )?;
    let accretional_energy = accretional_specific_energy(
        inputs.binding_energy_geometry,
        inputs.surface_gravity_m_per_s2,
        inputs.radius_m,
    )?;
    let peak_lo = peak_temperature_at(
        inputs.reference_temperature_k,
        slr_rise,
        accretional_energy,
        inputs.retention_efficiency_lo,
        inputs.specific_heat_j_per_kg_k,
    )?;
    let peak_hi = peak_temperature_at(
        inputs.reference_temperature_k,
        slr_rise,
        accretional_energy,
        inputs.retention_efficiency_hi,
        inputs.specific_heat_j_per_kg_k,
    )?;
    let handoff = magma_ocean_handoff_temperature(
        inputs.solidus_surface_k,
        inputs.solidus_slope_k_per_gpa,
        inputs.adiabat_slope_k_per_gpa,
        inputs.productivity_per_gpa,
        inputs.lockup_melt_fraction,
    );
    let melts_lo = peak_lo >= inputs.solidus_surface_k;
    let melts_hi = peak_hi >= inputs.solidus_surface_k;
    let giant_impact_class = inputs.planet_mass_earth >= inputs.giant_impact_mass_threshold_earth;

    let (regime, gapped, young_t) = if giant_impact_class || melts_lo {
        // Melts even at the low edge of the retention band, or is Earth-mass-and-up (giant impacts class-generic):
        // robustly MELTED, and the retention band cannot flip it (monotone: hotter at higher eta).
        let young_t = handoff.unwrap_or(inputs.solidus_surface_k);
        (YoungThermalRegime::Melted, true, young_t)
    } else if !melts_hi {
        // Stays sub-solidus even at the high edge: robustly NEVER-MELTED, the retention band cannot flip it.
        (YoungThermalRegime::NeverMelted, true, peak_hi)
    } else {
        // The band straddles the solidus: MARGINAL, carried at the cold lower branch (never asserted hot) until
        // the per-world impact list resolves it.
        (YoungThermalRegime::Marginal, false, peak_lo)
    };

    Some(YoungThermalVerdict {
        regime,
        gapped,
        young_potential_temperature_k: young_t,
        handoff_potential_temperature_k: if regime == YoungThermalRegime::Melted {
            handoff
        } else {
            None
        },
        slr_temperature_rise_k: slr_rise,
        cold_peak_temperature_k_lo: peak_lo,
        cold_peak_temperature_k_hi: peak_hi,
        provenance: ThermalProvenance::Interim,
        reset_epoch_myr: inputs.reset_epoch_myr,
        reset_epoch_half_band_myr: inputs.reset_epoch_half_band_myr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::melting::adiabatic_melt_column;

    /// A representative refractory-world input set: a ~1680 K derived solidus (the solar-condensed refractory
    /// set), a 130 K/GPa slope, the reserved McKenzie-Bickle rheology, a phi_c of 0.4, an ~1000 J/(kg*K) heat
    /// capacity, a ~279 K reference, a Mars-class gravity and radius, and the reserved retention band. The
    /// aluminium and iron mass fractions and the SLR ratios are the canonical solar-system draw. The formation
    /// time is the free parameter each test sets.
    fn base_inputs(formation_time_myr: Fixed, mass_earth: Fixed) -> YoungThermalInputs {
        YoungThermalInputs {
            solidus_surface_k: Fixed::from_int(1680),
            solidus_slope_k_per_gpa: Fixed::from_int(130),
            adiabat_slope_k_per_gpa: Fixed::from_ratio(155, 10),
            productivity_per_gpa: Fixed::from_ratio(12, 100),
            lockup_melt_fraction: Fixed::from_ratio(4, 10),
            specific_heat_j_per_kg_k: Fixed::from_int(1000),
            reference_temperature_k: Fixed::from_int(279),
            surface_gravity_m_per_s2: Fixed::from_ratio(37, 10),
            radius_m: Fixed::from_int(3_390_000),
            binding_energy_geometry: Fixed::from_ratio(3, 5),
            retention_efficiency_lo: Fixed::from_ratio(1, 100),
            retention_efficiency_hi: Fixed::from_ratio(4, 10),
            formation_time_myr,
            planet_mass_earth: mass_earth,
            giant_impact_mass_threshold_earth: Fixed::ONE,
            reset_epoch_myr: Fixed::from_int(73),
            reset_epoch_half_band_myr: Fixed::from_int(74),
        }
    }

    /// The canonical solar-system SLR family with realistic derived mass fractions: aluminium ~2 wt% of the
    /// silicate, iron ~19 wt% of the bulk, the canonical 26Al/27Al ~5.2e-5 and 60Fe/56Fe ~1e-8.
    fn canonical_family() -> [ShortLivedRadionuclide; 2] {
        solar_system_slr_family(
            Fixed::from_ratio(2, 100),
            Fixed::from_ratio(19, 100),
            Fixed::from_ratio(52, 1_000_000),
            Fixed::from_ratio(1, 100_000_000),
        )
    }

    #[test]
    fn the_handoff_pins_the_lock_up_melt_fraction() {
        // The mechanism's contract: feeding the handoff back into the banked adiabatic melt column reproduces the
        // lock-up melt fraction phi_c. This is why a melted world forms exactly super-solidus with F = phi_c.
        let solidus = Fixed::from_int(1680);
        let slope = Fixed::from_int(130);
        let adiabat = Fixed::from_ratio(155, 10);
        let productivity = Fixed::from_ratio(12, 100);
        let phi_c = Fixed::from_ratio(4, 10);
        let handoff =
            magma_ocean_handoff_temperature(solidus, slope, adiabat, productivity, phi_c).unwrap();
        assert!(
            handoff > solidus,
            "the handoff {} K is super-solidus (above {} K)",
            handoff.to_f64_lossy(),
            solidus.to_f64_lossy()
        );
        let column = adiabatic_melt_column(
            handoff,
            solidus,
            slope,
            adiabat,
            productivity,
            Fixed::from_int(3300),
            Fixed::from_ratio(37, 10),
        )
        .unwrap();
        let f = column.max_melt_fraction.to_f64_lossy();
        assert!(
            (f - 0.4).abs() < 0.01,
            "the column fed the handoff reproduces F = phi_c = 0.4, got {f}"
        );
    }

    #[test]
    fn an_early_formed_body_is_gapped_melted_by_slr_heat() {
        // A body forming within the 26Al window (~1 Myr) melts from short-lived-radionuclide heat alone, so it is
        // MELTED and GAPPED (the retention band cannot un-melt it: the SLR rise is eta-independent and already
        // crosses the solidus).
        let inputs = base_inputs(Fixed::ONE, Fixed::from_ratio(1, 10));
        let verdict = young_thermal_verdict(&inputs, &canonical_family()).unwrap();
        assert_eq!(verdict.regime, YoungThermalRegime::Melted);
        assert!(verdict.gapped, "an SLR-melted world is decidable now");
        assert!(
            verdict.slr_temperature_rise_k > Fixed::from_int(1000),
            "the 26Al rise at 1 Myr is thousands of K, got {} K",
            verdict.slr_temperature_rise_k.to_f64_lossy()
        );
        let young_t = verdict.young_potential_temperature_k;
        assert!(
            young_t > inputs.solidus_surface_k,
            "the young potential temperature {} K is super-solidus",
            young_t.to_f64_lossy()
        );
    }

    #[test]
    fn a_late_formed_slr_poor_body_is_never_melted_or_marginal_not_hot() {
        // A body forming at 73 Myr (the interim reset epoch, tens of mean lives after CAI) has no live 26Al: the
        // SLR rise is negligible and the verdict is decided by the cold-accretion budget. For a Mars-class body
        // this stays sub-solidus (or straddles the band), never a fabricated hot start.
        let inputs = base_inputs(Fixed::from_int(73), Fixed::from_ratio(1, 10));
        let verdict = young_thermal_verdict(&inputs, &canonical_family()).unwrap();
        assert!(
            verdict.slr_temperature_rise_k < Fixed::ONE,
            "the SLR rise at 73 Myr is dead, got {} K",
            verdict.slr_temperature_rise_k.to_f64_lossy()
        );
        assert_ne!(
            verdict.regime,
            YoungThermalRegime::Melted,
            "a late-formed Mars-class body is not SLR-melted"
        );
        assert!(
            verdict.young_potential_temperature_k <= inputs.solidus_surface_k,
            "the carried young temperature is not asserted super-solidus"
        );
    }

    #[test]
    fn an_earth_mass_body_is_gapped_melted_by_giant_impacts() {
        // An Earth-mass-and-up body is MELTED and GAPPED by the class-generic giant-impact gate, independent of
        // the formation time (even late-formed, the ~two-dozen giant impacts re-melt it).
        let inputs = base_inputs(Fixed::from_int(100), Fixed::ONE);
        let verdict = young_thermal_verdict(&inputs, &canonical_family()).unwrap();
        assert_eq!(verdict.regime, YoungThermalRegime::Melted);
        assert!(verdict.gapped);
        assert!(verdict.handoff_potential_temperature_k.is_some());
    }

    #[test]
    fn retention_band_cannot_flip_a_gapped_verdict() {
        // THE MANDATORY SELF-TEST. For every world, sweeping the energy-retention efficiency across its full
        // reserved band must not flip a verdict tagged GAPPED. A verdict that CAN flip is marginal and must be
        // tagged MARGINAL instead. Swept over a grid of formation times and masses spanning all three branches.
        let masses = [
            Fixed::from_ratio(1, 10),
            Fixed::from_ratio(5, 10),
            Fixed::ONE,
            Fixed::from_int(3),
        ];
        let formation_times = [
            Fixed::from_ratio(5, 10),
            Fixed::ONE,
            Fixed::from_int(2),
            Fixed::from_int(3),
            Fixed::from_int(5),
            Fixed::from_int(20),
            Fixed::from_int(73),
        ];
        // The sweep points across the retention band [lo, hi] the self-test probes at.
        let band_lo = Fixed::from_ratio(1, 100);
        let band_hi = Fixed::from_ratio(4, 10);
        let sweep = [
            band_lo,
            Fixed::from_ratio(5, 100),
            Fixed::from_ratio(1, 10),
            Fixed::from_ratio(2, 10),
            Fixed::from_ratio(3, 10),
            band_hi,
        ];
        for &mass in &masses {
            for &t_form in &formation_times {
                let base = base_inputs(t_form, mass);
                let verdict = young_thermal_verdict(&base, &canonical_family()).unwrap();
                if !verdict.gapped {
                    continue; // a MARGINAL verdict is allowed to flip; that is what MARGINAL means
                }
                for &eta in &sweep {
                    let probed = YoungThermalInputs {
                        retention_efficiency_lo: eta,
                        retention_efficiency_hi: eta,
                        ..base
                    };
                    let probed_verdict =
                        young_thermal_verdict(&probed, &canonical_family()).unwrap();
                    assert_eq!(
                        probed_verdict.regime, verdict.regime,
                        "a GAPPED verdict ({:?}) flipped to {:?} at eta = {} (mass {} M_earth, t_form {} Myr): \
                         it was marginal and mis-tagged",
                        verdict.regime,
                        probed_verdict.regime,
                        eta.to_f64_lossy(),
                        mass.to_f64_lossy(),
                        t_form.to_f64_lossy(),
                    );
                }
            }
        }
    }

    #[test]
    fn a_marginal_world_exists_and_is_not_gapped() {
        // A world whose cold-accretion budget straddles the solidus across the retention band is MARGINAL and not
        // gapped: the band flips it, so it is carried until the impact list resolves it. Constructed by choosing a
        // solidus that sits between the low-edge and high-edge cold peaks of a late-formed body.
        let mut inputs = base_inputs(Fixed::from_int(50), Fixed::from_ratio(3, 10));
        // Late formation (no SLR); the cold peaks straddle a solidus placed between them.
        let family = canonical_family();
        let slr = slr_family_temperature_rise(
            &family,
            inputs.formation_time_myr,
            inputs.specific_heat_j_per_kg_k,
        )
        .unwrap();
        let acc = accretional_specific_energy(
            inputs.binding_energy_geometry,
            inputs.surface_gravity_m_per_s2,
            inputs.radius_m,
        )
        .unwrap();
        let lo = peak_temperature_at(
            inputs.reference_temperature_k,
            slr,
            acc,
            inputs.retention_efficiency_lo,
            inputs.specific_heat_j_per_kg_k,
        )
        .unwrap();
        let hi = peak_temperature_at(
            inputs.reference_temperature_k,
            slr,
            acc,
            inputs.retention_efficiency_hi,
            inputs.specific_heat_j_per_kg_k,
        )
        .unwrap();
        assert!(hi > lo, "the high-eta peak exceeds the low-eta peak");
        // Put the solidus strictly between the two cold peaks.
        let mid = lo
            .checked_add(hi)
            .unwrap()
            .checked_div(Fixed::from_int(2))
            .unwrap();
        inputs.solidus_surface_k = mid;
        let verdict = young_thermal_verdict(&inputs, &family).unwrap();
        assert_eq!(verdict.regime, YoungThermalRegime::Marginal);
        assert!(!verdict.gapped, "a band-straddling world is not gapped");
    }

    #[test]
    fn the_slr_budget_stays_in_the_fixed_point_range() {
        // The unit-conversion ordering must not overflow (26Al, the dominant term) or underflow to break the
        // family (60Fe, the tiny subdominant term). At CAI formation the 26Al rise is thousands of K and finite,
        // and the family total is a valid non-negative number.
        let family = canonical_family();
        let dt = slr_family_temperature_rise(&family, Fixed::ZERO, Fixed::from_int(1000)).unwrap();
        let k = dt.to_f64_lossy();
        assert!(
            k > 5000.0 && k < 100_000.0,
            "the CAI-formation SLR rise is a large finite temperature, got {k} K"
        );
    }

    #[test]
    fn the_magma_ocean_lifetime_reads_the_blanket_per_volatile() {
        // The Lichtenberg per-volatile classification: H2 keeps a world molten past 100 Myr, water-class blankets
        // are intermediate, and the freely-radiating blankets are fast.
        assert_eq!(
            magma_ocean_lifetime_for_blanket("H2"),
            MagmaOceanLifetime::Extended
        );
        assert_eq!(
            magma_ocean_lifetime_for_blanket("H2O"),
            MagmaOceanLifetime::Intermediate
        );
        assert_eq!(
            magma_ocean_lifetime_for_blanket("N2"),
            MagmaOceanLifetime::Fast
        );
    }
}
