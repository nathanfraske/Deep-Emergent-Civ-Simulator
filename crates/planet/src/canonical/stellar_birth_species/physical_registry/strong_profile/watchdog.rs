//! Reverse obligation reconstruction for the confining interaction profile.

use super::*;
use crate::canonical::stellar_birth_species::law_premise;
use civsim_ledger::{Provenance, Tier};
use civsim_units::{digest::sha256, physics_floor::sealed_physical_floor_authority_binding};
use std::collections::{BTreeMap, BTreeSet};

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
struct ReverseSemanticEvidence {
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

#[derive(Clone, Copy)]
struct DescriptorObligation<'a> {
    key: &'static str,
    schema_id: &'static str,
    content: &'a str,
}

pub(super) fn sealed_packet() -> Result<StrongProfilePacket, StrongProfileRefusal> {
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| StrongProfileRefusal::SealedSourceUnavailable)?;
    let mut occupied_profile_slots = vec![
        super::super::neutral_bound_profile::RESIDUAL_SLOT_ID.to_owned(),
        super::super::charged_profile::RESIDUAL_SLOT_ID.to_owned(),
        super::super::primitive_profile::RESIDUAL_SLOT_ID.to_owned(),
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
    inspect_obligations(packet)?;
    inspect_obligations(sealed)?;
    let packet_bytes = encode_packet(packet);
    if packet_bytes != encode_packet(sealed) || packet_bytes.len() > MAX_CANONICAL_BYTES {
        return Err(StrongProfileRefusal::ProfileIdentityMismatch);
    }
    let input_sha256 = sha256(&packet_bytes);
    let evidence_custody_receipt_sha256 = reconstruct_evidence_custody(packet);
    let mut output =
        reconstruct_from_obligations(packet, input_sha256, evidence_custody_receipt_sha256)?;
    output.canonical_bytes = encode_checker_output(&output);
    if output.canonical_bytes.len() > MAX_CANONICAL_BYTES {
        return Err(StrongProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(output)
}

fn inspect_obligations(packet: &StrongProfilePacket) -> Result<(), StrongProfileRefusal> {
    let identity_obligations = BTreeMap::from([
        ("applicability", APPLICABILITY_ID),
        ("asymptotic", ASYMPTOTIC_DISPOSITION_ID),
        ("carrier", CARRIER_ID),
        ("confinement", CONFINEMENT_ID),
        ("conservation", CONSERVATION_ID),
        ("curvature", CURVATURE_LAW_ID),
        ("profile", PROFILE_ID),
        ("sector", SECTOR_ID),
        ("theory", THEORY_CLASS_ID),
        ("validity", VALIDITY_ID),
    ]);
    let observed_identities = BTreeMap::from([
        ("validity", packet.validity_id.as_str()),
        ("theory", packet.theory_class_id.as_str()),
        ("sector", packet.sector_id.as_str()),
        ("profile", packet.profile_id.as_str()),
        ("curvature", packet.curvature_law_id.as_str()),
        ("conservation", packet.conservation_id.as_str()),
        ("confinement", packet.confinement_id.as_str()),
        ("carrier", packet.carrier_id.as_str()),
        ("asymptotic", packet.asymptotic_disposition_id.as_str()),
        ("applicability", packet.applicability_id.as_str()),
    ]);
    if packet.schema_id != PACKET_SCHEMA_ID {
        return Err(StrongProfileRefusal::PacketSchemaMismatch);
    }
    if observed_identities != identity_obligations
        || packet.residual_slot_id != RESIDUAL_SLOT_ID
        || packet.owner_admission_record != OWNER_ADMISSION_RECORD
    {
        return Err(StrongProfileRefusal::ProfileIdentityMismatch);
    }

    let expected_custody = BTreeSet::from([
        EVIDENCE_ANCHOR,
        EVIDENCE_CITATION,
        EVIDENCE_FULL_SHA256_HEX,
        EVIDENCE_SLIM_SHA256_HEX,
        EVIDENCE_SOURCE_URL,
    ]);
    let observed_custody = BTreeSet::from([
        packet.evidence_anchor.as_str(),
        packet.evidence_citation.as_str(),
        packet.evidence_full_sha256_hex.as_str(),
        packet.evidence_slim_sha256_hex.as_str(),
        packet.evidence_source_url.as_str(),
    ]);
    if observed_custody != expected_custody {
        return Err(StrongProfileRefusal::EvidenceCustodyMismatch);
    }

    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| StrongProfileRefusal::SealedSourceUnavailable)?;
    if packet.floor_authority.schema_id.as_bytes() != floor.schema_id().as_str().as_bytes()
        || packet.floor_authority.digest_sha256 != floor.digest()
        || packet.floor_authority.digest_sha256 == [0; 32]
    {
        return Err(StrongProfileRefusal::FloorBindingMismatch);
    }

    let expected_slots = BTreeSet::from([
        super::super::charged_profile::RESIDUAL_SLOT_ID,
        super::super::neutral_bound_profile::RESIDUAL_SLOT_ID,
        super::super::primitive_profile::RESIDUAL_SLOT_ID,
    ]);
    let observed_slots: BTreeSet<_> = packet
        .occupied_profile_slots
        .iter()
        .map(String::as_str)
        .collect();
    if observed_slots != expected_slots
        || observed_slots.len() != packet.occupied_profile_slots.len()
        || observed_slots.contains(RESIDUAL_SLOT_ID)
    {
        return Err(StrongProfileRefusal::ResidualSlotInventoryMismatch);
    }
    if [
        packet.global_physical_vocabulary_coverage,
        packet.membership_authority,
        packet.carrier_species_membership,
        packet.confinement_theorem_claim,
    ]
    .into_iter()
    .any(|claim| claim)
    {
        return Err(StrongProfileRefusal::UnsupportedScope);
    }
    Ok(())
}

fn reconstruct_evidence_custody(packet: &StrongProfilePacket) -> [u8; 32] {
    let ordered = [
        packet.evidence_citation.as_bytes(),
        packet.evidence_source_url.as_bytes(),
        packet.evidence_full_sha256_hex.as_bytes(),
        packet.evidence_slim_sha256_hex.as_bytes(),
        packet.evidence_anchor.as_bytes(),
    ];
    reverse_digest_fields(EVIDENCE_DOMAIN, &ordered)
}

fn reconstruct_from_obligations(
    packet: &StrongProfilePacket,
    input_sha256: [u8; 32],
    evidence_custody_receipt_sha256: [u8; 32],
) -> Result<StrongCheckerOutput, StrongProfileRefusal> {
    let profile_payload = descriptor(
        PROFILE_SCHEMA_ID,
        &format!(
            "profile_id={}\ntheory_class={}\nevidence_custody_sha256={}\nconfinement_theorem_claim=false\nauthority_scope=claim-local\n",
            packet.profile_id,
            packet.theory_class_id,
            hex(evidence_custody_receipt_sha256)
        ),
    );
    let profile_root_identity = reverse_identity(&profile_payload)?;

    let obligations = [
        DescriptorObligation {
            key: "profile-role",
            schema_id: "civsim.confining-profile.role.profile-root.v1",
            content: "admitted-confining-profile-role",
        },
        DescriptorObligation {
            key: "sector-role",
            schema_id: "civsim.confining-profile.role.sector.v1",
            content: "interaction-sector-role",
        },
        DescriptorObligation {
            key: "sector",
            schema_id: "civsim.confining-profile.sector.v1",
            content: packet.sector_id.as_str(),
        },
        DescriptorObligation {
            key: "carrier-role",
            schema_id: "civsim.confining-profile.role.carrier.v1",
            content: "connection-carrier-family-role",
        },
        DescriptorObligation {
            key: "carrier",
            schema_id: "civsim.confining-profile.carrier.v1",
            content: packet.carrier_id.as_str(),
        },
        DescriptorObligation {
            key: "curvature-role",
            schema_id: "civsim.confining-profile.role.curvature.v1",
            content: "noncommutative-curvature-role",
        },
        DescriptorObligation {
            key: "curvature",
            schema_id: "civsim.confining-profile.curvature.v1",
            content: packet.curvature_law_id.as_str(),
        },
        DescriptorObligation {
            key: "conservation-role",
            schema_id: "civsim.confining-profile.role.conservation.v1",
            content: "covariant-conservation-role",
        },
        DescriptorObligation {
            key: "conservation",
            schema_id: "civsim.confining-profile.conservation.v1",
            content: packet.conservation_id.as_str(),
        },
        DescriptorObligation {
            key: "confinement-role",
            schema_id: "civsim.confining-profile.role.confinement.v1",
            content: "confining-asymptotic-boundary-role",
        },
        DescriptorObligation {
            key: "confinement",
            schema_id: "civsim.confining-profile.confinement.v1",
            content: packet.confinement_id.as_str(),
        },
        DescriptorObligation {
            key: "applicability-role",
            schema_id: "civsim.confining-profile.role.applicability.v1",
            content: "regime-applicability-role",
        },
        DescriptorObligation {
            key: "applicability",
            schema_id: "civsim.confining-profile.applicability.v1",
            content: packet.applicability_id.as_str(),
        },
        DescriptorObligation {
            key: "validity-role",
            schema_id: "civsim.confining-profile.role.validity.v1",
            content: "claim-validity-role",
        },
        DescriptorObligation {
            key: "validity",
            schema_id: "civsim.confining-profile.validity.v1",
            content: packet.validity_id.as_str(),
        },
        DescriptorObligation {
            key: "asymptotic-role",
            schema_id: "civsim.confining-profile.role.asymptotic-disposition.v1",
            content: "asymptotic-candidate-disposition-role",
        },
        DescriptorObligation {
            key: "asymptotic",
            schema_id: "civsim.confining-profile.asymptotic-disposition.v1",
            content: packet.asymptotic_disposition_id.as_str(),
        },
        DescriptorObligation {
            key: "constraint-role",
            schema_id: "civsim.confining-profile.role.constraint.v1",
            content: "confining-profile-constraint-role",
        },
    ];

    let mut candidates = Vec::with_capacity(ARTIFACT_COUNT);
    let mut identities = BTreeMap::new();
    for obligation in obligations.into_iter().rev() {
        let payload = descriptor(obligation.schema_id, obligation.content);
        let identity = reverse_identity(&payload)?;
        if identities.insert(obligation.key, identity).is_some() {
            return Err(StrongProfileRefusal::ArtifactConstructionFailure);
        }
        candidates.push(StrongArtifactCandidate {
            identity,
            admission: reverse_derived_admission(input_sha256, profile_root_identity, identity),
            payload,
        });
    }

    let id = |key: &str| {
        identities
            .get(key)
            .copied()
            .ok_or(StrongProfileRefusal::ArtifactConstructionFailure)
    };
    let profile_role_identity = id("profile-role")?;
    let sector_role = id("sector-role")?;
    let sector = id("sector")?;
    let carrier_role = id("carrier-role")?;
    let carrier = id("carrier")?;
    let curvature_role = id("curvature-role")?;
    let curvature = id("curvature")?;
    let conservation_role = id("conservation-role")?;
    let conservation = id("conservation")?;
    let confinement_role = id("confinement-role")?;
    let confinement = id("confinement")?;
    let applicability_role = id("applicability-role")?;
    let applicability = id("applicability")?;
    let validity_role = id("validity-role")?;
    let validity = id("validity")?;
    let asymptotic_role = id("asymptotic-role")?;
    let asymptotic = id("asymptotic")?;
    let constraint_role = id("constraint-role")?;

    let semantic = reverse_semantic_evidence(
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
    let applicability_receipt_sha256 = reverse_digest_fields(
        APPLICABILITY_PAIR_DOMAIN,
        &[
            &semantic.applicability_producer,
            &semantic.applicability_watchdog,
        ],
    );
    let validity_receipt_sha256 = reverse_digest_fields(
        VALIDITY_PAIR_DOMAIN,
        &[&semantic.validity_producer, &semantic.validity_watchdog],
    );
    let claim_identity = reverse_digest_fields(
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
        admission: reverse_irreducible_admission(packet, &admission_evidence),
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
    let constraint_law_identity = reverse_identity(&constraint_payload)?;
    candidates.push(StrongArtifactCandidate {
        identity: constraint_law_identity,
        admission: reverse_derived_admission(
            input_sha256,
            profile_root_identity,
            constraint_law_identity,
        ),
        payload: constraint_payload,
    });
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
fn reverse_semantic_evidence(
    profile_root: ArtifactIdentity,
    sector: ArtifactIdentity,
    carrier: ArtifactIdentity,
    curvature: ArtifactIdentity,
    conservation: ArtifactIdentity,
    confinement: ArtifactIdentity,
    applicability: ArtifactIdentity,
    validity: ArtifactIdentity,
    asymptotic: ArtifactIdentity,
) -> ReverseSemanticEvidence {
    let reverse = |domain: &[u8], fields: &[&[u8]]| reverse_digest_fields(domain, fields);
    ReverseSemanticEvidence {
        curvature_producer: reverse(
            CURVATURE_PRODUCER_DOMAIN,
            &[&sector.0, &carrier.0, &curvature.0],
        ),
        curvature_watchdog: reverse(
            CURVATURE_WATCHDOG_DOMAIN,
            &[&curvature.0, &carrier.0, &sector.0],
        ),
        conservation_producer: reverse(
            CONSERVATION_PRODUCER_DOMAIN,
            &[&profile_root.0, &sector.0, &conservation.0],
        ),
        conservation_watchdog: reverse(
            CONSERVATION_WATCHDOG_DOMAIN,
            &[&conservation.0, &sector.0, &profile_root.0],
        ),
        confinement_producer: reverse(
            CONFINEMENT_PRODUCER_DOMAIN,
            &[&sector.0, &confinement.0, &asymptotic.0],
        ),
        confinement_watchdog: reverse(
            CONFINEMENT_WATCHDOG_DOMAIN,
            &[&asymptotic.0, &confinement.0, &sector.0],
        ),
        applicability_producer: reverse(
            APPLICABILITY_PRODUCER_DOMAIN,
            &[&profile_root.0, &applicability.0, &confinement.0],
        ),
        applicability_watchdog: reverse(
            APPLICABILITY_WATCHDOG_DOMAIN,
            &[&confinement.0, &applicability.0, &profile_root.0],
        ),
        validity_producer: reverse(
            VALIDITY_PRODUCER_DOMAIN,
            &[&applicability.0, &validity.0, &profile_root.0],
        ),
        validity_watchdog: reverse(
            VALIDITY_WATCHDOG_DOMAIN,
            &[&profile_root.0, &validity.0, &applicability.0],
        ),
        asymptotic_producer: reverse(
            ASYMPTOTIC_PRODUCER_DOMAIN,
            &[&confinement.0, &asymptotic.0, &carrier.0],
        ),
        asymptotic_watchdog: reverse(
            ASYMPTOTIC_WATCHDOG_DOMAIN,
            &[&carrier.0, &asymptotic.0, &confinement.0],
        ),
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

fn reverse_identity(payload: &ArtifactPayload) -> Result<ArtifactIdentity, StrongProfileRefusal> {
    super::super::watchdog::derive_artifact_identity_for_authority(payload)
        .map_err(|_| StrongProfileRefusal::ArtifactConstructionFailure)
}

fn reverse_irreducible_admission(
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

fn reverse_derived_admission(
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
                reverse_digest_fields(
                    b"civsim.confining-profile.derived-ancestry.v1",
                    &[&profile_root_identity.0, &artifact_identity.0],
                ),
            ),
            semantic_checker_receipt: receipt_binding(
                "civsim.confining-profile.derived-forward-semantic.v1",
                reverse_digest_fields(
                    b"civsim.confining-profile.derived-forward-semantic.v1",
                    &[&input_sha256, &artifact_identity.0],
                ),
            ),
            independent_watchdog_receipt: receipt_binding(
                "civsim.confining-profile.derived-reverse-semantic.v1",
                reverse_digest_fields(
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
    let cases = reverse_mutations(packet);
    let mut transcript = WATCHDOG_CANARY_ID.as_bytes().to_vec();
    for (name, mutant) in cases.iter().rev() {
        let refusal = inspect_against_sealed(mutant, sealed)
            .err()
            .ok_or(StrongProfileRefusal::CanaryFailure)?;
        append_field(&mut transcript, 11, refusal.id().as_bytes());
        append_field(&mut transcript, 12, name.as_bytes());
    }
    Ok(CanaryEvidence {
        transcript_id: WATCHDOG_CANARY_ID,
        case_count: u32::try_from(cases.len()).map_err(|_| StrongProfileRefusal::CanaryFailure)?,
        transcript_sha256: sha256(&transcript),
    })
}

fn reverse_mutations(packet: &StrongProfilePacket) -> Vec<(&'static str, StrongProfilePacket)> {
    let mut cases = Vec::new();
    macro_rules! mutate {
        ($name:literal, $body:expr) => {{
            let mut mutant = packet.clone();
            $body(&mut mutant);
            cases.push(($name, mutant));
        }};
    }
    mutate!("theorem-claim", |p: &mut StrongProfilePacket| {
        p.confinement_theorem_claim = true;
    });
    mutate!("carrier-membership", |p: &mut StrongProfilePacket| {
        p.carrier_species_membership = true;
    });
    mutate!("membership-authority", |p: &mut StrongProfilePacket| {
        p.membership_authority = true;
    });
    mutate!("global-coverage", |p: &mut StrongProfilePacket| {
        p.global_physical_vocabulary_coverage = true;
    });
    mutate!("occupied-slot-collision", |p: &mut StrongProfilePacket| {
        p.occupied_profile_slots.push(RESIDUAL_SLOT_ID.to_owned());
    });
    mutate!("occupied-slot-omission", |p: &mut StrongProfilePacket| {
        p.occupied_profile_slots.remove(0);
    });
    mutate!("floor-binding", |p: &mut StrongProfilePacket| {
        p.floor_authority.digest_sha256[0] ^= 1;
    });
    mutate!("evidence-slim-hash", |p: &mut StrongProfilePacket| {
        p.evidence_slim_sha256_hex.push('0');
    });
    mutate!("evidence-full-hash", |p: &mut StrongProfilePacket| {
        p.evidence_full_sha256_hex.push('0');
    });
    mutate!("evidence-citation", |p: &mut StrongProfilePacket| {
        p.evidence_citation.push_str(".mutant");
    });
    mutate!("owner-record", |p: &mut StrongProfilePacket| {
        p.owner_admission_record.push_str(".mutant");
    });
    mutate!("residual-slot", |p: &mut StrongProfilePacket| {
        p.residual_slot_id.push_str(".mutant");
    });
    mutate!("asymptotic", |p: &mut StrongProfilePacket| {
        p.asymptotic_disposition_id.push_str(".mutant");
    });
    mutate!("validity", |p: &mut StrongProfilePacket| {
        p.validity_id.push_str(".mutant");
    });
    mutate!("applicability", |p: &mut StrongProfilePacket| {
        p.applicability_id.push_str(".mutant");
    });
    mutate!("confinement", |p: &mut StrongProfilePacket| {
        p.confinement_id.push_str(".mutant");
    });
    mutate!("conservation", |p: &mut StrongProfilePacket| {
        p.conservation_id.push_str(".mutant");
    });
    mutate!("curvature", |p: &mut StrongProfilePacket| {
        p.curvature_law_id.push_str(".mutant");
    });
    mutate!("carrier", |p: &mut StrongProfilePacket| {
        p.carrier_id.push_str(".mutant");
    });
    mutate!("sector", |p: &mut StrongProfilePacket| {
        p.sector_id.push_str(".mutant");
    });
    mutate!("theory-class", |p: &mut StrongProfilePacket| {
        p.theory_class_id.push_str(".mutant");
    });
    mutate!("profile-id", |p: &mut StrongProfilePacket| {
        p.profile_id.push_str(".mutant");
    });
    mutate!("packet-schema", |p: &mut StrongProfilePacket| {
        p.schema_id.push_str(".mutant");
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
    reverse_digest_fields(
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

/// Independent spelling of the canonical field digest used by the reverse
/// route. It traverses the supplied slice backward while prepending fields, so
/// its control flow differs from the forward producer and its final bytes are
/// still canonical order.
fn reverse_digest_fields(domain: &[u8], fields: &[&[u8]]) -> [u8; 32] {
    let mut encoded_fields = Vec::new();
    for field in fields.iter().rev() {
        let mut encoded = Vec::new();
        append_field(&mut encoded, 2, field);
        encoded_fields.insert(0, encoded);
    }
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, domain);
    for encoded in encoded_fields {
        bytes.extend_from_slice(&encoded);
    }
    sha256(&bytes)
}
