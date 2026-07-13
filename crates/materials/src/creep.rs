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

//! Stage 6, creep: the steady-state deformation rate, in LOG-SPACE (gate-ratified on #189).
//!
//! Creep is the one property whose absolute rate CANNOT live in Q32.32. The Mukherjee-Bird-Dorn rate
//! `eps_dot = A * (D*G*b / kT) * (b/d)^p * (sigma/G)^n` spans `~1e24` between its reference rate (`~1e9 /s`) and
//! its `(sigma/G)^n ~ 1e-15` term, against the fixed-point's `~19` decades, and `(sigma/G)^n = (1e-3)^5 = 1e-15`
//! rounds to zero below the `~2.3e-10` resolution. So the module works in the natural LOG observable of creep, the
//! quantity a deformation-mechanism map is drawn in (log strain-rate contours over log-stress and
//! homologous temperature):
//!
//! `ln(eps_dot) = ln A + ln D + ln G + ln b - ln kT + n*ln(sigma/G) + p*ln(b/d)`.
//!
//! Every term is representable, and comparing regime log-rates is monotone-equivalent to comparing rates with no
//! underflow. It rides entirely on the canon-pinned `Fixed::exp`/`ln`, so it is deterministic by construction.
//!
//! THE CONTRACT (the gate's five ratified refinements):
//!
//! 1. PER-REGIME log-rates, every regime, not the dominant alone. Creep mechanisms act in PARALLEL, so the
//!    physical total is `sum_i eps_dot_i`, recovered in log-space by [`creep_total_log_rate`] (a numerically-stable
//!    `logsumexp`). The dominant regime ([`creep_dominant_regime`], an `argmax`) is the deformation-map READOUT
//!    alongside, never a substitute for the total.
//! 2. DETERMINISTIC tiebreak: an exact-equal `argmax` (measure-zero but possible in fixed point) breaks to the
//!    LOWEST regime index, the same no-id-tiebreak discipline as the apportionment resolver.
//! 3. The return is NATURAL-LOG, ABSOLUTE, not pre-shifted (`ln eps_dot ~ -46..+5`). A consumer that needs a
//!    magnitude shifts by a reference into the `exp` window `[-22, 21.5]` and exponentiates AT THE POINT OF USE; a
//!    report quotes `log10(eps_dot)` (creep is always reported in decades anyway), needing no `exp`.
//! 4. TIER-2 COUPLING: creep is compare-and-report, so log is sufficient and it takes NO dependency on the
//!    in-flight units-pin Tier-2 wide-intermediate work. A FUTURE consumer that must ACCUMULATE a linear rate
//!    (`integral eps_dot dt`) or sum it with other conserved linear quantities takes it through Tier-2 or a
//!    documented working-scale linearization AT THAT CONSUMER, never by forcing an absolute `eps_dot` into Q32.32.
//! 5. The `a^2` fold: `ln D = 2*ln(a) + ln(R_freezer) + ln(1e-8)` reuses the freezer's `/ps` jump rate UNTOUCHED
//!    (preserving Stage-5 byte-neutrality); the `1/6` random-walk factor is knowingly ABSORBED into the reserved
//!    dislocation prefactor `A` (per the freezer's own `D0 ~ a^2*nu` convention, which already absorbs it), stated.
//!
//! THE REGIME REGISTRY (not a closed enum). A [`CreepRegime`] is a DATA row `{ln_prefactor, stress_exponent,
//! grain_size_exponent}`; the SET of regimes is caller-supplied data and grows, the mechanism (the MBD rate) is
//! fixed. The reserved content differs by regime, run through the derivation-hunter per the gate's ruling:
//! for DIFFUSIONAL regimes (Nabarro-Herring, Coble) `n = 1` is DERIVED (linear response), `p in {2, 3}` is the
//! DERIVED transport geometry, and `A` is a largely-DERIVED geometric factor; only the DISLOCATION regime carries
//! a truly reserved `{ln A, n ~ 3..5}` with `p = 0`. `p` is a discrete mechanism LABEL (the transport-path
//! geometry), never a calibratable tuneable.

use civsim_core::Fixed;

const ZERO: Fixed = Fixed::ZERO;

/// A bounded-log multiply. Every operand in this module is a natural log (`|ln(x)| <= ~22` for any representable
/// `x`) or a small integer coefficient, so the product cannot overflow Q32.32; `checked_mul` is used for form,
/// its `None` branch unreachable under that bound.
fn lmul(a: Fixed, b: Fixed) -> Fixed {
    a.checked_mul(b).unwrap_or(ZERO)
}

