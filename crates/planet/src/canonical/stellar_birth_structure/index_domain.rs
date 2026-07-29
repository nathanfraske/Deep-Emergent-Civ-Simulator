//! Shared index-domain identities for stellar-birth fields and histories.

use civsim_units::fundamentals::SiDimension;

pub(super) const SPATIAL_POSITION_DOMAIN_ID: &str = "stellar_birth.domain.spatial_position";
pub(super) const SPECIES_IDENTITY_DOMAIN_ID: &str = "stellar_birth.domain.species_identity";
pub(super) const LOG_FREQUENCY_DOMAIN_ID: &str = "stellar_birth.domain.log_frequency";
pub(super) const COMPONENT_IDENTITY_DOMAIN_ID: &str = "stellar_birth.domain.component_identity";
pub(super) const MATERIAL_ELEMENT_DOMAIN_ID: &str =
    "stellar_birth.domain.material_element_identity";
pub(super) const EVOLUTION_TIME_DOMAIN_ID: &str = "stellar_birth.domain.evolution_time";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum IndexDomainKind {
    SpatialPosition,
    SpeciesIdentity,
    LogFrequency,
    ComponentIdentity,
    MaterialElementIdentity,
    EvolutionTime,
}

impl IndexDomainKind {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::SpatialPosition => "spatial_position",
            Self::SpeciesIdentity => "species_identity",
            Self::LogFrequency => "natural_log_frequency_ratio",
            Self::ComponentIdentity => "component_identity",
            Self::MaterialElementIdentity => "lagrangian_material_element_identity",
            Self::EvolutionTime => "monotone_evolution_time",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DomainSupportRule {
    JointMeasureDefined,
    RegistryDefinedVariable,
    DynamicalLawDefined,
}

impl DomainSupportRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::JointMeasureDefined => "joint_measure_defined",
            Self::RegistryDefinedVariable => "registry_defined_variable",
            Self::DynamicalLawDefined => "dynamical_law_defined",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DomainResolutionRule {
    ExactRegistryMembership,
    ConvergenceDerivedOrNamedRefusal,
}

impl DomainResolutionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ExactRegistryMembership => "exact_registry_membership",
            Self::ConvergenceDerivedOrNamedRefusal => "convergence_derived_or_named_refusal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DomainCapacityRule {
    EngineLimitIsNamedRefusal,
}

impl DomainCapacityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::EngineLimitIsNamedRefusal => "engine_limit_is_named_refusal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DomainOrdinalRule {
    SerializationOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DomainReferenceRule {
    DerivedSpatialChartOrNamedRefusal,
    PhysicalContentIdentity,
    LogRatioReferenceIsGaugeShift,
    DerivedTimeOriginOrNamedRefusal,
}

impl DomainReferenceRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::DerivedSpatialChartOrNamedRefusal => "derived_spatial_chart_or_named_refusal",
            Self::PhysicalContentIdentity => "physical_content_identity",
            Self::LogRatioReferenceIsGaugeShift => "log_ratio_reference_is_gauge_shift",
            Self::DerivedTimeOriginOrNamedRefusal => "derived_time_origin_or_named_refusal",
        }
    }
}

impl DomainOrdinalRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::SerializationOnly => "serialization_only_never_identity_or_coordinate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DomainOrderingRule {
    LexicographicContentIdentity,
    PhysicalCoordinateAscending,
}

impl DomainOrderingRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::LexicographicContentIdentity => "lexicographic_content_identity",
            Self::PhysicalCoordinateAscending => "physical_coordinate_ascending",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct IndexDomain {
    pub(crate) id: &'static str,
    pub(crate) kind: IndexDomainKind,
    pub(crate) support_rule: DomainSupportRule,
    pub(crate) resolution_rule: DomainResolutionRule,
    pub(crate) ordering_rule: DomainOrderingRule,
    pub(crate) capacity_rule: DomainCapacityRule,
    pub(crate) ordinal_rule: DomainOrdinalRule,
    pub(crate) reference_rule: DomainReferenceRule,
    pub(crate) coordinate_dimension: [i8; 7],
}

impl IndexDomain {
    pub(super) fn is_canonical(&self) -> bool {
        self == &canonical_index_domain(self.kind)
    }
}

pub(super) fn canonical_index_domains() -> Vec<IndexDomain> {
    let mut domains = [
        IndexDomainKind::SpatialPosition,
        IndexDomainKind::SpeciesIdentity,
        IndexDomainKind::LogFrequency,
        IndexDomainKind::ComponentIdentity,
        IndexDomainKind::MaterialElementIdentity,
        IndexDomainKind::EvolutionTime,
    ]
    .into_iter()
    .map(canonical_index_domain)
    .collect::<Vec<_>>();
    domains.sort_by(|left, right| left.id.cmp(right.id));
    domains
}

