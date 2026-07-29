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
//! kilometres) never enter fixed point.
//!
//! That ratio staging has a WIDTH LIMIT, and it is tighter than it looks. A fraction needs `u^(1-p)` inside the
//! fixed-point range, so it holds only while `(p - 1) * ln(max/min)` stays under the `exp` window of about 22:
//! roughly 3.8 decades of size at the Dohnanyi slope, less at a steeper one. Past that the power SATURATES, and
//! saturation is silent, so the fractions now refuse instead (see `unsaturated_powf`); before that guard they
//! returned a confidently wrong number. The LOG-SPACE form is the answer for the quantity that most needs the
//! range, the number-weighted mean body mass ([`ln_mean_cube_size_ratio`]), which carries any width the size
//! bounds can express and is what a reservoir's physical body count is derived from.
//!
//! Admit-the-alien (a prime directive): every input is the reservoir's or the world's own datum (the sweep-up
//! timescale, the cascade slope, the size bounds). A different disk, a captured swarm, or a late instability
//! delivering a distinct population are each a different set of numbers through the same law, not a new code
//! path. Determinism (Principle 3, Principle 10): fixed-point throughout, the pinned [`Fixed::exp`] and
//! [`Fixed::powf`], staged in ratios or in logs so a physical intermediate that would rail is caught rather than
//! saturated; a non-physical or unstageable input fails soft to `None`, never a fabricated flux.

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

/// Draw the body SIZE at a given cumulative number-fraction-above, the INVERSE of
/// [`number_fraction_above_size`]: given a target fraction `f` in `[0, 1]` (a uniform deviate, for
/// inverse-transform sampling), return the size `D` whose fraction of bodies above it is exactly `f`. A
/// uniform draw over `f` therefore samples the size-frequency distribution BY NUMBER, so the drawn size is
/// small far more often than large (the Dohnanyi swarm: `f = 0` returns `max_size`, no body is larger; `f = 1`
/// returns `min_size`, every body is larger; monotone in between). The size is not authored: it is a draw from
/// the world's own reservoir distribution. Formed in ratios to `max_size` so the absolute sizes never enter
/// fixed point. `None` under the same conditions as [`number_fraction_above_size`]: a slope not above one (the
/// small end would not converge), non-physical or disordered sizes, a degenerate single-size reservoir, a
/// fraction outside `[0, 1]`, or a ratio power that overflows (a size range too wide, the log-space refinement).
pub fn size_at_number_fraction(
    fraction: Fixed,
    min_size: Fixed,
    max_size: Fixed,
    differential_slope: Fixed,
) -> Option<Fixed> {
    if min_size <= Fixed::ZERO || max_size <= Fixed::ZERO || min_size > max_size {
        return None;
    }
    if fraction < Fixed::ZERO || fraction > Fixed::ONE {
        return None;
    }
    // The cumulative-number exponent is `e = 1 - p`; convergence on the small end needs `p > 1`, so `e < 0`.
    let exponent = Fixed::ONE.checked_sub(differential_slope)?;
    if exponent >= Fixed::ZERO {
        return None;
    }
    // With `u = D/max_size`, the survival function is `f = (u^e - 1)/(umin^e - 1)`. Invert it: `umin^e > 1`
    // (since `umin < 1` and `e < 0`), so the span `umin^e - 1` is positive and `u^e = 1 + f (umin^e - 1)` runs
    // from 1 (at `f = 0`, size `= max_size`) up to `umin^e` (at `f = 1`, size `= min_size`).
    let umin = min_size.checked_div(max_size)?;
    let umin_e = unsaturated_powf(umin, exponent)?;
    let span = umin_e.checked_sub(Fixed::ONE)?;
    if span <= Fixed::ZERO {
        // A degenerate single-size reservoir (`min_size == max_size`): no distribution to draw from.
        return None;
    }
    let u_e = Fixed::ONE.checked_add(fraction.checked_mul(span)?)?;
    // Invert the power: `u = (u^e)^(1/e)`. The base `u_e >= 1 > 0`, so `powf` is well-defined; `1/e < 0`.
    let inv_exponent = Fixed::ONE.checked_div(exponent)?;
    let u = unsaturated_powf(u_e, inv_exponent)?;
    u.checked_mul(max_size)
}

