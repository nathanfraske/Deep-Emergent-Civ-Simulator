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

//! The experiential associative-learning substrate: a being correlates its own interoceptive harm
//! signal with the raw feature of the ground it stands on, and forms the belief "this feature harms
//! me" for itself (harm-learning arc slice b; Principles 8, 9, 11).
//!
//! This is the retirement of the injected `HAZARD` Observe. The old run authored the belief: it read a
//! high-level fact (a salinity dose over a reserved threshold) and pushed a pre-formed "a hazard is
//! present" observation into every being over that dose, so the being reasoned nothing. Here the being
//! reasons it. Each tick it feels whether a reserve fell beyond the ordinary metabolic-drain noise
//! (its OWN raw interoceptive signal, [`crate::homeostasis::is_harm_tick`] over the reserve delta), and
//! it senses the raw feature of the cell it stands on ([`crate::percept`]). It then contributes one
//! piece of evidence per present feature toward "this feature harms me" (a harm tick) or "this feature
//! is benign" (a harm-free tick), keyed on a per-feature belief subject.
//!
//! The associative learner IS the existing evidence engine ([`crate::evidence::InferenceFrame`] through
//! [`crate::agent::Mind::consider`]), keyed differently: the one global hazard subject becomes a
//! per-feature subject minted from the quantized feature the being senses, and the evidence weight is
//! the general [`crate::evidence::good_weight`] of two reserved likelihoods. Because the belief is then
//! an ordinary `(subject, attr)` frame, the shipped overhearing transmission carries it for free, and
//! the identical loop learns any ground-kind, any good place, any food that sickens, with zero
//! per-hazard code. Nothing reads a dose threshold, a hazard label, or a race id: the sign comes from
//! the being's own reserve falling, the subject from a raw quantized percept, and "this feature harms
//! me" emerges from the correlation over selection (Principles 8, 9).

use civsim_core::{Fixed, StableId, StateHasher};
use civsim_world::Coord3;
use std::collections::{BTreeMap, BTreeSet};

use crate::agent::Mind;
use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::evidence::{good_weight, AttrKindId, InferenceParams, ValueId};
use crate::homeostasis::AffordanceId;
use crate::locomotion::ResourceField;
use crate::material::MaterialField;
use crate::material_percept::MaterialPerceptRegistry;
use crate::percept::{feature_bucket, PerceptRegistry};

/// The generic attribute every experientially-learned feature belief is ABOUT: "does standing on this
/// feature harm me". One attribute for all features (the feature identity lives in the subject), a
/// reserved-high id disjoint from every other belief attribute so a harm belief never aliases another.
pub const HARM_ATTR: AttrKindId = AttrKindId(u32::MAX - 2);

/// The value meaning "this feature harms me": the belief a being forms when harm correlates with the
/// feature.
pub const HARMS: ValueId = 1;
/// The value meaning "this feature is benign": the competing hypothesis, reinforced on harm-free ticks
/// so the belief is defeasible for free (a feature that stops harming is un-learned).
pub const BENIGN: ValueId = 0;

/// The generic attribute every experientially-learned REWARD belief is ABOUT: "does the action a being took
/// in this context pay off" (ideation / experiential-discovery arc, piece 1, the appetitive learner). One
/// attribute for all sequences (the sequence identity lives in the subject), a reserved-high id disjoint
/// from [`HARM_ATTR`] (`u32::MAX - 2`) so a reward belief never aliases a harm belief on the same subject: a
/// being can hold both "this ground harms me" and "this action pays off" about the same percept key without
/// collision.
pub const REWARD_ATTR: AttrKindId = AttrKindId(u32::MAX - 3);

/// The value meaning "this action pays off": the belief a being forms when a felt reserve RISE correlates
/// with the action it took, the appetitive mirror of [`HARMS`].
pub const REWARDS: ValueId = 1;
/// The value meaning "this action is neutral": the competing hypothesis, reinforced on non-reward ticks so
/// the reward belief is defeasible for free (an action that stops paying off is un-learned), the mirror of
/// [`BENIGN`].
pub const NEUTRAL: ValueId = 0;

/// The value meaning a relational EDGE holds (relational-belief substrate, arc 2): "the head brings about the
/// tail", the yes of "A yields X" or "A causes B". RELATES is to a relation frame what [`REWARDS`] is to a
/// reward frame: a relation commits to it when the being's evidence that the edge holds clears the same
/// threshold a first-order belief does, so a relational belief is inferred by the SAME engine, never authored.
pub const RELATES: ValueId = 1;
/// The value meaning a relational edge does NOT hold: the competing hypothesis, reinforced when the head fails
/// to bring about the tail, so a relational belief is defeasible for free (the mirror of [`NEUTRAL`]).
pub const UNRELATED: ValueId = 0;

/// The first relational-belief KIND (relational-belief substrate, arc 2): the PRODUCTIVE / causal relation
/// "doing the head brings about the tail" (A yields X, A causes B), a reserved-high attribute disjoint from
/// [`HARM_ATTR`] (`u32::MAX - 2`), [`REWARD_ATTR`] (`u32::MAX - 3`), and every feature attribute, so a relation
/// never aliases a first-order belief on the same subject. One relation kind exists today; the store and the
/// multi-hop planner are GENERAL over the relation attribute (a being can hold relations of many kinds), and
/// the planner traverses ANY relation's RELATES edge as a reachability edge because every relation kind is
/// causal-productive so far. A later non-causal relation kind (a similarity, say) is default-off: it stays
/// absent from the reachability set ([`builtin_reachable_relations`]) until declared causal, so no
/// goal-to-action table is authored (Principle 9).
pub const YIELDS: AttrKindId = AttrKindId(u32::MAX - 4);

/// The relation KINDS the multi-hop planner may traverse as REACHABILITY edges: a kind in this set is
/// causal-productive, so an edge `(head, kind, tail)` the being holds RELATES reads as "do the head to bring
/// about the tail", and the planner is allowed to plan through it BACKWARD. A relation kind ABSENT from the
/// set is INERT to the planner (default-off): a being may hold and hash it, but the planner never treats it as
/// a means to an end. This is the data-defined hardening the planner needs so it authors no universal
/// means-ends reading over all relations (Principle 9): the planner's traversal is fixed Rust, the MEMBERSHIP
/// is data and grows with the world's relation vocabulary. Today the one built-in kind, [`YIELDS`], is
/// causal, so it is the sole member; a later non-causal kind (a similarity) is simply not added, and a later
/// data-defined relation registry supplies this set as a per-kind column rather than a code literal.
pub fn builtin_reachable_relations() -> BTreeSet<AttrKindId> {
    let mut kinds = BTreeSet::new();
    kinds.insert(YIELDS);
    kinds
}

/// The reserved base of the per-feature belief-subject band. A feature subject is minted at or above
/// this base, packed with the feature channel and its quantized bucket, so "salinity-bucket-2 ground
/// harms me" is a belief about a FEATURE-KIND and can never alias a belief about a being (being ids are
/// minted incrementally from zero and stay far below this base) or a reserved-high landmark id (the
/// packed subject stays below `1 << 63`). The band is the sibling of the conversational-cell place band
/// (`CELL_PLACE_BASE`): a stable function of the percept, folding into no stream on its own.
const FEATURE_SUBJECT_BASE: u64 = 1 << 62;
/// The bit position the feature channel is packed at, above the 32-bit bucket field.
const FEATURE_CHANNEL_SHIFT: u32 = 32;
/// The mask for the 32-bit bucket field.
const FEATURE_BUCKET_MASK: u64 = 0xFFFF_FFFF;

/// The channel base the MATERIAL-feature reward learner offsets its channels by (the lifetime/demography
/// keystone, pillar 2, trace slice C), so a material-feature belief subject never aliases a biology-feature
/// one even under the same belief attribute. Biology feature channels run from zero (harm keys them under
/// `HARM_ATTR`); material feature channels run from this base (the trace reward keys them under
/// `REWARD_ATTR`). The two are disjoint by attribute today, so this is a defensive future-proofing (the gate
/// asked for it): a later slice that keyed reward on biology features too would otherwise collide with the
/// material reward channels on the same subject. The high base leaves ample room below it for biology
/// channels (a handful) and above it for material ones (up to the `u16` ceiling).
pub const MATERIAL_FEATURE_CHANNEL_BASE: u16 = 1 << 15;

/// Mint the belief subject for a perceived feature: the `(channel, bucket)` pair packed into the
/// reserved feature-subject band. Two cells whose feature amount lands in the same bucket mint the SAME
/// subject, so a belief learned on one ground applies to another of the same perceived kind (the
/// generalisation the quantization buys); a different channel or a different bucket is a different
/// subject. The bucket is clamped non-negative (a feature amount is non-negative) and into the 32-bit
/// field, so the packing never overflows the band.
pub fn feature_subject(channel: u16, bucket: i64) -> StableId {
    let bucket = (bucket.max(0) as u64) & FEATURE_BUCKET_MASK;
    StableId(FEATURE_SUBJECT_BASE | ((channel as u64) << FEATURE_CHANNEL_SHIFT) | bucket)
}

