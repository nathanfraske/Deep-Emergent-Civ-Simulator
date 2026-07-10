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
use civsim_physics::laws::{DiscriminationLaw, ResponseLaw};
use civsim_world::Coord3;
use std::collections::{BTreeMap, BTreeSet};

use crate::perception_percept::{sense, ChannelTransduction};
use crate::perception_reach::{
    ChannelReach, ChannelReachRegistry, Reach, ReachBounds, DEV_OPTICAL,
};

use crate::agent::Mind;
use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::evidence::{good_weight, AttrKindId, InferenceParams, ValueId};
use crate::homeostasis::AffordanceId;
use crate::locomotion::ResourceField;
use crate::material::MaterialField;
use crate::material_percept::MaterialPerceptRegistry;
use crate::percept::{feature_bucket, PerceptRegistry};
use crate::sensorium::SenseChannelId;

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

/// The reserved base of the per-BEING-SIGNAL belief-subject band (the being-percept keystone, step 1): a
/// perceived being-signal, keyed by its sense channel and discriminated bucket, mints its harm or reward
/// belief subject HERE, in its OWN top-level band, disjoint from every environmental-feature subject and
/// every sequence subject. It sets bit 60 as well as bit 62, so a being-signal subject is DISJOINT from a
/// feature subject (whose payload stays in the low 48 bits, leaving bit 60 clear) and from a sequence subject
/// (which sets bit 61, clear here), and stays below `1 << 63` (below the reserved-high landmark ids). So a
/// being-signal on sense channel c never aliases the environmental biology feature at index c under the same
/// attribute (the slice-3 subject-namespace seam), closed by construction rather than by a
/// [`MATERIAL_FEATURE_CHANNEL_BASE`]-style channel offset (which would not fit the `u16` channel field, since
/// biology and material features already split it at `1 << 15`). The bit-60 placement is coordinated with the
/// affordance composer's hybrid belief-subject key as one of three disjoint top-level bands (features,
/// sequences and conjunctions, being-signals); if the hybrid claims bit 60 the base retargets to another free
/// bit, a one-line change, and this band is byte-neutral until the keystone's live wire consumes it.
const BEING_SIGNAL_SUBJECT_BASE: u64 = (1 << 62) | (1 << 60);

/// Mint the belief subject for a perceived being-signal: the `(channel, bucket)` pair packed into the
/// reserved being-signal band ([`BEING_SIGNAL_SUBJECT_BASE`]), the sibling of [`feature_subject`]'s
/// per-feature band. Two perceived beings whose signal on the same sense channel lands in the same bucket
/// mint the SAME subject (the generalisation the quantization buys), so a valence learned about one applies
/// to another emitting the same signal; a different channel or a different bucket is a different subject. The
/// bucket is clamped non-negative and into the 32-bit field, so the packing never overflows the band, exactly
/// as [`feature_subject`] does.
pub fn being_signal_subject(channel: u16, bucket: i64) -> StableId {
    let bucket = (bucket.max(0) as u64) & FEATURE_BUCKET_MASK;
    StableId(BEING_SIGNAL_SUBJECT_BASE | ((channel as u64) << FEATURE_CHANNEL_SHIFT) | bucket)
}

/// The reserved base of the per-SEQUENCE belief-subject band (ideation / experiential-discovery arc, piece
/// 1, slice 1b): a discovered ACTION is a wildcard template over a primitive sequence, and this band mints
/// the belief subject it is keyed on, the sibling of [`feature_subject`]'s per-feature band. It sets bit 61
/// as well as bit 62, so a sequence subject is DISJOINT from every feature subject (whose payload stays in
/// the low 48 bits, leaving bit 61 clear) and from every being id, and stays below `1 << 63` (below the
/// reserved-high landmark ids), exactly as the feature band does. So two beings that execute the SAME
/// affordance-typed sequence mint the IDENTICAL subject, and a gossiped belief about it does not diverge.
const SEQUENCE_SUBJECT_BASE: u64 = (1 << 62) | (1 << 61);
/// The packed-vs-hashed marker WITHIN the sequence band (R-DEEPTECH-COMPOSE hybrid belief-subject key):
/// bit 60. Clear = the EXACT widened pack (the common case, collision-free); set = the HASH-on-overflow
/// sub-band (a rare beyond-envelope sequence). Both keep bit 61 set (the sequence-band marker), so they stay
/// disjoint from Agent A's being-signal band (`(1 << 62) | (1 << 60)`, bit 61 clear) and the feature band
/// (bit 62 only): the four bands partition the `{60, 61}` pair under bit 62 (feature 00, being-signal 01,
/// sequence-exact 10, sequence-hash 11), collision-free and below the reserved-high landmark ids (bit 63).
const SEQ_HASH_MARKER: u64 = 1 << 60;
/// The most sequence steps the EXACT pack holds. A longer sequence mints via the hash sub-band, so an
/// arbitrary conjunction is still representable (the composer's need) with no truncation, never the old
/// prefix-truncation bound.
const SEQ_MAX_STEPS: usize = 4;
/// The exact-pack field widths. These are an ENGINE ENCODING-CAPACITY budget, NOT a value the owner sets:
/// the hash sub-band preserves the subject's identity and determinism at ANY width (two beings executing the
/// same sequence mint the identical subject regardless), so the widths steer only which sequences take the
/// exact path versus the hash path, never which affordances share a belief nor any emergent outcome, and
/// there is no physics-floor basis for a bit count. They are chosen, with a rationale: the PRIMITIVE field is
/// widened most (6 bits, 64 primitives) because it is the one that bites as the made-world affordance
/// alphabet grows past the old 4-bit / 16-value cap the FLAGGED BOUND named; the target-affordance and param
/// buckets take 3 bits (8 buckets each), NARROWER than the old uniform 4-bit / 16-value field, because the
/// exact tier only needs to cover the common few-channel, coarse-bucket case and the hash sub-band covers the
/// rest FULLY and deterministically (a rich-sensorium being's many-channel or fine-discrimination steps hash,
/// never lossily merge). Four steps of `SEQ_STEP_BITS` (12) plus a 3-bit count is 51 bits, inside the 60-bit
/// sequence-band envelope with headroom. Widening the exact tier later is a perf/exposure tuning (one more
/// re-pin), not a world call.
const SEQ_PRIMITIVE_BITS: u32 = 6;
const SEQ_TARGET_BITS: u32 = 3;
const SEQ_PARAM_BITS: u32 = 3;
/// The bit width of one packed step (its three fields) and each field's exact-pack maximum.
const SEQ_STEP_BITS: u32 = SEQ_PRIMITIVE_BITS + SEQ_TARGET_BITS + SEQ_PARAM_BITS;
const SEQ_PRIMITIVE_MAX: u64 = (1 << SEQ_PRIMITIVE_BITS) - 1;
const SEQ_TARGET_MAX: u64 = (1 << SEQ_TARGET_BITS) - 1;
const SEQ_PARAM_MAX: u64 = (1 << SEQ_PARAM_BITS) - 1;
/// The hash sub-band payload mask: the low 60 bits (0..=59), below bit 60's marker.
const SEQ_HASH_PAYLOAD_MASK: u64 = SEQ_HASH_MARKER - 1;

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
/// diverging); a different primitive, target kind, param kind, or ORDER is a different subject. The hybrid
/// key packs a sequence WITHIN the envelope (at most [`SEQ_MAX_STEPS`] steps, each field within its widened
/// width) EXACTLY, with no clamping and no over-merge, and routes any sequence beyond it to the hash
/// sub-band ([`sequence_subject_hashed`]) instead of truncating or merging it, so the pack never overflows
/// the band and no distinct sequence is silently conflated. The step COUNT is packed above the steps so a
/// prefix is not confused with the full sequence.
pub fn sequence_subject(steps: &[SequenceStep]) -> StableId {
    // The hybrid key: a sequence WITHIN the envelope (at most SEQ_MAX_STEPS steps, every field within its
    // widened width) mints the EXACT pack (bit 60 clear), collision-free; any sequence BEYOND the envelope
    // (more steps, or a field value past its width) mints via the hash sub-band (bit 60 set), so an
    // arbitrary conjunction is still representable without the common-case collision a pure hash would incur.
    // A negative target/param bucket clamps to zero (a bucket is non-negative) and is not itself overflow.
    let over_envelope = steps.len() > SEQ_MAX_STEPS
        || steps.iter().any(|s| {
            (s.primitive as u64) > SEQ_PRIMITIVE_MAX
                || (s.target_bucket.max(0) as u64) > SEQ_TARGET_MAX
                || (s.param_bucket.max(0) as u64) > SEQ_PARAM_MAX
        });
    if over_envelope {
        return sequence_subject_hashed(steps);
    }
    let n = steps.len();
    let mut payload: u64 = 0;
    for (i, step) in steps.iter().enumerate() {
        // Every field is within its width here (the envelope check above guarantees it).
        let primitive = step.primitive as u64;
        let target = step.target_bucket.max(0) as u64;
        let param = step.param_bucket.max(0) as u64;
        let packed_step = primitive
            | (target << SEQ_PRIMITIVE_BITS)
            | (param << (SEQ_PRIMITIVE_BITS + SEQ_TARGET_BITS));
        payload |= packed_step << (i as u32 * SEQ_STEP_BITS);
    }
    // The step count, packed above the step fields (bits 48..), so a 2-step prefix of a 3-step sequence
    // mints a different subject than the 3-step sequence itself.
    payload |= (n as u64) << (SEQ_MAX_STEPS as u32 * SEQ_STEP_BITS);
    StableId(SEQUENCE_SUBJECT_BASE | payload)
}

