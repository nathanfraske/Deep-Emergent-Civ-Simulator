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

//! The SATELLITESIMAL-ACCRETION POPULATION: how the regulated surviving satellite mass partitions into a set of
//! candidate satellites at orbits, with the NUMBER and the relative masses EMERGING from the disc rather than
//! authored. This is the rung between the mass budget ([`crate::moons_cpd`]) and the per-candidate #213 handoff
//! ([`crate::cpd_satellites`]): it decides how many moons form and where, so no moon count is ever written down.
//!
//! HOW THE NUMBER EMERGES. In oligarchic growth (Lissauer 1987; Kokubo and Ida 1998), a growing satellite clears a
//! feeding zone of width `b` mutual Hill radii and reaches its ISOLATION MASS, the solid mass in that zone:
//!
//! ```text
//! M_iso = (2 pi b a^2 Sigma_solid)^(3/2) / (3 M_planet)^(1/2),   R_H = a (M_iso / (3 M_planet))^(1/3)
//! ```
//!
//! Tiling the disc from its inner edge outward in steps of `b R_H` places one feeding zone after another, and the
//! NUMBER of zones falls out of where the tiling reaches the outer edge: a denser or wider disc packs more zones,
//! a thinner one fewer, and nothing is authored. The isolation mass of each zone is its natural weight.
//!
//! HOW THE MASSES EMERGE, and which surface density feeds them. The tiling is done with the SURVIVING solid
//! surface density (the regulated Canup-Ward budget spread over the disc), not the far larger instantaneous solid
//! density that is processed and lost: in the gas-starved disc most solids migrate into the planet, and only the
//! surviving scale forms lasting moons (the reframe in [`crate::moons_cpd`]). With the surviving density the zone
//! isolation mass IS the final oligarch mass, and summing it over the tiling recovers the budget by construction;
//! the masses are then renormalized to the budget exactly, absorbing the discretization. Because the ice line
//! raises the surviving solid surface density beyond it, the outer zones carry more mass and grow the heavier
//! moons, the Ganymede-and-Callisto-heavier ordering emerging rather than being placed.
//!
//! WHAT EMERGES AND WHAT IS A LAYER ON TOP. The number this reports is the OLIGARCH population, the moons the disc
//! tiles into at their feeding-zone mass. The dynamical CONSOLIDATION of oligarchs into a final few by mutual
//! stirring and mergers (the step from many oligarchs to the four Galilean moons) is a named layer above this,
//! not built here; this module supplies the pre-consolidation population the merger stage would act on.
//!
//! DERIVE-FIRST and ADMITS THE ALIEN: the number and masses are functions of the solid-density profile, the disc
//! extent, the host mass, and the surviving budget, all arguments; `b` is a reserved-with-basis caller input (its
//! basis the oligarchic feeding-zone width, about 10 Hill radii). An alien disc is a different profile, a data
//! row. Determinism (Principle 3): the tiling is a deterministic march, a degenerate input failing soft to `None`.
//! DORMANT: no run-path caller, so the two run pins hold bit-exact.

use civsim_core::Fixed;

/// A candidate satellite the population produces: where it accreted and the mass it grew to (its share of the
/// regulated surviving budget). Fed to [`crate::cpd_satellites::CandidateSatellite`] once its composition is read
/// from the local midplane temperature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SatelliteSeed {
    /// The orbital radius around the planet where the feeding zone sits, in AU.
    pub orbit_au: Fixed,
    /// The satellite mass in Earth masses (its share of the surviving budget).
    pub mass_earth: Fixed,
}

