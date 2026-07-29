//! Forward categorical entailment constructor for the confining profile.

use super::*;
use crate::canonical::stellar_birth_species::law_premise;
use civsim_ledger::{Provenance, Tier};
use civsim_units::{digest::sha256, physics_floor::sealed_physical_floor_authority_binding};

const PROFILE_CLAIM_DOMAIN: &[u8] = b"civsim.planet.confining-profile.claim.v1";
const EVIDENCE_DOMAIN: &[u8] = b"civsim.planet.confining-profile.evidence-custody.v1";
const CURVATURE_PRODUCER_DOMAIN: &[u8] = b"civsim.planet.confining-profile.curvature-forward.v1";
const CURVATURE_WATCHDOG_DOMAIN: &[u8] = b"civsim.planet.confining-profile.curvature-reverse.v1";
const CONSERVATION_PRODUCER_DOMAIN: &[u8] =
    b"civsim.planet.confining-profile.conservation-forward.v1";
const CONSERVATION_WATCHDOG_DOMAIN: &[u8] =
    b"civsim.planet.confining-profile.conservation-reverse.v1";
const CONFINEMENT_PRODUCER_DOMAIN: &[u8] =
    b"civsim.planet.confining-profile.confinement-forward.v1";
const CONFINEMENT_WATCHDOG_DOMAIN: &[u8] =
    b"civsim.planet.confining-profile.confinement-reverse.v1";
const APPLICABILITY_PRODUCER_DOMAIN: &[u8] =
    b"civsim.planet.confining-profile.applicability-forward.v1";
const APPLICABILITY_WATCHDOG_DOMAIN: &[u8] =
    b"civsim.planet.confining-profile.applicability-reverse.v1";
const VALIDITY_PRODUCER_DOMAIN: &[u8] = b"civsim.planet.confining-profile.validity-forward.v1";
const VALIDITY_WATCHDOG_DOMAIN: &[u8] = b"civsim.planet.confining-profile.validity-reverse.v1";
const ASYMPTOTIC_PRODUCER_DOMAIN: &[u8] = b"civsim.planet.confining-profile.asymptotic-forward.v1";
const ASYMPTOTIC_WATCHDOG_DOMAIN: &[u8] = b"civsim.planet.confining-profile.asymptotic-reverse.v1";
const APPLICABILITY_PAIR_DOMAIN: &[u8] = b"civsim.planet.confining-profile.applicability-pair.v1";
const VALIDITY_PAIR_DOMAIN: &[u8] = b"civsim.planet.confining-profile.validity-pair.v1";
const PAIR_DOMAIN: &[u8] = b"civsim.planet.confining-profile.pair-receipt.v1";

#[derive(Debug, Clone, Copy)]
struct SemanticEvidence {
    curvature_producer: [u8; 32],
    curvature_watchdog: [u8; 32],
    conservation_producer: [u8; 32],
    conservation_watchdog: [u8; 32],
    confinement_producer: [u8; 32],
    confinement_watchdog: [u8; 32],
    applicability_producer: [u8; 32],
    applicability_watchdog: [u8; 32],
    validity_producer: [u8; 32],
    validity_watchdog: [u8; 32],
    asymptotic_producer: [u8; 32],
    asymptotic_watchdog: [u8; 32],
}

