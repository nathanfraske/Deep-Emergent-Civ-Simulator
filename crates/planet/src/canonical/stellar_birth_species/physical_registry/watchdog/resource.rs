use super::super::model::{
    PhysicalRegistryRefusalCode, PhysicalRegistryResourceContract, ValidationCaps,
};
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
    let contract = PRODUCTION_RESOURCE_CONTRACT;
    let values = [
        contract.max_artifact_count.to_be_bytes().to_vec(),
        contract.max_registry_member_count.to_be_bytes().to_vec(),
        contract.max_references_per_artifact.to_be_bytes().to_vec(),
        contract.max_total_reference_count.to_be_bytes().to_vec(),
        contract.max_expression_node_count.to_be_bytes().to_vec(),
        contract.max_expression_edge_count.to_be_bytes().to_vec(),
        contract.max_expression_depth.to_be_bytes().to_vec(),
        contract.max_rational_component_bits.to_be_bytes().to_vec(),
        contract
            .max_intermediate_component_bits
            .to_be_bytes()
            .to_vec(),
        contract.max_dimension_abs_exponent.to_be_bytes().to_vec(),
        contract.max_dimension_term_count.to_be_bytes().to_vec(),
        contract.max_evaluation_steps.to_be_bytes().to_vec(),
        contract.max_closure_steps.to_be_bytes().to_vec(),
        contract.max_canonical_bytes.to_be_bytes().to_vec(),
        contract.max_canonical_token_bytes.to_be_bytes().to_vec(),
        contract.max_content_bytes.to_be_bytes().to_vec(),
        work_profile_id.to_vec(),
        ARTIFACT_IDENTITY_UNITS.to_be_bytes().to_vec(),
        MEMBER_IDENTITY_UNITS.to_be_bytes().to_vec(),
        EXPRESSION_SHAPE_NODE_UNITS.to_be_bytes().to_vec(),
        EXPRESSION_SHAPE_EDGE_UNITS.to_be_bytes().to_vec(),
        EXPRESSION_NODE_EXECUTION_UNITS.to_be_bytes().to_vec(),
        COMPONENT_DECODE_BYTE_UNITS.to_be_bytes().to_vec(),
        ADDITIVE_RATIONAL_CORE_UNITS.to_be_bytes().to_vec(),
        MULTIPLICATIVE_RATIONAL_CORE_UNITS.to_be_bytes().to_vec(),
        BOUNDED_PRODUCT_UNITS.to_be_bytes().to_vec(),
        RATIONAL_REDUCTION_UNITS.to_be_bytes().to_vec(),
        REGISTRY_ARTIFACT_UNITS.to_be_bytes().to_vec(),
        POWER_COMPONENT_FACTOR.to_be_bytes().to_vec(),
        CLOSURE_GRAPH_PHASES.to_be_bytes().to_vec(),
        CLOSURE_DECLARATION_UNITS.to_be_bytes().to_vec(),
        POWER_WORK_FORMULA_ID.to_vec(),
        CLOSURE_WORK_FORMULA_ID.to_vec(),
    ];
    let mut bytes = RESOURCE_CONTRACT_DOMAIN.to_vec();
    for (index, value) in values.iter().enumerate() {
        let tag = index as u16 + 1;
        bytes.extend_from_slice(&tag.to_be_bytes());
        bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
        bytes.extend_from_slice(value);
    }
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

pub(super) fn power_work_units(mut exponent: u32) -> Result<u64, PhysicalRegistryRefusalCode> {
    let mut units = 0_u64;
    while exponent != 0 {
        units = units
            .checked_add(POWER_COMPONENT_FACTOR)
            .ok_or(PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded)?;
        if exponent & 1 == 1 {
            units = units
                .checked_add(POWER_COMPONENT_FACTOR)
                .ok_or(PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded)?;
        }
        exponent >>= 1;
    }
    Ok(units)
}

pub(super) fn expression_work_units(
    node_count: u32,
    edge_count: u32,
) -> Result<u64, PhysicalRegistryRefusalCode> {
    [node_count, edge_count]
        .into_iter()
        .zip([EXPRESSION_SHAPE_NODE_UNITS, EXPRESSION_SHAPE_EDGE_UNITS])
        .try_fold(0_u64, |total, (count, weight)| {
            let weighted = u64::from(count)
                .checked_mul(weight)
                .ok_or(PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded)?;
            total
                .checked_add(weighted)
                .ok_or(PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded)
        })
}

pub(super) fn component_decode_work_units(
    byte_count: usize,
) -> Result<u64, PhysicalRegistryRefusalCode> {
    let mut units = 0_u64;
    for _ in 0..byte_count {
        units = units
            .checked_add(COMPONENT_DECODE_BYTE_UNITS)
            .ok_or(PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded)?;
    }
    Ok(units)
}

pub(super) const fn rational_reduction_work_units(numerator_is_zero: bool) -> u64 {
    match numerator_is_zero {
        true => 0,
        false => RATIONAL_REDUCTION_UNITS,
    }
}

pub(super) fn closure_work_units(
    rule_count: usize,
    dependency_count: usize,
    declared_count: usize,
) -> Result<u64, PhysicalRegistryRefusalCode> {
    let mut total = 0_u64;
    for _ in 0..CLOSURE_GRAPH_PHASES {
        for count in [rule_count, dependency_count] {
            let count = u64::try_from(count)
                .map_err(|_| PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)?;
            total = total
                .checked_add(count)
                .ok_or(PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)?;
        }
    }
    let declarations = u64::try_from(declared_count)
        .map_err(|_| PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)?
        .checked_mul(CLOSURE_DECLARATION_UNITS)
        .ok_or(PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)?;
    total
        .checked_add(declarations)
        .ok_or(PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)
}

pub(super) fn product_fits_component_cap(left: &BigUint, right: &BigUint, cap: u32) -> bool {
    if left.is_zero() || right.is_zero() {
        return true;
    }
    if cap == 0 {
        return false;
    }
    let guaranteed_bits = left
        .bit_len()
        .saturating_add(right.bit_len())
        .saturating_sub(1);
    if guaranteed_bits > cap {
        return false;
    }
    if left.bit_len().saturating_add(right.bit_len()) <= cap {
        return true;
    }

    let unit = BigUint::from_u64(1);
    let boundary_half = unit.shl_bits(cap - 1);
    let limit = boundary_half.sub(&unit).add(&boundary_half);
    let (largest_right, _) = limit.divmod(left);
    right.cmp_big(&largest_right) != Ordering::Greater
}
