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

//! The Stage-2 proposer: the candidate compounds for a local composition, over two tiers.
//!
//! A candidate's IDENTITY is its COMPOSITION (element -> count), the representation-independent observable. The
//! bonding descriptor is OPEN attached metadata ([`BondingHints`]) the disposer's energy models consume, NOT
//! part of the identity, so two proposer arrangements of one composition are the same candidate and the
//! disposer (not the proposer) resolves the valence and bonding. There is no closed bonding-mode enum; the
//! hints grow as new tiers attach what they derive.
//!
//! Two tiers emit into the one candidate stream, keyed and deduplicated on composition:
//! - The IONIC charge-balance tier ([`charge_neutral_primitives`]): every primitive (irreducible)
//!   charge-neutral stoichiometry, the Hilbert basis of `sum(state * count) = 0` over the (element,
//!   oxidation-state) species (FINITE by Gordan's lemma, the Lambert-Pottier bound derived from the charges,
//!   no cap, no reserved value; the mixed-valence Fe3O4 = FeO + Fe2O3 is reducible, so it is composed by the
//!   disposer, never a primitive). It attaches its oxidation-state arrangement as a hint.
//! - The MO-viability tier ([`mo_viable_diatomics`]): the covalent diatomics strict valence misses (CO, NO,
//!   the O2 diradical), viable iff the bond order is positive (a net bound state), the bond order computed from
//!   the total valence electrons filling the valence-shell molecular orbitals. It attaches the bond order.
//!
//! All oxidation states come from `Element::valence` and all valence-electron counts from the periodic table's
//! shell-filling cache (`PeriodicTable::main_group_valence`), keyed per element so an alien chemistry is a data
//! row (Principle 9), never an authored table.
//!
//! The LAZINESS INVARIANT ([`max_formable_amount`], [`prune_lazy`]) closes the proposer side: a candidate is
//! proposed only where the composition can form a representably-nonzero amount of it. The bound is the limiting
//! reagent, `min` over the candidate's constituents of `amount(element) / count(element)`, so it derives from
//! the composition amounts and the candidate's own integer stoichiometry, with the presence cut at the
//! fixed-point representability floor (`> Fixed::ZERO`), authoring no threshold. The energy-scaled cut (a
//! resolvable amount whose free-energy contribution falls below the deciding model's resolution) is NOT here:
//! that is the disposer's own `delta`-versus-`resolution_s` ladder, keyed on the model's resolution, so it lands
//! at Stage 4. The silicate polymerization arithmetic is a following sub-slice.

use crate::contract::Proposer;
use crate::verdict::{content_key, Candidate};
use civsim_core::{Fixed, StateHasher};
use civsim_physics::periodic::PeriodicTable;
use std::collections::BTreeMap;

/// A local composition (`x_local`): the locally dominant elements and their amounts, symbol-keyed (the
/// codebase convention, `crates/sim/src/material.rs`). Extensible, no closed element set; the amount is read
/// by the later laziness invariant, not by the charge-balance or MO passes.
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

    /// The elements keyed in the composition, in canonical (sorted) order (whatever their amount).
    pub fn elements(&self) -> impl Iterator<Item = &str> {
        self.amounts.keys().map(|s| s.as_str())
    }

    /// The elements PRESENT in a representably-nonzero amount, in canonical (sorted) order: the laziness
    /// invariant's element floor. An element keyed at `Fixed::ZERO` (named but absent, or an amount that rounds
    /// below the fixed-point representability floor) is not present, so the tiers do not enumerate over it and
    /// no candidate is proposed for it. The cut is `> Fixed::ZERO` (the Q32.32 epsilon), a property of the type,
    /// not an authored threshold.
    pub fn present_elements(&self) -> impl Iterator<Item = &str> {
        self.amounts
            .iter()
            .filter(|(_, amount)| **amount > Fixed::ZERO)
            .map(|(symbol, _)| symbol.as_str())
    }
}

/// The proposer's slice of the environment `E`: the accessible oxidation states per element, which the
/// mu-vector's buffer ladder sets per environment (multi-valence resolved per environment, never per element).
/// It constrains the IONIC tier; the covalent MO tier is environment-independent (a molecule's existence is
/// intrinsic, its abundance the disposer's and laziness's concern). An element absent from the map falls back
/// to its full periodic-table valence set, and an accessible state the element's valence does not carry is
/// dropped (the environment cannot grant a non-physical state).
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

