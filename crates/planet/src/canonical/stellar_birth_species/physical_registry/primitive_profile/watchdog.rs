//! Reverse reconstruction watchdog for the primitive profile.

use super::*;
use crate::canonical::stellar_birth_species::law_premise::repository_derived_relation_premise_frontier;
use civsim_ledger::{Provenance, Tier};
use civsim_units::{digest::sha256, physics_floor::sealed_physical_floor_authority_binding};
use std::collections::BTreeMap;

const INPUT_DOMAIN: &[u8] = b"civsim.planet.primitive-excitation-profile.producer-input.v1";
const OUTPUT_DOMAIN: &[u8] = b"civsim.planet.primitive-excitation-profile.canonical-output.v1";
const PAIR_DOMAIN: &[u8] = b"civsim.planet.primitive-excitation-profile.pair-receipt.v1";
const BYTE_LIMIT: u32 = 1_048_576;

pub(super) fn sealed_packet() -> Result<PrimitiveProfilePacket, PrimitiveProfileRefusal> {
    let eps0 = repository_derived_relation_premise_frontier()
        .map_err(|_| PrimitiveProfileRefusal::SealedSourceUnavailable)?;
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| PrimitiveProfileRefusal::SealedSourceUnavailable)?;
    Ok(PrimitiveProfilePacket {
        membership_authority: false,
        global_physical_vocabulary_coverage: false,
        eps0_capability_sha256: eps0.capability_sha256,
        eps0_pair_receipt_sha256: eps0.pair_receipt_sha256,
        floor_authority_sha256: floor.digest(),
        floor_authority_schema_id: floor.schema_id().as_str().to_owned(),
        evidence_anchor: EVIDENCE_ANCHOR.to_owned(),
        evidence_slim_sha256_hex: EVIDENCE_SLIM_SHA256_HEX.to_owned(),
        evidence_full_sha256_hex: EVIDENCE_FULL_SHA256_HEX.to_owned(),
        evidence_source_url: EVIDENCE_SOURCE_URL.to_owned(),
        evidence_citation: EVIDENCE_CITATION.to_owned(),
        owner_admission_record: OWNER_ADMISSION_RECORD.to_owned(),
        residual_slot_id: RESIDUAL_SLOT_ID.to_owned(),
        member_id: MEMBER_ID.to_owned(),
        excluded_term_id: EXCLUDED_TERM_ID.to_owned(),
        validity_id: VALIDITY_ID.to_owned(),
        sector_id: SECTOR_ID.to_owned(),
        state_id: STATE_ID.to_owned(),
        operator_id: OPERATOR_ID.to_owned(),
        field_id: FIELD_ID.to_owned(),
        symmetry_id: SYMMETRY_ID.to_owned(),
        theory_class_id: THEORY_CLASS_ID.to_owned(),
        profile_id: PROFILE_ID.to_owned(),
        schema_id: PACKET_SCHEMA_ID.to_owned(),
    })
}

