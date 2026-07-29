//! Reverse-map and completed-square watchdog for the neutral bound profile.

use super::*;
use crate::canonical::stellar_birth_species::law_premise;
use civsim_ledger::{Provenance, Tier};
use civsim_units::{
    bignum::BigRat,
    digest::sha256,
    physics_floor::{sealed_physical_floor_authority_binding, sealed_physical_floor_definitions},
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

const PROFILE_CLAIM_DOMAIN: &[u8] = b"civsim.planet.neutral-bound-profile.claim.v1";
const EVIDENCE_DOMAIN: &[u8] = b"civsim.planet.neutral-bound-profile.evidence-custody.v1";
const SOLVER_PRODUCER_DOMAIN: &[u8] = b"civsim.planet.neutral-bound-profile.variational-solver.v1";
const SOLVER_WATCHDOG_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.factorization-solver.v1";
const NORMALIZATION_PRODUCER_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.gamma-normalization.v1";
const NORMALIZATION_WATCHDOG_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.moment-normalization.v1";
const THRESHOLD_PRODUCER_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.threshold-forward.v1";
const THRESHOLD_WATCHDOG_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.threshold-reverse.v1";
const UNCERTAINTY_PRODUCER_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.uncertainty-monotone.v1";
const UNCERTAINTY_WATCHDOG_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.uncertainty-corners.v1";
const CONSERVATION_PRODUCER_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.conservation-forward.v1";
const CONSERVATION_WATCHDOG_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.conservation-reverse.v1";
const APPLICABILITY_DOMAIN: &[u8] = b"civsim.planet.neutral-bound-profile.applicability.v1";
const VALIDITY_DOMAIN: &[u8] = b"civsim.planet.neutral-bound-profile.validity.v1";
const CONSTITUENT_CHANNEL_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.free-constituent-channel.v1";
const DECAY_CHANNEL_DOMAIN: &[u8] =
    b"civsim.planet.neutral-bound-profile.neutral-carrier-decay-family.v1";
const PAIR_DOMAIN: &[u8] = b"civsim.planet.neutral-bound-profile.pair-receipt.v1";

#[derive(Clone)]
struct ReconstructedMath {
    candidate_mass_interval: ExactInterval,
    constituent_threshold_interval: ExactInterval,
    decay_threshold_interval: ExactInterval,
    dimensionless_ground_energy: BigRat,
    reduced_mass_factor: BigRat,
    binding_mass_factor: BigRat,
    normalization_value: BigRat,
    radial_residual_norm: BigRat,
    constituent_threshold_channel_identity: [u8; 32],
    decay_channel_family_identity: [u8; 32],
}

pub(super) fn sealed_packet() -> Result<NeutralBoundProfilePacket, NeutralBoundProfileRefusal> {
    let charged = super::super::charged_profile::project_admitted_profile()
        .map_err(|_| NeutralBoundProfileRefusal::SealedSourceUnavailable)?;
    let primitive = super::super::primitive_profile::project_admitted_profile()
        .map_err(|_| NeutralBoundProfileRefusal::SealedSourceUnavailable)?;
    let roots = super::super::repository_roots::project_repository_roots()
        .map_err(|_| NeutralBoundProfileRefusal::SealedSourceUnavailable)?;
    let bindings = super::super::repository_roots::coordinate_bindings(&roots)
        .map_err(|_| NeutralBoundProfileRefusal::SealedSourceUnavailable)?;
    let coupling = bindings
        .iter()
        .rev()
        .find(|binding| binding.entry_id.as_bytes() == COUPLING_SOURCE_ENTRY_ID.as_bytes())
        .ok_or(NeutralBoundProfileRefusal::SealedSourceUnavailable)?;
    let mass = bindings
        .iter()
        .rev()
        .find(|binding| binding.entry_id.as_bytes() == MASS_SOURCE_ENTRY_ID.as_bytes())
        .ok_or(NeutralBoundProfileRefusal::SealedSourceUnavailable)?;
    if coupling.root_pair_receipt != mass.root_pair_receipt {
        return Err(NeutralBoundProfileRefusal::RootBindingMismatch);
    }
    let definitions = sealed_physical_floor_definitions()
        .map_err(|_| NeutralBoundProfileRefusal::SealedSourceUnavailable)?;
    let coupling_definition = definitions
        .iter()
        .rev()
        .find(|definition| definition.symbol.as_bytes() == b"alpha")
        .ok_or(NeutralBoundProfileRefusal::SealedSourceUnavailable)?;
    let mass_definition = definitions
        .iter()
        .rev()
        .find(|definition| definition.symbol.as_bytes() == b"m_e")
        .ok_or(NeutralBoundProfileRefusal::SealedSourceUnavailable)?;
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| NeutralBoundProfileRefusal::SealedSourceUnavailable)?;
    Ok(NeutralBoundProfilePacket {
        membership_authority: false,
        global_physical_vocabulary_coverage: false,
        charged_residual_slot_id: charged.receipt.residual_slot_id.to_owned(),
        charged_members: charged.members,
        charged_profile_root_identity: charged.receipt.profile_root_identity,
        charged_profile_receipt: receipt_binding(
            charged.receipt.schema_id,
            charged.receipt.pair_receipt_sha256,
        ),
        primitive_residual_slot_id: primitive.receipt.residual_slot_id.to_owned(),
        primitive_member: primitive.member,
        primitive_sector_identity: primitive.sector_identity,
        primitive_profile_root_identity: primitive.receipt.profile_root_identity,
        primitive_profile_receipt: receipt_binding(
            primitive.receipt.schema_id,
            primitive.receipt.pair_receipt_sha256,
        ),
        coupling_uncertainty_decimal: coupling_definition.uncertainty.decimal().to_owned(),
        coupling_value_decimal: coupling_definition.value.to_owned(),
        coupling_scalar_ancestry_sha256: coupling.scalar_ancestry_sha256,
        coupling_scalar_identity: coupling.scalar_identity,
        coupling_source_entry_id: coupling.entry_id.clone(),
        mass_uncertainty_decimal: mass_definition.uncertainty.decimal().to_owned(),
        mass_value_decimal: mass_definition.value.to_owned(),
        mass_scalar_ancestry_sha256: mass.scalar_ancestry_sha256,
        mass_scalar_identity: mass.scalar_identity,
        mass_source_entry_id: mass.entry_id.clone(),
        root_pair_receipt: mass.root_pair_receipt.clone(),
        floor_authority: receipt_binding(floor.schema_id().as_str(), floor.digest()),
        evidence_secondary_anchor: EVIDENCE_SECONDARY_ANCHOR.to_owned(),
        evidence_secondary_sha256_hex: EVIDENCE_SECONDARY_SHA256_HEX.to_owned(),
        evidence_secondary_url: EVIDENCE_SECONDARY_URL.to_owned(),
        evidence_secondary_citation: EVIDENCE_SECONDARY_CITATION.to_owned(),
        evidence_primary_anchor: EVIDENCE_PRIMARY_ANCHOR.to_owned(),
        evidence_primary_sha256_hex: EVIDENCE_PRIMARY_SHA256_HEX.to_owned(),
        evidence_primary_url: EVIDENCE_PRIMARY_URL.to_owned(),
        evidence_primary_citation: EVIDENCE_PRIMARY_CITATION.to_owned(),
        owner_admission_record: OWNER_ADMISSION_RECORD.to_owned(),
        residual_slot_id: RESIDUAL_SLOT_ID.to_owned(),
        member_class_id: MEMBER_CLASS_ID.to_owned(),
        transition_id: TRANSITION_ID.to_owned(),
        stability_id: STABILITY_ID.to_owned(),
        decay_id: DECAY_ID.to_owned(),
        separation_id: SEPARATION_ID.to_owned(),
        conservation_id: CONSERVATION_ID.to_owned(),
        convergence_id: CONVERGENCE_ID.to_owned(),
        normalization_id: NORMALIZATION_ID.to_owned(),
        validity_id: VALIDITY_ID.to_owned(),
        state_id: STATE_ID.to_owned(),
        interaction_id: INTERACTION_ID.to_owned(),
        theory_class_id: THEORY_CLASS_ID.to_owned(),
        profile_id: PROFILE_ID.to_owned(),
        schema_id: PACKET_SCHEMA_ID.to_owned(),
    })
}

