//! One claim-scoped derived execution-relation premise.
//!
//! This authority binds the sealed `eps_0` execution coordinate to the exact
//! `e^2/(2*alpha*h*c)` derivation already contained in the physical-floor
//! authority. It does not admit an electromagnetic field, Maxwell dynamics, a
//! gauge sector, a species, uncertainty transport, or global vocabulary.

mod model;
mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use super::{
    inspect_law_premises, CandidateProof, ClaimScopedPremiseCapability, LawPremiseRequest,
    LawPremiseRequirement, PremiseCapabilitySeal, PremiseKey, RequirementProof,
};
use civsim_ledger::{Provenance, Tier};
use civsim_units::digest::sha256;
use model::{
    DerivedRelationCheckerOutput, DerivedRelationRefusal, ARTIFACT_SCHEMA_ID, CANARY_SUITE_ID,
    PRODUCER_CANARY_TRANSCRIPT_ID, RECEIPT_SCHEMA_ID, WATCHDOG_CANARY_TRANSCRIPT_ID,
};

const MAX_FIELD_BYTES: usize = 65_536;
const PREMISE_ID: &str = "planet.derived-law-premise-eps0";
const OUTPUT_SYMBOL: &str = "eps_0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DerivedRelationPremiseArtifact {
    schema_id: &'static str,
    canonical_relation_bytes: Vec<u8>,
    output_bits: i128,
    output_scale_bits: u32,
    output_projection_receipt_sha256: [u8; 32],
    ancestry_receipt_sha256: [u8; 32],
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
    producer_canary_transcript_id: &'static str,
    producer_canary_case_count: u32,
    producer_canary_sha256: [u8; 32],
    watchdog_canary_transcript_id: &'static str,
    watchdog_canary_case_count: u32,
    watchdog_canary_sha256: [u8; 32],
    pair_receipt_sha256: [u8; 32],
    capability: ClaimScopedPremiseCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::canonical::stellar_birth_species) struct RepositoryDerivedRelationPremiseFrontier {
    pub(in crate::canonical::stellar_birth_species) premise_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) artifact_schema_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) receipt_schema_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) canary_suite_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) output_symbol: &'static str,
    pub(in crate::canonical::stellar_birth_species) output_bits: i128,
    pub(in crate::canonical::stellar_birth_species) output_scale_bits: u32,
    pub(in crate::canonical::stellar_birth_species) output_projection_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) canonical_relation_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) producer_implementation_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) watchdog_implementation_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) producer_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) watchdog_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) producer_canary_transcript_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) producer_canary_case_count: u32,
    pub(in crate::canonical::stellar_birth_species) producer_canary_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) watchdog_canary_transcript_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) watchdog_canary_case_count: u32,
    pub(in crate::canonical::stellar_birth_species) watchdog_canary_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) decision_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) claim_identity_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) role_identity_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) content_identity_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) upstream_capability_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) applicability_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) validity_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) ancestry_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) pair_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) capability_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) tier_id: &'static str,
    pub(in crate::canonical::stellar_birth_species) provenance_tag: &'static str,
    pub(in crate::canonical::stellar_birth_species) content_proof_kind: &'static str,
    pub(in crate::canonical::stellar_birth_species) requested_premise_coverage: bool,
    pub(in crate::canonical::stellar_birth_species) species_membership_authority: bool,
    pub(in crate::canonical::stellar_birth_species) global_physical_premise_coverage: bool,
    pub(in crate::canonical::stellar_birth_species) authority_effect: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::canonical::stellar_birth_species) struct RepositoryDerivedRelationPremiseError {
    pub(in crate::canonical::stellar_birth_species) code: &'static str,
}

impl DerivedRelationPremiseArtifact {
    pub(super) const fn schema_id(&self) -> &'static str {
        self.schema_id
    }

    pub(super) fn canonical_relation_bytes(&self) -> &[u8] {
        &self.canonical_relation_bytes
    }

    pub(super) const fn output_bits(&self) -> i128 {
        self.output_bits
    }

    pub(super) const fn output_scale_bits(&self) -> u32 {
        self.output_scale_bits
    }

    pub(super) const fn output_projection_receipt_sha256(&self) -> [u8; 32] {
        self.output_projection_receipt_sha256
    }

    pub(super) const fn pair_receipt_sha256(&self) -> [u8; 32] {
        self.pair_receipt_sha256
    }

    pub(super) const fn producer_canary_sha256(&self) -> [u8; 32] {
        self.producer_canary_sha256
    }

    pub(super) const fn watchdog_canary_sha256(&self) -> [u8; 32] {
        self.watchdog_canary_sha256
    }

    pub(super) const fn species_membership_authority(&self) -> bool {
        false
    }

    pub(super) const fn global_physical_premise_coverage(&self) -> bool {
        false
    }
}