/// The reserved base of the per-SEQUENCE belief-subject band (ideation / experiential-discovery arc, piece
/// 1, slice 1b): a discovered ACTION is a wildcard template over a primitive sequence, and this band mints
/// the belief subject it is keyed on, the sibling of [`feature_subject`]'s per-feature band. It sets bit 61
/// as well as bit 62, so a sequence subject is DISJOINT from every feature subject (whose payload stays in
/// the low 48 bits, leaving bit 61 clear) and from every being id, and stays below `1 << 63` (below the
/// reserved-high landmark ids), exactly as the feature band does. So two beings that execute the SAME
/// affordance-typed sequence mint the IDENTICAL subject, and a gossiped belief about it does not diverge.
const SEQUENCE_SUBJECT_BASE: u64 = (1 << 62) | (1 << 61);
/// The most sequence steps packed into one subject. A longer sequence truncates to its first steps (the
/// honest first-cut bound; a wider or hashed key is the refinement if discovered actions grow past this).
const SEQ_MAX_STEPS: usize = 4;
/// The bit width of each packed step field (the primitive id, the target-affordance bucket, the param
/// bucket). A field value wider than this clamps to the field maximum (the honest bound; the reserved
/// quantization keeps the buckets small, so the clamp does not bite in practice).
///
/// FLAGGED BOUND (deep audit, social-learning arc piece 3): four bits caps EACH field at 16 distinct
/// values, so the belief-subject packing distinguishes at most 16 PRIMITIVES, 16 target CHANNELS, and 16
/// target-VALUE buckets; anything above saturates to 15 and MERGES with its neighbours (an over-merge, not a
/// mask, since the clamp is applied identically at write and read). This is currently LATENT: the affordance
/// alphabet is 10 primitives (ids 0..9), the demonstrations perceive one or two channels, and hard-vs-soft
/// needs two value buckets, all well under 16. It WILL bite as the affordance set grows (arc 3, the made
/// world, adds tool-use and composition primitives), at which point distinct primitives above 15 would share
/// one reward belief even in the non-granular default. The refinement is the owner's call and is NOT
/// byte-neutral (widening the field or hashing changes every existing belief subject), so it is surfaced
/// here rather than changed: widen `SEQ_FIELD_BITS` (a `u32` step is already 12 bits, with room to grow the
/// primitive field to the affordance `u16`), or mint the subject by a collision-resistant hash of the full
/// step, before the primitive alphabet crosses 16.
const SEQ_FIELD_BITS: u32 = 4;
/// The bit width of one packed step (its three fields).
const SEQ_STEP_BITS: u32 = SEQ_FIELD_BITS * 3;
/// The mask for one packed field.
const SEQ_FIELD_MASK: u64 = (1 << SEQ_FIELD_BITS) - 1;

/// One step of an executed primitive sequence (ideation arc, piece 1, slice 1b): the PRIMITIVE the being
/// enacted (an affordance id), the quantized TARGET-AFFORDANCE bucket of the matter it acted on (the raw
/// derived affordance scalar bucketed like a feature, which slice 2a supplies), and the quantized action
/// PARAM bucket (a force or aim level bucketed the same way). All three are small quantized ids, so a step
/// is a wildcard predicate `primitive(target-kind, param-kind)` a template can match, never an object id or
/// a coded primitive pair (Principles 8, 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequenceStep {
    /// The affordance id of the primitive enacted (grasp, extract, ...).
    pub primitive: u16,
    /// The quantized target-affordance bucket (the kind of thing acted on).
    pub target_bucket: i64,
    /// The quantized action-parameter bucket (the kind of how: a force or aim level).
    pub param_bucket: i64,
}

/// Mint the belief subject for an executed primitive SEQUENCE (ideation arc, piece 1, slice 1b): a
/// canonical, RNG-free bit-pack of up to [`SEQ_MAX_STEPS`] steps into the reserved sequence-subject band,
/// the sibling of [`feature_subject`]. Two beings that execute the SAME sequence of primitives against the
/// SAME affordance-and-param kinds mint the IDENTICAL subject (so a belief that "grasp(sharp),
/// actuate(fracturable) pays off" generalises across matter of the same kind and gossips without
/// diverging); a different primitive, target kind, param kind, or ORDER is a different subject. Each field
/// is clamped into [`SEQ_FIELD_BITS`] and the sequence into [`SEQ_MAX_STEPS`] (the honest bounds), so the
/// pack never overflows the band, and the step COUNT is packed above the steps so a prefix is not confused
/// with the full sequence.
pub fn sequence_subject(steps: &[SequenceStep]) -> StableId {
    let n = steps.len().min(SEQ_MAX_STEPS);
    let mut payload: u64 = 0;
    for (i, step) in steps.iter().take(SEQ_MAX_STEPS).enumerate() {
        let primitive = (step.primitive as u64).min(SEQ_FIELD_MASK);
        let target = (step.target_bucket.max(0) as u64).min(SEQ_FIELD_MASK);
        let param = (step.param_bucket.max(0) as u64).min(SEQ_FIELD_MASK);
        let packed_step = primitive | (target << SEQ_FIELD_BITS) | (param << (SEQ_FIELD_BITS * 2));
        payload |= packed_step << (i as u32 * SEQ_STEP_BITS);
    }
    // The step count, packed above the four step fields (bits 48..), so a 2-step prefix of a 3-step
    // sequence mints a different subject than the 3-step sequence itself.
    payload |= (n as u64) << (SEQ_MAX_STEPS as u32 * SEQ_STEP_BITS);
    StableId(SEQUENCE_SUBJECT_BASE | payload)
}

/// The belief subject one executed step keys on, at the caller's GRANULARITY (social-learning arc, piece 3,
/// material granularity). The single point that decides how coarsely a discovered action generalises, so the
/// credit that WRITES the belief and every read that consumes it (the discovery weight, the appetitive
/// salience, the planner match, the forward model) agree by construction: a keying mismatch here is the
/// belief-masking class the belief-key consistency fix caught, so every site routes through this one function.
///
/// When `granular` is FALSE (the default, and every scenario today), the belief keys on the PRIMITIVE ALONE
/// (`target_bucket` and `param_bucket` zeroed), the generalising key that makes a learned primitive preferred
/// against every present target, the exact key the whole ideation loop uses now, so an opted-out run is
/// byte-identical. When TRUE, the belief keys on the FULL step, the primitive against the target's affordance
/// CHANNEL (`target_bucket`, the kind of thing) AND its quantized perceived VALUE (`param_bucket`, the
/// just-noticeable how-hard), so "strike a hard thing" and "strike a soft thing" diverge as distinct learned
/// actions and a technique specialises to the target it pays off on rather than one flat "the primitive pays
/// off." Pure, RNG-free; the channel and value already ride the candidate the discovery loop proposes.
pub fn step_belief_subject(step: &SequenceStep, granular: bool) -> StableId {
    if granular {
        sequence_subject(std::slice::from_ref(step))
    } else {
        sequence_subject(&[SequenceStep {
            primitive: step.primitive,
            target_bucket: 0,
            param_bucket: 0,
        }])
    }
}

/// The reserved calibrations of the associative learner (Principle 11): the numbers that set when a
/// reserve fall counts as harm, how coarse the feature percept is, and how strong a single correlation
/// observation is. The mechanism is fixed Rust; these are the owner's to set, surfaced with a basis,
/// never fabricated. They REPLACE the retired `hazard_dose_threshold` and `hazard_weight`, which
/// authored the belief the being now forms for itself.
#[derive(Clone, Copy, Debug)]
pub struct HarmLearningCalib {
    /// The harm-noise floor: a per-tick reserve fall no deeper than this is ordinary metabolic drain,
    /// not harm ([`crate::homeostasis::is_harm_tick`]). RESERVED. Basis: the largest per-tick reserve
    /// fall a resting, unharmed body incurs (the resting `base_drain` scaled to the tick), so only a
    /// supra-drain fall registers as harm.
    pub harm_noise_floor: Fixed,
    /// The feature granularity: the quantization step that buckets a raw feature amount into a perceived
    /// kind ([`crate::percept::feature_bucket`]). RESERVED. Basis: the sensorium's per-class
    /// just-noticeable difference for the sensed substance, coarse enough that ordinary spatial
    /// variation in a hazard reads as one or two kinds rather than a continuum.
    pub feature_granularity: Fixed,
    /// P(the harm signal fires | this feature harms me): the base rate of a felt reserve fall on a cell
    /// whose feature is truly harmful. RESERVED. Basis: the fraction of ticks a naive being on a harmful
    /// cell is worn faster than it heals, the dose-and-physics harm rate the floor implies, set as
    /// `trace.rs` sets an observation reliability.
    pub p_harm_given_harms: Fixed,
    /// P(the harm signal fires | this feature is benign): the base rate of a spurious felt fall on a
    /// benign cell. RESERVED. Basis: the false-attribution rate of a transient reserve dip unrelated to
    /// the feature, set as `trace.rs` sets a false-attribution rate; low, so a benign cell rarely earns
    /// harm evidence.
    pub p_harm_given_benign: Fixed,
    /// The certainty clamp the per-observation weight is bounded to. RESERVED. Basis: the evidence
    /// engine's log-odds clamp (`evidence.log_odds_clamp`), set equal to it so a single correlation
    /// observation cannot exceed the engine's maximum admissible certainty.
    pub certainty_clamp: Fixed,
}