/// `x^e`, refusing where the pinned [`Fixed::powf`] would SATURATE rather than answer. `powf` composes
/// `exp(e * ln x)` and [`Fixed::exp`] rails to [`Fixed::MAX`] above an argument near 22, so a size ratio raised to
/// a steep negative exponent returns the fixed-point ceiling in place of a number decades larger: over six decades
/// of size at the Dohnanyi slope the true `u^(1-p)` is ~1e15 against a ceiling of ~2.1e9, and the cumulative
/// fraction built from it came back 4.7e5 times too large (5.7 orders of magnitude) with no signal. Reading the
/// saturation sentinel back (rather than restating `exp`'s window here, which would be a second copy of a constant
/// that lives in the numeric type) makes the ratio staging's documented fail-soft real: a size range too wide to
/// stage returns `None`, and [`ln_mean_cube_size_ratio`] is the log-space form that carries the same range.
fn unsaturated_powf(x: Fixed, exponent: Fixed) -> Option<Fixed> {
    if x <= Fixed::ZERO {
        return None;
    }
    let power = x.powf(exponent);
    if power >= Fixed::MAX {
        return None;
    }
    Some(power)
}

/// The natural log of the NUMBER-WEIGHTED MEAN CUBE of body size, as a ratio to the largest body's cube:
/// `ln(<D^3> / max_size^3)` for the differential power law `dN/dD = D^(-p)` over `[min_size, max_size]`. Derived
/// ANALYTICALLY from the power law (a ratio of two closed-form integrals) rather than sampled, so it carries no
/// draw noise and no iteration count. Where a reservoir's bodies share a bulk density and shape, the density and
/// the shape factor CANCEL out of this ratio, so it is equally the mean body MASS as a fraction of the largest
/// body's mass: that is how a reservoir's physical body COUNT reads it, as reservoir mass over largest-body mass
/// over this ratio.
///
/// Returned in LOG space because the linear ratio sinks through the fixed-point floor: a 100 m to 100 km reservoir
/// at the Dohnanyi slope puts it at `1.5e-7`, about 658 units of the `~2.3e-10` grid; one more decade of range
/// leaves 2 units, and two more decades put it under the grid entirely. The log stays comfortable at every width
/// the size bounds can express, so this helper is the log-space refinement the ratio-staged siblings above name.
///
/// THE ALGEBRA. With `u = min_size/max_size` and `T(e) = (1 - u^e) / e` (the integral `int D^(e-1) dD` over the
/// range, divided by `max_size^e`), the number-weighted mean cube is `T(4 - p) / T(1 - p)`; the `max_size^e`
/// factors cancel to exactly `max_size^3`, so only `u` and the slope enter and the absolute sizes never reach
/// fixed point. `T` takes three explicit branches, one per SIGN of its exponent, because the sign is which end of
/// the size range carries the integral and the two ends need different staging:
///
/// - `e > 0`, the LARGE end carries it: `u^e` lies in `(0, 1)`, so `ln T = ln(1 - u^e) - ln(e)` stages directly.
/// - `e < 0`, the SMALL end carries it: `u^e` is large and rails the fixed-point range, so the dominant end is
///   factored out first, `T(e) = u^e * (1 - u^(-e)) / (-e)`, giving `ln T = e*ln(u) + ln(1 - u^(-e)) - ln(-e)`
///   with every term representable however small `u` is.
/// - `e = 0`, the DEGENERATE exponent (`p = 4` in the numerator, `p = 1` in the denominator): the power rule has
///   no antiderivative there and the integral is a logarithm instead, `int D^(-1) dD = ln(max_size/min_size)`, so
///   `ln T = ln(-ln u)`. It is also the continuous limit of both branches above, so the three agree where they
///   meet and a slope approaching 1 or 4 does not jump.
///
/// CONVERGENCE, stated precisely because it is NOT the condition the two fraction helpers above guard: over a
/// BOUNDED range with `min_size > 0` both integrals are finite for EVERY slope, so this helper does not refuse on
/// the slope, and a cascade steeper than 4 (the mean cube then sitting just above `min_size`) is a valid data row.
/// The divergences that gate [`number_fraction_above_size`] at `p <= 1` and [`mass_fraction_above_size`] at
/// `p >= 4` belong to the UNBOUNDED idealizations, `min_size -> 0` and `max_size -> infinity`; the first is
/// reached here as `min_size <= 0`, which refuses.
///
/// `None` on non-physical or disordered sizes (a non-positive bound, or `min_size` above `max_size`), on a size
/// ratio the fixed-point grid cannot resolve (`u` underflowing to zero past about nine decades of range, or `u`
/// so near one that `1 - u^e` collapses to nothing), or on a product past the representable range. An EXACTLY
/// degenerate reservoir (`min_size == max_size`, a swarm of identical bodies) is not a refusal: every body is
/// `max_size`, so the mean cube ratio is exactly one and its log exactly zero.
pub fn ln_mean_cube_size_ratio(
    min_size: Fixed,
    max_size: Fixed,
    differential_slope: Fixed,
) -> Option<Fixed> {
    if min_size <= Fixed::ZERO || max_size <= Fixed::ZERO || min_size > max_size {
        return None;
    }
    if min_size == max_size {
        // A single-size reservoir: every body is `max_size`, so the mean cube ratio is exactly one.
        return Some(Fixed::ZERO);
    }
    let u = min_size.checked_div(max_size)?;
    if u <= Fixed::ZERO {
        return None; // the ratio underflowed the fixed-point grid: the size range is too wide to stage
    }
    let ln_u = u.ln();
    if ln_u >= Fixed::ZERO {
        return None; // `u` rounded to one: the two bounds are within a fixed-point step of each other
    }
    // The number-weighted mean of `D^3` is `int D^(3-p) dD / int D^(-p) dD`, so the two exponents are `4 - p`
    // (the numerator's `D^(e-1)` form) and `1 - p`.
    let numerator =
        ln_power_law_integral(ln_u, Fixed::from_int(4).checked_sub(differential_slope)?)?;
    let denominator = ln_power_law_integral(ln_u, Fixed::ONE.checked_sub(differential_slope)?)?;
    numerator.checked_sub(denominator)
}

