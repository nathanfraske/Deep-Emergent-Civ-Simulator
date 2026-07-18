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

//! THE EFFECTIVE CONVECTIVE VISCOSITY of an interior column, derived as a scalar fixed point over the landed
//! creep ladder, entirely IN LOG SPACE.
//!
//! # WHY A FIXED POINT, AND WHY IT IS NOT CIRCULAR
//!
//! The mantle's effective viscosity is not a material constant: it is the stress the rock carries divided by the
//! strain rate it deforms at, `eta = sigma / (2 * eps_dot)` (the deviatoric relation), and the rock's own creep
//! law ([`crate::creep_rows::ductile_strength_mpa`]) sets `sigma` for a given `eps_dot`. The convecting layer
//! imposes its OWN strain rate through the thermal boundary layer (below), which depends on the Rayleigh number,
//! which depends on the viscosity. So the rate needs `Ra`, `Ra` needs `eta`, and `eta` needs the rate. That
//! circle is not a wall; it is a SCALAR FIXED POINT, the same shape the moment-equivalence `T_e` solve already
//! runs ([`crate::moment_equivalence`]): a derived initial trial, iterate to convergence, a typed refusal when it
//! does not settle. THE STRAIN RATE IS ANSWERED RATHER THAN DESIGNED: it is the loop's own self-consistent value,
//! never a reference constant chosen at a call site.
//!
//! # THE STRAIN RATE IS THE BOUNDARY-LAYER DIFFUSIVE RATE
//!
//! Convective turnover is DIFFUSION-LIMITED: the cold boundary layer must thicken by conduction until it founders,
//! so the interior deforms at `eps_dot = kappa / delta^2`, where `delta = d * Ra^(-1/3)` is the marginally-critical
//! thermal boundary layer the engine already derives ([`crate::laws::thermal_boundary_layer`], the SAME `delta`
//! the lid geotherm and the driving stress span). This is the infinite-Prandtl boundary-layer scaling, and its
//! `Ra` exponent falls out of SQUARING a landed law, so it authors no new reservation.
//!
//! A RETIRED ALTERNATIVE, recorded because it was measured and it failed. An earlier form took the strain rate
//! from a Stokes TERMINAL velocity, `|v| = (2/9)|drho| g r^2 / eta` over a parcel length. Stokes terminal velocity
//! describes a rigid sphere free-falling through a PASSIVE fluid, decoupled from heat transport, so it runs away
//! to whatever speed the viscosity permits: at representative Mars-class inputs it gave `v ~ 0.44 m/s` (mantle flow
//! is centimetres per year) and `eta ~ 3e14 Pa*s`, about seven orders too low. Algebra confirmed that map was
//! self-consistent; only a MAGNITUDE spot-check at representative inputs revealed it did not describe convection.
//! The diffusive form gives `eta ~ 2e19 Pa*s` at the same inputs, where a hot asthenosphere-grade interior belongs.
//!
//! TERMS-DROPPED (the Prandtl conditioning). `eps_dot = kappa / delta^2` is the `Pr -> infinity` branch, valid for
//! every SOLID-STATE creeping layer, whose Prandtl number runs to twenty-plus orders. A low-Prandtl convecting
//! body (a liquid metal core, a deep magma ocean) turns over inertially and would dispatch to the `Ra^(1/2)`
//! free-fall branch instead; that branch is named here as the future dispatch, its convicting body declared, and
//! it is not built until such a body renders.
//!
//! # THE MAP IS A PROVEN CONTRACTION, OFF THE ADMITTED PHYSICS
//!
//! Write the map in `x = ln(eta)`. Then `ln(Ra) = ln|drho| + ln(g) + 3 ln(d) - x - ln(kappa)` (slope `-1` in `x`),
//! `ln(delta) = ln(d) - (1/3) ln(Ra)` (slope `+1/3`), and `ln(eps_dot) = ln(kappa) - 2 ln(delta)` (slope `-2/3`).
//! The creep ladder gives `sigma ~ eps_dot^(1/n)` for a stress exponent `n`, so
//! `ln(eta_new) = ln(sigma) - ln(2) - ln(eps_dot) = (1/n - 1) ln(eps_dot) + const`, and the map slope is
//! `(1/n - 1)(-2/3) = (2/3)(1 - 1/n)`. Every admissible creep mechanism has `n >= 1` (diffusion creep `n = 1`,
//! dislocation `n ~ 3.5`), so the slope lives in `[0, 2/3)` across the whole composite: diffusion converges in one
//! step, dislocation geometrically at about `0.48`. The contraction is guaranteed by the admitted set, not by luck
//! of the row, and [`solve_ln_effective_viscosity`] ASSERTS `n >= 1` on every admitted candidate so a future row
//! that would break the guarantee refuses rather than silently fails to settle.
//!
//! # LOG SPACE IS THE ONLY REPRESENTABLE FORM
//!
//! Mantle viscosities run `~1e19` to `~1e23 Pa*s` and geological strain rates `~1e-15 /s`; both overflow or
//! underflow Q32.32 outright. Their LOGARITHMS (`ln(eta) ~ 44` to `53`, `ln(eps_dot) ~ -35`) sit comfortably in
//! the representable window, so the whole loop is carried in logs and the raw viscosity never materializes. This
//! is the residency the creep module's own underflow guards already prove is necessary
//! ([`crate::creep_rows::CreepConditions::ln_strain_rate_per_s`]).
//!
//! # THE RESULT IS A CHORD OVER DEPTH, AND A V* BAND
//!
//! Viscosity varies by orders across the layer, so a single effective `eta` is a chord, and its evaluation state
//! is named rather than left implicit: the creep ladder is read at the interior POTENTIAL TEMPERATURE and the
//! MID-LAYER pressure (the standard convention), carried as [`ViscosityBand`]'s chord fields so the number can
//! never be misread as depth-independent. The band itself is the ACTIVATION-VOLUME bracket the deep `V*`
//! determinations were preserved to carry (the pressure-interval disagreement), the low and high ends of the
//! admitted rows' `V*`. There is no length-closure degree of freedom to band over: the diffusive rate fixes the
//! length to the derived boundary layer, so the best closure is no closure.