pub(super) fn inspect(
    packet: &PrimitiveProfilePacket,
) -> Result<PrimitiveProfileCheckerOutput, PrimitiveProfileRefusal> {
    inspect_packet(packet)?;
    let packet_bytes = write_packet(packet);
    if u32::try_from(packet_bytes.len()).map_or(true, |length| length > BYTE_LIMIT) {
        return Err(PrimitiveProfileRefusal::ArtifactConstructionFailure);
    }
    let input_sha256 = sha256(&packet_bytes);
    let evidence_custody_receipt_sha256 = hash_fields(
        b"civsim.planet.primitive-excitation-profile.evidence-custody.v1",
        [
            packet.evidence_citation.as_bytes(),
            packet.evidence_source_url.as_bytes(),
            packet.evidence_full_sha256_hex.as_bytes(),
            packet.evidence_slim_sha256_hex.as_bytes(),
            packet.evidence_anchor.as_bytes(),
        ],
    );
    let (candidates, member, profile_root_identity) =
        reconstruct_candidates(packet, input_sha256, evidence_custody_receipt_sha256)?;
    if candidates.len() != ARTIFACT_COUNT {
        return Err(PrimitiveProfileRefusal::ArtifactCountMismatch);
    }
    let canonical_bytes = write_output(
        input_sha256,
        &candidates,
        member,
        profile_root_identity,
        evidence_custody_receipt_sha256,
    );
    if u32::try_from(canonical_bytes.len()).map_or(true, |length| length > BYTE_LIMIT) {
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

fn inspect_packet(packet: &PrimitiveProfilePacket) -> Result<(), PrimitiveProfileRefusal> {
    if packet.schema_id.as_bytes() != PACKET_SCHEMA_ID.as_bytes() {
        return Err(PrimitiveProfileRefusal::PacketSchemaMismatch);
    }
    let profile_identity_matches = [
        (packet.profile_id.as_str(), PROFILE_ID),
        (packet.residual_slot_id.as_str(), RESIDUAL_SLOT_ID),
        (
            packet.owner_admission_record.as_str(),
            OWNER_ADMISSION_RECORD,
        ),
    ]
    .into_iter()
    .all(|(found, expected)| found == expected);
    if !profile_identity_matches {
        return Err(PrimitiveProfileRefusal::ProfileIdentityMismatch);
    }
    let theory_fields = [
        (packet.member_id.as_str(), MEMBER_ID),
        (packet.excluded_term_id.as_str(), EXCLUDED_TERM_ID),
        (packet.validity_id.as_str(), VALIDITY_ID),
        (packet.sector_id.as_str(), SECTOR_ID),
        (packet.state_id.as_str(), STATE_ID),
        (packet.operator_id.as_str(), OPERATOR_ID),
        (packet.field_id.as_str(), FIELD_ID),
        (packet.symmetry_id.as_str(), SYMMETRY_ID),
        (packet.theory_class_id.as_str(), THEORY_CLASS_ID),
    ];
    if theory_fields
        .into_iter()
        .any(|(found, expected)| found != expected)
    {
        return Err(PrimitiveProfileRefusal::TheoryProfileMismatch);
    }
    let evidence_fields = [
        (packet.evidence_anchor.as_str(), EVIDENCE_ANCHOR),
        (
            packet.evidence_slim_sha256_hex.as_str(),
            EVIDENCE_SLIM_SHA256_HEX,
        ),
        (
            packet.evidence_full_sha256_hex.as_str(),
            EVIDENCE_FULL_SHA256_HEX,
        ),
        (packet.evidence_source_url.as_str(), EVIDENCE_SOURCE_URL),
        (packet.evidence_citation.as_str(), EVIDENCE_CITATION),
    ];
    if evidence_fields
        .into_iter()
        .any(|(found, expected)| found != expected)
    {
        return Err(PrimitiveProfileRefusal::EvidenceCustodyMismatch);
    }
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| PrimitiveProfileRefusal::SealedSourceUnavailable)?;
    if (
        packet.floor_authority_schema_id.as_str(),
        packet.floor_authority_sha256,
    ) != (floor.schema_id().as_str(), floor.digest())
        || packet.floor_authority_sha256.iter().all(|byte| *byte == 0)
    {
        return Err(PrimitiveProfileRefusal::FloorBindingMismatch);
    }
    let eps0 = repository_derived_relation_premise_frontier()
        .map_err(|_| PrimitiveProfileRefusal::SealedSourceUnavailable)?;
    if (
        packet.eps0_capability_sha256,
        packet.eps0_pair_receipt_sha256,
    ) != (eps0.capability_sha256, eps0.pair_receipt_sha256)
        || packet.eps0_capability_sha256.iter().all(|byte| *byte == 0)
        || packet
            .eps0_pair_receipt_sha256
            .iter()
            .all(|byte| *byte == 0)
    {
        return Err(PrimitiveProfileRefusal::DerivedPremiseBindingMismatch);
    }
    if packet.membership_authority || packet.global_physical_vocabulary_coverage {
        return Err(PrimitiveProfileRefusal::UnsupportedScope);
    }
    Ok(())
}

fn reconstruct_candidates(
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
    let mut by_key = BTreeMap::<&'static str, ProfileArtifactCandidate>::new();
    let profile_payload = make_descriptor(
        PROFILE_SCHEMA_ID,
        format!(
            "profile_id={}\ntheory_class={}\nevidence_custody_sha256={}\nauthority_scope=claim-local\n",
            packet.profile_id,
            packet.theory_class_id,
            to_hex(evidence_custody_receipt_sha256)
        )
        .into_bytes(),
    );
    let profile_root_identity = recompute_identity(&profile_payload)?;
    by_key.insert(
        "profile",
        ProfileArtifactCandidate {
            identity: profile_root_identity,
            admission: reconstruct_irreducible_admission(
                packet,
                input_sha256,
                profile_root_identity,
            ),
            payload: profile_payload,
        },
    );

    let descriptor_specs = [
        (
            "derivation_kind",
            "civsim.physical-profile.derivation-kind.v1",
            "primitive-excitation-from-admitted-theory-profile",
        ),
        (
            "field_role",
            "civsim.physical-profile.role.field-input.v1",
            "field-input-role",
        ),
        (
            "field",
            "civsim.physical-profile.field.v1",
            packet.field_id.as_str(),
        ),
        (
            "operator_role",
            "civsim.physical-profile.role.operator-input.v1",
            "operator-input-role",
        ),
        (
            "operator",
            "civsim.physical-profile.operator.v1",
            packet.operator_id.as_str(),
        ),
        (
            "state_role",
            "civsim.physical-profile.role.state-requirement.v1",
            "state-requirement-role",
        ),
        (
            "state",
            "civsim.physical-profile.state.v1",
            packet.state_id.as_str(),
        ),
        (
            "sector_role",
            "civsim.physical-profile.role.sector-requirement.v1",
            "interaction-sector-requirement-role",
        ),
        (
            "sector",
            "civsim.physical-profile.sector.v1",
            packet.sector_id.as_str(),
        ),
        (
            "validity_role",
            "civsim.physical-profile.role.validity-requirement.v1",
            "validity-requirement-role",
        ),
        (
            "validity",
            "civsim.physical-profile.validity.v1",
            packet.validity_id.as_str(),
        ),
        (
            "constraint_role",
            "civsim.physical-profile.role.constraint.v1",
            "exact-null-dispersion-constraint-role",
        ),
    ];
    for (key, schema, text) in descriptor_specs {
        insert_derived(
            &mut by_key,
            key,
            profile_root_identity,
            input_sha256,
            make_descriptor(schema, text.as_bytes().to_vec()),
        )?;
    }

    let derivation_kind = lookup_identity(&by_key, "derivation_kind")?;
    let field_role = lookup_identity(&by_key, "field_role")?;
    let field = lookup_identity(&by_key, "field")?;
    let operator_role = lookup_identity(&by_key, "operator_role")?;
    let operator = lookup_identity(&by_key, "operator")?;
    let state_role = lookup_identity(&by_key, "state_role")?;
    let state = lookup_identity(&by_key, "state")?;
    let sector_role = lookup_identity(&by_key, "sector_role")?;
    let sector = lookup_identity(&by_key, "sector")?;
    let validity_role = lookup_identity(&by_key, "validity_role")?;
    let validity = lookup_identity(&by_key, "validity")?;
    let constraint_role = lookup_identity(&by_key, "constraint_role")?;

    let requirements = super::super::model::RequirementSet {
        artifact_relations: vec![
            edge(field_role, field),
            edge(state_role, state),
            edge(sector_role, sector),
            edge(validity_role, validity),
        ],
        species_dependencies: Vec::new(),
    };
    insert_derived(
        &mut by_key,
        "constraint",
        profile_root_identity,
        input_sha256,
        ArtifactPayload::ConstraintLaw(super::super::model::ConstraintLawArtifact {
            requirements: requirements.clone(),
        }),
    )?;
    let constraint = lookup_identity(&by_key, "constraint")?;
    let exact_zero = super::super::model::ExactZeroMassProof {
        subject: field,
        excluded_term: super::super::model::CanonicalArtifact {
            schema_id: "civsim.physical-profile.excluded-mass-term.v1".to_owned(),
            canonical_bytes: packet.excluded_term_id.as_bytes().to_vec(),
        },
        symmetry: sector,
        applicability_receipt: receipt_binding(
            "civsim.primitive-profile.exact-zero-applicability.v1",
            hash_fields(
                b"civsim.primitive-profile.exact-zero-applicability.v1",
                [
                    input_sha256.as_slice(),
                    field.0.as_slice(),
                    sector.0.as_slice(),
                    packet.validity_id.as_bytes(),
                ],
            ),
        ),
        exclusion_producer_receipt: receipt_binding(
            "civsim.primitive-profile.exact-zero-forward-proof.v1",
            hash_fields(
                b"civsim.primitive-profile.exact-zero-forward-proof.v1",
                [
                    input_sha256.as_slice(),
                    field.0.as_slice(),
                    sector.0.as_slice(),
                    packet.excluded_term_id.as_bytes(),
                ],
            ),
        ),
        exclusion_watchdog_receipt: receipt_binding(
            "civsim.primitive-profile.exact-zero-reverse-proof.v1",
            hash_fields(
                b"civsim.primitive-profile.exact-zero-reverse-proof.v1",
                [
                    input_sha256.as_slice(),
                    sector.0.as_slice(),
                    field.0.as_slice(),
                    packet.excluded_term_id.as_bytes(),
                ],
            ),
        ),
    };
    insert_derived(
        &mut by_key,
        "massless",
        profile_root_identity,
        input_sha256,
        ArtifactPayload::ExactMasslessLaw(super::super::model::MasslessLawArtifact {
            requirements: requirements.clone(),
            proof: exact_zero,
        }),
    )?;
    let massless = lookup_identity(&by_key, "massless")?;
    let blueprint = super::super::model::MemberBlueprint {
        physical_content: super::super::model::CanonicalArtifact {
            schema_id: "civsim.physical-profile.primitive-excitation.v1".to_owned(),
            canonical_bytes: packet.member_id.as_bytes().to_vec(),
        },
        requirements,
        mass_proof: super::super::model::MassProofReference::ExactMassless(massless),
        constraint_laws: vec![edge(constraint_role, constraint)],
    };
    let member = super::super::watchdog::derive_member_identity_for_authority(&blueprint)
        .map_err(|_| PrimitiveProfileRefusal::MemberIdentityMismatch)?;
    insert_derived(
        &mut by_key,
        "species",
        profile_root_identity,
        input_sha256,
        ArtifactPayload::SpeciesDerivation(super::super::model::SpeciesDerivationArtifact {
            derivation_kind,
            artifact_inputs: vec![edge(field_role, field), edge(operator_role, operator)],
            constituents: Vec::new(),
            output: blueprint,
        }),
    )?;

    let order = [
        "profile",
        "derivation_kind",
        "field_role",
        "field",
        "operator_role",
        "operator",
        "state_role",
        "state",
        "sector_role",
        "sector",
        "validity_role",
        "validity",
        "constraint_role",
        "constraint",
        "massless",
        "species",
    ];
    let candidates = order
        .into_iter()
        .map(|key| {
            by_key
                .remove(key)
                .ok_or(PrimitiveProfileRefusal::ArtifactConstructionFailure)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !by_key.is_empty() {
        return Err(PrimitiveProfileRefusal::ArtifactCountMismatch);
    }
    Ok((candidates, member, profile_root_identity))
}

fn make_descriptor(schema_id: &str, bytes: Vec<u8>) -> ArtifactPayload {
    ArtifactPayload::PhysicalDescriptor(super::super::model::CanonicalArtifact {
        schema_id: schema_id.to_owned(),
        canonical_bytes: bytes,
    })
}

fn lookup_identity(
    by_key: &BTreeMap<&'static str, ProfileArtifactCandidate>,
    key: &'static str,
) -> Result<ArtifactIdentity, PrimitiveProfileRefusal> {
    by_key
        .get(key)
        .map(|candidate| candidate.identity)
        .ok_or(PrimitiveProfileRefusal::ArtifactConstructionFailure)
}

fn edge(role: ArtifactIdentity, target: ArtifactIdentity) -> super::super::model::ArtifactRelation {
    super::super::model::ArtifactRelation { role, target }
}

fn recompute_identity(
    payload: &ArtifactPayload,
) -> Result<ArtifactIdentity, PrimitiveProfileRefusal> {
    super::super::watchdog::derive_artifact_identity_for_authority(payload)
        .map_err(|_| PrimitiveProfileRefusal::ArtifactConstructionFailure)
}

fn insert_derived(
    by_key: &mut BTreeMap<&'static str, ProfileArtifactCandidate>,
    key: &'static str,
    profile_root_identity: ArtifactIdentity,
    input_sha256: [u8; 32],
    payload: ArtifactPayload,
) -> Result<(), PrimitiveProfileRefusal> {
    let identity = recompute_identity(&payload)?;
    let candidate = ProfileArtifactCandidate {
        identity,
        admission: reconstruct_derived_admission(input_sha256, profile_root_identity, identity),
        payload,
    };
    if by_key.insert(key, candidate).is_some() {
        return Err(PrimitiveProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(())
}

fn reconstruct_irreducible_admission(
    packet: &PrimitiveProfilePacket,
    input_sha256: [u8; 32],
    profile_root_identity: ArtifactIdentity,
) -> RootAdmission {
    let exhaustion = hash_fields(
        b"civsim.primitive-profile.derive-first-exhaustion.v1",
        [
            packet.floor_authority_sha256.as_slice(),
            packet.eps0_pair_receipt_sha256.as_slice(),
            b"constants-and-execution-relations-do-not-select-field-ontology",
        ],
    );
    let pi = hash_fields(
        b"civsim.primitive-profile.buckingham-pi.v1",
        [
            input_sha256.as_slice(),
            b"dimensionless-groups-cannot-select-discrete-gauge-ontology",
        ],
    );
    let gap = hash_fields(
        b"civsim.primitive-profile.gap-law.v1",
        [
            input_sha256.as_slice(),
            b"structural-theory-seam-not-a-fitted-magnitude",
        ],
    );
    let chaos = hash_fields(
        b"civsim.primitive-profile.chaos-protocol.v1",
        [
            input_sha256.as_slice(),
            b"not-applicable-static-theory-profile-no-evolving-trajectory",
        ],
    );
    let residual = hash_fields(
        b"civsim.primitive-profile.residual-law.v1",
        [
            exhaustion.as_slice(),
            pi.as_slice(),
            gap.as_slice(),
            chaos.as_slice(),
            b"one-minimal-unbroken-abelian-profile",
        ],
    );
    let slot = hash_fields(
        b"civsim.primitive-profile.residual-slot.v1",
        [
            packet.residual_slot_id.as_bytes(),
            profile_root_identity.0.as_slice(),
            residual.as_slice(),
        ],
    );
    let owner = hash_fields(
        b"civsim.primitive-profile.owner-admission.v1",
        [
            packet.owner_admission_record.as_bytes(),
            profile_root_identity.0.as_slice(),
            exhaustion.as_slice(),
            pi.as_slice(),
            gap.as_slice(),
            chaos.as_slice(),
            residual.as_slice(),
            slot.as_slice(),
        ],
    );
    let independent = hash_fields(
        b"civsim.primitive-profile.independent-watchdog-route.v1",
        [
            input_sha256.as_slice(),
            profile_root_identity.0.as_slice(),
            owner.as_slice(),
            WATCHDOG_ID.as_bytes(),
        ],
    );
    RootAdmission {
        tier: Tier::Residue,
        provenance: Provenance::Authored,
        route: super::super::model::AdmissionRoute::Irreducible(Box::new(
            super::super::model::IrreducibleAdmission {
                independent_watchdog_receipt: receipt_binding(
                    "civsim.primitive-profile.independent-watchdog-route.v1",
                    independent,
                ),
                owner_admission_receipt: receipt_binding(
                    "civsim.primitive-profile.owner-admission.v1",
                    owner,
                ),
                residual_slot_receipt: receipt_binding(
                    "civsim.primitive-profile.residual-slot.v1",
                    slot,
                ),
                residual_slot_id: packet.residual_slot_id.clone(),
                residual_law_receipt: receipt_binding(
                    "civsim.primitive-profile.residual-law.v1",
                    residual,
                ),
                chaos_protocol_receipt: receipt_binding(
                    "civsim.primitive-profile.chaos-protocol.v1",
                    chaos,
                ),
                gap_law_receipt: receipt_binding("civsim.primitive-profile.gap-law.v1", gap),
                buckingham_pi_receipt: receipt_binding(
                    "civsim.primitive-profile.buckingham-pi.v1",
                    pi,
                ),
                derivation_exhaustion_receipt: receipt_binding(
                    "civsim.primitive-profile.derive-first-exhaustion.v1",
                    exhaustion,
                ),
            },
        )),
    }
}

fn reconstruct_derived_admission(
    input_sha256: [u8; 32],
    profile_root_identity: ArtifactIdentity,
    artifact_identity: ArtifactIdentity,
) -> RootAdmission {
    RootAdmission {
        tier: Tier::Residue,
        provenance: Provenance::Derived,
        route: super::super::model::AdmissionRoute::Derived(
            super::super::model::DerivedAdmission {
                independent_watchdog_receipt: receipt_binding(
                    "civsim.primitive-profile.derived-reverse-semantic.v1",
                    hash_fields(
                        b"civsim.primitive-profile.derived-reverse-semantic.v1",
                        [artifact_identity.0.as_slice(), input_sha256.as_slice()],
                    ),
                ),
                semantic_checker_receipt: receipt_binding(
                    "civsim.primitive-profile.derived-forward-semantic.v1",
                    hash_fields(
                        b"civsim.primitive-profile.derived-forward-semantic.v1",
                        [input_sha256.as_slice(), artifact_identity.0.as_slice()],
                    ),
                ),
                ancestry_receipt: receipt_binding(
                    "civsim.primitive-profile.derived-ancestry.v1",
                    hash_fields(
                        b"civsim.primitive-profile.derived-ancestry.v1",
                        [
                            profile_root_identity.0.as_slice(),
                            artifact_identity.0.as_slice(),
                        ],
                    ),
                ),
            },
        ),
    }
}

pub(super) fn canary_evidence(
    packet: &PrimitiveProfilePacket,
) -> Result<CanaryEvidence, PrimitiveProfileRefusal> {
    let cases = reversed_mutations(packet);
    let mut transcript = WATCHDOG_CANARY_ID.as_bytes().to_vec();
    for (name, mutant) in cases.iter().rev() {
        let refusal = inspect(mutant)
            .expect_err("every primitive-profile watchdog canary must refuse")
            .id();
        write_field(&mut transcript, 1, name.as_bytes());
        write_field(&mut transcript, 2, &sha256(&write_packet(mutant)));
        write_field(&mut transcript, 3, refusal.as_bytes());
    }
    Ok(CanaryEvidence {
        transcript_id: WATCHDOG_CANARY_ID,
        case_count: u32::try_from(cases.len())
            .map_err(|_| PrimitiveProfileRefusal::CanaryFailure)?,
        transcript_sha256: sha256(&transcript),
    })
}

fn reversed_mutations(
    packet: &PrimitiveProfilePacket,
) -> Vec<(&'static str, PrimitiveProfilePacket)> {
    let mut output = Vec::new();
    let mut changed = packet.clone();
    changed.membership_authority = true;
    output.push(("membership_escalation", changed));
    let mut changed = packet.clone();
    changed.global_physical_vocabulary_coverage = true;
    output.push(("global_coverage_escalation", changed));
    let mut changed = packet.clone();
    changed.owner_admission_record.insert(0, 'x');
    output.push(("owner_admission", changed));
    let mut changed = packet.clone();
    changed.eps0_capability_sha256[0] = changed.eps0_capability_sha256[0].wrapping_add(1);
    output.push(("eps0_capability", changed));
    let mut changed = packet.clone();
    changed.floor_authority_sha256[0] = changed.floor_authority_sha256[0].wrapping_add(1);
    output.push(("floor_authority", changed));
    let mut changed = packet.clone();
    changed.evidence_slim_sha256_hex.insert(0, '0');
    output.push(("evidence_slim_receipt", changed));
    let mut changed = packet.clone();
    changed.symmetry_id.insert(0, 'x');
    output.push(("symmetry", changed));
    let mut changed = packet.clone();
    changed.field_id.insert(0, 'x');
    output.push(("field", changed));
    let mut changed = packet.clone();
    changed.operator_id.insert(0, 'x');
    output.push(("operator", changed));
    let mut changed = packet.clone();
    changed.state_id.insert(0, 'x');
    output.push(("state", changed));
    let mut changed = packet.clone();
    changed.sector_id.insert(0, 'x');
    output.push(("sector", changed));
    let mut changed = packet.clone();
    changed.validity_id.insert(0, 'x');
    output.push(("validity", changed));
    let mut changed = packet.clone();
    changed.excluded_term_id.insert(0, 'x');
    output.push(("excluded_mass_term", changed));
    let mut changed = packet.clone();
    changed.member_id.insert(0, 'x');
    output.push(("member", changed));
    let mut changed = packet.clone();
    changed.residual_slot_id.insert(0, 'x');
    output.push(("residual_slot", changed));
    let mut changed = packet.clone();
    changed.theory_class_id.insert(0, 'x');
    output.push(("theory_class", changed));
    let mut changed = packet.clone();
    changed.profile_id.insert(0, 'x');
    output.push(("profile_identity", changed));
    let mut changed = packet.clone();
    changed.schema_id.insert(0, 'x');
    output.push(("packet_schema", changed));
    let mut changed = packet.clone();
    changed.evidence_full_sha256_hex.insert(0, '0');
    output.push(("evidence_full_receipt", changed));
    let mut changed = packet.clone();
    changed.evidence_source_url.insert(0, 'x');
    output.push(("evidence_source", changed));
    let mut changed = packet.clone();
    changed.evidence_citation.insert(0, 'x');
    output.push(("evidence_citation", changed));
    let mut changed = packet.clone();
    changed.evidence_anchor.insert(0, 'x');
    output.push(("evidence_anchor", changed));
    let mut changed = packet.clone();
    changed.eps0_pair_receipt_sha256[0] = changed.eps0_pair_receipt_sha256[0].wrapping_add(1);
    output.push(("eps0_pair_receipt", changed));
    output
}

pub(super) fn pair_receipt_digest(
    output: &PrimitiveProfileCheckerOutput,
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
    producer_canary: CanaryEvidence,
    watchdog_canary: CanaryEvidence,
) -> [u8; 32] {
    hash_fields(
        PAIR_DOMAIN,
        [
            RECEIPT_SCHEMA_ID.as_bytes(),
            CLAIM_ID.as_bytes(),
            PRODUCER_ID.as_bytes(),
            WATCHDOG_ID.as_bytes(),
            output.input_sha256.as_slice(),
            producer_result_sha256.as_slice(),
            watchdog_result_sha256.as_slice(),
            producer_canary.transcript_id.as_bytes(),
            producer_canary.case_count.to_be_bytes().as_slice(),
            producer_canary.transcript_sha256.as_slice(),
            watchdog_canary.transcript_id.as_bytes(),
            watchdog_canary.case_count.to_be_bytes().as_slice(),
            watchdog_canary.transcript_sha256.as_slice(),
            output.profile_root_identity.0.as_slice(),
            output.member.0.as_slice(),
            u32::try_from(output.candidates.len())
                .unwrap_or(u32::MAX)
                .to_be_bytes()
                .as_slice(),
            output.evidence_custody_receipt_sha256.as_slice(),
            &[0],
            &[0],
            b"none",
        ],
    )
}

fn write_packet(packet: &PrimitiveProfilePacket) -> Vec<u8> {
    let payloads = [
        packet.schema_id.as_bytes().to_vec(),
        packet.profile_id.as_bytes().to_vec(),
        packet.theory_class_id.as_bytes().to_vec(),
        packet.symmetry_id.as_bytes().to_vec(),
        packet.field_id.as_bytes().to_vec(),
        packet.operator_id.as_bytes().to_vec(),
        packet.state_id.as_bytes().to_vec(),
        packet.sector_id.as_bytes().to_vec(),
        packet.validity_id.as_bytes().to_vec(),
        packet.excluded_term_id.as_bytes().to_vec(),
        packet.member_id.as_bytes().to_vec(),
        packet.residual_slot_id.as_bytes().to_vec(),
        packet.owner_admission_record.as_bytes().to_vec(),
        packet.evidence_citation.as_bytes().to_vec(),
        packet.evidence_source_url.as_bytes().to_vec(),
        packet.evidence_full_sha256_hex.as_bytes().to_vec(),
        packet.evidence_slim_sha256_hex.as_bytes().to_vec(),
        packet.evidence_anchor.as_bytes().to_vec(),
        packet.floor_authority_schema_id.as_bytes().to_vec(),
        packet.floor_authority_sha256.to_vec(),
        packet.eps0_pair_receipt_sha256.to_vec(),
        packet.eps0_capability_sha256.to_vec(),
        vec![
            u8::from(packet.global_physical_vocabulary_coverage),
            u8::from(packet.membership_authority),
        ],
    ];
    let mut bytes = INPUT_DOMAIN.to_vec();
    for (index, payload) in payloads.iter().enumerate() {
        write_field(
            &mut bytes,
            u16::try_from(index + 1).unwrap_or(u16::MAX),
            payload,
        );
    }
    bytes
}

fn write_output(
    input_sha256: [u8; 32],
    candidates: &[ProfileArtifactCandidate],
    member: SpeciesContentIdentity,
    profile_root_identity: ArtifactIdentity,
    evidence_custody_receipt_sha256: [u8; 32],
) -> Vec<u8> {
    let mut bytes = OUTPUT_DOMAIN.to_vec();
    write_field(&mut bytes, 1, &input_sha256);
    write_field(&mut bytes, 2, &profile_root_identity.0);
    write_field(&mut bytes, 3, &member.0);
    write_field(&mut bytes, 4, &evidence_custody_receipt_sha256);
    write_field(
        &mut bytes,
        5,
        &u32::try_from(candidates.len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    for candidate in candidates {
        write_field(&mut bytes, 6, &candidate.identity.0);
        write_field(&mut bytes, 7, &write_admission(&candidate.admission));
    }
    write_field(&mut bytes, 8, &[0]);
    write_field(&mut bytes, 9, &[0]);
    bytes
}

fn write_admission(admission: &RootAdmission) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_field(&mut bytes, 1, admission.tier.id().as_bytes());
    write_field(&mut bytes, 2, admission.provenance.tag().as_bytes());
    match &admission.route {
        super::super::model::AdmissionRoute::Derived(route) => {
            write_field(&mut bytes, 3, b"derived");
            for receipt in [
                &route.ancestry_receipt,
                &route.semantic_checker_receipt,
                &route.independent_watchdog_receipt,
            ] {
                write_field(&mut bytes, 4, receipt.schema_id.as_bytes());
                write_field(&mut bytes, 5, &receipt.digest_sha256);
            }
        }
        super::super::model::AdmissionRoute::Irreducible(route) => {
            write_field(&mut bytes, 3, b"irreducible");
            for receipt in [
                &route.derivation_exhaustion_receipt,
                &route.buckingham_pi_receipt,
                &route.gap_law_receipt,
                &route.chaos_protocol_receipt,
                &route.residual_law_receipt,
            ] {
                write_field(&mut bytes, 4, receipt.schema_id.as_bytes());
                write_field(&mut bytes, 5, &receipt.digest_sha256);
            }
            write_field(&mut bytes, 6, route.residual_slot_id.as_bytes());
            for receipt in [
                &route.residual_slot_receipt,
                &route.owner_admission_receipt,
                &route.independent_watchdog_receipt,
            ] {
                write_field(&mut bytes, 7, receipt.schema_id.as_bytes());
                write_field(&mut bytes, 8, &receipt.digest_sha256);
            }
        }
        super::super::model::AdmissionRoute::EvidenceCustodyOnly { source_receipt } => {
            write_field(&mut bytes, 3, b"evidence-custody-only");
            write_field(&mut bytes, 4, source_receipt.schema_id.as_bytes());
            write_field(&mut bytes, 5, &source_receipt.digest_sha256);
        }
    }
    bytes
}

fn hash_fields<'a>(domain: &[u8], fields: impl IntoIterator<Item = &'a [u8]>) -> [u8; 32] {
    let mut bytes = domain.to_vec();
    for (index, field) in fields.into_iter().enumerate() {
        write_field(
            &mut bytes,
            u16::try_from(index + 1).unwrap_or(u16::MAX),
            field,
        );
    }
    sha256(&bytes)
}

fn write_field(bytes: &mut Vec<u8>, tag: u16, payload: &[u8]) {
    bytes.extend(tag.to_be_bytes());
    bytes.extend(
        u64::try_from(payload.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    bytes.extend(payload);
}

fn to_hex(bytes: [u8; 32]) -> String {
    bytes
        .into_iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}
