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

//! The type-level divide/log backstop (R-UNITS-PIN slice 3). The slice-2 planner ([`crate::plan`]) enforces
//! the floor invariant on a law that EXPOSES its op graph as a [`crate::plan::LawExpr`]; the walk sees every
//! `Div` and `Ln` node and cannot miss one. A HAND-THREADED law (one that needs control flow, a table lookup,
//! or a non-arithmetic step the op-graph DSL does not model, so it stays raw Rust: the guarded/clamped laws
//! named as honest limits, `nernst_emf`, the reversible-uptake flux, the contact family, and any stray
//! `checked_div`/`ln` on the live path) exposes no graph. This module is the backstop that keeps coverage
//! complete where the graph is not exposed: a hand-threaded law divides and takes logs through
//! [`guarded_div`] and [`guarded_ln`], and the floor DECLARATION ([`ZeroGuard`]) is a MANDATORY argument, so a
//! divide or a log cannot be written on the live path without stating how its zero-boundary is bounded. The
//! type forces the declaration; the runtime applies it. This is the observer-independence guard (Principle
//! 10) at the call site: without it a near-zero divisor or log argument is bounded only by the storage
//! epsilon, and once Tier 2 lowers that epsilon the bound becomes a representation artifact rather than a
//! physical fact.
//!
//! The declaration mirrors the planner's [`crate::plan::ZeroBoundary`], carrying the VALUE the planner only
//! needed to know existed: `ZeroBoundary::RequiresFloor` corresponds to a [`ZeroGuard::Floor`] whose value is
//! the quantity's declared `physical_floor` (per-world data), and `ZeroBoundary::PhysicalLimitAtZero`
//! corresponds to a [`ZeroGuard::LimitAtZero`] whose value is the limit the law node takes at zero (the
//! point-load pressure cap, a Nernst depleted-activity zero). The floor and limit VALUES are per-world data
//! (reserved-with-basis); the mechanism is fixed Rust.
//!
//! Byte-neutral: this is new machinery, called by no law yet. Each hand-threaded law's LIFT to the guarded
//! ops is its own later slice with its stated re-pin (the same staging as the `LawExpr` lifts).

use civsim_core::Fixed;

/// How a divisor's or a log argument's zero-boundary is bounded, supplied at every guarded call. The variant
/// carries the per-world data value; the enforcement is the fixed mechanism.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZeroGuard {
    /// The operand carries a declared physical floor (a strictly positive magnitude below which it is treated
    /// as being AT the floor, so its bound is the floor rather than the storage epsilon). The guarded op
    /// clamps the operand's magnitude up to this floor before dividing or taking the log. A non-positive
    /// floor is a mis-declaration and fails loud.
    Floor(Fixed),
    /// The operand's zero-boundary is a declared intentional physical limit: at or below the zero-boundary the
    /// operation returns this limit value instead of dividing or taking a log (the contact point-load
    /// returning the pressure cap at zero area, a Nernst depleted-activity zero).
    LimitAtZero(Fixed),
}

/// Divide `num` by `den` under a mandatory zero-boundary declaration, so a hand-threaded law's divide carries
/// the same floor invariant the planner enforces on a `LawExpr`. Under [`ZeroGuard::Floor`] the divisor's
/// magnitude is clamped up to the declared floor (never below it, never zero), so the quotient is bounded by
/// the physical floor rather than the storage epsilon. Under [`ZeroGuard::LimitAtZero`] a non-positive divisor
/// (the zero-boundary reached) returns the declared limit; otherwise the divide is exact. Deterministic and
/// float-free.
#[inline]
pub fn guarded_div(num: Fixed, den: Fixed, guard: ZeroGuard) -> Fixed {
    match guard {
        ZeroGuard::Floor(floor) => num.div(clamp_to_floor(den, floor)),
        ZeroGuard::LimitAtZero(limit) => {
            if den <= Fixed::ZERO {
                limit
            } else {
                num.div(den)
            }
        }
    }
}

/// The natural logarithm of `arg` under a mandatory zero-boundary declaration, so a hand-threaded law's log
/// carries the floor invariant. Under [`ZeroGuard::Floor`] the argument is clamped up to the declared floor
/// before the log (`ln` never sees a non-positive argument riding the epsilon, so it never returns its
/// [`Fixed::MIN`] sentinel from an underflow). Under [`ZeroGuard::LimitAtZero`] a non-positive argument
/// returns the declared limit; otherwise the log is exact. Deterministic and float-free.
#[inline]
pub fn guarded_ln(arg: Fixed, guard: ZeroGuard) -> Fixed {
    match guard {
        ZeroGuard::Floor(floor) => {
            assert!(
                floor > Fixed::ZERO,
                "guarded_ln: a physical floor must be strictly positive"
            );
            arg.max(floor).ln()
        }
        ZeroGuard::LimitAtZero(limit) => {
            if arg <= Fixed::ZERO {
                limit
            } else {
                arg.ln()
            }
        }
    }
}

