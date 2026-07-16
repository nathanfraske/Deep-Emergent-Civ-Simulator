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

//! The MULTI-BODY SYSTEM GENERATOR (the solar-system arc, task #72): from a star and its disk, DERIVE the field of
//! proto-planets, so a whole system EMERGES rather than one authored planet. Nothing here places a planet by hand or
//! fixes a count: the number, the masses, and the spacing all fall out of the disk's own surface-density profile and
//! the physics primitives already built (`astro::viscous_similarity_surface_density` the DERIVED gas reservoir,
//! `astro::isolation_mass_earth` the Kokubo-Ida oligarchic mass, `astro::hill_radius_au` the feeding-zone width,
//! `astro::disk_effective_temperature` the temperature that locates the ice line and scales the gas density). This slice derives the OLIGARCHIC EMBRYO FIELD (the proto-planets left
//! when each feeding zone has swept its isolation mass); the giant-impact assembly that merges the embryos into final
//! planets is the next slice (the deterministic realisation of a chaotic phase, flagged for grounding, item R-ASSEMBLY).
//!
//! The one emergence guard: zero the disk and no embryo forms; make the disk denser and the embryos grow more massive
//! and the count shifts; cross the ice line and the condensed solids jump, so the outer embryos are the larger ones
//! (the seeds a later gas-accretion arc turns into giants). The tests prove each, so the field is read off the disk,
//! never painted.
//!
//! DERIVED here, per system: the metal fraction Z (the disk composition's own, per-system, admitting the alien), the
//! ice-line orbit (where the derived disk temperature crosses the water snow-line temperature the condensation table
//! carries), the isolation mass at each orbit (from the local solid surface density), and the oligarchic spacing step
//! (from the embryo's own Hill radius). The oligarchic spacing width `b` ([`OLIGARCHIC_SPACING_HILL_WIDTHS`]) is a
//! UNIVERSAL class constant with a band ([M class] per the R-ASSEMBLY ruling), about 10 mutual Hill radii (the 5 to 10
//! band), pinned by the timescale-ratio equilibrium of orbital repulsion against dynamical friction whose steepness
//! makes the level set near-universal, weakly dependent on mass and orbit; it is not a per-system knob. The derived
//! repulsion-versus-friction form (calibrated to the Kokubo-Ida ensembles, fetch Goldreich-Lithwick-Sari 2004) is the
//! optional alien-admitting upgrade that gives the weak per-system modulation from the form rather than a bare number.
//! Still RESERVED-with-basis: the feeding-zone width `C` (already reserved in `isolation_mass_earth`), and the
//! refractory fraction of the metals that condenses inside the ice line (basis: the rock and metal formers as a share
//! of the disk metals, about half for a solar pattern; derive-down: the condensation substrate's own condensed-mass
//! fraction at the disk temperature, `condensed_amounts`). The disk surface-density residues `r_c`, `gamma`, `Sigma_c`
//! are now RETIRED (the GATE-G derivation, R-ASSEMBLY): the local gas surface density is DERIVED at each orbit from the
//! disk realization through the steady-state viscous similarity `Sigma = Mdot / (3*pi*nu)`
//! ([`crate::astro::viscous_similarity_surface_density`]), so it carries zero new per-system initial conditions, being
//! a VIEW of the accretion rate `Mdot`, the viscosity `alpha`, the star mass `M_star`, and the derived disk temperature
//! `T(r)`; the slope `gamma ~ 1` emerges from the viscous physics rather than being an authored residue. Still
//! RESERVED-with-basis at the disk level: `alpha` (the Shakura-Sunyaev turbulent viscosity, ~0.001 to 0.01, cited
//! Shakura & Sunyaev 1973) and `mu` (the disk-gas mean molecular weight, ~2.34 for a solar H2+He mix), each a per-disk
//! or per-composition datum, so a quiescent disk or a carbon-rich disk is a data row.

use civsim_core::Fixed;

use crate::astro::{
    disk_effective_temperature, hill_radius_au, isolation_mass_earth,
    viscous_similarity_surface_density,
};

