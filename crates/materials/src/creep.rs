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
//! THE OWNER'S BUILD-NOW RIDERS (ruled on #189):
//!
//! - (1a) [`creep_selection_gap`] emits `delta_log = ln(rate_top) - ln(rate_second)`, the Gap Law field in the
//!   mechanism-selection context (dimensionless in natural units, computed for free alongside the `argmax`).
//!   `delta_log >~ 1` (a decade) is the single-mechanism regime (a deformation-map field interior); `delta_log ~ 0`
//!   is the deformation-map BOUNDARY itself, where two mechanisms co-run, so the map's boundaries become computed
//!   `delta_log = 0` contours rather than drawn lines.
//! - (1b) [`CreepComposition`] tags the composition law PER BOUNDARY: `Parallel = logsumexp` (parallel mechanisms,
//!   e.g. dislocation alongside diffusion, ADD rates) and `Sequential = harmonic-min` (sequential accommodation,
//!   e.g. grain-boundary sliding limited by diffusion, is harmonic in rate-space, the slowest step governing).
//!   [`creep_total_log_rate`] dispatches on it; a bare `logsumexp` default would silently mis-compose the
//!   sequential case by up to a factor of two at the boundary.
//! - (1c) The `logsumexp` sums in CANONICAL operand order (the shifted terms sorted before reduction), so a
//!   permutation of the regime order yields a bit-identical total (the max-subtraction step adds an order
//!   sensitivity the linear deterministic-sum law did not have). Property-tested.
//!
//! THE LOG-SPACE BOUNDARY CONTRACT (refinement 5, extended). A consumer that must ACCUMULATE a linear quantity
//! (a strain integral, damage work, heat deposition) exponentiates EXACTLY ONCE, at the accumulation site, against
//! the pinned `exp`, with the ACCUMULATOR in a DIFFERENT fixed-point scale than the rate (strain-per-step stays
//! small even when strain-rate decades are wild, which is the whole point of the census), the per-domain scale
//! chosen by the units machinery. Never force an absolute `eps_dot` back into the rate's Q32.32 scale, and never
//! build a lossy Q32.32 compatibility bridge that would reintroduce the underflow one crate over: the single `exp`
//! at the accumulator, in the accumulator's scale, is the only linearization point.
//!
//! DISPLAY-TIME QUANTIZATION. The return is full-resolution natural-log; a renderer that wants decade buckets
//! (a deformation-map texture) floors to `log10` decades at DISPLAY time. Never quantize in the data plane what
//! the decision plane needs at full resolution (a `delta_log = 0.3` competition, a factor of two, must not round
//! to a tie).
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

/// How co-running creep mechanisms compose at a deformation-map boundary (rider 1b). PARALLEL mechanisms add their
/// rates (`logsumexp` in log-space); SEQUENTIAL accommodation is harmonic in rate-space (the slowest step governs,
/// the `min`-like combination). The type forces the caller to name the boundary's composition so a bare
/// `logsumexp` cannot silently mis-compose the sequential case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreepComposition {
    /// Rates add: `eps_dot_total = sum_i eps_dot_i` (dislocation in parallel with diffusion). The `logsumexp`.
    Parallel,
    /// Rates combine harmonically: `1/eps_dot_total = sum_i 1/eps_dot_i` (grain-boundary sliding limited by
    /// diffusion), the slowest step governing. The negated-`logsumexp`-of-negatives (a soft-min, `-> min`).
    Sequential,
}

/// The canonical-order `logsumexp` `m + ln(sum_i exp(x_i - m))`, `m = max_i x_i`. Each `exp(x_i - m)` is in
/// `(0, 1]` (in-window, no overflow). The shifted terms are summed in CANONICAL (sorted-ascending) order so the
/// reduction is permutation-independent (rider 1c), the fixed-topology-reduction discipline in log-space. The
/// slice must be non-empty (callers guard).
/// @provides log_sum_exp
fn logsumexp_canonical(values: &[Fixed]) -> Fixed {
    let mut max = values[0];
    for &v in &values[1..] {
        if v > max {
            max = v;
        }
    }
    let mut terms: Vec<Fixed> = values
        .iter()
        .map(|&v| v.checked_sub(max).unwrap_or(ZERO).exp())
        .collect();
    terms.sort();
    let mut sum = ZERO;
    for t in terms {
        sum = sum.saturating_add(t);
    }
    if sum <= ZERO {
        return max;
    }
    max.saturating_add(sum.ln())
}

/// The natural log of the TOTAL creep strain rate over co-running regimes, dispatched on the boundary's
/// [`CreepComposition`] (rider 1b). PARALLEL is the `logsumexp` (rates add), the physical total where mechanisms
/// run in parallel. SEQUENTIAL is `-logsumexp(-ln_i)` (rates combine harmonically, the slowest step governing),
/// the soft-min that goes to the minimum log-rate when the steps are well separated. Both reduce in canonical
/// operand order (rider 1c). The dominant regime from [`creep_dominant_regime`] is the map readout, not the total.
/// An empty slice yields [`Fixed::MIN`] (no creep).
pub fn creep_total_log_rate(regime_log_rates: &[Fixed], composition: CreepComposition) -> Fixed {
    if regime_log_rates.is_empty() {
        return Fixed::MIN;
    }
    match composition {
        CreepComposition::Parallel => logsumexp_canonical(regime_log_rates),
        CreepComposition::Sequential => {
            // 1/eps_tot = sum 1/eps_i, so ln(eps_tot) = -logsumexp(-ln_i): the harmonic (series-rate) combination,
            // the slowest step governing.
            let negated: Vec<Fixed> = regime_log_rates.iter().map(|&r| -r).collect();
            -logsumexp_canonical(&negated)
        }
    }
}

