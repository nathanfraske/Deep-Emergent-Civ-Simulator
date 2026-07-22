//! Value-free structural contract for the stellar-birth joint measure.
//!
//! The module family separates component identity, shared index domains, and
//! carrier shapes. It supplies no magnitudes, realized components, proof
//! constructors, or fallback state.

mod carrier;
mod component_registry;
mod index_domain;
mod species_registry;
mod view;
mod wire;

pub(crate) use carrier::{CarrierKind, CarrierSchema};
pub(crate) use component_registry::ComponentRegistrySchema;
pub(crate) use index_domain::IndexDomain;
pub(crate) use species_registry::SpeciesRegistrySchema;
pub use view::{
    CarrierSchemaView, ComponentRegistrySchemaView, IndexDomainView, SpeciesRegistrySchemaView,
};
pub(in crate::canonical) use wire::write_stellar_birth_structure;

use carrier::canonical_carrier_schemas;
use component_registry::{canonical_component_registry_schema, COMPONENT_REGISTRY_SCHEMA_ID};
use index_domain::canonical_index_domains;
use species_registry::{canonical_species_registry_schema, SPECIES_REGISTRY_SCHEMA_ID};
use std::{collections::BTreeSet, fmt};

pub(super) const STELLAR_BIRTH_STRUCTURE_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-structure.v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct StellarBirthStructureSchema {
    pub(super) schema_id: &'static str,
    pub(super) component_registry: ComponentRegistrySchema,
    pub(super) species_registry: SpeciesRegistrySchema,
    pub(super) index_domains: Vec<IndexDomain>,
    pub(super) carrier_schemas: Vec<CarrierSchema>,
}

impl StellarBirthStructureSchema {
    pub(super) fn carrier(&self, kind: CarrierKind) -> Option<&CarrierSchema> {
        self.carrier_schemas
            .iter()
            .find(|carrier| carrier.kind == kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StructureSchemaError {
    SchemaIdentityMismatch,
    ComponentRegistryMismatch,
    SpeciesRegistryMismatch,
    DuplicateIndexDomain(String),
    DuplicateCarrier(String),
    CatalogCardinalityMismatch {
        collection: &'static str,
        expected: usize,
        found: usize,
    },
    NoncanonicalOrder {
        collection: &'static str,
    },
    UnknownCarrierDomain {
        carrier_id: String,
        domain_id: String,
    },
    DuplicateCarrierDomain {
        carrier_id: String,
        domain_id: String,
    },
    NoncanonicalCarrier(String),
    NoncanonicalIndexDomain(String),
}

impl fmt::Display for StructureSchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaIdentityMismatch => f.write_str("stellar-birth structure schema mismatch"),
            Self::ComponentRegistryMismatch => write!(
                f,
                "component registry does not match sealed schema '{COMPONENT_REGISTRY_SCHEMA_ID}'"
            ),
            Self::SpeciesRegistryMismatch => write!(
                f,
                "species registry does not match sealed schema '{SPECIES_REGISTRY_SCHEMA_ID}'"
            ),
            Self::DuplicateIndexDomain(id) => {
                write!(f, "duplicate stellar-birth index domain '{id}'")
            }
            Self::DuplicateCarrier(id) => write!(f, "duplicate stellar-birth carrier '{id}'"),
            Self::CatalogCardinalityMismatch {
                collection,
                expected,
                found,
            } => write!(
                f,
                "stellar-birth {collection} catalog contains {found} entries; expected exactly {expected}"
            ),
            Self::NoncanonicalOrder { collection } => {
                write!(f, "stellar-birth {collection} are not identity-canonical")
            }
            Self::UnknownCarrierDomain {
                carrier_id,
                domain_id,
            } => write!(
                f,
                "carrier '{carrier_id}' references unknown index domain '{domain_id}'"
            ),
            Self::DuplicateCarrierDomain {
                carrier_id,
                domain_id,
            } => write!(
                f,
                "carrier '{carrier_id}' repeats index domain '{domain_id}'"
            ),
            Self::NoncanonicalCarrier(id) => {
                write!(
                    f,
                    "carrier '{id}' does not match its sealed structural contract"
                )
            }
            Self::NoncanonicalIndexDomain(id) => write!(
                f,
                "index domain '{id}' does not match its sealed structural contract"
            ),
        }
    }
}

impl std::error::Error for StructureSchemaError {}

pub(super) fn stellar_birth_structure_schema(
) -> Result<StellarBirthStructureSchema, StructureSchemaError> {
    let schema = StellarBirthStructureSchema {
        schema_id: STELLAR_BIRTH_STRUCTURE_SCHEMA_ID,
        component_registry: canonical_component_registry_schema(),
        species_registry: canonical_species_registry_schema(),
        index_domains: canonical_index_domains(),
        carrier_schemas: canonical_carrier_schemas(),
    };
    validate_structure_schema(&schema)?;
    Ok(schema)
}