/// The oligarchic embryo spacing `b`, in mutual Hill radii: a class constant with a band ([M class] per the R-ASSEMBLY
/// ruling, about 10, the 5 to 10 band), pinned by the timescale-ratio equilibrium of orbital repulsion against
/// dynamical friction, weakly dependent on protoplanet mass and orbit. It is not a per-system knob; the derived
/// repulsion-versus-friction form (fetch Goldreich-Lithwick-Sari 2004) is the optional alien-admitting upgrade. Passed
/// to [`oligarchic_embryo_field`] as `spacing_hill_widths`, so a scenario can still probe the band. BRANCH SCOPE (the
/// R-ASSEMBLY self-audit): this is the class constant of the PLANETESIMAL-driven oligarchic branch specifically, not
/// universal across all accretion; the rival is pebble accretion, which terminates at the pebble isolation mass (the
/// embryo halts the inward pebble flux) rather than feeding-zone exhaustion, at a different spacing. Which branch a
/// world takes is itself DERIVED per-system from the planetesimal-to-pebble surface-density (flux) ratio; the
/// pebble-branch embryo field is its own calibration target or a declared wall until bought. The constant's name
/// scopes it honestly.
pub const OLIGARCHIC_SPACING_HILL_WIDTHS: Fixed = Fixed::from_int(10);

/// The disk's THERMAL residues, the arguments the two-regime `disk_effective_temperature` reads to place the ice line:
/// the accretion rate (solar masses per Myr), the star's mass ratio and mass-luminosity exponent (its irradiation),
/// the reprocessing factor and inner-boundary factor (the irradiation geometry), and the `Fixed` overflow bound. Each
/// is the per-system disk's own datum, the same set `derive_planet` already threads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DiskThermalParams {
    /// The disk accretion rate in solar masses per Myr.
    pub accretion_rate_msun_myr: Fixed,
    /// The star mass ratio `M / M_sun`.
    pub star_mass_ratio: Fixed,
    /// The mass-luminosity exponent (the star's luminosity `L ~ M^alpha`).
    pub mass_luminosity_exponent: Fixed,
    /// The disk reprocessing factor (the fraction of stellar flux the disk intercepts and re-radiates).
    pub reprocessing_factor: Fixed,
    /// The inner-boundary factor (the magnetospheric truncation geometry).
    pub inner_boundary_factor: Fixed,
    /// The `Fixed` overflow bound handed to the temperature solve.
    pub t_max: Fixed,
}

/// The DERIVED solid disk: the disk's thermal and viscosity realization plus the two facts that turn gas into a solid
/// reservoir, the metal fraction `Z` (per system, the disk composition's own) and the ice-line orbit (where the disk
/// cools past the water snow line and ice joins the solids). Inside the ice line only the refractory share of the
/// metals is solid; beyond it the full metal fraction condenses. The gas surface density at each orbit is DERIVED from
/// the viscous similarity (no `Sigma_c`, `gamma`, or `r_c` residues, the retired GATE-G target). Build it through
/// [`SolidDisk::derive`], which locates the ice line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SolidDisk {
    /// The disk's thermal realization (accretion rate, star mass, irradiation geometry), read to derive both the disk
    /// temperature `T(r)` and, through it, the local gas surface density at each orbit.
    pub thermal: DiskThermalParams,
    /// The Shakura-Sunyaev turbulent-viscosity parameter `alpha` (reserved-with-basis: ~0.001 to 0.01, cited Shakura &
    /// Sunyaev 1973; a per-disk datum), passed to [`crate::astro::viscous_similarity_surface_density`].
    pub alpha_viscosity: Fixed,
    /// The disk-gas mean molecular weight `mu` (reserved-with-basis: ~2.34 for a solar H2+He mix; a per-composition
    /// datum, so a carbon-rich or metal-poor disk is a data row), passed to the viscous-similarity gas density.
    pub mean_molecular_weight: Fixed,
    /// The disk metal (heavy-element) mass fraction `Z`, the disk composition's own, per system.
    pub metal_fraction: Fixed,
    /// The refractory share of the metals that is solid inside the ice line (reserved-with-basis: the rock and metal
    /// formers as a fraction of the disk metals, about half for a solar pattern; derive-down: the condensation
    /// substrate's condensed-mass fraction at the disk temperature).
    pub refractory_fraction: Fixed,
    /// The DERIVED ice-line orbit (AU): inside it the disk is warmer than the water snow line, outside it colder.
    pub ice_line_au: Fixed,
}