fn resolve_repository_derived_relation_premise(
) -> Result<DerivedRelationPremiseArtifact, DerivedRelationRefusal> {
    let producer_packet = producer::sealed_packet()?;
    let watchdog_packet = watchdog::sealed_packet()?;
    let produced = producer::inspect(&producer_packet)?;
    let watched = watchdog::inspect(&watchdog_packet)?;
    if producer_packet != watchdog_packet || produced != watched {
        return Err(DerivedRelationRefusal::CheckerDisagreement);
    }
    let producer_canary = producer::canary_evidence(&producer_packet)?;
    let watchdog_canary = watchdog::canary_evidence(&watchdog_packet)?;
    if producer_canary.transcript_id != PRODUCER_CANARY_TRANSCRIPT_ID
        || watchdog_canary.transcript_id != WATCHDOG_CANARY_TRANSCRIPT_ID
        || producer_canary.transcript_id == watchdog_canary.transcript_id
        || producer_canary.case_count == 0
        || watchdog_canary.case_count == 0
        || producer_canary.transcript_sha256 == [0; 32]
        || watchdog_canary.transcript_sha256 == [0; 32]
    {
        return Err(DerivedRelationRefusal::CanaryFailure);
    }
    let producer_result = producer::result_sha256(&produced);
    let watchdog_result = watchdog::result_sha256(&watched);
    if producer_result == [0; 32] || watchdog_result == [0; 32] {
        return Err(DerivedRelationRefusal::CheckerDisagreement);
    }
    let producer_receipt = producer::pair_receipt_digest(
        &produced,
        producer_result,
        watchdog_result,
        producer_canary,
        watchdog_canary,
    );
    let watchdog_receipt = watchdog::pair_receipt_digest(
        &watched,
        producer_result,
        watchdog_result,
        producer_canary,
        watchdog_canary,
    );
    if producer_receipt == [0; 32] || producer_receipt != watchdog_receipt {
        return Err(DerivedRelationRefusal::CheckerDisagreement);
    }

    let capability = mint_capability(&produced, producer_receipt)?;
    let selection = inspect_law_premises(&LawPremiseRequest {
        claim_identity: produced.claim_identity,
        requirements: vec![LawPremiseRequirement {
            key: PremiseKey {
                role: produced.role_identity,
                content: produced.content_identity,
            },
            proof: RequirementProof::VerifiedDerivedContent,
        }],
        candidates: vec![capability],
    })
    .map_err(|_| DerivedRelationRefusal::SelectionFailure)?;
    if selection.selection.selected.len() != 1
        || selection.selection.selected[0].capability_sha256 != capability.capability_sha256
        || !selection.requested_requirement_coverage()
        || selection.global_physical_premise_coverage()
        || selection.authority_effect() != "none"
    {
        return Err(DerivedRelationRefusal::SelectionFailure);
    }

    Ok(DerivedRelationPremiseArtifact {
        schema_id: ARTIFACT_SCHEMA_ID,
        canonical_relation_bytes: produced.relation_bytes,
        output_bits: producer_packet.output.bits,
        output_scale_bits: producer_packet.output.scale_bits,
        output_projection_receipt_sha256: producer_packet.output.projection_receipt_sha256,
        ancestry_receipt_sha256: produced.ancestry_receipt_sha256,
        producer_result_sha256: producer_result,
        watchdog_result_sha256: watchdog_result,
        producer_canary_transcript_id: producer_canary.transcript_id,
        producer_canary_case_count: producer_canary.case_count,
        producer_canary_sha256: producer_canary.transcript_sha256,
        watchdog_canary_transcript_id: watchdog_canary.transcript_id,
        watchdog_canary_case_count: watchdog_canary.case_count,
        watchdog_canary_sha256: watchdog_canary.transcript_sha256,
        pair_receipt_sha256: producer_receipt,
        capability,
    })
}

