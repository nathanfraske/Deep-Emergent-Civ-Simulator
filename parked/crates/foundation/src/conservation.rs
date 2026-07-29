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

//! The conserved-projection registry (design Part 58).
//!
//! Promotion and demotion must conserve totals: the count of individuals plus the
//! counts in the aggregate pools must always equal the whole, with no entity
//! created or lost in the transition, or a world develops silent population and
//! resource leaks that read as nondeterminism. What must be conserved is not a fixed
//! list but a registry of conserved projections that each two-tier subsystem
//! declares for itself, so a future two-tier system (a magic-field network, a
//! disease model, a trade-route graph) is covered the moment it registers its own
//! projection, with nothing special-cased to the entries that exist today.
//!
//! The registry is generic over the world type `W`, so any subsystem registers a
//! projection as a function from its own state to an integer total. Totals are
//! integers (counts) or fixed-point bit patterns (stocks, wealth), where addition
//! is exact and associative, so a conserved quantity is conserved exactly rather
//! than within a tolerance.

use std::fmt;

/// A projection that fails to balance across a structural change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConservationError {
    /// The name of the projection that drifted.
    pub projection: String,
    /// The total before the change.
    pub before: i128,
    /// The total after the change.
    pub after: i128,
}

impl fmt::Display for ConservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "conservation violated for '{}': before={}, after={}, leak={}",
            self.projection,
            self.before,
            self.after,
            self.after - self.before
        )
    }
}

impl std::error::Error for ConservationError {}

type Measure<W> = Box<dyn Fn(&W) -> i128 + Send + Sync>;

/// A registry of conserved projections over a world of type `W`.
pub struct ConservationRegistry<W> {
    projections: Vec<(String, Measure<W>)>,
}

impl<W> ConservationRegistry<W> {
    /// An empty registry.
    pub fn new() -> Self {
        ConservationRegistry {
            projections: Vec::new(),
        }
    }

    /// Declare a conserved projection: a name and a function measuring its total
    /// across both tiers of the world.
    pub fn register(&mut self, name: &str, measure: impl Fn(&W) -> i128 + Send + Sync + 'static) {
        self.projections.push((name.to_string(), Box::new(measure)));
    }

    /// The names of the registered projections, in registration order.
    pub fn names(&self) -> Vec<&str> {
        self.projections.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Measure every projection against the current world.
    pub fn snapshot(&self, world: &W) -> Vec<(String, i128)> {
        self.projections
            .iter()
            .map(|(name, measure)| (name.clone(), measure(world)))
            .collect()
    }

    /// Check that every declared projection has the same total in `before` and
    /// `after`. Returns the first projection that drifted, if any.
    pub fn check(&self, before: &W, after: &W) -> Result<(), ConservationError> {
        for (name, measure) in &self.projections {
            let b = measure(before);
            let a = measure(after);
            if a != b {
                return Err(ConservationError {
                    projection: name.clone(),
                    before: b,
                    after: a,
                });
            }
        }
        Ok(())
    }

    /// Check a world against a snapshot taken earlier with [`Self::snapshot`].
    pub fn check_against(
        &self,
        baseline: &[(String, i128)],
        after: &W,
    ) -> Result<(), ConservationError> {
        for (name, measure) in &self.projections {
            let b = baseline
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| *v)
                .unwrap_or(0);
            let a = measure(after);
            if a != b {
                return Err(ConservationError {
                    projection: name.clone(),
                    before: b,
                    after: a,
                });
            }
        }
        Ok(())
    }
}

impl<W> Default for ConservationRegistry<W> {
    fn default() -> Self {
        ConservationRegistry::new()
    }
}

// Q1 Stone 3: the move-only conserved quantity and its ledger. The `ConservationRegistry` above measures
// a projection's total before and after a change (a test-time check); this is the complementary runtime
// mechanism that makes a leak impossible to write silently. A `Conserved<Q>` cannot be cloned (a conserved
// quantity has no duplicate) or changed off-ledger (it has no public arithmetic), and it moves only through
// the ledger's conserving operations (split, merge, transfer) or its tagged boundary operations (create,
// destroy). The per-step gate (Stone-3 slice 3) asserts the change in a quantity's world total equals the
// ledger's net boundary flow, so a module that drops mass fails at merge rather than in a hand-written test.