fn validate_structure_schema(
    schema: &StellarBirthStructureSchema,
) -> Result<(), StructureSchemaError> {
    if schema.schema_id != STELLAR_BIRTH_STRUCTURE_SCHEMA_ID {
        return Err(StructureSchemaError::SchemaIdentityMismatch);
    }
    if !schema.component_registry.is_canonical() {
        return Err(StructureSchemaError::ComponentRegistryMismatch);
    }
    if !schema.species_registry.is_canonical() {
        return Err(StructureSchemaError::SpeciesRegistryMismatch);
    }
    let mut domain_ids = BTreeSet::new();
    for domain in &schema.index_domains {
        if !domain_ids.insert(domain.id) {
            return Err(StructureSchemaError::DuplicateIndexDomain(
                domain.id.to_owned(),
            ));
        }
        if !domain.is_canonical() {
            return Err(StructureSchemaError::NoncanonicalIndexDomain(
                domain.id.to_owned(),
            ));
        }
    }
    if !schema
        .index_domains
        .windows(2)
        .all(|pair| pair[0].id < pair[1].id)
    {
        return Err(StructureSchemaError::NoncanonicalOrder {
            collection: "index domains",
        });
    }
    let expected_domain_count = canonical_index_domains().len();
    if schema.index_domains.len() != expected_domain_count {
        return Err(StructureSchemaError::CatalogCardinalityMismatch {
            collection: "index-domain",
            expected: expected_domain_count,
            found: schema.index_domains.len(),
        });
    }

    let mut carrier_ids = BTreeSet::new();
    for carrier in &schema.carrier_schemas {
        if !carrier_ids.insert(carrier.id()) {
            return Err(StructureSchemaError::DuplicateCarrier(
                carrier.id().to_owned(),
            ));
        }
        let mut referenced_domains = BTreeSet::new();
        for domain_id in &carrier.index_domain_ids {
            if !domain_ids.contains(domain_id) {
                return Err(StructureSchemaError::UnknownCarrierDomain {
                    carrier_id: carrier.id().to_owned(),
                    domain_id: (*domain_id).to_owned(),
                });
            }
            if !referenced_domains.insert(*domain_id) {
                return Err(StructureSchemaError::DuplicateCarrierDomain {
                    carrier_id: carrier.id().to_owned(),
                    domain_id: (*domain_id).to_owned(),
                });
            }
        }
        if !carrier.is_canonical() {
            return Err(StructureSchemaError::NoncanonicalCarrier(
                carrier.id().to_owned(),
            ));
        }
    }
    if !schema
        .carrier_schemas
        .windows(2)
        .all(|pair| pair[0].id() < pair[1].id())
    {
        return Err(StructureSchemaError::NoncanonicalOrder {
            collection: "carrier schemas",
        });
    }
    let expected_carrier_count = canonical_carrier_schemas().len();
    if schema.carrier_schemas.len() != expected_carrier_count {
        return Err(StructureSchemaError::CatalogCardinalityMismatch {
            collection: "carrier",
            expected: expected_carrier_count,
            found: schema.carrier_schemas.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::stellar_birth_structure::carrier::{
        CarrierMeasureSemantics, CarrierNormalization, CarrierValueShape,
    };

    #[test]
    fn structure_catalogs_are_canonical_and_variable_cardinality() {
        let schema = stellar_birth_structure_schema().unwrap();
        assert_eq!(schema.index_domains.len(), 6);
        assert_eq!(schema.carrier_schemas.len(), 11);
        assert_eq!(
            schema
                .index_domains
                .iter()
                .map(|domain| domain.id)
                .collect::<Vec<_>>(),
            [
                "stellar_birth.domain.component_identity",
                "stellar_birth.domain.evolution_time",
                "stellar_birth.domain.log_frequency",
                "stellar_birth.domain.material_element_identity",
                "stellar_birth.domain.spatial_position",
                "stellar_birth.domain.species_identity",
            ]
        );
        assert_eq!(
            schema
                .carrier_schemas
                .iter()
                .map(CarrierSchema::id)
                .collect::<Vec<_>>(),
            [
                "material_time_scalar_history",
                "material_time_vector_history",
                "scalar",
                "spatial_scalar_field",
                "spatial_vector_field",
                "species_number_fraction_simplex",
                "spectral_flux_density_per_log_frequency_field",
                "time_scalar_history",
                "variable_cardinality_topology",
                "variable_component_scalar_field",
                "variable_component_vector_field",
            ]
        );
        assert_eq!(
            schema.component_registry.cardinality_rule.id(),
            "realization_coordinate_defined_from_joint_measure_support"
        );
        assert_eq!(
            schema.component_registry.capacity_rule.id(),
            "engine_limit_is_named_refusal"
        );
        assert_eq!(
            schema.component_registry.ordinal_rule.id(),
            "serialization_only_never_identity_or_coordinate"
        );
        assert_eq!(
            schema.component_registry.encoding_rule.id(),
            "canonical_physical_content_bytes"
        );
        assert_eq!(
            schema.component_registry.collision_rule.id(),
            "symmetry_class_or_named_hash_collision_refusal"
        );
        assert_eq!(
            schema.component_registry.symmetry_rule.id(),
            "permutation_equivariant_multiset"
        );
        assert_eq!(
            schema.component_registry.topology_label_authority_rule.id(),
            "derived_physical_relation_only"
        );
        assert_eq!(
            schema.species_registry.membership_rule.id(),
            "floor_derived_only_or_named_refusal"
        );
    }

    #[test]
    fn spectral_species_and_topology_carriers_bind_their_exact_domains() {
        let schema = stellar_birth_structure_schema().unwrap();
        let spectral = schema
            .carrier(CarrierKind::SpectralFluxDensityPerLogFrequencyField)
            .unwrap();
        assert_eq!(
            spectral.index_domain_ids,
            [
                index_domain::SPATIAL_POSITION_DOMAIN_ID,
                index_domain::LOG_FREQUENCY_DOMAIN_ID,
            ]
        );
        assert_eq!(
            spectral.measure_semantics,
            CarrierMeasureSemantics::DensityPerNaturalLogFrequency
        );

        let species = schema
            .carrier(CarrierKind::SpeciesNumberFractionSimplex)
            .unwrap();
        assert_eq!(species.value_shape, CarrierValueShape::SimplexCoordinate);
        assert_eq!(
            species.normalization,
            CarrierNormalization::UnitSumPerSpatialCoordinate
        );
        assert_eq!(
            species.support_rule,
            carrier::CarrierSupportRule::CompleteRegistrySupportOrNamedRefusal
        );
        assert_eq!(
            species.measure_semantics,
            CarrierMeasureSemantics::NumberFractionOverCompleteSpeciesRegistry
        );

        let topology = schema
            .carrier(CarrierKind::VariableCardinalityTopology)
            .unwrap();
        assert_eq!(topology.value_shape, CarrierValueShape::LabeledHypergraph);
        assert_eq!(
            topology.index_domain_ids,
            [index_domain::COMPONENT_IDENTITY_DOMAIN_ID]
        );
    }

    #[test]
    fn a_structural_mutation_fails_closed() {
        let mut schema = stellar_birth_structure_schema().unwrap();
        schema.carrier_schemas[0]
            .index_domain_ids
            .push("stellar_birth.domain.unknown");
        assert!(matches!(
            validate_structure_schema(&schema),
            Err(StructureSchemaError::NoncanonicalCarrier(_))
                | Err(StructureSchemaError::UnknownCarrierDomain { .. })
        ));
    }

    #[test]
    fn missing_duplicate_reordered_and_mutated_catalog_entries_fail_closed() {
        let mut missing = stellar_birth_structure_schema().unwrap();
        missing.index_domains.clear();
        assert!(matches!(
            validate_structure_schema(&missing),
            Err(StructureSchemaError::CatalogCardinalityMismatch {
                collection: "index-domain",
                ..
            })
        ));

        let mut duplicate = stellar_birth_structure_schema().unwrap();
        duplicate
            .carrier_schemas
            .push(duplicate.carrier_schemas[0].clone());
        assert!(matches!(
            validate_structure_schema(&duplicate),
            Err(StructureSchemaError::DuplicateCarrier(_))
        ));

        let mut reordered = stellar_birth_structure_schema().unwrap();
        reordered.index_domains.swap(0, 1);
        assert!(matches!(
            validate_structure_schema(&reordered),
            Err(StructureSchemaError::NoncanonicalOrder {
                collection: "index domains"
            })
        ));

        let mut mutated = stellar_birth_structure_schema().unwrap();
        mutated.index_domains[0].coordinate_dimension[0] = 1;
        assert!(matches!(
            validate_structure_schema(&mutated),
            Err(StructureSchemaError::NoncanonicalIndexDomain(_))
        ));
    }

    #[test]
    fn every_variable_domain_refuses_authored_resolution_and_capacity_truncation() {
        let schema = stellar_birth_structure_schema().unwrap();
        for domain in &schema.index_domains {
            assert_eq!(domain.capacity_rule.id(), "engine_limit_is_named_refusal");
            assert_eq!(
                domain.ordinal_rule.id(),
                "serialization_only_never_identity_or_coordinate"
            );
            assert!(matches!(
                domain.resolution_rule,
                index_domain::DomainResolutionRule::ExactRegistryMembership
                    | index_domain::DomainResolutionRule::ConvergenceDerivedOrNamedRefusal
            ));
        }
        let log_frequency = schema
            .index_domains
            .iter()
            .find(|domain| domain.id == index_domain::LOG_FREQUENCY_DOMAIN_ID)
            .unwrap();
        assert_eq!(
            log_frequency.reference_rule.id(),
            "log_ratio_reference_is_gauge_shift"
        );
    }

    #[test]
    fn structure_schema_contains_no_familiar_world_or_authored_selector() {
        let debug = format!("{:?}", stellar_birth_structure_schema().unwrap()).to_lowercase();
        for forbidden in [
            "earth",
            "solar",
            "hydrogen",
            "helium",
            "primary",
            "secondary",
            "binary",
            "caller",
            "citation",
            "seed",
        ] {
            assert!(
                !debug.contains(forbidden),
                "found forbidden token '{forbidden}'"
            );
        }
    }
}