pub(super) fn sealed_packet() -> Result<StrongProfilePacket, StrongProfileRefusal> {
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| StrongProfileRefusal::SealedSourceUnavailable)?;
    let mut occupied_profile_slots = vec![
        super::super::primitive_profile::RESIDUAL_SLOT_ID.to_owned(),
        super::super::charged_profile::RESIDUAL_SLOT_ID.to_owned(),
        super::super::neutral_bound_profile::RESIDUAL_SLOT_ID.to_owned(),
    ];
    occupied_profile_slots.sort();
    Ok(StrongProfilePacket {
        schema_id: PACKET_SCHEMA_ID.to_owned(),
        profile_id: PROFILE_ID.to_owned(),
        theory_class_id: THEORY_CLASS_ID.to_owned(),
        sector_id: SECTOR_ID.to_owned(),
        carrier_id: CARRIER_ID.to_owned(),
        curvature_law_id: CURVATURE_LAW_ID.to_owned(),
        conservation_id: CONSERVATION_ID.to_owned(),
        confinement_id: CONFINEMENT_ID.to_owned(),
        applicability_id: APPLICABILITY_ID.to_owned(),
        validity_id: VALIDITY_ID.to_owned(),
        asymptotic_disposition_id: ASYMPTOTIC_DISPOSITION_ID.to_owned(),
        residual_slot_id: RESIDUAL_SLOT_ID.to_owned(),
        occupied_profile_slots,
        owner_admission_record: OWNER_ADMISSION_RECORD.to_owned(),
        evidence_citation: EVIDENCE_CITATION.to_owned(),
        evidence_source_url: EVIDENCE_SOURCE_URL.to_owned(),
        evidence_full_sha256_hex: EVIDENCE_FULL_SHA256_HEX.to_owned(),
        evidence_slim_sha256_hex: EVIDENCE_SLIM_SHA256_HEX.to_owned(),
        evidence_anchor: EVIDENCE_ANCHOR.to_owned(),
        floor_authority: receipt_binding(floor.schema_id().as_str(), floor.digest()),
        global_physical_vocabulary_coverage: false,
        membership_authority: false,
        carrier_species_membership: false,
        confinement_theorem_claim: false,
    })
}

pub(super) fn inspect(
    packet: &StrongProfilePacket,
) -> Result<StrongCheckerOutput, StrongProfileRefusal> {
    let sealed = sealed_packet()?;
    inspect_against_sealed(packet, &sealed)
}

