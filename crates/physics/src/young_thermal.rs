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
//! THE MELT CRITERION is monotone in each banded budget input: hotter at higher retention efficiency eta, hotter
//! at a shorter formation time (less short-lived-radionuclide decay), hotter at a larger initial SLR draw, and
//! (through the giant-impact gate) melted at a larger mass. So a verdict is GAPPED (decidable given these inputs)
//! exactly when the COLDEST corner and the HOTTEST corner of the joint band agree, and MARGINAL when they
//! disagree. The grade is NOT a one-axis read of the retention band alone: a world whose point formation time
//! makes the SLR heat dominant can look robustly melted while a formation time slower within its own reserved
//! interim band lets it fall to the sub-solidus branch, so keying the tag on any single axis over-claims. The
//! verdict therefore band-sweeps every input carried as an interim/reserved band (retention, formation time, the
//! SLR draw, mass) and demotes GAPPED to MARGINAL on any flip ([`young_thermal_verdict`], the self-test
//! `retention_band_cannot_flip_a_gapped_verdict`). A world that melts at its best point estimate but flips
//! across the interim band is carried MARGINAL at that best estimate (never asserted GAPPED, and not forced to the
//! cold branch either), pending the per-world impact list that collapses the bands to the world's own draw.
//!
//! THE PROVENANCE-DAG-WALK FOLLOW-ON. The general containment rule is that no input carried as a band (interim /
//! `[E]` prefactor / `[C]` closure) may flip a GAPPED verdict. This module hard-codes the four banded axes that reach
//! the melt budget today (retention, formation time, the SLR draw, mass). The standing-machinery form is a
//! GAPPED-tag writer that WALKS the verdict's provenance DAG and band-sweeps every ancestor tagged interim / `[E]` /
//! `[C]` automatically, so a new banded input cannot silently sneak past the containment rule without a panel. That
//! generalization is the flagged follow-on; this pass sweeps the four named axes explicitly.
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
/// FOLLOW-ON, THE CLOSED PAIR (the alien-admitting seam). This constructor hard-codes exactly `[26Al, 60Fe]`,
/// the correct pair for the Mirror default, and the caller reads the two parent elements (aluminium, iron)
/// explicitly. The consuming mechanism ([`slr_family_temperature_rise`], [`young_thermal_verdict`]) already takes
/// an OPEN `&[ShortLivedRadionuclide]` slice, so only this constructor and its parent-element reads are closed. The
/// general form is a data-driven SLR REGISTRY (sibling to the species set), the isotope inventory READ per birth
/// environment with each isotope's specific power, half-life, reference isotope, and parent element cited, so a
/// world born in an r-process-enriched cloud with live 244Pu or 182Hf, or one poor in 26Al, is a data row rather
/// than a code edit. Flagged here; the solar pair is not fabricated (it is the cited Mirror default).
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
/// banked [`crate::melting::adiabatic_melt_column`]: that column's peak melt fraction is
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