/// The HASH sub-band of the sequence-subject band (the hybrid key's overflow path): a beyond-envelope
/// sequence (more than [`SEQ_MAX_STEPS`] steps, or a field past its widened width) mints its subject by a
/// canonical RNG-free [`StateHasher`] digest of the FULL step sequence, folded into the low 60 payload bits
/// and marked by bit 60 (`SEQ_HASH_MARKER`), so an arbitrary conjunction is representable. Any collision is
/// confined to this rare overflow (a birthday collision in `2^60`), never the common in-envelope case, which
/// is exact. Deterministic (`StateHasher` is RNG-free, Principle 3): the same sequence hashes identically
/// wherever recomputed, so a gossiped belief about an over-envelope action does not diverge.
fn sequence_subject_hashed(steps: &[SequenceStep]) -> StableId {
    let mut h = StateHasher::new();
    h.write_u64(steps.len() as u64);
    for step in steps {
        // Clamp the non-negative buckets exactly as the exact-pack path and the over-envelope detector do
        // (`target_bucket.max(0)`), so the two branches agree on the "a negative bucket is the zero bucket"
        // convention: an over-envelope sequence with a negative bucket mints the same subject the same
        // sequence with a zero bucket would, matching the exact tier and the sibling feature band.
        h.write_u32(step.primitive as u32);
        h.write_i64(step.target_bucket.max(0));
        h.write_i64(step.param_bucket.max(0));
    }
    let payload = (h.finish() as u64) & SEQ_HASH_PAYLOAD_MASK;
    StableId(SEQUENCE_SUBJECT_BASE | SEQ_HASH_MARKER | payload)
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

/// Mint one observation on a belief subject: the SHARED minting the environmental-feature learner and the
/// being-signal learner both call, so a being-signal is minted by the IDENTICAL code path (not a copy),
/// and the no-special-casing property (Principle 8) holds structurally, not by two inline duplicates
/// staying in sync. The direction is `HARMS` on a harm tick and `BENIGN` otherwise; nothing but the subject,
/// the felt sign, and the weight enters.
fn observation_toward(subject: StableId, harm: bool, weight: Fixed) -> FeatureObservation {
    FeatureObservation {
        subject,
        toward: if harm { HARMS } else { BENIGN },
        weight,
    }
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
    features
        .iter()
        .enumerate()
        .filter(|(_, &amount)| amount > Fixed::ZERO)
        .map(|(channel, &amount)| {
            let bucket = feature_bucket(amount, calib.feature_granularity);
            observation_toward(feature_subject(channel as u16, bucket), harm, weight)
        })
        .collect()
}

/// The valence observation a being makes of a perceived BEING-SIGNAL (perception-substrate arc, slice 3, the
/// receiver-side valence learner core): the being correlates the signal, keyed by [`feature_subject`] on the
/// sense channel and discriminated bucket it perceived, with its own interoceptive harm bit, minting one
/// weight-of-evidence observation toward [`HARMS`] or [`BENIGN`] through the SHARED [`observation_toward`]
/// minting the environmental-feature learner also uses. The being-signal is never branched on as "a being",
/// so a signal's valence emerges receiver-side from the receiver's own correlated outcomes and is never
/// stamped at the emitter. A being learns another's alarm call means harm because perceiving it correlated
/// with its own reserves falling, exactly as it learns salty ground harms it. Pure and OFF the run path (no
/// live caller): the being-percept keystone consumes it, so this is byte-neutral by construction. The
/// (channel, bucket) come from the sensorium-gated magnitude percept
/// ([`crate::perception_percept::MagnitudePercept`]); the harm bit is the being's own
/// [`crate::homeostasis::is_harm_tick`] read; the weight is the existing learner's observation weight scaled
/// by the being's plasticity.
///
/// Scope of "identical": the MINTING is shared and bit-identical for a given (channel, bucket, harm,
/// weight). PRESENCE is NOT this core's job: [`feature_observations`] filters absent features (nothing to
/// correlate), whereas this core mints for whatever (channel, bucket) it is handed, delegating presence to
/// the upstream percept gate ([`crate::perception_percept::sense`] returns `None` below the being's detection
/// threshold, so silence and absence do not reach here). The keystone that wires this must pass only a
/// present, above-threshold percept.
///
/// RESERVED derive targets and keystone-wiring seams, surfaced with basis, deferred by the gate to their own
/// builds (this core reuses the existing learner and reserves them, so it moves no pin):
/// - SUBJECT NAMESPACE (RESOLVED by the keystone, step 1; was a section-9-caught seam): this mints into the
///   being-signal's OWN band through [`being_signal_subject`] ([`BEING_SIGNAL_SUBJECT_BASE`]), disjoint from
///   every environmental-feature subject and every sequence subject, so a being-signal on sense channel c no
///   longer aliases the environmental biology feature at index c under the same `HARM_ATTR`. The band bit is
///   coordinated with the affordance composer's hybrid belief-subject key as one of three disjoint top-level
///   bands, and the `SenseChannelId`-fits-the-channel-field question falls out of that hybrid encoding within
///   the being-signal band (the gate's packing ruling).
/// - The evidence WEIGHT's two likelihoods are the existing reserved `p_harm_given_harms`/`p_harm_given_benign`
///   (never the fixed dev 0.9/0.1). Their per-being derived form is the deepest derive target, a build shared
///   with the affordance composer, framed and sequenced separately. Basis, corrected by the section-9 catch:
///   a being-signal is a harm PREDICTOR, not a harm CAUSE (the predator harms, not the alarm call), so its
///   likelihood is NOT a dose-response of the signal (the signal has no dose): it is the receiver's own
///   EMPIRICAL co-occurrence reliability, the base rate the evidence engine already accumulates. The
///   [`civsim_physics::laws::harm_class`] dose-response crossed with the reserve-delta noise distribution is
///   the basis for an environmental harm-CAUSE's detection reliability, a distinct case.
/// - The harm bit's NOISE FLOOR is a flat authored scalar today; deriving it per-axis from
///   [`crate::homeostasis::DerivedDrain`], and making the outcome per-axis (so a signal harmful on one reserve
///   and beneficial on another, and an alien with heterogeneous reserves, are distinguished), is a
///   LIVE-learner behaviour change (it moves the pins), reserved for the keystone or its own stated-hash piece.
/// - The correlation is SAME-TICK: the harm path carries no eligibility trace. The reward pole's
///   `eligibility_decay` on the harm path, which credits a LAGGED co-occurrence (the predator-approach-then-harm
///   alarm cue, load-bearing for the predation payoff), is a live behaviour change reserved for the keystone.
/// - REFERENTIAL meaning (a signal predicting harm elsewhere or to another, not the receiver's own harm) is a
///   flagged open limit: the outcome the learner correlates against is the receiver's own reserve fall.
pub fn being_signal_observation(
    channel: u16,
    bucket: i64,
    harm: bool,
    plasticity: Fixed,
    calib: &HarmLearningCalib,
) -> FeatureObservation {
    let base = calib.observation_weight();
    let weight = base.checked_mul(plasticity).unwrap_or(base);
    observation_toward(being_signal_subject(channel, bucket), harm, weight)
}

/// The LAGGED-credit harm observations a being makes on a harm tick (the being-percept keystone, step 4, the
/// harm-path eligibility trace): one being-signal harm observation per subject still remembered in the
/// perceiver's harm-eligibility [`EligibilityTrace`], each toward `HARMS` on a harm tick and `BENIGN`
/// otherwise, weighted by the reserved harm observation weight scaled by the being's plasticity AND the
/// subject's decayed eligibility. So a being-signal perceived several ticks BEFORE the harm still earns
/// partial credit, falling with the ticks since it was perceived (the trace's decay), which is what lets a
/// predator-approach cue perceived at a distance be credited to the harm that follows: the lagged distal
/// association the same-tick [`being_signal_observation`] cannot form, load-bearing for the predation payoff.
/// The trace is the perceiver's OWN (recording the being-signals it perceived); the harm bit is its own
/// [`crate::homeostasis::is_harm_tick`]; nothing reads a species, trophic role, or being-hood. Pure and OFF
/// the run path (no live caller): the keystone's live wire (step 6) records each perceived being-signal into
/// the harm-eligibility trace, decays it each tick by the reserved harm eligibility latency, and calls this
/// on the being's harm bit, so this is byte-neutral by construction. The environmental-feature harm path
/// ([`feature_observations`]) is UNCHANGED and stays same-tick: only the being-signal path carries the trace.
pub fn being_signal_trace_observations(
    trace: &EligibilityTrace,
    harm: bool,
    plasticity: Fixed,
    calib: &HarmLearningCalib,
) -> Vec<FeatureObservation> {
    let base = calib.observation_weight();
    let weight = base.checked_mul(plasticity).unwrap_or(base);
    trace
        .entries()
        .map(|(&subject, &eligibility)| {
            let credited = weight.checked_mul(eligibility).unwrap_or(Fixed::ZERO);
            observation_toward(subject, harm, credited)
        })
        .collect()
}

/// The REWARD-frame counterpart of [`being_signal_observation`] (the being-percept keystone, step 2, the
/// PREDATION pole's learner): a being correlates a perceived being-signal, keyed by its sense `channel` and
/// discriminated `bucket`, with its OWN reward bit that tick ([`crate::homeostasis::is_reward_tick`], whether
/// any reserve ROSE beyond the noise floor), minting one observation toward `REWARDS` on a reward tick and
/// `NEUTRAL` otherwise, on the SAME [`being_signal_subject`] the harm core uses but fed into the disjoint
/// `(subject, REWARD_ATTR)` frame. So perceiving a being that correlates with the perceiver's own reserves
/// RISING (it ate) mints a reward belief on that being-signal, the substrate the predation pole rests on;
/// perceiving a being that correlates with reserves FALLING mints the harm belief through
/// [`being_signal_observation`]. Which belief forms, and so which pole a being later acts on, emerges from
/// the being's own outcomes, never from a species, trophic role, or being-hood read.
///
/// Minted INLINE toward `REWARDS`/`NEUTRAL`, exactly as [`reward_observations`] does for a material feature,
/// because the shared [`observation_toward`] helper is hardwired to the harm frame (`HARMS`/`BENIGN`) and a
/// reward observation points the other way. The weight is the reserved reward observation weight scaled by
/// the being's plasticity, reusing the existing reward likelihoods, never a new fabricated weight. Pure and
/// OFF the run path (no live caller): the keystone's live wire consumes it, so this is byte-neutral by
/// construction. The reserved derive targets and the eligibility-trace latency are the same ones
/// [`being_signal_observation`] documents; they are shared, not re-listed here.
pub fn being_signal_reward_observation(
    channel: u16,
    bucket: i64,
    reward: bool,
    plasticity: Fixed,
    calib: &RewardLearningCalib,
) -> FeatureObservation {
    being_signal_reward_for(
        being_signal_subject(channel, bucket),
        reward,
        plasticity,
        calib,
    )
}

/// The being-signal REWARD observation for an ALREADY-PERCEIVED subject (the being-percept keystone, step 6,
/// the live-wire reward pole): the subject-taking core of [`being_signal_reward_observation`], for the run
/// path where the perceiver already holds the [`being_signal_subject`] it formed via
/// [`perceive_being_signal`] rather than the raw channel and bucket. It mints a reward belief on that subject
/// toward `REWARDS` on a reward tick and `NEUTRAL` otherwise, weighted by the reserved reward observation
/// weight scaled by the being's plasticity, the reward-frame sibling of the harm-frame [`observation_toward`]
/// (which is hardwired to `HARMS`/`BENIGN`), reusing the existing reward likelihoods, never a fabricated
/// weight.
pub fn being_signal_reward_for(
    subject: StableId,
    reward: bool,
    plasticity: Fixed,
    calib: &RewardLearningCalib,
) -> FeatureObservation {
    let base = calib.observation_weight();
    let weight = base.checked_mul(plasticity).unwrap_or(base);
    FeatureObservation {
        subject,
        toward: if reward { REWARDS } else { NEUTRAL },
        weight,
    }
}

/// Perceive a being-signal from one emitter's resolved [`Reach`] and mint the belief subject the perceiver
/// keys its valence on, or `None` if the signal does not clear the perceiver's OWN detection threshold (the
/// emitter is out of reach, occluded by the strata, or too faint for this perceiver's sense). The
/// being-percept keystone, step 3a: the alien-clean consumption of the reach (slice 1) and the
/// sensorium-gated percept (slice 2). The received magnitude is the geometrically-spread reach attenuated by
/// the medium's Beer-Lambert transmittance `exp(-optical_depth)`, the transcendental
/// [`crate::perception_reach::Reach`] reports-then-defers to its consumer, so a strongly absorbing medium
/// (rock between the two) drives the transmittance toward zero and the emitter is not perceived: occlusion
/// emerges from the strata, no authored line-of-sight. [`sense`] transduces that magnitude through the
/// PERCEIVER's own channel transduction and gates it on the perceiver's own threshold; the discriminated
/// bucket keys [`being_signal_subject`]. Keyed on the EMISSION on a channel and the resolved reach, never on
/// a species, kingdom, trophic role, relatedness, or being-hood of the emitter (the emitter is a source of a
/// signal, whether a being or a fire). Pure and off the run path (no live caller): the keystone's live wire
/// (step 6) resolves each emitter's own emitted power and medium ([`crate::perception_reach::resolve_reach`],
/// where the emission assembly and the material field live) and calls this per perceived emitter and channel,
/// so this is byte-neutral by construction.
pub fn perceive_being_signal(
    reach: Reach,
    channel: u16,
    transduction: &ChannelTransduction,
    activation_max: Fixed,
) -> Option<StableId> {
    // The received magnitude is the geometric spread attenuated by the medium's transmittance, the
    // Beer-Lambert `exp(-optical_depth)` the reach substrate defers to here. The optical depth is
    // non-negative and capped, so the transmittance is in (0, 1] and the product cannot exceed the spread
    // nor overflow.
    let transmittance = (-reach.optical_depth).exp();
    let magnitude = reach
        .spread
        .checked_mul(transmittance)
        .unwrap_or(Fixed::ZERO);
    let percept = sense(magnitude, transduction, activation_max)?;
    Some(being_signal_subject(channel, percept.bucket))
}

/// Perceive a being-signal's MAGNITUDE (the perceiver's own transduced activation) from one emitter's resolved
/// [`Reach`], or `None` if it does not clear the perceiver's own detection threshold. The creature-tier sibling
/// of [`perceive_being_signal`]: where that mints the belief SUBJECT (channel and bucket) the founder keys its
/// learned valence on, this returns the transduced ACTIVATION the mind-less creature's
/// [`creature_being_direction`] weights its toward-pull by (mechanism B3, no belief, no bucket-keyed subject, no
/// channel). The received magnitude is the same geometrically-spread reach attenuated by the medium's
/// Beer-Lambert transmittance, transduced through the SUPPLIED channel transduction and gated on its threshold,
/// so occlusion and perceptibility key on that transduction exactly as the founder's perception does. The caller
/// passes the world's DECLARED transduction (a per-species/per-world datum, matching the founder path); deriving
/// it per creature from its sensory anatomy is the flagged shared follow-on, so "the creature's own sense" is
/// per-species declared data today, not yet per-creature-derived. Pure and RNG-free.
pub fn perceive_being_magnitude(
    reach: Reach,
    transduction: &ChannelTransduction,
    activation_max: Fixed,
) -> Option<Fixed> {
    let transmittance = (-reach.optical_depth).exp();
    let magnitude = reach
        .spread
        .checked_mul(transmittance)
        .unwrap_or(Fixed::ZERO);
    sense(magnitude, transduction, activation_max).map(|percept| percept.activation)
}

/// The being-directed belief gradient (the being-percept keystone, step 3b): over the perceived emitters
/// (each a position and the [`being_signal_subject`] the perceiver formed for it via
/// [`perceive_being_signal`]), the summed inverse-distance vector over every emitter the perceiver holds the
/// committed belief `committed` about on attribute `attr`, pointing AWAY from each such emitter when `toward`
/// is false (avoidance) and TOWARD it when true (attraction). A nearer emitter contributes harder (weight
/// `1/d`); a perceiver that believes nothing of the kind nearby gets a zero gradient. The being-directed
/// mirror of the material [`avoidance_gradient`] / [`attraction_gradient`], keyed on the perceived signal's
/// LEARNED belief, never on a species, kingdom, trophic role, relatedness, or being-hood. The horizontal
/// `(x, y)` direction only, matching the 2D heading the controller moves by (the reach already accounted for
/// the vertical separation in perceptibility). Pure and RNG-free.
fn being_directed_gradient(
    mind: &Mind,
    here: Coord3,
    perceived: &[(Coord3, StableId)],
    attr: AttrKindId,
    committed: ValueId,
    toward: bool,
    params: &InferenceParams,
) -> (Fixed, Fixed) {
    let mut ax = Fixed::ZERO;
    let mut ay = Fixed::ZERO;
    for &(pos, subject) in perceived {
        if mind.belief(subject, attr, params) != Some(committed) {
            continue;
        }
        let dx = pos.x - here.x;
        let dy = pos.y - here.y;
        // Skip a co-located emitter (no direction) and a separation whose square overflows i32: an emitter
        // that far has negligible reach and would not clear the threshold to be perceived anyway.
        let d2i = (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64);
        if d2i == 0 || d2i > i32::MAX as i64 {
            continue;
        }
        let d2 = Fixed::from_int(d2i as i32);
        // `(dx, dy)/d2` points TOWARD the emitter, `(-dx, -dy)/d2` AWAY; weighted by inverse distance.
        let (sx, sy) = if toward { (dx, dy) } else { (-dx, -dy) };
        if let (Some(cx), Some(cy)) = (
            Fixed::from_int(sx).checked_div(d2),
            Fixed::from_int(sy).checked_div(d2),
        ) {
            ax = ax.saturating_add(cx);
            ay = ay.saturating_add(cy);
        }
    }
    (ax, ay)
}

/// The being-directed expected-HARM avoidance gradient (the being-percept keystone, step 3b): the
/// being-directed mirror of [`avoidance_gradient`], the raw inverse-distance sum pointing away from every
/// perceived emitter the perceiver holds a committed `HARMS` belief about. This is a PERCEPT, not a heading:
/// the runner feeds it into the controller's direction slot, and only a heritable FREELY-SIGNED weight lifted
/// off founder-zero by selection turns it into avoidance (a positive weight, fleeing a believed-harmful
/// emitter) or approach (a negative weight, a being drawn to a harm-predicting emitter, a parasite or
/// scavenger), so the approach/avoid SIGN emerges rather than being authored (Principle 9). The caller
/// normalises the raw sum to a unit percept, exactly as the material gradient is normalised.
pub fn being_avoidance_gradient(
    mind: &Mind,
    here: Coord3,
    perceived: &[(Coord3, StableId)],
    params: &InferenceParams,
) -> (Fixed, Fixed) {
    being_directed_gradient(mind, here, perceived, HARM_ATTR, HARMS, false, params)
}

/// The being-directed expected-REWARD attraction gradient (the being-percept keystone, step 3b, the PREDATION
/// pole): the being-directed mirror of [`attraction_gradient`], the raw inverse-distance sum pointing toward
/// every perceived emitter the perceiver holds a committed `REWARDS` belief about. A PERCEPT, not a heading,
/// the exact behavioural mirror of [`being_avoidance_gradient`]: only a heritable freely-signed weight off
/// founder-zero turns it into approach (a positive weight, pursuing a believed-rewarding emitter, predation)
/// or avoidance (a negative weight), so the sign emerges (Principle 9). The caller normalises the raw sum to
/// a unit percept.
pub fn being_attraction_gradient(
    mind: &Mind,
    here: Coord3,
    perceived: &[(Coord3, StableId)],
    params: &InferenceParams,
) -> (Fixed, Fixed) {
    being_directed_gradient(mind, here, perceived, REWARD_ATTR, REWARDS, true, params)
}

/// The CREATURE-tier being-directed percept (creatures-react arc, mechanism B3, the magnitude-graded raw
/// direction). A mind-less creature (an Arc-7 `Walker` absent from the mind registry) cannot run the
/// belief-reading [`being_directed_gradient`] the founder tier uses; instead its toward/away disposition rides
/// on the RAW perceived signal with NO belief and NO committed valence category. Over the beings it perceives
/// (each an emitter position and the creature's OWN transduced magnitude for it, the perceiver-keyed sense the
/// build constraint requires), this sums the UNIT toward-direction of each emitter SCALED by that magnitude.
/// The magnitude is the SOLE distance factor: the reach already attenuated the signal with distance before it
/// was transduced, so a nearer, bigger, or warmer emitter reads a larger magnitude and pulls harder, and no
/// geometric `1/d` is applied on top (that would double-count distance). The summed vector's LENGTH is clamped
/// to the unit disc (scaled down preserving its direction, never per-component, which would rotate it), so it
/// lands in the same `[-1, 1]`-magnitude range the founder's unit percept does. Only the horizontal `(x, y)`
/// heading is produced, matching the 2D heading the controller moves by; the vertical separation already
/// entered the perceptibility through the reach that produced the magnitude, exactly as the founder gradient
/// treats z. It is a PERCEPT, not a heading: only the creature's OWN heritable FREELY-SIGNED weight, set by
/// selection off founder-zero, turns it into approach (a positive weight, hunting) or flight (a negative
/// weight, fleeing), so the toward/away SIGN emerges rather than being authored (Principle 9), keyed only on
/// the emitted signal and the creature's own sense, never on the emitter's species, kind, trophic role,
/// relatedness, or being-hood. Its honest limit (mechanism B3): the discrimination runs along MAGNITUDE alone,
/// so two same-magnitude emitters of different kind read alike; the per-bucket block (mechanism B2) is the
/// fuller follow-on. Pure and RNG-free; no valence, no belief, no category.
pub fn creature_being_direction(here: Coord3, perceived: &[(Coord3, Fixed)]) -> (Fixed, Fixed) {
    let mut dx = Fixed::ZERO;
    let mut dy = Fixed::ZERO;
    for &(pos, magnitude) in perceived {
        if magnitude <= Fixed::ZERO {
            continue; // nothing perceived from this emitter (sub-threshold or absent), no pull
        }
        // The coordinate deltas in i64 so the subtraction cannot overflow i32 for far-apart raw coordinates;
        // the sum-of-squares guard below bounds each delta to `sqrt(i32::MAX)` (< i32::MAX) before any cast.
        let ex = pos.x as i64 - here.x as i64;
        let ey = pos.y as i64 - here.y as i64;
        // Skip a co-located emitter (no direction) and a separation whose square overflows i32, exactly as
        // the founder gradient does; such an emitter is too far to clear the threshold and be perceived anyway.
        let d2i = ex * ex + ey * ey;
        if d2i == 0 || d2i > i32::MAX as i64 {
            continue;
        }
        let dist = Fixed::from_int(d2i as i32).sqrt(); // |d| > 0 here, exact and deterministic
                                                       // The UNIT toward-direction `(ex, ey) / |d|` scaled by the perceived magnitude (the sole distance
                                                       // factor). An overflow in either step drops this emitter's contribution rather than fabricating one.
        let contribute = |num: i64| -> Fixed {
            Fixed::from_int(num as i32) // safe: |num| <= sqrt(i32::MAX) after the guard above
                .checked_div(dist)
                .and_then(|unit| unit.checked_mul(magnitude))
                .unwrap_or(Fixed::ZERO)
        };
        dx = dx.saturating_add(contribute(ex));
        dy = dy.saturating_add(contribute(ey));
    }
    // Clamp the vector LENGTH to the unit disc, preserving direction (a per-component clamp would rotate a
    // diagonal pull toward an axis). A strong aggregate pull saturates at unit length, a weak one stays
    // proportional, so the length carries the graded pull strength in the founder's `[-1, 1]` percept range.
    match (dx.checked_mul(dx), dy.checked_mul(dy)) {
        (Some(x2), Some(y2)) => {
            let len2 = x2.saturating_add(y2);
            if len2 > Fixed::ONE {
                let len = len2.sqrt(); // > ONE here, so the divide scales the vector down to unit length
                (
                    dx.checked_div(len).unwrap_or(Fixed::ZERO),
                    dy.checked_div(len).unwrap_or(Fixed::ZERO),
                )
            } else {
                (dx, dy)
            }
        }
        // Overflow of a component's square is UNREACHABLE for any realistic perceived population (a component
        // would need magnitude > ~46000, tens of thousands of emitters pulling one way; the O(N^2) perceive scan
        // is separately capped upstream). In that impossible case direction-preservation is moot, so the
        // last-resort caps each component to the unit box: it trades the length-clamp's direction-preservation
        // for a guaranteed bound and no panic. This is the ONLY path that clamps per-component; every reachable
        // input takes the length-clamp above, which preserves direction.
        _ => (
            dx.clamp(-Fixed::ONE, Fixed::ONE),
            dy.clamp(-Fixed::ONE, Fixed::ONE),
        ),
    }
}

/// The being-percept feature's run-path configuration (the being-percept keystone, step 6, the live wire):
/// everything the perceive phase needs to run the being-directed perception and learning for one world. The
/// SUBSTRATE bindings are labelled dev fixtures and floor data (the sense channel a being emits and is
/// perceived on, its reach binding, the perceiver's transduction, the physics reach caps, and the sensorium
/// activation cap), and the two RESERVED values are the owner's, surfaced with basis and read fail-loud from
/// the manifest at the feature arming, never fabricated (Principle 11): the emission coupling coefficient
/// (`physiology::being_signal_emission`'s lever) and the harm eligibility-trace decay lambda. A world that
/// does not arm being-percept carries `None` and is byte-identical.
///
/// Honest limit (flagged, not a defect): the substrate bindings are labelled fixtures for the payoff, not
/// per-being anatomy-derived. The perceiver transduction is a single world-global fixture rather than each
/// being's own [`crate::perception_percept::derive_optical_transduction`] over its evolved eye, and the
/// channel and reach binding are the dev-fixture optical pair. Deriving the transduction per being from its
/// anatomy (slice 2's reserved-sense-params kernel) and binding the channel from the world's sensorium data
/// is the flagged follow-on; the reserved emission coupling and eligibility lag are the owner's calibration
/// the payoff needs first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeingPerceptField {
    /// The sense channel a being emits on and is perceived on (the thermal/optical channel today; a
    /// non-optical channel is the flagged alien follow-on the receiver reserves fail-loud).
    pub channel: SenseChannelId,
    /// The channel's reach binding: the spreading law and the medium absorption axis the signal attenuates by
    /// along the path, so occlusion emerges from the strata rather than an authored line-of-sight rule.
    pub row: ChannelReach,
    /// The perceiver's transduction on the channel: its response law and gain, its discrimination law and
    /// step, and its detection threshold (the sole perceptibility gate, slice 2's condition 3).
    pub transduction: ChannelTransduction,
    /// The physics reach caps: the geometric sphere coefficient for the derived dimensionality, the irradiance
    /// cap the spread saturates at, and the optical-depth cap the medium attenuation saturates at (the
    /// occlusion limit). Engine-mechanics representability bounds, not owner values.
    pub bounds: ReachBounds,
    /// The sensorium activation cap the transduced percept saturates at. Engine-mechanics.
    pub activation_max: Fixed,
    /// RESERVED (fail-loud from the manifest at the feature arming): the per-being emission coupling
    /// coefficient the emitter's [`crate::physiology::being_signal_emission`] scales its blackbody radiant
    /// flux by. Basis: the body's covering emissivity times its radiating area, folded into one lever until a
    /// per-body material-and-area vector exists on the run path to split it (the gate-ruled follow-on).
    pub emission_coefficient: Fixed,
    /// RESERVED (fail-loud from the manifest at the feature arming): the temporal-difference lambda the
    /// being-signal HARM eligibility trace decays by each tick. Basis: the interoceptive lag between
    /// perceiving a being-signal (a predator approaching) and the harm that follows; the harm sibling of the
    /// reward pole's `reward.eligibility_decay`, set relative to it. In (0, 1).
    pub harm_eligibility_decay: Fixed,
}

