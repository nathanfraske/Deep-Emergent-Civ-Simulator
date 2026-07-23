//! Conditional physical species proof graph and independent closure pair.
//!
//! The current repository supplies no admitted species derivation roots. The
//! executable production result is therefore a structured refusal. Synthetic
//! tests can exercise the pair, but agreement cannot construct the dormant
//! registry authority or enter conditioned support.

mod model;
mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use crate::canonical::stellar_birth_structure::stellar_birth_structure_schema;
use civsim_units::physics_floor::sealed_physical_floor_authority_binding;
use model::{
    AuthorityEffect, CheckerPairBinding, PhysicalRegistryInput, PhysicalRegistryRefusal,
    PhysicalRegistryRefusalCode, PhysicalRegistryResourceContract, ReceiptBinding,
    StructureAuthorityBinding, VerifiedPhysicalSpeciesRegistry, PRODUCER_ID, PROOF_GRAPH_SCHEMA_ID,
    REGISTRY_SCHEMA_ID, WATCHDOG_ID,
};

fn inspect_physical_registry(
    input: &PhysicalRegistryInput,
) -> Result<VerifiedPhysicalSpeciesRegistry, PhysicalRegistryRefusal> {
    let produced = producer::validate_and_encode(input);
    let watched = watchdog::validate_and_encode(input);
    match (produced, watched) {
        (Ok(produced), Ok(watched))
            if produced.members == watched.members
                && produced.canonical_bytes == watched.canonical_bytes =>
        {
            Ok(VerifiedPhysicalSpeciesRegistry {
                members: produced.members,
                canonical_bytes: produced.canonical_bytes,
                producer_id: PRODUCER_ID,
                watchdog_id: WATCHDOG_ID,
                authority_effect: AuthorityEffect::None,
            })
        }
        (Err(produced), Err(watched)) if produced == watched => {
            Err(PhysicalRegistryRefusal::from_code(produced))
        }
        _ => Err(PhysicalRegistryRefusal::from_code(
            PhysicalRegistryRefusalCode::CheckerDisagreement,
        )),
    }
}

fn repository_input() -> Result<PhysicalRegistryInput, PhysicalRegistryRefusalCode> {
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| PhysicalRegistryRefusalCode::FloorBindingMismatch)?;
    let structure = stellar_birth_structure_schema()
        .map_err(|_| PhysicalRegistryRefusalCode::StructureBindingMismatch)?;
    Ok(PhysicalRegistryInput {
        schema_id: REGISTRY_SCHEMA_ID.to_owned(),
        proof_graph_schema_id: PROOF_GRAPH_SCHEMA_ID.to_owned(),
        floor_binding: ReceiptBinding {
            schema_id: floor.schema_id().as_str().to_owned(),
            digest_sha256: floor.digest(),
        },
        structure_binding: StructureAuthorityBinding {
            structure_schema_id: structure.schema_id.to_owned(),
            species_registry_schema_id: structure.species_registry.schema_id.to_owned(),
            stellar_state_schema_id: structure.stellar_state.schema_id.to_owned(),
            state_coordinate_registry_schema_id: structure
                .stellar_state
                .state_coordinate_registry
                .schema_id
                .to_owned(),
            interaction_sector_registry_schema_id: structure
                .stellar_state
                .interaction_sector_registry
                .schema_id
                .to_owned(),
            physical_regime_registry_schema_id: structure
                .stellar_state
                .physical_regime_registry
                .schema_id
                .to_owned(),
        },
        checker_pair: CheckerPairBinding {
            producer_id: PRODUCER_ID.to_owned(),
            watchdog_id: WATCHDOG_ID.to_owned(),
        },
        resources: PhysicalRegistryResourceContract::PRODUCTION,
        admitted_artifacts: Vec::new(),
        declared_members: Vec::new(),
    })
}

fn resolve_repository_physical_species_registry(
) -> Result<VerifiedPhysicalSpeciesRegistry, PhysicalRegistryRefusal> {
    repository_input()
        .map_err(PhysicalRegistryRefusal::from_code)
        .and_then(|input| inspect_physical_registry(&input))
}
