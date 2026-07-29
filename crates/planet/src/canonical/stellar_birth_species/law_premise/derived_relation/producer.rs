use super::model::{
    AlphaAdmissionBinding, CanaryEvidence, ConstantBinding, DerivedRelationCheckerOutput,
    DerivedRelationPacket, DerivedRelationRefusal, InputRole, ARTIFACT_SCHEMA_ID, CANARY_SUITE_ID,
    PACKET_SCHEMA_ID, PRODUCER_CANARY_TRANSCRIPT_ID, PRODUCER_IMPLEMENTATION_ID,
    WATCHDOG_IMPLEMENTATION_ID,
};
use super::{append_field, digest_fields};
use civsim_ledger::{Provenance, Tier};
use civsim_units::{
    bignum::BigRat,
    constants::canonical_si_execution_magnitudes,
    digest::sha256,
    fundamentals::{
        composite, fundamental, Fundamental, FundamentalRole, SiDimension, VACUUM_PERMITTIVITY,
    },
    physics_floor::{
        sealed_physical_floor_authority_binding, sealed_physical_floor_irreducible_admissions,
    },
};

const CLAIM_DOMAIN: &[u8] = b"civsim.law-claim.execution-coordinate-relation.v1";
const ROLE_DOMAIN: &[u8] = b"civsim.semantic-role.derived-execution-coordinate.v1";
const CONTENT_DOMAIN: &[u8] = b"civsim.physical-content.derived-execution-relation.v1";
const UPSTREAM_DOMAIN: &[u8] = b"civsim.law-premise.upstream-floor-route.v1";
const APPLICABILITY_DOMAIN: &[u8] = b"civsim.law-premise.current-floor-applicability.v1";
const VALIDITY_DOMAIN: &[u8] = b"civsim.law-premise.exact-coordinate-validity.v1";
const ANCESTRY_DOMAIN: &[u8] = b"civsim.law-premise.derived-ancestry.v1";
const EXPECTED_FORMULA: &str = "e^2 / (2 * alpha * h * c)";
const EXPECTED_INPUTS: [&str; 4] = ["e", "alpha", "h", "c"];
const MAX_PACKET_BYTES: usize = 32_768;
const EXPECTED_CANARY_CASE_COUNT: u32 = 12;
const CLAIM_LOCAL_Q32_FRACTIONAL_BITS: u32 = 32;

pub(super) fn sealed_packet() -> Result<DerivedRelationPacket, DerivedRelationRefusal> {
    let floor_authority = sealed_physical_floor_authority_binding()
        .map_err(|_| DerivedRelationRefusal::SealedSourceUnavailable)?;
    let execution = canonical_si_execution_magnitudes()
        .map_err(|_| DerivedRelationRefusal::SealedSourceUnavailable)?;
    let alpha_admission = sealed_physical_floor_irreducible_admissions()
        .map_err(|_| DerivedRelationRefusal::SealedSourceUnavailable)?
        .into_iter()
        .find(|binding| binding.entry_id() == "fundamental.alpha")
        .ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?;

    let mut inputs = Vec::with_capacity(EXPECTED_INPUTS.len());
    for symbol in EXPECTED_INPUTS {
        let value = execution
            .get(symbol)
            .ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?;
        let definition = if symbol == "alpha" {
            execution
                .physical_invariant_definition(symbol)
                .ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?
        } else {
            fundamental(symbol).ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?
        };
        inputs.push(constant_binding(definition, value));
    }
    let output = execution
        .get("eps_0")
        .ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?;
    let relation = composite("eps_0").ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?;

    Ok(DerivedRelationPacket {
        schema_id: PACKET_SCHEMA_ID.to_owned(),
        floor_authority_schema_id: floor_authority.schema_id().as_str().to_owned(),
        floor_authority_sha256: floor_authority.digest(),
        alpha_admission: AlphaAdmissionBinding {
            schema_id: alpha_admission.schema_id().to_owned(),
            entry_id: alpha_admission.entry_id().to_owned(),
            derivation_exhaustion_receipt: alpha_admission.derivation_exhaustion_receipt(),
            buckingham_pi_receipt: alpha_admission.buckingham_pi_receipt(),
            gap_law_receipt: alpha_admission.gap_law_receipt(),
            chaos_protocol_receipt: alpha_admission.chaos_protocol_receipt(),
            residual_law_receipt: alpha_admission.residual_law_receipt(),
            residual_slot_receipt: alpha_admission.residual_slot_receipt(),
            owner_admission_receipt: alpha_admission.owner_admission_receipt(),
            independent_watchdog_receipt: alpha_admission.independent_watchdog_receipt(),
        },
        relation: super::model::RelationBinding {
            symbol: relation.symbol.to_owned(),
            name: relation.name.to_owned(),
            formula: relation.formula.to_owned(),
            inputs: relation
                .fundamentals
                .iter()
                .map(|symbol| (*symbol).to_owned())
                .collect(),
            unit: relation.unit.to_owned(),
            dimension: relation.dimension.exponents(),
        },
        inputs,
        output: super::model::OutputBinding {
            symbol: output.symbol().to_owned(),
            bits: output.bits(),
            scale_bits: output.scale_bits(),
            projection_receipt_sha256: output.projection_receipt_sha256(),
            unit: relation.unit.to_owned(),
            dimension: relation.dimension.exponents(),
        },
    })
}

