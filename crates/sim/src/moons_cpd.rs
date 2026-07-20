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

//! BRANCH A of the moon dispatch: CIRCUMPLANETARY-DISK (CPD) CO-ACCRETION, the regular satellites (the moon arc,
//! DORMANT). A gas or ice giant that accretes enough gas can host a circumplanetary disk, and its regular
//! satellites co-accrete from that disk (the Galilean and Saturnian systems are the type cases). This module holds
//! two derive-first primitives: the CIRCUMPLANETARY FLOW VERDICT (whether a rotationally-supported disk forms at
//! all) and, when it does, the REGULATED SURVIVING satellite-system mass.
//!
//! THE TWO CORRECTIONS THIS MODULE ENCODES (from the blocking review's P0-D):
//!
//! First, not every forming giant hosts a rotationally-supported CPD. Whether the infalling gas settles into a
//! disk or piles up as a pressure-supported envelope is DERIVED from the centrifugal radius against the planet
//! radius: if the gas's angular momentum circularizes it OUTSIDE the planet (`R_c > R_p`) a disk forms and regular
//! satellites co-accrete; if inside (`R_c <= R_p`) the gas cannot orbit and forms an envelope, and Branch A does
//! not apply even though the giant is physically valid. The verdict is [`circumplanetary_flow_verdict`], keyed on
//! the same `R_p`-over-`R_c` geometry the transport solve ([`crate::cpd_transport`]) rests on, not a boolean.
//!
//! Second, the Canup and Ward (2006) `M_sat / M_planet ~ 1e-4` scaling is NOT an initial mass budget handed to the
//! system to allocate once. It is a REGULATED SURVIVING mass: the attractor a continuing balance of satellitesimal
//! inflow and Type-I loss into the planet converges to, so far more than `1e-4` of the planet mass is processed
//! through the disk over its life, and only about `1e-4` survives at any time. Treating it as a starting budget
//! would erase the very migration-and-loss mechanism that sets the scaling. So [`evaluate_satellite_system_mass`]
//! carries it as a [`SatelliteMassMode::RegulatedSurvivingAttractor`], an accelerated-attractor summary standing
//! in for the unresolved inflow-accretion-migration-loss balance, NOT a resolved population output. When that
//! balance is later resolved (solids accretion and Type-I loss run), the surviving mass becomes an OUTPUT and the
//! Canup-Ward value is a validation TARGET, the [`SatelliteMassMode::ResolvedPopulationOutput`] member.
//!
//! DERIVE-FIRST (Principle 8) and ADMITS THE ALIEN: the flow verdict and the surviving mass are functions of the
//! planet's own derived mass and radii, on the argument list, so a super-Jupiter, an ice giant, or an alien giant
//! of any composition is a data row through the same criterion, never a new code path. Determinism (Principle 3):
//! fixed-point throughout; a degenerate input fails soft (`Unresolved` or `None`), never a fabricated verdict or
//! mass. DORMANT: no run-path caller (the dispatch wiring into `planetary_assembly` is a gated follow-on), so the
//! two run pins hold bit-exact.
//!
//! The value-authoring line (Principle 6): no number is authored. The mass-ratio band is the CITED Canup and Ward
//! (2006) satellite-system mass scaling (`docs/working/PIPELINE_FETCHES.md` section 6, Nature 441, 834, DOI
//! 10.1038/nature04860), a caller input reserved-with-basis at the call site, exactly as `k2`/`Q` are for the
//! tidal kernels. Its basis and its honest limit: the paper's headline supports an ORDER-`1e-4` regulated scale,
//! not automatically the exact `[1e-4, 4e-4]` band; the observed regular systems bracket it (the Galilean total
//! is about `2.07e-4` of Jupiter, the Saturnian regulars about `2.46e-4` of Saturn), and those two systems are
//! ECHOES the scaling was built to explain ([`ValidationRole::Echo`]), not independent anchors. Neither Canup-Ward
//! paper has a free full-text artifact to byte-hold (2002 AJ has no arXiv, 2006 Nature is paywalled), so the value
//! is held as a cited reserved input rather than a registry witness, the same handling as the tidal `k2`/`Q`.

use civsim_core::Fixed;

