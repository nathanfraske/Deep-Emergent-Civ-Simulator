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

//! The #213 HANDOFF: a candidate satellite that co-accreted from the circumplanetary disk, carried through the
//! tidal-survival filter and the tidal-heating rate of [`crate::moons`] to a retained, tidally-heated moon. This
//! is where moon Branch A closes: the mass budget, the orbit, and the emergent composition become an observable
//! body with a survival verdict and an interior heat.
//!
//! THE FULL CIRCLE, reusing the arc's own pieces (no grab bag; each concern stays its module):
//!
//! - the orbit and the mean motion come from the shared log-Kepler state [`crate::orbital_state`] (step 2), so the
//!   tidal heating's Keplerian premise is constructed once, not re-passed by a caller,
//! - the moon RADIUS is DERIVED from the mass and the emergent ice-versus-rock composition (a bulk density from
//!   the ice mass fraction, then a sphere), so an ice-rich moon is larger and dissipates more tidal heat, an
//!   emergent coupling rather than an authored radius,
//! - the survival verdict is the existing [`crate::moons::tidal_survival`] filter and the heat the existing
//!   [`crate::moons::tidal_heating_power_log10`], both consumed unchanged.
//!
//! WHAT IS AND IS NOT HERE. This evaluates ONE candidate satellite. How many candidates a disc forms and at what
//! masses (the satellitesimal accretion population that partitions the regulated surviving mass of
//! [`crate::moons_cpd`]) is a derive-first population model, a named rung not built here: authoring a moon count
//! would violate emergence. This handoff is the per-candidate close that the population feeds.
//!
//! DERIVE-FIRST and ADMITS THE ALIEN: every input is an argument (the candidate, the planet, the reserved tidal
//! and density parameters), so an alien moon of any composition is a data row. Determinism (Principle 3):
//! fixed-point throughout, a degenerate input failing soft. DORMANT: no run-path caller, so the two run pins hold.

use civsim_core::Fixed;

use crate::moons::{tidal_heating_power_log10, tidal_survival, MoonSurvival};
use crate::orbital_state::KeplerianOrbitState;

/// A candidate satellite that co-accreted from the circumplanetary disk: the arc's output for one body, before the
/// tidal filter decides whether it is retained. The mass comes from the regulated surviving budget partitioned by
/// the population model, the orbit from where it accreted, and the ice mass fraction from the local condensable
/// assemblage ([`crate::cpd_composition`]) at that orbit's midplane temperature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateSatellite {
    /// The orbital radius around the planet, in AU.
    pub orbit_au: Fixed,
    /// The satellite mass in Earth masses.
    pub mass_earth: Fixed,
    /// The water-ice mass fraction of the satellite (from the emergent composition), in `[0, 1)`.
    pub ice_mass_fraction: Fixed,
}

/// The outcome of carrying a candidate satellite through the tidal filter and heating rate: whether it is
/// retained, and the interior tidal heat if so. Produced by [`evaluate_candidate_satellite`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetainedMoonOutcome {
    /// The tidal-survival verdict (retained, or one of the failure modes).
    pub survival: MoonSurvival,
    /// `log10(E_dot / W)`, the tidal heat dissipated inside the moon, present regardless of the survival verdict
    /// (a disrupted or stripped moon still had a heating rate while it existed); `None` if the heating inputs are
    /// degenerate.
    pub tidal_heating_log10_w: Option<Fixed>,
    /// The moon radius as `log10(R / m)`, derived from the mass and composition, carried for the consumer.
    pub radius_log10_m: Fixed,
}

/// The BULK DENSITY (g/cm^3) of a satellite from its water-ice mass fraction, by volume additivity of the rock and
/// ice components: `1/rho = f_ice/rho_ice + (1 - f_ice)/rho_rock`. An ice-richer moon is less dense. `rho_rock`
/// and `rho_ice` are caller inputs reserved-with-basis (their basis the measured densities of rock, about 3.5, and
/// water ice, about 1.0 g/cm^3). `None` on a non-positive density or a fraction outside `[0, 1)`.
pub fn bulk_density_from_composition(
    ice_mass_fraction: Fixed,
    rock_density_g_cm3: Fixed,
    ice_density_g_cm3: Fixed,
) -> Option<Fixed> {
    if rock_density_g_cm3 <= Fixed::ZERO
        || ice_density_g_cm3 <= Fixed::ZERO
        || ice_mass_fraction < Fixed::ZERO
        || ice_mass_fraction >= Fixed::from_int(1)
    {
        return None;
    }
    let rock_fraction = Fixed::from_int(1).checked_sub(ice_mass_fraction)?;
    let inv = ice_mass_fraction
        .checked_div(ice_density_g_cm3)?
        .checked_add(rock_fraction.checked_div(rock_density_g_cm3)?)?;
    Fixed::from_int(1).checked_div(inv)
}

