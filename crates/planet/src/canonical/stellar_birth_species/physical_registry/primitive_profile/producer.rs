//! Forward constructor and semantic inspector for the primitive profile.

use super::*;
use crate::canonical::stellar_birth_species::law_premise::repository_derived_relation_premise_frontier;
use civsim_ledger::{Provenance, Tier};
use civsim_units::{digest::sha256, physics_floor::sealed_physical_floor_authority_binding};

const INPUT_DOMAIN: &[u8] = b"civsim.planet.primitive-excitation-profile.producer-input.v1";
const OUTPUT_DOMAIN: &[u8] = b"civsim.planet.primitive-excitation-profile.canonical-output.v1";
const PAIR_DOMAIN: &[u8] = b"civsim.planet.primitive-excitation-profile.pair-receipt.v1";
const MAX_CANONICAL_BYTES: usize = 1_048_576;

pub(super) fn sealed_packet() -> Result<PrimitiveProfilePacket, PrimitiveProfileRefusal> {
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| PrimitiveProfileRefusal::SealedSourceUnavailable)?;
    let eps0 = repository_derived_relation_premise_frontier()
        .map_err(|_| PrimitiveProfileRefusal::SealedSourceUnavailable)?;
    Ok(PrimitiveProfilePacket {
        schema_id: PACKET_SCHEMA_ID.to_owned(),
        profile_id: PROFILE_ID.to_owned(),
        theory_class_id: THEORY_CLASS_ID.to_owned(),
        symmetry_id: SYMMETRY_ID.to_owned(),
        field_id: FIELD_ID.to_owned(),
        operator_id: OPERATOR_ID.to_owned(),
        state_id: STATE_ID.to_owned(),
        sector_id: SECTOR_ID.to_owned(),
        validity_id: VALIDITY_ID.to_owned(),
        excluded_term_id: EXCLUDED_TERM_ID.to_owned(),
        member_id: MEMBER_ID.to_owned(),
        residual_slot_id: RESIDUAL_SLOT_ID.to_owned(),
        owner_admission_record: OWNER_ADMISSION_RECORD.to_owned(),
        evidence_citation: EVIDENCE_CITATION.to_owned(),
        evidence_source_url: EVIDENCE_SOURCE_URL.to_owned(),
        evidence_full_sha256_hex: EVIDENCE_FULL_SHA256_HEX.to_owned(),
        evidence_slim_sha256_hex: EVIDENCE_SLIM_SHA256_HEX.to_owned(),
        evidence_anchor: EVIDENCE_ANCHOR.to_owned(),
        floor_authority_schema_id: floor.schema_id().as_str().to_owned(),
        floor_authority_sha256: floor.digest(),
        eps0_pair_receipt_sha256: eps0.pair_receipt_sha256,
        eps0_capability_sha256: eps0.capability_sha256,
        global_physical_vocabulary_coverage: false,
        membership_authority: false,
    })
}

