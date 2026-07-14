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

//! Stage 6, the band-gap tier (`docs/working/STAGE6_ELECTRONIC_STRUCTURE_DESIGN.md`, section 10): slice 1, the
//! log-space thermal carrier activation, the fabrication-free census-discharge piece the tier's design surface
//! ruled buildable now (independent of the Harrison-rung fork held for the gate).
//!
//! A semiconductor's intrinsic carrier density is `n_i = N_eff * exp(-E_gap / 2kT)`, thermally activated across
//! the gap. The activation factor `exp(-E_gap / 2kT)` is the RANGE-CENSUS flag of the electronic sub-arc: for a
//! wide-gap insulator (diamond's `5.47 eV`) at world temperature the factor is `exp(-106) ~ 1e-46`, far below the
//! Q32.32 floor (`~2.3e-10`), so the bare factor underflows to zero and loses all ordering. So the quantity is
//! carried in LOG SPACE, on the same discipline the creep deformation-mechanism rates use: this slice returns the
//! natural-log activation exponent `-E_gap / 2kT` (non-positive, always representable), and a consumer exponentiates
//! only when the value is in range, comparing insulators' activations by their logs otherwise.
//!
//! WHAT IS RESERVED HERE: nothing. The gap `E_gap` is caller-supplied (a measured `[M]` datum at the top rung, a
//! Harrison estimate at the middle rung once the gate rules that fork, a compute-once eigenvalue at the bottom),
//! and the temperature is the world's. The one constant, the Boltzmann constant in the working units (eV per
//! kelvin), is not a folded dimensional decimal: it reassembles as `k_B[J/K] / e[C]`, a ratio of two exact SI
//! fundamental constants (the dimensionless-constant law), so the eV and the kelvin cancel and the activation
//! exponent is dimensionless by construction. This slice authors no metal/semiconductor/insulator classification:
//! that rides the `U/W` preflight over the banked correlation classifier (section 10.2), the next slice, and is
//! never shipped in the preflight-free form redirect 2 warned reintroduces the Mott failure.
//!
//! Byte-neutral: `civsim-materials` is a leaf, not linked into the run_world binary.

use civsim_core::Fixed;

const ZERO: Fixed = Fixed::ZERO;

/// The thermal-activation fold `2 * k_B` in eV/K (`~1.7234667e-4`), mapping `E_gap[eV] / (fold * T[K])` to the
/// dimensionless activation exponent. ASSEMBLED from the exact SI mantissas of the Boltzmann constant and the
/// elementary charge (the dimensionless-constant law, no folded dimensional decimal): `2 * k_B[eV/K] = 2 *
/// k_B[J/K] / e[C] = (2 * 1.380649 / 1.602176634) * 1e-4`, since `k_B` carries `10^-23` and `e` carries `10^-19`,
/// netting `10^-4`. The eV-per-kelvin convention IS `k_B / e`, so the fold reassembles from two fundamental
/// constants and the eV and the kelvin cancel: the activation exponent `E_gap / (fold * T)` is dimensionless.
fn two_kb_ev_per_k() -> Fixed {
    // 2 * k_B[J/K] mantissa (2 * 1.380649) and e[C] mantissa (1.602176634); the collapsed 10^-4 rides as the
    // /10000 below (k_B's 10^-23 over e's 10^-19).
    let two_kb_mantissa = Fixed::from_ratio(2 * 1_380_649, 1_000_000);
    let e_mantissa = Fixed::from_ratio(1_602_176_634, 1_000_000_000);
    let ratio = match two_kb_mantissa.checked_div(e_mantissa) {
        Some(v) => v,
        None => return ZERO,
    };
    ratio.checked_div(Fixed::from_int(10_000)).unwrap_or(ZERO)
}

