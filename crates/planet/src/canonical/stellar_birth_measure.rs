//! Executable closure evaluator for the first physical realization measure.
//!
//! The contract stays smaller than a world-seed schema. One joint physical
//! measure preserves correlations across its support, and a separate
//! coordinate law selects a deterministic contingency only after that measure
//! closes. The evaluator accepts repository-owned typed proofs, never identity
//! strings, provenance tags, citations, caller values, or closure booleans.

use super::{
    floor_magnitudes::AuditedFloorView,
    requirement_analysis::RequirementAnalysis,
    stellar_birth_artifacts::{
        resolve_repository_artifacts, RepositoryStellarBirthArtifacts,
        VerifiedJointPhysicalMeasure, VerifiedRealizationCoordinateLaw,
    },
    stellar_birth_dimensions::stellar_birth_dimensional_census,
};
use std::fmt;

/// The Stage 1 root required by the canonical pipeline.
pub(crate) const STELLAR_BIRTH_REALIZATION_MEASURE: &str = "stellar_birth.realization_measure";

/// One typed leaf in the fixed Stage 1 proof contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StellarBirthLeaf {
    JointPhysicalMeasure,
    RealizationCoordinateLaw,
}

impl StellarBirthLeaf {
    const ORDERED: [Self; 2] = [Self::JointPhysicalMeasure, Self::RealizationCoordinateLaw];

    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::JointPhysicalMeasure => "stellar_birth.joint_physical_measure",
            Self::RealizationCoordinateLaw => "stellar_birth.realization_coordinate_law",
        }
    }

    pub(crate) const fn obligations(self) -> &'static [StellarBirthClosureObligation] {
        match self {
            Self::JointPhysicalMeasure => JOINT_MEASURE_OBLIGATIONS,
            Self::RealizationCoordinateLaw => COORDINATE_LAW_OBLIGATIONS,
        }
    }
}

/// Machine-readable proof clauses required before one leaf can close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StellarBirthClosureObligation {
    DerivationCensus,
    BuckinghamPiCensus,
    EvidenceCustody,
    TypedSupport,
    Normalization,
    Conditioning,
    CorrelationPreservation,
    UncertaintyPropagation,
    GapLaw,
    ChaosProtocol,
    ResidualLaw,
    UniqueResidualSlotIfIrreducible,
    VersionedCoordinateSemantics,
    CanonicalContentCoordinate,
    ObserverIndependence,
    OrderingIndependence,
    ExactReplay,
    AbsoluteFloorBinding,
    ArtifactSchemaVersion,
    SemanticCheckerVersion,
    DependencyDigest,
    JointMeasureBinding,
    OpenStellarStateCoverage,
    DimensionalClosure,
    DependencyAdmission,
    ValidityDomainProof,
    GlobalConservationProof,
    CoordinateTotalityOverJointSupport,
    MeasureConsistentPushForward,
    PresentationIndependence,
}

impl StellarBirthClosureObligation {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::DerivationCensus => "derivation_census",
            Self::BuckinghamPiCensus => "buckingham_pi_census",
            Self::EvidenceCustody => "evidence_custody",
            Self::TypedSupport => "typed_support",
            Self::Normalization => "normalization",
            Self::Conditioning => "conditioning",
            Self::CorrelationPreservation => "correlation_preservation",
            Self::UncertaintyPropagation => "uncertainty_propagation",
            Self::GapLaw => "gap_law",
            Self::ChaosProtocol => "gap_law.chaos_protocol",
            Self::ResidualLaw => "residual_law",
            Self::UniqueResidualSlotIfIrreducible => "unique_residual_slot_if_irreducible",
            Self::VersionedCoordinateSemantics => "versioned_coordinate_semantics",
            Self::CanonicalContentCoordinate => "canonical_content_coordinate",
            Self::ObserverIndependence => "observer_independence",
            Self::OrderingIndependence => "ordering_independence",
            Self::ExactReplay => "exact_replay",
            Self::AbsoluteFloorBinding => "absolute_floor_binding",
            Self::ArtifactSchemaVersion => "artifact_schema_version",
            Self::SemanticCheckerVersion => "semantic_checker_version",
            Self::DependencyDigest => "dependency_digest",
            Self::JointMeasureBinding => "joint_measure_binding",
            Self::OpenStellarStateCoverage => "open_stellar_state_and_projection_coverage",
            Self::DimensionalClosure => "dimensional_closure",
            Self::DependencyAdmission => "dependency_admission",
            Self::ValidityDomainProof => "validity_domain_proof",
            Self::GlobalConservationProof => "global_conservation_proof",
            Self::CoordinateTotalityOverJointSupport => "coordinate_totality_over_joint_support",
            Self::MeasureConsistentPushForward => "measure_consistent_push_forward",
            Self::PresentationIndependence => "presentation_identity_and_taxonomy_independence",
        }
    }
}