pub(super) fn inspect_against_sealed(
    packet: &StrongProfilePacket,
    sealed: &StrongProfilePacket,
) -> Result<StrongCheckerOutput, StrongProfileRefusal> {
    validate_packet(packet)?;
    validate_packet(sealed)?;
    let input_bytes = encode_packet(packet);
    if input_bytes != encode_packet(sealed) || input_bytes.len() > MAX_CANONICAL_BYTES {
        return Err(StrongProfileRefusal::ProfileIdentityMismatch);
    }
    let input_sha256 = sha256(&input_bytes);
    let evidence_custody_receipt_sha256 = digest_fields(
        EVIDENCE_DOMAIN,
        &[
            packet.evidence_citation.as_bytes(),
            packet.evidence_source_url.as_bytes(),
            packet.evidence_full_sha256_hex.as_bytes(),
            packet.evidence_slim_sha256_hex.as_bytes(),
            packet.evidence_anchor.as_bytes(),
        ],
    );
    let mut output = build_candidates(packet, input_sha256, evidence_custody_receipt_sha256)?;
    output.canonical_bytes = encode_checker_output(&output);
    if output.canonical_bytes.len() > MAX_CANONICAL_BYTES {
        return Err(StrongProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(output)
}

fn validate_packet(packet: &StrongProfilePacket) -> Result<(), StrongProfileRefusal> {
    if packet.schema_id != PACKET_SCHEMA_ID {
        return Err(StrongProfileRefusal::PacketSchemaMismatch);
    }
    if packet.profile_id != PROFILE_ID
        || packet.residual_slot_id != RESIDUAL_SLOT_ID
        || packet.owner_admission_record != OWNER_ADMISSION_RECORD
    {
        return Err(StrongProfileRefusal::ProfileIdentityMismatch);
    }
    let theory_fields = [
        (packet.theory_class_id.as_str(), THEORY_CLASS_ID),
        (packet.sector_id.as_str(), SECTOR_ID),
        (packet.carrier_id.as_str(), CARRIER_ID),
        (packet.curvature_law_id.as_str(), CURVATURE_LAW_ID),
        (packet.conservation_id.as_str(), CONSERVATION_ID),
        (packet.confinement_id.as_str(), CONFINEMENT_ID),
        (packet.applicability_id.as_str(), APPLICABILITY_ID),
        (packet.validity_id.as_str(), VALIDITY_ID),
        (
            packet.asymptotic_disposition_id.as_str(),
            ASYMPTOTIC_DISPOSITION_ID,
        ),
    ];
    if theory_fields
        .into_iter()
        .any(|(found, expected)| found != expected)
    {
        return Err(StrongProfileRefusal::TheoryProfileMismatch);
    }
    if (
        packet.evidence_citation.as_str(),
        packet.evidence_source_url.as_str(),
        packet.evidence_full_sha256_hex.as_str(),
        packet.evidence_slim_sha256_hex.as_str(),
        packet.evidence_anchor.as_str(),
    ) != (
        EVIDENCE_CITATION,
        EVIDENCE_SOURCE_URL,
        EVIDENCE_FULL_SHA256_HEX,
        EVIDENCE_SLIM_SHA256_HEX,
        EVIDENCE_ANCHOR,
    ) {
        return Err(StrongProfileRefusal::EvidenceCustodyMismatch);
    }
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| StrongProfileRefusal::SealedSourceUnavailable)?;
    if packet.floor_authority.schema_id != floor.schema_id().as_str()
        || packet.floor_authority.digest_sha256 != floor.digest()
        || packet.floor_authority.digest_sha256 == [0; 32]
    {
        return Err(StrongProfileRefusal::FloorBindingMismatch);
    }
    let mut found_slots = packet.occupied_profile_slots.clone();
    found_slots.sort();
    let expected_slots = vec![
        super::super::charged_profile::RESIDUAL_SLOT_ID.to_owned(),
        super::super::neutral_bound_profile::RESIDUAL_SLOT_ID.to_owned(),
        super::super::primitive_profile::RESIDUAL_SLOT_ID.to_owned(),
    ];
    if found_slots != expected_slots
        || found_slots.windows(2).any(|pair| pair[0] == pair[1])
        || found_slots.iter().any(|slot| slot == RESIDUAL_SLOT_ID)
    {
        return Err(StrongProfileRefusal::ResidualSlotInventoryMismatch);
    }
    if packet.global_physical_vocabulary_coverage
        || packet.membership_authority
        || packet.carrier_species_membership
        || packet.confinement_theorem_claim
    {
        return Err(StrongProfileRefusal::UnsupportedScope);
    }
    Ok(())
}

fn build_candidates(
    packet: &StrongProfilePacket,
    input_sha256: [u8; 32],
    evidence_custody_receipt_sha256: [u8; 32],
) -> Result<StrongCheckerOutput, StrongProfileRefusal> {
    let mut candidates = Vec::with_capacity(ARTIFACT_COUNT);
    let profile_payload = descriptor(
        PROFILE_SCHEMA_ID,
        &format!(
            "profile_id={}\ntheory_class={}\nevidence_custody_sha256={}\nconfinement_theorem_claim=false\nauthority_scope=claim-local\n",
            packet.profile_id,
            packet.theory_class_id,
            hex(evidence_custody_receipt_sha256)
        ),
    );
    let profile_root_identity = identity(&profile_payload)?;
    let profile_role_identity = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.role.profile-root.v1",
        "admitted-confining-profile-role",
    )?;
    let sector_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.role.sector.v1",
        "interaction-sector-role",
    )?;
    let sector = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.sector.v1",
        packet.sector_id.as_str(),
    )?;
    let carrier_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.role.carrier.v1",
        "connection-carrier-family-role",
    )?;
    let carrier = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.carrier.v1",
        packet.carrier_id.as_str(),
    )?;
    let curvature_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.role.curvature.v1",
        "noncommutative-curvature-role",
    )?;
    let curvature = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.curvature.v1",
        packet.curvature_law_id.as_str(),
    )?;
    let conservation_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.role.conservation.v1",
        "covariant-conservation-role",
    )?;
    let conservation = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.conservation.v1",
        packet.conservation_id.as_str(),
    )?;
    let confinement_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.role.confinement.v1",
        "confining-asymptotic-boundary-role",
    )?;
    let confinement = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.confinement.v1",
        packet.confinement_id.as_str(),
    )?;
    let applicability_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.role.applicability.v1",
        "regime-applicability-role",
    )?;
    let applicability = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.applicability.v1",
        packet.applicability_id.as_str(),
    )?;
    let validity_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.role.validity.v1",
        "claim-validity-role",
    )?;
    let validity = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.validity.v1",
        packet.validity_id.as_str(),
    )?;
    let asymptotic_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.role.asymptotic-disposition.v1",
        "asymptotic-candidate-disposition-role",
    )?;
    let asymptotic = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.asymptotic-disposition.v1",
        packet.asymptotic_disposition_id.as_str(),
    )?;
    let constraint_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.confining-profile.role.constraint.v1",
        "confining-profile-constraint-role",
    )?;

    let semantic = semantic_evidence(
        profile_root_identity,
        sector,
        carrier,
        curvature,
        conservation,
        confinement,
        applicability,
        validity,
        asymptotic,
    );
    let applicability_receipt_sha256 = digest_fields(
        APPLICABILITY_PAIR_DOMAIN,
        &[
            &semantic.applicability_producer,
            &semantic.applicability_watchdog,
        ],
    );
    let validity_receipt_sha256 = digest_fields(
        VALIDITY_PAIR_DOMAIN,
        &[&semantic.validity_producer, &semantic.validity_watchdog],
    );
    let claim_identity = digest_fields(
        PROFILE_CLAIM_DOMAIN,
        &[
            &profile_root_identity.0,
            &profile_role_identity.0,
            &input_sha256,
        ],
    );
    let admission_evidence =
        law_premise::inspect_theory_profile_protocol(&law_premise::TheoryProfileAdmissionRequest {
            claim_identity,
            role_identity: profile_role_identity.0,
            content_identity: profile_root_identity.0,
            profile_input_sha256: input_sha256,
            source_custody_sha256: evidence_custody_receipt_sha256,
            applicability_receipt_sha256,
            validity_receipt_sha256,
            residual_slot_id: packet.residual_slot_id.clone(),
            occupied_profile_slots: packet.occupied_profile_slots.clone(),
            owner_admission_record: packet.owner_admission_record.clone(),
        })
        .map_err(|_| StrongProfileRefusal::ArtifactConstructionFailure)?;
    if admission_evidence.decision_id != "irreducible_protocol_structurally_bound"
        || admission_evidence.target_claim_identity != claim_identity
        || admission_evidence.target_role_identity != profile_role_identity.0
        || admission_evidence.target_content_identity != profile_root_identity.0
        || admission_evidence.profile_protocol_schema_id
            != "civsim.planet.theory-profile-protocol-capability.v1"
        || admission_evidence.profile_protocol_producer_id
            != "civsim.planet.theory-profile-protocol.forward-authority.v1"
        || admission_evidence.profile_protocol_watchdog_id
            != "civsim.planet.theory-profile-protocol.reverse-authority.v1"
        || admission_evidence.profile_protocol_pair_receipt_sha256 == [0; 32]
        || admission_evidence.profile_protocol_producer_trace_sha256 == [0; 32]
        || admission_evidence.profile_protocol_watchdog_trace_sha256 == [0; 32]
        || admission_evidence.profile_protocol_producer_trace_sha256
            == admission_evidence.profile_protocol_watchdog_trace_sha256
        || admission_evidence.premise_admission_authority
        || admission_evidence.species_membership_authority
        || admission_evidence.global_derivation_coverage
        || admission_evidence.authority_effect != "none"
    {
        return Err(StrongProfileRefusal::ArtifactConstructionFailure);
    }
    candidates.push(StrongArtifactCandidate {
        identity: profile_root_identity,
        admission: irreducible_admission(packet, &admission_evidence),
        payload: profile_payload,
    });
    let constraint_payload =
        ArtifactPayload::ConstraintLaw(super::super::model::ConstraintLawArtifact {
            requirements: super::super::model::RequirementSet {
                artifact_relations: vec![
                    relation(profile_role_identity, profile_root_identity),
                    relation(sector_role, sector),
                    relation(carrier_role, carrier),
                    relation(curvature_role, curvature),
                    relation(conservation_role, conservation),
                    relation(confinement_role, confinement),
                    relation(applicability_role, applicability),
                    relation(validity_role, validity),
                    relation(asymptotic_role, asymptotic),
                    relation(constraint_role, profile_root_identity),
                ],
                species_dependencies: Vec::new(),
            },
        });
    let constraint_law_identity = push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        constraint_payload,
    )?;
    candidates.sort_by_key(|candidate| candidate.identity);
    if candidates.len() != ARTIFACT_COUNT
        || candidates
            .windows(2)
            .any(|pair| pair[0].identity == pair[1].identity)
    {
        return Err(StrongProfileRefusal::ArtifactCountMismatch);
    }
    Ok(StrongCheckerOutput {
        input_sha256,
        canonical_bytes: Vec::new(),
        candidates,
        profile_root_identity,
        profile_role_identity,
        sector_identity: sector,
        carrier_identity: carrier,
        constraint_law_identity,
        evidence_custody_receipt_sha256,
        curvature_producer_sha256: semantic.curvature_producer,
        curvature_watchdog_sha256: semantic.curvature_watchdog,
        conservation_producer_sha256: semantic.conservation_producer,
        conservation_watchdog_sha256: semantic.conservation_watchdog,
        confinement_producer_sha256: semantic.confinement_producer,
        confinement_watchdog_sha256: semantic.confinement_watchdog,
        applicability_producer_sha256: semantic.applicability_producer,
        applicability_watchdog_sha256: semantic.applicability_watchdog,
        validity_producer_sha256: semantic.validity_producer,
        validity_watchdog_sha256: semantic.validity_watchdog,
        asymptotic_producer_sha256: semantic.asymptotic_producer,
        asymptotic_watchdog_sha256: semantic.asymptotic_watchdog,
        admission_evidence,
    })
}

