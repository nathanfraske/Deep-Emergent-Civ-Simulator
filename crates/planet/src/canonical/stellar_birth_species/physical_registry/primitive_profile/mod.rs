//! Repository-enrolled narrow primitive theory profile.
//!
//! The sealed constants and the derived `eps_0` execution relation do not
//! choose a field ontology. This module therefore runs the full derive-first
//! failure and irreducible protocol for one owner-reviewed, source-bound,
//! unbroken abelian profile. It then executes the independent complete
//! quadratic-basis, affine-action, exact excluded-operator, and action-bound
//! scope pairs before any artifact can receive a production capability.
//!
//! The resulting member is claim-local and partial. Global vocabulary
//! coverage, complete species membership, conditioned support, and downstream
//! stellar-birth measure authority remain false.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use super::super::SpeciesContentIdentity;
use super::model::{AdmissionRoute, ArtifactIdentity, ArtifactPayload, RootAdmission};
use civsim_units::digest::sha256;
use std::collections::BTreeSet;

pub(super) const PACKET_SCHEMA_ID: &str = "civsim.planet.primitive-excitation-profile-packet.v1";
pub(super) const PROFILE_SCHEMA_ID: &str = "civsim.physical-profile.unbroken-abelian-null.v1";
pub(super) const RECEIPT_SCHEMA_ID: &str =
    "civsim.planet.primitive-excitation-profile-pair-receipt.v1";
pub(super) const CLAIM_ID: &str = "planet.primitive-excitation.unbroken-abelian-null-mode";
pub(super) const PROFILE_ID: &str = "primitive-profile.unbroken-abelian-null-excitation.v1";
pub(super) const THEORY_CLASS_ID: &str = "compact-rank-one-unbroken-abelian-gauge-theory";
pub(super) const SYMMETRY_ID: &str = "unbroken-compact-rank-one-abelian-gauge-symmetry";
pub(super) const FIELD_ID: &str = "abelian-gauge-connection-field";
pub(super) const OPERATOR_ID: &str = "source-free-gauge-wave-operator";
pub(super) const STATE_ID: &str = "transverse-null-one-excitation-state";
pub(super) const SECTOR_ID: &str = "unbroken-abelian-interaction-sector";
pub(super) const VALIDITY_ID: &str = "local-source-free-linearized-unbroken-sector";
pub(super) const HELICITY_ID: &str = "massless-helicity-pair-minus-one-plus-one";
pub(super) const STATISTICS_ID: &str = "integer-spin-bose-statistics";
pub(super) const CHARGE_ID: &str = "zero-unbroken-abelian-self-charge";
pub(super) const CURRENT_ID: &str = "conserved-unbroken-abelian-current-coupling";
pub(super) const STABILITY_ID: &str = "stable-within-unbroken-source-free-validity-domain";
pub(super) const TRANSITION_ID: &str = "no-lower-profile-state-transition-within-validity-domain";
pub(super) const EXCLUDED_TERM_ID: &str = "gauge-noninvariant-rest-mass-term";
pub(super) const MEMBER_ID: &str = "primitive-null-abelian-gauge-excitation";
pub(super) const RESIDUAL_SLOT_ID: &str =
    "planet.primitive-excitation.theory-profile.unbroken-abelian.v1";
pub(super) const OWNER_ADMISSION_RECORD: &str =
    "owner-reviewed-candidate-pr215-2026-07-26-unbroken-abelian-null-profile-v1";
// @sources: pdg_2024_higgs_boson_review
pub(super) const EVIDENCE_CITATION: &str =
    "S. Navas et al. (Particle Data Group), Phys. Rev. D 110, 030001 (2024), section 11, PDF page 4";
pub(super) const EVIDENCE_SOURCE_URL: &str =
    "https://pdg.lbl.gov/2024/reviews/rpp2024-rev-higgs-boson.pdf";
pub(super) const EVIDENCE_FULL_SHA256_HEX: &str =
    "1850dcead11d3e885c76ee9e9eb21b474668fc75cca7a19c602d23e7be73c8a3";
pub(super) const EVIDENCE_SLIM_SHA256_HEX: &str =
    "61381048888c9a00b51fc2607d1b95e60af2665a45f322e01d991599c14d1327";