use civsim_core::Fixed;

use crate::creep_rows::{
    ductile_strength_mpa, CreepCandidate, CreepConditions, CreepRefusal, VolumeEnd,
};

/// The maximum number of fixed-point iterations before the solve reports non-convergence. A COMPUTATIONAL BOUND,
/// mirroring [`crate::moment_equivalence`]'s per-load solve: the map is iterated rather than bracketed, so it has
/// no derived step count, and hitting the cap is REPORTED (a [`ViscosityRefusal::DidNotConverge`]) rather than
/// papered over. The contraction assert makes the cap unreachable for an admitted set with every `n >= 1`; it is
/// a backstop, not a tuned budget. Sized well above the ~40 steps the worst admissible slope (`~0.48`) needs to
/// close from a derived seed to the representation floor.
const MAX_FIXED_POINT_ITERATIONS: u32 = 200;

/// The convergence tolerance on the `ln(eta)` step, RESIDUE-DERIVED rather than an authored precision. Q32.32
/// resolves `2^-32` absolutely, and the creep round-trip (`ln(exp(.))` per iteration) lifts the practical floor a
/// few bits; `2^-24` sits a few ulp above that noise at `ln(eta)` magnitudes near 48, so the iteration stops when
/// the step is at the arithmetic's own resolution (a relative viscosity closure of `~1e-7`), never at a chosen
/// physical epsilon.
const LN_VISCOSITY_CONVERGENCE_TOLERANCE: Fixed = Fixed::from_int(1).div(Fixed::from_int(1 << 24));

/// The convecting interior's own state at which the effective viscosity is solved. Every field is SI and a
/// derived per-column quantity; there is no fixture and no default.
#[derive(Clone, Copy, Debug)]
pub struct ViscosityInputs {
    /// The buoyancy density contrast magnitude `|drho|` (kg/m^3), the convecting interior against its cold
    /// reference (the absolute value of [`crate::laws::thermal_density_anomaly`]). Must be positive.
    pub density_anomaly_kg_m3: Fixed,
    /// Gravity (m/s^2). Must be positive.
    pub gravity_m_s2: Fixed,
    /// The convecting-layer depth `d` (m), the length the boundary-layer scaling keys on. Must be positive.
    pub layer_depth_m: Fixed,
    /// Thermal diffusivity `kappa` (m^2/s): the strain rate `kappa / delta^2` and the derived seed read it, and
    /// the boundary layer reads it through `Ra`. Must be positive.
    pub thermal_diffusivity_m2_s: Fixed,
    /// The temperature the creep ladder is evaluated at (K): the interior POTENTIAL TEMPERATURE, the chord field
    /// for the depth-averaged viscosity.
    pub eval_temperature_k: Fixed,
    /// The pressure the creep ladder is evaluated at (GPa): the MID-LAYER pressure, the chord field.
    pub eval_pressure_gpa: Fixed,
}