const JOINT_MEASURE_OBLIGATIONS: &[StellarBirthClosureObligation] = &[
    StellarBirthClosureObligation::DerivationCensus,
    StellarBirthClosureObligation::BuckinghamPiCensus,
    StellarBirthClosureObligation::EvidenceCustody,
    StellarBirthClosureObligation::TypedSupport,
    StellarBirthClosureObligation::Normalization,
    StellarBirthClosureObligation::Conditioning,
    StellarBirthClosureObligation::CorrelationPreservation,
    StellarBirthClosureObligation::UncertaintyPropagation,
    StellarBirthClosureObligation::GapLaw,
    StellarBirthClosureObligation::ChaosProtocol,
    StellarBirthClosureObligation::ResidualLaw,
    StellarBirthClosureObligation::UniqueResidualSlotIfIrreducible,
    StellarBirthClosureObligation::AbsoluteFloorBinding,
    StellarBirthClosureObligation::ArtifactSchemaVersion,
    StellarBirthClosureObligation::SemanticCheckerVersion,
    StellarBirthClosureObligation::DependencyDigest,
    StellarBirthClosureObligation::OpenStellarStateCoverage,
    StellarBirthClosureObligation::DimensionalClosure,
    StellarBirthClosureObligation::DependencyAdmission,
    StellarBirthClosureObligation::ValidityDomainProof,
    StellarBirthClosureObligation::GlobalConservationProof,
    StellarBirthClosureObligation::ObserverIndependence,
    StellarBirthClosureObligation::OrderingIndependence,
    StellarBirthClosureObligation::PresentationIndependence,
];

const COORDINATE_LAW_OBLIGATIONS: &[StellarBirthClosureObligation] = &[
    StellarBirthClosureObligation::VersionedCoordinateSemantics,
    StellarBirthClosureObligation::CanonicalContentCoordinate,
    StellarBirthClosureObligation::ObserverIndependence,
    StellarBirthClosureObligation::OrderingIndependence,
    StellarBirthClosureObligation::ExactReplay,
    StellarBirthClosureObligation::GapLaw,
    StellarBirthClosureObligation::ChaosProtocol,
    StellarBirthClosureObligation::ResidualLaw,
    StellarBirthClosureObligation::UniqueResidualSlotIfIrreducible,
    StellarBirthClosureObligation::AbsoluteFloorBinding,
    StellarBirthClosureObligation::ArtifactSchemaVersion,
    StellarBirthClosureObligation::SemanticCheckerVersion,
    StellarBirthClosureObligation::DependencyDigest,
    StellarBirthClosureObligation::JointMeasureBinding,
    StellarBirthClosureObligation::DerivationCensus,
    StellarBirthClosureObligation::BuckinghamPiCensus,
    StellarBirthClosureObligation::EvidenceCustody,
    StellarBirthClosureObligation::TypedSupport,
    StellarBirthClosureObligation::DimensionalClosure,
    StellarBirthClosureObligation::DependencyAdmission,
    StellarBirthClosureObligation::ValidityDomainProof,
    StellarBirthClosureObligation::GlobalConservationProof,
    StellarBirthClosureObligation::OpenStellarStateCoverage,
    StellarBirthClosureObligation::CoordinateTotalityOverJointSupport,
    StellarBirthClosureObligation::MeasureConsistentPushForward,
    StellarBirthClosureObligation::PresentationIndependence,
];

/// One unresolved typed leaf and the clauses its future proof must satisfy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StellarBirthOpenRequirement {
    leaf: StellarBirthLeaf,
    analyses: Vec<RequirementAnalysis>,
}

impl StellarBirthOpenRequirement {
    pub(crate) const fn requirement_id(&self) -> &'static str {
        self.leaf.id()
    }

    pub(crate) const fn obligations(&self) -> &'static [StellarBirthClosureObligation] {
        self.leaf.obligations()
    }

    pub(crate) fn analyses(&self) -> &[RequirementAnalysis] {
        &self.analyses
    }
}

/// Opaque capability produced only by both verified leaf proofs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StellarBirthMeasureCapability {
    _joint_measure: VerifiedJointPhysicalMeasure,
    _coordinate_law: VerifiedRealizationCoordinateLaw,
}

/// Typed reason Stage 1 cannot construct its stellar-birth measure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StellarBirthMeasureRefusal {
    frontier: Vec<StellarBirthOpenRequirement>,
}

impl StellarBirthMeasureRefusal {
    pub(crate) const fn requirement_id(&self) -> &'static str {
        STELLAR_BIRTH_REALIZATION_MEASURE
    }

    pub(crate) fn open_frontier(&self) -> &[StellarBirthOpenRequirement] {
        &self.frontier
    }
}

