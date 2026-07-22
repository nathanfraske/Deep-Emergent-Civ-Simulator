//! Exact, non-admitting dimensional census for the open stellar-birth measure.
//!
//! The census contains no magnitudes and closes no Stage 1 proof. It records
//! which SI relationships are possible, which dimensionless freedoms remain,
//! and which physical mechanisms still prevent a joint stellar-birth measure
//! from being constructed. Each phenomenon is analyzed separately so rank
//! supplied by one process cannot conceal a gap in another process.

use super::{
    floor_magnitudes::AuditedFloorView,
    stellar_birth_structure::{
        stellar_birth_structure_schema, CarrierKind, StellarBirthStructureSchema,
        StructureSchemaError,
    },
};
use civsim_units::{
    dimensional_analysis::{DimensionAnalysisError, SiDimensionAnalysis, SiDimensionColumn},
    fundamentals::{
        SiDimension, REPRESENTATION_DEFINITIONS, SI_BASE_DIMENSION_IDS, SI_REPRESENTATION_SCHEMA_ID,
    },
    physics_floor::{
        sealed_physical_floor_authority_binding, sealed_physical_floor_dimension_columns,
    },
};
use std::{collections::BTreeMap, fmt};

pub(super) const STELLAR_BIRTH_DIMENSIONAL_CENSUS_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-dimensional-census.v3";
pub(super) const EXACT_DIMENSIONAL_CHECKER_ID: &str = "civsim.units.exact-si-rref.v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CensusClosureEffect {
    None,
}

