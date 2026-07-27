// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Generic derive-first routing for one claim-scoped law premise.
//!
//! A route starts from opaque admitted premise capabilities and exact
//! derivation-rule capabilities. Two independent algorithms determine whether
//! the requested role and content follow. Failure to derive is not itself
//! irreducibility. The route can reach an irreducible-protocol state only when
//! a separate capability binds completeness of the exact derivation catalog.
//!
//! Buckingham Pi, Gap Law with its typed Chaos branch, Residual Law, and
//! collision-checked slot uniqueness are distinct typed bindings. Receipt
//! presence alone never mints a premise. This module remains non-authorizing
//! until a production target, complete semantic rule catalog, live canaries,
//! and an enrolled authority receipt exist.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use super::{derived_relation, ClaimScopedPremiseCapability, LawClaimIdentity, PremiseKey};
#[cfg(test)]
use super::{PhysicalContentIdentity, SemanticRoleIdentity};
use crate::canonical::stellar_birth_species::physical_registry;
use civsim_units::digest::sha256;

const RESULT_SCHEMA_ID: &str = "civsim.planet.law-premise-derive-first-route.v1";
const PRODUCER_ID: &str = "civsim.planet.law-premise-route.forward-closure-producer.v1";
const WATCHDOG_ID: &str = "civsim.planet.law-premise-route.dependency-watchdog.v1";

const MAX_SEED_COUNT: usize = 4_096;
const MAX_RULE_COUNT: usize = 4_096;
const MAX_RULE_PREMISES: usize = 256;
const MAX_TOTAL_RULE_PREMISES: usize = 65_536;
const MAX_WORK_UNITS: usize = 2_000_000;
const MAX_CANONICAL_BYTES: usize = 8_388_608;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct DerivationRuleIdentity([u8; 32]);

/// Opaque capability for one exact semantic derivation rule.
///
/// Production has no constructor in this slice. Future claim-specific
/// authority may construct it only after both semantic implementations and
/// their applicability and validity receipts agree.
#[derive(Debug, Clone, PartialEq, Eq)]
struct VerifiedPremiseDerivationRule {
    claim_identity: LawClaimIdentity,
    rule_identity: DerivationRuleIdentity,
    premises: Vec<PremiseKey>,
    conclusion: PremiseKey,
    semantic_producer_receipt_sha256: [u8; 32],
    semantic_watchdog_receipt_sha256: [u8; 32],
    applicability_receipt_sha256: [u8; 32],
    validity_receipt_sha256: [u8; 32],
    capability_sha256: [u8; 32],
    _seal: DerivationRuleSeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DerivationRuleSeal;

/// Separate proof that an exact claim and target saw the complete admitted
/// derivation catalog. An empty or unsuccessful search cannot construct this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DerivationCoverageCapability {
    claim_identity: LawClaimIdentity,
    target: PremiseKey,
    derivation_catalog_sha256: [u8; 32],
    producer_receipt_sha256: [u8; 32],
    watchdog_receipt_sha256: [u8; 32],
    capability_sha256: [u8; 32],
    _seal: DerivationCoverageSeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DerivationCoverageSeal;

/// Buckingham-Pi disposition for an irreducible semantic premise.
///
/// Some premise meanings are not magnitudes. They still require independent
/// proof that Buckingham Pi is inapplicable instead of silently bypassing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuckinghamPiDisposition {
    Applicable {
        variable_basis_receipt_sha256: [u8; 32],
        producer_receipt_sha256: [u8; 32],
        watchdog_receipt_sha256: [u8; 32],
        residual_group_count: u32,
    },
    SemanticallyInapplicable {
        producer_receipt_sha256: [u8; 32],
        watchdog_receipt_sha256: [u8; 32],
    },
}

