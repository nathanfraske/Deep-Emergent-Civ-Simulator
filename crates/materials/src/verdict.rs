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

//! The [`Verdict`] typestate and the sealed selection primitives ([`dispose`], [`seeded_draw`], [`trivial`]).
//!
//! The Gap Law made unrepresentable-to-violate: a caller can read a winner only from [`Decided`] or
//! [`Trivial`], and only these three functions construct a verdict, so a caller cannot fabricate a `Decided`
//! whose `delta` is inside the deciding model's resolution. When the model cannot separate the top two within
//! its resolution the verdict is [`Escalate`] (go up the provenance ladder) or, on a collapsed tie the ladder
//! cannot break, [`SeededDraw`] (a content-hash-keyed draw). Neither carries a winner, so the resolution-
//! ladder rule is a state that cannot be constructed rather than a rule anyone remembers.

use civsim_core::{canonical_sorted, content_id, Fixed, StateHasher};

/// The opaque provenance key every verdict carries: an interned content id referencing the seven-tag joined
/// register (`civsim_foundation::unified_provenance`). The kernel treats it opaquely so it stays below `foundation`
/// in the layering (`core -> physics -> materials -> foundation -> sim`); `foundation` resolves it against the
/// register, keeping the honesty query where the register lives. So a seeded authored draw becomes a counted entry on the authoring
/// surface, queryable rather than remembered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProvenanceKey(pub u64);

/// The named contingency slot a sub-resolution draw occupies: the layer-4 contingency vector's slot, so a
/// [`SeededDraw`] is a replayable, counted authored draw rather than an anonymous coin flip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TieSlot(pub u64);

/// The error band on the deciding model's winning energy: the resolution quantity the `delta`/`resolution_s`
/// dispatch reads. A single symmetric half-width for slice 1 (an engineering resolution quantity, not a
/// world-content value, so the value-authoring line does not bind its representation); the explicit
/// `[low, high]` interval is an additive refinement (`low = high = half_width` is the symmetric special case)
/// to wire if a real value-breaker with an asymmetric error band appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Band {
    half_width: Fixed,
}

impl Band {
    /// A symmetric band of the given half-width.
    pub fn symmetric(half_width: Fixed) -> Self {
        Band { half_width }
    }

    /// The half-width.
    pub fn half_width(self) -> Fixed {
        self.half_width
    }
}

/// A selection candidate whose IDENTITY is a function of its CONTENT, not its construction or enumeration
/// order (the canonicalization engineering law). The kernel keys the canonical ordering and the seeded draw
/// on this content, so a proposer refactor that permutes the candidate list never reshuffles a tie-broken
/// outcome (the byte-neutrality landmine one level down).
pub trait Candidate {
    /// Feed the candidate's canonical content into the hasher. Two candidates are the same candidate iff their
    /// fed content is byte-identical. Feed every field that distinguishes one candidate from another, in a
    /// fixed order.
    fn feed_content(&self, hasher: &mut StateHasher);
}

/// The content key of a candidate: a deterministic 64-bit id derived from its content via
/// [`civsim_core::content_id`]. Used for both the canonical ordering (a total order over the candidate set)
/// and the content-hash-keyed seeded draw, so both are pure functions of the candidate contents.
pub fn content_key<C: Candidate>(candidate: &C) -> u64 {
    content_id(|h| candidate.feed_content(h))
}

/// A selection verdict. A caller reads a winner only from [`Verdict::Decided`] or [`Verdict::Trivial`]; the
/// sub-resolution outcomes [`Verdict::Escalate`] and [`Verdict::SeededDraw`] carry no winner, so the
/// resolution-ladder rule is a state that cannot be constructed. Every variant carries the `provenance_key`
/// for the honesty accounting.
#[derive(Debug, Clone)]
pub enum Verdict<C> {
    /// The deciding model separated the winner from the runner-up by `delta >= resolution_s`. The winner is
    /// readable here (and on [`Verdict::Trivial`]) and nowhere else.
    Decided(Decided<C>),
    /// `delta < resolution_s`: the model cannot separate the top two within its own resolution. NO winner;
    /// escalate up the provenance ladder to a better-pinned model.
    Escalate(Escalate<C>),
    /// A collapsed tie the ladder could not break: a content-hash-keyed draw into a named contingency slot.
    /// NO winner in the decided sense; the drawn member is an authored pick nature did not derive either.
    SeededDraw(SeededDraw<C>),
    /// A genuinely single-candidate query: the physics is unambiguous, so the ceremony is cheap, but it is
    /// logged so ceremony-avoidance is itself auditable.
    Trivial(Trivial<C>),
}

