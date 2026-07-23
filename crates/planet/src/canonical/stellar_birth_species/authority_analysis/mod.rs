//! Non-admitting authority analysis for the open species-state derivation.

mod producer;
mod view;
mod watchdog;
mod wire;

pub use view::{SpeciesDerivationAnalysisView, SpeciesDerivationAttemptView};
pub(in crate::canonical) use wire::write_species_derivation_analysis;

use crate::canonical::{
    floor_magnitudes::AuditedFloorView,
    stellar_birth_structure::{
        stellar_birth_structure_schema, StellarBirthStructureSchema, StructureSchemaError,
    },
};
use civsim_units::physics_floor::sealed_physical_floor_authority_binding;
use std::fmt;

use producer::produce_frontier;
use watchdog::validate_analysis;

use super::COMPLETE_SPECIES_STATE_MEAN_PARTICLE_MASS_LAW_ID;

pub(in crate::canonical) const SPECIES_DERIVATION_ANALYSIS_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-species-derivation-analysis.v1";
pub(in crate::canonical) const SPECIES_DERIVATION_ANALYSIS_CHECKER_ID: &str =
    "civsim.planet.stellar-birth-species-derivation-watchdog.v2";

const FLOOR_ANCHOR_ID: &str = "fundamental.m_e";
const FLOOR_ANCHOR_SYMBOL: &str = "m_e";
const FLOOR_ANCHOR_ROLE: &str = "mass_coordinate_anchor_only";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AnalysisProgress {
    OpenDependencies,
    DimensionOnlyRelationNotPhysicalClosure,
    NotReached,
    NotClaimed,
    BlockedOpenProofs,
}

impl AnalysisProgress {
    const fn id(self) -> &'static str {
        match self {
            Self::OpenDependencies => "open_dependencies",
            Self::DimensionOnlyRelationNotPhysicalClosure => {
                "dimension_only_relation_not_physical_closure"
            }
            Self::NotReached => "not_reached",
            Self::NotClaimed => "not_claimed",
            Self::BlockedOpenProofs => "blocked_open_proofs",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SpeciesDerivationAttempt {
    id: &'static str,
    status: AnalysisProgress,
    input_ids: Vec<String>,
    open_proof_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FloorMassAnchor {
    id: &'static str,
    symbol: &'static str,
    bits: i128,
    scale_bits: u32,
    role: &'static str,
    membership_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::canonical) struct SpeciesDerivationAnalysis {
    floor_binding_schema_id: &'static str,
    floor_binding_sha256: String,
    structure_schema_id: &'static str,
    species_registry_schema_id: &'static str,
    stellar_state_schema_id: &'static str,
    state_coordinate_registry_schema_id: &'static str,
    interaction_sector_registry_schema_id: &'static str,
    physical_regime_registry_schema_id: &'static str,
    reducer_law_id: &'static str,
    floor_mass_anchor: FloorMassAnchor,
    attempts: Vec<SpeciesDerivationAttempt>,
    open_proof_ids: Vec<String>,
    candidate_member_count: usize,
    verified_support_member_count: usize,
    value_payload_present: bool,
    residual_slot_claim: bool,
    derive_first_status: AnalysisProgress,
    buckingham_pi_status: AnalysisProgress,
    gap_law_status: AnalysisProgress,
    chaos_protocol_status: AnalysisProgress,
    residual_law_status: AnalysisProgress,
    unique_residual_slot_status: AnalysisProgress,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::canonical) struct InvalidSpeciesDerivationAnalysis {
    error_code: &'static str,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::canonical) enum SpeciesDerivationAnalysisArtifact {
    Computed(Box<SpeciesDerivationAnalysis>),
    Invalid(InvalidSpeciesDerivationAnalysis),
}

struct SpeciesDerivationAnalysisAuthority {
    floor_binding_schema_id: &'static str,
    floor_binding_sha256: String,
    structure: StellarBirthStructureSchema,
    floor_mass_anchor: FloorMassAnchor,
}

impl SpeciesDerivationAnalysisAuthority {
    fn from_floor(floor: &AuditedFloorView<'_>) -> Result<Self, AnalysisBuildError> {
        let binding = sealed_physical_floor_authority_binding()
            .map_err(|error| AnalysisBuildError::FloorAuthority(error.to_string()))?;
        let structure = stellar_birth_structure_schema()?;
        let electron_mass = floor.magnitudes.electron_mass;
        if electron_mass.symbol() != FLOOR_ANCHOR_SYMBOL || electron_mass.bits() == 0 {
            return Err(AnalysisBuildError::FloorAnchorMismatch);
        }

        Ok(Self {
            floor_binding_schema_id: binding.schema_id().as_str(),
            floor_binding_sha256: binding.digest_hex(),
            structure,
            floor_mass_anchor: FloorMassAnchor {
                id: FLOOR_ANCHOR_ID,
                symbol: electron_mass.symbol(),
                bits: electron_mass.bits(),
                scale_bits: electron_mass.scale_bits(),
                role: FLOOR_ANCHOR_ROLE,
                membership_authority: false,
            },
        })
    }
}

#[derive(Debug)]
enum AnalysisBuildError {
    FloorAuthority(String),
    Structure(StructureSchemaError),
    FloorAnchorMismatch,
    InternalInvariant(String),
}

impl AnalysisBuildError {
    const fn code(&self) -> &'static str {
        match self {
            Self::FloorAuthority(_) => "floor_authority_unavailable",
            Self::Structure(_) => "structure_schema_unavailable",
            Self::FloorAnchorMismatch => "floor_mass_anchor_mismatch",
            Self::InternalInvariant(_) => "analysis_invariant_violation",
        }
    }
}

impl From<StructureSchemaError> for AnalysisBuildError {
    fn from(error: StructureSchemaError) -> Self {
        Self::Structure(error)
    }
}

impl fmt::Display for AnalysisBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FloorAuthority(detail) => write!(f, "physical-floor authority: {detail}"),
            Self::Structure(error) => write!(f, "stellar-birth structure: {error}"),
            Self::FloorAnchorMismatch => {
                f.write_str("audited electron-mass coordinate does not match its sealed anchor")
            }
            Self::InternalInvariant(detail) => write!(f, "analysis invariant: {detail}"),
        }
    }
}

