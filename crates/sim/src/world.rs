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

use std::collections::{BTreeMap, BTreeSet};

use crate::affect::{AffectAxisId, AffectState, AppraisalBinding};
use crate::agent::{AccessObs, Mind, SharedBelief};
use crate::axiom::{self, Axiom, AxiomAxisId, EvidenceRing, IntrinsicBeliefs};
use crate::calibration::{CalibrationError, CalibrationManifest, Profile};
use crate::decision::{ActionId, Behaviour, Curve, DriveId};
use crate::dialogue::{
    ContentRef, EffectSign, ForceFloor, ForceKind, Move, MoveKindId, MoveRegistry, ResolvedBand,
};
use crate::evidence::{AttrKindId, InferenceParams, ValueId};
use crate::genome::Genome;
use crate::language::{
    ConceptId, DriftParams, FormSystem, LangId, Language, LanguageParams, Lexicon, Word,
};
use crate::race::{BandSpec, Race};
use crate::sensorium::{SenseChannelId, Sensorium};
use crate::tom::{self, AccessChannelRegistry, AccessWeights};
use crate::value::RaceId;
use civsim_core::{
    CommandBuffer, CommandKey, DrawKey, EventId, EventLog, Fixed, Phase, Registry, StableId,
    StateHasher,
};

/// A place in the world. Minimal for now: two minds are co-located when they share a
/// place id, which is what lets one perceive a trace or talk to another. The full
/// spatial hierarchy (design Part 6) refines this later.
pub type PlaceId = u32;

/// The conventional access channel name a spoken belief travels through, used by the
/// gossip step to update the hearer's model of the speaker. If the data registry defines
/// no such channel, the model update is skipped (the first-order belief still passes).
const SAID_CHANNEL: &str = "said";

/// The CONVERSE-phase draw slot for choosing a move's addressee, namespaced so it cannot
/// collide with a future move-scoped draw on counter zero (the R-RNG-COORD slot rule).
const SLOT_ADDRESSEE: u32 = 0;

/// Read a felicity dimension from the world state the engine already carries (design Part
/// 9.5). The dialogue step gates a move on these readings; a dimension the world does not
/// yet model (an institutional role, a value distance, a channel reach) reads as `None`,
/// so a condition over it misfires until the subsystem that carries it is built, never on
/// a fabricated value. Trust is the one dimension modelled today.
fn felicity_reading(
    dim: &str,
    trust: &BTreeMap<(StableId, StableId), Fixed>,
    listener: StableId,
    speaker: StableId,
) -> Option<Fixed> {
    match dim {
        "trust" => trust.get(&(listener, speaker)).copied(),
        _ => None,
    }
}

/// Build an assertion move from a speaker to one listener, carrying a belief question as
/// its content. The ordinal and in-reply-to are filled in the write pass.
fn assertion_move(
    force: MoveKindId,
    speaker: StableId,
    listener: StableId,
    shared: &SharedBelief,
    channel: crate::tom::AccessChannelId,
    tick: u64,
) -> Move {
    Move {
        force,
        speaker,
        addressees: vec![listener],
        content: ContentRef::Belief {
            subject: shared.subject,
            attr: shared.attr,
        },
        in_reply_to: None,
        channel,
        tick,
        ordinal: 0,
    }
}

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
    /// The physical channel this trace travels on (the R-SENSORIUM channel gate). A being
    /// perceives it only if its sensorium reads this channel; a being with no installed
    /// sensorium reads every channel, so a trace on [`SenseChannelId::DEFAULT`] is perceived
    /// by everyone co-located, the back-compatible default.
    pub channel: SenseChannelId,
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

/// The installed modelled-dialogue substrate (design Part 9.5): the move registry, the
/// etic force floor, and the resolved felicity bands. Content-gated at install, so a
/// malformed substrate fails loud rather than running. None until set, and the dialogue
/// step is then a no-op; the one-pass gossip loop is the fallback for everyone the move
/// log does not cover.
struct DialogueConfig {
    registry: MoveRegistry,
    floor: ForceFloor,
    /// Resolved felicity band bounds, keyed by band name. Empty until the owner sets the
    /// reserved bounds; a felicity condition whose band is unresolved misfires (the move
    /// lands as a bare attempt), so no fabricated default is ever used.
    bands: BTreeMap<String, ResolvedBand>,
}

/// The [`CommandKey`] kind discriminant for a dialogue-move append command. This is the
/// command kind (every dialogue move is one "append this move" command), not the move's
/// own `MoveKindId`, which rides in the move itself; keeping one command kind for the
/// whole turn keeps a turn's moves contiguous in the total order, so a response always
/// follows the move it answers.
const CMD_DIALOGUE: u32 = 1;

/// One recorded dialogue move gathered in the converse read pass and appended to the log
/// in a second pass, so the read walk stays pure (the shape the perception and gossip
/// steps already use). Each move is keyed by [`CommandKey`] under its turn owner, and
/// the write pass applies the moves in total key order (R-CMD-ORDER, design Part 4.3).
struct PendingMove {
    mv: Move,
    /// The move this one answers, by its emission ordinal within the same turn (the
    /// same turn owner), so the in-reply-to event id can be resolved from the key map
    /// once the answered move has been appended. A turn's moves are contiguous and
    /// ordinal-ordered in the total order, so the answered move always precedes this one.
    answers: Option<usize>,
    /// Whether this move's content should point at the move it answers (true for an
    /// acceptance or refusal, which are about the prior move; false for an answer to a
    /// question, which carries its own belief content).
    reply_as_prior: bool,
    /// The first-order and theory-of-mind effects to apply when the move lands.
    effect: MoveEffect,
}

/// What a recorded move does to canonical state when it lands, drawn from the etic force
/// floor (design Part 9.5). Each variant is a call into a mechanism the engine already
/// has; the converse step composes them, authoring no new behaviour.
enum MoveEffect {
    /// A move whose felicity conditions failed: it is recorded as a bare attempt but
    /// lands no force (the Austin misfire made structural).
    Misfire,
    /// A told-evidence assertion: the listener integrates the belief (gated by the
    /// deception verdict) and models the speaker as having said it.
    Assert {
        listener: StableId,
        speaker: StableId,
        shared: SharedBelief,
        deception: bool,
        trust: Fixed,
    },
    /// An uptake: the original speaker models the listener's response as said evidence
    /// about whether the listener took up the claim (positive) or not (negative).
    Uptake {
        speaker: StableId,
        listener: StableId,
        shared: SharedBelief,
        sign: EffectSign,
    },
    /// A question: it seeds the inquiry goal in the hearer (design 9.13), so being asked
    /// makes the hearer wonder the question too. It moves no belief.
    Inquire {
        hearer: StableId,
        subject: StableId,
        attr: AttrKindId,
    },
}

