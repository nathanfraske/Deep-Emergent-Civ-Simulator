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

//! The CPD RADIAL POINT: the composition consumer that ties the whole moon Branch A pipeline together at one
//! orbit. This is the integration rung, where the derive-first descent pays off end to end: the solved transport
//! state gives the three heat fluxes, the fluxes and surface density give the coupled midplane temperature, and
//! the temperature gives the local condensable assemblage, so reading a point across radii puts the ice line
//! where the physics places it rather than where a roster would.
//!
//! WHAT IT COMPOSES, each a module of its own (no grab bag; this is only the wire):
//!
//! 1. the viscous flux [`crate::cpd_thermal::viscous_heating_flux_from_transport`] from the solved transport state,
//! 2. the accretion flux [`crate::cpd_thermal::accretion_heating_flux_log10`],
//! 3. the planet-irradiation flux [`crate::cpd_thermal::planet_irradiation_flux_log10`] (weighted by the absorbed
//!    fraction `k_s`),
//! 4. the coupled midplane temperature [`crate::cpd_temperature::solve_cpd_midplane_temperature`] over those
//!    fluxes and the opacity fixed point,
//! 5. the local condensable assemblage [`crate::cpd_composition::local_condensable_assemblage`] at that temperature.
//!
//! THE OUTER COUPLING, named not hidden. The surface density `Sigma_g` depends on the temperature through the
//! viscosity (`nu = alpha c_s^2 / Omega`, `c_s` from `T`), and the planet-irradiation grazing angle and
//! photosurface altitude `z_s` depend on the vertical structure, which the temperature sets. A fully
//! self-consistent radial point therefore wraps THIS composition in an outer Picard loop over `Sigma_g` and `z_s`.
//! This module is one pass of that loop: it takes `Sigma_g`, `z_s`, and the grazing angle as inputs (the
//! outer-coupled quantities), so the inner temperature-opacity fixed point is solved self-consistently while the
//! outer disc-structure coupling is a named wrapping. The ice-line RADIUS validation (the front falling between
//! Europa and Ganymede) needs that outer loop across radii and is the named next rung.
//!
//! DORMANT: no run-path caller, so the two run pins hold bit-exact. Every input is an argument, so an alien giant
//! or disc chemistry is a data row.

use civsim_core::Fixed;

use crate::cpd_composition::{local_condensable_assemblage, LocalCondensableAssemblage};
use crate::cpd_temperature::{solve_cpd_midplane_temperature, CpdThermalBranch};
use crate::cpd_thermal::{
    accretion_heating_flux_log10, planet_irradiation_flux_log10,
    viscous_heating_flux_from_transport, CpdViscousHeatingFlux,
};
use crate::cpd_transport::CpdSteadyTransportState;
use crate::orbital_state::KeplerianOrbitState;

/// The disc-structure and chemistry inputs a radial point needs beyond the transport and orbital states: the
/// outer-coupled quantities (surface density, photosurface geometry) and the reserved-with-basis parameters. Held
/// as one struct so the orchestrator signature names the disc, not a long argument list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpdRadialInputs {
    /// The planet mass in solar masses (for the accretion flux and the orbital frequency).
    pub planet_mass_solar: Fixed,
    /// The centrifugal radius `R_c` in AU (the transport state's scale; `r = x * R_c`).
    pub centrifugal_radius_au: Fixed,
    /// The PSN metallicity `X_d` (dimensionless, reserved-with-basis), for the accretion flux.
    pub metallicity_xd: Fixed,
    /// The CPD dust enrichment `chi` (dimensionless, reserved-with-basis), for the accretion flux.
    pub dust_enrichment_ratio: Fixed,
    /// The giant's formation luminosity `log10(L_p / W)` (from `giants`), for the planet-irradiation flux.
    pub log10_lp_watts: Fixed,
    /// The disc's grazing incidence to the planet's light `sin(zeta + eta)`, in `(0, 1]` (outer-coupled input).
    pub grazing_angle_sin: Fixed,
    /// The photosurface altitude `z_s` in AU (outer-coupled input from the vertical structure).
    pub photosurface_altitude_au: Fixed,
    /// The absorbed fraction `k_s` of the planet's light, in `(0, 1]` (Makalkin-Dorofeeva `~0.2`, reserved).
    pub absorbed_fraction_ks: Fixed,
    /// The gas surface density `Sigma_g` in g/cm^2 (outer-coupled input; couples to `T` through the viscosity).
    pub surface_density_g_cm2: Fixed,
    /// The nebular temperature floor `T_neb` in K.
    pub nebular_temperature_k: Fixed,
    /// The water-ice line temperature in K (read from the condensation substrate).
    pub water_ice_line_k: Fixed,
    /// The condensed water-ice-to-rock mass ratio when ice is stable (reserved-with-basis).
    pub ice_to_rock_mass_ratio: Fixed,
    /// The Picard iteration bound for the temperature solve (a performance bound, reserved).
    pub max_iterations: u32,
    /// The midplane convergence tolerance in K (reserved).
    pub tolerance_k: Fixed,
}

