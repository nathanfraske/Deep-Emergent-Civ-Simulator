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

//! The runtime spine: a world of minds and a serial deterministic tick (RUNBOOK
//! section 3, design Parts 4 and 57).
//!
//! A [`World`] owns the minds, the event log, a clock, and the calibrations the minds
//! reason under. Its [`World::tick`] advances the clock and applies a batch of stimuli
//! to the minds in one canonical order: stimuli are sorted by the target mind's
//! [`StableId`] and a caller-supplied ordinal before they are applied, so the result
//! never depends on the order the batch was assembled in. The underlying belief and
//! theory-of-mind accumulators are already order-independent, so the canonical sort is
//! belt-and-braces, and it is what a later phase relies on when perception and the
//! decision loop produce stimuli in parallel.
//!
//! This is deliberately the serial tick, not the parallel command scheduler: that
//! scheduler's determinism (the total command order and the non-associative combines)
//! is still open design (R-CMD-ORDER, R-REDUCE-ORDER), so the parallel form is left for
//! that resolution. Nothing here invents a calibration value. The minds' thresholds and
//! weights are loaded from the manifest and fail loud while reserved; a development run
//! uses a clearly-labelled fixtures profile, never the authoritative manifest's unset
//! entries.

use std::collections::BTreeMap;

use crate::agent::{AccessObs, Mind};
use crate::calibration::{CalibrationError, CalibrationManifest, Profile};
use crate::evidence::{AttrKindId, InferenceParams, ValueId};
use crate::tom::{self, AccessChannelRegistry, AccessWeights};
use civsim_core::{EventLog, Fixed, Registry, Rng, StableId, StateHasher};

/// A place in the world. Minimal for now: two minds are co-located when they share a
/// place id, which is what lets one perceive a trace or talk to another. The full
/// spatial hierarchy (design Part 6) refines this later.
pub type PlaceId = u32;

/// The RNG phase tag for a perception roll, namespacing it apart from other draws (a
/// placeholder until the phase registry of R-RNG-COORD pins the namespace).
const PHASE_PERCEPTION: u64 = 0x9001;

/// One stimulus delivered to a mind on a tick: either a first-order observation about
/// the world, or a second-order observation about a target mind's access. Phase 1
/// supplies these from a script; later phases supply them from perception and gossip.
#[derive(Clone, Debug)]
pub enum Stimulus {
    /// First-order evidence: a signed weight toward one value of a subject's attribute.
    Observe {
        /// The subject the belief is about.
        subject: StableId,
        /// Which attribute.
        attr: AttrKindId,
        /// The candidate values of the question.
        hyps: Vec<ValueId>,
        /// The value this evidence supports.
        toward: ValueId,
        /// The signed weight, before acuity scaling.
        weight: Fixed,
        /// Where the evidence came from.
        from: StableId,
    },
    /// Second-order evidence: an access observation about a target mind.
    Model {
        /// The mind being modelled.
        target: StableId,
        /// Which attribute of which subject the model is about.
        attr: AttrKindId,
        /// The candidate values of the target's belief.
        hyps: Vec<ValueId>,
        /// The access observation (channel, toward, provenance).
        obs: AccessObs,
    },
}

/// One scheduled input for a tick: which mind receives it, a caller-supplied ordinal
/// that pins its place in the canonical order, and the stimulus itself.
#[derive(Clone, Debug)]
pub struct TickInput {
    /// The mind that receives the stimulus.
    pub mind: StableId,
    /// A stable ordinal that orders inputs to the same mind deterministically.
    pub ordinal: u32,
    /// What the mind takes in.
    pub stim: Stimulus,
}

/// A perceptible, placed consequence of an event (design Part 9.9). A mind co-located
/// with a trace may perceive it and form an observed belief. The salience (a 0..1
/// perceptibility) and the belief weight are data carried from the trace kind's reserved
/// calibration; this struct is the placed instance the emitter drops into the world, so
/// the world's perception step invents no number of its own.
#[derive(Clone, Debug)]
pub struct Trace {
    /// The trace's own stable id (keys the perception roll).
    pub id: StableId,
    /// Where it sits; only co-located minds can perceive it.
    pub place: PlaceId,
    /// The subject the implied belief is about.
    pub subject: StableId,
    /// The attribute the implied belief is about.
    pub attr: AttrKindId,
    /// The candidate values of the question.
    pub hyps: Vec<ValueId>,
    /// The value perceiving the trace proposes.
    pub value: ValueId,
    /// Perceptibility in 0..1, scaled by a perceiver's acuity (data, reserved-calibrated).
    pub salience: Fixed,
    /// The belief weight a successful perception carries (data, reserved-calibrated).
    pub weight: Fixed,
    /// Provenance of the implied belief.
    pub from: StableId,
}

