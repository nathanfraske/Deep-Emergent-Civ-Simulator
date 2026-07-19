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

//! The GAS/ICE GIANT branch (task #73, SLICE 1): the verdict that decides, for each oligarchic embryo the
//! system generator already derives, whether it runs away into a gas giant or stays a terrestrial core, plus a
//! first-cut giant mass. Until this branch every embryo assembled into a rocky planet; the giant branch is what
//! lets a Jupiter emerge from the same disk that seeds an Earth, chosen by physics rather than authored by hand.
//!
//! The one physical idea: a giant forms when an embryo's CORE grows past a CRITICAL MASS and its gas envelope
//! then contracts fast enough to run away on the disk gas BEFORE the gas disperses. So the verdict is the meeting
//! of three derived facts, each a built substrate this module consumes rather than re-derives:
//!
//! - The CORE MASS is the embryo's own isolation mass (`Embryo::mass_earth` from
//!   [`crate::planetary_system::oligarchic_embryo_field`]), which already jumps upward across the derived ice
//!   line, so the outer embryos are the larger seeds. Nothing here re-sizes the core.
//!
//! - The CORE ACCRETION RATE `Mdot_c` is the oligarchic solid-delivery rate, DERIVED from the same inputs the
//!   embryo field uses: the local solid surface density ([`crate::planetary_system::SolidDisk`]), the Keplerian
//!   frequency ([`crate::astro::kepler_orbital_period_years`]), and the Hill radius
//!   ([`crate::astro::hill_radius_au`]). The shear-dominated capture cross-section carries the physical core
//!   radius, itself DERIVED from the core mass and its bulk density ([`crate::astro::planet_radius_m`]). The one
//!   order-unity collision coefficient is reserved with its basis, never fabricated.
//!
//! - The CRITICAL CORE MASS is the Ikoma relation `M_crit ~ 10 M_earth * (Mdot_c / 1e-6 M_earth/yr)^(1/4)`
//!   (Ikoma, Nakazawa & Emori 2000): a faster-accreting core radiates a hotter envelope, so it must grow more
//!   massive before the envelope can no longer support itself and begins to contract. The KELVIN-HELMHOLTZ
//!   contraction time is the banded power law `tau_KH ~ 10^c yr * (M_c/M_earth)^(-d) * (kappa/kappa_ref)` (Ikoma
//!   2000, Ida & Lin 2004), with the literature spread carried as the declared band (c in 8..10, d in 2..4,
//!   Ida-Lin fiducial c=9, d=3). The dominant lever is the ENVELOPE OPACITY kappa: a grain-rich envelope radiates
//!   slowly and stalls, a grain-free one runs away in well under a Myr. So kappa is not a bare number: it DERIVES
//!   from the disk's own metal fraction relative to a reference (a metal-poor disk contracts faster, a metal-rich
//!   one slower), admitting the alien as a data row, with the reference reserved rather than fabricated.
//!
//! The VERDICT is GIANT when the core is super-critical (`M_c > M_crit`) AND the envelope contracts before the
//! gas is gone (`tau_KH < disk gas lifetime`); otherwise TERRESTRIAL. The gas lifetime is the observed
//! protoplanetary-disk dispersal time, reserved with its basis. For a GIANT, the FIRST-CUT final mass adds to the
//! core the disk GAS available in the feeding annulus (the viscous-similarity gas surface density
//! [`crate::astro::viscous_similarity_surface_density`] integrated over the feeding zone, capped by that local
//! reservoir). The gap-opening and global-reservoir caps, and the Hill-zone that widens as the envelope grows,
//! are documented follow-ons for a later slice, not this one.
//!
//! Admit-the-alien (a prime directive): every per-system input is the disk's, the star's, or the embryo's own
//! datum, carried on the parameter structs, so a metal-poor nebula, a heavier star, a shorter-lived disk, or a
//! grain-depleted envelope are each a different set of numbers through the same law, never a new code path.
//! Determinism (Principle 3, Principle 10): fixed-point throughout, the pinned [`Fixed::ln`], [`Fixed::exp`], and
//! [`Fixed::sqrt`], the wide-magnitude core-accretion assembly done in LOG-SPACE (the
//! [`crate::astro::isolation_mass_earth`] precedent) so no unrepresentable intermediate forms, and the
//! Kelvin-Helmholtz comparison itself done in the log domain so the ~10^9-year timescales never overflow
//! fixed-point; a degenerate input fails soft to `None`, never a fabricated verdict. This module is DORMANT:
//! nothing here is wired into a pinned run path, so the run pins hold bit-exact.
//!
//! The value-authoring line (Principle 6). The only authored numbers are the CITED Ikoma law constants (the
//! 10 M_earth and 1e-6 M_earth/yr anchors and the 1/4 exponent of the critical-mass relation), each a
//! compute-once reference the mechanism reads, cited at its site. Everything else is DERIVED (the core mass, the
//! accretion rate, the opacity's metallicity dependence, the gas reservoir) or RESERVED-with-basis and surfaced
//! on the parameter structs (the Kelvin-Helmholtz band c and d, the reference opacity and reference metallicity,
//! the collision coefficient, the core bulk density, the disk gas lifetime, the feeding-zone width). Not one of
//! them is invented inline.

use civsim_core::Fixed;

use crate::astro::{
    disk_effective_temperature, disk_era_xray_disk_lifetime_myr, hill_radius_au,
    kepler_orbital_period_years, planet_radius_m, shu_inside_out_collapse_accretion_rate_msun_myr,
    viscous_similarity_surface_density, CollapseModel, XrayWindFit, ASTRONOMICAL_UNIT_M,
    EARTH_MASS_KG, SOLAR_MASS_KG,
};
use crate::planetary_system::{Embryo, SolidDisk};

/// The critical-core-mass ANCHOR in Earth masses: a core accreting solids at the reference rate `1e-6 M_earth/yr`
/// becomes critical (the envelope can no longer support itself) at this mass. A CITED compute-once law constant,
/// reserved for the owner's ratification with its basis the Ikoma, Nakazawa & Emori (2000) critical-core-mass
/// relation. It is the fixed form of the relation; the banded parts (the Kelvin-Helmholtz prefactor c and mass
/// exponent d) are data on [`GiantKhParams`]. The companion rate anchor (`1e-6 M_earth/yr`) and exponent (`1/4`)
/// are constructed at the site in [`critical_core_mass_earth`] (each a non-`const` exact ratio), cited there.
pub const CRIT_CORE_MASS_ANCHOR_EARTH: Fixed = Fixed::from_int(10);

/// The GAS-AND-ACCRETION parameters: the disk-lifetime and oligarchic-accretion residues the giant verdict reads,
/// each a per-system datum surfaced with its basis rather than fabricated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GiantGasParams {
    /// The disk GAS LIFETIME in megayears: the window the envelope has to run away before the gas disperses.
    /// Reserved-with-basis: the observed protoplanetary-disk dispersal time, ~1 to 10 Myr (the median class-II
    /// disk lifetime, Haisch, Lada & Lada 2001; Mamajek 2009), a per-star datum so a long-lived or short-lived
    /// disk is a data row.
    pub disk_gas_lifetime_myr: Fixed,
    /// The oligarchic COLLISION COEFFICIENT (order unity): the prefactor of the shear-dominated gravitational-
    /// focusing core accretion rate. Reserved-with-basis: the particle-in-a-box collision-rate coefficient of
    /// oligarchic growth (Ida & Nakazawa 1989; Greenzweig & Lissauer 1990), of order one. Because `M_crit`
    /// depends on `Mdot_c` only as the fourth root, this coefficient is a soft lever on the verdict.
    pub collision_coefficient: Fixed,
    /// The CORE BULK DENSITY in g/cm^3, the mean density the core radius is derived from
    /// ([`crate::astro::planet_radius_m`]). Reserved-with-basis: the core composition's mean bulk density (the
    /// materials and differentiation arc's output, ~3.3 for silicate rock through ~5.5 for a differentiated
    /// rock-metal core, lower for a rock-ice core beyond the ice line). Derive-down: read the core density from
    /// the materials arc when it wires through.
    pub core_bulk_density_g_cm3: Fixed,
    /// The FEEDING-ZONE WIDTH in Hill radii `C`: the annulus width the core sweeps solids from and the giant
    /// accretes gas from. Reserved-with-basis: the oligarchic feeding-zone width, a few to ~10 mutual Hill radii
    /// (Kokubo & Ida 1998/2000), the same `C` [`crate::astro::isolation_mass_earth`] integrates over.
    pub feeding_zone_hill_widths: Fixed,
    /// The gas-annulus integration resolution (a fixed cell count for the runaway-mass Riemann sum). An engine-
    /// accuracy bound, not a physical value, so determinism holds by construction.
    pub gas_integration_steps: u32,
}

/// The KELVIN-HELMHOLTZ (envelope-contraction) parameters: the Ikoma / Ida-Lin banded power law and the opacity
/// reference the giant verdict reads. The banded constants are data (Principle 11) so a scenario probes the
/// literature spread; the opacity DERIVES from the disk metallicity relative to the reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GiantKhParams {
    /// The Kelvin-Helmholtz LOG PREFACTOR `c` (the base-ten log of the contraction time in years at one Earth
    /// mass and the reference opacity). Reserved-with-basis: the Ikoma 2000 / Ida & Lin 2004 declared band
    /// 8 <= c <= 10, Ida-Lin fiducial c = 9; the literature spread in the envelope-contraction normalization.
    pub kh_log10_yr_c: Fixed,
    /// The Kelvin-Helmholtz MASS EXPONENT `d`: the contraction time falls as `M_c^(-d)` (a heavier envelope
    /// contracts faster). Reserved-with-basis: the Ikoma 2000 / Ida & Lin 2004 declared band 2 <= d <= 4,
    /// Ida-Lin fiducial d = 3 (Ikoma's -2.5 at the 10-M_earth anchor sits inside it).
    pub kh_mass_exponent_d: Fixed,
    /// The REFERENCE ENVELOPE OPACITY in cm^2/g the prefactor `c` is calibrated at. Reserved-with-basis: the
    /// interstellar-grain Rosseland opacity, ~1 cm^2/g, the reference Ikoma's `kappa / 1 cm^2 g^-1` is quoted
    /// against. The ACTUAL opacity derives from the disk metallicity (below), so this is the anchor, not the
    /// per-system value.
    pub reference_opacity_cm2_g: Fixed,
    /// The REFERENCE METAL FRACTION the reference opacity corresponds to (~0.0134, the solar heavy-element mass
    /// fraction, AGSS09). Reserved-with-basis. The envelope opacity is derived as
    /// `reference_opacity * (Z / reference_metal_fraction)`, so a metal-rich disk raises the opacity and slows
    /// contraction, a metal-poor disk lowers it and hastens runaway: the opacity lever, keyed to the disk's own
    /// `Z`, admitting the alien.
    pub reference_metal_fraction: Fixed,
}

/// The OUTCOME of the giant verdict: a rocky core that never ran away, or a gas giant with its first-cut mass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GiantOutcome {
    /// The embryo stays a terrestrial core (sub-critical, or the envelope could not contract before the gas
    /// dispersed).
    Terrestrial,
    /// The embryo ran away into a gas giant. `final_mass_earth` is the FIRST-CUT total mass: the core plus the
    /// disk gas swept from the feeding annulus (capped by that local reservoir; the gap-opening and global caps
    /// are a documented follow-on).
    ///
    /// TERMS-DROPPED, and it is load-bearing on the MASS (audit finding): THIS field still carries the RESERVOIR
    /// EXHAUSTION closure (the giant grows until the feeding annulus empties), an UPPER BOUND, not a predicted mass.
    /// It over-estimates: a dense-disk super-critical embryo returns tens of Jupiter masses (brown-dwarf-class). The
    /// accretion-termination MECHANISM exists as a dormant kernel: [`gap_opening_mass_earth`] (the Crida 2006
    /// gap-opening scale) caps the runaway, and [`runaway_terminated_giant_mass_earth`] applies `M_final =
    /// min(reservoir, M_gap)`, which trims the solar-row giant from ~24 to ~2 Jupiter masses. The TYPED OUTCOME also
    /// exists: [`giant_mass_class`] reads a mass against the vendored deuterium (~13 M_Jup) and hydrogen (~0.072
    /// M_sun) fusion boundaries into Planet / BrownDwarf / Star, so the terminated mass types as a planet while the
    /// un-terminated 24-M_Jup ceiling would type as a brown dwarf. The INTEGRATION is [`terminate_and_type_giant`]:
    /// it threads the disk aspect ratio at the embryo's orbit through the gap cap and the fusion classes and returns
    /// the sound [`TerminatedGiant`] (the ceiling, the cap, the terminated mass, and the class). THIS field stays
    /// the un-terminated first cut BY DESIGN, so the composition is additive and byte-neutral (no existing verdict
    /// changes); a caller wanting the sound mass reads `terminate_and_type_giant`, not `final_mass_earth`.
    Giant { final_mass_earth: Fixed },
}

/// The full VERDICT for one embryo: the outcome plus the diagnostics that produced it (the core mass and its
/// derived accretion rate, the critical mass, the Kelvin-Helmholtz time, and the derived envelope opacity), so a
/// caller can report why an embryo went the way it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GiantVerdict {
    /// The embryo's orbit (AU).
    pub orbit_au: Fixed,
    /// The core mass (Earth masses), the embryo's isolation mass.
    pub core_mass_earth: Fixed,
    /// The DERIVED oligarchic core (solid) accretion rate (Earth masses per year).
    pub core_accretion_rate_earth_per_yr: Fixed,
    /// The critical core mass (Earth masses) from the Ikoma relation at this accretion rate.
    pub critical_core_mass_earth: Fixed,
    /// The Kelvin-Helmholtz envelope-contraction time (years), the diagnostic timescale. It SATURATES to the
    /// representable ceiling for a light core whose true time crosses the fixed-point maximum (~10^9 yr); the
    /// verdict itself is decided in the log domain and never depends on this saturated read.
    pub kh_time_yr: Fixed,
    /// The DERIVED envelope opacity (cm^2/g) at this disk's metallicity.
    pub envelope_opacity_cm2_g: Fixed,
    /// The outcome.
    pub outcome: GiantOutcome,
}

/// The DISK GAS surface density (kg/m^2) at an orbit, the viscous-similarity gas the runaway envelope accretes:
/// the disk's derived two-regime temperature at the orbit fed into
/// [`crate::astro::viscous_similarity_surface_density`], the same gas column
/// [`crate::planetary_system::SolidDisk::solid_surface_density_kg_m2`] multiplies by the condensed fraction to
/// reach the solids. Reads the disk's own realization (accretion rate, star mass, viscosity, mean molecular
/// weight), no new input. `None` past the disk edge or on a non-positive input.
fn gas_surface_density_kg_m2(disk: &SolidDisk, orbit_au: Fixed) -> Option<Fixed> {
    let temperature = disk_effective_temperature(
        disk.thermal.accretion_rate_msun_myr,
        disk.thermal.star_mass_ratio,
        disk.thermal.mass_luminosity_exponent,
        orbit_au,
        disk.thermal.reprocessing_factor,
        disk.thermal.inner_boundary_factor,
        disk.thermal.t_max,
    )?;
    viscous_similarity_surface_density(
        orbit_au,
        disk.thermal.star_mass_ratio,
        disk.thermal.accretion_rate_msun_myr,
        temperature,
        disk.alpha_viscosity,
        disk.mean_molecular_weight,
    )
}