/// Open bonding hints: what a proposer tier derived about a candidate's bonding, for the disposer's energy
/// models. Each tier's finding is an optional field (extensible, never a closed bonding-mode enum), and the
/// hints are NOT part of the candidate identity, so they never split one composition into two candidates. When
/// two tiers propose the same composition their hints merge.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BondingHints {
    /// The ionic charge-balance tier's oxidation-state arrangement, `(element, state) -> count`, whose net
    /// charge is zero. `None` if no ionic tier proposed this composition.
    pub oxidation_states: Option<BTreeMap<(String, i8), u32>>,
    /// The MO-viability tier's diatomic bond order (net bonding electrons over two). `None` if the MO tier did
    /// not propose this composition.
    pub bond_order: Option<Fixed>,
}

impl BondingHints {
    /// Fill any hint this one lacks from `other` (the merge when two tiers propose one composition).
    fn merge(&mut self, other: BondingHints) {
        if self.oxidation_states.is_none() {
            self.oxidation_states = other.oxidation_states;
        }
        if self.bond_order.is_none() {
            self.bond_order = other.bond_order;
        }
    }
}

/// A candidate material compound: its IDENTITY is its COMPOSITION (element -> count), with the bonding
/// descriptor as open, non-identifying [`BondingHints`]. Two compounds are the same candidate iff their
/// compositions are equal, whatever bonding a proposer tier read into them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compound {
    composition: BTreeMap<String, u32>,
    hints: BondingHints,
}

impl Compound {
    /// The composition (element -> count), the candidate identity.
    pub fn composition(&self) -> &BTreeMap<String, u32> {
        &self.composition
    }

    /// The open bonding hints (not part of the identity).
    pub fn hints(&self) -> &BondingHints {
        &self.hints
    }
}

