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

//! The Stage-2 proposer, sub-slice a: the complete primitive charge-neutral stoichiometry enumeration.
//!
//! Over a local composition's elements at their environment-accessible oxidation states, the proposer emits
//! every PRIMITIVE (irreducible) charge-neutral formula. These are the Hilbert basis of the charge-neutrality
//! cone `sum(state * count) = 0` over the (element, oxidation-state) species: FINITE by Gordan's lemma and
//! computed from the charges alone, with each count bounded by the opposite side's maximum charge magnitude
//! (the Lambert-Pottier bound on the minimal solutions of a homogeneous linear Diophantine equation). So the
//! enumeration is complete with NO fabricated cap and NO reserved value: a non-primitive formula (a doubled
//! formula, or a mixed-valence phase like Fe3O4 = FeO + Fe2O3) is a reducible multiple the disposer composes,
//! never a proposer primitive. The oxidation states are read per element from `Element::valence`, keyed per
//! element so an alien chemistry is a data row (Principle 9), never an authored stoichiometry table.
//!
//! This is the classical-valence cheap first pass. The MO-viability tier (the CO/NO/O2-diradical world-content
//! strict valence misses), the silicate polymerization arithmetic, and the laziness invariant are following
//! sub-slices.

use crate::contract::Proposer;
use crate::verdict::{content_key, Candidate};
use civsim_core::{Fixed, StateHasher};
use civsim_physics::periodic::PeriodicTable;
use std::collections::{BTreeMap, BTreeSet};

/// A local composition (`x_local`): the locally dominant elements and their amounts, symbol-keyed (the
/// codebase convention, `crates/sim/src/material.rs`). Extensible, no closed element set; the amount is read
/// by the later laziness invariant, not by this charge-balance pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Composition {
    /// The amount (moles or mass, the caller's unit) per element symbol.
    pub amounts: BTreeMap<String, Fixed>,
}

impl Composition {
    /// A composition from `(symbol, amount)` pairs.
    pub fn from_pairs<I, S>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (S, Fixed)>,
        S: Into<String>,
    {
        Composition {
            amounts: pairs.into_iter().map(|(s, a)| (s.into(), a)).collect(),
        }
    }

    /// The elements present, in canonical (sorted) order.
    pub fn elements(&self) -> impl Iterator<Item = &str> {
        self.amounts.keys().map(|s| s.as_str())
    }
}

/// The proposer's slice of the environment `E`: the accessible oxidation states per element, which the
/// mu-vector's buffer ladder sets per environment (multi-valence resolved per environment, never per element).
/// Sub-slice a supplies this as an explicit filter; the full mu-vector buffer ladder is a later stage. An
/// element absent from the map falls back to its full periodic-table valence set (the environment does not
/// constrain it), and an accessible state the element's valence does not carry is dropped (the environment
/// cannot grant a non-physical state).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Environment {
    /// The environment-accessible oxidation states per element symbol.
    pub accessible_states: BTreeMap<String, Vec<i8>>,
}

impl Environment {
    /// An empty environment: every element falls back to its full valence set.
    pub fn unconstrained() -> Self {
        Environment::default()
    }

    /// Constrain an element to a specific accessible-state set.
    pub fn with_states(mut self, symbol: impl Into<String>, states: Vec<i8>) -> Self {
        self.accessible_states.insert(symbol.into(), states);
        self
    }
}

/// A charge-neutral stoichiometry candidate: element species (an element at an oxidation state) with integer
/// counts summing to zero net charge. Content-identified for the canonicalization law: two stoichiometries are
/// the same candidate iff their `(symbol, state) -> count` maps are equal, whatever order they were built in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stoichiometry {
    /// The species and their counts: `(element symbol, oxidation state) -> count`, all counts positive.
    pub species: BTreeMap<(String, i8), u32>,
}

impl Candidate for Stoichiometry {
    fn feed_content(&self, hasher: &mut StateHasher) {
        // The BTreeMap iterates in sorted key order, so the fed content is canonical (order-independent).
        for ((symbol, state), count) in &self.species {
            hasher.write_bytes(symbol.as_bytes());
            hasher.write_i64(*state as i64);
            hasher.write_u32(*count);
        }
    }
}