impl SolidDisk {
    /// Derive the solid disk, locating the ice line by the disk temperature: the orbit where the two-regime disk
    /// temperature crosses `snow_line_temperature_k` (the caller reads the water-ice onset from the condensation
    /// table, `t_first_k = 182 K`, never authored here). `alpha_viscosity` and `mean_molecular_weight` are the two
    /// reserved-with-basis disk data the viscous-similarity gas density reads. `inner_au` and `outer_au` bound the
    /// search. `None` if the ice line does not bracket in range or a temperature fails to resolve.
    #[allow(clippy::too_many_arguments)]
    pub fn derive(
        thermal: DiskThermalParams,
        alpha_viscosity: Fixed,
        mean_molecular_weight: Fixed,
        metal_fraction: Fixed,
        refractory_fraction: Fixed,
        snow_line_temperature_k: Fixed,
        inner_au: Fixed,
        outer_au: Fixed,
    ) -> Option<SolidDisk> {
        let ice_line_au = ice_line_au(&thermal, snow_line_temperature_k, inner_au, outer_au)?;
        Some(SolidDisk {
            thermal,
            alpha_viscosity,
            mean_molecular_weight,
            metal_fraction,
            refractory_fraction,
            ice_line_au,
        })
    }

    /// The SOLID surface density (kg/m^2) at an orbit: the DERIVED gas surface density (the viscous similarity read at
    /// this orbit's disk temperature) times the condensed metal fraction, which is the full metal fraction beyond the
    /// ice line (ice condensed) and the refractory share inside it. The ice-line jump is the ratio of the two, so the
    /// outer disk carries the larger solid reservoir. The gas density is a VIEW of the disk realization, no `Sigma_c`
    /// input. `None` if the disk temperature or the gas density fails to resolve (past the disk edge, or a
    /// non-positive accretion rate) or a product overflows.
    pub fn solid_surface_density_kg_m2(&self, orbit_au: Fixed) -> Option<Fixed> {
        let temperature = disk_effective_temperature(
            self.thermal.accretion_rate_msun_myr,
            self.thermal.star_mass_ratio,
            self.thermal.mass_luminosity_exponent,
            orbit_au,
            self.thermal.reprocessing_factor,
            self.thermal.inner_boundary_factor,
            self.thermal.t_max,
        )?;
        let gas = viscous_similarity_surface_density(
            orbit_au,
            self.thermal.star_mass_ratio,
            self.thermal.accretion_rate_msun_myr,
            temperature,
            self.alpha_viscosity,
            self.mean_molecular_weight,
        )?;
        let condensed_fraction = if orbit_au < self.ice_line_au {
            self.metal_fraction.checked_mul(self.refractory_fraction)?
        } else {
            self.metal_fraction
        };
        gas.checked_mul(condensed_fraction)
    }
}