/// A creep regime as a DATA row in the registry (not a closed enum): the log-prefactor `ln A`, the stress exponent
/// `n`, and the grain-size exponent `p`. Reserved content is per-regime (the gate's ruling): diffusional regimes
/// carry a DERIVED `n = 1`, a DERIVED transport-geometry `p in {2, 3}`, and a largely-DERIVED geometric `A`; the
/// dislocation regime carries a truly RESERVED `{ln A, n ~ 3..5}` with `p = 0`. All caller-supplied, never planted.
#[derive(Clone, Copy, Debug)]
pub struct CreepRegime {
    /// `ln A`, the natural log of the dimensionless prefactor. Derived-geometric for a diffusional regime;
    /// reserved-with-basis (the lumped empirical prefactor, which also absorbs the `1/6` random-walk factor) for
    /// the dislocation regime.
    pub ln_prefactor: Fixed,
    /// The stress exponent `n`. Exactly `1` (derived, linear response) for a diffusional regime; `~3..5`
    /// (reserved-with-basis) for the dislocation regime.
    pub stress_exponent: Fixed,
    /// The grain-size exponent `p`, a discrete mechanism LABEL (the transport-path geometry): `0` dislocation,
    /// `2` Nabarro-Herring (lattice diffusion through the grain volume), `3` Coble (grain-boundary diffusion).
    /// Never a calibratable tuneable.
    pub grain_size_exponent: i32,
}

/// `ln(10)`, the base of every unit-power log-constant below (derived from the built `ln`, no authored decimal).
fn ln_ten() -> Fixed {
    Fixed::from_int(10).ln()
}

/// `ln(k_B)` for `k_B = 1.380649e-23 J/K`, derived as `ln(1.380649) - 23*ln(10)` from the exact SI mantissa and
/// the built `ln` (no authored decimal). About `-52.64`.
fn ln_boltzmann() -> Fixed {
    Fixed::from_ratio(1_380_649, 1_000_000)
        .ln()
        .saturating_add(lmul(Fixed::from_int(-23), ln_ten()))
}

/// The natural log of the creep reference strain rate `eps_dot_0 = D*G*b / (kT)` (dimensionless log of a `/s`
/// rate), assembled from representable pieces so no linear underflow occurs:
/// `ln(eps_dot_0) = [2*ln(a) + ln(R) + ln(1e-8)] + [ln(G) + ln(1e9)] + [ln(b) + ln(1e-10)] - [ln(k_B) + ln(T)]`.
/// `R` is the freezer's `/ps` self-diffusion jump rate (`nu*exp(-E*/RT)`, reused UNTOUCHED), `a` the atomic spacing
/// (angstrom), `G` the shear modulus (GPa), `b` the Burgers vector (angstrom, `~a`), and the `1e-8`/`1e9`/`1e-10`
/// are the exact `Angstrom^2/ps -> m^2/s`, `GPa -> Pa`, and `Angstrom -> m` unit powers, each derived from
/// [`ln_ten`]. The `1/6` random-walk factor is knowingly absorbed into the reserved dislocation prefactor (per the
/// freezer's `D0 ~ a^2*nu` convention). Returns `None` when the jump rate is non-positive (below freeze-out the
/// freezer saturates `R` to zero): no diffusion, no creep, the material is frozen rather than fabricating a rate.
pub fn creep_ln_reference_rate(
    diffusion_jump_rate_per_ps: Fixed,
    atomic_spacing_angstrom: Fixed,
    shear_modulus_gpa: Fixed,
    burgers_vector_angstrom: Fixed,
    temperature: Fixed,
) -> Option<Fixed> {
    if diffusion_jump_rate_per_ps <= ZERO
        || atomic_spacing_angstrom <= ZERO
        || shear_modulus_gpa <= ZERO
        || burgers_vector_angstrom <= ZERO
        || temperature <= ZERO
    {
        return None;
    }
    let l10 = ln_ten();
    // ln D = 2*ln(a) + ln(R) + ln(1e-8), with ln(1e-8) = -8*ln(10).
    let ln_d = lmul(Fixed::from_int(2), atomic_spacing_angstrom.ln())
        .saturating_add(diffusion_jump_rate_per_ps.ln())
        .saturating_add(lmul(Fixed::from_int(-8), l10));
    // ln G[Pa] = ln(G[GPa]) + 9*ln(10).
    let ln_g = shear_modulus_gpa
        .ln()
        .saturating_add(lmul(Fixed::from_int(9), l10));
    // ln b[m] = ln(b[A]) - 10*ln(10).
    let ln_b = burgers_vector_angstrom
        .ln()
        .saturating_add(lmul(Fixed::from_int(-10), l10));
    // ln kT = ln(k_B) + ln(T).
    let ln_kt = ln_boltzmann().saturating_add(temperature.ln());
    let numerator = ln_d.saturating_add(ln_g).saturating_add(ln_b);
    Some(numerator.checked_sub(ln_kt).unwrap_or(ZERO))
}