impl BeingPerceptField {
    /// A labelled DEVELOPMENT FIXTURE, not owner data: the dev optical channel and its reach binding, a linear
    /// perceiver transduction with a modest gain, a small detection threshold, and a unit discrimination step,
    /// the D=3 physics reach caps (the sphere coefficient `4*pi` derived from `Fixed::PI`, not a truncated
    /// literal), and the two reserved values at their dev-fixture defaults. The real substrate is the world's
    /// data and the reserved values are the owner's; this stands up the harness and test paths.
    pub fn dev_fixture() -> BeingPerceptField {
        let row = ChannelReachRegistry::dev_terran()
            .get(DEV_OPTICAL)
            .expect("the dev optical reach row is present")
            .clone();
        let sphere_coeff = Fixed::from_int(4)
            .checked_mul(Fixed::PI)
            .expect("4*pi is representable");
        BeingPerceptField {
            channel: DEV_OPTICAL,
            row,
            transduction: ChannelTransduction {
                response: ResponseLaw::Linear,
                gain: Fixed::ONE,
                shape: Fixed::ZERO,
                discrimination: DiscriminationLaw::AbsoluteStep,
                step: Fixed::ONE,
                threshold: Fixed::from_ratio(1, 1_000_000),
            },
            bounds: ReachBounds {
                sphere_coeff,
                irrad_max: Fixed::from_int(1_000_000),
                tau_max: Fixed::from_int(1_000),
            },
            activation_max: Fixed::from_int(1_000_000),
            emission_coefficient: Fixed::from_ratio(1, 2),
            harm_eligibility_decay: Fixed::from_ratio(1, 2),
        }
    }

