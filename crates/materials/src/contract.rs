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

//! The kernel trait contract: the two pure functions and a fold, plus the preflight.
//!
//! Per the owner's Verdict contract, the selection loop is two pure functions and a fold, never a three-stage
//! monolith: the [`Proposer`] and [`Disposer`] are pure functions of `(x, E, seed)` memoized on quantized
//! keys, and the freezer is NOT stage three but [`Quench::quench`], a fold over the trajectory owned by the
//! time-marching layer. Inlining the freezer would smuggle history into the pure oracle and break the solver
//! law's memory clause. [`preflight`](Preflight::preflight) runs the representation theorems before any
//! candidate is proposed, so a reference-validity failure is a checked precondition rather than a deep runtime
//! surprise. Slice 1 defines these signatures; the physics fills the bodies in the thermochemical and
//! attractor instantiations.

use crate::verdict::{Candidate, Verdict};

/// The reference-validity verdict of [`Preflight::preflight`]: the representation theorems either admit the
/// inputs or reject them with a reason, before any candidate is proposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Validity {
    /// The inputs satisfy the representation theorems; proposing may proceed.
    Valid,
    /// A representation theorem rejected the inputs, with the reason (the failing check).
    Invalid(&'static str),
}

impl Validity {
    /// Whether the inputs are valid to propose over.
    pub fn is_valid(&self) -> bool {
        matches!(self, Validity::Valid)
    }
}

/// The preflight check: run the representation theorems (the reference-validity checks the owner names, the
/// boolean, the class window, the U-over-W window, the Bohr-van-Leeuwen-class checks) over the inputs BEFORE
/// any candidate is proposed, so a reference-validity failure is compile-path (a checked precondition) rather
/// than a deep runtime surprise. Physics-specific; the instantiation implements the theorem bodies.
pub trait Preflight {
    /// The composition input `x`.
    type Composition;
    /// The environment input `E` (the temperature, pressure, and chemical-potential vector).
    type Environment;

    /// Validate the inputs against the representation theorems before proposing.
    fn preflight(&self, x: &Self::Composition, e: &Self::Environment) -> Validity;
}

/// The proposer: a PURE function of composition, environment, and seed that enumerates viable candidates
/// (Stage 2). Purity and memoization on quantized keys are the solver-law contract; the kernel memo
/// ([`crate::memo::Memo`]) is the canonical cache. The candidate type carries the content identity the
/// canonicalization law keys on.
pub trait Proposer {
    /// The composition input `x`.
    type Composition;
    /// The environment input `E`.
    type Environment;
    /// The candidate type, content-identified for canonicalization.
    type Candidate: Candidate;

    /// Enumerate the viable candidates for the inputs. Pure: the same `(x, E, seed)` yields the same set.
    fn propose(
        &self,
        x: &Self::Composition,
        e: &Self::Environment,
        seed: u64,
    ) -> Vec<Self::Candidate>;
}

/// The disposer: a PURE function that assembles the energy per candidate and returns the [`Verdict`] whose
/// variant structurally encodes whether a winner is readable (Stage 4). The instantiation computes the free
/// energy (the physics) and calls [`crate::verdict::dispose`] (the generic selection primitive), so the
/// `delta`/`resolution_s` dispatch lives in one place and cannot be re-implemented per instantiation.
pub trait Disposer {
    /// The environment input `E`.
    type Environment;
    /// The candidate type.
    type Candidate: Candidate;

    /// Dispose over the proposed candidates, returning the verdict. Pure in `(candidates, E, seed)`.
    fn dispose(
        &self,
        candidates: Vec<Self::Candidate>,
        e: &Self::Environment,
        seed: u64,
    ) -> Verdict<Self::Candidate>;
}

/// A realized state and the equilibrium it was quenched from. The realization is a PATH functional, so its
/// distance from the equilibrium state is the archive (the Residual Law's disequilibrium record: an
/// assemblage sitting far from equilibrium with no cited barrier suggests a missing candidate).
#[derive(Debug, Clone)]
pub struct RealizedState<S> {
    /// The realized state (what the trajectory actually froze in).
    pub realized: S,
    /// The equilibrium state the quench started from (the state function's answer).
    pub equilibrium: S,
}

