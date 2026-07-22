//! Canonical carrier-shape catalog for value-free stellar-birth coordinates.

use super::index_domain::{
    COMPONENT_IDENTITY_DOMAIN_ID, EVOLUTION_TIME_DOMAIN_ID, LOG_FREQUENCY_DOMAIN_ID,
    MATERIAL_ELEMENT_DOMAIN_ID, SPATIAL_POSITION_DOMAIN_ID, SPECIES_IDENTITY_DOMAIN_ID,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CarrierKind {
    Scalar,
    SpatialScalarField,
    SpatialVectorField,
    SpeciesNumberFractionSimplex,
    SpectralFluxDensityPerLogFrequencyField,
    VariableComponentScalarField,
    VariableComponentVectorField,
    VariableCardinalityTopology,
    TimeScalarHistory,
    MaterialTimeScalarHistory,
    MaterialTimeVectorHistory,
}

impl CarrierKind {
    pub(super) const ORDERED: [Self; 11] = [
        Self::Scalar,
        Self::SpatialScalarField,
        Self::SpatialVectorField,
        Self::SpeciesNumberFractionSimplex,
        Self::SpectralFluxDensityPerLogFrequencyField,
        Self::VariableComponentScalarField,
        Self::VariableComponentVectorField,
        Self::VariableCardinalityTopology,
        Self::TimeScalarHistory,
        Self::MaterialTimeScalarHistory,
        Self::MaterialTimeVectorHistory,
    ];

    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::SpatialScalarField => "spatial_scalar_field",
            Self::SpatialVectorField => "spatial_vector_field",
            Self::SpeciesNumberFractionSimplex => "species_number_fraction_simplex",
            Self::SpectralFluxDensityPerLogFrequencyField => {
                "spectral_flux_density_per_log_frequency_field"
            }
            Self::VariableComponentScalarField => "variable_component_scalar_field",
            Self::VariableComponentVectorField => "variable_component_vector_field",
            Self::VariableCardinalityTopology => "variable_cardinality_topology",
            Self::TimeScalarHistory => "time_scalar_history",
            Self::MaterialTimeScalarHistory => "material_time_scalar_history",
            Self::MaterialTimeVectorHistory => "material_time_vector_history",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CarrierValueShape {
    Scalar,
    SpatialVector,
    SimplexCoordinate,
    LabeledHypergraph,
}

impl CarrierValueShape {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::SpatialVector => "spatial_vector",
            Self::SimplexCoordinate => "simplex_coordinate",
            Self::LabeledHypergraph => "labeled_hypergraph",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CarrierNormalization {
    None,
    UnitSumPerSpatialCoordinate,
}

impl CarrierNormalization {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::UnitSumPerSpatialCoordinate => "unit_sum_per_spatial_coordinate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CarrierMeasureSemantics {
    PointValue,
    NumberFractionOverCompleteSpeciesRegistry,
    DensityPerNaturalLogFrequency,
    LabeledHyperedgeIncidence,
}

impl CarrierMeasureSemantics {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::PointValue => "point_value",
            Self::NumberFractionOverCompleteSpeciesRegistry => {
                "number_fraction_over_complete_species_registry"
            }
            Self::DensityPerNaturalLogFrequency => "density_per_natural_log_frequency",
            Self::LabeledHyperedgeIncidence => "labeled_hyperedge_incidence",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CarrierSupportRule {
    NotApplicable,
    CompleteRegistrySupportOrNamedRefusal,
}

impl CarrierSupportRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::NotApplicable => "not_applicable",
            Self::CompleteRegistrySupportOrNamedRefusal => {
                "complete_registry_support_or_named_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CarrierSchema {
    pub(crate) kind: CarrierKind,
    pub(crate) value_shape: CarrierValueShape,
    pub(crate) index_domain_ids: Vec<&'static str>,
    pub(crate) normalization: CarrierNormalization,
    pub(crate) measure_semantics: CarrierMeasureSemantics,
    pub(crate) support_rule: CarrierSupportRule,
}

impl CarrierSchema {
    pub(crate) const fn id(&self) -> &'static str {
        self.kind.id()
    }

    pub(super) fn is_canonical(&self) -> bool {
        self == &canonical_carrier_schema(self.kind)
    }
}

pub(super) fn canonical_carrier_schemas() -> Vec<CarrierSchema> {
    let mut carriers = CarrierKind::ORDERED
        .into_iter()
        .map(canonical_carrier_schema)
        .collect::<Vec<_>>();
    carriers.sort_by_key(CarrierSchema::id);
    carriers
}

fn canonical_carrier_schema(kind: CarrierKind) -> CarrierSchema {
    use CarrierMeasureSemantics::{
        DensityPerNaturalLogFrequency, LabeledHyperedgeIncidence,
        NumberFractionOverCompleteSpeciesRegistry, PointValue,
    };
    use CarrierNormalization::{None, UnitSumPerSpatialCoordinate};
    use CarrierSupportRule::{CompleteRegistrySupportOrNamedRefusal, NotApplicable};
    use CarrierValueShape::{LabeledHypergraph, Scalar, SimplexCoordinate, SpatialVector};

    match kind {
        CarrierKind::Scalar => CarrierSchema {
            kind,
            value_shape: Scalar,
            index_domain_ids: vec![],
            normalization: None,
            measure_semantics: PointValue,
            support_rule: NotApplicable,
        },
        CarrierKind::SpatialScalarField => CarrierSchema {
            kind,
            value_shape: Scalar,
            index_domain_ids: vec![SPATIAL_POSITION_DOMAIN_ID],
            normalization: None,
            measure_semantics: PointValue,
            support_rule: NotApplicable,
        },
        CarrierKind::SpatialVectorField => CarrierSchema {
            kind,
            value_shape: SpatialVector,
            index_domain_ids: vec![SPATIAL_POSITION_DOMAIN_ID],
            normalization: None,
            measure_semantics: PointValue,
            support_rule: NotApplicable,
        },
        CarrierKind::SpeciesNumberFractionSimplex => CarrierSchema {
            kind,
            value_shape: SimplexCoordinate,
            index_domain_ids: vec![SPATIAL_POSITION_DOMAIN_ID, SPECIES_IDENTITY_DOMAIN_ID],
            normalization: UnitSumPerSpatialCoordinate,
            measure_semantics: NumberFractionOverCompleteSpeciesRegistry,
            support_rule: CompleteRegistrySupportOrNamedRefusal,
        },
        CarrierKind::SpectralFluxDensityPerLogFrequencyField => CarrierSchema {
            kind,
            value_shape: Scalar,
            index_domain_ids: vec![SPATIAL_POSITION_DOMAIN_ID, LOG_FREQUENCY_DOMAIN_ID],
            normalization: None,
            measure_semantics: DensityPerNaturalLogFrequency,
            support_rule: NotApplicable,
        },
        CarrierKind::VariableComponentScalarField => CarrierSchema {
            kind,
            value_shape: Scalar,
            index_domain_ids: vec![COMPONENT_IDENTITY_DOMAIN_ID],
            normalization: None,
            measure_semantics: PointValue,
            support_rule: NotApplicable,
        },
        CarrierKind::VariableComponentVectorField => CarrierSchema {
            kind,
            value_shape: SpatialVector,
            index_domain_ids: vec![COMPONENT_IDENTITY_DOMAIN_ID],
            normalization: None,
            measure_semantics: PointValue,
            support_rule: NotApplicable,
        },
        CarrierKind::VariableCardinalityTopology => CarrierSchema {
            kind,
            value_shape: LabeledHypergraph,
            index_domain_ids: vec![COMPONENT_IDENTITY_DOMAIN_ID],
            normalization: None,
            measure_semantics: LabeledHyperedgeIncidence,
            support_rule: NotApplicable,
        },
        CarrierKind::TimeScalarHistory => CarrierSchema {
            kind,
            value_shape: Scalar,
            index_domain_ids: vec![EVOLUTION_TIME_DOMAIN_ID],
            normalization: None,
            measure_semantics: PointValue,
            support_rule: NotApplicable,
        },
        CarrierKind::MaterialTimeScalarHistory => CarrierSchema {
            kind,
            value_shape: Scalar,
            index_domain_ids: vec![MATERIAL_ELEMENT_DOMAIN_ID, EVOLUTION_TIME_DOMAIN_ID],
            normalization: None,
            measure_semantics: PointValue,
            support_rule: NotApplicable,
        },
        CarrierKind::MaterialTimeVectorHistory => CarrierSchema {
            kind,
            value_shape: SpatialVector,
            index_domain_ids: vec![MATERIAL_ELEMENT_DOMAIN_ID, EVOLUTION_TIME_DOMAIN_ID],
            normalization: None,
            measure_semantics: PointValue,
            support_rule: NotApplicable,
        },
    }
}