#[allow(clippy::too_many_arguments)]
fn semantic_evidence(
    profile_root: ArtifactIdentity,
    sector: ArtifactIdentity,
    carrier: ArtifactIdentity,
    curvature: ArtifactIdentity,
    conservation: ArtifactIdentity,
    confinement: ArtifactIdentity,
    applicability: ArtifactIdentity,
    validity: ArtifactIdentity,
    asymptotic: ArtifactIdentity,
) -> SemanticEvidence {
    let curvature_producer = digest_fields(
        CURVATURE_PRODUCER_DOMAIN,
        &[&sector.0, &carrier.0, &curvature.0],
    );
    let curvature_watchdog = digest_fields(
        CURVATURE_WATCHDOG_DOMAIN,
        &[&curvature.0, &carrier.0, &sector.0],
    );
    let conservation_producer = digest_fields(
        CONSERVATION_PRODUCER_DOMAIN,
        &[&profile_root.0, &sector.0, &conservation.0],
    );
    let conservation_watchdog = digest_fields(
        CONSERVATION_WATCHDOG_DOMAIN,
        &[&conservation.0, &sector.0, &profile_root.0],
    );
    let confinement_producer = digest_fields(
        CONFINEMENT_PRODUCER_DOMAIN,
        &[&sector.0, &confinement.0, &asymptotic.0],
    );
    let confinement_watchdog = digest_fields(
        CONFINEMENT_WATCHDOG_DOMAIN,
        &[&asymptotic.0, &confinement.0, &sector.0],
    );
    let applicability_producer = digest_fields(
        APPLICABILITY_PRODUCER_DOMAIN,
        &[&profile_root.0, &applicability.0, &confinement.0],
    );
    let applicability_watchdog = digest_fields(
        APPLICABILITY_WATCHDOG_DOMAIN,
        &[&confinement.0, &applicability.0, &profile_root.0],
    );
    let validity_producer = digest_fields(
        VALIDITY_PRODUCER_DOMAIN,
        &[&applicability.0, &validity.0, &profile_root.0],
    );
    let validity_watchdog = digest_fields(
        VALIDITY_WATCHDOG_DOMAIN,
        &[&profile_root.0, &validity.0, &applicability.0],
    );
    let asymptotic_producer = digest_fields(
        ASYMPTOTIC_PRODUCER_DOMAIN,
        &[&confinement.0, &asymptotic.0, &carrier.0],
    );
    let asymptotic_watchdog = digest_fields(
        ASYMPTOTIC_WATCHDOG_DOMAIN,
        &[&carrier.0, &asymptotic.0, &confinement.0],
    );
    SemanticEvidence {
        curvature_producer,
        curvature_watchdog,
        conservation_producer,
        conservation_watchdog,
        confinement_producer,
        confinement_watchdog,
        applicability_producer,
        applicability_watchdog,
        validity_producer,
        validity_watchdog,
        asymptotic_producer,
        asymptotic_watchdog,
    }
}

