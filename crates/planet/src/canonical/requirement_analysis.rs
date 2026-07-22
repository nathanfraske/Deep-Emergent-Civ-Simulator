//! Read-only views over non-admitting analyses attached to open requirements.
//!
//! Construction remains private to the canonical runner. Public views expose
//! typed refusal details without granting proof authority or requiring callers
//! to parse the canonical text bitstream.

use super::stellar_birth_dimensions::{
    DimensionalDerivationAttempt, DimensionalVariable, PhenomenonDimensionalCensus,
    StellarBirthDimensionalCensus, StellarBirthDimensionalCensusArtifact,
};
use super::stellar_birth_structure::{
    CarrierSchema, CarrierSchemaView, ComponentRegistrySchemaView, IndexDomain, IndexDomainView,
    SpeciesRegistrySchemaView, StellarStateSchemaView,
};

/// A typed, non-admitting analysis attached to an unresolved proof leaf.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequirementAnalysis {
    pub(super) payload: RequirementAnalysisPayload,
}

/// Read-only typed access to one exact dimensional-census artifact.
#[derive(Debug, Clone, Copy)]
pub struct ExactDimensionalCensusView<'a> {
    artifact: &'a StellarBirthDimensionalCensusArtifact,
}

/// Read-only view of one coordinate in an exact dimensional census.
#[derive(Debug, Clone, Copy)]
pub struct DimensionalVariableView<'a> {
    variable: &'a DimensionalVariable,
}

/// Read-only view of one phenomenon-local exact dimension matrix.
#[derive(Debug, Clone, Copy)]
pub struct DimensionalPhenomenonView<'a> {
    phenomenon: &'a PhenomenonDimensionalCensus,
}

/// Read-only view of one dimension-only candidate relation.
#[derive(Debug, Clone, Copy)]
pub struct DimensionalAttemptView<'a> {
    attempt: &'a DimensionalDerivationAttempt,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RequirementAnalysisPayload {
    ExactDimensionalCensus(StellarBirthDimensionalCensusArtifact),
}

impl RequirementAnalysis {
    pub(super) fn exact_dimensional_census(census: StellarBirthDimensionalCensusArtifact) -> Self {
        Self {
            payload: RequirementAnalysisPayload::ExactDimensionalCensus(census),
        }
    }

    pub const fn kind_id(&self) -> &'static str {
        match self.payload {
            RequirementAnalysisPayload::ExactDimensionalCensus(_) => "exact_dimensional_census",
        }
    }

    pub const fn schema_id(&self) -> &'static str {
        match &self.payload {
            RequirementAnalysisPayload::ExactDimensionalCensus(census) => census.schema_id(),
        }
    }

    pub const fn checker_id(&self) -> &'static str {
        match &self.payload {
            RequirementAnalysisPayload::ExactDimensionalCensus(census) => census.checker_id(),
        }
    }

    pub const fn status_id(&self) -> &'static str {
        match &self.payload {
            RequirementAnalysisPayload::ExactDimensionalCensus(census) => census.status_id(),
        }
    }

    pub const fn closure_effect_id(&self) -> &'static str {
        match &self.payload {
            RequirementAnalysisPayload::ExactDimensionalCensus(census) => {
                census.closure_effect().id()
            }
        }
    }

    pub const fn coverage_claim(&self) -> bool {
        match &self.payload {
            RequirementAnalysisPayload::ExactDimensionalCensus(census) => census.coverage_claim(),
        }
    }

    /// Inspect the exact census payload without granting construction access.
    pub const fn exact_dimensional_census_view(&self) -> Option<ExactDimensionalCensusView<'_>> {
        match &self.payload {
            RequirementAnalysisPayload::ExactDimensionalCensus(artifact) => {
                Some(ExactDimensionalCensusView { artifact })
            }
        }
    }
}

