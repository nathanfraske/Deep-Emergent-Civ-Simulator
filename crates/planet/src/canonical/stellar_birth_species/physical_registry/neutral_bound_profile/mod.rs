//! One claim-local neutral two-body bound profile.
//!
//! The profile consumes the already admitted equal-mass opposite-charge orbit,
//! its Abelian carrier sector, and the sealed `alpha` and `m_e` coordinates.
//! It admits one leading central interaction premise only after the complete
//! derive-first and irreducible route. Every spectral, normalization,
//! uncertainty, conservation, threshold, and member result then derives from
//! that premise through an independent producer and watchdog pair.
//!
//! Constituent binding and decay are kept distinct. The emitted member is
//! strictly below its free constituent threshold inside the stated validity
//! domain, while a neutral massless-carrier decay family remains energetically
//! open. No stable-support or familiar-atom claim follows.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use super::super::SpeciesContentIdentity;
use super::model::{AdmissionRoute, ArtifactIdentity, ArtifactPayload, RootAdmission};
use civsim_units::{bignum::BigRat, digest::sha256};
use std::{cmp::Ordering, collections::BTreeSet};

pub(super) const PACKET_SCHEMA_ID: &str = "civsim.planet.neutral-bound-profile-packet.v1";
pub(super) const PROFILE_SCHEMA_ID: &str =
    "civsim.physical-profile.neutral-equal-mass-central-bound-state.v1";
pub(super) const RECEIPT_SCHEMA_ID: &str = "civsim.planet.neutral-bound-profile-pair-receipt.v1";
pub(super) const CLAIM_ID: &str =
    "planet.neutral-bound-state.equal-mass-opposite-charge-central-ground-level";
pub(super) const PROFILE_ID: &str =
    "bound-profile.neutral-equal-mass-opposite-charge-ground-level.v1";
pub(super) const THEORY_CLASS_ID: &str =
    "local-leading-nonrelativistic-attractive-rank-one-abelian-two-body-reduction";
pub(super) const INTERACTION_ID: &str =
    "attractive-central-inverse-radius-current-mediated-interaction";
pub(super) const STATE_ID: &str = "normalized-lowest-s-wave-equal-mass-opposite-charge-level";
pub(super) const VALIDITY_ID: &str = "positive-subunit-coupling-leading-central-two-body-validity";
pub(super) const NORMALIZATION_ID: &str = "dimensionless-radial-density-integrates-exactly-to-one";
pub(super) const CONVERGENCE_ID: &str =
    "stationary-rayleigh-quotient-and-zero-radial-residual-agree";
pub(super) const CONSERVATION_ID: &str = "opposite-relative-charge-weights-sum-exactly-to-zero";
pub(super) const SEPARATION_ID: &str = "strictly-below-free-two-constituent-threshold";
pub(super) const DECAY_ID: &str =
    "neutral-massless-carrier-decay-family-energetically-open-rate-unresolved";
pub(super) const STABILITY_ID: &str =
    "constituent-bound-with-open-decay-and-no-conditioned-support";
pub(super) const TRANSITION_ID: &str =
    "open-neutral-carrier-transition-family-without-lifetime-claim";
pub(super) const MEMBER_CLASS_ID: &str = "composite-neutral-two-body-bound-excitation";
pub(super) const MASS_SOURCE_ENTRY_ID: &str = "fundamental.m_e";
pub(super) const COUPLING_SOURCE_ENTRY_ID: &str = "fundamental.alpha";
pub(super) const RESIDUAL_SLOT_ID: &str =
    "planet.neutral-bound-state.leading-central-interaction.v1";
pub(super) const OWNER_ADMISSION_RECORD: &str =
    "owner-reviewed-candidate-pr215-2026-07-27-neutral-central-bound-profile-v1";

// @sources: nist_dlmf_18_39_coulomb_bound_states, nist_dlmf_33_22_two_body_coulomb
pub(super) const EVIDENCE_PRIMARY_CITATION: &str =
    "NIST Digital Library of Mathematical Functions, section 18.39(ii), The Quantum Coulomb Problem, version 1.2.7";
pub(super) const EVIDENCE_PRIMARY_URL: &str = "https://dlmf.nist.gov/18.39";
pub(super) const EVIDENCE_PRIMARY_SHA256_HEX: &str =
    "c57ce71322118fb1ffed17237ee4398085830364310190ddb72a9f65f072735a";
pub(super) const EVIDENCE_PRIMARY_ANCHOR: &str =
    "equations 18.39.27 through 18.39.31 and the negative discrete spectrum discussion";