pub(super) fn inspect(
    packet: &NeutralBoundProfilePacket,
) -> Result<NeutralBoundCheckerOutput, NeutralBoundProfileRefusal> {
    let sealed = sealed_packet()?;
    inspect_against_sealed(packet, &sealed)
}

pub(super) fn inspect_against_sealed(
    packet: &NeutralBoundProfilePacket,
    sealed: &NeutralBoundProfilePacket,
) -> Result<NeutralBoundCheckerOutput, NeutralBoundProfileRefusal> {
    inspect_packet_against(packet, sealed)?;
    let packet_bytes = encode_packet(packet)?;
    let input_sha256 = sha256(&packet_bytes);
    let evidence_custody_receipt_sha256 = reconstruct_evidence_digest(packet);
    let math = solve_by_factorization_and_corners(packet)?;
    let mut output =
        reconstruct_candidates(packet, input_sha256, evidence_custody_receipt_sha256, math)?;
    output.canonical_bytes = encode_checker_output(&output)?;
    Ok(output)
}

fn inspect_packet_against(
    packet: &NeutralBoundProfilePacket,
    sealed: &NeutralBoundProfilePacket,
) -> Result<(), NeutralBoundProfileRefusal> {
    if packet.membership_authority || packet.global_physical_vocabulary_coverage {
        return Err(NeutralBoundProfileRefusal::UnsupportedScope);
    }
    let mut slots = BTreeMap::new();
    for slot in [
        packet.residual_slot_id.as_str(),
        packet.charged_residual_slot_id.as_str(),
        packet.primitive_residual_slot_id.as_str(),
    ]
    .into_iter()
    .rev()
    {
        if slots.insert(slot, ()).is_some() {
            return Err(NeutralBoundProfileRefusal::ResidualSlotCollision);
        }
    }
    let mut member_map = BTreeMap::new();
    for member in packet.charged_members.iter().rev() {
        if member_map.insert(*member, ()).is_some() {
            return Err(NeutralBoundProfileRefusal::ChargedProfileBindingMismatch);
        }
    }
    if packet.charged_members != sealed.charged_members
        || member_map.keys().copied().collect::<Vec<_>>() != sealed.charged_members
        || packet.charged_profile_receipt != sealed.charged_profile_receipt
        || packet.charged_profile_root_identity != sealed.charged_profile_root_identity
        || packet.charged_residual_slot_id != sealed.charged_residual_slot_id
    {
        return Err(NeutralBoundProfileRefusal::ChargedProfileBindingMismatch);
    }
    if packet.primitive_profile_receipt != sealed.primitive_profile_receipt
        || packet.primitive_profile_root_identity != sealed.primitive_profile_root_identity
        || packet.primitive_sector_identity != sealed.primitive_sector_identity
        || packet.primitive_member != sealed.primitive_member
        || packet.primitive_residual_slot_id != sealed.primitive_residual_slot_id
    {
        return Err(NeutralBoundProfileRefusal::PrimitiveProfileBindingMismatch);
    }
    let root_observed = (
        packet.coupling_uncertainty_decimal.as_str(),
        packet.coupling_value_decimal.as_str(),
        packet.coupling_scalar_ancestry_sha256,
        packet.coupling_scalar_identity,
        packet.coupling_source_entry_id.as_str(),
        packet.mass_uncertainty_decimal.as_str(),
        packet.mass_value_decimal.as_str(),
        packet.mass_scalar_ancestry_sha256,
        packet.mass_scalar_identity,
        packet.mass_source_entry_id.as_str(),
        &packet.root_pair_receipt,
    );
    let root_expected = (
        sealed.coupling_uncertainty_decimal.as_str(),
        sealed.coupling_value_decimal.as_str(),
        sealed.coupling_scalar_ancestry_sha256,
        sealed.coupling_scalar_identity,
        sealed.coupling_source_entry_id.as_str(),
        sealed.mass_uncertainty_decimal.as_str(),
        sealed.mass_value_decimal.as_str(),
        sealed.mass_scalar_ancestry_sha256,
        sealed.mass_scalar_identity,
        sealed.mass_source_entry_id.as_str(),
        &sealed.root_pair_receipt,
    );
    if root_observed != root_expected || packet.root_pair_receipt.digest_sha256 == [0; 32] {
        return Err(NeutralBoundProfileRefusal::RootBindingMismatch);
    }
    if packet.floor_authority != sealed.floor_authority
        || packet.floor_authority.digest_sha256 == [0; 32]
    {
        return Err(NeutralBoundProfileRefusal::FloorBindingMismatch);
    }
    let evidence_observed = (
        packet.evidence_secondary_anchor.as_str(),
        packet.evidence_secondary_sha256_hex.as_str(),
        packet.evidence_secondary_url.as_str(),
        packet.evidence_secondary_citation.as_str(),
        packet.evidence_primary_anchor.as_str(),
        packet.evidence_primary_sha256_hex.as_str(),
        packet.evidence_primary_url.as_str(),
        packet.evidence_primary_citation.as_str(),
    );
    let evidence_expected = (
        EVIDENCE_SECONDARY_ANCHOR,
        EVIDENCE_SECONDARY_SHA256_HEX,
        EVIDENCE_SECONDARY_URL,
        EVIDENCE_SECONDARY_CITATION,
        EVIDENCE_PRIMARY_ANCHOR,
        EVIDENCE_PRIMARY_SHA256_HEX,
        EVIDENCE_PRIMARY_URL,
        EVIDENCE_PRIMARY_CITATION,
    );
    if evidence_observed != evidence_expected {
        return Err(NeutralBoundProfileRefusal::EvidenceCustodyMismatch);
    }
    let profile_fields = [
        (
            packet.owner_admission_record.as_str(),
            OWNER_ADMISSION_RECORD,
        ),
        (packet.residual_slot_id.as_str(), RESIDUAL_SLOT_ID),
        (packet.member_class_id.as_str(), MEMBER_CLASS_ID),
        (packet.transition_id.as_str(), TRANSITION_ID),
        (packet.stability_id.as_str(), STABILITY_ID),
        (packet.decay_id.as_str(), DECAY_ID),
        (packet.separation_id.as_str(), SEPARATION_ID),
        (packet.conservation_id.as_str(), CONSERVATION_ID),
        (packet.convergence_id.as_str(), CONVERGENCE_ID),
        (packet.normalization_id.as_str(), NORMALIZATION_ID),
        (packet.validity_id.as_str(), VALIDITY_ID),
        (packet.state_id.as_str(), STATE_ID),
        (packet.interaction_id.as_str(), INTERACTION_ID),
        (packet.theory_class_id.as_str(), THEORY_CLASS_ID),
        (packet.profile_id.as_str(), PROFILE_ID),
    ];
    if profile_fields
        .iter()
        .any(|(observed, expected)| observed != expected)
    {
        return Err(NeutralBoundProfileRefusal::ProfileIdentityMismatch);
    }
    if packet.schema_id.as_bytes() != PACKET_SCHEMA_ID.as_bytes() {
        return Err(NeutralBoundProfileRefusal::PacketSchemaMismatch);
    }
    Ok(())
}