fn constant_binding(
    definition: &Fundamental,
    value: civsim_units::constants::ScaledConstant,
) -> ConstantBinding {
    ConstantBinding {
        symbol: value.symbol().to_owned(),
        role: match definition.role {
            FundamentalRole::PhysicalInvariant => InputRole::PhysicalInvariant,
            FundamentalRole::RepresentationDefinition => InputRole::RepresentationDefinition,
        },
        bits: value.bits(),
        scale_bits: value.scale_bits(),
        projection_receipt_sha256: value.projection_receipt_sha256(),
        unit: definition.unit.to_owned(),
        dimension: definition.dimension.exponents(),
        source_id: definition.source_id.to_owned(),
        source_sha256: definition.source_sha256.to_owned(),
        source_anchor: definition.source_anchor.to_owned(),
    }
}

pub(super) fn inspect(
    packet: &DerivedRelationPacket,
) -> Result<DerivedRelationCheckerOutput, DerivedRelationRefusal> {
    validate(packet)?;
    let relation_bytes = encode_relation(packet)?;
    let input_bytes = encode_packet(packet)?;
    let input_sha256 = sha256(&input_bytes);
    let claim_identity = super::super::LawClaimIdentity(digest_fields(
        CLAIM_DOMAIN,
        &[&relation_bytes, &packet.floor_authority_sha256],
    ));
    let role_identity = super::super::SemanticRoleIdentity(sha256(ROLE_DOMAIN));
    let content_identity = super::super::PhysicalContentIdentity(digest_fields(
        CONTENT_DOMAIN,
        &[ARTIFACT_SCHEMA_ID.as_bytes(), &relation_bytes],
    ));
    Ok(DerivedRelationCheckerOutput {
        input_sha256,
        claim_identity,
        role_identity,
        content_identity,
        upstream_capability_sha256: digest_fields(
            UPSTREAM_DOMAIN,
            &[
                &packet.floor_authority_sha256,
                &packet.alpha_admission.owner_admission_receipt,
                &packet.output.projection_receipt_sha256,
            ],
        ),
        applicability_receipt_sha256: digest_fields(
            APPLICABILITY_DOMAIN,
            &[
                &packet.floor_authority_sha256,
                packet.floor_authority_schema_id.as_bytes(),
            ],
        ),
        validity_receipt_sha256: digest_fields(
            VALIDITY_DOMAIN,
            &[
                packet.relation.formula.as_bytes(),
                &dimension_bytes(packet.relation.dimension),
                &packet.output.projection_receipt_sha256,
            ],
        ),
        ancestry_receipt_sha256: digest_fields(
            ANCESTRY_DOMAIN,
            &[
                &input_sha256,
                &packet.alpha_admission.derivation_exhaustion_receipt,
                &packet.alpha_admission.independent_watchdog_receipt,
            ],
        ),
        relation_bytes,
    })
}