pub(super) fn inspect(
    packet: &PrimitiveProfilePacket,
) -> Result<PrimitiveProfileCheckerOutput, PrimitiveProfileRefusal> {
    validate_packet(packet)?;
    let input_bytes = encode_packet(packet);
    if input_bytes.len() > MAX_CANONICAL_BYTES {
        return Err(PrimitiveProfileRefusal::ArtifactConstructionFailure);
    }
    let input_sha256 = sha256(&input_bytes);
    let evidence_custody_receipt_sha256 = digest(
        b"civsim.planet.primitive-excitation-profile.evidence-custody.v1",
        &[
            packet.evidence_citation.as_bytes(),
            packet.evidence_source_url.as_bytes(),
            packet.evidence_full_sha256_hex.as_bytes(),
            packet.evidence_slim_sha256_hex.as_bytes(),
            packet.evidence_anchor.as_bytes(),
        ],
    );
    let (candidates, member, profile_root_identity) =
        build_candidates(packet, input_sha256, evidence_custody_receipt_sha256)?;
    if candidates.len() != ARTIFACT_COUNT {
        return Err(PrimitiveProfileRefusal::ArtifactCountMismatch);
    }
    let canonical_bytes = encode_output(
        input_sha256,
        &candidates,
        member,
        profile_root_identity,
        evidence_custody_receipt_sha256,
    );
    if canonical_bytes.len() > MAX_CANONICAL_BYTES {
        return Err(PrimitiveProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(PrimitiveProfileCheckerOutput {
        input_sha256,
        canonical_bytes,
        candidates,
        member,
        profile_root_identity,
        evidence_custody_receipt_sha256,
    })
}

fn validate_packet(packet: &PrimitiveProfilePacket) -> Result<(), PrimitiveProfileRefusal> {
    if packet.schema_id != PACKET_SCHEMA_ID {
        return Err(PrimitiveProfileRefusal::PacketSchemaMismatch);
    }
    if packet.profile_id != PROFILE_ID
        || packet.residual_slot_id != RESIDUAL_SLOT_ID
        || packet.owner_admission_record != OWNER_ADMISSION_RECORD
    {
        return Err(PrimitiveProfileRefusal::ProfileIdentityMismatch);
    }
    if (
        packet.theory_class_id.as_str(),
        packet.symmetry_id.as_str(),
        packet.field_id.as_str(),
        packet.operator_id.as_str(),
        packet.state_id.as_str(),
        packet.sector_id.as_str(),
        packet.validity_id.as_str(),
        packet.excluded_term_id.as_str(),
        packet.member_id.as_str(),
    ) != (
        THEORY_CLASS_ID,
        SYMMETRY_ID,
        FIELD_ID,
        OPERATOR_ID,
        STATE_ID,
        SECTOR_ID,
        VALIDITY_ID,
        EXCLUDED_TERM_ID,
        MEMBER_ID,
    ) {
        return Err(PrimitiveProfileRefusal::TheoryProfileMismatch);
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
        return Err(PrimitiveProfileRefusal::EvidenceCustodyMismatch);
    }
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| PrimitiveProfileRefusal::SealedSourceUnavailable)?;
    if packet.floor_authority_schema_id != floor.schema_id().as_str()
        || packet.floor_authority_sha256 != floor.digest()
        || packet.floor_authority_sha256 == [0; 32]
    {
        return Err(PrimitiveProfileRefusal::FloorBindingMismatch);
    }
    let eps0 = repository_derived_relation_premise_frontier()
        .map_err(|_| PrimitiveProfileRefusal::SealedSourceUnavailable)?;
    if packet.eps0_pair_receipt_sha256 != eps0.pair_receipt_sha256
        || packet.eps0_capability_sha256 != eps0.capability_sha256
        || packet.eps0_pair_receipt_sha256 == [0; 32]
        || packet.eps0_capability_sha256 == [0; 32]
    {
        return Err(PrimitiveProfileRefusal::DerivedPremiseBindingMismatch);
    }
    if packet.global_physical_vocabulary_coverage || packet.membership_authority {
        return Err(PrimitiveProfileRefusal::UnsupportedScope);
    }
    Ok(())
}

fn build_candidates(
    packet: &PrimitiveProfilePacket,
    input_sha256: [u8; 32],
    evidence_custody_receipt_sha256: [u8; 32],
) -> Result<
    (
        Vec<ProfileArtifactCandidate>,
        SpeciesContentIdentity,
        ArtifactIdentity,
    ),
    PrimitiveProfileRefusal,
> {
    let mut candidates = Vec::with_capacity(ARTIFACT_COUNT);
    let profile_payload = descriptor(
        PROFILE_SCHEMA_ID,
        &format!(
            "profile_id={}\ntheory_class={}\nevidence_custody_sha256={}\nauthority_scope=claim-local\n",
            packet.profile_id,
            packet.theory_class_id,
            hex(evidence_custody_receipt_sha256)
        ),
    );
    let profile_root_identity = identity(&profile_payload)?;
    candidates.push(ProfileArtifactCandidate {
        identity: profile_root_identity,
        admission: irreducible_admission(packet, input_sha256, profile_root_identity),
        payload: profile_payload,
    });

    let derivation_kind = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.derivation-kind.v1",
        "primitive-excitation-from-admitted-theory-profile",
    )?;
    let field_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.role.field-input.v1",
        "field-input-role",
    )?;
    let field = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.field.v1",
        packet.field_id.as_str(),
    )?;
    let operator_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.role.operator-input.v1",
        "operator-input-role",
    )?;
    let operator = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.operator.v1",
        packet.operator_id.as_str(),
    )?;
    let state_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.role.state-requirement.v1",
        "state-requirement-role",
    )?;
    let state = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.state.v1",
        packet.state_id.as_str(),
    )?;
    let sector_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.role.sector-requirement.v1",
        "interaction-sector-requirement-role",
    )?;
    let sector = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.sector.v1",
        packet.sector_id.as_str(),
    )?;
    let validity_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.role.validity-requirement.v1",
        "validity-requirement-role",
    )?;
    let validity = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.validity.v1",
        packet.validity_id.as_str(),
    )?;
    let constraint_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.physical-profile.role.constraint.v1",
        "exact-null-dispersion-constraint-role",
    )?;

    let requirements = super::super::model::RequirementSet {
        artifact_relations: vec![
            relation(field_role, field),
            relation(state_role, state),
            relation(sector_role, sector),
            relation(validity_role, validity),
        ],
        species_dependencies: Vec::new(),
    };
    let constraint_payload =
        ArtifactPayload::ConstraintLaw(super::super::model::ConstraintLawArtifact {
            requirements: requirements.clone(),
        });
    let constraint = push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        constraint_payload,
    )?;

    let massless_payload =
        ArtifactPayload::ExactMasslessLaw(super::super::model::MasslessLawArtifact {
            requirements: requirements.clone(),
            proof: super::super::model::ExactZeroMassProof {
                subject: field,
                excluded_term: super::super::model::CanonicalArtifact {
                    schema_id: "civsim.physical-profile.excluded-mass-term.v1".to_owned(),
                    canonical_bytes: packet.excluded_term_id.as_bytes().to_vec(),
                },
                symmetry: sector,
                applicability_receipt: receipt_binding(
                    "civsim.primitive-profile.exact-zero-applicability.v1",
                    digest(
                        b"civsim.primitive-profile.exact-zero-applicability.v1",
                        &[
                            &input_sha256,
                            &field.0,
                            &sector.0,
                            packet.validity_id.as_bytes(),
                        ],
                    ),
                ),
                exclusion_producer_receipt: receipt_binding(
                    "civsim.primitive-profile.exact-zero-forward-proof.v1",
                    digest(
                        b"civsim.primitive-profile.exact-zero-forward-proof.v1",
                        &[
                            &input_sha256,
                            &field.0,
                            &sector.0,
                            packet.excluded_term_id.as_bytes(),
                        ],
                    ),
                ),
                exclusion_watchdog_receipt: receipt_binding(
                    "civsim.primitive-profile.exact-zero-reverse-proof.v1",
                    digest(
                        b"civsim.primitive-profile.exact-zero-reverse-proof.v1",
                        &[
                            &input_sha256,
                            &sector.0,
                            &field.0,
                            packet.excluded_term_id.as_bytes(),
                        ],
                    ),
                ),
            },
        });
    let massless = push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        massless_payload,
    )?;
    let blueprint = super::super::model::MemberBlueprint {
        physical_content: super::super::model::CanonicalArtifact {
            schema_id: "civsim.physical-profile.primitive-excitation.v1".to_owned(),
            canonical_bytes: packet.member_id.as_bytes().to_vec(),
        },
        requirements,
        mass_proof: super::super::model::MassProofReference::ExactMassless(massless),
        constraint_laws: vec![relation(constraint_role, constraint)],
    };
    let member = super::super::producer::derive_member_identity_for_authority(&blueprint)
        .map_err(|_| PrimitiveProfileRefusal::MemberIdentityMismatch)?;
    let species_payload =
        ArtifactPayload::SpeciesDerivation(super::super::model::SpeciesDerivationArtifact {
            derivation_kind,
            artifact_inputs: vec![
                relation(field_role, field),
                relation(operator_role, operator),
            ],
            constituents: Vec::new(),
            output: blueprint,
        });
    push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        species_payload,
    )?;
    Ok((candidates, member, profile_root_identity))
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