/// A quantity that is conserved: it can be moved only through a [`ConservedLedger`]. It has no `Clone` (a
/// conserved quantity cannot be duplicated, so a hand-written clone fails to compile) and no public
/// arithmetic (it cannot be changed off-ledger); the only way to obtain one is [`ConservedLedger::create`],
/// and the only ways to move it are transfer, split, merge, or destroy, each recorded so the per-step net
/// is auditable. Dropping one without destroying it is an unaccounted leak that the per-step gate catches
/// as a net imbalance.
///
/// ```compile_fail
/// use civsim_foundation::conservation::ConservedLedger;
/// let mut ledger = ConservedLedger::new(0i128);
/// let c = ledger.create(10, "source");
/// let _dup = c.clone(); // a conserved quantity has no Clone: this must not compile
/// ```
#[derive(Debug)]
#[must_use = "a Conserved quantity dropped off-ledger is an unaccounted leak; destroy or transfer it"]
pub struct Conserved<Q> {
    amount: Q,
}

impl<Q: Copy> Conserved<Q> {
    /// Read the amount for an audit, without consuming it or extracting it off-ledger.
    pub fn amount(&self) -> Q {
        self.amount
    }
}

/// A per-step ledger for a conserved quantity `Q`. The conserving operations (transfer, split, merge) move
/// quantity between holders without creating or losing any, so they leave the net untouched; the boundary
/// operations (create, destroy) record quantity entering from a tagged source or leaving to a tagged sink.
/// [`Self::net`] is the net boundary flow: a CLOSED step (quantity moving only through the conserving
/// operations) nets to exactly zero, and any nonzero net is a real boundary flow, never a rounding artifact,
/// because `Q`'s addition is exact. The Stone-3 gate over a step asserts the change in the world total for
/// `Q` equals this net, so a silent drop of quantity (a leak) shows up as a mismatch.
pub struct ConservedLedger<Q> {
    zero: Q,
    net: Q,
    moves: usize,
}