/// A decided verdict: the fields are private and built only by [`dispose`], so a caller cannot fabricate one
/// whose `delta` is inside the resolution.
#[derive(Debug, Clone)]
pub struct Decided<C> {
    winner: C,
    runner_up: C,
    delta: Fixed,
    resolution_s: Fixed,
    band: Band,
    provenance_key: ProvenanceKey,
    tie_slot: TieSlot,
}

impl<C> Decided<C> {
    /// The winning candidate (the ground state the model resolved).
    pub fn winner(&self) -> &C {
        &self.winner
    }
    /// The runner-up (the next-best candidate the winner beat by `delta`).
    pub fn runner_up(&self) -> &C {
        &self.runner_up
    }
    /// The energy gap to the runner-up (`>= resolution_s`).
    pub fn delta(&self) -> Fixed {
        self.delta
    }
    /// The deciding model's own resolution (the gap threshold the decision cleared).
    pub fn resolution_s(&self) -> Fixed {
        self.resolution_s
    }
    /// The error band on the winning energy.
    pub fn band(&self) -> Band {
        self.band
    }
    /// The opaque provenance key for the honesty accounting.
    pub fn provenance_key(&self) -> ProvenanceKey {
        self.provenance_key
    }
    /// The contingency slot this selection site occupies (carried for replay and provenance; no draw fired).
    pub fn tie_slot(&self) -> TieSlot {
        self.tie_slot
    }
}

/// An escalation: `delta < resolution_s`, the near-degenerate candidate set with NO winner. There is
/// deliberately no `winner` accessor: reading a winner off an escalation does not compile.
#[derive(Debug, Clone)]
pub struct Escalate<C> {
    candidates: Vec<C>,
    delta: Fixed,
    resolution_s: Fixed,
    provenance_key: ProvenanceKey,
    tie_slot: TieSlot,
}

impl<C> Escalate<C> {
    /// The near-degenerate candidate set (in canonical order), the input to the next model up the ladder.
    pub fn candidates(&self) -> &[C] {
        &self.candidates
    }
    /// The sub-resolution gap between the top two (`< resolution_s`).
    pub fn delta(&self) -> Fixed {
        self.delta
    }
    /// The resolution the gap failed to clear.
    pub fn resolution_s(&self) -> Fixed {
        self.resolution_s
    }
    /// The opaque provenance key for the honesty accounting.
    pub fn provenance_key(&self) -> ProvenanceKey {
        self.provenance_key
    }
    /// The contingency slot a downstream draw would occupy if the ladder is exhausted here.
    pub fn tie_slot(&self) -> TieSlot {
        self.tie_slot
    }
}

/// A seeded draw: a content-hash-keyed pick into a named contingency slot, the terminal the escalation ladder
/// calls when a collapsed tie cannot be broken by a better-pinned model. There is deliberately no `winner`
/// accessor: a draw is not a decided winner.
#[derive(Debug, Clone)]
pub struct SeededDraw<C> {
    drawn: C,
    drawn_content_key: u64,
    tie_slot: TieSlot,
    provenance_key: ProvenanceKey,
}

impl<C> SeededDraw<C> {
    /// The drawn candidate.
    pub fn drawn(&self) -> &C {
        &self.drawn
    }
    /// The drawn candidate's content key (the identity the draw is keyed on, for replay and audit).
    pub fn drawn_content_key(&self) -> u64 {
        self.drawn_content_key
    }
    /// The named contingency slot this draw occupies.
    pub fn tie_slot(&self) -> TieSlot {
        self.tie_slot
    }
    /// The opaque provenance key for the honesty accounting (a seeded draw is a counted authored draw).
    pub fn provenance_key(&self) -> ProvenanceKey {
        self.provenance_key
    }
}

/// A trivial verdict: a single-candidate query, logged so ceremony-avoidance is auditable.
#[derive(Debug, Clone)]
pub struct Trivial<C> {
    winner: C,
    provenance_key: ProvenanceKey,
}

