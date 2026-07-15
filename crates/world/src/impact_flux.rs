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

//! The impact-flux (accretion-tail) model: how the impactor population arriving at a young planet DECLINES over
//! deep time as the leftover planetesimal reservoir is swept up, and how that population splits by size. It is
//! the temporal front of the impact chain, feeding the crater law ([`crate::crater`], each drawn impactor makes
//! a crater) and the ejecta fan behind it; the actual strikes are the deep-time run loop's deterministic
//! `seeded_draw` from the expected flux this model gives, the same contingency machinery that prices the
//! giant-impact event tier.
//!
//! The flux is NOT an authored bombardment curve (Principle 8): it is the accretion tail's own decay. The
//! reservoir of leftover planetesimals near the planet depletes as the planet accretes it, by the mean-field
//! sweep-up `dM/dt = -M / tau`: the accreted mass rate falls as `exp(-t / tau)` and the cumulative accreted
//! fraction climbs as `1 - exp(-t / tau)`, the exponential accretion tail, with `tau` the sweep-up timescale
//! (a per-world dynamical value, the planet's gravitational-focusing cross-section over the reservoir's
//! dynamical spreading). Working in these dimensionless fractions keeps the wide reservoir mass out of fixed
//! point: the caller scales them by its own reservoir mass. The linear `dM/dt = -M/tau` is the rung-0
//! depletion (a constant fractional sweep-up probability); a size-and-spreading-dependent power-law tail is the
//! named refinement.
//!
//! The size split is the collisional-cascade size-frequency distribution, a differential power law
//! `dN/dD = D^(-p)` with `p` the Dohnanyi slope (near `3.5`, the same cascade the grain-opacity wire reads), so
//! the number of bodies is dominated by the small end (`p > 1`) and the mass by the large end (`p < 4`): the
//! reservoir is a swarm of small bodies carrying its mass in a few large ones. The cumulative fractions above a
//! size are formed in size RATIOS to the largest body, so the absolute sizes (metres to thousands of
//! kilometres) never enter fixed point; a size range too wide for the fixed-point range fails soft (the named
//! log-space refinement) rather than overflowing.
//!
//! Admit-the-alien (a prime directive): every input is the reservoir's or the world's own datum (the sweep-up
//! timescale, the cascade slope, the size bounds). A different disk, a captured swarm, or a late instability
//! delivering a distinct population are each a different set of numbers through the same law, not a new code
//! path. Determinism (Principle 3, Principle 10): fixed-point throughout, the pinned [`Fixed::exp`] and
//! [`Fixed::powf`], staged in ratios so no physical intermediate rails; a non-physical input fails soft to
//! `None`, never a fabricated flux.

use civsim_core::Fixed;

/// The instantaneous impact-RATE at a time as a fraction of the initial rate: `exp(-t / tau)`, the accretion
/// tail's exponential decay. One at `t = 0` (the full early bombardment), falling toward zero over deep time as
/// the reservoir empties. `time` and `sweep_timescale` share the caller's time unit (megayears is the natural
/// one). `None` on a non-positive timescale or a negative time; a very late time underflows to zero (a spent
/// reservoir), the honest value, not an error.
pub fn tail_rate_fraction(time: Fixed, sweep_timescale: Fixed) -> Option<Fixed> {
    if sweep_timescale <= Fixed::ZERO || time < Fixed::ZERO {
        return None;
    }
    let ratio = time.checked_div(sweep_timescale)?;
    Some(Fixed::ZERO.checked_sub(ratio)?.exp())
}

/// The cumulative fraction of the reservoir accreted by a time: `1 - exp(-t / tau)`. Zero at `t = 0`, climbing
/// toward one as the tail is swept up. The complement of [`tail_rate_fraction`]'s integral, the total
/// bombardment delivered by `time`. `None` on a non-positive timescale or a negative time.
pub fn cumulative_accreted_fraction(time: Fixed, sweep_timescale: Fixed) -> Option<Fixed> {
    let rate = tail_rate_fraction(time, sweep_timescale)?;
    Fixed::ONE.checked_sub(rate)
}

/// The cumulative fraction of the reservoir's BODIES (by number) larger than `size`, from the differential
/// power-law size-frequency distribution `dN/dD = D^(-p)` over `[min_size, max_size]`. One at `min_size`, zero
/// at `max_size`, monotone decreasing: most bodies are small, so the fraction above a given size falls quickly.
/// Formed in ratios to `max_size` so the absolute sizes never enter fixed point. `None` if the slope is not
/// above one (the number integral would not converge on the small end), the sizes are non-physical or
/// disordered, or the ratio power overflows the fixed-point range (a size range too wide, the named log-space
/// refinement).
pub fn number_fraction_above_size(
    size: Fixed,
    min_size: Fixed,
    max_size: Fixed,
    differential_slope: Fixed,
) -> Option<Fixed> {
    // Cumulative-number exponent is `1 - p`; convergence on the small end needs `p > 1`.
    let exponent = Fixed::ONE.checked_sub(differential_slope)?;
    if exponent >= Fixed::ZERO {
        return None;
    }
    fraction_above(size, min_size, max_size, exponent, true)
}

