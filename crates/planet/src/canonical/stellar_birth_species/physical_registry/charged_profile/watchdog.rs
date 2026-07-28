//! Reverse reconstruction and independent inspector for the charged profile.

use super::*;
use crate::canonical::stellar_birth_species::law_premise;
use civsim_ledger::{Provenance, Tier};
use civsim_units::{digest::sha256, physics_floor::sealed_physical_floor_authority_binding};
use std::collections::BTreeMap;

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
const BYTE_LIMIT: u32 = 1_048_576;

struct CandidateReconstruction {
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
    let primitive = super::super::primitive_profile::project_admitted_profile()
        .map_err(|_| ChargedProfileRefusal::SealedSourceUnavailable)?;
    let roots = super::super::repository_roots::project_repository_roots()
        .map_err(|_| ChargedProfileRefusal::SealedSourceUnavailable)?;
    let bindings = super::super::repository_roots::coordinate_bindings(&roots)
        .map_err(|_| ChargedProfileRefusal::SealedSourceUnavailable)?;
    let mass = bindings
        .iter()
        .rev()
        .find(|binding| binding.entry_id.as_bytes() == MASS_SOURCE_ENTRY_ID.as_bytes())
        .ok_or(ChargedProfileRefusal::SealedSourceUnavailable)?;
    let coupling = bindings
        .iter()
        .rev()
        .find(|binding| binding.entry_id.as_bytes() == COUPLING_SOURCE_ENTRY_ID.as_bytes())
        .ok_or(ChargedProfileRefusal::SealedSourceUnavailable)?;
    let neutral_mass_projection_identity = mass
        .mass_projection_identity
        .ok_or(ChargedProfileRefusal::SealedSourceUnavailable)?;
    let neutral_mass_projection_ancestry_sha256 = mass
        .mass_projection_ancestry_sha256
        .ok_or(ChargedProfileRefusal::SealedSourceUnavailable)?;
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| ChargedProfileRefusal::SealedSourceUnavailable)?;
    Ok(ChargedProfilePacket {
        membership_authority: false,
        global_physical_vocabulary_coverage: false,
        primitive_residual_slot_id: primitive.receipt.residual_slot_id.to_owned(),
        primitive_member: primitive.member,
        primitive_sector_identity: primitive.sector_identity,
        primitive_profile_root_identity: primitive.receipt.profile_root_identity,
        primitive_profile_receipt: receipt_binding(
            primitive.receipt.schema_id,
            primitive.receipt.pair_receipt_sha256,
        ),
        coupling_scalar_ancestry_sha256: coupling.scalar_ancestry_sha256,
        coupling_scalar_identity: coupling.scalar_identity,
        coupling_source_entry_id: coupling.entry_id.clone(),
        neutral_mass_projection_ancestry_sha256,
        neutral_mass_projection_identity,
        mass_scalar_ancestry_sha256: mass.scalar_ancestry_sha256,
        mass_scalar_identity: mass.scalar_identity,
        mass_source_entry_id: mass.entry_id.clone(),
        root_pair_receipt: mass.root_pair_receipt.clone(),
        floor_authority: receipt_binding(floor.schema_id().as_str(), floor.digest()),
        evidence_anchor: EVIDENCE_ANCHOR.to_owned(),
        evidence_slim_sha256_hex: EVIDENCE_SLIM_SHA256_HEX.to_owned(),
        evidence_full_sha256_hex: EVIDENCE_FULL_SHA256_HEX.to_owned(),
        evidence_source_url: EVIDENCE_SOURCE_URL.to_owned(),
        evidence_citation: EVIDENCE_CITATION.to_owned(),
        owner_admission_record: OWNER_ADMISSION_RECORD.to_owned(),
        residual_slot_id: RESIDUAL_SLOT_ID.to_owned(),
        charge_weights: vec![-1, 1],
        member_class_id: MEMBER_CLASS_ID.to_owned(),
        conjugation_id: CONJUGATION_ID.to_owned(),
        transition_id: TRANSITION_ID.to_owned(),
        stability_id: STABILITY_ID.to_owned(),
        mobility_id: MOBILITY_ID.to_owned(),
        statistics_id: STATISTICS_ID.to_owned(),
        spin_id: SPIN_ID.to_owned(),
        validity_id: VALIDITY_ID.to_owned(),
        state_pair_class_id: STATE_PAIR_CLASS_ID.to_owned(),
        operator_id: OPERATOR_ID.to_owned(),
        field_id: FIELD_ID.to_owned(),
        theory_class_id: THEORY_CLASS_ID.to_owned(),
        profile_id: PROFILE_ID.to_owned(),
        schema_id: PACKET_SCHEMA_ID.to_owned(),
    })
}