/// The natural log of one regime's creep strain rate:
/// `ln(eps_dot) = ln A + ln(eps_dot_0) + n*ln(sigma/G) + p*ln(b/d)`, over the reference log-rate from
/// [`creep_ln_reference_rate`], the regime's reserved/derived `{ln A, n, p}`, and the log stress ratio
/// `ln(sigma/G)` (negative, since operating stress is well below the shear modulus) and log length ratio
/// `ln(b/d)` (negative, the Burgers vector far below the grain size). Absolute natural-log, not pre-shifted.
pub fn creep_regime_log_rate(
    ln_reference_rate: Fixed,
    regime: &CreepRegime,
    ln_stress_over_modulus: Fixed,
    ln_burgers_over_grain: Fixed,
) -> Fixed {
    let stress_term = lmul(regime.stress_exponent, ln_stress_over_modulus);
    let grain_term = lmul(
        Fixed::from_int(regime.grain_size_exponent),
        ln_burgers_over_grain,
    );
    regime
        .ln_prefactor
        .saturating_add(ln_reference_rate)
        .saturating_add(stress_term)
        .saturating_add(grain_term)
}

/// The natural log of the TOTAL creep strain rate over parallel regimes, the numerically-stable `logsumexp`:
/// `ln(sum_i eps_dot_i) = m + ln(sum_i exp(ln_i - m))` with `m = max_i ln_i`. Each `exp(ln_i - m)` is in `(0, 1]`
/// (in-window), so no overflow. This is the PHYSICAL total (mechanisms act in parallel); the dominant regime from
/// [`creep_dominant_regime`] is the map readout, not the total. An empty slice yields [`Fixed::MIN`] (no creep).
pub fn creep_total_log_rate(regime_log_rates: &[Fixed]) -> Fixed {
    if regime_log_rates.is_empty() {
        return Fixed::MIN;
    }
    let mut max = regime_log_rates[0];
    for &lr in &regime_log_rates[1..] {
        if lr > max {
            max = lr;
        }
    }
    let mut sum = ZERO;
    for &lr in regime_log_rates {
        // exp(lr - max) in (0, 1]; lr <= max so the difference is non-positive (no underflow), and a very
        // negative argument saturates the exp to zero (a negligible contributor).
        let shifted = lr.checked_sub(max).unwrap_or(ZERO);
        sum = sum.saturating_add(shifted.exp());
    }
    if sum <= ZERO {
        return max;
    }
    max.saturating_add(sum.ln())
}