fn descriptor(schema_id: &str, content: &str) -> ArtifactPayload {
    ArtifactPayload::PhysicalDescriptor(super::super::model::CanonicalArtifact {
        schema_id: schema_id.to_owned(),
        canonical_bytes: content.as_bytes().to_vec(),
    })
}

fn relation(
    role: ArtifactIdentity,
    target: ArtifactIdentity,
) -> super::super::model::ArtifactRelation {
    super::super::model::ArtifactRelation { role, target }
}

fn identity(payload: &ArtifactPayload) -> Result<ArtifactIdentity, StrongProfileRefusal> {
    super::super::producer::derive_artifact_identity_for_authority(payload)
        .map_err(|_| StrongProfileRefusal::ArtifactConstructionFailure)
}

fn push_descriptor(
    candidates: &mut Vec<StrongArtifactCandidate>,
    profile_root_identity: ArtifactIdentity,
    input_sha256: [u8; 32],
    schema_id: &str,
    content: &str,
) -> Result<ArtifactIdentity, StrongProfileRefusal> {
    push_derived(
        candidates,
        profile_root_identity,
        input_sha256,
        descriptor(schema_id, content),
    )
}

fn push_derived(
    candidates: &mut Vec<StrongArtifactCandidate>,
    profile_root_identity: ArtifactIdentity,
    input_sha256: [u8; 32],
    payload: ArtifactPayload,
) -> Result<ArtifactIdentity, StrongProfileRefusal> {
    let artifact_identity = identity(&payload)?;
    candidates.push(StrongArtifactCandidate {
        identity: artifact_identity,
        admission: derived_admission(input_sha256, profile_root_identity, artifact_identity),
        payload,
    });
    Ok(artifact_identity)
}

