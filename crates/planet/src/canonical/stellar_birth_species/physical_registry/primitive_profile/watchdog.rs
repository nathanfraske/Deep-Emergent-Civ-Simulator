//! Reverse reconstruction watchdog for the primitive profile.

use super::*;
use crate::canonical::stellar_birth_species::{
    law_premise::{self, repository_derived_relation_premise_frontier},
    symmetry_operator_exclusion,
};
use civsim_ledger::{Provenance, Tier};
use civsim_units::{digest::sha256, physics_floor::sealed_physical_floor_authority_binding};
use std::collections::BTreeMap;

const INPUT_DOMAIN: &[u8] = b"civsim.planet.primitive-excitation-profile.producer-input.v1";
const OUTPUT_DOMAIN: &[u8] = b"civsim.planet.primitive-excitation-profile.canonical-output.v1";
const PAIR_DOMAIN: &[u8] = b"civsim.planet.primitive-excitation-profile.pair-receipt.v1";
const BYTE_LIMIT: u32 = 1_048_576;
const PROFILE_CLAIM_DOMAIN: &[u8] = b"civsim.planet.primitive-profile-claim.v1";
const PROFILE_COMPONENT_DOMAIN: &[u8] = b"civsim.planet.primitive-profile-field-component.v1";
const PROFILE_SHIFT_DOMAIN: &[u8] = b"civsim.planet.primitive-profile-shift-generator.v1";
const PROFILE_EXCLUDED_OPERATOR_DOMAIN: &[u8] =
    b"civsim.planet.primitive-profile-excluded-operator.v1";
const PROFILE_APPLICABILITY_FACT_DOMAIN: &[u8] =
    b"civsim.planet.primitive-profile-applicability-fact.v1";
const PROFILE_VALIDITY_FACT_DOMAIN: &[u8] = b"civsim.planet.primitive-profile-validity-fact.v1";

struct CandidateReconstruction {
    candidates: Vec<ProfileArtifactCandidate>,
    member: SpeciesContentIdentity,
    profile_root_identity: ArtifactIdentity,
    profile_role_identity: ArtifactIdentity,
    admission_evidence: law_premise::TheoryProfileAdmissionEvidence,
    symmetry_evidence: symmetry_operator_exclusion::TheoryProfileSymmetryEvidence,
}

#[allow(clippy::too_many_arguments)]
fn execute_profile_evidence(
    packet: &PrimitiveProfilePacket,
    input_sha256: [u8; 32],
    evidence_custody_receipt_sha256: [u8; 32],
    profile_root_identity: ArtifactIdentity,
    profile_role_identity: ArtifactIdentity,
    field_identity: ArtifactIdentity,
    operator_identity: ArtifactIdentity,
    sector_identity: ArtifactIdentity,
    validity_identity: ArtifactIdentity,
) -> Result<
    (
        law_premise::TheoryProfileAdmissionEvidence,
        symmetry_operator_exclusion::TheoryProfileSymmetryEvidence,
    ),
    PrimitiveProfileRefusal,
