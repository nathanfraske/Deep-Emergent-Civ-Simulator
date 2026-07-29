use super::model::{
    AlphaAdmissionBinding, CanaryEvidence, ConstantBinding, DerivedRelationCheckerOutput,
    DerivedRelationPacket, DerivedRelationRefusal, InputRole, ARTIFACT_SCHEMA_ID, CANARY_SUITE_ID,
    PACKET_SCHEMA_ID, PRODUCER_IMPLEMENTATION_ID, WATCHDOG_CANARY_TRANSCRIPT_ID,
    WATCHDOG_IMPLEMENTATION_ID,
};
use super::{append_field, digest_fields};
use civsim_units::{
    bignum::BigRat,
    constants::canonical_si_execution_magnitudes,
    digest::sha256,
    fundamentals::{composite, fundamental, FundamentalRole},
    physics_floor::{
        sealed_physical_floor_authority_binding, sealed_physical_floor_irreducible_admissions,
    },
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_PACKET_BYTES: usize = 32_768;
const EXPECTED_CANARY_CASE_COUNT: u32 = 12;
const REQUIRED_SIGNIFICAND_BITS: i64 = 32;

pub(super) fn sealed_packet() -> Result<DerivedRelationPacket, DerivedRelationRefusal> {
    let authority = sealed_physical_floor_authority_binding()
        .map_err(|_| DerivedRelationRefusal::SealedSourceUnavailable)?;
    let values = canonical_si_execution_magnitudes()
        .map_err(|_| DerivedRelationRefusal::SealedSourceUnavailable)?;
    let admissions = sealed_physical_floor_irreducible_admissions()
        .map_err(|_| DerivedRelationRefusal::SealedSourceUnavailable)?;
    let alpha = admissions
        .iter()
        .find(|admission| admission.entry_id().as_bytes() == b"fundamental.alpha")
        .copied()
        .ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?;
    let relation = composite("eps_0").ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?;

    let mut inputs = Vec::new();
    for symbol in relation.fundamentals {
        let value = values
            .get(symbol)
            .ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?;
        let definition = values
            .physical_invariant_definition(symbol)
            .or_else(|| fundamental(symbol))
            .ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?;
        inputs.push(ConstantBinding {
            symbol: definition.symbol.to_owned(),
            role: if definition.role == FundamentalRole::PhysicalInvariant {
                InputRole::PhysicalInvariant
            } else {
                InputRole::RepresentationDefinition
            },
            bits: value.bits(),
            scale_bits: value.scale_bits(),
            projection_receipt_sha256: value.projection_receipt_sha256(),
            unit: definition.unit.to_owned(),
            dimension: definition.dimension.exponents(),
            source_id: definition.source_id.to_owned(),
            source_sha256: definition.source_sha256.to_owned(),
            source_anchor: definition.source_anchor.to_owned(),
        });
    }
    let output = values
        .get(relation.symbol)
        .ok_or(DerivedRelationRefusal::SealedSourceUnavailable)?;
    Ok(DerivedRelationPacket {
        schema_id: PACKET_SCHEMA_ID.to_owned(),
        floor_authority_schema_id: authority.schema_id().as_str().to_owned(),
        floor_authority_sha256: authority.digest(),
        alpha_admission: AlphaAdmissionBinding {
            schema_id: alpha.schema_id().to_owned(),
            entry_id: alpha.entry_id().to_owned(),
            derivation_exhaustion_receipt: alpha.derivation_exhaustion_receipt(),
            buckingham_pi_receipt: alpha.buckingham_pi_receipt(),
            gap_law_receipt: alpha.gap_law_receipt(),
            chaos_protocol_receipt: alpha.chaos_protocol_receipt(),
            residual_law_receipt: alpha.residual_law_receipt(),
            residual_slot_receipt: alpha.residual_slot_receipt(),
            owner_admission_receipt: alpha.owner_admission_receipt(),
            independent_watchdog_receipt: alpha.independent_watchdog_receipt(),
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

pub(super) fn inspect(
    packet: &DerivedRelationPacket,
) -> Result<DerivedRelationCheckerOutput, DerivedRelationRefusal> {
    check_packet(packet)?;
    let relation_bytes = write_relation(packet)?;
    let packet_bytes = write_packet(packet)?;
    let input_sha256 = sha256(&packet_bytes);
    let claim_identity = super::super::LawClaimIdentity(digest_fields(
        b"civsim.law-claim.execution-coordinate-relation.v1",
        &[&relation_bytes, &packet.floor_authority_sha256],
    ));
    let role_identity = super::super::SemanticRoleIdentity(sha256(
        b"civsim.semantic-role.derived-execution-coordinate.v1",
    ));
    let content_identity = super::super::PhysicalContentIdentity(digest_fields(
        b"civsim.physical-content.derived-execution-relation.v1",
        &[ARTIFACT_SCHEMA_ID.as_bytes(), &relation_bytes],
    ));
    Ok(DerivedRelationCheckerOutput {
        input_sha256,
        claim_identity,
        role_identity,
        content_identity,
        relation_bytes,
        upstream_capability_sha256: digest_fields(
            b"civsim.law-premise.upstream-floor-route.v1",
            &[
                &packet.floor_authority_sha256,
                &packet.alpha_admission.owner_admission_receipt,
                &packet.output.projection_receipt_sha256,
            ],
        ),
        applicability_receipt_sha256: digest_fields(
            b"civsim.law-premise.current-floor-applicability.v1",
            &[
                &packet.floor_authority_sha256,
                packet.floor_authority_schema_id.as_bytes(),
            ],
        ),
        validity_receipt_sha256: digest_fields(
            b"civsim.law-premise.exact-coordinate-validity.v1",
            &[
                packet.relation.formula.as_bytes(),
                &dimension_wire(packet.relation.dimension),
                &packet.output.projection_receipt_sha256,
            ],
        ),
        ancestry_receipt_sha256: digest_fields(
            b"civsim.law-premise.derived-ancestry.v1",
            &[
                &input_sha256,
                &packet.alpha_admission.derivation_exhaustion_receipt,
                &packet.alpha_admission.independent_watchdog_receipt,
            ],
        ),
    })
}

fn check_packet(packet: &DerivedRelationPacket) -> Result<(), DerivedRelationRefusal> {
    if packet.schema_id.as_bytes() != PACKET_SCHEMA_ID.as_bytes() {
        return Err(DerivedRelationRefusal::PacketSchemaMismatch);
    }
    if packet.floor_authority_schema_id.as_bytes()
        != b"civsim.units.physical-floor-authority-binding.v5"
        || packet.floor_authority_sha256.iter().all(|byte| *byte == 0)
    {
        return Err(DerivedRelationRefusal::FloorBindingInvalid);
    }
    check_alpha(&packet.alpha_admission)?;
    if (
        packet.relation.symbol.as_str(),
        packet.relation.name.as_str(),
        packet.relation.unit.as_str(),
    ) != ("eps_0", "vacuum electric permittivity", "F/m")
    {
        return Err(DerivedRelationRefusal::RelationIdentityMismatch);
    }
    let formula_tokens = packet
        .relation
        .formula
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    if formula_tokens != ["e^2", "/", "(2", "*", "alpha", "*", "h", "*", "c)"] {
        return Err(DerivedRelationRefusal::RelationFormulaMismatch);
    }
    if packet
        .relation
        .inputs
        .iter()
        .map(String::as_str)
        .ne(["e", "alpha", "h", "c"])
    {
        return Err(DerivedRelationRefusal::RelationInputMismatch);
    }
    let required_dimension = [-3, -1, 4, 2, 0, 0, 0];
    if packet.relation.dimension != required_dimension {
        return Err(DerivedRelationRefusal::RelationDimensionMismatch);
    }

    let mut inputs = BTreeMap::new();
    for input in packet.inputs.iter().rev() {
        if input.symbol.is_empty()
            || input.bits <= 0
            || input
                .projection_receipt_sha256
                .iter()
                .all(|byte| *byte == 0)
            || input.source_id.is_empty()
            || input.source_sha256.len() != 64
            || input.source_anchor.is_empty()
            || inputs.insert(input.symbol.as_str(), input).is_some()
        {
            return Err(DerivedRelationRefusal::InputBindingInvalid);
        }
    }
    if packet
        .inputs
        .iter()
        .map(|input| input.symbol.as_str())
        .collect::<Vec<_>>()
        != ["e", "alpha", "h", "c"]
    {
        return Err(DerivedRelationRefusal::InputBindingInvalid);
    }
    let expected = [
        (
            "e",
            InputRole::RepresentationDefinition,
            [0, 0, 1, 1, 0, 0, 0],
        ),
        ("alpha", InputRole::PhysicalInvariant, [0; 7]),
        (
            "h",
            InputRole::RepresentationDefinition,
            [2, 1, -1, 0, 0, 0, 0],
        ),
        (
            "c",
            InputRole::RepresentationDefinition,
            [1, 0, -1, 0, 0, 0, 0],
        ),
    ];
    for (symbol, role, dimension) in expected {
        let input = inputs
            .get(symbol)
            .ok_or(DerivedRelationRefusal::InputBindingInvalid)?;
        if input.role != role || input.dimension != dimension {
            return Err(DerivedRelationRefusal::InputBindingInvalid);
        }
    }
    if packet.output.symbol.as_bytes() != b"eps_0"
        || packet.output.unit.as_bytes() != b"F/m"
        || packet.output.dimension != required_dimension
        || packet.output.bits <= 0
        || packet
            .output
            .projection_receipt_sha256
            .iter()
            .all(|byte| *byte == 0)
    {
        return Err(DerivedRelationRefusal::OutputBindingInvalid);
    }

    let e = rational(inputs["e"]);
    let alpha = rational(inputs["alpha"]);
    let h_times_c = rational(inputs["h"]).mul(&rational(inputs["c"]));
    let two_alpha = alpha.mul(&BigRat::from_i64(2));
    let derived = e.div(&h_times_c).mul(&e.div(&two_alpha));
    if packet.output.scale_bits != independently_selected_scale(&derived)? {
        return Err(DerivedRelationRefusal::RepresentationScaleMismatch);
    }
    if derived.round_to_scale(packet.output.scale_bits) != Some(packet.output.bits) {
        return Err(DerivedRelationRefusal::ArithmeticMismatch);
    }
    Ok(())
}

fn independently_selected_scale(value: &BigRat) -> Result<u32, DerivedRelationRefusal> {
    let magnitude = value.floor_log2();
    let precision_scale = REQUIRED_SIGNIFICAND_BITS
        .checked_sub(magnitude)
        .ok_or(DerivedRelationRefusal::ResourceLimitExceeded)?;
    let selected = [0, REQUIRED_SIGNIFICAND_BITS, precision_scale]
        .into_iter()
        .max()
        .ok_or(DerivedRelationRefusal::ResourceLimitExceeded)?;
    let signed_i128_limit = 126_i64
        .checked_sub(magnitude)
        .ok_or(DerivedRelationRefusal::ResourceLimitExceeded)?;
    if selected > signed_i128_limit {
        return Err(DerivedRelationRefusal::ResourceLimitExceeded);
    }
    u32::try_from(selected).map_err(|_| DerivedRelationRefusal::ResourceLimitExceeded)
}

fn check_alpha(admission: &AlphaAdmissionBinding) -> Result<(), DerivedRelationRefusal> {
    if admission.schema_id.as_bytes()
        != b"civsim.units.physical-floor-irreducible-admission-binding.v1"
        || admission.entry_id.as_bytes() != b"fundamental.alpha"
    {
        return Err(DerivedRelationRefusal::AlphaAdmissionInvalid);
    }
    let receipts = BTreeSet::from([
        admission.derivation_exhaustion_receipt,
        admission.buckingham_pi_receipt,
        admission.gap_law_receipt,
        admission.chaos_protocol_receipt,
        admission.residual_law_receipt,
        admission.residual_slot_receipt,
        admission.owner_admission_receipt,
        admission.independent_watchdog_receipt,
    ]);
    if receipts.len() != 8 || receipts.contains(&[0; 32]) {
        return Err(DerivedRelationRefusal::AlphaAdmissionInvalid);
    }
    Ok(())
}

fn rational(input: &ConstantBinding) -> BigRat {
    BigRat::from_scaled_i128(input.bits, input.scale_bits)
}

pub(super) fn write_packet(
    packet: &DerivedRelationPacket,
) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut fields = BTreeMap::<u16, Vec<Vec<u8>>>::new();
    push(&mut fields, 1, packet.schema_id.as_bytes());
    push(&mut fields, 2, packet.floor_authority_schema_id.as_bytes());
    push(&mut fields, 3, &packet.floor_authority_sha256);
    push(&mut fields, 4, &write_alpha(&packet.alpha_admission)?);
    push(&mut fields, 5, packet.relation.symbol.as_bytes());
    push(&mut fields, 6, packet.relation.name.as_bytes());
    push(&mut fields, 7, packet.relation.formula.as_bytes());
    for input in &packet.relation.inputs {
        push(&mut fields, 8, input.as_bytes());
    }
    push(&mut fields, 9, packet.relation.unit.as_bytes());
    push(&mut fields, 10, &dimension_wire(packet.relation.dimension));
    for input in &packet.inputs {
        push(&mut fields, 11, &write_constant(input)?);
    }
    push(&mut fields, 12, &write_output(&packet.output)?);
    frame(PACKET_SCHEMA_ID.as_bytes(), fields)
}

fn write_relation(packet: &DerivedRelationPacket) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut fields = BTreeMap::<u16, Vec<Vec<u8>>>::new();
    push(&mut fields, 1, packet.relation.formula.as_bytes());
    push(&mut fields, 2, &dimension_wire(packet.relation.dimension));
    for input in &packet.inputs {
        push(&mut fields, 3, &write_constant(input)?);
    }
    push(&mut fields, 4, &write_output(&packet.output)?);
    push(&mut fields, 5, &packet.floor_authority_sha256);
    push(&mut fields, 6, &write_alpha(&packet.alpha_admission)?);
    frame(ARTIFACT_SCHEMA_ID.as_bytes(), fields)
}

fn write_alpha(admission: &AlphaAdmissionBinding) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut fields = BTreeMap::<u16, Vec<Vec<u8>>>::new();
    push(&mut fields, 1, admission.schema_id.as_bytes());
    push(&mut fields, 2, admission.entry_id.as_bytes());
    for (offset, receipt) in [
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
        push(
            &mut fields,
            u16::try_from(offset + 3).map_err(|_| DerivedRelationRefusal::ResourceLimitExceeded)?,
            &receipt,
        );
    }
    frame(&[], fields)
}

fn write_constant(binding: &ConstantBinding) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut fields = BTreeMap::<u16, Vec<Vec<u8>>>::new();
    push(&mut fields, 1, binding.symbol.as_bytes());
    push(&mut fields, 2, binding.role.id().as_bytes());
    push(&mut fields, 3, &binding.bits.to_be_bytes());
    push(&mut fields, 4, &binding.scale_bits.to_be_bytes());
    push(&mut fields, 5, &binding.projection_receipt_sha256);
    push(&mut fields, 6, binding.unit.as_bytes());
    push(&mut fields, 7, &dimension_wire(binding.dimension));
    push(&mut fields, 8, binding.source_id.as_bytes());
    push(&mut fields, 9, binding.source_sha256.as_bytes());
    push(&mut fields, 10, binding.source_anchor.as_bytes());
    frame(&[], fields)
}

fn write_output(output: &super::model::OutputBinding) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut fields = BTreeMap::<u16, Vec<Vec<u8>>>::new();
    push(&mut fields, 1, output.symbol.as_bytes());
    push(&mut fields, 2, &output.bits.to_be_bytes());
    push(&mut fields, 3, &output.scale_bits.to_be_bytes());
    push(&mut fields, 4, &output.projection_receipt_sha256);
    push(&mut fields, 5, output.unit.as_bytes());
    push(&mut fields, 6, &dimension_wire(output.dimension));
    frame(&[], fields)
}