/// The natural log of the thermal carrier activation factor, `ln(exp(-E_gap / 2kT)) = -E_gap / (2 * k_B * T)`, the
/// LOG-SPACE form of the semiconductor intrinsic-carrier suppression (the range-census discipline: the bare factor
/// underflows Q32.32 for a wide gap). The returned exponent is non-positive and always representable; a consumer
/// exponentiates it only when the value is in the `exp` window, and orders insulators by the log otherwise.
///
/// Reserves no value: `E_gap` (eV) and the temperature (K) are caller-supplied, and the `2 * k_B` fold reassembles
/// from `k_B` and `e`. `None` (escalate) for a negative gap (a band overlap is a metal, classified upstream, and
/// its carriers are not thermally activated across a gap) or a non-positive temperature (no thermal population).
pub fn ln_thermal_carrier_activation(e_gap_ev: Fixed, temperature_k: Fixed) -> Option<Fixed> {
    if e_gap_ev < ZERO || temperature_k <= ZERO {
        return None;
    }
    // 2 * k_B * T in eV (the thermal energy scale the gap is measured against).
    let thermal_scale = two_kb_ev_per_k().checked_mul(temperature_k)?;
    if thermal_scale <= ZERO {
        return None;
    }
    // -E_gap / (2 k_B T): the log-space activation exponent, non-positive, representable even where exp underflows.
    let ratio = e_gap_ev.checked_div(thermal_scale)?;
    ZERO.checked_sub(ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_fold_reassembles_from_the_boltzmann_and_charge_constants() {
        // THE DIMENSIONLESS-CONSTANT LAW: the 2*k_B[eV/K] fold reassembles as 2*k_B[J/K]/e[C] from the exact SI
        // mantissas, so the eV-per-kelvin convention is a ratio of two fundamental constants, not a folded
        // dimensional decimal. 2*k_B/e = 2 * 1.380649e-23 / 1.602176634e-19 = 1.7234667e-4 eV/K.
        let fold = two_kb_ev_per_k();
        assert!(
            close(fold, 1.7234667e-4, 1e-8),
            "2 k_B ~ 1.7234667e-4 eV/K, got {}",
            fold.to_f64_lossy()
        );
    }

    #[test]
    fn the_activation_exponent_is_minus_the_gap_over_2kt() {
        // The log-space activation exponent -E_gap/(2 k_B T) at world temperature (300 K, 2 k_B T = 0.05170 eV).
        let t300 = Fixed::from_int(300);
        // Germanium E_gap = 0.67 eV: -0.67 / 0.05170 = -12.96.
        let ge =
            ln_thermal_carrier_activation(Fixed::from_ratio(67, 100), t300).expect("Ge activation");
        assert!(
            close(ge, -12.96, 0.05),
            "Ge ln-activation ~ -12.96, got {}",
            ge.to_f64_lossy()
        );
        // Silicon E_gap = 1.12 eV: -1.12 / 0.05170 = -21.66.
        let si = ln_thermal_carrier_activation(Fixed::from_ratio(112, 100), t300)
            .expect("Si activation");
        assert!(
            close(si, -21.66, 0.05),
            "Si ln-activation ~ -21.66, got {}",
            si.to_f64_lossy()
        );
    }

    #[test]
    fn a_metal_gap_has_zero_suppression_and_bad_inputs_escalate() {
        let t300 = Fixed::from_int(300);
        // A zero gap (the metal/semimetal boundary): activation exponent 0 (factor 1, no thermal suppression).
        let metal =
            ln_thermal_carrier_activation(ZERO, t300).expect("a zero gap has a defined activation");
        assert_eq!(metal, ZERO, "a zero gap has no thermal suppression");
        // A negative gap (a band overlap, a metal) is classified upstream and escalates here rather than modelling
        // a gap that is not there.
        assert!(
            ln_thermal_carrier_activation(Fixed::from_int(-1), t300).is_none(),
            "a negative gap (overlap) escalates: it is not a thermally-activated semiconductor"
        );
        // A non-positive temperature has no thermal population defined and escalates.
        assert!(
            ln_thermal_carrier_activation(Fixed::from_int(1), ZERO).is_none(),
            "a non-positive temperature escalates"
        );
    }

    #[test]
    fn the_log_space_form_survives_an_insulator_where_the_bare_factor_underflows() {
        // THE CENSUS PAYOFF. Diamond's 5.47 eV gap gives an activation factor exp(-105.8) ~ 1e-46, far below the
        // Q32.32 floor (~2.3e-10): the bare factor underflows to zero and loses all ordering. The LOG-SPACE
        // exponent -105.8 is representable, so insulators' activations stay ordered without underflow. This is why
        // the carrier density is carried in log space (the range-census verdict of the electronic sub-arc).
        let t300 = Fixed::from_int(300);
        let diamond = ln_thermal_carrier_activation(Fixed::from_ratio(547, 100), t300)
            .expect("diamond activation");
        assert!(
            close(diamond, -105.8, 0.2),
            "diamond ln-activation ~ -105.8, got {}",
            diamond.to_f64_lossy()
        );
        // And a wider gap is strictly more suppressed (more negative), so the log-space values order correctly: the
        // insulator sits below the semiconductor.
        let si = ln_thermal_carrier_activation(Fixed::from_ratio(112, 100), t300).expect("Si");
        assert!(
            diamond < si,
            "the wider gap is more suppressed (a more negative log-activation)"
        );
    }

    #[test]
    fn the_activation_factor_round_trips_through_exp_in_window() {
        // For an in-window semiconductor exponent, exp rebuilds the activation factor: germanium's -12.96
        // exponentiates to ~2.35e-6, the thermal carrier fraction. (The insulator case above stays in log space by
        // design, which is the point of returning the log.)
        let t300 = Fixed::from_int(300);
        let ge_ln = ln_thermal_carrier_activation(Fixed::from_ratio(67, 100), t300).expect("Ge");
        let factor = ge_ln.exp();
        assert!(
            close(factor, 2.35e-6, 5e-7),
            "Ge activation factor ~ 2.35e-6, got {}",
            factor.to_f64_lossy()
        );
    }
}