    /// The being-percept field read at the feature arming: the labelled fixture substrate bindings with the
    /// two RESERVED values read fail-loud from the manifest (Principle 11). A world that arms being-percept
    /// under `Profile::Calibrated` refuses while either reserved value is unset rather than fabricating one,
    /// exactly the gated-feature-calib pattern the reward and conviction learners follow (the value refuses AT
    /// the feature arming, not at an earlier always-on gate).
    pub fn from_manifest(m: &CalibrationManifest) -> Result<BeingPerceptField, CalibrationError> {
        let mut field = BeingPerceptField::dev_fixture();
        field.emission_coefficient = m.require_fixed("being_percept.emission_coefficient")?;
        field.harm_eligibility_decay = m.require_fixed("harm.eligibility_decay")?;
        Ok(field)
    }

    /// The channel id as the 16-bit channel field the being-signal subject packs (the subject band's channel
    /// slot). The dev optical channel is `1`, well within the field; a world whose channel id exceeds the
    /// field is the reserved widened-pack the gate ruled to Agent B's hybrid encoding.
    pub fn channel_u16(&self) -> u16 {
        self.channel.0 as u16
    }
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
    fn a_being_signal_earns_a_valence_observation_pointed_by_the_harm_bit() {
        // Slice 3 core: a perceived being-signal (channel, bucket) correlated with the being's own harm bit
        // mints one observation toward HARMS on a harm tick, BENIGN otherwise, on its own subject.
        let calib = HarmLearningCalib::dev_default();
        let harm = being_signal_observation(2, 7, true, Fixed::ONE, &calib);
        assert_eq!(harm.toward, HARMS, "a harm tick points toward HARMS");
        assert_eq!(
            harm.subject,
            being_signal_subject(2, 7),
            "keyed on channel and bucket in the being-signal's own band"
        );
        assert!(harm.weight > Fixed::ZERO, "positive evidence weight");
        let benign = being_signal_observation(2, 7, false, Fixed::ONE, &calib);
        assert_eq!(
            benign.toward, BENIGN,
            "a harm-free tick points toward BENIGN"
        );
        assert_eq!(
            benign.subject, harm.subject,
            "the same signal is the same subject"
        );
    }