impl<C> Trivial<C> {
    /// The single candidate (the unambiguous winner).
    pub fn winner(&self) -> &C {
        &self.winner
    }
    /// The opaque provenance key for the honesty accounting.
    pub fn provenance_key(&self) -> ProvenanceKey {
        self.provenance_key
    }
}

/// Dispose over a candidate set by an energy model (lower energy is more stable, the ground state). The single
/// sealed constructor of [`Decided`]/[`Escalate`]/[`Trivial`]: a caller cannot fabricate a `Decided` whose
/// `delta` is inside the resolution because only this function builds one. Candidates are ordered canonically
/// by content key before selection, so the winner and runner-up are a pure function of the candidate SET, not
/// its enumeration order; an energy tie breaks by content key (the canonically-earlier candidate wins).
///
/// - 0 candidates: [`Verdict::Escalate`] with an empty set (a coverage failure: the proposer nominated
///   nothing, so there is nothing to decide and the honest signal is to escalate).
/// - 1 candidate: [`Verdict::Trivial`] (logged).
/// - `>= 2`, `delta >= resolution_s`: [`Verdict::Decided`].
/// - `>= 2`, `delta < resolution_s`: [`Verdict::Escalate`] (the model cannot separate them within its
///   resolution). The collapsed-tie [`Verdict::SeededDraw`] is the terminal the ladder calls via
///   [`seeded_draw`] once escalation is exhausted, so the kernel never fabricates a collapse threshold.
pub fn dispose<C, F>(
    candidates: Vec<C>,
    energy: F,
    resolution_s: Fixed,
    provenance_key: ProvenanceKey,
    tie_slot: TieSlot,
) -> Verdict<C>
where
    C: Candidate + Clone,
    F: Fn(&C) -> Fixed,
{
    if candidates.is_empty() {
        return Verdict::Escalate(Escalate {
            candidates: Vec::new(),
            delta: Fixed::ZERO,
            resolution_s,
            provenance_key,
            tie_slot,
        });
    }
    if candidates.len() == 1 {
        return Verdict::Trivial(Trivial {
            winner: candidates.into_iter().next().expect("len == 1"),
            provenance_key,
        });
    }
    // Canonical order: sort by content key so selection is a pure function of the candidate set. An energy
    // tie then breaks by content key (the earlier-in-canonical-order candidate wins), which is deterministic.
    let ordered = canonical_sorted(candidates, content_key);
    let energies: Vec<Fixed> = ordered.iter().map(&energy).collect();
    // The winner is the min-energy candidate; a strict `<` keeps the canonically-earlier one on an energy tie.
    let mut best = 0usize;
    for i in 1..ordered.len() {
        if energies[i] < energies[best] {
            best = i;
        }
    }
    // The runner-up is the min-energy candidate among the rest.
    let mut second = if best == 0 { 1 } else { 0 };
    for i in 0..ordered.len() {
        if i == best {
            continue;
        }
        if energies[i] < energies[second] {
            second = i;
        }
    }
    // The winner-to-runner-up gap. `checked_sub` keeps this `pub` kernel total: for any realistic energy pair
    // it is the exact difference, and on the (in-practice unreachable) overflow of two energies spanning the full
    // Q32.32 range it saturates to `MAX`, which decides (maximally-separated candidates are decidable), never a
    // panic on an in-contract input.
    let delta = energies[second]
        .checked_sub(energies[best])
        .unwrap_or(Fixed::MAX);
    if delta >= resolution_s {
        Verdict::Decided(Decided {
            winner: ordered[best].clone(),
            runner_up: ordered[second].clone(),
            delta,
            resolution_s,
            band: Band::symmetric(resolution_s),
            provenance_key,
            tie_slot,
        })
    } else {
        Verdict::Escalate(Escalate {
            candidates: ordered,
            delta,
            resolution_s,
            provenance_key,
            tie_slot,
        })
    }
}