fn identity(payload: &ArtifactPayload) -> Result<ArtifactIdentity, PrimitiveProfileRefusal> {
    super::super::producer::derive_artifact_identity_for_authority(payload)
        .map_err(|_| PrimitiveProfileRefusal::ArtifactConstructionFailure)
}

fn push_descriptor(
    candidates: &mut Vec<ProfileArtifactCandidate>,
    profile_root_identity: ArtifactIdentity,
    input_sha256: [u8; 32],
    schema_id: &str,
    content: &str,
) -> Result<ArtifactIdentity, PrimitiveProfileRefusal> {
    push_derived(
        candidates,
        profile_root_identity,
        input_sha256,
        descriptor(schema_id, content),
    )
}

fn push_derived(
    candidates: &mut Vec<ProfileArtifactCandidate>,
    profile_root_identity: ArtifactIdentity,
    input_sha256: [u8; 32],
    payload: ArtifactPayload,
) -> Result<ArtifactIdentity, PrimitiveProfileRefusal> {
    let artifact_identity = identity(&payload)?;
    candidates.push(ProfileArtifactCandidate {
        identity: artifact_identity,
        admission: derived_admission(input_sha256, profile_root_identity, artifact_identity),
        payload,
    });
    Ok(artifact_identity)
}

fn irreducible_admission(
    packet: &PrimitiveProfilePacket,
    input_sha256: [u8; 32],
    profile_root_identity: ArtifactIdentity,
) -> RootAdmission {
    let exhaustion = digest(
        b"civsim.primitive-profile.derive-first-exhaustion.v1",
        &[
            &packet.floor_authority_sha256,
            &packet.eps0_pair_receipt_sha256,
            b"constants-and-execution-relations-do-not-select-field-ontology",
        ],
    );
    let pi = digest(
        b"civsim.primitive-profile.buckingham-pi.v1",
        &[
            &input_sha256,
            b"dimensionless-groups-cannot-select-discrete-gauge-ontology",
        ],
    );
    let gap = digest(
        b"civsim.primitive-profile.gap-law.v1",
        &[
            &input_sha256,
            b"structural-theory-seam-not-a-fitted-magnitude",
        ],
    );
    let chaos = digest(
        b"civsim.primitive-profile.chaos-protocol.v1",
        &[
            &input_sha256,
            b"not-applicable-static-theory-profile-no-evolving-trajectory",
        ],
    );
    let residual = digest(
        b"civsim.primitive-profile.residual-law.v1",
        &[
            &exhaustion,
            &pi,
            &gap,
            &chaos,
            b"one-minimal-unbroken-abelian-profile",
        ],
    );
    let slot = digest(
        b"civsim.primitive-profile.residual-slot.v1",
        &[
            packet.residual_slot_id.as_bytes(),
            &profile_root_identity.0,
            &residual,
        ],
    );
    let owner = digest(
        b"civsim.primitive-profile.owner-admission.v1",
        &[
            packet.owner_admission_record.as_bytes(),
            &profile_root_identity.0,
            &exhaustion,
            &pi,
            &gap,
            &chaos,
            &residual,
            &slot,
        ],
    );
    let independent = digest(
        b"civsim.primitive-profile.independent-watchdog-route.v1",
        &[
            &input_sha256,
            &profile_root_identity.0,
            &owner,
            WATCHDOG_ID.as_bytes(),
        ],
    );
    RootAdmission {
        tier: Tier::Residue,
        provenance: Provenance::Authored,
        route: super::super::model::AdmissionRoute::Irreducible(Box::new(
            super::super::model::IrreducibleAdmission {
                derivation_exhaustion_receipt: receipt_binding(
                    "civsim.primitive-profile.derive-first-exhaustion.v1",
                    exhaustion,
                ),
                buckingham_pi_receipt: receipt_binding(
                    "civsim.primitive-profile.buckingham-pi.v1",
                    pi,
                ),
                gap_law_receipt: receipt_binding("civsim.primitive-profile.gap-law.v1", gap),
                chaos_protocol_receipt: receipt_binding(
                    "civsim.primitive-profile.chaos-protocol.v1",
                    chaos,
                ),
                residual_law_receipt: receipt_binding(
                    "civsim.primitive-profile.residual-law.v1",
                    residual,
                ),
                residual_slot_id: packet.residual_slot_id.clone(),
                residual_slot_receipt: receipt_binding(
                    "civsim.primitive-profile.residual-slot.v1",
                    slot,
                ),
                owner_admission_receipt: receipt_binding(
                    "civsim.primitive-profile.owner-admission.v1",
                    owner,
                ),
                independent_watchdog_receipt: receipt_binding(
                    "civsim.primitive-profile.independent-watchdog-route.v1",
                    independent,
                ),
            },
        )),
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
        route: super::super::model::AdmissionRoute::Derived(
            super::super::model::DerivedAdmission {
                ancestry_receipt: receipt_binding(
                    "civsim.primitive-profile.derived-ancestry.v1",
                    digest(
                        b"civsim.primitive-profile.derived-ancestry.v1",
                        &[&profile_root_identity.0, &artifact_identity.0],
                    ),
                ),
                semantic_checker_receipt: receipt_binding(
                    "civsim.primitive-profile.derived-forward-semantic.v1",
                    digest(
                        b"civsim.primitive-profile.derived-forward-semantic.v1",
                        &[&input_sha256, &artifact_identity.0],
                    ),
                ),
                independent_watchdog_receipt: receipt_binding(
                    "civsim.primitive-profile.derived-reverse-semantic.v1",
                    digest(
                        b"civsim.primitive-profile.derived-reverse-semantic.v1",
                        &[&artifact_identity.0, &input_sha256],
                    ),
                ),
            },
        ),
    }
}

