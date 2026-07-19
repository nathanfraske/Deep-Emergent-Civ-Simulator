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

//! The SMALL-BODY residue (asteroids and comets, task #74): the un-accreted planetesimals the planet did not
//! sweep up, instantiated as DISCRETE bodies whose orbit and composition DERIVE from the same disk state the
//! planet pipeline reads, never authored. This module holds the per-body derivation (slice 1) and the
//! belt-population sampler (slice 2, [`sample_belt`]); the resonance-sculpted gaps, moon-capture, and
//! dwarf-planet arrangements are flagged follow-ons (see the module tail).
//!
//! The derivation chain, each link a built substrate this module CONSUMES rather than re-deriving:
//!
//! - COMPOSITION (asteroid vs comet) is the water snow line. A body is refractory (rocky, asteroidal) inside the
//!   line and icy (volatile-rich, cometary) beyond it, and the line is where the disk temperature crosses the
//!   temperature at which water ice condenses. The disk temperature at the orbit is the two-regime profile
//!   [`crate::astro::disk_effective_temperature`] (irradiation plus viscous). Whether ice condenses there is the
//!   Murphy-Koop ice saturation curve [`civsim_physics::ice_sublimation`], the `H2O(gas) <-> H2O(ice)` front,
//!   evaluated at that temperature and compared to the nebular water partial pressure: water condenses when the
//!   saturation pressure falls below the available partial pressure. Because the saturation pressure is
//!   exponentially steep in temperature (Clausius-Clapeyron), the snow-line TEMPERATURE is pinned near ~150 to
//!   180 K almost regardless of the partial pressure, so the split is robust; the ORBIT it lands at is derived
//!   from the disk profile. This is the same condensation front the planet pipeline's composition arc reads, one
//!   species (water) deep, the species that defines the asteroid-comet boundary.
//!
//! - ORBIT (semi-major axis) is the residual disk. The un-accreted bodies populate the disk by its own mass, so
//!   a body's semi-major axis is the quantile of the residual disk-mass distribution: the surface-density profile
//!   [`crate::astro::disk_surface_density`] integrated `2*pi*r*Sigma(r)` outward, with the planet's feeding zone
//!   masked to zero (those bodies were accreted, so the swept gap has no residue and no body lands in it). A body
//!   indexed by a mass quantile in `[0, 1)` reads the inverse of this cumulative mass, so the reservoir is dense
//!   where the disk mass is and empty across the accreted gap, derived rather than placed by hand.
//!
//! - SIZE is the collisional cascade. A body's diameter (as a ratio to the largest body) is the quantile of the
//!   Dohnanyi size-frequency distribution [`civsim_world::impact_flux::number_fraction_above_size`], inverted, so
//!   the swarm is dominated by small bodies. This is the SAME distribution the impact chain draws strikes from,
//!   so the meteoroids and meteorites are the small tail of this one reservoir delivered by the seam-4 impact
//!   flux and crater law, never a second population re-derived here.
//!
//! - ECCENTRICITY is viscous stirring. The gravitational stirring of a planetesimal swarm gives a Rayleigh
//!   eccentricity distribution (the shape is derived), so a body's eccentricity is the Rayleigh inverse at its
//!   quantile. The stirring SCALE (the RMS eccentricity) is the one dynamical residue this needs, reserved with
//!   its basis rather than fabricated.
//!
//! Admit-the-alien (a prime directive): every input is the star's, the disk's, or the reservoir's own datum,
//! carried on the [`DiskReservoir`] data struct. A heavier star, a steeper cascade, a wider disk, or a
//! volatile-poor inner nebula are each a different set of numbers through the same law, not a new code path.
//! Determinism (Principle 3, Principle 10): fixed-point throughout, the pinned [`Fixed::exp`], [`Fixed::ln`],
//! [`Fixed::powf`], and [`Fixed::sqrt`], with bounded integer-only integrations and bisections (fixed iteration
//! counts, engine-accuracy bounds); a non-physical input fails soft to `None`, never a fabricated body. This
//! module is DORMANT: nothing here is wired into a pinned run path, so the run pins hold bit-exact.
//!
//! The DISCRETE bodies are instantiated by DETERMINISTIC QUANTILES (three per body: the residual-mass quantile
//! for the orbit, the stirring quantile for the eccentricity, the cascade quantile for the size). The
//! belt-population sampler [`sample_belt`] draws `count` representative bodies, each body's three quantiles a pure
//! function of the world seed and the body's sampling index through the core [`Rng`] SplitMix64 counter stream
//! (the same seeded-draw machinery the run's contingency draws use), so the population is a deterministic,
//! reproducible sample of the reservoir with no scheduler dependency. The `count` is a labeled DISPLAY BUDGET,
//! distinct from the physical population, which scales with the DERIVED residual disk mass ([`residual_disk_mass`]).
//!
//! The PHYSICAL population is now derived rather than flagged: [`residual_body_count_log10`] divides that residual
//! mass by the number-weighted mean body mass of the same Dohnanyi cascade the sizes are drawn from, and the mean
//! is an analytic closed form over the size distribution ([`civsim_world::impact_flux::ln_mean_cube_size_ratio`]),
//! never a sample. It is reported as `log10` of the count because a real reservoir holds far more bodies than
//! Q32.32 can express (a solar-nebula-grade disk of 100 m to 100 km planetesimals runs to ~1e14, against a ceiling
//! of ~2.1e9). READ ITS SCOPE CAREFULLY: it is the count of un-accreted bodies in the WHOLE SYSTEM, not the number
//! that will strike any one planet. Turning it into a per-target bombardment count needs a late-accretion
//! ALLOCATION (what fraction of the reservoir a given planet sweeps up, over what epoch), which does not exist
//! here; `planetary_assembly` pre-registers that mass flux as a future ledger edge and names it as crossing the
//! conserved boundary. Until it lands, no consumer may read this count as a per-planet impactor count.

use civsim_core::{Fixed, Rng};
use civsim_physics::ice_sublimation::IceSublimation;
use civsim_world::impact_flux;

use crate::astro;

/// The volatile class of a small body, the derived asteroid-versus-comet split at the water snow line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolatileClass {
    /// Refractory (rocky, asteroidal): inside the snow line, water stays gaseous and does not join the body.
    Refractory,
    /// Icy (volatile-rich, cometary): beyond the snow line, water ice condenses into the body.
    Icy,
}

/// A derived small body: its orbit, size, and composition class, each following from the disk state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmallBody {
    /// The semi-major axis (AU), the inverse of the residual disk-mass distribution at the body's mass quantile.
    pub semi_major_axis_au: Fixed,
    /// The orbital eccentricity, the Rayleigh viscous-stirring inverse at the body's stirring quantile.
    pub eccentricity: Fixed,
    /// The diameter as a RATIO to the largest body in the reservoir, the Dohnanyi cascade inverse at the body's
    /// size quantile. Held as a ratio so the absolute sizes (metres to thousands of kilometres) never enter
    /// fixed point, the same ratio discipline [`civsim_world::impact_flux`] uses.
    pub diameter_ratio: Fixed,
    /// The volatile class, refractory inside the snow line and icy beyond.
    pub volatile_class: VolatileClass,
}