/// The DERIVED specific heat capacity `c_p` (J/(kg*K)) of an assemblage at magma-ocean temperature, from the
/// Dulong-Petit law on the world's OWN mean atomic mass: each mole of ATOMS contributes `3R` to the molar heat
/// capacity, so the specific heat is `c_p = 3R / <m_atom>` where `<m_atom>` is the mean atomic molar mass
/// (kg/mol). `R` is the cited molar gas constant (`civsim_units::fundamentals::GAS_CONSTANT`, itself the exact
/// product `N_A * k_B`). This is a `[D]`-form derivation with ZERO new reserved rows: it reads only the
/// assemblage's composition, so an iron world (mean atomic mass ~0.0559 kg/mol, `c_p ~447 J/(kg*K)`) and a
/// forsterite-class silicate world (mean atomic mass ~0.0201 kg/mol, `c_p ~1240 J/(kg*K)`) each judge their own
/// melt budget by construction, both reproducing the measured high-temperature specific heats. This `c_p` is
/// LOAD-BEARING: it divides both the SLR rise and the cold-accretion rise, the whole budget the regime decision
/// rests on, so an iron world at ~447 rather than a silicate-fixed ~1000 heats ~2.2x more for the same energy and
/// can flip regime, which is why it is derived rather than authored. `None` on a non-positive mean atomic mass or
/// an arithmetic failure.
///
/// HONEST LIMIT (the Debye follow-on): Dulong-Petit is the high-temperature (`T` above the Debye temperature)
/// plateau, which the magma-ocean operating point sits on for silicates and iron. Below the Debye temperature the
/// specific heat falls off as the Debye integral, a banded correction keyed on the assemblage's own Debye
/// temperature; that correction is the flagged follow-on, not needed at the melt-decision temperature.
// @derives: the high-temperature specific heat <- mean atomic mass (Dulong-Petit)
pub fn dulong_petit_specific_heat(mean_atomic_mass_kg_per_mol: Fixed) -> Option<Fixed> {
    if mean_atomic_mass_kg_per_mol <= Fixed::ZERO {
        return None;
    }
    // 3R, from the molar gas constant DERIVED from the register fundamentals (R = N_A * k_B), never an authored
    // decimal: the same floor-read the gas thermochemistry uses.
    let r = crate::gas_thermochemistry::molar_gas_constant()?;
    let three_r = r.checked_mul(Fixed::from_int(3))?;
    three_r.checked_div(mean_atomic_mass_kg_per_mol)
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
    /// The body's formation time relative to CAI (megayears), the SLR decay clock, the POINT best estimate the
    /// reported SLR rise and the carried initial condition read. Reserved-with-basis interim band: the oligarchic
    /// isolation-mass growth time at the orbit (sub-megayear for the inner disk), a birth / assembly draw, upgraded
    /// to the world's own draw when the impact list lands.
    pub formation_time_myr: Fixed,
    /// The SHORT (fast-accretion) edge of the interim formation-time band (megayears), the HOTTEST-corner input
    /// (least SLR decay). The band is swept for the GAPPED / MARGINAL grade; set equal to `formation_time_myr` to
    /// assert the formation time is known (a point band, no formation-time contingency).
    pub formation_time_myr_lo: Fixed,
    /// The LONG (slow-accretion) edge of the interim formation-time band (megayears), the COLDEST-corner input
    /// (most SLR decay).
    pub formation_time_myr_hi: Fixed,
    /// The low edge of the birth-environment SLR-DRAW band, as a dimensionless SCALE on the family's initial
    /// abundance ratios (1.0 = the supplied canonical draw). The COLDEST-corner input for the draw axis. Basis: the
    /// star-forming region's supernova/AGB enrichment spread; set equal to the high edge (1.0/1.0) to assert the
    /// draw is known.
    pub slr_initial_ratio_scale_lo: Fixed,
    /// The high edge of the birth-environment SLR-draw band (the HOTTEST-corner input for the draw axis).
    pub slr_initial_ratio_scale_hi: Fixed,
    /// The DERIVED (or reserved-interim) planet mass in Earth masses, for the giant-impact class gate, the POINT
    /// best estimate.
    pub planet_mass_earth: Fixed,
    /// The low edge of the interim planet-mass band (Earth masses), the input to the giant-impact gate at the light
    /// end. Set equal to the high edge to assert the mass is known.
    pub planet_mass_earth_lo: Fixed,
    /// The high edge of the interim planet-mass band (Earth masses), the input to the giant-impact gate at the
    /// heavy end.
    pub planet_mass_earth_hi: Fixed,
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
    /// True when the regime cannot flip across ANY of the banded budget inputs (retention, formation time, the SLR
    /// draw, mass): both the coldest and the hottest corner of the joint band agree, so the regime is decidable
    /// without the per-world impact list. False for a MARGINAL world, whose joint band straddles the solidus (or
    /// the giant-impact mass threshold).
    pub gapped: bool,
    /// The young potential temperature (K) the deep-time province run starts from: the solidus-pinned handoff for
    /// a MELTED world, the cold peak for a NEVER-MELTED world, and, for a MARGINAL world, the CARRIED best estimate
    /// at the point inputs (the handoff when the point estimate melts, else the point cold peak). A MARGINAL world
    /// is carried at its best estimate, its grade surfaced as near-degenerate; it is never asserted GAPPED, and it
    /// is not forced to the cold branch either (the "MARGINAL-carried, not cold" discipline).
    pub young_potential_temperature_k: Fixed,
    /// The solidus-pinned lock-up handoff (K), `Some` when the carried young temperature IS that handoff (a MELTED
    /// world, or a MARGINAL world whose point estimate melts), the diagnostic the readout prints; `None` otherwise.
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
/// the cold-accretion budget (over the retention band), and the class-generic giant-impact gate, all against the
/// world's own derived solidus, and pins the melted handoff at the solidus-plus-lock-up temperature.
///
/// THE GRADE band-sweeps every budget input carried as a band. The peak temperature is monotone in each: hotter
/// at higher retention eta, hotter at a shorter formation time (less SLR decay), hotter at a larger SLR draw, and
/// (through the mass gate) melted at a larger mass. So the COLDEST corner is the slow-formation, small-draw,
/// low-retention combination and the HOTTEST corner is the fast-formation, large-draw, high-retention combination,
/// and a world is GAPPED MELTED when even the coldest corner melts (or is robustly Earth-mass-and-up), GAPPED
/// NEVER-MELTED when even the hottest corner stays sub-solidus (and is robustly below the giant-impact mass), and
/// MARGINAL when the joint band straddles. A world graded MELTED or NEVER-MELTED only because a single input was
/// pinned to its point value, while a plausible value elsewhere in that input's reserved band would flip it, is
/// MARGINAL: the containment rule is that no banded input may flip a GAPPED verdict. Setting an input's band edges
/// equal to its point value asserts that input is known and removes it from the sweep.
///
/// THE CARRIED young temperature is the best point estimate: a world that melts at its point inputs is carried at
/// the super-solidus handoff, one that does not at its point cold peak. A MARGINAL world is carried at that best
/// estimate (not forced to the cold branch), its grade surfaced as near-degenerate. `None` on an arithmetic
/// failure or a non-physical input.
pub fn young_thermal_verdict(
    inputs: &YoungThermalInputs,
    slr_family: &[ShortLivedRadionuclide],
) -> Option<YoungThermalVerdict> {
    let cp = inputs.specific_heat_j_per_kg_k;
    // The POINT-estimate SLR rise (the reported rise and the input to the carried initial condition): the family
    // decayed over the point formation time at the supplied (unscaled) draw.
    let slr_rise = slr_family_temperature_rise(slr_family, inputs.formation_time_myr, cp)?;
    let accretional_energy = accretional_specific_energy(
        inputs.binding_energy_geometry,
        inputs.surface_gravity_m_per_s2,
        inputs.radius_m,
    )?;
    // The POINT retention-band cold peaks (reported diagnostics, and the carried-IC melt test).
    let peak_lo = peak_temperature_at(
        inputs.reference_temperature_k,
        slr_rise,
        accretional_energy,
        inputs.retention_efficiency_lo,
        cp,
    )?;
    let peak_hi = peak_temperature_at(
        inputs.reference_temperature_k,
        slr_rise,
        accretional_energy,
        inputs.retention_efficiency_hi,
        cp,
    )?;
    let handoff = magma_ocean_handoff_temperature(
        inputs.solidus_surface_k,
        inputs.solidus_slope_k_per_gpa,
        inputs.adiabat_slope_k_per_gpa,
        inputs.productivity_per_gpa,
        inputs.lockup_melt_fraction,
    );

    // THE JOINT-BAND CORNERS. Coldest: the slow-formation edge (most SLR decay), the small-draw edge, the
    // low-retention edge. Hottest: the fast-formation edge, the large-draw edge, the high-retention edge. The SLR
    // rise is linear in the initial draw, so a uniform scale on the family scales it directly.
    let slr_cold = slr_family_temperature_rise(slr_family, inputs.formation_time_myr_hi, cp)?
        .checked_mul(inputs.slr_initial_ratio_scale_lo)?;
    let slr_hot = slr_family_temperature_rise(slr_family, inputs.formation_time_myr_lo, cp)?
        .checked_mul(inputs.slr_initial_ratio_scale_hi)?;
    let cold_peak = peak_temperature_at(
        inputs.reference_temperature_k,
        slr_cold,
        accretional_energy,
        inputs.retention_efficiency_lo,
        cp,
    )?;
    let hot_peak = peak_temperature_at(
        inputs.reference_temperature_k,
        slr_hot,
        accretional_energy,
        inputs.retention_efficiency_hi,
        cp,
    )?;
    let melts_cold = cold_peak >= inputs.solidus_surface_k;
    let melts_hot = hot_peak >= inputs.solidus_surface_k;
    // The giant-impact gate over the mass band: robustly class-generic when even the light mass edge crosses the
    // threshold, possibly class-generic when the heavy edge does.
    let gate_robust = inputs.planet_mass_earth_lo >= inputs.giant_impact_mass_threshold_earth;
    let gate_possible = inputs.planet_mass_earth_hi >= inputs.giant_impact_mass_threshold_earth;

    // THE GRADE from the joint-band corners.
    let (regime, gapped) = if gate_robust || melts_cold {
        (YoungThermalRegime::Melted, true)
    } else if !melts_hot && !gate_possible {
        (YoungThermalRegime::NeverMelted, true)
    } else {
        (YoungThermalRegime::Marginal, false)
    };

    // THE CARRIED best estimate at the point inputs. A world that ROBUSTLY melts at its point formation time (even
    // at the low-retention edge, or is class-generic at its point mass) is carried at the super-solidus handoff.
    // A MARGINAL world that robustly melts at its point estimate is carried hot (the default case: it melts at its
    // point formation, only a slower formation within its interim band would un-melt it), never forced to the cold
    // branch. One that does NOT robustly melt at its point estimate is carried at a sub-solidus temperature (the
    // point cold peak for a MARGINAL world, the warmest sub-solidus peak for a NEVER-MELTED one), never a
    // fabricated hot start. For a point-band world this reduces exactly to the point melt test.
    let point_gate = inputs.planet_mass_earth >= inputs.giant_impact_mass_threshold_earth;
    let point_melts = point_gate || peak_lo >= inputs.solidus_surface_k;
    let carried_from_handoff = match regime {
        YoungThermalRegime::Melted => true,
        YoungThermalRegime::NeverMelted => false,
        YoungThermalRegime::Marginal => point_melts,
    };
    let young_t = if carried_from_handoff {
        handoff.unwrap_or(inputs.solidus_surface_k)
    } else if regime == YoungThermalRegime::NeverMelted {
        peak_hi
    } else {
        peak_lo
    };

    Some(YoungThermalVerdict {
        regime,
        gapped,
        young_potential_temperature_k: young_t,
        handoff_potential_temperature_k: if carried_from_handoff { handoff } else { None },
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
            // The base inputs assert every other banded axis as a POINT (band edges equal to the point value), so
            // only the axis a given test varies is in play; the banded-regrade and hindcast tests set real bands.
            formation_time_myr_lo: formation_time_myr,
            formation_time_myr_hi: formation_time_myr,
            slr_initial_ratio_scale_lo: Fixed::ONE,
            slr_initial_ratio_scale_hi: Fixed::ONE,
            planet_mass_earth: mass_earth,
            planet_mass_earth_lo: mass_earth,
            planet_mass_earth_hi: mass_earth,
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
        // THE MANDATORY SELF-TEST, GENERALIZED. The containment rule is that NO input carried as a band may flip a
        // verdict tagged GAPPED. For every world, collapsing ANY of the four banded budget axes (retention,
        // formation time, the SLR draw, and mass) to any interior point of its band must not change the regime of a
        // GAPPED verdict; a verdict that CAN be flipped by any axis is MARGINAL and must be tagged so. The base
        // worlds carry REAL bands on every axis so the sweep is exercised, not asserted away with point bands.
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
        // The interior sweep points across each axis's band, the self-test probes with.
        let retention_band = [
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(5, 100),
            Fixed::from_ratio(1, 10),
            Fixed::from_ratio(2, 10),
            Fixed::from_ratio(3, 10),
            Fixed::from_ratio(4, 10),
        ];
        let formation_band = [
            Fixed::from_ratio(3, 10),
            Fixed::ONE,
            Fixed::from_int(2),
            Fixed::from_int(4),
        ];
        let draw_band = [Fixed::from_ratio(1, 2), Fixed::ONE, Fixed::from_int(2)];
        for &mass in &masses {
            for &t_form in &formation_times {
                // A world with a real band on every axis: retention [0.01, 0.4], formation [0.3 Myr, 4 Myr]
                // bracketing the point, the SLR draw [0.5, 2] around canonical, and the mass held at its point.
                let base = YoungThermalInputs {
                    formation_time_myr_lo: Fixed::from_ratio(3, 10),
                    formation_time_myr_hi: Fixed::from_int(4),
                    slr_initial_ratio_scale_lo: Fixed::from_ratio(1, 2),
                    slr_initial_ratio_scale_hi: Fixed::from_int(2),
                    ..base_inputs(t_form, mass)
                };
                let verdict = young_thermal_verdict(&base, &canonical_family()).unwrap();
                if !verdict.gapped {
                    continue; // a MARGINAL verdict is allowed to flip; that is what MARGINAL means
                }
                // Collapse EACH axis in turn to each interior point and confirm the GAPPED regime holds.
                let mut probes: Vec<(YoungThermalInputs, String)> = Vec::new();
                for &eta in &retention_band {
                    probes.push((
                        YoungThermalInputs {
                            retention_efficiency_lo: eta,
                            retention_efficiency_hi: eta,
                            ..base
                        },
                        format!("retention {}", eta.to_f64_lossy()),
                    ));
                }
                for &tf in &formation_band {
                    probes.push((
                        YoungThermalInputs {
                            formation_time_myr_lo: tf,
                            formation_time_myr_hi: tf,
                            ..base
                        },
                        format!("formation {} Myr", tf.to_f64_lossy()),
                    ));
                }
                for &scale in &draw_band {
                    probes.push((
                        YoungThermalInputs {
                            slr_initial_ratio_scale_lo: scale,
                            slr_initial_ratio_scale_hi: scale,
                            ..base
                        },
                        format!("draw scale {}", scale.to_f64_lossy()),
                    ));
                }
                for (probed, label) in &probes {
                    let probed_verdict =
                        young_thermal_verdict(probed, &canonical_family()).unwrap();
                    assert_eq!(
                        probed_verdict.regime, verdict.regime,
                        "a GAPPED verdict ({:?}) flipped to {:?} collapsing {} (point mass {} M_earth, point \
                         t_form {} Myr): it was marginal and mis-tagged",
                        verdict.regime,
                        probed_verdict.regime,
                        label,
                        mass.to_f64_lossy(),
                        t_form.to_f64_lossy(),
                    );
                }
            }
        }
    }

    #[test]
    fn a_banded_formation_time_regrades_a_point_gapped_world_marginal() {
        // THE RE-GRADE (fix 1 / the panel catch). A world that is GAPPED MELTED when its formation time is pinned
        // to a fast point value (26Al alive, the SLR heat dominant) is MARGINAL once the formation time carries its
        // real interim band up to a few megayears (26Al largely decayed at the slow edge, so the retention band
        // alone straddles the solidus). The verdict must not over-claim GAPPED on the fast point value alone. This
        // is the default Mars-class scene's honest grade.
        let point = base_inputs(Fixed::ONE, Fixed::from_ratio(1, 10));
        let point_verdict = young_thermal_verdict(&point, &canonical_family()).unwrap();
        assert_eq!(
            point_verdict.regime,
            YoungThermalRegime::Melted,
            "pinned to a 1 Myr formation, the SLR heat makes it look robustly melted"
        );
        assert!(
            point_verdict.gapped,
            "the point-value verdict over-claims GAPPED"
        );

        let banded = YoungThermalInputs {
            formation_time_myr_lo: Fixed::from_ratio(1, 2),
            formation_time_myr_hi: Fixed::from_int(4),
            ..point
        };
        let banded_verdict = young_thermal_verdict(&banded, &canonical_family()).unwrap();
        assert_eq!(
            banded_verdict.regime,
            YoungThermalRegime::Marginal,
            "with the real formation-time band the grade is MARGINAL, not GAPPED"
        );
        assert!(
            !banded_verdict.gapped,
            "a band-straddling world is not gapped"
        );
        // MARGINAL-carried, not cold: the world melts at its point formation (1 Myr), so it is carried at the
        // super-solidus handoff, its grade surfaced as near-degenerate, never forced to the cold branch.
        assert!(
            banded_verdict.young_potential_temperature_k > banded.solidus_surface_k,
            "the MARGINAL world is carried at its best-estimate super-solidus handoff, not forced cold"
        );
    }

    #[test]
    fn a_mars_class_world_at_mars_like_formation_is_not_never_melted_the_hindcast() {
        // THE HINDCAST ROW (the empirical anchor). Real Mars IS differentiated, has a metallic core, carries
        // crustal remanence from an early dynamo, and Hf/W dates its accretion to a few megayears. So any
        // parameterization that leaves a Mars-class world UNMELTED at Mars-like formation times fails the hindcast:
        // at a 2-4 Myr formation the melted outcome must remain reachable (the hot edge of the retention band
        // crosses the solidus), the verdict never GAPPED NEVER-MELTED. Here the formation band is Mars-like and the
        // retention band is the reserved [0.01, 0.4]; the grade may be MELTED or MARGINAL but never cold.
        for &t_form in &[Fixed::from_int(2), Fixed::from_int(3), Fixed::from_int(4)] {
            let inputs = YoungThermalInputs {
                formation_time_myr_lo: Fixed::from_int(2),
                formation_time_myr_hi: Fixed::from_int(4),
                ..base_inputs(t_form, Fixed::from_ratio(1, 10))
            };
            let verdict = young_thermal_verdict(&inputs, &canonical_family()).unwrap();
            assert_ne!(
                verdict.regime,
                YoungThermalRegime::NeverMelted,
                "hindcast: a Mars-class world at a {} Myr formation must not be graded categorically cold \
                 (real Mars is differentiated)",
                t_form.to_f64_lossy()
            );
            // The melted outcome is reachable: even the coldest corner is not asserted, and the hot edge melts.
            let hot_edge = YoungThermalInputs {
                retention_efficiency_lo: inputs.retention_efficiency_hi,
                formation_time_myr_lo: Fixed::from_int(2),
                formation_time_myr_hi: Fixed::from_int(2),
                ..inputs
            };
            let hot_verdict = young_thermal_verdict(&hot_edge, &canonical_family()).unwrap();
            assert_eq!(
                hot_verdict.regime,
                YoungThermalRegime::Melted,
                "hindcast: a fast-formed Mars-class world at the hot retention edge melts, so a melted Mars is \
                 a reachable, hindcast-consistent outcome"
            );
        }
    }

    #[test]
    fn the_dulong_petit_specific_heat_matches_the_measured_high_temperature_values() {
        // c_p = 3R / <m_atom>, the DERIVED specific heat at magma-ocean temperature keyed on the assemblage's own
        // mean atomic mass, so an iron world and a silicate world each judge their own budget by construction.
        // Iron: mean atomic mass 55.845 g/mol = 0.055845 kg/mol -> ~447 J/(kg*K), the measured high-T value.
        let iron = dulong_petit_specific_heat(Fixed::from_ratio(55_845, 1_000_000)).unwrap();
        assert!(
            (iron.to_f64_lossy() - 447.0).abs() < 5.0,
            "an iron world's c_p is ~447 J/(kg*K), got {}",
            iron.to_f64_lossy()
        );
        // Forsterite Mg2SiO4: molar mass 140.69 g/mol over 7 atoms = 20.098 g/mol = 0.020098 kg/mol -> ~1240.
        let forsterite = dulong_petit_specific_heat(Fixed::from_ratio(20_098, 1_000_000)).unwrap();
        assert!(
            (forsterite.to_f64_lossy() - 1240.0).abs() < 15.0,
            "a forsterite-class silicate world's c_p is ~1240 J/(kg*K), got {}",
            forsterite.to_f64_lossy()
        );
        // The lighter assemblage has the larger specific heat (more atoms per unit mass), and a non-physical mean
        // atomic mass fails loud.
        assert!(
            forsterite > iron,
            "the lighter assemblage holds more heat per kg"
        );
        assert!(dulong_petit_specific_heat(Fixed::ZERO).is_none());
    }

    #[test]
    fn the_derived_c_p_can_flip_the_melt_regime_it_is_load_bearing() {
        // c_p divides the whole budget, so a heavier (iron-rich) assemblage at ~447 heats far more than a silicate
        // default at ~1000 for the same energy: the melt decision is load-bearing on c_p, which is why it is
        // derived from the assemblage rather than fixed. Same world, two heat capacities, opposite melt outcomes.
        let acc_only_family: [ShortLivedRadionuclide; 2] = solar_system_slr_family(
            Fixed::ZERO, // no aluminium: kill the SLR term so the accretion budget alone decides
            Fixed::ZERO,
            Fixed::from_ratio(52, 1_000_000),
            Fixed::from_ratio(1, 100_000_000),
        );
        // A retention efficiency and a solidus tuned so a silicate c_p (1000) stays sub-solidus but an iron c_p
        // (447) crosses it, from the same accretional energy.
        let base = YoungThermalInputs {
            retention_efficiency_lo: Fixed::from_ratio(5, 100),
            retention_efficiency_hi: Fixed::from_ratio(5, 100),
            reference_temperature_k: Fixed::from_int(300),
            solidus_surface_k: Fixed::from_int(900),
            ..base_inputs(Fixed::from_int(100), Fixed::from_ratio(1, 10))
        };
        let silicate = YoungThermalInputs {
            specific_heat_j_per_kg_k: Fixed::from_int(1000),
            ..base
        };
        let iron = YoungThermalInputs {
            specific_heat_j_per_kg_k: Fixed::from_int(447),
            ..base
        };
        let sv = young_thermal_verdict(&silicate, &acc_only_family).unwrap();
        let iv = young_thermal_verdict(&iron, &acc_only_family).unwrap();
        assert_ne!(
            sv.regime, iv.regime,
            "the same world flips regime between a silicate and an iron specific heat: c_p is load-bearing \
             (silicate {:?} at {} K peak, iron {:?} at {} K peak)",
            sv.regime,
            sv.cold_peak_temperature_k_hi.to_f64_lossy(),
            iv.regime,
            iv.cold_peak_temperature_k_hi.to_f64_lossy(),
        );
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