> {
    let claim_identity = hash_fields(
        PROFILE_CLAIM_DOMAIN,
        [
            profile_root_identity.0.as_slice(),
            profile_role_identity.0.as_slice(),
            input_sha256.as_slice(),
        ],
    );
    let mut components_by_index = BTreeMap::new();
    for index in (0_u32..4).rev() {
        let index_bytes = index.to_be_bytes();
        components_by_index.insert(
            index,
            hash_fields(
                PROFILE_COMPONENT_DOMAIN,
                [field_identity.0.as_slice(), index_bytes.as_slice()],
            ),
        );
    }
    let components = components_by_index.values().copied().collect::<Vec<_>>();
    let shifts = components_by_index
        .iter()
        .map(|(index, component)| {
            let index_bytes = index.to_be_bytes();
            symmetry_operator_exclusion::TheoryProfileShift {
                field_component: *component,
                shift_generator: hash_fields(
                    PROFILE_SHIFT_DOMAIN,
                    [sector_identity.0.as_slice(), index_bytes.as_slice()],
                ),
                coefficient: 1,
            }
        })
        .collect::<Vec<_>>();
    let excluded_operator_terms = components_by_index
        .iter()
        .map(
            |(index, component)| symmetry_operator_exclusion::TheoryProfileQuadraticTerm {
                left_field: *component,
                right_field: *component,
                coefficient: if *index == 0 { 1 } else { -1 },
            },
        )
        .collect::<Vec<_>>();
    let excluded_operator_identity = hash_fields(
        PROFILE_EXCLUDED_OPERATOR_DOMAIN,
        [
            operator_identity.0.as_slice(),
            packet.excluded_term_id.as_bytes(),
            profile_root_identity.0.as_slice(),
        ],
    );
    let required_applicability_fact = hash_fields(
        PROFILE_APPLICABILITY_FACT_DOMAIN,
        [
            profile_root_identity.0.as_slice(),
            sector_identity.0.as_slice(),
            validity_identity.0.as_slice(),
        ],
    );
    let required_validity_fact = hash_fields(
        PROFILE_VALIDITY_FACT_DOMAIN,
        [
            required_applicability_fact.as_slice(),
            validity_identity.0.as_slice(),
            input_sha256.as_slice(),
        ],
    );
    let symmetry = symmetry_operator_exclusion::inspect_theory_profile_symmetry(
        &symmetry_operator_exclusion::TheoryProfileSymmetryRequest {
            claim_identity,
            subject_identity: field_identity.0,
            applicability_domain_identity: validity_identity.0,
            symmetry_identity: sector_identity.0,
            field_components: components,
            shifts,
            excluded_operator_identity,
            excluded_operator_terms,
            admitted_scope_fact: profile_root_identity.0,
            required_applicability_fact,
            required_validity_fact,
        },
    )
    .map_err(|_| PrimitiveProfileRefusal::ArtifactConstructionFailure)?;
    let admission = law_premise::inspect_theory_profile_admission(
        &law_premise::TheoryProfileAdmissionRequest {
            claim_identity,
            role_identity: profile_role_identity.0,
            content_identity: profile_root_identity.0,
            profile_input_sha256: input_sha256,
            source_custody_sha256: evidence_custody_receipt_sha256,
            applicability_receipt_sha256: symmetry.applicability_receipt_sha256,
            validity_receipt_sha256: symmetry.validity_receipt_sha256,
            residual_slot_id: packet.residual_slot_id.clone(),
            occupied_profile_slots: Vec::new(),
            owner_admission_record: packet.owner_admission_record.clone(),
        },
    )
    .map_err(|_| PrimitiveProfileRefusal::ArtifactConstructionFailure)?;
    let expected = (10, 10, 0, 1, 0);
    let observed = (
        symmetry.basis_element_count,
        symmetry.basis_excluded_count,
        symmetry.basis_invariant_count,
        symmetry.excluded_operator_count,
        symmetry.invariant_operator_count,
    );
    if admission.decision_id != "irreducible_protocol_structurally_bound"
        || (
            admission.target_claim_identity,
            admission.target_role_identity,
            admission.target_content_identity,
        ) != (
            claim_identity,
            profile_role_identity.0,
            profile_root_identity.0,
        )
        || observed != expected
    {
        return Err(PrimitiveProfileRefusal::ArtifactConstructionFailure);
    }
    Ok((admission, symmetry))
}

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
        transition_id: TRANSITION_ID.to_owned(),
        stability_id: STABILITY_ID.to_owned(),
        current_id: CURRENT_ID.to_owned(),
        charge_id: CHARGE_ID.to_owned(),
        statistics_id: STATISTICS_ID.to_owned(),
        helicity_id: HELICITY_ID.to_owned(),
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
    let reconstruction =
        reconstruct_candidates(packet, input_sha256, evidence_custody_receipt_sha256)?;
    if reconstruction.candidates.len() != ARTIFACT_COUNT {
        return Err(PrimitiveProfileRefusal::ArtifactCountMismatch);
    }
    let canonical_bytes = write_output(
        input_sha256,
        &reconstruction,
        evidence_custody_receipt_sha256,
    );
    if u32::try_from(canonical_bytes.len()).map_or(true, |length| length > BYTE_LIMIT) {
        return Err(PrimitiveProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(PrimitiveProfileCheckerOutput {
        input_sha256,
        canonical_bytes,
        candidates: reconstruction.candidates,
        member: reconstruction.member,
        profile_root_identity: reconstruction.profile_root_identity,
        profile_role_identity: reconstruction.profile_role_identity,
        evidence_custody_receipt_sha256,
        admission_evidence: reconstruction.admission_evidence,
        symmetry_evidence: reconstruction.symmetry_evidence,
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
        (packet.transition_id.as_str(), TRANSITION_ID),
        (packet.stability_id.as_str(), STABILITY_ID),
        (packet.current_id.as_str(), CURRENT_ID),
        (packet.charge_id.as_str(), CHARGE_ID),
        (packet.statistics_id.as_str(), STATISTICS_ID),
        (packet.helicity_id.as_str(), HELICITY_ID),
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
) -> Result<CandidateReconstruction, PrimitiveProfileRefusal> {
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

    let descriptor_specs = [
        (
            "profile_role",
            "civsim.physical-profile.role.profile-root.v1",
            "admitted-theory-profile-role",
        ),
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
            "helicity_role",
            "civsim.physical-profile.role.helicity-requirement.v1",
            "helicity-requirement-role",
        ),
        (
            "helicity",
            "civsim.physical-profile.helicity.v1",
            packet.helicity_id.as_str(),
        ),
        (
            "statistics_role",
            "civsim.physical-profile.role.statistics-requirement.v1",
            "statistics-requirement-role",
        ),
        (
            "statistics",
            "civsim.physical-profile.statistics.v1",
            packet.statistics_id.as_str(),
        ),
        (
            "charge_role",
            "civsim.physical-profile.role.charge-requirement.v1",
            "charge-requirement-role",
        ),
        (
            "charge",
            "civsim.physical-profile.charge.v1",
            packet.charge_id.as_str(),
        ),
        (
            "current_role",
            "civsim.physical-profile.role.current-requirement.v1",
            "current-requirement-role",
        ),
        (
            "current",
            "civsim.physical-profile.current.v1",
            packet.current_id.as_str(),
        ),
        (
            "stability_role",
            "civsim.physical-profile.role.stability-requirement.v1",
            "stability-requirement-role",
        ),
        (
            "stability",
            "civsim.physical-profile.stability.v1",
            packet.stability_id.as_str(),
        ),
        (
            "transition_role",
            "civsim.physical-profile.role.transition-requirement.v1",
            "transition-requirement-role",
        ),
        (
            "transition",
            "civsim.physical-profile.transition.v1",
            packet.transition_id.as_str(),
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

    let profile_role_identity = lookup_identity(&by_key, "profile_role")?;
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
    let helicity_role = lookup_identity(&by_key, "helicity_role")?;
    let helicity = lookup_identity(&by_key, "helicity")?;
    let statistics_role = lookup_identity(&by_key, "statistics_role")?;
    let statistics = lookup_identity(&by_key, "statistics")?;
    let charge_role = lookup_identity(&by_key, "charge_role")?;
    let charge = lookup_identity(&by_key, "charge")?;
    let current_role = lookup_identity(&by_key, "current_role")?;
    let current = lookup_identity(&by_key, "current")?;
    let stability_role = lookup_identity(&by_key, "stability_role")?;
    let stability = lookup_identity(&by_key, "stability")?;
    let transition_role = lookup_identity(&by_key, "transition_role")?;
    let transition = lookup_identity(&by_key, "transition")?;
    let constraint_role = lookup_identity(&by_key, "constraint_role")?;
    let (admission_evidence, symmetry_evidence) = execute_profile_evidence(
        packet,
        input_sha256,
        evidence_custody_receipt_sha256,
        profile_root_identity,
        profile_role_identity,
        field,
        operator,
        sector,
        validity,
    )?;
    by_key.insert(
        "profile",
        ProfileArtifactCandidate {
            identity: profile_root_identity,
            admission: reconstruct_irreducible_admission(packet, &admission_evidence),
            payload: profile_payload,
        },
    );

    let requirements = super::super::model::RequirementSet {
        artifact_relations: vec![
            edge(field_role, field),
            edge(state_role, state),
            edge(sector_role, sector),
            edge(validity_role, validity),
            edge(helicity_role, helicity),
            edge(statistics_role, statistics),
            edge(charge_role, charge),
            edge(current_role, current),
            edge(stability_role, stability),
            edge(transition_role, transition),
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
            symmetry_evidence.applicability_receipt_sha256,
        ),
        exclusion_producer_receipt: receipt_binding(
            "civsim.primitive-profile.exact-zero-forward-proof.v1",
            symmetry_evidence.exclusion_producer_result_sha256,
        ),
        exclusion_watchdog_receipt: receipt_binding(
            "civsim.primitive-profile.exact-zero-reverse-proof.v1",
            symmetry_evidence.exclusion_watchdog_result_sha256,
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
        "profile_role",
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
        "helicity_role",
        "helicity",
        "statistics_role",
        "statistics",
        "charge_role",
        "charge",
        "current_role",
        "current",
        "stability_role",
        "stability",
        "transition_role",
        "transition",
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
    Ok(CandidateReconstruction {
        candidates,
        member,
        profile_root_identity,
        profile_role_identity,
        admission_evidence,
        symmetry_evidence,
    })
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
    evidence: &law_premise::TheoryProfileAdmissionEvidence,
) -> RootAdmission {
    RootAdmission {
        tier: Tier::Residue,
        provenance: Provenance::Authored,
        route: super::super::model::AdmissionRoute::Irreducible(Box::new(
            super::super::model::IrreducibleAdmission {
                independent_watchdog_receipt: receipt_binding(
                    "civsim.primitive-profile.independent-watchdog-route.v1",
                    evidence.independent_watchdog_receipt_sha256,
                ),
                owner_admission_receipt: receipt_binding(
                    "civsim.primitive-profile.owner-admission.v1",
                    evidence.owner_admission_receipt_sha256,
                ),
                residual_slot_receipt: receipt_binding(
                    "civsim.primitive-profile.residual-slot.v1",
                    evidence.residual_slot_receipt_sha256,
                ),
                residual_slot_id: packet.residual_slot_id.clone(),
                residual_law_receipt: receipt_binding(
                    "civsim.primitive-profile.residual-law.v1",
                    evidence.residual_law_receipt_sha256,
                ),
                chaos_protocol_receipt: receipt_binding(
                    "civsim.primitive-profile.chaos-protocol.v1",
                    evidence.chaos_protocol_receipt_sha256,
                ),
                gap_law_receipt: receipt_binding(
                    "civsim.primitive-profile.gap-law.v1",
                    evidence.gap_law_receipt_sha256,
                ),
                buckingham_pi_receipt: receipt_binding(
                    "civsim.primitive-profile.buckingham-pi.v1",
                    evidence.buckingham_pi_receipt_sha256,
                ),
                derivation_exhaustion_receipt: receipt_binding(
                    "civsim.primitive-profile.derive-first-exhaustion.v1",
                    evidence.derivation_exhaustion_receipt_sha256,
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
    changed.helicity_id.insert(0, 'x');
    output.push(("helicity", changed));
    let mut changed = packet.clone();
    changed.statistics_id.insert(0, 'x');
    output.push(("statistics", changed));
    let mut changed = packet.clone();
    changed.charge_id.insert(0, 'x');
    output.push(("charge", changed));
    let mut changed = packet.clone();
    changed.current_id.insert(0, 'x');
    output.push(("current", changed));
    let mut changed = packet.clone();
    changed.stability_id.insert(0, 'x');
    output.push(("stability", changed));
    let mut changed = packet.clone();
    changed.transition_id.insert(0, 'x');
    output.push(("transition", changed));
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
            output.profile_role_identity.0.as_slice(),
            output.member.0.as_slice(),
            u32::try_from(output.candidates.len())
                .unwrap_or(u32::MAX)
                .to_be_bytes()
                .as_slice(),
            output.evidence_custody_receipt_sha256.as_slice(),
            output
                .admission_evidence
                .derivation_catalog_sha256
                .as_slice(),
            output
                .admission_evidence
                .repository_catalog_sha256
                .as_slice(),
            output
                .admission_evidence
                .protocol_producer_result_sha256
                .as_slice(),
            output
                .admission_evidence
                .protocol_watchdog_result_sha256
                .as_slice(),
            output
                .admission_evidence
                .derivation_coverage_capability_sha256
                .as_slice(),
            output
                .admission_evidence
                .irreducible_protocol_capability_sha256
                .as_slice(),
            output
                .symmetry_evidence
                .basis_element_count
                .to_be_bytes()
                .as_slice(),
            output
                .symmetry_evidence
                .exclusion_producer_result_sha256
                .as_slice(),
            output
                .symmetry_evidence
                .exclusion_watchdog_result_sha256
                .as_slice(),
            output.symmetry_evidence.action_binding_sha256.as_slice(),
            output
                .symmetry_evidence
                .applicability_receipt_sha256
                .as_slice(),
            output.symmetry_evidence.validity_receipt_sha256.as_slice(),
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
        packet.helicity_id.as_bytes().to_vec(),
        packet.statistics_id.as_bytes().to_vec(),
        packet.charge_id.as_bytes().to_vec(),
        packet.current_id.as_bytes().to_vec(),
        packet.stability_id.as_bytes().to_vec(),
        packet.transition_id.as_bytes().to_vec(),
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
    reconstruction: &CandidateReconstruction,
    evidence_custody_receipt_sha256: [u8; 32],
) -> Vec<u8> {
    let mut bytes = OUTPUT_DOMAIN.to_vec();
    write_field(&mut bytes, 1, &input_sha256);
    write_field(&mut bytes, 2, &reconstruction.profile_root_identity.0);
    write_field(&mut bytes, 3, &reconstruction.member.0);
    write_field(&mut bytes, 4, &evidence_custody_receipt_sha256);
    write_field(
        &mut bytes,
        5,
        &u32::try_from(reconstruction.candidates.len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    for candidate in &reconstruction.candidates {
        write_field(&mut bytes, 6, &candidate.identity.0);
        write_field(&mut bytes, 7, &write_admission(&candidate.admission));
    }
    write_field(&mut bytes, 8, &[0]);
    write_field(&mut bytes, 9, &[0]);
    write_field(&mut bytes, 10, &reconstruction.profile_role_identity.0);
    write_field(
        &mut bytes,
        11,
        &reconstruction.admission_evidence.derivation_catalog_sha256,
    );
    write_field(
        &mut bytes,
        12,
        &reconstruction
            .admission_evidence
            .derivation_coverage_capability_sha256,
    );
    write_field(
        &mut bytes,
        13,
        &reconstruction
            .admission_evidence
            .irreducible_protocol_capability_sha256,
    );
    write_field(
        &mut bytes,
        14,
        reconstruction.admission_evidence.decision_id.as_bytes(),
    );
    write_field(
        &mut bytes,
        15,
        &reconstruction
            .symmetry_evidence
            .basis_element_count
            .to_be_bytes(),
    );
    write_field(
        &mut bytes,
        16,
        &reconstruction
            .symmetry_evidence
            .basis_excluded_count
            .to_be_bytes(),
    );
    write_field(
        &mut bytes,
        17,
        &reconstruction
            .symmetry_evidence
            .excluded_operator_count
            .to_be_bytes(),
    );
    write_field(
        &mut bytes,
        18,
        &reconstruction
            .symmetry_evidence
            .exclusion_producer_result_sha256,
    );
    write_field(
        &mut bytes,
        19,
        &reconstruction
            .symmetry_evidence
            .exclusion_watchdog_result_sha256,
    );
    write_field(
        &mut bytes,
        20,
        &reconstruction.symmetry_evidence.action_binding_sha256,
    );
    write_field(
        &mut bytes,
        21,
        &reconstruction
            .symmetry_evidence
            .applicability_receipt_sha256,
    );
    write_field(
        &mut bytes,
        22,
        &reconstruction.symmetry_evidence.validity_receipt_sha256,
    );
    write_field(
        &mut bytes,
        23,
        &reconstruction.admission_evidence.repository_catalog_sha256,
    );
    write_field(
        &mut bytes,
        24,
        &reconstruction
            .admission_evidence
            .protocol_producer_result_sha256,
    );
    write_field(
        &mut bytes,
        25,
        &reconstruction
            .admission_evidence
            .protocol_watchdog_result_sha256,
    );
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
