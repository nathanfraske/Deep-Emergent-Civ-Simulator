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
mod profile_protocol;
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

/// Complete request for one repository-enrolled irreducible semantic target.
///
/// The target is opaque. The request binds source custody, executed
/// applicability and validity evidence, one residual-slot name, and the owner
/// review record. This route never accepts a magnitude or a familiar type
/// selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::canonical::stellar_birth_species) struct TheoryProfileAdmissionRequest {
    pub(in crate::canonical::stellar_birth_species) claim_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) role_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) content_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) profile_input_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) source_custody_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) applicability_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) validity_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) residual_slot_id: String,
    /// Residual slots already occupied by separately admitted theory profiles.
    ///
    /// The sealed floor contributes its own slots inside the inspector. This
    /// list keeps a later profile from colliding with an earlier admitted
    /// profile merely because that profile is not itself a floor leaf.
    pub(in crate::canonical::stellar_birth_species) occupied_profile_slots: Vec<String>,
    pub(in crate::canonical::stellar_birth_species) owner_admission_record: String,
}

/// Executed derive-first and irreducible-protocol evidence for one semantic
/// profile target. The report carries no species capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::canonical::stellar_birth_species) struct TheoryProfileAdmissionEvidence {
    pub(in crate::canonical::stellar_birth_species) target_claim_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) target_role_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) target_content_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) seed_count: u32,
    pub(in crate::canonical::stellar_birth_species) rule_count: u32,
    pub(in crate::canonical::stellar_birth_species) derivation_catalog_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) repository_catalog_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) protocol_producer_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) protocol_watchdog_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) open_producer_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) open_watchdog_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) final_producer_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) final_watchdog_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) derivation_coverage_capability_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) irreducible_protocol_capability_sha256:
        [u8; 32],
    pub(in crate::canonical::stellar_birth_species) derivation_exhaustion_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) buckingham_pi_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) gap_law_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) chaos_protocol_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) residual_law_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) residual_slot_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) residual_slot_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) owner_admission_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) independent_watchdog_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) decision_id: &'static str,
}

const PROFILE_GAP_DOMAIN: &[u8] = b"civsim.planet.theory-profile-gap-law.v1";
const PROFILE_SLOT_IDENTITY_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-residual-slot-identity.v1";
const PROFILE_EXHAUSTION_DOMAIN: &[u8] = b"civsim.planet.theory-profile-derivation-exhaustion.v1";
const PROFILE_PI_PAIR_DOMAIN: &[u8] = b"civsim.planet.theory-profile-buckingham-pi-pair.v1";
const PROFILE_CHAOS_PAIR_DOMAIN: &[u8] = b"civsim.planet.theory-profile-chaos-pair.v1";
const PROFILE_RESIDUAL_PAIR_DOMAIN: &[u8] = b"civsim.planet.theory-profile-residual-pair.v1";
const PROFILE_SLOT_PAIR_DOMAIN: &[u8] = b"civsim.planet.theory-profile-residual-slot-pair.v1";
const PROFILE_OWNER_PAIR_DOMAIN: &[u8] = b"civsim.planet.theory-profile-owner-admission-pair.v2";
const PROFILE_WATCHDOG_DOMAIN: &[u8] = b"civsim.planet.theory-profile-independent-watchdog.v1";

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