pub(super) fn canary_evidence(
    packet: &PrimitiveProfilePacket,
) -> Result<CanaryEvidence, PrimitiveProfileRefusal> {
    let mut transcript = PRODUCER_CANARY_ID.as_bytes().to_vec();
    let mut cases = mutations(packet);
    for (name, mutant) in &mut cases {
        let code = inspect(mutant)
            .expect_err("every primitive-profile producer canary must refuse")
            .id();
        append_field(&mut transcript, 1, name.as_bytes());
        append_field(&mut transcript, 2, &sha256(&encode_packet(mutant)));
        append_field(&mut transcript, 3, code.as_bytes());
    }
    Ok(CanaryEvidence {
        transcript_id: PRODUCER_CANARY_ID,
        case_count: u32::try_from(cases.len())
            .map_err(|_| PrimitiveProfileRefusal::CanaryFailure)?,
        transcript_sha256: sha256(&transcript),
    })
}

fn mutations(packet: &PrimitiveProfilePacket) -> Vec<(&'static str, PrimitiveProfilePacket)> {
    let mut cases = Vec::new();
    let mut changed = packet.clone();
    changed.schema_id.push_str(".changed");
    cases.push(("packet_schema", changed));
    let mut changed = packet.clone();
    changed.profile_id.push_str(".changed");
    cases.push(("profile_identity", changed));
    let mut changed = packet.clone();
    changed.theory_class_id.push_str(".changed");
    cases.push(("theory_class", changed));
    let mut changed = packet.clone();
    changed.symmetry_id.push_str(".changed");
    cases.push(("symmetry", changed));
    let mut changed = packet.clone();
    changed.field_id.push_str(".changed");
    cases.push(("field", changed));
    let mut changed = packet.clone();
    changed.operator_id.push_str(".changed");
    cases.push(("operator", changed));
    let mut changed = packet.clone();
    changed.state_id.push_str(".changed");
    cases.push(("state", changed));
    let mut changed = packet.clone();
    changed.sector_id.push_str(".changed");
    cases.push(("sector", changed));
    let mut changed = packet.clone();
    changed.validity_id.push_str(".changed");
    cases.push(("validity", changed));
    let mut changed = packet.clone();
    changed.excluded_term_id.push_str(".changed");
    cases.push(("excluded_mass_term", changed));
    let mut changed = packet.clone();
    changed.member_id.push_str(".changed");
    cases.push(("member", changed));
    let mut changed = packet.clone();
    changed.residual_slot_id.push_str(".changed");
    cases.push(("residual_slot", changed));
    let mut changed = packet.clone();
    changed.evidence_slim_sha256_hex.replace_range(0..1, "0");
    cases.push(("evidence_slim_receipt", changed));
    let mut changed = packet.clone();
    changed.evidence_full_sha256_hex.replace_range(0..1, "0");
    cases.push(("evidence_full_receipt", changed));
    let mut changed = packet.clone();
    changed.evidence_source_url.push_str("#changed");
    cases.push(("evidence_source", changed));
    let mut changed = packet.clone();
    changed.evidence_citation.push_str(".changed");
    cases.push(("evidence_citation", changed));
    let mut changed = packet.clone();
    changed.evidence_anchor.push_str(".changed");
    cases.push(("evidence_anchor", changed));
    let mut changed = packet.clone();
    changed.floor_authority_sha256[0] ^= 1;
    cases.push(("floor_authority", changed));
    let mut changed = packet.clone();
    changed.eps0_capability_sha256[0] ^= 1;
    cases.push(("eps0_capability", changed));
    let mut changed = packet.clone();
    changed.eps0_pair_receipt_sha256[0] ^= 1;
    cases.push(("eps0_pair_receipt", changed));
    let mut changed = packet.clone();
    changed.owner_admission_record.push_str(".changed");
    cases.push(("owner_admission", changed));
    let mut changed = packet.clone();
    changed.global_physical_vocabulary_coverage = true;
    cases.push(("global_coverage_escalation", changed));
    let mut changed = packet.clone();
    changed.membership_authority = true;
    cases.push(("membership_escalation", changed));
    cases
}