/// The disk-and-reservoir state a small body derives from, all per-world data (the admit-the-alien test), each
/// field reserved with its basis. Most fields are SHARED with the built substrates ([`crate::astro`],
/// [`civsim_world::impact_flux`]), so they carry the same reserved value the planet pipeline already uses; the
/// two new residues (the snow-line water partial pressure and the eccentricity stirring scale) are
/// named in the field docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskReservoir {
    /// The disk mass-accretion rate `Mdot` in solar masses per megayear (shared with
    /// [`crate::astro::disk_effective_temperature`]). Basis: the observed class-II disk accretion rate.
    pub accretion_rate_msun_myr: Fixed,
    /// The star's mass as a fraction of the sun (shared with [`crate::astro`]). Basis: the scenario's star.
    pub star_mass_ratio: Fixed,
    /// The mass-luminosity exponent (shared with [`crate::astro`]). Basis: the star's opacity regime, ~3.5.
    pub luminosity_exponent: Fixed,
    /// The disk reprocessing factor (shared with [`crate::astro`]). Basis: the absorb-to-reradiate geometry,
    /// `1/4` for the fast-rotator / spherical-grain equilibrium.
    pub reprocessing_factor: Fixed,
    /// The inner-edge suppression factor (shared with [`crate::astro`]). Basis: the inner truncation radius, ~1
    /// in the bulk disk where the condensation fronts sit.
    pub inner_boundary_factor: Fixed,
    /// The representable temperature ceiling the fourth-root reads cap at (shared with [`crate::astro`]). An
    /// engine bound, not a physical knob.
    pub t_max: Fixed,
    /// The disk characteristic (cutoff) radius `r_c` in AU (shared with [`crate::astro::disk_surface_density`]).
    /// Basis: the disk's viscous-spreading / angular-momentum radius.
    pub characteristic_radius_au: Fixed,
    /// The surface-density slope `gamma` (shared with [`crate::astro::disk_surface_density`]). Basis: the
    /// viscosity power law `nu ~ r^gamma`, ~1. Must be below 2 (the finite-mass condition).
    pub gamma: Fixed,
    /// The surface-density normalization `Sigma_c` in kg/m^2 (shared with
    /// [`crate::astro::disk_surface_density`]), the profile's value at the characteristic radius `r_c` rather than
    /// at 1 AU. Basis: the disk-mass fraction. This cancels in the orbit quantile (which reads mass ratios), so
    /// its value does not move the derived semi-major axis; it sets the ABSOLUTE scale of the residual mass
    /// ([`residual_disk_mass`]) and therefore the physical body count, which is proportional to it.
    pub surface_density_normalization: Fixed,
    /// The inner edge of the residual disk in AU. Basis: the disk inner truncation radius (the magnetospheric or
    /// co-rotation truncation), the same inner-edge datum the `inner_boundary_factor` keys on.
    pub inner_disk_edge_au: Fixed,
    /// The outer edge of the residual disk in AU (the integration bound). Basis: the disk truncation radius; the
    /// surface-density exponential cutoff beyond `r_c` makes the residual mass insensitive to its exact value
    /// past ~2 to 3 `r_c`, so it is derived up to `r_c`.
    pub outer_disk_edge_au: Fixed,
    /// The inner edge of the planet's swept feeding zone in AU (bodies here were accreted, so this annulus holds
    /// no residue). Basis: the planet's feeding-zone width, a few Hill radii, the same annulus
    /// [`crate::astro::feeding_zone_mass`] integrates.
    pub feeding_zone_inner_au: Fixed,
    /// The outer edge of the planet's swept feeding zone in AU. Basis: as `feeding_zone_inner_au`.
    pub feeding_zone_outer_au: Fixed,
    /// NEW RESERVED (snow line): the nebular water partial pressure (Pa) at the disk temperature the snow line
    /// sits at. Basis: the solar water mole fraction times the local disk midplane pressure (the disk vertical
    /// structure `Sigma`, `T`, mean molecular weight, and Kepler frequency) times the solar oxygen/water
    /// abundance; the snow-line temperature it implies (~150 to 182 K, the Lodders water-ice line) is then
    /// derived by inverting the saturation curve. Robust to ~10 K against a factor-100 pressure change because
    /// the saturation pressure is exponentially steep. Reserved until the disk-pressure and solar-abundance
    /// substrates wire through; it enters as this one datum, not a fabricated snow-line orbit.
    pub water_partial_pressure_pa: Fixed,
    /// The Dohnanyi differential slope `p` (shared with [`civsim_world::impact_flux`]). Basis: the collisional
    /// cascade, near `3.5`. Must be above 1 (the number integral converges) for the size sampler.
    pub dohnanyi_slope: Fixed,
    /// The smallest body as a RATIO to the largest (shared with [`civsim_world::impact_flux`]). Basis: the
    /// reservoir's size bounds (the smallest tracked body over the largest).
    pub min_size_ratio: Fixed,
    /// NEW RESERVED (count): the largest body's DIAMETER in metres, the absolute scale `min_size_ratio` is a ratio
    /// to. Basis: the reservoir's large-end size bound, the biggest surviving planetesimal below the planetary-
    /// embryo tier (an embryo merger is a separate event tier, not this population), of order tens to hundreds of
    /// km; the same large-end datum the impact chain's size-frequency upper bound carries. ONLY the physical body
    /// count reads it: every other derivation in this module works in size ratios, which is what keeps the
    /// absolute sizes out of fixed point.
    pub max_body_diameter_m: Fixed,
    /// NEW RESERVED (count): the bulk density of a reservoir body in kg/m^3, which with the diameter above turns a
    /// size distribution into a mass distribution. Basis: the bulk density of the assemblage that condensed at the
    /// body's orbit, refractory rock inside the snow line and an ice-rock mix beyond it. It DERIVES DOWN when the
    /// per-body condensation assemblage lands (the composition-deepening slice flagged at the module tail); the
    /// deep-time impact wire already derives its impactor density this way, from the planet's own uncompressed
    /// bulk density, on the ground that the leftover planetesimals are the reservoir the planet accreted from.
    /// HONEST LIMIT: one density stands for the whole reservoir, so a disk spanning the snow line carries a single
    /// rock-and-ice compromise rather than the refractory-versus-icy split this module already derives per body,
    /// and the body count inherits that. Splitting it needs the residual mass integrated on each side of the snow
    /// line against a density per class, which is the natural next slice on top of this one.
    pub body_bulk_density_kg_m3: Fixed,
    /// NEW RESERVED (dynamics): the eccentricity stirring scale (the Rayleigh sigma, the RMS eccentricity).
    /// Basis: the viscous-stirring RMS eccentricity, of order the escape velocity of the largest perturbers over
    /// the local Kepler velocity, or the observed reservoir spread (~0.1 for the main belt); a per-disk dynamical
    /// residue set by the largest embryos' stirring. The Rayleigh SHAPE is derived; only this scale is reserved.
    pub eccentricity_stirring_scale: Fixed,
    /// The integration resolution for the residual-mass grid (a fixed cell count). An engine-accuracy bound, not
    /// a physical value, so determinism holds by construction.
    pub integration_steps: u32,
}

/// The natural log of the ice saturation vapor pressure (Pa) at a temperature, the Murphy-Koop equation
/// `ln p = a + b/T + c*ln(T) + d*T` evaluated from the cited coefficients the ice column carries (Murphy & Koop
/// 2005). Reads the coefficients from the [`IceSublimation`] table rather than hardcoding them, so the provenance
/// stays in the data file. `None` on a non-positive temperature or a coefficient that fails to parse.
fn ln_ice_saturation_pressure(ice: &IceSublimation, temperature_k: Fixed) -> Option<Fixed> {
    if temperature_k <= Fixed::ZERO {
        return None;
    }
    let (a, b, c, d) = ice.equation_coefficients();
    // Deserialize the four cited Murphy-Koop coefficients (read from the physics ice column, not authored here).
    let coefficient = |s: &str| Fixed::from_decimal_str(s).ok();
    let a = coefficient(a)?;
    let b = coefficient(b)?;
    let c = coefficient(c)?;
    let d = coefficient(d)?;
    let b_over_t = b.checked_div(temperature_k)?;
    let c_ln_t = c.checked_mul(temperature_k.ln())?;
    let d_t = d.checked_mul(temperature_k)?;
    a.checked_add(b_over_t)?
        .checked_add(c_ln_t)?
        .checked_add(d_t)
}

/// Whether water ice condenses at a disk temperature and nebular water partial pressure: `true` when the ice
/// saturation pressure at that temperature has fallen below the available water partial pressure (the vapor is
/// supersaturated and precipitates), `false` when the saturation pressure is above it (water stays gaseous). This
/// is the derived asteroid-versus-comet criterion, one species (water) deep. `None` on a non-positive pressure or
/// an unresolved saturation curve.
pub fn ice_condenses(
    ice: &IceSublimation,
    disk_temperature_k: Fixed,
    water_partial_pressure_pa: Fixed,
) -> Option<bool> {
    if water_partial_pressure_pa <= Fixed::ZERO {
        return None;
    }
    let ln_p_sat = ln_ice_saturation_pressure(ice, disk_temperature_k)?;
    let ln_p_water = water_partial_pressure_pa.ln();
    Some(ln_p_sat <= ln_p_water)
}

/// The disk temperature (K) at an orbit, the two-regime irradiation-plus-viscous profile
/// [`crate::astro::disk_effective_temperature`] read with the reservoir's disk residues. `None` on a
/// non-positive orbit or a flux past the representable range.
///
/// HONEST LIMIT: this is the disk SURFACE effective temperature, the dependency-clean profile available in this
/// crate. Condensation strictly reads the FORMATION-era optically-thick MIDPLANE temperature
/// ([`crate::astro::formation_midplane_temperature`], which needs a materials opacity closure); the surface
/// profile is the same monotone-falling shape and lands the snow line near the same few-AU orbit, so the
/// refractory-versus-icy split holds, with the exact midplane epoch the flagged follow-on.
pub fn disk_temperature_at_orbit(reservoir: &DiskReservoir, orbit_au: Fixed) -> Option<Fixed> {
    astro::disk_effective_temperature(
        reservoir.accretion_rate_msun_myr,
        reservoir.star_mass_ratio,
        reservoir.luminosity_exponent,
        orbit_au,
        reservoir.reprocessing_factor,
        reservoir.inner_boundary_factor,
        reservoir.t_max,
    )
}

/// The volatile class of a body at an orbit: refractory inside the snow line, icy beyond it, from the disk
/// temperature there against the water saturation curve. `None` if the disk temperature or the saturation curve
/// does not resolve.
pub fn volatile_class_at_orbit(
    reservoir: &DiskReservoir,
    orbit_au: Fixed,
    ice: &IceSublimation,
) -> Option<VolatileClass> {
    let temperature = disk_temperature_at_orbit(reservoir, orbit_au)?;
    let icy = ice_condenses(ice, temperature, reservoir.water_partial_pressure_pa)?;
    Some(if icy {
        VolatileClass::Icy
    } else {
        VolatileClass::Refractory
    })
}