fn push(fields: &mut BTreeMap<u16, Vec<Vec<u8>>>, tag: u16, value: &[u8]) {
    fields.entry(tag).or_default().push(value.to_vec());
}

fn frame(
    domain: &[u8],
    fields: BTreeMap<u16, Vec<Vec<u8>>>,
) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut bytes = domain.to_vec();
    for (tag, values) in fields {
        for value in values {
            append_field(&mut bytes, tag, &value)?;
        }
    }
    if bytes.len() > MAX_PACKET_BYTES {
        return Err(DerivedRelationRefusal::ResourceLimitExceeded);
    }
    Ok(bytes)
}

fn dimension_wire(dimension: [i8; 7]) -> [u8; 7] {
    let mut bytes = [0; 7];
    for (slot, exponent) in bytes.iter_mut().zip(dimension) {
        *slot = exponent.to_ne_bytes()[0];
    }
    bytes
}

pub(super) fn canary_evidence(
    packet: &DerivedRelationPacket,
) -> Result<CanaryEvidence, DerivedRelationRefusal> {
    let baseline = inspect(packet)?;
    let mut mutants = BTreeMap::new();
    let mut mutant = packet.clone();
    mutant.floor_authority_sha256[0] ^= 1;
    mutants.insert("floor_digest", mutant);
    let mut mutant = packet.clone();
    mutant.alpha_admission.owner_admission_receipt[0] ^= 1;
    mutants.insert("owner_receipt", mutant);
    let mut mutant = packet.clone();
    mutant.relation.formula = "e^2 * (2 * alpha * h * c)".to_owned();
    mutants.insert("formula", mutant);
    let mut mutant = packet.clone();
    mutant.relation.inputs.swap(0, 1);
    mutants.insert("relation_input_order", mutant);
    let mut mutant = packet.clone();
    mutant.inputs.swap(0, 1);
    mutants.insert("input_order", mutant);
    let mut mutant = packet.clone();
    mutant.inputs[1].role = InputRole::RepresentationDefinition;
    mutants.insert("alpha_role", mutant);
    let mut mutant = packet.clone();
    mutant.inputs[0].projection_receipt_sha256[0] ^= 1;
    mutants.insert("input_projection_receipt", mutant);
    let mut mutant = packet.clone();
    mutant.output.bits = mutant.output.bits.saturating_add(1);
    mutants.insert("output_bits", mutant);
    let mut mutant = packet.clone();
    mutant.output.projection_receipt_sha256[0] ^= 1;
    mutants.insert("output_projection_receipt", mutant);
    let mut mutant = packet.clone();
    mutant.relation.dimension[0] = mutant.relation.dimension[0].saturating_add(1);
    mutants.insert("relation_dimension", mutant);
    let mut mutant = packet.clone();
    mutant.output.dimension[0] = mutant.output.dimension[0].saturating_add(1);
    mutants.insert("output_dimension", mutant);
    let mut mutant = packet.clone();
    mutant.output.scale_bits = mutant.output.scale_bits.saturating_add(1);
    mutants.insert("output_scale", mutant);

    let mut transcript = WATCHDOG_CANARY_TRANSCRIPT_ID.as_bytes().to_vec();
    let case_count =
        u32::try_from(mutants.len()).map_err(|_| DerivedRelationRefusal::ResourceLimitExceeded)?;
    if case_count != EXPECTED_CANARY_CASE_COUNT {
        return Err(DerivedRelationRefusal::CanaryFailure);
    }
    append_field(&mut transcript, 1, &case_count.to_be_bytes())?;
    for (name, mutant) in mutants {
        let preimage = write_packet(&mutant)?;
        let observation = inspect(&mutant);
        if observation.as_ref().is_ok_and(|output| output == &baseline) {
            return Err(DerivedRelationRefusal::CanaryFailure);
        }
        append_field(&mut transcript, 2, name.as_bytes())?;
        append_field(&mut transcript, 3, &preimage)?;
        append_field(&mut transcript, 4, &write_observation(&observation)?)?;
    }
    Ok(CanaryEvidence {
        transcript_id: WATCHDOG_CANARY_TRANSCRIPT_ID,
        case_count,
        transcript_sha256: sha256(&transcript),
    })
}