/// Typed Chaos Protocol branch carried by the Gap Law binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChaosDisposition {
    Nondynamical {
        producer_inapplicability_receipt_sha256: [u8; 32],
        watchdog_inapplicability_receipt_sha256: [u8; 32],
    },
    Dynamical {
        regime_partition_producer_receipt_sha256: [u8; 32],
        regime_partition_watchdog_receipt_sha256: [u8; 32],
        transition_law_receipt_sha256: [u8; 32],
    },
}

/// Complete protocol binding for one irreducible survivor.
///
/// The seal has no production minter in this slice. Even a structurally valid
/// test value produces a non-authorizing route report rather than a premise
/// capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IrreducibleProtocolCapability {
    claim_identity: LawClaimIdentity,
    target: PremiseKey,
    derivation_coverage_capability_sha256: [u8; 32],
    buckingham_pi: BuckinghamPiDisposition,
    gap_law_receipt_sha256: [u8; 32],
    chaos: ChaosDisposition,
    residual_law_producer_receipt_sha256: [u8; 32],
    residual_law_watchdog_receipt_sha256: [u8; 32],
    residual_slot_identity: [u8; 32],
    unique_slot_producer_receipt_sha256: [u8; 32],
    unique_slot_watchdog_receipt_sha256: [u8; 32],
    owner_admission_receipt_sha256: [u8; 32],
    capability_sha256: [u8; 32],
    _seal: IrreducibleProtocolSeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IrreducibleProtocolSeal;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PremiseAdmissionRouteInput {
    claim_identity: LawClaimIdentity,
    target: PremiseKey,
    admitted_seeds: Vec<ClaimScopedPremiseCapability>,
    admitted_rules: Vec<VerifiedPremiseDerivationRule>,
    derivation_coverage: Option<DerivationCoverageCapability>,
    irreducible_protocol: Option<IrreducibleProtocolCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DerivationWitness {
    target: PremiseKey,
    seed_capability_sha256: Vec<[u8; 32]>,
    rule_capability_sha256: Vec<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OpenDerivationFrontier {
    target: PremiseKey,
    reachable: Vec<PremiseKey>,
    unresolved_dependencies: Vec<PremiseKey>,
    derivation_catalog_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PremiseAdmissionDecision {
    Derived(DerivationWitness),
    OpenDerivationFrontier(OpenDerivationFrontier),
    IrreducibleProtocolRequired {
        target: PremiseKey,
        derivation_coverage_capability_sha256: [u8; 32],
    },
    IrreducibleProtocolStructurallyBound {
        target: PremiseKey,
        derivation_coverage_capability_sha256: [u8; 32],
        irreducible_protocol_capability_sha256: [u8; 32],
    },
}

impl PremiseAdmissionDecision {
    const fn id(&self) -> &'static str {
        match self {
            Self::Derived(_) => "derived",
            Self::OpenDerivationFrontier(_) => "open_derivation_frontier",
            Self::IrreducibleProtocolRequired { .. } => "irreducible_protocol_required",
            Self::IrreducibleProtocolStructurallyBound { .. } => {
                "irreducible_protocol_structurally_bound"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PremiseAdmissionEvaluation {
    claim_identity: LawClaimIdentity,
    target: PremiseKey,
    derivation_catalog_sha256: [u8; 32],
    seed_count: u32,
    rule_count: u32,
    decision: PremiseAdmissionDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PremiseAdmissionCheckerOutput {
    evaluation: PremiseAdmissionEvaluation,
    canonical_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PremiseAdmissionRouteReport {
    evaluation: PremiseAdmissionEvaluation,
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
}

impl PremiseAdmissionRouteReport {
    const fn decision_id(&self) -> &'static str {
        self.evaluation.decision.id()
    }

    const fn premise_admission_authority(&self) -> bool {
        false
    }

    const fn species_membership_authority(&self) -> bool {
        false
    }

    const fn global_derivation_coverage(&self) -> bool {
        false
    }

    const fn authority_effect(&self) -> &'static str {
        "none"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PremiseAdmissionRefusal {
    SealedSourceUnavailable,
    InvalidClaimIdentity,
    InvalidTarget,
    SeedCapacityExceeded,
    InvalidSeed,
    DuplicateSeed,
    RuleCapacityExceeded,
    InvalidRule,
    DuplicateRule,
    RulePremiseCapacityExceeded,
    TotalRulePremiseCapacityExceeded,
    RuleCapabilityDigestMismatch,
    DerivationCoverageCapabilityInvalid,
    IrreducibleProtocolWithoutCoverage,
    IrreducibleProtocolCapabilityInvalid,
    DuplicateProtocolReceipt,
    WorkLimitExceeded,
    CanonicalByteCapacityExceeded,
    CheckerDisagreement,
    RepositoryFrontierInvalid,
}

impl PremiseAdmissionRefusal {
    const fn id(self) -> &'static str {
        match self {
            Self::SealedSourceUnavailable => "sealed_source_unavailable",
            Self::InvalidClaimIdentity => "invalid_claim_identity",
            Self::InvalidTarget => "invalid_target",
            Self::SeedCapacityExceeded => "seed_capacity_exceeded",
            Self::InvalidSeed => "invalid_seed",
            Self::DuplicateSeed => "duplicate_seed",
            Self::RuleCapacityExceeded => "rule_capacity_exceeded",
            Self::InvalidRule => "invalid_rule",
            Self::DuplicateRule => "duplicate_rule",
            Self::RulePremiseCapacityExceeded => "rule_premise_capacity_exceeded",
            Self::TotalRulePremiseCapacityExceeded => "total_rule_premise_capacity_exceeded",
            Self::RuleCapabilityDigestMismatch => "rule_capability_digest_mismatch",
            Self::DerivationCoverageCapabilityInvalid => "derivation_coverage_capability_invalid",
            Self::IrreducibleProtocolWithoutCoverage => {
                "irreducible_protocol_without_derivation_coverage"
            }
            Self::IrreducibleProtocolCapabilityInvalid => "irreducible_protocol_capability_invalid",
            Self::DuplicateProtocolReceipt => "duplicate_protocol_receipt",
            Self::WorkLimitExceeded => "work_limit_exceeded",
            Self::CanonicalByteCapacityExceeded => "canonical_byte_capacity_exceeded",
            Self::CheckerDisagreement => "checker_disagreement",
            Self::RepositoryFrontierInvalid => "repository_frontier_invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::canonical::stellar_birth_species) struct RepositoryPremiseAdmissionFrontier {
    pub(in crate::canonical::stellar_birth_species) schema_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) producer_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) watchdog_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) producer_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) watchdog_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) derived_route_count: u32,
    pub(in crate::canonical::stellar_birth_species) derived_claim_identity_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) derived_role_identity_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) derived_content_identity_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) derived_decision_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) current_descriptor_role_count: u32,
    pub(in crate::canonical::stellar_birth_species) current_relation_target_count: u32,
    pub(in crate::canonical::stellar_birth_species) current_constraint_law_count: u32,
    pub(in crate::canonical::stellar_birth_species) next_target_decision_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) derivation_frontier_complete: bool,
    pub(in crate::canonical::stellar_birth_species) irreducible_protocol_started: bool,
    pub(in crate::canonical::stellar_birth_species) premise_admission_authority: bool,
    pub(in crate::canonical::stellar_birth_species) species_membership_authority: bool,
    pub(in crate::canonical::stellar_birth_species) authority_effect: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::canonical::stellar_birth_species) struct RepositoryPremiseAdmissionFrontierError {
    pub(in crate::canonical::stellar_birth_species) code: &'static str,
}