/// The environment-accessible oxidation states for an element: the environment's constraint intersected with
/// the element's periodic-table valence set, or the full valence set when the environment does not constrain.
fn accessible_states(symbol: &str, environment: &Environment, table: &PeriodicTable) -> Vec<i8> {
    let valence: Vec<i8> = table
        .element(symbol)
        .map(|e| e.valence.clone())
        .unwrap_or_default();
    match environment.accessible_states.get(symbol) {
        Some(constrained) => constrained
            .iter()
            .copied()
            .filter(|s| valence.contains(s))
            .collect(),
        None => valence,
    }
}

/// The Hilbert basis of the charge-neutrality cone over a species list (each species an `(element, state)`
/// with its integer charge = state): every primitive (irreducible) non-negative count vector with
/// `sum(charge * count) = 0`. Each count is bounded by the Lambert-Pottier bound (a cation count by the
/// maximum anion magnitude, an anion count by the maximum cation charge, a state-0 native by 1), a bound
/// DERIVED from the charges, so the enumeration is complete with no fabricated cap. Returned as count vectors
/// aligned with `species`.
fn hilbert_basis_charge_neutral(species: &[(String, i8)]) -> Vec<Vec<u32>> {
    let m = species.len();
    if m == 0 {
        return Vec::new();
    }
    let charges: Vec<i32> = species.iter().map(|(_, s)| *s as i32).collect();
    let max_cation: i32 = charges
        .iter()
        .copied()
        .filter(|c| *c > 0)
        .max()
        .unwrap_or(0);
    let max_anion: i32 = charges
        .iter()
        .copied()
        .filter(|c| *c < 0)
        .map(|c| -c)
        .max()
        .unwrap_or(0);
    // The per-species count bound (Lambert-Pottier), derived from the charges.
    let bounds: Vec<u32> = charges
        .iter()
        .map(|c| {
            if *c > 0 {
                max_anion as u32 // a cation is bounded by the maximum anion magnitude
            } else if *c < 0 {
                max_cation as u32 // an anion is bounded by the maximum cation charge
            } else {
                1 // a state-0 native contributes no charge; count 0 or 1 (the single-species native)
            }
        })
        .collect();

    // Enumerate every count vector within the bounds; keep the non-zero charge-neutral ones.
    let mut solutions: Vec<Vec<u32>> = Vec::new();
    let mut counts = vec![0u32; m];
    loop {
        if counts.iter().any(|c| *c > 0) {
            let net: i64 = charges
                .iter()
                .zip(&counts)
                .map(|(ch, c)| (*ch as i64) * (*c as i64))
                .sum();
            if net == 0 {
                solutions.push(counts.clone());
            }
        }
        // Mixed-radix increment over the bounds; stop when the most significant digit overflows.
        let mut i = 0;
        loop {
            if i == m {
                // The whole space is enumerated.
                return minimal_solutions(solutions);
            }
            if counts[i] < bounds[i] {
                counts[i] += 1;
                break;
            }
            counts[i] = 0;
            i += 1;
        }
    }
}

/// Filter a set of non-zero charge-neutral solutions to the minimal ones (the Hilbert basis): a solution is
/// reducible iff some other solution is componentwise `<=` it (then their difference is also a solution, so it
/// is a sum of two non-zero solutions). The minimal ones are the primitives.
fn minimal_solutions(solutions: Vec<Vec<u32>>) -> Vec<Vec<u32>> {
    let mut basis = Vec::new();
    for (i, s) in solutions.iter().enumerate() {
        let reducible = solutions
            .iter()
            .enumerate()
            .any(|(j, t)| j != i && t.iter().zip(s).all(|(x, y)| x <= y) && t != s);
        if !reducible {
            basis.push(s.clone());
        }
    }
    basis
}