/// The OLIGARCHIC CORE (solid) ACCRETION RATE `Mdot_c` in Earth masses per year, DERIVED from the disk and orbit
/// the embryo field already reads: `Mdot_c = coeff * Sigma_solid * Omega * R_H^2 * sqrt(R_c/R_H)`, the shear-
/// dominated gravitational-focusing collision rate of oligarchic growth (Ida & Nakazawa 1989; Greenzweig &
/// Lissauer 1990). The solid density feeds the flux, the Keplerian frequency sets the encounter rate, the Hill
/// radius sets the reach, and the `sqrt(R_c/R_H)` factor is the shear-dominated capture fraction (the physical
/// core radius `R_c` derived from the core mass and its bulk density).
///
/// The product spans many decades (a solid density in kg/m^2, a frequency in rad/yr, areas in AU^2, a core radius
/// in metres), so it is assembled in LOG-SPACE (the [`crate::astro::isolation_mass_earth`] precedent): the AU and
/// Earth-mass anchors enter as their decimal-string logs, so no unrepresentable intermediate forms. The result is
/// exponentiated once. `None` on a non-positive input, a disk-edge miss, or a value past the representable range.
fn core_accretion_rate_earth_per_yr(
    disk: &SolidDisk,
    star_mass_ratio: Fixed,
    orbit_au: Fixed,
    core_mass_earth: Fixed,
    collision_coefficient: Fixed,
    core_bulk_density_g_cm3: Fixed,
) -> Option<Fixed> {
    if orbit_au <= Fixed::ZERO
        || core_mass_earth <= Fixed::ZERO
        || collision_coefficient <= Fixed::ZERO
        || core_bulk_density_g_cm3 <= Fixed::ZERO
    {
        return None;
    }
    let sigma_solid = disk.solid_surface_density_kg_m2(orbit_au)?;
    if sigma_solid <= Fixed::ZERO {
        return None;
    }
    // Omega [rad/yr] = 2*pi / P[yr], the Keplerian frequency from the log-space Kepler period (valid to the outer
    // system where a seconds-form period would overflow).
    let period_years = kepler_orbital_period_years(orbit_au, star_mass_ratio)?;
    if period_years <= Fixed::ZERO {
        return None;
    }
    let two_pi = Fixed::PI.checked_add(Fixed::PI)?;
    let hill = hill_radius_au(orbit_au, core_mass_earth, star_mass_ratio)?;
    if hill <= Fixed::ZERO {
        return None;
    }
    let core_radius_m = planet_radius_m(core_mass_earth, core_bulk_density_g_cm3)?;
    if core_radius_m <= Fixed::ZERO {
        return None;
    }
    let ln_au = civsim_physics::saha::ln_of_decimal(ASTRONOMICAL_UNIT_M)?;
    let ln_earth = civsim_physics::saha::ln_of_decimal(EARTH_MASS_KG)?;
    // ln Sigma [M_earth/AU^2] = ln Sigma[kg/m^2] + 2*ln(AU_m) - ln(M_earth_kg): the solid surface density folded
    // from kg/m^2 into Earth masses per AU^2, so the whole rate lands in Earth-mass, AU, year units.
    let ln_sigma_mau = sigma_solid
        .ln()
        .checked_add(Fixed::from_int(2).checked_mul(ln_au)?)?
        .checked_sub(ln_earth)?;
    // ln Omega [rad/yr] = ln(2*pi) - ln(P[yr]).
    let ln_omega = two_pi.ln().checked_sub(period_years.ln())?;
    // ln R_c [AU] = ln R_c[m] - ln(AU_m).
    let ln_core_radius_au = core_radius_m.ln().checked_sub(ln_au)?;
    // ln Mdot = ln coeff + ln Sigma + ln Omega + 1.5*ln R_H + 0.5*ln R_c (the R_H^2 * sqrt(R_c/R_H) folded to
    // R_H^1.5 * R_c^0.5).
    let three_halves = Fixed::from_ratio(3, 2);
    let half = Fixed::from_ratio(1, 2);
    let ln_mdot = collision_coefficient
        .ln()
        .checked_add(ln_sigma_mau)?
        .checked_add(ln_omega)?
        .checked_add(three_halves.checked_mul(hill.ln())?)?
        .checked_add(half.checked_mul(ln_core_radius_au)?)?;
    // Fail loud past the representable exp ceiling rather than let `exp` saturate (the astro log-space precedent):
    // ln(2^31) = 31*ln2 is the log of the representation's own maximum, an engine bound.
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_mdot >= ln_ceiling {
        return None;
    }
    Some(ln_mdot.exp())
}

/// The CRITICAL CORE MASS in Earth masses from the Ikoma relation
/// `M_crit = 10 M_earth * (Mdot_c / 1e-6 M_earth/yr)^(1/4)` (Ikoma, Nakazawa & Emori 2000). Computed through the
/// log domain so the fourth root reads cleanly: `M_crit = anchor * exp(exponent * (ln Mdot_c - ln rate_anchor))`.
/// `None` on a non-positive accretion rate or a value past the representable range.
fn critical_core_mass_earth(core_accretion_rate_earth_per_yr: Fixed) -> Option<Fixed> {
    if core_accretion_rate_earth_per_yr <= Fixed::ZERO {
        return None;
    }
    // The CITED Ikoma (2000) rate anchor 1e-6 M_earth/yr and exponent 1/4, constructed as exact ratios at the
    // site (neither is `const`-constructible in fixed-point). Compute-once law constants, reserved-with-basis.
    let rate_anchor_earth_per_yr = Fixed::from_ratio(1, 1_000_000);
    let exponent = Fixed::from_ratio(1, 4);
    let ln_ratio = core_accretion_rate_earth_per_yr
        .ln()
        .checked_sub(rate_anchor_earth_per_yr.ln())?;
    let scale = exponent.checked_mul(ln_ratio)?.exp();
    CRIT_CORE_MASS_ANCHOR_EARTH.checked_mul(scale)
}

/// The natural log of the Kelvin-Helmholtz envelope-contraction time in years,
/// `ln tau_KH = c*ln(10) - d*ln(M_c/M_earth) + ln(kappa/kappa_ref)`, the banded Ikoma / Ida-Lin power law with the
/// opacity lever. Kept in the log domain because `tau_KH` reaches ~10^9 yr (past fixed-point) for a light core;
/// the verdict compares this against the log of the gas lifetime, so the timescale never has to be represented.
/// The opacity ratio `kappa/kappa_ref` equals the metallicity ratio `Z/Z_ref` (the reference opacity cancels), so
/// a metal-rich disk lengthens the contraction and a metal-poor disk shortens it. `None` on a non-positive input.
fn ln_kh_time_years(
    core_mass_earth: Fixed,
    metal_fraction: Fixed,
    kh: &GiantKhParams,
) -> Option<Fixed> {
    if core_mass_earth <= Fixed::ZERO
        || metal_fraction <= Fixed::ZERO
        || kh.reference_metal_fraction <= Fixed::ZERO
    {
        return None;
    }
    let ln_ten = Fixed::from_int(10).ln();
    let ln_opacity_ratio = metal_fraction
        .ln()
        .checked_sub(kh.reference_metal_fraction.ln())?;
    kh.kh_log10_yr_c
        .checked_mul(ln_ten)?
        .checked_sub(kh.kh_mass_exponent_d.checked_mul(core_mass_earth.ln())?)?
        .checked_add(ln_opacity_ratio)
}

/// The DISK GAS MASS in the feeding annulus, in Earth masses: the runaway reservoir. It integrates
/// `2*pi*r*Sigma_gas(r) dr` over `[orbit - C*R_H/2, orbit + C*R_H/2]` (the same feeding-zone width the core swept)
/// by a bounded midpoint Riemann sum (a fixed cell count, determinism by construction), with `Sigma_gas` the
/// viscous-similarity gas density, then folds the `(kg/m^2)*AU^2` integral to Earth masses through
/// [`crate::astro::feeding_zone_mass_earth`]. This is the LOCAL reservoir cap: the giant cannot accrete more gas
/// than the annulus holds. `None` on a degenerate annulus, a disk-edge miss, or an accumulation past the range.
fn feeding_zone_gas_mass_earth(
    disk: &SolidDisk,
    star_mass_ratio: Fixed,
    orbit_au: Fixed,
    core_mass_earth: Fixed,
    feeding_zone_hill_widths: Fixed,
    steps: u32,
) -> Option<Fixed> {
    if orbit_au <= Fixed::ZERO || feeding_zone_hill_widths <= Fixed::ZERO || steps == 0 {
        return None;
    }
    let hill = hill_radius_au(orbit_au, core_mass_earth, star_mass_ratio)?;
    let half_width = feeding_zone_hill_widths
        .checked_mul(hill)?
        .checked_div(Fixed::from_int(2))?;
    let inner_au = orbit_au.checked_sub(half_width)?;
    let outer_au = orbit_au.checked_add(half_width)?;
    if inner_au <= Fixed::ZERO || outer_au <= inner_au {
        return None;
    }
    let span = outer_au.checked_sub(inner_au)?;
    let dr = span.checked_div(Fixed::from_int(steps as i32))?;
    let half_dr = dr.checked_div(Fixed::from_int(2))?;
    let two_pi = Fixed::PI.checked_add(Fixed::PI)?;
    let mut integral = Fixed::ZERO;
    for i in 0..steps {
        let offset = dr
            .checked_mul(Fixed::from_int(i as i32))?
            .checked_add(half_dr)?;
        let r = inner_au.checked_add(offset)?;
        let sigma_gas = gas_surface_density_kg_m2(disk, r)?;
        let ring = two_pi
            .checked_mul(r)?
            .checked_mul(sigma_gas)?
            .checked_mul(dr)?;
        integral = integral.checked_add(ring)?;
    }
    crate::astro::feeding_zone_mass_earth(integral)
}

/// The DISK GAS CONTENT over `[inner_au, outer_au]`: the total gas mass in Earth masses AND its angular
/// momentum in the assembly's `m * sqrt(a)` proxy (Earth-mass times sqrt(AU)), both by midpoint quadrature over
/// the SAME static viscous-similarity surface density `Sigma(r)` the temperature and giant-drain code already
/// read. Each ring `2*pi*r*Sigma(r)*dr` folds to Earth masses through [`crate::astro::feeding_zone_mass_earth`],
/// then the ring carries `ring_mass * sqrt(r)` of proxy angular momentum. This is arithmetic over data in hand,
/// not the time-evolving disk (that arc is separate), so it lets the DiskGas ledger open its (mass, momentum)
/// snapshot DERIVED from the profile rather than as two free reserved scalars. `None` on a degenerate domain, a
/// disk-edge miss, or an accumulation past the representable range.
pub fn disk_gas_content(
    disk: &SolidDisk,
    inner_au: Fixed,
    outer_au: Fixed,
    steps: u32,
) -> Option<(Fixed, Fixed)> {
    if inner_au <= Fixed::ZERO || outer_au <= inner_au || steps == 0 {
        return None;
    }
    let span = outer_au.checked_sub(inner_au)?;
    let dr = span.checked_div(Fixed::from_int(steps as i32))?;
    let half_dr = dr.checked_div(Fixed::from_int(2))?;
    let two_pi = Fixed::PI.checked_add(Fixed::PI)?;
    let mut mass = Fixed::ZERO;
    let mut proxy_l = Fixed::ZERO;
    for i in 0..steps {
        let r = inner_au
            .checked_add(dr.checked_mul(Fixed::from_int(i as i32))?)?
            .checked_add(half_dr)?;
        let sigma_gas = gas_surface_density_kg_m2(disk, r)?;
        let ring = two_pi
            .checked_mul(r)?
            .checked_mul(sigma_gas)?
            .checked_mul(dr)?;
        let ring_mass = crate::astro::feeding_zone_mass_earth(ring)?;
        mass = mass.checked_add(ring_mass)?;
        proxy_l = proxy_l.checked_add(ring_mass.checked_mul(r.sqrt())?)?;
    }
    Some((mass, proxy_l))
}

/// The TRUNCATION GAS LEDGER: the birth disk's gas mass and proxy angular momentum PARTITIONED at the resonant
/// truncation radius `R_t`, in the same (Earth-mass, Earth-mass times sqrt(AU)) units [`disk_gas_content`] reports.
/// Capping the birth scale radius `R_1` to `R_t = f * R_L` (via [`crate::astro::tidally_capped_scale_radius_au`])
/// moves only the radius that feeds `t_visc`; the gas budget that radius implies never saw the cut. This ledger
/// closes that gap by reading the SAME static viscous-similarity profile over the birth domain `[inner, R_1]` and
/// splitting each quadrature ring into `retained` (midpoint inside `R_t`) or `removed` (midpoint between `R_t` and
/// `R_1`). Because the split is a PARTITION of one quadrature sum, `retained + removed == total` holds EXACTLY,
/// bit for bit, so the conservation account carries no numerical leak: the only approximation is the shared
/// midpoint quadrature of the profile, which both readings below inherit equally.
///
/// The ledger is INTERPRETATION-NEUTRAL: it reports the partition and leaves the physical reading to the owner,
/// because the two readings demand different bookkeeping and the choice is a design call, not a fact the profile
/// settles. Under the INITIAL-CONDITION reading the disk is born already truncated, so `retained` IS its whole gas
/// budget and `removed` is gas that never formed (the birth profile is the normalization over `[inner, R_t]`); no
/// sink is owed. Under the DYNAMIC reading the disk forms at `R_1` and the companion strips the outer gas over
/// time, so `removed` mass and proxy angular momentum are a real outflow that owes a named sink (accreted onto the
/// companion, fed to a circumbinary reservoir, or viscously spread back inward), and leaving it unsunk would break
/// the system's mass and angular-momentum budget. The ledger surfaces exactly the quantity each reading needs:
/// `retained` for the first, `removed` for the second. `None` on a degenerate domain, a disk-edge miss, or an
/// accumulation past the representable range, matching [`disk_gas_content`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TruncationGasLedger {
    /// Gas mass (Earth masses) in rings whose midpoint lies inside `R_t`: the truncated disk's retained budget.
    pub retained_mass_earth: Fixed,
    /// Proxy angular momentum (Earth-mass times sqrt(AU)) of the retained rings.
    pub retained_proxy_l: Fixed,
    /// Gas mass (Earth masses) in rings whose midpoint lies between `R_t` and `R_1`: the truncation residual.
    pub removed_mass_earth: Fixed,
    /// Proxy angular momentum (Earth-mass times sqrt(AU)) of the removed rings, the residual that owes a sink under
    /// the dynamic reading.
    pub removed_proxy_l: Fixed,
    /// Gas mass (Earth masses) of the full birth disk over `[inner, R_1]`, equal to `retained + removed` exactly.
    pub total_mass_earth: Fixed,
    /// Proxy angular momentum (Earth-mass times sqrt(AU)) of the full birth disk, equal to the sum of the two parts.
    pub total_proxy_l: Fixed,
}

// @derives: the disk-truncation gas/angular-momentum residual ledger <- the static viscous-similarity gas profile partitioned at the resonant truncation radius R_t=f*R_L, the retained budget and the removed residual an interpretation-neutral conservation account for the binarity cap
/// Partition the birth disk's gas content at the truncation radius. One midpoint-quadrature pass over
/// `[inner_au, birth_r1_au]` (the same ring math as [`disk_gas_content`], so the `total` here reconstructs
/// `disk_gas_content(disk, inner_au, birth_r1_au, steps)` to the ring), bucketing each ring by whether its midpoint
/// falls inside `truncation_radius_au`. A wide or absent companion (`truncation_radius_au >= birth_r1_au`) leaves
/// every ring retained and `removed` zero, the untruncated identity. `None` on a non-positive or inverted domain,
/// a zero step count, a disk-edge miss, or an overflow.
pub fn truncation_gas_ledger(
    disk: &SolidDisk,
    inner_au: Fixed,
    birth_r1_au: Fixed,
    truncation_radius_au: Fixed,
    steps: u32,
) -> Option<TruncationGasLedger> {
    if inner_au <= Fixed::ZERO
        || birth_r1_au <= inner_au
        || truncation_radius_au <= Fixed::ZERO
        || steps == 0
    {
        return None;
    }
    let span = birth_r1_au.checked_sub(inner_au)?;
    let dr = span.checked_div(Fixed::from_int(steps as i32))?;
    let half_dr = dr.checked_div(Fixed::from_int(2))?;
    let two_pi = Fixed::PI.checked_add(Fixed::PI)?;
    let mut retained_mass_earth = Fixed::ZERO;
    let mut retained_proxy_l = Fixed::ZERO;
    let mut removed_mass_earth = Fixed::ZERO;
    let mut removed_proxy_l = Fixed::ZERO;
    for i in 0..steps {
        let r = inner_au
            .checked_add(dr.checked_mul(Fixed::from_int(i as i32))?)?
            .checked_add(half_dr)?;
        let sigma_gas = gas_surface_density_kg_m2(disk, r)?;
        let ring = two_pi
            .checked_mul(r)?
            .checked_mul(sigma_gas)?
            .checked_mul(dr)?;
        let ring_mass = crate::astro::feeding_zone_mass_earth(ring)?;
        let ring_proxy_l = ring_mass.checked_mul(r.sqrt())?;
        if r < truncation_radius_au {
            retained_mass_earth = retained_mass_earth.checked_add(ring_mass)?;
            retained_proxy_l = retained_proxy_l.checked_add(ring_proxy_l)?;
        } else {
            removed_mass_earth = removed_mass_earth.checked_add(ring_mass)?;
            removed_proxy_l = removed_proxy_l.checked_add(ring_proxy_l)?;
        }
    }
    let total_mass_earth = retained_mass_earth.checked_add(removed_mass_earth)?;
    let total_proxy_l = retained_proxy_l.checked_add(removed_proxy_l)?;
    Some(TruncationGasLedger {
        retained_mass_earth,
        retained_proxy_l,
        removed_mass_earth,
        removed_proxy_l,
        total_mass_earth,
        total_proxy_l,
    })
}

