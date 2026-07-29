//! One claim-local confining interaction profile.
//!
//! The profile admits one compact noncommutative gauge-sector grammar only
//! after the complete derive-first and irreducible route. From that admitted
//! categorical root, two independent implementations reconstruct an adjoint
//! connection-carrier family, its bracket self-action, covariant conservation,
//! and an invariant-only asymptotic boundary.
//!
//! The confinement boundary is an explicit scoped premise. This module does
//! not claim a mathematical proof of four-dimensional confinement, choose a
//! group or rank, admit a coupling or transition scale, enumerate matter
//! constituents, or mint an isolated carrier as a species member.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use super::model::{AdmissionRoute, ArtifactIdentity, ArtifactPayload, RootAdmission};
use civsim_units::digest::sha256;
use std::collections::BTreeSet;

pub(super) const PACKET_SCHEMA_ID: &str = "civsim.planet.confining-profile-packet.v1";
pub(super) const PROFILE_SCHEMA_ID: &str =
    "civsim.physical-profile.compact-noncommutative-confining-sector.v1";
pub(super) const RECEIPT_SCHEMA_ID: &str = "civsim.planet.confining-profile-pair-receipt.v1";
pub(super) const CLAIM_ID: &str =
    "planet.interaction-profile.compact-noncommutative-confining-asymptotic-boundary";
pub(super) const PROFILE_ID: &str =
    "interaction-profile.compact-noncommutative-confining-sector.v1";
pub(super) const THEORY_CLASS_ID: &str =
    "compact-noncommutative-local-gauge-sector-with-admitted-confining-boundary";
pub(super) const SECTOR_ID: &str = "compact-noncommutative-local-gauge-sector";
pub(super) const CARRIER_ID: &str =
    "adjoint-connection-carrier-family-with-noncommutative-self-action";
pub(super) const CURVATURE_LAW_ID: &str = "connection-curvature-includes-nonzero-bilinear-bracket";
pub(super) const CONSERVATION_ID: &str = "covariant-current-continuity-with-local-gauss-constraint";
pub(super) const CONFINEMENT_ID: &str =
    "asymptotic-observable-candidates-require-sector-invariance";
pub(super) const APPLICABILITY_ID: &str =
    "separate-regime-capability-must-select-the-confining-domain";
pub(super) const VALIDITY_ID: &str =
    "categorical-only-without-rank-scale-coupling-spectrum-or-support";
pub(super) const ASYMPTOTIC_DISPOSITION_ID: &str =
    "isolated-noninvariant-carriers-are-not-asymptotic-member-candidates";
pub(super) const RESIDUAL_SLOT_ID: &str =
    "planet.interaction-profile.compact-noncommutative-confining-boundary.v1";
pub(super) const OWNER_ADMISSION_RECORD: &str =
    "owner-reviewed-candidate-pr215-2026-07-28-confining-profile-v1";

// @sources: pdg_2024_qcd_review
pub(super) const EVIDENCE_CITATION: &str =
    "Huston, J., Rabbertz, K. and Zanderighi, G., 2024, Quantum Chromodynamics, in Navas, S. et al. (Particle Data Group), Review of Particle Physics, Physical Review D 110, 030001";
pub(super) const EVIDENCE_SOURCE_URL: &str = "https://pdg.lbl.gov/2024/reviews/rpp2024-rev-qcd.pdf";
pub(super) const EVIDENCE_FULL_SHA256_HEX: &str =
    "d06948c331663c2341c3f922e48cd29b4aac83f3bbff71fe3ac8d70cb4f40c51";
pub(super) const EVIDENCE_SLIM_SHA256_HEX: &str =
    "35f0bb289cf8cdaaef4963bab99bc9e45a7cebcf60b766fb9d16b61ea1ac8051";
pub(super) const EVIDENCE_ANCHOR: &str =
    "printed pages 1 and 2, equations 9.1 and 9.2 plus the invariant-combination and carrier-self-interaction paragraphs";

pub(super) const PRODUCER_ID: &str =
    "civsim.planet.confining-profile.forward-entailment-producer.v1";
pub(super) const WATCHDOG_ID: &str =
    "civsim.planet.confining-profile.reverse-obligation-watchdog.v1";