pub(super) const EVIDENCE_ANCHOR: &str = "PDF page 4, paragraph after equation 11.5";
pub(super) const PRODUCER_ID: &str =
    "civsim.planet.primitive-excitation-profile.forward-producer.v1";
pub(super) const WATCHDOG_ID: &str =
    "civsim.planet.primitive-excitation-profile.reverse-watchdog.v1";
pub(super) const PRODUCER_CANARY_ID: &str =
    "civsim.planet.primitive-excitation-profile.producer-canaries.v1";
pub(super) const WATCHDOG_CANARY_ID: &str =
    "civsim.planet.primitive-excitation-profile.watchdog-canaries.v1";
pub(super) const ARTIFACT_COUNT: usize = 29;
pub(super) const DERIVE_FIRST_STATUS_ID: &str = "executed_open_frontier";
pub(super) const BUCKINGHAM_PI_STATUS_ID: &str = "semantic_inapplicability_paired";
pub(super) const GAP_LAW_STATUS_ID: &str = "executed_and_bound";
pub(super) const CHAOS_PROTOCOL_STATUS_ID: &str = "nondynamical_inapplicability_paired";
pub(super) const RESIDUAL_LAW_STATUS_ID: &str = "executed_and_bound";
pub(super) const RESIDUAL_SLOT_STATUS_ID: &str = "collision_checked_unique";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PrimitiveProfilePacket {
    pub(super) schema_id: String,
    pub(super) profile_id: String,
    pub(super) theory_class_id: String,
    pub(super) symmetry_id: String,
    pub(super) field_id: String,
    pub(super) operator_id: String,
    pub(super) state_id: String,
    pub(super) sector_id: String,
    pub(super) validity_id: String,
    pub(super) helicity_id: String,
    pub(super) statistics_id: String,
    pub(super) charge_id: String,
    pub(super) current_id: String,
    pub(super) stability_id: String,
    pub(super) transition_id: String,
    pub(super) excluded_term_id: String,
    pub(super) member_id: String,
    pub(super) residual_slot_id: String,
    pub(super) owner_admission_record: String,
    pub(super) evidence_citation: String,
    pub(super) evidence_source_url: String,
    pub(super) evidence_full_sha256_hex: String,
    pub(super) evidence_slim_sha256_hex: String,
    pub(super) evidence_anchor: String,
    pub(super) floor_authority_schema_id: String,
    pub(super) floor_authority_sha256: [u8; 32],
    pub(super) eps0_pair_receipt_sha256: [u8; 32],
    pub(super) eps0_capability_sha256: [u8; 32],
    pub(super) global_physical_vocabulary_coverage: bool,
    pub(super) membership_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProfileArtifactCandidate {
    pub(super) identity: ArtifactIdentity,
    pub(super) admission: RootAdmission,
    pub(super) payload: ArtifactPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PrimitiveProfileCheckerOutput {
    pub(super) input_sha256: [u8; 32],
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) candidates: Vec<ProfileArtifactCandidate>,
    pub(super) member: SpeciesContentIdentity,
    pub(super) profile_root_identity: ArtifactIdentity,
    pub(super) profile_role_identity: ArtifactIdentity,
    pub(super) evidence_custody_receipt_sha256: [u8; 32],
    pub(super) admission_evidence: super::super::law_premise::TheoryProfileAdmissionEvidence,
    pub(super) symmetry_evidence:
        super::super::symmetry_operator_exclusion::TheoryProfileSymmetryEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CanaryEvidence {
    pub(super) transcript_id: &'static str,
    pub(super) case_count: u32,
    pub(super) transcript_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PrimitiveProfileRefusal {
    SealedSourceUnavailable,
    PacketSchemaMismatch,
    ProfileIdentityMismatch,
    TheoryProfileMismatch,
    EvidenceCustodyMismatch,
    FloorBindingMismatch,
    DerivedPremiseBindingMismatch,
    UnsupportedScope,
    ArtifactConstructionFailure,
    ArtifactCountMismatch,
    MemberIdentityMismatch,
    CanaryFailure,
    CheckerDisagreement,
    PairReceiptMismatch,
}

impl PrimitiveProfileRefusal {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::SealedSourceUnavailable => "sealed_source_unavailable",
            Self::PacketSchemaMismatch => "packet_schema_mismatch",
            Self::ProfileIdentityMismatch => "profile_identity_mismatch",
            Self::TheoryProfileMismatch => "theory_profile_mismatch",
            Self::EvidenceCustodyMismatch => "evidence_custody_mismatch",
            Self::FloorBindingMismatch => "floor_binding_mismatch",
            Self::DerivedPremiseBindingMismatch => "derived_premise_binding_mismatch",
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PrimitiveProfileReceipt {
    pub(super) schema_id: &'static str,
    pub(super) claim_id: &'static str,
    pub(super) producer_id: &'static str,
    pub(super) watchdog_id: &'static str,
    pub(super) producer_result_sha256: [u8; 32],
    pub(super) watchdog_result_sha256: [u8; 32],
    pub(super) producer_canary: CanaryEvidence,
    pub(super) watchdog_canary: CanaryEvidence,
    pub(super) profile_root_identity: ArtifactIdentity,
    pub(super) profile_role_identity: ArtifactIdentity,
    pub(super) member: SpeciesContentIdentity,
    pub(super) artifact_count: u32,
    pub(super) evidence_custody_receipt_sha256: [u8; 32],
    pub(super) profile_id: &'static str,
    pub(super) theory_class_id: &'static str,
    pub(super) residual_slot_id: &'static str,
    pub(super) owner_admission_record: &'static str,
    pub(super) protocol: PrimitiveProfileProtocolReceipt,
    pub(super) symmetry_basis_element_count: u32,
    pub(super) symmetry_excluded_operator_count: u32,
    pub(super) symmetry_action_binding_sha256: [u8; 32],
    pub(super) symmetry_applicability_receipt_sha256: [u8; 32],
    pub(super) symmetry_validity_receipt_sha256: [u8; 32],
    pub(super) derivation_catalog_sha256: [u8; 32],
    pub(super) repository_catalog_sha256: [u8; 32],
    pub(super) protocol_producer_result_sha256: [u8; 32],
    pub(super) protocol_watchdog_result_sha256: [u8; 32],
    pub(super) derivation_coverage_capability_sha256: [u8; 32],
    pub(super) irreducible_protocol_capability_sha256: [u8; 32],
    pub(super) global_physical_vocabulary_coverage: bool,
    pub(super) membership_authority: bool,
    pub(super) authority_effect: &'static str,
    pub(super) pair_receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PrimitiveProfileProtocolReceipt {
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
pub(super) struct PrimitiveProfileProjection {
    pub(super) candidate_artifacts: Vec<ProfileArtifactCandidate>,
    pub(super) member: SpeciesContentIdentity,
    pub(super) receipt: PrimitiveProfileReceipt,
}

/// Opaque proof that one profile artifact was emitted by the fully executed
/// repository profile pair. No receipt-shaped caller value can construct it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PrimitiveProfileAdmissionCapability {
    claimed_identity: ArtifactIdentity,
    admission: RootAdmission,
    profile_root_identity: ArtifactIdentity,
    pair_receipt_sha256: [u8; 32],
}

impl PrimitiveProfileAdmissionCapability {
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
pub(super) struct AdmittedPrimitiveProfile {
    pub(super) admitted_artifacts: Vec<super::model::AdmittedArtifact>,
    pub(super) member: SpeciesContentIdentity,
    pub(super) sector_identity: ArtifactIdentity,
    pub(super) receipt: PrimitiveProfileReceipt,
}

pub(super) fn construct_profile_projection(
) -> Result<PrimitiveProfileProjection, PrimitiveProfileRefusal> {
    let producer_packet = producer::sealed_packet()?;
    let watchdog_packet = watchdog::sealed_packet()?;
    if producer_packet != watchdog_packet {
        return Err(PrimitiveProfileRefusal::CheckerDisagreement);
    }
    let produced = producer::inspect(&producer_packet)?;
    let watched = watchdog::inspect(&watchdog_packet)?;
    if produced != watched {
        return Err(PrimitiveProfileRefusal::CheckerDisagreement);
    }
    if produced.candidates.len() != ARTIFACT_COUNT
        || produced.member.0 == [0; 32]
        || produced.profile_root_identity.0 == [0; 32]
        || produced.profile_role_identity.0 == [0; 32]
        || produced.admission_evidence.decision_id != "irreducible_protocol_structurally_bound"
        || produced.symmetry_evidence.basis_element_count == 0
        || produced.symmetry_evidence.excluded_operator_count != 1
    {
        return Err(PrimitiveProfileRefusal::ArtifactCountMismatch);
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
        return Err(PrimitiveProfileRefusal::CanaryFailure);
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
        return Err(PrimitiveProfileRefusal::PairReceiptMismatch);
    }
    let protocol = extract_protocol_receipt(&produced.candidates, produced.profile_root_identity)?;

    let candidate_artifacts = produced.candidates;
    let artifact_count = u32::try_from(candidate_artifacts.len())
        .map_err(|_| PrimitiveProfileRefusal::ArtifactCountMismatch)?;
    Ok(PrimitiveProfileProjection {
        candidate_artifacts,
        member: produced.member,
        receipt: PrimitiveProfileReceipt {
            schema_id: RECEIPT_SCHEMA_ID,
            claim_id: CLAIM_ID,
            producer_id: PRODUCER_ID,
            watchdog_id: WATCHDOG_ID,
            producer_result_sha256,
            watchdog_result_sha256,
            producer_canary,
            watchdog_canary,
            profile_root_identity: produced.profile_root_identity,
            profile_role_identity: produced.profile_role_identity,
            member: produced.member,
            artifact_count,
            evidence_custody_receipt_sha256: produced.evidence_custody_receipt_sha256,
            profile_id: PROFILE_ID,
            theory_class_id: THEORY_CLASS_ID,
            residual_slot_id: RESIDUAL_SLOT_ID,
            owner_admission_record: OWNER_ADMISSION_RECORD,
            protocol,
            symmetry_basis_element_count: produced.symmetry_evidence.basis_element_count,
            symmetry_excluded_operator_count: produced.symmetry_evidence.excluded_operator_count,
            symmetry_action_binding_sha256: produced.symmetry_evidence.action_binding_sha256,
            symmetry_applicability_receipt_sha256: produced
                .symmetry_evidence
                .applicability_receipt_sha256,
            symmetry_validity_receipt_sha256: produced.symmetry_evidence.validity_receipt_sha256,
            derivation_catalog_sha256: produced.admission_evidence.derivation_catalog_sha256,
            repository_catalog_sha256: produced.admission_evidence.repository_catalog_sha256,
            protocol_producer_result_sha256: produced
                .admission_evidence
                .protocol_producer_result_sha256,
            protocol_watchdog_result_sha256: produced
                .admission_evidence
                .protocol_watchdog_result_sha256,
            derivation_coverage_capability_sha256: produced
                .admission_evidence
                .derivation_coverage_capability_sha256,
            irreducible_protocol_capability_sha256: produced
                .admission_evidence
                .irreducible_protocol_capability_sha256,
            global_physical_vocabulary_coverage: false,
            membership_authority: false,
            authority_effect: "none",
            pair_receipt_sha256: producer_receipt,
        },
    })
}

pub(super) fn project_admitted_profile() -> Result<AdmittedPrimitiveProfile, PrimitiveProfileRefusal>
{
    let projection = construct_profile_projection()?;
    if projection.receipt.pair_receipt_sha256 == [0; 32]
        || projection.receipt.authority_effect != "none"
        || projection.receipt.global_physical_vocabulary_coverage
        || projection.receipt.membership_authority
        || projection.receipt.artifact_count
            != u32::try_from(projection.candidate_artifacts.len())
                .map_err(|_| PrimitiveProfileRefusal::ArtifactCountMismatch)?
        || projection.receipt.member != projection.member
        || projection.receipt.protocol.derive_first_status_id != DERIVE_FIRST_STATUS_ID
        || projection.receipt.protocol.buckingham_pi_status_id != BUCKINGHAM_PI_STATUS_ID
        || projection.receipt.protocol.gap_law_status_id != GAP_LAW_STATUS_ID
        || projection.receipt.protocol.chaos_protocol_status_id != CHAOS_PROTOCOL_STATUS_ID
        || projection.receipt.protocol.residual_law_status_id != RESIDUAL_LAW_STATUS_ID
        || projection.receipt.protocol.residual_slot_status_id != RESIDUAL_SLOT_STATUS_ID
        || projection.receipt.symmetry_basis_element_count != 10
        || projection.receipt.symmetry_excluded_operator_count != 1
    {
        return Err(PrimitiveProfileRefusal::PairReceiptMismatch);
    }
    let profile_root_identity = projection.receipt.profile_root_identity;
    let pair_receipt_sha256 = projection.receipt.pair_receipt_sha256;
    let sector_identity = projection
        .candidate_artifacts
        .iter()
        .find_map(|candidate| match &candidate.payload {
            ArtifactPayload::PhysicalDescriptor(content)
                if content.schema_id == "civsim.physical-profile.sector.v1"
                    && content.canonical_bytes == SECTOR_ID.as_bytes() =>
            {
                Some(candidate.identity)
            }
            _ => None,
        })
        .ok_or(PrimitiveProfileRefusal::ArtifactConstructionFailure)?;
    let admitted_artifacts = projection
        .candidate_artifacts
        .into_iter()
        .map(|candidate| {
            let capability = PrimitiveProfileAdmissionCapability {
                claimed_identity: candidate.identity,
                admission: candidate.admission.clone(),
                profile_root_identity,
                pair_receipt_sha256,
            };
            super::model::AdmittedArtifact::from_primitive_profile(
                candidate.identity,
                candidate.admission,
                candidate.payload,
                capability,
            )
        })
        .collect();
    Ok(AdmittedPrimitiveProfile {
        admitted_artifacts,
        member: projection.member,
        sector_identity,
        receipt: projection.receipt,
    })
}

fn extract_protocol_receipt(
    candidates: &[ProfileArtifactCandidate],
    profile_root_identity: ArtifactIdentity,
) -> Result<PrimitiveProfileProtocolReceipt, PrimitiveProfileRefusal> {
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.identity == profile_root_identity)
        .ok_or(PrimitiveProfileRefusal::ArtifactConstructionFailure)?;
    let AdmissionRoute::Irreducible(route) = &candidate.admission.route else {
        return Err(PrimitiveProfileRefusal::ArtifactConstructionFailure);
    };
    let expected_schemas = [
        (
            route.derivation_exhaustion_receipt.schema_id.as_str(),
            "civsim.primitive-profile.derive-first-exhaustion.v1",
        ),
        (
            route.buckingham_pi_receipt.schema_id.as_str(),
            "civsim.primitive-profile.buckingham-pi.v1",
        ),
        (
            route.gap_law_receipt.schema_id.as_str(),
            "civsim.primitive-profile.gap-law.v1",
        ),
        (
            route.chaos_protocol_receipt.schema_id.as_str(),
            "civsim.primitive-profile.chaos-protocol.v1",
        ),
        (
            route.residual_law_receipt.schema_id.as_str(),
            "civsim.primitive-profile.residual-law.v1",
        ),
        (
            route.residual_slot_receipt.schema_id.as_str(),
            "civsim.primitive-profile.residual-slot.v1",
        ),
        (
            route.owner_admission_receipt.schema_id.as_str(),
            "civsim.primitive-profile.owner-admission.v1",
        ),
        (
            route.independent_watchdog_receipt.schema_id.as_str(),
            "civsim.primitive-profile.independent-watchdog-route.v1",
        ),
    ];
    if route.residual_slot_id != RESIDUAL_SLOT_ID
        || expected_schemas
            .into_iter()
            .any(|(found, expected)| found != expected)
    {
        return Err(PrimitiveProfileRefusal::ArtifactConstructionFailure);
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
        return Err(PrimitiveProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(PrimitiveProfileProtocolReceipt {
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

pub(super) fn append_field(bytes: &mut Vec<u8>, tag: u16, payload: &[u8]) {
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(payload.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
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