pub(in crate::canonical::stellar_birth_species) fn inspect_theory_profile_admission(
    request: &TheoryProfileAdmissionRequest,
) -> Result<TheoryProfileAdmissionEvidence, &'static str> {
    let fixed_digests = [
        request.claim_identity,
        request.role_identity,
        request.content_identity,
        request.profile_input_sha256,
        request.source_custody_sha256,
        request.applicability_receipt_sha256,
        request.validity_receipt_sha256,
    ];
    if fixed_digests.contains(&[0; 32])
        || fixed_digests
            .iter()
            .enumerate()
            .any(|(index, digest)| fixed_digests[index + 1..].contains(digest))
        || request.residual_slot_id.trim().is_empty()
        || request.residual_slot_id.len() > 192
        || !request.residual_slot_id.is_ascii()
        || request
            .occupied_profile_slots
            .iter()
            .any(|slot| slot.trim().is_empty() || slot.len() > 192 || !slot.is_ascii())
        || request
            .occupied_profile_slots
            .iter()
            .enumerate()
            .any(|(index, slot)| request.occupied_profile_slots[index + 1..].contains(slot))
        || request.owner_admission_record.trim().is_empty()
        || request.owner_admission_record.len() > 192
        || !request.owner_admission_record.is_ascii()
    {
        return Err("invalid_theory_profile_admission_request");
    }
    let target = PremiseKey {
        role: super::SemanticRoleIdentity(request.role_identity),
        content: super::PhysicalContentIdentity(request.content_identity),
    };
    let claim_identity = LawClaimIdentity(request.claim_identity);
    let eps0 = derived_relation::repository_derived_relation_capability()
        .ok_or("sealed_derived_seed_unavailable")?;
    let mut input = PremiseAdmissionRouteInput {
        claim_identity,
        target,
        admitted_seeds: Vec::new(),
        admitted_rules: Vec::new(),
        derivation_coverage: None,
        irreducible_protocol: None,
    };
    let open_report =
        inspect_premise_admission_route(&input).map_err(|_| "derive_first_route_refused")?;
    let PremiseAdmissionDecision::OpenDerivationFrontier(open) = &open_report.evaluation.decision
    else {
        return Err("theory_profile_was_not_an_open_derivation_frontier");
    };
    if !open.reachable.is_empty() || !open.unresolved_dependencies.is_empty() {
        return Err("unexpected_theory_profile_derivation_reachability");
    }
    let derivation_catalog_sha256 = open.derivation_catalog_sha256;
    let floor = crate::canonical::sealed_absolute_physics_floor()
        .map_err(|_| "sealed_floor_unavailable_for_slot_check")?;
    let mut occupied_slots = floor
        .entries()
        .filter_map(|entry| floor.receipt(&entry.id))
        .map(|receipt| receipt.residual_slot.trim().to_owned())
        .collect::<Vec<_>>();
    occupied_slots.extend(
        request
            .occupied_profile_slots
            .iter()
            .map(|slot| slot.trim().to_owned()),
    );
    let protocol_pair = profile_protocol::inspect(&profile_protocol::ProfileProtocolInput {
        claim_identity: request.claim_identity,
        role_identity: request.role_identity,
        content_identity: request.content_identity,
        profile_input_sha256: request.profile_input_sha256,
        source_custody_sha256: request.source_custody_sha256,
        applicability_receipt_sha256: request.applicability_receipt_sha256,
        validity_receipt_sha256: request.validity_receipt_sha256,
        derivation_catalog_sha256,
        repository_seeds: vec![profile_protocol::CatalogSeed {
            claim_identity: eps0.claim_identity.0,
            role_identity: eps0.key.role.0,
            content_identity: eps0.key.content.0,
            capability_sha256: eps0.capability_sha256,
        }],
        repository_rules: Vec::new(),
        residual_slot_id: request.residual_slot_id.clone(),
        occupied_residual_slots: occupied_slots,
        owner_admission_record: request.owner_admission_record.clone(),
    })?;
    if protocol_pair.assessment.repository_seed_count != 1
        || protocol_pair.assessment.repository_rule_count != 0
        || protocol_pair.assessment.target_scoped_seed_count != open_report.evaluation.seed_count
        || protocol_pair.assessment.target_scoped_rule_count != open_report.evaluation.rule_count
    {
        return Err("profile_protocol_catalog_mismatch");
    }
    let mut coverage = DerivationCoverageCapability {
        claim_identity,
        target,
        derivation_catalog_sha256,
        producer_receipt_sha256: protocol_pair.producer_receipts.coverage_sha256,
        watchdog_receipt_sha256: protocol_pair.watchdog_receipts.coverage_sha256,
        capability_sha256: [0; 32],
        _seal: DerivationCoverageSeal,
    };
    let produced_coverage = producer::coverage_capability_digest(&coverage);
    let watched_coverage = watchdog::coverage_capability_digest(&coverage);
    if produced_coverage == [0; 32] || produced_coverage != watched_coverage {
        return Err("derivation_coverage_checker_disagreement");
    }
    coverage.capability_sha256 = produced_coverage;
    input.derivation_coverage = Some(coverage);
    let coverage_report =
        inspect_premise_admission_route(&input).map_err(|_| "coverage_route_refused")?;
    if !matches!(
        coverage_report.evaluation.decision,
        PremiseAdmissionDecision::IrreducibleProtocolRequired { .. }
    ) {
        return Err("irreducible_protocol_was_not_required");
    }

    let residual_slot = request.residual_slot_id.trim();
    let residual_slot_identity = profile_digest(
        PROFILE_SLOT_IDENTITY_DOMAIN,
        &[residual_slot.as_bytes(), &request.content_identity],
    );
    let gap_law_receipt_sha256 = profile_digest(
        PROFILE_GAP_DOMAIN,
        &[
            &protocol_pair.producer_receipts.gap_law_sha256,
            &protocol_pair.watchdog_receipts.gap_law_sha256,
            &protocol_pair.producer_result_sha256,
            &protocol_pair.watchdog_result_sha256,
        ],
    );
    let owner_admission_receipt_sha256 = profile_digest(
        PROFILE_OWNER_PAIR_DOMAIN,
        &[
            &protocol_pair.producer_receipts.owner_admission_sha256,
            &protocol_pair.watchdog_receipts.owner_admission_sha256,
        ],
    );
    let mut protocol = IrreducibleProtocolCapability {
        claim_identity,
        target,
        derivation_coverage_capability_sha256: coverage.capability_sha256,
        buckingham_pi: BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256: protocol_pair.producer_receipts.buckingham_pi_sha256,
            watchdog_receipt_sha256: protocol_pair.watchdog_receipts.buckingham_pi_sha256,
        },
        gap_law_receipt_sha256,
        chaos: ChaosDisposition::Nondynamical {
            producer_inapplicability_receipt_sha256: protocol_pair
                .producer_receipts
                .chaos_protocol_sha256,
            watchdog_inapplicability_receipt_sha256: protocol_pair
                .watchdog_receipts
                .chaos_protocol_sha256,
        },
        residual_law_producer_receipt_sha256: protocol_pair.producer_receipts.residual_law_sha256,
        residual_law_watchdog_receipt_sha256: protocol_pair.watchdog_receipts.residual_law_sha256,
        residual_slot_identity,
        unique_slot_producer_receipt_sha256: protocol_pair.producer_receipts.residual_slot_sha256,
        unique_slot_watchdog_receipt_sha256: protocol_pair.watchdog_receipts.residual_slot_sha256,
        owner_admission_receipt_sha256,
        capability_sha256: [0; 32],
        _seal: IrreducibleProtocolSeal,
    };
    let produced_protocol = producer::protocol_capability_digest(&protocol);
    let watched_protocol = watchdog::protocol_capability_digest(&protocol);
    if produced_protocol == [0; 32] || produced_protocol != watched_protocol {
        return Err("irreducible_protocol_checker_disagreement");
    }
    protocol.capability_sha256 = produced_protocol;
    input.irreducible_protocol = Some(protocol);
    let final_report =
        inspect_premise_admission_route(&input).map_err(|_| "irreducible_route_refused")?;
    if !matches!(
        final_report.evaluation.decision,
        PremiseAdmissionDecision::IrreducibleProtocolStructurallyBound { .. }
    ) {
        return Err("irreducible_protocol_not_structurally_bound");
    }

    let derivation_exhaustion_receipt_sha256 = profile_digest(
        PROFILE_EXHAUSTION_DOMAIN,
        &[
            &open_report.producer_result_sha256,
            &open_report.watchdog_result_sha256,
            &coverage.capability_sha256,
            &derivation_catalog_sha256,
        ],
    );
    let buckingham_pi_receipt_sha256 = profile_digest(
        PROFILE_PI_PAIR_DOMAIN,
        &[
            &protocol_pair.producer_receipts.buckingham_pi_sha256,
            &protocol_pair.watchdog_receipts.buckingham_pi_sha256,
        ],
    );
    let chaos_protocol_receipt_sha256 = profile_digest(
        PROFILE_CHAOS_PAIR_DOMAIN,
        &[
            &protocol_pair.producer_receipts.chaos_protocol_sha256,
            &protocol_pair.watchdog_receipts.chaos_protocol_sha256,
        ],
    );
    let residual_law_receipt_sha256 = profile_digest(
        PROFILE_RESIDUAL_PAIR_DOMAIN,
        &[
            &protocol_pair.producer_receipts.residual_law_sha256,
            &protocol_pair.watchdog_receipts.residual_law_sha256,
        ],
    );
    let residual_slot_receipt_sha256 = profile_digest(
        PROFILE_SLOT_PAIR_DOMAIN,
        &[
            &residual_slot_identity,
            &protocol_pair.producer_receipts.residual_slot_sha256,
            &protocol_pair.watchdog_receipts.residual_slot_sha256,
        ],
    );
    let independent_watchdog_receipt_sha256 = profile_digest(
        PROFILE_WATCHDOG_DOMAIN,
        &[
            &final_report.watchdog_result_sha256,
            &final_report.producer_result_sha256,
            &protocol.capability_sha256,
            &request.profile_input_sha256,
            &protocol_pair.watchdog_result_sha256,
        ],
    );
    let combined = [
        derivation_exhaustion_receipt_sha256,
        buckingham_pi_receipt_sha256,
        gap_law_receipt_sha256,
        chaos_protocol_receipt_sha256,
        residual_law_receipt_sha256,
        residual_slot_receipt_sha256,
        owner_admission_receipt_sha256,
        independent_watchdog_receipt_sha256,
    ];
    if combined.contains(&[0; 32])
        || combined
            .iter()
            .enumerate()
            .any(|(index, digest)| combined[index + 1..].contains(digest))
    {
        return Err("collapsed_theory_profile_protocol_receipts");
    }

    Ok(TheoryProfileAdmissionEvidence {
        target_claim_identity: request.claim_identity,
        target_role_identity: request.role_identity,
        target_content_identity: request.content_identity,
        seed_count: open_report.evaluation.seed_count,
        rule_count: open_report.evaluation.rule_count,
        derivation_catalog_sha256,
        repository_catalog_sha256: protocol_pair.assessment.repository_catalog_sha256,
        protocol_producer_result_sha256: protocol_pair.producer_result_sha256,
        protocol_watchdog_result_sha256: protocol_pair.watchdog_result_sha256,
        open_producer_result_sha256: open_report.producer_result_sha256,
        open_watchdog_result_sha256: open_report.watchdog_result_sha256,
        final_producer_result_sha256: final_report.producer_result_sha256,
        final_watchdog_result_sha256: final_report.watchdog_result_sha256,
        derivation_coverage_capability_sha256: coverage.capability_sha256,
        irreducible_protocol_capability_sha256: protocol.capability_sha256,
        derivation_exhaustion_receipt_sha256,
        buckingham_pi_receipt_sha256,
        gap_law_receipt_sha256,
        chaos_protocol_receipt_sha256,
        residual_law_receipt_sha256,
        residual_slot_identity,
        residual_slot_receipt_sha256,
        owner_admission_receipt_sha256,
        independent_watchdog_receipt_sha256,
        decision_id: final_report.decision_id(),
    })
}

fn profile_digest(domain: &[u8], fields: &[&[u8]]) -> [u8; 32] {
    let mut bytes = domain.to_vec();
    for (index, field) in fields.iter().enumerate() {
        bytes.extend_from_slice(
            &u16::try_from(index + 1)
                .expect("bounded profile field index")
                .to_be_bytes(),
        );
        bytes.extend_from_slice(
            &u64::try_from(field.len())
                .expect("bounded profile field length")
                .to_be_bytes(),
        );
        bytes.extend_from_slice(field);
    }
    sha256(&bytes)
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
        irreducible_protocol_started: true,
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