/// The ice-line orbit (AU): the distance where the disk cools to the water snow line. The two-regime disk temperature
/// falls monotonically with distance, so a bounded bisection over `[inner_au, outer_au]` brackets the crossing. If the
/// inner edge is already colder than the snow line (a faint star, ice everywhere in range) the ice line is the inner
/// edge; if the outer edge is still warmer (no ice within range) the crossing is not bracketed and this returns `None`.
/// The iteration count is a fixed accuracy bound (determinism by construction), not a physical knob.
pub fn ice_line_au(
    thermal: &DiskThermalParams,
    snow_line_temperature_k: Fixed,
    inner_au: Fixed,
    outer_au: Fixed,
) -> Option<Fixed> {
    if inner_au <= Fixed::ZERO || outer_au <= inner_au {
        return None;
    }
    let t_at = |r: Fixed| -> Option<Fixed> {
        disk_effective_temperature(
            thermal.accretion_rate_msun_myr,
            thermal.star_mass_ratio,
            thermal.mass_luminosity_exponent,
            r,
            thermal.reprocessing_factor,
            thermal.inner_boundary_factor,
            thermal.t_max,
        )
    };
    let t_inner = t_at(inner_au)?;
    let t_outer = t_at(outer_au)?;
    // The disk falls with distance: warm inside, cold outside. The ice line needs the snow-line temperature bracketed.
    if t_inner <= snow_line_temperature_k {
        return Some(inner_au); // even the inner disk is colder than the snow line: ice line at the inner edge
    }
    if t_outer >= snow_line_temperature_k {
        return None; // the whole disk in range is warmer than the snow line: no ice line here
    }
    let mut lo = inner_au; // warmer than the snow line
    let mut hi = outer_au; // colder than the snow line
    let two = Fixed::from_int(2);
    // 48 halvings drive the bracket below the representable orbit resolution; a fixed count keeps determinism.
    for _ in 0..48 {
        let mid = lo.checked_add(hi)?.checked_div(two)?;
        let t_mid = t_at(mid)?;
        if t_mid > snow_line_temperature_k {
            lo = mid; // still warmer: the crossing is farther out
        } else {
            hi = mid; // colder: the crossing is farther in
        }
    }
    lo.checked_add(hi)?.checked_div(two)
}

/// One OLIGARCHIC EMBRYO: a proto-planet that has swept its feeding zone to its isolation mass, at its orbit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Embryo {
    /// The orbit (AU).
    pub orbit_au: Fixed,
    /// The isolation mass (Earth masses), swept from the local solid surface density.
    pub mass_earth: Fixed,
}