fn solve_by_factorization_and_corners(
    packet: &NeutralBoundProfilePacket,
) -> Result<ReconstructedMath, NeutralBoundProfileRefusal> {
    let zero = BigRat::from_i64(0);
    let one = BigRat::from_i64(1);
    let two = BigRat::from_i64(2);
    let four = BigRat::from_i64(4);
    let minus_half = BigRat::from_i64(-1).div(&two);

    // Complete the square independently:
    // epsilon(s) = (s - 1)^2/2 - 1/2. The remainder is nonnegative
    // everywhere and vanishes at s=1.
    let scale = one.clone();
    let square_remainder = scale.sub(&one).mul(&scale.sub(&one)).div(&two);
    if square_remainder.cmp_rat(&zero) != Ordering::Equal {
        return Err(NeutralBoundProfileRefusal::SolverEvidenceMismatch);
    }
    let dimensionless_ground_energy = square_remainder.add(&minus_half);
    let reduced_mass_factor = one.div(&one.add(&one));
    let binding_mass_factor = dimensionless_ground_energy.abs().mul(&reduced_mass_factor);
    if dimensionless_ground_energy.cmp_rat(&minus_half) != Ordering::Equal
        || reduced_mass_factor.cmp_rat(&one.div(&two)) != Ordering::Equal
        || binding_mass_factor.cmp_rat(&one.div(&four)) != Ordering::Equal
    {
        return Err(NeutralBoundProfileRefusal::SolverEvidenceMismatch);
    }

    // Reconstruct the second exponential moment by recurrence:
    // I_0=1/2, I_1=(1/2)I_0, I_2=(2/2)I_1.
    let moment_zero = one.div(&two);
    let moment_one = moment_zero.div(&two);
    let moment_two = moment_one.mul(&two).div(&two);
    let normalization_value = four.mul(&moment_two);
    let radial_residual_norm = square_remainder.mul(&two);
    if normalization_value.cmp_rat(&one) != Ordering::Equal
        || radial_residual_norm.cmp_rat(&zero) != Ordering::Equal
    {
        return Err(NeutralBoundProfileRefusal::NormalizationMismatch);
    }
    let charge_orbit = [BigRat::from_i64(1), BigRat::from_i64(-1)];
    if charge_orbit
        .iter()
        .rev()
        .fold(BigRat::from_i64(0), |sum, charge| sum.add(charge))
        .cmp_rat(&zero)
        != Ordering::Equal
    {
        return Err(NeutralBoundProfileRefusal::ConservationMismatch);
    }

    let mass = parse_positive_from_right(&packet.mass_value_decimal)?;
    let mass_uncertainty = parse_nonnegative_from_right(&packet.mass_uncertainty_decimal)?;
    let coupling = parse_positive_from_right(&packet.coupling_value_decimal)?;
    let coupling_uncertainty = parse_nonnegative_from_right(&packet.coupling_uncertainty_decimal)?;
    let mass_bounds = [mass.sub(&mass_uncertainty), mass.add(&mass_uncertainty)];
    let coupling_bounds = [
        coupling.sub(&coupling_uncertainty),
        coupling.add(&coupling_uncertainty),
    ];
    if mass_bounds[0].cmp_rat(&zero) != Ordering::Greater
        || coupling_bounds[0].cmp_rat(&zero) != Ordering::Greater
        || coupling_bounds[1].cmp_rat(&one) != Ordering::Less
    {
        return Err(NeutralBoundProfileRefusal::InvalidValidityDomain);
    }
    let mut candidate_corners = Vec::with_capacity(4);
    for coupling_corner in coupling_bounds.iter().rev() {
        for mass_corner in mass_bounds.iter().rev() {
            candidate_corners
                .push(mass_corner.mul(&two.sub(&coupling_corner.mul(coupling_corner).div(&four))));
        }
    }
    let candidate_mass_interval = ExactInterval {
        lower: exact_min(&candidate_corners)?,
        upper: exact_max(&candidate_corners)?,
    };
    let threshold_corners = [mass_bounds[1].mul(&two), mass_bounds[0].mul(&two)];
    let constituent_threshold_interval = ExactInterval {
        lower: exact_min(&threshold_corners)?,
        upper: exact_max(&threshold_corners)?,
    };
    if candidate_mass_interval
        .upper
        .cmp_rat(&constituent_threshold_interval.lower)
        != Ordering::Less
    {
        return Err(NeutralBoundProfileRefusal::UncertaintyTransportMismatch);
    }
    let decay_threshold_interval = ExactInterval {
        upper: zero.clone(),
        lower: zero,
    };
    let constituent_threshold_channel_identity = digest_fields(
        CONSTITUENT_CHANNEL_DOMAIN,
        &[
            &packet.charged_members[0].0,
            &packet.charged_members[1].0,
            &packet.charged_profile_receipt.digest_sha256,
        ],
    );
    let decay_channel_family_identity = digest_fields(
        DECAY_CHANNEL_DOMAIN,
        &[
            &packet.primitive_member.0,
            &packet.primitive_sector_identity.0,
            &packet.primitive_profile_receipt.digest_sha256,
        ],
    );
    Ok(ReconstructedMath {
        candidate_mass_interval,
        constituent_threshold_interval,
        decay_threshold_interval,
        dimensionless_ground_energy,
        reduced_mass_factor,
        binding_mass_factor,
        normalization_value,
        radial_residual_norm,
        constituent_threshold_channel_identity,
        decay_channel_family_identity,
    })
}

