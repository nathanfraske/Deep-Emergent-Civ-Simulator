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

use crate::agent::{AccessObs, Mind, SharedBelief};
use crate::calibration::{CalibrationError, CalibrationManifest, Profile};
use crate::decision::{ActionId, Behaviour, DriveId};
use crate::evidence::{AttrKindId, InferenceParams, ValueId};
use crate::language::{
    ConceptId, DriftParams, FormSystem, LangId, Language, LanguageParams, Lexicon, Word,
};
use crate::tom::{self, AccessChannelRegistry, AccessWeights};
use civsim_core::{DrawKey, EventLog, Fixed, Phase, Registry, StableId, StateHasher};

/// A place in the world. Minimal for now: two minds are co-located when they share a
/// place id, which is what lets one perceive a trace or talk to another. The full
/// spatial hierarchy (design Part 6) refines this later.
pub type PlaceId = u32;

/// The conventional access channel name a spoken belief travels through, used by the
/// gossip step to update the hearer's model of the speaker. If the data registry defines
/// no such channel, the model update is skipped (the first-order belief still passes).
const SAID_CHANNEL: &str = "said";

/// The reserved calibrations the gossip loop needs. Read from the manifest; until set,
/// reading them fails loud rather than running on a fabricated default.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GossipParams {
    /// The belief weight a heard assertion carries, before trust scaling.
    pub told_weight: Fixed,
    /// The trust a fresh listener extends to a speaker (a 0..1 multiplier on the weight).
    pub trust_baseline: Fixed,
    /// How much trust drops when the listener sees through a speaker's lie.
    pub trust_penalty: Fixed,
}

impl GossipParams {
    /// Read the gossip calibrations from the manifest, failing loud while reserved.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<Self, CalibrationError> {
        Ok(GossipParams {
            told_weight: m.require_fixed("gossip.told_weight")?,
            trust_baseline: m.require_fixed("gossip.trust_baseline")?,
            trust_penalty: m.require_fixed("gossip.trust_penalty")?,
        })
    }
}

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

