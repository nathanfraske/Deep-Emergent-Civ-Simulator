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

//! # civsim-sim: the staged simulation layer
//!
//! This crate holds the parts the runbook lists as buildable now at the structural
//! and determinism level, with tuned behaviour held until the owner sets the
//! reserved numbers:
//!
//! - [`calibration`]: the calibration manifest loader. Every reserved value loads
//!   as a fail-loud sentinel, so a system that reads an unset required value errors
//!   rather than running on a fabricated default. This is the operational form of
//!   the prime directive that the project never fabricates a value (runbook
//!   section 4, design Principle 11).
//! - [`conservation`]: the conserved-projection registry of design Part 58. What
//!   must be conserved is not a fixed list but a registry each two-tier subsystem
//!   declares for itself, so a future subsystem is covered the moment it registers
//!   its own projection.
//! - [`lod`]: a minimal two-tier world (individuals and aggregate pools) with
//!   promotion, demotion, merge, and split, used to exercise conservation and
//!   referential integrity (design Parts 11, 54, 58).
//! - [`substrate`]: data-driven substrate definitions with round-trip loading, the
//!   schema-and-loader plumbing the runbook says is buildable now while the content
//!   stays data.
//! - [`tom`]: recursive theory of mind (design Part 37, the resolved R-TOM-UPDATE
//!   work). The evidence engine run recursively on whether a target believes a thing,
//!   with a typed anti-projection guarantee (a nested frame admits only access evidence
//!   about its target) and a data-driven access-channel registry, so a false belief
//!   and a seen-through lie come from one mechanism without a closed enum of evidence.
//! - [`agent`]: a minimal [`agent::Mind`] composing belief and theory of mind into the
//!   epistemic core of an agent: it perceives, forms and revises beliefs, models other
//!   minds, is deceived, and sees lies through, all deterministically. It does not yet
//!   decide or act (design Part 8); that half is gated on the systems and reserved
//!   numbers the gating notes name.
//! - [`world`]: the runtime spine (design Parts 4, 57). A [`world::World`] owns the
//!   minds, the event log, a clock, and the calibrations, and a serial deterministic
//!   tick applies a batch of stimuli to the minds in one canonical order. The parallel
//!   command scheduler is held for its open determinism design (R-CMD-ORDER,
//!   R-REDUCE-ORDER); this is the serial form everything else can run on now.

pub mod agent;
pub mod axiom;
pub mod calibration;
pub mod conservation;
pub mod decision;
pub mod dialogue;
pub mod evidence;
pub mod genome;
pub mod language;
pub mod lod;
pub mod primes;
pub mod race;
pub mod substrate;
pub mod tom;
pub mod value;
pub mod world;

pub use agent::{AccessObs, Mind, SharedBelief};
pub use axiom::{
    bounded_confidence_mean, confidence_weighted_mean, confidence_weighted_variance, enculturate,
    entrenchment_threshold, inherit_seed, Appraisal, Axiom, AxiomAxisDef, AxiomAxisId,
    AxiomAxisRegistry, AxiomDomainId, DomainDef, DomainRegistry, EpistemicStance, EvidenceRing,
    EvidenceTag, IntrinsicBeliefs, SourceModeDef, SourceModeId,
};
pub use calibration::{CalibrationError, CalibrationManifest, Profile, ReservedValue};
pub use conservation::{ConservationError, ConservationRegistry};
pub use decision::{ActionDef, ActionId, Behaviour, Consideration, Curve, DriveDef, DriveId};
pub use dialogue::{
    conversation_of, ContentGateError, ContentRef, Conversation, EffectSign, FelicityCond,
    ForceEffectDef, ForceEffectId, ForceFloor, ForceKind, Move, MoveKindDef, MoveKindId,
    MoveRegistry, ResolvedBand, MOVE_EVENT_KIND,
};
pub use evidence::{AttrKindId, EvidenceRef, InferenceFrame, InferenceParams};
pub use genome::{
    Allele, AlleleState, BuildChannel, Channel, CognitionChannel, DominanceKind, DominanceMode,
    GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme, Genome, Haplotype,
    HybridOutcome, ImbuedChannel, Incompatibility, IncompatibilityKind, IncompatibilityTable,
    LifeHistoryChannel, LinkageGroup, ReproductionMode, SchemeId, TraitId,
};
pub use language::{
    ArticulationSubstrate, ConceptId, FeatureDimDef, FeatureDimId, FeatureValueDef, FeatureValueId,
    FormSegment, FormSystem, LanguageParams, Lexicon, ProductionModalityDef, ProductionModalityId,
    Word,
};
pub use lod::{Individual, Pool, TwoTierWorld};
pub use primes::{nsm_concept_ids, nsm_gloss, nsm_prime_count, nsm_primes, Prime};
pub use race::{BandSpec, Race};
pub use substrate::Substrate;
pub use tom::{
    detects_deception, AccessChannelDef, AccessChannelId, AccessChannelRegistry, AccessWeights,
    EvidenceOrder, NestedFrame, ProjectionRejected,
};
pub use value::{
    conflict_pressure, cross_race_distance, euclidean_distance, project_to_etic, value_distance,
    EmicProjection, EticAxisId, EticProfile, EticSubstrate, GraphEdge, GroundMetric, RaceId,
    RaceProjection, StructureKind, ValueAxisId, ValueProfile, ValueStructure,
};
pub use world::{GossipParams, PlaceId, Stimulus, TickInput, Trace, World};