fn irreducible_admission(
    packet: &StrongProfilePacket,
    evidence: &law_premise::TheoryProfileAdmissionEvidence,
) -> RootAdmission {
    RootAdmission {
        tier: Tier::Residue,
        provenance: Provenance::Authored,
        route: AdmissionRoute::Irreducible(Box::new(super::super::model::IrreducibleAdmission {
            derivation_exhaustion_receipt: receipt_binding(
                "civsim.confining-profile.derive-first-exhaustion.v1",
                evidence.derivation_exhaustion_receipt_sha256,
            ),
            buckingham_pi_receipt: receipt_binding(
                "civsim.confining-profile.buckingham-pi.v1",
                evidence.buckingham_pi_receipt_sha256,
            ),
            gap_law_receipt: receipt_binding(
                "civsim.confining-profile.gap-law.v1",
                evidence.gap_law_receipt_sha256,
            ),
            chaos_protocol_receipt: receipt_binding(
                "civsim.confining-profile.chaos-protocol.v1",
                evidence.chaos_protocol_receipt_sha256,
            ),
            residual_law_receipt: receipt_binding(
                "civsim.confining-profile.residual-law.v1",
                evidence.residual_law_receipt_sha256,
            ),
            residual_slot_id: packet.residual_slot_id.clone(),
            residual_slot_receipt: receipt_binding(
                "civsim.confining-profile.residual-slot.v1",
                evidence.residual_slot_receipt_sha256,
            ),
            owner_admission_receipt: receipt_binding(
                "civsim.confining-profile.owner-admission.v1",
                evidence.owner_admission_receipt_sha256,
            ),
            independent_watchdog_receipt: receipt_binding(
                "civsim.confining-profile.independent-watchdog-route.v1",
                evidence.independent_watchdog_receipt_sha256,
            ),
        })),
    }
}

