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

//! Scale-aware raw-bit arithmetic for the wide-range kernels (design Part 55, R-UNITS-PIN, the
//! scale-aware-arithmetic increment). A quantity value is raw `i64` bits at its declared per-quantity
//! fixed-point scale (`quantities::axis_scale`), and a kernel that multiplies and divides quantities
//! on differing scales cannot use the fixed Q32.32 shifts the `Fixed` kernels hardcode: the Coulomb
//! kernel's inline `<< 32` and `>> 32` assume both operands are Q32.32, which fails when charge is
//! (say) Q17.46 and the separation is Q32.32. This carries a value in `i128` with its fractional-bit
//! scale tracked through each multiply and divide, so a kernel sequences its operations exactly as it
//! does today (reduce before grow) but with the shifts computed from the operands' scales rather than
//! hardcoded, rescaling once to the declared output scale at the end.
//!
//! The primitive carries non-negative magnitudes: the wide-range kernels reduce their signed inputs
//! to magnitudes (the Coulomb kernel `sat_abs`es its charges) before entering, and only for a
//! non-negative value do the truncating divide and the flooring rescale below agree; a signed value
//! routed straight through would mix the two roundings, so a kernel keeps the sign aside and computes
//! the magnitude here.
//!
//! The rounding is the pinned canonical one (R-GPU-CANON-PIN, record 62.23): a multiply and the
//! final rescale floor (an arithmetic right shift), a divide truncates toward zero (the `i128`
//! quotient). Because it keeps full precision and rescales once at the end rather than flooring the
//! magnitude after each multiply (the `>> 32` the current `Fixed` kernels do), it matches the exact
//! truncated result, so it is more precise than the current wide-range kernels and not bit-identical
//! to them: routing a kernel through it moves its output toward the exact value. The gap is the low
//! bits the current kernel discards mid-product carried through the remaining multiplies, a relative
//! error below the canonical epsilon (`2^-32`) but an absolute one that grows with the later
//! multiplier magnitude (up to about the coefficient's own size for a kernel like Coulomb), so it is
//! negligible against the value rather than a fixed few ULPs. Every overflow and out-of-contract
//! shift returns `None` so the kernel routes it to the physical extreme, the totality discipline the
//! kernels already follow.

/// A raw fixed-point value carried in `i128` with its fractional-bit scale, so a chain of multiplies
/// and divides across differing per-quantity scales tracks precision exactly. The value it denotes is
/// `bits * 2^-scale`; `scale` is an `i32` because an intermediate product's fractional-bit exponent
/// can climb well past a single quantity's scale before the final rescale brings it back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scaled {
    bits: i128,
    scale: i32,
}

impl Scaled {
    /// A quantity's raw `i64` bits read at its per-quantity fixed-point scale.
    pub fn at(bits: i64, scale_bits: u32) -> Self {
        Scaled {
            bits: bits as i128,
            scale: scale_bits as i32,
        }
    }

    /// The raw `i128` value and its current fractional-bit scale, for a kernel that needs the pair
    /// (a comparison against a threshold, say) mid-chain.
    pub fn parts(self) -> (i128, i32) {
        (self.bits, self.scale)
    }

    /// Multiply: the bits multiply and the fractional-bit exponents add, exactly (no rounding, the
    /// product stays in `i128` until a later rescale reduces it). `None` when the `i128` product
    /// overflows, which for a wide-range kernel is a magnitude past the representable extreme.
    pub fn checked_mul(self, other: Scaled) -> Option<Scaled> {
        Some(Scaled {
            bits: self.bits.checked_mul(other.bits)?,
            scale: self.scale + other.scale,
        })
    }