    #[test]
    fn a_being_signal_is_learned_through_the_same_learner_on_its_own_disjoint_subject() {
        // The being-signal is fed through the exact same LEARNER: for the same channel, bucket, harm bit, and
        // plasticity, the valence direction and evidence weight match what the environmental learner mints, so
        // no "communication" path is special-cased and the valence emerges receiver-side from correlation
        // alone. But the SUBJECT is disjoint (the keystone step-1 namespace fix): a being-signal on channel c
        // mints in the being-signal band, never the environmental-feature band, so the two beliefs never
        // alias even at the same channel and bucket.
        let calib = HarmLearningCalib::dev_default();
        // An environmental feature whose amount buckets to a known bucket on channel 0.
        let amount = Fixed::from_int(2);
        let bucket = feature_bucket(amount, calib.feature_granularity);
        let env = feature_observations(true, &[amount], Fixed::ONE, &calib);
        let sig = being_signal_observation(0, bucket, true, Fixed::ONE, &calib);
        assert_eq!(sig.toward, env[0].toward, "same valence direction");
        assert_eq!(
            sig.weight, env[0].weight,
            "same evidence weight (the shared reserved likelihoods)"
        );
        assert_eq!(
            sig.subject,
            being_signal_subject(0, bucket),
            "minted in the being-signal band"
        );
        assert_ne!(
            sig.subject, env[0].subject,
            "disjoint from the environmental-feature subject at the same channel and bucket"
        );
    }