fn canonical_index_domain(kind: IndexDomainKind) -> IndexDomain {
    use DomainCapacityRule::EngineLimitIsNamedRefusal;
    use DomainOrderingRule::{LexicographicContentIdentity, PhysicalCoordinateAscending};
    use DomainOrdinalRule::SerializationOnly;
    use DomainReferenceRule::{
        DerivedSpatialChartOrNamedRefusal, DerivedTimeOriginOrNamedRefusal,
        LogRatioReferenceIsGaugeShift, PhysicalContentIdentity,
    };
    use DomainResolutionRule::{ConvergenceDerivedOrNamedRefusal, ExactRegistryMembership};
    use DomainSupportRule::{DynamicalLawDefined, JointMeasureDefined, RegistryDefinedVariable};

    match kind {
        IndexDomainKind::SpatialPosition => IndexDomain {
            id: SPATIAL_POSITION_DOMAIN_ID,
            kind,
            support_rule: JointMeasureDefined,
            resolution_rule: ConvergenceDerivedOrNamedRefusal,
            ordering_rule: LexicographicContentIdentity,
            capacity_rule: EngineLimitIsNamedRefusal,
            ordinal_rule: SerializationOnly,
            reference_rule: DerivedSpatialChartOrNamedRefusal,
            coordinate_dimension: SiDimension::new(1, 0, 0, 0, 0, 0, 0).exponents(),
        },
        IndexDomainKind::SpeciesIdentity => IndexDomain {
            id: SPECIES_IDENTITY_DOMAIN_ID,
            kind,
            support_rule: RegistryDefinedVariable,
            resolution_rule: ExactRegistryMembership,
            ordering_rule: LexicographicContentIdentity,
            capacity_rule: EngineLimitIsNamedRefusal,
            ordinal_rule: SerializationOnly,
            reference_rule: PhysicalContentIdentity,
            coordinate_dimension: SiDimension::DIMENSIONLESS.exponents(),
        },
        IndexDomainKind::LogFrequency => IndexDomain {
            id: LOG_FREQUENCY_DOMAIN_ID,
            kind,
            support_rule: JointMeasureDefined,
            resolution_rule: ConvergenceDerivedOrNamedRefusal,
            ordering_rule: PhysicalCoordinateAscending,
            capacity_rule: EngineLimitIsNamedRefusal,
            ordinal_rule: SerializationOnly,
            reference_rule: LogRatioReferenceIsGaugeShift,
            coordinate_dimension: SiDimension::DIMENSIONLESS.exponents(),
        },
        IndexDomainKind::ComponentIdentity => IndexDomain {
            id: COMPONENT_IDENTITY_DOMAIN_ID,
            kind,
            support_rule: RegistryDefinedVariable,
            resolution_rule: ExactRegistryMembership,
            ordering_rule: LexicographicContentIdentity,
            capacity_rule: EngineLimitIsNamedRefusal,
            ordinal_rule: SerializationOnly,
            reference_rule: PhysicalContentIdentity,
            coordinate_dimension: SiDimension::DIMENSIONLESS.exponents(),
        },
        IndexDomainKind::MaterialElementIdentity => IndexDomain {
            id: MATERIAL_ELEMENT_DOMAIN_ID,
            kind,
            support_rule: JointMeasureDefined,
            resolution_rule: ConvergenceDerivedOrNamedRefusal,
            ordering_rule: LexicographicContentIdentity,
            capacity_rule: EngineLimitIsNamedRefusal,
            ordinal_rule: SerializationOnly,
            reference_rule: PhysicalContentIdentity,
            coordinate_dimension: SiDimension::DIMENSIONLESS.exponents(),
        },
        IndexDomainKind::EvolutionTime => IndexDomain {
            id: EVOLUTION_TIME_DOMAIN_ID,
            kind,
            support_rule: DynamicalLawDefined,
            resolution_rule: ConvergenceDerivedOrNamedRefusal,
            ordering_rule: PhysicalCoordinateAscending,
            capacity_rule: EngineLimitIsNamedRefusal,
            ordinal_rule: SerializationOnly,
            reference_rule: DerivedTimeOriginOrNamedRefusal,
            coordinate_dimension: SiDimension::new(0, 0, 1, 0, 0, 0, 0).exponents(),
        },
    }
}
