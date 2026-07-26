//! Non-admitting authority analysis for the open species-state derivation.

mod producer;
mod view;
mod watchdog;
mod wire;

pub use view::{
    PhysicalRootAdmissionView, SpeciesDerivationAnalysisView, SpeciesDerivationAttemptView,
};
pub(in crate::canonical) use wire::write_species_derivation_analysis;

use crate::canonical::{
    floor_magnitudes::AuditedFloorView,
    stellar_birth_species::physical_registry::{
        repository_physical_registry_frontier, RepositoryPhysicalRegistryFrontier,
    },
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
    "civsim.planet.stellar-birth-species-derivation-analysis.v4";
pub(in crate::canonical) const SPECIES_DERIVATION_ANALYSIS_CHECKER_ID: &str =
    "civsim.planet.stellar-birth-species-derivation-watchdog.v5";

const FLOOR_ANCHOR_ID: &str = "fundamental.m_e";
const FLOOR_ANCHOR_SYMBOL: &str = "m_e";
const FLOOR_ANCHOR_ROLE: &str = "mass_coordinate_anchor_only";
const FRONTIER_SOURCE_ID: &str = "repository_physical_registry_live_refusal";
const FRONTIER_SCOPE_ID: &str = "first_executable_refusal_only";
pub(super) const LIVE_PHYSICAL_REGISTRY_ATTEMPT_ID: &str =
    "stellar_birth.species_derivation.live_physical_registry_refusal";

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
    physical_registry_frontier: RepositoryPhysicalRegistryFrontier,
    frontier_source_id: &'static str,
    frontier_scope_id: &'static str,
    frontier_completeness_claim: bool,
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
    physical_registry_frontier: RepositoryPhysicalRegistryFrontier,
}

impl SpeciesDerivationAnalysisAuthority {
    fn from_floor(floor: &AuditedFloorView<'_>) -> Result<Self, AnalysisBuildError> {
        let binding = sealed_physical_floor_authority_binding()
            .map_err(|error| AnalysisBuildError::FloorAuthority(error.to_string()))?;
        let structure = stellar_birth_structure_schema()?;
        let physical_registry_frontier = repository_physical_registry_frontier()
            .map_err(|error| AnalysisBuildError::PhysicalRegistryFrontier(error.code.to_owned()))?;
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
            physical_registry_frontier,
        })
    }
}

#[derive(Debug)]
enum AnalysisBuildError {
    FloorAuthority(String),
    Structure(StructureSchemaError),
    FloorAnchorMismatch,
    PhysicalRegistryFrontier(String),
    InternalInvariant(String),
}

impl AnalysisBuildError {
    const fn code(&self) -> &'static str {
        match self {
            Self::FloorAuthority(_) => "floor_authority_unavailable",
            Self::Structure(_) => "structure_schema_unavailable",
            Self::FloorAnchorMismatch => "floor_mass_anchor_mismatch",
            Self::PhysicalRegistryFrontier(_) => "physical_registry_frontier_unavailable",
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
            Self::PhysicalRegistryFrontier(detail) => {
                write!(f, "physical-registry frontier: {detail}")
            }
            Self::InternalInvariant(detail) => write!(f, "analysis invariant: {detail}"),
        }
    }
}