impl Candidate for Compound {
    fn feed_content(&self, hasher: &mut StateHasher) {
        // ONLY the composition (the identity) is fed; the bonding hints are metadata, never hashed, so two
        // arrangements of one composition share a content key and deduplicate. The BTreeMap iterates in sorted
        // key order, so the fed content is canonical (order-independent).
        for (symbol, count) in &self.composition {
            hasher.write_bytes(symbol.as_bytes());
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

/// The Hilbert basis of the charge-neutrality cone over a species list (each species an `(element, state)` with
/// its integer charge = state): every primitive (irreducible) non-negative count vector with
/// `sum(charge * count) = 0`. Each count is bounded by the Lambert-Pottier bound (a cation count by the maximum
/// anion magnitude, an anion count by the maximum cation charge, a state-0 native by 1), a bound DERIVED from
/// the charges, so the enumeration is complete with no fabricated cap. Returned as count vectors aligned with
/// `species`.
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
/// reducible iff some other solution is componentwise `<=` it (their difference is then a non-zero
/// charge-neutral solution, so it is a sum of two non-zero solutions). The minimal ones are the primitives.
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

/// The IONIC tier: the complete set of primitive charge-neutral compounds over a composition's elements at
/// their environment-accessible oxidation states. Each compound's identity is its composition (element ->
/// count); its oxidation-state arrangement is attached as a hint. Complete, no fabricated cap, no reserved
/// value. Deduplicated and canonically ordered.
///
/// The enumeration is over the subsets of the composition's elements, so each Hilbert-basis computation is
/// small and a formula is emitted once (from its exact element set: a primitive is kept for a subset only if it
/// uses every element of that subset). Complete for a local dominant composition (`x_local`, a handful of
/// elements); a composition beyond the machine subset-mask width is out of scope (the dominant cut and the
/// laziness invariant bound it in later slices) and returns empty rather than overflowing.
pub fn charge_neutral_primitives(
    composition: &Composition,
    environment: &Environment,
    table: &PeriodicTable,
) -> Vec<Compound> {
    // Enumerate only over elements present in a representably-nonzero amount (the laziness invariant's element
    // floor): an element keyed at zero proposes nothing.
    let elements: Vec<&str> = composition.present_elements().collect();
    let n = elements.len();
    let mut merged: BTreeMap<BTreeMap<String, u32>, BondingHints> = BTreeMap::new();
    if n == 0 || n >= 63 {
        return Vec::new();
    }
    for mask in 1u64..(1u64 << n) {
        let subset: Vec<&str> = (0..n)
            .filter(|i| mask & (1u64 << i) != 0)
            .map(|i| elements[i])
            .collect();
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
            // Keep the primitive only if it uses EVERY element of the subset, so the formula is emitted once
            // from its exact element set.
            let uses_all = subset.iter().all(|&el| {
                species
                    .iter()
                    .zip(&counts)
                    .any(|((s, _), c)| s == el && *c > 0)
            });
            if !uses_all {
                continue;
            }
            let mut composition_out: BTreeMap<String, u32> = BTreeMap::new();
            let mut arrangement: BTreeMap<(String, i8), u32> = BTreeMap::new();
            for ((el, st), c) in species.iter().zip(&counts) {
                if *c > 0 {
                    *composition_out.entry(el.clone()).or_insert(0) += c;
                    arrangement.insert((el.clone(), *st), *c);
                }
            }
            // First arrangement wins on a composition collision (the disposer re-derives the arrangement; the
            // hint is advisory).
            merged
                .entry(composition_out)
                .or_insert_with(|| BondingHints {
                    oxidation_states: Some(arrangement),
                    bond_order: None,
                });
        }
    }
    into_sorted_compounds(merged)
}

/// The valence shell a main-group element bonds through: period-1 (the `1s` duet) or the `ns np` shell of
/// period 2 and beyond. `None` for a d-block, f-block, or period-6/7 heavy centre (the shell-filling cache
/// returns no main-group valence there), which the MO tier routes out of scope rather than guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellKind {
    /// Period 1: the `1s` shell only (hydrogen, helium).
    SOnly,
    /// Period 2 and beyond main-group: the `ns np` shell.
    SP,
}

/// The valence shell of an element by symbol, or `None` when the shell-filling cache does not resolve a
/// main-group valence (a d/f-block or heavy centre).
fn shell_kind(symbol: &str, table: &PeriodicTable) -> Option<ShellKind> {
    table.main_group_valence(symbol)?;
    let z = table.element(symbol)?.z;
    Some(if z <= 2 {
        ShellKind::SOnly
    } else {
        ShellKind::SP
    })
}

/// The valence-shell molecular-orbital levels for a shell, as `(is_bonding, capacity)` in energy order,
/// DERIVED from the atomic-orbital structure rather than tabulated: each subshell of `k` orbitals gives a
/// bonding MO group of capacity `2k` and an antibonding group of capacity `2k` under LCAO (an orbital holds two
/// electrons). The `s` subshell has one orbital; the `p` subshell has three (one sigma plus two pi). The bond
/// order is insensitive to the order WITHIN the bonding or antibonding groups (the s-p mixing crossover that
/// flips sigma_2p and pi_2p between N2 and O2 moves no electron between bonding and antibonding), so the groups
/// are listed coalesced and the bond order is well-defined without tracking the crossover.
fn mo_sequence(shell: ShellKind) -> Vec<(bool, u32)> {
    const S_ORBITALS: u32 = 1;
    const P_ORBITALS: u32 = 3; // one sigma + two pi
    let mut levels = vec![
        (true, 2 * S_ORBITALS),  // sigma_s bonding
        (false, 2 * S_ORBITALS), // sigma*_s antibonding
    ];
    if shell == ShellKind::SP {
        levels.push((true, 2 * P_ORBITALS)); // the 2p bonding group (sigma_p + two pi_p)
        levels.push((false, 2 * P_ORBITALS)); // the 2p antibonding group (two pi*_p + sigma*_p)
    }
    levels
}

/// The net bonding electrons (bonding minus antibonding) for `n` valence electrons filling a shell's MO levels
/// in order. Twice the bond order; positive iff the molecule is a bound state.
fn net_bonding(n: u32, sequence: &[(bool, u32)]) -> i32 {
    let mut remaining = n;
    let mut net: i32 = 0;
    for &(is_bonding, capacity) in sequence {
        let filled = remaining.min(capacity) as i32;
        net += if is_bonding { filled } else { -filled };
        remaining -= filled as u32;
        if remaining == 0 {
            break;
        }
    }
    net
}

/// A diatomic `a`-`b` as a viable compound, or `None` when it is not viable or out of scope. Viable iff the
/// bond order is positive (a net bound state), from the total valence electrons filling the shared valence
/// shell's MO levels. Out of scope (returns `None`, a later tier's or an override's job, never a wrong answer):
/// a cross-shell pair (one period-1, one period-2+, where the simple diagram is an approximation), a d/f-block
/// centre, and (by construction, only neutral atoms are summed) a charged molecular ion.
fn diatomic_if_viable(a: &str, b: &str, table: &PeriodicTable) -> Option<Compound> {
    let shell_a = shell_kind(a, table)?;
    let shell_b = shell_kind(b, table)?;
    if shell_a != shell_b {
        return None; // cross-shell: the simple homonuclear-shape diagram is an approximation, out of scope
    }
    let va = table.main_group_valence(a)?;
    let vb = table.main_group_valence(b)?;
    let n = va as u32 + vb as u32;
    let net = net_bonding(n, &mo_sequence(shell_a));
    if net <= 0 {
        return None; // bond order <= 0: no bound state
    }
    let mut composition: BTreeMap<String, u32> = BTreeMap::new();
    *composition.entry(a.to_string()).or_insert(0) += 1;
    *composition.entry(b.to_string()).or_insert(0) += 1;
    Some(Compound {
        composition,
        hints: BondingHints {
            oxidation_states: None,
            bond_order: Some(Fixed::from_ratio(net as i64, 2)),
        },
    })
}

/// The MO-viability tier: the viable diatomics (homonuclear A2 and heteronuclear A-B) over the composition's
/// main-group elements, each with its bond order attached as a hint. Scope: same-valence-shell main-group
/// diatomics (the CO lesson); cross-shell pairs, d/f-block centres, charged molecular ions, and polyatomics are
/// out of scope (later tiers). Deduplicated and canonically ordered.
pub fn mo_viable_diatomics(composition: &Composition, table: &PeriodicTable) -> Vec<Compound> {
    // Only present (representably-nonzero) elements form diatomics (the laziness invariant's element floor).
    let elements: Vec<&str> = composition.present_elements().collect();
    let mut merged: BTreeMap<BTreeMap<String, u32>, BondingHints> = BTreeMap::new();
    for (i, &a) in elements.iter().enumerate() {
        for &b in &elements[i..] {
            if let Some(compound) = diatomic_if_viable(a, b, table) {
                merged.entry(compound.composition).or_insert(compound.hints);
            }
        }
    }
    into_sorted_compounds(merged)
}

/// The full Stage-2 proposal: the ionic and MO tiers merged into one candidate stream, keyed on composition
/// (two tiers proposing one composition merge their hints), canonically ordered. This is what the disposer
/// consumes.
pub fn propose_candidates(
    composition: &Composition,
    environment: &Environment,
    table: &PeriodicTable,
) -> Vec<Compound> {
    let mut merged: BTreeMap<BTreeMap<String, u32>, BondingHints> = BTreeMap::new();
    for compound in charge_neutral_primitives(composition, environment, table) {
        merge_compound(&mut merged, compound);
    }
    for compound in mo_viable_diatomics(composition, table) {
        merge_compound(&mut merged, compound);
    }
    // The laziness invariant: keep only candidates the composition can form a representably-nonzero amount of.
    prune_lazy(into_sorted_compounds(merged), composition)
}

/// The maximum amount of a candidate compound the composition can form: the limiting reagent, `min` over the
/// candidate's constituent elements of `amount(element) / count(element)`. A pure function of the composition
/// amounts and the candidate's own integer stoichiometry, deriving with no energy and no reserved threshold.
/// `Fixed::ZERO` when any constituent is absent (keyed at zero or not keyed at all) or when the limiting supply
/// rounds below one representable unit of the compound; that zero IS the laziness cut, at the fixed-point
/// representability floor rather than an authored value. The disposer and freezer read this same bound to scale
/// the extensive free energy and the phase fractions, so it is the proposer-side datum the later stages read,
/// over serving as a prune alone.
pub fn max_formable_amount(candidate: &Compound, composition: &Composition) -> Fixed {
    let mut limiting: Option<Fixed> = None;
    for (element, &count) in candidate.composition() {
        let available = composition
            .amounts
            .get(element)
            .copied()
            .unwrap_or(Fixed::ZERO);
        // available / count: how much of the candidate this element's supply allows. `count >= 1` for every
        // keyed element (a composition entry exists only with a positive count), so the divisor is never zero.
        let per_element = available.div(Fixed::from_int(count as i32));
        limiting = Some(match limiting {
            Some(current) if current <= per_element => current,
            _ => per_element,
        });
    }
    limiting.unwrap_or(Fixed::ZERO)
}

/// The laziness prune: keep only the candidates the composition can form a representably-nonzero amount of
/// ([`max_formable_amount`] above `Fixed::ZERO`). Lossless with respect to the disposer's verdict: a candidate
/// with zero formable amount can never be a ground state the disposer selects, so dropping it changes no
/// outcome. The energy-scaled cut (a resolvable amount whose free-energy contribution is below the deciding
/// model's resolution) is deliberately NOT here: it needs the per-candidate free energy the disposer assembles,
/// so it is the disposer's `delta`-versus-`resolution_s` ladder at Stage 4, not a proposer threshold.
pub fn prune_lazy(candidates: Vec<Compound>, composition: &Composition) -> Vec<Compound> {
    candidates
        .into_iter()
        .filter(|candidate| max_formable_amount(candidate, composition) > Fixed::ZERO)
        .collect()
}

/// Merge a compound into a by-composition map, filling in hints when the composition already exists.
fn merge_compound(merged: &mut BTreeMap<BTreeMap<String, u32>, BondingHints>, compound: Compound) {
    merged
        .entry(compound.composition)
        .or_default()
        .merge(compound.hints);
}

/// Build the canonically-ordered compound list from a by-composition hint map (content-key order, the kernel's
/// canonical candidate order).
fn into_sorted_compounds(merged: BTreeMap<BTreeMap<String, u32>, BondingHints>) -> Vec<Compound> {
    let mut out: Vec<Compound> = merged
        .into_iter()
        .map(|(composition, hints)| Compound { composition, hints })
        .collect();
    out.sort_by_key(content_key);
    out
}

/// The thermochemical proposer: the Stage-2 instantiation of the kernel's [`Proposer`] contract over the
/// periodic-table floor. Proposes the merged ionic and MO candidate stream.
pub struct ThermochemicalProposer<'t> {
    /// The periodic-table floor the tiers read from.
    pub table: &'t PeriodicTable,
}

