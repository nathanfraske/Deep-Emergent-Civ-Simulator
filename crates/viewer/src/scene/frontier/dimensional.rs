//! Read-only projection of exact dimensional reachability witnesses.

/// Summary of the attached exact dimensional census.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionalCensusScene<'a> {
    pub(super) computed: bool,
    pub(super) representation_schema_id: Option<&'a str>,
    pub(super) floor_binding_schema_id: Option<&'a str>,
    pub(super) floor_binding_sha256: Option<&'a str>,
    pub(super) base_dimension_ids: Vec<&'a str>,
    pub(super) structure_schema_id: Option<&'a str>,
    pub(super) variable_count: usize,
    pub(super) phenomena: Vec<DimensionalPhenomenonScene<'a>>,
    pub(super) coverage_gap_ids: Vec<&'a str>,
    pub(super) error_code: Option<&'a str>,
    pub(super) error_detail: Option<&'a str>,
}

/// One exact input exponent in a dimension-only scale relation.
///
/// This is a reachability witness, not a physical law or a value. The output
/// dimension equals the product of the input dimensions raised to these exact
/// rational exponents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DimensionOnlyTermScene<'a> {
    input_id: &'a str,
    exponent_numerator: i128,
    exponent_denominator: i128,
}

/// One dimensionally reachable, physically unclosed derivation attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionalAttemptScene<'a> {
    id: &'a str,
    law_id: &'a str,
    output_id: &'a str,
    status_id: &'static str,
    dimension_only_terms: Vec<DimensionOnlyTermScene<'a>>,
    dimension_only_support_ids: Vec<&'a str>,
    missing_dependency_ids: Vec<&'a str>,
    dropped_mechanism_ids: Vec<&'a str>,
}

/// One independently analyzed phenomenon in the current route census.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionalPhenomenonScene<'a> {
    id: &'a str,
    coverage_complete: bool,
    attempts: Vec<DimensionalAttemptScene<'a>>,
}

impl<'a> DimensionOnlyTermScene<'a> {
    /// Identity of the input coordinate whose exponent is reported.
    pub const fn input_id(&self) -> &'a str {
        self.input_id
    }

    /// Exact rational exponent as numerator and positive denominator.
    pub const fn exponent(&self) -> (i128, i128) {
        (self.exponent_numerator, self.exponent_denominator)
    }
}

impl<'a> DimensionalAttemptScene<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        id: &'a str,
        law_id: &'a str,
        output_id: &'a str,
        status_id: &'static str,
        input_ids: &'a [String],
        projection: impl IntoIterator<Item = (i128, i128)>,
        dimension_only_support_ids: &'a [String],
        missing_dependency_ids: &'a [String],
        dropped_mechanism_ids: &'a [String],
    ) -> Self {
        let dimension_only_terms = input_ids
            .iter()
            .map(String::as_str)
            .zip(projection)
            .map(
                |(input_id, (exponent_numerator, exponent_denominator))| DimensionOnlyTermScene {
                    input_id,
                    exponent_numerator,
                    exponent_denominator,
                },
            )
            .collect();
        Self {
            id,
            law_id,
            output_id,
            status_id,
            dimension_only_terms,
            dimension_only_support_ids: borrowed_ids(dimension_only_support_ids),
            missing_dependency_ids: borrowed_ids(missing_dependency_ids),
            dropped_mechanism_ids: borrowed_ids(dropped_mechanism_ids),
        }
    }

    /// Stable derive-first attempt identity.
    pub const fn id(&self) -> &'a str {
        self.id
    }

    /// Candidate law identity. This does not grant law admission.
    pub const fn law_id(&self) -> &'a str {
        self.law_id
    }

    /// Coordinate whose dimension is reachable by this relation.
    pub const fn output_id(&self) -> &'a str {
        self.output_id
    }

    /// Exact dimension-checker status.
    pub const fn status_id(&self) -> &'static str {
        self.status_id
    }

    /// Exact exponent for every input, including zero-exponent inputs.
    pub fn dimension_only_terms(&self) -> &[DimensionOnlyTermScene<'a>] {
        &self.dimension_only_terms
    }

    /// Inputs carrying nonzero exponents in the dimension-only relation.
    pub fn dimension_only_support_ids(&self) -> &[&'a str] {
        &self.dimension_only_support_ids
    }

    /// Dependencies still required before the relation can become physical.
    pub fn missing_dependency_ids(&self) -> &[&'a str] {
        &self.missing_dependency_ids
    }

    /// Mechanisms omitted by this candidate relation.
    pub fn dropped_mechanism_ids(&self) -> &[&'a str] {
        &self.dropped_mechanism_ids
    }
}