fn build_analysis(
    authority: SpeciesDerivationAnalysisAuthority,
) -> Result<SpeciesDerivationAnalysis, AnalysisBuildError> {
    let frontier = produce_frontier();
    let analysis = SpeciesDerivationAnalysis {
        floor_binding_schema_id: authority.floor_binding_schema_id,
        floor_binding_sha256: authority.floor_binding_sha256,
        structure_schema_id: authority.structure.schema_id,
        species_registry_schema_id: authority.structure.species_registry.schema_id,
        stellar_state_schema_id: authority.structure.stellar_state.schema_id,
        state_coordinate_registry_schema_id: authority
            .structure
            .stellar_state
            .state_coordinate_registry
            .schema_id,
        interaction_sector_registry_schema_id: authority
            .structure
            .stellar_state
            .interaction_sector_registry
            .schema_id,
        physical_regime_registry_schema_id: authority
            .structure
            .stellar_state
            .physical_regime_registry
            .schema_id,
        reducer_law_id: COMPLETE_SPECIES_STATE_MEAN_PARTICLE_MASS_LAW_ID,
        floor_mass_anchor: authority.floor_mass_anchor,
        attempts: frontier.attempts,
        open_proof_ids: frontier.open_proof_ids,
        candidate_member_count: frontier.candidate_member_count,
        verified_support_member_count: frontier.verified_support_member_count,
        value_payload_present: frontier.value_payload_present,
        residual_slot_claim: frontier.residual_slot_claim,
        derive_first_status: frontier.derive_first_status,
        buckingham_pi_status: frontier.buckingham_pi_status,
        gap_law_status: frontier.gap_law_status,
        chaos_protocol_status: frontier.chaos_protocol_status,
        residual_law_status: frontier.residual_law_status,
        unique_residual_slot_status: frontier.unique_residual_slot_status,
    };
    validate_analysis(&analysis)?;
    Ok(analysis)
}

pub(in crate::canonical) fn analyze_repository_species_state_support(
    floor: &AuditedFloorView<'_>,
) -> SpeciesDerivationAnalysisArtifact {
    SpeciesDerivationAnalysisAuthority::from_floor(floor)
        .and_then(build_analysis)
        .map_or_else(
            |error| {
                SpeciesDerivationAnalysisArtifact::Invalid(InvalidSpeciesDerivationAnalysis {
                    error_code: error.code(),
                    detail: error.to_string(),
                })
            },
            |analysis| SpeciesDerivationAnalysisArtifact::Computed(Box::new(analysis)),
        )
}