impl<'t> Proposer for ThermochemicalProposer<'t> {
    type Composition = Composition;
    type Environment = Environment;
    type Candidate = Compound;

    fn propose(&self, x: &Composition, e: &Environment, _seed: u64) -> Vec<Compound> {
        propose_candidates(x, e, self.table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the standard periodic table loads")
    }

    fn comp(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
        pairs.iter().map(|(s, c)| (s.to_string(), *c)).collect()
    }

    /// Find a candidate by its composition (the identity), if present.
    fn find<'a>(
        candidates: &'a [Compound],
        composition: &BTreeMap<String, u32>,
    ) -> Option<&'a Compound> {
        candidates.iter().find(|c| c.composition() == composition)
    }

    #[test]
    fn the_ionic_tier_yields_al2o3_as_a_primitive() {
        let t = table();
        let c = Composition::from_pairs([("Al", Fixed::from_int(2)), ("O", Fixed::from_int(3))]);
        let env = Environment::unconstrained().with_states("O", vec![-2]);
        let ionic = charge_neutral_primitives(&c, &env, &t);
        let al2o3 = find(&ionic, &comp(&[("Al", 2), ("O", 3)])).expect("Al2O3 present");
        // Its identity is the composition; the ionic arrangement is a hint.
        assert_eq!(
            al2o3
                .hints()
                .oxidation_states
                .as_ref()
                .unwrap()
                .get(&("Al".to_string(), 3)),
            Some(&2)
        );
    }

    #[test]
    fn the_ionic_tier_yields_feo_and_fe2o3_but_not_the_reducible_fe3o4() {
        let t = table();
        let c = Composition::from_pairs([("Fe", Fixed::from_int(1)), ("O", Fixed::from_int(1))]);
        let env = Environment::unconstrained().with_states("O", vec![-2]);
        let ionic = charge_neutral_primitives(&c, &env, &t);
        assert!(
            find(&ionic, &comp(&[("Fe", 1), ("O", 1)])).is_some(),
            "FeO is a primitive"
        );
        assert!(
            find(&ionic, &comp(&[("Fe", 2), ("O", 3)])).is_some(),
            "Fe2O3 is a primitive"
        );
        assert!(
            find(&ionic, &comp(&[("Fe", 3), ("O", 4)])).is_none(),
            "Fe3O4 is reducible and must not be a primitive"
        );
    }

    #[test]
    fn the_candidate_identity_is_composition_not_bonding() {
        // Two compounds of the same composition but different hints share a content key (the identity is the
        // composition, the hints are metadata), so they are the same candidate.
        let ionic = Compound {
            composition: comp(&[("C", 1), ("O", 1)]),
            hints: BondingHints {
                oxidation_states: Some(
                    [(("C".to_string(), 2), 1u32), (("O".to_string(), -2), 1)].into(),
                ),
                bond_order: None,
            },
        };
        let covalent = Compound {
            composition: comp(&[("C", 1), ("O", 1)]),
            hints: BondingHints {
                oxidation_states: None,
                bond_order: Some(Fixed::from_int(3)),
            },
        };
        assert_eq!(
            content_key(&ionic),
            content_key(&covalent),
            "one composition is one candidate whatever bonding was read into it"
        );
    }

    #[test]
    fn the_mo_tier_bond_orders_match_the_diagram() {
        let t = table();
        let bo = |a: &str, b: &str| -> Option<Fixed> {
            diatomic_if_viable(a, b, &t).and_then(|c| c.hints().bond_order)
        };
        // CO and N2: bond order 3. NO: 2.5. O2: 2 (the diradical). F2: 1.
        assert_eq!(bo("C", "O"), Some(Fixed::from_int(3)));
        assert_eq!(bo("N", "N"), Some(Fixed::from_int(3)));
        assert_eq!(bo("N", "O"), Some(Fixed::from_ratio(5, 2)));
        assert_eq!(bo("O", "O"), Some(Fixed::from_int(2)));
        assert_eq!(bo("F", "F"), Some(Fixed::from_int(1)));
        // B2 (BO 1), C2 (BO 2), Li2 (BO 1, a real gas-phase molecule).
        assert_eq!(bo("B", "B"), Some(Fixed::from_int(1)));
        assert_eq!(bo("C", "C"), Some(Fixed::from_int(2)));
        assert_eq!(bo("Li", "Li"), Some(Fixed::from_int(1)));
        // Ne2, He2, Be2: bond order 0, not viable (no bound state).
        assert_eq!(bo("Ne", "Ne"), None);
        assert_eq!(bo("He", "He"), None);
        assert_eq!(bo("Be", "Be"), None);
    }

    #[test]
    fn cross_shell_and_dblock_diatomics_are_out_of_scope() {
        let t = table();
        // H (period 1) with F (period 2): cross-shell, out of scope (a later refinement, not a wrong BO).
        assert!(
            diatomic_if_viable("H", "F", &t).is_none(),
            "cross-shell HF is out of scope"
        );
        // Fe is a 3d-block centre: no main-group valence, out of scope.
        assert!(
            diatomic_if_viable("Fe", "Fe", &t).is_none(),
            "a d-block diatomic is out of scope"
        );
    }

    #[test]
    fn the_mo_tier_admits_o2_where_charge_balance_omits_it() {
        // Over {O} alone the ionic tier produces nothing (O has only anionic states, no cation to balance), but
        // the MO tier admits O2 (bond order 2). The unified proposal carries it.
        let t = table();
        let c = Composition::from_pairs([("O", Fixed::from_int(1))]);
        let env = Environment::unconstrained();
        let ionic = charge_neutral_primitives(&c, &env, &t);
        assert!(ionic.is_empty(), "charge balance omits O2, got {ionic:?}");
        let all = propose_candidates(&c, &env, &t);
        let o2 = find(&all, &comp(&[("O", 2)])).expect("O2 present in the unified stream");
        assert_eq!(o2.hints().bond_order, Some(Fixed::from_int(2)));
    }

    #[test]
    fn co_carries_both_tiers_hints_merged_on_one_composition() {
        // CO is producible by BOTH tiers (ionic C+2/O-2 and covalent bond order 3). In the unified stream it is
        // one candidate carrying both hints, the disposer to resolve.
        let t = table();
        let c = Composition::from_pairs([("C", Fixed::from_int(1)), ("O", Fixed::from_int(1))]);
        let env = Environment::unconstrained()
            .with_states("C", vec![2])
            .with_states("O", vec![-2]);
        let all = propose_candidates(&c, &env, &t);
        let co = find(&all, &comp(&[("C", 1), ("O", 1)])).expect("CO present");
        assert!(
            co.hints().oxidation_states.is_some(),
            "the ionic arrangement hint is present"
        );
        assert_eq!(
            co.hints().bond_order,
            Some(Fixed::from_int(3)),
            "the MO bond-order hint is present"
        );
    }

    #[test]
    fn the_unified_stream_is_element_order_independent_and_deterministic() {
        let t = table();
        let env = Environment::unconstrained().with_states("O", vec![-2]);
        let a = Composition::from_pairs([("C", Fixed::from_int(1)), ("O", Fixed::from_int(1))]);
        let b = Composition::from_pairs([("O", Fixed::from_int(1)), ("C", Fixed::from_int(1))]);
        let pa = propose_candidates(&a, &env, &t);
        let pb = propose_candidates(&b, &env, &t);
        assert_eq!(
            pa, pb,
            "the candidate stream is a function of the composition, not the order"
        );
        assert_eq!(
            pa,
            propose_candidates(&a, &env, &t),
            "deterministic across runs"
        );
    }

    #[test]
    fn the_proposer_trait_wraps_the_unified_stream() {
        let t = table();
        let proposer = ThermochemicalProposer { table: &t };
        let c = Composition::from_pairs([("C", Fixed::from_int(1)), ("O", Fixed::from_int(1))]);
        let env = Environment::unconstrained().with_states("O", vec![-2]);
        assert_eq!(
            proposer.propose(&c, &env, 0),
            propose_candidates(&c, &env, &t)
        );
    }

    /// A compound with the given (element, count) composition and empty hints, for the laziness tests (the
    /// hints do not enter the formable-amount bound, which reads composition and amounts only).
    fn compound(pairs: &[(&str, u32)]) -> Compound {
        Compound {
            composition: comp(pairs),
            hints: BondingHints::default(),
        }
    }

    #[test]
    fn the_formable_amount_is_the_limiting_reagent() {
        // Al2O3 from Al:2, O:3: min(2/2, 3/3) = 1, both reagents exactly stoichiometric.
        let exact =
            Composition::from_pairs([("Al", Fixed::from_int(2)), ("O", Fixed::from_int(3))]);
        assert_eq!(
            max_formable_amount(&compound(&[("Al", 2), ("O", 3)]), &exact),
            Fixed::from_int(1)
        );
        // Al2O3 from Al:2, O:30: aluminium limits, min(2/2, 30/3) = min(1, 10) = 1.
        let al_limited =
            Composition::from_pairs([("Al", Fixed::from_int(2)), ("O", Fixed::from_int(30))]);
        assert_eq!(
            max_formable_amount(&compound(&[("Al", 2), ("O", 3)]), &al_limited),
            Fixed::from_int(1)
        );
        // Al2O3 from Al:20, O:3: oxygen limits, min(20/2, 3/3) = min(10, 1) = 1.
        let o_limited =
            Composition::from_pairs([("Al", Fixed::from_int(20)), ("O", Fixed::from_int(3))]);
        assert_eq!(
            max_formable_amount(&compound(&[("Al", 2), ("O", 3)]), &o_limited),
            Fixed::from_int(1)
        );
        // Al2O3 from Al:4, O:30: min(4/2, 30/3) = min(2, 10) = 2 (four aluminium make two formula units).
        let two_units =
            Composition::from_pairs([("Al", Fixed::from_int(4)), ("O", Fixed::from_int(30))]);
        assert_eq!(
            max_formable_amount(&compound(&[("Al", 2), ("O", 3)]), &two_units),
            Fixed::from_int(2)
        );
    }

    #[test]
    fn an_absent_constituent_gives_zero_formable_amount() {
        // A candidate needing oxygen, but the composition has no oxygen keyed: unformable (limiting reagent 0).
        let no_oxygen = Composition::from_pairs([("Al", Fixed::from_int(2))]);
        assert_eq!(
            max_formable_amount(&compound(&[("Al", 2), ("O", 3)]), &no_oxygen),
            Fixed::ZERO
        );
        // A constituent keyed at exactly zero is equally absent.
        let zero_oxygen = Composition::from_pairs([("Al", Fixed::from_int(2)), ("O", Fixed::ZERO)]);
        assert_eq!(
            max_formable_amount(&compound(&[("Al", 2), ("O", 3)]), &zero_oxygen),
            Fixed::ZERO
        );
    }

    #[test]
    fn the_presence_cut_is_the_representability_floor_not_an_authored_value() {
        // The limiting-reagent bound rounds to ZERO exactly when less than one representable unit of the
        // compound can form: the cut is the Q32.32 epsilon, a property of the type, not a fabricated threshold.
        // Three epsilons of a reagent that a count-4 compound needs makes 3/4 -> 0 of a unit (pruned); four
        // epsilons makes exactly one epsilon of a unit (kept). No authored number appears anywhere.
        let three_eps = Composition::from_pairs([("X", Fixed::from_bits(3))]);
        assert_eq!(
            max_formable_amount(&compound(&[("X", 4)]), &three_eps),
            Fixed::ZERO,
            "below one representable unit rounds to the laziness cut"
        );
        let four_eps = Composition::from_pairs([("X", Fixed::from_bits(4))]);
        assert_eq!(
            max_formable_amount(&compound(&[("X", 4)]), &four_eps),
            Fixed::from_bits(1),
            "exactly one representable unit survives the cut"
        );
    }

    #[test]
    fn prune_lazy_drops_the_unformable_and_keeps_the_formable() {
        let composition =
            Composition::from_pairs([("Al", Fixed::from_int(2)), ("O", Fixed::from_int(3))]);
        let formable = compound(&[("Al", 2), ("O", 3)]); // limiting reagent 1
        let unformable = compound(&[("Al", 2), ("O", 3), ("K", 1)]); // needs K, which is absent
        let pruned = prune_lazy(vec![formable.clone(), unformable], &composition);
        assert_eq!(
            pruned,
            vec![formable],
            "only the formable candidate survives"
        );
    }

    #[test]
    fn every_proposed_candidate_is_formable() {
        // The invariant the proposer upholds: every candidate in the unified stream has a positive formable
        // amount (the laziness prune ran), so no phantom candidate reaches the disposer.
        let t = table();
        let c = Composition::from_pairs([
            ("Fe", Fixed::from_int(1)),
            ("O", Fixed::from_int(2)),
            ("C", Fixed::from_int(1)),
        ]);
        let env = Environment::unconstrained().with_states("O", vec![-2]);
        let candidates = propose_candidates(&c, &env, &t);
        assert!(!candidates.is_empty(), "the composition proposes something");
        for candidate in &candidates {
            assert!(
                max_formable_amount(candidate, &c) > Fixed::ZERO,
                "proposed candidate {:?} must be formable",
                candidate.composition()
            );
        }
    }

    #[test]
    fn a_zero_amount_element_proposes_nothing() {
        // An element keyed at zero amount is not present, so the tiers do not enumerate over it: a composition
        // of real carbon and zero oxygen proposes no oxide, only what carbon alone can form.
        let t = table();
        let c = Composition::from_pairs([("C", Fixed::from_int(1)), ("O", Fixed::ZERO)]);
        let env = Environment::unconstrained().with_states("O", vec![-2]);
        let candidates = propose_candidates(&c, &env, &t);
        for candidate in &candidates {
            assert!(
                !candidate.composition().contains_key("O"),
                "no oxygen-bearing candidate when oxygen is absent, got {:?}",
                candidate.composition()
            );
        }
    }

    #[test]
    fn the_formable_amount_scales_with_the_amounts_not_an_absolute_floor() {
        // The derive-first property: the bound is relative to the composition amounts and the stoichiometry, so
        // scaling every amount by the same factor scales the formable amount by that factor (no fixed absolute
        // floor lurks). Doubling the supply doubles what can form.
        let base = Composition::from_pairs([("Al", Fixed::from_int(2)), ("O", Fixed::from_int(3))]);
        let doubled =
            Composition::from_pairs([("Al", Fixed::from_int(4)), ("O", Fixed::from_int(6))]);
        let cand = compound(&[("Al", 2), ("O", 3)]);
        assert_eq!(max_formable_amount(&cand, &base), Fixed::from_int(1));
        assert_eq!(max_formable_amount(&cand, &doubled), Fixed::from_int(2));
    }
}
