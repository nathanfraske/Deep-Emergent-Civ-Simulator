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
//! After the cognition phases, the tick beats a coarse life cadence (design Part 20,
//! R-AGING): on a tick that lands on the life-cadence period every tracked being ages one
//! step and, if an age-hazard curve is installed, faces the mortality roll, so a world left
//! running turns its generations over on the clock rather than only when a caller reaches in.
//! The period defaults to the owner-set [`crate::clock::LIFE_CADENCE_TICKS`] and the hazard is
//! owner-supplied, so nothing here fabricates when a being ages or how likely it is to die.
//!
//! This is deliberately the serial tick, not the parallel command scheduler: that
//! scheduler's determinism (the total command order and the non-associative combines)
//! is still open design (R-CMD-ORDER, R-REDUCE-ORDER), so the parallel form is left for
//! that resolution. Nothing here invents a calibration value. The minds' thresholds and
//! weights are loaded from the manifest and fail loud while reserved; a development run
//! uses a clearly-labelled fixtures profile, never the authoritative manifest's unset
//! entries.

use std::collections::{BTreeMap, BTreeSet};

use civsim_world::OrbitalElements;

use crate::affect::{AffectAxisId, AffectState, AppraisalBinding};
use crate::agent::{AccessObs, Mind, SharedBelief};
use crate::axiom::{self, Axiom, AxiomAxisId, EvidenceRing, IntrinsicBeliefs, RingCapacityLaw};
use crate::belief::{BeliefKey, BeliefPool};
use crate::breeding::{BreedingSystemRegistry, SexClass};
use crate::calibration::{CalibrationError, CalibrationManifest, Profile};
use crate::census::ReproductiveCensus;
use crate::clock::LIFE_CADENCE_TICKS;
use crate::decision::{ActionId, Behaviour, Curve, DriveId};
use crate::dialogue::{
    ContentRef, EffectSign, ForceFloor, ForceKind, Move, MoveKindId, MoveRegistry, ResolvedBand,
};
use crate::evidence::{AttrKindId, InferenceParams, ValueId};
use crate::genome::{Channel, Genome, ReproductionMode};
use crate::language::{
    ConceptId, DriftParams, FormSystem, LangId, Language, LanguageParams, Lexicon, Word,
};
use crate::mate_choice::{choose, MatePreference};
use crate::personality::{age_personality, PersonalityRegistry, TraitAxisId, TraitInstance};
use crate::race::{BandSpec, Race};
use crate::sensorium::{SenseChannelId, Sensorium};
use crate::tom::{self, AccessChannelRegistry, AccessWeights};
use crate::value::{
    EmicProjection, EticAxisId, EticSubstrate, RaceId, RaceProjection, ValueAxisId, ValueProfile,
};
use civsim_core::{
    gaussian_unit, CommandBuffer, CommandKey, DrawKey, EventId, EventLog, Fixed, GaussApprox,
    Phase, Registry, StableId, StateHasher,
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

/// The per-being developmental-environment offset (design Part 25.6): a mean-zero symmetric
/// deviation in `[-spread, +spread)` drawn once per being under [`Phase::DEVELOPMENT`], keyed on
/// the being's `id` and the `tick` (the dawn passes tick 0, a birth passes the generation). This
/// is the environmental-variance (V_E) source that makes [`crate::genome::GeneSet::express`] vary
/// between members of one cohort: today every member rides one shared `race.environment`, so V_E
/// is identically zero; adding this per-being offset to that baseline authors variance without a
/// direction, because the map `(2 * unit - 1)` is symmetric about zero. Its expectation over a
/// uniform `unit_fixed` draw on the half-open grid `[0, ONE)` is `-2^-32` (one fixed-point ULP
/// below zero, since the grid includes 0 but excludes ONE), not exactly zero: the residual cohort
/// mean shift is one ULP times `spread`, physically negligible and dwarfed by the per-being spread,
/// so no direction is authored (Principle 9). `spread` is the race's reserved `environment_variance`,
/// the half-width of the
/// deviation; at [`Fixed::ZERO`] the offset is exactly zero and the expressed mind is bit-identical
/// to the pre-offset dawn (the interim that reproduces the homogeneous world). Deterministic:
/// `unit_fixed` lies in `[0, ONE)` and the symmetric map is a pure function of the seed, the being,
/// and the tick, so it replays bit for bit; it is non-heritable, applied at expression and never
/// folded back into a pool's allele frequencies on demotion. Mirrors the mate-preference draw the
/// dawn and birth already use for symmetric per-being variation.
fn env_offset(seed: u64, id: StableId, tick: u64, spread: Fixed) -> Fixed {
    let unit = DrawKey::entity(id.0, tick, Phase::DEVELOPMENT)
        .rng(seed)
        .unit_fixed(0);
    (Fixed::from_int(2).mul(unit) - Fixed::ONE).mul(spread)
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

/// The race-normalized mortality pass ([`World::apply_mortality_by_race`]) could not race-normalize
/// a being: its race is untracked or absent from the supplied `races`, so there is no lifespan to
/// map its raw age onto the life-fraction domain the shared hazard curve is evaluated in. Rather
/// than read the curve in the wrong domain (raw age against a fraction curve, which would silently
/// make the unraced class near-immortal or cull it wholesale), the pass fails loud, naming the being
/// it could not normalize. Carries the offending being's id.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UnraceableBeing(pub StableId);

impl std::fmt::Display for UnraceableBeing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "being {:?} cannot be race-normalized for the life-fraction mortality pass (no tracked race with a lifespan); refusing rather than reading the hazard in the raw-age domain",
            self.0
        )
    }
}

impl std::error::Error for UnraceableBeing {}

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