/// The character of the circumplanetary gas flow around a forming giant, DERIVED from whether the infalling gas is
/// rotationally supported (a disk) or pressure supported (an envelope). Only a rotationally-supported disk
/// co-accretes regular satellites the Canup-Ward way; a giant can be physically valid while Branch A is
/// inapplicable to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CircumplanetaryFlowVerdict {
    /// The centrifugal radius exceeds the planet radius (`R_c > R_p`), so the infalling gas circularizes outside
    /// the planet and settles into a rotationally-supported circumplanetary disk. Regular satellites co-accrete.
    RotationallySupportedDisk,
    /// The centrifugal radius is within the planet radius (`R_c <= R_p`), so the gas cannot orbit and piles up as a
    /// pressure-supported envelope. No regular satellite system forms this way, though the giant is valid.
    PressureSupportedEnvelope,
    /// The verdict cannot be reached because the geometry inputs are missing or degenerate (a non-positive planet
    /// radius or centrifugal radius). The "refusal names the gap" pattern, not a silent default to either branch.
    Unresolved,
}

/// Derive the circumplanetary flow verdict from the planet radius and the centrifugal radius of the infalling gas
/// (both in AU, both the assembly's derived values). The criterion is geometric and first-order: whether the
/// gas's angular momentum circularizes it outside the planet. A cooling- and opacity-dependent refinement (a
/// marginally-supported disk that a slow-cooling envelope can still suppress) is a named deeper rung, not built
/// here; this verdict is the leading criterion. A non-positive radius returns [`CircumplanetaryFlowVerdict::Unresolved`].
pub fn circumplanetary_flow_verdict(
    planet_radius_au: Fixed,
    centrifugal_radius_au: Fixed,
) -> CircumplanetaryFlowVerdict {
    if planet_radius_au <= Fixed::ZERO || centrifugal_radius_au <= Fixed::ZERO {
        return CircumplanetaryFlowVerdict::Unresolved;
    }
    if centrifugal_radius_au > planet_radius_au {
        CircumplanetaryFlowVerdict::RotationallySupportedDisk
    } else {
        CircumplanetaryFlowVerdict::PressureSupportedEnvelope
    }
}

/// How the satellite-system mass value should be read: the interpretation frame that keeps an accelerated
/// attractor from being mistaken for a resolved output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SatelliteMassMode {
    /// The Canup-Ward regulated surviving-mass attractor, a shortcut standing in for the unresolved inflow,
    /// accretion, migration, and Type-I loss. It is the mass that survives at a time, NOT the total processed and
    /// NOT a one-time budget; it is carried with its ratio band so a consumer knows its basin.
    RegulatedSurvivingAttractor,
    /// The surviving mass resolved as an OUTPUT of a modelled inflow-accretion-migration-loss balance (a future
    /// member, once solids accretion and Type-I loss run). Then the Canup-Ward value is a validation target.
    ResolvedPopulationOutput,
}

/// Whether an observed system used to check the scaling is an ECHO (a system the model was built to explain, so a
/// reproduction check) or an independent HINDCAST. The regular-satellite mass scaling was constructed to explain
/// the Jovian, Saturnian, and Uranian regular systems, so those are echoes, not independent anchors: the
/// validation-constitution distinction the review calls for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationRole {
    /// A system the model was built to explain; agreement is reproduction, not independent confirmation.
    Echo,
    /// A system outside the model's construction set; agreement is a genuine out-of-sample test.
    Hindcast,
}

/// The regulated surviving satellite-system mass a rotationally-supported CPD host carries, banded, in Earth
/// masses, with the frame that says how to read it. Produced by [`evaluate_satellite_system_mass`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SatelliteSystemMassEvaluation {
    /// The lower edge of the regulated surviving satellite-system mass, in Earth masses.
    pub surviving_mass_lo_earth: Fixed,
    /// The upper edge of the regulated surviving satellite-system mass, in Earth masses.
    pub surviving_mass_hi_earth: Fixed,
    /// How to read the value (the attractor shortcut, or a resolved output once the balance runs).
    pub mode: SatelliteMassMode,
    /// The flow verdict the value rests on; a value is produced only for a rotationally-supported disk.
    pub flow: CircumplanetaryFlowVerdict,
    /// The Canup-Ward mass-ratio band, carried so a consumer sees the value's basis: the headline supports an
    /// order-`1e-4` regulated scale, and this band is where the observed echoes fall.
    pub ratio_lo: Fixed,
    pub ratio_hi: Fixed,
}