impl<'a> DimensionalPhenomenonScene<'a> {
    pub(super) fn new(
        id: &'a str,
        coverage_complete: bool,
        attempts: Vec<DimensionalAttemptScene<'a>>,
    ) -> Self {
        Self {
            id,
            coverage_complete,
            attempts,
        }
    }

    /// Stable phenomenon identity.
    pub const fn id(&self) -> &'a str {
        self.id
    }

    /// Whether the dimension census claims complete mechanism coverage.
    pub const fn coverage_complete(&self) -> bool {
        self.coverage_complete
    }

    /// Dimension-only candidate relations in canonical attempt order.
    pub fn attempts(&self) -> &[DimensionalAttemptScene<'a>] {
        &self.attempts
    }
}

impl<'a> DimensionalCensusScene<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        computed: bool,
        representation_schema_id: Option<&'a str>,
        floor_binding_schema_id: Option<&'a str>,
        floor_binding_sha256: Option<&'a str>,
        base_dimension_ids: Vec<&'a str>,
        structure_schema_id: Option<&'a str>,
        variable_count: usize,
        phenomena: Vec<DimensionalPhenomenonScene<'a>>,
        coverage_gap_ids: &'a [String],
        error_code: Option<&'a str>,
        error_detail: Option<&'a str>,
    ) -> Self {
        Self {
            computed,
            representation_schema_id,
            floor_binding_schema_id,
            floor_binding_sha256,
            base_dimension_ids,
            structure_schema_id,
            variable_count,
            phenomena,
            coverage_gap_ids: borrowed_ids(coverage_gap_ids),
            error_code,
            error_detail,
        }
    }

    /// Whether the census artifact passed its semantic checker.
    pub const fn is_computed(&self) -> bool {
        self.computed
    }

    /// SI representation schema bound by the analysis.
    pub const fn representation_schema_id(&self) -> Option<&'a str> {
        self.representation_schema_id
    }

    /// Physical-floor authority schema bound by the analysis.
    pub const fn floor_binding_schema_id(&self) -> Option<&'a str> {
        self.floor_binding_schema_id
    }

    /// Physical-floor authority digest bound by the analysis.
    pub const fn floor_binding_sha256(&self) -> Option<&'a str> {
        self.floor_binding_sha256
    }

    /// Base dimensions in the bound representation schema order.
    pub fn base_dimension_ids(&self) -> &[&'a str] {
        &self.base_dimension_ids
    }

    /// Value-free stellar-birth structure schema.
    pub const fn structure_schema_id(&self) -> Option<&'a str> {
        self.structure_schema_id
    }

    /// Number of typed dimensional variables.
    pub const fn variable_count(&self) -> usize {
        self.variable_count
    }

    /// Number of independently analyzed phenomena.
    pub const fn phenomenon_count(&self) -> usize {
        self.phenomena.len()
    }

    /// Phenomenon-local dimensional relations and their open physics.
    pub fn phenomena(&self) -> &[DimensionalPhenomenonScene<'a>] {
        &self.phenomena
    }

    /// Explicit coverage gaps in canonical source order.
    pub fn coverage_gap_ids(&self) -> &[&'a str] {
        &self.coverage_gap_ids
    }

    /// Typed checker error when the census is invalid.
    pub const fn error_code(&self) -> Option<&'a str> {
        self.error_code
    }

    /// Checker detail when the census is invalid.
    pub const fn error_detail(&self) -> Option<&'a str> {
        self.error_detail
    }
}

fn borrowed_ids(values: &[String]) -> Vec<&str> {
    values.iter().map(String::as_str).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_preserves_exact_scale_terms_and_open_physics() {
        let inputs = vec![
            "fundamental.G".to_owned(),
            "stellar_birth.sound_speed_field".to_owned(),
        ];
        let support = inputs.clone();
        let missing = vec![
            "collapse.dimensionless_similarity_eigenstructure".to_owned(),
            "collapse.initial_and_boundary_measure".to_owned(),
        ];
        let dropped = vec!["mechanism.magnetized_collapse".to_owned()];
        let attempt = DimensionalAttemptScene::new(
            "stellar_birth.attempt.collapse_flow_from_sound_speed",
            "candidate.self_similar_isothermal_collapse",
            "stellar_birth.collapse_mass_flow",
            "target_dimension_reachable",
            &inputs,
            [(-1, 1), (3, 1)],
            &support,
            &missing,
            &dropped,
        );

        assert_eq!(attempt.dimension_only_terms()[0].exponent(), (-1, 1));
        assert_eq!(attempt.dimension_only_terms()[1].exponent(), (3, 1));
        assert_eq!(attempt.dimension_only_support_ids(), inputs);
        assert_eq!(attempt.missing_dependency_ids(), missing);
        assert_eq!(attempt.dropped_mechanism_ids(), dropped);
    }
}
