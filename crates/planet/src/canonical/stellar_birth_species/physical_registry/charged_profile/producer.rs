//! Forward constructor and semantic inspector for the charged matter profile.

use super::*;
use crate::canonical::stellar_birth_species::law_premise;
use civsim_ledger::{Provenance, Tier};
use civsim_units::{digest::sha256, physics_floor::sealed_physical_floor_authority_binding};

const INPUT_DOMAIN: &[u8] = b"civsim.planet.charged-matter-profile.producer-input.v1";
const OUTPUT_DOMAIN: &[u8] = b"civsim.planet.charged-matter-profile.canonical-output.v1";
const PAIR_DOMAIN: &[u8] = b"civsim.planet.charged-matter-profile.pair-receipt.v1";
const PROFILE_CLAIM_DOMAIN: &[u8] = b"civsim.planet.charged-matter-profile.claim.v1";
const CHARGE_PRODUCER_DOMAIN: &[u8] =
    b"civsim.planet.charged-matter-profile.charge-conjugation-forward.v1";
const CHARGE_WATCHDOG_DOMAIN: &[u8] =
    b"civsim.planet.charged-matter-profile.charge-conjugation-reverse.v1";
const MASS_PRODUCER_DOMAIN: &[u8] =
    b"civsim.planet.charged-matter-profile.mass-transport-forward.v1";
const MASS_WATCHDOG_DOMAIN: &[u8] =
    b"civsim.planet.charged-matter-profile.mass-transport-reverse.v1";
const APPLICABILITY_DOMAIN: &[u8] = b"civsim.planet.charged-matter-profile.applicability.v1";
const VALIDITY_DOMAIN: &[u8] = b"civsim.planet.charged-matter-profile.validity.v1";
const MAX_CANONICAL_BYTES: usize = 1_048_576;

struct CandidateBuild {
    candidates: Vec<ChargedArtifactCandidate>,
    members: Vec<SpeciesContentIdentity>,
    profile_root_identity: ArtifactIdentity,
    profile_role_identity: ArtifactIdentity,
    charge_conjugation_producer_sha256: [u8; 32],
    charge_conjugation_watchdog_sha256: [u8; 32],
    mass_transport_producer_sha256: [u8; 32],
    mass_transport_watchdog_sha256: [u8; 32],
    applicability_receipt_sha256: [u8; 32],
    validity_receipt_sha256: [u8; 32],
    admission_evidence: law_premise::TheoryProfileAdmissionEvidence,
}

pub(super) fn sealed_packet() -> Result<ChargedProfilePacket, ChargedProfileRefusal> {
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| ChargedProfileRefusal::SealedSourceUnavailable)?;
    let roots = super::super::repository_roots::project_repository_roots()
        .map_err(|_| ChargedProfileRefusal::SealedSourceUnavailable)?;
    let bindings = super::super::repository_roots::coordinate_bindings(&roots)
        .map_err(|_| ChargedProfileRefusal::SealedSourceUnavailable)?;
    let mass = bindings
        .iter()
        .find(|binding| binding.entry_id == MASS_SOURCE_ENTRY_ID)
        .ok_or(ChargedProfileRefusal::SealedSourceUnavailable)?;
    let coupling = bindings
        .iter()
        .find(|binding| binding.entry_id == COUPLING_SOURCE_ENTRY_ID)
        .ok_or(ChargedProfileRefusal::SealedSourceUnavailable)?;
    let neutral_mass_projection_identity = mass
        .mass_projection_identity
        .ok_or(ChargedProfileRefusal::SealedSourceUnavailable)?;
    let neutral_mass_projection_ancestry_sha256 = mass
        .mass_projection_ancestry_sha256
        .ok_or(ChargedProfileRefusal::SealedSourceUnavailable)?;
    let primitive = super::super::primitive_profile::project_admitted_profile()
        .map_err(|_| ChargedProfileRefusal::SealedSourceUnavailable)?;
    Ok(ChargedProfilePacket {
        schema_id: PACKET_SCHEMA_ID.to_owned(),
        profile_id: PROFILE_ID.to_owned(),
        theory_class_id: THEORY_CLASS_ID.to_owned(),
        field_id: FIELD_ID.to_owned(),
        operator_id: OPERATOR_ID.to_owned(),
        state_pair_class_id: STATE_PAIR_CLASS_ID.to_owned(),
        validity_id: VALIDITY_ID.to_owned(),
        spin_id: SPIN_ID.to_owned(),
        statistics_id: STATISTICS_ID.to_owned(),
        mobility_id: MOBILITY_ID.to_owned(),
        stability_id: STABILITY_ID.to_owned(),
        transition_id: TRANSITION_ID.to_owned(),
        conjugation_id: CONJUGATION_ID.to_owned(),
        member_class_id: MEMBER_CLASS_ID.to_owned(),
        charge_weights: vec![-1, 1],
        residual_slot_id: RESIDUAL_SLOT_ID.to_owned(),
        owner_admission_record: OWNER_ADMISSION_RECORD.to_owned(),
        evidence_citation: EVIDENCE_CITATION.to_owned(),
        evidence_source_url: EVIDENCE_SOURCE_URL.to_owned(),
        evidence_full_sha256_hex: EVIDENCE_FULL_SHA256_HEX.to_owned(),
        evidence_slim_sha256_hex: EVIDENCE_SLIM_SHA256_HEX.to_owned(),
        evidence_anchor: EVIDENCE_ANCHOR.to_owned(),
        floor_authority: receipt_binding(floor.schema_id().as_str(), floor.digest()),
        root_pair_receipt: mass.root_pair_receipt.clone(),
        mass_source_entry_id: mass.entry_id.clone(),
        mass_scalar_identity: mass.scalar_identity,
        mass_scalar_ancestry_sha256: mass.scalar_ancestry_sha256,
        neutral_mass_projection_identity,
        neutral_mass_projection_ancestry_sha256,
        coupling_source_entry_id: coupling.entry_id.clone(),
        coupling_scalar_identity: coupling.scalar_identity,
        coupling_scalar_ancestry_sha256: coupling.scalar_ancestry_sha256,
        primitive_profile_receipt: receipt_binding(
            primitive.receipt.schema_id,
            primitive.receipt.pair_receipt_sha256,
        ),
        primitive_profile_root_identity: primitive.receipt.profile_root_identity,
        primitive_sector_identity: primitive.sector_identity,
        primitive_member: primitive.member,
        primitive_residual_slot_id: primitive.receipt.residual_slot_id.to_owned(),
        global_physical_vocabulary_coverage: false,
        membership_authority: false,
    })
}

