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

//! The capstone INTEGRATION SPINE (the generative-and-visible pipeline's backbone): from the authored inputs (a
//! star's mass and metallicity, an orbital distance) and the disk and planet residues, DERIVE the planet, chaining
//! the built stages into one deterministic call. This is the object the render draws (its radius the globe size, the
//! star's `T_eff` the light colour, the disk temperature the surface warmth) and the Hadean-Earth acceptance gate
//! measures. It reads only the floor and the reserved residues; nothing is authored here.
//!
//! The chain, each link a built derivation: the star (`stellar::main_sequence_star`, `L/R/T_eff` from mass and
//! metallicity) -> the disk temperature at the orbit (`astro::disk_effective_temperature`, the two-regime
//! irradiation-plus-viscous profile, the condensation input and the surface warmth) -> the planet radius
//! (`astro::planet_radius_m`, from the accreted mass and the condensed bulk density) -> the surface gravity
//! (`g = G M / R^2`, both `M` and `R` now derived, retiring the hardcoded value). The accreted MASS and the
//! condensed COMPOSITION (hence the bulk density) are the accretion and condensation arcs' outputs; until they wire
//! through (the condensation composition waits on the solar-abundance fetch), they enter as the caller's inputs, so
//! the spine is already whole and the last two links are a mechanical substitution, not a rewrite.

use civsim_core::Fixed;

/// The surface gravity `g` (m/s^2) of a planet from its mass and radius: `g = G M / R^2`, both DERIVED (the mass
/// from accretion, the radius from mass and bulk density), so the hardcoded `9.80665` retires. The wide-magnitude
/// product runs in LOG-SPACE (`G ~ 6.67e-11`, `M ~ 6e24 kg`, `R^2 ~ 4e13 m^2` each leave Q32.32 while the ~9.8
/// result fits): `ln g = ln G + ln M_kg - 2 ln R`. At one Earth mass and ~6371 km this derives ~9.8 m/s^2, the
/// Hadean-gate gravity target and the mandatory sanity check. `None` on a non-positive input or a register miss.
pub fn surface_gravity(mass_earth: Fixed, radius_m: Fixed) -> Option<Fixed> {
    if mass_earth <= Fixed::ZERO || radius_m <= Fixed::ZERO {
        return None;
    }
    let ln_g = civsim_physics::saha::ln_of_decimal(
        civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value,
    )?;
    let ln_m_kg = mass_earth
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(
            crate::astro::EARTH_MASS_KG,
        )?)?;
    let ln_r = radius_m.ln();
    let ln_gravity = ln_g
        .checked_add(ln_m_kg)?
        .checked_sub(ln_r.checked_mul(Fixed::from_int(2))?)?;
    Some(ln_gravity.exp())
}

/// A derived planet: the quantities the render and the Hadean gate read, each DERIVED from the star, the orbit, and
/// the physics floor. `star_luminosity_ratio` and `star_effective_temperature_k` are the star's (the latter the
/// render's blackbody light colour); `disk_temperature_k` is the two-regime disk temperature at the orbit (the
/// condensation input and the surface warmth); `mass_earth`, `bulk_density_g_cm3`, `radius_m`, and
/// `surface_gravity_m_s2` are the planet's, the radius the globe's on-screen size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DerivedPlanet {
    /// The star's luminosity `L / L_sun`.
    pub star_luminosity_ratio: Fixed,
    /// The star's effective temperature (K), the render's blackbody light colour.
    pub star_effective_temperature_k: Fixed,
    /// The disk temperature at the orbit (K), the condensation input and the surface warmth.
    pub disk_temperature_k: Fixed,
    /// The planet mass in Earth masses (the accretion output).
    pub mass_earth: Fixed,
    /// The planet's whole-planet mean density (g/cm^3, the condensed-composition output).
    pub bulk_density_g_cm3: Fixed,
    /// The planet radius (m), the globe's on-screen size.
    pub radius_m: Fixed,
    /// The derived surface gravity (m/s^2).
    pub surface_gravity_m_s2: Fixed,
}