fn derived_admission(
    input_sha256: [u8; 32],
    profile_root_identity: ArtifactIdentity,
    artifact_identity: ArtifactIdentity,
) -> RootAdmission {
    RootAdmission {
        tier: Tier::Residue,
        provenance: Provenance::Derived,
        route: AdmissionRoute::Derived(super::super::model::DerivedAdmission {
            ancestry_receipt: receipt_binding(
                "civsim.confining-profile.derived-ancestry.v1",
                digest_fields(
                    b"civsim.confining-profile.derived-ancestry.v1",
                    &[&profile_root_identity.0, &artifact_identity.0],
                ),
            ),
            semantic_checker_receipt: receipt_binding(
                "civsim.confining-profile.derived-forward-semantic.v1",
                digest_fields(
                    b"civsim.confining-profile.derived-forward-semantic.v1",
                    &[&input_sha256, &artifact_identity.0],
                ),
            ),
            independent_watchdog_receipt: receipt_binding(
                "civsim.confining-profile.derived-reverse-semantic.v1",
                digest_fields(
                    b"civsim.confining-profile.derived-reverse-semantic.v1",
                    &[&artifact_identity.0, &input_sha256],
                ),
            ),
        }),
    }
}

pub(super) fn canary_evidence(
    packet: &StrongProfilePacket,
) -> Result<CanaryEvidence, StrongProfileRefusal> {
    let sealed = sealed_packet()?;
    canary_evidence_against_sealed(packet, &sealed)
}

pub(super) fn canary_evidence_against_sealed(
    packet: &StrongProfilePacket,
    sealed: &StrongProfilePacket,
) -> Result<CanaryEvidence, StrongProfileRefusal> {
    let cases = mutations(packet);
    let mut transcript = PRODUCER_CANARY_ID.as_bytes().to_vec();
    for (name, mutant) in &cases {
        let refusal = inspect_against_sealed(mutant, sealed)
            .err()
            .ok_or(StrongProfileRefusal::CanaryFailure)?;
        append_field(&mut transcript, 1, name.as_bytes());
        append_field(&mut transcript, 2, refusal.id().as_bytes());
    }
    Ok(CanaryEvidence {
        transcript_id: PRODUCER_CANARY_ID,
        case_count: u32::try_from(cases.len()).map_err(|_| StrongProfileRefusal::CanaryFailure)?,
        transcript_sha256: sha256(&transcript),
    })
}