impl CensusClosureEffect {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DimensionalVariableRole {
    RepresentationDefinition,
    AdmittedPhysicalInvariant,
    CandidateCoordinate,
    DerivedIntermediate,
}

impl DimensionalVariableRole {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::RepresentationDefinition => "representation_definition",
            Self::AdmittedPhysicalInvariant => "admitted_physical_invariant",
            Self::CandidateCoordinate => "candidate_coordinate",
            Self::DerivedIntermediate => "derived_intermediate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DimensionalVariable {
    pub(super) id: String,
    pub(super) role: DimensionalVariableRole,
    pub(super) carrier: CarrierKind,
    pub(super) dimension: [i8; 7],
    pub(super) coupling_group_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DerivationAttemptStatus {
    TargetDimensionReachable,
}

impl DerivationAttemptStatus {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::TargetDimensionReachable => "target_dimension_reachable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ExactExponentRecord {
    pub(super) numerator: i128,
    pub(super) denominator: i128,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DimensionalDerivationAttempt {
    pub(super) attempt_id: String,
    pub(super) law_id: String,
    pub(super) input_ids: Vec<String>,
    pub(super) output_id: String,
    pub(super) status: DerivationAttemptStatus,
    pub(super) dimension_only_projection: Vec<ExactExponentRecord>,
    pub(super) dimension_only_support_ids: Vec<String>,
    pub(super) missing_dependency_ids: Vec<String>,
    pub(super) dropped_mechanism_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PhenomenonDimensionalCensus {
    pub(super) phenomenon_id: String,
    pub(super) input_ids: Vec<String>,
    pub(super) output_ids: Vec<String>,
    pub(super) matrix_orientation: &'static str,
    pub(super) matrix_columns: Vec<DimensionalVariable>,
    pub(super) rank: usize,
    pub(super) pivot_columns: Vec<usize>,
    pub(super) free_columns: Vec<usize>,
    pub(super) null_space_basis: Vec<Vec<i128>>,
    pub(super) coverage_complete: bool,
    pub(super) derivation_attempts: Vec<DimensionalDerivationAttempt>,
}

impl PhenomenonDimensionalCensus {
    pub(super) fn nullity(&self) -> usize {
        self.null_space_basis.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct StellarBirthDimensionalCensus {
    pub(super) schema_id: &'static str,
    pub(super) checker_id: &'static str,
    pub(super) closure_effect: CensusClosureEffect,
    pub(super) coverage_claim: bool,
    pub(super) representation_schema_id: &'static str,
    pub(super) floor_binding_schema_id: &'static str,
    pub(super) floor_binding_sha256: String,
    pub(super) base_dimension_ids: Vec<&'static str>,
    pub(super) structure: StellarBirthStructureSchema,
    pub(super) variables: Vec<DimensionalVariable>,
    pub(super) phenomena: Vec<PhenomenonDimensionalCensus>,
    pub(super) coverage_gap_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct InvalidStellarBirthDimensionalCensus {
    pub(super) schema_id: &'static str,
    pub(super) checker_id: &'static str,
    pub(super) closure_effect: CensusClosureEffect,
    pub(super) coverage_claim: bool,
    pub(super) error_code: &'static str,
    pub(super) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum StellarBirthDimensionalCensusArtifact {
    Computed(Box<StellarBirthDimensionalCensus>),
    Invalid(InvalidStellarBirthDimensionalCensus),
}

impl StellarBirthDimensionalCensusArtifact {
    pub(super) const fn status_id(&self) -> &'static str {
        match self {
            Self::Computed(_) => "computed",
            Self::Invalid(_) => "invalid",
        }
    }

    pub(super) const fn schema_id(&self) -> &'static str {
        match self {
            Self::Computed(census) => census.schema_id,
            Self::Invalid(census) => census.schema_id,
        }
    }

    pub(super) const fn checker_id(&self) -> &'static str {
        match self {
            Self::Computed(census) => census.checker_id,
            Self::Invalid(census) => census.checker_id,
        }
    }

    pub(super) const fn closure_effect(&self) -> CensusClosureEffect {
        match self {
            Self::Computed(census) => census.closure_effect,
            Self::Invalid(census) => census.closure_effect,
        }
    }

    pub(super) const fn coverage_claim(&self) -> bool {
        match self {
            Self::Computed(census) => census.coverage_claim,
            Self::Invalid(census) => census.coverage_claim,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CensusBuildError {
    FloorAuthority(String),
    Structure(StructureSchemaError),
    UnknownCarrier(String),
    DuplicateVariable(String),
    DuplicatePhenomenon(String),
    DuplicateAttempt {
        phenomenon_id: String,
        attempt_id: String,
    },
    DuplicateReference {
        phenomenon_id: String,
        variable_id: String,
    },
    UnknownVariable {
        phenomenon_id: String,
        variable_id: String,
    },
    AttemptInputOutsidePhenomenon {
        phenomenon_id: String,
        attempt_id: String,
        variable_id: String,
    },
    AttemptOutputOutsidePhenomenon {
        phenomenon_id: String,
        attempt_id: String,
        variable_id: String,
    },
    Dimension(DimensionAnalysisError),
    UnreachableAttempt {
        attempt_id: String,
        output_id: String,
    },
}

impl CensusBuildError {
    const fn code(&self) -> &'static str {
        match self {
            Self::FloorAuthority(_) => "floor_authority_unavailable",
            Self::Structure(_) => "structure_schema_invalid",
            Self::UnknownCarrier(_) => "unknown_carrier",
            Self::DuplicateVariable(_) => "duplicate_variable_id",
            Self::DuplicatePhenomenon(_) => "duplicate_phenomenon_id",
            Self::DuplicateAttempt { .. } => "duplicate_attempt_id",
            Self::DuplicateReference { .. } => "duplicate_variable_reference",
            Self::UnknownVariable { .. } => "unknown_variable_id",
            Self::AttemptInputOutsidePhenomenon { .. } => "attempt_input_outside_phenomenon",
            Self::AttemptOutputOutsidePhenomenon { .. } => "attempt_output_outside_phenomenon",
            Self::Dimension(_) => "dimension_analysis_failed",
            Self::UnreachableAttempt { .. } => "attempt_dimension_unreachable",
        }
    }
}

impl fmt::Display for CensusBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FloorAuthority(detail) => write!(f, "floor authority is unavailable: {detail}"),
            Self::Structure(error) => write!(f, "stellar-birth structure is invalid: {error}"),
            Self::UnknownCarrier(id) => {
                write!(f, "dimensional variable references unknown carrier '{id}'")
            }
            Self::DuplicateVariable(id) => write!(f, "duplicate variable identity '{id}'"),
            Self::DuplicatePhenomenon(id) => write!(f, "duplicate phenomenon identity '{id}'"),
            Self::DuplicateAttempt {
                phenomenon_id,
                attempt_id,
            } => write!(
                f,
                "phenomenon '{phenomenon_id}' repeats attempt identity '{attempt_id}'"
            ),
            Self::DuplicateReference {
                phenomenon_id,
                variable_id,
            } => write!(
                f,
                "phenomenon '{phenomenon_id}' repeats variable identity '{variable_id}'"
            ),
            Self::UnknownVariable {
                phenomenon_id,
                variable_id,
            } => write!(
                f,
                "phenomenon '{phenomenon_id}' references unknown variable '{variable_id}'"
            ),
            Self::AttemptInputOutsidePhenomenon {
                phenomenon_id,
                attempt_id,
                variable_id,
            } => write!(
                f,
                "attempt '{attempt_id}' in phenomenon '{phenomenon_id}' reads undeclared phenomenon input '{variable_id}'"
            ),
            Self::AttemptOutputOutsidePhenomenon {
                phenomenon_id,
                attempt_id,
                variable_id,
            } => write!(
                f,
                "attempt '{attempt_id}' in phenomenon '{phenomenon_id}' writes undeclared phenomenon output '{variable_id}'"
            ),
            Self::Dimension(error) => write!(f, "{error}"),
            Self::UnreachableAttempt {
                attempt_id,
                output_id,
            } => write!(
                f,
                "attempt '{attempt_id}' cannot dimensionally project output '{output_id}'"
            ),
        }
    }
}

impl From<DimensionAnalysisError> for CensusBuildError {
    fn from(error: DimensionAnalysisError) -> Self {
        Self::Dimension(error)
    }
}

impl From<StructureSchemaError> for CensusBuildError {
    fn from(error: StructureSchemaError) -> Self {
        Self::Structure(error)
    }
}

#[derive(Clone, Copy)]
struct VariableSpec {
    id: &'static str,
    role: DimensionalVariableRole,
    carrier: CarrierKind,
    dimension: SiDimension,
}

#[derive(Clone, Copy)]
struct AttemptSpec {
    attempt_id: &'static str,
    law_id: &'static str,
    input_ids: &'static [&'static str],
    output_id: &'static str,
    missing_dependency_ids: &'static [&'static str],
    dropped_mechanism_ids: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct PhenomenonSpec {
    phenomenon_id: &'static str,
    input_ids: &'static [&'static str],
    output_ids: &'static [&'static str],
    coverage_complete: bool,
    attempts: &'static [AttemptSpec],
}

const JOINT_COUPLING_GROUP: &str = "stellar_birth.joint_physical_measure";
const MATRIX_ORIENTATION: &str = "si_base_dimensions_by_variable_columns";

/// Build the report only after the canonical runner has constructed its
/// audited floor view. Failure remains an analysis record beneath the same
/// open proof leaf and cannot become a fallback value.
pub(super) fn stellar_birth_dimensional_census(
    _floor: &AuditedFloorView<'_>,
) -> StellarBirthDimensionalCensusArtifact {
    match build_census() {
        Ok(census) => StellarBirthDimensionalCensusArtifact::Computed(Box::new(census)),
        Err(error) => {
            StellarBirthDimensionalCensusArtifact::Invalid(InvalidStellarBirthDimensionalCensus {
                schema_id: STELLAR_BIRTH_DIMENSIONAL_CENSUS_SCHEMA_ID,
                checker_id: EXACT_DIMENSIONAL_CHECKER_ID,
                closure_effect: CensusClosureEffect::None,
                coverage_claim: false,
                error_code: error.code(),
                detail: error.to_string(),
            })
        }
    }
}

fn build_census() -> Result<StellarBirthDimensionalCensus, CensusBuildError> {
    let binding = sealed_physical_floor_authority_binding()
        .map_err(|error| CensusBuildError::FloorAuthority(error.to_string()))?;
    let structure = stellar_birth_structure_schema()?;
    let variables = build_variables(&structure)?;
    let registry: BTreeMap<_, _> = variables
        .iter()
        .map(|variable| (variable.id.as_str(), variable))
        .collect();
    let mut phenomena = Vec::new();
    for spec in phenomenon_specs() {
        if phenomena
            .iter()
            .any(|candidate: &PhenomenonDimensionalCensus| {
                candidate.phenomenon_id == spec.phenomenon_id
            })
        {
            return Err(CensusBuildError::DuplicatePhenomenon(
                spec.phenomenon_id.to_owned(),
            ));
        }
        phenomena.push(build_phenomenon(spec, &registry)?);
    }
    phenomena.sort_by(|left, right| left.phenomenon_id.cmp(&right.phenomenon_id));

    let mut coverage_gap_ids = vec![
        "stellar_birth.gap.fragmentation_and_multiplicity_generalization".to_owned(),
        "stellar_birth.gap.full_field_and_tensor_carriers".to_owned(),
        "stellar_birth.gap.magnetic_braking_and_material_history".to_owned(),
        "stellar_birth.gap.nonbinary_variable_cardinality_dynamics".to_owned(),
        "stellar_birth.gap.radiation_and_cooling_spectral_closure".to_owned(),
        "stellar_birth.gap.stochastic_and_chaotic_regime_measure".to_owned(),
    ];
    coverage_gap_ids.sort();

    Ok(StellarBirthDimensionalCensus {
        schema_id: STELLAR_BIRTH_DIMENSIONAL_CENSUS_SCHEMA_ID,
        checker_id: EXACT_DIMENSIONAL_CHECKER_ID,
        closure_effect: CensusClosureEffect::None,
        coverage_claim: false,
        representation_schema_id: SI_REPRESENTATION_SCHEMA_ID,
        floor_binding_schema_id: binding.schema_id().as_str(),
        floor_binding_sha256: binding.digest_hex(),
        base_dimension_ids: SI_BASE_DIMENSION_IDS.to_vec(),
        structure,
        variables,
        phenomena,
        coverage_gap_ids,
    })
}

fn build_variables(
    structure: &StellarBirthStructureSchema,
) -> Result<Vec<DimensionalVariable>, CensusBuildError> {
    let physical_floor_columns = sealed_physical_floor_dimension_columns()
        .map_err(|error| CensusBuildError::FloorAuthority(error.to_string()))?;
    let mut variables = REPRESENTATION_DEFINITIONS
        .iter()
        .map(|definition| DimensionalVariable {
            id: format!("representation.{}", definition.symbol),
            role: DimensionalVariableRole::RepresentationDefinition,
            carrier: CarrierKind::Scalar,
            dimension: definition.dimension.exponents(),
            coupling_group_id: JOINT_COUPLING_GROUP.to_owned(),
        })
        .chain(
            physical_floor_columns
                .iter()
                .map(|column| DimensionalVariable {
                    id: column.id().to_owned(),
                    role: DimensionalVariableRole::AdmittedPhysicalInvariant,
                    carrier: CarrierKind::Scalar,
                    dimension: column.dimension().exponents(),
                    coupling_group_id: JOINT_COUPLING_GROUP.to_owned(),
                }),
        )
        .chain(
            candidate_variable_specs()
                .into_iter()
                .map(|spec| DimensionalVariable {
                    id: spec.id.to_owned(),
                    role: spec.role,
                    carrier: spec.carrier,
                    dimension: spec.dimension.exponents(),
                    coupling_group_id: JOINT_COUPLING_GROUP.to_owned(),
                }),
        )
        .collect::<Vec<_>>();
    variables.sort_by(|left, right| left.id.cmp(&right.id));
    for pair in variables.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(CensusBuildError::DuplicateVariable(pair[0].id.clone()));
        }
    }
    for variable in &variables {
        if structure.carrier(variable.carrier).is_none() {
            return Err(CensusBuildError::UnknownCarrier(
                variable.carrier.id().to_owned(),
            ));
        }
    }
    Ok(variables)
}