/// The snow-line orbit (AU): the orbit where the disk temperature crosses the water condensation temperature, so
/// bodies inside are refractory and bodies beyond are icy. Found by bounded bisection over the residual disk
/// (the temperature falls monotonically with orbit, so the crossing is unique). `None` if the disk inner edge is
/// not refractory or the outer edge is not icy (no snow line inside the disk, an all-rocky or all-icy reservoir),
/// or if a temperature does not resolve.
pub fn snow_line_orbit_au(reservoir: &DiskReservoir, ice: &IceSublimation) -> Option<Fixed> {
    let mut lo = reservoir.inner_disk_edge_au;
    let mut hi = reservoir.outer_disk_edge_au;
    if lo <= Fixed::ZERO || hi <= lo {
        return None;
    }
    // The inner edge must be rocky and the outer edge icy for a snow line to sit between them.
    if volatile_class_at_orbit(reservoir, lo, ice)? != VolatileClass::Refractory
        || volatile_class_at_orbit(reservoir, hi, ice)? != VolatileClass::Icy
    {
        return None;
    }
    for _ in 0..60 {
        let mid = lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?;
        match volatile_class_at_orbit(reservoir, mid, ice)? {
            VolatileClass::Refractory => lo = mid,
            VolatileClass::Icy => hi = mid,
        }
    }
    lo.checked_add(hi)?.checked_div(Fixed::from_int(2))
}

/// Whether an orbit lies in the reservoir's OWN swept feeding zone (where the residue was accreted, so no small
/// body survives). A degenerate zone (`inner >= outer`) means no feeding zone was swept.
fn in_feeding_zone(reservoir: &DiskReservoir, orbit_au: Fixed) -> bool {
    reservoir.feeding_zone_inner_au < reservoir.feeding_zone_outer_au
        && orbit_au >= reservoir.feeding_zone_inner_au
        && orbit_au <= reservoir.feeding_zone_outer_au
}

/// Whether an orbit lies in ANY swept zone: the reservoir's own feeding zone (the single-planet residue slice 1
/// masks) OR any of the caller-supplied `swept_zones` (a population's several cleared feeding zones). A body in any
/// accreted gap has no residue, so the mask is the UNION of the two; an empty `swept_zones` recovers the
/// single-planet behaviour. Each swept zone is an `(inner_au, outer_au)` annulus; a degenerate one (`inner >=
/// outer`) masks nothing.
fn orbit_is_swept(
    reservoir: &DiskReservoir,
    swept_zones: &[(Fixed, Fixed)],
    orbit_au: Fixed,
) -> bool {
    if in_feeding_zone(reservoir, orbit_au) {
        return true;
    }
    swept_zones
        .iter()
        .any(|&(inner, outer)| inner < outer && orbit_au >= inner && orbit_au <= outer)
}

/// The residual disk mass in a ring at an orbit, `2*pi*r*Sigma(r)` masked to zero inside any swept zone (the
/// reservoir's own feeding zone and every caller-supplied `swept_zones` annulus). This is the per-radius density of
/// the residual-mass distribution the orbit quantile inverts. `None` on a bad surface density.
fn residual_ring_density(
    reservoir: &DiskReservoir,
    swept_zones: &[(Fixed, Fixed)],
    orbit_au: Fixed,
) -> Option<Fixed> {
    if orbit_is_swept(reservoir, swept_zones, orbit_au) {
        return Some(Fixed::ZERO);
    }
    let sigma = astro::disk_surface_density(
        orbit_au,
        reservoir.characteristic_radius_au,
        reservoir.gamma,
        reservoir.surface_density_normalization,
    )?;
    let two_pi = Fixed::PI.checked_add(Fixed::PI)?;
    two_pi.checked_mul(orbit_au)?.checked_mul(sigma)
}

/// The total residual disk mass in `Sigma_c * AU^2`, so with `Sigma_c` in kg/m^2 the mass in kilograms is this
/// times the square of the astronomical unit in metres: the integral `int 2*pi*r*Sigma(r) dr` over the residual
/// disk with the reservoir's own feeding zone masked, a bounded midpoint Riemann sum over `integration_steps`
/// cells. It is left in this scaled unit rather than converted because the kilogram value (~1e26 for a
/// solar-nebula-grade disk) is decades past the fixed-point ceiling; [`residual_body_count_log10`] does the
/// conversion in log space. This is the DERIVED size of the reservoir the belt samples from, the mass the
/// un-accreted planetesimals carry (the "later mass readout" the `surface_density_normalization` field
/// anticipates). The PHYSICAL body count is this mass divided by the cascade's number-weighted mean body mass,
/// which [`residual_body_count_log10`] derives; a belt's `count` is a DISPLAY sample of that population, never the
/// population itself. `None` on a degenerate disk range, zero steps, or a surface density that fails to resolve;
/// `Some(0)` when the residual disk is fully swept.
pub fn residual_disk_mass(reservoir: &DiskReservoir) -> Option<Fixed> {
    residual_disk_mass_masked(reservoir, &[])
}

/// The total residual disk mass with an additional set of `swept_zones` masked alongside the reservoir's own
/// feeding zone (the union of masks). Shared first pass of the residual-mass integration.
fn residual_disk_mass_masked(
    reservoir: &DiskReservoir,
    swept_zones: &[(Fixed, Fixed)],
) -> Option<Fixed> {
    let inner = reservoir.inner_disk_edge_au;
    let outer = reservoir.outer_disk_edge_au;
    if inner <= Fixed::ZERO || outer <= inner || reservoir.integration_steps == 0 {
        return None;
    }
    let steps = reservoir.integration_steps;
    let span = outer.checked_sub(inner)?;
    let dr = span.checked_div(Fixed::from_int(steps as i32))?;
    let half_dr = dr.checked_div(Fixed::from_int(2))?;
    let mut total = Fixed::ZERO;
    for i in 0..steps {
        let r = inner.checked_add(
            dr.checked_mul(Fixed::from_int(i as i32))?
                .checked_add(half_dr)?,
        )?;
        let ring = residual_ring_density(reservoir, swept_zones, r)?.checked_mul(dr)?;
        total = total.checked_add(ring)?;
    }
    Some(total)
}

/// The PHYSICAL residual body count of the reservoir, returned as `log10` of the count: the derived un-accreted
/// disk mass divided by the number-weighted mean mass of one body. This is the quantity a bombardment is drawn
/// against, replacing a placeholder count with the reservoir's own arithmetic.
///
/// ITS SCOPE, WHICH IS THE FIRST THING A CONSUMER MUST READ: this is the count of un-accreted bodies in the WHOLE
/// SYSTEM, the entire residual disk outside the masked feeding zones. It is NOT the number of bodies that will
/// strike any particular planet, and it is larger than that number by many orders of magnitude. Converting it to
/// a per-target bombardment count needs a LATE-ACCRETION ALLOCATION that does not exist anywhere in this engine:
/// what fraction of the reservoir a given planet's gravitational cross-section sweeps up, over what epoch, from
/// which annuli. [`crate::planetary_assembly`] pre-registers exactly that mass flux as a future ledger edge and
/// names it as one that crosses the conserved boundary, so it is a known gap with a known home, not an oversight
/// here. Until it lands, no consumer may read this count as a per-planet impactor count.
///
/// WHY `log10`. The count is not representable: a solar-nebula-grade residual disk of 100 m to 100 km
/// planetesimals runs to ~1e14 bodies against a Q32.32 ceiling of ~2.1e9, and the intermediates are worse (the
/// mass in kilograms is ~9e25, the cube of the largest diameter 1e15 m^3). Every term is therefore assembled in LOG
/// space, in the same manner as the crate's other decades-wide readouts, and only the log is exported. A consumer
/// that needs a linear count must first apply the missing allocation fraction; the allocated count is small
/// enough to exponentiate, which the global one is not.
///
/// THE ASSEMBLY, term by term. The mass side is `ln(residual_disk_mass) + 2*ln(AU in metres)`, the second term
/// converting the `Sigma_c * AU^2` unit the integral carries into kilograms (`Sigma_c` being kg/m^2), read from
/// the IAU astronomical unit through the log-of-a-decimal-string helper because the constant itself overflows
/// fixed point. The body side is `ln(density) + ln(pi/6) + 3*ln(max diameter) + ln(mean cube ratio)`: a sphere's
/// mass from the reservoir's bulk density, and the number-weighted mean cube of size as a ratio to the largest
/// body ([`civsim_world::impact_flux::ln_mean_cube_size_ratio`], an analytic closed form over the same Dohnanyi
/// cascade the sizes are drawn from, so the count and the drawn bodies cannot disagree about the distribution).
/// The difference over `ln 10` is the answer.
///
/// HONEST LIMITS. One bulk density stands for the whole reservoir, so a disk spanning the snow line prices icy
/// and rocky bodies alike (see `body_bulk_density_kg_m3`). The mass integral is the midpoint sum's, so the count
/// inherits its resolution. A count below one (a negative result) is not clamped: it means the reservoir data are
/// inconsistent, carrying less mass than the single largest body they declare, and saying so beats hiding it.
///
/// `None` on a non-positive residual mass (a fully swept disk), a non-positive diameter or density, a size ratio
/// the mean-cube form cannot stage, or an intermediate past the representable range.
pub fn residual_body_count_log10(reservoir: &DiskReservoir) -> Option<Fixed> {
    residual_body_count_log10_masked(reservoir, &[])
}