    #[test]
    fn the_being_signal_weight_scales_by_plasticity() {
        // The weight is the existing learner's observation weight scaled by the being's plasticity (a
        // keener learner extracts more evidence per observation), reusing the reserved likelihoods.
        let calib = HarmLearningCalib::dev_default();
        let base = being_signal_observation(1, 3, true, Fixed::ONE, &calib);
        let keen = being_signal_observation(1, 3, true, Fixed::from_int(2), &calib);
        assert_eq!(base.weight, calib.observation_weight());
        assert!(
            keen.weight > base.weight,
            "a higher plasticity extracts more evidence"
        );
    }

    #[test]
    fn the_being_signal_band_is_disjoint_from_the_feature_and_sequence_bands() {
        // Keystone step 1: a being-signal subject sets bit 60 (with bit 62), so it never equals a feature
        // subject (bit 60 clear, its payload in the low 48 bits) nor a sequence subject (bit 61 set, clear
        // here), whatever the channel and bucket, so a being's learned valence about a perceived signal never
        // aliases a belief about a standing-on feature or a discovered action, even at the same channel and
        // bucket (the section-9 slice-3 aliasing seam, closed by construction).
        for ch in 0..8u16 {
            for bk in 0..8i64 {
                assert_ne!(
                    being_signal_subject(ch, bk).0,
                    feature_subject(ch, bk).0,
                    "disjoint from the feature band at the same channel and bucket"
                );
            }
        }
        // The band markers: the being-signal band sets bit 60 and clears bit 61; the feature band clears bit
        // 60; the sequence band sets bit 61. So the three bands are pairwise disjoint by their marker bits.
        assert_eq!(
            being_signal_subject(0, 0).0 & (1 << 60),
            1 << 60,
            "being-signal band marker (bit 60) set"
        );
        assert_eq!(
            being_signal_subject(u16::MAX, i64::MAX).0 & (1 << 61),
            0,
            "being-signal band leaves bit 61 clear, disjoint from the sequence band"
        );
        assert_eq!(
            feature_subject(0, 0).0 & (1 << 60),
            0,
            "feature band leaves bit 60 clear"
        );
        assert_eq!(
            SEQUENCE_SUBJECT_BASE & (1 << 61),
            1 << 61,
            "sequence band sets bit 61"
        );
        assert!(
            being_signal_subject(u16::MAX, i64::MAX).0 < (1 << 63),
            "stays below the reserved-high landmark ids, exactly as the feature and sequence bands do"
        );
    }

    #[test]
    fn perceive_being_signal_gates_on_the_perceivers_threshold_and_occlusion() {
        use civsim_physics::laws::{DiscriminationLaw, ResponseLaw};
        // A perceiver whose channel transduces linearly at unit gain, with a detection threshold of 10.
        let transduction = ChannelTransduction {
            response: ResponseLaw::Linear,
            gain: Fixed::ONE,
            shape: Fixed::ZERO,
            discrimination: DiscriminationLaw::AbsoluteStep,
            step: Fixed::ONE,
            threshold: Fixed::from_int(10),
        };
        let cap = Fixed::from_int(1_000_000);
        // A strong, unoccluded reach (spread 100, no optical depth) clears the threshold and mints the
        // subject in the being-signal band, keyed on the channel and the discriminated bucket.
        let clear = Reach {
            spread: Fixed::from_int(100),
            optical_depth: Fixed::ZERO,
        };
        let expected_bucket = sense(Fixed::from_int(100), &transduction, cap)
            .expect("an unoccluded strong signal is sensed")
            .bucket;
        assert_eq!(
            perceive_being_signal(clear, 5, &transduction, cap),
            Some(being_signal_subject(5, expected_bucket)),
            "an above-threshold reach mints the being-signal subject on its channel and bucket"
        );
        // The SAME emission behind a strongly-absorbing medium (a large optical depth) is attenuated below
        // the threshold by the Beer-Lambert transmittance, so occlusion emerges and the emitter is not
        // perceived, with no authored line-of-sight rule.
        let occluded = Reach {
            spread: Fixed::from_int(100),
            optical_depth: Fixed::from_int(20),
        };
        assert_eq!(
            perceive_being_signal(occluded, 5, &transduction, cap),
            None,
            "a strongly occluded emitter falls below the perceiver's threshold"
        );
        // A faint emission below the threshold is not perceived either.
        let faint = Reach {
            spread: Fixed::from_int(1),
            optical_depth: Fixed::ZERO,
        };
        assert_eq!(
            perceive_being_signal(faint, 5, &transduction, cap),
            None,
            "a sub-threshold emission is not perceived"
        );
    }

    #[test]
    fn perceive_being_magnitude_returns_the_transduced_activation_and_shares_the_founder_threshold()
    {
        use civsim_physics::laws::{DiscriminationLaw, ResponseLaw};
        // The creature-tier magnitude perceiver reads the SAME reach through the SAME transduction the founder
        // subject perceiver uses, but returns the transduced ACTIVATION (the creature's pull weight) rather than
        // the belief subject. Same perceptibility gate (threshold, occlusion), no belief.
        let transduction = ChannelTransduction {
            response: ResponseLaw::Linear,
            gain: Fixed::ONE,
            shape: Fixed::ZERO,
            discrimination: DiscriminationLaw::AbsoluteStep,
            step: Fixed::ONE,
            threshold: Fixed::from_int(10),
        };
        let cap = Fixed::from_int(1_000_000);
        let clear = Reach {
            spread: Fixed::from_int(100),
            optical_depth: Fixed::ZERO,
        };
        // The returned magnitude is exactly the transduced activation the shared `sense` produces, so the
        // creature and the founder read one magnitude from one reach and differ only in what they key on.
        let expected_activation = sense(Fixed::from_int(100), &transduction, cap)
            .expect("an unoccluded strong signal is sensed")
            .activation;
        assert_eq!(
            perceive_being_magnitude(clear, &transduction, cap),
            Some(expected_activation),
            "an above-threshold reach returns the transduced activation, the creature's pull weight"
        );
        // Occlusion emerges the same way: a strongly absorbing medium attenuates it below threshold, no percept.
        let occluded = Reach {
            spread: Fixed::from_int(100),
            optical_depth: Fixed::from_int(20),
        };
        assert_eq!(
            perceive_being_magnitude(occluded, &transduction, cap),
            None,
            "a strongly occluded emitter falls below the creature's threshold (no pull)"
        );
        // A sub-threshold emission is not perceived, so it contributes no magnitude.
        let faint = Reach {
            spread: Fixed::from_int(1),
            optical_depth: Fixed::ZERO,
        };
        assert_eq!(
            perceive_being_magnitude(faint, &transduction, cap),
            None,
            "a sub-threshold emission returns no magnitude"
        );
        // INTERMEDIATE attenuation (0 < transmittance < 1): a partly-absorbing medium reduces the magnitude
        // below the unoccluded value but keeps it above threshold, so the attenuation arithmetic itself is
        // exercised (not just the full-pass and total-kill extremes). A wrong-sign or mis-scaled transmittance
        // would land on a different activation and fail here.
        let partial = Reach {
            spread: Fixed::from_int(100),
            optical_depth: Fixed::ONE, // transmittance = exp(-1) ~ 0.368, so magnitude ~ 36.8, above threshold
        };
        let partial_mag = perceive_being_magnitude(partial, &transduction, cap)
            .expect("a partly-attenuated strong signal is still above threshold");
        assert!(
            partial_mag < expected_activation && partial_mag > Fixed::ZERO,
            "partial occlusion reduces the magnitude below the unoccluded value but keeps it perceptible"
        );
        // It equals the transduced value of the explicitly attenuated magnitude, pinning the exact arithmetic.
        let attenuated = Fixed::from_int(100)
            .checked_mul((-Fixed::ONE).exp())
            .unwrap();
        assert_eq!(
            perceive_being_magnitude(partial, &transduction, cap),
            sense(attenuated, &transduction, cap).map(|p| p.activation),
            "the perceived magnitude is the transduced spread times the Beer-Lambert transmittance"
        );
        // The magnitude perceiver and the founder subject perceiver share ONE perceptibility gate: on any reach
        // where one perceives, so does the other, and where one is gated out, so is the other (they differ only
        // in what they key on, the activation versus the channel-and-bucket subject).
        for reach in [clear, occluded, faint, partial] {
            assert_eq!(
                perceive_being_magnitude(reach, &transduction, cap).is_some(),
                perceive_being_signal(reach, 5, &transduction, cap).is_some(),
                "the creature magnitude and the founder subject share the same perceptibility gate"
            );
        }
    }

