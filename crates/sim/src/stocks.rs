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

//! Stocks and flows, the ecological substrate (design Part 15).
//!
//! A [`Stock`] is a compartment holding a fixed-point quantity (vegetation biomass, game
//! density, standing water, a population count) that regenerates logistically toward a
//! carrying capacity and collapses when sustained draw exceeds regeneration. A [`flow`]
//! moves quantity between stocks at an ecological transfer efficiency, the trophic step the
//! food web is built from. This module is the substrate under Parts 16 and 17: the species
//! and the food web are coupled sets of stocks, and the generate-and-validate seeder's
//! hybrid closure lets these stock dynamics cull non-viable pools over the early ticks
//! rather than rejecting them at seed time.
//!
//! Everything here is closed-form Q32.32 integer arithmetic with the products formed to
//! avoid a large intermediate, clamped to the physical range, and drawing no randomness, so
//! a stock's trajectory is a pure function of its parameters and its draws and reproduces
//! bit for bit on any machine (Principle 3). The regeneration rate, the carrying capacity,
//! and the transfer efficiency are per-stock and per-edge data (Principle 11), reserved
//! with their basis and never baked into the kernel: the mechanism is fixed, the numbers
//! are the world's.

use civsim_core::Fixed;

/// One ecological compartment: a quantity that regenerates logistically toward a carrying
/// capacity and is reduced by draw. The amount is held in `[0, capacity]` at all times, so
/// the chaotic overshoot regime of the discrete logistic map never arises and the
/// trajectory stays a stable, reproducible function of its parameters.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Stock {
    amount: Fixed,
    capacity: Fixed,
    regen_rate: Fixed,
}

impl Stock {
    /// A stock holding `amount` toward `capacity`, regenerating at `regen_rate` per step.
    /// The amount is clamped into `[0, capacity]` and a negative capacity is treated as
    /// zero, so a stock is always physical however it is constructed.
    pub fn new(amount: Fixed, capacity: Fixed, regen_rate: Fixed) -> Stock {
        let capacity = capacity.clamp(Fixed::ZERO, Fixed::MAX);
        Stock {
            amount: amount.clamp(Fixed::ZERO, capacity),
            capacity,
            regen_rate,
        }
    }

    /// The current quantity in the compartment.
    #[inline]
    pub fn amount(&self) -> Fixed {
        self.amount
    }

    /// The carrying capacity.
    #[inline]
    pub fn capacity(&self) -> Fixed {
        self.capacity
    }

    /// The per-step regeneration rate.
    #[inline]
    pub fn regen_rate(&self) -> Fixed {
        self.regen_rate
    }

    /// Whether the compartment has collapsed to empty.
    #[inline]
    pub fn is_collapsed(&self) -> bool {
        self.amount <= Fixed::ZERO
    }

    /// The fraction of capacity currently filled, in `[0, ONE]`: `amount / capacity`, with an
    /// empty or non-positive capacity reading as zero. This is the canonical normaliser a
    /// consumer reads a stock through (for example a soil-fertility field feeding the
    /// biome-fit law), so the whole `[0, capacity]` range maps to `[0, ONE]` deterministically
    /// with a single guarded divide.
    pub fn occupancy(&self) -> Fixed {
        if self.capacity <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        // amount is held in [0, capacity], so the quotient is in [0, ONE] and the divide
        // (guarded against a zero denominator above) cannot overflow.
        self.amount
            .checked_div(self.capacity)
            .unwrap_or(Fixed::ZERO)
    }

    /// The logistic regeneration increment for one step, `r * amount * (1 - amount/capacity)`,
    /// formed occupancy-first so no intermediate exceeds the amount. It is zero at capacity
    /// (the gap closes) and zero at empty (nothing to grow from), so a collapsed stock does
    /// not spontaneously revive.
    pub fn regen_increment(&self) -> Fixed {
        if self.capacity <= Fixed::ZERO || self.amount <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        // ratio in [0, 1] because amount is held in [0, capacity]; gap in [0, 1].
        let ratio = match self.amount.checked_div(self.capacity) {
            Some(r) => r,
            None => return Fixed::ZERO,
        };
        let gap = Fixed::ONE - ratio;
        // Form amount*gap first: it never exceeds amount, so the intermediate is always
        // representable; only then scale by the rate. This is the wave-0/wave-1 lesson,
        // the quotient (and the bounded factor) before the product that could overflow.
        let occupied_growth = match self.amount.checked_mul(gap) {
            Some(v) => v,
            None => return Fixed::ZERO,
        };
        self.regen_rate
            .checked_mul(occupied_growth)
            .unwrap_or(Fixed::ZERO)
    }

