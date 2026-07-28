//! Repository-enrolled charge-conjugation-closed matter profile.
//!
//! A measured mass coordinate and an admitted Abelian sector do not select a
//! matter ontology. This module therefore executes the generic derive-first,
//! Buckingham Pi, Gap with Chaos, Residual, unique-slot, and owner-admission
//! route for one local complex spinor profile. Independent forward and reverse
//! checkers then agree on a two-member conjugation orbit before either member
//! receives a production capability.
//!
//! The result is local. It does not claim a complete particle vocabulary,
//! global stability, conditioned support, or a familiar particle name.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use super::super::SpeciesContentIdentity;
use super::model::{AdmissionRoute, ArtifactIdentity, ArtifactPayload, RootAdmission};
use civsim_units::digest::sha256;
use std::collections::BTreeSet;

pub(super) const PACKET_SCHEMA_ID: &str = "civsim.planet.charged-matter-profile-packet.v1";
pub(super) const PROFILE_SCHEMA_ID: &str =
    "civsim.physical-profile.charge-conjugate-massive-spinor.v1";
pub(super) const RECEIPT_SCHEMA_ID: &str = "civsim.planet.charged-matter-profile-pair-receipt.v1";
pub(super) const CLAIM_ID: &str = "planet.charged-matter.charge-conjugate-massive-spinor-pair";
pub(super) const PROFILE_ID: &str = "matter-profile.charge-conjugate-massive-spinor-pair.v1";
pub(super) const THEORY_CLASS_ID: &str =
    "local-lorentz-covariant-complex-spinor-under-admitted-rank-one-abelian-sector";
pub(super) const FIELD_ID: &str = "massive-complex-spinor-matter-field";
pub(super) const OPERATOR_ID: &str = "massive-first-order-spinor-wave-operator";
pub(super) const STATE_PAIR_CLASS_ID: &str = "two-state-charge-conjugation-orbit";
pub(super) const VALIDITY_ID: &str = "local-linearized-charge-conserving-matter-sector";
pub(super) const SPIN_ID: &str = "twice-spin-one";
pub(super) const STATISTICS_ID: &str = "half-integer-spin-fermi-statistics";
pub(super) const MOBILITY_ID: &str = "nonzero-current-mobile-excitation-within-local-validity";
pub(super) const STABILITY_ID: &str = "stable-as-lightest-charged-orbit-within-local-profile-only";
pub(super) const TRANSITION_ID: &str = "charge-neutral-pair-transition-within-local-profile-only";
pub(super) const CONJUGATION_ID: &str = "involutive-equal-mass-opposite-charge-two-state-closure";
pub(super) const MEMBER_CLASS_ID: &str = "primitive-massive-abelian-charged-matter-excitation";
pub(super) const MASS_SOURCE_ENTRY_ID: &str = "fundamental.m_e";
pub(super) const COUPLING_SOURCE_ENTRY_ID: &str = "fundamental.alpha";
pub(super) const RESIDUAL_SLOT_ID: &str = "planet.charged-matter.theory-profile.complex-spinor.v1";
pub(super) const OWNER_ADMISSION_RECORD: &str =
    "owner-reviewed-candidate-pr215-2026-07-27-charge-conjugate-massive-spinor-v1";

// @sources: pdg_2024_electroweak_model_review
pub(super) const EVIDENCE_CITATION: &str =
    "J. Erler and A. Freitas in S. Navas et al. (Particle Data Group), Phys. Rev. D 110, 030001 (2024), section 10.1, printed pages 2-3";
pub(super) const EVIDENCE_SOURCE_URL: &str =
    "https://pdg.lbl.gov/2024/reviews/rpp2024-rev-standard-model.pdf";
pub(super) const EVIDENCE_FULL_SHA256_HEX: &str =
    "4689caae925ee279228684249333a8395a40dda74a94863549b48f758d3fdd3f";
pub(super) const EVIDENCE_SLIM_SHA256_HEX: &str =
    "0caa17a5eb4c3329219a45537b52f6ca42c2ec18e9e317e55995179e6b9aee64";
pub(super) const EVIDENCE_ANCHOR: &str =
    "printed page 2 equation 10.2 and printed page 3 first two paragraphs";

pub(super) const PRODUCER_ID: &str = "civsim.planet.charged-matter-profile.forward-producer.v1";
pub(super) const WATCHDOG_ID: &str = "civsim.planet.charged-matter-profile.reverse-watchdog.v1";
pub(super) const PRODUCER_CANARY_ID: &str =
    "civsim.planet.charged-matter-profile.producer-canaries.v1";
pub(super) const WATCHDOG_CANARY_ID: &str =
    "civsim.planet.charged-matter-profile.watchdog-canaries.v1";