/// Enumerate the complete set of primitive charge-neutral stoichiometries over a composition's elements at
/// their environment-accessible oxidation states. Complete (the Hilbert basis over every element subset), with
/// no fabricated cap and no reserved value. Returned in canonical (content-key) order, deterministic.
///
/// The enumeration is over the subsets of the composition's elements, so each Hilbert-basis computation is
/// small and a formula is emitted once (from its exact element set: a primitive is kept for a subset only if
/// it uses every element of that subset). This is complete for a local dominant composition (`x_local`, a
/// handful of elements); a composition beyond the machine subset-mask width is out of this sub-slice's scope
/// (the dominant cut and the laziness invariant bound it in later slices), and returns empty rather than
/// overflowing.
pub fn charge_neutral_primitives(
    composition: &Composition,
    environment: &Environment,
    table: &PeriodicTable,
) -> Vec<Stoichiometry> {
    let elements: Vec<&str> = composition.elements().collect();
    let n = elements.len();
    let mut out: Vec<Stoichiometry> = Vec::new();
    if n == 0 || n >= 63 {
        return out;
    }
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    for mask in 1u64..(1u64 << n) {
        let subset: Vec<&str> = (0..n)
            .filter(|i| mask & (1u64 << i) != 0)
            .map(|i| elements[i])
            .collect();
        // The species: every (element, accessible-state) over the subset.
        let mut species: Vec<(String, i8)> = Vec::new();
        for &el in &subset {
            for st in accessible_states(el, environment, table) {
                species.push((el.to_string(), st));
            }
        }
        if species.is_empty() {
            continue;
        }
        for counts in hilbert_basis_charge_neutral(&species) {
            // Keep the primitive only if it uses EVERY element of the subset (some state of each has a
            // positive count), so the formula is emitted once from its exact element set.
            let uses_all = subset.iter().all(|&el| {
                species
                    .iter()
                    .zip(&counts)
                    .any(|((s, _), c)| s == el && *c > 0)
            });
            if !uses_all {
                continue;
            }
            let mut sp: BTreeMap<(String, i8), u32> = BTreeMap::new();
            for ((el, st), c) in species.iter().zip(&counts) {
                if *c > 0 {
                    sp.insert((el.clone(), *st), *c);
                }
            }
            let stoich = Stoichiometry { species: sp };
            if seen.insert(content_key(&stoich)) {
                out.push(stoich);
            }
        }
    }
    out.sort_by_key(content_key);
    out
}

/// The thermochemical proposer: the Stage-2 instantiation of the kernel's [`Proposer`] contract over the
/// periodic-table floor. Sub-slice a proposes the primitive charge-neutral stoichiometries; the MO-viability
/// tier is a following sub-slice.
pub struct ThermochemicalProposer<'t> {
    /// The periodic-table floor the charge-balance reads the oxidation states from.
    pub table: &'t PeriodicTable,
}

impl<'t> Proposer for ThermochemicalProposer<'t> {
    type Composition = Composition;
    type Environment = Environment;
    type Candidate = Stoichiometry;