pub(super) const EVIDENCE_SECONDARY_CITATION: &str =
    "NIST Digital Library of Mathematical Functions, section 33.22(v), Particle Scattering and Atomic and Molecular Spectra, version 1.2.7";
pub(super) const EVIDENCE_SECONDARY_URL: &str = "https://dlmf.nist.gov/33.22";
pub(super) const EVIDENCE_SECONDARY_SHA256_HEX: &str =
    "2169602261e5563744f26ed9ac38ddb06d73bab356babbe3d644ad9df9aa15eb";
pub(super) const EVIDENCE_SECONDARY_ANCHOR: &str =
    "two-particle reduced-mass radial equation and attractive Coulomb bound-state paragraph";

pub(super) const PRODUCER_ID: &str = "civsim.planet.neutral-bound-profile.variational-producer.v1";
pub(super) const WATCHDOG_ID: &str =
    "civsim.planet.neutral-bound-profile.factorization-watchdog.v1";
pub(super) const PRODUCER_CANARY_ID: &str =
    "civsim.planet.neutral-bound-profile.producer-canaries.v1";
pub(super) const WATCHDOG_CANARY_ID: &str =
    "civsim.planet.neutral-bound-profile.watchdog-canaries.v1";
pub(super) const ARTIFACT_COUNT: usize = 33;
pub(super) const MEMBER_COUNT: usize = 1;
pub(super) const DERIVE_FIRST_STATUS_ID: &str = "executed_open_frontier";
pub(super) const BUCKINGHAM_PI_STATUS_ID: &str = "semantic_inapplicability_paired";
pub(super) const GAP_LAW_STATUS_ID: &str = "executed_and_bound";
pub(super) const CHAOS_PROTOCOL_STATUS_ID: &str = "nondynamical_inapplicability_paired";
pub(super) const RESIDUAL_LAW_STATUS_ID: &str = "executed_and_bound";
pub(super) const RESIDUAL_SLOT_STATUS_ID: &str = "collision_checked_unique";
pub(super) const MAX_CANONICAL_BYTES: usize = 1_048_576;
pub(super) const MAX_RATIONAL_COMPONENT_BYTES: usize = 512;

#[derive(Debug, Clone)]
pub(in crate::canonical::stellar_birth_species) struct ExactInterval {
    pub(in crate::canonical::stellar_birth_species) lower: BigRat,
    pub(in crate::canonical::stellar_birth_species) upper: BigRat,
}

impl PartialEq for ExactInterval {
    fn eq(&self, other: &Self) -> bool {
        self.lower.cmp_rat(&other.lower) == Ordering::Equal
            && self.upper.cmp_rat(&other.upper) == Ordering::Equal
    }
}