/// The OLIGARCHIC EMBRYO FIELD across the disk: starting at the inner edge, an embryo forms at its local isolation
/// mass, then the next forms one oligarchic spacing outward (the spacing measured in the embryo's own Hill radii), and
/// so on to the outer edge. The number, the masses, and the spacing all EMERGE from the solid disk: a denser disk
/// grows more massive embryos, and crossing the ice line the solids jump so the outer embryos are the larger seeds.
/// Nothing fixes the count; `max_embryos` is only a loop bound (a determinism and cost cap, not a physical limit),
/// and the field stops early at the disk edge (where the solid density fails to resolve) or on a non-positive mass.
///
/// `spacing_hill_widths` is the oligarchic width `b`, the universal class constant [`OLIGARCHIC_SPACING_HILL_WIDTHS`]
/// (about 10 mutual Hill radii, the 5 to 10 band, [M class] per R-ASSEMBLY); it is passed as an argument so a scenario
/// can probe the band, but it is not a per-system knob. `feeding_zone_hill_widths` is the width `C` the isolation mass
/// integrates over (already reserved). The spacing step uses the embryo's own Hill radius as a forward proxy for the
/// mutual Hill radius (the next embryo's mass is not yet known); the exact mutual form is a refinement.
pub fn oligarchic_embryo_field(
    disk: &SolidDisk,
    star_mass_ratio: Fixed,
    spacing_hill_widths: Fixed,
    feeding_zone_hill_widths: Fixed,
    inner_au: Fixed,
    outer_au: Fixed,
    max_embryos: usize,
) -> Vec<Embryo> {
    let mut embryos = Vec::new();
    if inner_au <= Fixed::ZERO || outer_au <= inner_au || spacing_hill_widths <= Fixed::ZERO {
        return embryos;
    }
    let mut orbit = inner_au;
    while orbit <= outer_au && embryos.len() < max_embryos {
        let sigma_solid = match disk.solid_surface_density_kg_m2(orbit) {
            Some(s) if s > Fixed::ZERO => s,
            _ => break, // past the disk's solid edge
        };
        let mass_earth = match isolation_mass_earth(
            orbit,
            star_mass_ratio,
            sigma_solid,
            feeding_zone_hill_widths,
        ) {
            Some(m) if m > Fixed::ZERO => m,
            _ => break,
        };
        embryos.push(Embryo {
            orbit_au: orbit,
            mass_earth,
        });
        // Step one oligarchic spacing outward, in the embryo's own Hill radii.
        let hill = match hill_radius_au(orbit, mass_earth, star_mass_ratio) {
            Some(h) if h > Fixed::ZERO => h,
            _ => break,
        };
        let step = match spacing_hill_widths.checked_mul(hill) {
            Some(s) if s > Fixed::ZERO => s,
            _ => break,
        };
        orbit = match orbit.checked_add(step) {
            Some(o) => o,
            None => break,
        };
    }
    embryos
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    /// A representative Mirror-like disk: a solar metal fraction, a Shakura-Sunyaev viscosity and a solar-mix mean
    /// molecular weight, and the two-regime thermal residues `derive_planet` uses. The gas surface density is now
    /// DERIVED from the viscous similarity (no `Sigma_c` input). The numbers are the per-system disk's own data (a
    /// scenario), the test asserting EMERGENCE (how the field responds), not any absolute calibration.
    fn mirror_solid_disk() -> SolidDisk {
        let thermal = DiskThermalParams {
            accretion_rate_msun_myr: r(1, 100), // 0.01 M_sun/Myr (~1e-8 M_sun/yr, the class-II range)
            star_mass_ratio: Fixed::ONE,
            mass_luminosity_exponent: r(35, 10),
            reprocessing_factor: r(5, 100),
            inner_boundary_factor: Fixed::from_int(4),
            t_max: Fixed::from_int(2_000_000),
        };
        SolidDisk::derive(
            thermal,
            r(1, 100),            // alpha = 0.01 (Shakura-Sunyaev, reserved-with-basis)
            r(234, 100),          // mu = 2.34 (solar H2+He mix, reserved-with-basis)
            r(134, 10_000), // Z = 0.0134, the AGSS09 solar metal mass fraction (per-system datum)
            r(1, 2),        // refractory fraction ~ 0.5 (reserved-with-basis)
            Fixed::from_int(182), // the water snow-line temperature (K), read from the condensation table
            Fixed::ONE,
            Fixed::from_int(40),
        )
        .expect("the Mirror disk locates its ice line in [1, 40] AU")
    }

    #[test]
    fn the_ice_line_falls_between_the_warm_inner_and_cold_outer_disk() {
        let disk = mirror_solid_disk();
        // The ice line is bracketed inside the search span, warm-inside and cold-outside.
        assert!(disk.ice_line_au > Fixed::ONE && disk.ice_line_au < Fixed::from_int(40));
    }

    #[test]
    fn the_solids_jump_outward_across_the_ice_line() {
        // At one common orbit, a disk whose ice line sits FARTHER OUT leaves that orbit inside the snow line (only the
        // refractory metals are solid); a disk whose ice line sits FARTHER IN leaves the same orbit outside it (ice
        // has condensed too). The two disks are identical but for the ice line, so the same gas density is multiplied
        // by the refractory share versus the full metal fraction: the outer regime carries more solids, the jump the
        // reciprocal of the refractory fraction. This exercises `solid_surface_density_kg_m2`, not a tautology.
        let base = mirror_solid_disk();
        let sample_orbit = Fixed::from_int(3);
        let mut inside_regime = base;
        inside_regime.ice_line_au = Fixed::from_int(5); // sample orbit (3) is inside the ice line: refractory only
        let mut outside_regime = base;
        outside_regime.ice_line_au = Fixed::from_int(2); // sample orbit (3) is beyond the ice line: full metals
        let solids_inside = inside_regime
            .solid_surface_density_kg_m2(sample_orbit)
            .unwrap();
        let solids_outside = outside_regime
            .solid_surface_density_kg_m2(sample_orbit)
            .unwrap();
        assert!(
            solids_outside > solids_inside,
            "crossing the ice line boosts the solids at a fixed orbit ({} outside vs {} inside)",
            solids_outside.to_f64_lossy(),
            solids_inside.to_f64_lossy()
        );
    }

    #[test]
    fn the_field_emerges_ordered_and_spaced() {
        let disk = mirror_solid_disk();
        let field = oligarchic_embryo_field(
            &disk,
            Fixed::ONE,
            Fixed::from_int(10), // b ~ 10 mutual Hill radii (Kokubo-Ida)
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(40),
            64,
        );
        assert!(
            field.len() >= 3,
            "a disk this size seeds several embryos, got {}",
            field.len()
        );
        // Orbits strictly increase (an ordered field, no two embryos at one orbit).
        for pair in field.windows(2) {
            assert!(pair[1].orbit_au > pair[0].orbit_au);
        }
    }

    #[test]
    fn a_denser_disk_grows_more_massive_embryos() {
        // A denser disk is now a higher-accretion-rate realization (the steady-state column rises with `Mdot`), so
        // tripling the accretion rate raises the derived gas (and thus solid) surface density at a fixed orbit. The
        // ice line is unchanged between the two, so the condensed fraction at the sample orbit is identical and the
        // solid ratio is the gas ratio.
        let base = mirror_solid_disk();
        let mut denser = base;
        denser.thermal.accretion_rate_msun_myr = base
            .thermal
            .accretion_rate_msun_myr
            .checked_mul(Fixed::from_int(3))
            .unwrap();
        let m_base = base
            .solid_surface_density_kg_m2(Fixed::from_int(2))
            .unwrap();
        let m_denser = denser
            .solid_surface_density_kg_m2(Fixed::from_int(2))
            .unwrap();
        assert!(m_denser > m_base, "a denser disk carries more solids");
        // The isolation mass rises with the solid surface density, so the embryo at a fixed orbit is more massive.
        let iso_base =
            isolation_mass_earth(Fixed::from_int(2), Fixed::ONE, m_base, Fixed::from_int(5))
                .unwrap();
        let iso_denser =
            isolation_mass_earth(Fixed::from_int(2), Fixed::ONE, m_denser, Fixed::from_int(5))
                .unwrap();
        assert!(
            iso_denser > iso_base,
            "a denser disk grows a more massive embryo"
        );
    }

    #[test]
    fn the_embryo_count_is_not_authored() {
        // Two disks of different mass yield different embryo counts: the number is read off the disk, never fixed. The
        // denser disk is a higher-accretion-rate realization, whose larger embryos carry wider Hill radii and so
        // wider oligarchic spacing, changing how many fit across the disk. The loop bound is set high enough that
        // neither field saturates it, so the count reflects the disk rather than the cap.
        let a = mirror_solid_disk();
        let mut b = a;
        b.thermal.accretion_rate_msun_myr = a
            .thermal
            .accretion_rate_msun_myr
            .checked_mul(Fixed::from_int(8))
            .unwrap();
        let field_a = oligarchic_embryo_field(
            &a,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(40),
            4096,
        );
        let field_b = oligarchic_embryo_field(
            &b,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(40),
            4096,
        );
        assert!(!field_a.is_empty() && !field_b.is_empty());
        assert_ne!(
            field_a.len(),
            field_b.len(),
            "the embryo count moves with the disk mass ({} vs {})",
            field_a.len(),
            field_b.len()
        );
    }

    #[test]
    fn the_field_is_deterministic() {
        let disk = mirror_solid_disk();
        let f1 = oligarchic_embryo_field(
            &disk,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(40),
            64,
        );
        let f2 = oligarchic_embryo_field(
            &disk,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(40),
            64,
        );
        assert_eq!(f1, f2, "same disk, same field, bit for bit");
    }

    #[test]
    fn an_empty_disk_yields_no_embryos() {
        // A steady viscous disk with no mass flux carries no gas (`Sigma = Mdot / (3*pi*nu) = 0` at `Mdot = 0`), so
        // the viscous similarity routes to None and no embryo forms: no disk, no planets.
        let mut disk = mirror_solid_disk();
        disk.thermal.accretion_rate_msun_myr = Fixed::ZERO;
        let field = oligarchic_embryo_field(
            &disk,
            Fixed::ONE,
            Fixed::from_int(10),
            Fixed::from_int(5),
            Fixed::ONE,
            Fixed::from_int(40),
            64,
        );
        assert!(field.is_empty(), "no disk, no planets");
    }
}