impl<'a> ExactDimensionalCensusView<'a> {
    fn computed(self) -> Option<&'a StellarBirthDimensionalCensus> {
        match self.artifact {
            StellarBirthDimensionalCensusArtifact::Computed(census) => Some(census),
            StellarBirthDimensionalCensusArtifact::Invalid(_) => None,
        }
    }

    pub fn is_computed(self) -> bool {
        self.computed().is_some()
    }

    pub fn representation_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|census| census.representation_schema_id)
    }

    pub fn floor_binding_schema_id(self) -> Option<&'a str> {
        self.computed().map(|census| census.floor_binding_schema_id)
    }

    pub fn floor_binding_sha256(self) -> Option<&'a str> {
        self.computed()
            .map(|census| census.floor_binding_sha256.as_str())
    }

    pub fn base_dimension_ids(self) -> &'a [&'static str] {
        self.computed()
            .map_or(&[], |census| census.base_dimension_ids.as_slice())
    }

    pub fn structure_schema_id(self) -> Option<&'a str> {
        self.computed().map(|census| census.structure.schema_id)
    }

    pub fn component_registry_schema(self) -> Option<ComponentRegistrySchemaView<'a>> {
        self.computed()
            .map(|census| ComponentRegistrySchemaView::new(&census.structure.component_registry))
    }

    pub fn species_registry_schema(self) -> Option<SpeciesRegistrySchemaView<'a>> {
        self.computed()
            .map(|census| SpeciesRegistrySchemaView::new(&census.structure.species_registry))
    }

    pub fn stellar_state_schema(self) -> Option<StellarStateSchemaView<'a>> {
        self.computed()
            .map(|census| StellarStateSchemaView::new(&census.structure.stellar_state))
    }

    pub fn index_domains(self) -> impl ExactSizeIterator<Item = IndexDomainView<'a>> + 'a {
        let domains: &'a [IndexDomain] = self
            .computed()
            .map_or(&[], |census| census.structure.index_domains.as_slice());
        domains.iter().map(IndexDomainView::new)
    }

    pub fn carrier_schemas(self) -> impl ExactSizeIterator<Item = CarrierSchemaView<'a>> + 'a {
        let schemas: &'a [CarrierSchema] = self
            .computed()
            .map_or(&[], |census| census.structure.carrier_schemas.as_slice());
        schemas.iter().map(CarrierSchemaView::new)
    }

    pub fn variables(self) -> impl ExactSizeIterator<Item = DimensionalVariableView<'a>> + 'a {
        let variables: &'a [DimensionalVariable] = self
            .computed()
            .map_or(&[], |census| census.variables.as_slice());
        variables
            .iter()
            .map(|variable| DimensionalVariableView { variable })
    }

    pub fn phenomena(self) -> impl ExactSizeIterator<Item = DimensionalPhenomenonView<'a>> + 'a {
        let phenomena: &'a [PhenomenonDimensionalCensus] = self
            .computed()
            .map_or(&[], |census| census.phenomena.as_slice());
        phenomena
            .iter()
            .map(|phenomenon| DimensionalPhenomenonView { phenomenon })
    }

    pub fn coverage_gap_ids(self) -> &'a [String] {
        self.computed()
            .map_or(&[], |census| census.coverage_gap_ids.as_slice())
    }

    pub fn error_code(self) -> Option<&'a str> {
        match self.artifact {
            StellarBirthDimensionalCensusArtifact::Computed(_) => None,
            StellarBirthDimensionalCensusArtifact::Invalid(census) => Some(census.error_code),
        }
    }

    pub fn error_detail(self) -> Option<&'a str> {
        match self.artifact {
            StellarBirthDimensionalCensusArtifact::Computed(_) => None,
            StellarBirthDimensionalCensusArtifact::Invalid(census) => Some(&census.detail),
        }
    }
}

impl<'a> DimensionalVariableView<'a> {
    pub fn id(self) -> &'a str {
        &self.variable.id
    }

    pub fn role_id(self) -> &'static str {
        self.variable.role.id()
    }

    pub fn carrier_id(self) -> &'static str {
        self.variable.carrier.id()
    }

    pub fn dimension(self) -> [i8; 7] {
        self.variable.dimension
    }

    pub fn coupling_group_id(self) -> &'a str {
        &self.variable.coupling_group_id
    }
}

impl<'a> DimensionalPhenomenonView<'a> {
    pub fn id(self) -> &'a str {
        &self.phenomenon.phenomenon_id
    }

    pub fn input_ids(self) -> &'a [String] {
        &self.phenomenon.input_ids
    }

    pub fn output_ids(self) -> &'a [String] {
        &self.phenomenon.output_ids
    }

    pub fn matrix_orientation(self) -> &'static str {
        self.phenomenon.matrix_orientation
    }

    pub fn matrix_columns(self) -> impl ExactSizeIterator<Item = DimensionalVariableView<'a>> + 'a {
        self.phenomenon
            .matrix_columns
            .iter()
            .map(|variable| DimensionalVariableView { variable })
    }

    pub fn rank(self) -> usize {
        self.phenomenon.rank
    }

    pub fn pivot_columns(self) -> &'a [usize] {
        &self.phenomenon.pivot_columns
    }

    pub fn free_columns(self) -> &'a [usize] {
        &self.phenomenon.free_columns
    }

    pub fn null_space_basis(self) -> &'a [Vec<i128>] {
        &self.phenomenon.null_space_basis
    }

    pub fn coverage_complete(self) -> bool {
        self.phenomenon.coverage_complete
    }

    pub fn derivation_attempts(
        self,
    ) -> impl ExactSizeIterator<Item = DimensionalAttemptView<'a>> + 'a {
        self.phenomenon
            .derivation_attempts
            .iter()
            .map(|attempt| DimensionalAttemptView { attempt })
    }
}

impl<'a> DimensionalAttemptView<'a> {
    pub fn id(self) -> &'a str {
        &self.attempt.attempt_id
    }

    pub fn law_id(self) -> &'a str {
        &self.attempt.law_id
    }

    pub fn input_ids(self) -> &'a [String] {
        &self.attempt.input_ids
    }

    pub fn output_id(self) -> &'a str {
        &self.attempt.output_id
    }

    pub fn status_id(self) -> &'static str {
        self.attempt.status.id()
    }

    pub fn dimension_only_projection(self) -> impl ExactSizeIterator<Item = (i128, i128)> + 'a {
        self.attempt
            .dimension_only_projection
            .iter()
            .map(|exponent| (exponent.numerator, exponent.denominator))
    }

    pub fn dimension_only_support_ids(self) -> &'a [String] {
        &self.attempt.dimension_only_support_ids
    }

    pub fn missing_dependency_ids(self) -> &'a [String] {
        &self.attempt.missing_dependency_ids
    }

    pub fn dropped_mechanism_ids(self) -> &'a [String] {
        &self.attempt.dropped_mechanism_ids
    }
}