fn inspect_premise_admission_route(
    input: &PremiseAdmissionRouteInput,
) -> Result<PremiseAdmissionRouteReport, PremiseAdmissionRefusal> {
    let produced = producer::route(input);
    let watched = watchdog::route(input);
    match (produced, watched) {
        (Ok(produced), Ok(watched))
            if produced.evaluation == watched.evaluation
                && produced.canonical_bytes == watched.canonical_bytes =>
        {
            let producer_result_sha256 = sha256(&produced.canonical_bytes);
            let watchdog_result_sha256 = sha256(&watched.canonical_bytes);
            Ok(PremiseAdmissionRouteReport {
                evaluation: produced.evaluation,
                producer_result_sha256,
                watchdog_result_sha256,
            })
        }
        (Err(produced), Err(watched)) if produced == watched => Err(produced),
        _ => Err(PremiseAdmissionRefusal::CheckerDisagreement),
    }
}

pub(in crate::canonical::stellar_birth_species) fn repository_premise_admission_frontier(
) -> Result<RepositoryPremiseAdmissionFrontier, RepositoryPremiseAdmissionFrontierError> {
    let capability = derived_relation::repository_derived_relation_capability().ok_or(
        RepositoryPremiseAdmissionFrontierError {
            code: PremiseAdmissionRefusal::SealedSourceUnavailable.id(),
        },
    )?;
    let input = PremiseAdmissionRouteInput {
        claim_identity: capability.claim_identity,
        target: capability.key,
        admitted_seeds: vec![capability],
        admitted_rules: Vec::new(),
        derivation_coverage: None,
        irreducible_protocol: None,
    };
    let report = inspect_premise_admission_route(&input)
        .map_err(|error| RepositoryPremiseAdmissionFrontierError { code: error.id() })?;
    if report.decision_id() != "derived"
        || report.premise_admission_authority()
        || report.species_membership_authority()
        || report.global_derivation_coverage()
        || report.authority_effect() != "none"
    {
        return Err(RepositoryPremiseAdmissionFrontierError {
            code: PremiseAdmissionRefusal::RepositoryFrontierInvalid.id(),
        });
    }

    let physical = physical_registry::repository_physical_registry_frontier().map_err(|_| {
        RepositoryPremiseAdmissionFrontierError {
            code: PremiseAdmissionRefusal::RepositoryFrontierInvalid.id(),
        }
    })?;
    let next_target_decision_id = next_target_decision(
        physical.vocabulary_descriptor_role_count,
        physical.vocabulary_relation_target_count,
        physical.vocabulary_constraint_law_count,
    );

    Ok(RepositoryPremiseAdmissionFrontier {
        schema_id: RESULT_SCHEMA_ID,
        producer_id: PRODUCER_ID,
        watchdog_id: WATCHDOG_ID,
        producer_result_sha256: report.producer_result_sha256,
        watchdog_result_sha256: report.watchdog_result_sha256,
        derived_route_count: 1,
        derived_claim_identity_sha256: input.claim_identity.0,
        derived_role_identity_sha256: input.target.role.0,
        derived_content_identity_sha256: input.target.content.0,
        derived_decision_id: report.decision_id(),
        current_descriptor_role_count: physical.vocabulary_descriptor_role_count,
        current_relation_target_count: physical.vocabulary_relation_target_count,
        current_constraint_law_count: physical.vocabulary_constraint_law_count,
        next_target_decision_id,
        derivation_frontier_complete: false,
        irreducible_protocol_started: false,
        premise_admission_authority: false,
        species_membership_authority: false,
        authority_effect: "none",
    })
}

const fn next_target_decision(
    descriptor_role_count: u32,
    relation_target_count: u32,
    constraint_law_count: u32,
) -> &'static str {
    if descriptor_role_count == 0 {
        "no_admitted_semantic_target_role"
    } else if relation_target_count == 0 {
        "no_admitted_semantic_target_content"
    } else if constraint_law_count == 0 {
        "no_admitted_premise_derivation_rule"
    } else {
        "next_target_not_bound"
    }
}

fn valid_key(key: PremiseKey) -> bool {
    key.role.0 != [0; 32] && key.content.0 != [0; 32]
}