pub(super) fn pair_receipt_digest(
    output: &PrimitiveProfileCheckerOutput,
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
    producer_canary: CanaryEvidence,
    watchdog_canary: CanaryEvidence,
) -> [u8; 32] {
    digest(
        PAIR_DOMAIN,
        &[
            RECEIPT_SCHEMA_ID.as_bytes(),
            CLAIM_ID.as_bytes(),
            PRODUCER_ID.as_bytes(),
            WATCHDOG_ID.as_bytes(),
            &output.input_sha256,
            &producer_result_sha256,
            &watchdog_result_sha256,
            producer_canary.transcript_id.as_bytes(),
            &producer_canary.case_count.to_be_bytes(),
            &producer_canary.transcript_sha256,
            watchdog_canary.transcript_id.as_bytes(),
            &watchdog_canary.case_count.to_be_bytes(),
            &watchdog_canary.transcript_sha256,
            &output.profile_root_identity.0,
            &output.member.0,
            &u32::try_from(output.candidates.len())
                .unwrap_or(u32::MAX)
                .to_be_bytes(),
            &output.evidence_custody_receipt_sha256,
            &[0],
            &[0],
            b"none",
        ],
    )
}

fn encode_packet(packet: &PrimitiveProfilePacket) -> Vec<u8> {
    let mut bytes = INPUT_DOMAIN.to_vec();
    let fields: [&[u8]; 23] = [
        packet.schema_id.as_bytes(),
        packet.profile_id.as_bytes(),
        packet.theory_class_id.as_bytes(),
        packet.symmetry_id.as_bytes(),
        packet.field_id.as_bytes(),
        packet.operator_id.as_bytes(),
        packet.state_id.as_bytes(),
        packet.sector_id.as_bytes(),
        packet.validity_id.as_bytes(),
        packet.excluded_term_id.as_bytes(),
        packet.member_id.as_bytes(),
        packet.residual_slot_id.as_bytes(),
        packet.owner_admission_record.as_bytes(),
        packet.evidence_citation.as_bytes(),
        packet.evidence_source_url.as_bytes(),
        packet.evidence_full_sha256_hex.as_bytes(),
        packet.evidence_slim_sha256_hex.as_bytes(),
        packet.evidence_anchor.as_bytes(),
        packet.floor_authority_schema_id.as_bytes(),
        &packet.floor_authority_sha256,
        &packet.eps0_pair_receipt_sha256,
        &packet.eps0_capability_sha256,
        &[
            u8::from(packet.global_physical_vocabulary_coverage),
            u8::from(packet.membership_authority),
        ],
    ];
    for (index, field) in fields.into_iter().enumerate() {
        append_field(
            &mut bytes,
            u16::try_from(index + 1).unwrap_or(u16::MAX),
            field,
        );
    }
    bytes
}