/// The physical residual body count as `log10`, with an additional set of `swept_zones` masked alongside the
/// reservoir's own feeding zone (the union of a population's cleared annuli), so the count is of what a whole
/// assembled system left behind. Every caveat on [`residual_body_count_log10`] applies unchanged, above all that
/// the result is SYSTEM-WIDE and not a per-target impactor count.
pub fn residual_body_count_log10_masked(
    reservoir: &DiskReservoir,
    swept_zones: &[(Fixed, Fixed)],
) -> Option<Fixed> {
    let residual_mass = residual_disk_mass_masked(reservoir, swept_zones)?;
    if residual_mass <= Fixed::ZERO {
        return None; // a fully swept disk holds no bodies to count
    }
    let diameter = reservoir.max_body_diameter_m;
    let density = reservoir.body_bulk_density_kg_m3;
    if diameter <= Fixed::ZERO || density <= Fixed::ZERO {
        return None;
    }
    // The mass side: the integral's `Sigma_c * AU^2` unit into kilograms. The astronomical unit in metres is far
    // past the fixed-point range, so its log is read from the cited decimal string rather than the value.
    let ln_au = civsim_physics::saha::ln_of_decimal(astro::ASTRONOMICAL_UNIT_M)?;
    let ln_mass = residual_mass
        .ln()
        .checked_add(Fixed::from_int(2).checked_mul(ln_au)?)?;
    // The body side: a sphere of the reservoir's bulk density at the cascade's number-weighted mean cube of size.
    // The mean cube is a ratio to the largest body, so `min_size_ratio` against a unit maximum is the whole input.
    let ln_mean_cube = impact_flux::ln_mean_cube_size_ratio(
        reservoir.min_size_ratio,
        Fixed::ONE,
        reservoir.dohnanyi_slope,
    )?;
    let sphere_factor = Fixed::PI.checked_div(Fixed::from_int(6))?;
    let ln_body_mass = density
        .ln()
        .checked_add(sphere_factor.ln())?
        .checked_add(Fixed::from_int(3).checked_mul(diameter.ln())?)?
        .checked_add(ln_mean_cube)?;
    ln_mass
        .checked_sub(ln_body_mass)?
        .checked_div(Fixed::from_int(10).ln())
}

/// The semi-major axis (AU) of a small body at a residual-mass quantile in `[0, 1)`: the inverse of the
/// cumulative residual disk-mass distribution, with the reservoir's own feeding zone masked. `None` on a quantile
/// outside `[0, 1)`, a degenerate disk range, zero steps, no residual mass, or a surface density that fails to
/// resolve.
pub fn residual_semi_major_axis(quantile: Fixed, reservoir: &DiskReservoir) -> Option<Fixed> {
    residual_semi_major_axis_masked(quantile, reservoir, &[])
}

/// The semi-major axis (AU) at a residual-mass quantile in `[0, 1)`, with an additional set of `swept_zones`
/// masked alongside the reservoir's own feeding zone: the inverse of the cumulative residual disk-mass
/// distribution over the un-swept disk. A body's orbit is the radius that encloses that fraction of the residual
/// mass, so the reservoir is dense where the disk mass is and empty across every accreted gap (no body lands in a
/// swept zone). The cumulative mass is a bounded midpoint Riemann sum over `integration_steps` cells (a fixed
/// resolution, an engine-accuracy bound, so determinism holds); the normalization `Sigma_c` cancels because the
/// quantile reads a mass RATIO. `None` on a quantile outside `[0, 1)`, a degenerate disk range, zero steps, no
/// residual mass, or a surface density that fails to resolve.
pub fn residual_semi_major_axis_masked(
    quantile: Fixed,
    reservoir: &DiskReservoir,
    swept_zones: &[(Fixed, Fixed)],
) -> Option<Fixed> {
    if quantile < Fixed::ZERO || quantile >= Fixed::ONE {
        return None;
    }
    let inner = reservoir.inner_disk_edge_au;
    let outer = reservoir.outer_disk_edge_au;
    if inner <= Fixed::ZERO || outer <= inner || reservoir.integration_steps == 0 {
        return None;
    }
    let total = residual_disk_mass_masked(reservoir, swept_zones)?;
    if total <= Fixed::ZERO {
        return None; // no residual mass to place a body in
    }
    let steps = reservoir.integration_steps;
    let span = outer.checked_sub(inner)?;
    let dr = span.checked_div(Fixed::from_int(steps as i32))?;
    let half_dr = dr.checked_div(Fixed::from_int(2))?;
    let target = quantile.checked_mul(total)?;
    // The first cell whose cumulative mass reaches the target. A zero-mass (swept) cell never becomes the first
    // cell to reach the target, because the cell before it already holds the same cumulative mass, so no body is
    // placed in a swept zone.
    let mut cumulative = Fixed::ZERO;
    let mut last_r = inner.checked_add(half_dr)?;
    for i in 0..steps {
        let r = inner.checked_add(
            dr.checked_mul(Fixed::from_int(i as i32))?
                .checked_add(half_dr)?,
        )?;
        let ring = residual_ring_density(reservoir, swept_zones, r)?.checked_mul(dr)?;
        cumulative = cumulative.checked_add(ring)?;
        last_r = r;
        if cumulative >= target {
            return Some(r);
        }
    }
    Some(last_r)
}