pub(super) fn inspect(
    packet: &ChargedProfilePacket,
) -> Result<ChargedProfileCheckerOutput, ChargedProfileRefusal> {
    let sealed = sealed_packet()?;
    validate_packet_against(packet, &sealed)?;
    let input_bytes = encode_packet(packet);
    if input_bytes.len() > MAX_CANONICAL_BYTES {
        return Err(ChargedProfileRefusal::ArtifactConstructionFailure);
    }
    let input_sha256 = sha256(&input_bytes);
    let evidence_custody_receipt_sha256 = digest(
        b"civsim.planet.charged-matter-profile.evidence-custody.v1",
        &[
            packet.evidence_citation.as_bytes(),
            packet.evidence_source_url.as_bytes(),
            packet.evidence_full_sha256_hex.as_bytes(),
            packet.evidence_slim_sha256_hex.as_bytes(),
            packet.evidence_anchor.as_bytes(),
        ],
    );
    let build = build_candidates(packet, input_sha256, evidence_custody_receipt_sha256)?;
    if build.candidates.len() != ARTIFACT_COUNT || build.members.len() != MEMBER_COUNT {
        return Err(ChargedProfileRefusal::ArtifactCountMismatch);
    }
    let canonical_bytes = encode_output(input_sha256, &build, evidence_custody_receipt_sha256);
    if canonical_bytes.len() > MAX_CANONICAL_BYTES {
        return Err(ChargedProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(ChargedProfileCheckerOutput {
        input_sha256,
        canonical_bytes,
        candidates: build.candidates,
        members: build.members,
        profile_root_identity: build.profile_root_identity,
        profile_role_identity: build.profile_role_identity,
        evidence_custody_receipt_sha256,
        charge_conjugation_producer_sha256: build.charge_conjugation_producer_sha256,
        charge_conjugation_watchdog_sha256: build.charge_conjugation_watchdog_sha256,
        mass_transport_producer_sha256: build.mass_transport_producer_sha256,
        mass_transport_watchdog_sha256: build.mass_transport_watchdog_sha256,
        applicability_receipt_sha256: build.applicability_receipt_sha256,
        validity_receipt_sha256: build.validity_receipt_sha256,
        admission_evidence: build.admission_evidence,
    })
}

fn validate_packet_against(
    packet: &ChargedProfilePacket,
    sealed: &ChargedProfilePacket,
) -> Result<(), ChargedProfileRefusal> {
    if packet.schema_id != PACKET_SCHEMA_ID {
        return Err(ChargedProfileRefusal::PacketSchemaMismatch);
    }
    if packet.profile_id != PROFILE_ID
        || packet.residual_slot_id != RESIDUAL_SLOT_ID
        || packet.owner_admission_record != OWNER_ADMISSION_RECORD
    {
        return Err(ChargedProfileRefusal::ProfileIdentityMismatch);
    }
    let theory = [
        (packet.theory_class_id.as_str(), THEORY_CLASS_ID),
        (packet.field_id.as_str(), FIELD_ID),
        (packet.operator_id.as_str(), OPERATOR_ID),
        (packet.state_pair_class_id.as_str(), STATE_PAIR_CLASS_ID),
        (packet.validity_id.as_str(), VALIDITY_ID),
        (packet.spin_id.as_str(), SPIN_ID),
        (packet.statistics_id.as_str(), STATISTICS_ID),
        (packet.mobility_id.as_str(), MOBILITY_ID),
        (packet.stability_id.as_str(), STABILITY_ID),
        (packet.transition_id.as_str(), TRANSITION_ID),
        (packet.conjugation_id.as_str(), CONJUGATION_ID),
        (packet.member_class_id.as_str(), MEMBER_CLASS_ID),
    ];
    if theory
        .into_iter()
        .any(|(found, expected)| found != expected)
    {
        return Err(ChargedProfileRefusal::TheoryProfileMismatch);
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
        return Err(ChargedProfileRefusal::EvidenceCustodyMismatch);
    }
    if packet.floor_authority != sealed.floor_authority
        || packet.floor_authority.digest_sha256 == [0; 32]
    {
        return Err(ChargedProfileRefusal::FloorBindingMismatch);
    }
    let root_fields_match = packet.root_pair_receipt == sealed.root_pair_receipt
        && packet.root_pair_receipt.digest_sha256 != [0; 32]
        && packet.mass_source_entry_id == MASS_SOURCE_ENTRY_ID
        && packet.mass_source_entry_id == sealed.mass_source_entry_id
        && packet.mass_scalar_identity == sealed.mass_scalar_identity
        && packet.mass_scalar_ancestry_sha256 == sealed.mass_scalar_ancestry_sha256
        && packet.neutral_mass_projection_identity == sealed.neutral_mass_projection_identity
        && packet.neutral_mass_projection_ancestry_sha256
            == sealed.neutral_mass_projection_ancestry_sha256
        && packet.coupling_source_entry_id == COUPLING_SOURCE_ENTRY_ID
        && packet.coupling_source_entry_id == sealed.coupling_source_entry_id
        && packet.coupling_scalar_identity == sealed.coupling_scalar_identity
        && packet.coupling_scalar_ancestry_sha256 == sealed.coupling_scalar_ancestry_sha256;
    if !root_fields_match {
        return Err(ChargedProfileRefusal::RootBindingMismatch);
    }
    let primitive_fields_match = packet.primitive_profile_receipt
        == sealed.primitive_profile_receipt
        && packet.primitive_profile_receipt.digest_sha256 != [0; 32]
        && packet.primitive_profile_root_identity == sealed.primitive_profile_root_identity
        && packet.primitive_sector_identity == sealed.primitive_sector_identity
        && packet.primitive_member == sealed.primitive_member
        && packet.primitive_residual_slot_id == sealed.primitive_residual_slot_id;
    if !primitive_fields_match {
        return Err(ChargedProfileRefusal::PrimitiveProfileBindingMismatch);
    }
    let mut weights = packet.charge_weights.clone();
    weights.sort_unstable();
    if weights != [-1, 1] {
        return Err(ChargedProfileRefusal::ChargeConjugationMismatch);
    }
    if packet.primitive_residual_slot_id == packet.residual_slot_id {
        return Err(ChargedProfileRefusal::ResidualSlotCollision);
    }
    if packet.global_physical_vocabulary_coverage || packet.membership_authority {
        return Err(ChargedProfileRefusal::UnsupportedScope);
    }
    Ok(())
}

fn build_candidates(
    packet: &ChargedProfilePacket,
    input_sha256: [u8; 32],
    evidence_custody_receipt_sha256: [u8; 32],
) -> Result<CandidateBuild, ChargedProfileRefusal> {
    let mut candidates = Vec::with_capacity(ARTIFACT_COUNT);
    let profile_payload = descriptor(
        PROFILE_SCHEMA_ID,
        &format!(
            "profile_id={}\ntheory_class={}\nevidence_custody_sha256={}\nprimitive_profile_root={}\nmass_coordinate={}\ncoupling_coordinate={}\nauthority_scope=claim-local\n",
            packet.profile_id,
            packet.theory_class_id,
            hex(evidence_custody_receipt_sha256),
            hex(packet.primitive_profile_root_identity.0),
            hex(packet.mass_scalar_identity.0),
            hex(packet.coupling_scalar_identity.0),
        ),
    );
    let profile_root_identity = identity(&profile_payload)?;
    let profile_role_identity = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.profile-root.v1",
        "admitted-charged-matter-profile-role",
    )?;
    let derivation_kind = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.derivation-kind.v1",
        "charge-conjugate-matter-from-admitted-local-profile",
    )?;
    let field_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.field-input.v1",
        "matter-field-input-role",
    )?;
    let field = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.field.v1",
        packet.field_id.as_str(),
    )?;
    let operator_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.operator-input.v1",
        "matter-operator-input-role",
    )?;
    let operator = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.operator.v1",
        packet.operator_id.as_str(),
    )?;
    let state_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.state-requirement.v1",
        "conjugate-state-requirement-role",
    )?;
    let state_negative = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.state.v1",
        "conjugate-orbit-state:relative-charge-weight=-1",
    )?;
    let state_positive = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.state.v1",
        "conjugate-orbit-state:relative-charge-weight=1",
    )?;
    let sector_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.sector-requirement.v1",
        "admitted-abelian-sector-requirement-role",
    )?;
    let validity_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.validity-requirement.v1",
        "local-validity-requirement-role",
    )?;
    let validity = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.validity.v1",
        packet.validity_id.as_str(),
    )?;
    let spin_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.spin-requirement.v1",
        "twice-spin-requirement-role",
    )?;
    let spin = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.spin.v1",
        packet.spin_id.as_str(),
    )?;
    let statistics_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.statistics-requirement.v1",
        "statistics-requirement-role",
    )?;
    let statistics = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.statistics.v1",
        packet.statistics_id.as_str(),
    )?;
    let charge_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.charge-requirement.v1",
        "relative-abelian-charge-weight-role",
    )?;
    let current_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.current-requirement.v1",
        "conserved-current-requirement-role",
    )?;
    let current_negative = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.current.v1",
        "conserved-current:relative-charge-weight=-1",
    )?;
    let current_positive = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.current.v1",
        "conserved-current:relative-charge-weight=1",
    )?;
    let mobility_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.mobility-requirement.v1",
        "mobility-requirement-role",
    )?;
    let mobility = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.mobility.v1",
        packet.mobility_id.as_str(),
    )?;
    let stability_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.stability-requirement.v1",
        "local-stability-requirement-role",
    )?;
    let stability = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.stability.v1",
        packet.stability_id.as_str(),
    )?;
    let transition_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.transition-requirement.v1",
        "local-transition-requirement-role",
    )?;
    let transition = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.transition.v1",
        packet.transition_id.as_str(),
    )?;
    let conjugation_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.conjugation-requirement.v1",
        "charge-conjugation-closure-requirement-role",
    )?;

    let charge_conjugation_producer_sha256 = charge_conjugation_receipt(
        CHARGE_PRODUCER_DOMAIN,
        packet,
        profile_root_identity,
        state_negative,
        state_positive,
    );
    let charge_conjugation_watchdog_sha256 = charge_conjugation_receipt(
        CHARGE_WATCHDOG_DOMAIN,
        packet,
        profile_root_identity,
        state_negative,
        state_positive,
    );
    let mass_transport_producer_sha256 =
        mass_transport_receipt(MASS_PRODUCER_DOMAIN, packet, profile_root_identity);
    let mass_transport_watchdog_sha256 =
        mass_transport_receipt(MASS_WATCHDOG_DOMAIN, packet, profile_root_identity);
    let charge_receipts = [
        charge_conjugation_producer_sha256,
        charge_conjugation_watchdog_sha256,
    ];
    if charge_receipts.contains(&[0; 32]) || charge_receipts[0] == charge_receipts[1] {
        return Err(ChargedProfileRefusal::ChargeConjugationMismatch);
    }
    let mass_receipts = [
        mass_transport_producer_sha256,
        mass_transport_watchdog_sha256,
    ];
    if mass_receipts.contains(&[0; 32]) || mass_receipts[0] == mass_receipts[1] {
        return Err(ChargedProfileRefusal::MassTransportMismatch);
    }
    let structural_receipts = [
        charge_receipts[0],
        charge_receipts[1],
        mass_receipts[0],
        mass_receipts[1],
    ];
    if structural_receipts
        .into_iter()
        .collect::<BTreeSet<_>>()
        .len()
        != structural_receipts.len()
    {
        return Err(ChargedProfileRefusal::ChargeConjugationMismatch);
    }
    let conjugation = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.charge-conjugation-closure.v1",
        &format!(
            "{}\nforward_sha256={}\nreverse_sha256={}",
            packet.conjugation_id,
            hex(charge_conjugation_producer_sha256),
            hex(charge_conjugation_watchdog_sha256),
        ),
    )?;
    let mass_source_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.mass-source.v1",
        "transported-species-rest-mass-source-role",
    )?;
    let coupling_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.coupling-source.v1",
        "dimensionless-coupling-coordinate-role",
    )?;
    let constraint_role = push_descriptor(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        "civsim.charged-profile.role.constraint.v1",
        "member-local-charge-conjugation-constraint-role",
    )?;

    let charge_negative = push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        charge_coordinate(-1),
    )?;
    let charge_positive = push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        charge_coordinate(1),
    )?;
    let mass_projection = push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        ArtifactPayload::MassProjection(super::super::model::MassProjectionArtifact {
            expression: super::super::model::ExactExpression {
                nodes: vec![super::super::model::ExactExpressionNode::Coordinate(
                    packet.mass_scalar_identity,
                )],
                output_node: 0,
            },
            scope: super::super::model::MassProjectionScope::SpeciesRestMass,
            uncertainty_transport: Some(super::super::model::MassUncertaintyTransportProof {
                sources: vec![super::super::model::MassUncertaintySourceProof {
                    source_coordinate: packet.mass_scalar_identity,
                    source_pair_receipt: packet.root_pair_receipt.clone(),
                }],
                producer_receipt: receipt_binding(
                    "civsim.charged-profile.mass-uncertainty-forward.v1",
                    mass_transport_producer_sha256,
                ),
                watchdog_receipt: receipt_binding(
                    "civsim.charged-profile.mass-uncertainty-reverse.v1",
                    mass_transport_watchdog_sha256,
                ),
            }),
        }),
    )?;

    let applicability_receipt_sha256 = digest(
        APPLICABILITY_DOMAIN,
        &[
            &charge_conjugation_producer_sha256,
            &charge_conjugation_watchdog_sha256,
            &mass_transport_producer_sha256,
            &mass_transport_watchdog_sha256,
            &packet.primitive_profile_receipt.digest_sha256,
            &packet.root_pair_receipt.digest_sha256,
        ],
    );
    let validity_receipt_sha256 = digest(
        VALIDITY_DOMAIN,
        &[
            &applicability_receipt_sha256,
            packet.validity_id.as_bytes(),
            &input_sha256,
        ],
    );
    let claim_identity = digest(
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
            occupied_profile_slots: vec![packet.primitive_residual_slot_id.clone()],
            owner_admission_record: packet.owner_admission_record.clone(),
        })
        .map_err(|_| ChargedProfileRefusal::ArtifactConstructionFailure)?;
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
        return Err(ChargedProfileRefusal::ArtifactConstructionFailure);
    }
    candidates.insert(
        0,
        ChargedArtifactCandidate {
            identity: profile_root_identity,
            admission: irreducible_admission(packet, &admission_evidence),
            payload: profile_payload,
        },
    );

    let shared_relations = vec![
        relation(field_role, field),
        relation(sector_role, packet.primitive_sector_identity),
        relation(validity_role, validity),
        relation(spin_role, spin),
        relation(statistics_role, statistics),
        relation(mobility_role, mobility),
        relation(stability_role, stability),
        relation(transition_role, transition),
        relation(conjugation_role, conjugation),
        relation(mass_source_role, packet.mass_scalar_identity),
        relation(coupling_role, packet.coupling_scalar_identity),
    ];
    let requirements_negative = requirements_for_orientation(
        &shared_relations,
        state_role,
        state_negative,
        charge_role,
        charge_negative,
        current_role,
        current_negative,
    );
    let requirements_positive = requirements_for_orientation(
        &shared_relations,
        state_role,
        state_positive,
        charge_role,
        charge_positive,
        current_role,
        current_positive,
    );
    let constraint_negative = push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        ArtifactPayload::ConstraintLaw(super::super::model::ConstraintLawArtifact {
            requirements: requirements_negative.clone(),
        }),
    )?;
    let constraint_positive = push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        ArtifactPayload::ConstraintLaw(super::super::model::ConstraintLawArtifact {
            requirements: requirements_positive.clone(),
        }),
    )?;
    let blueprint_negative = blueprint(
        -1,
        requirements_negative,
        mass_projection,
        constraint_role,
        constraint_negative,
    );
    let blueprint_positive = blueprint(
        1,
        requirements_positive,
        mass_projection,
        constraint_role,
        constraint_positive,
    );
    let member_negative =
        super::super::producer::derive_member_identity_for_authority(&blueprint_negative)
            .map_err(|_| ChargedProfileRefusal::MemberIdentityMismatch)?;
    let member_positive =
        super::super::producer::derive_member_identity_for_authority(&blueprint_positive)
            .map_err(|_| ChargedProfileRefusal::MemberIdentityMismatch)?;
    let artifact_inputs = vec![
        relation(field_role, field),
        relation(operator_role, operator),
        relation(sector_role, packet.primitive_sector_identity),
        relation(mass_source_role, packet.mass_scalar_identity),
        relation(coupling_role, packet.coupling_scalar_identity),
    ];
    push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        ArtifactPayload::SpeciesDerivation(super::super::model::SpeciesDerivationArtifact {
            derivation_kind,
            artifact_inputs: artifact_inputs.clone(),
            constituents: Vec::new(),
            output: blueprint_negative,
        }),
    )?;
    push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        ArtifactPayload::SpeciesDerivation(super::super::model::SpeciesDerivationArtifact {
            derivation_kind,
            artifact_inputs,
            constituents: Vec::new(),
            output: blueprint_positive,
        }),
    )?;
    let mut members = vec![member_negative, member_positive];
    members.sort_unstable();
    if members[0] == members[1] {
        return Err(ChargedProfileRefusal::ChargeConjugationMismatch);
    }
    Ok(CandidateBuild {
        candidates,
        members,
        profile_root_identity,
        profile_role_identity,
        charge_conjugation_producer_sha256,
        charge_conjugation_watchdog_sha256,
        mass_transport_producer_sha256,
        mass_transport_watchdog_sha256,
        applicability_receipt_sha256,
        validity_receipt_sha256,
        admission_evidence,
    })
}