/// One gossip exchange, gathered in the read pass and applied in the write pass so the
/// deception check reads the model before it is updated.
struct GossipAction {
    listener: StableId,
    speaker: StableId,
    shared: SharedBelief,
    deception: bool,
    trust: Fixed,
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
    /// The data-driven decision definitions (drives, curves, actions). None until set.
    behaviour: Option<Behaviour>,
    /// Per-mind drive levels, in the unit interval.
    drive_levels: BTreeMap<StableId, BTreeMap<DriveId, Fixed>>,
    /// The action each mind chose on the last tick, for inspection.
    last_action: BTreeMap<StableId, ActionId>,
    /// The data-defined access channels (for resolving the spoken channel in gossip).
    channels: AccessChannelRegistry,
    /// The gossip calibrations. None until set; gossip is then a no-op.
    gossip: Option<GossipParams>,
    /// Per-pair trust, keyed (listener, speaker): a 0..1 multiplier on a heard weight.
    trust: BTreeMap<(StableId, StableId), Fixed>,
    /// Per-mind lexicons (concept to word).
    lexicons: BTreeMap<StableId, Lexicon>,
    /// The concepts a band coordinates words for (data).
    concepts: Vec<ConceptId>,
    /// The language lineages, by id. Each carries its own articulation system, change log,
    /// and parent pointer; the naming game and drift operate per lineage. Empty until a form
    /// system is installed (which creates the default lineage), and the naming game is then a
    /// no-op.
    languages: BTreeMap<LangId, Language>,
    /// Which lineage each mind speaks. A mind with no entry speaks the default lineage.
    lang_of: BTreeMap<StableId, LangId>,
    /// The drift calibration. None until set; regular form change is then a no-op.
    drift: Option<DriftParams>,
    /// The language calibration. None until set; the naming game is then a no-op.
    language: Option<LanguageParams>,
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
            behaviour: None,
            drive_levels: BTreeMap::new(),
            last_action: BTreeMap::new(),
            channels: AccessChannelRegistry::default(),
            gossip: None,
            trust: BTreeMap::new(),
            lexicons: BTreeMap::new(),
            concepts: Vec::new(),
            languages: BTreeMap::new(),
            lang_of: BTreeMap::new(),
            drift: None,
            language: None,
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
            "gossip.told_weight",
            "gossip.trust_baseline",
            "gossip.trust_penalty",
        ];
        manifest.gate(profile, &required)?;
        let belief_params = InferenceParams::from_manifest(manifest)?;
        let meta_params = tom::meta_params_from_manifest(manifest)?;
        let weights = AccessWeights::from_manifest(channels, manifest)?;
        let gossip = GossipParams::from_manifest(manifest)?;
        let mut world = World::new(belief_params, meta_params, weights);
        world.channels = channels.clone();
        world.gossip = Some(gossip);
        Ok(world)
    }

    /// Install the access-channel registry (for resolving the spoken channel in gossip).
    /// [`World::from_manifest`] does this for you; tests use it with the direct
    /// constructor.
    pub fn set_channels(&mut self, channels: AccessChannelRegistry) {
        self.channels = channels;
    }

    /// Install the gossip calibrations. Until set, the gossip step is a no-op.
    pub fn set_gossip(&mut self, params: GossipParams) {
        self.gossip = Some(params);
    }

    /// The trust a listener extends to a speaker, if any has been recorded.
    pub fn trust(&self, listener: StableId, speaker: StableId) -> Option<Fixed> {
        self.trust.get(&(listener, speaker)).copied()
    }

    /// Install the concepts a band coordinates words for (data the owner provides).
    pub fn set_concepts(&mut self, concepts: impl IntoIterator<Item = ConceptId>) {
        self.concepts = concepts.into_iter().collect();
    }

    /// Install the language calibration. Until set, the naming game is a no-op.
    pub fn set_language(&mut self, params: LanguageParams) {
        self.language = Some(params);
    }

    /// Install the articulation system words are built from (data) as the default lineage
    /// (`LangId(0)`), which every mind speaks unless assigned otherwise. Until set, the naming
    /// game is a no-op. For several lineages, use [`World::add_language`] and
    /// [`World::set_language_of`].
    pub fn set_form_system(&mut self, fs: FormSystem) {
        self.languages
            .insert(LangId(0), Language::new(LangId(0), fs));
    }

    /// Register a language lineage (its own articulation system, change log, and parent).
    pub fn add_language(&mut self, lang: Language) {
        self.languages.insert(lang.id(), lang);
    }

    /// Assign which lineage a mind speaks. Without this a mind speaks the default lineage.
    pub fn set_language_of(&mut self, mind: StableId, lang: LangId) {
        self.lang_of.insert(mind, lang);
    }

    /// Install the drift calibration. Until set, regular form change is a no-op.
    pub fn set_drift(&mut self, params: DriftParams) {
        self.drift = Some(params);
    }

    /// A language lineage by id, for inspecting its parent and change log.
    pub fn lineage(&self, id: LangId) -> Option<&Language> {
        self.languages.get(&id)
    }

    /// The lineage a mind speaks: its explicit assignment, else the default lineage, else any
    /// registered lineage (a deterministic fallback for a single-lineage world).
    fn lang_of_mind(&self, mind: StableId) -> Option<LangId> {
        if let Some(l) = self.lang_of.get(&mind) {
            return Some(*l);
        }
        if self.languages.contains_key(&LangId(0)) {
            return Some(LangId(0));
        }
        self.languages.keys().next().copied()
    }

    /// The word a mind uses for a concept, if it has settled on one.
    pub fn word_for(&self, mind: StableId, concept: ConceptId) -> Option<Word> {
        self.lexicons.get(&mind)?.word_for(concept).cloned()
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

    /// Install the data-driven decision definitions. Until this is set, the decide phase
    /// is a no-op and minds only perceive and reason; with it, minds choose actions.
    pub fn set_behaviour(&mut self, behaviour: Behaviour) {
        self.behaviour = Some(behaviour);
    }

    /// A mind's current level of a drive, or `None` if it has none.
    pub fn drive_level(&self, mind: StableId, drive: DriveId) -> Option<Fixed> {
        self.drive_levels.get(&mind)?.get(&drive).copied()
    }

    /// The action a mind chose on the last tick, if it has chosen.
    pub fn last_action(&self, mind: StableId) -> Option<ActionId> {
        self.last_action.get(&mind).copied()
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
        self.decide();
        self.gossip();
        self.converse_language();
        self.drift_languages();
    }

    /// The naming-game step (design 33.9): each co-located speaker and a chosen listener
    /// align on a word for a chosen concept. If the speaker has a word it shares it (and
    /// with the reserved innovation rate coins a fresh variant instead); if it has none
    /// it coins one. Both adopt the word, so a band converges and isolated bands diverge.
    /// The partner, the concept, the innovation roll, and the minted word form are all
    /// keyed on counter-based RNG, and speakers are walked in id order, so it replays bit
    /// for bit. A no-op until the language calibration and some concepts are set.
    fn converse_language(&mut self) {
        let lp = match self.language {
            Some(l) => l,
            None => return,
        };
        if self.languages.is_empty() || self.concepts.is_empty() {
            return;
        }
        // Applied sequentially in speaker-id order: a word coined or reused by an earlier
        // speaker is visible to a later one in the same tick, which is what drives the band
        // to consensus. Serial id order is deterministic, so this still replays bit for bit.
        let ids: Vec<StableId> = self.minds.keys().copied().collect();
        for speaker in ids {
            let place = match self.place_of.get(&speaker) {
                Some(p) => *p,
                None => continue,
            };
            let listeners: Vec<StableId> = self
                .minds
                .keys()
                .copied()
                .filter(|l| *l != speaker && self.place_of.get(l) == Some(&place))
                .collect();
            if listeners.is_empty() {
                continue;
            }
            let pair = DrawKey::entity(speaker.0, self.clock, Phase::LANGUAGE).rng(self.seed);
            let listener = listeners[pair.range_u32(0, listeners.len() as u32) as usize];
            let concept = self.concepts[pair.range_u32(1, self.concepts.len() as u32) as usize];
            let existing: Option<Word> = self
                .lexicons
                .get(&speaker)
                .and_then(|lex| lex.word_for(concept).cloned());
            let innovate = DrawKey::pair(speaker.0, concept.0 as u64, self.clock, Phase::INNOVATE)
                .rng(self.seed)
                .unit_fixed(0)
                < lp.innovation_rate;
            let word = match existing {
                Some(w) if !innovate => w,
                _ => {
                    let lang_id = match self.lang_of_mind(speaker) {
                        Some(l) => l,
                        None => continue,
                    };
                    let fs = match self.languages.get(&lang_id) {
                        Some(l) if !l.form_system().is_empty() => l.form_system(),
                        _ => continue,
                    };
                    fs.coin(
                        DrawKey::pair(speaker.0, concept.0 as u64, self.clock, Phase::COIN)
                            .rng(self.seed),
                    )
                }
            };
            self.lexicons
                .entry(speaker)
                .or_default()
                .adopt(concept, word.clone());
            self.lexicons
                .entry(listener)
                .or_default()
                .adopt(concept, word);
        }
    }

    /// The drift step (design 33.4): once per generation each lineage may innovate a regular
    /// form change, which is then applied in innovation order to every word its speakers hold,
    /// so the lineage's lexicon drifts as a unit and two separated lineages diverge into
    /// sisters. A no-op until the drift calibration is set. Deterministic: each lineage's
    /// innovation is keyed by counter RNG on the lineage, the generation, and the phase, and
    /// the speaker walk is id-ordered.
    fn drift_languages(&mut self) {
        let params = match self.drift {
            Some(p) => p,
            None => return,
        };
        if self.languages.is_empty()
            || self.clock == 0
            || !self.clock.is_multiple_of(params.generation_ticks)
        {
            return;
        }
        let generation = self.clock / params.generation_ticks;
        let lang_ids: Vec<LangId> = self.languages.keys().copied().collect();
        for lang_id in lang_ids {
            let rng = DrawKey::entity(lang_id.0 as u64, generation, Phase::DRIFT).rng(self.seed);
            let new_rules = match self.languages.get_mut(&lang_id) {
                Some(l) => l.innovate(rng, &params),
                None => continue,
            };
            if new_rules.is_empty() {
                continue;
            }
            // Speakers of this lineage, id-ordered; their converged words drift together.
            let speakers: Vec<StableId> = self
                .minds
                .keys()
                .copied()
                .filter(|m| self.lang_of_mind(*m) == Some(lang_id))
                .collect();
            for m in speakers {
                if let Some(lex) = self.lexicons.get_mut(&m) {
                    let concepts: Vec<ConceptId> = lex.entries().map(|(c, _)| *c).collect();
                    for c in concepts {
                        if let Some(mut w) = lex.word_for(c).cloned() {
                            for rule in &new_rules {
                                w = rule.apply(&w);
                            }
                            lex.adopt(c, w);
                        }
                    }
                }
            }
        }
    }

    /// The transmission step (design 9.5): each co-located speaker shares one belief with
    /// one co-located listener chosen by counter-based RNG. The listener updates its model
    /// of the speaker (the speaker said it) and, if it does not see the assertion as a lie,
    /// integrates it into its own belief at a trust-scaled weight; a seen-through lie
    /// instead lowers trust and is not integrated. Speakers are walked in id order and the
    /// deception check reads the model before the model is updated (read-old, write-new),
    /// so the step is deterministic. A no-op until the gossip calibrations are set.
    fn gossip(&mut self) {
        let gp = match self.gossip {
            Some(g) => g,
            None => return,
        };
        let actions: Vec<GossipAction> = {
            let ids: Vec<StableId> = self.minds.keys().copied().collect();
            let mut out = Vec::new();
            for &speaker in &ids {
                let place = match self.place_of.get(&speaker) {
                    Some(p) => *p,
                    None => continue,
                };
                let listeners: Vec<StableId> = ids
                    .iter()
                    .copied()
                    .filter(|l| *l != speaker && self.place_of.get(l) == Some(&place))
                    .collect();
                if listeners.is_empty() {
                    continue;
                }
                let shared = match self
                    .minds
                    .get(&speaker)
                    .and_then(|m| m.first_committed(&self.belief_params))
                {
                    Some(s) => s,
                    None => continue,
                };
                let idx = DrawKey::entity(speaker.0, self.clock, Phase::GOSSIP)
                    .rng(self.seed)
                    .range_u32(0, listeners.len() as u32) as usize;
                let listener = listeners[idx];
                let deception = self
                    .minds
                    .get(&listener)
                    .map(|m| m.detects_lie(speaker, shared.attr, shared.value, &self.meta_params))
                    .unwrap_or(false);
                let trust = self
                    .trust
                    .get(&(listener, speaker))
                    .copied()
                    .unwrap_or(gp.trust_baseline);
                out.push(GossipAction {
                    listener,
                    speaker,
                    shared,
                    deception,
                    trust,
                });
            }
            out
        };
        let said = self.channels.by_name(SAID_CHANNEL).map(|c| c.id);
        for a in actions {
            if let Some(channel) = said {
                let weights = &self.weights;
                if let Some(listener) = self.minds.get_mut(&a.listener) {
                    let _ = listener.model(
                        weights,
                        a.speaker,
                        a.shared.attr,
                        a.shared.hyps.iter().copied(),
                        AccessObs {
                            channel,
                            toward: a.shared.value,
                            from: a.speaker,
                        },
                    );
                }
            }
            if a.deception {
                let lowered = (a.trust - gp.trust_penalty).clamp(Fixed::ZERO, Fixed::ONE);
                self.trust.insert((a.listener, a.speaker), lowered);
            } else {
                let w = gp.told_weight.mul(a.trust);
                if let Some(listener) = self.minds.get_mut(&a.listener) {
                    listener.consider(
                        a.shared.subject,
                        a.shared.attr,
                        a.shared.hyps.iter().copied(),
                        a.shared.value,
                        w,
                        a.speaker,
                    );
                }
                self.trust.entry((a.listener, a.speaker)).or_insert(a.trust);
            }
        }
    }

    /// The decision step (design Part 8): each mind's drives rise, then it scores its
    /// actions and takes the highest, which reduces the drives that action satisfies. A
    /// no-op until a [`Behaviour`] is installed. Minds are walked in id order, and the
    /// choice is a deterministic argmax (lowest action id breaks ties), so it is
    /// bit-identical on replay. The behaviour is moved out for the pass so the per-mind
    /// drive maps can be borrowed mutably without conflict, then restored.
    fn decide(&mut self) {
        let behaviour = std::mem::take(&mut self.behaviour);
        if let Some(b) = &behaviour {
            let ids: Vec<StableId> = self.minds.keys().copied().collect();
            for id in ids {
                let levels = self.drive_levels.entry(id).or_default();
                for d in &b.drives {
                    let lvl = levels.entry(d.id).or_insert(Fixed::ZERO);
                    *lvl = (*lvl + d.rise_per_tick).clamp(Fixed::ZERO, Fixed::ONE);
                }
                if let Some(chosen) = b.choose(levels) {
                    self.last_action.insert(id, chosen);
                    if let Some(act) = b.action(chosen) {
                        for satisfied in &act.satisfies {
                            let amount = b
                                .drives
                                .iter()
                                .find(|d| d.id == *satisfied)
                                .map(|d| d.satisfy_amount)
                                .unwrap_or(Fixed::ZERO);
                            if let Some(lvl) = levels.get_mut(satisfied) {
                                *lvl = (*lvl - amount).clamp(Fixed::ZERO, Fixed::ONE);
                            }
                        }
                    }
                }
            }
        }
        self.behaviour = behaviour;
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
                    let roll = DrawKey::pair(mind_id.0, t.id.0, self.clock, Phase::PERCEPTION)
                        .rng(self.seed)
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
            // The mind's drive levels in drive-id order, then its last action.
            if let Some(levels) = self.drive_levels.get(id) {
                for (drive, level) in levels {
                    h.write_u32(drive.0);
                    h.write_fixed(*level);
                }
            }
            h.write_u32(self.last_action.get(id).map(|a| a.0).unwrap_or(u32::MAX));
        }
        // Active traces, in id order.
        let mut traces: Vec<&Trace> = self.traces.iter().collect();
        traces.sort_by_key(|t| t.id);
        for t in traces {
            h.write_stable(t.id);
            h.write_u32(t.place);
            h.write_stable(t.subject);
            h.write_u32(t.attr.0);
            for v in &t.hyps {
                h.write_u32(*v);
            }
            h.write_u32(t.value);
            h.write_fixed(t.salience);
            h.write_fixed(t.weight);
            h.write_stable(t.from);
        }
        // Trust edges, in (listener, speaker) order (the BTreeMap is already sorted).
        for ((listener, speaker), level) in &self.trust {
            h.write_stable(*listener);
            h.write_stable(*speaker);
            h.write_fixed(*level);
        }
        // Lexicons, by mind id then concept id.
        for (mind, lex) in &self.lexicons {
            h.write_stable(*mind);
            for (concept, word) in lex.entries() {
                h.write_u32(concept.0);
                h.write_u32(word.modality().0);
                h.write_u64(word.len() as u64);
                for seg in word.segments() {
                    h.write_u64(seg.features().len() as u64);
                    for (dim, val) in seg.features() {
                        h.write_u32(dim.0);
                        h.write_u32(val.0);
                    }
                }
            }
        }
        // Language lineages, by id: parent and the regular-form-change log.
        for (id, lang) in &self.languages {
            h.write_u32(id.0);
            h.write_u32(lang.parent().map(|p| p.0).unwrap_or(u32::MAX));
            for rule in lang.change_log() {
                h.write_u32(rule.dim.0);
                h.write_u32(rule.from.0);
                h.write_u32(rule.to.0);
                h.write_u64(rule.innovation_index);
            }
        }
        // Which lineage each mind speaks, by mind id.
        for (mind, lang) in &self.lang_of {
            h.write_stable(*mind);
            h.write_u32(lang.0);
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

    fn behaviour() -> crate::decision::Behaviour {
        use crate::decision::{ActionDef, Behaviour, Consideration, Curve, DriveDef};
        let hunger = DriveId(0);
        let fatigue = DriveId(1);
        let ramp = Curve::new([(Fixed::ZERO, Fixed::ZERO), (Fixed::ONE, Fixed::ONE)]);
        Behaviour {
            drives: vec![
                DriveDef {
                    id: hunger,
                    rise_per_tick: Fixed::from_ratio(3, 10),
                    satisfy_amount: Fixed::from_ratio(1, 2),
                },
                DriveDef {
                    id: fatigue,
                    rise_per_tick: Fixed::from_ratio(1, 10),
                    satisfy_amount: Fixed::from_ratio(1, 2),
                },
            ],
            curves: vec![ramp],
            actions: vec![
                ActionDef {
                    id: ActionId(0), // forage
                    weight: Fixed::ONE,
                    considerations: vec![Consideration {
                        drive: hunger,
                        curve: 0,
                    }],
                    satisfies: vec![hunger],
                },
                ActionDef {
                    id: ActionId(1), // rest
                    weight: Fixed::ONE,
                    considerations: vec![Consideration {
                        drive: fatigue,
                        curve: 0,
                    }],
                    satisfies: vec![fatigue],
                },
            ],
        }
    }

    #[test]
    fn a_hungry_agent_forages() {
        let mut w = world();
        w.set_behaviour(behaviour());
        let a = w.spawn(Fixed::ONE);
        w.tick(&[]);
        // Hunger rises faster than fatigue, so forage outscores rest.
        assert_eq!(w.last_action(a), Some(ActionId(0)), "the agent forages");
        // Foraging reduced hunger below its post-rise level.
        assert_eq!(w.drive_level(a, DriveId(0)), Some(Fixed::ZERO));
    }

    #[test]
    fn the_decision_loop_replays_deterministically() {
        let build = || {
            let mut w = world().with_seed(0x1234);
            w.set_behaviour(behaviour());
            let _a = w.spawn(Fixed::ONE);
            let _b = w.spawn(Fixed::from_ratio(1, 2));
            for _ in 0..5 {
                w.tick(&[]);
            }
            w.state_hash()
        };
        assert_eq!(build(), build(), "the decision loop is reproducible");
    }

    const WITNESSED: crate::tom::AccessChannelId = crate::tom::AccessChannelId(1);
    const SAID: crate::tom::AccessChannelId = crate::tom::AccessChannelId(3);

    fn gossip_world() -> World {
        let mut w = World::new(
            params(),
            params(),
            AccessWeights::from_pairs([
                (WITNESSED, Fixed::from_int(4)),
                (SAID, Fixed::from_int(2)),
            ]),
        )
        .with_seed(0x6055);
        w.set_channels(
            AccessChannelRegistry::from_toml_str(
                r#"
[[channels]]
id = 1
name = "witnessed"
[[channels]]
id = 3
name = "said"
"#,
            )
            .unwrap(),
        );
        w.set_gossip(GossipParams {
            told_weight: Fixed::from_int(3),
            trust_baseline: Fixed::ONE,
            trust_penalty: Fixed::from_ratio(1, 2),
        });
        w
    }

    fn observe_for(mind: StableId, toward: ValueId) -> TickInput {
        TickInput {
            mind,
            ordinal: 0,
            stim: Stimulus::Observe {
                subject: StableId(99),
                attr: AttrKindId(0),
                hyps: vec![10, 20],
                toward,
                weight: Fixed::from_int(5),
                from: mind,
            },
        }
    }

    #[test]
    fn a_rumour_spreads_to_a_co_located_listener() {
        let mut w = gossip_world();
        let speaker = w.spawn(Fixed::ONE);
        let listener = w.spawn(Fixed::ONE);
        w.set_place(speaker, 1);
        w.set_place(listener, 1);
        // The speaker observes a value; gossip at the end of the tick passes it on.
        w.tick(&[observe_for(speaker, 10)]);
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(speaker)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            Some(10)
        );
        assert_eq!(
            w.mind(listener)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            Some(10),
            "the rumour reached the co-located listener"
        );
    }

    #[test]
    fn a_caught_lie_lowers_trust_and_is_refused() {
        let mut w = gossip_world();
        let speaker = w.spawn(Fixed::ONE);
        let listener = w.spawn(Fixed::ONE);
        w.set_place(speaker, 1);
        w.set_place(listener, 1);
        // In one tick: the speaker comes to believe 10 (so it will assert 10), while the
        // listener witnessed that the speaker actually has access to 20. At gossip the
        // listener sees the assertion as a lie, refuses it, and lowers trust.
        w.tick(&[
            observe_for(speaker, 10),
            TickInput {
                mind: listener,
                ordinal: 0,
                stim: Stimulus::Model {
                    target: speaker,
                    attr: AttrKindId(0),
                    hyps: vec![10, 20],
                    obs: AccessObs {
                        channel: WITNESSED,
                        toward: 20,
                        from: listener,
                    },
                },
            },
        ]);
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(listener)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            None,
            "the listener refused the lie"
        );
        assert_eq!(
            w.trust(listener, speaker),
            Some(Fixed::from_ratio(1, 2)),
            "trust dropped by the penalty"
        );
    }

    #[test]
    fn the_band_replays_deterministically() {
        let build = || {
            let mut w = gossip_world();
            let a = w.spawn(Fixed::ONE);
            let b = w.spawn(Fixed::ONE);
            let c = w.spawn(Fixed::ONE);
            for m in [a, b, c] {
                w.set_place(m, 1);
            }
            w.tick(&[observe_for(a, 10)]);
            for _ in 0..5 {
                w.tick(&[]);
            }
            w.state_hash()
        };
        assert_eq!(build(), build(), "the band's gossip replays bit for bit");
    }

    fn language_world() -> World {
        use crate::language::{ArticulationSubstrate, LanguageParams};
        let mut w = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(0xABBA);
        w.set_concepts([ConceptId(1)]);
        let (_substrate, forms) = ArticulationSubstrate::syllabic(
            ["ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo"].map(String::from),
            2,
            3,
        );
        w.set_form_system(forms);
        // Innovation off, so a band converges cleanly to one coined word.
        w.set_language(LanguageParams {
            innovation_rate: Fixed::ZERO,
        });
        w
    }

    #[test]
    fn two_isolated_bands_grow_different_words_for_one_concept() {
        let mut w = language_world();
        let band_a: Vec<StableId> = (0..3).map(|_| w.spawn(Fixed::ONE)).collect();
        let band_b: Vec<StableId> = (0..3).map(|_| w.spawn(Fixed::ONE)).collect();
        for &m in &band_a {
            w.set_place(m, 1);
        }
        for &m in &band_b {
            w.set_place(m, 2);
        }
        for _ in 0..40 {
            w.tick(&[]);
        }
        let c = ConceptId(1);
        let wa = w.word_for(band_a[0], c);
        let wb = w.word_for(band_b[0], c);
        assert!(wa.is_some() && wb.is_some(), "each band coined a word");
        // Each band converged internally.
        for &m in &band_a {
            assert_eq!(w.word_for(m, c), wa, "band A shares one word");
        }
        for &m in &band_b {
            assert_eq!(w.word_for(m, c), wb, "band B shares one word");
        }
        // The two isolated bands coined different words: language is emergent.
        assert_ne!(wa, wb, "isolated bands diverged");
    }

    #[test]
    fn the_naming_game_replays_deterministically() {
        let build = || {
            let mut w = language_world();
            let ids: Vec<StableId> = (0..4).map(|_| w.spawn(Fixed::ONE)).collect();
            for &m in &ids {
                w.set_place(m, 1);
            }
            for _ in 0..20 {
                w.tick(&[]);
            }
            w.state_hash()
        };
        assert_eq!(build(), build(), "the naming game replays bit for bit");
    }
}