/// Why the effective-viscosity solve refused. Every variant is a refusal to answer, never a degraded number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViscosityRefusal {
    /// A buoyancy, gravity, depth, or diffusivity input was non-positive: no convective drive, and the log the
    /// whole solve lives in has no real value there.
    NonPositiveInput,
    /// A log-space intermediate left the representable window.
    NotRepresentable,
    /// The creep ladder refused (no admitted row at the pressure, a grade escalation, an unrepresentable stress).
    /// Propagated BY NAME rather than papered into a strength.
    Creep(CreepRefusal),
    /// An admitted mechanism had a stress exponent below one, so the log-map slope is not guaranteed inside
    /// `[0, 2/3)` and the fixed point is not a proven contraction. Every H&K row has `n >= 1`; this guards a
    /// future row that would not, refusing rather than iterating a map that might not settle.
    ContractionNotGuaranteed,
    /// The fixed point did not settle within the iteration cap. The contraction assert makes this unreachable for
    /// an admitted set with every `n >= 1`; it is reported, with the final `ln(eta)` step, rather than papered
    /// over.
    DidNotConverge {
        /// The last iteration's `|ln(eta_new) - ln(eta)|`, surfaced per the residual discipline.
        final_delta_ln: Fixed,
    },
}

/// The effective viscosity as a BAND: the honest interval `ln(eta)` spans as the `V*` activation-volume bracket
/// ranges over its ends. Carries the declared-primary value and the (T, P) chord.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViscosityBand {
    /// The least `ln(eta)` [ln Pa*s] over the `V*` bracket.
    pub ln_viscosity_min: Fixed,
    /// The greatest `ln(eta)` [ln Pa*s] over the `V*` bracket.
    pub ln_viscosity_max: Fixed,
    /// The DECLARED-PRIMARY `ln(eta)`: the low `V*` end (the weakest the row can be at a positive pressure). The
    /// value a single-figure consumer reads; `min`/`max` are its honest uncertainty, never a discarded alternative.
    pub ln_viscosity_primary: Fixed,
    /// CHORD: the temperature the ladder was evaluated at (K), the interior potential temperature.
    pub eval_temperature_k: Fixed,
    /// CHORD: the pressure the ladder was evaluated at (GPa), mid-layer.
    pub eval_pressure_gpa: Fixed,
}

/// The log-domain Rayleigh number for this column at a trial `ln(eta)`, delegating to the ONE HOME of the formula
/// ([`crate::laws::ln_rayleigh_number`]) rather than carrying a second, uncompared copy of it.
fn ln_rayleigh(inputs: &ViscosityInputs, ln_eta: Fixed) -> Result<Fixed, ViscosityRefusal> {
    crate::laws::ln_rayleigh_number(
        inputs.density_anomaly_kg_m3,
        inputs.gravity_m_s2,
        inputs.layer_depth_m,
        ln_eta,
        inputs.thermal_diffusivity_m2_s,
    )
    .ok_or(ViscosityRefusal::NotRepresentable)
}

/// `ln(delta) = ln(d) - (1/3) max(0, ln Ra)`, the log of the landed thermal boundary layer `delta = d Ra^(-1/3)`
/// capped at the layer depth (below onset, `Ra <= 1`, the whole layer is the length). The log form of
/// [`crate::laws::thermal_boundary_layer`], twinned against it in the tests.
fn ln_boundary_layer(ln_depth: Fixed, ln_rayleigh: Fixed) -> Result<Fixed, ViscosityRefusal> {
    let positive_ln_ra = if ln_rayleigh > Fixed::ZERO {
        ln_rayleigh
    } else {
        Fixed::ZERO
    };
    let drop = positive_ln_ra
        .checked_mul(Fixed::from_ratio(1, 3))
        .ok_or(ViscosityRefusal::NotRepresentable)?;
    ln_depth
        .checked_sub(drop)
        .ok_or(ViscosityRefusal::NotRepresentable)
}