    /// Advance one step: regenerate toward capacity, then apply `draw` (a negative draw is
    /// treated as zero). The amount stays in `[0, capacity]`; sustained draw above the
    /// regeneration drives it to collapse, which is the Part 15 over-harvest feedback.
    pub fn step(&mut self, draw: Fixed) {
        let after_regen = self
            .amount
            .saturating_add(self.regen_increment())
            .clamp(Fixed::ZERO, self.capacity);
        let drawn = if draw < Fixed::ZERO {
            Fixed::ZERO
        } else {
            draw
        };
        // after_regen and drawn are both in [0, capacity], so the difference cannot overflow.
        self.amount = (after_regen - drawn).clamp(Fixed::ZERO, self.capacity);
    }

    /// Set the carrying capacity (for example when a biome shifts), re-clamping the amount
    /// so it never exceeds the new ceiling.
    pub fn set_capacity(&mut self, capacity: Fixed) {
        self.capacity = capacity.clamp(Fixed::ZERO, Fixed::MAX);
        self.amount = self.amount.clamp(Fixed::ZERO, self.capacity);
    }

    /// Remove up to `want` from the stock, returning what was actually removed (never more
    /// than the amount present, never negative). This is the primitive a consumer's draw and
    /// a [`flow`] are built on.
    pub fn take(&mut self, want: Fixed) -> Fixed {
        let want = want.clamp(Fixed::ZERO, self.amount);
        self.amount -= want;
        want
    }

    /// Add `add` to the stock, capped at capacity, returning what was actually stored (the
    /// remainder is overflow that the compartment cannot hold).
    pub fn deposit(&mut self, add: Fixed) -> Fixed {
        let before = self.amount;
        self.amount = self
            .amount
            .saturating_add(add.clamp(Fixed::ZERO, Fixed::MAX))
            .clamp(Fixed::ZERO, self.capacity);
        self.amount - before
    }
}

/// The exact accounting of a [`flow`]: what left the source, what reached the destination,
/// and what was lost. `moved` always equals `delivered + lost`, so nothing is silently
/// dropped and the transfer is conservation-honest (design Part 58, R-PROJ-REGISTER).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FlowResult {
    /// The quantity removed from the source.
    pub moved: Fixed,
    /// The quantity that reached the destination.
    pub delivered: Fixed,
    /// The quantity lost in transfer (ecological inefficiency plus any destination overflow).
    pub lost: Fixed,
}