pub(super) fn inspect(
    packet: &ChargedProfilePacket,
) -> Result<ChargedProfileCheckerOutput, ChargedProfileRefusal> {
    let sealed = sealed_packet()?;
    inspect_packet_against(packet, &sealed)?;
    let packet_bytes = write_packet(packet);
    if u32::try_from(packet_bytes.len()).map_or(true, |length| length > BYTE_LIMIT) {
        return Err(ChargedProfileRefusal::ArtifactConstructionFailure);
    }
    let input_sha256 = sha256(&packet_bytes);
    let evidence_custody_receipt_sha256 = hash_fields(
        b"civsim.planet.charged-matter-profile.evidence-custody.v1",
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
    if reconstruction.candidates.len() != ARTIFACT_COUNT
        || reconstruction.members.len() != MEMBER_COUNT
    {
        return Err(ChargedProfileRefusal::ArtifactCountMismatch);
    }
    let canonical_bytes = write_output(
        input_sha256,
        &reconstruction,
        evidence_custody_receipt_sha256,
    );
    if u32::try_from(canonical_bytes.len()).map_or(true, |length| length > BYTE_LIMIT) {
        return Err(ChargedProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(ChargedProfileCheckerOutput {
        input_sha256,
        canonical_bytes,
        candidates: reconstruction.candidates,
        members: reconstruction.members,
        profile_root_identity: reconstruction.profile_root_identity,
        profile_role_identity: reconstruction.profile_role_identity,
        evidence_custody_receipt_sha256,
        charge_conjugation_producer_sha256: reconstruction.charge_conjugation_producer_sha256,
        charge_conjugation_watchdog_sha256: reconstruction.charge_conjugation_watchdog_sha256,
        mass_transport_producer_sha256: reconstruction.mass_transport_producer_sha256,
        mass_transport_watchdog_sha256: reconstruction.mass_transport_watchdog_sha256,
        applicability_receipt_sha256: reconstruction.applicability_receipt_sha256,
        validity_receipt_sha256: reconstruction.validity_receipt_sha256,
        admission_evidence: reconstruction.admission_evidence,
    })
}

fn inspect_packet_against(
    packet: &ChargedProfilePacket,
    sealed: &ChargedProfilePacket,
) -> Result<(), ChargedProfileRefusal> {
    if packet.schema_id.as_bytes() != PACKET_SCHEMA_ID.as_bytes() {
        return Err(ChargedProfileRefusal::PacketSchemaMismatch);
    }
    if packet.profile_id.as_bytes() != PROFILE_ID.as_bytes()
        || packet.residual_slot_id.as_bytes() != RESIDUAL_SLOT_ID.as_bytes()
        || packet.owner_admission_record.as_bytes() != OWNER_ADMISSION_RECORD.as_bytes()
    {
        return Err(ChargedProfileRefusal::ProfileIdentityMismatch);
    }
    let theory = [
        (
            packet.member_class_id.as_bytes(),
            MEMBER_CLASS_ID.as_bytes(),
        ),
        (packet.conjugation_id.as_bytes(), CONJUGATION_ID.as_bytes()),
        (packet.transition_id.as_bytes(), TRANSITION_ID.as_bytes()),
        (packet.stability_id.as_bytes(), STABILITY_ID.as_bytes()),
        (packet.mobility_id.as_bytes(), MOBILITY_ID.as_bytes()),
        (packet.statistics_id.as_bytes(), STATISTICS_ID.as_bytes()),
        (packet.spin_id.as_bytes(), SPIN_ID.as_bytes()),
        (packet.validity_id.as_bytes(), VALIDITY_ID.as_bytes()),
        (
            packet.state_pair_class_id.as_bytes(),
            STATE_PAIR_CLASS_ID.as_bytes(),
        ),
        (packet.operator_id.as_bytes(), OPERATOR_ID.as_bytes()),
        (packet.field_id.as_bytes(), FIELD_ID.as_bytes()),
        (
            packet.theory_class_id.as_bytes(),
            THEORY_CLASS_ID.as_bytes(),
        ),
    ];
    if theory
        .into_iter()
        .rev()
        .any(|(found, expected)| found != expected)
    {
        return Err(ChargedProfileRefusal::TheoryProfileMismatch);
    }
    let custody = [
        (
            packet.evidence_anchor.as_bytes(),
            EVIDENCE_ANCHOR.as_bytes(),
        ),
        (
            packet.evidence_slim_sha256_hex.as_bytes(),
            EVIDENCE_SLIM_SHA256_HEX.as_bytes(),
        ),
        (
            packet.evidence_full_sha256_hex.as_bytes(),
            EVIDENCE_FULL_SHA256_HEX.as_bytes(),
        ),
        (
            packet.evidence_source_url.as_bytes(),
            EVIDENCE_SOURCE_URL.as_bytes(),
        ),
        (
            packet.evidence_citation.as_bytes(),
            EVIDENCE_CITATION.as_bytes(),
        ),
    ];
    if custody
        .into_iter()
        .any(|(found, expected)| found != expected)
    {
        return Err(ChargedProfileRefusal::EvidenceCustodyMismatch);
    }
    if packet.floor_authority.schema_id.as_bytes() != sealed.floor_authority.schema_id.as_bytes()
        || packet.floor_authority.digest_sha256 != sealed.floor_authority.digest_sha256
        || packet
            .floor_authority
            .digest_sha256
            .iter()
            .all(|byte| *byte == 0)
    {
        return Err(ChargedProfileRefusal::FloorBindingMismatch);
    }
    let root_match = packet.root_pair_receipt == sealed.root_pair_receipt
        && packet.root_pair_receipt.digest_sha256 != [0; 32]
        && packet.mass_source_entry_id.as_bytes() == MASS_SOURCE_ENTRY_ID.as_bytes()
        && packet.mass_source_entry_id == sealed.mass_source_entry_id
        && packet.mass_scalar_identity == sealed.mass_scalar_identity
        && packet.mass_scalar_ancestry_sha256 == sealed.mass_scalar_ancestry_sha256
        && packet.neutral_mass_projection_identity == sealed.neutral_mass_projection_identity
        && packet.neutral_mass_projection_ancestry_sha256
            == sealed.neutral_mass_projection_ancestry_sha256
        && packet.coupling_source_entry_id.as_bytes() == COUPLING_SOURCE_ENTRY_ID.as_bytes()
        && packet.coupling_source_entry_id == sealed.coupling_source_entry_id
        && packet.coupling_scalar_identity == sealed.coupling_scalar_identity
        && packet.coupling_scalar_ancestry_sha256 == sealed.coupling_scalar_ancestry_sha256;
    if !root_match {
        return Err(ChargedProfileRefusal::RootBindingMismatch);
    }
    let primitive_match = packet.primitive_residual_slot_id == sealed.primitive_residual_slot_id
        && packet.primitive_member == sealed.primitive_member
        && packet.primitive_sector_identity == sealed.primitive_sector_identity
        && packet.primitive_profile_root_identity == sealed.primitive_profile_root_identity
        && packet.primitive_profile_receipt == sealed.primitive_profile_receipt
        && packet.primitive_profile_receipt.digest_sha256 != [0; 32];
    if !primitive_match {
        return Err(ChargedProfileRefusal::PrimitiveProfileBindingMismatch);
    }
    let mut weights = packet.charge_weights.clone();
    weights.sort_unstable_by(|left, right| right.cmp(left));
    if weights != [1, -1] {
        return Err(ChargedProfileRefusal::ChargeConjugationMismatch);
    }
    if packet.primitive_residual_slot_id.as_bytes() == packet.residual_slot_id.as_bytes() {
        return Err(ChargedProfileRefusal::ResidualSlotCollision);
    }
    if packet.membership_authority || packet.global_physical_vocabulary_coverage {
        return Err(ChargedProfileRefusal::UnsupportedScope);
    }
    Ok(())
}

fn reconstruct_candidates(
    packet: &ChargedProfilePacket,
    input_sha256: [u8; 32],
    evidence_custody_receipt_sha256: [u8; 32],
) -> Result<CandidateReconstruction, ChargedProfileRefusal> {
    let mut by_key = BTreeMap::<&'static str, ChargedArtifactCandidate>::new();
    let profile_payload = make_descriptor(
        PROFILE_SCHEMA_ID,
        format!(
            "profile_id={}\ntheory_class={}\nevidence_custody_sha256={}\nprimitive_profile_root={}\nmass_coordinate={}\ncoupling_coordinate={}\nauthority_scope=claim-local\n",
            packet.profile_id,
            packet.theory_class_id,
            to_hex(evidence_custody_receipt_sha256),
            to_hex(packet.primitive_profile_root_identity.0),
            to_hex(packet.mass_scalar_identity.0),
            to_hex(packet.coupling_scalar_identity.0),
        )
        .into_bytes(),
    );
    let profile_root_identity = recompute_identity(&profile_payload)?;
    let descriptor_specs = [
        (
            "profile_role",
            "civsim.charged-profile.role.profile-root.v1",
            "admitted-charged-matter-profile-role",
        ),
        (
            "derivation_kind",
            "civsim.charged-profile.derivation-kind.v1",
            "charge-conjugate-matter-from-admitted-local-profile",
        ),
        (
            "field_role",
            "civsim.charged-profile.role.field-input.v1",
            "matter-field-input-role",
        ),
        (
            "field",
            "civsim.charged-profile.field.v1",
            packet.field_id.as_str(),
        ),
        (
            "operator_role",
            "civsim.charged-profile.role.operator-input.v1",
            "matter-operator-input-role",
        ),
        (
            "operator",
            "civsim.charged-profile.operator.v1",
            packet.operator_id.as_str(),
        ),
        (
            "state_role",
            "civsim.charged-profile.role.state-requirement.v1",
            "conjugate-state-requirement-role",
        ),
        (
            "state_negative",
            "civsim.charged-profile.state.v1",
            "conjugate-orbit-state:relative-charge-weight=-1",
        ),
        (
            "state_positive",
            "civsim.charged-profile.state.v1",
            "conjugate-orbit-state:relative-charge-weight=1",
        ),
        (
            "sector_role",
            "civsim.charged-profile.role.sector-requirement.v1",
            "admitted-abelian-sector-requirement-role",
        ),
        (
            "validity_role",
            "civsim.charged-profile.role.validity-requirement.v1",
            "local-validity-requirement-role",
        ),
        (
            "validity",
            "civsim.charged-profile.validity.v1",
            packet.validity_id.as_str(),
        ),
        (
            "spin_role",
            "civsim.charged-profile.role.spin-requirement.v1",
            "twice-spin-requirement-role",
        ),
        (
            "spin",
            "civsim.charged-profile.spin.v1",
            packet.spin_id.as_str(),
        ),
        (
            "statistics_role",
            "civsim.charged-profile.role.statistics-requirement.v1",
            "statistics-requirement-role",
        ),
        (
            "statistics",
            "civsim.charged-profile.statistics.v1",
            packet.statistics_id.as_str(),
        ),
        (
            "charge_role",
            "civsim.charged-profile.role.charge-requirement.v1",
            "relative-abelian-charge-weight-role",
        ),
        (
            "current_role",
            "civsim.charged-profile.role.current-requirement.v1",
            "conserved-current-requirement-role",
        ),
        (
            "current_negative",
            "civsim.charged-profile.current.v1",
            "conserved-current:relative-charge-weight=-1",
        ),
        (
            "current_positive",
            "civsim.charged-profile.current.v1",
            "conserved-current:relative-charge-weight=1",
        ),
        (
            "mobility_role",
            "civsim.charged-profile.role.mobility-requirement.v1",
            "mobility-requirement-role",
        ),
        (
            "mobility",
            "civsim.charged-profile.mobility.v1",
            packet.mobility_id.as_str(),
        ),
        (
            "stability_role",
            "civsim.charged-profile.role.stability-requirement.v1",
            "local-stability-requirement-role",
        ),
        (
            "stability",
            "civsim.charged-profile.stability.v1",
            packet.stability_id.as_str(),
        ),
        (
            "transition_role",
            "civsim.charged-profile.role.transition-requirement.v1",
            "local-transition-requirement-role",
        ),
        (
            "transition",
            "civsim.charged-profile.transition.v1",
            packet.transition_id.as_str(),
        ),
        (
            "conjugation_role",
            "civsim.charged-profile.role.conjugation-requirement.v1",
            "charge-conjugation-closure-requirement-role",
        ),
        (
            "mass_source_role",
            "civsim.charged-profile.role.mass-source.v1",
            "transported-species-rest-mass-source-role",
        ),
        (
            "coupling_role",
            "civsim.charged-profile.role.coupling-source.v1",
            "dimensionless-coupling-coordinate-role",
        ),
        (
            "constraint_role",
            "civsim.charged-profile.role.constraint.v1",
            "member-local-charge-conjugation-constraint-role",
        ),
    ];
    for (key, schema, text) in descriptor_specs.into_iter().rev() {
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
    let state_negative = lookup_identity(&by_key, "state_negative")?;
    let state_positive = lookup_identity(&by_key, "state_positive")?;
    let sector_role = lookup_identity(&by_key, "sector_role")?;
    let validity_role = lookup_identity(&by_key, "validity_role")?;
    let validity = lookup_identity(&by_key, "validity")?;
    let spin_role = lookup_identity(&by_key, "spin_role")?;
    let spin = lookup_identity(&by_key, "spin")?;
    let statistics_role = lookup_identity(&by_key, "statistics_role")?;
    let statistics = lookup_identity(&by_key, "statistics")?;
    let charge_role = lookup_identity(&by_key, "charge_role")?;
    let current_role = lookup_identity(&by_key, "current_role")?;
    let current_negative = lookup_identity(&by_key, "current_negative")?;
    let current_positive = lookup_identity(&by_key, "current_positive")?;
    let mobility_role = lookup_identity(&by_key, "mobility_role")?;
    let mobility = lookup_identity(&by_key, "mobility")?;
    let stability_role = lookup_identity(&by_key, "stability_role")?;
    let stability = lookup_identity(&by_key, "stability")?;
    let transition_role = lookup_identity(&by_key, "transition_role")?;
    let transition = lookup_identity(&by_key, "transition")?;
    let conjugation_role = lookup_identity(&by_key, "conjugation_role")?;
    let mass_source_role = lookup_identity(&by_key, "mass_source_role")?;
    let coupling_role = lookup_identity(&by_key, "coupling_role")?;
    let constraint_role = lookup_identity(&by_key, "constraint_role")?;

    let charge_conjugation_producer_sha256 = charge_receipt(
        CHARGE_PRODUCER_DOMAIN,
        packet,
        profile_root_identity,
        state_negative,
        state_positive,
    );
    let charge_conjugation_watchdog_sha256 = charge_receipt(
        CHARGE_WATCHDOG_DOMAIN,
        packet,
        profile_root_identity,
        state_negative,
        state_positive,
    );
    let mass_transport_producer_sha256 =
        mass_receipt(MASS_PRODUCER_DOMAIN, packet, profile_root_identity);
    let mass_transport_watchdog_sha256 =
        mass_receipt(MASS_WATCHDOG_DOMAIN, packet, profile_root_identity);
    let mass_receipts = [
        mass_transport_watchdog_sha256,
        mass_transport_producer_sha256,
    ];
    if mass_receipts
        .iter()
        .any(|digest| digest.iter().all(|byte| *byte == 0))
        || mass_receipts[0] == mass_receipts[1]
    {
        return Err(ChargedProfileRefusal::MassTransportMismatch);
    }
    let charge_receipts = [
        charge_conjugation_watchdog_sha256,
        charge_conjugation_producer_sha256,
    ];
    if charge_receipts
        .iter()
        .any(|digest| digest.iter().all(|byte| *byte == 0))
        || charge_receipts[0] == charge_receipts[1]
    {
        return Err(ChargedProfileRefusal::ChargeConjugationMismatch);
    }
    let receipts = [
        mass_receipts[0],
        mass_receipts[1],
        charge_receipts[0],
        charge_receipts[1],
    ];
    if receipts
        .iter()
        .enumerate()
        .any(|(index, digest)| receipts[index + 1..].contains(digest))
    {
        return Err(ChargedProfileRefusal::ChargeConjugationMismatch);
    }
    insert_derived(
        &mut by_key,
        "conjugation",
        profile_root_identity,
        input_sha256,
        make_descriptor(
            "civsim.charged-profile.charge-conjugation-closure.v1",
            format!(
                "{}\nforward_sha256={}\nreverse_sha256={}",
                packet.conjugation_id,
                to_hex(charge_conjugation_producer_sha256),
                to_hex(charge_conjugation_watchdog_sha256),
            )
            .into_bytes(),
        ),
    )?;
    let conjugation = lookup_identity(&by_key, "conjugation")?;
    insert_derived(
        &mut by_key,
        "charge_negative",
        profile_root_identity,
        input_sha256,
        make_charge_coordinate(-1),
    )?;
    insert_derived(
        &mut by_key,
        "charge_positive",
        profile_root_identity,
        input_sha256,
        make_charge_coordinate(1),
    )?;
    let charge_negative = lookup_identity(&by_key, "charge_negative")?;
    let charge_positive = lookup_identity(&by_key, "charge_positive")?;
    insert_derived(
        &mut by_key,
        "mass_projection",
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
                source_coordinate: packet.mass_scalar_identity,
                source_pair_receipt: packet.root_pair_receipt.clone(),
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
    let mass_projection = lookup_identity(&by_key, "mass_projection")?;

    let applicability_receipt_sha256 = hash_fields(
        APPLICABILITY_DOMAIN,
        [
            charge_conjugation_producer_sha256.as_slice(),
            charge_conjugation_watchdog_sha256.as_slice(),
            mass_transport_producer_sha256.as_slice(),
            mass_transport_watchdog_sha256.as_slice(),
            packet.primitive_profile_receipt.digest_sha256.as_slice(),
            packet.root_pair_receipt.digest_sha256.as_slice(),
        ],
    );
    let validity_receipt_sha256 = hash_fields(
        VALIDITY_DOMAIN,
        [
            applicability_receipt_sha256.as_slice(),
            packet.validity_id.as_bytes(),
            input_sha256.as_slice(),
        ],
    );
    let claim_identity = hash_fields(
        PROFILE_CLAIM_DOMAIN,
        [
            profile_root_identity.0.as_slice(),
            profile_role_identity.0.as_slice(),
            input_sha256.as_slice(),
        ],
    );
    let admission_evidence = law_premise::inspect_theory_profile_admission(
        &law_premise::TheoryProfileAdmissionRequest {
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
        },
    )
    .map_err(|_| ChargedProfileRefusal::ArtifactConstructionFailure)?;
    if admission_evidence.decision_id != "irreducible_protocol_structurally_bound"
        || admission_evidence.target_claim_identity != claim_identity
        || admission_evidence.target_role_identity != profile_role_identity.0
        || admission_evidence.target_content_identity != profile_root_identity.0
    {
        return Err(ChargedProfileRefusal::ArtifactConstructionFailure);
    }
    by_key.insert(
        "profile",
        ChargedArtifactCandidate {
            identity: profile_root_identity,
            admission: reconstruct_irreducible_admission(packet, &admission_evidence),
            payload: profile_payload,
        },
    );

    let common = vec![
        edge(field_role, field),
        edge(sector_role, packet.primitive_sector_identity),
        edge(validity_role, validity),
        edge(spin_role, spin),
        edge(statistics_role, statistics),
        edge(mobility_role, mobility),
        edge(stability_role, stability),
        edge(transition_role, transition),
        edge(conjugation_role, conjugation),
        edge(mass_source_role, packet.mass_scalar_identity),
        edge(coupling_role, packet.coupling_scalar_identity),
    ];
    let negative_requirements = oriented_requirements(
        &common,
        state_role,
        state_negative,
        charge_role,
        charge_negative,
        current_role,
        current_negative,
    );
    let positive_requirements = oriented_requirements(
        &common,
        state_role,
        state_positive,
        charge_role,
        charge_positive,
        current_role,
        current_positive,
    );
    insert_derived(
        &mut by_key,
        "constraint_negative",
        profile_root_identity,
        input_sha256,
        ArtifactPayload::ConstraintLaw(super::super::model::ConstraintLawArtifact {
            requirements: negative_requirements.clone(),
        }),
    )?;
    insert_derived(
        &mut by_key,
        "constraint_positive",
        profile_root_identity,
        input_sha256,
        ArtifactPayload::ConstraintLaw(super::super::model::ConstraintLawArtifact {
            requirements: positive_requirements.clone(),
        }),
    )?;
    let constraint_negative = lookup_identity(&by_key, "constraint_negative")?;
    let constraint_positive = lookup_identity(&by_key, "constraint_positive")?;
    let negative_blueprint = make_blueprint(
        -1,
        negative_requirements,
        mass_projection,
        constraint_role,
        constraint_negative,
    );
    let positive_blueprint = make_blueprint(
        1,
        positive_requirements,
        mass_projection,
        constraint_role,
        constraint_positive,
    );
    let member_negative =
        super::super::watchdog::derive_member_identity_for_authority(&negative_blueprint)
            .map_err(|_| ChargedProfileRefusal::MemberIdentityMismatch)?;
    let member_positive =
        super::super::watchdog::derive_member_identity_for_authority(&positive_blueprint)
            .map_err(|_| ChargedProfileRefusal::MemberIdentityMismatch)?;
    let artifact_inputs = vec![
        edge(field_role, field),
        edge(operator_role, operator),
        edge(sector_role, packet.primitive_sector_identity),
        edge(mass_source_role, packet.mass_scalar_identity),
        edge(coupling_role, packet.coupling_scalar_identity),
    ];
    insert_derived(
        &mut by_key,
        "species_negative",
        profile_root_identity,
        input_sha256,
        ArtifactPayload::SpeciesDerivation(super::super::model::SpeciesDerivationArtifact {
            derivation_kind,
            artifact_inputs: artifact_inputs.clone(),
            constituents: Vec::new(),
            output: negative_blueprint,
        }),
    )?;
    insert_derived(
        &mut by_key,
        "species_positive",
        profile_root_identity,
        input_sha256,
        ArtifactPayload::SpeciesDerivation(super::super::model::SpeciesDerivationArtifact {
            derivation_kind,
            artifact_inputs,
            constituents: Vec::new(),
            output: positive_blueprint,
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
        "state_negative",
        "state_positive",
        "sector_role",
        "validity_role",
        "validity",
        "spin_role",
        "spin",
        "statistics_role",
        "statistics",
        "charge_role",
        "current_role",
        "current_negative",
        "current_positive",
        "mobility_role",
        "mobility",
        "stability_role",
        "stability",
        "transition_role",
        "transition",
        "conjugation_role",
        "conjugation",
        "mass_source_role",
        "coupling_role",
        "constraint_role",
        "charge_negative",
        "charge_positive",
        "mass_projection",
        "constraint_negative",
        "constraint_positive",
        "species_negative",
        "species_positive",
    ];
    let candidates = order
        .into_iter()
        .map(|key| {
            by_key
                .remove(key)
                .ok_or(ChargedProfileRefusal::ArtifactConstructionFailure)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !by_key.is_empty() {
        return Err(ChargedProfileRefusal::ArtifactCountMismatch);
    }
    let mut members = vec![member_positive, member_negative];
    members.sort_unstable();
    if members[0] == members[1] {
        return Err(ChargedProfileRefusal::ChargeConjugationMismatch);
    }
    Ok(CandidateReconstruction {
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

fn oriented_requirements(
    common: &[super::super::model::ArtifactRelation],
    state_role: ArtifactIdentity,
    state: ArtifactIdentity,
    charge_role: ArtifactIdentity,
    charge: ArtifactIdentity,
    current_role: ArtifactIdentity,
    current: ArtifactIdentity,
) -> super::super::model::RequirementSet {
    let mut artifact_relations = Vec::with_capacity(common.len() + 3);
    artifact_relations.extend_from_slice(common);
    artifact_relations.push(edge(state_role, state));
    artifact_relations.push(edge(charge_role, charge));
    artifact_relations.push(edge(current_role, current));
    super::super::model::RequirementSet {
        artifact_relations,
        species_dependencies: Vec::new(),
    }
}

fn make_blueprint(
    weight: i8,
    requirements: super::super::model::RequirementSet,
    mass_projection: ArtifactIdentity,
    constraint_role: ArtifactIdentity,
    constraint: ArtifactIdentity,
) -> super::super::model::MemberBlueprint {
    super::super::model::MemberBlueprint {
        physical_content: super::super::model::CanonicalArtifact {
            schema_id: "civsim.charged-profile.matter-excitation.v1".to_owned(),
            canonical_bytes: format!("{MEMBER_CLASS_ID}:relative-charge-weight={weight}")
                .into_bytes(),
        },
        requirements,
        mass_proof: super::super::model::MassProofReference::Projection(mass_projection),
        constraint_laws: vec![edge(constraint_role, constraint)],
    }
}

fn make_charge_coordinate(weight: i8) -> ArtifactPayload {
    ArtifactPayload::ScalarCoordinate(Box::new(super::super::model::ScalarCoordinateArtifact {
        coordinate: super::super::model::CanonicalArtifact {
            schema_id: "civsim.charged-profile.relative-charge-coordinate.v1".to_owned(),
            canonical_bytes: format!("relative-abelian-charge-weight={weight}").into_bytes(),
        },
        exact_value: super::super::model::ExactRationalWire {
            negative: weight.is_negative(),
            numerator_be: vec![1],
            denominator_be: vec![1],
        },
        dimension: super::super::model::DimensionVector::dimensionless(),
    }))
}

fn make_descriptor(schema_id: &str, bytes: Vec<u8>) -> ArtifactPayload {
    ArtifactPayload::PhysicalDescriptor(super::super::model::CanonicalArtifact {
        schema_id: schema_id.to_owned(),
        canonical_bytes: bytes,
    })
}

fn lookup_identity(
    by_key: &BTreeMap<&'static str, ChargedArtifactCandidate>,
    key: &'static str,
) -> Result<ArtifactIdentity, ChargedProfileRefusal> {
    by_key
        .get(key)
        .map(|candidate| candidate.identity)
        .ok_or(ChargedProfileRefusal::ArtifactConstructionFailure)
}

fn edge(role: ArtifactIdentity, target: ArtifactIdentity) -> super::super::model::ArtifactRelation {
    super::super::model::ArtifactRelation { role, target }
}

fn recompute_identity(
    payload: &ArtifactPayload,
) -> Result<ArtifactIdentity, ChargedProfileRefusal> {
    super::super::watchdog::derive_artifact_identity_for_authority(payload)
        .map_err(|_| ChargedProfileRefusal::ArtifactConstructionFailure)
}

fn insert_derived(
    by_key: &mut BTreeMap<&'static str, ChargedArtifactCandidate>,
    key: &'static str,
    profile_root_identity: ArtifactIdentity,
    input_sha256: [u8; 32],
    payload: ArtifactPayload,
) -> Result<(), ChargedProfileRefusal> {
    let identity = recompute_identity(&payload)?;
    let candidate = ChargedArtifactCandidate {
        identity,
        admission: reconstruct_derived_admission(input_sha256, profile_root_identity, identity),
        payload,
    };
    if by_key.insert(key, candidate).is_some() {
        return Err(ChargedProfileRefusal::ArtifactConstructionFailure);
    }
    Ok(())
}

fn reconstruct_irreducible_admission(
    packet: &ChargedProfilePacket,
    evidence: &law_premise::TheoryProfileAdmissionEvidence,
) -> RootAdmission {
    RootAdmission {
        tier: Tier::Residue,
        provenance: Provenance::Authored,
        route: AdmissionRoute::Irreducible(Box::new(super::super::model::IrreducibleAdmission {
            independent_watchdog_receipt: receipt_binding(
                "civsim.charged-profile.independent-watchdog-route.v1",
                evidence.independent_watchdog_receipt_sha256,
            ),
            owner_admission_receipt: receipt_binding(
                "civsim.charged-profile.owner-admission.v1",
                evidence.owner_admission_receipt_sha256,
            ),
            residual_slot_receipt: receipt_binding(
                "civsim.charged-profile.residual-slot.v1",
                evidence.residual_slot_receipt_sha256,
            ),
            residual_slot_id: packet.residual_slot_id.clone(),
            residual_law_receipt: receipt_binding(
                "civsim.charged-profile.residual-law.v1",
                evidence.residual_law_receipt_sha256,
            ),
            chaos_protocol_receipt: receipt_binding(
                "civsim.charged-profile.chaos-protocol.v1",
                evidence.chaos_protocol_receipt_sha256,
            ),
            gap_law_receipt: receipt_binding(
                "civsim.charged-profile.gap-law.v1",
                evidence.gap_law_receipt_sha256,
            ),
            buckingham_pi_receipt: receipt_binding(
                "civsim.charged-profile.buckingham-pi.v1",
                evidence.buckingham_pi_receipt_sha256,
            ),
            derivation_exhaustion_receipt: receipt_binding(
                "civsim.charged-profile.derive-first-exhaustion.v1",
                evidence.derivation_exhaustion_receipt_sha256,
            ),
        })),
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
        route: AdmissionRoute::Derived(super::super::model::DerivedAdmission {
            independent_watchdog_receipt: receipt_binding(
                "civsim.charged-profile.derived-reverse-semantic.v1",
                hash_fields(
                    b"civsim.charged-profile.derived-reverse-semantic.v1",
                    [artifact_identity.0.as_slice(), input_sha256.as_slice()],
                ),
            ),
            semantic_checker_receipt: receipt_binding(
                "civsim.charged-profile.derived-forward-semantic.v1",
                hash_fields(
                    b"civsim.charged-profile.derived-forward-semantic.v1",
                    [input_sha256.as_slice(), artifact_identity.0.as_slice()],
                ),
            ),
            ancestry_receipt: receipt_binding(
                "civsim.charged-profile.derived-ancestry.v1",
                hash_fields(
                    b"civsim.charged-profile.derived-ancestry.v1",
                    [
                        profile_root_identity.0.as_slice(),
                        artifact_identity.0.as_slice(),
                    ],
                ),
            ),
        }),
    }
}

fn charge_receipt(
    domain: &[u8],
    packet: &ChargedProfilePacket,
    profile_root_identity: ArtifactIdentity,
    state_negative: ArtifactIdentity,
    state_positive: ArtifactIdentity,
) -> [u8; 32] {
    let negative = (-1_i8).to_be_bytes();
    let positive = 1_i8.to_be_bytes();
    hash_fields(
        domain,
        [
            profile_root_identity.0.as_slice(),
            state_negative.0.as_slice(),
            state_positive.0.as_slice(),
            negative.as_slice(),
            positive.as_slice(),
            packet.mass_scalar_identity.0.as_slice(),
            packet.primitive_sector_identity.0.as_slice(),
            packet.conjugation_id.as_bytes(),
        ],
    )
}

fn mass_receipt(
    domain: &[u8],
    packet: &ChargedProfilePacket,
    profile_root_identity: ArtifactIdentity,
) -> [u8; 32] {
    hash_fields(
        domain,
        [
            profile_root_identity.0.as_slice(),
            packet.mass_scalar_identity.0.as_slice(),
            packet.mass_scalar_ancestry_sha256.as_slice(),
            packet.neutral_mass_projection_identity.0.as_slice(),
            packet.neutral_mass_projection_ancestry_sha256.as_slice(),
            packet.root_pair_receipt.digest_sha256.as_slice(),
        ],
    )
}

pub(super) fn canary_evidence(
    packet: &ChargedProfilePacket,
) -> Result<CanaryEvidence, ChargedProfileRefusal> {
    let sealed = sealed_packet()?;
    let cases = reversed_mutations(packet);
    let mut transcript = WATCHDOG_CANARY_ID.as_bytes().to_vec();
    for (name, mutant) in cases.iter().rev() {
        let refusal = inspect_packet_against(mutant, &sealed)
            .expect_err("every charged-profile watchdog canary must refuse")
            .id();
        write_field(&mut transcript, 1, name.as_bytes());
        write_field(&mut transcript, 2, &sha256(&write_packet(mutant)));
        write_field(&mut transcript, 3, refusal.as_bytes());
    }
    Ok(CanaryEvidence {
        transcript_id: WATCHDOG_CANARY_ID,
        case_count: u32::try_from(cases.len()).map_err(|_| ChargedProfileRefusal::CanaryFailure)?,
        transcript_sha256: sha256(&transcript),
    })
}

fn reversed_mutations(packet: &ChargedProfilePacket) -> Vec<(&'static str, ChargedProfilePacket)> {
    let mut cases = Vec::new();
    let mut changed = packet.clone();
    changed.membership_authority = true;
    cases.push(("membership_authority", changed));
    let mut changed = packet.clone();
    changed.global_physical_vocabulary_coverage = true;
    cases.push(("global_coverage", changed));
    let mut changed = packet.clone();
    changed.primitive_member.0[0] ^= 1;
    cases.push(("primitive_member", changed));
    let mut changed = packet.clone();
    changed.primitive_sector_identity.0[0] ^= 1;
    cases.push(("primitive_sector", changed));
    let mut changed = packet.clone();
    changed.primitive_profile_root_identity.0[0] ^= 1;
    cases.push(("primitive_root", changed));
    let mut changed = packet.clone();
    changed.primitive_profile_receipt.digest_sha256[0] ^= 1;
    cases.push(("primitive_receipt", changed));
    let mut changed = packet.clone();
    changed.coupling_scalar_identity.0[0] ^= 1;
    cases.push(("coupling_scalar", changed));
    let mut changed = packet.clone();
    changed.neutral_mass_projection_identity.0[0] ^= 1;
    cases.push(("neutral_mass_projection", changed));
    let mut changed = packet.clone();
    changed.mass_scalar_ancestry_sha256[0] ^= 1;
    cases.push(("mass_ancestry", changed));
    let mut changed = packet.clone();
    changed.mass_scalar_identity.0[0] ^= 1;
    cases.push(("mass_scalar", changed));
    let mut changed = packet.clone();
    changed.root_pair_receipt.digest_sha256[0] ^= 1;
    cases.push(("root_pair", changed));
    let mut changed = packet.clone();
    changed.floor_authority.digest_sha256[0] ^= 1;
    cases.push(("floor_binding", changed));
    let mut changed = packet.clone();
    changed.charge_weights = vec![-1, -1];
    cases.push(("charge_pair", changed));
    macro_rules! changed_string {
        ($name:literal, $field:ident) => {{
            let mut changed = packet.clone();
            changed.$field.push_str(".changed");
            cases.push(($name, changed));
        }};
    }
    changed_string!("evidence_hash", evidence_full_sha256_hex);
    changed_string!("owner_record", owner_admission_record);
    changed_string!("residual_slot", residual_slot_id);
    changed_string!("member_class", member_class_id);
    changed_string!("conjugation", conjugation_id);
    changed_string!("transition", transition_id);
    changed_string!("stability", stability_id);
    changed_string!("mobility", mobility_id);
    changed_string!("statistics", statistics_id);
    changed_string!("spin", spin_id);
    changed_string!("validity", validity_id);
    changed_string!("state_pair", state_pair_class_id);
    changed_string!("operator", operator_id);
    changed_string!("field", field_id);
    changed_string!("theory_class", theory_class_id);
    changed_string!("profile_id", profile_id);
    changed_string!("packet_schema", schema_id);
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
    write_field(&mut bytes, 1, &output.input_sha256);
    write_field(&mut bytes, 2, &producer_result_sha256);
    write_field(&mut bytes, 3, &watchdog_result_sha256);
    write_field(&mut bytes, 4, producer_canary.transcript_id.as_bytes());
    write_field(&mut bytes, 5, &producer_canary.transcript_sha256);
    write_field(&mut bytes, 6, watchdog_canary.transcript_id.as_bytes());
    write_field(&mut bytes, 7, &watchdog_canary.transcript_sha256);
    write_field(&mut bytes, 8, &output.profile_root_identity.0);
    write_field(&mut bytes, 9, &output.profile_role_identity.0);
    for member in &output.members {
        write_field(&mut bytes, 10, &member.0);
    }
    write_field(&mut bytes, 11, &output.evidence_custody_receipt_sha256);
    write_field(&mut bytes, 12, &output.charge_conjugation_producer_sha256);
    write_field(&mut bytes, 13, &output.charge_conjugation_watchdog_sha256);
    write_field(&mut bytes, 14, &output.mass_transport_producer_sha256);
    write_field(&mut bytes, 15, &output.mass_transport_watchdog_sha256);
    write_field(
        &mut bytes,
        16,
        &output
            .admission_evidence
            .irreducible_protocol_capability_sha256,
    );
    sha256(&bytes)
}

fn write_packet(packet: &ChargedProfilePacket) -> Vec<u8> {
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
        write_field(
            &mut bytes,
            u16::try_from(index + 1).unwrap_or(u16::MAX),
            value,
        );
    }
    let mut weights = packet.charge_weights.clone();
    weights.sort_unstable();
    for weight in weights {
        write_field(&mut bytes, 22, &weight.to_be_bytes());
    }
    write_field(&mut bytes, 23, packet.floor_authority.schema_id.as_bytes());
    write_field(&mut bytes, 24, &packet.floor_authority.digest_sha256);
    write_field(
        &mut bytes,
        25,
        packet.root_pair_receipt.schema_id.as_bytes(),
    );
    write_field(&mut bytes, 26, &packet.root_pair_receipt.digest_sha256);
    write_field(&mut bytes, 27, packet.mass_source_entry_id.as_bytes());
    write_field(&mut bytes, 28, &packet.mass_scalar_identity.0);
    write_field(&mut bytes, 29, &packet.mass_scalar_ancestry_sha256);
    write_field(&mut bytes, 30, &packet.neutral_mass_projection_identity.0);
    write_field(
        &mut bytes,
        31,
        &packet.neutral_mass_projection_ancestry_sha256,
    );
    write_field(&mut bytes, 32, packet.coupling_source_entry_id.as_bytes());
    write_field(&mut bytes, 33, &packet.coupling_scalar_identity.0);
    write_field(&mut bytes, 34, &packet.coupling_scalar_ancestry_sha256);
    write_field(
        &mut bytes,
        35,
        packet.primitive_profile_receipt.schema_id.as_bytes(),
    );
    write_field(
        &mut bytes,
        36,
        &packet.primitive_profile_receipt.digest_sha256,
    );
    write_field(&mut bytes, 37, &packet.primitive_profile_root_identity.0);
    write_field(&mut bytes, 38, &packet.primitive_sector_identity.0);
    write_field(&mut bytes, 39, &packet.primitive_member.0);
    write_field(&mut bytes, 40, packet.primitive_residual_slot_id.as_bytes());
    write_field(
        &mut bytes,
        41,
        &[u8::from(packet.global_physical_vocabulary_coverage)],
    );
    write_field(&mut bytes, 42, &[u8::from(packet.membership_authority)]);
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
    write_field(&mut bytes, 3, &reconstruction.profile_role_identity.0);
    for candidate in &reconstruction.candidates {
        let mut artifact = candidate.identity.0.to_vec();
        artifact.extend_from_slice(&write_admission(&candidate.admission));
        write_field(&mut bytes, 4, &artifact);
    }
    for member in &reconstruction.members {
        write_field(&mut bytes, 5, &member.0);
    }
    write_field(&mut bytes, 6, &evidence_custody_receipt_sha256);
    write_field(
        &mut bytes,
        7,
        &reconstruction.charge_conjugation_producer_sha256,
    );
    write_field(
        &mut bytes,
        8,
        &reconstruction.charge_conjugation_watchdog_sha256,
    );
    write_field(
        &mut bytes,
        9,
        &reconstruction.mass_transport_producer_sha256,
    );
    write_field(
        &mut bytes,
        10,
        &reconstruction.mass_transport_watchdog_sha256,
    );
    write_field(&mut bytes, 11, &reconstruction.applicability_receipt_sha256);
    write_field(&mut bytes, 12, &reconstruction.validity_receipt_sha256);
    write_field(
        &mut bytes,
        13,
        &reconstruction
            .admission_evidence
            .irreducible_protocol_capability_sha256,
    );
    bytes
}

fn write_admission(admission: &RootAdmission) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_field(&mut bytes, 1, admission.tier.id().as_bytes());
    write_field(
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
            write_field(&mut bytes, 3, b"derived");
            write_field(&mut bytes, 4, &route.ancestry_receipt.digest_sha256);
            write_field(&mut bytes, 5, &route.semantic_checker_receipt.digest_sha256);
            write_field(
                &mut bytes,
                6,
                &route.independent_watchdog_receipt.digest_sha256,
            );
        }
        AdmissionRoute::Irreducible(route) => {
            write_field(&mut bytes, 3, b"irreducible");
            write_field(
                &mut bytes,
                4,
                &route.derivation_exhaustion_receipt.digest_sha256,
            );
            write_field(&mut bytes, 5, &route.buckingham_pi_receipt.digest_sha256);
            write_field(&mut bytes, 6, &route.gap_law_receipt.digest_sha256);
            write_field(&mut bytes, 7, &route.chaos_protocol_receipt.digest_sha256);
            write_field(&mut bytes, 8, &route.residual_law_receipt.digest_sha256);
            write_field(&mut bytes, 9, route.residual_slot_id.as_bytes());
            write_field(&mut bytes, 10, &route.residual_slot_receipt.digest_sha256);
            write_field(&mut bytes, 11, &route.owner_admission_receipt.digest_sha256);
            write_field(
                &mut bytes,
                12,
                &route.independent_watchdog_receipt.digest_sha256,
            );
        }
        AdmissionRoute::EvidenceCustodyOnly { source_receipt } => {
            write_field(&mut bytes, 3, b"evidence-custody-only");
            write_field(&mut bytes, 4, &source_receipt.digest_sha256);
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
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(payload.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
}

fn to_hex(bytes: [u8; 32]) -> String {
    let mut text = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(text, "{byte:02x}");
    }
    text
}
