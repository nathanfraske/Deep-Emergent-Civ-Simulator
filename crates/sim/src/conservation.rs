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
}