/// The diameter (as a RATIO to the largest body) of a small body at a size quantile in `[0, 1)`: the inverse of
/// the Dohnanyi cumulative number-above-size distribution [`civsim_world::impact_flux::number_fraction_above_size`],
/// so a quantile of `q` is the size above which a fraction `q` of the bodies lie. Because the number is dominated
/// by the small end, most quantiles map to small bodies (the swarm), the collisional-cascade signature. Found by
/// bounded bisection over `[min_size_ratio, 1]` (the number fraction falls monotonically with size). This inverts
/// the SAME distribution the impact chain draws strikes from, so the meteoroid tail is this reservoir's small
/// end, not a second population. `None` on a quantile outside `[0, 1)`, a slope not above one, a bad size range,
/// or an unresolved fraction.
pub fn residual_diameter_ratio(
    size_quantile: Fixed,
    min_size_ratio: Fixed,
    dohnanyi_slope: Fixed,
) -> Option<Fixed> {
    if size_quantile < Fixed::ZERO || size_quantile >= Fixed::ONE {
        return None;
    }
    if min_size_ratio <= Fixed::ZERO || min_size_ratio >= Fixed::ONE {
        return None;
    }
    // The largest body is the ratio unit (max_size_ratio = 1); impact_flux forms everything in ratios to it.
    let max = Fixed::ONE;
    // Confirm the distribution resolves at the bounds (guards the slope-convergence condition once).
    impact_flux::number_fraction_above_size(min_size_ratio, min_size_ratio, max, dohnanyi_slope)?;
    let mut lo = min_size_ratio;
    let mut hi = max;
    for _ in 0..60 {
        let mid = lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?;
        let fraction =
            impact_flux::number_fraction_above_size(mid, min_size_ratio, max, dohnanyi_slope)?;
        if fraction > size_quantile {
            // More than the target fraction lies above mid: mid is too small, search larger.
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo.checked_add(hi)?.checked_div(Fixed::from_int(2))
}

/// The orbital eccentricity of a small body at a stirring quantile in `[0, 1)`: the Rayleigh inverse
/// `e = sigma * sqrt(-2*ln(1 - q))`, the distribution gravitational viscous stirring of a planetesimal swarm
/// produces (the shape is derived). `stirring_scale` is the Rayleigh sigma (the RMS eccentricity), the one
/// reserved dynamical residue. Zero at `q = 0`, rising with the quantile. `None` on a quantile outside `[0, 1)`,
/// a negative scale, or an intermediate past the representable range.
pub fn stirred_eccentricity(stirring_quantile: Fixed, stirring_scale: Fixed) -> Option<Fixed> {
    if stirring_quantile < Fixed::ZERO
        || stirring_quantile >= Fixed::ONE
        || stirring_scale < Fixed::ZERO
    {
        return None;
    }
    let one_minus_q = Fixed::ONE.checked_sub(stirring_quantile)?;
    // one_minus_q is in (0, 1], so its log is <= 0 and -2*ln is >= 0.
    let neg_two_ln = Fixed::ZERO.checked_sub(Fixed::from_int(2).checked_mul(one_minus_q.ln())?)?;
    let root = neg_two_ln.sqrt();
    stirring_scale.checked_mul(root)
}

/// Assemble a discrete small body from its three deterministic quantiles and the disk state: the orbit from the
/// residual-mass quantile, the eccentricity from the stirring quantile, the diameter from the size quantile, and
/// the volatile class from the snow line at the derived orbit. In the run wire the three quantiles come from the
/// content-keyed seeded draw (the contingency machinery); this slice supplies the derivation the draw feeds.
/// `None` if any link fails to resolve.
pub fn derive_small_body(
    mass_quantile: Fixed,
    stirring_quantile: Fixed,
    size_quantile: Fixed,
    reservoir: &DiskReservoir,
    ice: &IceSublimation,
) -> Option<SmallBody> {
    derive_small_body_masked(
        mass_quantile,
        stirring_quantile,
        size_quantile,
        reservoir,
        &[],
        ice,
    )
}

/// Assemble a discrete small body as [`derive_small_body`], but with an additional set of `swept_zones` masked
/// alongside the reservoir's own feeding zone (the union of cleared feeding zones a population of planets sweeps),
/// so no body lands in any accreted gap. `None` if any link fails to resolve.
pub fn derive_small_body_masked(
    mass_quantile: Fixed,
    stirring_quantile: Fixed,
    size_quantile: Fixed,
    reservoir: &DiskReservoir,
    swept_zones: &[(Fixed, Fixed)],
    ice: &IceSublimation,
) -> Option<SmallBody> {
    let semi_major_axis_au =
        residual_semi_major_axis_masked(mass_quantile, reservoir, swept_zones)?;
    let eccentricity =
        stirred_eccentricity(stirring_quantile, reservoir.eccentricity_stirring_scale)?;
    let diameter_ratio = residual_diameter_ratio(
        size_quantile,
        reservoir.min_size_ratio,
        reservoir.dohnanyi_slope,
    )?;
    let volatile_class = volatile_class_at_orbit(reservoir, semi_major_axis_au, ice)?;
    Some(SmallBody {
        semi_major_axis_au,
        eccentricity,
        diameter_ratio,
        volatile_class,
    })
}

/// The hash-domain separator for the belt-sampling stream, a STRUCTURAL constant (like the `phase` argument to
/// [`Rng::for_entity`]) that namespaces the belt draws away from any other subsystem keyed on the same world seed.
/// It carries no physical meaning and is not a world value; it is the ASCII tag `BELTSMPL`.
const BELT_SAMPLE_DOMAIN: u64 = u64::from_be_bytes(*b"BELTSMPL");

/// Sample a POPULATION of small bodies (the emergent asteroid-and-comet belt) from the reservoir: `count`
/// representative bodies, each fully DERIVED through [`derive_small_body_masked`]. The belt EMERGES, never
/// authored: the asteroid-versus-comet split falls out of the snow line (refractory inside, icy beyond), the orbit
/// spread out of the residual disk mass with every swept zone masked, and the small-dominated size distribution
/// out of the Dohnanyi collisional cascade. The count of asteroids versus comets, the orbit spread, and the size
/// distribution are all consequences of the reservoir and the snow line, not inputs.
///
/// `count` IS A LABELED SAMPLING / DISPLAY BUDGET: how many representative bodies to instantiate for the belt view,
/// NOT the physical number of bodies. The physical population is a SEPARATE quantity, the DERIVED
/// [`residual_disk_mass`] divided by the cascade's number-weighted mean body mass, which
/// [`residual_body_count_log10`] now derives (and which is far too large to be a display budget); the sampled
/// `count` is a display draw from that population. One number is the world's, the other is the viewer's budget.
///
/// `swept_zones` are the planets' cleared feeding zones (each an `(inner_au, outer_au)` annulus), caller-supplied
/// from the assembly's final planet orbits and Hill radii; the mask is the UNION of these and the reservoir's own
/// feeding zone, so no body lands in any accreted gap. For this dormant slice the zones are an input; the assembly
/// wiring is the follow-on. The resonance-sculpted gaps (Kirkwood gaps) are a SEPARATE follow-on: one planet's
/// swept zone is not a mean-motion resonance, so those gaps need the multi-body generator (#72) and the secular
/// resonances (#44), flagged at the module tail.
///
/// DETERMINISM (Principle 3, Principle 10): each body's three quantiles are a pure function of
/// `(world_seed, BELT_SAMPLE_DOMAIN, index)` through the [`Rng`] SplitMix64 counter stream, so the same reservoir,
/// seed, and count reproduce the belt bit-for-bit on any machine and at any thread count. FAIL-SOFT: a body whose
/// derivation does not resolve (a pathological reservoir) is dropped rather than fabricated, so the returned belt
/// may be shorter than `count`, never a fabricated body.
pub fn sample_belt(
    reservoir: &DiskReservoir,
    swept_zones: &[(Fixed, Fixed)],
    count: usize,
    world_seed: u64,
    ice: &IceSublimation,
) -> Vec<SmallBody> {
    let mut belt = Vec::with_capacity(count);
    for index in 0..count {
        // Three quantiles per body, keyed on the world seed plus the body's sampling index through the core
        // seeded-draw stream (the belt domain tag namespaces the stream from other subsystems on the same seed).
        let stream = Rng::for_coords(world_seed, &[BELT_SAMPLE_DOMAIN, index as u64]);
        let mass_quantile = stream.unit_fixed(0);
        let stirring_quantile = stream.unit_fixed(1);
        let size_quantile = stream.unit_fixed(2);
        if let Some(body) = derive_small_body_masked(
            mass_quantile,
            stirring_quantile,
            size_quantile,
            reservoir,
            swept_zones,
            ice,
        ) {
            belt.push(body);
        }
    }
    belt
}

// FLAGGED FOLLOW-ONS (the next small-body slices, grounded and flagged here, not forced into this one because
// each needs a substrate this slice does not yet reach):
//
//  - The RESONANCE-SCULPTED belt structure (Kirkwood gaps, the belt as a swept gap between two planets, the
//    Trojan clouds). The slice-2 population sampler ([`sample_belt`]) places bodies by the residual disk mass with
//    the union of the planets' swept feeding zones masked, so it derives a rocky-inner and icy-outer belt with the
//    accreted gaps empty, but NOT the mean-motion-resonance gaps a resonant planet (a Jupiter) carves: a swept
//    feeding zone is a cleared annulus, not a resonance. Those gaps need the emergent MULTI-BODY arrangement (the
//    solar-system generator, task #72) that supplies the several planets' orbits AND the secular / mean-motion
//    resonances (task #44) that sculpt the reservoir; the Kirkwood gap is that arc's payoff, not authorable here
//    from one planet or from a static swept-zone mask.
//
//  - The FORMATION-EPOCH condensation temperature. The composition split reads the mature two-regime disk
//    surface temperature ([`disk_temperature_at_orbit`]); the epoch-correct condensation temperature is the hot
//    optically-thick FORMATION midplane ([`crate::astro::formation_midplane_temperature`], which needs a
//    materials opacity closure `kappa_of_t`). The two share the monotone-falling shape and land the snow line
//    near the same orbit, so the split holds; wiring the formation midplane (and the full materials condensation
//    assemblage per body, the refractory mineralogy inside the line that the planet pipeline also still takes as
//    a fixture) is the composition-deepening slice.
//
//  - MOONS (task #75, capture and co-accretion dynamics). A moon is not a residual-disk sample: it is captured
//    into or co-accreted within a planet's Hill sphere, so its orbit derives from the planet's gravity (the Hill
//    radius, the circumplanetary disk), not the stellar disk's residual mass. That is a distinct dynamical
//    substrate (the planet-centred two-body-plus-tides problem), flagged as its own slice.
//
//  - DWARF PLANETS / Pluto-likes. The largest residual bodies (the top of the Dohnanyi size tail beyond the
//    ice line) that reached hydrostatic rounding but never cleared their orbit. This slice already derives their
//    size (the large end of the cascade) and their icy composition; the hydrostatic-rounding threshold (the
//    size and material strength at which self-gravity rounds a body) is a materials/strength read, the
//    classification slice that sits on top of this reservoir.

#[cfg(test)]
mod tests {
    use super::*;

    fn ice_table() -> IceSublimation {
        IceSublimation::standard().expect("the Murphy-Koop ice column loads")
    }

    // A Sun-and-solar-nebula reservoir, the numbers cited fixtures standing in for a world's reserved values (not
    // authored floor constants): a solar-mass star (mass ratio 1, mass-luminosity exponent ~3.5), a class-II
    // accretion rate (~0.01 M_sun/Myr), the spherical-grain reprocessing factor 1/4, a Lynden-Bell-Pringle disk
    // (characteristic radius ~30 AU, gamma ~1), the disk spanning ~0.2 to 40 AU, Earth's feeding zone swept
    // (~0.9 to 1.1 AU), the water partial pressure the ~180 K Lodders snow line implies (the ice saturation
    // pressure at 180 K, ~5.4e-3 Pa), the Dohnanyi cascade (p ~3.5, sizes ~1e-3 of the largest body), and a
    // main-belt-grade stirring scale (~0.1). The two absolute scales the body count reads: the largest body 100 km
    // across (so the `1e-3` size ratio puts the small end at 100 m, the same 100 m to 100 km reservoir the impact
    // chain's own fixture spans) and a 3000 kg/m^3 bulk density (the same rocky-planetesimal fixture density the
    // deep-time bombardment fixture uses, kept identical so the two do not drift). The `Sigma_c` of 1 kg/m^2 is a
    // PLACEHOLDER, not a nebula: it is the value at `r_c` = 30 AU, it was chosen when nothing read it (every other
    // derivation here cancels it), and the body count is exactly proportional to it, so the count below is the
    // count for THIS normalization and scales with a world's own.
    fn sun_reservoir() -> DiskReservoir {
        DiskReservoir {
            accretion_rate_msun_myr: Fixed::from_ratio(1, 100),
            star_mass_ratio: Fixed::ONE,
            luminosity_exponent: Fixed::from_ratio(35, 10),
            reprocessing_factor: Fixed::from_ratio(1, 4),
            inner_boundary_factor: Fixed::ONE,
            t_max: Fixed::from_int(100_000),
            characteristic_radius_au: Fixed::from_int(30),
            gamma: Fixed::ONE,
            surface_density_normalization: Fixed::ONE,
            inner_disk_edge_au: Fixed::from_ratio(2, 10),
            outer_disk_edge_au: Fixed::from_int(40),
            feeding_zone_inner_au: Fixed::from_ratio(9, 10),
            feeding_zone_outer_au: Fixed::from_ratio(11, 10),
            water_partial_pressure_pa: Fixed::from_ratio(54, 10_000),
            dohnanyi_slope: Fixed::from_ratio(35, 10),
            min_size_ratio: Fixed::from_ratio(1, 1000),
            max_body_diameter_m: Fixed::from_int(100_000),
            body_bulk_density_kg_m3: Fixed::from_int(3000),
            eccentricity_stirring_scale: Fixed::from_ratio(1, 10),
            integration_steps: 512,
        }
    }

    #[test]
    fn the_snow_line_splits_rocky_inside_from_icy_beyond() {
        // The core derived fact: at a warm inner orbit (1 AU) water stays gaseous, so a body is refractory
        // (asteroidal); at a cold outer orbit (10 AU) water ice condenses, so a body is icy (cometary). This is
        // the asteroid-versus-comet split, derived from the disk temperature against the saturation curve.
        let res = sun_reservoir();
        let ice = ice_table();
        assert_eq!(
            volatile_class_at_orbit(&res, Fixed::ONE, &ice).unwrap(),
            VolatileClass::Refractory,
            "1 AU is inside the snow line: rocky"
        );
        assert_eq!(
            volatile_class_at_orbit(&res, Fixed::from_int(10), &ice).unwrap(),
            VolatileClass::Icy,
            "10 AU is beyond the snow line: icy"
        );
    }

    #[test]
    fn the_snow_line_orbit_lands_in_the_derived_few_au_band() {
        // The snow line is an EMERGENT orbit, not an authored one: where the two-regime disk temperature crosses
        // the ~180 K water condensation temperature. For the Sun-and-solar-nebula fixture that is a few AU, near
        // the real ~2.7 AU water-ice line at the outer edge of the asteroid belt, derived from the disk profile
        // and the saturation curve with no fit to that orbit.
        let res = sun_reservoir();
        let ice = ice_table();
        let a_snow = snow_line_orbit_au(&res, &ice).expect("a snow line sits inside the disk");
        let au = a_snow.to_f64_lossy();
        assert!(
            (1.5..4.0).contains(&au),
            "the derived snow line lands in the few-AU band, got {au} AU"
        );
        // Just inside is rocky, just beyond is icy (the split brackets the derived line).
        let inside = a_snow.checked_sub(Fixed::from_ratio(3, 10)).unwrap();
        let beyond = a_snow.checked_add(Fixed::from_ratio(3, 10)).unwrap();
        assert_eq!(
            volatile_class_at_orbit(&res, inside, &ice).unwrap(),
            VolatileClass::Refractory
        );
        assert_eq!(
            volatile_class_at_orbit(&res, beyond, &ice).unwrap(),
            VolatileClass::Icy
        );
    }

    #[test]
    fn ice_condensation_follows_the_saturation_curve() {
        // A direct check of the criterion: at a warm temperature the ice saturation pressure is well above the
        // nebular water partial pressure (no condensation), and at a cold temperature it is well below it
        // (condensation). The crossover is the snow-line temperature.
        let ice = ice_table();
        let p_water = Fixed::from_ratio(54, 10_000);
        assert!(
            !ice_condenses(&ice, Fixed::from_int(250), p_water).unwrap(),
            "250 K is warm: water stays gaseous"
        );
        assert!(
            ice_condenses(&ice, Fixed::from_int(140), p_water).unwrap(),
            "140 K is cold: water ice condenses"
        );
    }

    #[test]
    fn the_saturation_pressure_matches_the_tabulated_murphy_koop_point() {
        // The Murphy-Koop evaluation from the coefficients reproduces the ice column's own tabulated point at
        // 180 K (~5.40e-3 Pa), the numerical twin that validates the fixed-point equation evaluation against the
        // physics crate's independent tabulation.
        let ice = ice_table();
        let ln_p = ln_ice_saturation_pressure(&ice, Fixed::from_int(180)).unwrap();
        let p = ln_p.exp().to_f64_lossy();
        assert!(
            (p - 5.40e-3).abs() < 5e-4,
            "p_sat(180 K) reproduces the tabulated 5.40e-3 Pa, got {p}"
        );
    }

    #[test]
    fn the_diameter_sampler_inverts_the_impact_flux_size_frequency() {
        // The meteoroid-tail connection made a test: sampling a diameter at a size quantile and reading the
        // impact_flux number-fraction-above back must round-trip, proving the small-body size distribution IS the
        // same Dohnanyi cascade the impact chain draws strikes from, not a second population.
        let res = sun_reservoir();
        for &(quantile, q) in &[
            (Fixed::from_ratio(1, 10), 0.1_f64),
            (Fixed::from_ratio(1, 2), 0.5_f64),
            (Fixed::from_ratio(9, 10), 0.9_f64),
        ] {
            let d = residual_diameter_ratio(quantile, res.min_size_ratio, res.dohnanyi_slope)
                .expect("the size resolves");
            let fraction = impact_flux::number_fraction_above_size(
                d,
                res.min_size_ratio,
                Fixed::ONE,
                res.dohnanyi_slope,
            )
            .expect("the fraction resolves")
            .to_f64_lossy();
            assert!(
                (fraction - q).abs() < 0.02,
                "the fraction above the sampled size {} returns the quantile {q}, got {fraction}",
                d.to_f64_lossy()
            );
        }
    }

    #[test]
    fn the_swarm_is_dominated_by_small_bodies() {
        // The collisional-cascade signature carried into the sampler: a middling quantile still maps to a small
        // body, because most of the number lies at the small end (p > 1). At the median size quantile the body
        // is far below the largest (well under a tenth of it).
        let res = sun_reservoir();
        let d_median = residual_diameter_ratio(
            Fixed::from_ratio(1, 2),
            res.min_size_ratio,
            res.dohnanyi_slope,
        )
        .unwrap();
        assert!(
            d_median.to_f64_lossy() < 0.1,
            "the median body is a small fraction of the largest, got {}",
            d_median.to_f64_lossy()
        );
    }

    #[test]
    fn the_semi_major_axis_spans_the_disk_and_avoids_the_swept_gap() {
        // The orbit derivation: sweeping the mass quantile places bodies from the inner disk to the outer edge,
        // monotone in the quantile, and NEVER inside the planet's swept feeding zone (the accreted gap holds no
        // residue). This is the derived reservoir structure, not hand-placed orbits.
        let res = sun_reservoir();
        let mut previous = Fixed::ZERO;
        for i in 1..20 {
            let q = Fixed::from_ratio(i, 20);
            let a = residual_semi_major_axis(q, &res).expect("the orbit resolves");
            assert!(
                a >= res.inner_disk_edge_au && a <= res.outer_disk_edge_au,
                "the orbit stays inside the disk, got {} AU",
                a.to_f64_lossy()
            );
            assert!(
                !in_feeding_zone(&res, a),
                "no body lands in the swept feeding zone, got {} AU",
                a.to_f64_lossy()
            );
            assert!(
                a >= previous,
                "the orbit rises monotonically with the mass quantile"
            );
            previous = a;
        }
    }

    #[test]
    fn the_eccentricity_follows_the_rayleigh_stirring_law() {
        // The Rayleigh inverse: zero at the zero quantile, rising with it, and the median at sigma*sqrt(2*ln 2)
        // (~1.177*sigma), the defining Rayleigh median that proves the shape.
        let sigma = Fixed::from_ratio(1, 10);
        assert!(
            stirred_eccentricity(Fixed::ZERO, sigma)
                .unwrap()
                .to_f64_lossy()
                < 1e-6,
            "the zero quantile gives zero eccentricity"
        );
        let low = stirred_eccentricity(Fixed::from_ratio(1, 4), sigma).unwrap();
        let high = stirred_eccentricity(Fixed::from_ratio(3, 4), sigma).unwrap();
        assert!(high > low, "the eccentricity rises with the quantile");
        let median = stirred_eccentricity(Fixed::from_ratio(1, 2), sigma)
            .unwrap()
            .to_f64_lossy();
        let expected = 0.1 * (2.0_f64 * 2.0_f64.ln()).sqrt();
        assert!(
            (median - expected).abs() < 1e-3,
            "the median eccentricity is sigma*sqrt(2 ln 2) ~{expected}, got {median}"
        );
    }

    #[test]
    fn a_body_derives_rocky_near_and_icy_far() {
        // The end-to-end assembly: a low mass quantile places a body in the inner disk (inside the snow line, so
        // refractory), a high mass quantile in the outer disk (beyond the snow line, so icy), each with a
        // Rayleigh eccentricity and a cascade size. This is a discrete asteroid and a discrete comet, both
        // derived from the one disk state.
        let res = sun_reservoir();
        let ice = ice_table();
        let inner = derive_small_body(
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(3, 10),
            Fixed::from_ratio(6, 10),
            &res,
            &ice,
        )
        .expect("the inner body derives");
        assert_eq!(
            inner.volatile_class,
            VolatileClass::Refractory,
            "the inner-disk body is a rocky asteroid, at {} AU",
            inner.semi_major_axis_au.to_f64_lossy()
        );
        assert!(inner.semi_major_axis_au.to_f64_lossy() < 2.0);
        let outer = derive_small_body(
            Fixed::from_ratio(99, 100),
            Fixed::from_ratio(7, 10),
            Fixed::from_ratio(4, 10),
            &res,
            &ice,
        )
        .expect("the outer body derives");
        assert_eq!(
            outer.volatile_class,
            VolatileClass::Icy,
            "the outer-disk body is an icy comet, at {} AU",
            outer.semi_major_axis_au.to_f64_lossy()
        );
        assert!(outer.semi_major_axis_au.to_f64_lossy() > 4.0);
        // Both carry a non-negative eccentricity and a size that is a fraction of the largest body.
        for body in [inner, outer] {
            assert!(body.eccentricity >= Fixed::ZERO);
            assert!(
                body.diameter_ratio > Fixed::ZERO && body.diameter_ratio <= Fixed::ONE,
                "the diameter is a valid size ratio"
            );
        }
    }

    #[test]
    fn an_alien_reservoir_is_a_data_row() {
        // A heavier, brighter star with a steeper cascade and a wider disk: the same laws, a resolved snow line
        // pushed outward by the extra luminosity, a resolved body. No Terran assumption blocks it.
        let mut res = sun_reservoir();
        res.star_mass_ratio = Fixed::from_int(2);
        res.dohnanyi_slope = Fixed::from_ratio(38, 10);
        res.outer_disk_edge_au = Fixed::from_int(80);
        let ice = ice_table();
        let a_snow = snow_line_orbit_au(&res, &ice).expect("the alien snow line resolves");
        assert!(
            a_snow.to_f64_lossy()
                > snow_line_orbit_au(&sun_reservoir(), &ice_table())
                    .unwrap()
                    .to_f64_lossy(),
            "a brighter star pushes the snow line outward"
        );
        let body = derive_small_body(
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 2),
            &res,
            &ice,
        )
        .expect("an alien body derives");
        assert!(body.semi_major_axis_au > Fixed::ZERO);
    }

    #[test]
    fn non_physical_inputs_fail_soft() {
        let res = sun_reservoir();
        let ice = ice_table();
        // A non-positive water partial pressure has no snow-line criterion.
        assert!(ice_condenses(&ice, Fixed::from_int(180), Fixed::ZERO).is_none());
        // A quantile at or past one is out of the open unit interval.
        assert!(residual_semi_major_axis(Fixed::ONE, &res).is_none());
        assert!(stirred_eccentricity(Fixed::ONE, Fixed::from_ratio(1, 10)).is_none());
        assert!(
            residual_diameter_ratio(Fixed::ONE, res.min_size_ratio, res.dohnanyi_slope).is_none()
        );
        // A degenerate size range (min ratio at or above the largest body) has no distribution.
        assert!(
            residual_diameter_ratio(Fixed::from_ratio(1, 2), Fixed::ONE, res.dohnanyi_slope)
                .is_none()
        );
    }

    #[test]
    fn the_belt_sample_is_deterministic() {
        // The reproducibility contract: the same reservoir, seed, and count draw the same belt bit-for-bit, and a
        // different world seed draws a different belt. This is the seeded-draw determinism (Principle 3).
        let res = sun_reservoir();
        let ice = ice_table();
        let seed = 0xBEEF_CAFE_1234_5678u64;
        let a = sample_belt(&res, &[], 64, seed, &ice);
        let b = sample_belt(&res, &[], 64, seed, &ice);
        assert_eq!(
            a, b,
            "the same reservoir + seed + count reproduces the belt bit-for-bit"
        );
        assert_eq!(a.len(), 64, "a valid reservoir resolves every sampled body");
        let c = sample_belt(&res, &[], 64, seed ^ 1, &ice);
        assert_ne!(a, c, "a different world seed draws a different belt");
    }

    #[test]
    fn a_swept_zone_masks_out_bodies_in_the_belt() {
        // The swept-zone mask: a caller-supplied cleared feeding zone holds no residue, so no sampled body lands in
        // it. Proven against an unmasked belt that DOES populate the band (the mask has work to do), and the
        // reservoir's own feeding zone is avoided by the union even with an empty caller list.
        let res = sun_reservoir();
        let ice = ice_table();
        let seed = 0x5A17_BEE7_0000_0001u64;
        let band = (Fixed::from_int(8), Fixed::from_int(16));
        let in_band =
            |b: &SmallBody| b.semi_major_axis_au >= band.0 && b.semi_major_axis_au <= band.1;
        let open = sample_belt(&res, &[], 400, seed, &ice);
        assert!(
            open.iter().any(in_band),
            "without a mask the belt populates the 8-16 AU band"
        );
        let masked = sample_belt(&res, &[band], 400, seed, &ice);
        assert!(
            !masked.iter().any(in_band),
            "the swept zone masks every body out of the 8-16 AU band"
        );
        // The union also masks the reservoir's OWN feeding zone (0.9-1.1 AU), with or without a caller zone.
        assert!(
            !masked
                .iter()
                .any(|b| in_feeding_zone(&res, b.semi_major_axis_au)),
            "no body lands in the reservoir's own swept feeding zone"
        );
    }

    #[test]
    fn the_belt_split_follows_the_snow_line() {
        // The emergent asteroid-versus-comet split: every sampled body inside the derived snow line is refractory
        // (a rocky asteroid), every body beyond it is icy (a comet), and a disk that spans the line yields BOTH.
        // The split is a consequence of the snow line and the orbit spread, never an authored roster.
        let res = sun_reservoir();
        let ice = ice_table();
        let a_snow = snow_line_orbit_au(&res, &ice).expect("a snow line sits inside the disk");
        let belt = sample_belt(&res, &[], 300, 0x01CE_A57E_0000_0002u64, &ice);
        let mut refractory = 0;
        let mut icy = 0;
        for body in &belt {
            match body.volatile_class {
                VolatileClass::Refractory => {
                    assert!(
                        body.semi_major_axis_au <= a_snow,
                        "a refractory body sits inside the snow line, got {} AU",
                        body.semi_major_axis_au.to_f64_lossy()
                    );
                    refractory += 1;
                }
                VolatileClass::Icy => {
                    assert!(
                        body.semi_major_axis_au >= a_snow,
                        "an icy body sits beyond the snow line, got {} AU",
                        body.semi_major_axis_au.to_f64_lossy()
                    );
                    icy += 1;
                }
            }
        }
        assert!(
            refractory > 0 && icy > 0,
            "the belt spans the snow line: both asteroids ({refractory}) and comets ({icy}) emerge"
        );
    }

    #[test]
    fn the_belt_size_distribution_is_small_dominated() {
        // The Dohnanyi collisional-cascade signature carried into the population: the overwhelming majority of the
        // number lies at the small end, so nearly every sampled body is a small fraction of the largest. The bound
        // (four-fifths under a tenth) is conservative; the cumulative number above a tenth is ~1e-5 here.
        let res = sun_reservoir();
        let ice = ice_table();
        let belt = sample_belt(&res, &[], 400, 0x0051_2E00_0000_0003u64, &ice);
        assert!(!belt.is_empty(), "the belt is populated");
        let tenth = Fixed::from_ratio(1, 10);
        let small = belt.iter().filter(|b| b.diameter_ratio < tenth).count();
        assert!(
            small * 5 > belt.len() * 4,
            "at least four-fifths of the belt is under a tenth of the largest body, got {small}/{}",
            belt.len()
        );
    }

    #[test]
    fn the_count_is_a_display_budget_over_the_derived_physical_mass() {
        // The count-versus-population distinction: `count` is a labeled DISPLAY budget, distinct from the DERIVED
        // residual disk mass (the physical size of the reservoir). Different budgets draw different-length belts
        // from the same reservoir, the derived mass does not move with the budget, and a smaller budget is the
        // same seeded sub-sample (a prefix), the reproducibility that lets a viewer draw fewer or more at will.
        let res = sun_reservoir();
        let ice = ice_table();
        let mass = residual_disk_mass(&res).expect("the residual disk mass resolves");
        assert!(
            mass > Fixed::ZERO,
            "the reservoir carries a positive derived residual mass"
        );
        let seed = 0xB0D9_E700_0000_0004u64;
        let small = sample_belt(&res, &[], 16, seed, &ice);
        let large = sample_belt(&res, &[], 128, seed, &ice);
        assert_eq!(small.len(), 16);
        assert_eq!(large.len(), 128);
        assert_eq!(
            residual_disk_mass(&res).unwrap(),
            mass,
            "the physical mass is independent of the display count"
        );
        assert_eq!(
            &large[..16],
            &small[..],
            "a smaller budget is the same seeded sub-sample, a prefix of a larger"
        );
    }

    /// The physical body count assembled INDEPENDENTLY in f64: no fixed point, no log staging, the closed form
    /// written out in full rather than called. The residual mass is read back from the module because that
    /// midpoint sum is an input to the count rather than part of the count's own arithmetic. This twin validates
    /// the ASSEMBLY; the closed form it shares with the module is separately validated against direct quadrature
    /// in `civsim_world::impact_flux`, so the two checks do not lean on each other.
    fn body_count_log10_twin(
        residual_mass_sigma_au2: f64,
        u: f64,
        p: f64,
        rho: f64,
        d_max: f64,
    ) -> f64 {
        let au_m = 149_597_870_700.0_f64;
        let residual_kg = residual_mass_sigma_au2 * au_m * au_m;
        let mean_cube =
            ((1.0 - u.powf(4.0 - p)) / (4.0 - p)) / ((1.0 - u.powf(1.0 - p)) / (1.0 - p));
        let mean_body_kg = rho * (std::f64::consts::PI / 6.0) * d_max.powi(3) * mean_cube;
        (residual_kg / mean_body_kg).log10()
    }

    #[test]
    fn the_physical_body_count_derives_from_the_residual_mass_and_the_mean_body_mass() {
        // The derivation this slice exists for: the reservoir's own residual disk mass over the number-weighted
        // mean body mass of its own size cascade, with nothing authored between them.
        let res = sun_reservoir();
        let got = residual_body_count_log10(&res)
            .expect("the body count derives")
            .to_f64_lossy();
        let reference = body_count_log10_twin(
            residual_disk_mass(&res).unwrap().to_f64_lossy(),
            1e-3,
            3.5,
            3000.0,
            1e5,
        );
        assert!(
            (got - reference).abs() < 0.01,
            "log10 body count {got} against the independent twin {reference}"
        );
        // The magnitude, computed rather than reasoned to. This reservoir holds ~1e14 bodies, which is the whole
        // reason the count is exported as a log: Q32.32 tops out near 2.1e9, log10 ~9.33, so a linear export
        // could not have carried it and a linear intermediate would have railed silently.
        assert!(
            got > 9.34,
            "the derived count is past the fixed-point ceiling (log10 ~9.33), so the log export is load-bearing, got {got}"
        );
        assert!(
            (14.0..15.0).contains(&got),
            "a solar-nebula-grade residual disk of 100 m to 100 km bodies runs to ~1e14, got 1e{got}"
        );
        // SYSTEM-WIDE, and the gap stated as an assertion rather than only in prose: this count exceeds the
        // per-target placeholder a bombardment is drawn against by more than ten orders of magnitude, so it
        // cannot be read as a per-planet impactor count. The allocation that would bridge them (what fraction of
        // the reservoir one planet sweeps up) does not exist, and nothing here supplies it.
        assert!(
            got > 12.0,
            "the system-wide count dwarfs any per-target bombardment count; the missing piece is the allocation fraction, not the reservoir, got 1e{got}"
        );
    }

    #[test]
    fn the_body_count_scales_as_its_derivation_says_it_should() {
        // The count is `reservoir mass / mean body mass`, so it must move EXACTLY as each input moves it: linearly
        // with the surface-density normalization (which scales the mass), inversely with the bulk density, and
        // with the inverse CUBE of the largest body's diameter (which scales every body's mass). Three derived
        // relations, each a structural check that no term was dropped or double-counted in the log assembly.
        let base = residual_body_count_log10(&sun_reservoir())
            .unwrap()
            .to_f64_lossy();
        let log2 = 2.0_f64.log10();

        let mut denser_disk = sun_reservoir();
        denser_disk.surface_density_normalization = Fixed::from_int(2);
        let got = residual_body_count_log10(&denser_disk)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (got - (base + log2)).abs() < 0.01,
            "twice the surface density is twice the bodies: expected {}, got {got}",
            base + log2
        );

        let mut denser_bodies = sun_reservoir();
        denser_bodies.body_bulk_density_kg_m3 = Fixed::from_int(6000);
        let got = residual_body_count_log10(&denser_bodies)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (got - (base - log2)).abs() < 0.01,
            "twice as dense a body is half as many bodies: expected {}, got {got}",
            base - log2
        );

        let mut bigger_bodies = sun_reservoir();
        bigger_bodies.max_body_diameter_m = Fixed::from_int(200_000);
        let got = residual_body_count_log10(&bigger_bodies)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (got - (base - 3.0 * log2)).abs() < 0.01,
            "twice the diameter is an eighth of the bodies: expected {}, got {got}",
            base - 3.0 * log2
        );

        // A steeper cascade moves the mean body onto the small end, so the same mass buys MORE bodies.
        let mut steep = sun_reservoir();
        steep.dohnanyi_slope = Fixed::from_int(4);
        assert!(
            residual_body_count_log10(&steep).unwrap().to_f64_lossy() > base,
            "a steeper cascade spends the same mass on more, smaller bodies"
        );
    }

    #[test]
    fn the_body_count_follows_the_reservoir_the_planets_left() {
        // The count is of what SURVIVED accretion, so sweeping more of the disk into planets must lower it. This
        // is also the shape of the missing allocation: mass leaving the reservoir for a planet is the same edge
        // that would deliver a per-target bombardment, and here it only subtracts.
        let res = sun_reservoir();
        let whole = residual_body_count_log10(&res).unwrap();
        let swept =
            residual_body_count_log10_masked(&res, &[(Fixed::from_int(2), Fixed::from_int(20))])
                .expect("a partly swept disk still holds bodies");
        assert!(
            swept < whole,
            "clearing more feeding zones leaves fewer residual bodies"
        );
        // An alien reservoir is a data row: a steeper cascade, a different size range, a different body density.
        let mut alien = sun_reservoir();
        alien.dohnanyi_slope = Fixed::from_ratio(38, 10);
        alien.min_size_ratio = Fixed::from_ratio(1, 500);
        alien.max_body_diameter_m = Fixed::from_int(400_000);
        alien.body_bulk_density_kg_m3 = Fixed::from_int(900);
        let alien_count = residual_body_count_log10(&alien).expect("an alien reservoir counts");
        assert!(
            alien_count > Fixed::ZERO,
            "an icy, steeper, larger-bodied reservoir resolves through the same law"
        );
    }

    #[test]
    fn the_body_count_fails_soft_rather_than_fabricating_one() {
        let res = sun_reservoir();
        // A fully swept disk has no residue to count.
        let whole_disk = (Fixed::ZERO, Fixed::from_int(1000));
        assert!(residual_body_count_log10_masked(&res, &[whole_disk]).is_none());
        // The two absolute scales must be physical; neither has a defensible default.
        let mut no_size = res;
        no_size.max_body_diameter_m = Fixed::ZERO;
        assert!(residual_body_count_log10(&no_size).is_none());
        let mut no_density = res;
        no_density.body_bulk_density_kg_m3 = Fixed::ZERO;
        assert!(residual_body_count_log10(&no_density).is_none());
        // A reservoir with no small end has no size distribution to average over.
        let mut no_small_end = res;
        no_small_end.min_size_ratio = Fixed::ZERO;
        assert!(residual_body_count_log10(&no_small_end).is_none());
        // The other way round, the log staging's reach is wider than the linear helpers': a size ratio at the
        // fixed-point floor is nine and a half decades of range (a 100 km largest body down to ~23 micron grains)
        // and it still resolves, to a far larger count, because the number sits at the small end. The linear
        // cumulative fractions rail at about four decades; this is the range the log form was written for.
        let mut widest = res;
        widest.min_size_ratio = Fixed::EPSILON;
        let dusty = residual_body_count_log10(&widest)
            .expect("a nine-decade reservoir resolves in log space")
            .to_f64_lossy();
        assert!(
            dusty > residual_body_count_log10(&res).unwrap().to_f64_lossy(),
            "grinding the small end down to dust raises the body count, got 1e{dusty}"
        );
    }

    #[test]
    fn an_over_swept_disk_yields_a_short_belt_not_a_fabricated_one() {
        // Fail-soft under an aggressive mask: a swept zone covering the whole residual disk leaves no residue, so
        // every draw fails to resolve and the belt is empty rather than fabricating a body in an accreted gap.
        let res = sun_reservoir();
        let ice = ice_table();
        let whole_disk = (Fixed::ZERO, Fixed::from_int(1000));
        let belt = sample_belt(&res, &[whole_disk], 32, 0x0DEA_D000_0000_0005u64, &ice);
        assert!(
            belt.is_empty(),
            "a fully swept disk yields no bodies, never a fabricated one"
        );
    }
}