/// Clamp a divisor's magnitude up to a strictly positive floor, preserving its sign, so the returned value is
/// never smaller in magnitude than the floor and never zero. Fails loud on a non-positive floor, which is a
/// mis-declaration (a floor is a positive physical bound).
#[inline]
fn clamp_to_floor(den: Fixed, floor: Fixed) -> Fixed {
    assert!(
        floor > Fixed::ZERO,
        "guarded_div: a physical floor must be strictly positive"
    );
    if den >= Fixed::ZERO {
        den.max(floor)
    } else {
        den.min(Fixed::ZERO - floor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(int: i32) -> Fixed {
        Fixed::from_int(int)
    }

    #[test]
    fn floor_passes_an_above_floor_divisor_through_exactly() {
        // A divisor at or above its floor divides exactly, identical to a plain divide.
        let num = f(10);
        let den = f(4);
        let floor = Fixed::from_ratio(1, 100); // 0.01
        assert_eq!(guarded_div(num, den, ZeroGuard::Floor(floor)), num.div(den));
    }

    #[test]
    fn floor_clamps_a_sub_floor_divisor_up_to_the_floor() {
        // A divisor below its floor is clamped up, so the quotient is bounded by the floor, not the epsilon.
        let num = f(1);
        let tiny = Fixed::from_bits(3); // ~7e-10, far below any physical floor
        let floor = Fixed::from_ratio(1, 1000); // 0.001
        let guarded = guarded_div(num, tiny, ZeroGuard::Floor(floor));
        // The result is exactly num / floor (the clamped divisor), NOT the runaway num / tiny.
        assert_eq!(guarded, num.div(floor));
        assert!(
            guarded < num.div(tiny),
            "the clamp bounds the quotient below the epsilon-ridden value"
        );
    }

    #[test]
    fn floor_clamps_a_zero_divisor_rather_than_panicking() {
        // A zero divisor under a declared floor divides by the floor instead of panicking: the physical floor
        // says the quantity is never truly zero.
        let num = f(5);
        let floor = Fixed::from_ratio(1, 2); // 0.5
        assert_eq!(
            guarded_div(num, Fixed::ZERO, ZeroGuard::Floor(floor)),
            num.div(floor)
        );
    }

    #[test]
    fn floor_preserves_the_sign_of_a_negative_divisor() {
        // A negative sub-floor divisor clamps in magnitude and keeps its sign, so the quotient's sign is right.
        let num = f(6);
        let small_neg = Fixed::from_ratio(-1, 1000); // -0.001
        let floor = Fixed::from_ratio(1, 100); // 0.01
        let guarded = guarded_div(num, small_neg, ZeroGuard::Floor(floor));
        assert_eq!(guarded, num.div(Fixed::ZERO - floor));
        assert!(guarded < Fixed::ZERO, "6 / (-0.01) is negative");
    }

    #[test]
    #[should_panic(expected = "strictly positive")]
    fn a_non_positive_floor_declaration_fails_loud() {
        // A floor is a positive physical bound; a zero or negative one is a mis-declaration and must fail loud.
        let _ = guarded_div(f(1), f(2), ZeroGuard::Floor(Fixed::ZERO));
    }

    #[test]
    fn limit_at_zero_returns_the_limit_at_the_boundary_and_divides_above_it() {
        // The declared limit stands at and below the zero-boundary (the point-load cap, the depleted zero);
        // above it the divide is exact.
        let num = f(12);
        let cap = f(999); // the declared physical limit (e.g. a yield-pressure cap)
        assert_eq!(
            guarded_div(num, Fixed::ZERO, ZeroGuard::LimitAtZero(cap)),
            cap
        );
        let neg = Fixed::from_ratio(-1, 10);
        assert_eq!(guarded_div(num, neg, ZeroGuard::LimitAtZero(cap)), cap);
        let pos = f(3);
        assert_eq!(
            guarded_div(num, pos, ZeroGuard::LimitAtZero(cap)),
            num.div(pos)
        );
    }

    #[test]
    fn guarded_ln_floor_never_returns_the_min_sentinel() {
        // A non-positive argument under a declared floor takes the log of the floor, never the ln MIN sentinel.
        let floor = Fixed::from_ratio(1, 100); // 0.01, a positive activity floor
        let at_floor = guarded_ln(Fixed::ZERO, ZeroGuard::Floor(floor));
        assert_eq!(at_floor, floor.ln());
        assert_ne!(
            at_floor,
            Fixed::MIN,
            "the floor guard prevents the non-positive ln sentinel"
        );
        // An argument above the floor logs exactly.
        let x = f(2);
        assert_eq!(guarded_ln(x, ZeroGuard::Floor(floor)), x.ln());
    }

    #[test]
    fn guarded_ln_limit_at_zero_returns_the_declared_limit() {
        // A depleted (non-positive) activity returns its declared limit; a positive one logs exactly.
        let limit = f(-20); // the declared value ln takes at the depleted boundary
        assert_eq!(
            guarded_ln(Fixed::ZERO, ZeroGuard::LimitAtZero(limit)),
            limit
        );
        let x = f(5);
        assert_eq!(guarded_ln(x, ZeroGuard::LimitAtZero(limit)), x.ln());
    }
}