fn reconstruct_candidates(
    packet: &NeutralBoundProfilePacket,
    input_sha256: [u8; 32],
    evidence_custody_receipt_sha256: [u8; 32],
    math: ReconstructedMath,
) -> Result<NeutralBoundCheckerOutput, NeutralBoundProfileRefusal> {
    let profile_payload = descriptor(
        PROFILE_SCHEMA_ID,
        &format!(
            "profile_id={}\ntheory_class={}\nevidence_custody_sha256={}\ncharged_profile_root={}\nprimitive_sector={}\nmass_coordinate={}\ncoupling_coordinate={}\nauthority_scope=claim-local\n",
            packet.profile_id,
            packet.theory_class_id,
            hex(evidence_custody_receipt_sha256),
            hex(packet.charged_profile_root_identity.0),
            hex(packet.primitive_sector_identity.0),
            hex(packet.mass_scalar_identity.0),
            hex(packet.coupling_scalar_identity.0),
        ),
    );
    let profile_root_identity = reverse_identity(&profile_payload)?;
    let semantic = reconstruct_semantic_receipts(packet, profile_root_identity, &math)?;
    let specs = vec![
        (
            "profile_role",
            "civsim.neutral-bound-profile.role.profile-root.v1",
            "admitted-neutral-bound-profile-role".to_owned(),
        ),
        (
            "derivation_kind",
            "civsim.neutral-bound-profile.derivation-kind.v1",
            "neutral-two-body-level-from-admitted-central-interaction".to_owned(),
        ),
        (
            "interaction_role",
            "civsim.neutral-bound-profile.role.interaction.v1",
            "leading-central-interaction-requirement-role".to_owned(),
        ),
        (
            "interaction",
            "civsim.neutral-bound-profile.interaction.v1",
            format!(
                "{}\nsolver_forward_sha256={}\nsolver_reverse_sha256={}",
                packet.interaction_id,
                hex(semantic.solver_producer),
                hex(semantic.solver_watchdog),
            ),
        ),
        (
            "state_role",
            "civsim.neutral-bound-profile.role.state.v1",
            "normalized-bound-level-state-role".to_owned(),
        ),
        (
            "state",
            "civsim.neutral-bound-profile.state.v1",
            packet.state_id.clone(),
        ),
        (
            "validity_role",
            "civsim.neutral-bound-profile.role.validity.v1",
            "claim-local-validity-role".to_owned(),
        ),
        (
            "validity",
            "civsim.neutral-bound-profile.validity.v1",
            format!(
                "{}\nvalidity_receipt_sha256={}",
                packet.validity_id,
                hex(semantic.validity),
            ),
        ),
        (
            "normalization_role",
            "civsim.neutral-bound-profile.role.normalization.v1",
            "exact-normalization-requirement-role".to_owned(),
        ),
        (
            "normalization",
            "civsim.neutral-bound-profile.normalization.v1",
            format!(
                "{}\nforward_sha256={}\nreverse_sha256={}",
                packet.normalization_id,
                hex(semantic.normalization_producer),
                hex(semantic.normalization_watchdog),
            ),
        ),
        (
            "convergence_role",
            "civsim.neutral-bound-profile.role.convergence.v1",
            "exact-residual-convergence-requirement-role".to_owned(),
        ),
        (
            "convergence",
            "civsim.neutral-bound-profile.convergence.v1",
            packet.convergence_id.clone(),
        ),
        (
            "conservation_role",
            "civsim.neutral-bound-profile.role.conservation.v1",
            "net-relative-charge-conservation-role".to_owned(),
        ),
        (
            "conservation",
            "civsim.neutral-bound-profile.conservation.v1",
            format!(
                "{}\nforward_sha256={}\nreverse_sha256={}",
                packet.conservation_id,
                hex(semantic.conservation_producer),
                hex(semantic.conservation_watchdog),
            ),
        ),
        (
            "separation_role",
            "civsim.neutral-bound-profile.role.separation.v1",
            "free-constituent-separation-threshold-role".to_owned(),
        ),
        (
            "separation",
            "civsim.neutral-bound-profile.separation.v1",
            format!(
                "{}\nthreshold_binding_sha256={}",
                packet.separation_id,
                hex(semantic.threshold_binding),
            ),
        ),
        (
            "decay_role",
            "civsim.neutral-bound-profile.role.decay.v1",
            "energetically-open-decay-family-role".to_owned(),
        ),
        (
            "decay",
            "civsim.neutral-bound-profile.decay.v1",
            packet.decay_id.clone(),
        ),
        (
            "stability_role",
            "civsim.neutral-bound-profile.role.stability.v1",
            "bounded-constituent-not-global-stability-role".to_owned(),
        ),
        (
            "stability",
            "civsim.neutral-bound-profile.stability.v1",
            packet.stability_id.clone(),
        ),
        (
            "transition_role",
            "civsim.neutral-bound-profile.role.transition.v1",
            "open-transition-family-without-rate-role".to_owned(),
        ),
        (
            "transition",
            "civsim.neutral-bound-profile.transition.v1",
            packet.transition_id.clone(),
        ),
        (
            "mass_source_role",
            "civsim.neutral-bound-profile.role.mass-source.v1",
            "transported-rest-mass-coordinate-role".to_owned(),
        ),
        (
            "coupling_source_role",
            "civsim.neutral-bound-profile.role.coupling-source.v1",
            "transported-dimensionless-coupling-coordinate-role".to_owned(),
        ),
        (
            "constituent_channel_role",
            "civsim.neutral-bound-profile.role.constituent-channel.v1",
            "covered-free-constituent-channel-role".to_owned(),
        ),
        (
            "constituent_channel",
            "civsim.neutral-bound-profile.constituent-channel.v1",
            hex(math.constituent_threshold_channel_identity),
        ),
        (
            "decay_channel_role",
            "civsim.neutral-bound-profile.role.decay-channel.v1",
            "open-neutral-carrier-channel-family-role".to_owned(),
        ),
        (
            "decay_channel",
            "civsim.neutral-bound-profile.decay-channel.v1",
            hex(math.decay_channel_family_identity),
        ),
        (
            "constraint_role",
            "civsim.neutral-bound-profile.role.constraint.v1",
            "claim-local-bound-level-constraint-role".to_owned(),
        ),
    ];
    let mut candidates = Vec::with_capacity(ARTIFACT_COUNT);
    let mut identities = BTreeMap::new();
    for (key, schema, content) in specs.into_iter().rev() {
        let payload = descriptor(schema, &content);
        let identity = reverse_identity(&payload)?;
        if identities.insert(key, identity).is_some() {
            return Err(NeutralBoundProfileRefusal::ArtifactConstructionFailure);
        }
        candidates.push(NeutralBoundArtifactCandidate {
            identity,
            admission: reverse_derived_admission(input_sha256, profile_root_identity, identity),
            payload,
        });
    }
    let id = |key: &'static str| {
        identities
            .get(key)
            .copied()
            .ok_or(NeutralBoundProfileRefusal::ArtifactConstructionFailure)
    };
    let profile_role_identity = id("profile_role")?;
    let mut sources = vec![
        super::super::model::MassUncertaintySourceProof {
            source_coordinate: packet.coupling_scalar_identity,
            source_pair_receipt: packet.root_pair_receipt.clone(),
        },
        super::super::model::MassUncertaintySourceProof {
            source_coordinate: packet.mass_scalar_identity,
            source_pair_receipt: packet.root_pair_receipt.clone(),
        },
    ];
    sources.sort_by_key(|source| source.source_coordinate);
    let mass_projection_payload =
        ArtifactPayload::MassProjection(super::super::model::MassProjectionArtifact {
            expression: reverse_mass_expression(packet),
            scope: super::super::model::MassProjectionScope::SpeciesRestMass,
            uncertainty_transport: Some(super::super::model::MassUncertaintyTransportProof {
                sources,
                producer_receipt: receipt_binding(
                    "civsim.neutral-bound-profile.mass-uncertainty-forward.v1",
                    semantic.uncertainty_producer,
                ),
                watchdog_receipt: receipt_binding(
                    "civsim.neutral-bound-profile.mass-uncertainty-reverse.v1",
                    semantic.uncertainty_watchdog,
                ),
            }),
        });
    let mass_projection = reverse_push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        mass_projection_payload,
    )?;
    let requirements = super::super::model::RequirementSet {
        artifact_relations: vec![
            relation(id("interaction_role")?, id("interaction")?),
            relation(id("state_role")?, id("state")?),
            relation(id("validity_role")?, id("validity")?),
            relation(id("normalization_role")?, id("normalization")?),
            relation(id("convergence_role")?, id("convergence")?),
            relation(id("conservation_role")?, id("conservation")?),
            relation(id("separation_role")?, id("separation")?),
            relation(id("decay_role")?, id("decay")?),
            relation(id("stability_role")?, id("stability")?),
            relation(id("transition_role")?, id("transition")?),
            relation(id("mass_source_role")?, packet.mass_scalar_identity),
            relation(id("coupling_source_role")?, packet.coupling_scalar_identity),
            relation(id("constituent_channel_role")?, id("constituent_channel")?),
            relation(id("decay_channel_role")?, id("decay_channel")?),
        ],
        species_dependencies: packet.charged_members.clone(),
    };
    let constraint = reverse_push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        ArtifactPayload::ConstraintLaw(super::super::model::ConstraintLawArtifact {
            requirements: requirements.clone(),
        }),
    )?;
    let blueprint = super::super::model::MemberBlueprint {
        physical_content: super::super::model::CanonicalArtifact {
            schema_id: "civsim.neutral-bound-profile.bound-excitation.v1".to_owned(),
            canonical_bytes: packet.member_class_id.as_bytes().to_vec(),
        },
        requirements,
        mass_proof: super::super::model::MassProofReference::Projection(mass_projection),
        constraint_laws: vec![relation(id("constraint_role")?, constraint)],
    };
    let member = super::super::watchdog::derive_member_identity_for_authority(&blueprint)
        .map_err(|_| NeutralBoundProfileRefusal::MemberIdentityMismatch)?;
    reverse_push_derived(
        &mut candidates,
        profile_root_identity,
        input_sha256,
        ArtifactPayload::SpeciesDerivation(super::super::model::SpeciesDerivationArtifact {
            derivation_kind: id("derivation_kind")?,
            artifact_inputs: vec![
                relation(id("interaction_role")?, id("interaction")?),
                relation(id("state_role")?, id("state")?),
                relation(id("mass_source_role")?, packet.mass_scalar_identity),
                relation(id("coupling_source_role")?, packet.coupling_scalar_identity),
                relation(id("constituent_channel_role")?, id("constituent_channel")?),
                relation(id("decay_channel_role")?, id("decay_channel")?),
            ],
            constituents: packet.charged_members.clone(),
            output: blueprint,
        }),
    )?;

    let applicability_receipt_sha256 = digest_fields(
        APPLICABILITY_DOMAIN,
        &[
            &semantic.solver_producer,
            &semantic.solver_watchdog,
            &semantic.normalization_producer,
            &semantic.normalization_watchdog,
            &semantic.threshold_producer,
            &semantic.threshold_watchdog,
            &semantic.uncertainty_producer,
            &semantic.uncertainty_watchdog,
            &semantic.conservation_producer,
            &semantic.conservation_watchdog,
            &packet.charged_profile_receipt.digest_sha256,
            &packet.primitive_profile_receipt.digest_sha256,
        ],
    );
    if applicability_receipt_sha256 != semantic.applicability {
        return Err(NeutralBoundProfileRefusal::ArtifactConstructionFailure);
    }
    let claim_identity = digest_fields(
        PROFILE_CLAIM_DOMAIN,
        &[
            &profile_root_identity.0,
            &profile_role_identity.0,
            &input_sha256,
        ],
    );
    let mut occupied_profile_slots = vec![
        packet.charged_residual_slot_id.clone(),
        packet.primitive_residual_slot_id.clone(),
    ];
    occupied_profile_slots.sort();
    let admission_evidence =
        law_premise::inspect_theory_profile_protocol(&law_premise::TheoryProfileAdmissionRequest {
            claim_identity,
            role_identity: profile_role_identity.0,
            content_identity: profile_root_identity.0,
            profile_input_sha256: input_sha256,
            source_custody_sha256: evidence_custody_receipt_sha256,
            applicability_receipt_sha256,
            validity_receipt_sha256: semantic.validity,
            residual_slot_id: packet.residual_slot_id.clone(),
            occupied_profile_slots,
            owner_admission_record: packet.owner_admission_record.clone(),
        })
        .map_err(|_| NeutralBoundProfileRefusal::ArtifactConstructionFailure)?;
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
        return Err(NeutralBoundProfileRefusal::ArtifactConstructionFailure);
    }
    candidates.push(NeutralBoundArtifactCandidate {
        identity: profile_root_identity,
        admission: reverse_irreducible_admission(packet, &admission_evidence),
        payload: profile_payload,
    });
    candidates.sort_by_key(|candidate| candidate.identity);
    if candidates.len() != ARTIFACT_COUNT
        || candidates
            .windows(2)
            .any(|pair| pair[0].identity >= pair[1].identity)
    {
        return Err(NeutralBoundProfileRefusal::ArtifactCountMismatch);
    }
    Ok(NeutralBoundCheckerOutput {
        input_sha256,
        canonical_bytes: Vec::new(),
        candidates,
        member,
        profile_root_identity,
        profile_role_identity,
        constituent_threshold_channel_identity: math.constituent_threshold_channel_identity,
        decay_channel_family_identity: math.decay_channel_family_identity,
        candidate_mass_interval: math.candidate_mass_interval,
        constituent_threshold_interval: math.constituent_threshold_interval,
        decay_threshold_interval: math.decay_threshold_interval,
        dimensionless_ground_energy: math.dimensionless_ground_energy,
        reduced_mass_factor: math.reduced_mass_factor,
        binding_mass_factor: math.binding_mass_factor,
        normalization_value: math.normalization_value,
        radial_residual_norm: math.radial_residual_norm,
        evidence_custody_receipt_sha256,
        solver_producer_sha256: semantic.solver_producer,
        solver_watchdog_sha256: semantic.solver_watchdog,
        normalization_producer_sha256: semantic.normalization_producer,
        normalization_watchdog_sha256: semantic.normalization_watchdog,
        threshold_binding_sha256: semantic.threshold_binding,
        threshold_coverage_producer_sha256: semantic.threshold_producer,
        threshold_coverage_watchdog_sha256: semantic.threshold_watchdog,
        uncertainty_transport_producer_sha256: semantic.uncertainty_producer,
        uncertainty_transport_watchdog_sha256: semantic.uncertainty_watchdog,
        conservation_producer_sha256: semantic.conservation_producer,
        conservation_watchdog_sha256: semantic.conservation_watchdog,
        applicability_receipt_sha256: semantic.applicability,
        validity_receipt_sha256: semantic.validity,
        admission_evidence,
    })
}