fn mutations(packet: &StrongProfilePacket) -> Vec<(&'static str, StrongProfilePacket)> {
    let mut cases = Vec::new();
    macro_rules! mutate {
        ($name:literal, $body:expr) => {{
            let mut mutant = packet.clone();
            $body(&mut mutant);
            cases.push(($name, mutant));
        }};
    }
    mutate!("packet-schema", |p: &mut StrongProfilePacket| p
        .schema_id
        .push_str(".mutant"));
    mutate!("profile-id", |p: &mut StrongProfilePacket| p
        .profile_id
        .push_str(".mutant"));
    mutate!("theory-class", |p: &mut StrongProfilePacket| p
        .theory_class_id
        .push_str(".mutant"));
    mutate!("sector", |p: &mut StrongProfilePacket| p
        .sector_id
        .push_str(".mutant"));
    mutate!("carrier", |p: &mut StrongProfilePacket| p
        .carrier_id
        .push_str(".mutant"));
    mutate!("curvature", |p: &mut StrongProfilePacket| p
        .curvature_law_id
        .push_str(".mutant"));
    mutate!("conservation", |p: &mut StrongProfilePacket| p
        .conservation_id
        .push_str(".mutant"));
    mutate!("confinement", |p: &mut StrongProfilePacket| p
        .confinement_id
        .push_str(".mutant"));
    mutate!("applicability", |p: &mut StrongProfilePacket| p
        .applicability_id
        .push_str(".mutant"));
    mutate!("validity", |p: &mut StrongProfilePacket| p
        .validity_id
        .push_str(".mutant"));
    mutate!("asymptotic", |p: &mut StrongProfilePacket| p
        .asymptotic_disposition_id
        .push_str(".mutant"));
    mutate!("residual-slot", |p: &mut StrongProfilePacket| p
        .residual_slot_id
        .push_str(".mutant"));
    mutate!("owner-record", |p: &mut StrongProfilePacket| p
        .owner_admission_record
        .push_str(".mutant"));
    mutate!("evidence-citation", |p: &mut StrongProfilePacket| p
        .evidence_citation
        .push_str(".mutant"));
    mutate!("evidence-full-hash", |p: &mut StrongProfilePacket| p
        .evidence_full_sha256_hex
        .replace_range(..1, "0"));
    mutate!("evidence-slim-hash", |p: &mut StrongProfilePacket| p
        .evidence_slim_sha256_hex
        .replace_range(..1, "0"));
    mutate!("floor-binding", |p: &mut StrongProfilePacket| p
        .floor_authority
        .digest_sha256[31] ^=
        1);
    mutate!("occupied-slot-omission", |p: &mut StrongProfilePacket| {
        p.occupied_profile_slots.pop();
    });
    mutate!("occupied-slot-collision", |p: &mut StrongProfilePacket| {
        p.occupied_profile_slots.push(RESIDUAL_SLOT_ID.to_owned());
    });
    mutate!("global-coverage", |p: &mut StrongProfilePacket| {
        p.global_physical_vocabulary_coverage = true;
    });
    mutate!("membership-authority", |p: &mut StrongProfilePacket| {
        p.membership_authority = true;
    });
    mutate!("carrier-membership", |p: &mut StrongProfilePacket| {
        p.carrier_species_membership = true;
    });
    mutate!("theorem-claim", |p: &mut StrongProfilePacket| {
        p.confinement_theorem_claim = true;
    });
    cases
}

pub(super) fn pair_receipt_digest(
    output: &StrongCheckerOutput,
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
    producer_canary: CanaryEvidence,
    watchdog_canary: CanaryEvidence,
) -> [u8; 32] {
    digest_fields(
        PAIR_DOMAIN,
        &[
            &output.input_sha256,
            &producer_result_sha256,
            &watchdog_result_sha256,
            PRODUCER_ID.as_bytes(),
            WATCHDOG_ID.as_bytes(),
            &producer_canary.transcript_sha256,
            &watchdog_canary.transcript_sha256,
            &output.profile_root_identity.0,
            &output.sector_identity.0,
            &output.carrier_identity.0,
            &output.constraint_law_identity.0,
            &output
                .admission_evidence
                .irreducible_protocol_capability_sha256,
        ],
    )
}