/// The oligarchic ISOLATION MASS in Earth masses at one orbit: `M_iso = (2 pi b a^2 Sigma)^(3/2) / (3 M_p)^(1/2)`,
/// the solid mass a satellite clears from a feeding zone `b` Hill radii wide. Computed in the log domain (the SI
/// intermediates are wide).
///
/// `orbit_au` the radius in AU; `solid_surface_density_g_cm2` the solid (condensed) surface density in g/cm^2;
/// `planet_mass_solar` the host mass in solar masses; `feeding_zone_hill_widths` the width `b` in mutual Hill
/// radii (reserved-with-basis, about 10). `None` on a non-positive input or an overflow.
pub fn satellite_isolation_mass_earth(
    orbit_au: Fixed,
    solid_surface_density_g_cm2: Fixed,
    planet_mass_solar: Fixed,
    feeding_zone_hill_widths: Fixed,
) -> Option<Fixed> {
    if orbit_au <= Fixed::ZERO
        || solid_surface_density_g_cm2 <= Fixed::ZERO
        || planet_mass_solar <= Fixed::ZERO
        || feeding_zone_hill_widths <= Fixed::ZERO
    {
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
    let log10_au = log10_decimal(crate::astro::ASTRONOMICAL_UNIT_M)?;
    let log10_m_sun = log10_decimal(crate::astro::SOLAR_MASS_KG)?;
    let log10_m_earth = log10_decimal(crate::astro::EARTH_MASS_KG)?;
    // a in metres; Sigma in kg/m^2 = g/cm^2 * 10, so log10(Sigma_si) = log10(Sigma_cgs) + 1.
    let log10_a_m = log10(orbit_au)?.checked_add(log10_au)?;
    let log10_sigma_si = log10(solid_surface_density_g_cm2)?.checked_add(Fixed::from_int(1))?;
    let log10_2pi_b = log10(
        Fixed::from_int(2)
            .checked_mul(Fixed::PI)?
            .checked_mul(feeding_zone_hill_widths)?,
    )?;
    // log10(2 pi b a^2 Sigma) = log10(2 pi b) + 2 log10(a) + log10(Sigma).
    let log10_zone = log10_2pi_b
        .checked_add(Fixed::from_int(2).checked_mul(log10_a_m)?)?
        .checked_add(log10_sigma_si)?;
    // log10(3 M_p) = log10(3) + log10(M_p).
    let log10_3mp = log10(Fixed::from_int(3))?
        .checked_add(planet_mass_solar.ln().checked_div(ln10)?)?
        .checked_add(log10_m_sun)?;
    // log10(M_iso_kg) = (3/2) log10(zone) - (1/2) log10(3 M_p).
    let log10_m_iso_kg = Fixed::from_ratio(3, 2)
        .checked_mul(log10_zone)?
        .checked_sub(Fixed::from_ratio(1, 2).checked_mul(log10_3mp)?)?;
    // Convert kg -> Earth masses and exponentiate.
    let log10_m_iso_earth = log10_m_iso_kg.checked_sub(log10_m_earth)?;
    Some(log10_m_iso_earth.checked_mul(ln10)?.exp())
}

/// Grow the emergent satellite population: tile the disc from `inner_au` to `outer_au` into oligarchic feeding
/// zones, then partition the `surviving_budget_earth` among them by isolation weight. Returns the
/// [`SatelliteSeed`]s (orbit and final mass), or `None` on a degenerate input.
///
/// The number of seeds emerges from the tiling; `max_satellites` is a safety bound on the march (a runaway tiling
/// from a vanishing Hill step is capped and the result is `None`, never a silent truncation).
/// `surviving_solid_density_at` gives the SURVIVING solid surface density in g/cm^2 at a radius (the budget's
/// spatial distribution, which the ice line raises beyond the front). The seeds' masses sum to the budget within
/// fixed-point tolerance.
pub fn oligarchic_satellite_population(
    inner_au: Fixed,
    outer_au: Fixed,
    planet_mass_solar: Fixed,
    feeding_zone_hill_widths: Fixed,
    surviving_budget_earth: Fixed,
    surviving_solid_density_at: impl Fn(Fixed) -> Option<Fixed>,
    max_satellites: usize,
) -> Option<Vec<SatelliteSeed>> {
    if inner_au <= Fixed::ZERO
        || outer_au <= inner_au
        || planet_mass_solar <= Fixed::ZERO
        || feeding_zone_hill_widths <= Fixed::ZERO
        || surviving_budget_earth <= Fixed::ZERO
        || max_satellites == 0
    {
        return None;
    }
    let earth_to_sun = crate::astro::earth_to_sun_mass_ratio()?;
    let three_mp = Fixed::from_int(3).checked_mul(planet_mass_solar)?;
    // March through feeding zones, collecting (orbit, isolation weight).
    let mut zones: Vec<(Fixed, Fixed)> = Vec::new();
    let mut a = inner_au;
    while a < outer_au {
        if zones.len() >= max_satellites {
            // The tiling did not close within the safety bound: refuse rather than silently truncate.
            return None;
        }
        let sigma = surviving_solid_density_at(a)?;
        let m_iso =
            satellite_isolation_mass_earth(a, sigma, planet_mass_solar, feeding_zone_hill_widths)?;
        zones.push((a, m_iso));
        // R_H = a (M_iso / (3 M_p))^(1/3), with the mass ratio taken to solar units through EARTH_TO_SUN.
        // Step a += b R_H. The cube root is exp(ln(x)/3).
        let mass_ratio_solar = m_iso.checked_mul(earth_to_sun)?.checked_div(three_mp)?;
        if mass_ratio_solar <= Fixed::ZERO {
            return None;
        }
        let cube_root = mass_ratio_solar.ln().checked_div(Fixed::from_int(3))?.exp();
        let hill_au = a.checked_mul(cube_root)?;
        let step = feeding_zone_hill_widths.checked_mul(hill_au)?;
        if step <= Fixed::ZERO {
            // A vanishing Hill step would loop forever; refuse.
            return None;
        }
        a = a.checked_add(step)?;
    }
    if zones.is_empty() {
        return None;
    }
    // Partition the surviving budget by isolation weight: mass_i = (M_iso_i / sum) * budget.
    let mut total = Fixed::ZERO;
    for (_, w) in &zones {
        total = total.checked_add(*w)?;
    }
    if total <= Fixed::ZERO {
        return None;
    }
    let mut seeds = Vec::with_capacity(zones.len());
    for (orbit_au, weight) in zones {
        let share = weight.checked_div(total)?;
        let mass_earth = share.checked_mul(surviving_budget_earth)?;
        seeds.push(SatelliteSeed {
            orbit_au,
            mass_earth,
        });
    }
    Some(seeds)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    // The isolation mass matches an independent f64 reference from the formula.
    #[test]
    fn the_isolation_mass_matches_a_reference() {
        let (a_au, sigma, mp, b) = (0.01_f64, 100.0_f64, 9.54e-4_f64, 10.0_f64);
        let au = 1.495_978_707e11_f64;
        let m_sun = 1.989e30_f64;
        let m_earth = 5.9722e24_f64;
        let a_m = a_au * au;
        let sigma_si = sigma * 10.0;
        let zone = 2.0 * std::f64::consts::PI * b * a_m * a_m * sigma_si;
        let m_iso_kg = zone.powf(1.5) / (3.0 * mp * m_sun).powf(0.5);
        let m_iso_earth_ref = m_iso_kg / m_earth;
        let got = satellite_isolation_mass_earth(r(1, 100), r(100, 1), r(954, 1_000_000), r(10, 1))
            .expect("the isolation mass resolves");
        // Compare in log10 (the value is tiny; a relative check).
        assert!(
            (got.to_f64_lossy().log10() - m_iso_earth_ref.log10()).abs() < 1e-2,
            "M_iso {} vs reference {}",
            got.to_f64_lossy(),
            m_iso_earth_ref
        );
    }

    // The population number EMERGES from the tiling (a handful of oligarchs, the pre-consolidation Galilean-scale
    // set), the total mass equals the budget exactly (conserved partition), and the masses land at the observed
    // scale (a few 0.001 to 0.03 M_earth). With a uniform surviving density the a^2 weight makes the outer moons
    // heavier: the mass ordering emerges from the disc geometry.
    #[test]
    fn the_population_number_and_masses_emerge() {
        let budget = r(658, 10000); // 0.0658 M_earth, the Galilean total
        let seeds = oligarchic_satellite_population(
            r(5, 1000),  // inner 0.005 AU (~1 R_J)
            r(13, 1000), // outer 0.013 AU (~2.7 R_J), the Galilean span
            r(954, 1_000_000),
            r(10, 1),
            budget,
            |_a| Some(r(2000, 1)), // uniform surviving solid density, g/cm^2
            64,
        )
        .expect("the population resolves");
        assert!(
            seeds.len() > 1,
            "a handful of oligarchs emerge from the tiling"
        );
        // The masses sum to the budget within tolerance.
        let mut total = Fixed::ZERO;
        for s in &seeds {
            total = total.checked_add(s.mass_earth).unwrap();
        }
        assert!(
            (total.to_f64_lossy() - budget.to_f64_lossy()).abs() < 1e-4,
            "the seed masses sum to the budget: {} vs {}",
            total.to_f64_lossy(),
            budget.to_f64_lossy()
        );
        // The masses land at the observed Galilean scale (each between 0.001 and 0.04 Earth masses).
        for s in &seeds {
            let m = s.mass_earth.to_f64_lossy();
            assert!(
                m > 1e-3 && m < 0.04,
                "a satellite mass at the Galilean scale: {m}"
            );
        }
        // With a uniform surviving density the isolation weight rises with a^2, so the outermost seed outweighs the
        // innermost: the mass ordering emerges from the disc geometry.
        assert!(seeds.last().unwrap().mass_earth > seeds.first().unwrap().mass_earth);
    }

    // The ice line's boost to the surviving solid surface density AMPLIFIES the outer moons: the outer-to-inner
    // mass ratio is larger with the ice-line enhancement than under a flat density, the Ganymede-and-Callisto
    // heavier ordering emerging from the composition rather than placed.
    #[test]
    fn the_ice_line_makes_outer_moons_heavier() {
        let budget = r(658, 10000);
        let ice_line_au = r(9, 1000);
        // A flat surviving density, and one that jumps 3x beyond the ice line (rock plus condensed ice).
        let flat = |_a: Fixed| -> Option<Fixed> { Some(r(2000, 1)) };
        let boosted = move |a: Fixed| -> Option<Fixed> {
            if a < ice_line_au {
                Some(r(1000, 1))
            } else {
                Some(r(3000, 1))
            }
        };
        let outer_inner_ratio = |seeds: &[SatelliteSeed]| -> f64 {
            let (mut inner_sum, mut inner_n, mut outer_sum, mut outer_n) = (0.0, 0.0, 0.0, 0.0);
            for s in seeds {
                if s.orbit_au < ice_line_au {
                    inner_sum += s.mass_earth.to_f64_lossy();
                    inner_n += 1.0;
                } else {
                    outer_sum += s.mass_earth.to_f64_lossy();
                    outer_n += 1.0;
                }
            }
            assert!(
                inner_n > 0.0 && outer_n > 0.0,
                "the disc straddles the ice line"
            );
            (outer_sum / outer_n) / (inner_sum / inner_n)
        };
        let flat_seeds = oligarchic_satellite_population(
            r(5, 1000),
            r(14, 1000),
            r(954, 1_000_000),
            r(10, 1),
            budget,
            flat,
            64,
        )
        .expect("the flat population resolves");
        let seeds = oligarchic_satellite_population(
            r(5, 1000),
            r(14, 1000),
            r(954, 1_000_000),
            r(10, 1),
            budget,
            boosted,
            64,
        )
        .expect("the population resolves");
        assert!(
            outer_inner_ratio(&seeds) > outer_inner_ratio(&flat_seeds),
            "the ice-line density boost amplifies the outer moons beyond the flat a^2 effect"
        );
        // The boosted outer moons are also heavier than the boosted inner ones in absolute terms.
        assert!(outer_inner_ratio(&seeds) > 1.0);
    }

    #[test]
    fn a_runaway_tiling_refuses_rather_than_truncating() {
        // A tiny max bound against a disc that would tile into many zones: refuse, do not silently truncate.
        let out = oligarchic_satellite_population(
            r(3, 1000),
            r(300, 1000),
            r(954, 1_000_000),
            r(10, 1),
            r(658, 10000),
            |_a| Some(r(100, 1)),
            2,
        );
        assert!(out.is_none(), "a tiling that exceeds the bound refuses");
    }

    #[test]
    fn degenerate_inputs_fail_soft() {
        assert!(
            satellite_isolation_mass_earth(Fixed::ZERO, r(100, 1), r(1, 1000), r(10, 1)).is_none()
        );
        assert!(oligarchic_satellite_population(
            r(30, 1000),
            r(3, 1000), // outer < inner
            r(954, 1_000_000),
            r(10, 1),
            r(658, 10000),
            |_a| Some(r(100, 1)),
            64
        )
        .is_none());
    }

    #[test]
    fn the_population_is_deterministic() {
        let call = || {
            oligarchic_satellite_population(
                r(3, 1000),
                r(30, 1000),
                r(954, 1_000_000),
                r(10, 1),
                r(658, 10000),
                |_a| Some(r(100, 1)),
                64,
            )
        };
        assert_eq!(call(), call());
    }
}