struct ReconstructedReceipts {
    solver_producer: [u8; 32],
    solver_watchdog: [u8; 32],
    normalization_producer: [u8; 32],
    normalization_watchdog: [u8; 32],
    threshold_binding: [u8; 32],
    threshold_producer: [u8; 32],
    threshold_watchdog: [u8; 32],
    uncertainty_producer: [u8; 32],
    uncertainty_watchdog: [u8; 32],
    conservation_producer: [u8; 32],
    conservation_watchdog: [u8; 32],
    applicability: [u8; 32],
    validity: [u8; 32],
}

fn reconstruct_semantic_receipts(
    packet: &NeutralBoundProfilePacket,
    profile_root_identity: ArtifactIdentity,
    math: &ReconstructedMath,
) -> Result<ReconstructedReceipts, NeutralBoundProfileRefusal> {
    let binding = encode_rational(&math.binding_mass_factor)?;
    let reduced_mass = encode_rational(&math.reduced_mass_factor)?;
    let energy = encode_rational(&math.dimensionless_ground_energy)?;
    let solver_watchdog = digest_fields(
        SOLVER_WATCHDOG_DOMAIN,
        &[&binding, &reduced_mass, &energy, &profile_root_identity.0],
    );
    let solver_producer = digest_fields(
        SOLVER_PRODUCER_DOMAIN,
        &[&profile_root_identity.0, &energy, &reduced_mass, &binding],
    );
    let residual = encode_rational(&math.radial_residual_norm)?;
    let normalization = encode_rational(&math.normalization_value)?;
    let normalization_watchdog = digest_fields(
        NORMALIZATION_WATCHDOG_DOMAIN,
        &[&residual, &normalization, &profile_root_identity.0],
    );
    let normalization_producer = digest_fields(
        NORMALIZATION_PRODUCER_DOMAIN,
        &[&profile_root_identity.0, &normalization, &residual],
    );
    let threshold_binding = threshold_binding_sha256(
        &math.candidate_mass_interval,
        math.constituent_threshold_channel_identity,
        &math.constituent_threshold_interval,
    )?;
    let threshold_watchdog = digest_fields(
        THRESHOLD_WATCHDOG_DOMAIN,
        &[
            &packet.charged_profile_receipt.digest_sha256,
            &threshold_binding,
        ],
    );
    let threshold_producer = digest_fields(
        THRESHOLD_PRODUCER_DOMAIN,
        &[
            &threshold_binding,
            &packet.charged_profile_receipt.digest_sha256,
        ],
    );
    let candidate_interval = encode_interval(&math.candidate_mass_interval)?;
    let uncertainty_watchdog = digest_fields(
        UNCERTAINTY_WATCHDOG_DOMAIN,
        &[
            &packet.root_pair_receipt.digest_sha256,
            &packet.coupling_scalar_identity.0,
            &packet.mass_scalar_identity.0,
            &candidate_interval,
            &profile_root_identity.0,
        ],
    );
    let uncertainty_producer = digest_fields(
        UNCERTAINTY_PRODUCER_DOMAIN,
        &[
            &profile_root_identity.0,
            &candidate_interval,
            &packet.mass_scalar_identity.0,
            &packet.coupling_scalar_identity.0,
            &packet.root_pair_receipt.digest_sha256,
        ],
    );
    let conservation_watchdog = digest_fields(
        CONSERVATION_WATCHDOG_DOMAIN,
        &[
            &[0],
            &packet.charged_profile_receipt.digest_sha256,
            &packet.charged_members[1].0,
            &packet.charged_members[0].0,
        ],
    );
    let conservation_producer = digest_fields(
        CONSERVATION_PRODUCER_DOMAIN,
        &[
            &packet.charged_members[0].0,
            &packet.charged_members[1].0,
            &packet.charged_profile_receipt.digest_sha256,
            &[0],
        ],
    );
    let receipts = [
        conservation_watchdog,
        conservation_producer,
        uncertainty_watchdog,
        uncertainty_producer,
        threshold_watchdog,
        threshold_producer,
        normalization_watchdog,
        normalization_producer,
        solver_watchdog,
        solver_producer,
    ];
    if receipts.contains(&[0; 32])
        || receipts.into_iter().collect::<BTreeSet<_>>().len() != receipts.len()
    {
        return Err(NeutralBoundProfileRefusal::CheckerDisagreement);
    }
    let applicability = digest_fields(
        APPLICABILITY_DOMAIN,
        &[
            &solver_producer,
            &solver_watchdog,
            &normalization_producer,
            &normalization_watchdog,
            &threshold_producer,
            &threshold_watchdog,
            &uncertainty_producer,
            &uncertainty_watchdog,
            &conservation_producer,
            &conservation_watchdog,
            &packet.charged_profile_receipt.digest_sha256,
            &packet.primitive_profile_receipt.digest_sha256,
        ],
    );
    let validity = digest_fields(
        VALIDITY_DOMAIN,
        &[
            &applicability,
            packet.validity_id.as_bytes(),
            packet.coupling_value_decimal.as_bytes(),
            packet.coupling_uncertainty_decimal.as_bytes(),
        ],
    );
    Ok(ReconstructedReceipts {
        solver_producer,
        solver_watchdog,
        normalization_producer,
        normalization_watchdog,
        threshold_binding,
        threshold_producer,
        threshold_watchdog,
        uncertainty_producer,
        uncertainty_watchdog,
        conservation_producer,
        conservation_watchdog,
        applicability,
        validity,
    })
}