/// `ln T(e)` for `T(e) = (1 - u^e)/e` (and `T(0) = -ln u`), the log of the power-law integral
/// `int_{min_size}^{max_size} D^(e-1) dD` divided by `max_size^e`, taken from `ln u` alone so the absolute sizes
/// never enter. The three branches and their staging are documented on [`ln_mean_cube_size_ratio`]. Requires
/// `ln_u < 0` (a strictly ordered range), which the caller guarantees. `None` if a product leaves the
/// representable range or if a `1 - u^e` term collapses to nothing on the fixed-point grid.
fn ln_power_law_integral(ln_u: Fixed, exponent: Fixed) -> Option<Fixed> {
    // `u^e` as `exp(e * ln u)` from the log already taken. The product is CHECKED because `Fixed::powf` forms the
    // same product with the wrapping `mul`, which would turn an extreme exponent into a plausible wrong answer
    // rather than a refusal. Both call sites below pass a POSITIVE exponent against a negative `ln u`, so the
    // argument is always negative and `exp` can only underflow toward zero, never saturate at the maximum; an
    // underflow to zero is the correct limit (`u^e` vanishing leaves `1 - u^e = 1`).
    let power = |e: Fixed| -> Option<Fixed> { Some(e.checked_mul(ln_u)?.exp()) };
    if exponent == Fixed::ZERO {
        // The degenerate exponent: the integral is `ln(max_size/min_size) = -ln u`, positive for an ordered range.
        return Some(Fixed::ZERO.checked_sub(ln_u)?.ln());
    }
    if exponent > Fixed::ZERO {
        // The large end carries the integral; `u^e` is in `(0, 1)` and needs no factoring.
        let head = Fixed::ONE.checked_sub(power(exponent)?)?;
        if head <= Fixed::ZERO {
            return None;
        }
        return head.ln().checked_sub(exponent.ln());
    }
    // The small end carries the integral: factor the dominant `u^e` out so nothing large is ever formed.
    let flipped = Fixed::ZERO.checked_sub(exponent)?;
    let head = Fixed::ONE.checked_sub(power(flipped)?)?;
    if head <= Fixed::ZERO {
        return None;
    }
    exponent
        .checked_mul(ln_u)?
        .checked_add(head.ln())?
        .checked_sub(flipped.ln())
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
    let u_e = unsaturated_powf(u, exponent)?;
    let umin_e = unsaturated_powf(umin, exponent)?;
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
        let slope = Fixed::from_ratio(38, 10);
        let n = number_fraction_above_size(
            Fixed::from_int(5000),
            Fixed::from_int(50),
            Fixed::from_int(50_000),
            slope,
        )
        .expect("an alien reservoir resolves");
        assert!(n > Fixed::ZERO && n < Fixed::ONE, "a finite size fraction");
        // The ratio staging has a WIDTH limit, and it is tighter than it looks: the fraction needs `u^(1-p)`
        // inside the fixed-point range, which holds while `(p - 1) * ln(max/min)` stays under `exp`'s window of
        // about 22, so a p = 3.8 cascade can span about 3.4 decades of size and no more. This alien reservoir at
        // four decades is past it, and it REFUSES rather than answering from a railed power (it previously
        // answered, wrongly). The reservoir is still a data row: the quantity a body count needs, the
        // number-weighted mean cube, has a log-space form that carries this range and every wider one.
        assert!(
            number_fraction_above_size(
                Fixed::from_int(5000),
                Fixed::from_int(50),
                Fixed::from_int(500_000),
                slope
            )
            .is_none(),
            "four decades of size is past the linear ratio staging, so it refuses"
        );
        assert!(
            ln_mean_cube_size_ratio(Fixed::from_int(50), Fixed::from_int(500_000), slope).is_some(),
            "the log-space mean carries the same four-decade alien reservoir"
        );
    }

    #[test]
    fn the_size_draw_inverts_the_survival_function() {
        // size_at_number_fraction is the inverse of number_fraction_above_size: drawing the size at a target
        // fraction and asking its fraction-above recovers the target. The endpoints are exact (f=0 -> max, f=1
        // -> min); interior fractions round-trip within the fixed-point and powf tolerance.
        let max =
            size_at_number_fraction(Fixed::ZERO, dmin(), dmax(), slope()).expect("f=0 resolves");
        let min =
            size_at_number_fraction(Fixed::ONE, dmin(), dmax(), slope()).expect("f=1 resolves");
        assert!(
            (max.to_f64_lossy() - dmax().to_f64_lossy()).abs() / dmax().to_f64_lossy() < 1e-6,
            "the fraction-above-zero size is the largest body"
        );
        assert!(
            (min.to_f64_lossy() - dmin().to_f64_lossy()).abs() / dmin().to_f64_lossy() < 1e-3,
            "the fraction-above-one size is the smallest body, got {}",
            min.to_f64_lossy()
        );
        for &(num, den) in &[(1i64, 10i64), (1, 4), (1, 2), (3, 4), (9, 10)] {
            let f = Fixed::from_ratio(num, den);
            let size = size_at_number_fraction(f, dmin(), dmax(), slope()).expect("draws a size");
            let back =
                number_fraction_above_size(size, dmin(), dmax(), slope()).expect("its fraction");
            assert!(
                (back.to_f64_lossy() - f.to_f64_lossy()).abs() < 0.01,
                "round-trip f={} recovered {} (size {})",
                f.to_f64_lossy(),
                back.to_f64_lossy(),
                size.to_f64_lossy()
            );
        }
    }

    #[test]
    fn a_uniform_draw_over_the_survival_function_yields_mostly_small_bodies() {
        // Inverse-transform sampling: uniform fractions map to sizes concentrated near the small end (the
        // collisional-cascade swarm), so the median drawn size is far below the geometric mean of the range.
        let geo_mean = 3162.0; // sqrt(100 * 100000)
        let mut below = 0;
        let n = 21;
        for k in 1..n {
            let f = Fixed::from_ratio(k as i64, n as i64);
            let size = size_at_number_fraction(f, dmin(), dmax(), slope()).expect("draws");
            if size.to_f64_lossy() < geo_mean {
                below += 1;
            }
        }
        assert!(
            below > (n - 1) * 3 / 4,
            "most uniform draws land below the geometric mean (small bodies dominate), got {below}/{}",
            n - 1
        );
    }

    #[test]
    fn the_size_draw_fails_soft_on_non_physical_inputs() {
        // A slope not above one, a fraction outside [0,1], and a degenerate single-size reservoir each refuse.
        assert!(size_at_number_fraction(
            Fixed::from_ratio(1, 2),
            dmin(),
            dmax(),
            Fixed::from_ratio(5, 10)
        )
        .is_none());
        assert!(size_at_number_fraction(Fixed::from_int(2), dmin(), dmax(), slope()).is_none());
        assert!(size_at_number_fraction(Fixed::from_int(-1), dmin(), dmax(), slope()).is_none());
        assert!(
            size_at_number_fraction(Fixed::from_ratio(1, 2), dmin(), dmin(), slope()).is_none()
        );
    }

    /// The number-weighted mean cube by direct QUADRATURE over the power law, an independent arithmetic path
    /// that never touches the closed form under test: a Riemann sum in log-size (so a nine-decade range costs no
    /// more than a one-decade one), with `dD = D dx` under the substitution `D = exp(x)`. Returns the mean cube
    /// as a ratio to `max_size^3`, taking `u = min_size/max_size` and the slope, the same two numbers the
    /// analytic form reads.
    fn mean_cube_by_quadrature(u: f64, p: f64) -> f64 {
        let (lo, hi, n) = (u.ln(), 0.0_f64, 200_000);
        let h = (hi - lo) / n as f64;
        let (mut num, mut den) = (0.0_f64, 0.0_f64);
        for i in 0..n {
            let d = (lo + h * (i as f64 + 0.5)).exp();
            let weight = d.powf(-p) * d * h;
            den += weight;
            num += weight * d * d * d;
        }
        num / den
    }

    #[test]
    fn the_mean_cube_matches_an_independent_quadrature() {
        // The numerical twin for the analytic mean: four size ranges and slopes spanning both signs of both
        // exponents, each against the quadrature above. This validates the closed form and its branch staging;
        // the Dohnanyi citation and the end-dominance test below carry the physics.
        for &(num, den, slope_num, slope_den) in &[
            (1i64, 1000i64, 35i64, 10i64), // the Dohnanyi cascade over three decades
            (1, 100, 20, 10), // a shallow slope: both exponents change sign against the case above
            (1, 1000, 50, 10), // steeper than four: the mean cube collapses onto the small end
            (1, 4, 35, 10),   // a narrow range: the whole reservoir within a factor of four
        ] {
            let (u, p) = (num as f64 / den as f64, slope_num as f64 / slope_den as f64);
            let reference = mean_cube_by_quadrature(u, p).ln();
            let got = ln_mean_cube_size_ratio(
                Fixed::from_ratio(num, den),
                Fixed::ONE,
                Fixed::from_ratio(slope_num, slope_den),
            )
            .expect("the mean cube resolves")
            .to_f64_lossy();
            assert!(
                (got - reference).abs() < 1e-3,
                "ln mean cube for u={u} p={p}: got {got}, quadrature twin {reference}"
            );
        }
    }

    #[test]
    fn the_degenerate_exponents_take_the_logarithmic_closed_form() {
        // The two slopes where the power rule has no antiderivative and the integral becomes a logarithm:
        // p = 4 zeroes the numerator's exponent and p = 1 zeroes the denominator's. Both are evaluated against
        // the same independent quadrature, so the degenerate branch is proven rather than asserted.
        for &(slope, u) in &[(4i64, 1e-3_f64), (1, 1e-3)] {
            let reference = mean_cube_by_quadrature(u, slope as f64).ln();
            let got = ln_mean_cube_size_ratio(
                Fixed::from_ratio(1, 1000),
                Fixed::ONE,
                Fixed::from_int(slope as i32),
            )
            .expect("the degenerate exponent resolves")
            .to_f64_lossy();
            assert!(
                (got - reference).abs() < 1e-3,
                "the p={slope} logarithmic form: got {got}, quadrature twin {reference}"
            );
        }
        // The branches JOIN: a slope a thousandth away from each degenerate value lands within a thousandth of
        // it, so a cascade near 1 or 4 does not jump between the logarithmic and the power branch.
        for &(exact, near) in &[(4000i64, 3999i64), (1000, 1001)] {
            let a = ln_mean_cube_size_ratio(
                Fixed::from_ratio(1, 1000),
                Fixed::ONE,
                Fixed::from_ratio(exact, 1000),
            )
            .expect("resolves")
            .to_f64_lossy();
            let b = ln_mean_cube_size_ratio(
                Fixed::from_ratio(1, 1000),
                Fixed::ONE,
                Fixed::from_ratio(near, 1000),
            )
            .expect("resolves")
            .to_f64_lossy();
            assert!(
                (a - b).abs() < 1e-2,
                "the degenerate branch joins its neighbour: p={} gives {a}, p={} gives {b}",
                exact as f64 / 1000.0,
                near as f64 / 1000.0
            );
        }
    }

    #[test]
    fn the_slope_decides_which_end_of_the_range_carries_the_mass() {
        // The physical content of the exponent's sign. Read as a diameter, the mean cube is
        // `Dbar = max_size * exp(ln_mean_cube / 3)`. A cascade steeper than four puts the mass at the SMALL end,
        // so `Dbar` sits just above `min_size`; a shallow cascade puts it at the LARGE end, so `Dbar` climbs far
        // above `min_size`. Over a three-decade range (`min_size` a thousandth of `max_size`) the computed
        // multiples of `min_size` are ~1.6 at p=5, ~5.4 at p=3.5, and ~79 at p=2, monotone in the slope.
        let mut previous = 0.0_f64;
        for (slope, expected_multiple) in [(50i64, 1.585_f64), (35, 5.353), (20, 79.43)] {
            let ln_mean = ln_mean_cube_size_ratio(
                Fixed::from_ratio(1, 1000),
                Fixed::ONE,
                Fixed::from_ratio(slope, 10),
            )
            .expect("resolves")
            .to_f64_lossy();
            // `Dbar` in units of `min_size`, which is a thousandth of `max_size`.
            let multiple = (ln_mean / 3.0).exp() * 1000.0;
            assert!(
                (multiple - expected_multiple).abs() / expected_multiple < 0.01,
                "at p={} the mean-cube diameter is {expected_multiple} times the smallest body, got {multiple}",
                slope as f64 / 10.0
            );
            assert!(
                multiple > previous,
                "walking the slope down from 5 to 2 moves the mean cube off the small end toward the large"
            );
            previous = multiple;
        }
    }

    #[test]
    fn the_log_form_resolves_a_range_that_rails_the_linear_fractions() {
        // The reason the mean is exported in LOG space, and the reason the ratio powers are now guarded. Over six
        // decades of size at the Dohnanyi slope the ratio power `u^(1-p)` is ~1e15 against a fixed-point ceiling
        // of ~2.1e9, so the linear cumulative fraction cannot hold the range: before the saturation guard it
        // answered 1.5e-2 at a probe size where the true fraction is 3.2e-8, five orders of magnitude high and
        // with no signal that anything had failed. It now refuses. The log form carries the same reservoir and
        // still matches the independent quadrature.
        let (umin, slope) = (Fixed::from_ratio(1, 1_000_000), Fixed::from_ratio(35, 10));
        assert!(
            number_fraction_above_size(Fixed::from_ratio(1, 1000), umin, Fixed::ONE, slope)
                .is_none(),
            "a six-decade size range refuses rather than returning a railed fraction"
        );
        let ln_mean = ln_mean_cube_size_ratio(umin, Fixed::ONE, slope)
            .expect("the log form resolves the same range")
            .to_f64_lossy();
        let reference = mean_cube_by_quadrature(1e-6, 3.5).ln();
        assert!(
            (ln_mean - reference).abs() < 1e-3,
            "the log form over six decades: got {ln_mean}, quadrature twin {reference}"
        );
    }

    #[test]
    fn the_mean_cube_refuses_the_non_physical_and_answers_the_degenerate() {
        let slope = slope();
        // Non-physical or disordered bounds have no distribution to average over.
        assert!(ln_mean_cube_size_ratio(Fixed::ZERO, Fixed::ONE, slope).is_none());
        assert!(ln_mean_cube_size_ratio(Fixed::from_int(-1), Fixed::ONE, slope).is_none());
        assert!(ln_mean_cube_size_ratio(Fixed::ONE, Fixed::ZERO, slope).is_none());
        assert!(ln_mean_cube_size_ratio(Fixed::from_int(2), Fixed::ONE, slope).is_none());
        // A size ratio under the fixed-point grid (twelve decades of range) cannot be staged.
        assert!(
            ln_mean_cube_size_ratio(Fixed::EPSILON, Fixed::from_int(1000), slope).is_none(),
            "a size ratio that underflows the grid refuses rather than returning a number"
        );
        // A single-size reservoir is NOT a refusal: every body is the largest, so the mean cube ratio is one.
        assert_eq!(
            ln_mean_cube_size_ratio(dmax(), dmax(), slope),
            Some(Fixed::ZERO),
            "a swarm of identical bodies has a mean cube ratio of exactly one"
        );
        // Over a bounded range the integrals converge at EVERY slope, so a slope the cumulative fractions refuse
        // still yields a mean: this helper does not inherit their unbounded-idealization guards.
        assert!(
            number_fraction_above_size(dmin(), dmin(), dmax(), Fixed::from_ratio(5, 10)).is_none()
                && ln_mean_cube_size_ratio(dmin(), dmax(), Fixed::from_ratio(5, 10)).is_some(),
            "a slope below one has no cumulative fraction but does have a bounded-range mean"
        );
        assert!(
            mass_fraction_above_size(dmin(), dmin(), dmax(), Fixed::from_int(4)).is_none()
                && ln_mean_cube_size_ratio(dmin(), dmax(), Fixed::from_int(4)).is_some(),
            "a slope at four has no cumulative mass fraction but does have a bounded-range mean"
        );
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