fn requirements_for_orientation(
    shared: &[super::super::model::ArtifactRelation],
    state_role: ArtifactIdentity,
    state: ArtifactIdentity,
    charge_role: ArtifactIdentity,
    charge: ArtifactIdentity,
    current_role: ArtifactIdentity,
    current: ArtifactIdentity,
) -> super::super::model::RequirementSet {
    let mut artifact_relations = shared.to_vec();
    artifact_relations.extend([
        relation(state_role, state),
        relation(charge_role, charge),
        relation(current_role, current),
    ]);
    super::super::model::RequirementSet {
        artifact_relations,
        species_dependencies: Vec::new(),
    }
}

fn blueprint(
    charge_weight: i8,
    requirements: super::super::model::RequirementSet,
    mass_projection: ArtifactIdentity,
    constraint_role: ArtifactIdentity,
    constraint: ArtifactIdentity,
) -> super::super::model::MemberBlueprint {
    super::super::model::MemberBlueprint {
        physical_content: super::super::model::CanonicalArtifact {
            schema_id: "civsim.charged-profile.matter-excitation.v1".to_owned(),
            canonical_bytes: format!("{MEMBER_CLASS_ID}:relative-charge-weight={charge_weight}")
                .into_bytes(),
        },
        requirements,
        mass_proof: super::super::model::MassProofReference::Projection(mass_projection),
        constraint_laws: vec![relation(constraint_role, constraint)],
    }
}