/// The GAP-OPENING MODEL: the cited coefficients of the Crida, Morbidelli and Masset (2006) combined
/// thermal-viscous gap-opening criterion, the fixed physics [`gap_opening_mass_earth`] solves. The FORM is fixed
/// Rust; these are the paper's own numbers (Eq. 15), a declared model the way [`crate::astro::CollapseModel`]
/// carries its members, so a recalibration is a data row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GapOpeningModel {
    /// The gravity (thermal) term coefficient, `3/4` (Crida 2006 Eq. 15, the `(3/4)(H/R_H)` term).
    pub gravity_term_coeff: Fixed,
    /// The viscous term coefficient, `50` (Crida 2006 Eq. 15, the `50/(q R)` term).
    pub viscous_term_coeff: Fixed,
    /// The Hill-radius denominator `3` in `R_H = r (q/3)^(1/3)` (Crida 2006 Sec. 2.2, Goldreich and Tremaine 1980).
    pub hill_factor_denominator: Fixed,
}

impl GapOpeningModel {
    /// Crida, Morbidelli and Masset 2006, the vendored coefficients (Icarus 181, 587; arXiv astro-ph/0511082;
    /// receipt in the disk_arc_literature manifest): the gap opens when `P = (3/4)(H/R_H) + 50/(q R) <= 1` (Eq. 15),
    /// with the Hill radius `R_H = r (q/3)^(1/3)`.
    pub fn crida_2006() -> Self {
        Self {
            gravity_term_coeff: Fixed::from_ratio(3, 4),
            viscous_term_coeff: Fixed::from_int(50),
            hill_factor_denominator: Fixed::from_int(3),
        }
    }
}

/// The DISK REYNOLDS NUMBER `R = r^2 Omega / nu` at the planet's orbit, formed from the Shakura-Sunyaev alpha and
/// the disk aspect ratio by the standard alpha-disk substitution `nu = alpha c_s H` (Shakura and Sunyaev 1973),
/// which gives `R = 1 / (alpha (H/r)^2)`. This is the bridge the [`gap_opening_mass_earth`] criterion needs, kept
/// SEPARATE and sourced to its true origin: Crida 2006 works in a constant kinematic viscosity and never names alpha, so the
/// alpha-to-Reynolds conversion is the standard alpha-disk relation (a derivation, coefficient one), NOT a Crida
/// claim. The caller composes this with the gap criterion. `None` on a non-positive input or an overflow (an
/// inviscid disk, `alpha -> 0`, sends `R` past the representable range and fails loud rather than saturating).
pub fn alpha_disk_reynolds_number(
    alpha_viscosity: Fixed,
    disk_aspect_ratio_h_over_r: Fixed,
) -> Option<Fixed> {
    if alpha_viscosity <= Fixed::ZERO || disk_aspect_ratio_h_over_r <= Fixed::ZERO {
        return None;
    }
    let denom = alpha_viscosity
        .checked_mul(disk_aspect_ratio_h_over_r)?
        .checked_mul(disk_aspect_ratio_h_over_r)?;
    Fixed::ONE.checked_div(denom)
}

/// The GAP-OPENING MASS `M_gap` (Earth masses): the planet mass at which a giant opens a gap in the disk, DERIVED
/// by solving the Crida, Morbidelli and Masset (2006) criterion `P(q) = (3/4)(H/R_H) + 50/(q R) = 1` for the
/// mass ratio `q = M_p/M_star`, with `R_H = r (q/3)^(1/3)` so `H/R_H = (H/r)/(q/3)^(1/3)`. Runaway gas accretion
/// halts near this mass: once the gap opens, the disk can no longer feed the planet at the runaway rate, so
/// `M_gap` is the accretion-TERMINATION scale that caps the reservoir-exhaustion upper bound the giant gate
/// otherwise reports (the audit finding that a super-critical embryo returns tens of Jupiter masses). The
/// terminated final mass is `min(reservoir-exhaustion mass, M_gap)`, [`runaway_terminated_giant_mass_earth`].
///
/// The value line: every physical input is an EXPLICIT named argument (the [`crate::astro::centrifugal_radius_au`]
/// precedent), so the kernel owns the criterion and nothing else. The disk aspect ratio `H/r` and the Reynolds
/// number `R` (from [`alpha_disk_reynolds_number`], the standard alpha-disk bridge) are the disk state; the
/// coefficients are the cited [`GapOpeningModel`]. Zero fabricated values. `P(q)` decreases monotonically in `q`
/// (both terms fall as the planet grows), so the gap-opening mass is the single crossing `P = 1`, found by
/// bounded bisection over `q in [1e-6, 0.1]`. `None` if the bracket does not straddle the crossing (no gap-opening
/// mass exists in range for these disk conditions), a fail-loud rather than a fabricated root.
///
/// DECLARED CLOSURE, not a Crida claim: identifying gap-opening (`P <= 1`, a SURFACE-DENSITY criterion) with the
/// HALT of runaway gas accretion is the standard modeling assumption the caller applies, stated here rather than
/// implied. Crida 2006 also works in a constant kinematic viscosity (the alpha bridge is [`alpha_disk_reynolds_number`]'s,
/// not Crida's), for a 2D non-migrating giant with `q << 1`, the regime the caller stays inside. The two asymptotes
/// cross-check the solve: the thermal (inviscid) limit `q = 3(H/r)^3` (Hill radius equal to the scale height) and
/// the viscous limit (Crida Eq. 3), both self-anchored in the vendored bytes.
///
// @derives: the giant-planet gap-opening mass M_gap <- the Crida 2006 thermal-viscous gap criterion P(q)=(3/4)(H/R_H)+50/(qR)=1 solved for the mass ratio, over the disk aspect ratio and Reynolds number, the accretion-termination scale
pub fn gap_opening_mass_earth(
    disk_aspect_ratio_h_over_r: Fixed,
    reynolds_number: Fixed,
    star_mass_ratio: Fixed,
    model: &GapOpeningModel,
) -> Option<Fixed> {
    if disk_aspect_ratio_h_over_r <= Fixed::ZERO
        || reynolds_number <= Fixed::ZERO
        || star_mass_ratio <= Fixed::ZERO
        || model.hill_factor_denominator <= Fixed::ZERO
    {
        return None;
    }
    let third = Fixed::from_ratio(1, 3);
    // P(q) = gravity*(H/r)/(q/hill_denom)^(1/3) + viscous/(q*R). Monotone decreasing in q.
    let p_of_q = |q: Fixed| -> Option<Fixed> {
        if q <= Fixed::ZERO {
            return None;
        }
        let hill = q.checked_div(model.hill_factor_denominator)?.powf(third); // (q/3)^(1/3)
        if hill <= Fixed::ZERO {
            return None;
        }
        let h_over_rh = disk_aspect_ratio_h_over_r.checked_div(hill)?;
        let gravity_term = model.gravity_term_coeff.checked_mul(h_over_rh)?;
        let viscous_term = model
            .viscous_term_coeff
            .checked_div(q.checked_mul(reynolds_number)?)?;
        gravity_term.checked_add(viscous_term)
    };
    let q_lo = Fixed::from_ratio(1, 1_000_000); // 1e-6, sub-gap (P > 1)
    let q_hi = Fixed::from_ratio(1, 10); // 0.1, super-gap (P < 1)
                                         // The bracket must straddle the crossing: no gap at the low end, a gap at the high end. Fail loud otherwise.
    if !(p_of_q(q_lo)? > Fixed::ONE && p_of_q(q_hi)? < Fixed::ONE) {
        return None;
    }
    let mut lo = q_lo;
    let mut hi = q_hi;
    for _ in 0..60 {
        let mid = lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?;
        if p_of_q(mid)? > Fixed::ONE {
            lo = mid; // still sub-gap, the opening mass is larger
        } else {
            hi = mid;
        }
    }
    let q_gap = lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?;
    // M_gap[earth] = q_gap * M_star * (M_sun / M_earth), the ratio formed in the log domain (both masses overflow).
    let ln_sun_over_earth = civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?
        .checked_sub(civsim_physics::saha::ln_of_decimal(EARTH_MASS_KG)?)?;
    let sun_to_earth = ln_sun_over_earth.exp();
    q_gap
        .checked_mul(star_mass_ratio)?
        .checked_mul(sun_to_earth)
}

/// The RUNAWAY-TERMINATED giant mass (Earth masses): the accretion-termination rule applied, `M_final =
/// min(reservoir-exhaustion mass, gap-opening mass)`. The giant grows by runaway gas accretion until either the
/// feeding-zone reservoir empties or it opens a gap ([`gap_opening_mass_earth`]) and chokes its own supply,
/// whichever comes first. This RETIRES the reservoir-exhaustion upper bound the giant gate reports (the audit
/// finding): a super-critical embryo whose reservoir would carry it to tens of Jupiter masses is instead capped at
/// the gap-opening mass, a Jupiter-class value below the deuterium boundary, so the deuterium-versus-brown-dwarf
/// typed outcome becomes sound on a real mass. TERMS DROPPED: the slow gap-limited accretion that continues after
/// the gap opens (a modest growth past `M_gap` over the remaining disk lifetime `tau_disk`) is omitted, so this is
/// a lower-leaning bound in the gap-limited regime; naming it is the throttled-supply follow-on. `None` on a
/// non-positive input.
pub fn runaway_terminated_giant_mass_earth(
    reservoir_exhaustion_mass_earth: Fixed,
    gap_opening_mass_earth: Fixed,
) -> Option<Fixed> {
    if reservoir_exhaustion_mass_earth <= Fixed::ZERO || gap_opening_mass_earth <= Fixed::ZERO {
        return None;
    }
    Some(
        if reservoir_exhaustion_mass_earth < gap_opening_mass_earth {
            reservoir_exhaustion_mass_earth
        } else {
            gap_opening_mass_earth
        },
    )
}

/// IAU nominal Jupiter mass (kg), the unit the deuterium-burning line is quoted in. Local to the giant mass
/// classification; the sibling [`crate::astro::EARTH_MASS_KG`] and [`crate::astro::SOLAR_MASS_KG`] live in astro.
const JUPITER_MASS_KG: &str = "1.89813e27";

/// The FUSION MASS BOUNDARIES that type a substellar body, cited and vendored: the deuterium-burning minimum mass
/// (the giant-planet / brown-dwarf line) and the hydrogen-burning minimum mass (the brown-dwarf / star line). The
/// values are the paper fiducials; a declared model the way [`crate::astro::CollapseModel`] carries its members.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BurningLimits {
    /// The deuterium-burning minimum mass (Jupiter masses). Below it a body never fuses deuterium: a giant planet.
    pub deuterium_limit_m_jup: Fixed,
    /// The hydrogen-burning minimum mass (solar masses). At or above it a body sustains hydrogen fusion: a star.
    pub hydrogen_limit_m_sun: Fixed,
}

impl BurningLimits {
    /// The vendored fiducials (receipts in the disk_arc_literature manifest): the deuterium line ~13 M_Jup
    /// (Spiegel, Burrows and Milsom 2011, the 50-percent-burn fiducial, `13.0 +/- 0.8 M_Jup`) and the
    /// hydrogen line ~0.072 M_sun (Chabrier, Baraffe, Allard and Hauschildt 2000, the grain-free HBMM). The
    /// COMPOSITION dependence is the named debt: the deuterium line runs 11.0 to 16.3 M_Jup across helium fraction,
    /// deuterium abundance, metallicity, and burn fraction (Spiegel Table 1), and the hydrogen line rises toward
    /// ~0.08 to 0.09 M_sun at low metallicity (channel-relayed, not in the vendored Chabrier bytes). The metallicity
    /// axis of the deuterium line is now DERIVED per world ([`BurningLimits::from_metallicity`] via
    /// [`deuterium_burning_limit_m_jup`]); the helium-fraction and burn-fraction axes and the hydrogen line's
    /// metallicity rise remain the follow-on. This fiducial holds all at solar.
    pub fn spiegel_chabrier() -> Self {
        Self {
            deuterium_limit_m_jup: Fixed::from_int(13),
            hydrogen_limit_m_sun: Fixed::from_ratio(72, 1000),
        }
    }

    /// The per-world limits with the deuterium line DERIVED from the disk metallicity
    /// ([`deuterium_burning_limit_m_jup`]) rather than held at the solar fiducial, so a metal-rich world (a lower
    /// deuterium line) types a giant of a given mass differently from a metal-poor one (admit the alien). The
    /// hydrogen line stays the Chabrier solar fiducial (its metallicity rise is the channel-relayed named debt).
    /// `None` on a non-physical metallicity.
    pub fn from_metallicity(metallicity_z_solar: Fixed, deuterium: &DeuteriumLine) -> Option<Self> {
        Some(Self {
            deuterium_limit_m_jup: deuterium_burning_limit_m_jup(metallicity_z_solar, deuterium)?,
            hydrogen_limit_m_sun: Fixed::from_ratio(72, 1000),
        })
    }
}

/// The typed substellar OUTCOME: which side of the two fusion boundaries a mass falls on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GiantMassClass {
    /// Below the deuterium-burning line: a giant planet.
    Planet,
    /// Between the deuterium and hydrogen lines: a brown dwarf (fuses deuterium, never hydrogen).
    BrownDwarf,
    /// At or above the hydrogen-burning line: a star.
    Star,
}

/// The TYPED giant outcome (Planet / BrownDwarf / Star) for a body of a given mass, DERIVED by comparing the mass
/// to the two vendored fusion boundaries ([`BurningLimits`]). This is what makes the deuterium bound SOUND on the
/// terminated mass (the audit finding): a value crossing the ~13 M_Jup deuterium line is TYPED a brown dwarf, not
/// reported as a giant planet, and the terminated ~2 M_Jup mass ([`runaway_terminated_giant_mass_earth`]) types as
/// a planet. The mass enters in Earth masses (the giant gate's unit); the boundaries convert through the cited
/// Jupiter and solar masses in the log domain (both overflow Q32.32 raw).
///
/// The value line: the boundaries are the caller's cited [`BurningLimits`], the mass is per-body data (admit the
/// alien), and the classification is fixed Rust. Zero fabricated values. TERMS DROPPED: the composition dependence
/// of both lines (the deuterium line's 11-to-16 M_Jup range, the hydrogen line's metallicity rise) is folded into
/// the fiducial limits; a per-world derivation from the drawn composition is the named debt on [`BurningLimits`].
/// `None` on a non-physical input.
pub fn giant_mass_class(mass_earth: Fixed, limits: &BurningLimits) -> Option<GiantMassClass> {
    if mass_earth <= Fixed::ZERO
        || limits.deuterium_limit_m_jup <= Fixed::ZERO
        || limits.hydrogen_limit_m_sun <= Fixed::ZERO
    {
        return None;
    }
    let ln_earth = civsim_physics::saha::ln_of_decimal(EARTH_MASS_KG)?;
    let jup_to_earth = civsim_physics::saha::ln_of_decimal(JUPITER_MASS_KG)?
        .checked_sub(ln_earth)?
        .exp();
    let sun_to_earth = civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?
        .checked_sub(ln_earth)?
        .exp();
    let deuterium_line_earth = limits.deuterium_limit_m_jup.checked_mul(jup_to_earth)?;
    let hydrogen_line_earth = limits.hydrogen_limit_m_sun.checked_mul(sun_to_earth)?;
    Some(if mass_earth < deuterium_line_earth {
        GiantMassClass::Planet
    } else if mass_earth < hydrogen_line_earth {
        GiantMassClass::BrownDwarf
    } else {
        GiantMassClass::Star
    })
}

/// The METALLICITY track of the deuterium-burning limit, the Spiegel (2011) COOLTLUSTY sequence (Table 1, rows
/// T0.3 / T1 / T3) at the fiducial helium fraction `Y = 0.25` and the 50-percent-burn criterion: three cited
/// anchors of the limit mass `M_D` (Jupiter masses) at 0.316, 1, and 3.16 times solar metallicity. A declared
/// model the way [`BurningLimits`] and [`GapOpeningModel`] carry theirs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DeuteriumLine {
    /// `M_D` (Jupiter masses) at `0.316` solar metallicity (Spiegel 2011 Table 1, T0.3, 50-percent burn).
    pub m_jup_at_third_solar: Fixed,
    /// `M_D` at solar metallicity (T1).
    pub m_jup_at_solar: Fixed,
    /// `M_D` at `3.16` solar metallicity (T3).
    pub m_jup_at_triple_solar: Fixed,
}

impl DeuteriumLine {
    /// The vendored Spiegel 2011 COOLTLUSTY metallicity track (Table 1, receipt in the disk_arc_literature
    /// manifest): `M_D(50%) = 13.77 / 13.48 / 13.13 M_Jup` at `0.316 / 1 / 3.16` solar metallicity, the sequence the
    /// paper itself uses to build its metallicity-dependent limit (Table 3).
    pub fn spiegel_cooltlusty() -> Self {
        Self {
            m_jup_at_third_solar: Fixed::from_ratio(1377, 100),
            m_jup_at_solar: Fixed::from_ratio(1348, 100),
            m_jup_at_triple_solar: Fixed::from_ratio(1313, 100),
        }
    }
}