/// The dominant creep regime (the deformation-mechanism-map readout): the index of the fastest regime, the
/// `argmax` over the log-rates. DETERMINISTIC tiebreak: an exact-equal maximum breaks to the LOWEST index (a
/// strict `>` keeps the first). `None` for an empty slice.
pub fn creep_dominant_regime(regime_log_rates: &[Fixed]) -> Option<usize> {
    if regime_log_rates.is_empty() {
        return None;
    }
    let mut best = 0usize;
    for (i, &lr) in regime_log_rates.iter().enumerate().skip(1) {
        if lr > regime_log_rates[best] {
            best = i;
        }
    }
    Some(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_reference_log_rate_assembles_from_representable_pieces() {
        // Iron-ish at 1200 K: jump rate R = 1e-3 /ps, a = b = 2.48 A, G = 64 GPa. The linear eps_dot_0 ~ 5.9e10 /s
        // is representable in log-space (ln ~24.8) though the constituent D ~1e-13 m^2/s and kT ~1.66e-20 J are not.
        let ln_ref = creep_ln_reference_rate(
            Fixed::from_ratio(1, 1000),
            Fixed::from_ratio(248, 100),
            Fixed::from_int(64),
            Fixed::from_ratio(248, 100),
            Fixed::from_int(1200),
        )
        .expect("reference rate");
        assert!(
            close(ln_ref, 24.80, 0.2),
            "ln(eps_dot_0) ~24.8 (eps_dot_0 ~5.9e10 /s): {ln_ref:?}"
        );
        // A faster jump rate (hotter) raises the reference rate; determinism.
        let ln_ref_hot = creep_ln_reference_rate(
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(248, 100),
            Fixed::from_int(64),
            Fixed::from_ratio(248, 100),
            Fixed::from_int(1200),
        )
        .expect("hot reference rate");
        assert!(
            ln_ref_hot > ln_ref,
            "a faster jump rate raises the reference rate"
        );
        // Frozen: a non-positive jump rate (below freeze-out) yields None, not a fabricated rate.
        assert!(
            creep_ln_reference_rate(
                ZERO,
                Fixed::from_ratio(248, 100),
                Fixed::from_int(64),
                Fixed::from_ratio(248, 100),
                Fixed::from_int(1200)
            )
            .is_none(),
            "a frozen (zero) jump rate escalates to None"
        );
    }

    #[test]
    fn the_operative_creep_regime_emerges_from_the_competition() {
        // Reference log-rate ~24.8 (above). Regimes: N-H (derived n=1, p=2, geometric A~14), Coble (n=1, p=3,
        // A~50), dislocation (RESERVED n=5, A~7e5, p=0). ln(sigma/G), ln(b/d) caller-supplied.
        let ln_ref = Fixed::from_ratio(2480, 100);
        let nh = CreepRegime {
            ln_prefactor: Fixed::from_int(14).ln(),
            stress_exponent: Fixed::from_int(1),
            grain_size_exponent: 2,
        };
        let coble = CreepRegime {
            ln_prefactor: Fixed::from_int(50).ln(),
            stress_exponent: Fixed::from_int(1),
            grain_size_exponent: 3,
        };
        let disloc = CreepRegime {
            ln_prefactor: Fixed::from_int(700_000).ln(),
            stress_exponent: Fixed::from_int(5),
            grain_size_exponent: 0,
        };
        let regimes = [nh, coble, disloc];
        let ln_bd = Fixed::from_ratio(1, 10000).ln(); // b/d = 1e-4

        // HIGH stress (sigma/G = 1e-2): the dislocation regime (n=5) dominates.
        let ln_sg_hi = Fixed::from_ratio(1, 100).ln();
        let rates_hi: Vec<Fixed> = regimes
            .iter()
            .map(|r| creep_regime_log_rate(ln_ref, r, ln_sg_hi, ln_bd))
            .collect();
        assert_eq!(
            creep_dominant_regime(&rates_hi),
            Some(2),
            "dislocation creep dominates at high stress"
        );

        // LOW stress (sigma/G = 1e-4): the dislocation term (n=5) collapses and a diffusional regime takes over.
        let ln_sg_lo = Fixed::from_ratio(1, 10000).ln();
        let rates_lo: Vec<Fixed> = regimes
            .iter()
            .map(|r| creep_regime_log_rate(ln_ref, r, ln_sg_lo, ln_bd))
            .collect();
        let dom_lo = creep_dominant_regime(&rates_lo).expect("dominant");
        assert!(
            dom_lo < 2,
            "a diffusional regime (N-H or Coble) dominates at low stress: {dom_lo}"
        );
        // The crossover is EMERGENT: the dominant regime changed with stress, never authored.
        assert_ne!(
            creep_dominant_regime(&rates_hi),
            creep_dominant_regime(&rates_lo),
            "the operative regime changes across the stress crossover (emergent map)"
        );

        // The TOTAL (logsumexp) is bounded by [max, max + ln(n)]: at least the dominant regime, at most the
        // dominant plus ln(number of regimes). (The absolute magnitude here reflects the test's A-conventions, not
        // physics; the competition STRUCTURE is what is validated.)
        let total_hi = creep_total_log_rate(&rates_hi);
        let max_hi = rates_hi[2];
        let ln_n = Fixed::from_int(3).ln();
        assert!(
            total_hi >= max_hi && total_hi <= max_hi.saturating_add(ln_n),
            "the total log-rate is within [max, max + ln(n)] of the dominant: {total_hi:?}"
        );

        // Deterministic tiebreak: exactly-equal log-rates break to the lowest index.
        let tie = [Fixed::from_int(3), Fixed::from_int(3), Fixed::from_int(1)];
        assert_eq!(
            creep_dominant_regime(&tie),
            Some(0),
            "an exact tie breaks to the lowest regime index"
        );
        // Empty registry: no dominant, total is MIN (no creep).
        assert_eq!(creep_dominant_regime(&[]), None);
        assert_eq!(creep_total_log_rate(&[]), Fixed::MIN);
    }
}