fn validate(packet: &DerivedRelationPacket) -> Result<(), DerivedRelationRefusal> {
    if packet.schema_id != PACKET_SCHEMA_ID {
        return Err(DerivedRelationRefusal::PacketSchemaMismatch);
    }
    if packet.floor_authority_schema_id != "civsim.units.physical-floor-authority-binding.v5"
        || packet.floor_authority_sha256 == [0; 32]
    {
        return Err(DerivedRelationRefusal::FloorBindingInvalid);
    }
    validate_alpha_admission(&packet.alpha_admission)?;
    if packet.relation.symbol != VACUUM_PERMITTIVITY.symbol
        || packet.relation.name != VACUUM_PERMITTIVITY.name
        || packet.relation.unit != VACUUM_PERMITTIVITY.unit
    {
        return Err(DerivedRelationRefusal::RelationIdentityMismatch);
    }
    if packet.relation.formula != EXPECTED_FORMULA {
        return Err(DerivedRelationRefusal::RelationFormulaMismatch);
    }
    if packet.relation.inputs
        != EXPECTED_INPUTS
            .iter()
            .map(|symbol| (*symbol).to_owned())
            .collect::<Vec<_>>()
    {
        return Err(DerivedRelationRefusal::RelationInputMismatch);
    }
    let expected_dimension = SiDimension::new(-3, -1, 4, 2, 0, 0, 0).exponents();
    if packet.relation.dimension != expected_dimension {
        return Err(DerivedRelationRefusal::RelationDimensionMismatch);
    }
    validate_inputs(&packet.inputs)?;
    if packet.output.symbol != "eps_0"
        || packet.output.unit != "F/m"
        || packet.output.dimension != expected_dimension
        || packet.output.bits <= 0
        || packet.output.projection_receipt_sha256 == [0; 32]
    {
        return Err(DerivedRelationRefusal::OutputBindingInvalid);
    }
    let by_symbol = |symbol: &str| {
        packet
            .inputs
            .iter()
            .find(|input| input.symbol == symbol)
            .ok_or(DerivedRelationRefusal::InputBindingInvalid)
    };
    let e = exact(by_symbol("e")?);
    let alpha = exact(by_symbol("alpha")?);
    let h = exact(by_symbol("h")?);
    let c = exact(by_symbol("c")?);
    let denominator = BigRat::from_i64(2).mul(&alpha).mul(&h).mul(&c);
    let derived = e.mul(&e).div(&denominator);
    if packet.output.scale_bits != expected_output_scale(&derived)? {
        return Err(DerivedRelationRefusal::RepresentationScaleMismatch);
    }
    if derived.round_to_scale(packet.output.scale_bits) != Some(packet.output.bits) {
        return Err(DerivedRelationRefusal::ArithmeticMismatch);
    }
    Ok(())
}

fn expected_output_scale(value: &BigRat) -> Result<u32, DerivedRelationRefusal> {
    let magnitude_log2 = value.floor_log2();
    let significand_floor =
        i64::from(CLAIM_LOCAL_Q32_FRACTIONAL_BITS).saturating_sub(magnitude_log2);
    let scale = i64::from(CLAIM_LOCAL_Q32_FRACTIONAL_BITS)
        .max(significand_floor)
        .max(0);
    let maximum_scale = (i64::from(i128::BITS) - 2).saturating_sub(magnitude_log2);
    if scale > maximum_scale {
        return Err(DerivedRelationRefusal::ResourceLimitExceeded);
    }
    u32::try_from(scale).map_err(|_| DerivedRelationRefusal::ResourceLimitExceeded)
}

fn validate_alpha_admission(
    admission: &AlphaAdmissionBinding,
) -> Result<(), DerivedRelationRefusal> {
    if admission.schema_id != "civsim.units.physical-floor-irreducible-admission-binding.v1"
        || admission.entry_id != "fundamental.alpha"
    {
        return Err(DerivedRelationRefusal::AlphaAdmissionInvalid);
    }
    let receipts = [
        admission.derivation_exhaustion_receipt,
        admission.buckingham_pi_receipt,
        admission.gap_law_receipt,
        admission.chaos_protocol_receipt,
        admission.residual_law_receipt,
        admission.residual_slot_receipt,
        admission.owner_admission_receipt,
        admission.independent_watchdog_receipt,
    ];
    if receipts.contains(&[0; 32])
        || receipts
            .iter()
            .enumerate()
            .any(|(index, receipt)| receipts[index + 1..].contains(receipt))
    {
        return Err(DerivedRelationRefusal::AlphaAdmissionInvalid);
    }
    Ok(())
}