/// The DEUTERIUM-BURNING LIMIT `M_D` (Jupiter masses) at a given metallicity, DERIVED by interpolating the cited
/// Spiegel (2011) COOLTLUSTY track ([`DeuteriumLine`]) log-linearly in metallicity. Higher metallicity gives a
/// LOWER limit (a metal-rich object's higher atmospheric opacity blankets the core, so it reaches deuterium fusion
/// at a lower mass), so a metal-rich world's giant crosses into brown-dwarf territory at a lower mass than a
/// metal-poor world's, the admit-the-alien behaviour the fiducial `~13 M_Jup` folds away.
///
/// The metallicity is the caller's per-world data (in solar units); the anchors are the cited track. Interpolation
/// is piecewise-linear in `log10(Z)` between the three anchors (`0.316`, `1`, `3.16` solar) and CLAMPED at the
/// endpoints. TERMS DROPPED: beyond the `0.316` to `3.16` solar span the clamp holds the endpoint value, so a very
/// metal-poor object's higher limit (the pure-metallicity sequence reaches `~15.4 M_Jup` at zero metal) is the
/// named debt; the helium-fraction and burn-fraction axes are held at the fiducial (the He- and D-sequences and the
/// `~0.8-0.9 M_Jup`-per-cutoff shift are the further debts); and the track co-varies the deuterium abundance with
/// metallicity along the paper's own realistic sequence. `None` on a non-physical metallicity.
pub fn deuterium_burning_limit_m_jup(
    metallicity_z_solar: Fixed,
    line: &DeuteriumLine,
) -> Option<Fixed> {
    if metallicity_z_solar <= Fixed::ZERO {
        return None;
    }
    let half = Fixed::from_ratio(1, 2);
    let neg_half = Fixed::ZERO.checked_sub(half)?;
    // log10(Z/Z_sun); the anchors sit at log10 = -0.5, 0, +0.5.
    let log_z = metallicity_z_solar
        .ln()
        .checked_div(Fixed::from_int(10).ln())?;
    let m = if log_z <= neg_half {
        line.m_jup_at_third_solar
    } else if log_z <= Fixed::ZERO {
        let frac = log_z.checked_sub(neg_half)?.checked_div(half)?;
        line.m_jup_at_third_solar.checked_add(
            frac.checked_mul(line.m_jup_at_solar.checked_sub(line.m_jup_at_third_solar)?)?,
        )?
    } else if log_z <= half {
        let frac = log_z.checked_div(half)?;
        line.m_jup_at_solar.checked_add(
            frac.checked_mul(
                line.m_jup_at_triple_solar
                    .checked_sub(line.m_jup_at_solar)?,
            )?,
        )?
    } else {
        line.m_jup_at_triple_solar
    };
    Some(m)
}

/// The disk GAS ASPECT RATIO `H/r` at an orbit, DERIVED from the disk temperature: the isothermal scale height
/// `H = c_s/Omega` over the orbital radius, so `H/r = c_s/v_kep` with the isothermal sound speed
/// `c_s = (k_B T/(mu m_H))^(1/2)` and the Keplerian velocity `v_kep = (G M_star/r)^(1/2)`. The Crida gap-opening
/// criterion reads it. Computed in the log domain (the disk-clock precedent), so no unrepresentable intermediate
/// forms. `None` past the disk edge (the temperature solve fails) or on an overflow.
fn disk_aspect_ratio_at_orbit(disk: &SolidDisk, orbit_au: Fixed) -> Option<Fixed> {
    let temperature = disk_effective_temperature(
        disk.thermal.accretion_rate_msun_myr,
        disk.thermal.star_mass_ratio,
        disk.thermal.mass_luminosity_exponent,
        orbit_au,
        disk.thermal.reprocessing_factor,
        disk.thermal.inner_boundary_factor,
        disk.thermal.t_max,
    )?;
    if temperature <= Fixed::ZERO
        || disk.mean_molecular_weight <= Fixed::ZERO
        || disk.thermal.star_mass_ratio <= Fixed::ZERO
        || orbit_au <= Fixed::ZERO
    {
        return None;
    }
    let ln_k_b = civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::BOLTZMANN.value)?;
    // ln m_H = ln(1e-3) - ln(N_A): one atomic mass unit (one gram per mole per amu).
    let ln_m_h = civsim_physics::saha::ln_of_decimal("1e-3")?.checked_sub(
        civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::AVOGADRO.value)?,
    )?;
    let ln_g = civsim_physics::saha::ln_of_decimal(
        civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value,
    )?;
    // ln c_s = 0.5*(ln k_B + ln T - ln mu - ln m_H).
    let ln_c_s = Fixed::from_ratio(1, 2).checked_mul(
        ln_k_b
            .checked_add(temperature.ln())?
            .checked_sub(disk.mean_molecular_weight.ln())?
            .checked_sub(ln_m_h)?,
    )?;
    // ln v_kep = 0.5*(ln G + ln M_star + ln M_sun - ln r), with r = orbit_au * AU in metres.
    let ln_r = orbit_au
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(ASTRONOMICAL_UNIT_M)?)?;
    let ln_v_kep = Fixed::from_ratio(1, 2).checked_mul(
        ln_g.checked_add(disk.thermal.star_mass_ratio.ln())?
            .checked_add(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?
            .checked_sub(ln_r)?,
    )?;
    Some(ln_c_s.checked_sub(ln_v_kep)?.exp())
}

/// The TERMINATED-AND-TYPED giant result: the un-terminated reservoir ceiling, the gap-opening cap, the terminated
/// mass, and the fusion class. [`terminate_and_type_giant`] composes it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TerminatedGiant {
    /// The reservoir-exhaustion first cut (Earth masses), the [`GiantOutcome::Giant`] `final_mass_earth` ceiling.
    pub un_terminated_mass_earth: Fixed,
    /// The Crida gap-opening cap (Earth masses), the runaway-termination scale.
    pub gap_mass_earth: Fixed,
    /// The terminated mass (Earth masses), `min(reservoir, M_gap)`.
    pub terminated_mass_earth: Fixed,
    /// The fusion class of the terminated mass.
    pub mass_class: GiantMassClass,
}

/// TERMINATE AND TYPE a giant verdict: the integration that makes the giant gate's mass SOUND. It threads the disk
/// aspect ratio at the embryo's orbit ([`disk_aspect_ratio_at_orbit`]) through the Crida gap-opening criterion to
/// cap the runaway (`M_final = min(reservoir, M_gap)`), then types the capped mass against the fusion boundaries.
/// This retires the reservoir-exhaustion upper bound the audit flagged: the un-terminated ~24-Jupiter-mass ceiling
/// becomes a ~2-Jupiter-mass planet, and only a mass truly past the deuterium line types as a brown dwarf.
///
/// The wiring is ADDITIVE and byte-neutral: [`giant_formation`] stays the un-terminated first cut (its
/// `final_mass_earth` is the honest ceiling), and this composes the sound result on top from the disk and the two
/// cited models, so no existing caller or verdict changes. `None` when the verdict is TERRESTRIAL (no giant to
/// terminate) or a derivation fails (a disk-edge orbit, an overflow), a fail-soft that never fabricates a mass.
pub fn terminate_and_type_giant(
    verdict: &GiantVerdict,
    disk: &SolidDisk,
    star_mass_ratio: Fixed,
    gap_model: &GapOpeningModel,
    burning_limits: &BurningLimits,
) -> Option<TerminatedGiant> {
    let un_terminated_mass_earth = match verdict.outcome {
        GiantOutcome::Giant { final_mass_earth } => final_mass_earth,
        GiantOutcome::Terrestrial => return None,
    };
    let h_over_r = disk_aspect_ratio_at_orbit(disk, verdict.orbit_au)?;
    let reynolds = alpha_disk_reynolds_number(disk.alpha_viscosity, h_over_r)?;
    let gap_mass_earth = gap_opening_mass_earth(h_over_r, reynolds, star_mass_ratio, gap_model)?;
    let terminated_mass_earth =
        runaway_terminated_giant_mass_earth(un_terminated_mass_earth, gap_mass_earth)?;
    let mass_class = giant_mass_class(terminated_mass_earth, burning_limits)?;
    Some(TerminatedGiant {
        un_terminated_mass_earth,
        gap_mass_earth,
        terminated_mass_earth,
        mass_class,
    })
}

/// The GIANT-FORMATION VERDICT for one embryo (task #73, slice 1). It derives the core accretion rate, the
/// critical mass, the envelope opacity, and the Kelvin-Helmholtz time, then declares GIANT when the core is
/// super-critical AND the envelope contracts before the disk gas disperses; otherwise TERRESTRIAL. For a giant it
/// adds the feeding-zone gas to the core for a first-cut final mass. `star_mass_ratio` is the star mass in solar
/// masses (the same argument [`crate::planetary_system::oligarchic_embryo_field`] takes). Every diagnostic is
/// returned so a caller can see why the embryo went the way it did. `None` on a degenerate input (a disk-edge
/// orbit, a non-positive mass, an overflow), a fail-soft that never fabricates a verdict.
pub fn giant_formation(
    embryo: &Embryo,
    disk: &SolidDisk,
    star_mass_ratio: Fixed,
    gas: &GiantGasParams,
    kh: &GiantKhParams,
) -> Option<GiantVerdict> {
    let orbit_au = embryo.orbit_au;
    let core_mass_earth = embryo.mass_earth;
    if core_mass_earth <= Fixed::ZERO {
        return None;
    }
    // The derived oligarchic core accretion rate and the Ikoma critical mass it implies.
    let core_accretion_rate_earth_per_yr = core_accretion_rate_earth_per_yr(
        disk,
        star_mass_ratio,
        orbit_au,
        core_mass_earth,
        gas.collision_coefficient,
        gas.core_bulk_density_g_cm3,
    )?;
    let critical_core_mass_earth = critical_core_mass_earth(core_accretion_rate_earth_per_yr)?;
    // The derived envelope opacity at the disk's metallicity (the reported diagnostic; the tau_KH lever uses the
    // metallicity ratio directly).
    let envelope_opacity_cm2_g = kh
        .reference_opacity_cm2_g
        .checked_mul(disk.metal_fraction)?
        .checked_div(kh.reference_metal_fraction)?;
    // The Kelvin-Helmholtz condition, decided in the log domain: tau_KH < gas lifetime.
    let ln_tau_kh = ln_kh_time_years(core_mass_earth, disk.metal_fraction, kh)?;
    let ln_gas_lifetime_yr = gas
        .disk_gas_lifetime_myr
        .ln()
        .checked_add(Fixed::from_int(1_000_000).ln())?;
    let envelope_contracts_in_time = ln_tau_kh < ln_gas_lifetime_yr;
    // A saturated diagnostic read of tau_KH in years: exp if representable, else the fixed-point ceiling. The
    // verdict never depends on this read (it is decided above in the log domain).
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    let kh_time_yr = if ln_tau_kh >= ln_ceiling {
        Fixed::MAX
    } else {
        ln_tau_kh.exp()
    };
    let super_critical = core_mass_earth > critical_core_mass_earth;
    let outcome = if super_critical && envelope_contracts_in_time {
        // Runaway: add the feeding-zone gas reservoir to the core for the first-cut giant mass. If the gas
        // reservoir fails to resolve (a disk-edge annulus), fall back to the core mass alone rather than
        // fabricating a value; the giant verdict itself stands on the physics above.
        let gas_mass_earth = feeding_zone_gas_mass_earth(
            disk,
            star_mass_ratio,
            orbit_au,
            core_mass_earth,
            gas.feeding_zone_hill_widths,
            gas.gas_integration_steps,
        )
        .unwrap_or(Fixed::ZERO);
        let final_mass_earth = core_mass_earth.checked_add(gas_mass_earth)?;
        GiantOutcome::Giant { final_mass_earth }
    } else {
        GiantOutcome::Terrestrial
    };
    Some(GiantVerdict {
        orbit_au,
        core_mass_earth,
        core_accretion_rate_earth_per_yr,
        critical_core_mass_earth,
        kh_time_yr,
        envelope_opacity_cm2_g,
        outcome,
    })
}

/// The giant-formation verdict across a whole EMBRYO FIELD: [`giant_formation`] applied to each embryo, dropping
/// any that fail soft. A convenience for running the branch over
/// [`crate::planetary_system::oligarchic_embryo_field`] output.
pub fn giant_formation_field(
    embryos: &[Embryo],
    disk: &SolidDisk,
    star_mass_ratio: Fixed,
    gas: &GiantGasParams,
    kh: &GiantKhParams,
) -> Vec<GiantVerdict> {
    embryos
        .iter()
        .filter_map(|embryo| giant_formation(embryo, disk, star_mass_ratio, gas, kh))
        .collect()
}

/// The band-membership of a giant verdict when the disk gas lifetime is a DERIVED INTERVAL (the three-row wind
/// ensemble's `tau_disk` band) rather than a single value. Condition 3 of the slice-2 run-path wire: the #73 gate
/// consumes the interval, never a silently chosen central row, so an embryo whose Kelvin-Helmholtz time falls
/// inside the band is carried as NEAR-DEGENERATE (giant under the long-lifetime edge, terrestrial under the short)
/// per the Gap Law, never resolved to one row. A point-valued wire would collapse a declared model band at the
/// exact moment it first touches world content, the one failure mode that would make the slice dishonest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandedGiantOutcome {
    /// Giant under EVERY wind row: the KH time beats even the shortest derived disk lifetime, so the verdict does
    /// not depend on which wind row the ensemble picks. `final_mass_earth` is the mass at the short (least
    /// favorable) edge, the conservative giant mass.
    RobustGiant { final_mass_earth: Fixed },
    /// Terrestrial under EVERY wind row: the KH time exceeds even the longest derived disk lifetime.
    RobustTerrestrial,
    /// NEAR-DEGENERATE: giant under the long-lifetime edge, terrestrial under the short. The KH time falls INSIDE
    /// the declared wind band, so the verdict is a carried band-membership datum, never silently resolved.
    /// `giant_mass_earth` is the mass under the giant (long) edge.
    NearDegenerate { giant_mass_earth: Fixed },
}

/// The giant verdict across the derived disk-gas-lifetime BAND: the verdict at each band edge plus the
/// band-membership classification the caller reports. The two edge verdicts carry every diagnostic (why each edge
/// went the way it did).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BandedGiantVerdict {
    /// The verdict at the SHORT (least favorable to giant) disk-lifetime edge, the low end of the `tau_disk` band.
    pub short_lifetime: GiantVerdict,
    /// The verdict at the LONG (most favorable) edge, the high end of the band.
    pub long_lifetime: GiantVerdict,
    /// The band-membership classification.
    pub band: BandedGiantOutcome,
}

/// The #73 giant gate evaluated ACROSS the derived disk-gas-lifetime band (condition 3 of the slice-2 wire). The
/// three-row wind ensemble yields a `tau_disk` interval, and the gate must consume the interval rather than a
/// silently chosen central row. `gas` carries every giant-gas parameter except the lifetime; `lifetime_low_myr` and
/// `lifetime_high_myr` are the shortest and longest `tau_disk` the ensemble produces. The verdict is monotone in the
/// lifetime (giant-hood needs `ln_tau_kh < ln(lifetime)`, and the critical mass does not depend on the lifetime), so
/// the short edge is the least favorable to giant and the long edge the most: the classification is robust when the
/// two edges agree and near-degenerate when they differ. `None` on an unordered band (`low > high`) or if either
/// edge verdict fails soft. A shorter lifetime cannot make giant MORE likely, so a giant-at-short with
/// terrestrial-at-long is a monotonicity violation and fails loud (`None`) rather than fabricating a band.
pub fn giant_formation_banded(
    embryo: &Embryo,
    disk: &SolidDisk,
    star_mass_ratio: Fixed,
    gas: &GiantGasParams,
    kh: &GiantKhParams,
    lifetime_low_myr: Fixed,
    lifetime_high_myr: Fixed,
) -> Option<BandedGiantVerdict> {
    if lifetime_low_myr > lifetime_high_myr {
        return None;
    }
    let gas_short = GiantGasParams {
        disk_gas_lifetime_myr: lifetime_low_myr,
        ..*gas
    };
    let gas_long = GiantGasParams {
        disk_gas_lifetime_myr: lifetime_high_myr,
        ..*gas
    };
    let short_lifetime = giant_formation(embryo, disk, star_mass_ratio, &gas_short, kh)?;
    let long_lifetime = giant_formation(embryo, disk, star_mass_ratio, &gas_long, kh)?;
    let band = match (short_lifetime.outcome, long_lifetime.outcome) {
        (GiantOutcome::Giant { final_mass_earth }, GiantOutcome::Giant { .. }) => {
            BandedGiantOutcome::RobustGiant { final_mass_earth }
        }
        (GiantOutcome::Terrestrial, GiantOutcome::Terrestrial) => {
            BandedGiantOutcome::RobustTerrestrial
        }
        (GiantOutcome::Terrestrial, GiantOutcome::Giant { final_mass_earth }) => {
            BandedGiantOutcome::NearDegenerate {
                giant_mass_earth: final_mass_earth,
            }
        }
        (GiantOutcome::Giant { .. }, GiantOutcome::Terrestrial) => return None,
    };
    Some(BandedGiantVerdict {
        short_lifetime,
        long_lifetime,
        band,
    })
}