fn candidate_variable_specs() -> Vec<VariableSpec> {
    use CarrierKind as Carrier;
    use DimensionalVariableRole::{
        CandidateCoordinate as Candidate, DerivedIntermediate as Derived,
    };

    vec![
        VariableSpec {
            id: "stellar_birth.background_temperature",
            role: Candidate,
            carrier: Carrier::SpatialScalarField,
            dimension: SiDimension::new(0, 0, 0, 0, 1, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.material_element.circularization_length_history",
            role: Derived,
            carrier: Carrier::MaterialTimeScalarHistory,
            dimension: SiDimension::new(1, 0, 0, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.collapse_elapsed_time",
            role: Candidate,
            carrier: Carrier::Scalar,
            dimension: SiDimension::new(0, 0, 1, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.collapse_mass_flow",
            role: Derived,
            carrier: Carrier::TimeScalarHistory,
            dimension: SiDimension::new(0, 1, -1, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.column_number_density",
            role: Candidate,
            carrier: Carrier::SpatialScalarField,
            dimension: SiDimension::new(-2, 0, 0, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.component.mass_field",
            role: Candidate,
            carrier: Carrier::VariableComponentScalarField,
            dimension: SiDimension::new(0, 1, 0, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.component.position_field",
            role: Candidate,
            carrier: Carrier::VariableComponentVectorField,
            dimension: SiDimension::new(1, 0, 0, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.component.topology",
            role: Candidate,
            carrier: Carrier::VariableCardinalityTopology,
            dimension: SiDimension::DIMENSIONLESS,
        },
        VariableSpec {
            id: "stellar_birth.component.velocity_field",
            role: Candidate,
            carrier: Carrier::VariableComponentVectorField,
            dimension: SiDimension::new(1, 0, -1, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.composition.species_number_fraction_field",
            role: Candidate,
            carrier: Carrier::SpeciesNumberFractionSimplex,
            dimension: SiDimension::DIMENSIONLESS,
        },
        VariableSpec {
            id: "stellar_birth.core.number_density_field",
            role: Candidate,
            carrier: Carrier::SpatialScalarField,
            dimension: SiDimension::new(-3, 0, 0, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.dust_temperature_field",
            role: Derived,
            carrier: Carrier::SpatialScalarField,
            dimension: SiDimension::new(0, 0, 0, 0, 1, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.material_element.mass_history",
            role: Derived,
            carrier: Carrier::MaterialTimeScalarHistory,
            dimension: SiDimension::new(0, 1, 0, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.gas_temperature_field",
            role: Derived,
            carrier: Carrier::SpatialScalarField,
            dimension: SiDimension::new(0, 0, 0, 0, 1, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.ionization_rate_field",
            role: Candidate,
            carrier: Carrier::SpatialScalarField,
            dimension: SiDimension::new(0, 0, -1, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.magnetic_flux_density_field",
            role: Candidate,
            carrier: Carrier::SpatialVectorField,
            dimension: SiDimension::new(0, 1, -2, -1, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.mean_particle_mass_field",
            role: Derived,
            carrier: Carrier::SpatialScalarField,
            dimension: SiDimension::new(0, 1, 0, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.radiation_flux_spectrum",
            role: Candidate,
            carrier: Carrier::SpectralFluxDensityPerLogFrequencyField,
            dimension: SiDimension::new(0, 1, -3, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.sound_speed_field",
            role: Derived,
            carrier: Carrier::SpatialScalarField,
            dimension: SiDimension::new(1, 0, -1, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.material_element.specific_angular_momentum_history",
            role: Candidate,
            carrier: Carrier::MaterialTimeVectorHistory,
            dimension: SiDimension::new(2, 0, -1, 0, 0, 0, 0),
        },
        VariableSpec {
            id: "stellar_birth.turbulent_velocity_field",
            role: Candidate,
            carrier: Carrier::SpatialVectorField,
            dimension: SiDimension::new(1, 0, -1, 0, 0, 0, 0),
        },
    ]
}

fn phenomenon_specs() -> Vec<PhenomenonSpec> {
    vec![
        PhenomenonSpec {
            phenomenon_id: "stellar_birth.phenomenon.absolute_floor_dimension_basis",
            input_ids: &[
                "representation.Delta_nu_Cs",
                "representation.c",
                "representation.h",
                "representation.e",
                "representation.k_B",
                "representation.N_A",
                "representation.K_cd",
                "fundamental.alpha",
                "fundamental.G",
                "fundamental.m_e",
            ],
            output_ids: &[],
            coverage_complete: true,
            attempts: &[],
        },
        PhenomenonSpec {
            phenomenon_id: "stellar_birth.phenomenon.mean_particle_mass",
            input_ids: &[
                "fundamental.m_e",
                "stellar_birth.composition.species_number_fraction_field",
            ],
            output_ids: &["stellar_birth.mean_particle_mass_field"],
            coverage_complete: false,
            attempts: &[AttemptSpec {
                attempt_id: "stellar_birth.attempt.mean_particle_mass_from_composition",
                law_id: "candidate.composition_weighted_particle_mass",
                input_ids: &[
                    "fundamental.m_e",
                    "stellar_birth.composition.species_number_fraction_field",
                ],
                output_id: "stellar_birth.mean_particle_mass_field",
                missing_dependency_ids: &[
                    "particle.species_rest_mass_spectrum",
                    "particle.species_charge_and_state_registry",
                ],
                dropped_mechanism_ids: &[],
            }],
        },
        PhenomenonSpec {
            phenomenon_id: "stellar_birth.phenomenon.core_thermal_balance",
            input_ids: &[
                "representation.k_B",
                "stellar_birth.background_temperature",
                "stellar_birth.column_number_density",
                "stellar_birth.composition.species_number_fraction_field",
                "stellar_birth.core.number_density_field",
                "stellar_birth.ionization_rate_field",
                "stellar_birth.magnetic_flux_density_field",
                "stellar_birth.radiation_flux_spectrum",
                "stellar_birth.turbulent_velocity_field",
            ],
            output_ids: &[
                "stellar_birth.dust_temperature_field",
                "stellar_birth.gas_temperature_field",
            ],
            coverage_complete: false,
            attempts: &[
                AttemptSpec {
                    attempt_id: "stellar_birth.attempt.gas_thermal_balance",
                    law_id: "candidate.local_heating_cooling_balance",
                    input_ids: &[
                        "representation.k_B",
                        "stellar_birth.background_temperature",
                        "stellar_birth.column_number_density",
                        "stellar_birth.composition.species_number_fraction_field",
                        "stellar_birth.core.number_density_field",
                        "stellar_birth.ionization_rate_field",
                        "stellar_birth.magnetic_flux_density_field",
                        "stellar_birth.radiation_flux_spectrum",
                        "stellar_birth.turbulent_velocity_field",
                    ],
                    output_id: "stellar_birth.gas_temperature_field",
                    missing_dependency_ids: &[
                        "microphysics.collisional_rate_laws",
                        "microphysics.cooling_spectrum",
                        "radiation.frequency_dependent_transfer_law",
                    ],
                    dropped_mechanism_ids: &[
                        "mechanism.chemical_reaction_network",
                        "mechanism.velocity_gradient_line_transfer",
                    ],
                },
                AttemptSpec {
                    attempt_id: "stellar_birth.attempt.dust_thermal_balance",
                    law_id: "candidate.radiative_dust_equilibrium",
                    input_ids: &[
                        "stellar_birth.background_temperature",
                        "stellar_birth.column_number_density",
                        "stellar_birth.composition.species_number_fraction_field",
                        "stellar_birth.radiation_flux_spectrum",
                    ],
                    output_id: "stellar_birth.dust_temperature_field",
                    missing_dependency_ids: &[
                        "dust.grain_population_measure",
                        "dust.frequency_dependent_absorption_law",
                    ],
                    dropped_mechanism_ids: &["mechanism.dust_evolution_and_coagulation"],
                },
            ],
        },
        PhenomenonSpec {
            phenomenon_id: "stellar_birth.phenomenon.sound_speed",
            input_ids: &[
                "representation.k_B",
                "stellar_birth.gas_temperature_field",
                "stellar_birth.mean_particle_mass_field",
            ],
            output_ids: &["stellar_birth.sound_speed_field"],
            coverage_complete: false,
            attempts: &[AttemptSpec {
                attempt_id: "stellar_birth.attempt.sound_speed_from_thermal_state",
                law_id: "candidate.thermodynamic_sound_speed",
                input_ids: &[
                    "representation.k_B",
                    "stellar_birth.gas_temperature_field",
                    "stellar_birth.mean_particle_mass_field",
                ],
                output_id: "stellar_birth.sound_speed_field",
                missing_dependency_ids: &[
                    "equation_of_state.adiabatic_index_law",
                    "equation_of_state.phase_and_ionization_closure",
                ],
                dropped_mechanism_ids: &[
                    "mechanism.magnetic_pressure_support",
                    "mechanism.turbulent_pressure_support",
                ],
            }],
        },
        PhenomenonSpec {
            phenomenon_id: "stellar_birth.phenomenon.collapse_mass_flow",
            input_ids: &["fundamental.G", "stellar_birth.sound_speed_field"],
            output_ids: &["stellar_birth.collapse_mass_flow"],
            coverage_complete: false,
            attempts: &[AttemptSpec {
                attempt_id: "stellar_birth.attempt.collapse_flow_from_sound_speed",
                law_id: "candidate.self_similar_isothermal_collapse",
                input_ids: &["fundamental.G", "stellar_birth.sound_speed_field"],
                output_id: "stellar_birth.collapse_mass_flow",
                missing_dependency_ids: &[
                    "collapse.dimensionless_similarity_eigenstructure",
                    "collapse.initial_and_boundary_measure",
                ],
                dropped_mechanism_ids: &[
                    "mechanism.magnetized_collapse",
                    "mechanism.rotating_fragmenting_collapse",
                ],
            }],
        },
        PhenomenonSpec {
            phenomenon_id: "stellar_birth.phenomenon.material_element_mass_history",
            input_ids: &[
                "stellar_birth.collapse_elapsed_time",
                "stellar_birth.collapse_mass_flow",
            ],
            output_ids: &["stellar_birth.material_element.mass_history"],
            coverage_complete: false,
            attempts: &[AttemptSpec {
                attempt_id: "stellar_birth.attempt.material_mass_from_flow_history",
                law_id: "candidate.material_element_mass_continuity",
                input_ids: &[
                    "stellar_birth.collapse_elapsed_time",
                    "stellar_birth.collapse_mass_flow",
                ],
                output_id: "stellar_birth.material_element.mass_history",
                missing_dependency_ids: &[
                    "collapse.initial_material_mass_or_integration_boundary",
                    "collapse.material_element_flux_and_topology_history",
                    "collapse.time_dependent_accretion_history",
                    "collapse.mass_loss_and_fragmentation_balance",
                ],
                dropped_mechanism_ids: &["mechanism.outflow_feedback"],
            }],
        },
        PhenomenonSpec {
            phenomenon_id: "stellar_birth.phenomenon.material_element_circularization_length",
            input_ids: &[
                "fundamental.G",
                "stellar_birth.material_element.mass_history",
                "stellar_birth.material_element.specific_angular_momentum_history",
            ],
            output_ids: &["stellar_birth.material_element.circularization_length_history"],
            coverage_complete: false,
            attempts: &[AttemptSpec {
                attempt_id: "stellar_birth.attempt.circularization_from_material_state",
                law_id: "candidate.local_angular_momentum_circularization",
                input_ids: &[
                    "fundamental.G",
                    "stellar_birth.material_element.mass_history",
                    "stellar_birth.material_element.specific_angular_momentum_history",
                ],
                output_id: "stellar_birth.material_element.circularization_length_history",
                missing_dependency_ids: &[
                    "kinematics.derived_component_local_frame_law",
                    "collapse.multicenter_binding_mass_functional",
                    "collapse.angular_momentum_material_history",
                    "collapse.torque_and_magnetic_braking_law",
                    "multiplicity.variable_component_interaction_law",
                ],
                dropped_mechanism_ids: &[
                    "mechanism.nonaxisymmetric_transport",
                    "mechanism.fragment_exchange_torque",
                ],
            }],
        },
    ]
}

fn build_phenomenon(
    spec: PhenomenonSpec,
    registry: &BTreeMap<&str, &DimensionalVariable>,
) -> Result<PhenomenonDimensionalCensus, CensusBuildError> {
    let input_ids = canonical_references(spec.phenomenon_id, spec.input_ids, registry)?;
    let output_ids = canonical_references(spec.phenomenon_id, spec.output_ids, registry)?;
    let mut matrix_ids = input_ids
        .iter()
        .chain(output_ids.iter())
        .cloned()
        .collect::<Vec<_>>();
    matrix_ids.sort();
    for pair in matrix_ids.windows(2) {
        if pair[0] == pair[1] {
            return Err(CensusBuildError::DuplicateReference {
                phenomenon_id: spec.phenomenon_id.to_owned(),
                variable_id: pair[0].clone(),
            });
        }
    }
    let matrix_columns = matrix_ids
        .iter()
        .map(|id| registry[id.as_str()].clone())
        .collect::<Vec<_>>();
    let columns = matrix_columns
        .iter()
        .map(|variable| {
            SiDimensionColumn::new(&variable.id, dimension_from_exponents(variable.dimension))
        })
        .collect::<Vec<_>>();
    let analysis = SiDimensionAnalysis::analyze(&columns)?;
    let free_columns = (0..columns.len())
        .filter(|column| !analysis.pivot_columns().contains(column))
        .collect();
    let mut derivation_attempts = spec
        .attempts
        .iter()
        .map(|attempt| build_attempt(*attempt, spec.phenomenon_id, registry))
        .collect::<Result<Vec<_>, _>>()?;
    for attempt in &derivation_attempts {
        for variable_id in &attempt.input_ids {
            if input_ids.binary_search(variable_id).is_err() {
                return Err(CensusBuildError::AttemptInputOutsidePhenomenon {
                    phenomenon_id: spec.phenomenon_id.to_owned(),
                    attempt_id: attempt.attempt_id.clone(),
                    variable_id: variable_id.clone(),
                });
            }
        }
        if output_ids.binary_search(&attempt.output_id).is_err() {
            return Err(CensusBuildError::AttemptOutputOutsidePhenomenon {
                phenomenon_id: spec.phenomenon_id.to_owned(),
                attempt_id: attempt.attempt_id.clone(),
                variable_id: attempt.output_id.clone(),
            });
        }
    }
    derivation_attempts.sort_by(|left, right| left.attempt_id.cmp(&right.attempt_id));
    for pair in derivation_attempts.windows(2) {
        if pair[0].attempt_id == pair[1].attempt_id {
            return Err(CensusBuildError::DuplicateAttempt {
                phenomenon_id: spec.phenomenon_id.to_owned(),
                attempt_id: pair[0].attempt_id.clone(),
            });
        }
    }

    Ok(PhenomenonDimensionalCensus {
        phenomenon_id: spec.phenomenon_id.to_owned(),
        input_ids,
        output_ids,
        matrix_orientation: MATRIX_ORIENTATION,
        matrix_columns,
        rank: analysis.rank(),
        pivot_columns: analysis.pivot_columns().to_vec(),
        free_columns,
        null_space_basis: analysis.null_space_basis().to_vec(),
        coverage_complete: spec.coverage_complete,
        derivation_attempts,
    })
}

fn build_attempt(
    spec: AttemptSpec,
    phenomenon_id: &str,
    registry: &BTreeMap<&str, &DimensionalVariable>,
) -> Result<DimensionalDerivationAttempt, CensusBuildError> {
    let input_ids = canonical_references(phenomenon_id, spec.input_ids, registry)?;
    let output = registry
        .get(spec.output_id)
        .ok_or_else(|| CensusBuildError::UnknownVariable {
            phenomenon_id: phenomenon_id.to_owned(),
            variable_id: spec.output_id.to_owned(),
        })?;
    let input_columns = input_ids
        .iter()
        .map(|id| {
            let variable = registry[id.as_str()];
            SiDimensionColumn::new(id, dimension_from_exponents(variable.dimension))
        })
        .collect::<Vec<_>>();
    let analysis = SiDimensionAnalysis::analyze(&input_columns)?;
    let projection = analysis
        .project_dimension(dimension_from_exponents(output.dimension))?
        .ok_or_else(|| CensusBuildError::UnreachableAttempt {
            attempt_id: spec.attempt_id.to_owned(),
            output_id: spec.output_id.to_owned(),
        })?;
    let dimension_only_support_ids = input_ids
        .iter()
        .zip(projection.exponents())
        .filter(|(_, exponent)| !exponent.is_zero())
        .map(|(id, _)| id.clone())
        .collect();
    let dimension_only_projection = projection
        .exponents()
        .iter()
        .map(|exponent| ExactExponentRecord {
            numerator: exponent.numerator(),
            denominator: exponent.denominator(),
        })
        .collect();
    let mut missing_dependency_ids = spec
        .missing_dependency_ids
        .iter()
        .map(|id| (*id).to_owned())
        .collect::<Vec<_>>();
    missing_dependency_ids.sort();
    let mut dropped_mechanism_ids = spec
        .dropped_mechanism_ids
        .iter()
        .map(|id| (*id).to_owned())
        .collect::<Vec<_>>();
    dropped_mechanism_ids.sort();

    Ok(DimensionalDerivationAttempt {
        attempt_id: spec.attempt_id.to_owned(),
        law_id: spec.law_id.to_owned(),
        input_ids,
        output_id: spec.output_id.to_owned(),
        status: DerivationAttemptStatus::TargetDimensionReachable,
        dimension_only_projection,
        dimension_only_support_ids,
        missing_dependency_ids,
        dropped_mechanism_ids,
    })
}

fn canonical_references(
    phenomenon_id: &str,
    ids: &[&str],
    registry: &BTreeMap<&str, &DimensionalVariable>,
) -> Result<Vec<String>, CensusBuildError> {
    let mut canonical = ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
    canonical.sort();
    for id in &canonical {
        if !registry.contains_key(id.as_str()) {
            return Err(CensusBuildError::UnknownVariable {
                phenomenon_id: phenomenon_id.to_owned(),
                variable_id: id.clone(),
            });
        }
    }
    for pair in canonical.windows(2) {
        if pair[0] == pair[1] {
            return Err(CensusBuildError::DuplicateReference {
                phenomenon_id: phenomenon_id.to_owned(),
                variable_id: pair[0].clone(),
            });
        }
    }
    Ok(canonical)
}

fn dimension_from_exponents(exponents: [i8; 7]) -> SiDimension {
    SiDimension::new(
        exponents[0],
        exponents[1],
        exponents[2],
        exponents[3],
        exponents[4],
        exponents[5],
        exponents[6],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::sealed_absolute_physics_floor;

    fn computed_census() -> StellarBirthDimensionalCensus {
        let floor = sealed_absolute_physics_floor().unwrap();
        let view = AuditedFloorView::from_floor(&floor).unwrap();
        match stellar_birth_dimensional_census(&view) {
            StellarBirthDimensionalCensusArtifact::Computed(census) => *census,
            StellarBirthDimensionalCensusArtifact::Invalid(error) => {
                panic!("current dimensional census is invalid: {}", error.detail)
            }
        }
    }

    #[test]
    fn current_census_is_partial_and_cannot_close_a_proof() {
        let census = computed_census();
        assert_eq!(census.closure_effect, CensusClosureEffect::None);
        assert!(!census.coverage_claim);
        assert!(!census.coverage_gap_ids.is_empty());
        assert!(
            census
                .phenomena
                .iter()
                .filter(|phenomenon| !phenomenon.coverage_complete)
                .count()
                >= 6
        );
    }

    #[test]
    fn floor_basis_reports_rank_seven_and_three_pi_groups() {
        let census = computed_census();
        let basis = census
            .phenomena
            .iter()
            .find(|phenomenon| {
                phenomenon.phenomenon_id
                    == "stellar_birth.phenomenon.absolute_floor_dimension_basis"
            })
            .unwrap();
        assert_eq!(basis.rank, 7);
        assert_eq!(basis.nullity(), 3);
        assert!(basis.coverage_complete);
    }

    #[test]
    fn every_serialized_null_vector_is_dimensionless() {
        let census = computed_census();
        for phenomenon in &census.phenomena {
            let columns = phenomenon
                .matrix_columns
                .iter()
                .map(|variable| {
                    SiDimensionColumn::new(
                        &variable.id,
                        dimension_from_exponents(variable.dimension),
                    )
                })
                .collect::<Vec<_>>();
            let analysis = SiDimensionAnalysis::analyze(&columns).unwrap();
            for vector in &phenomenon.null_space_basis {
                assert!(analysis.relation_is_dimensionless(vector).unwrap());
            }
        }
    }

    #[test]
    fn registry_order_is_identity_canonical_and_carries_no_values() {
        let census = computed_census();
        assert!(census
            .variables
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id));
        let debug = format!("{census:?}").to_ascii_lowercase();
        for forbidden in [
            "source_sha",
            "citation",
            "provenance",
            "seed",
            "caller",
            "magnitude",
            "world_identity",
        ] {
            assert!(
                !debug.contains(forbidden),
                "found forbidden field '{forbidden}'"
            );
        }
    }

    #[test]
    fn candidate_component_support_is_not_binary_shaped() {
        let census = computed_census();
        let topology = census
            .variables
            .iter()
            .find(|variable| variable.id == "stellar_birth.component.topology")
            .unwrap();
        assert_eq!(topology.carrier, CarrierKind::VariableCardinalityTopology);
        assert!(census
            .coverage_gap_ids
            .iter()
            .any(|gap| { gap == "stellar_birth.gap.nonbinary_variable_cardinality_dynamics" }));
    }

    #[test]
    fn dimensional_witnesses_report_reachability_and_their_nonzero_support() {
        let census = computed_census();
        let mean_particle_mass = census
            .phenomena
            .iter()
            .find(|phenomenon| {
                phenomenon.phenomenon_id == "stellar_birth.phenomenon.mean_particle_mass"
            })
            .unwrap();
        let attempt = &mean_particle_mass.derivation_attempts[0];
        assert_eq!(
            attempt.status,
            DerivationAttemptStatus::TargetDimensionReachable
        );
        assert_eq!(attempt.dimension_only_support_ids, ["fundamental.m_e"]);
        assert!(attempt
            .missing_dependency_ids
            .iter()
            .any(|id| id == "particle.species_rest_mass_spectrum"));
    }

    #[test]
    fn spectral_and_history_semantics_are_explicitly_open() {
        let census = computed_census();
        let radiation = census
            .variables
            .iter()
            .find(|variable| variable.id == "stellar_birth.radiation_flux_spectrum")
            .unwrap();
        assert_eq!(
            radiation.carrier,
            CarrierKind::SpectralFluxDensityPerLogFrequencyField
        );

        let collapse_flow = census
            .variables
            .iter()
            .find(|variable| variable.id == "stellar_birth.collapse_mass_flow")
            .unwrap();
        assert_eq!(collapse_flow.carrier, CarrierKind::TimeScalarHistory);
        let material_mass = census
            .variables
            .iter()
            .find(|variable| variable.id == "stellar_birth.material_element.mass_history")
            .unwrap();
        assert_eq!(
            material_mass.carrier,
            CarrierKind::MaterialTimeScalarHistory
        );
        let angular_momentum = census
            .variables
            .iter()
            .find(|variable| {
                variable.id == "stellar_birth.material_element.specific_angular_momentum_history"
            })
            .unwrap();
        assert_eq!(
            angular_momentum.carrier,
            CarrierKind::MaterialTimeVectorHistory
        );

        let history = census
            .phenomena
            .iter()
            .find(|phenomenon| {
                phenomenon.phenomenon_id == "stellar_birth.phenomenon.material_element_mass_history"
            })
            .unwrap();
        assert!(history.derivation_attempts[0]
            .missing_dependency_ids
            .iter()
            .any(|id| id == "collapse.initial_material_mass_or_integration_boundary"));
    }

    #[test]
    fn phenomenon_and_attempt_inputs_are_canonical_under_reordering() {
        let structure = stellar_birth_structure_schema().unwrap();
        let variables = build_variables(&structure).unwrap();
        let registry: BTreeMap<_, _> = variables
            .iter()
            .map(|variable| (variable.id.as_str(), variable))
            .collect();
        let forward = canonical_references(
            "fixture",
            &["stellar_birth.sound_speed_field", "fundamental.G"],
            &registry,
        )
        .unwrap();
        let reversed = canonical_references(
            "fixture",
            &["fundamental.G", "stellar_birth.sound_speed_field"],
            &registry,
        )
        .unwrap();
        assert_eq!(forward, reversed);

        let forward = build_phenomenon(
            PhenomenonSpec {
                phenomenon_id: "fixture.sound_speed",
                input_ids: &["stellar_birth.sound_speed_field", "fundamental.G"],
                output_ids: &["stellar_birth.collapse_mass_flow"],
                coverage_complete: false,
                attempts: &[],
            },
            &registry,
        )
        .unwrap();
        let reversed = build_phenomenon(
            PhenomenonSpec {
                phenomenon_id: "fixture.sound_speed",
                input_ids: &["fundamental.G", "stellar_birth.sound_speed_field"],
                output_ids: &["stellar_birth.collapse_mass_flow"],
                coverage_complete: false,
                attempts: &[],
            },
            &registry,
        )
        .unwrap();
        assert_eq!(forward, reversed);
    }

    #[test]
    fn a_variable_whose_carrier_is_absent_fails_closed() {
        let mut structure = stellar_birth_structure_schema().unwrap();
        structure
            .carrier_schemas
            .retain(|carrier| carrier.kind != CarrierKind::Scalar);
        assert!(matches!(
            build_variables(&structure),
            Err(CensusBuildError::UnknownCarrier(id)) if id == "scalar"
        ));
    }
}