pub(super) const PRODUCER_CANARY_ID: &str = "civsim.planet.confining-profile.producer-canaries.v1";
pub(super) const WATCHDOG_CANARY_ID: &str = "civsim.planet.confining-profile.watchdog-canaries.v1";
pub(super) const ARTIFACT_COUNT: usize = 20;
pub(super) const MEMBER_COUNT: usize = 0;
pub(super) const DERIVE_FIRST_STATUS_ID: &str = "executed_open_frontier";
pub(super) const BUCKINGHAM_PI_STATUS_ID: &str = "semantic_inapplicability_paired";
pub(super) const GAP_LAW_STATUS_ID: &str = "executed_and_bound";
pub(super) const CHAOS_PROTOCOL_STATUS_ID: &str = "nondynamical_inapplicability_paired";
pub(super) const RESIDUAL_LAW_STATUS_ID: &str = "executed_and_bound";
pub(super) const RESIDUAL_SLOT_STATUS_ID: &str = "collision_checked_unique";
pub(super) const MAX_CANONICAL_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StrongProfilePacket {
    pub(super) schema_id: String,
    pub(super) profile_id: String,
    pub(super) theory_class_id: String,
    pub(super) sector_id: String,
    pub(super) carrier_id: String,
    pub(super) curvature_law_id: String,
    pub(super) conservation_id: String,
    pub(super) confinement_id: String,
    pub(super) applicability_id: String,
    pub(super) validity_id: String,
    pub(super) asymptotic_disposition_id: String,
    pub(super) residual_slot_id: String,
    pub(super) occupied_profile_slots: Vec<String>,
    pub(super) owner_admission_record: String,
    pub(super) evidence_citation: String,
    pub(super) evidence_source_url: String,
    pub(super) evidence_full_sha256_hex: String,
    pub(super) evidence_slim_sha256_hex: String,
    pub(super) evidence_anchor: String,
    pub(super) floor_authority: super::model::ReceiptBinding,
    pub(super) global_physical_vocabulary_coverage: bool,
    pub(super) membership_authority: bool,
    pub(super) carrier_species_membership: bool,
    pub(super) confinement_theorem_claim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StrongArtifactCandidate {
    pub(super) identity: ArtifactIdentity,
    pub(super) admission: RootAdmission,
    pub(super) payload: ArtifactPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StrongCheckerOutput {
    pub(super) input_sha256: [u8; 32],
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) candidates: Vec<StrongArtifactCandidate>,
    pub(super) profile_root_identity: ArtifactIdentity,
    pub(super) profile_role_identity: ArtifactIdentity,
    pub(super) sector_identity: ArtifactIdentity,
    pub(super) carrier_identity: ArtifactIdentity,
    pub(super) constraint_law_identity: ArtifactIdentity,
    pub(super) evidence_custody_receipt_sha256: [u8; 32],
    pub(super) curvature_producer_sha256: [u8; 32],
    pub(super) curvature_watchdog_sha256: [u8; 32],
    pub(super) conservation_producer_sha256: [u8; 32],
    pub(super) conservation_watchdog_sha256: [u8; 32],
    pub(super) confinement_producer_sha256: [u8; 32],
    pub(super) confinement_watchdog_sha256: [u8; 32],
    pub(super) applicability_producer_sha256: [u8; 32],
    pub(super) applicability_watchdog_sha256: [u8; 32],
    pub(super) validity_producer_sha256: [u8; 32],
    pub(super) validity_watchdog_sha256: [u8; 32],
    pub(super) asymptotic_producer_sha256: [u8; 32],
    pub(super) asymptotic_watchdog_sha256: [u8; 32],
    pub(super) admission_evidence: super::super::law_premise::TheoryProfileAdmissionEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CanaryEvidence {
    pub(super) transcript_id: &'static str,
    pub(super) case_count: u32,
    pub(super) transcript_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StrongProfileRefusal {
    SealedSourceUnavailable,
    PacketSchemaMismatch,
    ProfileIdentityMismatch,
    TheoryProfileMismatch,
    EvidenceCustodyMismatch,
    FloorBindingMismatch,
    ResidualSlotInventoryMismatch,
    UnsupportedScope,
    ArtifactConstructionFailure,
    ArtifactCountMismatch,
    CanaryFailure,
    CheckerDisagreement,
    PairReceiptMismatch,
}

impl StrongProfileRefusal {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::SealedSourceUnavailable => "sealed_source_unavailable",
            Self::PacketSchemaMismatch => "packet_schema_mismatch",
            Self::ProfileIdentityMismatch => "profile_identity_mismatch",
            Self::TheoryProfileMismatch => "theory_profile_mismatch",
            Self::EvidenceCustodyMismatch => "evidence_custody_mismatch",
            Self::FloorBindingMismatch => "floor_binding_mismatch",
            Self::ResidualSlotInventoryMismatch => "residual_slot_inventory_mismatch",
            Self::UnsupportedScope => "unsupported_scope",
            Self::ArtifactConstructionFailure => "artifact_construction_failure",
            Self::ArtifactCountMismatch => "artifact_count_mismatch",
            Self::CanaryFailure => "canary_failure",
            Self::CheckerDisagreement => "checker_disagreement",
            Self::PairReceiptMismatch => "pair_receipt_mismatch",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StrongProtocolReceipt {
    pub(super) derive_first_status_id: &'static str,
    pub(super) buckingham_pi_status_id: &'static str,
    pub(super) gap_law_status_id: &'static str,
    pub(super) chaos_protocol_status_id: &'static str,
    pub(super) residual_law_status_id: &'static str,
    pub(super) residual_slot_status_id: &'static str,
    pub(super) derivation_exhaustion_sha256: [u8; 32],
    pub(super) buckingham_pi_sha256: [u8; 32],
    pub(super) gap_law_sha256: [u8; 32],
    pub(super) chaos_protocol_sha256: [u8; 32],
    pub(super) residual_law_sha256: [u8; 32],
    pub(super) residual_slot_sha256: [u8; 32],
    pub(super) owner_admission_sha256: [u8; 32],
    pub(super) independent_watchdog_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StrongProfileReceipt {
    pub(super) schema_id: &'static str,
    pub(super) claim_id: &'static str,
    pub(super) profile_id: &'static str,
    pub(super) theory_class_id: &'static str,
    pub(super) residual_slot_id: &'static str,
    pub(super) producer_id: &'static str,
    pub(super) watchdog_id: &'static str,
    pub(super) producer_result_sha256: [u8; 32],
    pub(super) watchdog_result_sha256: [u8; 32],
    pub(super) producer_canary: CanaryEvidence,
    pub(super) watchdog_canary: CanaryEvidence,
    pub(super) profile_root_identity: ArtifactIdentity,
    pub(super) profile_role_identity: ArtifactIdentity,
    pub(super) sector_identity: ArtifactIdentity,
    pub(super) carrier_identity: ArtifactIdentity,
    pub(super) constraint_law_identity: ArtifactIdentity,
    pub(super) artifact_count: u32,
    pub(super) member_count: u32,
    pub(super) evidence_custody_receipt_sha256: [u8; 32],
    pub(super) curvature_producer_sha256: [u8; 32],
    pub(super) curvature_watchdog_sha256: [u8; 32],
    pub(super) conservation_producer_sha256: [u8; 32],
    pub(super) conservation_watchdog_sha256: [u8; 32],
    pub(super) confinement_producer_sha256: [u8; 32],
    pub(super) confinement_watchdog_sha256: [u8; 32],
    pub(super) applicability_producer_sha256: [u8; 32],
    pub(super) applicability_watchdog_sha256: [u8; 32],
    pub(super) validity_producer_sha256: [u8; 32],
    pub(super) validity_watchdog_sha256: [u8; 32],
    pub(super) asymptotic_producer_sha256: [u8; 32],
    pub(super) asymptotic_watchdog_sha256: [u8; 32],
    pub(super) protocol: StrongProtocolReceipt,
    pub(super) global_physical_vocabulary_coverage: bool,
    pub(super) membership_authority: bool,
    pub(super) carrier_species_membership: bool,
    pub(super) conditioned_support_authority: bool,
    pub(super) confinement_theorem_claim: bool,
    pub(super) confining_asymptotic_boundary_admitted: bool,
    pub(super) authority_effect: &'static str,
    pub(super) pair_receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StrongProfileProjection {
    pub(super) candidate_artifacts: Vec<StrongArtifactCandidate>,
    pub(super) receipt: StrongProfileReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct StrongProfileAdmissionCapability {
    claimed_identity: ArtifactIdentity,
    admission: RootAdmission,
    profile_root_identity: ArtifactIdentity,
    pair_receipt_sha256: [u8; 32],
}

impl StrongProfileAdmissionCapability {
    pub(super) const fn claimed_identity(&self) -> ArtifactIdentity {
        self.claimed_identity
    }

    pub(super) const fn admission(&self) -> &RootAdmission {
        &self.admission
    }

    pub(super) const fn profile_root_identity(&self) -> ArtifactIdentity {
        self.profile_root_identity
    }

    pub(super) const fn pair_receipt_sha256(&self) -> [u8; 32] {
        self.pair_receipt_sha256
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AdmittedStrongProfile {
    pub(super) admitted_artifacts: Vec<super::model::AdmittedArtifact>,
    pub(super) receipt: StrongProfileReceipt,
}

// @derives: one generic confining interaction grammar and isolated-carrier refusal <- the claim-local admitted compact noncommutative sector profile and its independent categorical entailment pair
pub(super) fn construct_profile_projection() -> Result<StrongProfileProjection, StrongProfileRefusal>
{
    let producer_packet = producer::sealed_packet()?;
    let watchdog_packet = watchdog::sealed_packet()?;
    if producer_packet != watchdog_packet {
        return Err(StrongProfileRefusal::CheckerDisagreement);
    }
    let produced = producer::inspect_against_sealed(&producer_packet, &producer_packet)?;
    let watched = watchdog::inspect_against_sealed(&watchdog_packet, &watchdog_packet)?;
    if produced != watched
        || produced.candidates.len() != ARTIFACT_COUNT
        || MEMBER_COUNT != 0
        || produced.profile_root_identity.0 == [0; 32]
        || produced.profile_role_identity.0 == [0; 32]
        || produced.sector_identity.0 == [0; 32]
        || produced.carrier_identity.0 == [0; 32]
        || produced.constraint_law_identity.0 == [0; 32]
        || produced.admission_evidence.decision_id != "irreducible_protocol_structurally_bound"
    {
        return Err(StrongProfileRefusal::CheckerDisagreement);
    }
    let semantic_pairs = [
        (
            produced.curvature_producer_sha256,
            produced.curvature_watchdog_sha256,
        ),
        (
            produced.conservation_producer_sha256,
            produced.conservation_watchdog_sha256,
        ),
        (
            produced.confinement_producer_sha256,
            produced.confinement_watchdog_sha256,
        ),
        (
            produced.applicability_producer_sha256,
            produced.applicability_watchdog_sha256,
        ),
        (
            produced.validity_producer_sha256,
            produced.validity_watchdog_sha256,
        ),
        (
            produced.asymptotic_producer_sha256,
            produced.asymptotic_watchdog_sha256,
        ),
    ];
    if semantic_pairs
        .iter()
        .any(|(forward, reverse)| *forward == [0; 32] || *reverse == [0; 32] || forward == reverse)
    {
        return Err(StrongProfileRefusal::CheckerDisagreement);
    }

    let producer_canary =
        producer::canary_evidence_against_sealed(&producer_packet, &producer_packet)?;
    let watchdog_canary =
        watchdog::canary_evidence_against_sealed(&watchdog_packet, &watchdog_packet)?;
    if producer_canary.transcript_id != PRODUCER_CANARY_ID
        || watchdog_canary.transcript_id != WATCHDOG_CANARY_ID
        || producer_canary.transcript_id == watchdog_canary.transcript_id
        || producer_canary.case_count == 0
        || producer_canary.case_count != watchdog_canary.case_count
        || producer_canary.transcript_sha256 == [0; 32]
        || watchdog_canary.transcript_sha256 == [0; 32]
        || producer_canary.transcript_sha256 == watchdog_canary.transcript_sha256
    {
        return Err(StrongProfileRefusal::CanaryFailure);
    }

    let producer_result_sha256 = sha256(&produced.canonical_bytes);
    let watchdog_result_sha256 = sha256(&watched.canonical_bytes);
    let producer_receipt = producer::pair_receipt_digest(
        &produced,
        producer_result_sha256,
        watchdog_result_sha256,
        producer_canary,
        watchdog_canary,
    );
    let watchdog_receipt = watchdog::pair_receipt_digest(
        &watched,
        producer_result_sha256,
        watchdog_result_sha256,
        producer_canary,
        watchdog_canary,
    );
    if producer_receipt == [0; 32] || producer_receipt != watchdog_receipt {
        return Err(StrongProfileRefusal::PairReceiptMismatch);
    }
    let protocol = extract_protocol_receipt(&produced.candidates, produced.profile_root_identity)?;
    let artifact_count = u32::try_from(produced.candidates.len())
        .map_err(|_| StrongProfileRefusal::ArtifactCountMismatch)?;
    Ok(StrongProfileProjection {
        candidate_artifacts: produced.candidates,
        receipt: StrongProfileReceipt {
            schema_id: RECEIPT_SCHEMA_ID,
            claim_id: CLAIM_ID,
            profile_id: PROFILE_ID,
            theory_class_id: THEORY_CLASS_ID,
            residual_slot_id: RESIDUAL_SLOT_ID,
            producer_id: PRODUCER_ID,
            watchdog_id: WATCHDOG_ID,
            producer_result_sha256,
            watchdog_result_sha256,
            producer_canary,
            watchdog_canary,
            profile_root_identity: produced.profile_root_identity,
            profile_role_identity: produced.profile_role_identity,
            sector_identity: produced.sector_identity,
            carrier_identity: produced.carrier_identity,
            constraint_law_identity: produced.constraint_law_identity,
            artifact_count,
            member_count: 0,
            evidence_custody_receipt_sha256: produced.evidence_custody_receipt_sha256,
            curvature_producer_sha256: produced.curvature_producer_sha256,
            curvature_watchdog_sha256: produced.curvature_watchdog_sha256,
            conservation_producer_sha256: produced.conservation_producer_sha256,
            conservation_watchdog_sha256: produced.conservation_watchdog_sha256,
            confinement_producer_sha256: produced.confinement_producer_sha256,
            confinement_watchdog_sha256: produced.confinement_watchdog_sha256,
            applicability_producer_sha256: produced.applicability_producer_sha256,
            applicability_watchdog_sha256: produced.applicability_watchdog_sha256,
            validity_producer_sha256: produced.validity_producer_sha256,
            validity_watchdog_sha256: produced.validity_watchdog_sha256,
            asymptotic_producer_sha256: produced.asymptotic_producer_sha256,
            asymptotic_watchdog_sha256: produced.asymptotic_watchdog_sha256,
            protocol,
            global_physical_vocabulary_coverage: false,
            membership_authority: false,
            carrier_species_membership: false,
            conditioned_support_authority: false,
            confinement_theorem_claim: false,
            confining_asymptotic_boundary_admitted: true,
            authority_effect: "none",
            pair_receipt_sha256: producer_receipt,
        },
    })
}

pub(super) fn project_admitted_profile() -> Result<AdmittedStrongProfile, StrongProfileRefusal> {
    let projection = construct_profile_projection()?;
    if projection.receipt.pair_receipt_sha256 == [0; 32]
        || projection.receipt.authority_effect != "none"
        || projection.receipt.global_physical_vocabulary_coverage
        || projection.receipt.membership_authority
        || projection.receipt.carrier_species_membership
        || projection.receipt.conditioned_support_authority
        || projection.receipt.confinement_theorem_claim
        || !projection.receipt.confining_asymptotic_boundary_admitted
        || projection.receipt.member_count != 0
        || projection.receipt.protocol.derive_first_status_id != DERIVE_FIRST_STATUS_ID
        || projection.receipt.protocol.buckingham_pi_status_id != BUCKINGHAM_PI_STATUS_ID
        || projection.receipt.protocol.gap_law_status_id != GAP_LAW_STATUS_ID
        || projection.receipt.protocol.chaos_protocol_status_id != CHAOS_PROTOCOL_STATUS_ID
        || projection.receipt.protocol.residual_law_status_id != RESIDUAL_LAW_STATUS_ID
        || projection.receipt.protocol.residual_slot_status_id != RESIDUAL_SLOT_STATUS_ID
    {
        return Err(StrongProfileRefusal::PairReceiptMismatch);
    }
    let profile_root_identity = projection.receipt.profile_root_identity;
    let pair_receipt_sha256 = projection.receipt.pair_receipt_sha256;
    let admitted_artifacts = projection
        .candidate_artifacts
        .into_iter()
        .map(|candidate| {
            let capability = StrongProfileAdmissionCapability {
                claimed_identity: candidate.identity,
                admission: candidate.admission.clone(),
                profile_root_identity,
                pair_receipt_sha256,
            };
            super::model::AdmittedArtifact::from_strong_profile(
                candidate.identity,
                candidate.admission,
                candidate.payload,
                capability,
            )
        })
        .collect();
    Ok(AdmittedStrongProfile {
        admitted_artifacts,
        receipt: projection.receipt,
    })
}

fn extract_protocol_receipt(
    candidates: &[StrongArtifactCandidate],
    profile_root_identity: ArtifactIdentity,
) -> Result<StrongProtocolReceipt, StrongProfileRefusal> {
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.identity == profile_root_identity)
        .ok_or(StrongProfileRefusal::ArtifactConstructionFailure)?;
    let AdmissionRoute::Irreducible(route) = &candidate.admission.route else {
        return Err(StrongProfileRefusal::ArtifactConstructionFailure);
    };
    if route.residual_slot_id != RESIDUAL_SLOT_ID {
        return Err(StrongProfileRefusal::ArtifactConstructionFailure);
    }
    let digests = [
        route.derivation_exhaustion_receipt.digest_sha256,
        route.buckingham_pi_receipt.digest_sha256,
        route.gap_law_receipt.digest_sha256,
        route.chaos_protocol_receipt.digest_sha256,
        route.residual_law_receipt.digest_sha256,
        route.residual_slot_receipt.digest_sha256,
        route.owner_admission_receipt.digest_sha256,
        route.independent_watchdog_receipt.digest_sha256,
    ];
    if digests.contains(&[0; 32])
        || digests.into_iter().collect::<BTreeSet<_>>().len() != digests.len()
    {
        return Err(StrongProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(StrongProtocolReceipt {
        derive_first_status_id: DERIVE_FIRST_STATUS_ID,
        buckingham_pi_status_id: BUCKINGHAM_PI_STATUS_ID,
        gap_law_status_id: GAP_LAW_STATUS_ID,
        chaos_protocol_status_id: CHAOS_PROTOCOL_STATUS_ID,
        residual_law_status_id: RESIDUAL_LAW_STATUS_ID,
        residual_slot_status_id: RESIDUAL_SLOT_STATUS_ID,
        derivation_exhaustion_sha256: digests[0],
        buckingham_pi_sha256: digests[1],
        gap_law_sha256: digests[2],
        chaos_protocol_sha256: digests[3],
        residual_law_sha256: digests[4],
        residual_slot_sha256: digests[5],
        owner_admission_sha256: digests[6],
        independent_watchdog_sha256: digests[7],
    })
}

pub(super) fn receipt_binding(
    schema_id: &str,
    digest_sha256: [u8; 32],
) -> super::model::ReceiptBinding {
    super::model::ReceiptBinding {
        schema_id: schema_id.to_owned(),
        digest_sha256,
    }
}

pub(super) fn append_field(bytes: &mut Vec<u8>, tag: u16, payload: &[u8]) {
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u32::try_from(payload.len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
}

pub(super) fn encode_packet(packet: &StrongProfilePacket) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, PACKET_SCHEMA_ID.as_bytes());
    for (tag, value) in [
        (2, packet.schema_id.as_str()),
        (3, packet.profile_id.as_str()),
        (4, packet.theory_class_id.as_str()),
        (5, packet.sector_id.as_str()),
        (6, packet.carrier_id.as_str()),
        (7, packet.curvature_law_id.as_str()),
        (8, packet.conservation_id.as_str()),
        (9, packet.confinement_id.as_str()),
        (10, packet.applicability_id.as_str()),
        (11, packet.validity_id.as_str()),
        (12, packet.asymptotic_disposition_id.as_str()),
        (13, packet.residual_slot_id.as_str()),
        (14, packet.owner_admission_record.as_str()),
        (15, packet.evidence_citation.as_str()),
        (16, packet.evidence_source_url.as_str()),
        (17, packet.evidence_full_sha256_hex.as_str()),
        (18, packet.evidence_slim_sha256_hex.as_str()),
        (19, packet.evidence_anchor.as_str()),
        (20, packet.floor_authority.schema_id.as_str()),
    ] {
        append_field(&mut bytes, tag, value.as_bytes());
    }
    append_field(&mut bytes, 21, &packet.floor_authority.digest_sha256);
    let mut slots = packet.occupied_profile_slots.clone();
    slots.sort();
    for slot in slots {
        append_field(&mut bytes, 22, slot.as_bytes());
    }
    append_field(
        &mut bytes,
        23,
        &[u8::from(packet.global_physical_vocabulary_coverage)],
    );
    append_field(&mut bytes, 24, &[u8::from(packet.membership_authority)]);
    append_field(
        &mut bytes,
        25,
        &[u8::from(packet.carrier_species_membership)],
    );
    append_field(
        &mut bytes,
        26,
        &[u8::from(packet.confinement_theorem_claim)],
    );
    bytes
}

pub(super) fn encode_checker_output(output: &StrongCheckerOutput) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_field(
        &mut bytes,
        1,
        b"civsim.planet.confining-profile.canonical-output.v1",
    );
    append_field(&mut bytes, 2, &output.input_sha256);
    append_field(&mut bytes, 3, &output.profile_root_identity.0);
    append_field(&mut bytes, 4, &output.profile_role_identity.0);
    append_field(&mut bytes, 5, &output.sector_identity.0);
    append_field(&mut bytes, 6, &output.carrier_identity.0);
    append_field(&mut bytes, 7, &output.constraint_law_identity.0);
    append_field(&mut bytes, 8, &output.evidence_custody_receipt_sha256);
    for (tag, digest) in [
        (9, output.curvature_producer_sha256),
        (10, output.curvature_watchdog_sha256),
        (11, output.conservation_producer_sha256),
        (12, output.conservation_watchdog_sha256),
        (13, output.confinement_producer_sha256),
        (14, output.confinement_watchdog_sha256),
        (15, output.applicability_producer_sha256),
        (16, output.applicability_watchdog_sha256),
        (17, output.validity_producer_sha256),
        (18, output.validity_watchdog_sha256),
        (19, output.asymptotic_producer_sha256),
        (20, output.asymptotic_watchdog_sha256),
        (
            21,
            output
                .admission_evidence
                .derivation_coverage_capability_sha256,
        ),
        (
            22,
            output
                .admission_evidence
                .irreducible_protocol_capability_sha256,
        ),
    ] {
        append_field(&mut bytes, tag, &digest);
    }
    for candidate in &output.candidates {
        append_field(&mut bytes, 23, &candidate.identity.0);
        append_field(&mut bytes, 24, &encode_admission(&candidate.admission));
    }
    append_field(
        &mut bytes,
        25,
        &output
            .admission_evidence
            .profile_protocol_pair_receipt_sha256,
    );
    bytes
}

fn encode_admission(admission: &RootAdmission) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, admission.tier.id().as_bytes());
    append_field(
        &mut bytes,
        2,
        admission
            .provenance
            .bracket_tag()
            .unwrap_or("[invalid]")
            .as_bytes(),
    );
    match &admission.route {
        AdmissionRoute::Derived(route) => {
            append_field(&mut bytes, 3, b"derived");
            for binding in [
                &route.ancestry_receipt,
                &route.semantic_checker_receipt,
                &route.independent_watchdog_receipt,
            ] {
                append_field(&mut bytes, 4, binding.schema_id.as_bytes());
                append_field(&mut bytes, 5, &binding.digest_sha256);
            }
        }
        AdmissionRoute::Irreducible(route) => {
            append_field(&mut bytes, 3, b"irreducible");
            for binding in [
                &route.derivation_exhaustion_receipt,
                &route.buckingham_pi_receipt,
                &route.gap_law_receipt,
                &route.chaos_protocol_receipt,
                &route.residual_law_receipt,
                &route.residual_slot_receipt,
                &route.owner_admission_receipt,
                &route.independent_watchdog_receipt,
            ] {
                append_field(&mut bytes, 4, binding.schema_id.as_bytes());
                append_field(&mut bytes, 5, &binding.digest_sha256);
            }
            append_field(&mut bytes, 6, route.residual_slot_id.as_bytes());
        }
        AdmissionRoute::EvidenceCustodyOnly { source_receipt } => {
            append_field(&mut bytes, 3, b"evidence-custody-only");
            append_field(&mut bytes, 4, source_receipt.schema_id.as_bytes());
            append_field(&mut bytes, 5, &source_receipt.digest_sha256);
        }
    }
    bytes
}

pub(super) fn digest_fields(domain: &[u8], fields: &[&[u8]]) -> [u8; 32] {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, domain);
    for field in fields {
        append_field(&mut bytes, 2, field);
    }
    sha256(&bytes)
}

pub(super) fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