    #[test]
    fn the_being_directed_gradients_point_by_the_learned_belief_and_are_zero_without_it() {
        let subject = being_signal_subject(5, 3);
        let here = Coord3::ground(0, 0);
        let east = Coord3::ground(4, 0);
        let perceived = [(east, subject)];
        // A perceiver with NO belief reads a zero gradient on both poles: founder-inert until a belief forms
        // and an evolved weight lifts it (Principle 9).
        let blank = Mind::new(StableId(20), Fixed::ONE);
        assert_eq!(
            being_avoidance_gradient(&blank, here, &perceived, &params()),
            (Fixed::ZERO, Fixed::ZERO),
            "no avoidance without a harm belief"
        );
        assert_eq!(
            being_attraction_gradient(&blank, here, &perceived, &params()),
            (Fixed::ZERO, Fixed::ZERO),
            "no attraction without a reward belief"
        );
        // A perceiver that has learned this signal HARMS it: the avoidance gradient points WEST (away from
        // the emitter to its east, negative x), and the attraction gradient stays zero.
        let mut fearful = Mind::new(StableId(21), Fixed::ONE);
        let harm_calib = HarmLearningCalib::dev_default();
        for _ in 0..5 {
            fearful.consider(
                subject,
                HARM_ATTR,
                [HARMS, BENIGN],
                HARMS,
                harm_calib.observation_weight(),
                fearful.id,
            );
        }
        assert_eq!(
            fearful.belief(subject, HARM_ATTR, &params()),
            Some(HARMS),
            "committed the harm belief"
        );
        let (ax, ay) = being_avoidance_gradient(&fearful, here, &perceived, &params());
        assert!(
            ax < Fixed::ZERO,
            "avoidance points away from the due-east emitter (west, negative x)"
        );
        assert_eq!(
            ay,
            Fixed::ZERO,
            "no north-south component for a due-east emitter"
        );
        assert_eq!(
            being_attraction_gradient(&fearful, here, &perceived, &params()),
            (Fixed::ZERO, Fixed::ZERO),
            "no attraction while the belief is harm"
        );
        // A perceiver that has learned this signal REWARDS it: the attraction gradient points EAST (toward
        // the emitter, positive x), the predation pole; the sign a being ACTS on is still the evolved weight.
        let mut hungry = Mind::new(StableId(22), Fixed::ONE);
        let reward_calib = RewardLearningCalib::dev_default();
        for _ in 0..5 {
            hungry.consider(
                subject,
                REWARD_ATTR,
                [REWARDS, NEUTRAL],
                REWARDS,
                reward_calib.observation_weight(),
                hungry.id,
            );
        }
        assert_eq!(
            hungry.belief(subject, REWARD_ATTR, &params()),
            Some(REWARDS),
            "committed the reward belief"
        );
        let (bx, by) = being_attraction_gradient(&hungry, here, &perceived, &params());
        assert!(
            bx > Fixed::ZERO,
            "attraction points toward the due-east emitter (positive x), the predation pole"
        );
        assert_eq!(
            by,
            Fixed::ZERO,
            "no north-south component for a due-east emitter"
        );
    }

    #[test]
    fn the_creature_being_direction_is_belief_free_magnitude_graded_and_points_toward_the_emitter()
    {
        // Creatures-react arc, mechanism B3. The mind-less creature path takes only positions and the
        // creature's own perceived magnitudes, no Mind, no belief, no valence.
        let here = Coord3::ground(0, 0);
        let east = Coord3::ground(4, 0);
        // Empty perception (or an all-sub-threshold read) is a zero direction: a creature that perceives
        // nothing has no pull, the true null before any weight acts.
        assert_eq!(
            creature_being_direction(here, &[]),
            (Fixed::ZERO, Fixed::ZERO),
            "no perception, no pull"
        );
        assert_eq!(
            creature_being_direction(here, &[(east, Fixed::ZERO)]),
            (Fixed::ZERO, Fixed::ZERO),
            "a zero-magnitude (sub-threshold) emitter contributes nothing"
        );
        // The direction points TOWARD the due-east emitter (positive x, zero y). The sign the creature ACTS
        // on (approach vs flee) is the controller weight's, not this percept's: this always points toward.
        let (px, py) = creature_being_direction(here, &[(east, Fixed::from_ratio(1, 4))]);
        assert!(
            px > Fixed::ZERO,
            "points toward the east emitter (positive x)"
        );
        assert_eq!(
            py,
            Fixed::ZERO,
            "no north-south component for a due-east emitter"
        );
        // Magnitude grading: a stronger signal from the SAME position pulls harder (a larger x component),
        // below the saturation bound.
        let (weak_x, _) = creature_being_direction(here, &[(east, Fixed::from_ratio(1, 8))]);
        let (strong_x, _) = creature_being_direction(here, &[(east, Fixed::from_ratio(1, 2))]);
        assert!(
            strong_x > weak_x,
            "a stronger perceived magnitude pulls harder (B3 magnitude grading)"
        );
        // Distance is NOT double-counted: the perceived magnitude is the sole distance factor (the reach
        // already attenuated it), so at EQUAL magnitude a near and a far due-east emitter pull IDENTICALLY.
        // A closer emitter pulls harder only because its perceived magnitude is higher (attenuation, upstream).
        let near = Coord3::ground(2, 0);
        let far = Coord3::ground(8, 0);
        assert_eq!(
            creature_being_direction(here, &[(near, Fixed::from_ratio(1, 4))]),
            creature_being_direction(here, &[(far, Fixed::from_ratio(1, 4))]),
            "equal magnitude, different distance: identical pull (distance enters only through magnitude)"
        );
        // A DIAGONAL emitter yields a diagonal direction, both components toward it (positive), so the percept
        // is not an x-axis artefact of the earlier cases.
        let ne = Coord3::ground(3, 4);
        let (nx, ny) = creature_being_direction(here, &[(ne, Fixed::from_ratio(1, 4))]);
        assert!(
            nx > Fixed::ZERO && ny > Fixed::ZERO,
            "points toward a north-east emitter on both axes"
        );
        // Multi-emitter summation: an emitter due-east and one due-north of equal magnitude sum to a north-east
        // pull (both components positive and, by symmetry, equal).
        let north = Coord3::ground(4, 0); // reuse magnitude symmetry via mirrored positions
        let east4 = Coord3::ground(0, 4);
        let (mx, my) = creature_being_direction(
            here,
            &[
                (north, Fixed::from_ratio(1, 4)),
                (east4, Fixed::from_ratio(1, 4)),
            ],
        );
        assert!(
            mx > Fixed::ZERO && my > Fixed::ZERO,
            "two orthogonal emitters sum to a diagonal pull"
        );
        assert_eq!(
            mx, my,
            "symmetric orthogonal emitters give an equal-component diagonal"
        );
        // A co-located emitter has no direction and is skipped (no divide, no pull).
        assert_eq!(
            creature_being_direction(here, &[(here, Fixed::ONE)]),
            (Fixed::ZERO, Fixed::ZERO),
            "a co-located emitter contributes no direction"
        );
        // Saturation preserves DIRECTION (length-clamp, not per-component): a very strong diagonal pull clamps
        // to unit length while keeping its 45-degree heading (equal components), never rotated toward an axis.
        let diag = Coord3::ground(1, 1);
        let (sx, sy) = creature_being_direction(here, &[(diag, Fixed::from_int(100))]);
        assert_eq!(
            sx, sy,
            "a saturated diagonal pull keeps equal components (direction preserved)"
        );
        assert!(
            sx > Fixed::ZERO,
            "the saturated diagonal still points toward the emitter"
        );
        // and its length is at the unit bound (within a fixed-point tie), never past it.
        let len2 = sx
            .checked_mul(sx)
            .unwrap()
            .saturating_add(sy.checked_mul(sy).unwrap());
        assert!(
            len2 <= Fixed::ONE,
            "the clamped length never exceeds the unit disc"
        );
    }