/// A world of minds advanced by a serial deterministic tick.
pub struct World {
    clock: u64,
    seed: u64,
    reg: Registry,
    minds: BTreeMap<StableId, Mind>,
    place_of: BTreeMap<StableId, PlaceId>,
    /// Per-being genome, the inheritance a member was seeded or born with (design Part 25).
    /// Populated at the dawn by [`World::seed_dawn_populations`].
    genomes: BTreeMap<StableId, Genome>,
    /// Per-being intrinsic beliefs, the innate disposition seeded at the dawn (design Part 28).
    intrinsic: BTreeMap<StableId, IntrinsicBeliefs>,
    /// Per-being transient affect, the event-driven emotional state (the R-EMOTION gap).
    affect: BTreeMap<StableId, AffectState>,
    /// Per-being age in life-cadence ticks, for the aging-and-mortality loop (the R-AGING gap).
    ages: BTreeMap<StableId, u32>,
    /// Per-being sensorium, the channels it can perceive (the R-SENSORIUM channel gate). A being
    /// with no entry reads every channel, so perception is gated only where a sensorium is set.
    sensorium: BTreeMap<StableId, Sensorium>,
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
    /// The installed dialogue substrate. None until set; the dialogue step is then a no-op.
    dialogue: Option<DialogueConfig>,
    /// The minds promoted to move-by-move dialogue (design Part 54). The dialogue step runs
    /// only for a promoted speaker, and the gossip fallback skips it; empty means nobody is
    /// promoted, so dialogue is inert and gossip covers everyone.
    promoted: BTreeSet<StableId>,
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
    /// The execution width of the parallel read stage (the tick's ActionStage, design
    /// Part 4.1): how many worker threads compute speaker turns concurrently. This is a
    /// non-canonical execution parameter, never hashed and proven unable to change any
    /// canonical outcome, because the produced commands are re-ordered at the barrier by
    /// [`CommandKey`] before application (R-CMD-ORDER; the determinism harness asserts
    /// bit-identity across a worker sweep). 1 means serial.
    workers: usize,
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
            genomes: BTreeMap::new(),
            intrinsic: BTreeMap::new(),
            affect: BTreeMap::new(),
            ages: BTreeMap::new(),
            sensorium: BTreeMap::new(),
            traces: Vec::new(),
            behaviour: None,
            drive_levels: BTreeMap::new(),
            last_action: BTreeMap::new(),
            channels: AccessChannelRegistry::default(),
            gossip: None,
            dialogue: None,
            promoted: BTreeSet::new(),
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
            workers: 1,
        }
    }

    /// Set the execution width of the parallel read stage (the ActionStage worker
    /// count). Purely an execution choice: the canonical result is proven identical for
    /// any width, because the barrier re-orders the produced commands by [`CommandKey`]
    /// before any of them applies (R-CMD-ORDER). Clamped to at least 1.
    pub fn set_workers(&mut self, workers: usize) {
        self.workers = workers.max(1);
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

    /// Install the modelled-dialogue substrate (the move registry and the etic force
    /// floor). Content-gated at install: a malformed substrate is refused rather than run
    /// (design Part 9.5, Part 41). The dialogue step stays a no-op until both this and the
    /// gossip calibrations are set and some minds are promoted, since a move's magnitude
    /// reuses the reserved gossip told-weight. The felicity bands start empty; set them
    /// with [`World::set_felicity_band`] once the owner's reserved bounds are known.
    pub fn set_dialogue(
        &mut self,
        registry: MoveRegistry,
        floor: ForceFloor,
    ) -> Result<(), crate::dialogue::ContentGateError> {
        registry.content_gate(&floor)?;
        self.dialogue = Some(DialogueConfig {
            registry,
            floor,
            bands: BTreeMap::new(),
        });
        Ok(())
    }

    /// Supply a resolved felicity band (the owner's reserved bounds for one band key). A
    /// felicity condition whose band is not supplied misfires, so this never invents a
    /// default; it is the route by which a set reserved value reaches the dialogue gate.
    pub fn set_felicity_band(&mut self, band: impl Into<String>, bounds: ResolvedBand) {
        if let Some(d) = &mut self.dialogue {
            d.bands.insert(band.into(), bounds);
        }
    }

    /// Promote a mind to move-by-move dialogue (design Part 54). A promoted speaker runs
    /// the dialogue step and is skipped by the one-pass gossip fallback, so it is not
    /// double-counted. Promotion is the significance gate; the per-tick budget and the
    /// promotion thresholds are reserved owner values, so a tool promotes explicitly.
    pub fn promote(&mut self, mind: StableId) {
        self.promoted.insert(mind);
    }

    /// Whether a mind is promoted to move-by-move dialogue.
    pub fn is_promoted(&self, mind: StableId) -> bool {
        self.promoted.contains(&mind)
    }

    /// Seed an open question a mind is motivated to resolve (an inquiry goal of design
    /// 9.13). A being that wonders a question it cannot answer will ask a co-located peer
    /// in the dialogue step; the answer, if a peer holds it, grounds back. Being asked
    /// seeds the same goal in the hearer, so curiosity spreads through a conversation.
    pub fn set_wondering(&mut self, mind: StableId, subject: StableId, attr: AttrKindId) {
        if let Some(m) = self.minds.get_mut(&mind) {
            m.wonder(subject, attr);
        }
    }

    /// Whether a mind still has this question open: it is curious about it and has not yet
    /// committed a belief, so it would ask. Once it learns the answer the question is no
    /// longer open, so this returns false even though the curiosity was once registered.
    pub fn is_wondering(&self, mind: StableId, subject: StableId, attr: AttrKindId) -> bool {
        self.minds
            .get(&mind)
            .map(|m| {
                m.is_wondering(subject, attr)
                    && m.belief(subject, attr, &self.belief_params).is_none()
            })
            .unwrap_or(false)
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

    /// A mind's lexicon, for rendering a thought in its coined words (the legibility layer
    /// over the naming game). `None` if the mind has coined nothing yet.
    pub fn lexicon(&self, mind: StableId) -> Option<&Lexicon> {
        self.lexicons.get(&mind)
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

    /// Seed the dawn of sentience: place proto-populations of each race onto the world (design
    /// Part 28, the step that replaces the abstract civilization placement of the old worldgen
    /// pass). For each band, for each member, a fresh id is minted, a genome is promoted from
    /// the race's pool (Hardy-Weinberg sampling keyed by the new being's id, so members of a
    /// band differ genetically, design 25.8), the member's mind is expressed from that genome
    /// through the race's gene set ([`Mind::from_genome`], the cognition phenotype of Part
    /// 25.6), its innate disposition is seeded from the race ([`crate::axiom::IntrinsicBeliefs`],
    /// Part 28), and it is placed. Returns the seeded ids in seeding order. A band whose race
    /// is not in `races` is skipped.
    ///
    /// This is the convergence point of the deep being model: the map, the genome, the value
    /// substrate (Part 21), and the axiom kernel (Part 28) first run together here. It is
    /// genesis-time and deterministic: the seeding order is fixed by the band list and the
    /// member loop, so the minted ids and the genome draws keyed on them are reproducible from
    /// the seed and the inputs alone (Principle 3); being genesis-time, the id-keyed draw is
    /// not observer-influenced, so the Principle 10 caveat on allocation-order keying does not
    /// bite here. At the dawn every member of a race shares the innate belief seed; per-member
    /// divergence is the later inheritance and enculturation work. Cognition expressed from a
    /// pool-promoted genome rides the race's environment baseline and the Mendelian dominance
    /// deviations, since the quantitative breeding-value tier of the pool is a follow-on.
    pub fn seed_dawn_populations(
        &mut self,
        races: &BTreeMap<RaceId, Race>,
        bands: &[BandSpec],
    ) -> Vec<StableId> {
        let mut seeded = Vec::new();
        for band in bands {
            let Some(race) = races.get(&band.race) else {
                continue;
            };
            for _ in 0..band.members {
                let id = self.reg.mint();
                let genome = race.pool.promote(self.seed, id.0, race.ploidy());
                let mind = Mind::from_genome(id, &race.genes, &genome, race.environment);
                self.minds.insert(id, mind);
                self.genomes.insert(id, genome);
                self.intrinsic.insert(id, race.intrinsic.clone());
                self.place_of.insert(id, band.place);
                self.ages.insert(id, 0);
                seeded.push(id);
            }
        }
        seeded
    }

    /// A mind by id, for inspection.
    pub fn mind(&self, id: StableId) -> Option<&Mind> {
        self.minds.get(&id)
    }

    /// A being's genome by id, for inspection (populated at the dawn).
    pub fn genome_of(&self, id: StableId) -> Option<&Genome> {
        self.genomes.get(&id)
    }

    /// A being's intrinsic beliefs by id, for inspection (the innate disposition seeded at the
    /// dawn).
    pub fn intrinsic_of(&self, id: StableId) -> Option<&IntrinsicBeliefs> {
        self.intrinsic.get(&id)
    }

    /// Set a being's intrinsic beliefs (used by the dawn seeding, by later inheritance, and by
    /// tools and tests). The being need not already hold beliefs.
    pub fn set_intrinsic(&mut self, id: StableId, beliefs: IntrinsicBeliefs) {
        self.intrinsic.insert(id, beliefs);
    }

    /// Run one round of enculturation over a band on one axiom axis (design Part 28): each
    /// member moves its stance toward the band's confidence-weighted mean stance, anchored to
    /// its own innate seed by its effective stubbornness (the Friedkin-Johnsen rule). The mean
    /// is computed once from the members' pre-update stances (a synchronous update), in a
    /// canonical 128-bit order-independent reduction, so the round is bit-identical regardless
    /// of member order or thread count. A member that does not hold the axis is left untouched
    /// and does not enter the mean; if no member holds it (zero confidence), the round is a
    /// no-op. This is not a culture-level kernel firing: the band's profile is the derived
    /// aggregate of its members, and only members move. The bounded-confidence neighbour
    /// selection and the conformist and prestige biases (which sharpen this into schism) are
    /// the deferred next brick; this is the plain anchored average.
    pub fn enculturate_band(&mut self, members: &[StableId], axis: AxiomAxisId) {
        let mean = {
            let pairs = members.iter().filter_map(|id| {
                let intr = self.intrinsic.get(id)?;
                let ax = intr.axioms.iter().find(|a| a.axis == axis)?;
                Some((ax.stance, ax.confidence))
            });
            match axiom::confidence_weighted_mean(pairs) {
                Some(m) => m,
                None => return,
            }
        };
        for id in members {
            if let Some(intr) = self.intrinsic.get_mut(id) {
                let IntrinsicBeliefs {
                    axioms, epistemic, ..
                } = intr;
                if let Some(ax) = axioms.iter_mut().find(|a| a.axis == axis) {
                    let theta = epistemic.effective_stubbornness(ax.stubbornness);
                    ax.stance = axiom::enculturate(mean, ax.innate_seed, theta);
                }
            }
        }
    }

    /// Run one bounded-confidence enculturation round over a band on one axiom axis (design
    /// Part 28, the schism mechanism). Each member moves toward the confidence-weighted mean of
    /// only those members within the reserved confidence band `epsilon` of its own stance, then
    /// anchors to its innate seed by its effective stubbornness. Members far apart admit none of
    /// each other, so the band fractures into clusters (sects) rather than pulling to one mean,
    /// which is what produces schism. The round is synchronous (every member reads the same
    /// pre-update snapshot, the Hegselmann-Krause form) and order-independent, so it replays bit
    /// for bit. A member outside everyone's band moves only toward its own seed. The conformist
    /// and prestige transmission biases that further sharpen this are the deferred refinement;
    /// the prestige arm in particular waits on a status system.
    pub fn enculturate_band_bounded(
        &mut self,
        members: &[StableId],
        axis: AxiomAxisId,
        epsilon: Fixed,
    ) {
        let snapshot: Vec<(StableId, Fixed, Fixed)> = members
            .iter()
            .filter_map(|&id| {
                let intr = self.intrinsic.get(&id)?;
                let ax = intr.axioms.iter().find(|a| a.axis == axis)?;
                Some((id, ax.stance, ax.confidence))
            })
            .collect();
        for &id in members {
            let Some(&(_, my_stance, _)) = snapshot.iter().find(|(sid, _, _)| *sid == id) else {
                continue;
            };
            let neighbours = snapshot.iter().map(|&(_, s, c)| (s, c));
            let Some(mean) = axiom::bounded_confidence_mean(my_stance, neighbours, epsilon) else {
                continue;
            };
            if let Some(intr) = self.intrinsic.get_mut(&id) {
                let IntrinsicBeliefs {
                    axioms, epistemic, ..
                } = intr;
                if let Some(ax) = axioms.iter_mut().find(|a| a.axis == axis) {
                    let theta = epistemic.effective_stubbornness(ax.stubbornness);
                    ax.stance = axiom::enculturate(mean, ax.innate_seed, theta);
                }
            }
        }
    }

    /// The confidence-weighted variance of a band's stances on one axiom axis, the fission
    /// signal (design Part 28): a wide spread on a central axiom is a group splitting. `None`
    /// if no member holds the axis.
    pub fn axiom_variance(&self, members: &[StableId], axis: AxiomAxisId) -> Option<Fixed> {
        let pairs = members.iter().filter_map(|id| {
            let intr = self.intrinsic.get(id)?;
            let ax = intr.axioms.iter().find(|a| a.axis == axis)?;
            Some((ax.stance, ax.confidence))
        });
        axiom::confidence_weighted_variance(pairs)
    }

    /// Whether a band is fissioning on an axiom axis: its stance variance has reached the
    /// reserved fission threshold (design Part 28). A no-op axis (no holders) is not fissioning.
    pub fn is_fissioning(&self, members: &[StableId], axis: AxiomAxisId, threshold: Fixed) -> bool {
        self.axiom_variance(members, axis)
            .is_some_and(|v| v >= threshold)
    }

    /// The sects a band falls into on one axiom axis: the bounded-confidence clusters at band
    /// width `epsilon` (design Part 28). In one dimension these are the maximal runs of stances
    /// whose consecutive gaps do not exceed `epsilon`, which are exactly the connected
    /// components of the within-band influence graph. Members are gathered for the axis, sorted
    /// canonically by stance then id, and split where a gap exceeds the band, so the partition
    /// is deterministic. A band that has not fractured returns a single cluster.
    pub fn stance_clusters(
        &self,
        members: &[StableId],
        axis: AxiomAxisId,
        epsilon: Fixed,
    ) -> Vec<Vec<StableId>> {
        let mut pairs: Vec<(Fixed, StableId)> = members
            .iter()
            .filter_map(|&id| {
                let intr = self.intrinsic.get(&id)?;
                let ax = intr.axioms.iter().find(|a| a.axis == axis)?;
                Some((ax.stance, id))
            })
            .collect();
        pairs.sort();
        let mut clusters: Vec<Vec<StableId>> = Vec::new();
        let mut last: Option<Fixed> = None;
        for (stance, id) in pairs {
            let start_new = match last {
                Some(prev) => (stance - prev).abs() > epsilon,
                None => true,
            };
            if start_new {
                clusters.push(vec![id]);
            } else if let Some(c) = clusters.last_mut() {
                c.push(id);
            }
            last = Some(stance);
        }
        clusters
    }

    /// Produce a child by inheriting intrinsic beliefs from a parent and the local band (design
    /// Part 28). A fresh id is minted; for each axiom the parent holds, the child's innate seed
    /// (and its starting stance) is the heritable-plus-encultured blend of the parent's seed and
    /// the band's local mean on that axis, plus a bounded mutation drawn by counter-RNG keyed on
    /// the child's id and the axis ([`Phase::AXIOM_INHERIT`]), so a child resembles both its
    /// parent and its local culture and varies by the mutation. The heritability and mutation
    /// spread are reserved owner values supplied by the caller; the per-axis heritability of the
    /// axiom registry is the refinement. The child copies the parent's epistemic stance and
    /// value profile (their deeper inheritance is a follow-on), and each child axiom gets a
    /// fresh empty evidence ring of the parent axiom's capacity. Returns the child's id, or
    /// `None` if the parent holds no intrinsic beliefs.
    ///
    /// This is the intrinsic-belief half of a birth. The genome half (a genome from
    /// `GeneticScheme::reproduce` and a mind from [`Mind::from_genome`]) and combining the two
    /// into one birth are the integration follow-on, so the returned child carries beliefs but
    /// no mind or genome yet. Deterministic: the draw is keyed on the child's canonical id, so
    /// it is reproducible as long as birth order is a deterministic function of canonical state
    /// (an observer-driven birth path would key on a birth-event coordinate instead, the
    /// Principle 10 caveat).
    pub fn inherit_child(
        &mut self,
        parent: StableId,
        band: &[StableId],
        heritability: Fixed,
        mutation_spread: Fixed,
        generation: u64,
    ) -> Option<StableId> {
        let child = self.reg.mint();
        let beliefs = self.inherited_beliefs(
            child,
            parent,
            band,
            heritability,
            mutation_spread,
            generation,
        )?;
        self.intrinsic.insert(child, beliefs);
        Some(child)
    }

    /// The intrinsic beliefs a child of `parent` inherits, keyed on the already-minted
    /// `child` id (design Part 28). Shared by [`World::inherit_child`] and [`World::birth`]: for
    /// each axiom the parent holds, the child's innate seed (and starting stance) is the
    /// heritable-plus-encultured blend of the parent's seed and the band's local mean plus a
    /// bounded mutation drawn under [`Phase::AXIOM_INHERIT`] keyed on the child and the axis;
    /// the child copies the parent's epistemic stance and values and gets fresh evidence rings.
    /// `None` if the parent holds no intrinsic beliefs.
    fn inherited_beliefs(
        &self,
        child: StableId,
        parent: StableId,
        band: &[StableId],
        heritability: Fixed,
        mutation_spread: Fixed,
        generation: u64,
    ) -> Option<IntrinsicBeliefs> {
        let parent_beliefs = self.intrinsic.get(&parent)?;
        let mut child_axioms = Vec::with_capacity(parent_beliefs.axioms.len());
        for pax in &parent_beliefs.axioms {
            let local_mean = {
                let pairs = band.iter().filter_map(|id| {
                    let intr = self.intrinsic.get(id)?;
                    let a = intr.axioms.iter().find(|a| a.axis == pax.axis)?;
                    Some((a.stance, a.confidence))
                });
                axiom::confidence_weighted_mean(pairs).unwrap_or(pax.innate_seed)
            };
            let unit = DrawKey::pair(child.0, pax.axis.0 as u64, generation, Phase::AXIOM_INHERIT)
                .rng(self.seed)
                .unit_fixed(0);
            let seed = axiom::inherit_seed(
                pax.innate_seed,
                local_mean,
                heritability,
                mutation_spread,
                unit,
            );
            child_axioms.push(Axiom {
                axis: pax.axis,
                stance: seed,
                strength: pax.strength,
                confidence: pax.confidence,
                entrenchment: pax.entrenchment,
                salience: pax.salience,
                stubbornness: pax.stubbornness,
                innate_seed: seed,
                evidence: EvidenceRing::new(pax.evidence.cap()),
            });
        }
        Some(IntrinsicBeliefs {
            values: parent_beliefs.values.clone(),
            axioms: child_axioms,
            epistemic: parent_beliefs.epistemic.clone(),
        })
    }

    /// A full birth: a child of two parents that inherits both halves of its being (design
    /// Parts 25 and 28), the integration point where the genome and the axiom kernel meet. The
    /// child's genome is recombined from the two parents' genomes under the race's genetic
    /// scheme (`GeneticScheme::reproduce`, keyed under [`Phase::REPRODUCE`] on the parents and
    /// the generation), its mind is expressed from that genome through the race's gene set
    /// ([`Mind::from_genome`]), and its intrinsic beliefs are inherited from the first parent
    /// and the local band (the heritable-plus-encultured blend). The child is registered with a
    /// genome, a mind, and intrinsic beliefs; the caller places it. Returns the child id, or
    /// `None` if either parent has no genome or the first parent has no beliefs.
    ///
    /// Deterministic and reproducible from the seed and the inputs: the genome draws key on the
    /// parents and the generation, the belief mutation keys on the child id and the axis. The
    /// Principle 10 caveat on the child-id keying of the belief draw stands as for
    /// [`World::inherit_child`]: it is safe while birth order is a deterministic function of
    /// canonical state. The genetic scheme's reproduction mode chooses sexual recombination,
    /// haploid, or clonal; a single-parent mode ignores the second parent.
    #[allow(clippy::too_many_arguments)]
    pub fn birth(
        &mut self,
        race: &Race,
        parent_a: StableId,
        parent_b: StableId,
        band: &[StableId],
        heritability: Fixed,
        mutation_spread: Fixed,
        generation: u64,
    ) -> Option<StableId> {
        let genome_a = self.genomes.get(&parent_a)?.clone();
        let genome_b = self.genomes.get(&parent_b)?.clone();
        let child = self.reg.mint();
        let beliefs = self.inherited_beliefs(
            child,
            parent_a,
            band,
            heritability,
            mutation_spread,
            generation,
        )?;
        let child_genome = race.scheme.reproduce(
            &genome_a,
            parent_a.0,
            &genome_b,
            parent_b.0,
            race.genes.genes.len(),
            self.seed,
            generation,
        );
        let mind = Mind::from_genome(child, &race.genes, &child_genome, race.environment);
        self.minds.insert(child, mind);
        self.genomes.insert(child, child_genome);
        self.intrinsic.insert(child, beliefs);
        self.ages.insert(child, 0);
        Some(child)
    }

    /// A quiet-phase calcification pass over a band on one axiom axis (design Part 28): each
    /// member's axiom on that axis that went unchallenged this phase gains entrenchment toward
    /// the reserved cap, so an unchallenged conviction hardens across the people. The rate (the
    /// per-axis `calcify` datum) and the cap are reserved owner values. Members not holding the
    /// axis are skipped. Calcification raises the entrenchment gate, so a calcified band resists
    /// the enculturation and challenge it would once have yielded to, the labile-to-calcified
    /// transition over deep time.
    pub fn calcify_band(&mut self, members: &[StableId], axis: AxiomAxisId, rate: i32, cap: i32) {
        for id in members {
            if let Some(intr) = self.intrinsic.get_mut(id) {
                if let Some(ax) = intr.axioms.iter_mut().find(|a| a.axis == axis) {
                    ax.calcify(rate, cap);
                }
            }
        }
    }

    // --- Affect: the transient, event-driven emotional layer (the R-EMOTION gap) ---

    /// A being's transient affective state, if it has one (for inspection).
    pub fn affect_of(&self, id: StableId) -> Option<&AffectState> {
        self.affect.get(&id)
    }

    /// A being's current felt level on one affect axis (zero if the being has no affect
    /// state or has never touched the axis).
    pub fn affect_level(&self, id: StableId, axis: AffectAxisId) -> Fixed {
        self.affect
            .get(&id)
            .map(|a| a.level(axis))
            .unwrap_or(Fixed::ZERO)
    }

    /// Install a being's affective state (its baselines and any current values). A being's
    /// affect axes and baselines are properly derived from its race and genome; this is the
    /// route by which a tool, a test, or that later derivation sets them.
    pub fn set_affect(&mut self, id: StableId, state: AffectState) {
        self.affect.insert(id, state);
    }

    /// Appraise a change in one of a being's drives into affect and apply it (design Part 40,
    /// the derived-appraisal half of R-EMOTION). The race's [`AppraisalBinding`] maps the drive
    /// change to a signed delta on an affect axis (the gain and the relief sign are data), and
    /// the delta lands on the being's affect state, clamped to range. The being's affect state
    /// is created at the zero baseline if it had none. Returns the applied `(axis, delta)`, or
    /// `None` if the race does not appraise that drive. Nothing is invented: the magnitude is the
    /// measured drive change times the reserved gain the binding carries, so the engine authors
    /// no event-to-emotion reaction.
    pub fn appraise(
        &mut self,
        id: StableId,
        drive: DriveId,
        drive_change: Fixed,
        binding: &AppraisalBinding,
    ) -> Option<(AffectAxisId, Fixed)> {
        let (axis, delta) = binding.delta(drive, drive_change)?;
        self.affect.entry(id).or_default().apply(axis, delta);
        Some((axis, delta))
    }

    /// Relax one being's transient affect toward its baseline by `rate` (the deterministic
    /// fade between events; design Part 40). The rate is a reserved owner value. A no-op for a
    /// being with no affect state.
    pub fn decay_affect(&mut self, id: StableId, rate: Fixed) {
        if let Some(a) = self.affect.get_mut(&id) {
            a.decay(rate);
        }
    }

    /// Harden one being's baseline on an affect axis under a sustained strong feeling (trauma;
    /// design Part 40): if the deviation from baseline exceeds `threshold`, the baseline drifts
    /// toward the current feeling by `fraction` of the excess, leaving a residue ordinary decay
    /// no longer erases. The threshold and fraction are reserved owner values. Returns whether
    /// the baseline moved; a no-op (false) for a being with no affect state.
    pub fn harden_affect(
        &mut self,
        id: StableId,
        axis: AffectAxisId,
        threshold: Fixed,
        fraction: Fixed,
    ) -> bool {
        self.affect
            .get_mut(&id)
            .map(|a| a.harden(axis, threshold, fraction))
            .unwrap_or(false)
    }

    // --- Aging and mortality: the clock-driven life-process loop (the R-AGING gap) ---

    /// A being's age in life-cadence steps, if tracked (seeded at the dawn and at birth).
    pub fn age_of(&self, id: StableId) -> Option<u32> {
        self.ages.get(&id).copied()
    }

    /// Set a being's age (used by the dawn seeding for a founding cohort that is not newborn,
    /// and by tools and tests).
    pub fn set_age(&mut self, id: StableId, age: u32) {
        self.ages.insert(id, age);
    }

    /// Advance every tracked being's age by one life-cadence step (design Part 20). This is the
    /// life-process beat the gap names: the caller runs it once per life cadence (the cadence
    /// period in ticks is a reserved owner value, so wiring it into [`World::tick`] on a fixed
    /// period waits on that value, never a fabricated one). Aging is saturating, so a long-lived
    /// being's age never wraps.
    pub fn age_step(&mut self) {
        for age in self.ages.values_mut() {
            *age = age.saturating_add(1);
        }
    }

    /// Run one mortality pass over every tracked being against an age-hazard curve (design Part
    /// 20, the R-AGING life-process loop). For each being in id order, the curve maps its age to
    /// a per-cadence death probability (a rising-hazard curve is the data-driven default, owner
    /// supplied as `hazard`), and a counter-RNG roll keyed on the being and its age under
    /// [`Phase::MORTALITY`] decides whether it dies this cadence. The dead are removed (their
    /// per-being state pruned) and their ids returned in id order. Deterministic and
    /// observer-independent: the roll is a pure function of the seed, the being's canonical id,
    /// and its age, so a being faces the same hazard on the same age on replay and the pass is
    /// independent of thread count. The curve is evaluated in the owner's age units (age as a
    /// whole-number [`Fixed`]); the cadence period and the curve shape are reserved owner values.
    pub fn apply_mortality(&mut self, hazard: &Curve) -> Vec<StableId> {
        let dead: Vec<StableId> = self
            .ages
            .iter()
            .filter_map(|(&id, &age)| {
                let chance = hazard
                    .eval(Fixed::from_int(age as i32))
                    .clamp(Fixed::ZERO, Fixed::ONE);
                let roll = DrawKey::entity(id.0, age as u64, Phase::MORTALITY)
                    .rng(self.seed)
                    .unit_fixed(0);
                (roll < chance).then_some(id)
            })
            .collect();
        for id in &dead {
            self.remove_being(*id);
        }
        dead
    }

    /// Remove a being from the world, pruning every per-being map it appears in (the death and
    /// out-migration primitive of design Part 20). Minds, placement, genome, intrinsic beliefs,
    /// affect, age, sensorium, drives, the last action, lexicon, language assignment, the
    /// promoted set, and every trust edge naming the being are all dropped, so no dangling
    /// reference to a departed being survives (referential integrity, design Part 58). Idempotent:
    /// removing an unknown being is a no-op.
    pub fn remove_being(&mut self, id: StableId) {
        self.minds.remove(&id);
        self.place_of.remove(&id);
        self.genomes.remove(&id);
        self.intrinsic.remove(&id);
        self.affect.remove(&id);
        self.ages.remove(&id);
        self.sensorium.remove(&id);
        self.drive_levels.remove(&id);
        self.last_action.remove(&id);
        self.lexicons.remove(&id);
        self.lang_of.remove(&id);
        self.promoted.remove(&id);
        self.trust
            .retain(|(listener, speaker), _| *listener != id && *speaker != id);
    }

    // --- Sensorium: the channel gate over perception (the R-SENSORIUM gap) ---

    /// Install a being's sensorium, the channels it can perceive and its acuity on each (design
    /// Part 33.3, the R-SENSORIUM channel gate). Until a sensorium is installed a being reads
    /// every channel at full channel acuity, so perception is gated only where a sensorium is
    /// declared. A being's sensorium is properly derived from its genome and anatomy; this is
    /// the route by which that derivation, a tool, or a test sets it.
    pub fn set_sensorium(&mut self, id: StableId, sensorium: Sensorium) {
        self.sensorium.insert(id, sensorium);
    }

    /// A being's sensorium, if one has been installed (for inspection).
    pub fn sensorium_of(&self, id: StableId) -> Option<&Sensorium> {
        self.sensorium.get(&id)
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

    /// Group the placed minds by their place, in canonical mind-id order. The co-location phases
    /// (the naming game, dialogue, gossip) use this to find a speaker's neighbours in
    /// O(occupants) rather than rescanning every mind, turning each phase's inner scan from
    /// O(N^2) to O(N). The per-place lists are in mind-id order, identical to the old
    /// `minds.keys().filter(place)` scan they replace, so the draws that index into a listener list
    /// are unchanged and the tick replays bit for bit (profile-guided, Part 13; determinism
    /// preserved, Principle 3).
    fn colocated_index(&self) -> BTreeMap<PlaceId, Vec<StableId>> {
        let mut idx: BTreeMap<PlaceId, Vec<StableId>> = BTreeMap::new();
        for (&mind, &place) in &self.place_of {
            idx.entry(place).or_default().push(mind);
        }
        idx
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
        self.converse();
        self.gossip();
        self.converse_language();
        self.drift_languages();
    }

    /// A profiling aid, not part of the simulation: advance one tick with no stimuli and return the
    /// wall-clock nanoseconds spent in each of the six phases (perceive, decide, converse, gossip,
    /// converse_language, drift_languages), so a benchmark can see where a tick's time goes. It
    /// produces exactly the state `tick(&[])` would, since it runs the same phases in the same order
    /// with an empty input batch; the `Instant` it reads is non-canonical and never enters state, so
    /// the resulting hash is unchanged (Principle 3). This exists to answer "profile before
    /// optimizing" (Part 13), and it is compiled into the library only as a measurement tool.
    pub fn tick_timed(&mut self) -> [u128; 6] {
        use std::time::Instant;
        self.clock += 1;
        let mut ns = [0u128; 6];
        let s = Instant::now();
        self.perceive();
        ns[0] = s.elapsed().as_nanos();
        let s = Instant::now();
        self.decide();
        ns[1] = s.elapsed().as_nanos();
        let s = Instant::now();
        self.converse();
        ns[2] = s.elapsed().as_nanos();
        let s = Instant::now();
        self.gossip();
        ns[3] = s.elapsed().as_nanos();
        let s = Instant::now();
        self.converse_language();
        ns[4] = s.elapsed().as_nanos();
        let s = Instant::now();
        self.drift_languages();
        ns[5] = s.elapsed().as_nanos();
        ns
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
        let by_place = self.colocated_index();
        let ids: Vec<StableId> = self.minds.keys().copied().collect();
        for speaker in ids {
            let place = match self.place_of.get(&speaker) {
                Some(p) => *p,
                None => continue,
            };
            let listeners: Vec<StableId> = by_place
                .get(&place)
                .map(|v| v.iter().copied().filter(|&l| l != speaker).collect())
                .unwrap_or_default();
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

    /// The dialogue step (design Part 9.5): the promoted-tier refinement of the gossip
    /// loop. For each promoted, co-located speaker (id order) with a committed belief, it
    /// records an assertion move as a canonical event, then the addressee's response move
    /// (an acceptance, or a refusal if the addressee sees the assertion as a lie), and
    /// applies their forces through mechanisms the engine already has: a told-evidence
    /// integration gated by the deception verdict (the same magnitude gossip uses), and
    /// the speaker's theory-of-mind co-update from the response. Grounding is said
    /// evidence into the existing first-order and second-order channels with no new
    /// common-ground prior, so two parties who merely accept a thing in talk never
    /// manufacture co-witnessed common ground. A two-pass shape gathers the moves in a
    /// pure read walk, then appends them and applies their effects. Deterministic:
    /// speakers in id order, the addressee chosen by counter RNG on the CONVERSE phase,
    /// move ordinals assigned in walk order. A no-op until the dialogue substrate and the
    /// gossip calibrations are set, the said channel exists, and some minds are promoted.
    ///
    /// This is the serial form. The four determinism pins the design states for the
    /// parallel form (per-draw-site slots, the union gossip partition, the phase-frozen
    /// verdict, the barrier-ordered move-kind mint) belong to the parallel scheduler and
    /// its open cluster (R-CMD-ORDER, R-REDUCE-ORDER); on this serial tick the id-ordered
    /// walk and the resolved draw keying give determinism directly.
    fn converse(&mut self) {
        let gp = match self.gossip {
            Some(g) => g,
            None => return,
        };
        if self.dialogue.is_none() || self.promoted.is_empty() {
            return;
        }
        let said_channel = match self.channels.by_name(SAID_CHANNEL).map(|c| c.id) {
            Some(c) => c,
            None => return,
        };

        // Read pass: borrow the substrate and the minds immutably, produce owned, keyed
        // moves. Each promoted speaker's turn is computed independently over the frozen
        // state (the parallel ActionStage of design Part 4.1) and the moves re-order at
        // the barrier by CommandKey, so the production order, and therefore the worker
        // count, cannot influence the applied order (R-CMD-ORDER).
        let ordered: Vec<(CommandKey, PendingMove)> = {
            let cfg = self.dialogue.as_ref().unwrap();
            let assert_kind =
                match cfg
                    .registry
                    .first_realizing(&cfg.floor, ForceKind::TellEvidence, None)
                {
                    Some(k) => k,
                    None => return,
                };
            let accept_kind = cfg.registry.first_realizing(
                &cfg.floor,
                ForceKind::RegisterUptake,
                Some(EffectSign::Positive),
            );
            let refuse_kind = cfg.registry.first_realizing(
                &cfg.floor,
                ForceKind::RegisterUptake,
                Some(EffectSign::Negative),
            );
            let inquiry_kind =
                cfg.registry
                    .first_realizing(&cfg.floor, ForceKind::RaiseInquiry, None);
            let assert_def = cfg.registry.move_kind(assert_kind).unwrap();
            // A closure: does an assertion-kind move from `speaker` to `listener` land,
            // given the felicity reading the world carries? (No felicity in the common case.)
            let assertion_lands = |speaker: StableId, listener: StableId| -> bool {
                assert_def.felicitous(
                    |dim| felicity_reading(dim, &self.trust, listener, speaker),
                    |band| cfg.bands.get(band).copied(),
                )
            };

            let by_place = self.colocated_index();
            let ids: Vec<StableId> = self.minds.keys().copied().collect();
            let clock = self.clock;
            // One speaker's whole turn over the frozen state: a pure function of
            // (&World, speaker), since every read is immutable and every draw is keyed
            // by DrawKey, so turns can be computed by any worker in any order.
            let turn = |speaker: StableId| -> Vec<PendingMove> {
                let mut out: Vec<PendingMove> = Vec::new();
                if !self.promoted.contains(&speaker) {
                    return out;
                }
                let place = match self.place_of.get(&speaker) {
                    Some(p) => *p,
                    None => return out,
                };
                // Move-by-move dialogue needs a promoted partner; demoted neighbours are
                // covered by the one-pass gossip fallback instead.
                let peers: Vec<StableId> = by_place
                    .get(&place)
                    .map(|v| {
                        v.iter()
                            .copied()
                            .filter(|&l| l != speaker && self.promoted.contains(&l))
                            .collect()
                    })
                    .unwrap_or_default();
                if peers.is_empty() {
                    return out;
                }
                let mind = match self.minds.get(&speaker) {
                    Some(m) => m,
                    None => return out,
                };

                // INFORM: a committed belief that some peer does not, in the speaker's own
                // model, already hold. Modelling the peer is the redundancy gate: the
                // speaker stops telling a peer once its model of that peer (built from the
                // peer's said acceptances) commits to the value, so the talk converges.
                let mut informed = false;
                for shared in mind.committed_beliefs(&self.belief_params) {
                    let lacking: Vec<StableId> = peers
                        .iter()
                        .copied()
                        .filter(|l| {
                            mind.modeled_belief(*l, shared.attr, &self.meta_params)
                                != Some(shared.value)
                        })
                        .collect();
                    if lacking.is_empty() {
                        continue;
                    }
                    let idx = DrawKey::entity(speaker.0, self.clock, Phase::CONVERSE)
                        .slot(SLOT_ADDRESSEE)
                        .rng(self.seed)
                        .range_u32(0, lacking.len() as u32) as usize;
                    let listener = lacking[idx];
                    let lands = assertion_lands(speaker, listener);
                    let deception = self
                        .minds
                        .get(&listener)
                        .map(|m| {
                            m.detects_lie(speaker, shared.attr, shared.value, &self.meta_params)
                        })
                        .unwrap_or(false);
                    let trust = self
                        .trust
                        .get(&(listener, speaker))
                        .copied()
                        .unwrap_or(gp.trust_baseline);
                    let assertion_idx = out.len();
                    out.push(PendingMove {
                        mv: assertion_move(
                            assert_kind,
                            speaker,
                            listener,
                            &shared,
                            said_channel,
                            self.clock,
                        ),
                        answers: None,
                        reply_as_prior: false,
                        effect: if lands {
                            MoveEffect::Assert {
                                listener,
                                speaker,
                                shared: shared.clone(),
                                deception,
                                trust,
                            }
                        } else {
                            MoveEffect::Misfire
                        },
                    });
                    if lands {
                        let (resp_kind, sign) = if deception {
                            (refuse_kind, EffectSign::Negative)
                        } else {
                            (accept_kind, EffectSign::Positive)
                        };
                        if let Some(rk) = resp_kind {
                            out.push(PendingMove {
                                mv: Move {
                                    force: rk,
                                    speaker: listener,
                                    addressees: vec![speaker],
                                    content: ContentRef::Belief {
                                        subject: shared.subject,
                                        attr: shared.attr,
                                    },
                                    in_reply_to: None,
                                    channel: said_channel,
                                    tick: self.clock,
                                    ordinal: 0,
                                },
                                answers: Some(assertion_idx),
                                reply_as_prior: true,
                                effect: MoveEffect::Uptake {
                                    speaker,
                                    listener,
                                    shared: shared.clone(),
                                    sign,
                                },
                            });
                        }
                    }
                    informed = true;
                    break;
                }
                if informed {
                    return out;
                }

                // INQUIRE: an open question the speaker wonders about but cannot answer. It
                // asks a peer; the question seeds the inquiry goal in that peer, and if the
                // peer holds the answer it tells it back, which the asker grounds.
                let inquiry_kind = match inquiry_kind {
                    Some(k) => k,
                    None => return out,
                };
                let open = mind.open_questions(&self.belief_params);
                let (subject, attr) = match open.first() {
                    Some(q) => *q,
                    None => return out,
                };
                // This draw shares its exact key (speaker, clock, CONVERSE, addressee
                // slot) with the INFORM listener pick above; the two are mutually
                // exclusive per turn (the informed early return), so the coordinates
                // never collide. A third draw site in Phase::CONVERSE must take a
                // distinct slot (R-RNG-COORD).
                let idx = DrawKey::entity(speaker.0, self.clock, Phase::CONVERSE)
                    .slot(SLOT_ADDRESSEE)
                    .rng(self.seed)
                    .range_u32(0, peers.len() as u32) as usize;
                let listener = peers[idx];
                let question_idx = out.len();
                out.push(PendingMove {
                    mv: Move {
                        force: inquiry_kind,
                        speaker,
                        addressees: vec![listener],
                        content: ContentRef::Inquiry { subject, attr },
                        in_reply_to: None,
                        channel: said_channel,
                        tick: self.clock,
                        ordinal: 0,
                    },
                    answers: None,
                    reply_as_prior: false,
                    effect: MoveEffect::Inquire {
                        hearer: listener,
                        subject,
                        attr,
                    },
                });
                // The answer: if the asked peer holds the belief, it tells it back, and the
                // asker grounds it under the same sincerity frame the INFORM path uses. The
                // asker usually has no model of the answerer on this question (it asked
                // because it did not know), so the verdict is usually false; but if it does
                // hold an access-built model that out-ranks the answer, the answer is seen
                // through as a lie exactly as a volunteered assertion would be (Part 37,
                // Part 9.5), rather than being trusted blindly.
                if let Some(answer) = self
                    .minds
                    .get(&listener)
                    .and_then(|m| m.shared_belief(subject, attr, &self.belief_params))
                {
                    if assertion_lands(listener, speaker) {
                        let deception = self
                            .minds
                            .get(&speaker)
                            .map(|m| {
                                m.detects_lie(
                                    listener,
                                    answer.attr,
                                    answer.value,
                                    &self.meta_params,
                                )
                            })
                            .unwrap_or(false);
                        let trust = self
                            .trust
                            .get(&(speaker, listener))
                            .copied()
                            .unwrap_or(gp.trust_baseline);
                        out.push(PendingMove {
                            mv: assertion_move(
                                assert_kind,
                                listener,
                                speaker,
                                &answer,
                                said_channel,
                                self.clock,
                            ),
                            answers: Some(question_idx),
                            reply_as_prior: false,
                            effect: MoveEffect::Assert {
                                listener: speaker,
                                speaker: listener,
                                shared: answer,
                                deception,
                                trust,
                            },
                        });
                    }
                }
                out
            };

            // The barrier merge (R-CMD-ORDER): turns are computed serially or by worker
            // threads, each move keyed (tick, turn owner, CMD_DIALOGUE, ordinal within
            // the turn), and the buffer drains in total key order, so the applied order
            // is a pure function of the produced set whatever the worker count. Turn
            // owners are walked in ascending id order and a turn's moves keep their
            // emission order, so this total order coincides with the serial walk and
            // adoption changes no canonical outcome.
            let workers = self.workers.max(1);
            let mut buf = CommandBuffer::new();
            if workers == 1 {
                for &s in &ids {
                    for (ord, pm) in turn(s).into_iter().enumerate() {
                        buf.push(CommandKey::new(clock, s, CMD_DIALOGUE, ord as u64), pm);
                    }
                }
            } else {
                let turn = &turn;
                let ids = &ids;
                let parts: Vec<Vec<(CommandKey, PendingMove)>> = std::thread::scope(|sc| {
                    let handles: Vec<_> = (0..workers)
                        .map(|w| {
                            sc.spawn(move || {
                                let mut part: Vec<(CommandKey, PendingMove)> = Vec::new();
                                for (i, &s) in ids.iter().enumerate() {
                                    if i % workers == w {
                                        for (ord, pm) in turn(s).into_iter().enumerate() {
                                            part.push((
                                                CommandKey::new(clock, s, CMD_DIALOGUE, ord as u64),
                                                pm,
                                            ));
                                        }
                                    }
                                }
                                part
                            })
                        })
                        .collect();
                    handles
                        .into_iter()
                        .map(|h| h.join().expect("a turn worker panicked"))
                        .collect()
                });
                for part in parts {
                    for (k, pm) in part {
                        buf.push(k, pm);
                    }
                }
            }
            buf.into_ordered()
        };

        // Write pass (the single-threaded ActionApply barrier, design Part 4.1): append
        // the moves in total CommandKey order, resolving in-reply-to through the key map,
        // and apply their effects. The substrate borrow is released, so the &mut self
        // effect helpers are free to run.
        let mut appended: BTreeMap<CommandKey, EventId> = BTreeMap::new();
        for (ordinal, (key, mut pm)) in ordered.into_iter().enumerate() {
            if let Some(ans) = pm.answers {
                let qkey = CommandKey::new(key.tick, key.primary, key.kind, ans as u64);
                let target = *appended
                    .get(&qkey)
                    .expect("an answered move precedes its answer in the total order");
                pm.mv.in_reply_to = Some(target);
                if pm.reply_as_prior {
                    pm.mv.content = ContentRef::PriorMove { event: target };
                }
            }
            pm.mv.ordinal = ordinal as u32;
            let id = self.events.append(pm.mv.to_event());
            appended.insert(key, id);
            match pm.effect {
                MoveEffect::Misfire => {}
                MoveEffect::Assert {
                    listener,
                    speaker,
                    shared,
                    deception,
                    trust,
                } => {
                    self.apply_assertion(
                        said_channel,
                        listener,
                        speaker,
                        shared,
                        deception,
                        trust,
                        gp,
                    );
                }
                MoveEffect::Uptake {
                    speaker,
                    listener,
                    shared,
                    sign,
                } => {
                    // The response is access evidence about whether the listener took up
                    // the claim. A positive uptake models the listener as having said it
                    // (admitted under the anti-projection rule as access about the
                    // listener); a refusal records the move and moves no first-order belief.
                    if sign == EffectSign::Positive {
                        let weights = &self.weights;
                        if let Some(spk) = self.minds.get_mut(&speaker) {
                            let _ = spk.model(
                                weights,
                                listener,
                                shared.attr,
                                shared.hyps.iter().copied(),
                                AccessObs {
                                    channel: said_channel,
                                    toward: shared.value,
                                    from: listener,
                                },
                            );
                        }
                    }
                }
                MoveEffect::Inquire {
                    hearer,
                    subject,
                    attr,
                } => {
                    // Being asked seeds the inquiry goal in the hearer (design 9.13).
                    if let Some(h) = self.minds.get_mut(&hearer) {
                        h.wonder(subject, attr);
                    }
                }
            }
        }
    }

    /// Apply a landed told-evidence assertion: the listener models the speaker as having
    /// said the claim (the said channel), and either integrates the belief at the
    /// trust-scaled told-weight or, on a seen-through lie, lowers trust and refuses it.
    /// This is the gossip integration reused at the promoted tier, so a move delivers
    /// exactly the magnitude the one-pass loop would (design Part 9.5).
    #[allow(clippy::too_many_arguments)]
    fn apply_assertion(
        &mut self,
        channel: crate::tom::AccessChannelId,
        listener: StableId,
        speaker: StableId,
        shared: SharedBelief,
        deception: bool,
        trust: Fixed,
        gp: GossipParams,
    ) {
        {
            let weights = &self.weights;
            if let Some(l) = self.minds.get_mut(&listener) {
                let _ = l.model(
                    weights,
                    speaker,
                    shared.attr,
                    shared.hyps.iter().copied(),
                    AccessObs {
                        channel,
                        toward: shared.value,
                        from: speaker,
                    },
                );
            }
        }
        if deception {
            let lowered = (trust - gp.trust_penalty).clamp(Fixed::ZERO, Fixed::ONE);
            self.trust.insert((listener, speaker), lowered);
        } else {
            let w = gp.told_weight.mul(trust);
            if let Some(l) = self.minds.get_mut(&listener) {
                l.consider(
                    shared.subject,
                    shared.attr,
                    shared.hyps.iter().copied(),
                    shared.value,
                    w,
                    speaker,
                );
            }
            self.trust.entry((listener, speaker)).or_insert(trust);
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
            let by_place = self.colocated_index();
            let ids: Vec<StableId> = self.minds.keys().copied().collect();
            let mut out = Vec::new();
            for &speaker in &ids {
                // A promoted speaker runs the move-by-move dialogue step instead, so the
                // one-pass fallback must not also transmit for it (no double-counting).
                if self.promoted.contains(&speaker) {
                    continue;
                }
                let place = match self.place_of.get(&speaker) {
                    Some(p) => *p,
                    None => continue,
                };
                let listeners: Vec<StableId> = by_place
                    .get(&place)
                    .map(|v| v.iter().copied().filter(|&l| l != speaker).collect())
                    .unwrap_or_default();
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
                    // Channel gate (R-SENSORIUM): a being with an installed sensorium perceives
                    // the trace only on a channel it reads, and its channel acuity scales the
                    // roll; a being with no sensorium reads every channel at full acuity, so the
                    // place-based perception of every existing world is unchanged.
                    let channel_acuity = match self.sensorium.get(mind_id) {
                        Some(s) => match s.reads(t.channel) {
                            Some(a) => a,
                            None => continue,
                        },
                        None => Fixed::ONE,
                    };
                    let acuity = mind.acuity.mul(channel_acuity);
                    let chance = t.salience.mul(acuity).clamp(Fixed::ZERO, Fixed::ONE);
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

    /// A canonical 128-bit hash of the world's *outcome* state: the clock, the id
    /// registry, the event-log length, then every mind, trace, trust edge, lexicon, and
    /// lineage in id order. A pure function of that state, so a replay reproduces it bit
    /// for bit.
    ///
    /// Deliberate boundary: this folds the *outcomes* of dialogue (the belief, trust, and
    /// theory-of-mind state moves produce) but NOT the move log's content (only its
    /// length) and NOT the dialogue substrate, the promoted set, or the other static
    /// inputs (the behaviour, gossip, channel, and weight config). That is required, not an
    /// oversight: the Part 41 Steering Audit invariants assert that permuting the move-kind
    /// and force-effect labels (content-blindness) and swapping an equal-capacity channel
    /// leave this hash invariant, and both the move-kind id and the channel id live in the
    /// move payload, so folding the move log or the substrate here would break those
    /// invariants. The move sequence is hashed separately by [`World::event_log_hash`] for
    /// replay integrity. Do not fold the substrate or the move payload into this hash.
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
            h.write_u32(t.channel.0);
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

    /// A canonical 128-bit hash of the move sequence: every logged event in append order,
    /// folding its tick, kind, actors, subjects, and payload bytes. This is the integrity
    /// hash for the move log, kept separate from [`World::state_hash`] precisely because it
    /// is *not* content-blind (it folds the move-kind and channel ids), so it must never be
    /// used in the Steering Audit invariants. A same-seed, same-setup replay reproduces the
    /// move log byte for byte, so this catches a divergence in the move sequence that the
    /// outcome hash could miss (different moves that happen to net to the same belief state).
    pub fn event_log_hash(&self) -> u128 {
        let mut h = StateHasher::new();
        h.write_u64(self.events.len() as u64);
        for e in self.events.iter() {
            h.write_u64(e.id.0);
            h.write_u64(e.tick);
            h.write_u32(e.kind.0);
            h.write_u64(e.actors.len() as u64);
            for a in &e.actors {
                h.write_stable(*a);
            }
            h.write_u64(e.subjects.len() as u64);
            for s in &e.subjects {
                h.write_stable(*s);
            }
            h.write_bytes(&e.payload);
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
            channel: SenseChannelId::DEFAULT,
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

    // --- Modelled dialogue (the promoted-tier converse step, design Part 9.5) ---

    fn dialogue_substrate(
        felicity_on_assert: bool,
    ) -> (crate::dialogue::MoveRegistry, crate::dialogue::ForceFloor) {
        use crate::dialogue::{
            EffectSign, FelicityCond, ForceEffectDef, ForceEffectId, ForceFloor, ForceKind,
            MoveKindDef, MoveKindId, MoveRegistry,
        };
        let floor = ForceFloor {
            effects: vec![
                ForceEffectDef {
                    id: ForceEffectId(1),
                    kind: ForceKind::TellEvidence,
                    sign: EffectSign::Neutral,
                    name: "assert".to_string(),
                },
                ForceEffectDef {
                    id: ForceEffectId(2),
                    kind: ForceKind::RegisterUptake,
                    sign: EffectSign::Positive,
                    name: "accept".to_string(),
                },
                ForceEffectDef {
                    id: ForceEffectId(3),
                    kind: ForceKind::RegisterUptake,
                    sign: EffectSign::Negative,
                    name: "refuse".to_string(),
                },
                ForceEffectDef {
                    id: ForceEffectId(4),
                    kind: ForceKind::RaiseInquiry,
                    sign: EffectSign::Neutral,
                    name: "ask".to_string(),
                },
            ],
        };
        let assert_felicity = if felicity_on_assert {
            vec![FelicityCond {
                dimension: "role.command".to_string(),
                band: "felicity.assert.role".to_string(),
            }]
        } else {
            vec![]
        };
        let registry = MoveRegistry {
            moves: vec![
                MoveKindDef {
                    id: MoveKindId(1),
                    name: "assertion".to_string(),
                    force: vec![ForceEffectId(1)],
                    expects: vec![MoveKindId(2), MoveKindId(3)],
                    sincerity_judged: true,
                    felicity: assert_felicity,
                    gloss: "tells that".to_string(),
                },
                MoveKindDef {
                    id: MoveKindId(2),
                    name: "acceptance".to_string(),
                    force: vec![ForceEffectId(2)],
                    expects: vec![],
                    sincerity_judged: false,
                    felicity: vec![],
                    gloss: "agrees".to_string(),
                },
                MoveKindDef {
                    id: MoveKindId(3),
                    name: "refusal".to_string(),
                    force: vec![ForceEffectId(3)],
                    expects: vec![],
                    sincerity_judged: false,
                    felicity: vec![],
                    gloss: "declines".to_string(),
                },
                MoveKindDef {
                    id: MoveKindId(4),
                    name: "question".to_string(),
                    force: vec![ForceEffectId(4)],
                    expects: vec![MoveKindId(1)],
                    sincerity_judged: false,
                    felicity: vec![],
                    gloss: "asks".to_string(),
                },
            ],
        };
        (registry, floor)
    }

    fn dialogue_world() -> World {
        let mut w = gossip_world();
        let (reg, floor) = dialogue_substrate(false);
        w.set_dialogue(reg, floor).unwrap();
        w
    }

    #[test]
    fn a_promoted_pair_holds_a_conversation_in_the_log() {
        let mut w = dialogue_world();
        let speaker = w.spawn(Fixed::ONE);
        let listener = w.spawn(Fixed::ONE);
        w.set_place(speaker, 1);
        w.set_place(listener, 1);
        w.promote(speaker);
        w.promote(listener);
        // The speaker observes 10; the dialogue step asserts it and the listener accepts.
        w.tick(&[observe_for(speaker, 10)]);
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(listener)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            Some(10),
            "the asserted belief reached the addressee"
        );
        // The move log holds the assertion and the acceptance, one reassembled conversation.
        let first = w
            .events()
            .iter()
            .next()
            .map(|e| e.id)
            .expect("a move logged");
        let conv = crate::dialogue::conversation_of(w.events(), first, 10).unwrap();
        assert_eq!(conv.event_ids.len(), 2, "an assertion and its acceptance");
        assert_eq!(conv.participants, vec![speaker, listener]);
        // The acceptance answers the assertion (the in-reply-to adjacency).
        let reply = Move::from_event(w.events().get(conv.event_ids[1])).unwrap();
        assert_eq!(reply.in_reply_to, Some(conv.event_ids[0]));
    }

    #[test]
    fn gossip_skips_a_promoted_speaker() {
        // A promoted speaker with no promoted partner present must not fall back to the
        // one-pass gossip transmission (the dialogue step handles it, and it needs a
        // promoted partner). So a lone promoted speaker neither gossips nor logs a move.
        let mut w = dialogue_world();
        let speaker = w.spawn(Fixed::ONE);
        let listener = w.spawn(Fixed::ONE); // not promoted
        w.set_place(speaker, 1);
        w.set_place(listener, 1);
        w.promote(speaker);
        w.tick(&[observe_for(speaker, 10)]);
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(listener)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            None,
            "a promoted speaker does not also gossip"
        );
        assert_eq!(
            w.events().len(),
            0,
            "no move logged without a promoted partner"
        );
    }

    #[test]
    fn a_seen_through_lie_in_dialogue_yields_a_refusal_and_no_belief() {
        let mut w = dialogue_world();
        let speaker = w.spawn(Fixed::ONE);
        let listener = w.spawn(Fixed::ONE);
        w.set_place(speaker, 1);
        w.set_place(listener, 1);
        w.promote(speaker);
        w.promote(listener);
        // Speaker comes to believe 10; listener witnessed the speaker actually has access
        // to 20, so the listener sees the assertion as a lie and refuses it.
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
        // The response move is a refusal (move kind 3), pointing back at the assertion.
        let reply = Move::from_event(w.events().get(EventId(1))).unwrap();
        assert_eq!(
            reply.force,
            crate::dialogue::MoveKindId(3),
            "the reply is a refusal"
        );
        assert_eq!(reply.in_reply_to, Some(EventId(0)));
    }

    #[test]
    fn an_infelicitous_move_misfires_as_a_bare_attempt() {
        let mut w = gossip_world();
        let (reg, floor) = dialogue_substrate(true); // the assertion is gated by a role
        w.set_dialogue(reg, floor).unwrap();
        // Resolve the role band, so the misfire is due to the unmodelled role dimension
        // (it reads as absent and fails closed), not an unset band.
        w.set_felicity_band(
            "felicity.assert.role",
            ResolvedBand {
                lo: Fixed::ONE,
                hi: Fixed::from_int(10),
            },
        );
        let speaker = w.spawn(Fixed::ONE);
        let listener = w.spawn(Fixed::ONE);
        w.set_place(speaker, 1);
        w.set_place(listener, 1);
        w.promote(speaker);
        w.promote(listener);
        w.tick(&[observe_for(speaker, 10)]);
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(listener)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            None,
            "a misfired assertion lands no force"
        );
        assert_eq!(
            w.events().len(),
            1,
            "the bare attempt is logged, with no response"
        );
    }

    #[test]
    fn grounding_accumulates_as_said_evidence() {
        // Grounding is the second-order model approaching agreement through said evidence,
        // with no common-ground prior: the speaker comes to model the listener as holding
        // the claim purely from the listener's acceptances over the said channel (the two
        // share no witnessed access to each other's beliefs), so the convergence is
        // defeasible said evidence a deception probe can tell from co-witnessing.
        let mut w = dialogue_world();
        let speaker = w.spawn(Fixed::ONE);
        let listener = w.spawn(Fixed::ONE);
        w.set_place(speaker, 1);
        w.set_place(listener, 1);
        w.promote(speaker);
        w.promote(listener);
        w.tick(&[observe_for(speaker, 10)]);
        for _ in 0..3 {
            w.tick(&[]);
        }
        let mp = *w.meta_params();
        assert_eq!(
            w.mind(speaker)
                .unwrap()
                .modeled_belief(listener, AttrKindId(0), &mp),
            Some(10),
            "the speaker models the listener as having taken up the claim"
        );
    }

    #[test]
    fn a_curious_being_asks_and_is_answered() {
        // One member knows where the water is; the other wonders but cannot answer, so it
        // asks, and the knower's answer grounds into it.
        let mut w = dialogue_world();
        let knower = w.spawn(Fixed::ONE);
        let seeker = w.spawn(Fixed::ONE);
        w.set_place(knower, 1);
        w.set_place(seeker, 1);
        w.promote(knower);
        w.promote(seeker);
        w.set_wondering(seeker, StableId(99), AttrKindId(0)); // the seeker is curious first
        assert!(w.is_wondering(seeker, StableId(99), AttrKindId(0)));
        w.tick(&[observe_for(knower, 10)]); // the knower commits the value, the seeker asks
        for _ in 0..4 {
            w.tick(&[]);
        }
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(seeker)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            Some(10),
            "the seeker learned the answer"
        );
        assert!(
            !w.is_wondering(seeker, StableId(99), AttrKindId(0)),
            "having learned, the seeker stops wondering"
        );
        // A question move was logged: the seeker actually asked, it did not only overhear.
        let asked = w
            .events()
            .iter()
            .filter_map(Move::from_event)
            .any(|m| matches!(m.content, ContentRef::Inquiry { .. }));
        assert!(asked, "the seeker raised a question");
    }

    #[test]
    fn being_asked_seeds_the_inquiry_goal_in_the_hearer() {
        // A seeker asks a peer who also does not know; the question seeds the goal, so the
        // hearer comes to wonder it too (curiosity spreads, design 9.13).
        let mut w = dialogue_world();
        let seeker = w.spawn(Fixed::ONE);
        let peer = w.spawn(Fixed::ONE);
        w.set_place(seeker, 1);
        w.set_place(peer, 1);
        w.promote(seeker);
        w.promote(peer);
        w.set_wondering(seeker, StableId(99), AttrKindId(0));
        w.tick(&[]);
        assert!(
            w.is_wondering(peer, StableId(99), AttrKindId(0)),
            "being asked makes the hearer wonder the question too"
        );
    }

    #[test]
    fn redundancy_suppression_quiets_the_talk() {
        // Once each party models the other as holding the claim, there is nothing left to
        // tell and no open question, so the conversation falls silent rather than looping.
        let mut w = dialogue_world();
        let a = w.spawn(Fixed::ONE);
        let b = w.spawn(Fixed::ONE);
        w.set_place(a, 1);
        w.set_place(b, 1);
        w.promote(a);
        w.promote(b);
        w.tick(&[observe_for(a, 10)]);
        let mut prev = w.events().len();
        let mut quiet_streak = 0;
        for _ in 0..30 {
            w.tick(&[]);
            let now = w.events().len();
            if now == prev {
                quiet_streak += 1;
            } else {
                quiet_streak = 0;
            }
            prev = now;
            if quiet_streak >= 3 {
                break;
            }
        }
        assert!(
            quiet_streak >= 3,
            "the conversation falls silent once everyone is modelled as knowing"
        );
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(b).unwrap().belief(StableId(99), AttrKindId(0), &bp),
            Some(10),
            "the belief still spread before the talk quieted"
        );
    }

    #[test]
    fn an_answer_that_conflicts_with_the_askers_model_is_seen_through() {
        // A seeker wonders where the herd ranges and cannot answer, but has witnessed that
        // the answerer's access points north. The answerer answers south; the seeker runs
        // the same sincerity frame on the answer as on a volunteered assertion, sees it
        // conflicts with its model, and refuses it rather than grounding it blindly.
        let mut w = dialogue_world();
        let seeker = w.spawn(Fixed::ONE);
        let answerer = w.spawn(Fixed::ONE);
        w.set_place(seeker, 1);
        w.set_place(answerer, 1);
        w.promote(seeker);
        w.promote(answerer);
        w.set_wondering(seeker, StableId(99), AttrKindId(0));
        // One tick: the seeker asks, the answerer answers 20, and the verdict is judged
        // against the witnessed model (10) frozen at the start of the tick. (Over many
        // ticks repeated said-evidence would erode that one witnessed observation, the
        // defeasible-inference dynamic, so the seen-through guarantee is checked here on the
        // turn the answer is given, as the design's phase-frozen snapshot intends.)
        w.tick(&[
            // The answerer comes to believe 20 (south), so that is what it will answer.
            observe_for(answerer, 20),
            // The seeker witnessed the answerer's access pointing at 10 (north).
            TickInput {
                mind: seeker,
                ordinal: 0,
                stim: Stimulus::Model {
                    target: answerer,
                    attr: AttrKindId(0),
                    hyps: vec![10, 20],
                    obs: AccessObs {
                        channel: WITNESSED,
                        toward: 10,
                        from: seeker,
                    },
                },
            },
        ]);
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(seeker)
                .unwrap()
                .belief(StableId(99), AttrKindId(0), &bp),
            None,
            "the seeker refused the answer that conflicts with what it witnessed"
        );
    }

    #[test]
    fn a_logged_conversation_replays_deterministically() {
        let build = || {
            let mut w = dialogue_world();
            let s = w.spawn(Fixed::ONE);
            let l = w.spawn(Fixed::ONE);
            w.set_place(s, 1);
            w.set_place(l, 1);
            w.promote(s);
            w.promote(l);
            w.tick(&[observe_for(s, 10)]);
            for _ in 0..4 {
                w.tick(&[]);
            }
            // Both the outcome hash and the move-log integrity hash must replay.
            (w.state_hash(), w.event_log_hash())
        };
        assert_eq!(
            build(),
            build(),
            "the logged conversation replays bit for bit, outcomes and move log alike"
        );
    }

    // --- Affect: the transient emotional layer wired through the world (R-EMOTION) ---

    #[test]
    fn a_world_appraises_a_drive_change_into_a_beings_affect() {
        use crate::affect::DriveAppraisal;
        const JOY: AffectAxisId = AffectAxisId(0);
        let hunger = DriveId(0);
        let mut w = world();
        let being = w.spawn(Fixed::ONE);
        let mut binding = AppraisalBinding::new();
        // Relief from hunger reads positive on joy, gain 2.
        binding.bind(
            hunger,
            DriveAppraisal {
                axis: JOY,
                gain: Fixed::from_int(2),
                relief_positive: true,
            },
        );
        // Hunger fell by 0.25 (relieved): joy rises by 0.25 * 2 = 0.5, landing on the being.
        let applied = w.appraise(
            being,
            hunger,
            Fixed::ZERO - Fixed::from_ratio(1, 4),
            &binding,
        );
        assert_eq!(applied, Some((JOY, Fixed::from_ratio(1, 2))));
        assert_eq!(w.affect_level(being, JOY), Fixed::from_ratio(1, 2));
        // An unbound drive does not appraise and leaves affect untouched.
        assert_eq!(w.appraise(being, DriveId(9), Fixed::ONE, &binding), None);
        // Affect relaxes toward its baseline.
        w.decay_affect(being, Fixed::ONE);
        assert_eq!(
            w.affect_level(being, JOY),
            Fixed::ZERO,
            "decayed to baseline"
        );
    }

    #[test]
    fn a_strong_feeling_hardens_a_beings_baseline_through_the_world() {
        const DREAD: AffectAxisId = AffectAxisId(1);
        let mut w = world();
        let being = w.spawn(Fixed::ONE);
        let mut state = AffectState::new();
        state.apply(DREAD, Fixed::ONE);
        w.set_affect(being, state);
        // Below threshold: no hardening.
        assert!(!w.harden_affect(being, DREAD, Fixed::from_int(2), Fixed::from_ratio(1, 2)));
        // Above threshold: half the excess becomes the new baseline, and decay no longer
        // returns all the way to zero (the persistent residue of trauma).
        assert!(w.harden_affect(
            being,
            DREAD,
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 2)
        ));
        w.decay_affect(being, Fixed::ONE);
        assert_eq!(w.affect_level(being, DREAD), Fixed::from_ratio(1, 2));
    }

    // --- Sensorium: the channel gate over perception (R-SENSORIUM) ---

    fn channel_trace(id: u64, place: PlaceId, channel: SenseChannelId, subject: u64) -> Trace {
        Trace {
            id: StableId(id),
            place,
            channel,
            subject: StableId(subject),
            attr: AttrKindId(0),
            hyps: vec![10, 20],
            value: 10,
            salience: Fixed::ONE,
            weight: Fixed::from_int(5),
            from: StableId(id),
        }
    }

    #[test]
    fn a_being_perceives_only_on_channels_its_sensorium_reads() {
        const SIGHT: SenseChannelId = SenseChannelId(1);
        const SCENT: SenseChannelId = SenseChannelId(2);
        let mut w = world().with_seed(0x5E45E);
        let anna = w.spawn(Fixed::ONE);
        w.set_place(anna, 1);
        // Anna reads sight but is blind to scent.
        w.set_sensorium(anna, Sensorium::with([(SIGHT, Fixed::ONE)]));
        // A sight trace about subject 70 and a scent trace about subject 80, both co-located
        // and fully salient.
        w.emit_trace(channel_trace(500, 1, SIGHT, 70));
        w.emit_trace(channel_trace(501, 1, SCENT, 80));
        w.tick(&[]);
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(anna)
                .unwrap()
                .belief(StableId(70), AttrKindId(0), &bp),
            Some(10),
            "the sight trace was perceived"
        );
        assert_eq!(
            w.mind(anna)
                .unwrap()
                .belief(StableId(80), AttrKindId(0), &bp),
            None,
            "the scent trace was missed: the being is blind to that channel"
        );
    }

    #[test]
    fn a_being_with_no_sensorium_reads_every_channel() {
        // Back-compatibility: a being that has never been given a sensorium perceives a trace
        // on any channel exactly as before the channel gate existed.
        const MANA: SenseChannelId = SenseChannelId(7);
        let mut w = world().with_seed(0xBEEF);
        let anna = w.spawn(Fixed::ONE);
        w.set_place(anna, 1);
        w.emit_trace(channel_trace(500, 1, MANA, 70));
        w.tick(&[]);
        let bp = *w.belief_params();
        assert_eq!(
            w.mind(anna)
                .unwrap()
                .belief(StableId(70), AttrKindId(0), &bp),
            Some(10),
            "a being with no sensorium reads every channel"
        );
    }

    // --- Aging and mortality: the life-process loop (R-AGING) ---

    fn rising_hazard() -> Curve {
        // A simple rising hazard: certain survival at age 0, certain death by age 100. The shape
        // is the data-driven default; the owner sets the real curve.
        Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::from_int(100), Fixed::ONE),
        ])
    }

    #[test]
    fn an_old_being_dies_and_a_young_one_survives() {
        let mut w = world().with_seed(0xA6E);
        let young = w.spawn(Fixed::ONE);
        let old = w.spawn(Fixed::ONE);
        w.set_age(young, 0);
        w.set_age(old, 100);
        // Give the old being some state to confirm the prune reaches every map.
        w.set_place(old, 3);
        w.set_sensorium(old, Sensorium::with([(SenseChannelId(1), Fixed::ONE)]));
        let dead = w.apply_mortality(&rising_hazard());
        assert_eq!(dead, vec![old], "the old being died, the young one did not");
        assert!(w.mind(old).is_none(), "the dead being's mind was pruned");
        assert!(w.age_of(old).is_none(), "its age was pruned");
        assert!(w.place_of(old).is_none(), "its placement was pruned");
        assert!(w.sensorium_of(old).is_none(), "its sensorium was pruned");
        assert!(w.mind(young).is_some(), "the survivor is untouched");
        assert_eq!(w.population(), 1);
    }

    #[test]
    fn age_step_advances_every_being() {
        let mut w = world();
        let a = w.spawn(Fixed::ONE);
        let b = w.spawn(Fixed::ONE);
        w.set_age(a, 4);
        w.set_age(b, 9);
        w.age_step();
        assert_eq!(w.age_of(a), Some(5));
        assert_eq!(w.age_of(b), Some(10));
    }

    #[test]
    fn mortality_replays_deterministically() {
        // A middling hazard exercises the stochastic path; the same seed must kill the same
        // beings on the same ages every run.
        let build = || {
            let mut w = world().with_seed(0xD1CE);
            let ids: Vec<StableId> = (0..8).map(|_| w.spawn(Fixed::ONE)).collect();
            for (k, &id) in ids.iter().enumerate() {
                w.set_age(id, 40 + k as u32); // a spread of ages around the half-hazard region
            }
            w.apply_mortality(&rising_hazard())
        };
        assert_eq!(build(), build(), "the same beings die on replay");
    }
}