/// The disk-era star-and-disk state the DERIVED disk clock reads, bundled so the composed giant gate takes one
/// parameter rather than the clock's full argument list. The birth accretion rate is NOT a field: it DERIVES from
/// the birth conditions carried here (`cloud_core_temp_k`, `mean_molecular_weight`, `collapse`) through the Shu
/// inside-out collapse ([`shu_inside_out_collapse_accretion_rate_msun_myr`]), the derive-first retirement of the
/// old `Mdot_0` interim: the clock now runs from a cloud-core TEMPERATURE and the world's own composition rather
/// than a handed-in accretion rate. What remains interim is `cloud_core_temp_k` (a birth condition bottoming out at
/// the layer-4 draw, admit-the-alien a data row), `t_visc_myr` (from the disk birth size `R_1`), and
/// `rotation_period_days` (the disk-locked rotation), each a per-system datum the caller supplies, never authored
/// here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskClockState {
    /// The drawn star mass `M / Msun`.
    pub mass_ratio: Fixed,
    /// The pre-main-sequence Hayashi-wall `T_eff` (K), read per-star from the BHAC15 grid on the live path.
    pub hayashi_temp_k: Fixed,
    /// The disk-era evaluation age (Myr), the disk-hosting epoch.
    pub age_myr: Fixed,
    /// The disk-locked rotation period (days), a tagged interim until the gyrochronology draw.
    pub rotation_period_days: Fixed,
    /// The mixing-length turnover coefficient (reserved-with-basis, banked).
    pub mlt_coefficient: Fixed,
    /// The saturation Rossby number of the activity fit (reserved-with-basis).
    pub ro_sat: Fixed,
    /// The saturated `log10(L_X/L_bol)` fraction (reserved-with-basis).
    pub saturated_log10_fraction: Fixed,
    /// The activity-decline power-law index `beta` (reserved-with-basis).
    pub beta: Fixed,
    /// The molecular cloud-core TEMPERATURE (K) the birth accretion rate derives from (`Mdot_0 ~ T^(3/2)`): a
    /// per-system birth condition bottoming out at the layer-4 draw, reserved-with-basis `~10 K` until it lands.
    pub cloud_core_temp_k: Fixed,
    /// The disk-gas MEAN MOLECULAR WEIGHT the Shu sound speed reads (the world's own derived value,
    /// [`crate::astro::derive_disk_gas_mean_molecular_weight`]).
    pub mean_molecular_weight: Fixed,
    /// The declared [`CollapseModel`] (Shu 0.975 versus the faster Larson-Penston rival), the model-structure band.
    pub collapse: CollapseModel,
    /// The viscous time `t_visc` (Myr) from `R_1`, a TAGGED SOLAR INTERIM (its retirement is the layer-4 draw).
    pub t_visc_myr: Fixed,
    /// The LBP surface-density decline index `gamma` (bare algebra, `gamma = 1`).
    pub decline_gamma: Fixed,
}

impl DiskClockState {
    /// The birth accretion rate `Mdot_0` (Msun/Myr) DERIVED for a GIVEN collapse model, over this state's cloud-core
    /// temperature and gas mean molecular weight through the Shu inside-out collapse. Taking the collapse as an
    /// argument (rather than reading `self.collapse`) is what lets the interval propagation evaluate both endpoints
    /// of the collapse-model band from one state. `None` if the collapse rate refuses.
    pub fn derived_birth_accretion_for(&self, collapse: &CollapseModel) -> Option<Fixed> {
        shu_inside_out_collapse_accretion_rate_msun_myr(
            self.cloud_core_temp_k,
            self.mean_molecular_weight,
            collapse,
        )
    }

    /// The birth accretion rate `Mdot_0` (Msun/Myr) for this state's DEFAULTS-TAKEN collapse member (`self.collapse`,
    /// the Shu interim until the band ships), never a stored interim. `None` if the collapse rate refuses.
    pub fn derived_birth_accretion_msun_myr(&self) -> Option<Fixed> {
        self.derived_birth_accretion_for(&self.collapse)
    }
}

/// STAGE 4 of the slice-2 wire (DORMANT): the #73 giant gate DRIVEN BY THE DERIVED DISK CLOCK. It replaces the
/// reserved `disk_gas_lifetime_myr` with the `tau_disk` band the composed clock derives from the star's own
/// X-ray-driven photoevaporation history. The wind ensemble ([`XrayWindFit`], the declared model-structure band)
/// is passed as its two edge rows; the clock is evaluated at each, and the two `tau_disk` values are ordered into a
/// lifetime band fed to [`giant_formation_banded`]. So the giant-versus-terrestrial verdict reads a DERIVED gas
/// clock, not an authored 3 Myr placeholder, and carries the wind-model band through to a near-degenerate outcome
/// where the runaway threshold falls inside it. The strongest-wind row gives the shortest disk life and the
/// weakest the longest, but the ordering here is done on the computed `tau_disk` values, so which row is which
/// never needs asserting.
///
/// `gas.disk_gas_lifetime_myr` is IGNORED (the derived band supersedes it); the field is carried on
/// [`GiantGasParams`] only for the OTHER gas residues (the collision coefficient, the core bulk density, the
/// feeding-zone width, the integration steps), and its full retirement from the struct is a census item for the
/// flip, not this dormant composition.
///
/// The birth accretion rate the clock needs is DERIVED, not handed in: `star.derived_birth_accretion_msun_myr()`
/// runs the Shu inside-out collapse over the state's cloud-core temperature, composition, and collapse model, so the
/// whole giant verdict runs from birth CONDITIONS end to end (temperature, composition, rotation, disk size), the
/// derive-first thesis cashed. A refusal from the collapse (a non-physical birth condition) propagates as `None`.
///
/// THE COLLAPSE-TO-CLOCK WELD IS A DECLARED CLOSURE, NOT AN IDENTITY. The Shu rate is the ENVELOPE INFALL onto the
/// star-plus-disk; the Lynden-Bell-Pringle clock's `Mdot_0` is the DISK's own initial accretion rate (the viscous
/// similarity solution's `Mdot(t=0)`). These are different physical quantities, and equating them is a QUASI-STEADY
/// TRANSMISSION closure: in the embedded class-0/I phase, matter that falls onto the disk is processed through it
/// onto the star at nearly the infall rate, so `Mdot_disk(0) ~ Mdot_infall`. VALIDITY: the closure holds while
/// infall dominates (the embedded phase) and degrades once infall ends and the disk drains on its own viscous time,
/// which is precisely the regime the LBP decline then governs, so the two meet at the handover. EPOCH CONVENTION:
/// the clock's `t = 0` is the DISK-ASSEMBLY epoch (the end of the main infall), one abscissa among the family (core
/// formation and end of infall are earlier), stated so a later per-world epoch draw keys to the right zero. The
/// LBP primary's `Mdot_0` is its initial-condition normalization, which this closure supplies from the collapse
/// rather than reserving; the correspondence is named here so the weld is auditable, not silent.
///
/// RANGE-COLLAPSE, DECLARED. The wind ensemble now flows as an INTERVAL (the two `XrayWindFit` edges), but the
/// COLLAPSE model is consumed as a POINT: `star.collapse` is one member (`CollapseModel::shu_1977` in the tests),
/// DEFAULTS-TAKEN, so the derived `Mdot_0` is conditioned on the Shu member and a provenance readout must carry
/// "conditioned on the Shu member" verbatim until the collapse-band interval propagation lands. That propagation
/// (evaluate the collapse at both endpoints, feed the `Mdot_0` interval through the race, interval in and interval
/// out) is its own slice; a weld that silently collapses the declared factor-48 band onto its Shu edge would be the
/// range-collapse defect standing one step downstream of the wind band this wire already carries.
///
/// DORMANT and BYTE-NEUTRAL: no run-path caller (both the disk clock and the giant gate are dormant), so the pins
/// hold bit-exact. The FLIP that feeds this into `run_world` and moves the pins is the capstone event under the
/// owner's signature, not this composition, and it waits on the layer-4 draws that retire the remaining interims
/// (the cloud-core temperature, `t_visc`, the rotation). `None` if the collapse or either clock evaluation refuses
/// (a link's domain door) or the giant gate refuses (an unordered band or a monotonicity violation), the refusal
/// propagated rather than swallowed.
pub fn giant_formation_on_derived_clock(
    embryo: &Embryo,
    disk: &SolidDisk,
    star: &DiskClockState,
    wind_fit_a: &XrayWindFit,
    wind_fit_b: &XrayWindFit,
    gas: &GiantGasParams,
    kh: &GiantKhParams,
) -> Option<BandedGiantVerdict> {
    // Mdot_0 DERIVES from the birth conditions (Shu collapse), never a stored interim.
    let mdot_0_msun_myr = star.derived_birth_accretion_msun_myr()?;
    let tau_disk = |fit: &XrayWindFit| {
        disk_era_xray_disk_lifetime_myr(
            star.mass_ratio,
            star.hayashi_temp_k,
            star.age_myr,
            star.rotation_period_days,
            star.mlt_coefficient,
            star.ro_sat,
            star.saturated_log10_fraction,
            star.beta,
            fit,
            mdot_0_msun_myr,
            star.t_visc_myr,
            star.decline_gamma,
        )
    };
    let tau_a = tau_disk(wind_fit_a)?;
    let tau_b = tau_disk(wind_fit_b)?;
    let (tau_low, tau_high) = if tau_a <= tau_b {
        (tau_a, tau_b)
    } else {
        (tau_b, tau_a)
    };
    giant_formation_banded(embryo, disk, star.mass_ratio, gas, kh, tau_low, tau_high)
}

