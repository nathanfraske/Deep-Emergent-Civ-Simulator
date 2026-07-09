// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.

//! The conviction-experience associative record (Branch 1 of the learned experience-to-conviction coupling,
//! `docs/working/OWNER_DECISIONS_LOG.md` R2 and R4): a per-being learned association between the being's own
//! FELT EXPERIENCE and each conviction it HOLDS. This is the substrate that lets "sustained hardship comes to
//! bear on a conviction" become a LEARNED per-being fact rather than an authored kind-of-experience to
//! kind-of-conviction route (the blind framing panel's ruling): felt experience emits only a magnitude and a
//! valence sign (`crate::physiology::felt_salience`, Prereq A), and which conviction that felt outcome bears
//! on is discovered by CORRELATION over the being's own life.
//!
//! The mechanism is a leaky signed accumulator per conviction axis. Each felt event folds the being's
//! felt-experience summary (valence times intensity) into the accumulator of each conviction it CURRENTLY
//! holds, weighted by the SIGNED stance (POLE-REFERENCED), with a retention decay so the record tracks RECENT
//! lived valence and can un-form (defeasible, the harm learner's own property). Because the weight is the
//! signed stance, `sign(A)` is a relabel-invariant learned fact ("which pole was good to hold, for me"):
//! under a relabel of the axis (swapping which pole is +1) the stance flips sign, the accumulator flips sign,
//! and Branch 2's move tracks the SAME physical pole, so the engine never reads which pole "means" what (the
//! third framing panel's ruling, R5). It reads NO behaviour weight, so it is weight-agnostic: a founder whose
//! convictions do not yet drive behaviour, and a sessile being with no locomotion, still learn the association
//! from the same felt reserve swings (the R4 correction of the first-cut `weight x stance` framing, which a
//! fully-blind panel unanimously caught as a static, motility-parasitic mask). It keys on no axis meaning
//! (`crate::axiom::AxiomAxisId` alone, the Steering Audit), so an alien's convictions and reserves fold through
//! the identical call (Principle 9).
//!
//! It is INERT: recording an association changes no conviction and no behaviour (Branch 2, the credit
//! assignment, consumes it to move a conviction). It is EMPTY by default, so a being that holds no conviction,
//! or a world that does not arm the felt-conviction learner, folds nothing into `state_hash` (opt-in,
//! byte-neutral). HONEST LIMIT (panel-confirmed, R4): at the controller-percept tier the felt outcome folds
//! into EVERY held conviction (weighted by strength), so per-conviction attribution is DIFFUSE; crisp
//! attribution to the one conviction a being acted under needs the deliberative tier, a future refinement.
//! Per-axis divergence still emerges from WHEN each conviction is held relative to the being's lived valence.

use crate::axiom::AxiomAxisId;
use crate::physiology::FeltExperience;
use civsim_core::{Fixed, StateHasher};
use std::collections::BTreeMap;

/// A per-being learned association between felt experience and each conviction the being holds (Branch 1). A
/// leaky signed accumulator per conviction axis: positive when the being's recent lived experience while
/// holding the conviction was net-good, negative when net-bad, absent (zero) when the conviction is unheld or
/// the record has washed out. EMPTY by default, so a being that holds no conviction, or a world that does not
/// arm the felt-conviction learner, folds nothing into the hash (opt-in, byte-neutral).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConvictionExperience {
    /// Per-conviction-axis accumulated signed felt-experience association, in canonical `BTreeMap` order so
    /// the fold and every read are reproducible and thread-invariant (Principle 3). A pruned-to-zero axis is
    /// absent, so the map stays bounded and empty-neutral.
    assoc: BTreeMap<AxiomAxisId, Fixed>,
}

impl ConvictionExperience {
    /// An empty record: no association held, so nothing folds into the hash until the first felt event that
    /// finds a held conviction. The default and the opt-out.
    pub fn new() -> ConvictionExperience {
        ConvictionExperience::default()
    }

    /// Whether no association is held (an empty record folds nothing into the hash, the opt-out state).
    pub fn is_empty(&self) -> bool {
        self.assoc.is_empty()
    }

    /// The accumulated felt-experience association for a conviction axis (Branch 2 reads this to decide how a
    /// conviction moves): POLE-REFERENCED, so `sign(A)` names which pole the being's recent lived experience
    /// favoured (positive if the being tended to thrive while holding the +pole or suffer while holding the
    /// -pole, negative for the mirror), and zero if the being never held it or the record washed out. This is
    /// relabel-invariant (it flips sign with a pole relabel), so no absolute pole meaning is read (R5).
    pub fn association(&self, axis: AxiomAxisId) -> Fixed {
        self.assoc.get(&axis).copied().unwrap_or(Fixed::ZERO)
    }