/// One perception success, gathered in the read pass and applied in the write pass so
/// the perception walk stays a pure read.
struct PerceptionHit {
    mind: StableId,
    subject: StableId,
    attr: AttrKindId,
    hyps: Vec<ValueId>,
    value: ValueId,
    weight: Fixed,
    from: StableId,
}

/// A world of minds advanced by a serial deterministic tick.
pub struct World {
    clock: u64,
    seed: u64,
    reg: Registry,
    minds: BTreeMap<StableId, Mind>,
    place_of: BTreeMap<StableId, PlaceId>,
    traces: Vec<Trace>,
    events: EventLog,
    /// The first-order belief calibrations (the `evidence.*` reserved values).
    belief_params: InferenceParams,
    /// The theory-of-mind calibrations (the `tom.*` reserved values).
    meta_params: InferenceParams,
    /// The data-defined access channels and their reserved weights.
    weights: AccessWeights,
}

impl World {
    /// A world with calibrations supplied directly. Tests and tools use this with
    /// clearly-labelled fixtures; production uses [`World::from_manifest`].
    pub fn new(
        belief_params: InferenceParams,
        meta_params: InferenceParams,
        weights: AccessWeights,
    ) -> Self {
        World {
            clock: 0,
            seed: 0,
            reg: Registry::new(),
            minds: BTreeMap::new(),
            place_of: BTreeMap::new(),
            traces: Vec::new(),
            events: EventLog::new(),
            belief_params,
            meta_params,
            weights,
        }
    }

    /// Set the master seed that keys every stochastic draw (perception rolls and, in
    /// later phases, gossip pairing and decisions). The seed and the world alone
    /// determine the canonical timeline (design Principle 10).
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// A world whose calibrations are loaded from the manifest under a profile. Under
    /// [`Profile::Calibrated`] this fails loud if any required value is still reserved,
    /// so production never runs on an unset number; under [`Profile::Development`] a
    /// fixtures profile supplies placeholder values so the engine can run before the
    /// owner sets the real ones.
    pub fn from_manifest(
        manifest: &CalibrationManifest,
        channels: &AccessChannelRegistry,
        profile: Profile,
    ) -> Result<Self, CalibrationError> {
        let required = [
            "evidence.log_odds_clamp",
            "evidence.commit_threshold",
            "evidence.runner_up_margin",
            "tom.meta_log_odds_clamp",
            "tom.meta_commit_threshold",
            "tom.meta_runner_up_margin",
        ];
        manifest.gate(profile, &required)?;
        let belief_params = InferenceParams::from_manifest(manifest)?;
        let meta_params = tom::meta_params_from_manifest(manifest)?;
        let weights = AccessWeights::from_manifest(channels, manifest)?;
        Ok(World::new(belief_params, meta_params, weights))
    }

    /// The current tick.
    pub fn clock(&self) -> u64 {
        self.clock
    }

    /// How many minds the world holds.
    pub fn population(&self) -> usize {
        self.minds.len()
    }

    /// The event log, for inspection (nothing emits into it until perception and the
    /// decision loop land in later phases).
    pub fn events(&self) -> &EventLog {
        &self.events
    }

    /// Create a mind with the given acuity, minting a fresh never-reused id.
    pub fn spawn(&mut self, acuity: Fixed) -> StableId {
        let id = self.reg.mint();
        self.minds.insert(id, Mind::new(id, acuity));
        id
    }

    /// A mind by id, for inspection.
    pub fn mind(&self, id: StableId) -> Option<&Mind> {
        self.minds.get(&id)
    }

    /// Place a mind. Two minds in the same place are co-located, which is the condition
    /// for perceiving a shared trace and (in later phases) for talking.
    pub fn set_place(&mut self, mind: StableId, place: PlaceId) {
        self.place_of.insert(mind, place);
    }

    /// Where a mind is, if it has been placed.
    pub fn place_of(&self, mind: StableId) -> Option<PlaceId> {
        self.place_of.get(&mind).copied()
    }

    /// Drop a perceptible trace into the world. Co-located minds may perceive it on a
    /// later tick. The trace carries its own salience and weight as data; the world adds
    /// no number of its own.
    pub fn emit_trace(&mut self, trace: Trace) {
        self.traces.push(trace);
    }