/// The cumulative fraction of the reservoir's MASS in bodies larger than `size`, from the same power-law
/// `dN/dD = D^(-p)` weighted by each body's mass `~ D^3`. One at `min_size`, zero at `max_size`, monotone
/// decreasing but far shallower than the number fraction: for `1 < p < 4` the mass sits in the largest bodies,
/// so most of the mass is above even a fairly large size. Formed in ratios to `max_size`. `None` if the slope
/// is not below four (the mass integral would not converge on the large end), the sizes are non-physical, or a
/// power overflows.
pub fn mass_fraction_above_size(
    size: Fixed,
    min_size: Fixed,
    max_size: Fixed,
    differential_slope: Fixed,
) -> Option<Fixed> {
    // Cumulative-mass exponent is `4 - p`; convergence on the large end needs `p < 4` (so the exponent is
    // positive and the mass is carried by the big bodies).
    let exponent = Fixed::from_int(4).checked_sub(differential_slope)?;
    if exponent <= Fixed::ZERO {
        return None;
    }
    fraction_above(size, min_size, max_size, exponent, false)
}

/// The shared cumulative-fraction-above-size kernel for the power-law distribution, in ratios to `max_size`.
/// With `u = size/max_size` and `umin = min_size/max_size`, a NEGATIVE `exponent` (the number case, `1 - p`)
/// gives `(u^e - 1) / (umin^e - 1)` and a POSITIVE `exponent` (the mass case, `4 - p`) gives
/// `(1 - u^e) / (1 - umin^e)`; both run from one at `min_size` to zero at `max_size`. `negative_exponent` picks
/// the branch. `None` on non-physical or disordered sizes, or if a ratio power rails.
fn fraction_above(
    size: Fixed,
    min_size: Fixed,
    max_size: Fixed,
    exponent: Fixed,
    negative_exponent: bool,
) -> Option<Fixed> {
    if min_size <= Fixed::ZERO || max_size <= Fixed::ZERO || min_size > max_size {
        return None;
    }
    if size < min_size || size > max_size {
        return None;
    }
    let u = size.checked_div(max_size)?;
    let umin = min_size.checked_div(max_size)?;
    let u_e = u.powf(exponent);
    let umin_e = umin.powf(exponent);
    let (numer, denom) = if negative_exponent {
        // u^e >= 1 for u <= 1 and e < 0; the fraction is (u^e - 1)/(umin^e - 1).
        (
            u_e.checked_sub(Fixed::ONE)?,
            umin_e.checked_sub(Fixed::ONE)?,
        )
    } else {
        // u^e <= 1 for u <= 1 and e > 0; the fraction is (1 - u^e)/(1 - umin^e).
        (
            Fixed::ONE.checked_sub(u_e)?,
            Fixed::ONE.checked_sub(umin_e)?,
        )
    };
    if denom <= Fixed::ZERO {
        // A degenerate size range (min_size == max_size, a single-size reservoir): no distribution to split.
        return None;
    }
    numer.checked_div(denom)
}

#[cfg(test)]
mod tests {
    use super::*;

    // An illustrative late-accretion reservoir, the numbers cited to the literature (test fixtures standing in
    // for a world's reserved values, not authored floor constants): a sweep-up timescale ~ 30 Myr (Wetherill,
    // Chambers late-accretion), a Dohnanyi cascade slope p ~ 3.5 (Dohnanyi 1969), sizes from 100 m to 100 km.
    fn tau() -> Fixed {
        Fixed::from_int(30)
    }
    fn slope() -> Fixed {
        Fixed::from_ratio(35, 10)
    }
    fn dmin() -> Fixed {
        Fixed::from_int(100)
    }
    fn dmax() -> Fixed {
        Fixed::from_int(100_000)
    }

    #[test]
    fn the_accretion_tail_decays_and_the_cumulative_fills() {
        let early = tail_rate_fraction(Fixed::ZERO, tau()).expect("t=0 resolves");
        let mid = tail_rate_fraction(Fixed::from_int(30), tau()).expect("t=tau resolves");
        let late = tail_rate_fraction(Fixed::from_int(120), tau()).expect("t=4tau resolves");
        // Rate is 1 at t=0 and falls monotonically.
        assert_eq!(early, Fixed::ONE, "the initial rate is the full reservoir");
        assert!(
            mid < early && late < mid,
            "the impact rate decays over time"
        );
        // At one timescale exp(-1) ~ 0.368.
        let mid_f = mid.to_f64_lossy();
        assert!(
            (mid_f - 0.3679).abs() < 0.01,
            "one sweep-up timescale leaves exp(-1) ~ 0.368 of the rate, got {mid_f}"
        );
        // Cumulative climbs toward one.
        let c_mid = cumulative_accreted_fraction(Fixed::from_int(30), tau()).expect("resolves");
        let c_late = cumulative_accreted_fraction(Fixed::from_int(120), tau()).expect("resolves");
        assert!(
            c_late > c_mid && c_late < Fixed::ONE && c_mid > Fixed::ZERO,
            "the cumulative accreted fraction fills toward one"
        );
        // Rate and cumulative are complements at every time.
        assert_eq!(
            mid.checked_add(c_mid).unwrap(),
            Fixed::ONE,
            "rate + cumulative = 1"
        );
    }