/// The terminal draw the escalation ladder calls when a collapsed tie cannot be broken by a better-pinned
/// model: a content-hash-keyed pick into a named contingency slot. The draw scores each candidate by mixing
/// the seed with the candidate's content key and picks the minimum score (ties in score broken by content
/// key), so the drawn member is a pure function of the candidate SET and the seed, never enumeration order
/// (the canonicalization law). Panics on an empty set (the ladder never draws over nothing; an empty draw is
/// a coverage bug, not a runtime condition).
pub fn seeded_draw<C>(
    candidates: Vec<C>,
    tie_slot: TieSlot,
    provenance_key: ProvenanceKey,
    seed: u64,
) -> SeededDraw<C>
where
    C: Candidate + Clone,
{
    let drawn = candidates
        .into_iter()
        .min_by_key(|c| {
            let ck = content_key(c);
            // The lottery score: the seed mixed with the candidate content, so the draw is deterministic in
            // the seed and keyed on content. The content key breaks a score collision for a total order.
            let score = content_id(|h| {
                h.write_u64(seed);
                h.write_u64(ck);
            });
            (score, ck)
        })
        .expect("seeded_draw requires a non-empty candidate set");
    let drawn_content_key = content_key(&drawn);
    SeededDraw {
        drawn,
        drawn_content_key,
        tie_slot,
        provenance_key,
    }
}