    /// Divide, shifting the numerator up by `guard` fractional bits first so the integer quotient
    /// keeps `guard` bits of precision below the operand scales (the `<< 32` the Coulomb kernel does
    /// before `/ r`, now a parameter). The quotient truncates toward zero, the canonical divide.
    /// `None` on a zero divisor or when the pre-shift numerator overflows `i128`.
    pub fn checked_div(self, other: Scaled, guard: u32) -> Option<Scaled> {
        // A zero divisor or a guard at or past the i128 shift width is out of contract: route to the
        // extreme (None) rather than panic. The guard bound must precede the shift even when the
        // numerator is zero, since a shift by 128 or more panics regardless of the value shifted.
        if other.bits == 0 || guard >= 128 {
            return None;
        }
        // The `<< guard` must not overflow i128: a magnitude needing more than 127 - guard bits is
        // out of range (the kernel then routes to the extreme). Conservative at the exact negative
        // i128 boundary, which no i64-origin quantity reaches.
        if self.bits != 0 && self.bits.unsigned_abs().leading_zeros() <= guard {
            return None;
        }
        Some(Scaled {
            bits: (self.bits << guard) / other.bits,
            scale: self.scale + guard as i32 - other.scale,
        })
    }

    /// Rescale to `target_scale` and narrow to `i64`, or `None` if the value does not fit `i64`. A
    /// down-scale floors by an arithmetic right shift (the canonical multiply-side rounding); an
    /// up-scale shifts left, reporting out of range rather than overflowing the `i128` intermediate.
    pub fn to_scale(self, target_scale: u32) -> Option<i64> {
        let target = target_scale as i32;
        let out: i128 = if target >= self.scale {
            let shift = (target - self.scale) as u32;
            if self.bits == 0 {
                0
            } else if shift >= 127 || self.bits.unsigned_abs().leading_zeros() <= shift {
                return None;
            } else {
                self.bits << shift
            }
        } else {
            let shift = (self.scale - target) as u32;
            if shift >= 128 {
                // Past the width of i128 the arithmetic shift saturates to the sign, which for a
                // reduced magnitude is zero; report it as zero rather than invoke UB.
                if self.bits < 0 {
                    -1
                } else {
                    0
                }
            } else {
                self.bits >> shift
            }
        };
        if out < i64::MIN as i128 || out > i64::MAX as i128 {
            None
        } else {
            Some(out as i64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_core::Fixed;

    // A reference Coulomb-force magnitude via the scale-aware primitive: F = k*|q1|*|q2|/r^2, the
    // same reduce-before-grow sequence the laws.rs kernel uses (|q1|/r, |q2|/r, product, times k),
    // but with each operand carried at its declared scale. The caller passes charge magnitudes.
    // Returns the raw force bits at `force_scale`, or None on overflow (the route-to-extreme case).
    fn coulomb_via_scaled(
        q1: Scaled,
        q2: Scaled,
        r: Scaled,
        k: Scaled,
        force_scale: u32,
    ) -> Option<i64> {
        let a = q1.checked_div(r, 32)?; // |q1|/r
        let b = q2.checked_div(r, 32)?; // |q2|/r
        let base = a.checked_mul(b)?; // |q1||q2|/r^2
        let force = base.checked_mul(k)?;
        force.to_scale(force_scale)
    }

    #[test]
    fn multiply_adds_scales_and_divide_tracks_precision() {
        // 6.0 (Q32) * 0.5 (Q32) = 3.0, read back at Q32.
        let six = Scaled::at(Fixed::from_int(6).to_bits(), 32);
        let half = Scaled::at(Fixed::from_ratio(1, 2).to_bits(), 32);
        let prod = six.checked_mul(half).unwrap();
        assert_eq!(prod.to_scale(32), Some(Fixed::from_ratio(3, 1).to_bits()));
        // 6.0 / 2.0 = 3.0.
        let two = Scaled::at(Fixed::from_int(2).to_bits(), 32);
        let quo = six.checked_div(two, 32).unwrap();
        assert_eq!(quo.to_scale(32), Some(Fixed::from_int(3).to_bits()));
    }

    #[test]
    fn a_sub_epsilon_operand_keeps_its_magnitude_across_scales() {
        // A charge of 1e-9 C stored on a fine Q4.45 scale (bits = 1e-9 * 2^45) divided by a
        // separation of 2 m on Q32.32 gives 5e-10, representable on a fine output scale though it
        // underflows Q32.32. This is the whole point: the scale travels with the value.
        let q_scale = 45u32;
        let q_bits = ((1i128 << q_scale) / 1_000_000_000) as i64; // ~1e-9 at Q?.45
        let charge = Scaled::at(q_bits, q_scale);
        let r = Scaled::at(Fixed::from_int(2).to_bits(), 32);
        let half_charge = charge.checked_div(r, 32).unwrap();
        // Read back at the same fine scale: ~5e-10.
        let out = half_charge.to_scale(q_scale).unwrap();
        let expected = ((1i128 << q_scale) / 2_000_000_000) as i64;
        // Within a couple of ULPs of 5e-10 at this scale (the guard divide is exact here).
        assert!(
            (out - expected).abs() <= 2,
            "got {out}, expected ~{expected}"
        );
    }

    #[test]
    fn overflow_and_zero_divisor_route_to_none() {
        // A zero divisor is None (the kernel sends it to the extreme).
        let one = Scaled::at(Fixed::ONE.to_bits(), 32);
        let zero = Scaled::at(0, 32);
        assert_eq!(one.checked_div(zero, 32), None);
        // An i128 product overflow is None.
        let huge = Scaled::at(i64::MAX, 0);
        let huge2 = Scaled::at(i64::MAX, 0);
        let p = huge.checked_mul(huge2).unwrap(); // fits i128 (~2^126)
        assert_eq!(p.checked_mul(huge), None, "the third factor overflows i128");
        // An up-scale that exceeds i64 is None.
        assert_eq!(Scaled::at(i64::MAX, 0).to_scale(4), None);
        // A guard at or past the i128 shift width routes to None, not a panic, even when the
        // numerator is zero (the short-circuit that would otherwise skip the shift bound).
        assert_eq!(Scaled::at(0, 32).checked_div(one, 128), None);
        assert_eq!(Scaled::at(5, 32).checked_div(one, 200), None);
    }

    #[test]
    fn a_coulomb_shaped_product_computes_to_its_exact_truncated_value() {
        // The primitive sequenced as a real kernel (F = k*|q1||q2|/r^2) computes the exact truncated
        // result, because it rescales once at the end rather than truncating after each multiply.
        // F = 3 * 2 * 2 / 1^2 = 12.
        let at32 = |v: Fixed| Scaled::at(v.to_bits(), 32);
        let f = coulomb_via_scaled(
            at32(Fixed::from_int(2)),
            at32(Fixed::from_int(2)),
            at32(Fixed::ONE),
            at32(Fixed::from_int(3)),
            32,
        )
        .unwrap();
        assert_eq!(f, Fixed::from_int(12).to_bits());
        // F = 1 * 1 * 1 / 2^2 = 0.25.
        let f = coulomb_via_scaled(
            at32(Fixed::ONE),
            at32(Fixed::ONE),
            at32(Fixed::from_int(2)),
            at32(Fixed::ONE),
            32,
        )
        .unwrap();
        assert_eq!(f, Fixed::from_ratio(1, 4).to_bits());
    }

    #[test]
    fn rescaling_once_is_more_precise_than_the_current_truncate_each_step_kernel() {
        // Honest record of the design choice: the current laws.rs wide-range kernels floor the
        // magnitude after each multiply (`>> 32`), so they carry a small mid-computation truncation
        // error; the scale-aware form keeps full precision and rescales once, matching the exact
        // truncated result, so it sits at or above the kernel. The gap is the discarded low bits
        // carried through the later multiplies, so its absolute size scales with the multiplier (here
        // k = 7 keeps it to a handful of ULPs; a large coefficient widens it, though it stays a
        // relative error below 2^-32). This case has non-zero low bits in the charge product.
        let q1 = Fixed::from_bits((1i64 << 32) + 12345);
        let q2 = Fixed::from_bits((1i64 << 32) + 67890);
        let r = Fixed::from_int(3);
        let k = Fixed::from_int(7);
        let f_max = Fixed::from_int(1_000_000_000);
        let (kernel, _) = crate::laws::coulomb_force(q1, q2, r, k, f_max);
        let at32 = |v: Fixed| Scaled::at(v.to_bits(), 32);
        let scaled = coulomb_via_scaled(at32(q1), at32(q2), at32(r), at32(k), 32).unwrap();
        // The scaled form is at or above the kernel by the low bits the kernel dropped, and the gap
        // is a handful of ULPs, negligible against any physical tolerance.
        assert!(
            scaled >= kernel.to_bits() && scaled - kernel.to_bits() <= 16,
            "scaled {scaled} vs kernel {} should agree within a few ULPs",
            kernel.to_bits()
        );
    }
}
