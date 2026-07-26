use super::super::model::{PhysicalRegistryResourceContract, ValidationCaps};
use civsim_units::{bignum::BigUint, digest::sha256};
use std::cmp::Ordering;

const RESOURCE_CONTRACT_DOMAIN: &[u8] = b"civsim.physical-species.resource-contract.v2";
const SEMANTIC_WORK_PROFILE_ID: &[u8] = b"civsim.physical-species.semantic-work-profile.v1";
const POWER_WORK_FORMULA_ID: &[u8] = b"two-components-times-bit-width-plus-population.v1";
const CLOSURE_WORK_FORMULA_ID: &[u8] = b"acyclicity-v-plus-e;closure-v-plus-e;declaration-d.v1";
const ARTIFACT_IDENTITY_UNITS: u64 = 1;
const MEMBER_IDENTITY_UNITS: u64 = 1;
const EXPRESSION_SHAPE_NODE_UNITS: u64 = 1;
const EXPRESSION_SHAPE_EDGE_UNITS: u64 = 1;
const EXPRESSION_NODE_EXECUTION_UNITS: u64 = 1;
const COMPONENT_DECODE_BYTE_UNITS: u64 = 1;
const ADDITIVE_RATIONAL_CORE_UNITS: u64 = 4;
const MULTIPLICATIVE_RATIONAL_CORE_UNITS: u64 = 6;
const BOUNDED_PRODUCT_UNITS: u64 = 1;
const RATIONAL_REDUCTION_UNITS: u64 = 3;
const REGISTRY_ARTIFACT_UNITS: u64 = 1;
const POWER_COMPONENT_FACTOR: u64 = 2;
const CLOSURE_GRAPH_PHASES: u64 = 2;
const CLOSURE_DECLARATION_UNITS: u64 = 1;

pub(super) const PRODUCTION_CAPS: ValidationCaps = ValidationCaps {
    artifact_count: 4_096,
    registry_member_count: 4_096,
    references_per_artifact: 4_096,
    total_reference_count: 65_536,
    expression_node_count: 65_536,
    expression_edge_count: 131_072,
    expression_depth: 1_024,
    rational_component_bits: 4_096,
    intermediate_component_bits: 65_536,
    dimension_abs_exponent: 4_096,
    dimension_term_count: 64,
    evaluation_steps: 1_000_000,
    closure_steps: 1_000_000,
    canonical_bytes: 16_777_216,
    canonical_token_bytes: 192,
    content_bytes: 1_048_576,
};

pub(super) const PRODUCTION_RESOURCE_CONTRACT: PhysicalRegistryResourceContract =
    resource_contract(PRODUCTION_CAPS);

const fn resource_contract(caps: ValidationCaps) -> PhysicalRegistryResourceContract {
    PhysicalRegistryResourceContract {
        max_artifact_count: caps.artifact_count,
        max_registry_member_count: caps.registry_member_count,
        max_references_per_artifact: caps.references_per_artifact,
        max_total_reference_count: caps.total_reference_count,
        max_expression_node_count: caps.expression_node_count,
        max_expression_edge_count: caps.expression_edge_count,
        max_expression_depth: caps.expression_depth,
        max_rational_component_bits: caps.rational_component_bits,
        max_intermediate_component_bits: caps.intermediate_component_bits,
        max_dimension_abs_exponent: caps.dimension_abs_exponent,
        max_dimension_term_count: caps.dimension_term_count,
        max_evaluation_steps: caps.evaluation_steps,
        max_closure_steps: caps.closure_steps,
        max_canonical_bytes: caps.canonical_bytes,
        max_canonical_token_bytes: caps.canonical_token_bytes,
        max_content_bytes: caps.content_bytes,
    }
}

pub(super) fn contract_sha256() -> [u8; 32] {
    contract_sha256_with_profile(SEMANTIC_WORK_PROFILE_ID)
}

fn contract_sha256_with_profile(work_profile_id: &[u8]) -> [u8; 32] {
    fn field(bytes: &mut Vec<u8>, tag: u16, value: &[u8]) {
        bytes.extend_from_slice(&tag.to_be_bytes());
        bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
        bytes.extend_from_slice(value);
    }

    let contract = PRODUCTION_RESOURCE_CONTRACT;
    let mut bytes = RESOURCE_CONTRACT_DOMAIN.to_vec();
    field(&mut bytes, 1, &contract.max_artifact_count.to_be_bytes());
    field(
        &mut bytes,
        2,
        &contract.max_registry_member_count.to_be_bytes(),
    );
    field(
        &mut bytes,
        3,
        &contract.max_references_per_artifact.to_be_bytes(),
    );
    field(
        &mut bytes,
        4,
        &contract.max_total_reference_count.to_be_bytes(),
    );
    field(
        &mut bytes,
        5,
        &contract.max_expression_node_count.to_be_bytes(),
    );
    field(
        &mut bytes,
        6,
        &contract.max_expression_edge_count.to_be_bytes(),
    );
    field(&mut bytes, 7, &contract.max_expression_depth.to_be_bytes());
    field(
        &mut bytes,
        8,
        &contract.max_rational_component_bits.to_be_bytes(),
    );
    field(
        &mut bytes,
        9,
        &contract.max_intermediate_component_bits.to_be_bytes(),
    );
    field(
        &mut bytes,
        10,
        &contract.max_dimension_abs_exponent.to_be_bytes(),
    );
    field(
        &mut bytes,
        11,
        &contract.max_dimension_term_count.to_be_bytes(),
    );
    field(&mut bytes, 12, &contract.max_evaluation_steps.to_be_bytes());
    field(&mut bytes, 13, &contract.max_closure_steps.to_be_bytes());
    field(&mut bytes, 14, &contract.max_canonical_bytes.to_be_bytes());
    field(
        &mut bytes,
        15,
        &contract.max_canonical_token_bytes.to_be_bytes(),
    );
    field(&mut bytes, 16, &contract.max_content_bytes.to_be_bytes());
    field(&mut bytes, 17, work_profile_id);
    for (tag, units) in [
        (18, ARTIFACT_IDENTITY_UNITS),
        (19, MEMBER_IDENTITY_UNITS),
        (20, EXPRESSION_SHAPE_NODE_UNITS),
        (21, EXPRESSION_SHAPE_EDGE_UNITS),
        (22, EXPRESSION_NODE_EXECUTION_UNITS),
        (23, COMPONENT_DECODE_BYTE_UNITS),
        (24, ADDITIVE_RATIONAL_CORE_UNITS),
        (25, MULTIPLICATIVE_RATIONAL_CORE_UNITS),
        (26, BOUNDED_PRODUCT_UNITS),
        (27, RATIONAL_REDUCTION_UNITS),
        (28, REGISTRY_ARTIFACT_UNITS),
        (29, POWER_COMPONENT_FACTOR),
        (30, CLOSURE_GRAPH_PHASES),
        (31, CLOSURE_DECLARATION_UNITS),
    ] {
        field(&mut bytes, tag, &units.to_be_bytes());
    }
    field(&mut bytes, 32, POWER_WORK_FORMULA_ID);
    field(&mut bytes, 33, CLOSURE_WORK_FORMULA_ID);
    sha256(&bytes)
}