impl fmt::Display for StellarBirthMeasureRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "derived or admitted absolute-floor measure '{}' with {} open leaf requirement(s)",
            self.requirement_id(),
            self.frontier.len()
        )
    }
}

fn evaluate_artifacts(
    artifacts: RepositoryStellarBirthArtifacts,
    joint_measure_analysis: Option<RequirementAnalysis>,
) -> Result<StellarBirthMeasureCapability, StellarBirthMeasureRefusal> {
    let mut frontier = Vec::new();
    for leaf in StellarBirthLeaf::ORDERED {
        let closed = match leaf {
            StellarBirthLeaf::JointPhysicalMeasure => artifacts.joint_measure.is_some(),
            StellarBirthLeaf::RealizationCoordinateLaw => artifacts.coordinate_law.is_some(),
        };
        if !closed {
            let analyses = if leaf == StellarBirthLeaf::JointPhysicalMeasure {
                joint_measure_analysis.clone().into_iter().collect()
            } else {
                Vec::new()
            };
            frontier.push(StellarBirthOpenRequirement { leaf, analyses });
        }
    }

    if !frontier.is_empty() {
        return Err(StellarBirthMeasureRefusal { frontier });
    }

    Ok(StellarBirthMeasureCapability {
        _joint_measure: artifacts
            .joint_measure
            .expect("an empty frontier proves the joint measure is present"),
        _coordinate_law: artifacts
            .coordinate_law
            .expect("an empty frontier proves the coordinate law is present"),
    })
}