impl HarmLearningCalib {
    /// Read the learner calibrations fail-loud from the manifest (Principle 11): a reserved value left
    /// unset refuses to build rather than running on a fabricated default. The certainty clamp reads the
    /// same `evidence.log_odds_clamp` the inference engine uses, so the two agree by construction.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<HarmLearningCalib, CalibrationError> {
        Ok(HarmLearningCalib {
            harm_noise_floor: m.require_fixed("harm.noise_floor")?,
            feature_granularity: m.require_fixed("harm.feature_granularity")?,
            p_harm_given_harms: m.require_fixed("harm.p_harm_given_harms")?,
            p_harm_given_benign: m.require_fixed("harm.p_harm_given_benign")?,
            certainty_clamp: m.require_fixed("evidence.log_odds_clamp")?,
        })
    }

    /// A labelled DEVELOPMENT FIXTURE standing up the same magnitudes the manifest would carry, for the
    /// test and harness paths that build a runner without a manifest. A noise floor of a hundredth (above
    /// the energy base drain, below the salt-flat condition harm), a unit granularity (a salt-flat dose
    /// range buckets into one or two kinds), likelihoods of nine-tenths and a tenth (so a single harm
    /// observation carries ln(9) of evidence and a naive being commits in a couple of harm ticks), and
    /// the engine's fifty-nat log-odds clamp.
    pub fn dev_default() -> HarmLearningCalib {
        HarmLearningCalib {
            harm_noise_floor: Fixed::from_ratio(1, 100),
            feature_granularity: Fixed::ONE,
            p_harm_given_harms: Fixed::from_ratio(9, 10),
            p_harm_given_benign: Fixed::from_ratio(1, 10),
            certainty_clamp: Fixed::from_int(50),
        }
    }

    /// The unsigned per-observation weight a single correlation carries: the I.J. Good weight of evidence
    /// of the two reserved likelihoods, `ln(P(harm|harms) / P(harm|benign))`, clamped to the certainty
    /// bound. General over the two probabilities, reading no kind, race, or feature (the feature identity
    /// is in the subject, not the weight).
    pub fn observation_weight(&self) -> Fixed {
        good_weight(
            self.p_harm_given_harms,
            self.p_harm_given_benign,
            self.certainty_clamp,
        )
    }
}

/// The reserved constants the APPETITIVE reward learner reads (ideation / experiential-discovery arc, piece
/// 1), the exact mirror of [`HarmLearningCalib`]: fail-loud from the manifest under a Calibrated run or a
/// labelled dev fixture in a test, every value reserved-with-basis (Principle 11). The mechanism is fixed
/// Rust; these numbers are the owner's. They are the reward complement of the harm calib, keyed on a felt
/// reserve RISE instead of a fall, and the certainty clamp reads the SAME `evidence.log_odds_clamp` the harm
/// learner and the inference engine use, so a single reward observation cannot exceed the engine ceiling.
#[derive(Clone, Copy, Debug)]
pub struct RewardLearningCalib {
    /// The reward-noise floor: a per-tick reserve rise no larger than this is ordinary metabolic recovery,
    /// not reward ([`crate::homeostasis::is_reward_tick`]). RESERVED. Basis: the largest per-tick reserve
    /// RISE a resting body's recovery incurs (the resting recovery rate scaled to the tick), the sign-mirror
    /// of the harm-noise floor, so only a supra-recovery rise registers as reward.
    pub reward_noise_floor: Fixed,
    /// The feature granularity: the quantization step that buckets a raw feature amount into a perceived kind
    /// ([`crate::percept::feature_bucket`]), the key a per-feature belief subject is minted from. RESERVED.
    /// Basis: the sensorium's per-class just-noticeable difference for the sensed feature, the same acuity
    /// the harm learner buckets on, so a reward belief generalises over the same feature kinds a harm belief
    /// does.
    pub feature_granularity: Fixed,
    /// P(the reward signal fires | this action pays off): the base rate of a felt reserve rise on a truly
    /// beneficial action. RESERVED. Basis: the fraction of ticks a naive being that took a beneficial action
    /// feels a supra-recovery rise, the reward mirror of `p_harm_given_harms`, set as the harm arc sets its
    /// likelihoods.
    pub p_reward_given_rewards: Fixed,
    /// P(the reward signal fires | this action is neutral): the base rate of a spurious felt rise on a
    /// neutral action. RESERVED. Basis: the false-attribution rate of a transient reserve rise unrelated to
    /// the action, the reward mirror of `p_harm_given_benign`; low, so a neutral action rarely earns reward
    /// evidence.
    pub p_reward_given_neutral: Fixed,
    /// The certainty clamp the per-observation weight is bounded to. RESERVED. Basis: the evidence engine's
    /// log-odds clamp (`evidence.log_odds_clamp`), set equal to it (and to the harm learner's) so a single
    /// correlation observation cannot exceed the engine's maximum admissible certainty.
    pub certainty_clamp: Fixed,
    /// The eligibility decay, the temporal-difference lambda the [`EligibilityTrace`] falls by each tick
    /// (slice 1b). RESERVED. Basis: the interoceptive lag between an action and its felt reserve rise the
    /// physiology implies, bounded by the retention window the belief system already forgets on, so credit
    /// reaches back only as far as the substrate remembers. In `(0, 1)`: nearer one credits a longer-lagged
    /// action, nearer zero credits only the immediately-preceding one. It enters through the existing
    /// observation `weight`, so it fabricates no new engine constant.
    pub eligibility_decay: Fixed,
}

impl RewardLearningCalib {
    /// Read the reward-learner calibrations fail-loud from the manifest (Principle 11): a reserved value left
    /// unset refuses to build rather than running on a fabricated default. The certainty clamp reads the same
    /// `evidence.log_odds_clamp` the inference engine and the harm learner use, so the three agree by
    /// construction.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<RewardLearningCalib, CalibrationError> {
        Ok(RewardLearningCalib {
            reward_noise_floor: m.require_fixed("reward.noise_floor")?,
            feature_granularity: m.require_fixed("reward.feature_granularity")?,
            p_reward_given_rewards: m.require_fixed("reward.p_reward_given_rewards")?,
            p_reward_given_neutral: m.require_fixed("reward.p_reward_given_neutral")?,
            certainty_clamp: m.require_fixed("evidence.log_odds_clamp")?,
            eligibility_decay: m.require_fixed("reward.eligibility_decay")?,
        })
    }

    /// A labelled DEVELOPMENT FIXTURE standing up the same magnitudes the manifest would carry, for the test
    /// and harness paths that build without a manifest, the reward mirror of [`HarmLearningCalib::dev_default`]:
    /// a noise floor of a hundredth (above the recovery base rate, below a real reserve gain), a unit
    /// granularity, likelihoods of nine-tenths and a tenth (so a single reward observation carries ln(9) of
    /// evidence and a naive being commits in a couple of reward ticks), and the engine's fifty-nat log-odds
    /// clamp. Not owner canon.
    pub fn dev_default() -> RewardLearningCalib {
        RewardLearningCalib {
            reward_noise_floor: Fixed::from_ratio(1, 100),
            feature_granularity: Fixed::ONE,
            p_reward_given_rewards: Fixed::from_ratio(9, 10),
            p_reward_given_neutral: Fixed::from_ratio(1, 10),
            certainty_clamp: Fixed::from_int(50),
            eligibility_decay: Fixed::from_ratio(1, 2),
        }
    }

    /// The unsigned per-observation weight a single reward correlation carries: the I.J. Good weight of
    /// evidence of the two reserved likelihoods, `ln(P(reward|rewards) / P(reward|neutral))`, clamped to the
    /// certainty bound. General over the two probabilities, reading no kind, race, or action (the action
    /// identity is in the subject, not the weight). The reward mirror of
    /// [`HarmLearningCalib::observation_weight`].
    pub fn observation_weight(&self) -> Fixed {
        good_weight(
            self.p_reward_given_rewards,
            self.p_reward_given_neutral,
            self.certainty_clamp,
        )
    }
}