impl<Q> ConservedLedger<Q>
where
    Q: Copy + PartialEq + PartialOrd + std::ops::Add<Output = Q> + std::ops::Sub<Output = Q>,
{
    /// A fresh ledger for a quantity whose additive identity is `zero` (`Fixed::ZERO` for mass, `0` for a
    /// count). Passing the zero avoids a numeric-trait bound and keeps the ledger usable for any exact `Q`.
    pub fn new(zero: Q) -> Self {
        ConservedLedger {
            zero,
            net: zero,
            moves: 0,
        }
    }

    /// A quantity entering the system from a tagged SOURCE (an external input), adding to the net boundary
    /// flow. The tag names the source for the audit.
    pub fn create(&mut self, amount: Q, _source: &'static str) -> Conserved<Q> {
        self.net = self.net + amount;
        self.moves += 1;
        Conserved { amount }
    }

    /// A quantity leaving the system to a tagged SINK (an external output), subtracting from the net. The
    /// token is consumed.
    pub fn destroy(&mut self, c: Conserved<Q>, _sink: &'static str) {
        self.net = self.net - c.amount;
        self.moves += 1;
    }

    /// Move a quantity between two tagged holders. The token is re-tagged, not changed, so the total and the
    /// net are both unaffected: a transfer conserves by construction.
    pub fn transfer(
        &mut self,
        c: Conserved<Q>,
        _from: &'static str,
        _to: &'static str,
    ) -> Conserved<Q> {
        self.moves += 1;
        c
    }

    /// Split a quantity into two whose amounts sum exactly to the original (`a + b == c`). `amount` is
    /// clamped to `[zero, c.amount]` so a split can never create quantity. The net is unaffected.
    pub fn split(&mut self, c: Conserved<Q>, amount: Q) -> (Conserved<Q>, Conserved<Q>) {
        let taken = if amount < self.zero {
            self.zero
        } else if amount > c.amount {
            c.amount
        } else {
            amount
        };
        self.moves += 1;
        (
            Conserved { amount: taken },
            Conserved {
                amount: c.amount - taken,
            },
        )
    }

    /// Merge two quantities into one whose amount is exactly their sum (`c == a + b`). The net is unaffected.
    pub fn merge(&mut self, a: Conserved<Q>, b: Conserved<Q>) -> Conserved<Q> {
        self.moves += 1;
        Conserved {
            amount: a.amount + b.amount,
        }
    }

    /// The net boundary flow over the step (created minus destroyed). Zero for a closed, conservative step.
    pub fn net(&self) -> Q {
        self.net
    }

    /// Whether the step is closed: the net boundary flow is zero, so every created quantity was destroyed
    /// or is still held on-ledger and nothing leaked across the boundary unaccounted.
    pub fn is_closed(&self) -> bool {
        self.net == self.zero
    }

    /// The number of recorded moves (the audit size).
    pub fn move_count(&self) -> usize {
        self.moves
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Toy {
        a: i128,
        b: i128,
    }

    #[test]
    fn balanced_change_passes() {
        let mut reg = ConservationRegistry::new();
        reg.register("total", |t: &Toy| t.a + t.b);
        let before = Toy { a: 10, b: 5 };
        let after = Toy { a: 7, b: 8 }; // moved 3 from a to b; total unchanged
        assert!(reg.check(&before, &after).is_ok());
    }

    #[test]
    fn leak_is_caught_with_detail() {
        let mut reg = ConservationRegistry::new();
        reg.register("total", |t: &Toy| t.a + t.b);
        let before = Toy { a: 10, b: 5 };
        let after = Toy { a: 7, b: 7 }; // total dropped by 1
        let err = reg.check(&before, &after).unwrap_err();
        assert_eq!(err.projection, "total");
        assert_eq!(err.before, 15);
        assert_eq!(err.after, 14);
    }

    #[test]
    fn a_new_subsystem_is_covered_by_registering() {
        // Nothing is special-cased: registering a second projection extends coverage.
        let mut reg = ConservationRegistry::new();
        reg.register("a_only", |t: &Toy| t.a);
        let before = Toy { a: 10, b: 5 };
        let after = Toy { a: 7, b: 8 };
        // The total is conserved, but 'a' alone is not, and the registry says so.
        assert!(reg.check(&before, &after).is_err());
        assert_eq!(reg.names(), vec!["a_only"]);
    }

    #[test]
    fn split_and_merge_conserve_the_total_exactly() {
        let mut ledger = ConservedLedger::new(0i128);
        let whole = ledger.create(10, "world_initial");
        let (part, rest) = ledger.split(whole, 3);
        assert_eq!(part.amount(), 3);
        assert_eq!(
            rest.amount(),
            7,
            "the split remainder plus the part must be the original"
        );
        let rejoined = ledger.merge(part, rest);
        assert_eq!(rejoined.amount(), 10, "merge restores the exact total");
        ledger.destroy(rejoined, "world_final");
        assert!(
            ledger.is_closed(),
            "create then destroy of the same total nets to zero"
        );
        assert_eq!(ledger.net(), 0);
    }

    #[test]
    fn a_split_clamps_so_it_never_creates_quantity() {
        let mut ledger = ConservedLedger::new(0i128);
        let whole = ledger.create(5, "src");
        // Asking for more than exists takes all of it and leaves an empty remainder; total is preserved.
        let (a, b) = ledger.split(whole, 9);
        assert_eq!(a.amount(), 5);
        assert_eq!(b.amount(), 0);
        ledger.destroy(a, "sink");
        ledger.destroy(b, "sink");
        assert!(ledger.is_closed());
    }

    #[test]
    fn a_transfer_conserves_and_a_dropped_conserved_leaves_the_ledger_open() {
        let mut ledger = ConservedLedger::new(0i128);
        let m = ledger.create(10, "world_initial");
        let m = ledger.transfer(m, "cell", "ground"); // re-tags, conserves, net unaffected
        let (kept, leaked) = ledger.split(m, 7);
        ledger.destroy(kept, "world_final");
        // `leaked` is dropped without a destroy: an unaccounted leak. The net stays nonzero, so the gate
        // over a step would fail, which is the whole point of the ledger.
        drop(leaked);
        assert!(
            !ledger.is_closed(),
            "a dropped conserved quantity must leave the ledger open (net {})",
            ledger.net()
        );
        assert_eq!(
            ledger.net(),
            3,
            "the created 10 minus the destroyed 7 is the leaked 3"
        );
    }
}