impl SpeciesDerivationAnalysisArtifact {
    pub(in crate::canonical) const fn schema_id(&self) -> &'static str {
        SPECIES_DERIVATION_ANALYSIS_SCHEMA_ID
    }

    pub(in crate::canonical) const fn checker_id(&self) -> &'static str {
        SPECIES_DERIVATION_ANALYSIS_CHECKER_ID
    }

    pub(in crate::canonical) const fn status_id(&self) -> &'static str {
        match self {
            Self::Computed(_) => "open_dependencies",
            Self::Invalid(_) => "invalid",
        }
    }

    pub(in crate::canonical) const fn closure_effect_id(&self) -> &'static str {
        "none"
    }

    pub(in crate::canonical) const fn coverage_claim(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::{floor_magnitudes::AuditedFloorView, sealed_absolute_physics_floor};

    fn analysis() -> SpeciesDerivationAnalysisArtifact {
        let floor = sealed_absolute_physics_floor().expect("the physical floor seals");
        let floor_view =
            AuditedFloorView::from_floor(&floor).expect("the floor has typed magnitudes");
        analyze_repository_species_state_support(&floor_view)
    }

    #[test]
    fn production_analysis_is_bound_and_permanently_non_admitting() {
        let artifact = analysis();
        let view = SpeciesDerivationAnalysisView::new(&artifact);

        assert!(view.is_computed());
        assert!(view.floor_binding_sha256().is_some());
        assert_eq!(view.floor_anchor_id(), Some("fundamental.m_e"));
        assert_eq!(view.floor_anchor_symbol(), Some("m_e"));
        assert_eq!(
            view.floor_anchor_role(),
            Some("mass_coordinate_anchor_only")
        );
        assert_eq!(view.floor_anchor_membership_authority(), Some(false));
        assert_eq!(view.candidate_member_count(), Some(0));
        assert_eq!(view.verified_support_member_count(), Some(0));
        assert_eq!(view.value_payload_present(), Some(false));
        assert_eq!(view.residual_slot_claim(), Some(false));
        assert_eq!(view.gap_law_status_id(), Some("not_reached"));
        assert_eq!(view.chaos_protocol_status_id(), Some("not_reached"));
        assert_eq!(view.attempts().len(), 3);
        assert_eq!(
            view.open_proof_ids(),
            [
                "canonical_species_state_descriptor_checker_unavailable",
                "charge_state_sector_validity_proof_unavailable",
                "conditioned_zero_or_sparse_support_semantics_unavailable",
                "finite_exact_resource_domain_unavailable",
                "integer_projection_schema_unavailable",
                "joint_measure_support_binding_unavailable",
                "physical_species_membership_derivation_unavailable",
                "rest_mass_dimension_and_ancestry_proof_unavailable",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
        );
    }

    #[test]
    fn analysis_order_is_repeatable() {
        assert_eq!(analysis(), analysis());
        let artifact = analysis();
        let view = SpeciesDerivationAnalysisView::new(&artifact);
        assert_eq!(
            view.attempts()
                .map(|attempt| attempt.id())
                .collect::<Vec<_>>(),
            [
                "stellar_birth.species_derivation.complete_registry",
                "stellar_birth.species_derivation.complete_conditioned_support",
                "stellar_birth.species_derivation.exact_mean_mass_projection",
            ]
        );
    }

    #[test]
    fn an_anchor_mutation_fails_the_semantic_checker() {
        let SpeciesDerivationAnalysisArtifact::Computed(mut analysis) = analysis() else {
            panic!("the production analysis should compute");
        };
        analysis.floor_mass_anchor.bits += 1;

        assert!(validate_analysis(&analysis).is_err());
    }

    #[test]
    fn a_frontier_mutation_fails_the_independent_seal() {
        let SpeciesDerivationAnalysisArtifact::Computed(mut analysis) = analysis() else {
            panic!("the production analysis should compute");
        };
        analysis.attempts[0].id = "producer-selected-replacement";
        assert!(validate_analysis(&analysis).is_err());
    }
}