    /// The held associations with their axes, in canonical order (the walk a reader or the hash fold runs).
    pub fn entries(&self) -> impl Iterator<Item = (&AxiomAxisId, &Fixed)> {
        self.assoc.iter()
    }

    /// Fold one felt event into the being's per-conviction accumulators (the Branch-1 record). `felt` is the
    /// being's own felt-experience summary this tick ([`crate::physiology::felt_salience`], keyed on no axis
    /// identity). `held` is the being's currently-held convictions as `(axis, stance)` pairs (from its
    /// intrinsic beliefs). `retention` is the leak (below one): every existing accumulator decays by it first
    /// (so a conviction no longer held, and an old lived valence, fade, and the record stays defeasible), then
    /// each held conviction gains `felt.valence * felt.intensity * stance` (the SIGNED, pole-referenced stance,
    /// so `sign(A)` names which pole the being's experience favoured, relabel-invariant, R5). Prunes
    /// accumulators that reach zero, so the record stays bounded and
    /// empty-neutral. Reads NO behaviour weight (weight-agnostic, R4) and changes no conviction (inert). A pure
    /// deterministic fold in canonical axis order, drawing no randomness; arithmetic saturates rather than
    /// wrapping so an extreme run of felt events cannot panic under the release overflow checks.
    pub fn fold(&mut self, felt: FeltExperience, held: &[(AxiomAxisId, Fixed)], retention: Fixed) {
        // The leak, applied to every existing accumulator FIRST (before this event's evidence), so the record
        // tracks recent experience and a conviction no longer held fades toward zero. Prune the underflowed.
        self.assoc.retain(|_, v| {
            *v = v.checked_mul(retention).unwrap_or(Fixed::ZERO);
            *v != Fixed::ZERO
        });
        // A calm tick (no net reserve movement) carries no valence to fold, so nothing new is credited (the
        // felt summary's own honest default); the leak above still ran, so a quiet spell erodes old records.
        if felt.valence == Fixed::ZERO || felt.intensity == Fixed::ZERO {
            return;
        }
        let signed = felt
            .valence
            .checked_mul(felt.intensity)
            .unwrap_or(Fixed::ZERO);
        for &(axis, stance) in held {
            // POLE-REFERENCED engagement: the SIGNED stance (not its magnitude), so the accumulator records the
            // correlation between the felt valence and WHICH POLE the being held. This makes `sign(A)` a
            // relabel-invariant learned fact ("which pole was good to hold, for me"): under a relabel of the
            // axis (swapping which pole is +1) the stance flips sign, so the increment and the whole accumulator
            // flip sign, and Branch 2's move tracks the SAME physical pole, so the engine never reads which pole
            // "means" what (the third framing panel's ruling, R5). The magnitude still scales with |stance|, so
            // a more strongly held conviction accrues a stronger association. An unheld (zero-stance) conviction
            // is not at stake, so nothing is credited.
            if stance == Fixed::ZERO {
                continue;
            }
            let increment = signed.checked_mul(stance).unwrap_or(Fixed::ZERO);
            let entry = self.assoc.entry(axis).or_insert(Fixed::ZERO);
            *entry = entry.checked_add(increment).unwrap_or(*entry);
            // A cancellation back to exactly zero drops the axis, keeping the record empty-neutral.
            if *entry == Fixed::ZERO {
                self.assoc.remove(&axis);
            }
        }
    }

    /// Fold the record into a hash in canonical (axis, association) order, beside the being's other dynamic
    /// state. An empty record folds nothing, so an opted-out run is byte-identical. The `BTreeMap` walks in
    /// canonical key order, so the fold is reproducible and thread-invariant (Principle 3).
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (axis, v) in &self.assoc {
            h.write_u64(axis.0 as u64);
            h.write_fixed(*v);
        }
    }
}

/// The MOVE parameters for Branch 2 (the credit-assignment half, `docs/working/OWNER_DECISIONS_LOG.md` R2/R5):
/// the gate the felt drive must clear to move a conviction and the step it then takes. Present only when a
/// world opts into the felt-experience-MOVES-conviction leg; absent, the learner only RECORDS (Branch 1, inert).
#[derive(Clone, Copy, Debug)]
pub struct ConvictionMoveParams {
    /// The entrenchment gate the felt drive `|polarity * association|` must clear to move a conviction (below
    /// it the felt evidence is absorbed without moving the stance, belief perseverance). RESERVED. Basis: the
    /// axiom kernel's own entrenchment-threshold curve (`axiom.entrenchment_curve`) read at the axiom's rank is
    /// the canonical gate; this flat value is the dev interim for a baseline rank, the entrenchment-scaled gate
    /// the faithful refinement.
    pub threshold: Fixed,
    /// The step scale of the felt-experience move: the fraction of the gap to the target pole closed per unit
    /// of cleared pressure, the same role plasticity plays in [`crate::axiom::Axiom::appraise`]. RESERVED.
    /// Basis: the belief-plasticity phenotype the axiom kernel already uses (`Mind::plasticity`); this value is
    /// the dev interim.
    pub plasticity: Fixed,
}