/// Require the closed measure contract before Stage 1 can generate a draw.
pub(crate) fn require_birth_measure(
    floor: &AuditedFloorView<'_>,
) -> Result<StellarBirthMeasureCapability, StellarBirthMeasureRefusal> {
    let census =
        RequirementAnalysis::exact_dimensional_census(stellar_birth_dimensional_census(floor));
    evaluate_artifacts(resolve_repository_artifacts(floor), Some(census))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::sealed_absolute_physics_floor;
    use civsim_ledger::AbsolutePhysicsFloor;

    fn audited_floor() -> AbsolutePhysicsFloor {
        sealed_absolute_physics_floor().expect("the audited physical catalog is admissible")
    }

    fn frontier_ids(refusal: &StellarBirthMeasureRefusal) -> Vec<&'static str> {
        refusal
            .open_frontier()
            .iter()
            .map(|requirement| requirement.requirement_id())
            .collect()
    }

    #[test]
    fn current_repository_artifacts_report_the_exact_open_frontier() {
        let floor = audited_floor();
        let floor_view =
            AuditedFloorView::from_floor(&floor).expect("the audited floor has typed magnitudes");
        let refusal = require_birth_measure(&floor_view)
            .expect_err("the present repository closes neither Stage 1 leaf");

        assert_eq!(
            frontier_ids(&refusal),
            vec![
                "stellar_birth.joint_physical_measure",
                "stellar_birth.realization_coordinate_law",
            ]
        );
        assert!(!frontier_ids(&refusal).contains(&STELLAR_BIRTH_REALIZATION_MEASURE));
    }

    #[test]
    fn each_partial_proof_reports_only_the_other_leaf() {
        let joint_only = evaluate_artifacts(
            RepositoryStellarBirthArtifacts::test_fixture(true, false),
            None,
        )
        .unwrap_err();
        assert_eq!(
            frontier_ids(&joint_only),
            vec!["stellar_birth.realization_coordinate_law"]
        );

        let coordinate_only = evaluate_artifacts(
            RepositoryStellarBirthArtifacts::test_fixture(false, true),
            None,
        )
        .unwrap_err();
        assert_eq!(
            frontier_ids(&coordinate_only),
            vec!["stellar_birth.joint_physical_measure"]
        );
    }

    #[test]
    fn the_root_closes_only_from_both_verified_typed_leaf_proofs() {
        assert!(evaluate_artifacts(
            RepositoryStellarBirthArtifacts::test_fixture(false, false),
            None,
        )
        .is_err());
        assert!(evaluate_artifacts(
            RepositoryStellarBirthArtifacts::test_fixture(true, false),
            None,
        )
        .is_err());
        assert!(evaluate_artifacts(
            RepositoryStellarBirthArtifacts::test_fixture(false, true),
            None,
        )
        .is_err());
        assert!(evaluate_artifacts(
            RepositoryStellarBirthArtifacts::test_fixture(true, true),
            None,
        )
        .is_ok());
    }

    #[test]
    fn frontier_order_is_canonical_and_repeatable() {
        let first = evaluate_artifacts(
            RepositoryStellarBirthArtifacts::test_fixture(false, false),
            None,
        )
        .unwrap_err();
        let second = evaluate_artifacts(
            RepositoryStellarBirthArtifacts::test_fixture(false, false),
            None,
        )
        .unwrap_err();
        assert_eq!(first, second);
        assert_eq!(frontier_ids(&first), frontier_ids(&second));
    }

    #[test]
    fn both_leaves_carry_the_chaos_protocol_as_a_gap_law_clause() {
        for leaf in StellarBirthLeaf::ORDERED {
            let ids: Vec<_> = leaf
                .obligations()
                .iter()
                .map(|obligation| obligation.id())
                .collect();
            assert!(ids.contains(&"gap_law"));
            assert!(ids.contains(&"gap_law.chaos_protocol"));
        }
        let coordinate_ids: Vec<_> = StellarBirthLeaf::RealizationCoordinateLaw
            .obligations()
            .iter()
            .map(|obligation| obligation.id())
            .collect();
        assert!(coordinate_ids.contains(&"residual_law"));
        assert!(coordinate_ids.contains(&"unique_residual_slot_if_irreducible"));
        let joint_ids: Vec<_> = StellarBirthLeaf::JointPhysicalMeasure
            .obligations()
            .iter()
            .map(|obligation| obligation.id())
            .collect();
        assert!(joint_ids.contains(&"open_stellar_state_and_projection_coverage"));
    }

    #[test]
    fn both_leaves_carry_the_complete_causal_admission_bundle() {
        let common = [
            StellarBirthClosureObligation::DerivationCensus,
            StellarBirthClosureObligation::BuckinghamPiCensus,
            StellarBirthClosureObligation::EvidenceCustody,
            StellarBirthClosureObligation::TypedSupport,
            StellarBirthClosureObligation::GapLaw,
            StellarBirthClosureObligation::ChaosProtocol,
            StellarBirthClosureObligation::ResidualLaw,
            StellarBirthClosureObligation::UniqueResidualSlotIfIrreducible,
            StellarBirthClosureObligation::AbsoluteFloorBinding,
            StellarBirthClosureObligation::ArtifactSchemaVersion,
            StellarBirthClosureObligation::SemanticCheckerVersion,
            StellarBirthClosureObligation::DependencyDigest,
            StellarBirthClosureObligation::DimensionalClosure,
            StellarBirthClosureObligation::DependencyAdmission,
            StellarBirthClosureObligation::ValidityDomainProof,
            StellarBirthClosureObligation::GlobalConservationProof,
            StellarBirthClosureObligation::OpenStellarStateCoverage,
        ];
        for leaf in StellarBirthLeaf::ORDERED {
            for &obligation in &common {
                assert!(leaf.obligations().contains(&obligation));
            }
        }

        let joint = StellarBirthLeaf::JointPhysicalMeasure.obligations();
        assert!(joint.contains(&StellarBirthClosureObligation::ObserverIndependence));
        assert!(joint.contains(&StellarBirthClosureObligation::OrderingIndependence));
        assert!(joint.contains(&StellarBirthClosureObligation::PresentationIndependence));

        let coordinate = StellarBirthLeaf::RealizationCoordinateLaw.obligations();
        assert!(
            coordinate.contains(&StellarBirthClosureObligation::CoordinateTotalityOverJointSupport)
        );
        assert!(coordinate.contains(&StellarBirthClosureObligation::MeasureConsistentPushForward));
    }

    #[test]
    fn production_attaches_one_non_admitting_census_only_to_the_joint_leaf() {
        let floor = audited_floor();
        let floor_view =
            AuditedFloorView::from_floor(&floor).expect("the audited floor has typed magnitudes");
        let refusal = require_birth_measure(&floor_view).unwrap_err();
        let joint = &refusal.open_frontier()[0];
        let coordinate = &refusal.open_frontier()[1];

        assert_eq!(joint.analyses().len(), 1);
        assert_eq!(joint.analyses()[0].kind_id(), "exact_dimensional_census");
        assert_eq!(joint.analyses()[0].status_id(), "computed");
        assert_eq!(joint.analyses()[0].closure_effect_id(), "none");
        assert!(!joint.analyses()[0].coverage_claim());
        assert!(coordinate.analyses().is_empty());
    }

    #[test]
    fn the_joint_measure_remains_one_correlation_preserving_obligation() {
        let ids: Vec<_> = StellarBirthLeaf::ORDERED
            .into_iter()
            .map(StellarBirthLeaf::id)
            .collect();
        assert!(ids.contains(&"stellar_birth.joint_physical_measure"));
        assert!(StellarBirthLeaf::JointPhysicalMeasure
            .obligations()
            .contains(&StellarBirthClosureObligation::CorrelationPreservation));
        assert!(ids.iter().all(|id| {
            !id.contains("epoch")
                && !id.contains("environment")
                && !id.contains("metallicity")
                && !id.contains("scalar")
        }));
    }
}