#[cfg(test)]
pub(super) fn contract_sha256_with_profile_for_test(work_profile_id: &[u8]) -> [u8; 32] {
    contract_sha256_with_profile(work_profile_id)
}

pub(super) const fn artifact_identity_work_units() -> u64 {
    ARTIFACT_IDENTITY_UNITS
}

pub(super) const fn member_identity_work_units() -> u64 {
    MEMBER_IDENTITY_UNITS
}

pub(super) const fn expression_node_execution_work_units() -> u64 {
    EXPRESSION_NODE_EXECUTION_UNITS
}

pub(super) const fn additive_rational_core_work_units() -> u64 {
    ADDITIVE_RATIONAL_CORE_UNITS
}

pub(super) const fn multiplicative_rational_core_work_units() -> u64 {
    MULTIPLICATIVE_RATIONAL_CORE_UNITS
}

pub(super) const fn bounded_product_work_units() -> u64 {
    BOUNDED_PRODUCT_UNITS
}

pub(super) const fn registry_artifact_work_units() -> u64 {
    REGISTRY_ARTIFACT_UNITS
}

pub(super) fn power_work_units(
    exponent: u32,
) -> Result<u64, super::super::model::PhysicalRegistryRefusalCode> {
    let width = u64::from(u32::BITS - exponent.leading_zeros());
    let populated = u64::from(exponent.count_ones());
    width
        .checked_add(populated)
        .and_then(|units| units.checked_mul(POWER_COMPONENT_FACTOR))
        .ok_or(super::super::model::PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded)
}

pub(super) fn expression_work_units(
    node_count: u32,
    edge_count: u32,
) -> Result<u64, super::super::model::PhysicalRegistryRefusalCode> {
    u64::from(node_count)
        .checked_mul(EXPRESSION_SHAPE_NODE_UNITS)
        .and_then(|nodes| {
            u64::from(edge_count)
                .checked_mul(EXPRESSION_SHAPE_EDGE_UNITS)
                .and_then(|edges| nodes.checked_add(edges))
        })
        .ok_or(super::super::model::PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded)
}

pub(super) fn component_decode_work_units(
    byte_count: usize,
) -> Result<u64, super::super::model::PhysicalRegistryRefusalCode> {
    u64::try_from(byte_count)
        .ok()
        .and_then(|bytes| bytes.checked_mul(COMPONENT_DECODE_BYTE_UNITS))
        .ok_or(super::super::model::PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded)
}

pub(super) const fn rational_reduction_work_units(numerator_is_zero: bool) -> u64 {
    if numerator_is_zero {
        0
    } else {
        RATIONAL_REDUCTION_UNITS
    }
}

pub(super) fn closure_work_units(
    rule_count: usize,
    dependency_count: usize,
    declared_count: usize,
) -> Result<u64, super::super::model::PhysicalRegistryRefusalCode> {
    let rules = u64::try_from(rule_count)
        .map_err(|_| super::super::model::PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)?;
    let dependencies = u64::try_from(dependency_count)
        .map_err(|_| super::super::model::PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)?;
    let declared = u64::try_from(declared_count)
        .map_err(|_| super::super::model::PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)?;
    rules
        .checked_add(dependencies)
        .and_then(|structure| structure.checked_mul(CLOSURE_GRAPH_PHASES))
        .and_then(|phases| {
            declared
                .checked_mul(CLOSURE_DECLARATION_UNITS)
                .and_then(|declarations| phases.checked_add(declarations))
        })
        .ok_or(super::super::model::PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)
}

pub(super) fn product_fits_component_cap(left: &BigUint, right: &BigUint, cap: u32) -> bool {
    if left.is_zero() || right.is_zero() {
        return true;
    }
    if cap == 0 {
        return false;
    }
    let possible_bits = u64::from(left.bit_len()).saturating_add(u64::from(right.bit_len()));
    let cap_u64 = u64::from(cap);
    if possible_bits <= cap_u64 {
        return true;
    }
    if possible_bits > cap_u64.saturating_add(1) {
        return false;
    }

    let one = BigUint::from_u64(1);
    let highest = one.shl_bits(cap - 1);
    let maximum = highest.add(&highest.sub(&one));
    let (largest_left, _) = maximum.divmod(right);
    left.cmp_big(&largest_left) != Ordering::Greater
}