/// The single-candidate constructor for a genuinely unambiguous query, so the discipline stays cheap where
/// physics leaves one candidate. Logged (via [`crate::log::VerdictLog`]) so ceremony-avoidance is auditable.
pub fn trivial<C>(winner: C, provenance_key: ProvenanceKey) -> Verdict<C> {
    Verdict::Trivial(Trivial {
        winner,
        provenance_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A toy candidate: an id (its content identity) plus an energy the toy energy model reads. Only the id
    /// is fed as content, so two candidates with the same id are the same candidate whatever their order.
    #[derive(Debug, Clone, PartialEq)]
    struct Cand {
        id: u64,
        energy: Fixed,
    }

    impl Candidate for Cand {
        fn feed_content(&self, hasher: &mut StateHasher) {
            hasher.write_u64(self.id);
        }
    }

    fn cand(id: u64, energy_int: i32) -> Cand {
        Cand {
            id,
            energy: Fixed::from_int(energy_int),
        }
    }

    fn energy_of(c: &Cand) -> Fixed {
        c.energy
    }

    const PK: ProvenanceKey = ProvenanceKey(7);
    const SLOT: TieSlot = TieSlot(1);

    #[test]
    fn a_clear_winner_is_decided_with_the_min_energy() {
        // energies 3 and 10, resolution 1: gap 7 >= 1, decided.
        let v = dispose(
            vec![cand(1, 10), cand(2, 3)],
            energy_of,
            Fixed::from_int(1),
            PK,
            SLOT,
        );
        match v {
            Verdict::Decided(d) => {
                assert_eq!(d.winner().id, 2, "the min-energy candidate wins");
                assert_eq!(d.runner_up().id, 1);
                assert_eq!(d.delta(), Fixed::from_int(7));
                assert_eq!(d.band().half_width(), Fixed::from_int(1));
                assert_eq!(d.provenance_key(), PK);
            }
            other => panic!("expected Decided, got {other:?}"),
        }
    }

    #[test]
    fn within_resolution_escalates_with_no_winner() {
        // energies 3 and 4, resolution 5: gap 1 < 5, escalate. The typestate: the returned verdict has no
        // winner accessor to read (Escalate exposes candidates(), not winner()).
        let v = dispose(
            vec![cand(1, 3), cand(2, 4)],
            energy_of,
            Fixed::from_int(5),
            PK,
            SLOT,
        );
        match v {
            Verdict::Escalate(e) => {
                assert_eq!(e.delta(), Fixed::from_int(1));
                assert_eq!(e.candidates().len(), 2);
                // There is no e.winner(): the sub-resolution outcome cannot yield a winner. This is the
                // sealed-construction typestate, also proven by the compile_fail doctest on `Escalate`.
            }
            other => panic!("expected Escalate, got {other:?}"),
        }
    }

    #[test]
    fn a_single_candidate_is_trivial() {
        let v = dispose(vec![cand(9, 5)], energy_of, Fixed::from_int(1), PK, SLOT);
        assert!(matches!(v, Verdict::Trivial(t) if t.winner().id == 9));
    }

    #[test]
    fn an_empty_set_escalates() {
        let v: Verdict<Cand> = dispose(Vec::new(), energy_of, Fixed::from_int(1), PK, SLOT);
        assert!(matches!(v, Verdict::Escalate(e) if e.candidates().is_empty()));
    }

    #[test]
    fn the_winner_is_order_independent() {
        // The same set in two orders decides the identical winner (the canonical ordering).
        let forward = dispose(
            vec![cand(1, 10), cand(2, 3), cand(3, 7)],
            energy_of,
            Fixed::from_int(1),
            PK,
            SLOT,
        );
        let reversed = dispose(
            vec![cand(3, 7), cand(2, 3), cand(1, 10)],
            energy_of,
            Fixed::from_int(1),
            PK,
            SLOT,
        );
        let win = |v: &Verdict<Cand>| match v {
            Verdict::Decided(d) => d.winner().id,
            _ => panic!("expected Decided"),
        };
        assert_eq!(
            win(&forward),
            win(&reversed),
            "the winner is a function of the set, not the order"
        );
        assert_eq!(win(&forward), 2);
    }

    #[test]
    fn an_energy_tie_breaks_by_content_key_deterministically() {
        // Two candidates of equal energy: the winner-vs-runner-up split is decided by content key, so it is
        // the same in any input order (and the gap is 0, so with any positive resolution it escalates).
        let a = dispose(
            vec![cand(1, 5), cand(2, 5)],
            energy_of,
            Fixed::from_int(1),
            PK,
            SLOT,
        );
        let b = dispose(
            vec![cand(2, 5), cand(1, 5)],
            energy_of,
            Fixed::from_int(1),
            PK,
            SLOT,
        );
        // Equal energy -> gap 0 < 1 -> escalate, and the canonical candidate order is identical either way.
        let ids = |v: &Verdict<Cand>| match v {
            Verdict::Escalate(e) => e.candidates().iter().map(|c| c.id).collect::<Vec<_>>(),
            _ => panic!("expected Escalate on a zero gap"),
        };
        assert_eq!(
            ids(&a),
            ids(&b),
            "the canonical candidate order is input-order-independent"
        );
    }

    #[test]
    fn a_seeded_draw_keys_on_content_not_enumeration_order() {
        // THE CANONICALIZATION DEMONSTRATE-FAILURE. A collapsed tie drawn by seed must draw the identical
        // member whatever the candidate order, because the draw keys on the content hash, not the index.
        let forward = vec![cand(11, 5), cand(22, 5), cand(33, 5), cand(44, 5)];
        let permuted = vec![cand(33, 5), cand(11, 5), cand(44, 5), cand(22, 5)];
        let draw_f = seeded_draw(forward.clone(), SLOT, PK, 0xC0FFEE);
        let draw_p = seeded_draw(permuted.clone(), SLOT, PK, 0xC0FFEE);
        assert_eq!(
            draw_f.drawn().id,
            draw_p.drawn().id,
            "the content-hash-keyed draw is invariant under candidate permutation"
        );
        assert_eq!(draw_f.drawn_content_key(), content_key(draw_f.drawn()));

        // The counterexample that makes the content-hash keying load-bearing: an order-keyed draw (take index
        // 0) DOES reshuffle under permutation, so keying on order would silently move the tie-broken world.
        let order_keyed = |set: &[Cand]| set[0].id;
        assert_ne!(
            order_keyed(&forward),
            order_keyed(&permuted),
            "an enumeration-order draw is permutation-sensitive, which is the landmine content keying avoids"
        );
    }

    #[test]
    fn a_seeded_draw_is_deterministic_in_the_seed() {
        let set = vec![cand(11, 5), cand(22, 5), cand(33, 5)];
        let d1 = seeded_draw(set.clone(), SLOT, PK, 42);
        let d2 = seeded_draw(set.clone(), SLOT, PK, 42);
        assert_eq!(d1.drawn().id, d2.drawn().id, "same seed, same draw");
    }

    #[test]
    fn the_trivial_constructor_makes_a_logged_single_candidate() {
        let v = trivial(cand(5, 0), PK);
        assert!(matches!(v, Verdict::Trivial(t) if t.winner().id == 5 && t.provenance_key() == PK));
    }
}

/// Compile-fail proof of the typestate: an [`Escalate`] has no `winner` accessor, so reading a winner off a
/// sub-resolution verdict does not compile. This is the sealed-construction typestate as a compile-path
/// guarantee, the complement of the runtime `within_resolution_escalates_with_no_winner` test.
///
/// ```compile_fail
/// use civsim_materials::verdict::Escalate;
/// fn read_a_winner_off_an_escalation<C>(e: &Escalate<C>) -> &C {
///     e.winner()
/// }
/// ```
#[cfg(doctest)]
struct EscalationHasNoWinnerAccessor;