fn validate_inputs(inputs: &[ConstantBinding]) -> Result<(), DerivedRelationRefusal> {
    if inputs.len() != EXPECTED_INPUTS.len() {
        return Err(DerivedRelationRefusal::InputBindingInvalid);
    }
    let expected_roles = [
        InputRole::RepresentationDefinition,
        InputRole::PhysicalInvariant,
        InputRole::RepresentationDefinition,
        InputRole::RepresentationDefinition,
    ];
    for ((input, symbol), role) in inputs.iter().zip(EXPECTED_INPUTS).zip(expected_roles) {
        if input.symbol != symbol
            || input.role != role
            || input.bits <= 0
            || input.projection_receipt_sha256 == [0; 32]
            || input.source_id.is_empty()
            || input.source_sha256.len() != 64
            || input.source_anchor.is_empty()
        {
            return Err(DerivedRelationRefusal::InputBindingInvalid);
        }
    }
    if inputs[0].dimension != [0, 0, 1, 1, 0, 0, 0]
        || inputs[1].dimension != [0; 7]
        || inputs[2].dimension != [2, 1, -1, 0, 0, 0, 0]
        || inputs[3].dimension != [1, 0, -1, 0, 0, 0, 0]
    {
        return Err(DerivedRelationRefusal::InputBindingInvalid);
    }
    Ok(())
}

fn exact(binding: &ConstantBinding) -> BigRat {
    BigRat::from_scaled_i128(binding.bits, binding.scale_bits)
}

pub(super) fn encode_packet(
    packet: &DerivedRelationPacket,
) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut bytes = PACKET_SCHEMA_ID.as_bytes().to_vec();
    append_field(&mut bytes, 1, packet.schema_id.as_bytes())?;
    append_field(&mut bytes, 2, packet.floor_authority_schema_id.as_bytes())?;
    append_field(&mut bytes, 3, &packet.floor_authority_sha256)?;
    append_field(&mut bytes, 4, &encode_alpha(&packet.alpha_admission)?)?;
    append_field(&mut bytes, 5, packet.relation.symbol.as_bytes())?;
    append_field(&mut bytes, 6, packet.relation.name.as_bytes())?;
    append_field(&mut bytes, 7, packet.relation.formula.as_bytes())?;
    for input in &packet.relation.inputs {
        append_field(&mut bytes, 8, input.as_bytes())?;
    }
    append_field(&mut bytes, 9, packet.relation.unit.as_bytes())?;
    append_field(&mut bytes, 10, &dimension_bytes(packet.relation.dimension))?;
    for input in &packet.inputs {
        append_field(&mut bytes, 11, &encode_constant(input)?)?;
    }
    append_field(&mut bytes, 12, &encode_output(&packet.output)?)?;
    if bytes.len() > MAX_PACKET_BYTES {
        return Err(DerivedRelationRefusal::ResourceLimitExceeded);
    }
    Ok(bytes)
}

fn encode_relation(packet: &DerivedRelationPacket) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut bytes = ARTIFACT_SCHEMA_ID.as_bytes().to_vec();
    append_field(&mut bytes, 1, packet.relation.formula.as_bytes())?;
    append_field(&mut bytes, 2, &dimension_bytes(packet.relation.dimension))?;
    for input in &packet.inputs {
        append_field(&mut bytes, 3, &encode_constant(input)?)?;
    }
    append_field(&mut bytes, 4, &encode_output(&packet.output)?)?;
    append_field(&mut bytes, 5, &packet.floor_authority_sha256)?;
    append_field(&mut bytes, 6, &encode_alpha(&packet.alpha_admission)?)?;
    Ok(bytes)
}

fn encode_alpha(admission: &AlphaAdmissionBinding) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, admission.schema_id.as_bytes())?;
    append_field(&mut bytes, 2, admission.entry_id.as_bytes())?;
    for (tag, receipt) in [
        admission.derivation_exhaustion_receipt,
        admission.buckingham_pi_receipt,
        admission.gap_law_receipt,
        admission.chaos_protocol_receipt,
        admission.residual_law_receipt,
        admission.residual_slot_receipt,
        admission.owner_admission_receipt,
        admission.independent_watchdog_receipt,
    ]
    .into_iter()
    .enumerate()
    {
        append_field(
            &mut bytes,
            u16::try_from(tag + 3).map_err(|_| DerivedRelationRefusal::ResourceLimitExceeded)?,
            &receipt,
        )?;
    }
    Ok(bytes)
}