    fn propose(&self, x: &Composition, e: &Environment, _seed: u64) -> Vec<Stoichiometry> {
        charge_neutral_primitives(x, e, self.table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the standard periodic table loads")
    }

    /// Build the expected stoichiometry from `(symbol, state, count)` triples, for assertions.
    fn stoich(triples: &[(&str, i8, u32)]) -> Stoichiometry {
        Stoichiometry {
            species: triples
                .iter()
                .map(|(s, st, c)| ((s.to_string(), *st), *c))
                .collect(),
        }
    }

    #[test]
    fn al_o_yields_al2o3_as_a_primitive() {
        // The completeness spot-check. Al(+3), O(-2): the primitive charge-neutral binary is Al2O3.
        let t = table();
        let comp = Composition::from_pairs([("Al", Fixed::from_int(2)), ("O", Fixed::from_int(3))]);
        let env = Environment::unconstrained().with_states("O", vec![-2]);
        let primitives = charge_neutral_primitives(&comp, &env, &t);
        assert!(
            primitives.contains(&stoich(&[("Al", 3, 2), ("O", -2, 3)])),
            "Al2O3 is a primitive charge-neutral stoichiometry, got {primitives:?}"
        );
    }

    #[test]
    fn fe_o_yields_feo_and_fe2o3_but_not_the_reducible_fe3o4() {
        // The mixed-valence + reducibility check. Fe(+2, +3), O(-2): the primitives are FeO and Fe2O3;
        // magnetite Fe3O4 = FeO + Fe2O3 is reducible, so it is NOT a proposer primitive (it is the disposer's
        // buffer-ladder composition).
        let t = table();
        let comp = Composition::from_pairs([("Fe", Fixed::from_int(1)), ("O", Fixed::from_int(1))]);
        let env = Environment::unconstrained().with_states("O", vec![-2]);
        let primitives = charge_neutral_primitives(&comp, &env, &t);
        assert!(
            primitives.contains(&stoich(&[("Fe", 2, 1), ("O", -2, 1)])),
            "FeO is a primitive"
        );
        assert!(
            primitives.contains(&stoich(&[("Fe", 3, 2), ("O", -2, 3)])),
            "Fe2O3 is a primitive"
        );
        // Fe3O4 = (Fe+2)1 (Fe+3)2 O4 is reducible (FeO + Fe2O3), so it must not appear.
        assert!(
            !primitives.contains(&stoich(&[("Fe", 2, 1), ("Fe", 3, 2), ("O", -2, 4)])),
            "Fe3O4 is reducible and must not be a proposer primitive, got {primitives:?}"
        );
    }

    #[test]
    fn the_enumeration_is_element_order_independent_and_deterministic() {
        // Two compositions with the SAME elements built in different insertion orders yield the identical
        // candidate set in the identical (content-key) order.
        let t = table();
        let a = Composition::from_pairs([("Fe", Fixed::from_int(1)), ("O", Fixed::from_int(1))]);
        let b = Composition::from_pairs([("O", Fixed::from_int(1)), ("Fe", Fixed::from_int(1))]);
        let env = Environment::unconstrained().with_states("O", vec![-2]);
        let pa = charge_neutral_primitives(&a, &env, &t);
        let pb = charge_neutral_primitives(&b, &env, &t);
        assert_eq!(
            pa, pb,
            "the candidate set is a pure function of the element set, not the order"
        );
        // Running twice is identical (determinism).
        assert_eq!(pa, charge_neutral_primitives(&a, &env, &t));
    }

    #[test]
    fn the_environment_filters_the_accessible_states() {
        // Allowing O(-1) as well as O(-2) admits extra charge-neutral candidates the -2-only environment does
        // not, so the environment genuinely constrains the proposal (multi-valence resolved per environment).
        let t = table();
        let comp = Composition::from_pairs([("Al", Fixed::from_int(2)), ("O", Fixed::from_int(3))]);
        let only_oxide = Environment::unconstrained().with_states("O", vec![-2]);
        let with_peroxide = Environment::unconstrained().with_states("O", vec![-2, -1]);
        let a = charge_neutral_primitives(&comp, &only_oxide, &t);
        let b = charge_neutral_primitives(&comp, &with_peroxide, &t);
        assert!(
            b.len() > a.len(),
            "admitting the O(-1) state widens the proposal (a {} vs b {})",
            a.len(),
            b.len()
        );
        // Al2O3 is present under both.
        assert!(a.contains(&stoich(&[("Al", 3, 2), ("O", -2, 3)])));
        assert!(b.contains(&stoich(&[("Al", 3, 2), ("O", -2, 3)])));
    }

    #[test]
    fn a_lone_cation_yields_no_candidate() {
        // Al(+3) alone cannot form a charge-neutral formula (no anion to balance, no native state-0), so the
        // proposal is empty. The bound is derived: with no anion, the cation's Lambert-Pottier bound is 0.
        let t = table();
        let comp = Composition::from_pairs([("Al", Fixed::from_int(1))]);
        let env = Environment::unconstrained();
        let primitives = charge_neutral_primitives(&comp, &env, &t);
        assert!(
            primitives.is_empty(),
            "a lone cation yields nothing, got {primitives:?}"
        );
    }

    #[test]
    fn the_proposer_trait_wraps_the_enumeration() {
        let t = table();
        let proposer = ThermochemicalProposer { table: &t };
        let comp = Composition::from_pairs([("Al", Fixed::from_int(2)), ("O", Fixed::from_int(3))]);
        let env = Environment::unconstrained().with_states("O", vec![-2]);
        let proposed = proposer.propose(&comp, &env, 0);
        assert_eq!(proposed, charge_neutral_primitives(&comp, &env, &t));
    }
}
