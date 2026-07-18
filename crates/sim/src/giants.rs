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
    EARTH_MASS_KG,
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
/// componentwise monotone over the box, so the extrema sit at the corners. THE CORNERS ARE THE ENSEMBLE EXTREMES:
/// the wind ensemble is three rows (Owen appendix-B central, Owen equation-9, Sellek), and `wind_fit_a`/`wind_fit_b`
/// must be its RATE EXTREMES (Owen equation-9 the strongest wind and shortest life, Sellek the weakest and longest),
/// with the Owen appendix-B central row riding as the interior CROSS-CHECK the box-midpoint sentinel evaluates, not a
/// corner. This is where the central-member ruling's warning bites or does not: if the factor-48 collapse band makes
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
        // THE MEASUREMENT, BAKED AS A FINDING (the ruling's spread report): the giant verdict over the wind band at
        // EACH collapse corner (Shu and LP), to learn whether the factor-48 collapse band is verdict-stable on this
        // row or splits it. The point wire evaluates each collapse member's own wind-banded verdict.
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
        // REPORTED FINDING: on this super-critical row the factor-48 collapse band is VERDICT-STABLE, RobustGiant at
        // both corners with the same giant mass (the mass is set by the feeding-zone gas reservoir, not the collapse
        // rate, so it is invariant across the band). So the wide collapse band does NOT render this verdict vacuous:
        // the priority signal says the collapse-model retirement ladder is not urgent for a clearly super-critical
        // embryo (a near-threshold embryo could still be sensitive, untested here). No collapse member is selected.
        assert_eq!(
            shu_corner.band, lp_corner.band,
            "the collapse band is verdict-stable on this row (Shu corner {:?}, LP corner {:?})",
            shu_corner.band, lp_corner.band
        );
        assert!(
            matches!(shu_corner.band, BandedGiantOutcome::RobustGiant { .. }),
            "the super-critical embryo is a robust giant at the Shu corner"
        );
        assert_eq!(
            banded.band, shu_corner.band,
            "the compound band agrees with the stable corners"
        );
    }
}