/// The per-being ELIGIBILITY TRACE (ideation / experiential-discovery arc, piece 1, slice 1b): a short
/// memory of the primitive SEQUENCES a being recently executed, each with a recency-decayed eligibility in
/// `(0, 1]`, so a reserve rise felt some ticks after an action can still credit the sequence that produced
/// it (temporal-difference credit assignment). The head sequence (just executed) carries full eligibility;
/// each tick every trace decays by the reserved [`RewardLearningCalib::eligibility_decay`] (the TD lambda)
/// and a trace that underflows to zero is pruned, so the memory reaches back only as far as the lag allows.
///
/// This is new per-being DYNAMIC state, the sibling of [`crate::homeostasis::ReserveMemory`]: it folds into
/// `state_hash` in canonical (sequence-subject, eligibility) order, draws no randomness (a run stays
/// bit-identical across worker widths), and is EMPTY-BY-DEFAULT, so a being that has executed no sequence
/// folds nothing and a scenario that does not opt in replays bit-for-bit. Slice 1c populates it on the run
/// path and reads it to route delayed credit through the shipped `consider` path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EligibilityTrace {
    /// The recently-executed sequences keyed by their [`sequence_subject`], each with its current
    /// eligibility factor. Canonical `BTreeMap` order, so the fold and the credit walk are reproducible.
    traces: BTreeMap<StableId, Fixed>,
}

impl EligibilityTrace {
    /// An empty trace: no sequence remembered, so nothing folds into the hash until the first record.
    pub fn new() -> EligibilityTrace {
        EligibilityTrace::default()
    }

    /// Whether no sequence is remembered (an empty trace folds nothing into the hash, the opt-out state).
    pub fn is_empty(&self) -> bool {
        self.traces.is_empty()
    }

    /// Record a just-executed sequence at FULL eligibility (one), the head of the trace: it earns full
    /// credit for a reserve rise felt this tick and decays from there. Re-executing a sequence refreshes it
    /// to full.
    pub fn record(&mut self, subject: StableId) {
        self.traces.insert(subject, Fixed::ONE);
    }

    /// Decay every trace by the eligibility lambda and prune those that underflow to zero, so a sequence's
    /// eligibility for delayed credit falls with the ticks since it was executed. With a lambda in `(0, 1)`
    /// each trace shrinks and eventually leaves the memory, keeping it bounded and empty-neutral. A pure
    /// deterministic fold in canonical key order.
    pub fn decay(&mut self, lambda: Fixed) {
        self.traces.retain(|_, e| {
            *e = e.checked_mul(lambda).unwrap_or(Fixed::ZERO);
            *e > Fixed::ZERO
        });
    }

    /// The current eligibility of a sequence (how much delayed credit it still earns), zero if it was not
    /// recently executed. Slice 1c scales the reward observation's weight by this.
    pub fn eligibility(&self, subject: StableId) -> Fixed {
        self.traces.get(&subject).copied().unwrap_or(Fixed::ZERO)
    }

    /// The remembered sequences with their eligibilities, in canonical order (the credit walk slice 1c runs).
    pub fn entries(&self) -> impl Iterator<Item = (&StableId, &Fixed)> {
        self.traces.iter()
    }

    /// Fold the trace into a hash in canonical (sequence-subject, eligibility) order, beside the reserve
    /// memory. An empty trace folds nothing, so an opted-out run is byte-identical. The `BTreeMap` walks in
    /// canonical key order, so the fold is reproducible and thread-invariant.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (subject, eligibility) in &self.traces {
            h.write_u64(subject.0);
            h.write_fixed(*eligibility);
        }
    }
}

/// One piece of evidence a being contributes this tick: the per-feature subject to key it on, the value
/// it points toward (`HARMS` on a harm tick, `BENIGN` otherwise), and the signed weight. Fed straight
/// into [`crate::agent::Mind::consider`] (which scales it by the mind's acuity and accumulates it into
/// the `(subject, HARM_ATTR)` frame), so the belief commits at read past the engine's threshold and
/// margin with no learner-specific commit logic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FeatureObservation {
    /// The per-feature belief subject ([`feature_subject`]).
    pub subject: StableId,
    /// The value this observation points toward (`HARMS` or `BENIGN`).
    pub toward: ValueId,
    /// The signed evidence weight (the observation weight, scaled by the being's plasticity).
    pub weight: Fixed,
}

/// The observations a being makes this tick: one per PRESENT feature of the cell it stands on
/// (a channel whose amount is positive), toward `HARMS` if it felt harm this tick and `BENIGN`
/// otherwise, each weighted by the reserved observation weight scaled by the being's belief plasticity
/// (its heritable learning rate, `Mind::plasticity`, neutral at one). A cell with no present feature
/// yields nothing (there is nothing to correlate). The harm bit is the being's OWN interoceptive signal
/// (whether any reserve fell beyond the noise floor); the feature is a raw percept; nothing reads a dose
/// threshold, a hazard label, or a race id, so the belief emerges from correlation (Principles 8, 9).
pub fn feature_observations(
    harm: bool,
    features: &[Fixed],
    plasticity: Fixed,
    calib: &HarmLearningCalib,
) -> Vec<FeatureObservation> {
    let base = calib.observation_weight();
    let weight = base.checked_mul(plasticity).unwrap_or(base);
    let toward = if harm { HARMS } else { BENIGN };
    features
        .iter()
        .enumerate()
        .filter(|(_, &amount)| amount > Fixed::ZERO)
        .map(|(channel, &amount)| {
            let bucket = feature_bucket(amount, calib.feature_granularity);
            FeatureObservation {
                subject: feature_subject(channel as u16, bucket),
                toward,
                weight,
            }
        })
        .collect()
}

/// The REWARD observations a being makes this tick (ideation / experiential-discovery arc, piece 1, slice 1a
/// in its degenerate single-tick form): one per PRESENT feature of the cell it stands on (a channel whose
/// amount is positive), toward `REWARDS` if it felt a supra-recovery reserve RISE this tick and `NEUTRAL`
/// otherwise, each weighted by the reserved observation weight scaled by the being's belief plasticity (its
/// heritable learning rate, `Mind::plasticity`, neutral at one). A near-verbatim clone of
/// [`feature_observations`] with the sign flipped: the reward bit is the being's OWN interoceptive signal
/// (whether any reserve ROSE beyond the noise floor, [`crate::homeostasis::is_reward_tick`]), the feature is
/// the same raw percept, and the produced [`FeatureObservation`]s are fed into the `(subject, REWARD_ATTR)`
/// frame (disjoint from the harm frame on the same subject), so the reward belief emerges from correlation
/// with nothing read but the raw feature and the felt sign (Principles 8, 9). The `channel_base` offsets the
/// feature channel into a disjoint band ([`MATERIAL_FEATURE_CHANNEL_BASE`] for the physical-trace material
/// percept, the lifetime/demography keystone pillar 2), so a material-feature reward subject never aliases a
/// biology-feature one; pass zero to key channels from the base. The run path uses this to re-earn "eating
/// where this residue lies pays off" from the material composition underfoot, feature-keyed exactly as the
/// harm learner keys felt harm to a ground feature.
pub fn reward_observations(
    reward: bool,
    features: &[Fixed],
    plasticity: Fixed,
    calib: &RewardLearningCalib,
    channel_base: u16,
) -> Vec<FeatureObservation> {
    let base = calib.observation_weight();
    let weight = base.checked_mul(plasticity).unwrap_or(base);
    let toward = if reward { REWARDS } else { NEUTRAL };
    features
        .iter()
        .enumerate()
        .filter(|(_, &amount)| amount > Fixed::ZERO)
        .map(|(channel, &amount)| {
            let bucket = feature_bucket(amount, calib.feature_granularity);
            FeatureObservation {
                subject: feature_subject(channel_base + channel as u16, bucket),
                toward,
                weight,
            }
        })
        .collect()
}