    #[test]
    fn the_size_fractions_run_from_one_to_zero_and_are_monotone() {
        // At the smallest size the fraction above is one; at the largest it is zero.
        let n_min = number_fraction_above_size(dmin(), dmin(), dmax(), slope()).expect("resolves");
        let n_max = number_fraction_above_size(dmax(), dmin(), dmax(), slope()).expect("resolves");
        assert!(
            (n_min.to_f64_lossy() - 1.0).abs() < 1e-6,
            "all bodies are larger than the smallest size"
        );
        assert!(
            n_max.to_f64_lossy().abs() < 1e-6,
            "no body is larger than the largest size"
        );
        // Monotone decreasing across the range.
        let n_small = number_fraction_above_size(Fixed::from_int(1000), dmin(), dmax(), slope())
            .expect("resolves");
        let n_big = number_fraction_above_size(Fixed::from_int(10_000), dmin(), dmax(), slope())
            .expect("resolves");
        assert!(
            n_small > n_big,
            "fewer bodies are larger than a bigger size"
        );
    }

    #[test]
    fn the_number_is_dominated_by_the_small_the_mass_by_the_large() {
        // At the geometric-mean size (~3162 m), the fraction of BODIES above it is tiny (the swarm is small
        // bodies) while the fraction of MASS above it is large (the mass is in the big bodies). This is the
        // collisional-cascade signature for 1 < p < 4, the physical content of the model.
        let d_mid = Fixed::from_int(3162); // sqrt(100 * 100000) ~ 3162.
        let n = number_fraction_above_size(d_mid, dmin(), dmax(), slope()).expect("resolves");
        let m = mass_fraction_above_size(d_mid, dmin(), dmax(), slope()).expect("resolves");
        assert!(
            n.to_f64_lossy() < 0.01,
            "far fewer than 1% of bodies are above the geometric mean, got {}",
            n.to_f64_lossy()
        );
        assert!(
            m.to_f64_lossy() > 0.5,
            "most of the mass is in bodies above the geometric mean, got {}",
            m.to_f64_lossy()
        );
        assert!(m > n, "the mass is far more top-heavy than the number");
    }

    #[test]
    fn the_size_fractions_track_an_independent_float_evaluation() {
        // A numerical twin: the same power-law fractions in f64 (an independent arithmetic path), validating the
        // fixed-point ratio staging and the pinned powf, not the physics (the monotonicity and dominance tests
        // and the Dohnanyi citation carry that).
        let (d, lo, hi, p) = (3162.0_f64, 100.0_f64, 100_000.0_f64, 3.5_f64);
        let (u, umin) = (d / hi, lo / hi);
        let n_ref = (u.powf(1.0 - p) - 1.0) / (umin.powf(1.0 - p) - 1.0);
        let m_ref = (1.0 - u.powf(4.0 - p)) / (1.0 - umin.powf(4.0 - p));
        let n = number_fraction_above_size(Fixed::from_int(3162), dmin(), dmax(), slope())
            .expect("resolves")
            .to_f64_lossy();
        let m = mass_fraction_above_size(Fixed::from_int(3162), dmin(), dmax(), slope())
            .expect("resolves")
            .to_f64_lossy();
        assert!(
            (n - n_ref).abs() / n_ref < 0.02,
            "number fraction {n} within 2% of the float twin {n_ref}"
        );
        assert!(
            (m - m_ref).abs() / m_ref < 0.02,
            "mass fraction {m} within 2% of the float twin {m_ref}"
        );
    }

    #[test]
    fn the_alien_reservoir_is_a_data_row() {
        // A steeper cascade (p = 3.8) and a different size range: the same law, a finite split. No Terran
        // assumption blocks it.
        let n = number_fraction_above_size(
            Fixed::from_int(5000),
            Fixed::from_int(50),
            Fixed::from_int(500_000),
            Fixed::from_ratio(38, 10),
        )
        .expect("an alien reservoir resolves");
        assert!(n > Fixed::ZERO && n < Fixed::ONE, "a finite size fraction");
    }

    #[test]
    fn non_physical_and_out_of_convergence_inputs_fail_soft() {
        // A non-positive timescale: no decay.
        assert!(tail_rate_fraction(Fixed::from_int(10), Fixed::ZERO).is_none());
        // A slope not above one: the number integral does not converge on the small end.
        assert!(
            number_fraction_above_size(dmin(), dmin(), dmax(), Fixed::from_ratio(5, 10)).is_none()
        );
        // A slope not below four: the mass integral does not converge on the large end.
        assert!(mass_fraction_above_size(dmin(), dmin(), dmax(), Fixed::from_int(4)).is_none());
        // A size outside the range: undefined.
        assert!(number_fraction_above_size(Fixed::from_int(10), dmin(), dmax(), slope()).is_none());
        // A degenerate single-size reservoir: no distribution to split.
        assert!(number_fraction_above_size(dmin(), dmin(), dmin(), slope()).is_none());
    }
}
