//! Value-free species membership and identity contract.
//!
//! This schema does not enumerate familiar chemistry. A future registry may
//! contain only species identities derived from physical state reachable from
//! the sealed admitted floor. A missing derivation is a named refusal. Residual
//! admission can add a value to the floor, but cannot author registry members.

pub(super) const SPECIES_REGISTRY_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-species-registry.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SpeciesMembershipRule {
    FloorDerivedOnlyOrNamedRefusal,
}

impl SpeciesMembershipRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::FloorDerivedOnlyOrNamedRefusal => "floor_derived_only_or_named_refusal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SpeciesIdentityRule {
    ContentDerivedFromPhysicalState,
}

impl SpeciesIdentityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ContentDerivedFromPhysicalState => "content_derived_from_physical_state",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SpeciesOrderingRule {
    LexicographicContentIdentity,
}

impl SpeciesOrderingRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::LexicographicContentIdentity => "lexicographic_content_identity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SpeciesCapacityRule {
    EngineLimitIsNamedRefusal,
}

impl SpeciesCapacityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::EngineLimitIsNamedRefusal => "engine_limit_is_named_refusal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SpeciesOrdinalRule {
    SerializationOnly,
}

impl SpeciesOrdinalRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::SerializationOnly => "serialization_only_never_identity_or_coordinate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SpeciesRegistrySchema {
    pub(crate) schema_id: &'static str,
    pub(crate) membership_rule: SpeciesMembershipRule,
    pub(crate) identity_rule: SpeciesIdentityRule,
    pub(crate) ordering_rule: SpeciesOrderingRule,
    pub(crate) capacity_rule: SpeciesCapacityRule,
    pub(crate) ordinal_rule: SpeciesOrdinalRule,
}

impl SpeciesRegistrySchema {
    pub(super) fn is_canonical(&self) -> bool {
        self == &canonical_species_registry_schema()
    }
}

pub(super) const fn canonical_species_registry_schema() -> SpeciesRegistrySchema {
    SpeciesRegistrySchema {
        schema_id: SPECIES_REGISTRY_SCHEMA_ID,
        membership_rule: SpeciesMembershipRule::FloorDerivedOnlyOrNamedRefusal,
        identity_rule: SpeciesIdentityRule::ContentDerivedFromPhysicalState,
        ordering_rule: SpeciesOrderingRule::LexicographicContentIdentity,
        capacity_rule: SpeciesCapacityRule::EngineLimitIsNamedRefusal,
        ordinal_rule: SpeciesOrdinalRule::SerializationOnly,
    }
}