/// The belief-derived expected-harm AVOIDANCE gradient for a being at `here` (harm-learning arc slice
/// c): the summed inverse-distance repulsion away from every cell within `sense_range` whose feature-kind
/// the being holds a committed HARMS belief about. Each believed-harmful cell contributes a vector
/// pointing from the cell toward the being, weighted by inverse distance (nearer harmful ground repels
/// harder), so the raw sum points away from the bulk of believed harm; a being that believes nothing
/// nearby harms it gets a zero gradient. The caller normalises the raw sum to a unit percept, exactly as
/// the temperature gradient is normalised.
///
/// This is a PERCEPT, not a heading. The runner feeds it into the reserve axis's direction slot the
/// evolved controller reads, and ONLY a heritable weight lifted off founding-zero by selection turns it
/// into avoidance, so the flight behaviour emerges rather than being authored (Principle 9): the
/// mechanism never subtracts a harm term from the heading itself. It reads the being's own beliefs and
/// the raw features it senses, never a hazard label or a race id (Principle 8). Pure and RNG-free.
pub fn avoidance_gradient(
    mind: &Mind,
    here: Coord3,
    resources: &ResourceField,
    percepts: &PerceptRegistry,
    sense_range: i64,
    granularity: Fixed,
    params: &InferenceParams,
) -> (Fixed, Fixed) {
    if percepts.is_empty() {
        return (Fixed::ZERO, Fixed::ZERO);
    }
    let mut ax = Fixed::ZERO;
    let mut ay = Fixed::ZERO;
    let r = sense_range.max(0) as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx == 0 && dy == 0 {
                continue;
            }
            let cell = Coord3::ground(here.x + dx, here.y + dy);
            let features = percepts.perceive(resources.composition(cell));
            let believes_harm = features.iter().enumerate().any(|(channel, &amount)| {
                amount > Fixed::ZERO && {
                    let subject =
                        feature_subject(channel as u16, feature_bucket(amount, granularity));
                    mind.belief(subject, HARM_ATTR, params) == Some(HARMS)
                }
            });
            if believes_harm {
                // A vector from the harmful cell toward the being (away from the harm), weighted by
                // inverse distance: (-dx, -dy) / (dx^2 + dy^2). No square root; nearer harm repels harder.
                let d2 = Fixed::from_int(dx * dx + dy * dy);
                if let (Some(cx), Some(cy)) = (
                    Fixed::from_int(-dx).checked_div(d2),
                    Fixed::from_int(-dy).checked_div(d2),
                ) {
                    ax = ax.saturating_add(cx);
                    ay = ay.saturating_add(cy);
                }
            }
        }
    }
    (ax, ay)
}

/// The belief-derived expected-reward ATTRACTION gradient for a being at `here` (the lifetime/demography
/// keystone, pillar 2, physical-trace persistence, trace slice C3): the positive mirror of
/// [`avoidance_gradient`], the summed inverse-distance attraction TOWARD every cell within `sense_range`
/// whose MATERIAL signature the being holds a committed REWARDS belief about. Each believed-rewarding cell
/// contributes a vector pointing from the being toward the cell, weighted by inverse distance (a nearer
/// rewarding place pulls harder), so the raw sum points toward the bulk of believed reward; a being that
/// believes nothing nearby pays off gets a zero gradient. The caller normalises the raw sum to a unit
/// percept, exactly as [`avoidance_gradient`] and the temperature gradient are normalised. The material
/// feature subject is reconstructed at `channel_base` ([`MATERIAL_FEATURE_CHANNEL_BASE`]), the same offset
/// the trace reward learner committed it under, so this reads the belief that learner formed.
///
/// This is a PERCEPT, not a heading, the exact behavioural mirror of avoidance: the runner feeds it into a
/// direction slot the evolved controller reads, and ONLY a heritable weight lifted off founder-zero by
/// selection turns it into approach, so seeking the trace-marked place emerges rather than being authored
/// (Principle 9): the mechanism never adds a reward term to the heading itself. It reads the being's own
/// reward beliefs and the raw material it senses, never a label or a race id (Principle 8). Pure and
/// RNG-free. The physical trace only BIASES the being toward the place; the being's own felt reward stays
/// the sole gate to a committed belief.
#[allow(clippy::too_many_arguments)]
pub fn attraction_gradient(
    mind: &Mind,
    here: Coord3,
    material: &MaterialField,
    material_percepts: &MaterialPerceptRegistry,
    sense_range: i64,
    granularity: Fixed,
    channel_base: u16,
    params: &InferenceParams,
) -> (Fixed, Fixed) {
    if material_percepts.is_empty() {
        return (Fixed::ZERO, Fixed::ZERO);
    }
    let mut ax = Fixed::ZERO;
    let mut ay = Fixed::ZERO;
    let r = sense_range.max(0) as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx == 0 && dy == 0 {
                continue;
            }
            let cell = Coord3::ground(here.x + dx, here.y + dy);
            let features = material_percepts.perceive(material.cell(cell));
            let believes_reward = features.iter().enumerate().any(|(channel, &amount)| {
                amount > Fixed::ZERO && {
                    let subject = feature_subject(
                        channel_base + channel as u16,
                        feature_bucket(amount, granularity),
                    );
                    mind.belief(subject, REWARD_ATTR, params) == Some(REWARDS)
                }
            });
            if believes_reward {
                // A vector from the being TOWARD the rewarding cell (the sign flip of avoidance's repulsion):
                // (dx, dy) / (dx^2 + dy^2). No square root; a nearer rewarding place pulls harder.
                let d2 = Fixed::from_int(dx * dx + dy * dy);
                if let (Some(cx), Some(cy)) = (
                    Fixed::from_int(dx).checked_div(d2),
                    Fixed::from_int(dy).checked_div(d2),
                ) {
                    ax = ax.saturating_add(cx);
                    ay = ay.saturating_add(cy);
                }
            }
        }
    }
    (ax, ay)
}