/// Evaluate the regulated surviving satellite-system mass for a giant whose circumplanetary flow verdict is known.
/// A value is produced ONLY for a [`CircumplanetaryFlowVerdict::RotationallySupportedDisk`]: a pressure-supported
/// envelope grows no regular satellites this way, and an unresolved verdict cannot, so both return `None`. The
/// mass is `[ratio_lo, ratio_hi] * M_planet` in the [`SatelliteMassMode::RegulatedSurvivingAttractor`] frame, the
/// endpoint of the inflow-loss balance rather than a starting budget.
///
/// `m_planet_earth` is the planet mass in Earth masses. `flow` is the derived verdict from
/// [`circumplanetary_flow_verdict`]. `ratio_lo` and `ratio_hi` are the Canup-Ward band, reserved-with-basis at the
/// call site. `None` on a non-host flow verdict, a non-positive mass, a non-positive `ratio_lo`, an inverted band,
/// or an overflow: fail-soft, never a fabricated mass.
pub fn evaluate_satellite_system_mass(
    m_planet_earth: Fixed,
    flow: CircumplanetaryFlowVerdict,
    ratio_lo: Fixed,
    ratio_hi: Fixed,
) -> Option<SatelliteSystemMassEvaluation> {
    if flow != CircumplanetaryFlowVerdict::RotationallySupportedDisk
        || m_planet_earth <= Fixed::ZERO
        || ratio_lo <= Fixed::ZERO
        || ratio_hi < ratio_lo
    {
        return None;
    }
    let lo = m_planet_earth.checked_mul(ratio_lo)?;
    let hi = m_planet_earth.checked_mul(ratio_hi)?;
    Some(SatelliteSystemMassEvaluation {
        surviving_mass_lo_earth: lo,
        surviving_mass_hi_earth: hi,
        mode: SatelliteMassMode::RegulatedSurvivingAttractor,
        flow,
        ratio_lo,
        ratio_hi,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    // A disk forms when the centrifugal radius exceeds the planet radius; an envelope when it does not; and a
    // degenerate geometry is Unresolved, not silently defaulted. The derived verdict replaces the old boolean.
    #[test]
    fn the_flow_verdict_is_derived_from_the_centrifugal_radius() {
        // R_c (0.02 AU) > R_p (~1 R_J = 4.78e-4 AU): a rotationally-supported disk.
        assert_eq!(
            circumplanetary_flow_verdict(r(4_779, 10_000_000), r(2, 100)),
            CircumplanetaryFlowVerdict::RotationallySupportedDisk
        );
        // R_c (2e-4 AU) < R_p (~1 R_J): the gas circularizes inside the planet, a pressure-supported envelope.
        assert_eq!(
            circumplanetary_flow_verdict(r(4_779, 10_000_000), r(2, 10_000)),
            CircumplanetaryFlowVerdict::PressureSupportedEnvelope
        );
        // Degenerate geometry: unresolved, not defaulted.
        assert_eq!(
            circumplanetary_flow_verdict(Fixed::ZERO, r(2, 100)),
            CircumplanetaryFlowVerdict::Unresolved
        );
    }

    // ECHO CHECK, not an independent anchor: the Canup-Ward band on Jupiter's mass brackets the OBSERVED Galilean
    // satellite-system mass, but the scaling was BUILT to explain the Galilean system, so this is reproduction
    // (ValidationRole::Echo), not independent confirmation. The mass is a regulated surviving attractor.
    #[test]
    fn the_galilean_system_is_an_echo_reproduction() {
        let role = ValidationRole::Echo;
        assert_eq!(role, ValidationRole::Echo);
        let jupiter_earth = r(31782, 100); // 317.82 M_earth
        let eval = evaluate_satellite_system_mass(
            jupiter_earth,
            CircumplanetaryFlowVerdict::RotationallySupportedDisk,
            r(1, 10000),
            r(4, 10000),
        )
        .expect("a rotationally-supported disk carries a surviving mass");
        assert_eq!(eval.mode, SatelliteMassMode::RegulatedSurvivingAttractor);
        let galilean_observed = r(658, 10000); // 0.0658 M_earth
        assert!(
            eval.surviving_mass_lo_earth <= galilean_observed
                && galilean_observed <= eval.surviving_mass_hi_earth,
            "observed Galilean mass {} sits in the regulated band [{}, {}]",
            galilean_observed.to_f64_lossy(),
            eval.surviving_mass_lo_earth.to_f64_lossy(),
            eval.surviving_mass_hi_earth.to_f64_lossy()
        );
    }

    // A second echo: the Saturnian regulars on Saturn's mass, a different host, the same scaling.
    #[test]
    fn the_saturnian_regulars_are_an_echo_reproduction() {
        let role = ValidationRole::Echo;
        assert_eq!(role, ValidationRole::Echo);
        let saturn_earth = r(9516, 100); // 95.16 M_earth
        let eval = evaluate_satellite_system_mass(
            saturn_earth,
            CircumplanetaryFlowVerdict::RotationallySupportedDisk,
            r(1, 10000),
            r(4, 10000),
        )
        .expect("a rotationally-supported disk carries a surviving mass");
        let saturnian_observed = r(234, 10000); // 0.0234 M_earth
        assert!(
            eval.surviving_mass_lo_earth <= saturnian_observed
                && saturnian_observed <= eval.surviving_mass_hi_earth
        );
    }

    // The surviving mass scales with the host mass at fixed ratio: the Canup-Ward "grows with the host" signature.
    #[test]
    fn the_surviving_mass_scales_with_the_host() {
        let ratio = r(2, 10000);
        let jup = evaluate_satellite_system_mass(
            r(31782, 100),
            CircumplanetaryFlowVerdict::RotationallySupportedDisk,
            ratio,
            ratio,
        )
        .unwrap();
        let sat = evaluate_satellite_system_mass(
            r(9516, 100),
            CircumplanetaryFlowVerdict::RotationallySupportedDisk,
            ratio,
            ratio,
        )
        .unwrap();
        assert!(jup.surviving_mass_lo_earth > sat.surviving_mass_lo_earth);
        let budget_ratio =
            jup.surviving_mass_lo_earth.to_f64_lossy() / sat.surviving_mass_lo_earth.to_f64_lossy();
        assert!((budget_ratio - 317.82 / 95.16).abs() < 1e-2);
    }

    // A pressure-supported envelope grows no regular satellites this way: no value, not a fabricated one. The
    // derive-first gate now keys on the derived flow verdict, not a boolean host flag.
    #[test]
    fn an_envelope_host_grows_no_regular_satellites() {
        assert!(evaluate_satellite_system_mass(
            r(31782, 100),
            CircumplanetaryFlowVerdict::PressureSupportedEnvelope,
            r(1, 10000),
            r(4, 10000)
        )
        .is_none());
        // An unresolved verdict likewise yields no value.
        assert!(evaluate_satellite_system_mass(
            r(31782, 100),
            CircumplanetaryFlowVerdict::Unresolved,
            r(1, 10000),
            r(4, 10000)
        )
        .is_none());
    }

    // Determinism (Principle 3) and fail-soft: identical inputs give the identical evaluation, and a non-positive
    // mass, a non-positive ratio, or an inverted band each return `None`.
    #[test]
    fn the_evaluation_is_deterministic_and_fails_soft() {
        let disk = CircumplanetaryFlowVerdict::RotationallySupportedDisk;
        assert_eq!(
            evaluate_satellite_system_mass(r(31782, 100), disk, r(1, 10000), r(4, 10000)),
            evaluate_satellite_system_mass(r(31782, 100), disk, r(1, 10000), r(4, 10000))
        );
        assert!(
            evaluate_satellite_system_mass(Fixed::ZERO, disk, r(1, 10000), r(4, 10000)).is_none()
        );
        assert!(
            evaluate_satellite_system_mass(r(31782, 100), disk, Fixed::ZERO, r(4, 10000)).is_none()
        );
        // Inverted band (hi < lo) is rejected.
        assert!(
            evaluate_satellite_system_mass(r(31782, 100), disk, r(4, 10000), r(1, 10000)).is_none()
        );
    }
}