/// The mechanism-selection GAP `delta_log = ln(rate_top) - ln(rate_second)` (rider 1a), the Gap Law field for
/// creep: the log-rate margin of the fastest regime over the runner-up, dimensionless in natural units and always
/// non-negative. `delta_log` around a decade or more is the single-mechanism regime (a deformation-map field
/// interior); `delta_log` near zero is the deformation-map BOUNDARY, where two mechanisms co-run and the
/// composition ([`CreepComposition`]) matters. `None` when fewer than two regimes are present (no competition).
pub fn creep_selection_gap(regime_log_rates: &[Fixed]) -> Option<Fixed> {
    if regime_log_rates.len() < 2 {
        return None;
    }
    // The top two log-rates by value (order-independent), so delta_log is permutation-independent.
    let mut top = Fixed::MIN;
    let mut second = Fixed::MIN;
    for &lr in regime_log_rates {
        if lr > top {
            second = top;
            top = lr;
        } else if lr > second {
            second = lr;
        }
    }
    Some(top.checked_sub(second).unwrap_or(ZERO))
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

        // The PARALLEL TOTAL (logsumexp) is bounded by [max, max + ln(n)]: at least the dominant regime, at most
        // the dominant plus ln(number of regimes). (The absolute magnitude here reflects the test's A-conventions,
        // not physics; the competition STRUCTURE is what is validated.)
        let total_hi = creep_total_log_rate(&rates_hi, CreepComposition::Parallel);
        let max_hi = rates_hi[2];
        let ln_n = Fixed::from_int(3).ln();
        assert!(
            total_hi >= max_hi && total_hi <= max_hi.saturating_add(ln_n),
            "the parallel total log-rate is within [max, max + ln(n)] of the dominant: {total_hi:?}"
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
        assert_eq!(
            creep_total_log_rate(&[], CreepComposition::Parallel),
            Fixed::MIN
        );
    }

    #[test]
    fn the_selection_gap_and_composition_carry_the_boundary_physics() {
        // Rider 1a: delta_log = ln(rate_top) - ln(rate_second), the Gap Law field. Well-separated rates give a
        // large gap (single-mechanism regime); near-equal rates give a gap near zero (the map boundary).
        let separated = [Fixed::from_int(10), Fixed::from_int(2), Fixed::from_int(-5)];
        let gap = creep_selection_gap(&separated).expect("gap");
        assert!(
            close(gap, 8.0, 1e-6),
            "delta_log is the top-minus-second margin (10 - 2 = 8): {gap:?}"
        );
        let boundary = [Fixed::from_int(5), Fixed::from_int(5), Fixed::from_int(1)];
        assert_eq!(
            creep_selection_gap(&boundary),
            Some(ZERO),
            "co-running mechanisms give delta_log = 0 (the deformation-map boundary)"
        );
        // Fewer than two regimes: no competition, no gap.
        assert_eq!(creep_selection_gap(&[Fixed::from_int(3)]), None);

        // Rider 1b: at a boundary the composition matters. Two equal rates (ln = 2, eps_dot = e^2 each):
        // PARALLEL adds them -> ln(2*e^2) = 2 + ln(2) ~2.69; SEQUENTIAL is harmonic -> the same two equal rates
        // give half the rate -> ln(e^2 / 2) = 2 - ln(2) ~1.31. They straddle the single-rate value 2.
        let pair = [Fixed::from_int(2), Fixed::from_int(2)];
        let par = creep_total_log_rate(&pair, CreepComposition::Parallel);
        let seq = creep_total_log_rate(&pair, CreepComposition::Sequential);
        assert!(
            close(par, 2.0 + 2.0_f64.ln(), 0.01),
            "parallel adds: {par:?}"
        );
        assert!(
            close(seq, 2.0 - 2.0_f64.ln(), 0.01),
            "sequential is harmonic: {seq:?}"
        );
        assert!(
            par > Fixed::from_int(2) && seq < Fixed::from_int(2),
            "parallel exceeds and sequential falls below the single-mechanism rate"
        );

        // Rider 1c: the logsumexp is permutation-INDEPENDENT (canonical operand order), bit-identical under a
        // reordering of the regimes.
        let a = [
            Fixed::from_int(7),
            Fixed::from_int(3),
            Fixed::from_int(-2),
            Fixed::from_int(5),
        ];
        let b = [
            Fixed::from_int(-2),
            Fixed::from_int(7),
            Fixed::from_int(5),
            Fixed::from_int(3),
        ];
        assert_eq!(
            creep_total_log_rate(&a, CreepComposition::Parallel),
            creep_total_log_rate(&b, CreepComposition::Parallel),
            "the parallel total is bit-identical under a permutation of the regimes"
        );
        assert_eq!(
            creep_total_log_rate(&a, CreepComposition::Sequential),
            creep_total_log_rate(&b, CreepComposition::Sequential),
            "the sequential total is bit-identical under a permutation of the regimes"
        );
    }
}