/// The belief-derived APPETITIVE salience over a being's affordances (ideation / experiential-discovery
/// arc, piece 1, the belief-to-behaviour feedback, the appetitive mirror of [`avoidance_gradient`]): for
/// each affordance in the caller's canonical order, ONE when the being holds a committed REWARDS belief
/// about that affordance's single-primitive sequence, ZERO otherwise. It is the exact mirror of the
/// avoidance gradient's committed-belief test (`mind.belief(...) == Some(HARMS)`), read per AFFORDANCE
/// rather than per CELL, because the reward belief is keyed on the ACTION the being took rather than the
/// ground it stands on, so there is no spatial distance to weight by: a believed-rewarding action reads a
/// full unit signal, every other reads zero.
///
/// This is a PERCEPT, not a decision. The runner writes each channel into the controller's appetitive
/// input block (canonical affordance order), and ONLY a heritable weight lifted off founder-zero by
/// selection turns "I believe this action pays off" into "issue it again", so REPETITION emerges rather
/// than being authored (Principle 9): the mechanism never adds a reward term to an affordance's activation
/// directly, and the afforded-set gate in [`crate::controller::ControllerLayout::decide`] still bounds
/// which action can win, so a believed-rewarding action the body cannot currently perform is never forced.
/// Reads only the being's own reward beliefs and the affordance ids, never an affordance's authored valence
/// or a race id (Principle 8). Pure and RNG-free; the belief key it reads matches exactly what the slice-1c
/// credit pass commits at the same grain, so the belief this reads is the belief that pass formed.
///
/// SOCIAL-LEARNING ARC, PIECE 3 (material granularity), THE DEEP RECONCILIATION SPOT (flagged): this read
/// was TARGET-BLIND (one belief per primitive), at odds with a value-keyed belief. It is now made
/// CANDIDATE-AWARE so it agrees with the granular credit at the same grain. When `granular` is FALSE (the
/// default), it keys the primitive-only subject exactly as before, so `candidates` is unused and the salience
/// is byte-identical. When TRUE, an affordance lights when the being holds a committed REWARDS belief about
/// ANY currently-PRESENT candidate of that affordance (a target of the kind and value it can act on now,
/// keyed at the target's grain through [`step_belief_subject`]), so the appetitive drive fires for "I believe
/// a present target of this affordance pays off" rather than a target-blind "this primitive pays off." The
/// output stays per-affordance in the caller's canonical order (the controller's appetitive input block), and
/// the afforded-set gate in [`crate::controller::ControllerLayout::decide`] still bounds which action can win.
pub fn appetitive_salience(
    mind: &Mind,
    affordances: &[AffordanceId],
    candidates: &[SequenceStep],
    granular: bool,
    params: &InferenceParams,
) -> Vec<Fixed> {
    affordances
        .iter()
        .map(|a| {
            let believed = if granular {
                // Any PRESENT candidate of this affordance the being believes pays off, keyed at the target's
                // grain, so the belief this reads is exactly the one the granular credit commits.
                candidates.iter().filter(|c| c.primitive == a.0).any(|c| {
                    mind.belief(step_belief_subject(c, true), REWARD_ATTR, params) == Some(REWARDS)
                })
            } else {
                // The primitive-only key, byte-identical to the pre-granularity read.
                let step = SequenceStep {
                    primitive: a.0,
                    target_bucket: 0,
                    param_bucket: 0,
                };
                mind.belief(step_belief_subject(&step, false), REWARD_ATTR, params) == Some(REWARDS)
            };
            if believed {
                Fixed::ONE
            } else {
                Fixed::ZERO
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::homeostasis::{HomeostaticRegistry, CONDITION};
    use crate::physiology::SALINITY;

    fn params() -> InferenceParams {
        // The evidence engine's dev thresholds: clamp 50, commit at 3 nats, margin 1.
        InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        }
    }

    #[test]
    fn a_feature_subject_is_disjoint_from_beings_and_landmarks_and_keys_by_channel_and_bucket() {
        let s = feature_subject(0, 2);
        // Above every being id (minted from zero) and below the reserved-high landmark ids.
        assert!(s.0 >= FEATURE_SUBJECT_BASE);
        assert!(s.0 < u64::MAX - 1);
        // Same channel and bucket mint the same subject (the generalisation); a different bucket or
        // channel is a different subject.
        assert_eq!(feature_subject(0, 2), s);
        assert_ne!(feature_subject(0, 3), s);
        assert_ne!(feature_subject(1, 2), s);
        // A negative bucket clamps to zero rather than wrapping the packing.
        assert_eq!(feature_subject(0, -5), feature_subject(0, 0));
    }

    #[test]
    fn only_present_features_earn_an_observation_pointed_by_the_harm_bit() {
        let calib = HarmLearningCalib::dev_default();
        // Two channels: the first absent (zero), the second present (a salt dose of two).
        let features = vec![Fixed::ZERO, Fixed::from_int(2)];
        let harm = feature_observations(true, &features, Fixed::ONE, &calib);
        assert_eq!(
            harm.len(),
            1,
            "only the present feature earns an observation"
        );
        assert_eq!(harm[0].toward, HARMS, "a harm tick points toward HARMS");
        assert_eq!(
            harm[0].subject,
            feature_subject(
                1,
                feature_bucket(Fixed::from_int(2), calib.feature_granularity)
            )
        );
        assert!(
            harm[0].weight > Fixed::ZERO,
            "the observation carries positive evidence"
        );
        // A harm-free tick on the same cell points the same subject toward BENIGN.
        let benign = feature_observations(false, &features, Fixed::ONE, &calib);
        assert_eq!(benign[0].toward, BENIGN);
        assert_eq!(benign[0].subject, harm[0].subject);
        // A cell with no present feature yields nothing to correlate.
        assert!(feature_observations(true, &[Fixed::ZERO], Fixed::ONE, &calib).is_empty());
    }

    #[test]
    fn a_being_repeatedly_harmed_on_a_feature_commits_harms_on_its_own() {
        // The milestone core: a mind fed repeated harm-while-on-the-feature commits a HARMS belief about
        // that feature-kind, with no injected observation, purely from the correlation.
        let calib = HarmLearningCalib::dev_default();
        let features = vec![Fixed::from_int(2)]; // one present feature (the salt dose)
        let subject = feature_subject(
            0,
            feature_bucket(Fixed::from_int(2), calib.feature_granularity),
        );
        let mut mind = Mind::new(StableId(1), Fixed::ONE); // neutral acuity

        // Before any experience, no belief.
        assert_eq!(mind.belief(subject, HARM_ATTR, &params()), None);
        // Feel harm on the feature for three ticks.
        for _ in 0..3 {
            for obs in feature_observations(true, &features, Fixed::ONE, &calib) {
                mind.consider(
                    obs.subject,
                    HARM_ATTR,
                    [HARMS, BENIGN],
                    obs.toward,
                    obs.weight,
                    mind.id,
                );
            }
        }
        assert_eq!(
            mind.belief(subject, HARM_ATTR, &params()),
            Some(HARMS),
            "the being forms the HARMS belief about the feature from its own repeated harm"
        );
    }

    #[test]
    fn only_present_features_earn_a_reward_observation_pointed_by_the_reward_bit() {
        // The reward mirror of the harm observation: only a present feature earns an observation, a reward
        // tick points it toward REWARDS and a non-reward tick toward NEUTRAL, on the same subject key.
        let calib = RewardLearningCalib::dev_default();
        let features = vec![Fixed::ZERO, Fixed::from_int(2)];
        let reward = reward_observations(true, &features, Fixed::ONE, &calib, 0);
        assert_eq!(
            reward.len(),
            1,
            "only the present feature earns an observation"
        );
        assert_eq!(
            reward[0].toward, REWARDS,
            "a reward tick points toward REWARDS"
        );
        assert_eq!(
            reward[0].subject,
            feature_subject(
                1,
                feature_bucket(Fixed::from_int(2), calib.feature_granularity)
            )
        );
        assert!(
            reward[0].weight > Fixed::ZERO,
            "the observation carries positive evidence"
        );
        // A non-reward tick on the same cell points the same subject toward NEUTRAL.
        let neutral = reward_observations(false, &features, Fixed::ONE, &calib, 0);
        assert_eq!(neutral[0].toward, NEUTRAL);
        assert_eq!(neutral[0].subject, reward[0].subject);
        // A cell with no present feature yields nothing to correlate.
        assert!(reward_observations(true, &[Fixed::ZERO], Fixed::ONE, &calib, 0).is_empty());
    }

    #[test]
    fn a_channel_base_offsets_the_material_reward_subject_disjoint_from_a_biology_feature() {
        // The lifetime/demography keystone, pillar 2, trace slice C, sub-seam 1 (the gate's disjoint-channel
        // requirement): a material-feature reward observation offsets its channel by the channel base, so its
        // subject can never alias a biology-feature subject on the same channel index even under the same
        // belief attribute. The same present feature at base zero and at the material base mint DIFFERENT
        // subjects; the material one equals the feature subject at the offset channel.
        let calib = RewardLearningCalib::dev_default();
        let features = vec![Fixed::from_int(2)];
        let at_zero = reward_observations(true, &features, Fixed::ONE, &calib, 0);
        let at_material = reward_observations(
            true,
            &features,
            Fixed::ONE,
            &calib,
            MATERIAL_FEATURE_CHANNEL_BASE,
        );
        assert_ne!(
            at_zero[0].subject, at_material[0].subject,
            "the material reward subject is disjoint from the base-zero (biology) subject"
        );
        let bucket = feature_bucket(Fixed::from_int(2), calib.feature_granularity);
        assert_eq!(at_zero[0].subject, feature_subject(0, bucket));
        assert_eq!(
            at_material[0].subject,
            feature_subject(MATERIAL_FEATURE_CHANNEL_BASE, bucket),
            "the material channel is offset by the base"
        );
    }

    #[test]
    fn a_being_repeatedly_rewarded_on_a_feature_commits_rewards_on_its_own() {
        // The appetitive milestone core (slice 1a single-tick form): a mind fed repeated reward-while-on-the-
        // feature commits a REWARDS belief about that feature-kind, purely from correlation, the exact mirror
        // of the harm learner forming HARMS.
        let calib = RewardLearningCalib::dev_default();
        let features = vec![Fixed::from_int(2)];
        let subject = feature_subject(
            0,
            feature_bucket(Fixed::from_int(2), calib.feature_granularity),
        );
        let mut mind = Mind::new(StableId(1), Fixed::ONE);
        assert_eq!(mind.belief(subject, REWARD_ATTR, &params()), None);
        for _ in 0..3 {
            for obs in reward_observations(true, &features, Fixed::ONE, &calib, 0) {
                mind.consider(
                    obs.subject,
                    REWARD_ATTR,
                    [REWARDS, NEUTRAL],
                    obs.toward,
                    obs.weight,
                    mind.id,
                );
            }
        }
        assert_eq!(
            mind.belief(subject, REWARD_ATTR, &params()),
            Some(REWARDS),
            "the being forms the REWARDS belief about the feature from its own repeated reward"
        );
    }

    #[test]
    fn a_reward_belief_and_a_harm_belief_coexist_on_one_subject_via_disjoint_attrs() {
        // The disjointness guarantee: REWARD_ATTR (u32::MAX - 3) and HARM_ATTR (u32::MAX - 2) never alias, so
        // a being can hold "this action pays off" and "this ground harms me" about the SAME feature subject
        // at once without collision. The reward and harm frames are independent (the appetitive and aversive
        // halves of one interoceptive signal split at zero).
        assert_ne!(REWARD_ATTR, HARM_ATTR);
        let rcalib = RewardLearningCalib::dev_default();
        let hcalib = HarmLearningCalib::dev_default();
        let features = vec![Fixed::from_int(2)];
        let subject = feature_subject(
            0,
            feature_bucket(Fixed::from_int(2), rcalib.feature_granularity),
        );
        let mut mind = Mind::new(StableId(3), Fixed::ONE);
        for _ in 0..3 {
            for obs in reward_observations(true, &features, Fixed::ONE, &rcalib, 0) {
                mind.consider(
                    obs.subject,
                    REWARD_ATTR,
                    [REWARDS, NEUTRAL],
                    obs.toward,
                    obs.weight,
                    mind.id,
                );
            }
            for obs in feature_observations(true, &features, Fixed::ONE, &hcalib) {
                mind.consider(
                    obs.subject,
                    HARM_ATTR,
                    [HARMS, BENIGN],
                    obs.toward,
                    obs.weight,
                    mind.id,
                );
            }
        }
        assert_eq!(
            mind.belief(subject, REWARD_ATTR, &params()),
            Some(REWARDS),
            "the reward belief commits on its own attr"
        );
        assert_eq!(
            mind.belief(subject, HARM_ATTR, &params()),
            Some(HARMS),
            "the harm belief commits on its own attr, on the same subject, without collision"
        );
    }

    #[test]
    fn a_sequence_subject_is_canonical_and_disjoint_from_feature_subjects_and_beings() {
        // Slice 1b: a primitive-sequence belief subject is a canonical function of the executed steps, minted
        // in a band disjoint from the feature band and from being ids, below the reserved-high landmarks.
        let step = |p: u16, t: i64, q: i64| SequenceStep {
            primitive: p,
            target_bucket: t,
            param_bucket: q,
        };
        let seq = [step(3, 1, 0), step(4, 2, 1)]; // grasp(kind 1), extract(kind 2, param 1)
        let s = sequence_subject(&seq);
        // Above every being id (minted from zero) and below the reserved-high landmark ids.
        assert!(s.0 >= SEQUENCE_SUBJECT_BASE);
        assert!(s.0 < u64::MAX - 1);
        // Canonical: the same steps mint the same subject; a different primitive, target, param, or ORDER is
        // a different subject.
        assert_eq!(sequence_subject(&seq), s);
        assert_ne!(sequence_subject(&[step(3, 1, 0), step(5, 2, 1)]), s); // different primitive
        assert_ne!(sequence_subject(&[step(3, 9, 0), step(4, 2, 1)]), s); // different target kind
        assert_ne!(sequence_subject(&[step(4, 2, 1), step(3, 1, 0)]), s); // reversed order
        assert_ne!(sequence_subject(&[step(3, 1, 0)]), s); // a prefix is not the whole sequence
                                                           // Disjoint from the feature band: a sequence subject (bit 61 set) never equals a feature subject
                                                           // (bit 61 clear), whatever the low bits, so a reward belief about an ACTION never aliases a belief
                                                           // about a standing-on FEATURE.
        assert_eq!(
            SEQUENCE_SUBJECT_BASE & FEATURE_SUBJECT_BASE,
            FEATURE_SUBJECT_BASE
        );
        assert_ne!(sequence_subject(&seq).0, feature_subject(0, 0).0);
        for ch in 0..4u16 {
            for bk in 0..8i64 {
                assert_ne!(s.0, feature_subject(ch, bk).0);
            }
        }
    }

    #[test]
    fn appetitive_salience_lights_only_the_affordance_the_being_believes_pays_off() {
        // Slice 1d (READ): the belief-to-behaviour feedback PERCEPT, the appetitive mirror of the avoidance
        // gradient. A being that has committed the REWARDS belief about ONE affordance's sequence reads a unit
        // appetitive signal on that affordance's channel and zero on the others; a being that believes nothing
        // reads all zeros, so a founder's percept is inert until a belief forms and an evolved weight lifts it.
        let ingest = AffordanceId(1);
        let grasp = AffordanceId(3);
        let extract = AffordanceId(4);
        let affordances = [ingest, grasp, extract];

        let calib = RewardLearningCalib::dev_default();
        let ingest_subject = sequence_subject(&[SequenceStep {
            primitive: ingest.0,
            target_bucket: 0,
            param_bucket: 0,
        }]);
        let mut mind = Mind::new(StableId(7), Fixed::ONE);
        // A being that has learned nothing reads a flat-zero appetitive percept (no signal to act on).
        assert_eq!(
            appetitive_salience(&mind, &affordances, &[], false, &params()),
            vec![Fixed::ZERO; 3],
            "a being with no reward belief reads no appetitive signal on any affordance"
        );
        // Teach it that its INGEST pays off (the belief slice 1c's credit pass forms).
        for _ in 0..3 {
            mind.consider(
                ingest_subject,
                REWARD_ATTR,
                [REWARDS, NEUTRAL],
                REWARDS,
                calib.observation_weight(),
                mind.id,
            );
        }
        assert_eq!(
            mind.belief(ingest_subject, REWARD_ATTR, &params()),
            Some(REWARDS),
            "the being has committed the ingest-pays-off belief"
        );
        // The appetitive percept now lights ONLY the ingest channel, in the caller's canonical order, and the
        // channels for the actions it holds no belief about stay dark.
        assert_eq!(
            appetitive_salience(&mind, &affordances, &[], false, &params()),
            vec![Fixed::ONE, Fixed::ZERO, Fixed::ZERO],
            "the appetitive percept lights only the affordance the being believes pays off"
        );
        // The salience aligns to the caller's affordance order, not a fixed index: reorder the inputs and the
        // lit channel moves with ingest.
        assert_eq!(
            appetitive_salience(&mind, &[grasp, ingest], &[], false, &params()),
            vec![Fixed::ZERO, Fixed::ONE],
            "the salience aligns to the caller's canonical affordance order"
        );
    }

    #[test]
    fn the_eligibility_trace_records_decays_prunes_and_folds_empty_neutral() {
        // Slice 1b: the eligibility trace remembers a just-executed sequence at full eligibility, decays it by
        // the TD lambda each tick, prunes it when it underflows, and folds empty-neutral (opt-in).
        let seq = [SequenceStep {
            primitive: 3,
            target_bucket: 1,
            param_bucket: 0,
        }];
        let subject = sequence_subject(&seq);
        let lambda = Fixed::from_ratio(1, 2);

        let mut trace = EligibilityTrace::new();
        assert!(trace.is_empty(), "a fresh trace remembers nothing");
        assert_eq!(
            trace.eligibility(subject),
            Fixed::ZERO,
            "an unrecorded sequence earns no delayed credit"
        );

        // Recording puts the sequence at full eligibility, the head of the trace.
        trace.record(subject);
        assert_eq!(trace.eligibility(subject), Fixed::ONE);
        assert!(!trace.is_empty());

        // Each decay halves the eligibility (the TD lambda), so a later-felt reward credits it less.
        trace.decay(lambda);
        assert_eq!(trace.eligibility(subject), Fixed::from_ratio(1, 2));
        trace.decay(lambda);
        assert_eq!(trace.eligibility(subject), Fixed::from_ratio(1, 4));

        // Enough decays underflow it to zero and prune it, so the memory reaches back only as far as the lag
        // allows and returns to empty (byte-neutral again).
        for _ in 0..64 {
            trace.decay(lambda);
        }
        assert!(
            trace.is_empty(),
            "a long-past sequence is pruned and the trace returns to empty"
        );

        // The fold is empty-neutral (an empty trace folds nothing: it leaves the hash identical to one no
        // trace ever touched) and canonical (independent of record order).
        let empty = EligibilityTrace::new();
        let mut h_empty = StateHasher::new();
        empty.hash_into(&mut h_empty);
        assert_eq!(
            h_empty.finish(),
            StateHasher::new().finish(),
            "an empty trace folds nothing"
        );

        let sa = sequence_subject(&[SequenceStep {
            primitive: 1,
            target_bucket: 0,
            param_bucket: 0,
        }]);
        let sb = sequence_subject(&[SequenceStep {
            primitive: 2,
            target_bucket: 0,
            param_bucket: 0,
        }]);
        let mut ta = EligibilityTrace::new();
        ta.record(sa);
        ta.record(sb);
        let mut tb = EligibilityTrace::new();
        tb.record(sb);
        ta.record(sa); // re-record is idempotent to full; order differs from tb
        tb.record(sa);
        let mut ha = StateHasher::new();
        ta.hash_into(&mut ha);
        let mut hb = StateHasher::new();
        tb.hash_into(&mut hb);
        assert_eq!(
            ha.finish(),
            hb.finish(),
            "the canonical BTreeMap fold is independent of record order"
        );
    }

    #[test]
    fn a_being_never_on_the_feature_never_forms_the_belief() {
        let calib = HarmLearningCalib::dev_default();
        let salt_subject = feature_subject(
            0,
            feature_bucket(Fixed::from_int(2), calib.feature_granularity),
        );
        let mut mind = Mind::new(StableId(2), Fixed::ONE);
        // The being only ever stands on plain ground (no present feature), and is harmed by nothing.
        for _ in 0..10 {
            for obs in feature_observations(false, &[Fixed::ZERO], Fixed::ONE, &calib) {
                mind.consider(
                    obs.subject,
                    HARM_ATTR,
                    [HARMS, BENIGN],
                    obs.toward,
                    obs.weight,
                    mind.id,
                );
            }
        }
        assert_eq!(
            mind.belief(salt_subject, HARM_ATTR, &params()),
            None,
            "a being that never senses the salt feature never forms a belief about it"
        );
    }

    #[test]
    fn the_run_path_forms_the_belief_and_reads_no_injected_hazard() {
        // A structural guarantee that the retirement is complete (the sibling of the metabolism
        // substrate's identity-blindness check): the runner's canonical path no longer authors a hazard
        // belief off a dose threshold, and it forms the belief through the associative learner instead.
        // A future edit that reaches back for the injected hazard subject fails this build.
        let src = include_str!("runner.rs");
        assert!(
            !src.contains("HAZARD_SUBJECT") && !src.contains("HAZARD_ATTR"),
            "the injected hazard belief is retired: no HAZARD_* constant remains on the canonical path"
        );
        assert!(
            src.contains("feature_observations("),
            "the runner forms the belief through the experiential associative learner"
        );
    }

    #[test]
    fn harm_free_ticks_on_a_feature_self_correct_toward_benign() {
        // Absence self-correction, the falsifier for free: a being that stands on the feature but is not
        // harmed (a tolerant halophile, or the hazard removed) accumulates BENIGN and commits it.
        let calib = HarmLearningCalib::dev_default();
        let features = vec![Fixed::from_int(2)];
        let subject = feature_subject(
            0,
            feature_bucket(Fixed::from_int(2), calib.feature_granularity),
        );
        let mut mind = Mind::new(StableId(3), Fixed::ONE);
        for _ in 0..3 {
            for obs in feature_observations(false, &features, Fixed::ONE, &calib) {
                mind.consider(
                    obs.subject,
                    HARM_ATTR,
                    [HARMS, BENIGN],
                    obs.toward,
                    obs.weight,
                    mind.id,
                );
            }
        }
        assert_eq!(
            mind.belief(subject, HARM_ATTR, &params()),
            Some(BENIGN),
            "harm-free experience on the feature commits BENIGN, so the belief is defeasible"
        );
    }

    fn salt_field(cell: Coord3, dose: Fixed) -> ResourceField {
        let mut f = ResourceField::new();
        let mut toxins = std::collections::BTreeMap::new();
        toxins.insert(SALINITY.to_string(), dose);
        f.set(
            cell,
            crate::edibility::Composition {
                nutrients: std::collections::BTreeMap::new(),
                toxins,
            },
        );
        f
    }

    #[test]
    fn the_avoidance_gradient_points_away_from_a_believed_harmful_cell_and_is_zero_without_the_belief(
    ) {
        // Harm-learning arc slice c, the belief-to-behaviour percept: a being that has learned the salt
        // harms it reads an avoidance gradient pointing AWAY from the believed-harmful ground; a being
        // that has learned nothing reads none. The gradient is a percept the controller weights, so no
        // flight is authored here (that emerges when selection lifts the weight).
        let _ = (HomeostaticRegistry::dev_default(), CONDITION); // the axis the gradient routes into
        let calib = HarmLearningCalib::dev_default();
        let percepts = PerceptRegistry::dev_salinity();
        let here = Coord3::ground(5, 5);
        let salt = Coord3::ground(7, 5); // two tiles due east of the being
        let dose = Fixed::from_int(2);
        let field = salt_field(salt, dose);
        let subject = feature_subject(0, feature_bucket(dose, calib.feature_granularity));

        // A being that believes nothing harmful nearby has no avoidance gradient.
        let mut mind = Mind::new(StableId(1), Fixed::ONE);
        assert_eq!(
            avoidance_gradient(
                &mind,
                here,
                &field,
                &percepts,
                4,
                calib.feature_granularity,
                &params()
            ),
            (Fixed::ZERO, Fixed::ZERO),
            "no learned harm nearby, no avoidance gradient"
        );

        // Teach it that the salt harms (commit HARMS about the salt feature-kind).
        for _ in 0..3 {
            mind.consider(
                subject,
                HARM_ATTR,
                [HARMS, BENIGN],
                HARMS,
                calib.observation_weight(),
                mind.id,
            );
        }
        assert_eq!(mind.belief(subject, HARM_ATTR, &params()), Some(HARMS));
        let (gx, gy) = avoidance_gradient(
            &mind,
            here,
            &field,
            &percepts,
            4,
            calib.feature_granularity,
            &params(),
        );
        // The salt is to the EAST (+x), so the avoidance gradient points WEST (-x), away from it, with
        // no north-south component (the salt is due east).
        assert!(
            gx < Fixed::ZERO,
            "the gradient points away from the salt to the east: gx={gx:?}"
        );
        assert_eq!(
            gy,
            Fixed::ZERO,
            "the salt is due east, so no north-south pull"
        );
    }

    #[test]
    fn a_world_with_no_percepts_reads_no_avoidance_gradient() {
        // The opt-out short-circuit: without a declared percept the being senses no feature and forms no
        // avoidance, so the gradient is zero and the run is unchanged.
        let mind = Mind::new(StableId(1), Fixed::ONE);
        let field = salt_field(Coord3::ground(6, 5), Fixed::from_int(2));
        assert_eq!(
            avoidance_gradient(
                &mind,
                Coord3::ground(5, 5),
                &field,
                &PerceptRegistry::empty(),
                4,
                Fixed::ONE,
                &params()
            ),
            (Fixed::ZERO, Fixed::ZERO),
        );
    }

    #[test]
    fn a_being_reads_an_attraction_gradient_toward_a_believed_rewarding_material() {
        // Trace slice C3, the belief-to-behaviour percept, the positive mirror of the avoidance gradient: a
        // being that has re-earned "this material marks a place that pays off" reads an attraction gradient
        // pointing TOWARD the believed-rewarding cell; a being that has learned nothing reads none. The
        // gradient is a percept the controller weights, so no approach is authored here (it emerges when
        // selection lifts the weight). The subject is reconstructed at the material channel base, the same
        // offset the trace reward learner committed it under.
        use crate::material::MaterialField;
        use crate::material_percept::MaterialPerceptRegistry;

        let percepts = MaterialPerceptRegistry::from_substances(&["spent_hull"]);
        let here = Coord3::ground(5, 5);
        let hull_cell = Coord3::ground(7, 5); // two tiles due east of the being
        let amount = Fixed::from_int(2);
        let mut field = MaterialField::new();
        field.deposit(hull_cell, "spent_hull", amount);
        let gran = Fixed::ONE;
        let subject = feature_subject(MATERIAL_FEATURE_CHANNEL_BASE, feature_bucket(amount, gran));

        // A being that believes nothing rewarding nearby has no attraction gradient.
        let mut mind = Mind::new(StableId(1), Fixed::ONE);
        assert_eq!(
            attraction_gradient(
                &mind,
                here,
                &field,
                &percepts,
                4,
                gran,
                MATERIAL_FEATURE_CHANNEL_BASE,
                &params()
            ),
            (Fixed::ZERO, Fixed::ZERO),
            "no learned reward nearby, no attraction gradient"
        );

        // Teach it that the hull marks a rewarding place (commit REWARDS about the material feature-kind).
        for _ in 0..3 {
            mind.consider(
                subject,
                REWARD_ATTR,
                [REWARDS, NEUTRAL],
                REWARDS,
                RewardLearningCalib::dev_default().observation_weight(),
                mind.id,
            );
        }
        assert_eq!(mind.belief(subject, REWARD_ATTR, &params()), Some(REWARDS));
        let (gx, gy) = attraction_gradient(
            &mind,
            here,
            &field,
            &percepts,
            4,
            gran,
            MATERIAL_FEATURE_CHANNEL_BASE,
            &params(),
        );
        // The hull is to the EAST (+x), so the attraction gradient points EAST (+x), toward it, with no
        // north-south component (the hull is due east). The exact sign flip of the avoidance gradient.
        assert!(
            gx > Fixed::ZERO,
            "the gradient points toward the hull to the east: gx={gx:?}"
        );
        assert_eq!(
            gy,
            Fixed::ZERO,
            "the hull is due east, so no north-south pull"
        );

        // Opt-out: an empty material-percept registry senses nothing, so the gradient is zero.
        assert_eq!(
            attraction_gradient(
                &mind,
                here,
                &field,
                &MaterialPerceptRegistry::empty(),
                4,
                gran,
                MATERIAL_FEATURE_CHANNEL_BASE,
                &params()
            ),
            (Fixed::ZERO, Fixed::ZERO),
        );
    }
}