/// THE COLLAPSE-BAND INTERVAL PROPAGATION (the small slice the range-collapse ruling sequenced): the #73 giant gate
/// over the COMPOUND band, the collapse-model band crossed with the wind-model band. It retires the range-collapse
/// of [`giant_formation_on_derived_clock`], which consumed the collapse as a POINT (the Shu DEFAULTS-TAKEN member)
/// while flowing only the wind band. Here BOTH bands flow: the birth accretion rate is derived at each collapse
/// endpoint (`collapse_a`, `collapse_b`, the Shu and Larson-Penston edges, a factor ~48 apart), the disk clock is
/// evaluated at every (collapse, wind) corner, and the `tau_disk` band is the interval over the four corners. The
/// gate consumes that interval, interval in and interval out, never collapsing the declared band onto one member.
///
/// THE LICENSE for corner evaluation: min/max over the four corners is EXACT interval propagation ONLY when
/// `tau_disk` is monotone in each argument over the box (a non-monotone dependence would hide an interior extremum
/// and silently narrow the band, range-collapse wearing rigor's clothes). Here it is, with the SIGN of each
/// dependence stated: `tau_disk` RISES with the birth accretion rate `Mdot_0` (a higher birth rate is a longer disk
/// life), and `Mdot_0` rises linearly with the collapse coefficient `m0`, so `tau_disk` rises with the collapse
/// member; and `tau_disk` FALLS with the wind rate (a stronger wind is a shorter life). Both dependences are
/// componentwise monotone over the box, so the extrema sit at the corners. THE CORNERS ARE THE ENSEMBLE EXTREMES,
/// with their LINEAGE recorded (a role swap is a ruling-visible event, not a refactor): the three wind rates in
/// order are Owen equation-9 `8e-9`, Owen appendix-B `6.25e-9`, and Sellek `4.32e-9`, so the RATE EXTREMES are Owen
/// equation-9 (strongest wind, shortest life) and Sellek (weakest, longest), and `wind_fit_a`/`wind_fit_b` must be
/// those two, with Owen appendix-B (the ruled central instance) riding as the interior CROSS-CHECK the box-midpoint
/// sentinel evaluates, not a corner. PROVENANCE OF THE HIGH EDGE: equation-9 and appendix-B are two readings of the
/// same Owen 2012 primary, so the band's high-wind (short-life) edge is an INTRA-SOURCE variant, and its width there
/// measures Owen's own internal spread (equation-9 versus appendix-B), while the low-wind edge (Sellek) is
/// CROSS-SOURCE disagreement; legitimate to band over, but a different provenance than the ensemble as ruled, noted
/// so the ledger reads true. This is where the central-member ruling's warning bites or does not: if the collapse band makes
/// every embryo near-degenerate, the band is the priority signal for the retirement ladder (surfaced by the reported
/// hindcast, never selected here).
///
/// THE MEMBER-SELECTION GUARD IS STRUCTURAL, not a comment: this returns a [`BandedGiantVerdict`], which carries the
/// two edge verdicts and the band classification and has NO "chosen member" field. There is no code path that reads
/// one collapse endpoint and discards the other; a consumer that wanted to select a member would have to reach past
/// the interval this returns, which the type does not offer. Selection is impossible here by construction.
///
/// DORMANT and BYTE-NEUTRAL, the successor to the point wire: when the band ships (the central-member ruling's end
/// state, no default), this is the wire the giant verdict reads; until then the point version rides with its
/// DEFAULTS-TAKEN Shu member. `None` if any collapse or clock evaluation refuses, or the gate refuses.
#[allow(clippy::too_many_arguments)]
pub fn giant_formation_on_derived_clock_banded(
    embryo: &Embryo,
    disk: &SolidDisk,
    star: &DiskClockState,
    collapse_a: &CollapseModel,
    collapse_b: &CollapseModel,
    wind_fit_a: &XrayWindFit,
    wind_fit_b: &XrayWindFit,
    gas: &GiantGasParams,
    kh: &GiantKhParams,
) -> Option<BandedGiantVerdict> {
    let tau_for = |mdot_0: Fixed, fit: &XrayWindFit| {
        disk_era_xray_disk_lifetime_myr(
            star.mass_ratio,
            star.hayashi_temp_k,
            star.age_myr,
            star.rotation_period_days,
            star.mlt_coefficient,
            star.ro_sat,
            star.saturated_log10_fraction,
            star.beta,
            fit,
            mdot_0,
            star.t_visc_myr,
            star.decline_gamma,
        )
    };
    // Each collapse endpoint derives its own Mdot_0; each (collapse, wind) corner gives a tau_disk. The band is the
    // min and max over the four corners (the verdict is monotone in tau, so the interior corners cannot widen it).
    let mut tau_lo: Option<Fixed> = None;
    let mut tau_hi: Option<Fixed> = None;
    for collapse in [collapse_a, collapse_b] {
        let mdot_0 = star.derived_birth_accretion_for(collapse)?;
        for fit in [wind_fit_a, wind_fit_b] {
            let tau = tau_for(mdot_0, fit)?;
            tau_lo = Some(tau_lo.map_or(tau, |lo| if tau < lo { tau } else { lo }));
            tau_hi = Some(tau_hi.map_or(tau, |hi| if tau > hi { tau } else { hi }));
        }
    }
    giant_formation_banded(embryo, disk, star.mass_ratio, gas, kh, tau_lo?, tau_hi?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planetary_system::{oligarchic_embryo_field, DiskThermalParams};

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    /// The representative Mirror-like disk the embryo-field tests use (solar metallicity, a Shakura-Sunyaev
    /// viscosity and a solar-mix mean molecular weight, the two-regime thermal residues). `accretion_boost`
    /// scales the disk accretion rate so a denser disk (more solids, larger cores) is a data row.
    fn mirror_disk(accretion_boost: Fixed) -> SolidDisk {
        let thermal = DiskThermalParams {
            accretion_rate_msun_myr: r(1, 100).checked_mul(accretion_boost).unwrap(),
            star_mass_ratio: Fixed::ONE,
            mass_luminosity_exponent: r(35, 10),
            reprocessing_factor: r(5, 100),
            inner_boundary_factor: Fixed::from_int(4),
            t_max: Fixed::from_int(2_000_000),
        };
        SolidDisk::derive(
            thermal,
            r(1, 100),
            r(234, 100),
            r(134, 10_000),
            r(1, 2),
            Fixed::from_int(182),
            Fixed::ONE,
            Fixed::from_int(40),
        )
        .expect("the Mirror disk locates its ice line")
    }

    fn gas_params() -> GiantGasParams {
        GiantGasParams {
            disk_gas_lifetime_myr: Fixed::from_int(3), // ~3 Myr, mid observed disk lifetime (reserved-with-basis)
            collision_coefficient: Fixed::ONE,         // order unity (reserved-with-basis)
            core_bulk_density_g_cm3: r(4, 1), // ~4 g/cm^3 rock-ice core (reserved-with-basis)
            feeding_zone_hill_widths: Fixed::from_int(5),
            gas_integration_steps: 64,
        }
    }

    fn kh_params() -> GiantKhParams {
        GiantKhParams {
            kh_log10_yr_c: Fixed::from_int(9), // Ida-Lin fiducial c = 9 (reserved-with-basis)
            kh_mass_exponent_d: Fixed::from_int(3), // Ida-Lin fiducial d = 3 (reserved-with-basis)
            reference_opacity_cm2_g: Fixed::ONE, // ~1 cm^2/g ISM grains (reserved-with-basis)
            reference_metal_fraction: r(134, 10_000), // solar Z ~ 0.0134 (reserved-with-basis)
        }
    }

    /// An inner rocky embryo is TERRESTRIAL: its Mars-class isolation-mass core is far below the Ikoma critical
    /// mass, so no envelope runs away.
    #[test]
    fn an_inner_embryo_stays_terrestrial() {
        let disk = mirror_disk(Fixed::ONE);
        let embryo = Embryo {
            orbit_au: Fixed::ONE,
            mass_earth: r(1, 10), // ~0.1 M_earth Mars-class oligarch
        };
        let verdict =
            giant_formation(&embryo, &disk, Fixed::ONE, &gas_params(), &kh_params()).unwrap();
        assert_eq!(verdict.outcome, GiantOutcome::Terrestrial);
        assert!(
            verdict.core_mass_earth < verdict.critical_core_mass_earth,
            "the inner core ({}) is sub-critical ({})",
            verdict.core_mass_earth.to_f64_lossy(),
            verdict.critical_core_mass_earth.to_f64_lossy()
        );
    }

    /// A massive outer core crosses to GIANT: a super-critical core beyond the ice line contracts fast enough to
    /// run away before the gas disperses, and its first-cut mass exceeds the core (the feeding-zone gas is added).
    #[test]
    fn a_massive_outer_core_becomes_a_giant() {
        let disk = mirror_disk(Fixed::ONE);
        let embryo = Embryo {
            orbit_au: Fixed::from_int(6),
            mass_earth: Fixed::from_int(15), // a super-critical outer core
        };
        let verdict =
            giant_formation(&embryo, &disk, Fixed::ONE, &gas_params(), &kh_params()).unwrap();
        match verdict.outcome {
            GiantOutcome::Giant { final_mass_earth } => {
                assert!(
                    final_mass_earth > verdict.core_mass_earth,
                    "the giant accretes gas beyond its core ({} > {})",
                    final_mass_earth.to_f64_lossy(),
                    verdict.core_mass_earth.to_f64_lossy()
                );
            }
            GiantOutcome::Terrestrial => {
                panic!("a 15 M_earth outer core should run away to a giant")
            }
        }
        assert!(verdict.core_mass_earth > verdict.critical_core_mass_earth);
    }

    /// The critical mass RISES with the core accretion rate (the Ikoma fourth-root law), and a denser disk raises
    /// the accretion rate: the physics that makes a fast-growing core need more mass before it can contract.
    #[test]
    fn a_denser_disk_raises_the_accretion_rate_and_critical_mass() {
        let sparse = mirror_disk(Fixed::ONE);
        let dense = mirror_disk(Fixed::from_int(4));
        let embryo = Embryo {
            orbit_au: Fixed::from_int(3),
            mass_earth: Fixed::ONE,
        };
        let v_sparse =
            giant_formation(&embryo, &sparse, Fixed::ONE, &gas_params(), &kh_params()).unwrap();
        let v_dense =
            giant_formation(&embryo, &dense, Fixed::ONE, &gas_params(), &kh_params()).unwrap();
        assert!(
            v_dense.core_accretion_rate_earth_per_yr > v_sparse.core_accretion_rate_earth_per_yr,
            "a denser disk delivers solids faster"
        );
        assert!(
            v_dense.critical_core_mass_earth > v_sparse.critical_core_mass_earth,
            "a faster-accreting core has a higher critical mass"
        );
    }

    /// The verdict MOVES WITH THE OPACITY, isolated as the deciding lever: a grainier envelope (a higher opacity)
    /// lengthens the Kelvin-Helmholtz time, so a super-critical core that ran away at low opacity fails the
    /// runaway race at high opacity, with the core mass and the critical mass held fixed. The opacity is raised by
    /// referencing the same disk metallicity to a lower reference metallicity (a grain-property axis, admitting
    /// the alien), which leaves `Sigma_solid`, `Mdot_c`, and `M_crit` untouched and moves only `tau_KH`.
    #[test]
    fn the_opacity_lever_moves_the_kh_time_and_the_verdict() {
        let disk = mirror_disk(Fixed::ONE);
        // A super-critical outer core so the Kelvin-Helmholtz race, not the critical-mass test, decides.
        let embryo = Embryo {
            orbit_au: Fixed::from_int(6),
            mass_earth: Fixed::from_int(15),
        };
        let low_opacity = kh_params();
        let mut high_opacity = kh_params();
        high_opacity.reference_metal_fraction = kh_params()
            .reference_metal_fraction
            .checked_div(Fixed::from_int(30))
            .unwrap();
        let v_low =
            giant_formation(&embryo, &disk, Fixed::ONE, &gas_params(), &low_opacity).unwrap();
        let v_high =
            giant_formation(&embryo, &disk, Fixed::ONE, &gas_params(), &high_opacity).unwrap();
        // Same disk, same core: the critical mass is unchanged; only the opacity and the KH time move.
        assert_eq!(
            v_low.critical_core_mass_earth, v_high.critical_core_mass_earth,
            "the opacity lever leaves the critical mass fixed"
        );
        assert!(
            v_high.envelope_opacity_cm2_g > v_low.envelope_opacity_cm2_g,
            "the grainier envelope has a higher opacity"
        );
        assert!(
            v_high.kh_time_yr > v_low.kh_time_yr,
            "a higher opacity lengthens the contraction time ({} vs {})",
            v_high.kh_time_yr.to_f64_lossy(),
            v_low.kh_time_yr.to_f64_lossy()
        );
        assert!(
            matches!(v_low.outcome, GiantOutcome::Giant { .. }),
            "at low opacity the super-critical core runs away"
        );
        assert_eq!(
            v_high.outcome,
            GiantOutcome::Terrestrial,
            "at high opacity the envelope cannot contract before the gas disperses"
        );
    }

    /// The Kelvin-Helmholtz time FALLS with the core mass (a heavier envelope contracts faster), the `M_c^(-d)`
    /// law that lets a big core win the runaway race.
    #[test]
    fn the_kh_time_falls_with_the_core_mass() {
        let disk = mirror_disk(Fixed::ONE);
        let ln_light = ln_kh_time_years(Fixed::from_int(5), disk.metal_fraction, &kh_params())
            .unwrap()
            .to_f64_lossy();
        let ln_heavy = ln_kh_time_years(Fixed::from_int(20), disk.metal_fraction, &kh_params())
            .unwrap()
            .to_f64_lossy();
        assert!(
            ln_heavy < ln_light,
            "a heavier core contracts faster (ln tau {} < {})",
            ln_heavy,
            ln_light
        );
    }

    /// Determinism (Principle 3): the same embryo and disk give the same verdict, bit for bit.
    #[test]
    fn the_verdict_is_deterministic() {
        let disk = mirror_disk(Fixed::ONE);
        let embryo = Embryo {
            orbit_au: Fixed::from_int(6),
            mass_earth: Fixed::from_int(15),
        };
        let a = giant_formation(&embryo, &disk, Fixed::ONE, &gas_params(), &kh_params());
        let b = giant_formation(&embryo, &disk, Fixed::ONE, &gas_params(), &kh_params());
        assert_eq!(a, b);
    }

    /// Fail-soft (Principle 3): a degenerate input (a non-positive orbit, a zero-mass core, a disk-edge orbit)
    /// returns `None`, never a fabricated verdict.
    #[test]
    fn a_degenerate_input_fails_soft() {
        let disk = mirror_disk(Fixed::ONE);
        let zero_mass = Embryo {
            orbit_au: Fixed::ONE,
            mass_earth: Fixed::ZERO,
        };
        assert!(
            giant_formation(&zero_mass, &disk, Fixed::ONE, &gas_params(), &kh_params()).is_none()
        );
        let bad_orbit = Embryo {
            orbit_au: Fixed::ZERO,
            mass_earth: Fixed::ONE,
        };
        assert!(
            giant_formation(&bad_orbit, &disk, Fixed::ONE, &gas_params(), &kh_params()).is_none()
        );
        // A disk with no gas flux carries no solids, so the accretion rate cannot resolve: fail soft.
        let mut empty = disk;
        empty.thermal.accretion_rate_msun_myr = Fixed::ZERO;
        let ok_embryo = Embryo {
            orbit_au: Fixed::from_int(6),
            mass_earth: Fixed::from_int(15),
        };
        assert!(
            giant_formation(&ok_embryo, &empty, Fixed::ONE, &gas_params(), &kh_params()).is_none()
        );
    }

    /// The branch runs over a whole EMBRYO FIELD and SPLITS it: the inner embryos stay terrestrial and the outer
    /// ice-line embryos, whose cores grow super-critical, cross to giants, so the giant branch is read off the
    /// disk, never painted. The disk is dense enough that the outer cores win the runaway race (with the honest
    /// low embryo-mass normalization the crossing sits in the outer disk; the mechanism and the split are what is
    /// proven, not the absolute orbit).
    #[test]
    fn the_field_splits_into_terrestrials_and_giants() {
        let disk = mirror_disk(Fixed::from_int(30));
        let field = oligarchic_embryo_field(
            &disk,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(30),
            256,
        );
        assert!(
            field.len() >= 4,
            "the disk seeds several embryos, got {}",
            field.len()
        );
        let verdicts =
            giant_formation_field(&field, &disk, Fixed::ONE, &gas_params(), &kh_params());
        assert_eq!(
            verdicts.len(),
            field.len(),
            "every embryo resolves a verdict"
        );
        // The innermost embryo (a small core) is terrestrial; the outermost (the largest core) is a giant.
        let inner = verdicts.first().unwrap();
        assert_eq!(
            inner.outcome,
            GiantOutcome::Terrestrial,
            "the innermost embryo (core {}) stays terrestrial",
            inner.core_mass_earth.to_f64_lossy()
        );
        let outer = verdicts.last().unwrap();
        assert!(
            matches!(outer.outcome, GiantOutcome::Giant { .. }),
            "the outermost embryo (core {}) crosses to a giant",
            outer.core_mass_earth.to_f64_lossy()
        );
        // Both classes appear from one disk: at least one terrestrial and at least one giant.
        let giants = verdicts
            .iter()
            .filter(|v| matches!(v.outcome, GiantOutcome::Giant { .. }))
            .count();
        let terrestrials = verdicts.len() - giants;
        assert!(
            giants >= 1,
            "a dense disk grows at least one giant, got {}",
            giants
        );
        assert!(
            terrestrials >= 1,
            "the inner disk stays terrestrial, got {}",
            terrestrials
        );
    }

    /// Condition 3 of the slice-2 wire: the #73 gate consumes the derived `tau_disk` INTERVAL, not a picked central
    /// row. A super-critical embryo whose Kelvin-Helmholtz time falls INSIDE the derived disk-lifetime band is
    /// carried as near-degenerate (giant under the long edge, terrestrial under the short); one whose KH time beats
    /// even the short edge is a robust giant; one whose KH time exceeds even the long edge is a robust terrestrial.
    /// The band is calibrated off the embryo's OWN KH time, so the test rests on the mechanism, not a magic number.
    #[test]
    fn the_banded_giant_verdict_carries_the_wind_band() {
        let disk = mirror_disk(Fixed::from_int(30));
        // The outer embryo of the dense-disk field is super-critical (the giant end of the field-splits test).
        let field = oligarchic_embryo_field(
            &disk,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(30),
            256,
        );
        let embryo = *field.last().expect("the dense disk seeds embryos");
        // Probe at a generous lifetime to read the KH time and confirm the embryo is super-critical (a giant when
        // the disk lives long enough); a sub-critical embryo would be terrestrial at every lifetime and the band
        // test would be vacuous.
        let probe = giant_formation(
            &embryo,
            &disk,
            Fixed::ONE,
            &GiantGasParams {
                disk_gas_lifetime_myr: Fixed::from_int(1000),
                ..gas_params()
            },
            &kh_params(),
        )
        .expect("the probe verdict resolves");
        assert!(
            matches!(probe.outcome, GiantOutcome::Giant { .. }),
            "the probe embryo is super-critical at a long lifetime (core {})",
            probe.core_mass_earth.to_f64_lossy()
        );
        // The KH time in Myr: giant-hood needs the disk lifetime to exceed it (ln_tau_kh < ln(lifetime * 1e6)).
        let kh_myr = probe
            .kh_time_yr
            .checked_div(Fixed::from_int(1_000_000))
            .expect("the KH time in Myr");
        let two = Fixed::from_int(2);
        let four = Fixed::from_int(4);
        let half = Fixed::from_ratio(1, 2);
        let quarter = Fixed::from_ratio(1, 4);
        let band = |lo: Fixed, hi: Fixed| {
            giant_formation_banded(
                &embryo,
                &disk,
                Fixed::ONE,
                &gas_params(),
                &kh_params(),
                lo,
                hi,
            )
        };
        // Both edges longer than the KH time: giant under every row.
        let rg = band(
            kh_myr.checked_mul(two).unwrap(),
            kh_myr.checked_mul(four).unwrap(),
        )
        .expect("the robust-giant band resolves");
        assert!(
            matches!(rg.band, BandedGiantOutcome::RobustGiant { .. }),
            "the KH time beats even the short edge, so giant is robust across the wind band"
        );
        // Both edges shorter than the KH time: terrestrial under every row (the envelope never contracts in time).
        let rt = band(
            kh_myr.checked_mul(quarter).unwrap(),
            kh_myr.checked_mul(half).unwrap(),
        )
        .expect("the robust-terrestrial band resolves");
        assert_eq!(
            rt.band,
            BandedGiantOutcome::RobustTerrestrial,
            "the KH time exceeds even the long edge, so terrestrial is robust across the wind band"
        );
        // The band STRADDLES the KH time: near-degenerate, the Gap Law's carried datum.
        let nd = band(
            kh_myr.checked_mul(half).unwrap(),
            kh_myr.checked_mul(two).unwrap(),
        )
        .expect("the near-degenerate band resolves");
        assert!(
            matches!(nd.band, BandedGiantOutcome::NearDegenerate { .. }),
            "the KH time inside the band gives a near-degenerate verdict, carried not collapsed"
        );
        // An unordered band (low > high) fails soft rather than fabricating a verdict.
        assert!(
            band(
                kh_myr.checked_mul(four).unwrap(),
                kh_myr.checked_mul(two).unwrap()
            )
            .is_none(),
            "an unordered band fails soft"
        );
    }

    #[test]
    fn the_derived_clock_drives_the_giant_gate_through_the_wind_band() {
        // STAGE 4 (dormant): the composed wire, disk clock -> tau_disk band -> banded giant verdict, proved by
        // twin-independence (the composition byte-equals the hand-chained clock-then-gate) and by the wind band
        // being carried (the two rows produce an ordered, non-degenerate tau_disk interval, never collapsed).
        let disk = mirror_disk(Fixed::from_int(30));
        let field = oligarchic_embryo_field(
            &disk,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(30),
            256,
        );
        let embryo = *field.last().expect("the dense disk seeds embryos");
        // The birth conditions the clock derives from (the derive-first form): a fixture wall, disk-locked rotation,
        // a ~10 K solar-composition cloud core and the Shu collapse model (from which Mdot_0 DERIVES, no interim
        // accretion rate), unit t_visc, gamma = 1. The conditions are TEST inputs, not authored by the wire.
        let star = DiskClockState {
            mass_ratio: Fixed::ONE,
            hayashi_temp_k: Fixed::from_int(4200),
            age_myr: Fixed::ONE,
            rotation_period_days: Fixed::from_int(8),
            mlt_coefficient: Fixed::from_ratio(3, 2),
            ro_sat: Fixed::from_ratio(13, 100),
            saturated_log10_fraction: Fixed::from_ratio(-313, 100),
            beta: Fixed::from_ratio(-27, 10),
            cloud_core_temp_k: Fixed::from_int(10), // ~10 K molecular cloud core, reserved-with-basis birth draw
            mean_molecular_weight: Fixed::from_ratio(233, 100), // solar disk gas, the world's derived value
            collapse: CollapseModel::shu_1977(), // m0 = 0.975, the vendored central collapse row
            t_visc_myr: Fixed::ONE,
            decline_gamma: Fixed::ONE,
        };
        // Mdot_0 is DERIVED from the birth conditions, not a stored field: the Shu collapse gives ~1.5 Msun/Myr at
        // 10 K solar composition (the vendored oracle), so the hand-chain reads the SAME derived rate the wire does.
        let mdot_0 = star
            .derived_birth_accretion_msun_myr()
            .expect("the birth accretion rate derives from the 10 K core");
        assert!(
            mdot_0.to_f64_lossy() > 1.4 && mdot_0.to_f64_lossy() < 1.7,
            "the derived Mdot_0 is the ~1.5 Msun/Myr Shu rate (got {})",
            mdot_0.to_f64_lossy()
        );
        // The wind ensemble's two lifetime edges: the strongest-wind row (Owen eq. 9, 8e-9) is the shortest disk
        // life, the weakest (Sellek, 4.32e-9) the longest.
        let strong = XrayWindFit::owen_equation_9();
        let weak = XrayWindFit::sellek_2024();

        // Hand-chain the two tau_disk values and confirm the wind band is carried (a stronger wind, a shorter life).
        let clock = |fit: &XrayWindFit| {
            disk_era_xray_disk_lifetime_myr(
                star.mass_ratio,
                star.hayashi_temp_k,
                star.age_myr,
                star.rotation_period_days,
                star.mlt_coefficient,
                star.ro_sat,
                star.saturated_log10_fraction,
                star.beta,
                fit,
                mdot_0,
                star.t_visc_myr,
                star.decline_gamma,
            )
        };
        let tau_strong = clock(&strong).expect("the strong-wind clock resolves");
        let tau_weak = clock(&weak).expect("the weak-wind clock resolves");
        assert!(
            tau_weak > tau_strong,
            "the weaker wind (Sellek) gives the longer disk life, the wind band carried (weak {} Myr, strong {} Myr)",
            tau_weak.to_f64_lossy(),
            tau_strong.to_f64_lossy()
        );

        // TWIN-INDEPENDENCE: the composition equals the hand-chain (order the tau band, call the banded gate direct).
        let expected = giant_formation_banded(
            &embryo,
            &disk,
            Fixed::ONE,
            &gas_params(),
            &kh_params(),
            tau_strong,
            tau_weak,
        );
        let composed = giant_formation_on_derived_clock(
            &embryo,
            &disk,
            &star,
            &strong,
            &weak,
            &gas_params(),
            &kh_params(),
        );
        assert_eq!(
            composed, expected,
            "the composed wire byte-equals the hand-chained clock-then-gate"
        );
        assert!(
            composed.is_some(),
            "the composed wire resolves to a banded verdict"
        );

        // The wind-row argument ORDER is irrelevant: the wire orders on the computed tau, not on which argument is
        // which, so passing the rows swapped yields the same verdict.
        let swapped = giant_formation_on_derived_clock(
            &embryo,
            &disk,
            &star,
            &weak,
            &strong,
            &gas_params(),
            &kh_params(),
        );
        assert_eq!(
            swapped, composed,
            "the wire orders on tau, so the wind-row argument order does not change the verdict"
        );
    }

    /// A representative super-critical star-and-disk state for the compound-band tests: the same tagged interims
    /// the stage-4 test uses, with the Shu member as the DEFAULTS-TAKEN point collapse on the state.
    fn banded_clock_state() -> DiskClockState {
        DiskClockState {
            mass_ratio: Fixed::ONE,
            hayashi_temp_k: Fixed::from_int(4200),
            age_myr: Fixed::ONE,
            rotation_period_days: Fixed::from_int(8),
            mlt_coefficient: Fixed::from_ratio(3, 2),
            ro_sat: Fixed::from_ratio(13, 100),
            saturated_log10_fraction: Fixed::from_ratio(-313, 100),
            beta: Fixed::from_ratio(-27, 10),
            cloud_core_temp_k: Fixed::from_int(10),
            mean_molecular_weight: Fixed::from_ratio(233, 100),
            collapse: CollapseModel::shu_1977(),
            t_visc_myr: Fixed::ONE,
            decline_gamma: Fixed::ONE,
        }
    }

    #[test]
    fn the_collapse_band_propagates_as_an_interval_through_the_gate() {
        // The interval-propagation slice: the collapse-model band crossed with the wind band. Proved by
        // twin-independence (the banded gate byte-equals the hand-computed four-corner min/max) and by the collapse
        // band CARRYING (the LP endpoint widens the tau_disk band beyond the Shu-point wind band).
        let disk = mirror_disk(Fixed::from_int(30));
        let field = oligarchic_embryo_field(
            &disk,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(30),
            256,
        );
        let embryo = *field.last().expect("the dense disk seeds embryos");
        let star = banded_clock_state();
        let shu = CollapseModel::shu_1977();
        let lp = CollapseModel::larson_penston();
        let strong = XrayWindFit::owen_equation_9(); // strongest wind, shortest disk life
        let weak = XrayWindFit::sellek_2024(); // weakest wind, longest disk life

        // Hand-compute the four corner tau_disk values (collapse x wind).
        let tau = |collapse: &CollapseModel, fit: &XrayWindFit| {
            let mdot_0 = star
                .derived_birth_accretion_for(collapse)
                .expect("the collapse rate resolves");
            disk_era_xray_disk_lifetime_myr(
                star.mass_ratio,
                star.hayashi_temp_k,
                star.age_myr,
                star.rotation_period_days,
                star.mlt_coefficient,
                star.ro_sat,
                star.saturated_log10_fraction,
                star.beta,
                fit,
                mdot_0,
                star.t_visc_myr,
                star.decline_gamma,
            )
            .expect("the clock resolves")
        };
        // The collapse band WIDENS the band upward: the faster LP collapse gives a higher Mdot_0, so a longer disk
        // life, so the compound high edge (LP + weak wind) exceeds the Shu-point high edge (Shu + weak wind).
        assert!(
            tau(&lp, &weak) > tau(&shu, &weak),
            "the LP collapse endpoint lengthens the disk life beyond the Shu member (lp {} Myr, shu {} Myr)",
            tau(&lp, &weak).to_f64_lossy(),
            tau(&shu, &weak).to_f64_lossy()
        );
        let corners = [
            tau(&shu, &strong),
            tau(&shu, &weak),
            tau(&lp, &strong),
            tau(&lp, &weak),
        ];
        let hand_lo = corners
            .iter()
            .copied()
            .fold(corners[0], |a, b| if b < a { b } else { a });
        let hand_hi = corners
            .iter()
            .copied()
            .fold(corners[0], |a, b| if b > a { b } else { a });

        // BOX-MIDPOINT SENTINEL (necessary, not sufficient): the compound evaluated at the box interior (the
        // mid-collapse m0 and the central Owen appendix-B wind row) must lie INSIDE the corner-derived band. This
        // costs one evaluation and falsifies the corner-evaluation license the day some future consumer bends the
        // map non-monotone, the range-collapse-wearing-rigor case.
        let mid_collapse = CollapseModel {
            collapse_coefficient_m0: shu
                .collapse_coefficient_m0
                .checked_add(lp.collapse_coefficient_m0)
                .unwrap()
                .checked_div(Fixed::from_int(2))
                .unwrap(),
            instability_parameter_a: shu.instability_parameter_a,
        };
        let tau_mid = tau(&mid_collapse, &XrayWindFit::owen_appendix_b());
        assert!(
            tau_mid >= hand_lo && tau_mid <= hand_hi,
            "the box-midpoint tau ({} Myr) lies inside the corner band [{}, {}], the monotonicity witness",
            tau_mid.to_f64_lossy(),
            hand_lo.to_f64_lossy(),
            hand_hi.to_f64_lossy()
        );

        // TWIN-INDEPENDENCE: the banded gate equals the hand-computed four-corner min/max fed to the banded verdict.
        let expected = giant_formation_banded(
            &embryo,
            &disk,
            Fixed::ONE,
            &gas_params(),
            &kh_params(),
            hand_lo,
            hand_hi,
        );
        let banded = giant_formation_on_derived_clock_banded(
            &embryo,
            &disk,
            &star,
            &shu,
            &lp,
            &strong,
            &weak,
            &gas_params(),
            &kh_params(),
        );
        assert_eq!(
            banded, expected,
            "the interval propagation byte-equals the hand-computed four-corner min/max band"
        );
        assert!(banded.is_some(), "the compound-band verdict resolves");
        // The endpoint argument order is irrelevant: the function min/maxes over all corners.
        let swapped = giant_formation_on_derived_clock_banded(
            &embryo,
            &disk,
            &star,
            &lp,
            &shu,
            &weak,
            &strong,
            &gas_params(),
            &kh_params(),
        );
        assert_eq!(
            swapped, banded,
            "the collapse and wind endpoint argument order does not change the verdict"
        );
    }

    #[test]
    fn the_both_endpoints_hindcast_reports_the_band_verdict_without_selecting_a_member() {
        // REPORTED, NEVER GATED (the central-member ruling's measurement): evaluate the giant verdict for a
        // super-critical embryo across the full compound (collapse x wind) band, to learn whether the factor-48
        // collapse band renders the verdict vacuous. This asserts the mechanism resolves and NEVER selects a
        // collapse member (the fitting-trap guard); which banded outcome it lands on is the reported datum.
        let disk = mirror_disk(Fixed::from_int(30));
        let field = oligarchic_embryo_field(
            &disk,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(30),
            256,
        );
        let embryo = *field.last().expect("the dense disk seeds embryos");
        let star = banded_clock_state();
        let banded = giant_formation_on_derived_clock_banded(
            &embryo,
            &disk,
            &star,
            &CollapseModel::shu_1977(),
            &CollapseModel::larson_penston(),
            &XrayWindFit::owen_equation_9(),
            &XrayWindFit::sellek_2024(),
            &gas_params(),
            &kh_params(),
        )
        .expect("the compound-band verdict resolves");
        // THE MEASUREMENT (the ruling's spread report), split into three licenses below: the giant verdict over the
        // wind band at EACH collapse corner (Shu and LP), to learn whether the factor-48 collapse band is
        // verdict-stable on this row or splits it. The point wire evaluates each collapse member's own wind-banded
        // verdict.
        let shu_star = DiskClockState {
            collapse: CollapseModel::shu_1977(),
            ..star
        };
        let lp_star = DiskClockState {
            collapse: CollapseModel::larson_penston(),
            ..star
        };
        let corner = |s: &DiskClockState| {
            giant_formation_on_derived_clock(
                &embryo,
                &disk,
                s,
                &XrayWindFit::owen_equation_9(),
                &XrayWindFit::sellek_2024(),
                &gas_params(),
                &kh_params(),
            )
            .expect("the corner verdict resolves")
        };
        let shu_corner = corner(&shu_star);
        let lp_corner = corner(&lp_star);
        // THE SPREAD, SPLIT INTO ITS THREE LICENSES (audit ruling). (1) BAND-AGREEMENT, a machinery property with a
        // tight license, kept BAKED: both collapse corners produce the same verdict CLASS at this fixture. Compared
        // by discriminant, NOT by the full band, so the giant MASS is deliberately NOT asserted here (baking the mass
        // would enshrine the missing termination term as a regression target, the worst thing a test can do).
        assert_eq!(
            std::mem::discriminant(&shu_corner.band),
            std::mem::discriminant(&lp_corner.band),
            "the collapse band is verdict-CLASS-stable on this row (Shu {:?}, LP {:?})",
            shu_corner.band,
            lp_corner.band
        );
        assert_eq!(
            std::mem::discriminant(&banded.band),
            std::mem::discriminant(&shu_corner.band),
            "the compound band agrees in class with the stable corners"
        );
        // (2) THE SOLAR-CLASS VERDICT, gated with its CATASTROPHE-WIDTH justification on the page (the oracle-redesign
        // rule, so this is not a tuned oracle): a clearly super-critical embryo forming NO giant would be a
        // mechanism-class failure decades wide in the underlying quantities (the KH time would have to exceed the
        // derived disk life by orders of magnitude), so RobustGiant here is catastrophe-width, not a calibration.
        assert!(
            matches!(shu_corner.band, BandedGiantOutcome::RobustGiant { .. }),
            "a clearly super-critical embryo forms a giant across the whole band (catastrophe-width)"
        );
        // (3) THE MASS IS REPORTED, NEVER BAKED. The measured giant mass is ~7678 M_earth = ~24.2 M_Jupiter, which is
        // 1.86x above the ~13 M_Jup deuterium-burning boundary: by mass a BROWN DWARF, not a planet. It is a REPORTED
        // hindcast miss (7678 against Jupiter's ~318 M_earth), NOT asserted, because the closure is RESERVOIR
        // EXHAUSTION with NO accretion termination (gap-opening, throttled supply end runaway well before the zone
        // empties), so the mass is an UPPER BOUND only (the TERMS-DROPPED at the giant-mass site, routed to the
        // termination slice). CONDITIONING: the mass invariance across the collapse band is a CONSEQUENCE of the
        // terminationless closure; once termination lands, mass may couple to the supply rate and this spread must be
        // RE-MEASURED, so the termination slice re-runs this fixture. A near-threshold-embryo row (where the factor-48
        // bites the verdict hardest) is a named debt, the sensitivity the population hindcast would otherwise owe.
    }

    #[test]
    fn the_gap_opening_mass_terminates_the_runaway_at_a_jupiter_class_cap() {
        // The ACCRETION-TERMINATION mechanism (the audit debt): runaway gas accretion halts when the giant opens a
        // gap, so the Crida 2006 gap-opening mass caps the reservoir-exhaustion upper bound. Coefficients are the
        // vendored Crida numbers (disk_arc_literature manifest).
        let crida = GapOpeningModel::crida_2006();
        assert_eq!(crida.gravity_term_coeff, Fixed::from_ratio(3, 4)); // Eq. 15
        assert_eq!(crida.viscous_term_coeff, Fixed::from_int(50)); // Eq. 15
        assert_eq!(crida.hill_factor_denominator, Fixed::from_int(3)); // Hill radius (q/3)^(1/3)
                                                                       // The alpha-disk Reynolds bridge (the standard Shakura-Sunyaev substitution, not Crida's): alpha=1e-2,
                                                                       // H/r=0.05 gives R = 1/(alpha (H/r)^2) = 40000.
        let h_over_r = Fixed::from_ratio(5, 100);
        let alpha = Fixed::from_ratio(1, 100);
        let re = alpha_disk_reynolds_number(alpha, h_over_r).unwrap();
        assert!(
            (re.to_f64_lossy() - 40000.0).abs() < 1.0,
            "R = 1/(alpha (H/r)^2) = 40000 (got {})",
            re.to_f64_lossy()
        );
        // VALIDATION: for solar-ish disk conditions (H/r=0.05, alpha=1e-2) around a 1 M_sun star the gap-opening
        // mass is a Jupiter-class ~2 M_Jup (~710 Earth masses), the physical cap that retires the reservoir-
        // exhaustion tens-of-Jupiter upper bound. Reported inside a band, the mechanism not a fit.
        let m_gap = gap_opening_mass_earth(h_over_r, re, Fixed::ONE, &crida).unwrap();
        assert!(
            m_gap.to_f64_lossy() > 600.0 && m_gap.to_f64_lossy() < 850.0,
            "the gap-opening mass is Jupiter-class ~710 M_earth (~2 M_Jup) (got {})",
            m_gap.to_f64_lossy()
        );
        // THE TERMINATION RULE: M_final = min(reservoir, M_gap). A super-critical embryo whose reservoir would carry
        // it to the audited 7678 M_earth (24 M_Jup, brown-dwarf-class) is instead capped at the gap-opening mass.
        let terminated = runaway_terminated_giant_mass_earth(Fixed::from_int(7678), m_gap).unwrap();
        assert_eq!(
            terminated, m_gap,
            "the runaway is capped at the gap-opening mass"
        );
        // and when the reservoir is the smaller bound it wins (a gas-poor annulus never reaches the gap).
        let gas_poor = runaway_terminated_giant_mass_earth(Fixed::from_int(100), m_gap).unwrap();
        assert_eq!(gas_poor, Fixed::from_int(100));
        // THERMAL (inviscid) ASYMPTOTE: as R grows the viscous term vanishes and the criterion reduces to
        // P = (3/4)(H/R_H) = 1, giving q = 3*((3/4)(H/r))^3, its own fitted thermal limit (distinct from the
        // R_H>=H physical criterion's 3(H/r)^3, the 3/4 fit coefficient the difference). For H/r=0.05 that is
        // ~53 M_earth.
        let m_thermal =
            gap_opening_mass_earth(h_over_r, Fixed::from_int(1_000_000_000), Fixed::ONE, &crida)
                .unwrap();
        assert!(
            m_thermal.to_f64_lossy() > 45.0 && m_thermal.to_f64_lossy() < 62.0,
            "the near-inviscid gap mass approaches the Crida thermal limit ~53 M_earth (got {})",
            m_thermal.to_f64_lossy()
        );
        // MECHANISM, a more viscous disk holds a wider gap open, so it takes a HEAVIER planet: M_gap rises with alpha.
        let re_visc = alpha_disk_reynolds_number(Fixed::from_ratio(3, 100), h_over_r).unwrap();
        let m_gap_visc = gap_opening_mass_earth(h_over_r, re_visc, Fixed::ONE, &crida).unwrap();
        assert!(
            m_gap_visc > m_gap,
            "a more viscous disk needs a heavier planet to open a gap (alpha=3e-2 {} > alpha=1e-2 {})",
            m_gap_visc.to_f64_lossy(),
            m_gap.to_f64_lossy()
        );
        // MECHANISM, a THINNER disk opens a gap at a LIGHTER planet: M_gap falls with H/r.
        let thin = Fixed::from_ratio(3, 100);
        let re_thin = alpha_disk_reynolds_number(alpha, thin).unwrap();
        let m_gap_thin = gap_opening_mass_earth(thin, re_thin, Fixed::ONE, &crida).unwrap();
        assert!(
            m_gap_thin < m_gap,
            "a thinner disk opens a gap at a lighter planet (H/r=0.03 {} < H/r=0.05 {})",
            m_gap_thin.to_f64_lossy(),
            m_gap.to_f64_lossy()
        );
        // An extreme viscosity whose gap-opening mass would exceed the q=0.1 (~100 M_Jup, stellar) bracket ceiling
        // fails loud, never a fabricated cap. Only an unphysical alpha (~0.9) pushes the gap mass past that ceiling.
        let re_thick = alpha_disk_reynolds_number(Fixed::from_ratio(9, 10), h_over_r).unwrap();
        assert!(gap_opening_mass_earth(h_over_r, re_thick, Fixed::ONE, &crida).is_none());
        // Non-physical inputs fail soft.
        assert!(alpha_disk_reynolds_number(Fixed::ZERO, h_over_r).is_none());
        assert!(gap_opening_mass_earth(Fixed::ZERO, re, Fixed::ONE, &crida).is_none());
        assert!(runaway_terminated_giant_mass_earth(Fixed::ZERO, m_gap).is_none());
    }

    #[test]
    fn the_typed_outcome_makes_the_deuterium_bound_sound_on_the_terminated_mass() {
        // The audit resolution, end to end: termination caps the runaway, and the typed outcome reads the capped
        // mass against the vendored fusion boundaries, so a genuine planet is a planet and only a body past the
        // deuterium line is a brown dwarf. Boundaries are the vendored Spiegel 2011 / Chabrier 2000 fiducials.
        let limits = BurningLimits::spiegel_chabrier();
        assert_eq!(limits.deuterium_limit_m_jup, Fixed::from_int(13)); // ~13 M_Jup (Spiegel 2011)
        assert_eq!(limits.hydrogen_limit_m_sun, Fixed::from_ratio(72, 1000)); // ~0.072 M_sun (Chabrier 2000)
                                                                              // THE FIX, end to end: the audited super-critical embryo's UN-terminated reservoir mass (7678 M_earth =
                                                                              // 24.2 M_Jup) types as a BROWN DWARF (past the ~13 M_Jup deuterium line, the audit's mis-report), but the
                                                                              // TERMINATED mass min(reservoir, M_gap) types as a PLANET, the sound outcome.
        let crida = GapOpeningModel::crida_2006();
        let re = alpha_disk_reynolds_number(Fixed::from_ratio(1, 100), Fixed::from_ratio(5, 100))
            .unwrap();
        let m_gap =
            gap_opening_mass_earth(Fixed::from_ratio(5, 100), re, Fixed::ONE, &crida).unwrap();
        let un_terminated = Fixed::from_int(7678);
        let terminated = runaway_terminated_giant_mass_earth(un_terminated, m_gap).unwrap();
        assert_eq!(
            giant_mass_class(un_terminated, &limits).unwrap(),
            GiantMassClass::BrownDwarf,
            "the un-terminated 24 M_Jup ceiling is a brown dwarf by mass (the audit finding)"
        );
        assert_eq!(
            giant_mass_class(terminated, &limits).unwrap(),
            GiantMassClass::Planet,
            "the terminated ~2 M_Jup giant is a genuine planet (the sound outcome)"
        );
        // THE BOUNDARIES: the deuterium line is ~4132 M_earth (13 M_Jup), the hydrogen line ~23980 M_earth
        // (0.072 M_sun). Straddle each with a body just below and just above.
        assert_eq!(
            giant_mass_class(Fixed::from_int(4000), &limits).unwrap(),
            GiantMassClass::Planet
        );
        assert_eq!(
            giant_mass_class(Fixed::from_int(4300), &limits).unwrap(),
            GiantMassClass::BrownDwarf
        );
        assert_eq!(
            giant_mass_class(Fixed::from_int(23000), &limits).unwrap(),
            GiantMassClass::BrownDwarf
        );
        assert_eq!(
            giant_mass_class(Fixed::from_int(24500), &limits).unwrap(),
            GiantMassClass::Star,
            "past ~0.072 M_sun the body sustains hydrogen fusion, a star"
        );
        // Non-physical inputs fail soft.
        assert!(giant_mass_class(Fixed::ZERO, &limits).is_none());
    }

    #[test]
    fn the_integration_terminates_and_types_the_giant_from_the_disk() {
        // The INTEGRATION: threading the disk aspect ratio through a giant verdict caps the runaway and types the
        // capped mass, so the giant gate reports a sound mass instead of the reservoir-exhaustion ceiling. Additive
        // over the existing verdict, so giant_formation is unchanged.
        let disk = mirror_disk(Fixed::from_int(30));
        let field = oligarchic_embryo_field(
            &disk,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(30),
            256,
        );
        let embryo = *field
            .last()
            .expect("the dense disk seeds a super-critical embryo");
        let verdict =
            giant_formation(&embryo, &disk, Fixed::ONE, &gas_params(), &kh_params()).unwrap();
        let un_terminated = match verdict.outcome {
            GiantOutcome::Giant { final_mass_earth } => final_mass_earth,
            GiantOutcome::Terrestrial => panic!("the super-critical embryo should run away"),
        };
        // The derived disk aspect ratio at the embryo's orbit is a plausible thin-disk value.
        let h_over_r = disk_aspect_ratio_at_orbit(&disk, embryo.orbit_au).unwrap();
        assert!(
            h_over_r.to_f64_lossy() > 0.01 && h_over_r.to_f64_lossy() < 0.2,
            "H/r is a plausible disk aspect ratio (got {})",
            h_over_r.to_f64_lossy()
        );
        let gap_model = GapOpeningModel::crida_2006();
        let limits = BurningLimits::spiegel_chabrier();
        let t = terminate_and_type_giant(&verdict, &disk, Fixed::ONE, &gap_model, &limits).unwrap();
        // Termination never raises the mass, and the terminated mass is min(reservoir, M_gap).
        assert_eq!(t.un_terminated_mass_earth, un_terminated);
        assert!(
            t.terminated_mass_earth <= un_terminated,
            "termination never raises the mass"
        );
        let expected =
            runaway_terminated_giant_mass_earth(un_terminated, t.gap_mass_earth).unwrap();
        assert_eq!(t.terminated_mass_earth, expected);
        // The class is the class of the terminated (sound) mass, not the reservoir ceiling.
        assert_eq!(
            t.mass_class,
            giant_mass_class(t.terminated_mass_earth, &limits).unwrap()
        );
        // THE AUDIT FIX, reported: if the un-terminated ceiling was a brown dwarf (or a star) by mass, the
        // terminated mass is a strictly lower class, so termination rescued a genuine planet from a mis-report.
        let ceiling_class = giant_mass_class(un_terminated, &limits).unwrap();
        if ceiling_class != GiantMassClass::Planet {
            assert!(
                t.terminated_mass_earth < un_terminated,
                "a super-Jupiter reservoir ceiling is trimmed by the gap cap (ceiling {:?}, terminated {})",
                ceiling_class,
                t.terminated_mass_earth.to_f64_lossy()
            );
        }
        // A TERRESTRIAL verdict has no giant to terminate.
        let terrestrial = GiantVerdict {
            outcome: GiantOutcome::Terrestrial,
            ..verdict
        };
        assert!(
            terminate_and_type_giant(&terrestrial, &disk, Fixed::ONE, &gap_model, &limits)
                .is_none()
        );
    }

    #[test]
    fn the_deuterium_line_derives_from_metallicity_and_moves_the_class() {
        // The composition follow-on: the deuterium line derives from the disk metallicity off the Spiegel
        // COOLTLUSTY track, so a metal-rich world types a giant of a given mass differently from a metal-poor one.
        let line = DeuteriumLine::spiegel_cooltlusty();
        assert_eq!(line.m_jup_at_solar, Fixed::from_ratio(1348, 100)); // 13.48 M_Jup at solar (Spiegel T1)
                                                                       // The cited anchors, at 0.316 / 1 / 3.16 solar metallicity.
        let solar = deuterium_burning_limit_m_jup(Fixed::ONE, &line).unwrap();
        assert!(
            (solar.to_f64_lossy() - 13.48).abs() < 0.02,
            "the solar-metallicity deuterium line is 13.48 M_Jup (got {})",
            solar.to_f64_lossy()
        );
        let metal_rich = deuterium_burning_limit_m_jup(Fixed::from_ratio(316, 100), &line).unwrap();
        let metal_poor =
            deuterium_burning_limit_m_jup(Fixed::from_ratio(316, 1000), &line).unwrap();
        assert!(
            (metal_rich.to_f64_lossy() - 13.13).abs() < 0.03
                && (metal_poor.to_f64_lossy() - 13.77).abs() < 0.03,
            "the track endpoints are 13.13 (3.16 solar) and 13.77 (0.316 solar) (got {}, {})",
            metal_rich.to_f64_lossy(),
            metal_poor.to_f64_lossy()
        );
        // MECHANISM: higher metallicity lowers the line (opacity blankets the core to fusion at a lower mass).
        assert!(
            metal_rich < solar && solar < metal_poor,
            "higher metallicity gives a lower deuterium line"
        );
        // The clamp holds the endpoints beyond the track span.
        assert_eq!(
            deuterium_burning_limit_m_jup(Fixed::from_int(10), &line).unwrap(),
            line.m_jup_at_triple_solar
        );
        // ADMIT THE ALIEN, the point of the derivation: a body of 4300 M_earth (~13.5 M_Jup) sits BETWEEN the
        // metal-rich line (13.13 M_Jup) and the metal-poor line (13.77 M_Jup), so it types as a brown dwarf in a
        // metal-rich world and a genuine planet in a metal-poor one. The fiducial ~13 M_Jup folds this away.
        let mass = Fixed::from_int(4300);
        let rich_limits =
            BurningLimits::from_metallicity(Fixed::from_ratio(316, 100), &line).unwrap();
        let poor_limits =
            BurningLimits::from_metallicity(Fixed::from_ratio(316, 1000), &line).unwrap();
        assert_eq!(
            giant_mass_class(mass, &rich_limits).unwrap(),
            GiantMassClass::BrownDwarf,
            "a metal-rich world's lower deuterium line makes the same mass a brown dwarf"
        );
        assert_eq!(
            giant_mass_class(mass, &poor_limits).unwrap(),
            GiantMassClass::Planet,
            "a metal-poor world's higher deuterium line makes the same mass a planet"
        );
        // Non-physical metallicity fails soft.
        assert!(deuterium_burning_limit_m_jup(Fixed::ZERO, &line).is_none());
        assert!(BurningLimits::from_metallicity(Fixed::ZERO, &line).is_none());
    }

    /// The truncation ledger is EXACTLY conserving: retained plus removed reconstructs the total bit for bit,
    /// because the partition buckets the same quadrature rings, so no residual can leak between the two readings.
    #[test]
    fn the_truncation_ledger_conserves_exactly() {
        let disk = mirror_disk(Fixed::ONE);
        let inner = r(1, 10);
        let birth_r1 = Fixed::from_int(30);
        let truncation_radius = Fixed::from_int(12); // a mid-domain cap, so both buckets are non-empty
        let ledger = truncation_gas_ledger(&disk, inner, birth_r1, truncation_radius, 128).unwrap();
        assert_eq!(
            ledger.total_mass_earth,
            ledger
                .retained_mass_earth
                .checked_add(ledger.removed_mass_earth)
                .unwrap(),
            "retained + removed mass reconstructs the total exactly"
        );
        assert_eq!(
            ledger.total_proxy_l,
            ledger
                .retained_proxy_l
                .checked_add(ledger.removed_proxy_l)
                .unwrap(),
            "retained + removed angular momentum reconstructs the total exactly"
        );
        // A real cut removes gas AND angular momentum: the residual is non-empty.
        assert!(ledger.removed_mass_earth > Fixed::ZERO);
        assert!(ledger.removed_proxy_l > Fixed::ZERO);
        assert!(ledger.retained_mass_earth > Fixed::ZERO);
    }

    /// The `total` the ledger reports is the same object [`disk_gas_content`] integrates over `[inner, R_1]` with the
    /// same step count: the ledger only re-buckets that sum, so their totals agree ring for ring.
    #[test]
    fn the_ledger_total_matches_the_unpartitioned_gas_content() {
        let disk = mirror_disk(Fixed::ONE);
        let inner = r(1, 10);
        let birth_r1 = Fixed::from_int(30);
        let steps = 96;
        let ledger =
            truncation_gas_ledger(&disk, inner, birth_r1, Fixed::from_int(12), steps).unwrap();
        let (mass, proxy_l) = disk_gas_content(&disk, inner, birth_r1, steps).unwrap();
        assert_eq!(ledger.total_mass_earth, mass);
        assert_eq!(ledger.total_proxy_l, proxy_l);
    }

    /// A wide or absent companion (`R_t >= R_1`) leaves everything retained and nothing removed: the untruncated
    /// identity, so the ledger costs the disk nothing when there is no cut to make.
    #[test]
    fn a_wide_companion_removes_nothing() {
        let disk = mirror_disk(Fixed::ONE);
        let inner = r(1, 10);
        let birth_r1 = Fixed::from_int(30);
        // The truncation radius sits beyond the birth radius: no ring is outside it.
        let ledger =
            truncation_gas_ledger(&disk, inner, birth_r1, Fixed::from_int(45), 64).unwrap();
        assert_eq!(ledger.removed_mass_earth, Fixed::ZERO);
        assert_eq!(ledger.removed_proxy_l, Fixed::ZERO);
        assert_eq!(ledger.retained_mass_earth, ledger.total_mass_earth);
        assert_eq!(ledger.retained_proxy_l, ledger.total_proxy_l);
    }

    /// A tighter cut retains less: monotonicity in the truncation radius, the property a dynamic sink would read.
    #[test]
    fn a_tighter_cut_retains_less_gas() {
        let disk = mirror_disk(Fixed::ONE);
        let inner = r(1, 10);
        let birth_r1 = Fixed::from_int(30);
        let wide = truncation_gas_ledger(&disk, inner, birth_r1, Fixed::from_int(20), 128).unwrap();
        let tight = truncation_gas_ledger(&disk, inner, birth_r1, Fixed::from_int(8), 128).unwrap();
        assert!(
            tight.retained_mass_earth < wide.retained_mass_earth,
            "a tighter truncation radius retains less gas ({} < {})",
            tight.retained_mass_earth.to_f64_lossy(),
            wide.retained_mass_earth.to_f64_lossy()
        );
        assert!(
            tight.removed_mass_earth > wide.removed_mass_earth,
            "a tighter truncation radius removes more gas"
        );
    }

    /// Degenerate domains fail soft, matching [`disk_gas_content`].
    #[test]
    fn the_truncation_ledger_rejects_degenerate_domains() {
        let disk = mirror_disk(Fixed::ONE);
        let birth_r1 = Fixed::from_int(30);
        // Inverted domain (inner past the birth radius).
        assert!(truncation_gas_ledger(
            &disk,
            Fixed::from_int(40),
            birth_r1,
            Fixed::from_int(12),
            64
        )
        .is_none());
        // Zero steps.
        assert!(truncation_gas_ledger(&disk, r(1, 10), birth_r1, Fixed::from_int(12), 0).is_none());
        // Non-positive truncation radius.
        assert!(truncation_gas_ledger(&disk, r(1, 10), birth_r1, Fixed::ZERO, 64).is_none());
    }
}