    #[test]
    fn the_harm_eligibility_trace_credits_each_remembered_subject_by_its_decayed_eligibility() {
        let calib = HarmLearningCalib::dev_default();
        let recent = being_signal_subject(3, 1);
        let older = being_signal_subject(3, 2);
        let mut trace = EligibilityTrace::new();
        trace.record(older); // perceived first
        trace.decay(Fixed::from_ratio(1, 2)); // and one tick older now, decayed to 1/2
        trace.record(recent); // perceived this tick, full eligibility one
        let base = calib.observation_weight();
        // On a HARM tick, each remembered being-signal earns a harm observation weighted by base times its
        // decayed eligibility: the just-perceived signal earns full credit, the distal one less, so a
        // predator-approach cue perceived a tick before the harm is still credited (the lagged association).
        let obs = being_signal_trace_observations(&trace, true, Fixed::ONE, &calib);
        assert_eq!(obs.len(), 2, "one observation per remembered being-signal");
        assert!(
            obs.iter().all(|o| o.toward == HARMS),
            "a harm tick credits toward HARMS"
        );
        let recent_obs = obs
            .iter()
            .find(|o| o.subject == recent)
            .expect("recent credited");
        let older_obs = obs
            .iter()
            .find(|o| o.subject == older)
            .expect("older credited");
        assert_eq!(
            recent_obs.weight, base,
            "the just-perceived signal earns full credit"
        );
        assert_eq!(
            older_obs.weight,
            base.checked_mul(Fixed::from_ratio(1, 2)).unwrap(),
            "the distal signal earns decayed credit"
        );
        assert!(
            older_obs.weight < recent_obs.weight,
            "a more distal perception earns less credit"
        );
        // On a harm-free tick the same subjects are credited toward BENIGN.
        let benign = being_signal_trace_observations(&trace, false, Fixed::ONE, &calib);
        assert!(
            benign.iter().all(|o| o.toward == BENIGN),
            "a harm-free tick credits toward BENIGN"
        );
        // An empty trace credits nothing (the byte-neutral opt-out state).
        assert!(
            being_signal_trace_observations(&EligibilityTrace::new(), true, Fixed::ONE, &calib)
                .is_empty(),
            "an empty trace credits nothing"
        );
    }

    #[test]
    fn a_being_signal_earns_a_reward_observation_pointed_by_the_reward_bit() {
        // Keystone step 2 (the predation pole): a perceived being-signal correlated with the being's own
        // reward bit mints one observation toward REWARDS on a reward tick, NEUTRAL otherwise, on the SAME
        // being-signal subject the harm core uses (disjoint frames by attribute), so a perceived being carries
        // both a harm belief and a reward belief that emerge from the perceiver's own outcomes.
        let calib = RewardLearningCalib::dev_default();
        let rewarded = being_signal_reward_observation(2, 7, true, Fixed::ONE, &calib);
        assert_eq!(
            rewarded.toward, REWARDS,
            "a reward tick points toward REWARDS"
        );
        assert_eq!(
            rewarded.subject,
            being_signal_subject(2, 7),
            "keyed on channel and bucket in the being-signal band"
        );
        assert!(rewarded.weight > Fixed::ZERO, "positive evidence weight");
        let neutral = being_signal_reward_observation(2, 7, false, Fixed::ONE, &calib);
        assert_eq!(
            neutral.toward, NEUTRAL,
            "a reward-free tick points toward NEUTRAL"
        );
        // The reward core and the harm core share the SAME subject (one perceived signal, two disjoint belief
        // frames), so a being's harm and reward beliefs about a signal never split across subjects.
        let hcalib = HarmLearningCalib::dev_default();
        let harm = being_signal_observation(2, 7, true, Fixed::ONE, &hcalib);
        assert_eq!(
            rewarded.subject, harm.subject,
            "harm and reward beliefs share the being-signal subject"
        );
    }

    #[test]
    fn the_being_signal_reward_core_mints_inline_like_the_material_reward_core() {
        // The being-signal reward core is fed through the same INLINE reward mint the material reward learner
        // uses: for the same channel, bucket, reward bit, and plasticity, the valence direction and evidence
        // weight match reward_observations, so no being path is special-cased; only the SUBJECT differs (the
        // disjoint being-signal band), so a being-signal reward belief never aliases a material one.
        let calib = RewardLearningCalib::dev_default();
        let amount = Fixed::from_int(2);
        let bucket = feature_bucket(amount, calib.feature_granularity);
        let mat = reward_observations(true, &[amount], Fixed::ONE, &calib, 0);
        let sig = being_signal_reward_observation(0, bucket, true, Fixed::ONE, &calib);
        assert_eq!(
            sig.toward, mat[0].toward,
            "same valence direction (REWARDS)"
        );
        assert_eq!(
            sig.weight, mat[0].weight,
            "same evidence weight (the shared reserved likelihoods)"
        );
        assert_eq!(
            sig.subject,
            being_signal_subject(0, bucket),
            "minted in the being-signal band"
        );
        assert_ne!(
            sig.subject, mat[0].subject,
            "disjoint from the material-feature reward subject"
        );
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
        assert_ne!(sequence_subject(&[step(3, 6, 0), step(4, 2, 1)]), s); // different target kind (in-envelope)
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
    fn the_hybrid_key_dissolves_the_cap_in_envelope_and_hashes_on_overflow() {
        // R-DEEPTECH-COMPOSE hybrid belief-subject key: the exact widened pack distinguishes every primitive
        // within its widened field (dissolving the old 16-value cap, so 20 and 21 no longer over-merge), and
        // a beyond-envelope sequence (more steps, or a field past its width) mints via the hash sub-band (bit
        // 60), disjoint from the exact pack (bit 60 clear), from Agent A's being-signal band (bit 61 clear),
        // and from the feature band, so an arbitrary conjunction is representable without the common-case
        // collision a pure hash would incur, and it stays deterministic.
        let step = |p: u16, t: i64, q: i64| SequenceStep {
            primitive: p,
            target_bucket: t,
            param_bucket: q,
        };
        // In-envelope: the widened primitive field distinguishes ids the old 4-bit cap merged.
        let a = sequence_subject(&[step(20, 1, 0)]);
        let b = sequence_subject(&[step(21, 1, 0)]);
        assert_ne!(
            a, b,
            "primitives 20 and 21 mint distinct subjects (the cap is dissolved)"
        );
        // Both in-envelope subjects are EXACT (bit 60 clear).
        assert_eq!(a.0 & SEQ_HASH_MARKER, 0);
        assert_eq!(b.0 & SEQ_HASH_MARKER, 0);
        // Over-envelope by STEP COUNT: five steps exceed SEQ_MAX_STEPS, so it hashes (bit 60 set).
        let long = [
            step(1, 0, 0),
            step(2, 0, 0),
            step(3, 0, 0),
            step(4, 0, 0),
            step(5, 0, 0),
        ];
        let h = sequence_subject(&long);
        assert_eq!(
            h.0 & SEQ_HASH_MARKER,
            SEQ_HASH_MARKER,
            "an over-length sequence hashes"
        );
        // Over-envelope by FIELD WIDTH: a primitive past the 6-bit width hashes.
        let wide = sequence_subject(&[step(1000, 0, 0)]);
        assert_eq!(
            wide.0 & SEQ_HASH_MARKER,
            SEQ_HASH_MARKER,
            "a too-wide field hashes"
        );
        // The other two width disjuncts of the envelope check, exercised so no field's overflow branch is
        // untested: a target bucket past the 3-bit width, and a param bucket past the 3-bit width, each
        // hashes. These are the fields the widening narrowed most (three bits), so their overflow is the
        // one most reached in practice.
        let wide_target = sequence_subject(&[step(1, SEQ_TARGET_MAX as i64 + 1, 0)]);
        assert_eq!(
            wide_target.0 & SEQ_HASH_MARKER,
            SEQ_HASH_MARKER,
            "a target bucket past its width hashes"
        );
        let wide_param = sequence_subject(&[step(1, 0, SEQ_PARAM_MAX as i64 + 1)]);
        assert_eq!(
            wide_param.0 & SEQ_HASH_MARKER,
            SEQ_HASH_MARKER,
            "a param bucket past its width hashes"
        );
        // A field exactly AT its width stays exact (the boundary is inclusive), so the hash path is entered
        // only past the envelope, never one step early.
        let at_edge = sequence_subject(&[step(
            SEQ_PRIMITIVE_MAX as u16,
            SEQ_TARGET_MAX as i64,
            SEQ_PARAM_MAX as i64,
        )]);
        assert_eq!(
            at_edge.0 & SEQ_HASH_MARKER,
            0,
            "every field exactly at its width still packs exactly"
        );
        // The hash sub-band stays in the sequence band (bits 62 and 61 set) and below the landmark ids.
        assert_eq!(h.0 & SEQUENCE_SUBJECT_BASE, SEQUENCE_SUBJECT_BASE);
        assert!(h.0 < 1u64 << 63);
        // Disjoint from Agent A's being-signal band (bits 62+60, bit 61 clear): the hash sets bit 61.
        assert_ne!(
            h.0 & (1u64 << 61),
            0,
            "the sequence-hash band sets bit 61, being-signal leaves it clear"
        );
        // Deterministic: the same over-envelope sequence hashes to the same subject.
        assert_eq!(sequence_subject(&long), h);
        // Disjoint from the exact pack: an in-envelope and an over-envelope sequence never collide.
        assert_ne!(h, a);
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