/// Move up to `requested` from `from` to `to` at an `efficiency` in `[0, 1]`, the trophic
/// step: a fraction of the biomass a consumer draws becomes consumer biomass and the rest is
/// respiration loss (the ecological-efficiency rule). The amount leaving `from` is exact and
/// the amount reaching `to` is capped at its capacity; the returned [`FlowResult`] accounts
/// for every unit so the pair conserves mass.
pub fn flow(from: &mut Stock, to: &mut Stock, requested: Fixed, efficiency: Fixed) -> FlowResult {
    let moved = from.take(requested);
    let efficiency = efficiency.clamp(Fixed::ZERO, Fixed::ONE);
    // moved and efficiency are both in [0, .] with efficiency <= 1, so the product is at
    // most `moved` and cannot overflow.
    let offered = moved.checked_mul(efficiency).unwrap_or(Fixed::ZERO);
    let delivered = to.deposit(offered);
    FlowResult {
        moved,
        delivered,
        lost: moved - delivered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    // from_ratio truncates, so an exact-arithmetic result can differ from a freshly
    // constructed ratio by a few epsilons; compare within a tolerance far above the Q32.32
    // resolution but far below any quantity the test cares about.
    fn approx(a: Fixed, b: Fixed) -> bool {
        (a - b).abs() <= Fixed::from_ratio(1, 100_000)
    }

    #[test]
    fn logistic_growth_climbs_toward_capacity_and_stops() {
        let mut s = Stock::new(f(1, 100), Fixed::ONE, f(3, 10));
        let mut last = s.amount();
        for _ in 0..200 {
            s.step(Fixed::ZERO);
            assert!(s.amount() <= s.capacity(), "never exceeds capacity");
            assert!(s.amount() >= last, "monotone increase with no draw");
            last = s.amount();
        }
        // Converges to capacity within a small tolerance.
        assert!(
            s.capacity() - s.amount() < f(1, 100),
            "settles at carrying capacity"
        );
    }

    #[test]
    fn a_full_stock_is_stable() {
        let mut s = Stock::new(Fixed::ONE, Fixed::ONE, f(5, 10));
        s.step(Fixed::ZERO);
        assert_eq!(s.amount(), Fixed::ONE, "at capacity the increment is zero");
    }

    #[test]
    fn an_empty_stock_does_not_revive() {
        let mut s = Stock::new(Fixed::ZERO, Fixed::ONE, f(5, 10));
        s.step(Fixed::ZERO);
        assert!(s.is_collapsed(), "nothing to grow from");
    }

    #[test]
    fn sustained_overdraw_collapses_the_stock() {
        let mut s = Stock::new(Fixed::ONE, Fixed::ONE, f(2, 10));
        // Draw a fifth of capacity every step, above the regeneration near the ceiling.
        for _ in 0..100 {
            s.step(f(2, 10));
        }
        assert!(s.is_collapsed(), "sustained over-harvest drives collapse");
    }

    #[test]
    fn a_light_draw_settles_below_capacity() {
        let mut s = Stock::new(Fixed::ONE, Fixed::ONE, f(3, 10));
        for _ in 0..500 {
            s.step(f(2, 100));
        }
        assert!(!s.is_collapsed(), "a sustainable draw does not collapse");
        assert!(
            s.amount() < s.capacity(),
            "a standing draw holds it below capacity"
        );
    }

    #[test]
    fn the_trajectory_is_deterministic() {
        let run = || {
            let mut s = Stock::new(f(1, 10), Fixed::ONE, f(4, 10));
            let mut trace = Vec::new();
            for i in 0..50 {
                s.step(f(i % 3, 100));
                trace.push(s.amount().to_bits());
            }
            trace
        };
        assert_eq!(run(), run(), "same parameters and draws, same trajectory");
    }

    #[test]
    fn flow_conserves_and_accounts() {
        let mut prey = Stock::new(Fixed::ONE, Fixed::ONE, f(3, 10));
        let mut pred = Stock::new(f(1, 10), Fixed::ONE, f(1, 10));
        let r = flow(&mut prey, &mut pred, f(3, 10), f(1, 10));
        assert_eq!(
            r.moved,
            f(3, 10),
            "the requested draw was available and removed"
        );
        assert_eq!(r.moved, r.delivered + r.lost, "every unit is accounted for");
        // A tenth efficiency delivers a tenth of what moved.
        assert!(approx(r.delivered, f(3, 100)));
        assert!(
            approx(prey.amount(), f(7, 10)),
            "the prey lost exactly what moved"
        );
    }

    #[test]
    fn flow_cannot_move_more_than_is_present() {
        let mut from = Stock::new(f(1, 10), Fixed::ONE, Fixed::ZERO);
        let mut to = Stock::new(Fixed::ZERO, Fixed::ONE, Fixed::ZERO);
        let r = flow(&mut from, &mut to, Fixed::ONE, Fixed::ONE);
        assert_eq!(r.moved, f(1, 10), "capped at what the source holds");
        assert!(from.is_collapsed());
    }

    #[test]
    fn occupancy_normalises_amount_over_capacity() {
        assert_eq!(
            Stock::new(Fixed::ZERO, Fixed::ONE, Fixed::ZERO).occupancy(),
            Fixed::ZERO
        );
        assert_eq!(
            Stock::new(Fixed::ONE, Fixed::ONE, Fixed::ZERO).occupancy(),
            Fixed::ONE
        );
        assert!(approx(
            Stock::new(f(1, 2), Fixed::ONE, Fixed::ZERO).occupancy(),
            f(1, 2)
        ));
        // A wide capacity still normalises into [0, ONE].
        let s = Stock::new(f(30, 1), Fixed::from_int(100), Fixed::ZERO);
        assert!(approx(s.occupancy(), f(3, 10)), "30 of 100 reads as 0.3");
        // A zero capacity reads as empty, not a divide by zero.
        assert_eq!(
            Stock::new(Fixed::ZERO, Fixed::ZERO, Fixed::ZERO).occupancy(),
            Fixed::ZERO
        );
    }

    #[test]
    fn deposit_is_capped_at_capacity() {
        let mut s = Stock::new(f(9, 10), Fixed::ONE, Fixed::ZERO);
        let stored = s.deposit(f(5, 10));
        assert!(
            approx(stored, f(1, 10)),
            "only the room to capacity is stored"
        );
        assert_eq!(s.amount(), Fixed::ONE);
    }
}