fn encode_constant(binding: &ConstantBinding) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, binding.symbol.as_bytes())?;
    append_field(&mut bytes, 2, binding.role.id().as_bytes())?;
    append_field(&mut bytes, 3, &binding.bits.to_be_bytes())?;
    append_field(&mut bytes, 4, &binding.scale_bits.to_be_bytes())?;
    append_field(&mut bytes, 5, &binding.projection_receipt_sha256)?;
    append_field(&mut bytes, 6, binding.unit.as_bytes())?;
    append_field(&mut bytes, 7, &dimension_bytes(binding.dimension))?;
    append_field(&mut bytes, 8, binding.source_id.as_bytes())?;
    append_field(&mut bytes, 9, binding.source_sha256.as_bytes())?;
    append_field(&mut bytes, 10, binding.source_anchor.as_bytes())?;
    Ok(bytes)
}

fn encode_output(output: &super::model::OutputBinding) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, output.symbol.as_bytes())?;
    append_field(&mut bytes, 2, &output.bits.to_be_bytes())?;
    append_field(&mut bytes, 3, &output.scale_bits.to_be_bytes())?;
    append_field(&mut bytes, 4, &output.projection_receipt_sha256)?;
    append_field(&mut bytes, 5, output.unit.as_bytes())?;
    append_field(&mut bytes, 6, &dimension_bytes(output.dimension))?;
    Ok(bytes)
}

fn dimension_bytes(dimension: [i8; 7]) -> [u8; 7] {
    dimension.map(|exponent| exponent.to_be_bytes()[0])
}

pub(super) fn canary_evidence(
    packet: &DerivedRelationPacket,
) -> Result<CanaryEvidence, DerivedRelationRefusal> {
    let baseline = inspect(packet)?;
    let mut mutants = Vec::new();

    let mut changed = packet.clone();
    changed.floor_authority_sha256[0] ^= 1;
    mutants.push(("floor_digest", changed));
    let mut changed = packet.clone();
    changed.alpha_admission.owner_admission_receipt[0] ^= 1;
    mutants.push(("owner_receipt", changed));
    let mut changed = packet.clone();
    changed.relation.formula = "e^2 * (2 * alpha * h * c)".to_owned();
    mutants.push(("formula", changed));
    let mut changed = packet.clone();
    changed.relation.inputs.swap(0, 1);
    mutants.push(("relation_input_order", changed));
    let mut changed = packet.clone();
    changed.inputs.swap(0, 1);
    mutants.push(("input_order", changed));
    let mut changed = packet.clone();
    changed.inputs[1].role = InputRole::RepresentationDefinition;
    mutants.push(("alpha_role", changed));
    let mut changed = packet.clone();
    changed.inputs[0].projection_receipt_sha256[0] ^= 1;
    mutants.push(("input_projection_receipt", changed));
    let mut changed = packet.clone();
    changed.output.bits = changed.output.bits.saturating_add(1);
    mutants.push(("output_bits", changed));
    let mut changed = packet.clone();
    changed.output.projection_receipt_sha256[0] ^= 1;
    mutants.push(("output_projection_receipt", changed));
    let mut changed = packet.clone();
    changed.relation.dimension[0] = changed.relation.dimension[0].saturating_add(1);
    mutants.push(("relation_dimension", changed));
    let mut changed = packet.clone();
    changed.output.dimension[0] = changed.output.dimension[0].saturating_add(1);
    mutants.push(("output_dimension", changed));
    let mut changed = packet.clone();
    changed.output.scale_bits = changed.output.scale_bits.saturating_add(1);
    mutants.push(("output_scale", changed));
    mutants.sort_unstable_by_key(|(name, _)| *name);

    let mut transcript = PRODUCER_CANARY_TRANSCRIPT_ID.as_bytes().to_vec();
    let case_count =
        u32::try_from(mutants.len()).map_err(|_| DerivedRelationRefusal::ResourceLimitExceeded)?;
    if case_count != EXPECTED_CANARY_CASE_COUNT {
        return Err(DerivedRelationRefusal::CanaryFailure);
    }
    append_field(&mut transcript, 1, &case_count.to_be_bytes())?;
    for (name, mutant) in mutants {
        let preimage = encode_packet(&mutant)?;
        let observed = inspect(&mutant);
        if observed.as_ref().is_ok_and(|output| output == &baseline) {
            return Err(DerivedRelationRefusal::CanaryFailure);
        }
        append_field(&mut transcript, 2, name.as_bytes())?;
        append_field(&mut transcript, 3, &preimage)?;
        append_field(&mut transcript, 4, &encode_observation(&observed)?)?;
    }
    Ok(CanaryEvidence {
        transcript_id: PRODUCER_CANARY_TRANSCRIPT_ID,
        case_count,
        transcript_sha256: sha256(&transcript),
    })
}

