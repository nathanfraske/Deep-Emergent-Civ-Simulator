//! Immutable public views over the value-free stellar-birth structure.

use super::{CarrierSchema, ComponentRegistrySchema, IndexDomain, SpeciesRegistrySchema};

/// Read-only view of the variable-cardinality component registry contract.
#[derive(Debug, Clone, Copy)]
pub struct ComponentRegistrySchemaView<'a> {
    schema: &'a ComponentRegistrySchema,
}

/// Read-only view of the floor-derived species registry contract.
#[derive(Debug, Clone, Copy)]
pub struct SpeciesRegistrySchemaView<'a> {
    schema: &'a SpeciesRegistrySchema,
}

/// Read-only view of one shared field or history index domain.
#[derive(Debug, Clone, Copy)]
pub struct IndexDomainView<'a> {
    domain: &'a IndexDomain,
}

/// Read-only view of one value-free carrier-shape contract.
#[derive(Debug, Clone, Copy)]
pub struct CarrierSchemaView<'a> {
    schema: &'a CarrierSchema,
}

impl<'a> ComponentRegistrySchemaView<'a> {
    pub(in crate::canonical) const fn new(schema: &'a ComponentRegistrySchema) -> Self {
        Self { schema }
    }

    pub fn schema_id(self) -> &'static str {
        self.schema.schema_id
    }

    pub fn cardinality_rule_id(self) -> &'static str {
        self.schema.cardinality_rule.id()
    }

    pub fn identity_rule_id(self) -> &'static str {
        self.schema.identity_rule.id()
    }

    pub fn ordering_rule_id(self) -> &'static str {
        self.schema.ordering_rule.id()
    }

    pub fn topology_rule_id(self) -> &'static str {
        self.schema.topology_rule.id()
    }

    pub fn capacity_rule_id(self) -> &'static str {
        self.schema.capacity_rule.id()
    }

    pub fn ordinal_rule_id(self) -> &'static str {
        self.schema.ordinal_rule.id()
    }

    pub fn encoding_rule_id(self) -> &'static str {
        self.schema.encoding_rule.id()
    }

    pub fn collision_rule_id(self) -> &'static str {
        self.schema.collision_rule.id()
    }

    pub fn symmetry_rule_id(self) -> &'static str {
        self.schema.symmetry_rule.id()
    }

    pub fn topology_label_authority_rule_id(self) -> &'static str {
        self.schema.topology_label_authority_rule.id()
    }
}

impl<'a> SpeciesRegistrySchemaView<'a> {
    pub(in crate::canonical) const fn new(schema: &'a SpeciesRegistrySchema) -> Self {
        Self { schema }
    }

    pub fn schema_id(self) -> &'static str {
        self.schema.schema_id
    }

    pub fn membership_rule_id(self) -> &'static str {
        self.schema.membership_rule.id()
    }

    pub fn identity_rule_id(self) -> &'static str {
        self.schema.identity_rule.id()
    }

    pub fn ordering_rule_id(self) -> &'static str {
        self.schema.ordering_rule.id()
    }

    pub fn capacity_rule_id(self) -> &'static str {
        self.schema.capacity_rule.id()
    }

    pub fn ordinal_rule_id(self) -> &'static str {
        self.schema.ordinal_rule.id()
    }
}

impl<'a> IndexDomainView<'a> {
    pub(in crate::canonical) const fn new(domain: &'a IndexDomain) -> Self {
        Self { domain }
    }

    pub fn id(self) -> &'static str {
        self.domain.id
    }

    pub fn kind_id(self) -> &'static str {
        self.domain.kind.id()
    }

    pub fn support_rule_id(self) -> &'static str {
        self.domain.support_rule.id()
    }

    pub fn resolution_rule_id(self) -> &'static str {
        self.domain.resolution_rule.id()
    }

    pub fn ordering_rule_id(self) -> &'static str {
        self.domain.ordering_rule.id()
    }

    pub fn capacity_rule_id(self) -> &'static str {
        self.domain.capacity_rule.id()
    }

    pub fn ordinal_rule_id(self) -> &'static str {
        self.domain.ordinal_rule.id()
    }

    pub fn reference_rule_id(self) -> &'static str {
        self.domain.reference_rule.id()
    }

    pub fn coordinate_dimension(self) -> [i8; 7] {
        self.domain.coordinate_dimension
    }
}

impl<'a> CarrierSchemaView<'a> {
    pub(in crate::canonical) const fn new(schema: &'a CarrierSchema) -> Self {
        Self { schema }
    }

    pub fn id(self) -> &'static str {
        self.schema.id()
    }

    pub fn value_shape_id(self) -> &'static str {
        self.schema.value_shape.id()
    }

    pub fn index_domain_ids(self) -> &'a [&'static str] {
        &self.schema.index_domain_ids
    }

    pub fn normalization_id(self) -> &'static str {
        self.schema.normalization.id()
    }

    pub fn measure_semantics_id(self) -> &'static str {
        self.schema.measure_semantics.id()
    }

    pub fn support_rule_id(self) -> &'static str {
        self.schema.support_rule.id()
    }
}