/// The reserved calibration the felt-conviction learner reads: the retention (leak) applied to each
/// conviction-experience accumulator per felt event (Branch 1), and optionally the MOVE parameters that let
/// the record actually move a conviction (Branch 2). The mechanism is fixed Rust; these numbers are the owner's.
#[derive(Clone, Copy, Debug)]
pub struct FeltConvictionCalib {
    /// The retention (leak) per felt event, below one, so the association tracks RECENT lived valence and stays
    /// defeasible (a run of opposite-valence experience erodes an earlier record). RESERVED. Basis: the
    /// eligibility-decay and evidence-ring retention the reward and harm learners already use, set on the same
    /// order for consistency (the felt-conviction record is the reward/harm learner's sibling over convictions
    /// rather than actions or features). A retention of one would never forget (non-defeasible); a small one
    /// would forget within a few ticks (no lifetime integration).
    pub retention: Fixed,
    /// The Branch-2 MOVE parameters, or `None` to RECORD only (Branch 1, inert: the association accumulates and
    /// folds into the hash but moves no conviction). `Some` opts a world into the felt-experience-moves-belief
    /// leg, gated by the being's per-race epistemic polarity ([`crate::axiom::EpistemicStance`]).
    pub move_params: Option<ConvictionMoveParams>,
}

impl FeltConvictionCalib {
    /// A labelled DEV fixture (not owner data) for the RECORD-only leg (Branch 1, inert): a mild leak
    /// (fifteen-sixteenths) so many felt events accumulate over a life while old experience fades, matching the
    /// reward learner's eligibility-decay order, and NO move (a conviction is recorded against but not moved).
    /// The owner sets the canonical values via the calibration manifest when the learner is armed on a
    /// Calibrated world (the reserved keys, a follow-on shared with the other opt-in learners' dev-fixture
    /// calibs).
    pub fn dev_default() -> FeltConvictionCalib {
        FeltConvictionCalib {
            retention: Fixed::from_ratio(15, 16),
            move_params: None,
        }
    }