/// One creep evaluation: the log stress (ln Pa) that sustains the log strain rate `ln_eps` at the chord state.
/// Wraps [`ductile_strength_mpa`] (MPa) and lifts it to log-space pascals. The `ln`/`exp` round-trip through the
/// MPa return is a sub-ulp cost the contraction absorbs, and it keeps this module non-invasive to the audited
/// creep row file.
fn ln_stress_pa(
    candidates: &[CreepCandidate<'_>],
    ln_eps: Fixed,
    inputs: &ViscosityInputs,
    v_star_end: VolumeEnd,
    ln_mpa_to_pa: Fixed,
) -> Result<Fixed, ViscosityRefusal> {
    let conditions = CreepConditions {
        ln_strain_rate_per_s: ln_eps,
        temperature_k: inputs.eval_temperature_k,
        pressure_gpa: inputs.eval_pressure_gpa,
        grain_size_um: None,
        water: None,
    };
    let sigma_mpa = ductile_strength_mpa(candidates, conditions, v_star_end)
        .map_err(ViscosityRefusal::Creep)?;
    if sigma_mpa <= Fixed::ZERO {
        return Err(ViscosityRefusal::NotRepresentable);
    }
    sigma_mpa
        .ln()
        .checked_add(ln_mpa_to_pa)
        .ok_or(ViscosityRefusal::NotRepresentable)
}

/// SOLVE the effective viscosity at ONE `V*` bracket end: the converged `ln(eta)` [ln Pa*s].
///
/// The scalar fixed point of the section-level map (`eta -> Ra -> delta -> eps_dot -> sigma -> eta`), seeded from
/// the DERIVED whole-layer diffusive strain rate `kappa / d^2` (a physically-grounded initial trial off the
/// column's own inputs) and iterated to the residue tolerance. The contraction assert (`n >= 1` on every admitted
/// candidate) makes convergence a property of the admitted physics. Refuses by name on a bad input, a creep-ladder
/// refusal, a broken contraction guarantee, or a walk that does not settle within the cap.
pub fn solve_ln_effective_viscosity(
    inputs: &ViscosityInputs,
    candidates: &[CreepCandidate<'_>],
    v_star_end: VolumeEnd,
) -> Result<Fixed, ViscosityRefusal> {
    if inputs.density_anomaly_kg_m3 <= Fixed::ZERO
        || inputs.gravity_m_s2 <= Fixed::ZERO
        || inputs.layer_depth_m <= Fixed::ZERO
        || inputs.thermal_diffusivity_m2_s <= Fixed::ZERO
    {
        return Err(ViscosityRefusal::NonPositiveInput);
    }

    // THE CONTRACTION ASSERT, off the admitted physics: every candidate's stress exponent must be at least one,
    // or the log-map slope is not guaranteed inside [0, 2/3) and the fixed point is not a proven contraction.
    for candidate in candidates {
        if candidate.row.stress_exponent < Fixed::ONE {
            return Err(ViscosityRefusal::ContractionNotGuaranteed);
        }
    }

    let ln_depth = inputs.layer_depth_m.ln();
    let ln_kappa = inputs.thermal_diffusivity_m2_s.ln();
    let ln_two = Fixed::from_int(2).ln();
    // ln(1e6), the MPa-to-Pa decade, without forming the unrepresentable 1e6 as a stress.
    let ln_mpa_to_pa = Fixed::from_int(10)
        .ln()
        .checked_mul(Fixed::from_int(6))
        .ok_or(ViscosityRefusal::NotRepresentable)?;

    // THE DERIVED SEED: the whole-layer diffusive strain rate `eps_dot = kappa / d^2` (the boundary layer at the
    // sub-onset floor `delta = d`), a physically-meaningful rate off the column's own inputs, gives a first stress
    // and hence a first `ln(eta)`. The contraction makes the converged value independent of the seed; a grounded
    // seed keeps the early iterations inside the creep ladder's representable stress window.
    let ln_eps_seed = ln_kappa
        .checked_sub(ln_depth)
        .and_then(|x| x.checked_sub(ln_depth))
        .ok_or(ViscosityRefusal::NotRepresentable)?;
    let ln_sigma_seed = ln_stress_pa(candidates, ln_eps_seed, inputs, v_star_end, ln_mpa_to_pa)?;
    let mut ln_eta = ln_sigma_seed
        .checked_sub(ln_two)
        .and_then(|x| x.checked_sub(ln_eps_seed))
        .ok_or(ViscosityRefusal::NotRepresentable)?;

    // ITERATE the boundary-layer map to convergence. Each step: the Rayleigh number at the current eta, the
    // thermal boundary layer it implies, the diffusive strain rate across that layer, the creep stress at that
    // rate, and the deviatoric viscosity it implies.
    let mut final_delta = Fixed::MAX;
    for _ in 0..MAX_FIXED_POINT_ITERATIONS {
        let ln_ra = ln_rayleigh(inputs, ln_eta)?;
        let ln_bl = ln_boundary_layer(ln_depth, ln_ra)?;
        // ln(eps_dot) = ln(kappa) - 2 ln(delta).
        let ln_eps = ln_kappa
            .checked_sub(ln_bl)
            .and_then(|x| x.checked_sub(ln_bl))
            .ok_or(ViscosityRefusal::NotRepresentable)?;
        let ln_sigma = ln_stress_pa(candidates, ln_eps, inputs, v_star_end, ln_mpa_to_pa)?;
        // eta = sigma / (2 eps_dot): ln(eta_new) = ln(sigma) - ln(2) - ln(eps_dot).
        let ln_eta_new = ln_sigma
            .checked_sub(ln_two)
            .and_then(|x| x.checked_sub(ln_eps))
            .ok_or(ViscosityRefusal::NotRepresentable)?;
        let delta = (ln_eta_new - ln_eta).abs();
        ln_eta = ln_eta_new;
        final_delta = delta;
        if delta <= LN_VISCOSITY_CONVERGENCE_TOLERANCE {
            return Ok(ln_eta);
        }
    }
    Err(ViscosityRefusal::DidNotConverge {
        final_delta_ln: final_delta,
    })
}

/// The effective viscosity as a BAND over the `V*` activation-volume bracket ends, with the declared-primary value
/// (the low end) and the (T, P) chord. The band is the pressure-interval disagreement the deep activation-volume
/// determinations were preserved to carry. Refuses if either end refuses; the band's honesty is that both ends are
/// real.
pub fn effective_viscosity_band(
    inputs: &ViscosityInputs,
    candidates: &[CreepCandidate<'_>],
) -> Result<ViscosityBand, ViscosityRefusal> {
    let low = solve_ln_effective_viscosity(inputs, candidates, VolumeEnd::Low)?;
    let high = solve_ln_effective_viscosity(inputs, candidates, VolumeEnd::High)?;
    Ok(ViscosityBand {
        ln_viscosity_min: low.min(high),
        ln_viscosity_max: low.max(high),
        ln_viscosity_primary: low,
        eval_temperature_k: inputs.eval_temperature_k,
        eval_pressure_gpa: inputs.eval_pressure_gpa,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creep_rows::{hk_dry_dislocation, hk_dry_dislocation_activation_volumes};
    use crate::laws::thermal_boundary_layer;

    // The dry-dislocation mantle candidate, the mechanism that admits on the production path (p = 0, no grain
    // size, dry): a single CreepCandidate the tests share. Its stress exponent is ~3.5, so the map slope is
    // ~0.48, a geometric contraction.
    fn dry_dislocation_volumes() -> [crate::creep_rows::ActivationVolume; 8] {
        hk_dry_dislocation_activation_volumes()
    }

    fn candidates_slice(
        volumes: &[crate::creep_rows::ActivationVolume],
    ) -> [CreepCandidate<'_>; 1] {
        [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes,
        }]
    }

    // A Mars-class convecting interior: a real buoyancy contrast, Mars gravity, an ~1.8 Mm mantle depth in SI
    // metres, a mantle diffusivity, an interior potential temperature, and a mid-layer pressure. The point of the
    // slice: these are all SI and physical, not the retired scaled fixtures.
    fn mars_inputs() -> ViscosityInputs {
        ViscosityInputs {
            density_anomaly_kg_m3: Fixed::from_int(50),
            gravity_m_s2: Fixed::from_ratio(37, 10),
            layer_depth_m: Fixed::from_int(1_800_000),
            thermal_diffusivity_m2_s: Fixed::from_ratio(1, 1_000_000),
            eval_temperature_k: Fixed::from_int(1600),
            eval_pressure_gpa: Fixed::from_int(10),
        }
    }

    #[test]
    fn the_effective_viscosity_lands_in_the_solid_state_creep_bracket() {
        // THE MAGNITUDE REFEREE, at CATASTROPHE WIDTH. This is a MECHANISM-CLASS bracket, not an Earth-mantle
        // window: `eta in [1e15, 1e26] Pa*s` (ln in [34.5, 59.9]) spans everything a solid-state creeping silicate
        // can be, from a near-molten asthenosphere (~1e15) to a cold thick lithosphere (~1e26), decades wider than
        // Earth's own ~1e19..1e22 subset. It is wide enough that no honest alien world's physics can trip it and
        // narrow enough that a seven-order mechanism failure is caught cold: the retired Stokes closure landed at
        // ~3e14 (below 1e15) and would fail this. Asserting Earth's mantle range here would be calibration wearing
        // a test's clothes (the banned Haisch-Lada move); an Earth comparison, if ever wanted, routes through the
        // hindcast machinery as a Mirror row with bands, never through CI.
        let volumes = dry_dislocation_volumes();
        let candidates = candidates_slice(&volumes);
        let ln_eta = solve_ln_effective_viscosity(&mars_inputs(), &candidates, VolumeEnd::Low)
            .expect("resolves");
        let ln = ln_eta.to_f64_lossy();
        assert!(
            (34.5..=59.9).contains(&ln),
            "ln(eta) = {ln} outside the solid-state creep bracket [1e15, 1e26]; a mechanism failure"
        );
    }

    #[test]
    fn the_boundary_layer_log_form_twins_the_landed_law() {
        // TWIN: the log delta this module iterates on must be the log of the LANDED `thermal_boundary_layer`. At a
        // representable Ra both forms exist, and their agreement proves the strain rate rides the derived boundary
        // layer rather than a re-derivation of it.
        let inputs = mars_inputs();
        let ln_depth = inputs.layer_depth_m.ln();
        let ra = Fixed::from_int(1_000_000);
        let ln_delta = ln_boundary_layer(ln_depth, ra.ln()).expect("delta");
        let delta_landed = thermal_boundary_layer(inputs.layer_depth_m, ra);
        let ratio = ln_delta.exp().to_f64_lossy() / delta_landed.to_f64_lossy();
        assert!(
            (0.999..=1.001).contains(&ratio),
            "the log boundary layer must reproduce the landed thermal_boundary_layer, ratio {ratio}"
        );
    }

    #[test]
    fn the_converged_fixed_point_is_self_consistent_by_an_independent_route() {
        // TWIN: recompute the map's right-hand side from the converged eta by a route that does NOT run the
        // iteration, and assert it returns eta. eta is a fixed point iff eta = sigma(eps_dot(eta)) / (2 eps_dot).
        let volumes = dry_dislocation_volumes();
        let candidates = candidates_slice(&volumes);
        let inputs = mars_inputs();
        let ln_eta =
            solve_ln_effective_viscosity(&inputs, &candidates, VolumeEnd::Low).expect("resolves");

        let ln_depth = inputs.layer_depth_m.ln();
        let ln_kappa = inputs.thermal_diffusivity_m2_s.ln();
        let ln_ra = ln_rayleigh(&inputs, ln_eta).expect("ra");
        let ln_bl = ln_boundary_layer(ln_depth, ln_ra).expect("bl");
        let ln_eps = ln_kappa - ln_bl - ln_bl;
        let ln_mpa_to_pa = Fixed::from_int(10).ln() * Fixed::from_int(6);
        let conditions = CreepConditions {
            ln_strain_rate_per_s: ln_eps,
            temperature_k: inputs.eval_temperature_k,
            pressure_gpa: inputs.eval_pressure_gpa,
            grain_size_um: None,
            water: None,
        };
        let sigma_mpa =
            ductile_strength_mpa(&candidates, conditions, VolumeEnd::Low).expect("stress");
        let ln_eta_rhs = sigma_mpa.ln() + ln_mpa_to_pa - Fixed::from_int(2).ln() - ln_eps;
        assert!(
            (ln_eta_rhs - ln_eta).abs() <= LN_VISCOSITY_CONVERGENCE_TOLERANCE * Fixed::from_int(4),
            "self-consistency residual {} exceeds the solve tolerance",
            (ln_eta_rhs - ln_eta).abs().to_f64_lossy()
        );
    }

    #[test]
    fn the_solve_is_deterministic() {
        // Principle 3: the same inputs give the same bits, and the converged value is a property of the fixed
        // point, not of the iteration.
        let volumes = dry_dislocation_volumes();
        let candidates = candidates_slice(&volumes);
        let a = solve_ln_effective_viscosity(&mars_inputs(), &candidates, VolumeEnd::Low);
        let b = solve_ln_effective_viscosity(&mars_inputs(), &candidates, VolumeEnd::Low);
        assert_eq!(a, b, "the deterministic solve must reproduce to the bit");
    }

    #[test]
    fn a_hotter_interior_is_less_viscous() {
        // MUTATION guard and a physics check: creep is faster at higher temperature, so a given strain rate needs
        // less stress, so the effective viscosity FALLS. A test that could not distinguish hotter from colder
        // would not be testing the coupling; this asserts the sign.
        let volumes = dry_dislocation_volumes();
        let candidates = candidates_slice(&volumes);
        let cool = mars_inputs();
        let mut hot = mars_inputs();
        hot.eval_temperature_k = Fixed::from_int(1900);
        let ln_cool = solve_ln_effective_viscosity(&cool, &candidates, VolumeEnd::Low)
            .expect("cool resolves");
        let ln_hot =
            solve_ln_effective_viscosity(&hot, &candidates, VolumeEnd::Low).expect("hot resolves");
        assert!(
            ln_hot < ln_cool,
            "a hotter interior must be less viscous: ln_hot {} !< ln_cool {}",
            ln_hot.to_f64_lossy(),
            ln_cool.to_f64_lossy()
        );
    }

    #[test]
    fn a_non_positive_input_refuses_by_name() {
        let volumes = dry_dislocation_volumes();
        let candidates = candidates_slice(&volumes);
        let mut bad = mars_inputs();
        bad.layer_depth_m = Fixed::ZERO;
        assert_eq!(
            solve_ln_effective_viscosity(&bad, &candidates, VolumeEnd::Low),
            Err(ViscosityRefusal::NonPositiveInput)
        );
    }

    #[test]
    fn a_sub_unity_stress_exponent_refuses_the_contraction() {
        // The contraction guarantee is n >= 1. A synthetic candidate with n < 1 must refuse rather than iterate a
        // map whose slope could leave [0, 2/3).
        let volumes = dry_dislocation_volumes();
        let mut row = hk_dry_dislocation();
        row.stress_exponent = Fixed::from_ratio(1, 2);
        let candidates = [CreepCandidate {
            row,
            volumes: &volumes,
        }];
        assert_eq!(
            solve_ln_effective_viscosity(&mars_inputs(), &candidates, VolumeEnd::Low),
            Err(ViscosityRefusal::ContractionNotGuaranteed)
        );
    }

    #[test]
    fn the_band_brackets_the_primary_and_carries_the_chord() {
        let volumes = dry_dislocation_volumes();
        let candidates = candidates_slice(&volumes);
        let inputs = mars_inputs();
        let band = effective_viscosity_band(&inputs, &candidates).expect("the band resolves");
        assert!(
            band.ln_viscosity_min <= band.ln_viscosity_primary
                && band.ln_viscosity_primary <= band.ln_viscosity_max,
            "the band must bracket its declared primary"
        );
        assert!(
            band.ln_viscosity_min < band.ln_viscosity_max,
            "a positive-pressure column has a non-degenerate V* band"
        );
        assert_eq!(band.eval_temperature_k, inputs.eval_temperature_k);
        assert_eq!(band.eval_pressure_gpa, inputs.eval_pressure_gpa);
    }
}