fn encode_observation(
    observed: &Result<DerivedRelationCheckerOutput, DerivedRelationRefusal>,
) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut bytes = b"civsim.planet.derived-law-premise-eps0-canary-observation.v1".to_vec();
    match observed {
        Ok(output) => {
            append_field(&mut bytes, 1, b"accepted_changed")?;
            for (tag, value) in [
                (2, output.input_sha256.as_slice()),
                (3, output.claim_identity.0.as_slice()),
                (4, output.role_identity.0.as_slice()),
                (5, output.content_identity.0.as_slice()),
                (6, output.relation_bytes.as_slice()),
                (7, output.upstream_capability_sha256.as_slice()),
                (8, output.applicability_receipt_sha256.as_slice()),
                (9, output.validity_receipt_sha256.as_slice()),
                (10, output.ancestry_receipt_sha256.as_slice()),
            ] {
                append_field(&mut bytes, tag, value)?;
            }
        }
        Err(error) => {
            append_field(&mut bytes, 1, b"refused")?;
            append_field(&mut bytes, 2, error.id().as_bytes())?;
        }
    }
    Ok(bytes)
}

pub(super) fn semantic_receipt(
    output: &DerivedRelationCheckerOutput,
    pair_receipt_sha256: [u8; 32],
) -> [u8; 32] {
    digest_fields(
        PRODUCER_IMPLEMENTATION_ID.as_bytes(),
        &[
            &output.input_sha256,
            &output.content_identity.0,
            &pair_receipt_sha256,
        ],
    )
}

pub(super) fn pair_receipt_digest(
    output: &DerivedRelationCheckerOutput,
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
    producer_canary: CanaryEvidence,
    watchdog_canary: CanaryEvidence,
) -> [u8; 32] {
    digest_fields(
        b"civsim.planet.derived-law-premise-eps0-pair-receipt.v2",
        &[
            super::model::RECEIPT_SCHEMA_ID.as_bytes(),
            &output.input_sha256,
            &output.claim_identity.0,
            &output.role_identity.0,
            &output.content_identity.0,
            PRODUCER_IMPLEMENTATION_ID.as_bytes(),
            WATCHDOG_IMPLEMENTATION_ID.as_bytes(),
            &producer_result_sha256,
            &watchdog_result_sha256,
            CANARY_SUITE_ID.as_bytes(),
            producer_canary.transcript_id.as_bytes(),
            &producer_canary.case_count.to_be_bytes(),
            &producer_canary.transcript_sha256,
            watchdog_canary.transcript_id.as_bytes(),
            &watchdog_canary.case_count.to_be_bytes(),
            &watchdog_canary.transcript_sha256,
            b"agreed",
            b"planet.derived-law-premise-eps0",
            Tier::Universal.id().as_bytes(),
            Provenance::Derived
                .bracket_tag()
                .expect("derived is canonical")
                .as_bytes(),
            b"verified_one_claim_scoped_derived_execution_relation",
        ],
    )
}

pub(super) const fn implementation_id() -> &'static str {
    PRODUCER_IMPLEMENTATION_ID
}

pub(super) fn result_sha256(output: &DerivedRelationCheckerOutput) -> [u8; 32] {
    digest_fields(
        b"civsim.planet.derived-law-premise-eps0-producer-result.v1",
        &[
            &output.input_sha256,
            &output.claim_identity.0,
            &output.role_identity.0,
            &output.content_identity.0,
            &output.relation_bytes,
            &output.upstream_capability_sha256,
            &output.applicability_receipt_sha256,
            &output.validity_receipt_sha256,
            &output.ancestry_receipt_sha256,
        ],
    )
}