pub(in crate::canonical::stellar_birth_species) fn repository_derived_relation_premise_frontier(
) -> Result<RepositoryDerivedRelationPremiseFrontier, RepositoryDerivedRelationPremiseError> {
    let artifact = resolve_repository_derived_relation_premise()
        .map_err(|error| RepositoryDerivedRelationPremiseError { code: error.id() })?;
    Ok(RepositoryDerivedRelationPremiseFrontier {
        premise_id: PREMISE_ID,
        artifact_schema_id: artifact.schema_id,
        receipt_schema_id: RECEIPT_SCHEMA_ID,
        canary_suite_id: CANARY_SUITE_ID,
        output_symbol: OUTPUT_SYMBOL,
        output_bits: artifact.output_bits,
        output_scale_bits: artifact.output_scale_bits,
        output_projection_receipt_sha256: artifact.output_projection_receipt_sha256,
        canonical_relation_sha256: sha256(&artifact.canonical_relation_bytes),
        producer_implementation_id: producer::implementation_id(),
        watchdog_implementation_id: watchdog::implementation_id(),
        producer_result_sha256: artifact.producer_result_sha256,
        watchdog_result_sha256: artifact.watchdog_result_sha256,
        producer_canary_transcript_id: artifact.producer_canary_transcript_id,
        producer_canary_case_count: artifact.producer_canary_case_count,
        producer_canary_sha256: artifact.producer_canary_sha256,
        watchdog_canary_transcript_id: artifact.watchdog_canary_transcript_id,
        watchdog_canary_case_count: artifact.watchdog_canary_case_count,
        watchdog_canary_sha256: artifact.watchdog_canary_sha256,
        decision_id: "agreed",
        claim_identity_sha256: artifact.capability.claim_identity.0,
        role_identity_sha256: artifact.capability.key.role.0,
        content_identity_sha256: artifact.capability.key.content.0,
        upstream_capability_sha256: artifact.capability.upstream_capability_sha256,
        applicability_receipt_sha256: artifact.capability.applicability_receipt_sha256,
        validity_receipt_sha256: artifact.capability.validity_receipt_sha256,
        ancestry_receipt_sha256: artifact.ancestry_receipt_sha256,
        pair_receipt_sha256: artifact.pair_receipt_sha256,
        capability_sha256: artifact.capability.capability_sha256,
        tier_id: Tier::Universal.id(),
        provenance_tag: Provenance::Derived
            .bracket_tag()
            .expect("derived is one of the seven canonical provenance types"),
        content_proof_kind: "verified_derived_content",
        requested_premise_coverage: true,
        species_membership_authority: false,
        global_physical_premise_coverage: false,
        authority_effect: "none",
    })
}

fn mint_capability(
    output: &DerivedRelationCheckerOutput,
    pair_receipt_sha256: [u8; 32],
) -> Result<ClaimScopedPremiseCapability, DerivedRelationRefusal> {
    let mut capability = ClaimScopedPremiseCapability {
        claim_identity: output.claim_identity,
        key: PremiseKey {
            role: output.role_identity,
            content: output.content_identity,
        },
        upstream_capability_sha256: output.upstream_capability_sha256,
        semantic_producer_receipt_sha256: producer::semantic_receipt(output, pair_receipt_sha256),
        semantic_watchdog_receipt_sha256: watchdog::semantic_receipt(output, pair_receipt_sha256),
        applicability_receipt_sha256: output.applicability_receipt_sha256,
        validity_receipt_sha256: output.validity_receipt_sha256,
        proof: CandidateProof::VerifiedDerivedContent,
        capability_sha256: [0; 32],
        _seal: PremiseCapabilitySeal,
    };
    let produced = super::producer::capability_digest(&capability);
    let watched = super::watchdog::capability_digest(&capability);
    if produced == [0; 32] || produced != watched {
        return Err(DerivedRelationRefusal::CapabilityDigestMismatch);
    }
    capability.capability_sha256 = produced;
    Ok(capability)
}

pub(super) fn verify_current_artifact(artifact: &DerivedRelationPremiseArtifact) -> bool {
    resolve_repository_derived_relation_premise().is_ok_and(|current| &current == artifact)
}

fn append_field(
    bytes: &mut Vec<u8>,
    tag: u16,
    payload: &[u8],
) -> Result<(), DerivedRelationRefusal> {
    let next = bytes
        .len()
        .checked_add(10)
        .and_then(|length| length.checked_add(payload.len()))
        .ok_or(DerivedRelationRefusal::ResourceLimitExceeded)?;
    if next > MAX_FIELD_BYTES {
        return Err(DerivedRelationRefusal::ResourceLimitExceeded);
    }
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(payload.len())
            .map_err(|_| DerivedRelationRefusal::ResourceLimitExceeded)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
    Ok(())
}

pub(super) fn digest_fields(domain: &[u8], fields: &[&[u8]]) -> [u8; 32] {
    let mut bytes = domain.to_vec();
    for (index, field) in fields.iter().enumerate() {
        let tag = u16::try_from(index + 1).unwrap_or(u16::MAX);
        bytes.extend_from_slice(&tag.to_be_bytes());
        bytes.extend_from_slice(&u64::try_from(field.len()).unwrap_or(u64::MAX).to_be_bytes());
        bytes.extend_from_slice(field);
    }
    sha256(&bytes)
}

pub(super) const fn receipt_schema_id() -> &'static str {
    RECEIPT_SCHEMA_ID
}

pub(super) const fn canary_suite_id() -> &'static str {
    CANARY_SUITE_ID
}