fn reverse_mass_expression(
    packet: &NeutralBoundProfilePacket,
) -> super::super::model::ExactExpression {
    use super::super::model::ExactExpressionNode;
    // Reconstructed from the output node backwards. Node order remains the
    // canonical wire order required by the registry expression evaluator.
    super::super::model::ExactExpression {
        nodes: vec![
            ExactExpressionNode::Coordinate(packet.mass_scalar_identity),
            ExactExpressionNode::Coordinate(packet.coupling_scalar_identity),
            ExactExpressionNode::IntegerPower {
                base: 1,
                exponent: 0,
            },
            ExactExpressionNode::Add { left: 2, right: 2 },
            ExactExpressionNode::Add { left: 3, right: 3 },
            ExactExpressionNode::IntegerPower {
                base: 1,
                exponent: 2,
            },
            ExactExpressionNode::Divide {
                numerator: 5,
                denominator: 4,
            },
            ExactExpressionNode::Subtract { left: 3, right: 6 },
            ExactExpressionNode::Multiply { left: 0, right: 7 },
        ],
        output_node: 8,
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

fn reverse_identity(
    payload: &ArtifactPayload,
) -> Result<ArtifactIdentity, NeutralBoundProfileRefusal> {
    super::super::watchdog::derive_artifact_identity_for_authority(payload)
        .map_err(|_| NeutralBoundProfileRefusal::ArtifactConstructionFailure)
}

fn reverse_push_derived(
    candidates: &mut Vec<NeutralBoundArtifactCandidate>,
    profile_root_identity: ArtifactIdentity,
    input_sha256: [u8; 32],
    payload: ArtifactPayload,
) -> Result<ArtifactIdentity, NeutralBoundProfileRefusal> {
    let identity = reverse_identity(&payload)?;
    candidates.push(NeutralBoundArtifactCandidate {
        identity,
        admission: reverse_derived_admission(input_sha256, profile_root_identity, identity),
        payload,
    });
    Ok(identity)
}

fn reverse_irreducible_admission(
    packet: &NeutralBoundProfilePacket,
    evidence: &law_premise::TheoryProfileAdmissionEvidence,
) -> RootAdmission {
    RootAdmission {
        tier: Tier::Residue,
        provenance: Provenance::Authored,
        route: AdmissionRoute::Irreducible(Box::new(super::super::model::IrreducibleAdmission {
            derivation_exhaustion_receipt: receipt_binding(
                "civsim.neutral-bound-profile.derive-first-exhaustion.v1",
                evidence.derivation_exhaustion_receipt_sha256,
            ),
            buckingham_pi_receipt: receipt_binding(
                "civsim.neutral-bound-profile.buckingham-pi.v1",
                evidence.buckingham_pi_receipt_sha256,
            ),
            gap_law_receipt: receipt_binding(
                "civsim.neutral-bound-profile.gap-law.v1",
                evidence.gap_law_receipt_sha256,
            ),
            chaos_protocol_receipt: receipt_binding(
                "civsim.neutral-bound-profile.chaos-protocol.v1",
                evidence.chaos_protocol_receipt_sha256,
            ),
            residual_law_receipt: receipt_binding(
                "civsim.neutral-bound-profile.residual-law.v1",
                evidence.residual_law_receipt_sha256,
            ),
            residual_slot_id: packet.residual_slot_id.clone(),
            residual_slot_receipt: receipt_binding(
                "civsim.neutral-bound-profile.residual-slot.v1",
                evidence.residual_slot_receipt_sha256,
            ),
            owner_admission_receipt: receipt_binding(
                "civsim.neutral-bound-profile.owner-admission.v1",
                evidence.owner_admission_receipt_sha256,
            ),
            independent_watchdog_receipt: receipt_binding(
                "civsim.neutral-bound-profile.independent-watchdog-route.v1",
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
                "civsim.neutral-bound-profile.derived-ancestry.v1",
                digest_fields(
                    b"civsim.neutral-bound-profile.derived-ancestry.v1",
                    &[&profile_root_identity.0, &artifact_identity.0],
                ),
            ),
            semantic_checker_receipt: receipt_binding(
                "civsim.neutral-bound-profile.derived-forward-semantic.v1",
                digest_fields(
                    b"civsim.neutral-bound-profile.derived-forward-semantic.v1",
                    &[&input_sha256, &artifact_identity.0],
                ),
            ),
            independent_watchdog_receipt: receipt_binding(
                "civsim.neutral-bound-profile.derived-reverse-semantic.v1",
                digest_fields(
                    b"civsim.neutral-bound-profile.derived-reverse-semantic.v1",
                    &[&artifact_identity.0, &input_sha256],
                ),
            ),
        }),
    }
}

fn reconstruct_evidence_digest(packet: &NeutralBoundProfilePacket) -> [u8; 32] {
    digest_fields(
        EVIDENCE_DOMAIN,
        &[
            packet.evidence_primary_citation.as_bytes(),
            packet.evidence_primary_url.as_bytes(),
            packet.evidence_primary_sha256_hex.as_bytes(),
            packet.evidence_primary_anchor.as_bytes(),
            packet.evidence_secondary_citation.as_bytes(),
            packet.evidence_secondary_url.as_bytes(),
            packet.evidence_secondary_sha256_hex.as_bytes(),
            packet.evidence_secondary_anchor.as_bytes(),
        ],
    )
}

fn parse_positive_from_right(value: &str) -> Result<BigRat, NeutralBoundProfileRefusal> {
    let reversed = value.as_bytes().iter().rev().copied().collect::<Vec<_>>();
    let restored = reversed.iter().rev().copied().collect::<Vec<_>>();
    let text = std::str::from_utf8(&restored)
        .map_err(|_| NeutralBoundProfileRefusal::ExactArithmeticFailure)?;
    let parsed = BigRat::from_decimal_str(text)
        .map_err(|_| NeutralBoundProfileRefusal::ExactArithmeticFailure)?;
    if parsed.cmp_rat(&BigRat::from_i64(0)) != Ordering::Greater {
        return Err(NeutralBoundProfileRefusal::InvalidValidityDomain);
    }
    Ok(parsed)
}

fn parse_nonnegative_from_right(value: &str) -> Result<BigRat, NeutralBoundProfileRefusal> {
    let restored = value
        .chars()
        .rev()
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    let parsed = BigRat::from_decimal_str(&restored)
        .map_err(|_| NeutralBoundProfileRefusal::ExactArithmeticFailure)?;
    if parsed.cmp_rat(&BigRat::from_i64(0)) == Ordering::Less {
        return Err(NeutralBoundProfileRefusal::InvalidValidityDomain);
    }
    Ok(parsed)
}

fn exact_min(values: &[BigRat]) -> Result<BigRat, NeutralBoundProfileRefusal> {
    values
        .iter()
        .cloned()
        .min_by(|left, right| left.cmp_rat(right))
        .ok_or(NeutralBoundProfileRefusal::ExactArithmeticFailure)
}

fn exact_max(values: &[BigRat]) -> Result<BigRat, NeutralBoundProfileRefusal> {
    values
        .iter()
        .cloned()
        .max_by(|left, right| left.cmp_rat(right))
        .ok_or(NeutralBoundProfileRefusal::ExactArithmeticFailure)
}

pub(super) fn canary_evidence(
    packet: &NeutralBoundProfilePacket,
) -> Result<CanaryEvidence, NeutralBoundProfileRefusal> {
    let sealed = sealed_packet()?;
    canary_evidence_against_sealed(packet, &sealed)
}

pub(super) fn canary_evidence_against_sealed(
    packet: &NeutralBoundProfilePacket,
    sealed: &NeutralBoundProfilePacket,
) -> Result<CanaryEvidence, NeutralBoundProfileRefusal> {
    let cases = reverse_mutations(packet);
    let mut transcript = WATCHDOG_CANARY_ID.as_bytes().to_vec();
    for (name, mutant) in cases.iter().rev() {
        let refusal = inspect_packet_against(mutant, sealed)
            .err()
            .ok_or(NeutralBoundProfileRefusal::CanaryFailure)?;
        append_field(&mut transcript, 2, refusal.id().as_bytes());
        append_field(&mut transcript, 1, name.as_bytes());
    }
    Ok(CanaryEvidence {
        transcript_id: WATCHDOG_CANARY_ID,
        case_count: u32::try_from(cases.len())
            .map_err(|_| NeutralBoundProfileRefusal::CanaryFailure)?,
        transcript_sha256: sha256(&transcript),
    })
}

fn reverse_mutations(
    packet: &NeutralBoundProfilePacket,
) -> Vec<(&'static str, NeutralBoundProfilePacket)> {
    let mut cases = Vec::new();
    macro_rules! mutate {
        ($name:literal, $body:expr) => {{
            let mut mutant = packet.clone();
            $body(&mut mutant);
            cases.push(($name, mutant));
        }};
    }
    mutate!("packet-schema", |p: &mut NeutralBoundProfilePacket| p
        .schema_id
        .push_str(".mutant"));
    mutate!("profile-id", |p: &mut NeutralBoundProfilePacket| p
        .profile_id
        .push_str(".mutant"));
    mutate!("primary-custody", |p: &mut NeutralBoundProfilePacket| p
        .evidence_primary_sha256_hex
        .replace_range(..1, "0"));
    mutate!("secondary-custody", |p: &mut NeutralBoundProfilePacket| p
        .evidence_secondary_sha256_hex
        .replace_range(..1, "0"));
    mutate!("mass-value", |p: &mut NeutralBoundProfilePacket| p
        .mass_value_decimal
        .push('0'));
    mutate!("mass-uncertainty", |p: &mut NeutralBoundProfilePacket| p
        .mass_uncertainty_decimal
        .push('0'));
    mutate!("coupling-value", |p: &mut NeutralBoundProfilePacket| p
        .coupling_value_decimal
        .push('0'));
    mutate!(
        "coupling-uncertainty",
        |p: &mut NeutralBoundProfilePacket| p.coupling_uncertainty_decimal.push('0')
    );
    mutate!("mass-identity", |p: &mut NeutralBoundProfilePacket| p
        .mass_scalar_identity
        .0[31] ^=
        1);
    mutate!("coupling-identity", |p: &mut NeutralBoundProfilePacket| {
        p.coupling_scalar_identity.0[31] ^= 1
    });
    mutate!("primitive-receipt", |p: &mut NeutralBoundProfilePacket| {
        p.primitive_profile_receipt.digest_sha256[31] ^= 1
    });
    mutate!("charged-receipt", |p: &mut NeutralBoundProfilePacket| p
        .charged_profile_receipt
        .digest_sha256[31] ^=
        1);
    mutate!(
        "charged-member-omission",
        |p: &mut NeutralBoundProfilePacket| {
            if !p.charged_members.is_empty() {
                p.charged_members.remove(0);
            }
        }
    );
    mutate!(
        "charged-member-reorder",
        |p: &mut NeutralBoundProfilePacket| {
            p.charged_members.reverse();
        }
    );
    mutate!(
        "primitive-slot-collision",
        |p: &mut NeutralBoundProfilePacket| {
            p.residual_slot_id = p.primitive_residual_slot_id.clone();
        }
    );
    mutate!(
        "charged-slot-collision",
        |p: &mut NeutralBoundProfilePacket| {
            p.residual_slot_id = p.charged_residual_slot_id.clone();
        }
    );
    mutate!("global-coverage", |p: &mut NeutralBoundProfilePacket| {
        p.global_physical_vocabulary_coverage = true;
    });
    mutate!(
        "membership-authority",
        |p: &mut NeutralBoundProfilePacket| {
            p.membership_authority = true;
        }
    );
    cases
}

pub(super) fn pair_receipt_digest(
    output: &NeutralBoundCheckerOutput,
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
            &producer_canary.transcript_sha256,
            &watchdog_canary.transcript_sha256,
            &output.profile_root_identity.0,
            &output.profile_role_identity.0,
            &output.member.0,
            &u32::try_from(output.candidates.len())
                .unwrap_or(u32::MAX)
                .to_be_bytes(),
            &output
                .admission_evidence
                .irreducible_protocol_capability_sha256,
        ],
    )
}