impl Eq for ExactInterval {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NeutralBoundProfilePacket {
    pub(super) schema_id: String,
    pub(super) profile_id: String,
    pub(super) theory_class_id: String,
    pub(super) interaction_id: String,
    pub(super) state_id: String,
    pub(super) validity_id: String,
    pub(super) normalization_id: String,
    pub(super) convergence_id: String,
    pub(super) conservation_id: String,
    pub(super) separation_id: String,
    pub(super) decay_id: String,
    pub(super) stability_id: String,
    pub(super) transition_id: String,
    pub(super) member_class_id: String,
    pub(super) residual_slot_id: String,
    pub(super) owner_admission_record: String,
    pub(super) evidence_primary_citation: String,
    pub(super) evidence_primary_url: String,
    pub(super) evidence_primary_sha256_hex: String,
    pub(super) evidence_primary_anchor: String,
    pub(super) evidence_secondary_citation: String,
    pub(super) evidence_secondary_url: String,
    pub(super) evidence_secondary_sha256_hex: String,
    pub(super) evidence_secondary_anchor: String,
    pub(super) floor_authority: super::model::ReceiptBinding,
    pub(super) root_pair_receipt: super::model::ReceiptBinding,
    pub(super) mass_source_entry_id: String,
    pub(super) mass_scalar_identity: ArtifactIdentity,
    pub(super) mass_scalar_ancestry_sha256: [u8; 32],
    pub(super) mass_value_decimal: String,
    pub(super) mass_uncertainty_decimal: String,
    pub(super) coupling_source_entry_id: String,
    pub(super) coupling_scalar_identity: ArtifactIdentity,
    pub(super) coupling_scalar_ancestry_sha256: [u8; 32],
    pub(super) coupling_value_decimal: String,
    pub(super) coupling_uncertainty_decimal: String,
    pub(super) primitive_profile_receipt: super::model::ReceiptBinding,
    pub(super) primitive_profile_root_identity: ArtifactIdentity,
    pub(super) primitive_sector_identity: ArtifactIdentity,
    pub(super) primitive_member: SpeciesContentIdentity,
    pub(super) primitive_residual_slot_id: String,
    pub(super) charged_profile_receipt: super::model::ReceiptBinding,
    pub(super) charged_profile_root_identity: ArtifactIdentity,
    pub(super) charged_members: Vec<SpeciesContentIdentity>,
    pub(super) charged_residual_slot_id: String,
    pub(super) global_physical_vocabulary_coverage: bool,
    pub(super) membership_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NeutralBoundArtifactCandidate {
    pub(super) identity: ArtifactIdentity,
    pub(super) admission: RootAdmission,
    pub(super) payload: ArtifactPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NeutralBoundCheckerOutput {
    pub(super) input_sha256: [u8; 32],
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) candidates: Vec<NeutralBoundArtifactCandidate>,
    pub(super) member: SpeciesContentIdentity,
    pub(super) profile_root_identity: ArtifactIdentity,
    pub(super) profile_role_identity: ArtifactIdentity,
    pub(super) constituent_threshold_channel_identity: [u8; 32],
    pub(super) decay_channel_family_identity: [u8; 32],
    pub(super) candidate_mass_interval: ExactInterval,
    pub(super) constituent_threshold_interval: ExactInterval,
    pub(super) decay_threshold_interval: ExactInterval,
    pub(super) dimensionless_ground_energy: BigRat,
    pub(super) reduced_mass_factor: BigRat,
    pub(super) binding_mass_factor: BigRat,
    pub(super) normalization_value: BigRat,
    pub(super) radial_residual_norm: BigRat,
    pub(super) evidence_custody_receipt_sha256: [u8; 32],
    pub(super) solver_producer_sha256: [u8; 32],
    pub(super) solver_watchdog_sha256: [u8; 32],
    pub(super) normalization_producer_sha256: [u8; 32],
    pub(super) normalization_watchdog_sha256: [u8; 32],
    pub(super) threshold_binding_sha256: [u8; 32],
    pub(super) threshold_coverage_producer_sha256: [u8; 32],
    pub(super) threshold_coverage_watchdog_sha256: [u8; 32],
    pub(super) uncertainty_transport_producer_sha256: [u8; 32],
    pub(super) uncertainty_transport_watchdog_sha256: [u8; 32],
    pub(super) conservation_producer_sha256: [u8; 32],
    pub(super) conservation_watchdog_sha256: [u8; 32],
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
pub(in crate::canonical::stellar_birth_species) enum NeutralBoundProfileRefusal {
    SealedSourceUnavailable,
    PacketSchemaMismatch,
    ProfileIdentityMismatch,
    EvidenceCustodyMismatch,
    FloorBindingMismatch,
    RootBindingMismatch,
    PrimitiveProfileBindingMismatch,
    ChargedProfileBindingMismatch,
    ResidualSlotCollision,
    UnsupportedScope,
    ExactArithmeticFailure,
    InvalidValidityDomain,
    SolverEvidenceMismatch,
    NormalizationMismatch,
    ConservationMismatch,
    ThresholdCoverageMismatch,
    UncertaintyTransportMismatch,
    ArtifactConstructionFailure,
    ArtifactCountMismatch,
    MemberIdentityMismatch,
    CanaryFailure,
    CheckerDisagreement,
    PairReceiptMismatch,
}

impl NeutralBoundProfileRefusal {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::SealedSourceUnavailable => "sealed_source_unavailable",
            Self::PacketSchemaMismatch => "packet_schema_mismatch",
            Self::ProfileIdentityMismatch => "profile_identity_mismatch",
            Self::EvidenceCustodyMismatch => "evidence_custody_mismatch",
            Self::FloorBindingMismatch => "floor_binding_mismatch",
            Self::RootBindingMismatch => "root_binding_mismatch",
            Self::PrimitiveProfileBindingMismatch => "primitive_profile_binding_mismatch",
            Self::ChargedProfileBindingMismatch => "charged_profile_binding_mismatch",
            Self::ResidualSlotCollision => "residual_slot_collision",
            Self::UnsupportedScope => "unsupported_scope",
            Self::ExactArithmeticFailure => "exact_arithmetic_failure",
            Self::InvalidValidityDomain => "invalid_validity_domain",
            Self::SolverEvidenceMismatch => "solver_evidence_mismatch",
            Self::NormalizationMismatch => "normalization_mismatch",
            Self::ConservationMismatch => "conservation_mismatch",
            Self::ThresholdCoverageMismatch => "threshold_coverage_mismatch",
            Self::UncertaintyTransportMismatch => "uncertainty_transport_mismatch",
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
pub(super) struct NeutralBoundProtocolReceipt {
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
pub(super) struct NeutralBoundProfileReceipt {
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
    pub(super) member: SpeciesContentIdentity,
    pub(super) artifact_count: u32,
    pub(super) evidence_custody_receipt_sha256: [u8; 32],
    pub(super) solver_producer_sha256: [u8; 32],
    pub(super) solver_watchdog_sha256: [u8; 32],
    pub(super) normalization_producer_sha256: [u8; 32],
    pub(super) normalization_watchdog_sha256: [u8; 32],
    pub(super) threshold_coverage_producer_sha256: [u8; 32],
    pub(super) threshold_coverage_watchdog_sha256: [u8; 32],
    pub(super) uncertainty_transport_producer_sha256: [u8; 32],
    pub(super) uncertainty_transport_watchdog_sha256: [u8; 32],
    pub(super) conservation_producer_sha256: [u8; 32],
    pub(super) conservation_watchdog_sha256: [u8; 32],
    pub(super) constituent_threshold_channel_identity: [u8; 32],
    pub(super) decay_channel_family_identity: [u8; 32],
    pub(super) binding_disposition_id: &'static str,
    pub(super) decay_disposition_id: &'static str,
    pub(super) mass_interval_sha256: [u8; 32],
    pub(super) threshold_interval_sha256: [u8; 32],
    pub(super) protocol: NeutralBoundProtocolReceipt,
    pub(super) global_physical_vocabulary_coverage: bool,
    pub(super) membership_authority: bool,
    pub(super) conditioned_support_authority: bool,
    pub(super) global_stability_claim: bool,
    pub(super) authority_effect: &'static str,
    pub(super) pair_receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NeutralBoundProfileProjection {
    pub(super) candidate_artifacts: Vec<NeutralBoundArtifactCandidate>,
    pub(super) member: SpeciesContentIdentity,
    pub(super) receipt: NeutralBoundProfileReceipt,
}

/// Opaque proof that one neutral-bound-profile artifact came from the agreed
/// repository pair.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct NeutralBoundProfileAdmissionCapability {
    claimed_identity: ArtifactIdentity,
    admission: RootAdmission,
    profile_root_identity: ArtifactIdentity,
    pair_receipt_sha256: [u8; 32],
}

impl NeutralBoundProfileAdmissionCapability {
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
pub(super) struct AdmittedNeutralBoundProfile {
    pub(super) admitted_artifacts: Vec<super::model::AdmittedArtifact>,
    pub(super) member: SpeciesContentIdentity,
    pub(super) receipt: NeutralBoundProfileReceipt,
}

/// Claim-scoped proof consumed only by the exact threshold arithmetic pair.
pub(in crate::canonical::stellar_birth_species) struct ThresholdCoverageCapability {
    binding_sha256: [u8; 32],
    producer_receipt_sha256: [u8; 32],
    watchdog_receipt_sha256: [u8; 32],
}

impl ThresholdCoverageCapability {
    pub(in crate::canonical::stellar_birth_species) const fn binding_sha256(&self) -> [u8; 32] {
        self.binding_sha256
    }

    pub(in crate::canonical::stellar_birth_species) const fn producer_receipt_sha256(
        &self,
    ) -> [u8; 32] {
        self.producer_receipt_sha256
    }

    pub(in crate::canonical::stellar_birth_species) const fn watchdog_receipt_sha256(
        &self,
    ) -> [u8; 32] {
        self.watchdog_receipt_sha256
    }
}

// @derives: one neutral two-body mass interval and strict free-constituent separation <- the sealed m_e and alpha coordinates, the admitted charged orbit, the admitted abelian carrier sector, and the claim-local leading central interaction
pub(super) fn construct_profile_projection(
) -> Result<NeutralBoundProfileProjection, NeutralBoundProfileRefusal> {
    let producer_packet = producer::sealed_packet()?;
    let watchdog_packet = watchdog::sealed_packet()?;
    if producer_packet != watchdog_packet {
        return Err(NeutralBoundProfileRefusal::CheckerDisagreement);
    }
    let produced = producer::inspect_against_sealed(&producer_packet, &producer_packet)?;
    let watched = watchdog::inspect_against_sealed(&watchdog_packet, &watchdog_packet)?;
    if produced != watched
        || produced.candidates.len() != ARTIFACT_COUNT
        || MEMBER_COUNT != 1
        || produced.profile_root_identity.0 == [0; 32]
        || produced.profile_role_identity.0 == [0; 32]
        || produced.admission_evidence.decision_id != "irreducible_protocol_structurally_bound"
    {
        return Err(NeutralBoundProfileRefusal::CheckerDisagreement);
    }

    let threshold_binding_sha256 = threshold_binding_sha256(
        &produced.candidate_mass_interval,
        produced.constituent_threshold_channel_identity,
        &produced.constituent_threshold_interval,
    )?;
    if threshold_binding_sha256 != produced.threshold_binding_sha256
        || threshold_binding_sha256 != watched.threshold_binding_sha256
        || produced.threshold_coverage_producer_sha256 == [0; 32]
        || produced.threshold_coverage_watchdog_sha256 == [0; 32]
        || produced.threshold_coverage_producer_sha256
            == produced.threshold_coverage_watchdog_sha256
    {
        return Err(NeutralBoundProfileRefusal::ThresholdCoverageMismatch);
    }
    let threshold_capability = ThresholdCoverageCapability {
        binding_sha256: threshold_binding_sha256,
        producer_receipt_sha256: produced.threshold_coverage_producer_sha256,
        watchdog_receipt_sha256: produced.threshold_coverage_watchdog_sha256,
    };
    let threshold_report = super::super::bound_state_threshold::inspect_authorized_thresholds(
        &produced.candidate_mass_interval.lower,
        &produced.candidate_mass_interval.upper,
        &[(
            produced.constituent_threshold_channel_identity,
            produced.constituent_threshold_interval.lower.clone(),
            produced.constituent_threshold_interval.upper.clone(),
        )],
        &[produced.constituent_threshold_channel_identity],
        &threshold_capability,
    )
    .map_err(|_| NeutralBoundProfileRefusal::ThresholdCoverageMismatch)?;
    if threshold_report.disposition_id() != "strictly_below_all_thresholds"
        || threshold_report.authority_effect() != "none"
        || produced
            .candidate_mass_interval
            .lower
            .cmp_rat(&BigRat::from_i64(0))
            != Ordering::Greater
        || produced
            .candidate_mass_interval
            .lower
            .cmp_rat(&produced.decay_threshold_interval.upper)
            != Ordering::Greater
    {
        return Err(NeutralBoundProfileRefusal::ThresholdCoverageMismatch);
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
        return Err(NeutralBoundProfileRefusal::CanaryFailure);
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
        return Err(NeutralBoundProfileRefusal::PairReceiptMismatch);
    }
    let protocol = extract_protocol_receipt(&produced.candidates, produced.profile_root_identity)?;
    let artifact_count = u32::try_from(produced.candidates.len())
        .map_err(|_| NeutralBoundProfileRefusal::ArtifactCountMismatch)?;
    let mass_interval_sha256 = interval_sha256(
        b"civsim.planet.neutral-bound-profile.mass-interval.v1",
        &produced.candidate_mass_interval,
    )?;
    let threshold_interval_sha256 = interval_sha256(
        b"civsim.planet.neutral-bound-profile.threshold-interval.v1",
        &produced.constituent_threshold_interval,
    )?;
    Ok(NeutralBoundProfileProjection {
        candidate_artifacts: produced.candidates,
        member: produced.member,
        receipt: NeutralBoundProfileReceipt {
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
            member: produced.member,
            artifact_count,
            evidence_custody_receipt_sha256: produced.evidence_custody_receipt_sha256,
            solver_producer_sha256: produced.solver_producer_sha256,
            solver_watchdog_sha256: produced.solver_watchdog_sha256,
            normalization_producer_sha256: produced.normalization_producer_sha256,
            normalization_watchdog_sha256: produced.normalization_watchdog_sha256,
            threshold_coverage_producer_sha256: produced.threshold_coverage_producer_sha256,
            threshold_coverage_watchdog_sha256: produced.threshold_coverage_watchdog_sha256,
            uncertainty_transport_producer_sha256: produced.uncertainty_transport_producer_sha256,
            uncertainty_transport_watchdog_sha256: produced.uncertainty_transport_watchdog_sha256,
            conservation_producer_sha256: produced.conservation_producer_sha256,
            conservation_watchdog_sha256: produced.conservation_watchdog_sha256,
            constituent_threshold_channel_identity: produced.constituent_threshold_channel_identity,
            decay_channel_family_identity: produced.decay_channel_family_identity,
            binding_disposition_id: "strictly_below_free_constituent_threshold",
            decay_disposition_id: "energetically_open_neutral_massless_carrier_family",
            mass_interval_sha256,
            threshold_interval_sha256,
            protocol,
            global_physical_vocabulary_coverage: false,
            membership_authority: false,
            conditioned_support_authority: false,
            global_stability_claim: false,
            authority_effect: "none",
            pair_receipt_sha256: producer_receipt,
        },
    })
}

pub(super) fn project_admitted_profile(
) -> Result<AdmittedNeutralBoundProfile, NeutralBoundProfileRefusal> {
    let projection = construct_profile_projection()?;
    if projection.receipt.pair_receipt_sha256 == [0; 32]
        || projection.receipt.authority_effect != "none"
        || projection.receipt.global_physical_vocabulary_coverage
        || projection.receipt.membership_authority
        || projection.receipt.conditioned_support_authority
        || projection.receipt.global_stability_claim
        || projection.receipt.protocol.derive_first_status_id != DERIVE_FIRST_STATUS_ID
        || projection.receipt.protocol.buckingham_pi_status_id != BUCKINGHAM_PI_STATUS_ID
        || projection.receipt.protocol.gap_law_status_id != GAP_LAW_STATUS_ID
        || projection.receipt.protocol.chaos_protocol_status_id != CHAOS_PROTOCOL_STATUS_ID
        || projection.receipt.protocol.residual_law_status_id != RESIDUAL_LAW_STATUS_ID
        || projection.receipt.protocol.residual_slot_status_id != RESIDUAL_SLOT_STATUS_ID
    {
        return Err(NeutralBoundProfileRefusal::PairReceiptMismatch);
    }
    let profile_root_identity = projection.receipt.profile_root_identity;
    let pair_receipt_sha256 = projection.receipt.pair_receipt_sha256;
    let admitted_artifacts = projection
        .candidate_artifacts
        .into_iter()
        .map(|candidate| {
            let capability = NeutralBoundProfileAdmissionCapability {
                claimed_identity: candidate.identity,
                admission: candidate.admission.clone(),
                profile_root_identity,
                pair_receipt_sha256,
            };
            super::model::AdmittedArtifact::from_neutral_bound_profile(
                candidate.identity,
                candidate.admission,
                candidate.payload,
                capability,
            )
        })
        .collect();
    Ok(AdmittedNeutralBoundProfile {
        admitted_artifacts,
        member: projection.member,
        receipt: projection.receipt,
    })
}

fn extract_protocol_receipt(
    candidates: &[NeutralBoundArtifactCandidate],
    profile_root_identity: ArtifactIdentity,
) -> Result<NeutralBoundProtocolReceipt, NeutralBoundProfileRefusal> {
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.identity == profile_root_identity)
        .ok_or(NeutralBoundProfileRefusal::ArtifactConstructionFailure)?;
    let AdmissionRoute::Irreducible(route) = &candidate.admission.route else {
        return Err(NeutralBoundProfileRefusal::ArtifactConstructionFailure);
    };
    if route.residual_slot_id != RESIDUAL_SLOT_ID {
        return Err(NeutralBoundProfileRefusal::ArtifactConstructionFailure);
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
        return Err(NeutralBoundProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(NeutralBoundProtocolReceipt {
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

pub(in crate::canonical::stellar_birth_species) fn threshold_binding_sha256(
    candidate: &ExactInterval,
    channel_identity: [u8; 32],
    threshold: &ExactInterval,
) -> Result<[u8; 32], NeutralBoundProfileRefusal> {
    let mut bytes = Vec::new();
    append_field(
        &mut bytes,
        1,
        b"civsim.planet.neutral-bound-profile.threshold-binding.v1",
    );
    append_field(&mut bytes, 2, &encode_interval(candidate)?);
    append_field(&mut bytes, 3, &channel_identity);
    append_field(&mut bytes, 4, &encode_interval(threshold)?);
    Ok(sha256(&bytes))
}

pub(super) fn interval_sha256(
    domain: &[u8],
    interval: &ExactInterval,
) -> Result<[u8; 32], NeutralBoundProfileRefusal> {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, domain);
    append_field(&mut bytes, 2, &encode_interval(interval)?);
    Ok(sha256(&bytes))
}

pub(super) fn encode_interval(
    interval: &ExactInterval,
) -> Result<Vec<u8>, NeutralBoundProfileRefusal> {
    if interval.lower.cmp_rat(&interval.upper) == Ordering::Greater {
        return Err(NeutralBoundProfileRefusal::ExactArithmeticFailure);
    }
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, &encode_rational(&interval.lower)?);
    append_field(&mut bytes, 2, &encode_rational(&interval.upper)?);
    Ok(bytes)
}

pub(super) fn encode_rational(value: &BigRat) -> Result<Vec<u8>, NeutralBoundProfileRefusal> {
    let (negative, numerator, denominator) = value
        .bounded_canonical_components(MAX_RATIONAL_COMPONENT_BYTES)
        .map_err(|_| NeutralBoundProfileRefusal::ExactArithmeticFailure)?;
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, &[u8::from(negative)]);
    append_field(&mut bytes, 2, &numerator);
    append_field(&mut bytes, 3, &denominator);
    Ok(bytes)
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

/// Shared wire grammar only. The producer and watchdog do not share packet
/// validation, exact solving, interval propagation, artifact construction, or
/// canary logic.
pub(super) fn encode_packet(
    packet: &NeutralBoundProfilePacket,
) -> Result<Vec<u8>, NeutralBoundProfileRefusal> {
    let mut bytes = b"civsim.planet.neutral-bound-profile.packet-wire.v1".to_vec();
    let strings = [
        packet.schema_id.as_bytes(),
        packet.profile_id.as_bytes(),
        packet.theory_class_id.as_bytes(),
        packet.interaction_id.as_bytes(),
        packet.state_id.as_bytes(),
        packet.validity_id.as_bytes(),
        packet.normalization_id.as_bytes(),
        packet.convergence_id.as_bytes(),
        packet.conservation_id.as_bytes(),
        packet.separation_id.as_bytes(),
        packet.decay_id.as_bytes(),
        packet.stability_id.as_bytes(),
        packet.transition_id.as_bytes(),
        packet.member_class_id.as_bytes(),
        packet.residual_slot_id.as_bytes(),
        packet.owner_admission_record.as_bytes(),
        packet.evidence_primary_citation.as_bytes(),
        packet.evidence_primary_url.as_bytes(),
        packet.evidence_primary_sha256_hex.as_bytes(),
        packet.evidence_primary_anchor.as_bytes(),
        packet.evidence_secondary_citation.as_bytes(),
        packet.evidence_secondary_url.as_bytes(),
        packet.evidence_secondary_sha256_hex.as_bytes(),
        packet.evidence_secondary_anchor.as_bytes(),
    ];
    for (index, value) in strings.into_iter().enumerate() {
        let tag = u16::try_from(index + 1)
            .map_err(|_| NeutralBoundProfileRefusal::ArtifactConstructionFailure)?;
        append_field(&mut bytes, tag, value);
    }
    let mut tag = 25_u16;
    for payload in [
        packet.floor_authority.schema_id.as_bytes(),
        packet.floor_authority.digest_sha256.as_slice(),
        packet.root_pair_receipt.schema_id.as_bytes(),
        packet.root_pair_receipt.digest_sha256.as_slice(),
        packet.mass_source_entry_id.as_bytes(),
        packet.mass_scalar_identity.0.as_slice(),
        packet.mass_scalar_ancestry_sha256.as_slice(),
        packet.mass_value_decimal.as_bytes(),
        packet.mass_uncertainty_decimal.as_bytes(),
        packet.coupling_source_entry_id.as_bytes(),
        packet.coupling_scalar_identity.0.as_slice(),
        packet.coupling_scalar_ancestry_sha256.as_slice(),
        packet.coupling_value_decimal.as_bytes(),
        packet.coupling_uncertainty_decimal.as_bytes(),
        packet.primitive_profile_receipt.schema_id.as_bytes(),
        packet.primitive_profile_receipt.digest_sha256.as_slice(),
        packet.primitive_profile_root_identity.0.as_slice(),
        packet.primitive_sector_identity.0.as_slice(),
        packet.primitive_member.0.as_slice(),
        packet.primitive_residual_slot_id.as_bytes(),
        packet.charged_profile_receipt.schema_id.as_bytes(),
        packet.charged_profile_receipt.digest_sha256.as_slice(),
        packet.charged_profile_root_identity.0.as_slice(),
    ] {
        append_field(&mut bytes, tag, payload);
        tag = tag
            .checked_add(1)
            .ok_or(NeutralBoundProfileRefusal::ArtifactConstructionFailure)?;
    }
    for member in &packet.charged_members {
        append_field(&mut bytes, tag, &member.0);
    }
    tag = tag
        .checked_add(1)
        .ok_or(NeutralBoundProfileRefusal::ArtifactConstructionFailure)?;
    append_field(&mut bytes, tag, packet.charged_residual_slot_id.as_bytes());
    append_field(
        &mut bytes,
        tag.checked_add(1)
            .ok_or(NeutralBoundProfileRefusal::ArtifactConstructionFailure)?,
        &[u8::from(packet.global_physical_vocabulary_coverage)],
    );
    append_field(
        &mut bytes,
        tag.checked_add(2)
            .ok_or(NeutralBoundProfileRefusal::ArtifactConstructionFailure)?,
        &[u8::from(packet.membership_authority)],
    );
    if bytes.len() > MAX_CANONICAL_BYTES {
        return Err(NeutralBoundProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(bytes)
}

pub(super) fn encode_checker_output(
    output: &NeutralBoundCheckerOutput,
) -> Result<Vec<u8>, NeutralBoundProfileRefusal> {
    let mut bytes = b"civsim.planet.neutral-bound-profile.output-wire.v1".to_vec();
    append_field(&mut bytes, 1, &output.input_sha256);
    append_field(&mut bytes, 2, &output.profile_root_identity.0);
    append_field(&mut bytes, 3, &output.profile_role_identity.0);
    for candidate in &output.candidates {
        let mut artifact = candidate.identity.0.to_vec();
        artifact.extend_from_slice(&encode_admission(&candidate.admission));
        append_field(&mut bytes, 4, &artifact);
    }
    append_field(&mut bytes, 5, &output.member.0);
    append_field(
        &mut bytes,
        6,
        &output.constituent_threshold_channel_identity,
    );
    append_field(&mut bytes, 7, &output.decay_channel_family_identity);
    append_field(
        &mut bytes,
        8,
        &encode_interval(&output.candidate_mass_interval)?,
    );
    append_field(
        &mut bytes,
        9,
        &encode_interval(&output.constituent_threshold_interval)?,
    );
    append_field(
        &mut bytes,
        10,
        &encode_interval(&output.decay_threshold_interval)?,
    );
    for (tag, value) in [
        (11, &output.dimensionless_ground_energy),
        (12, &output.reduced_mass_factor),
        (13, &output.binding_mass_factor),
        (14, &output.normalization_value),
        (15, &output.radial_residual_norm),
    ] {
        append_field(&mut bytes, tag, &encode_rational(value)?);
    }
    for (tag, digest) in [
        (16, output.evidence_custody_receipt_sha256),
        (17, output.solver_producer_sha256),
        (18, output.solver_watchdog_sha256),
        (19, output.normalization_producer_sha256),
        (20, output.normalization_watchdog_sha256),
        (21, output.threshold_binding_sha256),
        (22, output.threshold_coverage_producer_sha256),
        (23, output.threshold_coverage_watchdog_sha256),
        (24, output.uncertainty_transport_producer_sha256),
        (25, output.uncertainty_transport_watchdog_sha256),
        (26, output.conservation_producer_sha256),
        (27, output.conservation_watchdog_sha256),
        (28, output.applicability_receipt_sha256),
        (29, output.validity_receipt_sha256),
        (
            30,
            output
                .admission_evidence
                .irreducible_protocol_capability_sha256,
        ),
        (
            31,
            output
                .admission_evidence
                .profile_protocol_pair_receipt_sha256,
        ),
    ] {
        append_field(&mut bytes, tag, &digest);
    }
    if bytes.len() > MAX_CANONICAL_BYTES {
        return Err(NeutralBoundProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(bytes)
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
            append_field(&mut bytes, 4, &route.ancestry_receipt.digest_sha256);
            append_field(&mut bytes, 5, &route.semantic_checker_receipt.digest_sha256);
            append_field(
                &mut bytes,
                6,
                &route.independent_watchdog_receipt.digest_sha256,
            );
        }
        AdmissionRoute::Irreducible(route) => {
            append_field(&mut bytes, 3, b"irreducible");
            append_field(
                &mut bytes,
                4,
                &route.derivation_exhaustion_receipt.digest_sha256,
            );
            append_field(&mut bytes, 5, &route.buckingham_pi_receipt.digest_sha256);
            append_field(&mut bytes, 6, &route.gap_law_receipt.digest_sha256);
            append_field(&mut bytes, 7, &route.chaos_protocol_receipt.digest_sha256);
            append_field(&mut bytes, 8, &route.residual_law_receipt.digest_sha256);
            append_field(&mut bytes, 9, route.residual_slot_id.as_bytes());
            append_field(&mut bytes, 10, &route.residual_slot_receipt.digest_sha256);
            append_field(&mut bytes, 11, &route.owner_admission_receipt.digest_sha256);
            append_field(
                &mut bytes,
                12,
                &route.independent_watchdog_receipt.digest_sha256,
            );
        }
        AdmissionRoute::EvidenceCustodyOnly { source_receipt } => {
            append_field(&mut bytes, 3, b"evidence-custody-only");
            append_field(&mut bytes, 4, &source_receipt.digest_sha256);
        }
    }
    bytes
}

pub(super) fn digest_fields(domain: &[u8], fields: &[&[u8]]) -> [u8; 32] {
    let mut bytes = domain.to_vec();
    for (index, field) in fields.iter().enumerate() {
        append_field(
            &mut bytes,
            u16::try_from(index + 1).unwrap_or(u16::MAX),
            field,
        );
    }
    sha256(&bytes)
}

pub(super) fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