/// The state of the CPD at one orbit: the midplane temperature, its regime branch, and the local condensable
/// assemblage the satellite-forming material has there. Produced by [`cpd_radial_point`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpdRadialPoint {
    /// The dimensionless radius `x = r / R_c`.
    pub radius_over_rc: Fixed,
    /// The solved midplane temperature in kelvin.
    pub midplane_k: Fixed,
    /// The optical-depth regime the solve converged in.
    pub thermal_branch: CpdThermalBranch,
    /// The local condensable assemblage (rock versus ice) at this temperature.
    pub assemblage: LocalCondensableAssemblage,
}

/// `10^y`, guarded: the linear value of a base-ten log, or `None` on overflow. Used to turn the log-domain fluxes
/// into the linear W/m^2 the temperature solve consumes.
fn pow10(y: Fixed) -> Option<Fixed> {
    Some(y.checked_mul(Fixed::from_int(10).ln())?.exp())
}

/// Compose the full pipeline at one orbit `x = r / R_c`: the three heat fluxes from the solved transport state and
/// the disc inputs, the coupled midplane temperature over them, and the local condensable assemblage at that
/// temperature. Returns the [`CpdRadialPoint`], or `None` if any stage fails (a degenerate input, a flux or
/// temperature that does not resolve, or a non-converging opacity), the refusal propagating rather than a partial
/// point being reported.
pub fn cpd_radial_point(
    transport: &CpdSteadyTransportState,
    orbit: &KeplerianOrbitState,
    x: Fixed,
    inputs: &CpdRadialInputs,
    planck_opacity_at: impl Fn(Fixed) -> Option<Fixed>,
    rosseland_opacity_at: impl Fn(Fixed) -> Option<Fixed>,
) -> Option<CpdRadialPoint> {
    // 1. The viscous flux from the solved transport state; a torque-free boundary contributes a physical zero.
    let f_vis = match viscous_heating_flux_from_transport(transport, orbit, x)? {
        CpdViscousHeatingFlux::TorqueFreeZero => Fixed::ZERO,
        CpdViscousHeatingFlux::Log10PerFaceWm2(v) => pow10(v)?,
    };
    // 2. The accretion flux (log domain -> linear).
    let f_acc = pow10(accretion_heating_flux_log10(
        x,
        transport.log10_mdot_supply_kg_s,
        inputs.planet_mass_solar,
        inputs.centrifugal_radius_au,
        inputs.metallicity_xd,
        inputs.dust_enrichment_ratio,
    )?)?;
    // 3. The planet-irradiation flux (log domain -> linear), weighted by the absorbed fraction k_s.
    let r_au = x.checked_mul(inputs.centrifugal_radius_au)?;
    let f_p_raw = pow10(planet_irradiation_flux_log10(
        inputs.log10_lp_watts,
        inputs.grazing_angle_sin,
        r_au,
        inputs.photosurface_altitude_au,
    )?)?;
    let f_p = f_p_raw.checked_mul(inputs.absorbed_fraction_ks)?;
    // 4. The coupled midplane temperature over the three fluxes and the opacity fixed point.
    let thermal = solve_cpd_midplane_temperature(
        f_vis,
        f_acc,
        f_p,
        inputs.surface_density_g_cm2,
        inputs.nebular_temperature_k,
        planck_opacity_at,
        rosseland_opacity_at,
        inputs.max_iterations,
        inputs.tolerance_k,
    )?;
    // 5. The local condensable assemblage at the midplane temperature.
    let assemblage = local_condensable_assemblage(
        thermal.midplane_k,
        inputs.water_ice_line_k,
        inputs.ice_to_rock_mass_ratio,
    )?;
    Some(CpdRadialPoint {
        radius_over_rc: x,
        midplane_k: thermal.midplane_k,
        thermal_branch: thermal.branch,
        assemblage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpd_composition::WaterIceState;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    fn transport() -> CpdSteadyTransportState {
        CpdSteadyTransportState::new(
            r(4_779, 10_000_000), // R_p ~ 1 R_J
            r(15, 1_000),         // R_c = 0.015 AU
            r(150, 1_000),        // r_d = 0.15 AU
            r(13, 1),             // log10(Mdot) = 13
        )
        .expect("well-posed CPD")
    }

    fn orbit_at(x: Fixed) -> KeplerianOrbitState {
        let radius_au = x.checked_mul(r(15, 1_000)).unwrap();
        KeplerianOrbitState::new(radius_au, r(954, 1_000_000)).expect("orbit resolves")
    }

    fn inputs() -> CpdRadialInputs {
        CpdRadialInputs {
            planet_mass_solar: r(954, 1_000_000),
            centrifugal_radius_au: r(15, 1_000),
            metallicity_xd: r(2, 100),
            dust_enrichment_ratio: r(5, 1),
            log10_lp_watts: r(25, 1),
            grazing_angle_sin: r(5, 100),
            photosurface_altitude_au: r(1, 1_000),
            absorbed_fraction_ks: r(2, 10),
            surface_density_g_cm2: r(50, 1),
            nebular_temperature_k: r(30, 1),
            water_ice_line_k: r(182, 1),
            ice_to_rock_mass_ratio: Fixed::from_int(1),
            max_iterations: 60,
            tolerance_k: r(1, 10),
        }
    }

    // THE END-TO-END DEMONSTRATION: with a moderate opacity, the full pipeline (transport -> fluxes -> temperature
    // -> composition) resolves at a mid-disc orbit and returns a coherent point.
    #[test]
    fn the_pipeline_resolves_a_radial_point() {
        let x = r(1, 2);
        let point = cpd_radial_point(
            &transport(),
            &orbit_at(x),
            x,
            &inputs(),
            |_t| Some(r(3, 10)),
            |_t| Some(r(3, 10)),
        )
        .expect("the full pipeline resolves");
        assert_eq!(point.radius_over_rc, x);
        assert!(point.midplane_k > Fixed::ZERO);
    }

    // THE EMERGENT GRADIENT through the FULL pipeline, with a single disc and a single opacity: the SAME inputs at
    // an inner and an outer orbit give a hot rock inner disc and a cold ice-bearing outer disc, with the ice line
    // falling in between at the radius where the physics puts the midplane through the water-ice temperature. The
    // rock-to-ice switch is derived, not rostered, and the outer point is cold because the fluxes fall off with
    // radius (the geometry factor toward the torque-free edge, the accretion Gaussian, the irradiation inverse
    // square), not because its inputs were tuned cold. This is the Galilean gradient the whole descent was for.
    #[test]
    fn the_ice_rock_gradient_emerges_from_the_pipeline() {
        let disc = transport();
        let opacity = |_t: Fixed| Some(r(3, 10));
        let inner_x = Fixed::from_int(1);
        let inner = cpd_radial_point(
            &disc,
            &orbit_at(inner_x),
            inner_x,
            &inputs(),
            opacity,
            opacity,
        )
        .expect("the inner point resolves");
        let outer_x = Fixed::from_int(8);
        let outer = cpd_radial_point(
            &disc,
            &orbit_at(outer_x),
            outer_x,
            &inputs(),
            opacity,
            opacity,
        )
        .expect("the outer point resolves");
        assert!(
            inner.midplane_k > outer.midplane_k,
            "the inner disc is hotter: inner {} vs outer {}",
            inner.midplane_k.to_f64_lossy(),
            outer.midplane_k.to_f64_lossy()
        );
        // The inner disc is above the water-ice line (rock), the outer below it (ice), the emergent switch.
        assert_eq!(inner.assemblage.water_ice, WaterIceState::Sublimated);
        assert_eq!(outer.assemblage.water_ice, WaterIceState::Stable);
        assert!(outer.assemblage.ice_mass_fraction > inner.assemblage.ice_mass_fraction);
    }

    #[test]
    fn a_failing_opacity_propagates_the_refusal() {
        let x = r(1, 2);
        let out = cpd_radial_point(
            &transport(),
            &orbit_at(x),
            x,
            &inputs(),
            |_t| None, // an opacity that never resolves
            |_t| Some(r(3, 10)),
        );
        assert!(
            out.is_none(),
            "a failing stage must propagate the refusal, not report a partial point"
        );
    }

    #[test]
    fn the_radial_point_is_deterministic() {
        let x = r(1, 2);
        let call = || {
            cpd_radial_point(
                &transport(),
                &orbit_at(x),
                x,
                &inputs(),
                |_t| Some(r(3, 10)),
                |_t| Some(r(3, 10)),
            )
        };
        assert_eq!(call(), call());
    }
}
