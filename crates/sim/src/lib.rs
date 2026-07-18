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
//! - [`civsim_bio::calibration`]: the calibration manifest loader. Every reserved value loads
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
//! - [`civsim_bio::tom`]: recursive theory of mind (design Part 37, the resolved R-TOM-UPDATE
//!   work). The evidence engine run recursively on whether a target believes a thing,
//!   with a typed anti-projection guarantee (a nested frame admits only access evidence
//!   about its target) and a data-driven access-channel registry, so a false belief
//!   and a seen-through lie come from one mechanism without a closed enum of evidence.
//! - [`civsim_bio::agent`]: a minimal [`civsim_bio::agent::Mind`] composing belief and theory of mind into the
//!   epistemic core of an agent: it perceives, forms and revises beliefs, models other
//!   minds, is deceived, and sees lies through, all deterministically. It does not yet
//!   decide or act (design Part 8); that half is gated on the systems and reserved
//!   numbers the gating notes name.
//! - [`transmission`]: the knowledge-transmission substrate (design Parts 20, 23, 25, 41). A
//!   culture copies opaque content-addressed designs; the copy drifts a fidelity-scaled amount
//!   and an under-practised design is lost, exposing the drift and loss rates
//!   `compose.transmission_stability` derives from. The transmit and loss kernels are race-blind,
//!   reading per-race perception and memory rather than an authored per-race fidelity table
//!   (Principle 9), and the transmitted unit is a `DesignId`, never a technique enum (Principle 4).
//! - [`world`]: the runtime spine (design Parts 4, 57). A [`world::World`] owns the
//!   minds, the event log, a clock, and the calibrations, and a serial deterministic
//!   tick applies a batch of stimuli to the minds in one canonical order. The parallel
//!   command scheduler is held for its open determinism design (R-CMD-ORDER,
//!   R-REDUCE-ORDER); this is the serial form everything else can run on now.

pub mod absence;
pub mod affect;
pub mod affordance_percept;
pub mod astro;
pub mod axiom;
pub mod base_rates;
pub mod biosphere;
pub mod body;
pub mod breeding;
pub mod census;
pub mod clock;
pub mod conservation;
pub mod contact_transfer;
pub mod contact_wound;
pub mod controller;
pub mod conviction_experience;
pub mod conviction_percept;
pub mod dawn_harness;
pub mod decompose;
pub mod deeptime;
pub mod demography;
pub mod derive_gate;
pub mod dialogue;
pub mod discovery;
pub mod edibility;
pub mod environ;
pub mod epoch;
pub mod evolve;
pub mod forward_model;
pub mod genesis;
pub mod geodynamics;
pub mod geodynamics_surface;
pub mod giants;
pub mod homeostasis;
pub mod institution;
pub mod insult;
pub mod langdist;
pub mod langmod;
pub mod language;
pub mod learn;
pub mod located;
pub mod locomotion;
pub mod lod;
pub mod material;
pub mod material_percept;
pub mod medium;
pub mod moons;
pub mod morphogen;
pub mod nonlocal_coupling;
pub mod orbit;
pub mod perceivable_feature;
pub mod percept;
pub mod perception_percept;
pub mod perception_reach;
pub mod personality;
pub mod physiology;
pub mod planet;
pub mod planetary_assembly;
pub mod planetary_system;
pub mod planning;
pub mod primes;
pub mod profile;
pub mod race;
pub mod runner;
pub mod scenario;
pub mod secular;
pub mod semantics;
pub mod sensorium;
pub mod smallbody;
pub mod stellar;
pub mod stellar_evolution;
pub mod stocks;
pub mod substrate;
pub mod surface_drivers;
pub mod surface_transport;
pub mod tectonic_regime;
pub mod trace;
pub mod transmission;
pub mod typology;
pub mod unified_provenance;
pub mod value;
pub mod world;

