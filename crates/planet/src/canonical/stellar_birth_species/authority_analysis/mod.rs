//! Non-admitting authority analysis for the open species-state derivation.

mod view;
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

use super::COMPLETE_SPECIES_STATE_MEAN_PARTICLE_MASS_LAW_ID;

pub(in crate::canonical) const SPECIES_DERIVATION_ANALYSIS_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-species-derivation-analysis.v1";
pub(in crate::canonical) const SPECIES_DERIVATION_ANALYSIS_CHECKER_ID: &str =
    "civsim.planet.stellar-birth-species-derivation-checker.v1";

const FLOOR_ANCHOR_ID: &str = "fundamental.m_e";
const FLOOR_ANCHOR_SYMBOL: &str = "m_e";
const FLOOR_ANCHOR_ROLE: &str = "mass_coordinate_anchor_only";

const OPEN_PROOF_IDS: [&str; 8] = [
    "canonical_species_state_descriptor_checker_unavailable",
    "charge_state_sector_validity_proof_unavailable",
    "conditioned_zero_or_sparse_support_semantics_unavailable",
    "finite_exact_resource_domain_unavailable",
    "integer_projection_schema_unavailable",
    "joint_measure_support_binding_unavailable",
    "physical_species_membership_derivation_unavailable",
    "rest_mass_dimension_and_ancestry_proof_unavailable",
];

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

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn expected_attempts() -> Vec<SpeciesDerivationAttempt> {
    vec![
        SpeciesDerivationAttempt {
            id: "stellar_birth.species_derivation.complete_registry",
            status: AnalysisProgress::BlockedOpenProofs,
            input_ids: strings(&["fundamental.alpha", "fundamental.G", "fundamental.m_e"]),
            open_proof_ids: strings(&[
                "canonical_species_state_descriptor_checker_unavailable",
                "charge_state_sector_validity_proof_unavailable",
                "physical_species_membership_derivation_unavailable",
                "rest_mass_dimension_and_ancestry_proof_unavailable",
            ]),
        },
        SpeciesDerivationAttempt {
            id: "stellar_birth.species_derivation.complete_conditioned_support",
            status: AnalysisProgress::BlockedOpenProofs,
            input_ids: strings(&[
                "stellar_birth.composition.species_number_fraction_field",
                "stellar_birth.species_state_registry",
            ]),
            open_proof_ids: strings(&[
                "conditioned_zero_or_sparse_support_semantics_unavailable",
                "finite_exact_resource_domain_unavailable",
                "joint_measure_support_binding_unavailable",
            ]),
        },
        SpeciesDerivationAttempt {
            id: "stellar_birth.species_derivation.exact_mean_mass_projection",
            status: AnalysisProgress::BlockedOpenProofs,
            input_ids: strings(&["stellar_birth.conditioned_species_state_support"]),
            open_proof_ids: strings(&[
                "finite_exact_resource_domain_unavailable",
                "integer_projection_schema_unavailable",
                "joint_measure_support_binding_unavailable",
            ]),
        },
    ]
}

fn build_analysis(
    authority: SpeciesDerivationAnalysisAuthority,
) -> Result<SpeciesDerivationAnalysis, AnalysisBuildError> {
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
        attempts: expected_attempts(),
        open_proof_ids: strings(&OPEN_PROOF_IDS),
        candidate_member_count: 0,
        verified_support_member_count: 0,
        value_payload_present: false,
        residual_slot_claim: false,
        derive_first_status: AnalysisProgress::OpenDependencies,
        buckingham_pi_status: AnalysisProgress::DimensionOnlyRelationNotPhysicalClosure,
        gap_law_status: AnalysisProgress::NotReached,
        chaos_protocol_status: AnalysisProgress::NotReached,
        residual_law_status: AnalysisProgress::NotReached,
        unique_residual_slot_status: AnalysisProgress::NotClaimed,
    };
    validate_analysis(&analysis)?;
    Ok(analysis)
}

fn validate_analysis(analysis: &SpeciesDerivationAnalysis) -> Result<(), AnalysisBuildError> {
    let binding = sealed_physical_floor_authority_binding()
        .map_err(|error| AnalysisBuildError::FloorAuthority(error.to_string()))?;
    let structure = stellar_birth_structure_schema()?;
    let floor = crate::canonical::sealed_absolute_physics_floor()
        .map_err(|error| AnalysisBuildError::FloorAuthority(error.to_string()))?;
    let floor_view = AuditedFloorView::from_floor(&floor)
        .map_err(|error| AnalysisBuildError::FloorAuthority(error.to_string()))?;
    let expected_mass_anchor = floor_view.magnitudes.electron_mass;
    let expected_schema_ids = (
        structure.schema_id,
        structure.species_registry.schema_id,
        structure.stellar_state.schema_id,
        structure.stellar_state.state_coordinate_registry.schema_id,
        structure
            .stellar_state
            .interaction_sector_registry
            .schema_id,
        structure.stellar_state.physical_regime_registry.schema_id,
    );
    let found_schema_ids = (
        analysis.structure_schema_id,
        analysis.species_registry_schema_id,
        analysis.stellar_state_schema_id,
        analysis.state_coordinate_registry_schema_id,
        analysis.interaction_sector_registry_schema_id,
        analysis.physical_regime_registry_schema_id,
    );
    if found_schema_ids != expected_schema_ids {
        return Err(AnalysisBuildError::InternalInvariant(
            "schema binding differs from the canonical structure".to_owned(),
        ));
    }
    if analysis.floor_binding_schema_id != binding.schema_id().as_str()
        || analysis.floor_binding_sha256 != binding.digest_hex()
    {
        return Err(AnalysisBuildError::InternalInvariant(
            "physical-floor binding differs from the sealed authority".to_owned(),
        ));
    }
    if analysis.reducer_law_id != COMPLETE_SPECIES_STATE_MEAN_PARTICLE_MASS_LAW_ID
        || analysis.floor_mass_anchor.id != FLOOR_ANCHOR_ID
        || analysis.floor_mass_anchor.symbol != FLOOR_ANCHOR_SYMBOL
        || analysis.floor_mass_anchor.bits != expected_mass_anchor.bits()
        || analysis.floor_mass_anchor.scale_bits != expected_mass_anchor.scale_bits()
        || analysis.floor_mass_anchor.role != FLOOR_ANCHOR_ROLE
        || analysis.floor_mass_anchor.membership_authority
    {
        return Err(AnalysisBuildError::InternalInvariant(
            "floor mass anchor gained species membership authority or changed identity".to_owned(),
        ));
    }
    if analysis.attempts != expected_attempts()
        || analysis.open_proof_ids != strings(&OPEN_PROOF_IDS)
        || analysis.candidate_member_count != 0
        || analysis.verified_support_member_count != 0
        || analysis.value_payload_present
        || analysis.residual_slot_claim
        || analysis.derive_first_status != AnalysisProgress::OpenDependencies
        || analysis.buckingham_pi_status
            != AnalysisProgress::DimensionOnlyRelationNotPhysicalClosure
        || analysis.gap_law_status != AnalysisProgress::NotReached
        || analysis.chaos_protocol_status != AnalysisProgress::NotReached
        || analysis.residual_law_status != AnalysisProgress::NotReached
        || analysis.unique_residual_slot_status != AnalysisProgress::NotClaimed
    {
        return Err(AnalysisBuildError::InternalInvariant(
            "non-admitting derivation frontier changed".to_owned(),
        ));
    }
    Ok(())
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
        assert_eq!(view.open_proof_ids(), strings(&OPEN_PROOF_IDS));
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
}