pub(super) const ARTIFACT_COUNT: usize = 39;
pub(super) const MEMBER_COUNT: usize = 2;
pub(super) const DERIVE_FIRST_STATUS_ID: &str = "executed_open_frontier";
pub(super) const BUCKINGHAM_PI_STATUS_ID: &str = "semantic_inapplicability_paired";
pub(super) const GAP_LAW_STATUS_ID: &str = "executed_and_bound";
pub(super) const CHAOS_PROTOCOL_STATUS_ID: &str = "nondynamical_inapplicability_paired";
pub(super) const RESIDUAL_LAW_STATUS_ID: &str = "executed_and_bound";
pub(super) const RESIDUAL_SLOT_STATUS_ID: &str = "collision_checked_unique";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChargedProfilePacket {
    pub(super) schema_id: String,
    pub(super) profile_id: String,
    pub(super) theory_class_id: String,
    pub(super) field_id: String,
    pub(super) operator_id: String,
    pub(super) state_pair_class_id: String,
    pub(super) validity_id: String,
    pub(super) spin_id: String,
    pub(super) statistics_id: String,
    pub(super) mobility_id: String,
    pub(super) stability_id: String,
    pub(super) transition_id: String,
    pub(super) conjugation_id: String,
    pub(super) member_class_id: String,
    pub(super) charge_weights: Vec<i8>,
    pub(super) residual_slot_id: String,
    pub(super) owner_admission_record: String,
    pub(super) evidence_citation: String,
    pub(super) evidence_source_url: String,
    pub(super) evidence_full_sha256_hex: String,
    pub(super) evidence_slim_sha256_hex: String,
    pub(super) evidence_anchor: String,
    pub(super) floor_authority: super::model::ReceiptBinding,
    pub(super) root_pair_receipt: super::model::ReceiptBinding,
    pub(super) mass_source_entry_id: String,
    pub(super) mass_scalar_identity: ArtifactIdentity,
    pub(super) mass_scalar_ancestry_sha256: [u8; 32],
    pub(super) neutral_mass_projection_identity: ArtifactIdentity,
    pub(super) neutral_mass_projection_ancestry_sha256: [u8; 32],
    pub(super) coupling_source_entry_id: String,
    pub(super) coupling_scalar_identity: ArtifactIdentity,
    pub(super) coupling_scalar_ancestry_sha256: [u8; 32],
    pub(super) primitive_profile_receipt: super::model::ReceiptBinding,
    pub(super) primitive_profile_root_identity: ArtifactIdentity,
    pub(super) primitive_sector_identity: ArtifactIdentity,
    pub(super) primitive_member: SpeciesContentIdentity,
    pub(super) primitive_residual_slot_id: String,
    pub(super) global_physical_vocabulary_coverage: bool,
    pub(super) membership_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChargedArtifactCandidate {
    pub(super) identity: ArtifactIdentity,
    pub(super) admission: RootAdmission,
    pub(super) payload: ArtifactPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChargedProfileCheckerOutput {
    pub(super) input_sha256: [u8; 32],
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) candidates: Vec<ChargedArtifactCandidate>,
    pub(super) members: Vec<SpeciesContentIdentity>,
    pub(super) profile_root_identity: ArtifactIdentity,
    pub(super) profile_role_identity: ArtifactIdentity,
    pub(super) evidence_custody_receipt_sha256: [u8; 32],
    pub(super) charge_conjugation_producer_sha256: [u8; 32],
    pub(super) charge_conjugation_watchdog_sha256: [u8; 32],
    pub(super) mass_transport_producer_sha256: [u8; 32],
    pub(super) mass_transport_watchdog_sha256: [u8; 32],
    pub(super) applicability_receipt_sha256: [u8; 32],
    pub(super) validity_receipt_sha256: [u8; 32],
    pub(super) admission_evidence: super::super::law_premise::TheoryProfileAdmissionEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CanaryEvidence {
    pub(super) transcript_id: &'static str,
    pub(super) case_count: u32,
    pub(super) transcript_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChargedProfileRefusal {
    SealedSourceUnavailable,
    PacketSchemaMismatch,
    ProfileIdentityMismatch,
    TheoryProfileMismatch,
    EvidenceCustodyMismatch,
    FloorBindingMismatch,
    RootBindingMismatch,
    PrimitiveProfileBindingMismatch,
    ChargeConjugationMismatch,
    MassTransportMismatch,
    ResidualSlotCollision,
    UnsupportedScope,
    ArtifactConstructionFailure,
    ArtifactCountMismatch,
    MemberIdentityMismatch,
    CanaryFailure,
    CheckerDisagreement,
    PairReceiptMismatch,
}

impl ChargedProfileRefusal {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::SealedSourceUnavailable => "sealed_source_unavailable",
            Self::PacketSchemaMismatch => "packet_schema_mismatch",
            Self::ProfileIdentityMismatch => "profile_identity_mismatch",
            Self::TheoryProfileMismatch => "theory_profile_mismatch",
            Self::EvidenceCustodyMismatch => "evidence_custody_mismatch",
            Self::FloorBindingMismatch => "floor_binding_mismatch",
            Self::RootBindingMismatch => "root_binding_mismatch",
            Self::PrimitiveProfileBindingMismatch => "primitive_profile_binding_mismatch",
            Self::ChargeConjugationMismatch => "charge_conjugation_mismatch",
            Self::MassTransportMismatch => "mass_transport_mismatch",
            Self::ResidualSlotCollision => "residual_slot_collision",
            Self::UnsupportedScope => "unsupported_scope",
            Self::ArtifactConstructionFailure => "artifact_construction_failure",
            Self::ArtifactCountMismatch => "artifact_count_mismatch",
            Self::MemberIdentityMismatch => "member_identity_mismatch",
            Self::CanaryFailure => "canary_failure",
            Self::CheckerDisagreement => "checker_disagreement",
            Self::PairReceiptMismatch => "pair_receipt_mismatch",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChargedProfileProtocolReceipt {
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
pub(super) struct ChargedProfileReceipt {
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
    pub(super) members: Vec<SpeciesContentIdentity>,
    pub(super) artifact_count: u32,
    pub(super) evidence_custody_receipt_sha256: [u8; 32],
    pub(super) charge_conjugation_producer_sha256: [u8; 32],
    pub(super) charge_conjugation_watchdog_sha256: [u8; 32],
    pub(super) mass_transport_producer_sha256: [u8; 32],
    pub(super) mass_transport_watchdog_sha256: [u8; 32],
    pub(super) root_pair_receipt_sha256: [u8; 32],
    pub(super) mass_scalar_identity: ArtifactIdentity,
    pub(super) coupling_scalar_identity: ArtifactIdentity,
    pub(super) primitive_profile_receipt_sha256: [u8; 32],
    pub(super) primitive_profile_root_identity: ArtifactIdentity,
    pub(super) primitive_sector_identity: ArtifactIdentity,
    pub(super) protocol: ChargedProfileProtocolReceipt,
    pub(super) global_physical_vocabulary_coverage: bool,
    pub(super) membership_authority: bool,
    pub(super) authority_effect: &'static str,
    pub(super) pair_receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChargedProfileProjection {
    pub(super) candidate_artifacts: Vec<ChargedArtifactCandidate>,
    pub(super) members: Vec<SpeciesContentIdentity>,
    pub(super) receipt: ChargedProfileReceipt,
}

/// Opaque proof that one charged-profile artifact was emitted by the agreed
/// repository pair.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ChargedProfileAdmissionCapability {
    claimed_identity: ArtifactIdentity,
    admission: RootAdmission,
    profile_root_identity: ArtifactIdentity,
    pair_receipt_sha256: [u8; 32],
}

impl ChargedProfileAdmissionCapability {
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
pub(super) struct AdmittedChargedProfile {
    pub(super) admitted_artifacts: Vec<super::model::AdmittedArtifact>,
    pub(super) members: Vec<SpeciesContentIdentity>,
    pub(super) receipt: ChargedProfileReceipt,
}

pub(super) fn construct_profile_projection(
) -> Result<ChargedProfileProjection, ChargedProfileRefusal> {
    let producer_packet = producer::sealed_packet()?;
    let watchdog_packet = watchdog::sealed_packet()?;
    if producer_packet != watchdog_packet {
        return Err(ChargedProfileRefusal::CheckerDisagreement);
    }
    let produced = producer::inspect(&producer_packet)?;
    let watched = watchdog::inspect(&watchdog_packet)?;
    if produced != watched {
        return Err(ChargedProfileRefusal::CheckerDisagreement);
    }
    if produced.candidates.len() != ARTIFACT_COUNT
        || produced.members.len() != MEMBER_COUNT
        || produced.members.windows(2).any(|pair| pair[0] >= pair[1])
        || produced.profile_root_identity.0 == [0; 32]
        || produced.profile_role_identity.0 == [0; 32]
        || produced.admission_evidence.decision_id != "irreducible_protocol_structurally_bound"
    {
        return Err(ChargedProfileRefusal::ArtifactCountMismatch);
    }

    let producer_canary = producer::canary_evidence(&producer_packet)?;
    let watchdog_canary = watchdog::canary_evidence(&watchdog_packet)?;
    if producer_canary.transcript_id != PRODUCER_CANARY_ID
        || watchdog_canary.transcript_id != WATCHDOG_CANARY_ID
        || producer_canary.transcript_id == watchdog_canary.transcript_id
        || producer_canary.case_count == 0
        || watchdog_canary.case_count == 0
        || producer_canary.transcript_sha256 == [0; 32]
        || watchdog_canary.transcript_sha256 == [0; 32]
    {
        return Err(ChargedProfileRefusal::CanaryFailure);
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
        return Err(ChargedProfileRefusal::PairReceiptMismatch);
    }
    let protocol = extract_protocol_receipt(&produced.candidates, produced.profile_root_identity)?;
    let candidate_artifacts = produced.candidates;
    let artifact_count = u32::try_from(candidate_artifacts.len())
        .map_err(|_| ChargedProfileRefusal::ArtifactCountMismatch)?;
    Ok(ChargedProfileProjection {
        candidate_artifacts,
        members: produced.members.clone(),
        receipt: ChargedProfileReceipt {
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
            members: produced.members,
            artifact_count,
            evidence_custody_receipt_sha256: produced.evidence_custody_receipt_sha256,
            charge_conjugation_producer_sha256: produced.charge_conjugation_producer_sha256,
            charge_conjugation_watchdog_sha256: produced.charge_conjugation_watchdog_sha256,
            mass_transport_producer_sha256: produced.mass_transport_producer_sha256,
            mass_transport_watchdog_sha256: produced.mass_transport_watchdog_sha256,
            root_pair_receipt_sha256: producer_packet.root_pair_receipt.digest_sha256,
            mass_scalar_identity: producer_packet.mass_scalar_identity,
            coupling_scalar_identity: producer_packet.coupling_scalar_identity,
            primitive_profile_receipt_sha256: producer_packet
                .primitive_profile_receipt
                .digest_sha256,
            primitive_profile_root_identity: producer_packet.primitive_profile_root_identity,
            primitive_sector_identity: producer_packet.primitive_sector_identity,
            protocol,
            global_physical_vocabulary_coverage: false,
            membership_authority: false,
            authority_effect: "none",
            pair_receipt_sha256: producer_receipt,
        },
    })
}

pub(super) fn project_admitted_profile() -> Result<AdmittedChargedProfile, ChargedProfileRefusal> {
    let projection = construct_profile_projection()?;
    if projection.receipt.pair_receipt_sha256 == [0; 32]
        || projection.receipt.authority_effect != "none"
        || projection.receipt.global_physical_vocabulary_coverage
        || projection.receipt.membership_authority
        || projection.receipt.members != projection.members
        || projection.receipt.protocol.derive_first_status_id != DERIVE_FIRST_STATUS_ID
        || projection.receipt.protocol.buckingham_pi_status_id != BUCKINGHAM_PI_STATUS_ID
        || projection.receipt.protocol.gap_law_status_id != GAP_LAW_STATUS_ID
        || projection.receipt.protocol.chaos_protocol_status_id != CHAOS_PROTOCOL_STATUS_ID
        || projection.receipt.protocol.residual_law_status_id != RESIDUAL_LAW_STATUS_ID
        || projection.receipt.protocol.residual_slot_status_id != RESIDUAL_SLOT_STATUS_ID
    {
        return Err(ChargedProfileRefusal::PairReceiptMismatch);
    }
    let profile_root_identity = projection.receipt.profile_root_identity;
    let pair_receipt_sha256 = projection.receipt.pair_receipt_sha256;
    let admitted_artifacts = projection
        .candidate_artifacts
        .into_iter()
        .map(|candidate| {
            let capability = ChargedProfileAdmissionCapability {
                claimed_identity: candidate.identity,
                admission: candidate.admission.clone(),
                profile_root_identity,
                pair_receipt_sha256,
            };
            super::model::AdmittedArtifact::from_charged_profile(
                candidate.identity,
                candidate.admission,
                candidate.payload,
                capability,
            )
        })
        .collect();
    Ok(AdmittedChargedProfile {
        admitted_artifacts,
        members: projection.members,
        receipt: projection.receipt,
    })
}

fn extract_protocol_receipt(
    candidates: &[ChargedArtifactCandidate],
    profile_root_identity: ArtifactIdentity,
) -> Result<ChargedProfileProtocolReceipt, ChargedProfileRefusal> {
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.identity == profile_root_identity)
        .ok_or(ChargedProfileRefusal::ArtifactConstructionFailure)?;
    let AdmissionRoute::Irreducible(route) = &candidate.admission.route else {
        return Err(ChargedProfileRefusal::ArtifactConstructionFailure);
    };
    if route.residual_slot_id != RESIDUAL_SLOT_ID {
        return Err(ChargedProfileRefusal::ArtifactConstructionFailure);
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
        return Err(ChargedProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(ChargedProfileProtocolReceipt {
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
        &u64::try_from(payload.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
}