pub use absence::{
    absence_window, characteristic_lifespan, AbsenceScheduleDef, AbsenceScheduleId, AbsenceStage,
    LIFESPAN_HAZARD_THRESHOLD,
};
pub use affect::{AffectAxisId, AffectState, AppraisalBinding, DriveAppraisal};
pub use axiom::{
    bounded_confidence_mean, confidence_weighted_mean, confidence_weighted_variance, enculturate,
    enculturation_pull_rate, entrenchment_threshold, inherit_seed, Appraisal, Axiom, AxiomAxisDef,
    AxiomAxisId, AxiomAxisRegistry, AxiomDomainId, DomainDef, DomainRegistry, EpistemicStance,
    EvidenceRing, EvidenceTag, IntrinsicBeliefs, RingCapacityLaw, SourceModeDef, SourceModeId,
};
pub use base_rates::{RaceBaseRateRegistry, RaceBaseRates};
pub use body::{
    apply_insult, strike, Body, BodyParams, BodyPart, DamageModeDef, DamageModeId,
    DamageModeRegistry, FluidDef, FluidKindId, FluidPool, FluidRegistry, FunctionId, Insult,
    MeasureKind, PartCondition, TissueLayer, TissueMaterial, TissueMaterialId, TissueRegistry,
    WoundRecord,
};
pub use breeding::{
    fisher_select_step, sex_ratio_selection_coeff, AssignmentRule, BreedingSystem,
    BreedingSystemId, BreedingSystemRegistry, CompatibilityRule, SexClass,
};
pub use census::{
    effective_size_classes, effective_size_sex, effective_size_var, ReproductiveCensus,
    ReproductiveMoments,
};
pub use civsim_bio::agent::{AccessObs, Mind, RetentionLaw, SharedBelief};
pub use civsim_bio::belief::{
    instantiate_strength, BeliefKey, BeliefParams, BeliefPool, FacetStrength, PrevailingBelief,
};
pub use civsim_bio::calibration::{CalibrationError, CalibrationManifest, Profile, ReservedValue};
pub use civsim_bio::decision::{
    ActionDef, ActionId, Behaviour, Consideration, Curve, DriveDef, DriveId, InputId,
};
pub use civsim_bio::evidence::{
    aggregate_diffusion_rate, derive_aggregate_diffusion_rate, good_weight, AttrKindId,
    EvidenceRef, InferenceFrame, InferenceParams,
};
pub use civsim_bio::genome::{
    append_controller_block, append_morphogen_block, append_scalar_channel, Allele, AlleleState,
    BuildChannel, Channel, CognitionChannel, CompositionAxisId, ControllerParamId, DominanceKind,
    DominanceMode, GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme, Genome,
    Haplotype, HybridOutcome, ImbuedChannel, Incompatibility, IncompatibilityKind,
    IncompatibilityTable, LifeHistoryChannel, LinkageGroup, MorphogenParamId, ReproductionMode,
    SchemeId, ToleranceAxisId, TraitId,
};
pub use clock::{PlaybackDriver, SimClock, Steppable, LIFE_CADENCE_TICKS, YEARS_PER_GENERATION};
pub use conservation::{ConservationError, ConservationRegistry};
pub use controller::{
    forage_taxis_weights, taxis_move_weights, weight_count, Controller, ControllerDecision,
    ControllerLayout, ForageGains,
};
pub use decompose::{
    CombineMode, DecomposerDriver, DecomposerDriverRegistry, DecomposerKernelId,
    DecomposerStockField,
};
pub use demography::{hazard_age, AgeHistogram};
pub use dialogue::{
    conversation_of, ContentGateError, ContentRef, Conversation, EffectSign, FelicityCond,
    ForceEffectDef, ForceEffectId, ForceFloor, ForceKind, Move, MoveKindDef, MoveKindId,
    MoveRegistry, ResolvedBand, MOVE_EVENT_KIND,
};
pub use evolve::{
    controller_gene_set, episode_survival, evolve, evolve_forage_controller, full_episode_survival,
    homeostatic_coefficient, reserve_conflict_survival, selection_gradient, EvolveParams,
    EvolveReport,
};
pub use homeostasis::{
    AffordanceDef, AffordanceId, AffordanceParam, AffordanceRegistry, Homeostasis,
    HomeostaticAxisDef, HomeostaticAxisId, HomeostaticRegistry,
};
pub use morphogen::{
    express_program, grow, morphogen_gene_set, AxisSpec, MorphogenProgram, Segment, Structure,
};
// The function-law dispatch types the affordance and body APIs read, re-exported so an external caller
// can name the capability context an [`AffordanceRegistry::afforded`] or a [`body::BodyParams`] needs
// (emergent-anatomy step one; the mechanism lives in `civsim_compose`).
pub use civsim_bio::mate_choice::{choose, genetic_distance, realised_fitness, MatePreference};
pub use civsim_bio::tom::{
    detects_deception, AccessChannelDef, AccessChannelId, AccessChannelRegistry, AccessWeights,
    EvidenceOrder, NestedFrame, ProjectionRejected,
};
pub use civsim_compose::{
    derive_capabilities, CapabilityCaps, CapabilityKernel, CapabilityRefs, CapabilityVector,
    FunctionLawDef, FunctionLawId, FunctionLawRegistry,
};
pub use dawn_harness::{
    arm_dawn_languages, build_dawn_runner, DawnPeoples, EmbodimentGenesis, LanguageGenesis,
};
pub use institution::{
    crystallization_order, crystallize, emit_undertaking, institution_distance, norm_fires,
    recognize, signature_distance, weighted_tanimoto, AggregateInstitution, Atom, AttributeSel,
    ConditionExpr, Conditions, CoordinationObservation, CrystallizationParams, DecisionPropensity,
    Deontic, EticDescriptor, FeatureSignature, FunctionAxisDef, FunctionAxisId, FunctionRegistry,
    FunctionVec, Institution, Norm, NormType, Predicate, RecognitionTemplate, Role, RoleId,
    TemplateId, TemplateLibrary, STRUCTURAL_FEATURES,
};
pub use langdist::{
    distance_component_weights, language_distance, lexical_distance, phonological_distance,
    ComponentWeights,
};
pub use langmod::{
    acquisition_split, articulated_geometry, capability_gate, capability_halves,
    form_system_from_values, phoneme_priors, producible_values, CapabilityGate, FormSystemError,
};
pub use language::{
    ArticulationSubstrate, ConceptId, FeatureDimDef, FeatureDimId, FeatureValueDef, FeatureValueId,
    FormSegment, FormSystem, L2AcquisitionLaw, LangKnowledge, LanguageParams, Lexicon,
    Linearization, ProductionModalityDef, ProductionModalityId, SalienceDecayLaw, Word,
};
pub use locomotion::{LocomotionParams, ResourceField, Terrain, Walker};
pub use lod::{Individual, Pool, TwoTierWorld};
pub use personality::{
    age_personality, plasticity_at, PersonalityProfile, PersonalityRegistry, TraitAxisId, TraitDef,
    TraitInstance,
};
pub use primes::{nsm_concept_ids, nsm_gloss, nsm_prime_count, nsm_primes, Prime};
pub use race::{Articulation, BandSpec, Race};
pub use scenario::{Direction, MagicPosture, RacePosture, Scenario, ScenarioError, ScenarioMeta};
pub use semantics::{
    concept_thresholds, substrate_quantization, Concept, ConceptThresholds, SemanticSubstrate,
    QUANTIZATION_DIVISOR,
};
pub use sensorium::{SenseChannelId, Sensorium};
pub use substrate::Substrate;
pub use trace::{
    corroding_salience, mortality_implication_weight, organic_salience, TraceImplicationSpec,
    TraceKindDef, TraceKindId, TraceKindRegistry, TransformKernelId, TransformKind,
};
pub use transmission::{
    copy_drift, copy_fidelity, drift_similarity_radius, erode_and_cull, is_stabilised,
    stability_span, transmit, transmit_draw, DesignHistory, DesignId, Knowledge,
    TransmissionParams,
};
pub use typology::{
    grammar_parse_cost, information_weights, sample_profile, tilted_weights, typology_distance,
    validate as validate_typology, wals_seed, HarmonyBias, HarmonyModel, TiltParams, TypologyError,
    TypologyParamDef, TypologyParamId, TypologyParams, TypologyPrior, TypologyProfile,
    TypologyRegistry, TypologyValueDef, TypologyValueId, ValueMetric,
};
pub use value::{
    conflict_pressure, cross_race_distance, euclidean_distance, incommensurability_ceiling,
    project_to_etic, project_to_etic_with_loss, value_distance, EmicProjection, EticAxisId,
    EticProfile, EticSubstrate, GraphEdge, GroundMetric, RaceId, RaceProjection, StructureKind,
    ValueAxisId, ValueProfile, ValueStructure,
};
pub use world::{
    build_etic_substrate, GossipParams, PlaceId, ReproductionParams, Stimulus, TickInput, Trace,
    World,
};