/// Derive the planet from the star, the orbit, and the residues: the capstone spine. The star's mass and metallicity
/// ratios and the four stellar slopes fix `L/R/T_eff` ([`crate::stellar::main_sequence_star`]); the orbit and the
/// disk residues fix the disk temperature ([`crate::astro::disk_effective_temperature`]); the accreted mass and the
/// condensed bulk density fix the radius ([`crate::astro::planet_radius_m`]) and the gravity ([`surface_gravity`]).
/// The stellar and disk residues are reserved-with-basis (the mass-luminosity and mass-radius exponents, the
/// metallicity slopes from the model grids, the accretion rate and reprocessing factor from the disk physics); the
/// mass and bulk density are the accretion and condensation outputs (caller inputs until those arcs wire through).
/// `None` if any link fails to resolve.
#[allow(clippy::too_many_arguments)]
pub fn derive_planet(
    star_mass_ratio: Fixed,
    star_metallicity_ratio: Fixed,
    mass_luminosity_exponent: Fixed,
    mass_radius_exponent: Fixed,
    metallicity_luminosity_exponent: Fixed,
    metallicity_radius_exponent: Fixed,
    orbit_au: Fixed,
    accretion_rate_msun_myr: Fixed,
    reprocessing_factor: Fixed,
    inner_boundary_factor: Fixed,
    planet_mass_earth: Fixed,
    planet_bulk_density_g_cm3: Fixed,
    t_max: Fixed,
) -> Option<DerivedPlanet> {
    let star = crate::stellar::main_sequence_star(
        star_mass_ratio,
        star_metallicity_ratio,
        mass_luminosity_exponent,
        mass_radius_exponent,
        metallicity_luminosity_exponent,
        metallicity_radius_exponent,
        t_max,
    )?;
    let disk_temperature_k = crate::astro::disk_effective_temperature(
        accretion_rate_msun_myr,
        star_mass_ratio,
        mass_luminosity_exponent,
        orbit_au,
        reprocessing_factor,
        inner_boundary_factor,
        t_max,
    )?;
    let radius_m = crate::astro::planet_radius_m(planet_mass_earth, planet_bulk_density_g_cm3)?;
    let surface_gravity_m_s2 = surface_gravity(planet_mass_earth, radius_m)?;
    Some(DerivedPlanet {
        star_luminosity_ratio: star.luminosity_ratio,
        star_effective_temperature_k: star.effective_temperature_k,
        disk_temperature_k,
        mass_earth: planet_mass_earth,
        bulk_density_g_cm3: planet_bulk_density_g_cm3,
        radius_m,
        surface_gravity_m_s2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    #[test]
    fn the_surface_gravity_derives_earth() {
        // g = G M / R^2 at one Earth mass and ~6371 km derives ~9.8 m/s^2 (the Hadean-gate sanity check), the
        // hardcoded 9.80665 retired now that both M and R are derived.
        let radius = crate::astro::planet_radius_m(Fixed::ONE, r(5514, 1000)).unwrap();
        let g = surface_gravity(Fixed::ONE, radius).unwrap();
        assert!(
            (g.to_f64_lossy() - 9.8).abs() < 0.2,
            "Earth surface gravity ~9.8 m/s^2, got {}",
            g.to_f64_lossy()
        );
    }

    #[test]
    fn the_spine_derives_a_sun_earth_planet_end_to_end() {
        // The capstone spine, Sun + 1 AU + Earth's accreted mass and mean density: the star reads its ~5772 K T_eff
        // (the render's light colour), the disk its ~equilibrium surface warmth at 1 AU, the planet its ~6371 km
        // radius and ~9.8 m/s^2 gravity. The stellar slopes are the grid-extracted values (alpha ~3.5, beta ~0.8,
        // lambda -0.44, mu -0.018); the disk residues are Mirror fixtures. This is the generative chain end to end;
        // the exact Hadean-Earth match is the acceptance gate, not this smoke test.
        let planet = derive_planet(
            Fixed::ONE,    // star mass = 1 M_sun
            Fixed::ONE,    // metallicity = solar
            r(35, 10),     // mass-luminosity exponent alpha
            r(8, 10),      // mass-radius exponent beta
            r(-44, 100),   // metallicity-luminosity lambda
            r(-18, 1000),  // metallicity-radius mu
            Fixed::ONE,    // orbit = 1 AU
            r(1, 100),     // accretion rate ~1e-8 M_sun/yr (Mirror fixture)
            r(1, 4),       // reprocessing factor 1/4 (spherical-grain equilibrium)
            Fixed::ONE,    // inner-boundary factor ~1 in the bulk disk
            Fixed::ONE,    // accreted mass = 1 Earth mass (accretion output, wired later)
            r(5514, 1000), // bulk density 5.514 g/cm^3 (condensation output, wired later)
            Fixed::from_int(100_000),
        )
        .expect("the Sun-Earth spine derives");
        assert!(
            (planet.star_effective_temperature_k.to_f64_lossy() - 5772.0).abs() < 60.0,
            "the star reads ~5772 K, got {}",
            planet.star_effective_temperature_k.to_f64_lossy()
        );
        assert!(
            (planet.radius_m.to_f64_lossy() - 6.371e6).abs() < 1.0e5,
            "the planet radius ~6371 km, got {:.0} km",
            planet.radius_m.to_f64_lossy() / 1000.0
        );
        assert!(
            (planet.surface_gravity_m_s2.to_f64_lossy() - 9.8).abs() < 0.2,
            "the surface gravity ~9.8 m/s^2, got {}",
            planet.surface_gravity_m_s2.to_f64_lossy()
        );
        assert!(
            planet.disk_temperature_k.to_f64_lossy() > 150.0
                && planet.disk_temperature_k.to_f64_lossy() < 400.0,
            "the disk temperature at 1 AU is a plausible surface warmth, got {}",
            planet.disk_temperature_k.to_f64_lossy()
        );
    }
}