/// The freezer: NOT a stage in the pure oracle but a FOLD over the trajectory, owned by the time-marching
/// layer. It consumes the path `h` and quenches the equilibrium state against it (the Dodson closure), so
/// history stays out of the pure oracle. Equilibrium is a state function; realization is a path functional;
/// this signature keeps them separate, so a caller cannot inline the freezer into a pure disposer.
pub trait Quench {
    /// The state type (an assemblage, an attractor state).
    type State;
    /// The path type `h` (the cooling-rate, strain, damage, and fluid-flux record the fold consumes).
    type Path;

    /// Quench the equilibrium state against the trajectory, returning the realized state and its equilibrium.
    fn quench(&self, equilibrium: Self::State, path: &Self::Path) -> RealizedState<Self::State>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::{content_key, dispose, ProvenanceKey, TieSlot};
    use civsim_core::{Fixed, StateHasher};

    // A toy candidate and a toy proposer/disposer, to prove the trait contract composes end to end: propose
    // a set, dispose over it, read a verdict. The physics instantiations fill these bodies for real later.
    #[derive(Debug, Clone)]
    struct Species(u64, Fixed);

    impl Candidate for Species {
        fn feed_content(&self, hasher: &mut StateHasher) {
            hasher.write_u64(self.0);
        }
    }

    struct ToyProposer;
    impl Proposer for ToyProposer {
        type Composition = u64;
        type Environment = ();
        type Candidate = Species;
        fn propose(&self, x: &u64, _e: &(), _seed: u64) -> Vec<Species> {
            // Two candidates whose energies depend on the composition, so the disposer has a real choice.
            vec![
                Species(*x, Fixed::from_int(10)),
                Species(x.wrapping_add(1), Fixed::from_int(3)),
            ]
        }
    }

    struct ToyDisposer;
    impl Disposer for ToyDisposer {
        type Environment = ();
        type Candidate = Species;
        fn dispose(&self, candidates: Vec<Species>, _e: &(), _seed: u64) -> Verdict<Species> {
            dispose(
                candidates,
                |s: &Species| s.1,
                Fixed::from_int(1),
                ProvenanceKey(0),
                TieSlot(0),
            )
        }
    }

    struct ToyPreflight;
    impl Preflight for ToyPreflight {
        type Composition = u64;
        type Environment = ();
        fn preflight(&self, x: &u64, _e: &()) -> Validity {
            if *x == 0 {
                Validity::Invalid("empty composition")
            } else {
                Validity::Valid
            }
        }
    }

    #[test]
    fn the_proposer_disposer_contract_composes_end_to_end() {
        let pf = ToyPreflight;
        assert!(pf.preflight(&5, &()).is_valid());
        assert!(!pf.preflight(&0, &()).is_valid());

        let proposed = ToyProposer.propose(&5, &(), 0);
        assert_eq!(proposed.len(), 2);
        let v = ToyDisposer.dispose(proposed, &(), 0);
        match v {
            Verdict::Decided(d) => assert_eq!(
                content_key(d.winner()),
                content_key(&Species(6, Fixed::ZERO))
            ),
            other => panic!("expected a decided verdict, got {other:?}"),
        }
    }

    #[test]
    fn a_realized_state_carries_its_equilibrium_for_the_residual_archive() {
        struct ToyQuench;
        impl Quench for ToyQuench {
            type State = i64;
            type Path = i64;
            fn quench(&self, equilibrium: i64, path: &i64) -> RealizedState<i64> {
                // A toy quench: the faster the path (larger), the further the realized state lags equilibrium.
                RealizedState {
                    realized: equilibrium - *path,
                    equilibrium,
                }
            }
        }
        let r = ToyQuench.quench(100, &7);
        assert_eq!(r.realized, 93);
        assert_eq!(r.equilibrium, 100);
    }
}