fn build_analysis(
    authority: SpeciesDerivationAnalysisAuthority,
) -> Result<SpeciesDerivationAnalysis, AnalysisBuildError> {
    let frontier = produce_frontier(&authority.physical_registry_frontier);
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
        physical_registry_frontier: authority.physical_registry_frontier,
        frontier_source_id: FRONTIER_SOURCE_ID,
        frontier_scope_id: FRONTIER_SCOPE_ID,
        frontier_completeness_claim: false,
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
        assert_eq!(
            view.physical_registry_root_claim_id(),
            Some("planet.stellar-species-floor-coordinate-projection")
        );
        assert_eq!(
            view.physical_registry_root_canary_suite_id(),
            Some("civsim.planet.stellar-birth-repository-physical-root-canaries.v6")
        );
        assert_eq!(
            view.physical_registry_root_decision_id(),
            Some("agreed_projected")
        );
        assert_eq!(view.physical_registry_scalar_coordinate_count(), Some(3));
        assert_eq!(
            view.physical_registry_membership_neutral_mass_projection_count(),
            Some(1)
        );
        assert_eq!(view.physical_registry_admitted_root_count(), Some(4));
        assert_eq!(view.physical_registry_membership_authority(), Some(false));
        assert_eq!(view.physical_vocabulary_counts(), Some((4, 0, 4, 0)));
        assert_eq!(view.physical_vocabulary_scope(), Some((true, false, false)));
        assert_eq!(
            view.physical_vocabulary_relation_target_identities()
                .map(|identities| identities.len()),
            Some(4)
        );
        assert_ne!(view.physical_vocabulary_receipt_sha256(), Some([0; 32]));
        assert_eq!(
            view.physical_registry_refusal_code(),
            Some("no_admitted_species_derivation_rules")
        );
        assert_eq!(view.physical_registry_open_obligations().len(), 6);
        assert_ne!(view.physical_registry_root_receipt_sha256(), Some([0; 32]));
        assert_eq!(
            view.frontier_source_id(),
            Some("repository_physical_registry_live_refusal")
        );
        assert_eq!(
            view.frontier_scope_id(),
            Some("first_executable_refusal_only")
        );
        assert_eq!(view.frontier_completeness_claim(), Some(false));
        assert_eq!(view.candidate_member_count(), Some(0));
        assert_eq!(view.verified_support_member_count(), Some(0));
        assert_eq!(view.value_payload_present(), Some(false));
        assert_eq!(view.residual_slot_claim(), Some(false));
        assert_eq!(view.gap_law_status_id(), Some("not_reached"));
        assert_eq!(view.chaos_protocol_status_id(), Some("not_reached"));
        assert_eq!(view.attempts().len(), 1);
        assert_eq!(
            view.open_proof_ids(),
            [
                "admitted_constraint_laws",
                "admitted_physical_descriptor_roles",
                "admitted_species_derivation_rules",
                "complete_global_physical_vocabulary_coverage",
                "complete_registry_closure_domain",
                "species_mass_uncertainty_transport",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
        );
        assert_eq!(
            view.attempts()
                .next()
                .expect("one live attempt")
                .input_ids(),
            [
                "planet.stellar-species-floor-coordinate-projection",
                "civsim.planet.stellar-birth-physical-vocabulary.partition.v1",
            ]
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
            ["stellar_birth.species_derivation.live_physical_registry_refusal"]
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

    #[test]
    fn omitting_a_live_registry_obligation_fails_the_checker() {
        let SpeciesDerivationAnalysisArtifact::Computed(mut analysis) = analysis() else {
            panic!("the production analysis should compute");
        };
        let omitted = analysis.attempts[0]
            .open_proof_ids
            .pop()
            .expect("the live refusal has obligations");
        analysis.open_proof_ids.retain(|proof| proof != &omitted);

        assert!(validate_analysis(&analysis).is_err());
    }

    #[test]
    fn an_authored_downstream_path_cannot_gain_a_completeness_implication() {
        let SpeciesDerivationAnalysisArtifact::Computed(mut analysis) = analysis() else {
            panic!("the production analysis should compute");
        };
        analysis.attempts.push(SpeciesDerivationAttempt {
            id: "stellar_birth.species_derivation.authored_future_path",
            status: AnalysisProgress::BlockedOpenProofs,
            input_ids: vec!["future.authored.input".to_owned()],
            open_proof_ids: vec!["future.authored.proof".to_owned()],
        });
        analysis
            .open_proof_ids
            .push("future.authored.proof".to_owned());
        analysis.open_proof_ids.sort();

        assert!(validate_analysis(&analysis).is_err());
    }

    #[test]
    fn completeness_escalation_fails_the_checker() {
        let SpeciesDerivationAnalysisArtifact::Computed(mut analysis) = analysis() else {
            panic!("the production analysis should compute");
        };
        analysis.frontier_completeness_claim = true;

        assert!(validate_analysis(&analysis).is_err());
    }

    #[test]
    fn producer_follows_an_unfamiliar_live_obligation_without_an_authored_mirror() {
        let SpeciesDerivationAnalysisArtifact::Computed(analysis) = analysis() else {
            panic!("the production analysis should compute");
        };
        let mut physical = analysis.physical_registry_frontier.clone();
        physical.open_obligations = vec!["unfamiliar.live.registry.obligation"];

        let produced = produce_frontier(&physical);

        assert_eq!(produced.attempts.len(), 1);
        assert_eq!(
            produced.attempts[0].open_proof_ids,
            ["unfamiliar.live.registry.obligation"]
        );
        assert_eq!(
            produced.open_proof_ids,
            ["unfamiliar.live.registry.obligation"]
        );
    }
}