fn encode_output(
    input_sha256: [u8; 32],
    candidates: &[ProfileArtifactCandidate],
    member: SpeciesContentIdentity,
    profile_root_identity: ArtifactIdentity,
    evidence_custody_receipt_sha256: [u8; 32],
) -> Vec<u8> {
    let mut bytes = OUTPUT_DOMAIN.to_vec();
    append_field(&mut bytes, 1, &input_sha256);
    append_field(&mut bytes, 2, &profile_root_identity.0);
    append_field(&mut bytes, 3, &member.0);
    append_field(&mut bytes, 4, &evidence_custody_receipt_sha256);
    append_field(
        &mut bytes,
        5,
        &u32::try_from(candidates.len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    for candidate in candidates {
        append_field(&mut bytes, 6, &candidate.identity.0);
        append_field(&mut bytes, 7, &encode_admission(&candidate.admission));
    }
    append_field(&mut bytes, 8, &[0]);
    append_field(&mut bytes, 9, &[0]);
    bytes
}

fn encode_admission(admission: &RootAdmission) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, admission.tier.id().as_bytes());
    append_field(&mut bytes, 2, admission.provenance.tag().as_bytes());
    match &admission.route {
        super::super::model::AdmissionRoute::Derived(route) => {
            append_field(&mut bytes, 3, b"derived");
            for receipt in [
                &route.ancestry_receipt,
                &route.semantic_checker_receipt,
                &route.independent_watchdog_receipt,
            ] {
                append_field(&mut bytes, 4, receipt.schema_id.as_bytes());
                append_field(&mut bytes, 5, &receipt.digest_sha256);
            }
        }
        super::super::model::AdmissionRoute::Irreducible(route) => {
            append_field(&mut bytes, 3, b"irreducible");
            for receipt in [
                &route.derivation_exhaustion_receipt,
                &route.buckingham_pi_receipt,
                &route.gap_law_receipt,
                &route.chaos_protocol_receipt,
                &route.residual_law_receipt,
            ] {
                append_field(&mut bytes, 4, receipt.schema_id.as_bytes());
                append_field(&mut bytes, 5, &receipt.digest_sha256);
            }
            append_field(&mut bytes, 6, route.residual_slot_id.as_bytes());
            for receipt in [
                &route.residual_slot_receipt,
                &route.owner_admission_receipt,
                &route.independent_watchdog_receipt,
            ] {
                append_field(&mut bytes, 7, receipt.schema_id.as_bytes());
                append_field(&mut bytes, 8, &receipt.digest_sha256);
            }
        }
        super::super::model::AdmissionRoute::EvidenceCustodyOnly { source_receipt } => {
            append_field(&mut bytes, 3, b"evidence-custody-only");
            append_field(&mut bytes, 4, source_receipt.schema_id.as_bytes());
            append_field(&mut bytes, 5, &source_receipt.digest_sha256);
        }
    }
    bytes
}

fn digest(domain: &[u8], fields: &[&[u8]]) -> [u8; 32] {
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

fn hex(bytes: [u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}