fn charge_coordinate(weight: i8) -> ArtifactPayload {
    ArtifactPayload::ScalarCoordinate(Box::new(super::super::model::ScalarCoordinateArtifact {
        coordinate: super::super::model::CanonicalArtifact {
            schema_id: "civsim.charged-profile.relative-charge-coordinate.v1".to_owned(),
            canonical_bytes: format!("relative-abelian-charge-weight={weight}").into_bytes(),
        },
        exact_value: super::super::model::ExactRationalWire {
            negative: weight < 0,
            numerator_be: vec![1],
            denominator_be: vec![1],
        },
        dimension: super::super::model::DimensionVector::dimensionless(),
    }))
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

fn identity(payload: &ArtifactPayload) -> Result<ArtifactIdentity, ChargedProfileRefusal> {
    super::super::producer::derive_artifact_identity_for_authority(payload)
        .map_err(|_| ChargedProfileRefusal::ArtifactConstructionFailure)
}

fn push_descriptor(
    candidates: &mut Vec<ChargedArtifactCandidate>,
    profile_root_identity: ArtifactIdentity,
    input_sha256: [u8; 32],
    schema_id: &str,
    content: &str,
) -> Result<ArtifactIdentity, ChargedProfileRefusal> {
    push_derived(
        candidates,
        profile_root_identity,
        input_sha256,
        descriptor(schema_id, content),
    )
}

fn push_derived(
    candidates: &mut Vec<ChargedArtifactCandidate>,
    profile_root_identity: ArtifactIdentity,
    input_sha256: [u8; 32],
    payload: ArtifactPayload,
) -> Result<ArtifactIdentity, ChargedProfileRefusal> {
    let artifact_identity = identity(&payload)?;
    candidates.push(ChargedArtifactCandidate {
        identity: artifact_identity,
        admission: derived_admission(input_sha256, profile_root_identity, artifact_identity),
        payload,
    });
    Ok(artifact_identity)
}

fn irreducible_admission(
    packet: &ChargedProfilePacket,
    evidence: &law_premise::TheoryProfileAdmissionEvidence,
) -> RootAdmission {
    RootAdmission {
        tier: Tier::Residue,
        provenance: Provenance::Authored,
        route: AdmissionRoute::Irreducible(Box::new(super::super::model::IrreducibleAdmission {
            derivation_exhaustion_receipt: receipt_binding(
                "civsim.charged-profile.derive-first-exhaustion.v1",
                evidence.derivation_exhaustion_receipt_sha256,
            ),
            buckingham_pi_receipt: receipt_binding(
                "civsim.charged-profile.buckingham-pi.v1",
                evidence.buckingham_pi_receipt_sha256,
            ),
            gap_law_receipt: receipt_binding(
                "civsim.charged-profile.gap-law.v1",
                evidence.gap_law_receipt_sha256,
            ),
            chaos_protocol_receipt: receipt_binding(
                "civsim.charged-profile.chaos-protocol.v1",
                evidence.chaos_protocol_receipt_sha256,
            ),
            residual_law_receipt: receipt_binding(
                "civsim.charged-profile.residual-law.v1",
                evidence.residual_law_receipt_sha256,
            ),
            residual_slot_id: packet.residual_slot_id.clone(),
            residual_slot_receipt: receipt_binding(
                "civsim.charged-profile.residual-slot.v1",
                evidence.residual_slot_receipt_sha256,
            ),
            owner_admission_receipt: receipt_binding(
                "civsim.charged-profile.owner-admission.v1",
                evidence.owner_admission_receipt_sha256,
            ),
            independent_watchdog_receipt: receipt_binding(
                "civsim.charged-profile.independent-watchdog-route.v1",
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
                "civsim.charged-profile.derived-ancestry.v1",
                digest(
                    b"civsim.charged-profile.derived-ancestry.v1",
                    &[&profile_root_identity.0, &artifact_identity.0],
                ),
            ),
            semantic_checker_receipt: receipt_binding(
                "civsim.charged-profile.derived-forward-semantic.v1",
                digest(
                    b"civsim.charged-profile.derived-forward-semantic.v1",
                    &[&input_sha256, &artifact_identity.0],
                ),
            ),
            independent_watchdog_receipt: receipt_binding(
                "civsim.charged-profile.derived-reverse-semantic.v1",
                digest(
                    b"civsim.charged-profile.derived-reverse-semantic.v1",
                    &[&artifact_identity.0, &input_sha256],
                ),
            ),
        }),
    }
}

fn charge_conjugation_receipt(
    domain: &[u8],
    packet: &ChargedProfilePacket,
    profile_root_identity: ArtifactIdentity,
    state_negative: ArtifactIdentity,
    state_positive: ArtifactIdentity,
) -> [u8; 32] {
    digest(
        domain,
        &[
            &profile_root_identity.0,
            &state_negative.0,
            &state_positive.0,
            &(-1_i8).to_be_bytes(),
            &1_i8.to_be_bytes(),
            &packet.mass_scalar_identity.0,
            &packet.primitive_sector_identity.0,
            packet.conjugation_id.as_bytes(),
        ],
    )
}

fn mass_transport_receipt(
    domain: &[u8],
    packet: &ChargedProfilePacket,
    profile_root_identity: ArtifactIdentity,
) -> [u8; 32] {
    digest(
        domain,
        &[
            &profile_root_identity.0,
            &packet.mass_scalar_identity.0,
            &packet.mass_scalar_ancestry_sha256,
            &packet.neutral_mass_projection_identity.0,
            &packet.neutral_mass_projection_ancestry_sha256,
            &packet.root_pair_receipt.digest_sha256,
        ],
    )
}

pub(super) fn canary_evidence(
    packet: &ChargedProfilePacket,
) -> Result<CanaryEvidence, ChargedProfileRefusal> {
    let sealed = sealed_packet()?;
    let cases = mutations(packet);
    let mut transcript = PRODUCER_CANARY_ID.as_bytes().to_vec();
    for (name, mutant) in &cases {
        let refusal = validate_packet_against(mutant, &sealed)
            .expect_err("every charged-profile producer canary must refuse")
            .id();
        append_field(&mut transcript, 1, name.as_bytes());
        append_field(&mut transcript, 2, &sha256(&encode_packet(mutant)));
        append_field(&mut transcript, 3, refusal.as_bytes());
    }
    Ok(CanaryEvidence {
        transcript_id: PRODUCER_CANARY_ID,
        case_count: u32::try_from(cases.len()).map_err(|_| ChargedProfileRefusal::CanaryFailure)?,
        transcript_sha256: sha256(&transcript),
    })
}

fn mutations(packet: &ChargedProfilePacket) -> Vec<(&'static str, ChargedProfilePacket)> {
    let mut cases = Vec::new();
    macro_rules! mutate_string {
        ($name:literal, $field:ident) => {{
            let mut changed = packet.clone();
            changed.$field.push_str(".changed");
            cases.push(($name, changed));
        }};
    }
    mutate_string!("packet_schema", schema_id);
    mutate_string!("profile_id", profile_id);
    mutate_string!("theory_class", theory_class_id);
    mutate_string!("field", field_id);
    mutate_string!("operator", operator_id);
    mutate_string!("state_pair", state_pair_class_id);
    mutate_string!("validity", validity_id);
    mutate_string!("spin", spin_id);
    mutate_string!("statistics", statistics_id);
    mutate_string!("mobility", mobility_id);
    mutate_string!("stability", stability_id);
    mutate_string!("transition", transition_id);
    mutate_string!("conjugation", conjugation_id);
    mutate_string!("member_class", member_class_id);
    mutate_string!("residual_slot", residual_slot_id);
    mutate_string!("owner_record", owner_admission_record);
    mutate_string!("evidence_hash", evidence_full_sha256_hex);
    let mut changed = packet.clone();
    changed.charge_weights = vec![1, 1];
    cases.push(("charge_pair", changed));
    let mut changed = packet.clone();
    changed.floor_authority.digest_sha256[0] ^= 1;
    cases.push(("floor_binding", changed));
    let mut changed = packet.clone();
    changed.root_pair_receipt.digest_sha256[0] ^= 1;
    cases.push(("root_pair", changed));
    let mut changed = packet.clone();
    changed.mass_scalar_identity.0[0] ^= 1;
    cases.push(("mass_scalar", changed));
    let mut changed = packet.clone();
    changed.mass_scalar_ancestry_sha256[0] ^= 1;
    cases.push(("mass_ancestry", changed));
    let mut changed = packet.clone();
    changed.neutral_mass_projection_identity.0[0] ^= 1;
    cases.push(("neutral_mass_projection", changed));
    let mut changed = packet.clone();
    changed.coupling_scalar_identity.0[0] ^= 1;
    cases.push(("coupling_scalar", changed));
    let mut changed = packet.clone();
    changed.primitive_profile_receipt.digest_sha256[0] ^= 1;
    cases.push(("primitive_receipt", changed));
    let mut changed = packet.clone();
    changed.primitive_profile_root_identity.0[0] ^= 1;
    cases.push(("primitive_root", changed));
    let mut changed = packet.clone();
    changed.primitive_sector_identity.0[0] ^= 1;
    cases.push(("primitive_sector", changed));
    let mut changed = packet.clone();
    changed.primitive_member.0[0] ^= 1;
    cases.push(("primitive_member", changed));
    let mut changed = packet.clone();
    changed.global_physical_vocabulary_coverage = true;
    cases.push(("global_coverage", changed));
    let mut changed = packet.clone();
    changed.membership_authority = true;
    cases.push(("membership_authority", changed));
    cases
}

pub(super) fn pair_receipt_digest(
    output: &ChargedProfileCheckerOutput,
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
    producer_canary: CanaryEvidence,
    watchdog_canary: CanaryEvidence,
) -> [u8; 32] {
    let mut bytes = PAIR_DOMAIN.to_vec();
    append_field(&mut bytes, 1, &output.input_sha256);
    append_field(&mut bytes, 2, &producer_result_sha256);
    append_field(&mut bytes, 3, &watchdog_result_sha256);
    append_field(&mut bytes, 4, producer_canary.transcript_id.as_bytes());
    append_field(&mut bytes, 5, &producer_canary.transcript_sha256);
    append_field(&mut bytes, 6, watchdog_canary.transcript_id.as_bytes());
    append_field(&mut bytes, 7, &watchdog_canary.transcript_sha256);
    append_field(&mut bytes, 8, &output.profile_root_identity.0);
    append_field(&mut bytes, 9, &output.profile_role_identity.0);
    for member in &output.members {
        append_field(&mut bytes, 10, &member.0);
    }
    append_field(&mut bytes, 11, &output.evidence_custody_receipt_sha256);
    append_field(&mut bytes, 12, &output.charge_conjugation_producer_sha256);
    append_field(&mut bytes, 13, &output.charge_conjugation_watchdog_sha256);
    append_field(&mut bytes, 14, &output.mass_transport_producer_sha256);
    append_field(&mut bytes, 15, &output.mass_transport_watchdog_sha256);
    append_field(
        &mut bytes,
        16,
        &output
            .admission_evidence
            .irreducible_protocol_capability_sha256,
    );
    sha256(&bytes)
}

fn encode_packet(packet: &ChargedProfilePacket) -> Vec<u8> {
    let mut bytes = INPUT_DOMAIN.to_vec();
    let strings = [
        packet.schema_id.as_bytes(),
        packet.profile_id.as_bytes(),
        packet.theory_class_id.as_bytes(),
        packet.field_id.as_bytes(),
        packet.operator_id.as_bytes(),
        packet.state_pair_class_id.as_bytes(),
        packet.validity_id.as_bytes(),
        packet.spin_id.as_bytes(),
        packet.statistics_id.as_bytes(),
        packet.mobility_id.as_bytes(),
        packet.stability_id.as_bytes(),
        packet.transition_id.as_bytes(),
        packet.conjugation_id.as_bytes(),
        packet.member_class_id.as_bytes(),
        packet.residual_slot_id.as_bytes(),
        packet.owner_admission_record.as_bytes(),
        packet.evidence_citation.as_bytes(),
        packet.evidence_source_url.as_bytes(),
        packet.evidence_full_sha256_hex.as_bytes(),
        packet.evidence_slim_sha256_hex.as_bytes(),
        packet.evidence_anchor.as_bytes(),
    ];
    for (index, value) in strings.into_iter().enumerate() {
        append_field(
            &mut bytes,
            u16::try_from(index + 1).unwrap_or(u16::MAX),
            value,
        );
    }
    let mut weights = packet.charge_weights.clone();
    weights.sort_unstable();
    for weight in weights {
        append_field(&mut bytes, 22, &weight.to_be_bytes());
    }
    append_field(&mut bytes, 23, packet.floor_authority.schema_id.as_bytes());
    append_field(&mut bytes, 24, &packet.floor_authority.digest_sha256);
    append_field(
        &mut bytes,
        25,
        packet.root_pair_receipt.schema_id.as_bytes(),
    );
    append_field(&mut bytes, 26, &packet.root_pair_receipt.digest_sha256);
    append_field(&mut bytes, 27, packet.mass_source_entry_id.as_bytes());
    append_field(&mut bytes, 28, &packet.mass_scalar_identity.0);
    append_field(&mut bytes, 29, &packet.mass_scalar_ancestry_sha256);
    append_field(&mut bytes, 30, &packet.neutral_mass_projection_identity.0);
    append_field(
        &mut bytes,
        31,
        &packet.neutral_mass_projection_ancestry_sha256,
    );
    append_field(&mut bytes, 32, packet.coupling_source_entry_id.as_bytes());
    append_field(&mut bytes, 33, &packet.coupling_scalar_identity.0);
    append_field(&mut bytes, 34, &packet.coupling_scalar_ancestry_sha256);
    append_field(
        &mut bytes,
        35,
        packet.primitive_profile_receipt.schema_id.as_bytes(),
    );
    append_field(
        &mut bytes,
        36,
        &packet.primitive_profile_receipt.digest_sha256,
    );
    append_field(&mut bytes, 37, &packet.primitive_profile_root_identity.0);
    append_field(&mut bytes, 38, &packet.primitive_sector_identity.0);
    append_field(&mut bytes, 39, &packet.primitive_member.0);
    append_field(&mut bytes, 40, packet.primitive_residual_slot_id.as_bytes());
    append_field(
        &mut bytes,
        41,
        &[u8::from(packet.global_physical_vocabulary_coverage)],
    );
    append_field(&mut bytes, 42, &[u8::from(packet.membership_authority)]);
    bytes
}

fn encode_output(
    input_sha256: [u8; 32],
    build: &CandidateBuild,
    evidence_custody_receipt_sha256: [u8; 32],
) -> Vec<u8> {
    let mut bytes = OUTPUT_DOMAIN.to_vec();
    append_field(&mut bytes, 1, &input_sha256);
    append_field(&mut bytes, 2, &build.profile_root_identity.0);
    append_field(&mut bytes, 3, &build.profile_role_identity.0);
    for candidate in &build.candidates {
        let mut artifact = candidate.identity.0.to_vec();
        artifact.extend_from_slice(&encode_admission(&candidate.admission));
        append_field(&mut bytes, 4, &artifact);
    }
    for member in &build.members {
        append_field(&mut bytes, 5, &member.0);
    }
    append_field(&mut bytes, 6, &evidence_custody_receipt_sha256);
    append_field(&mut bytes, 7, &build.charge_conjugation_producer_sha256);
    append_field(&mut bytes, 8, &build.charge_conjugation_watchdog_sha256);
    append_field(&mut bytes, 9, &build.mass_transport_producer_sha256);
    append_field(&mut bytes, 10, &build.mass_transport_watchdog_sha256);
    append_field(&mut bytes, 11, &build.applicability_receipt_sha256);
    append_field(&mut bytes, 12, &build.validity_receipt_sha256);
    append_field(
        &mut bytes,
        13,
        &build
            .admission_evidence
            .irreducible_protocol_capability_sha256,
    );
    append_field(
        &mut bytes,
        14,
        &build
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
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