fn write_observation(
    observation: &Result<DerivedRelationCheckerOutput, DerivedRelationRefusal>,
) -> Result<Vec<u8>, DerivedRelationRefusal> {
    let mut fields = BTreeMap::<u16, Vec<Vec<u8>>>::new();
    match observation {
        Ok(output) => {
            for (tag, value) in [
                (1, b"accepted_changed".as_slice()),
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
                push(&mut fields, tag, value);
            }
        }
        Err(error) => {
            push(&mut fields, 1, b"refused");
            push(&mut fields, 2, error.id().as_bytes());
        }
    }
    frame(
        b"civsim.planet.derived-law-premise-eps0-canary-observation.v1",
        fields,
    )
}

pub(super) fn semantic_receipt(
    output: &DerivedRelationCheckerOutput,
    pair_receipt_sha256: [u8; 32],
) -> [u8; 32] {
    digest_fields(
        WATCHDOG_IMPLEMENTATION_ID.as_bytes(),
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
    let mut fields = BTreeMap::<u16, Vec<Vec<u8>>>::new();
    for (tag, value) in [
        (1, super::model::RECEIPT_SCHEMA_ID.as_bytes()),
        (2, output.input_sha256.as_slice()),
        (3, output.claim_identity.0.as_slice()),
        (4, output.role_identity.0.as_slice()),
        (5, output.content_identity.0.as_slice()),
        (6, PRODUCER_IMPLEMENTATION_ID.as_bytes()),
        (7, WATCHDOG_IMPLEMENTATION_ID.as_bytes()),
        (8, producer_result_sha256.as_slice()),
        (9, watchdog_result_sha256.as_slice()),
        (10, CANARY_SUITE_ID.as_bytes()),
        (11, producer_canary.transcript_id.as_bytes()),
        (12, producer_canary.case_count.to_be_bytes().as_slice()),
        (13, producer_canary.transcript_sha256.as_slice()),
        (14, watchdog_canary.transcript_id.as_bytes()),
        (15, watchdog_canary.case_count.to_be_bytes().as_slice()),
        (16, watchdog_canary.transcript_sha256.as_slice()),
        (17, b"agreed"),
        (18, b"planet.derived-law-premise-eps0"),
        (19, b"universal"),
        (20, b"[D]"),
        (21, b"verified_one_claim_scoped_derived_execution_relation"),
    ] {
        push(&mut fields, tag, value);
    }
    frame(
        b"civsim.planet.derived-law-premise-eps0-pair-receipt.v2",
        fields,
    )
    .map(|bytes| sha256(&bytes))
    .unwrap_or([0; 32])
}

pub(super) const fn implementation_id() -> &'static str {
    WATCHDOG_IMPLEMENTATION_ID
}

pub(super) fn result_sha256(output: &DerivedRelationCheckerOutput) -> [u8; 32] {
    let mut fields = BTreeMap::<u16, Vec<Vec<u8>>>::new();
    for (tag, value) in [
        (1, output.input_sha256.as_slice()),
        (2, output.claim_identity.0.as_slice()),
        (3, output.role_identity.0.as_slice()),
        (4, output.content_identity.0.as_slice()),
        (5, output.relation_bytes.as_slice()),
        (6, output.upstream_capability_sha256.as_slice()),
        (7, output.applicability_receipt_sha256.as_slice()),
        (8, output.validity_receipt_sha256.as_slice()),
        (9, output.ancestry_receipt_sha256.as_slice()),
    ] {
        push(&mut fields, tag, value);
    }
    frame(
        b"civsim.planet.derived-law-premise-eps0-watchdog-result.v1",
        fields,
    )
    .map(|bytes| sha256(&bytes))
    .unwrap_or([0; 32])
}