    /// How many traces are currently in the world.
    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }

    /// The belief calibrations the world reasons under.
    pub fn belief_params(&self) -> &InferenceParams {
        &self.belief_params
    }

    /// The theory-of-mind calibrations the world reasons under.
    pub fn meta_params(&self) -> &InferenceParams {
        &self.meta_params
    }

    /// Advance one tick: the clock steps, then the batch of stimuli is applied to the
    /// minds in canonical order (by target id, then ordinal), so the resulting state is
    /// independent of the order the batch was assembled in. A stimulus for an unknown
    /// mind is ignored.
    pub fn tick(&mut self, inputs: &[TickInput]) {
        self.clock += 1;
        let mut ordered: Vec<&TickInput> = inputs.iter().collect();
        ordered.sort_by_key(|i| (i.mind, i.ordinal));
        for input in ordered {
            let weights = &self.weights;
            if let Some(mind) = self.minds.get_mut(&input.mind) {
                match &input.stim {
                    Stimulus::Observe {
                        subject,
                        attr,
                        hyps,
                        toward,
                        weight,
                        from,
                    } => {
                        mind.consider(
                            *subject,
                            *attr,
                            hyps.iter().copied(),
                            *toward,
                            *weight,
                            *from,
                        );
                    }
                    Stimulus::Model {
                        target,
                        attr,
                        hyps,
                        obs,
                    } => {
                        // The nested write path refuses anything but access about the
                        // target, so a rejected stimulus simply does not move the model.
                        let _ = mind.model(weights, *target, *attr, hyps.iter().copied(), *obs);
                    }
                }
            }
        }
        self.perceive();
    }

    /// The perception step (design Part 9.9): each co-located mind rolls against each
    /// trace's salience scaled by its own acuity, and on success forms an observed
    /// belief. Traces are walked in id order and minds in id order, and each roll is
    /// keyed on counter-based RNG over the seed, the trace, the perceiver, the tick, and
    /// the perception phase, so the result is bit-identical on replay and independent of
    /// thread count. A two-pass shape (decide, then apply) keeps the walk a pure read.
    fn perceive(&mut self) {
        let hits: Vec<PerceptionHit> = {
            let mut traces: Vec<&Trace> = self.traces.iter().collect();
            traces.sort_by_key(|t| t.id);
            let mut out = Vec::new();
            for t in traces {
                for (mind_id, mind) in &self.minds {
                    if self.place_of.get(mind_id) != Some(&t.place) {
                        continue;
                    }
                    let chance = t.salience.mul(mind.acuity).clamp(Fixed::ZERO, Fixed::ONE);
                    let roll = Rng::for_coords(
                        self.seed,
                        &[t.id.0, mind_id.0, self.clock, PHASE_PERCEPTION],
                    )
                    .unit_fixed(0);
                    if roll < chance {
                        out.push(PerceptionHit {
                            mind: *mind_id,
                            subject: t.subject,
                            attr: t.attr,
                            hyps: t.hyps.clone(),
                            value: t.value,
                            weight: t.weight,
                            from: t.from,
                        });
                    }
                }
            }
            out
        };
        for hit in hits {
            if let Some(mind) = self.minds.get_mut(&hit.mind) {
                mind.consider(
                    hit.subject,
                    hit.attr,
                    hit.hyps.iter().copied(),
                    hit.value,
                    hit.weight,
                    hit.from,
                );
            }
        }
    }

    /// A canonical 128-bit hash of the whole world: the clock, the id registry, the
    /// event log length, then every mind in id order. A pure function of canonical
    /// state, so a replay reproduces it bit for bit.
    pub fn state_hash(&self) -> u128 {
        let mut h = StateHasher::new();
        h.write_u64(self.clock);
        h.write_u64(self.seed);
        self.reg.hash_into(&mut h);
        h.write_u64(self.events.len() as u64);
        for (id, mind) in &self.minds {
            h.write_stable(*id);
            if let Some(place) = self.place_of.get(id) {
                h.write_u32(*place);
            } else {
                h.write_u32(u32::MAX);
            }
            // Fold each mind's own canonical state hash in as a 128-bit value.
            let mh = mind.state_hash(&self.belief_params, &self.meta_params);
            h.write_u64(mh as u64);
            h.write_u64((mh >> 64) as u64);
        }
        // Active traces, in id order.
        let mut traces: Vec<&Trace> = self.traces.iter().collect();
        traces.sort_by_key(|t| t.id);
        for t in traces {
            h.write_stable(t.id);
            h.write_u32(t.place);
            h.write_stable(t.subject);
            h.write_u32(t.attr.0);
            h.write_u32(t.value);
            h.write_fixed(t.salience);
            h.write_fixed(t.weight);
        }
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> InferenceParams {
        InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        }
    }

    fn world() -> World {
        World::new(params(), params(), AccessWeights::from_pairs([]))
    }

    #[test]
    fn spawn_mints_distinct_ids_and_counts_population() {
        let mut w = world();
        let a = w.spawn(Fixed::ONE);
        let b = w.spawn(Fixed::ONE);
        assert_ne!(a, b);
        assert_eq!(w.population(), 2);
        assert!(w.mind(a).is_some());
    }

    #[test]
    fn a_tick_applies_observations_and_advances_the_clock() {
        let mut w = world();
        let anna = w.spawn(Fixed::ONE);
        let marble = StableId(99);
        w.tick(&[TickInput {
            mind: anna,
            ordinal: 0,
            stim: Stimulus::Observe {
                subject: marble,
                attr: AttrKindId(0),
                hyps: vec![10, 20],
                toward: 10,
                weight: Fixed::from_int(4),
                from: anna,
            },
        }]);
        assert_eq!(w.clock(), 1);
        assert_eq!(
            w.mind(anna)
                .unwrap()
                .belief(marble, AttrKindId(0), w.belief_params()),
            Some(10)
        );
    }

    #[test]
    fn within_a_tick_input_order_does_not_change_the_world() {
        let marble = StableId(99);
        let build = |reversed: bool| -> u128 {
            let mut w = world();
            let anna = w.spawn(Fixed::ONE);
            let mk = |ordinal: u32, toward: ValueId, weight: i32| TickInput {
                mind: anna,
                ordinal,
                stim: Stimulus::Observe {
                    subject: marble,
                    attr: AttrKindId(0),
                    hyps: vec![10, 20],
                    toward,
                    weight: Fixed::from_int(weight),
                    from: anna,
                },
            };
            let mut batch = vec![mk(0, 10, 4), mk(1, 20, 2), mk(2, 10, 3)];
            if reversed {
                batch.reverse();
            }
            w.tick(&batch);
            w.state_hash()
        };
        assert_eq!(build(false), build(false), "replay reproduces the world");
        assert_eq!(
            build(false),
            build(true),
            "a tick is independent of the batch assembly order"
        );
    }

    #[test]
    fn from_manifest_fails_loud_under_calibrated_while_reserved() {
        // The authoritative manifest with everything reserved must refuse to start a
        // calibrated world, so production never runs on an unset number.
        let toml = r#"
[[reserved]]
id = "evidence.log_odds_clamp"
basis = "x"
status = "reserved"
source = "Part 9"
"#;
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        let chans = AccessChannelRegistry::default();
        assert!(World::from_manifest(&m, &chans, Profile::Calibrated).is_err());
    }

    fn trace(place: PlaceId, value: ValueId, salience: Fixed) -> Trace {
        Trace {
            id: StableId(500),
            place,
            subject: StableId(99),
            attr: AttrKindId(0),
            hyps: vec![10, 20],
            value,
            salience,
            weight: Fixed::from_int(5),
            from: StableId(500),
        }
    }

    #[test]
    fn co_located_minds_perceive_a_trace_and_others_do_not() {
        let mut w = world().with_seed(0x5EED);
        let (here, elsewhere) = (1u32, 2u32);
        let anna = w.spawn(Fixed::ONE);
        let boris = w.spawn(Fixed::ONE);
        w.set_place(anna, here);
        w.set_place(boris, elsewhere);
        // Salience 1 and acuity 1 give a certain perception for the co-located mind.
        w.emit_trace(trace(here, 10, Fixed::ONE));
        w.tick(&[]);
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(anna)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            Some(10),
            "the co-located mind perceived the trace"
        );
        assert_eq!(
            w.mind(boris)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            None,
            "a mind elsewhere perceived nothing"
        );
    }

    #[test]
    fn an_imperceptible_trace_is_missed() {
        let mut w = world().with_seed(7);
        let anna = w.spawn(Fixed::ONE);
        w.set_place(anna, 1);
        // Salience 0 gives a zero chance, so the trace is never perceived.
        w.emit_trace(trace(1, 10, Fixed::ZERO));
        w.tick(&[]);
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(anna)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            None
        );
    }

    #[test]
    fn the_perception_roll_replays_deterministically() {
        // A middling chance exercises the stochastic path; it must replay bit for bit.
        let build = || {
            let mut w = world().with_seed(0xABCD);
            let a = w.spawn(Fixed::from_ratio(1, 2));
            w.set_place(a, 1);
            w.emit_trace(trace(1, 10, Fixed::from_ratio(1, 2)));
            w.tick(&[]);
            w.state_hash()
        };
        assert_eq!(build(), build());
    }
}