/// The moon RADIUS as `log10(R / m)` from its mass (Earth masses) and bulk density (g/cm^3), a uniform sphere
/// `R = (3 M / (4 pi rho))^(1/3)`, formed in the log domain (the mass in kg is wide). `None` on a non-positive
/// input.
fn radius_log10_m(mass_earth: Fixed, bulk_density_g_cm3: Fixed) -> Option<Fixed> {
    if mass_earth <= Fixed::ZERO || bulk_density_g_cm3 <= Fixed::ZERO {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let log10 = |v: Fixed| -> Option<Fixed> {
        if v <= Fixed::ZERO {
            return None;
        }
        v.ln().checked_div(ln10)
    };
    let log10_decimal =
        |s: &str| -> Option<Fixed> { civsim_physics::saha::ln_of_decimal(s)?.checked_div(ln10) };
    // log10(M / kg) = log10(M / M_earth) + log10(M_earth / kg).
    let log10_m_kg = log10(mass_earth)?.checked_add(log10_decimal(crate::astro::EARTH_MASS_KG)?)?;
    // rho in kg/m^3 = rho[g/cm^3] * 1000, so log10(rho_si) = log10(rho_cgs) + 3.
    let log10_rho_si = log10(bulk_density_g_cm3)?.checked_add(Fixed::from_int(3))?;
    let log10_3 = log10(Fixed::from_int(3))?;
    let log10_4pi = log10(Fixed::from_int(4).checked_mul(Fixed::PI)?)?;
    // log10(R) = (1/3)(log10(3) + log10(M) - log10(4 pi) - log10(rho)).
    log10_3
        .checked_add(log10_m_kg)?
        .checked_sub(log10_4pi)?
        .checked_sub(log10_rho_si)?
        .checked_div(Fixed::from_int(3))
}

/// Carry a candidate satellite through the tidal-survival filter and the tidal-heating rate. The orbit and mean
/// motion are derived from the shared log-Kepler state, the moon radius from the mass and composition; the tidal
/// bounds and the moon's tidal parameters are caller inputs reserved-with-basis.
///
/// `candidate` is the co-accreted body; `planet_mass_solar` the host mass; `k2`, `q_factor`, `eccentricity` the
/// moon's tidal parameters; `rock_density_g_cm3`, `ice_density_g_cm3` the component densities; the survival bounds
/// (`roche_limit_au`, `stable_axis_au`, `corotation_radius_au`, `recession_rate_au_per_myr`, `system_age_myr`) the
/// planet's pre-computed tidal geometry in a consistent AU and megayear unit. `None` on any degenerate input or a
/// stage that fails to resolve.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_candidate_satellite(
    candidate: &CandidateSatellite,
    planet_mass_solar: Fixed,
    k2: Fixed,
    q_factor: Fixed,
    eccentricity: Fixed,
    rock_density_g_cm3: Fixed,
    ice_density_g_cm3: Fixed,
    roche_limit_au: Fixed,
    stable_axis_au: Fixed,
    corotation_radius_au: Fixed,
    recession_rate_au_per_myr: Fixed,
    system_age_myr: Fixed,
) -> Option<RetainedMoonOutcome> {
    if planet_mass_solar <= Fixed::ZERO {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let log10 = |v: Fixed| -> Option<Fixed> {
        if v <= Fixed::ZERO {
            return None;
        }
        v.ln().checked_div(ln10)
    };
    let log10_decimal =
        |s: &str| -> Option<Fixed> { civsim_physics::saha::ln_of_decimal(s)?.checked_div(ln10) };
    // The orbit and mean motion from the shared log-Kepler state (step 2), so the tidal-heating Keplerian premise
    // is constructed once, not re-passed by a caller.
    let orbit = KeplerianOrbitState::new(candidate.orbit_au, planet_mass_solar)?;
    let log10_a_m = log10(candidate.orbit_au)?
        .checked_add(log10_decimal(crate::astro::ASTRONOMICAL_UNIT_M)?)?;
    let log10_mean_motion = orbit.log10_omega_s_inv;
    let log10_m_planet_kg =
        log10(planet_mass_solar)?.checked_add(log10_decimal(crate::astro::SOLAR_MASS_KG)?)?;
    // The moon radius from the mass and the emergent composition.
    let density = bulk_density_from_composition(
        candidate.ice_mass_fraction,
        rock_density_g_cm3,
        ice_density_g_cm3,
    )?;
    let log10_r_moon_m = radius_log10_m(candidate.mass_earth, density)?;
    // The survival verdict (existing #213 filter) and the tidal heat (existing #213 kernel).
    let survival = tidal_survival(
        candidate.orbit_au,
        roche_limit_au,
        stable_axis_au,
        corotation_radius_au,
        recession_rate_au_per_myr,
        system_age_myr,
    )?;
    let tidal_heating_log10_w = tidal_heating_power_log10(
        k2,
        q_factor,
        eccentricity,
        log10_m_planet_kg,
        log10_r_moon_m,
        log10_a_m,
        log10_mean_motion,
    );
    Some(RetainedMoonOutcome {
        survival,
        tidal_heating_log10_w,
        radius_log10_m: log10_r_moon_m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    // An ice-rich moon is less dense than a rock moon, and so larger at fixed mass: the composition-to-radius
    // coupling that makes an ice moon dissipate more tidal heat.
    #[test]
    fn an_ice_rich_moon_is_less_dense_and_larger() {
        let rock =
            bulk_density_from_composition(Fixed::ZERO, r(35, 10), Fixed::from_int(1)).unwrap();
        let icy = bulk_density_from_composition(r(1, 2), r(35, 10), Fixed::from_int(1)).unwrap();
        assert!(
            icy < rock,
            "ice-rich is less dense: {} vs {}",
            icy.to_f64_lossy(),
            rock.to_f64_lossy()
        );
        // Ganymede-mass (0.025 M_earth): the icy body has the larger radius.
        let r_rock = radius_log10_m(r(25, 1000), rock).unwrap();
        let r_icy = radius_log10_m(r(25, 1000), icy).unwrap();
        assert!(r_icy > r_rock);
    }

    // The bulk density matches a hand computation: at f_ice = 0.5 with rock 3.5 and ice 1.0,
    // 1/rho = 0.5/1 + 0.5/3.5 = 0.6429, rho = 1.556 g/cm^3, a Ganymede-like value.
    #[test]
    fn the_bulk_density_matches_a_hand_computation() {
        let rho = bulk_density_from_composition(r(1, 2), r(35, 10), Fixed::from_int(1)).unwrap();
        assert!((rho.to_f64_lossy() - 1.5556).abs() < 1e-2);
    }

    // A retained candidate: an orbit inside the stable band and outside the Roche limit, receding slowly over the
    // age, is Retained and carries a finite tidal heat derived from its mass and composition.
    #[test]
    fn a_stable_candidate_is_retained_and_heated() {
        let candidate = CandidateSatellite {
            orbit_au: r(715, 100_000), // ~15 R_J, Ganymede-ish
            mass_earth: r(25, 1000),
            ice_mass_fraction: r(45, 100),
        };
        let outcome = evaluate_candidate_satellite(
            &candidate,
            r(954, 1_000_000),     // Jupiter mass in solar
            r(3, 100),             // k2
            r(100, 1),             // Q
            r(1, 100),             // eccentricity
            r(35, 10),             // rock density
            Fixed::from_int(1),    // ice density
            r(120, 100_000),       // Roche limit AU (~2.5 R_J)
            r(2000, 100_000),      // stable axis AU (well outside)
            r(100, 100_000),       // corotation radius AU (inside the orbit -> recession)
            r(1, 1_000_000),       // slow recession, AU/Myr
            Fixed::from_int(4500), // 4.5 Gyr in Myr
        )
        .expect("the candidate resolves");
        assert_eq!(outcome.survival, MoonSurvival::Retained);
        assert!(outcome.tidal_heating_log10_w.is_some());
    }

    #[test]
    fn degenerate_inputs_fail_soft() {
        assert!(
            bulk_density_from_composition(Fixed::from_int(1), r(35, 10), Fixed::from_int(1))
                .is_none()
        );
        assert!(radius_log10_m(Fixed::ZERO, r(2, 1)).is_none());
        let c = CandidateSatellite {
            orbit_au: r(1, 100),
            mass_earth: r(1, 100),
            ice_mass_fraction: r(1, 2),
        };
        assert!(evaluate_candidate_satellite(
            &c,
            Fixed::ZERO,
            r(3, 100),
            r(100, 1),
            r(1, 100),
            r(35, 10),
            Fixed::from_int(1),
            r(1, 1000),
            r(1, 10),
            r(1, 100),
            r(1, 1_000_000),
            Fixed::from_int(4500)
        )
        .is_none());
    }
}