    /// A labelled DEV fixture (not owner data) for the full RECORD-and-MOVE leg (Branch 1 + Branch 2): the
    /// record retention above plus a low move gate and a moderate step, so a being whose accumulated felt
    /// association clears the gate has the relevant conviction moved by its per-race epistemic polarity. Dev
    /// values; the owner sets the canonical gate (the entrenchment curve) and step (the plasticity phenotype).
    pub fn dev_with_move() -> FeltConvictionCalib {
        FeltConvictionCalib {
            retention: Fixed::from_ratio(15, 16),
            move_params: Some(ConvictionMoveParams {
                threshold: Fixed::from_ratio(1, 100),
                plasticity: Fixed::from_ratio(1, 4),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn felt(valence: i32, intensity: Fixed) -> FeltExperience {
        FeltExperience {
            intensity,
            valence: Fixed::from_int(valence),
        }
    }

    #[test]
    fn an_empty_record_is_the_byte_neutral_opt_out() {
        let e = ConvictionExperience::new();
        assert!(e.is_empty());
        assert_eq!(e.association(AxiomAxisId(0)), Fixed::ZERO);
        assert_eq!(e, ConvictionExperience::default());
    }

    #[test]
    fn a_felt_event_accumulates_signed_by_valence_and_scaled_by_stance_strength() {
        // The Branch-1 record: a felt-negative event on a POSITIVELY-held conviction accrues a NEGATIVE
        // association, scaled by the conviction's strength; the stronger the stance, the stronger the record.
        // Weight-agnostic by construction: the fold reads no behaviour weight at all (the R4 correction).
        let mut e = ConvictionExperience::new();
        let held = [
            (AxiomAxisId(0), Fixed::from_ratio(8, 10)), // strongly held, +pole
            (AxiomAxisId(1), Fixed::from_ratio(2, 10)), // weakly held, +pole
        ];
        e.fold(felt(-1, Fixed::ONE), &held, Fixed::ONE); // retention one to isolate the increment
        assert!(
            e.association(AxiomAxisId(0)) < Fixed::ZERO,
            "hardship while holding the +pole of conviction 0 accrues a negative association"
        );
        assert!(
            e.association(AxiomAxisId(1)) < Fixed::ZERO,
            "hardship while holding the +pole of conviction 1 accrues a negative association"
        );
        assert!(
            e.association(AxiomAxisId(0)) < e.association(AxiomAxisId(1)),
            "the more strongly held conviction accrues the stronger (more negative) association"
        );
    }

    #[test]
    fn the_association_is_pole_referenced_and_relabel_invariant() {
        // The R5 correction: the accumulator weights by the SIGNED stance, so the SAME felt-negative event
        // accrues an OPPOSITE-sign association for a being holding the -pole versus the +pole. This is what
        // makes sign(A) name "which pole was good to hold" (relabel-invariant), never an absolute pole meaning.
        let mut plus = ConvictionExperience::new();
        let mut minus = ConvictionExperience::new();
        plus.fold(
            felt(-1, Fixed::ONE),
            &[(AxiomAxisId(0), Fixed::ONE)],
            Fixed::ONE,
        );
        minus.fold(
            felt(-1, Fixed::ONE),
            &[(AxiomAxisId(0), Fixed::ZERO - Fixed::ONE)],
            Fixed::ONE,
        );
        assert!(
            plus.association(AxiomAxisId(0)) < Fixed::ZERO,
            "suffering while holding the +pole favours the -pole (negative A)"
        );
        assert!(
            minus.association(AxiomAxisId(0)) > Fixed::ZERO,
            "suffering while holding the -pole favours the +pole (positive A): the mirror, relabel-invariant"
        );
        assert_eq!(
            plus.association(AxiomAxisId(0)),
            Fixed::ZERO - minus.association(AxiomAxisId(0)),
            "the two are exact negatives: a pole relabel flips the record's sign and nothing else"
        );
    }

    #[test]
    fn the_record_is_defeasible_a_run_of_good_erodes_an_earlier_bad() {
        // Template-case defeasibility (the panel's requirement): the association is not unconditional glue. A
        // being that suffered while holding a conviction, then lives through a run of good experience, sees the
        // negative association erode back toward and through zero, because the accumulator is a LEAKY integrator
        // of the being's recent lived valence, not a permanent mark.
        let mut e = ConvictionExperience::new();
        let held = [(AxiomAxisId(0), Fixed::ONE)];
        let retention = Fixed::from_ratio(1, 2); // a brisk leak so the reversal is visible in a few events
        for _ in 0..4 {
            e.fold(felt(-1, Fixed::ONE), &held, retention);
        }
        let after_hardship = e.association(AxiomAxisId(0));
        assert!(
            after_hardship < Fixed::ZERO,
            "sustained hardship left a negative record"
        );
        for _ in 0..8 {
            e.fold(felt(1, Fixed::ONE), &held, retention);
        }
        assert!(
            e.association(AxiomAxisId(0)) > after_hardship,
            "a run of good experience eroded the earlier negative association (defeasible)"
        );
    }

    #[test]
    fn a_calm_tick_credits_nothing_but_still_leaks() {
        // A tick with no net reserve movement (valence zero) carries no felt signal, so it credits no new
        // association, but the leak still runs, so a quiet spell erodes an old record (recency, not a freeze).
        let mut e = ConvictionExperience::new();
        let held = [(AxiomAxisId(0), Fixed::ONE)];
        e.fold(felt(-1, Fixed::ONE), &held, Fixed::from_ratio(1, 2));
        let before = e.association(AxiomAxisId(0));
        e.fold(
            FeltExperience {
                intensity: Fixed::ZERO,
                valence: Fixed::ZERO,
            },
            &held,
            Fixed::from_ratio(1, 2),
        );
        let after = e.association(AxiomAxisId(0));
        assert!(
            after > before,
            "a calm tick leaked the record toward zero (before {before:?}, after {after:?})"
        );
        assert!(
            after < Fixed::ZERO,
            "but did not credit new evidence, so the sign is unchanged"
        );
    }

    #[test]
    fn an_unheld_conviction_earns_no_association() {
        // A being that holds no stance on an axis (engagement zero) forms no association for it: there is
        // nothing at stake, the honest default (no conviction, no felt coupling).
        let mut e = ConvictionExperience::new();
        let held = [(AxiomAxisId(0), Fixed::ZERO)];
        e.fold(felt(-1, Fixed::ONE), &held, Fixed::ONE);
        assert!(e.is_empty(), "a zero-stance conviction accrues nothing");
    }
}