/// The reserved calibrations the reproduction beat needs beyond what each race derives for itself
/// (design Parts 25, 28, R-REPRO). The narrow-sense heritability is derived per race from its own pool
/// (`GenePool::narrow_sense_heritability`), so it is not carried here; what remains is the belief
/// inheritance mutation spread (the bounded deviation a child's inherited disposition and mate
/// preference draw around the midparent) and the evidence-ring law a child's ring is sized through.
/// The mechanism is fixed Rust; these are data (Principle 11). Fecundity is NOT here: a mature,
/// compatible pair produces one offspring per reproductive cadence, so lifetime fecundity falls out of
/// maturity, lifespan, and the cadence, never an authored rate.
#[derive(Clone, Debug)]
pub struct ReproductionParams {
    /// The bounded mutation spread the belief and mate-preference inheritance draw around the
    /// midparent (reserved; the same spread the belief inheritance already uses). A child's inherited
    /// disposition is the parent's plus a mean-zero deviation of this half-width.
    pub mutation_spread: Fixed,
    /// The evidence-ring capacity law a child's axiom rings are sized through, from its own recombined
    /// memory phenotype (`RingCapacityLaw`, reserved through the manifest).
    pub ring_law: RingCapacityLaw,
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
    /// The reserved dogmatism/freezing split weight the enculturation band applies through
    /// [`crate::axiom::EpistemicStance::effective_stubbornness`] (`axiom.stubbornness_dogmatism_weight`).
    /// `None` until installed with [`World::set_stubbornness_split`], and the enculturation band beats
    /// are then a no-op rather than running on a fabricated half (Principle 11), the same
    /// fail-quiet-until-declared convention gossip and the mortality hazard use.
    stubbornness_split: Option<Fixed>,
    /// Per-being heritable mate preference, the direction of assortment as a per-being trait
    /// (R-REPRO). Seeded at the dawn with unbiased variation (symmetric about indifference, so
    /// no direction is authored) and inherited at birth by midparent plus bounded mutation, so
    /// which way a being assorts is shaped by differential reproduction rather than a per-race
    /// lever. Used by [`World::choose_mate`]; the selection that shapes it is proven in
    /// [`crate::mate_choice`].
    mate_prefs: BTreeMap<StableId, MatePreference>,
    /// Per-being transient affect, the event-driven emotional state (the R-EMOTION gap).
    affect: BTreeMap<StableId, AffectState>,
    /// Per-being age in life-cadence ticks, for the aging-and-mortality loop (the R-AGING gap).
    ages: BTreeMap<StableId, u32>,
    /// Per-being race identity: which [`RaceId`] a being belongs to (design Part 20, R-AGING).
    /// Mirrors `ages` (seeded at the dawn and at birth, pruned on death), and lets a per-being
    /// mechanism reach its race's data, so mortality can normalize age by the race's own lifespan
    /// (see [`World::apply_mortality_by_race`]) rather than a single hardcoded scale. A being with
    /// no entry is untracked, and a race-keyed pass falls back to its raw-age behaviour for it.
    race_of: BTreeMap<StableId, RaceId>,
    /// The race records the world knows, by id (design Part 20, Part 33.4). Populated at the dawn by
    /// [`World::seed_dawn_populations`] (which already receives the registry) and settable directly
    /// with [`World::set_races`]. It is the registry the per-lineage drift cadence reads: a
    /// lineage's [`Language::race`] is looked up here for its `maturity_years`, so drift derives its
    /// generation length per lineage (see [`World::drift_languages`]) rather than one global scalar.
    /// A lineage whose race is absent has no maturity to derive a cadence from and does not drift (a
    /// fabricated cadence is never invented, Principle 11). This is owner config, not per-tick state,
    /// so the whole record is not folded into [`World::state_hash`]; but each lineage's own race
    /// maturity_years and lifespan_years (the two quantities the drift cadence and mortality
    /// normalization read) ARE folded there in the languages loop, alongside the lineage's race id, so
    /// a change to a race's maturity or lifespan surfaces in the fingerprint at once rather than only
    /// through the drift it later produces.
    races: BTreeMap<RaceId, Race>,
    /// The per-race personality profiles (design Part 20, R-BEING-REP): the age-personality
    /// substrate's `being.plasticity_by_age` (the maturation-timed plasticity curves) and
    /// `being.maturity_targets` (where each trait matures to), per race. Empty until
    /// [`World::set_personality_registry`] installs one, and the life-cadence personality beat is
    /// then inert, so a world that declares no personality drift runs exactly as before.
    personality: PersonalityRegistry,
    /// Per-being live personality: the current trait values the life-cadence personality beat drifts
    /// toward each race's maturity targets. Seeded birth-neutral when a being of a race that carries
    /// a profile is created; a being whose race declares no profile carries no instance. It rides the
    /// aging cadence, but a divergent personality trajectory is a divergent world (future drift keys
    /// off it), so it IS folded into [`World::state_hash`] in mind-id then axis-id order, alongside
    /// the `ages` it rides on.
    traits: BTreeMap<StableId, TraitInstance>,
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
    /// The per-band aggregate belief pools (design Part 54), keyed by place: each holds the
    /// prevailing beliefs of the members at that place, their intensive knowledge level and extensive
    /// mass. The belief-diffusion beat raises them toward saturation. Empty until a producer or a test
    /// seeds one ([`World::seed_belief`]); the dawn seeds each being's own intrinsic beliefs, not
    /// aggregate pools, so a world that seeds none runs the diffusion beat as a no-op.
    belief_pools: BTreeMap<PlaceId, BeliefPool>,
    /// The aggregate belief-diffusion rate (`evidence.aggregate_diffusion_rate`, derived from the
    /// gossip parameters). None until set; the belief-diffusion beat is then a no-op.
    belief_diffusion_rate: Option<Fixed>,
    /// The reproduction calibrations (design Parts 25, 28, R-REPRO). None until set; the reproduce
    /// half of the life cadence is then a no-op, so a world that installs none only ages and dies.
    reproduction: Option<ReproductionParams>,
    /// The life-fraction mortality hazard (a curve on `age / race lifespan`, design Part 20, R-AGING):
    /// when set, the life cadence culls through [`World::apply_mortality_by_race`], so short- and
    /// long-lived races die on their own timescales from one curve. Distinct from the raw-age
    /// [`World::mortality_hazard`]: a world may install either, and the by-race path takes precedence.
    mortality_hazard_by_race: Option<Curve>,
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
    /// The life-cadence period in ticks: how often [`World::tick`] beats aging and the mortality
    /// roll (design Part 20, R-AGING). The owner-set default is [`LIFE_CADENCE_TICKS`] (one
    /// in-world year); [`World::set_life_cadence`] overrides it per world (a calibration override,
    /// like the base-tick duration, so a test or a faster-aging world can shrink it). This is
    /// canonical: it sets when aging and mortality happen, so it is part of the world's timeline.
    life_cadence_ticks: u64,
    /// The installed age-hazard curve the mortality beat rolls against (design Part 20). `None`
    /// until [`World::set_mortality_hazard`] installs one, and the mortality half of the life
    /// cadence is then a no-op, so the hazard shape is never fabricated: aging (a bare increment)
    /// runs on the cadence regardless, but a being dies only against an owner-supplied curve.
    mortality_hazard: Option<Curve>,
    /// The data-driven breeding-system registry (design Part 25, R-REPRO): resolves a race's
    /// [`crate::breeding::BreedingSystemId`] to its sex classes and assignment rule, so a being's
    /// sex is read off its sex-determination locus. Empty by default; when a race's system is not
    /// registered, sex determination falls back to a single class, so the census authors no ratio.
    breeding_systems: BreedingSystemRegistry,
    /// The reproductive-success census for the current window (design Part 25, R-REPRO): the sex of
    /// each being and the offspring credited to each contributing parent, from which an effective
    /// population size Ne derives ([`ReproductiveCensus::effective_size`]). Credited in
    /// [`World::birth`] and seeded at the dawn; a window transient like the pool-tier accumulator,
    /// so it is kept out of [`World::state_hash`] and carries its own stamped reduction hash.
    census: ReproductiveCensus,
    /// The stamped integer-Gaussian approximation the world's mean-zero draws use (design 25.10;
    /// `genome.gauss_approx`, a world-identity value). One shape for the whole world, so the
    /// axiom-inheritance belief-mutation deviate ([`World::inherited_beliefs`]) draws through the
    /// same approximation the genome's continuous mutation and the controller mutation do, rather
    /// than a bare `k = 12` literal duplicated at each consumer. [`World::new`] seeds the labelled
    /// stamped default; a canonical build overrides it with [`World::set_gauss_approx`] from the
    /// manifest.
    gauss_approx: GaussApprox,
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
            stubbornness_split: None,
            mate_prefs: BTreeMap::new(),
            affect: BTreeMap::new(),
            ages: BTreeMap::new(),
            race_of: BTreeMap::new(),
            races: BTreeMap::new(),
            personality: PersonalityRegistry::new(),
            traits: BTreeMap::new(),
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
            belief_pools: BTreeMap::new(),
            belief_diffusion_rate: None,
            reproduction: None,
            mortality_hazard_by_race: None,
            drift: None,
            language: None,
            events: EventLog::new(),
            belief_params,
            meta_params,
            weights,
            workers: 1,
            life_cadence_ticks: LIFE_CADENCE_TICKS,
            mortality_hazard: None,
            breeding_systems: BreedingSystemRegistry::new(),
            census: ReproductiveCensus::new(),
            // The labelled stamped default (design 25.10, `genome.gauss_approx = SumOfUniforms{k=12}`):
            // the same shape the genome and controller mutations draw through, so the belief-mutation
            // deviate shares one world-identity approximation. A canonical build overrides it from the
            // manifest via `set_gauss_approx`.
            gauss_approx: GaussApprox::SumOfUniforms { k: 12 },
        }
    }

    /// Install the breeding-system registry (design Part 25, R-REPRO). Until set, sex determination
    /// falls back to a single class and the reproductive census records every breeder as that class,
    /// so a world that has not declared its mating types authors no sex ratio.
    pub fn set_breeding_systems(&mut self, registry: BreedingSystemRegistry) {
        self.breeding_systems = registry;
    }

    /// The reproductive-success census for the current window, for inspecting the sex tally and the
    /// derived effective population size Ne (design Part 25, R-REPRO).
    pub fn census(&self) -> &ReproductiveCensus {
        &self.census
    }

    /// Close the current census window and open the next (bumps the window stamp and clears the
    /// tally). A run drives this on its generation cadence so each window's Ne measures a fresh
    /// cohort; between windows the tally accumulates births.
    pub fn reset_census_window(&mut self) {
        self.census.reset();
    }

    /// A being's sex class, read off its race's sex-determination locus through the ordinary
    /// expression map (design Part 25, R-REPRO). Deterministic and RNG-free: sex is a pure function
    /// of the genome and the race's gene set and breeding system. `None` when the race's breeding
    /// system is not registered, so a caller never runs on a fabricated class.
    fn express_sex(&self, race: &Race, genome: &Genome) -> Option<SexClass> {
        let system = self.breeding_systems.get(race.breeding)?;
        let expressed = race
            .genes
            .express(genome, Channel::SexDetermination, Fixed::ZERO);
        Some(system.assign(expressed))
    }

    /// Set the execution width of the parallel read stage (the ActionStage worker
    /// count). Purely an execution choice: the canonical result is proven identical for
    /// any width, because the barrier re-orders the produced commands by [`CommandKey`]
    /// before any of them applies (R-CMD-ORDER). Clamped to at least 1.
    pub fn set_workers(&mut self, workers: usize) {
        self.workers = workers.max(1);
    }

    /// Override the life-cadence period in ticks (design Part 20, R-AGING). The default is the
    /// owner-set [`LIFE_CADENCE_TICKS`] (one in-world year); this per-world override lets a test
    /// or a faster-aging world beat aging and mortality on a shorter period. Unlike the worker
    /// count this is canonical: it changes when aging and mortality happen, so two worlds with
    /// different cadences have different (each still deterministic) timelines. Clamped to at least
    /// 1, so the beat never divides by a zero period.
    pub fn set_life_cadence(&mut self, ticks: u64) {
        self.life_cadence_ticks = ticks.max(1);
    }

    /// The life-cadence period in ticks the world currently beats aging and mortality on
    /// (design Part 20). On the canonical path this is the value derived from the world's orbit
    /// by [`World::from_manifest_with_orbital`]; with the direct constructor it is the labelled
    /// dev fallback [`LIFE_CADENCE_TICKS`] until [`World::set_life_cadence`] overrides it.
    pub fn life_cadence_ticks(&self) -> u64 {
        self.life_cadence_ticks
    }

    /// Install the age-hazard curve the mortality beat rolls against (design Part 20). Until this
    /// is set the mortality half of the life cadence is a no-op, so the world never runs on a
    /// fabricated hazard shape: the owner supplies the curve, its shape reserved with its basis
    /// (the failure boundary of senescence the age data implies). Aging still beats without it.
    pub fn set_mortality_hazard(&mut self, hazard: Curve) {
        self.mortality_hazard = Some(hazard);
    }

    /// Install the life-fraction mortality hazard (design Part 20, R-AGING): a curve on
    /// `age / race lifespan` in `[0, 1]`, so one curve culls short- and long-lived races each on its
    /// own timescale ([`World::apply_mortality_by_race`]). When set it takes precedence over the
    /// raw-age [`World::set_mortality_hazard`] in the life cadence. Until either is set the mortality
    /// beat is a no-op. The curve's shape is reserved with its basis (the senescence failure boundary).
    pub fn set_mortality_hazard_by_race(&mut self, hazard: Curve) {
        self.mortality_hazard_by_race = Some(hazard);
    }

    /// Install the reproduction calibrations (design Parts 25, 28, R-REPRO). Until set, the reproduce
    /// half of the life cadence is a no-op, so a world that installs none only ages and dies.
    pub fn set_reproduction(&mut self, params: ReproductionParams) {
        self.reproduction = Some(params);
    }

    /// Stamp the world's integer-Gaussian approximation (design 25.10, `genome.gauss_approx`), the
    /// one world-identity shape the mean-zero draws use. Overrides the labelled [`World::new`]
    /// default so the axiom-inheritance belief mutation draws through the same approximation the
    /// genome and controller mutations do. A canonical build reads it from the manifest and installs
    /// it here; changing it re-rolls the world (it feeds every quantitative lineage).
    pub fn set_gauss_approx(&mut self, gauss: GaussApprox) {
        self.gauss_approx = gauss;
    }

    /// The world's stamped integer-Gaussian approximation.
    pub fn gauss_approx(&self) -> GaussApprox {
        self.gauss_approx
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
        // The two per-world orbital scalars join the required gate list here (alongside the base
        // tick), so the PRODUCTION constructor derives the life cadence from the world's own orbit
        // rather than inheriting World::new's Earth-year LIFE_CADENCE_TICKS fallback. A calibrated
        // world with a reserved orbit fails loud (its year is unknown), never runs on the Earth
        // constant. The orbit is read through the same fail-loud manifest path as every other value.
        let mut world = World::from_manifest_gated(
            manifest,
            channels,
            profile,
            &[
                "world.orbital_period_seconds",
                "world.rotation_period_seconds",
                "time.base_tick_seconds",
            ],
        )?;
        let orbital = crate::clock::orbital_from_manifest(manifest)?;
        let base_tick = crate::clock::base_tick_seconds_fixed(manifest)?;
        let cadence = crate::clock::ticks_from_seconds(orbital.orbital_period_seconds, base_tick)?;
        world.set_life_cadence(cadence);
        Ok(world)
    }

    /// The shared gate-and-build core of the two manifest constructors: it enforces the profile gate
    /// over the base required set plus `extra_required`, reads the cognition, theory-of-mind, access,
    /// and gossip calibrations, and returns a world whose life cadence is still the [`World::new`]
    /// fallback (the caller derives and installs the real cadence). Split out so
    /// [`World::from_manifest`] can gate on the manifest orbit while
    /// [`World::from_manifest_with_orbital`] gates only on the base tick (the caller supplies the
    /// orbit), without either constructor gating the other's orbital keys.
    fn from_manifest_gated(
        manifest: &CalibrationManifest,
        channels: &AccessChannelRegistry,
        profile: Profile,
        extra_required: &[&str],
    ) -> Result<Self, CalibrationError> {
        let mut required = vec![
            "evidence.log_odds_clamp",
            "evidence.commit_threshold",
            "evidence.runner_up_margin",
            "gossip.told_weight",
            "gossip.trust_baseline",
            "gossip.trust_penalty",
        ];
        required.extend_from_slice(extra_required);
        manifest.gate(profile, &required)?;
        let belief_params = InferenceParams::from_manifest(manifest)?;
        // The meta-frame params and the witnessed/told/said assertion ladder DERIVE from the
        // first-order evidence params (record 62.11); only the independent access levers
        // (reachable, absence, denied) are read from the manifest.
        let meta_params = tom::meta_params_from_evidence(&belief_params);
        let weights = AccessWeights::from_evidence_and_manifest(channels, &meta_params, manifest)?;
        let gossip = GossipParams::from_manifest(manifest)?;
        let mut world = World::new(belief_params, meta_params, weights);
        world.channels = channels.clone();
        world.gossip = Some(gossip);
        Ok(world)
    }

    /// A world whose calibrations are loaded from the manifest and whose life-cadence beat is
    /// derived from a CALLER-SUPPLIED orbit (design Parts 14.6, 20, 54). Unlike
    /// [`World::from_manifest`], which reads the orbit from the manifest's reserved per-world
    /// scalars, this takes the orbital elements as an argument (a labelled fixture in tests, a
    /// world's declared orbit from a tool), so it does NOT gate on the manifest orbit and can run a
    /// calibrated determinism check against a reserved manifest that has not yet declared its orbit.
    /// The base tick is still read live from the manifest as a canonical [`Fixed`]. Fails loud if the
    /// base tick is reserved or the derived cadence is degenerate, so a world never runs on a
    /// fabricated or zero cadence.
    pub fn from_manifest_with_orbital(
        manifest: &CalibrationManifest,
        channels: &AccessChannelRegistry,
        profile: Profile,
        orbital: OrbitalElements,
    ) -> Result<Self, CalibrationError> {
        let mut world =
            World::from_manifest_gated(manifest, channels, profile, &["time.base_tick_seconds"])?;
        let base_tick = crate::clock::base_tick_seconds_fixed(manifest)?;
        let cadence = crate::clock::ticks_from_seconds(orbital.orbital_period_seconds, base_tick)?;
        world.set_life_cadence(cadence);
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

    /// Install the reserved dogmatism/freezing split weight the enculturation band applies
    /// (`axiom.stubbornness_dogmatism_weight`, read through
    /// [`crate::axiom::stubbornness_dogmatism_weight`]). Until set, [`World::enculturate_band`] and
    /// [`World::enculturate_band_bounded`] are a no-op rather than running the Friedkin-Johnsen anchor
    /// on a fabricated half (Principle 11). A test installs a clearly-labelled fixture weight.
    pub fn set_stubbornness_split(&mut self, weight: Fixed) {
        self.stubbornness_split = Some(weight);
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
    /// [`World::set_language_of`]. The default lineage belongs to [`RaceId`] zero (the conventional
    /// first race, matching how [`World::lang_of_mind`] defaults to [`LangId`] zero); use
    /// [`World::set_form_system_for`] to place it on a named race, which the drift cadence reads.
    pub fn set_form_system(&mut self, fs: FormSystem) {
        self.set_form_system_for(RaceId(0), fs);
    }

    /// Install the default-lineage articulation system on a named race, whose `maturity_years` the
    /// per-lineage drift cadence derives its generation length from (design Part 33.4). The race
    /// must be present in the world's registry (through [`World::set_races`] or the dawn seeding)
    /// for that lineage to drift.
    pub fn set_form_system_for(&mut self, race: RaceId, fs: FormSystem) {
        self.languages
            .insert(LangId(0), Language::new(LangId(0), race, fs));
    }

    /// Install the race records the world knows, by id (design Part 20, Part 33.4). This is the
    /// registry the per-lineage drift cadence reads for each lineage's `maturity_years`; the dawn
    /// seeding populates it automatically, and a test or a lineage-only world sets it directly.
    pub fn set_races(&mut self, races: BTreeMap<RaceId, Race>) {
        self.races = races;
    }

    /// Install the per-race personality registry (design Part 20, R-BEING-REP): the age-personality
    /// substrate's plasticity curves and maturity targets, per race. Until set, the life-cadence
    /// personality beat is inert and no being carries a trait instance, so a world that declares no
    /// personality drift is unchanged. Beings seeded at the dawn or born after this is installed,
    /// whose race carries a profile, get a birth-neutral instance; [`World::install_personality`]
    /// seeds one directly for a test or an expressed starting personality.
    pub fn set_personality_registry(&mut self, registry: PersonalityRegistry) {
        self.personality = registry;
    }

    /// Install a being's live personality directly (a test seed, or an expressed starting
    /// personality). The life-cadence personality beat then drifts it toward its race's maturity
    /// targets at the being's own age-scaled plasticity.
    pub fn install_personality(&mut self, id: StableId, instance: TraitInstance) {
        self.traits.insert(id, instance);
    }

    /// A being's current value on a personality trait axis, or `None` if the being carries no
    /// personality instance (its race declares no profile, or it was never seeded).
    pub fn trait_value(&self, id: StableId, axis: TraitAxisId) -> Option<Fixed> {
        self.traits.get(&id).map(|inst| inst.value(axis))
    }

    /// Seed a being's birth-neutral personality if its race carries a profile (design Part 20). A
    /// no-op when no profile is registered for the race, so the seeding path stays inert until a
    /// world installs personality data.
    fn seed_personality(&mut self, id: StableId, race: RaceId) {
        if let Some(profile) = self.personality.profile(race) {
            self.traits.insert(id, profile.birth_instance());
        }
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

    /// Install the aggregate belief-diffusion rate (`evidence.aggregate_diffusion_rate`, derived from
    /// the gossip parameters through [`crate::belief::BeliefParams`]). Until set, the belief-diffusion
    /// beat is a no-op, so a world that installs no rate runs exactly as before.
    pub fn set_belief_diffusion_rate(&mut self, rate: Fixed) {
        self.belief_diffusion_rate = Some(rate);
    }

    /// Seed a band's prevailing belief at a place (the aggregate-tier belief substrate, design Part
    /// 54): the members at `place` hold the belief `key` at knowledge level `level`, with `count`
    /// holders. The belief-diffusion beat then raises the level toward saturation on the SI logistic.
    /// A producer or a test seeds; the dawn seeds each being's own intrinsic beliefs rather than these
    /// aggregate pools, so a world that seeds none diffuses nothing.
    pub fn seed_belief(&mut self, place: PlaceId, key: BeliefKey, level: Fixed, count: u32) {
        self.belief_pools
            .entry(place)
            .or_default()
            .seed(key, level, count);
    }

    /// A band's belief pool, if any has been seeded at the place (for inspection and tools).
    pub fn belief_pool(&self, place: PlaceId) -> Option<&BeliefPool> {
        self.belief_pools.get(&place)
    }

    /// A band's knowledge level for a belief, if the place holds a pool carrying that belief (a read
    /// of canon, for tests and the view).
    pub fn belief_level(&self, place: PlaceId, key: &BeliefKey) -> Option<Fixed> {
        self.belief_pools
            .get(&place)?
            .get(key)
            .map(|b| b.knowledge_level())
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

    /// The ids of every being the world holds, in canonical id order (a read of canon, for tools and
    /// tests that walk the population). The order is the `BTreeMap` id order, so it is deterministic
    /// and independent of insertion order.
    pub fn being_ids(&self) -> Vec<StableId> {
        self.minds.keys().copied().collect()
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
    /// Part 28) with each axiom's evidence ring sized from the member's own expressed memory
    /// through `ring_law` ([`RingCapacityLaw::capacity_for`]), and it is placed. Returns the
    /// seeded ids in seeding order. A band whose race is not in `races` is skipped.
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
        ring_law: &RingCapacityLaw,
    ) -> Vec<StableId> {
        // Record the race registry so the per-lineage drift cadence can read each lineage's race
        // maturity without the registry being threaded through every tick (design Part 33.4).
        for (id, race) in races {
            self.races.entry(*id).or_insert_with(|| race.clone());
        }
        let mut seeded = Vec::new();
        for band in bands {
            let Some(race) = races.get(&band.race) else {
                continue;
            };
            for _ in 0..band.members {
                let id = self.reg.mint();
                let genome = race.pool.promote(self.seed, id.0, race.ploidy());
                // Express the member's mind on the race's environment baseline plus its own
                // mean-zero developmental offset (the V_E spread, design Part 25.6), keyed on the
                // member's id at the dawn tick 0. At the reserved interim `environment_variance` of
                // zero the offset is exactly zero, so this reproduces the pre-offset dawn bit for
                // bit; a positive spread makes members of one band express different minds from one
                // genome-and-environment rule, without shifting the band mean.
                let env =
                    race.environment + env_offset(self.seed, id, 0, race.environment_variance);
                let mind = Mind::from_genome(id, &race.genes, &genome, env);
                // The dawn member's evidence ring is sized from its own expressed memory through
                // the shared ring-capacity law, not copied from the race template's literal cap,
                // so a mindful founder carries a larger ring than a forgetful one of the same
                // race (design Part 25.6, Part 9; the law reads only the memory value, never the
                // race, Principle 9).
                let cap = ring_law.capacity_for(mind.memory);
                let mut intrinsic = race.intrinsic.clone();
                for ax in &mut intrinsic.axioms {
                    ax.evidence = EvidenceRing::new(cap);
                }
                self.minds.insert(id, mind);
                self.genomes.insert(id, genome);
                self.intrinsic.insert(id, intrinsic);
                // Seed the mate preference with unbiased variation: a weight in [-1, 1] drawn per
                // being under Phase::MATE_CHOICE. The draw is symmetric about zero (indifference),
                // so the dawn population carries variation without an authored direction; which
                // way selection later pushes it is a consequence, not a seed.
                let unit = DrawKey::entity(id.0, 0, Phase::MATE_CHOICE)
                    .rng(self.seed)
                    .unit_fixed(0);
                let weight = Fixed::from_int(2).mul(unit) - Fixed::ONE;
                self.mate_prefs.insert(id, MatePreference::new(weight));
                self.place_of.insert(id, band.place);
                self.ages.insert(id, 0);
                self.race_of.insert(id, band.race);
                // Seed a birth-neutral personality if the race carries a profile (inert otherwise).
                self.seed_personality(id, band.race);
                // Stamp the founder's gene-fed sex into the census (read off its sex-determination
                // locus, no RNG). A founding cohort thus carries a sex ratio that emerged from its
                // pool's sex-determination allele frequencies rather than being drawn.
                let founder_sex = self
                    .genomes
                    .get(&id)
                    .and_then(|g| self.express_sex(race, g));
                if let Some(sex) = founder_sex {
                    self.census.record_sex(id, sex);
                }
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

    /// A being's mate preference by id, if one has been set (for inspection). Populated at the
    /// dawn and inherited at birth.
    pub fn mate_pref_of(&self, id: StableId) -> Option<&MatePreference> {
        self.mate_prefs.get(&id)
    }

    /// Set a being's mate preference (used by the dawn seeding, by birth inheritance, and by
    /// tools and tests). The being need not already hold one.
    pub fn set_mate_pref(&mut self, id: StableId, pref: MatePreference) {
        self.mate_prefs.insert(id, pref);
    }

    /// Choose a mate for `chooser` from a band of `candidates` under the chooser's own heritable
    /// [`MatePreference`] over the genetic distance to each candidate (design Part 25; R-REPRO).
    /// The chooser is excluded from its own candidate set, and a candidate with no recorded
    /// genome is skipped. Returns the chosen candidate's id, or `None` if the chooser has no
    /// genome or preference or no eligible candidate remains.
    ///
    /// This is the call site the R-REPRO follow-on named: births today take both parents
    /// pre-chosen, and this lets the chooser pick under its inherited preference. The mechanism
    /// authors no direction: it reads the chooser's own preference weight, whose sign is the
    /// heritable, dawn-seeded, birth-inherited trait selection shapes. The incompatibility axis
    /// (the viability cue a distance preference cannot carry) is proven in [`crate::mate_choice`]
    /// and rides a further change that gives the choose site a Dobzhansky-Muller table.
    pub fn choose_mate(&self, chooser: StableId, candidates: &[StableId]) -> Option<StableId> {
        let pref = self.mate_prefs.get(&chooser)?;
        let chooser_genome = self.genomes.get(&chooser)?;
        let mut ids = Vec::with_capacity(candidates.len());
        let mut genomes = Vec::with_capacity(candidates.len());
        for &c in candidates {
            if c == chooser {
                continue;
            }
            if let Some(g) = self.genomes.get(&c) {
                ids.push(c);
                genomes.push(g.clone());
            }
        }
        let idx = choose(pref, chooser_genome, &genomes)?;
        Some(ids[idx])
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
        // The dogmatism/freezing split weight is a reserved value; without it the anchor cannot be
        // computed without fabricating a half, so the round is a no-op until it is installed.
        let split = match self.stubbornness_split {
            Some(w) => w,
            None => return,
        };
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
                    let theta = epistemic.effective_stubbornness(ax.stubbornness, split);
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
        // The dogmatism/freezing split weight is reserved; the round is a no-op until it is installed
        // rather than fabricating a half.
        let split = match self.stubbornness_split {
            Some(w) => w,
            None => return,
        };
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
                    let theta = epistemic.effective_stubbornness(ax.stubbornness, split);
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

    /// The band-mean memory capacity of a founding band: the arithmetic mean of every member's
    /// [`Mind::memory`], folded in canonical [`StableId`] order (design Part 33.4, the
    /// R-LANG-DET salience decay). This is the representative memory a
    /// [`crate::language::SalienceDecayLaw`] reads to set the band's concept-salience decay
    /// rate, so the rate a founding culture's lexicon leaks at is a consequence of who founds it
    /// rather than an authored constant. The members are visited in sorted id order, and the sum
    /// accumulates in 128-bit space before a single divide, so the mean is bit-identical
    /// regardless of the order the members are supplied or the thread count (Principle 3).
    /// Returns `None` if no member of the band has a mind (nothing to average), so the caller
    /// never divides by zero or invents a mean. The fold reads only per-being memory, never a
    /// race id, so two bands of different composition give different rates from one rule.
    pub fn band_mean_memory(&self, members: &[StableId]) -> Option<Fixed> {
        let mut ids: Vec<StableId> = members.to_vec();
        ids.sort();
        let mut numerator: i128 = 0;
        let mut count: i128 = 0;
        for id in ids {
            if let Some(mind) = self.minds.get(&id) {
                numerator += mind.memory.to_bits() as i128;
                count += 1;
            }
        }
        if count == 0 {
            return None;
        }
        Some(Fixed::from_bits((numerator / count) as i64))
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
    /// fresh empty evidence ring sized from the child's own `child_memory` through `ring_law`
    /// ([`RingCapacityLaw::capacity_for`]), not copied from the parent axiom's cap. Returns the
    /// child's id, or `None` if the parent holds no intrinsic beliefs.
    ///
    /// This is the intrinsic-belief half of a birth, decoupled from `self.minds`: the caller
    /// pushes the child's expressed memory in (the axiom-only harness passes [`Fixed::ONE`], the
    /// neutral memory of a bare being). The genome half (a genome from `GeneticScheme::reproduce`
    /// and a mind from [`Mind::from_genome`]) and combining the two into one birth are
    /// [`World::birth`], which passes the child's own expressed memory. Deterministic: the draw is
    /// keyed on the child's canonical id, so it is reproducible as long as birth order is a
    /// deterministic function of canonical state (an observer-driven birth path would key on a
    /// birth-event coordinate instead, the Principle 10 caveat).
    #[allow(clippy::too_many_arguments)]
    pub fn inherit_child(
        &mut self,
        parent: StableId,
        band: &[StableId],
        heritability: Fixed,
        mutation_spread: Fixed,
        generation: u64,
        child_memory: Fixed,
        ring_law: &RingCapacityLaw,
    ) -> Option<StableId> {
        let child = self.reg.mint();
        let beliefs = self.inherited_beliefs(
            child,
            parent,
            band,
            heritability,
            mutation_spread,
            generation,
            child_memory,
            ring_law,
        )?;
        self.intrinsic.insert(child, beliefs);
        Some(child)
    }

    /// The intrinsic beliefs a child of `parent` inherits, keyed on the already-minted
    /// `child` id (design Part 28). Shared by [`World::inherit_child`] and [`World::birth`]: for
    /// each axiom the parent holds, the child's innate seed (and starting stance) is the
    /// heritable-plus-encultured blend of the parent's seed and the band's local mean plus a
    /// bounded mutation drawn under [`Phase::AXIOM_INHERIT`] keyed on the child and the axis;
    /// the child copies the parent's epistemic stance and values and gets fresh evidence rings
    /// sized from the child's own `child_memory` through `ring_law`. Kept decoupled from
    /// `self.minds` so the axiom-only harness can drive it with an explicit memory; `None` if the
    /// parent holds no intrinsic beliefs.
    #[allow(clippy::too_many_arguments)]
    fn inherited_beliefs(
        &self,
        child: StableId,
        parent: StableId,
        band: &[StableId],
        heritability: Fixed,
        mutation_spread: Fixed,
        generation: u64,
        child_memory: Fixed,
        ring_law: &RingCapacityLaw,
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
            // Draw the belief-mutation deviate through the world's stamped Gaussian approximation
            // (design 25.10), the one shape the genome and controller mutations also draw through,
            // rather than a bare k=12 literal at this consumer.
            let deviate = gaussian_unit(
                &DrawKey::pair(child.0, pax.axis.0 as u64, generation, Phase::AXIOM_INHERIT)
                    .rng(self.seed),
                0,
                self.gauss_approx,
            );
            let seed = axiom::inherit_seed(
                pax.innate_seed,
                local_mean,
                heritability,
                mutation_spread,
                deviate,
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
                evidence: EvidenceRing::new(ring_law.capacity_for(child_memory)),
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
    /// and the local band (the heritable-plus-encultured blend), with each axiom's evidence ring
    /// sized from the child's own recombined memory through `ring_law`. The child is registered
    /// with a genome, a mind, and intrinsic beliefs; the caller places it. Returns the child id,
    /// or `None` if either parent has no genome or the first parent has no beliefs.
    ///
    /// The genome and the mind are expressed before the beliefs are inherited, so the ring cap
    /// reads the child's own recombined memory phenotype rather than the first parent's cap; the
    /// counter-keyed RNG makes the ordering immaterial to determinism.
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
        ring_law: &RingCapacityLaw,
    ) -> Option<StableId> {
        let genome_a = self.genomes.get(&parent_a)?.clone();
        let genome_b = self.genomes.get(&parent_b)?.clone();
        let child = self.reg.mint();
        // Express the child's genome and mind first, so its evidence ring is sized from its own
        // recombined memory phenotype rather than the first parent's cap (the counter-keyed RNG
        // is order-independent, so computing these before the belief inheritance does not change
        // any draw).
        let child_genome = race.scheme.reproduce(
            &genome_a,
            parent_a.0,
            &genome_b,
            parent_b.0,
            race.genes.genes.len(),
            self.seed,
            generation,
        );
        // Express the child's mind on the race's environment baseline plus its own mean-zero
        // developmental offset (design Part 25.6), keyed on the child's id and this generation.
        // Two siblings recombined identically from one pair of parents at one generation share a
        // genome but draw distinct offsets from their distinct ids, so V_E makes their expressed
        // minds differ; at the reserved interim `environment_variance` of zero the offset vanishes
        // and this reproduces the pre-offset birth.
        let env =
            race.environment + env_offset(self.seed, child, generation, race.environment_variance);
        let mind = Mind::from_genome(child, &race.genes, &child_genome, env);
        let beliefs = self.inherited_beliefs(
            child,
            parent_a,
            band,
            heritability,
            mutation_spread,
            generation,
            mind.memory,
            ring_law,
        )?;
        self.minds.insert(child, mind);
        self.genomes.insert(child, child_genome);
        self.intrinsic.insert(child, beliefs);
        self.ages.insert(child, 0);
        self.race_of.insert(child, race.id);
        // Seed a birth-neutral personality if the race carries a profile (inert otherwise).
        self.seed_personality(child, race.id);
        // Inherit the mate preference as a quantitative trait: the midparent of the two parents'
        // weights plus a bounded mutation drawn under Phase::MATE_CHOICE keyed on the child,
        // scaled by the same `mutation_spread` the belief inheritance uses (so no new value is
        // introduced). A parent with no recorded preference contributes indifference (zero). The
        // child is clamped to [-1, 1], and only the sign it inherits, not the mechanism, carries
        // a direction.
        let a_w = self
            .mate_prefs
            .get(&parent_a)
            .map(|p| p.distance_weight)
            .unwrap_or(Fixed::ZERO);
        let b_w = self
            .mate_prefs
            .get(&parent_b)
            .map(|p| p.distance_weight)
            .unwrap_or(Fixed::ZERO);
        let midparent = (a_w + b_w).mul(Fixed::from_ratio(1, 2));
        let unit = DrawKey::pair(child.0, 0, generation, Phase::MATE_CHOICE)
            .rng(self.seed)
            .unit_fixed(0);
        let mutation = (Fixed::from_int(2).mul(unit) - Fixed::ONE).mul(mutation_spread);
        let child_w = (midparent + mutation).clamp(Fixed::from_int(-1), Fixed::from_int(1));
        self.mate_prefs.insert(child, MatePreference::new(child_w));
        // Credit the reproductive census (design Part 25, R-REPRO). Sex is a gene-fed phenotype read
        // off the sex-determination locus, deterministic and RNG-free: the child's sex, and each
        // contributing parent's sex, are expressed the same way any other channel is. A two-parent
        // (sexual diploid) birth credits both parents once; a single-parent (haploid or clonal)
        // birth credits only the first, so the offspring tally stays exactly the summed parental
        // contribution. An unknown sex (no registered breeding system) folds into the default class,
        // so the tally never leaks a credit even before mating types are declared.
        let child_sex = self
            .genomes
            .get(&child)
            .and_then(|g| self.express_sex(race, g))
            .unwrap_or_default();
        let a_sex = self
            .genomes
            .get(&parent_a)
            .and_then(|g| self.express_sex(race, g))
            .unwrap_or_default();
        let mut parents = vec![(parent_a, a_sex)];
        if matches!(race.scheme.reproduction, ReproductionMode::SexualDiploid) {
            let b_sex = self
                .genomes
                .get(&parent_b)
                .and_then(|g| self.express_sex(race, g))
                .unwrap_or_default();
            parents.push((parent_b, b_sex));
        }
        self.census.record_birth(&parents, child, child_sex);
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

    /// A being's race, if its identity is tracked (seeded at the dawn and at birth). A being with
    /// no recorded race returns `None`, and the race-keyed mortality pass falls back to raw-age
    /// hazard for it (see [`World::apply_mortality_by_race`]).
    pub fn race_of(&self, id: StableId) -> Option<RaceId> {
        self.race_of.get(&id).copied()
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
                    .eval(crate::demography::hazard_age(age))
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

    /// Run one mortality pass that evaluates the hazard at each being's race-normalized life
    /// fraction rather than its raw age (design Part 20, R-AGING). For each being in id order, its
    /// race is looked up through `race_of` and the hazard is evaluated at [`Race::life_fraction`]
    /// (raw age divided by that race's own lifespan), so one shared curve culls a short-lived and a
    /// long-lived race each on its own scale, keyed only off per-race data (Principle 9). The
    /// `hazard` curve here is defined on the life-fraction domain `[0, 1]`, distinct from the
    /// raw-age curve [`World::apply_mortality`] reads.
    ///
    /// A being whose race is untracked, or whose race is absent from `races`, cannot be mapped onto
    /// the life-fraction domain (there is no lifespan to normalize its age by), so the pass FAILS
    /// LOUD with [`UnraceableBeing`] rather than reading the curve in the wrong domain. The earlier
    /// fallback evaluated such a being at its RAW age against this fraction curve, a domain mismatch
    /// that read the curve's far end (making the unraced class either near-immortal or culled
    /// wholesale depending on the curve). The check runs before any removal, so a refused pass
    /// leaves the population untouched. Only the curve's x-input differs from
    /// [`World::apply_mortality`]: the [`Phase::MORTALITY`] roll, the comparison, and the removal
    /// path are identical, so the two share a deterministic, observer-independent,
    /// thread-count-independent roll.
    pub fn apply_mortality_by_race(
        &mut self,
        races: &BTreeMap<RaceId, Race>,
        hazard: &Curve,
    ) -> Result<Vec<StableId>, UnraceableBeing> {
        // Resolve every being's life fraction first, refusing on the first being that cannot be
        // race-normalized, so no partial cull runs before the refusal.
        let mut fractions: Vec<(StableId, u32, Fixed)> = Vec::with_capacity(self.ages.len());
        for (&id, &age) in &self.ages {
            let race = self
                .race_of
                .get(&id)
                .and_then(|rid| races.get(rid))
                .ok_or(UnraceableBeing(id))?;
            fractions.push((id, age, race.life_fraction(age)));
        }
        let dead: Vec<StableId> = fractions
            .into_iter()
            .filter_map(|(id, age, x)| {
                let chance = hazard.eval(x).clamp(Fixed::ZERO, Fixed::ONE);
                let roll = DrawKey::entity(id.0, age as u64, Phase::MORTALITY)
                    .rng(self.seed)
                    .unit_fixed(0);
                (roll < chance).then_some(id)
            })
            .collect();
        for id in &dead {
            self.remove_being(*id);
        }
        Ok(dead)
    }

    /// The life-cadence beat, run once per [`World::tick`] but firing only on the cadence period
    /// (design Part 20, R-AGING). On a tick whose clock is a whole multiple of
    /// [`World::set_life_cadence`]'s period, every tracked being ages one step and then, if an
    /// age-hazard curve is installed, faces the mortality roll; on every other tick it is a no-op,
    /// so a run shorter than one cadence never ages. Aging precedes mortality, matching the
    /// established order the generational-turnover test uses, so a being faces the hazard at its
    /// new age. The order is pinned and both halves are deterministic (id-ordered, counter-keyed),
    /// so the beat replays bit for bit and is independent of the field-worker width in the composed
    /// runner. Without an installed hazard only aging beats: the world never runs on a fabricated
    /// hazard shape.
    fn life_cadence(&mut self) {
        if self.clock == 0 || !self.clock.is_multiple_of(self.life_cadence_ticks) {
            return;
        }
        self.age_step();
        self.drift_personalities();
        self.reproduce();
        // Mortality: the life-fraction by-race curve takes precedence (short- and long-lived races
        // cull on their own timescales); else the raw-age curve; else no cull. The by-race pass reads
        // the race registry, so it is moved out and back around the mutable cull (a cheap pointer swap
        // that cannot change state). A by-race pass that meets an unraceable being refuses rather than
        // partially culling, so the population is left intact on a config error.
        if let Some(hazard) = self.mortality_hazard_by_race.clone() {
            let races = std::mem::take(&mut self.races);
            let _ = self.apply_mortality_by_race(&races, &hazard);
            self.races = races;
        } else if let Some(hazard) = self.mortality_hazard.clone() {
            self.apply_mortality(&hazard);
        }
    }

    /// Whether two beings of a race can mate: both express a sex class through the race's breeding
    /// system and the two classes are compatible (design Part 25, R-REPRO). A race with no registered
    /// breeding system, or a being with no genome, cannot pair (fail-quiet), so reproduction requires a
    /// declared breeding system. Reads the gene-fed sex phenotype, never a race branch (Principle 9).
    fn sexes_compatible(&self, race: &Race, a: StableId, b: StableId) -> bool {
        let system = match self.breeding_systems.get(race.breeding) {
            Some(s) => s,
            None => return false,
        };
        let ga = match self.genomes.get(&a) {
            Some(g) => g,
            None => return false,
        };
        let gb = match self.genomes.get(&b) {
            Some(g) => g,
            None => return false,
        };
        match (self.express_sex(race, ga), self.express_sex(race, gb)) {
            (Some(sa), Some(sb)) => system.compatible(sa, sb),
            _ => false,
        }
    }

    /// The reproduce half of the life cadence (design Parts 25, 28, R-REPRO, the keystone that makes a
    /// run grow as well as shrink). Within each band, mature compatible beings pair under their own
    /// heritable mate preference, and each pair bears one child that inherits both halves of its being
    /// (genome recombined under the race's scheme, mind expressed from it, intrinsic beliefs and mate
    /// preference inherited from the parents and the band), so lexicons, axioms, and mate preferences
    /// carry across generations and lineages persist and diverge. Fecundity falls out of the structure,
    /// not an authored rate: a mature, compatible pair bears one child per cadence, so lifetime
    /// fecundity is set by maturity, lifespan, and the cadence.
    ///
    /// Two passes so it is worker-count independent and free of within-pass order effects (Principle
    /// 3): pass one is a pure read walk that pairs the mature beings of each band in canonical
    /// `(PlaceId, StableId)` order, each being paired at most once with the lower-id partner initiating
    /// under its own preference; pass two bears and places the children. Every draw the birth makes
    /// keys through [`civsim_core::Phase::REPRODUCE`] and [`civsim_core::Phase::MATE_CHOICE`] on the
    /// parents and the generation, never a sequential index, so replay is bit-exact and observer
    /// independent. Inert until [`World::set_reproduction`] installs the calibrations. Reads no race
    /// branch: the pairing keys off the gene-fed sex phenotype and the heritable preference alone.
    fn reproduce(&mut self) {
        let params = match &self.reproduction {
            Some(p) => p.clone(),
            None => return,
        };
        if self.life_cadence_ticks == 0 {
            return;
        }
        let generation = self.clock / self.life_cadence_ticks;

        // Group the mature beings by band (place), in canonical id order.
        let mut by_place: BTreeMap<PlaceId, Vec<StableId>> = BTreeMap::new();
        for id in self.minds.keys().copied().collect::<Vec<_>>() {
            let (Some(place), Some(race_id)) = (
                self.place_of.get(&id).copied(),
                self.race_of.get(&id).copied(),
            ) else {
                continue;
            };
            let Some(race) = self.races.get(&race_id) else {
                continue;
            };
            let age = self.ages.get(&id).copied().unwrap_or(0);
            if race.is_mature(age) {
                by_place.entry(place).or_default().push(id);
            }
        }

        // Pass one (pure read): pair mature, compatible beings within each band. The lower-id being
        // initiates and picks its mate under its own preference; each being pairs at most once.
        let mut pairs: Vec<(RaceId, PlaceId, StableId, StableId)> = Vec::new();
        for (place, members) in &by_place {
            let mut paired: std::collections::BTreeSet<StableId> =
                std::collections::BTreeSet::new();
            for &chooser in members {
                if paired.contains(&chooser) {
                    continue;
                }
                let Some(race_id) = self.race_of.get(&chooser).copied() else {
                    continue;
                };
                let Some(race) = self.races.get(&race_id) else {
                    continue;
                };
                let candidates: Vec<StableId> = members
                    .iter()
                    .copied()
                    .filter(|&c| {
                        c != chooser
                            && !paired.contains(&c)
                            && self.race_of.get(&c) == Some(&race_id)
                            && self.sexes_compatible(race, chooser, c)
                    })
                    .collect();
                if candidates.is_empty() {
                    continue;
                }
                if let Some(mate) = self.choose_mate(chooser, &candidates) {
                    paired.insert(chooser);
                    paired.insert(mate);
                    pairs.push((race_id, *place, chooser, mate));
                }
            }
        }

        // Pass two (the write walk): each pair bears one child, placed into its band. The race is
        // cloned out so the mutable birth borrow does not conflict with the race registry; heritability
        // derives from the race's own pool (V_A over V_A+V_E), never authored.
        for (race_id, place, a, b) in pairs {
            let Some(race) = self.races.get(&race_id).cloned() else {
                continue;
            };
            let band: Vec<StableId> = by_place.get(&place).cloned().unwrap_or_default();
            let heritability = race
                .pool
                .narrow_sense_heritability(race.environment_variance);
            if let Some(child) = self.birth(
                &race,
                a,
                b,
                &band,
                heritability,
                params.mutation_spread,
                generation,
                &params.ring_law,
            ) {
                self.place_of.insert(child, place);
            }
        }
    }

    /// The personality beat of the life cadence (design Part 20, R-BEING-REP): every tracked being
    /// whose race carries a personality profile drifts its traits one step toward that race's
    /// maturity targets at its own age-scaled plasticity, through the shared
    /// [`crate::personality::age_personality`] kernel. Beings are walked in canonical id order (the
    /// `BTreeMap` order), and the drift is a pure fixed-point function of each being's new age and its
    /// per-race data, with no RNG and no race branch (Principles 3, 9), so the beat replays bit for
    /// bit. It runs after aging (so a being drifts at its new age) and before mortality, matching the
    /// established life-cadence order. Inert when no personality registry is installed or no being
    /// carries an instance, so the world never drifts on a fabricated profile.
    fn drift_personalities(&mut self) {
        if self.personality.is_empty() || self.traits.is_empty() {
            return;
        }
        for (id, inst) in self.traits.iter_mut() {
            let Some(race_id) = self.race_of.get(id).copied() else {
                continue;
            };
            let Some(profile) = self.personality.profile(race_id) else {
                continue;
            };
            let Some(race) = self.races.get(&race_id) else {
                continue;
            };
            let age = self.ages.get(id).copied().unwrap_or(0);
            age_personality(inst, profile, race, age);
        }
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
        self.mate_prefs.remove(&id);
        self.affect.remove(&id);
        self.ages.remove(&id);
        self.race_of.remove(&id);
        self.sensorium.remove(&id);
        self.traits.remove(&id);
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

    /// Whether an authored decision repertoire (drives, curves, actions) is installed. This is the
    /// sentient deliberative tier of design Part 8.1, an AUTHORED action-and-drive policy that Part
    /// 8.4 marks as steering at the level of behaviour, distinct from the emergent evolved controller.
    /// The canonical runner reads this to keep that authored path off its emergent-behaviour spine
    /// (`crate::runner::Runner::with_world`).
    pub fn has_behaviour(&self) -> bool {
        self.behaviour.is_some()
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
    /// mind is ignored. The cognition phases run first, then the coarse life-cadence beat
    /// (`life_cadence`) ages the tracked beings and rolls mortality, but only on a tick that
    /// falls on the cadence period, so a short run sees only cognition (design Part 20,
    /// R-AGING).
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
        self.diffuse_beliefs();
        self.converse_language();
        self.drift_step();
        self.life_cadence();
    }

    /// A profiling aid, not part of the simulation: advance one tick with no stimuli and return the
    /// wall-clock nanoseconds spent in each of the six cognition phases (perceive, decide, converse,
    /// gossip, converse_language, drift_languages), so a benchmark can see where a tick's time goes.
    /// It produces exactly the state `tick(&[])` would, since it runs the same phases in the same
    /// order with an empty input batch, including the coarse life-cadence beat (run after the six,
    /// untimed because it fires only on the cadence period); the `Instant` it reads is non-canonical
    /// and never enters state, so the resulting hash is unchanged (Principle 3). This exists to
    /// answer "profile before optimizing" (Part 13), and it is compiled into the library only as a
    /// measurement tool.
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
        self.drift_step();
        ns[5] = s.elapsed().as_nanos();
        // The coarse life beat runs after the six cognition phases so tick_timed produces the same
        // state tick would; it is not one of the profiled six (it fires only on the cadence period).
        self.life_cadence();
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

    /// The belief-diffusion beat (design Part 54, the aggregate-tier contagion of the resolved
    /// R-EVIDENCE belief substrate): each band's prevailing beliefs advance one SI-logistic diffusion
    /// step toward saturation, raising the knowledge level from its seed toward one and carrying the
    /// extensive mass with it. Content-blind and RNG-free (Principles 9, 3): the rate is the reserved
    /// aggregate diffusion rate, identical for every belief; the within-band coupling is full (distance
    /// one); and the pools and their beliefs are walked in canonical place then [`BeliefKey`] order, so
    /// the beat replays bit for bit and is worker-count independent. Inert until the diffusion rate is
    /// installed. Spreading a band's belief to a neighbouring band on a distance lag waits on a place
    /// adjacency the abstract-place world does not yet carry (a named follow-on); this beat is the
    /// within-band climb.
    fn diffuse_beliefs(&mut self) {
        let rate = match self.belief_diffusion_rate {
            Some(r) => r,
            None => return,
        };
        // Walk pools in canonical place order, then each pool's beliefs in canonical key order.
        for pool in self.belief_pools.values_mut() {
            for key in pool.keys_in_order() {
                if let Some(belief) = pool.get_mut(&key) {
                    belief.advance_diffusion(rate, Fixed::ONE);
                }
            }
        }
    }

    /// The tick's drift beat: run [`World::drift_languages`] against the world's own race registry.
    /// The registry is moved out and back rather than borrowed so the drift pass can hold `&mut
    /// self` for the lexicon rewrites while reading each lineage's race maturity; the move is a
    /// cheap pointer swap and cannot change the state, so replay is bit-identical.
    fn drift_step(&mut self) {
        let races = std::mem::take(&mut self.races);
        self.drift_languages(&races);
        self.races = races;
    }

    /// The drift step (design 33.4): once per generation each lineage may innovate a regular
    /// form change, which is then applied in innovation order to every word its speakers hold,
    /// so the lineage's lexicon drifts as a unit and two separated lineages diverge into
    /// sisters. The generation length is not one global scalar: it DERIVES per lineage from the
    /// speaking race's `maturity_years` against the world's `life_cadence_ticks` (the orbital year
    /// in ticks), so lineages of races with different maturities drift on different cadences from
    /// one mechanism, and `races` is the registry the per-lineage race maturity is read from. A
    /// no-op until the drift calibration is set, and a lineage whose race is absent does not drift.
    /// Deterministic: each lineage's innovation is keyed by counter RNG on the lineage, its own
    /// generation, and the phase, and the speaker walk is id-ordered.
    fn drift_languages(&mut self, races: &BTreeMap<RaceId, Race>) {
        let params = match self.drift {
            Some(p) => p,
            None => return,
        };
        if self.languages.is_empty() || self.clock == 0 {
            return;
        }
        let base_cadence = self.life_cadence_ticks;
        let lang_ids: Vec<LangId> = self.languages.keys().copied().collect();
        for lang_id in lang_ids {
            // The drift cadence DERIVES per lineage from the speaking race's own maturity: a
            // generation is that race's `maturity_years` in world-time, its maturity times the
            // orbital year in ticks (`life_cadence_ticks`, itself derived from the world's orbit).
            // Two lineages of races with different `maturity_years` therefore beat drift on
            // different cadences from this one mechanism, retiring the single Earth-year scalar. A
            // lineage whose race is absent from the registry has no maturity to derive a cadence
            // from and does not drift (a fabricated cadence is never invented, Principle 11).
            let Some(race) = self
                .languages
                .get(&lang_id)
                .and_then(|l| races.get(&l.race()))
            else {
                continue;
            };
            let cadence = (race.maturity_years as u64)
                .saturating_mul(base_cadence)
                .max(1);
            if !self.clock.is_multiple_of(cadence) {
                continue;
            }
            let generation = self.clock / cadence;
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
        // Gather pass: every located being rolls, per public trace it can sense, whether it notices
        // it. The rolls are draw-keyed per (being, trace, clock), so the hit set is a pure function of
        // the state and the gather parallelises across `workers` threads (the ActionStage pattern the
        // converse phase proves). The sorted traces are split into contiguous chunks, each worker
        // gathers its chunk's hits, and the chunks are concatenated in order, which reproduces the
        // serial hit sequence exactly, so the phase is bit-identical at every worker count.
        let mut traces: Vec<&Trace> = self.traces.iter().collect();
        traces.sort_by_key(|t| t.id);
        let workers = self.workers.max(1);
        let n = traces.len();
        let hits: Vec<PerceptionHit> = if workers == 1 || n <= 1 {
            let mut out = Vec::new();
            for t in &traces {
                self.gather_trace(t, &mut out);
            }
            out
        } else {
            // Dynamic load balancing, heterogeneous-core aware: each worker pulls the next trace
            // index from a shared counter, so a faster core (an Intel P-core, or an AMD V-cache CCD)
            // gathers more traces than a slower one, and no thread idles at the barrier on an equal
            // static slice. The gather writes nothing (a pure read of the frozen `&World`, which is
            // Sync). Each worker tags its output with the trace index; the merge sorts by that index
            // (a `traces`-length key, not a per-hit one, so it is cheap) and flattens, which
            // reproduces the serial trace-then-being order exactly. So the result is a pure function
            // of state whatever core gathered which trace: that is what lets the schedule adapt to a
            // hybrid CPU without touching correctness, and without a per-hit sort that would Amdahl-
            // cap the phase.
            let this: &World = &*self;
            let next = std::sync::atomic::AtomicUsize::new(0);
            let mut indexed: Vec<(usize, Vec<PerceptionHit>)> = std::thread::scope(|sc| {
                let handles: Vec<_> = (0..workers)
                    .map(|_| {
                        sc.spawn(|| {
                            let mut local: Vec<(usize, Vec<PerceptionHit>)> = Vec::new();
                            loop {
                                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                if i >= n {
                                    break;
                                }
                                let mut out = Vec::new();
                                this.gather_trace(traces[i], &mut out);
                                if !out.is_empty() {
                                    local.push((i, out));
                                }
                            }
                            local
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .flat_map(|h| h.join().expect("a perceive worker panicked"))
                    .collect()
            });
            indexed.sort_by_key(|(i, _)| *i);
            indexed.into_iter().flat_map(|(_, v)| v).collect()
        };
        // Apply pass (single-threaded): each noticed being integrates the evidence into its beliefs,
        // in the canonical trace-then-being order the gather reproduced. `consider` is
        // order-independent, so this apply is bit-identical whatever the worker count.
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

    /// Gather the perception hits for one trace across all located, sensing beings: the inner pass of
    /// [`perceive`], a pure read of `&self` (the minds, places, sensoria, clock, and seed), so it runs
    /// on a worker thread with no contention. Each being's notice roll is draw-keyed by
    /// `(being, trace, clock, PERCEPTION)`, so the hit set is a pure function of the state, independent
    /// of the worker that computed it.
    fn gather_trace(&self, t: &Trace, out: &mut Vec<PerceptionHit>) {
        for (mind_id, mind) in &self.minds {
            if self.place_of.get(mind_id) != Some(&t.place) {
                continue;
            }
            // Channel gate (R-SENSORIUM): a being with an installed sensorium perceives the trace
            // only on a channel it reads, and its channel acuity scales the roll; a being with no
            // sensorium reads every channel at full acuity, so the place-based perception of every
            // existing world is unchanged.
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
            // The being's age in life-cadence steps and its live personality trait values (axis
            // order). Both ride the aging cadence, but a divergent age or personality trajectory is a
            // divergent world (mortality and future drift key off them), so they surface in the
            // fingerprint rather than only in the drift they later produce. An untracked being folds a
            // sentinel age and an empty trait run, each length-prefixed so a boundary is unambiguous.
            h.write_u32(self.ages.get(id).copied().unwrap_or(u32::MAX));
            match self.traits.get(id) {
                None => h.write_u64(u64::MAX),
                Some(inst) => {
                    let entries: Vec<(crate::personality::TraitAxisId, Fixed)> =
                        inst.entries().collect();
                    h.write_u64(entries.len() as u64);
                    for (axis, value) in entries {
                        h.write_u32(axis.0);
                        h.write_fixed(value);
                    }
                }
            }
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
        // Language lineages, by id: race (the drift-cadence datum), parent, and the
        // regular-form-change log.
        for (id, lang) in &self.languages {
            h.write_u32(id.0);
            h.write_u32(lang.race().0);
            // The lineage's race maturity and lifespan in life-cadence steps: the per-lineage drift
            // cadence derives its generation length from maturity_years, and mortality normalizes age
            // by lifespan_years, so two worlds whose lineages age or drift on different schedules are
            // different worlds. These fold alongside the global life_cadence and mortality_hazard
            // below, so a change to a race's maturity or lifespan surfaces in the fingerprint at once
            // rather than only through the drift it later produces. A lineage whose race is absent
            // folds a sentinel pair, distinct from any real race's counts.
            match self.races.get(&lang.race()) {
                Some(race) => {
                    h.write_u32(race.maturity_years);
                    h.write_u32(race.lifespan_years);
                }
                None => {
                    h.write_u32(u32::MAX);
                    h.write_u32(u32::MAX);
                }
            }
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
        // The life-cadence period and the installed mortality-hazard curve are canonical timeline
        // state (design Part 20): two worlds that age and die on different schedules are different
        // worlds, so they fold in at a pinned tail position, the way LivingWorld::state_hash folds
        // the orbit. Aging and mortality are RNG-free beats keyed off these, so an unfolded change
        // would silently diverge replay. The curve folds as its length then its ascending-x points;
        // an absent hazard folds as a sentinel length distinct from any real curve.
        h.write_u64(self.life_cadence_ticks);
        for hazard in [&self.mortality_hazard, &self.mortality_hazard_by_race] {
            match hazard {
                None => h.write_u64(u64::MAX),
                Some(curve) => {
                    let pts = curve.points();
                    h.write_u64(pts.len() as u64);
                    for (x, y) in pts {
                        h.write_fixed(*x);
                        h.write_fixed(*y);
                    }
                }
            }
        }
        // The reproductive census (design Part 25, R-REPRO): the per-parent offspring tallies and the
        // gene-fed sex classes are canonical state, so two worlds whose reproduction diverged fold to
        // different hashes. Walked in canonical id order inside census::hash_into.
        self.census.hash_into(&mut h);
        // The aggregate belief pools, by place then belief key (canonical walk, R-CANON-WALK): the
        // belief-diffusion trajectory is canonical state, so two worlds whose beliefs diffuse to
        // different levels are different worlds. The place is length-agnostic here (the map is already
        // ordered), and each pool folds its own length-prefixed key-ordered contents.
        h.write_u64(self.belief_pools.len() as u64);
        for (place, pool) in &self.belief_pools {
            h.write_u32(*place);
            pool.hash_into(&mut h);
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

/// Grow the shared etic comparison substrate bottom-up from the emic value axes that recur
/// across races (design Part 21, the R-VALUE-METRIC `value_metric.etic_substrate_axes`
/// calibration). Cross-race value comparison passes through a shared [`EticSubstrate`], but
/// its membership is not an authored human set: an etic axis is minted for each emic
/// [`ValueAxisId`] whose race-count reaches `recurrence_min`, in ascending emic-id order, so
/// the substrate carries exactly the axes that recur and an idiosyncratic axis held
/// by one race stays private (a blind spot in cross-race comparison). Each race then projects
/// identically onto the shared axes it carries: an emic axis that made the substrate maps onto
/// its minted etic axis with unit weight ([`project_to_etic`] reads these), and a private axis
/// has no projection and is absent. The mechanism is fixed Rust and iterates races and axes
/// generically in canonical id order, never branching on a specific race id (Principle 9); the
/// membership is a consequence of the per-race value profiles (Principle 11). Returns the
/// substrate and the per-race projections; a `recurrence_min` above the race count yields an
/// empty substrate and empty projections. Deterministic by construction: the counts and the
/// mint walk sorted maps.
///
/// [`project_to_etic`]: crate::value::project_to_etic
pub fn build_etic_substrate(
    races: &BTreeMap<RaceId, ValueProfile>,
    recurrence_min: usize,
) -> (EticSubstrate, BTreeMap<RaceId, RaceProjection>) {
    // Count, per emic axis, how many races carry a stance on it. A BTreeMap walks axes in
    // ascending id order, so the recurrence pass is canonical.
    let mut recurrence: BTreeMap<ValueAxisId, usize> = BTreeMap::new();
    for profile in races.values() {
        for (axis, _stance) in profile.axes() {
            *recurrence.entry(axis).or_insert(0) += 1;
        }
    }
    // Mint one fresh etic axis per recurring emic axis, in ascending emic-id order, and record
    // the emic-to-etic map the identity projections read.
    let mut shared: BTreeMap<ValueAxisId, EticAxisId> = BTreeMap::new();
    let mut axes: Vec<EticAxisId> = Vec::new();
    for (&axis, &count) in &recurrence {
        if count >= recurrence_min {
            let etic = EticAxisId(axes.len() as u32);
            shared.insert(axis, etic);
            axes.push(etic);
        }
    }
    let substrate = EticSubstrate { axes };
    // Each race projects identically onto the shared axes it carries; a private axis contributes
    // no projection entry and stays absent.
    let mut projections: BTreeMap<RaceId, RaceProjection> = BTreeMap::new();
    for (&race, profile) in races {
        let mut per_axis: BTreeMap<ValueAxisId, EmicProjection> = BTreeMap::new();
        for (axis, _stance) in profile.axes() {
            if let Some(&etic) = shared.get(&axis) {
                per_axis.insert(
                    axis,
                    EmicProjection {
                        onto: vec![(etic, Fixed::ONE)],
                    },
                );
            }
        }
        projections.insert(race, RaceProjection { per_axis });
    }
    (substrate, projections)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::SalienceDecayLaw;

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
    fn a_seeded_belief_climbs_an_s_curve_to_saturation_and_replays() {
        // The belief-diffusion beat (design Part 54): a band's seeded belief climbs the SI logistic
        // toward saturation, monotone and without a one-step jump, and the trajectory is bit-for-bit
        // deterministic (no RNG). The extensive mass grows with the level, and the level reads back
        // exactly (mass over count).
        let key = BeliefKey {
            subject: StableId(1),
            attr: AttrKindId(0),
            value: 7,
        };
        let seed_level = Fixed::from_ratio(1, 20); // 0.05
        let run = || {
            let mut w = world().with_seed(0xBE11E);
            w.seed_belief(10, key, seed_level, 8);
            w.set_belief_diffusion_rate(Fixed::from_ratio(4, 10)); // a fixture rate
            let mut levels = Vec::new();
            for _ in 0..300 {
                w.tick(&[]);
                levels.push(w.belief_level(10, &key).unwrap());
            }
            (levels, w.state_hash())
        };
        let (levels, hash) = run();
        // Monotone climb with no one-step jump.
        let mut prev = seed_level;
        let mut max_step = Fixed::ZERO;
        for &lvl in &levels {
            assert!(
                lvl >= prev,
                "the level climbs monotonically: {lvl:?} < {prev:?}"
            );
            let step = lvl - prev;
            if step > max_step {
                max_step = step;
            }
            prev = lvl;
        }
        assert!(
            *levels.last().unwrap() > Fixed::from_ratio(9, 10),
            "climbs past 0.9 to saturation: {:?}",
            levels.last().unwrap()
        );
        assert!(
            max_step < Fixed::from_ratio(1, 2),
            "no one-step jump: max step {max_step:?}"
        );
        // Bit-for-bit replay, including the belief pool's contribution to the state hash.
        let (levels2, hash2) = run();
        assert_eq!(
            levels, levels2,
            "the diffusion trajectory replays bit for bit"
        );
        assert_eq!(
            hash, hash2,
            "the belief pool folds into a bit-identical state hash"
        );
        // Inert without a rate: the belief does not move.
        let mut inert = world().with_seed(0xBE11E);
        inert.seed_belief(10, key, seed_level, 8);
        for _ in 0..50 {
            inert.tick(&[]);
        }
        assert_eq!(
            inert.belief_level(10, &key),
            Some(seed_level),
            "without a diffusion rate the beat is inert"
        );
    }

    #[test]
    fn the_state_hash_folds_ages_and_personality_trait_trajectories() {
        // Defect 7: a being's age and live personality ride the aging cadence, but a divergent age or
        // personality trajectory is a divergent world (mortality and future drift key off them), so
        // they surface in the fingerprint rather than only in the drift they later produce.
        use crate::personality::{TraitAxisId, TraitInstance};
        let hash_of = |age: u32, trait_value: Option<Fixed>| -> u128 {
            let mut w = world();
            let a = w.spawn(Fixed::ONE);
            w.set_age(a, age);
            if let Some(v) = trait_value {
                w.install_personality(a, TraitInstance::from_values([(TraitAxisId(0), v)]));
            }
            w.state_hash()
        };
        let base = hash_of(5, None);
        assert_eq!(
            base,
            hash_of(5, None),
            "identical age and trait state hashes the same"
        );
        assert_ne!(
            base,
            hash_of(9, None),
            "a divergent age trajectory surfaces in the fingerprint"
        );
        let drifted = hash_of(5, Some(Fixed::from_ratio(1, 3)));
        let drifted_more = hash_of(5, Some(Fixed::from_ratio(2, 3)));
        assert_ne!(
            base, drifted,
            "installing a personality trait surfaces in the fingerprint"
        );
        assert_ne!(
            drifted, drifted_more,
            "a divergent trait value surfaces in the fingerprint"
        );
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

    #[test]
    fn the_life_cadence_derives_from_the_orbit_via_from_manifest_with_orbital() {
        // The non-steering property at the world level: two worlds built from the same dev
        // manifest and channels, differing only in their orbit, get different life_cadence_ticks,
        // because the cadence derives from the orbital year over the base tick rather than a
        // hardcoded per-world number. Earth's orbit reproduces today's interim; a faster world
        // beats aging on a shorter year, all from one derivation.
        let manifest = CalibrationManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../calibration/profiles/dev-fixtures.toml"
        ))
        .expect("dev fixtures load");
        let chans = AccessChannelRegistry::from_toml_str(
            r#"
[[channels]]
id = 1
name = "witnessed"
margin_steps = 1
[[channels]]
id = 2
name = "told"
margin_steps = 0
[[channels]]
id = 3
name = "said"
margin_steps = -1
"#,
        )
        .unwrap();
        let earth = World::from_manifest_with_orbital(
            &manifest,
            &chans,
            Profile::Development,
            OrbitalElements::dev_earth(),
        )
        .expect("earth world builds");
        assert_eq!(earth.life_cadence_ticks(), 31_536_000);
        let fast_orbit = OrbitalElements {
            orbital_period_seconds: Fixed::from_int(86_400),
            rotation_period_seconds: Fixed::from_int(3_600),
        };
        let fast =
            World::from_manifest_with_orbital(&manifest, &chans, Profile::Development, fast_orbit)
                .expect("fast world builds");
        assert_eq!(fast.life_cadence_ticks(), 86_400);
        assert_ne!(
            earth.life_cadence_ticks(),
            fast.life_cadence_ticks(),
            "two orbits, two cadences from one formula"
        );
    }

    /// A full inline manifest for the production `from_manifest` path, differing only in the orbital
    /// period, so the regression exercises the constructor that ships (not the test-only
    /// `from_manifest_with_orbital`).
    fn manifest_with_orbit(orbit_seconds: &str) -> String {
        let set = |id: &str, v: &str| {
            format!(
                "[[reserved]]\nid = \"{id}\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"{v}\"\nsource = \"s\"\n"
            )
        };
        [
            set("evidence.log_odds_clamp", "4"),
            set("evidence.commit_threshold", "2"),
            set("evidence.runner_up_margin", "1"),
            set("tom.meta_log_odds_clamp", "4"),
            set("tom.meta_commit_threshold", "2"),
            set("tom.meta_runner_up_margin", "1"),
            set("gossip.told_weight", "1"),
            set("gossip.trust_baseline", "0.5"),
            set("gossip.trust_penalty", "0.2"),
            set("time.base_tick_seconds", "1"),
            set("world.orbital_period_seconds", orbit_seconds),
            set("world.rotation_period_seconds", "86400"),
        ]
        .concat()
    }

    #[test]
    fn from_manifest_derives_the_cadence_from_the_manifest_orbit() {
        // Regression (audit CRITICAL defect 1): the PRODUCTION constructor `from_manifest` derives
        // the life cadence from the world's orbit read from the manifest, not from World::new's
        // Earth-year LIFE_CADENCE_TICKS fallback. Two manifests differing only in the orbital period
        // yield two different life_cadence_ticks, and neither is the bare 31_536_000 unless the orbit
        // says so.
        let chans = AccessChannelRegistry::default();
        let fast = CalibrationManifest::from_toml_str(&manifest_with_orbit("86400")).unwrap();
        let slow = CalibrationManifest::from_toml_str(&manifest_with_orbit("126144000")).unwrap();
        let wf = World::from_manifest(&fast, &chans, Profile::Calibrated).unwrap();
        let ws = World::from_manifest(&slow, &chans, Profile::Calibrated).unwrap();
        assert_eq!(
            wf.life_cadence_ticks(),
            86_400,
            "the fast orbit's year in ticks"
        );
        assert_eq!(
            ws.life_cadence_ticks(),
            126_144_000,
            "the slow orbit's year"
        );
        assert_ne!(
            wf.life_cadence_ticks(),
            ws.life_cadence_ticks(),
            "two orbital periods, two derived cadences"
        );
        assert_ne!(
            wf.life_cadence_ticks(),
            crate::clock::LIFE_CADENCE_TICKS,
            "the derived cadence is not the bare Earth constant unless the orbit says so"
        );
        // And an Earth orbit does reproduce the Earth constant (the orbit says so), so the constant
        // is not banished, only no longer the unconditional default.
        let earth = CalibrationManifest::from_toml_str(&manifest_with_orbit("31536000")).unwrap();
        let we = World::from_manifest(&earth, &chans, Profile::Calibrated).unwrap();
        assert_eq!(we.life_cadence_ticks(), crate::clock::LIFE_CADENCE_TICKS);
    }

    #[test]
    fn from_manifest_fails_loud_under_calibrated_when_the_orbit_is_reserved() {
        // The other half of defect 1: a calibrated world whose orbit is unset cannot run on the
        // Earth constant; it fails loud, because its year is unknown.
        let mut toml = manifest_with_orbit("31536000");
        toml = toml.replace(
            "[[reserved]]\nid = \"world.orbital_period_seconds\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"31536000\"\nsource = \"s\"\n",
            "[[reserved]]\nid = \"world.orbital_period_seconds\"\nbasis = \"b\"\nstatus = \"reserved\"\nsource = \"s\"\n",
        );
        let m = CalibrationManifest::from_toml_str(&toml).unwrap();
        assert!(
            World::from_manifest(&m, &AccessChannelRegistry::default(), Profile::Calibrated)
                .is_err()
        );
    }

    #[test]
    fn state_hash_folds_the_life_cadence_and_the_mortality_hazard() {
        // Regression (audit defect 4): two worlds differing only in the life cadence, or only in the
        // installed mortality hazard, get different state hashes, so those canonical timeline fields
        // cannot silently diverge replay.
        let make = || World::new(params(), params(), AccessWeights::default()).with_seed(0x5EED);
        // Two cadences.
        let mut a = make();
        let mut b = make();
        a.set_life_cadence(1000);
        b.set_life_cadence(2000);
        assert_ne!(
            a.state_hash(),
            b.state_hash(),
            "a different life cadence changes the state hash"
        );
        // Two hazards (same cadence).
        let mut c = make();
        let mut d = make();
        c.set_life_cadence(1000);
        d.set_life_cadence(1000);
        c.set_mortality_hazard(Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::ONE, Fixed::ONE),
        ]));
        d.set_mortality_hazard(Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::ONE, Fixed::from_ratio(1, 2)),
        ]));
        assert_ne!(
            c.state_hash(),
            d.state_hash(),
            "a different mortality-hazard curve changes the state hash"
        );
        // An absent hazard differs from a present one.
        let mut e = make();
        e.set_life_cadence(1000);
        assert_ne!(
            c.state_hash(),
            e.state_hash(),
            "installing a hazard changes the state hash"
        );
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
    fn perceive_is_bit_identical_across_worker_counts() {
        // The intra-phase parallelism (the throughput lever, not the phase-level scheduler): perceive
        // gathers per-(being, trace) notice rolls across worker threads, then applies in canonical
        // order. With many co-located sensing beings and many distinct traces at salience below one
        // (so the roll bites and only some beings notice), the parallel gather must reproduce the
        // serial result exactly at every worker count, because the rolls are draw-keyed and the trace
        // chunks concatenate in canonical order.
        let build = |workers: usize| -> u128 {
            let mut w = world().with_seed(0x9E5C_0A11);
            w.set_workers(workers);
            let place = 1u32;
            for _ in 0..24 {
                let m = w.spawn(Fixed::ONE);
                w.set_place(m, place);
            }
            for k in 0..16u64 {
                let value = if k % 2 == 0 { 10 } else { 20 };
                let mut t = trace(place, value, Fixed::from_ratio(1, 2));
                t.id = StableId(1000 + k);
                t.from = StableId(1000 + k);
                w.emit_trace(t);
            }
            w.perceive();
            w.state_hash()
        };
        let serial = build(1);
        for workers in [2usize, 3, 8, 16] {
            assert_eq!(
                build(workers),
                serial,
                "parallel perceive at {workers} workers diverged from the serial gather"
            );
        }
    }

    #[test]
    #[ignore] // a manual throughput check on the actual machine, not a CI gate
    fn perceive_timing_across_workers() {
        use std::time::Instant;
        let make = || {
            let mut w = world().with_seed(0xB16_B00C);
            let place = 1u32;
            for _ in 0..2000 {
                let m = w.spawn(Fixed::ONE);
                w.set_place(m, place);
            }
            for k in 0..2000u64 {
                let value = if k % 2 == 0 { 10 } else { 20 };
                let mut t = trace(place, value, Fixed::from_ratio(1, 2));
                t.id = StableId(100_000 + k);
                t.from = StableId(100_000 + k);
                w.emit_trace(t);
            }
            w
        };
        for workers in [1usize, 2, 4, 8, 16, 20] {
            let mut w = make();
            w.set_workers(workers);
            let t0 = Instant::now();
            w.perceive();
            eprintln!("perceive workers={:2}: {:?}", workers, t0.elapsed());
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

    #[test]
    fn the_life_cadence_beats_aging_only_on_its_period() {
        // With the cadence set to four ticks and no hazard installed, aging beats only on ticks 4
        // and 8, never in between, and nobody dies (mortality is a no-op without a curve).
        let mut w = world();
        w.set_life_cadence(4);
        let a = w.spawn(Fixed::ONE);
        w.set_age(a, 10);
        for _ in 0..3 {
            w.tick(&[]);
        }
        assert_eq!(w.age_of(a), Some(10), "no beat before the cadence period");
        w.tick(&[]); // clock == 4, the first beat
        assert_eq!(w.age_of(a), Some(11), "aged one step on the cadence tick");
        for _ in 0..3 {
            w.tick(&[]);
        }
        assert_eq!(w.age_of(a), Some(11), "no beat between cadence periods");
        w.tick(&[]); // clock == 8, the second beat
        assert_eq!(w.age_of(a), Some(12), "aged again on the next cadence tick");
        assert_eq!(w.population(), 1, "no hazard installed, so nobody died");
    }

    #[test]
    fn the_default_cadence_does_not_beat_in_a_short_run() {
        // Left at the owner-set default (one in-world year of ticks), a short run never ages, so
        // the wiring does not disturb the cognition-only tests that predate it.
        let mut w = world();
        let a = w.spawn(Fixed::ONE);
        w.set_age(a, 7);
        for _ in 0..100 {
            w.tick(&[]);
        }
        assert_eq!(
            w.age_of(a),
            Some(7),
            "the default cadence is far longer than the run"
        );
    }

    #[test]
    fn the_tick_ages_and_culls_on_the_cadence_and_replays() {
        // With the cadence set to one tick and a hazard installed, the tick ages every being and
        // rolls mortality each tick. The hazard is flat-zero below age fifty and rising above it,
        // so over an eight-tick run the three youngest (ages 0, 20, 40, none reaching fifty) are
        // guaranteed to survive and the eldest (age 100, certain death) is guaranteed to die,
        // making the cull deterministic regardless of the seed. Because ages are not folded into
        // the tick hash, the replay is proven directly on the surviving ages.
        let safe_zone_hazard = || {
            Curve::new([
                (Fixed::ZERO, Fixed::ZERO),
                (Fixed::from_int(50), Fixed::ZERO),
                (Fixed::from_int(100), Fixed::ONE),
            ])
        };
        let build = || {
            let mut w = world().with_seed(0x11FE);
            w.set_life_cadence(1);
            w.set_mortality_hazard(safe_zone_hazard());
            let ids: Vec<StableId> = (0..6).map(|_| w.spawn(Fixed::ONE)).collect();
            for (k, &id) in ids.iter().enumerate() {
                w.set_age(id, 20 * k as u32); // 0, 20, 40, 60, 80, 100
            }
            for _ in 0..8 {
                w.tick(&[]);
            }
            // The survivors as (founder index, age), so the replay compares stable coordinates.
            let survivors: Vec<(usize, u32)> = ids
                .iter()
                .enumerate()
                .filter_map(|(k, &id)| w.age_of(id).map(|age| (k, age)))
                .collect();
            (w.population(), survivors)
        };
        let (pop, survivors) = build();
        assert!(
            pop < 6,
            "the eldest died: the population was culled on the cadence"
        );
        assert!(pop >= 3, "the three in the flat-zero zone survived");
        // Every survivor aged by exactly the eight cadence beats it lived through, and the three
        // guaranteed survivors are the youngest founders.
        for &(k, age) in &survivors {
            assert_eq!(
                age,
                20 * k as u32 + 8,
                "a survivor aged one step per cadence beat"
            );
        }
        assert_eq!(
            (pop, survivors),
            build(),
            "the aged-and-culled world replays bit for bit"
        );
    }

    #[test]
    fn band_memory_sets_the_salience_decay_rate_per_composition() {
        let mut w = world();
        // A forgetful band and a sharp band. Each member's founding memory phenotype is set
        // directly; the band mean is the representative memory the law reads.
        let dull = [w.spawn(Fixed::ONE), w.spawn(Fixed::ONE)];
        for id in dull {
            w.minds.get_mut(&id).unwrap().memory = Fixed::from_ratio(1, 5);
        }
        let keen = [w.spawn(Fixed::ONE), w.spawn(Fixed::ONE)];
        for id in keen {
            w.minds.get_mut(&id).unwrap().memory = Fixed::from_ratio(4, 5);
        }
        let dull_mem = w.band_mean_memory(&dull).unwrap();
        let keen_mem = w.band_mean_memory(&keen).unwrap();
        assert_eq!(dull_mem, Fixed::from_ratio(1, 5));
        assert_eq!(keen_mem, Fixed::from_ratio(4, 5));
        // The fold is canonical: a reversed member slice gives the same mean.
        let rev: Vec<StableId> = dull.iter().rev().copied().collect();
        assert_eq!(w.band_mean_memory(&rev), Some(dull_mem));
        // A decreasing curve turns the two representative memories into two different rates,
        // with no race branch anywhere in the path.
        let law = SalienceDecayLaw {
            curve: Curve::new([
                (Fixed::ZERO, Fixed::from_ratio(1, 2)),
                (Fixed::ONE, Fixed::from_ratio(1, 20)),
            ]),
            floor: Fixed::from_ratio(1, 100),
        };
        let dull_rate = law.rate_for(dull_mem);
        let keen_rate = law.rate_for(keen_mem);
        assert!(
            dull_rate > keen_rate,
            "the forgetful band decays concept salience faster"
        );
        // A flat curve collapses both bands to one rate: the memory channel is switched off.
        let flat = SalienceDecayLaw {
            curve: Curve::new([
                (Fixed::ZERO, Fixed::from_ratio(1, 4)),
                (Fixed::ONE, Fixed::from_ratio(1, 4)),
            ]),
            floor: Fixed::from_ratio(1, 100),
        };
        assert_eq!(flat.rate_for(dull_mem), flat.rate_for(keen_mem));
        // An empty band has no representative memory, never a fabricated one.
        assert_eq!(w.band_mean_memory(&[]), None);
    }

    #[test]
    fn etic_substrate_grows_from_recurring_emic_axes() {
        // Three races over emic value axes. Axis 1 recurs in races 0 and 1; axis 3 recurs in
        // races 0 and 2; axes 5, 7, 9 are each private to one race.
        let mut races: BTreeMap<RaceId, ValueProfile> = BTreeMap::new();
        races.insert(
            RaceId(0),
            ValueProfile::with([
                (ValueAxisId(1), 1),
                (ValueAxisId(3), 1),
                (ValueAxisId(5), 1),
            ]),
        );
        races.insert(
            RaceId(1),
            ValueProfile::with([(ValueAxisId(1), 1), (ValueAxisId(7), 1)]),
        );
        races.insert(
            RaceId(2),
            ValueProfile::with([(ValueAxisId(3), 1), (ValueAxisId(9), 1)]),
        );
        let (substrate, projections) = build_etic_substrate(&races, 2);
        // Exactly the two shared axes, minted as fresh etic ids 0 and 1 in ascending emic order.
        assert_eq!(substrate.axes, vec![EticAxisId(0), EticAxisId(1)]);
        // Race 0 carries both shared axes: identity projections onto etic 0 (from emic 1) and
        // etic 1 (from emic 3); its private axis 5 is absent.
        let p0 = &projections[&RaceId(0)];
        assert_eq!(p0.per_axis.len(), 2);
        assert_eq!(
            p0.per_axis[&ValueAxisId(1)].onto,
            vec![(EticAxisId(0), Fixed::ONE)]
        );
        assert_eq!(
            p0.per_axis[&ValueAxisId(3)].onto,
            vec![(EticAxisId(1), Fixed::ONE)]
        );
        assert!(
            !p0.per_axis.contains_key(&ValueAxisId(5)),
            "the private axis is absent from the projection"
        );
        // Races 1 and 2 each carry only one shared axis; their private axes are absent.
        let p1 = &projections[&RaceId(1)];
        assert_eq!(p1.per_axis.len(), 1);
        assert_eq!(
            p1.per_axis[&ValueAxisId(1)].onto,
            vec![(EticAxisId(0), Fixed::ONE)]
        );
        let p2 = &projections[&RaceId(2)];
        assert_eq!(p2.per_axis.len(), 1);
        assert_eq!(
            p2.per_axis[&ValueAxisId(3)].onto,
            vec![(EticAxisId(1), Fixed::ONE)]
        );
        // Raising the recurrence minimum past the race count empties the substrate.
        let (empty, projs) = build_etic_substrate(&races, 4);
        assert!(empty.axes.is_empty());
        for p in projs.values() {
            assert!(p.per_axis.is_empty(), "no axis recurs four times");
        }
    }
}
