//! Complete value-free stellar-state trajectory contract.
//!
//! The contract composes open coordinate, interaction, physical-regime, and
//! presentation registries. It admits no state member, class, law, or value.

use super::{
    classification_registry::{
        canonical_classification_registry_schema, ClassificationRegistrySchema,
        ClassificationRegistrySchemaView,
    },
    interaction_sector_registry::{
        canonical_interaction_sector_registry_schema, InteractionSectorRegistrySchema,
        InteractionSectorRegistrySchemaView,
    },
    physical_regime_registry::{
        canonical_physical_regime_registry_schema, PhysicalRegimeRegistrySchema,
        PhysicalRegimeRegistrySchemaView,
    },
    state_coordinate_registry::{
        canonical_state_coordinate_registry_schema, StateCoordinateRegistrySchema,
        StateCoordinateRegistrySchemaView,
    },
    wire::is_canonical_wire_prefix,
};
use crate::canonical::transcript::canonical_text;
use std::fmt;

pub(super) const STELLAR_STATE_SCHEMA_ID: &str = "civsim.planet.stellar-state.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StellarStateRule {
    SealedAbsoluteFloorAndDerivedStateOnly,
    PresealDerivedFloorLawsMeasureCoordinateOrRefusal,
    MembershipCountAndIdentityDerivedOnly,
    InitialAndBoundaryStateDerivedOnlyOrRefusal,
    DispatchCandidatesFromAdmittedLawDependenciesOnly,
    ExecutableTransitionAuthorizedBySealedDagOnly,
    LineagePreservingCompletePhysicalStateHistory,
    PhysicalBirthDeathMergeSplitLineageOrNamedRefusal,
    CompleteApplicableConstituentStateHistoryOrNamedRefusal,
    GlobalSourceSinkTransportAndCrossSectorLedger,
    ApplicableAngularMomentumInertiaAndTorqueHistoryOrRefusal,
    EveryAdmittedFieldTopologyAndFluxHistoryOrRefusal,
    EveryAdmittedApplicableRadiationSpectrumAndTransportHistoryOrRefusal,
    DerivedApplicableGeometryAndBoundaryHistoryOrRefusal,
    ClassCannotSubstituteAndBoundaryCompletenessMustBeLawEntailed,
    DeterministicIntegerOrExactRationalCausalFloatConfirmationOnly,
    NoClassificationFeedback,
}

impl StellarStateRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::SealedAbsoluteFloorAndDerivedStateOnly => {
                "sealed_absolute_floor_and_derived_state_only"
            }
            Self::PresealDerivedFloorLawsMeasureCoordinateOrRefusal => {
                "preseal_derived_floor_laws_measure_coordinate_or_refusal"
            }
            Self::MembershipCountAndIdentityDerivedOnly => {
                "membership_count_and_identity_derived_only"
            }
            Self::InitialAndBoundaryStateDerivedOnlyOrRefusal => {
                "initial_and_boundary_state_derived_only_or_refusal"
            }
            Self::DispatchCandidatesFromAdmittedLawDependenciesOnly => {
                "dispatch_candidates_from_admitted_law_dependencies_only"
            }
            Self::ExecutableTransitionAuthorizedBySealedDagOnly => {
                "executable_transition_authorized_by_sealed_dag_only"
            }
            Self::LineagePreservingCompletePhysicalStateHistory => {
                "lineage_preserving_complete_physical_state_history"
            }
            Self::PhysicalBirthDeathMergeSplitLineageOrNamedRefusal => {
                "physical_birth_death_merge_split_lineage_or_named_refusal"
            }
            Self::CompleteApplicableConstituentStateHistoryOrNamedRefusal => {
                "complete_applicable_constituent_state_history_or_named_refusal"
            }
            Self::GlobalSourceSinkTransportAndCrossSectorLedger => {
                "global_source_sink_transport_and_cross_sector_ledger"
            }
            Self::ApplicableAngularMomentumInertiaAndTorqueHistoryOrRefusal => {
                "applicable_angular_momentum_inertia_and_torque_history_or_refusal"
            }
            Self::EveryAdmittedFieldTopologyAndFluxHistoryOrRefusal => {
                "every_admitted_field_topology_and_flux_history_or_refusal"
            }
            Self::EveryAdmittedApplicableRadiationSpectrumAndTransportHistoryOrRefusal => {
                "every_admitted_applicable_radiation_spectrum_and_transport_history_or_refusal"
            }
            Self::DerivedApplicableGeometryAndBoundaryHistoryOrRefusal => {
                "derived_applicable_geometry_and_boundary_history_or_refusal"
            }
            Self::ClassCannotSubstituteAndBoundaryCompletenessMustBeLawEntailed => {
                "class_cannot_substitute_and_boundary_completeness_must_be_law_entailed"
            }
            Self::DeterministicIntegerOrExactRationalCausalFloatConfirmationOnly => {
                "deterministic_integer_or_exact_rational_causal_float_confirmation_only"
            }
            Self::NoClassificationFeedback => "none",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StellarStateSchema {
    pub(crate) schema_id: &'static str,
    pub(crate) value_authority_rule: StellarStateRule,
    pub(crate) realization_rule: StellarStateRule,
    pub(crate) component_membership_rule: StellarStateRule,
    pub(crate) initialization_rule: StellarStateRule,
    pub(crate) dispatch_candidate_rule: StellarStateRule,
    pub(crate) transition_rule: StellarStateRule,
    pub(crate) trajectory_rule: StellarStateRule,
    pub(crate) lineage_rule: StellarStateRule,
    pub(crate) composition_rule: StellarStateRule,
    pub(crate) balance_rule: StellarStateRule,
    pub(crate) angular_momentum_rule: StellarStateRule,
    pub(crate) field_topology_rule: StellarStateRule,
    pub(crate) radiation_rule: StellarStateRule,
    pub(crate) geometry_rule: StellarStateRule,
    pub(crate) unresolved_interior_rule: StellarStateRule,
    pub(crate) numeric_rule: StellarStateRule,
    pub(crate) classification_feedback_rule: StellarStateRule,
    pub(crate) state_coordinate_registry: StateCoordinateRegistrySchema,
    pub(crate) interaction_sector_registry: InteractionSectorRegistrySchema,
    pub(crate) physical_regime_registry: PhysicalRegimeRegistrySchema,
    pub(crate) classification_registry: ClassificationRegistrySchema,
}

impl StellarStateSchema {
    pub(super) fn is_canonical(&self) -> bool {
        self == &canonical_stellar_state_schema()
            && self.state_coordinate_registry.is_canonical()
            && self.interaction_sector_registry.is_canonical()
            && self.physical_regime_registry.is_canonical()
            && self.classification_registry.is_canonical()
    }
}

pub(super) const fn canonical_stellar_state_schema() -> StellarStateSchema {
    StellarStateSchema {
        schema_id: STELLAR_STATE_SCHEMA_ID,
        value_authority_rule: StellarStateRule::SealedAbsoluteFloorAndDerivedStateOnly,
        realization_rule: StellarStateRule::PresealDerivedFloorLawsMeasureCoordinateOrRefusal,
        component_membership_rule: StellarStateRule::MembershipCountAndIdentityDerivedOnly,
        initialization_rule: StellarStateRule::InitialAndBoundaryStateDerivedOnlyOrRefusal,
        dispatch_candidate_rule:
            StellarStateRule::DispatchCandidatesFromAdmittedLawDependenciesOnly,
        transition_rule: StellarStateRule::ExecutableTransitionAuthorizedBySealedDagOnly,
        trajectory_rule: StellarStateRule::LineagePreservingCompletePhysicalStateHistory,
        lineage_rule: StellarStateRule::PhysicalBirthDeathMergeSplitLineageOrNamedRefusal,
        composition_rule: StellarStateRule::CompleteApplicableConstituentStateHistoryOrNamedRefusal,
        balance_rule: StellarStateRule::GlobalSourceSinkTransportAndCrossSectorLedger,
        angular_momentum_rule:
            StellarStateRule::ApplicableAngularMomentumInertiaAndTorqueHistoryOrRefusal,
        field_topology_rule: StellarStateRule::EveryAdmittedFieldTopologyAndFluxHistoryOrRefusal,
        radiation_rule:
            StellarStateRule::EveryAdmittedApplicableRadiationSpectrumAndTransportHistoryOrRefusal,
        geometry_rule: StellarStateRule::DerivedApplicableGeometryAndBoundaryHistoryOrRefusal,
        unresolved_interior_rule:
            StellarStateRule::ClassCannotSubstituteAndBoundaryCompletenessMustBeLawEntailed,
        numeric_rule:
            StellarStateRule::DeterministicIntegerOrExactRationalCausalFloatConfirmationOnly,
        classification_feedback_rule: StellarStateRule::NoClassificationFeedback,
        state_coordinate_registry: canonical_state_coordinate_registry_schema(),
        interaction_sector_registry: canonical_interaction_sector_registry_schema(),
        physical_regime_registry: canonical_physical_regime_registry_schema(),
        classification_registry: canonical_classification_registry_schema(),
    }
}

/// Read-only view of the composed stellar-state contract.
#[derive(Debug, Clone, Copy)]
pub struct StellarStateSchemaView<'a> {
    schema: &'a StellarStateSchema,
}

impl<'a> StellarStateSchemaView<'a> {
    pub(in crate::canonical) const fn new(schema: &'a StellarStateSchema) -> Self {
        Self { schema }
    }

    pub fn schema_id(self) -> &'static str {
        self.schema.schema_id
    }
    pub fn value_authority_rule_id(self) -> &'static str {
        self.schema.value_authority_rule.id()
    }
    pub fn realization_rule_id(self) -> &'static str {
        self.schema.realization_rule.id()
    }
    pub fn component_membership_rule_id(self) -> &'static str {
        self.schema.component_membership_rule.id()
    }
    pub fn initialization_rule_id(self) -> &'static str {
        self.schema.initialization_rule.id()
    }
    pub fn dispatch_candidate_rule_id(self) -> &'static str {
        self.schema.dispatch_candidate_rule.id()
    }
    pub fn transition_rule_id(self) -> &'static str {
        self.schema.transition_rule.id()
    }
    pub fn trajectory_rule_id(self) -> &'static str {
        self.schema.trajectory_rule.id()
    }
    pub fn lineage_rule_id(self) -> &'static str {
        self.schema.lineage_rule.id()
    }
    pub fn composition_rule_id(self) -> &'static str {
        self.schema.composition_rule.id()
    }
    pub fn balance_rule_id(self) -> &'static str {
        self.schema.balance_rule.id()
    }
    pub fn angular_momentum_rule_id(self) -> &'static str {
        self.schema.angular_momentum_rule.id()
    }
    pub fn field_topology_rule_id(self) -> &'static str {
        self.schema.field_topology_rule.id()
    }
    pub fn radiation_rule_id(self) -> &'static str {
        self.schema.radiation_rule.id()
    }
    pub fn geometry_rule_id(self) -> &'static str {
        self.schema.geometry_rule.id()
    }
    pub fn unresolved_interior_rule_id(self) -> &'static str {
        self.schema.unresolved_interior_rule.id()
    }
    pub fn numeric_rule_id(self) -> &'static str {
        self.schema.numeric_rule.id()
    }
    pub fn classification_feedback_rule_id(self) -> &'static str {
        self.schema.classification_feedback_rule.id()
    }

    pub fn state_coordinate_registry(self) -> StateCoordinateRegistrySchemaView<'a> {
        StateCoordinateRegistrySchemaView::new(&self.schema.state_coordinate_registry)
    }

    pub fn interaction_sector_registry(self) -> InteractionSectorRegistrySchemaView<'a> {
        InteractionSectorRegistrySchemaView::new(&self.schema.interaction_sector_registry)
    }

    pub fn physical_regime_registry(self) -> PhysicalRegimeRegistrySchemaView<'a> {
        PhysicalRegimeRegistrySchemaView::new(&self.schema.physical_regime_registry)
    }

    pub fn classification_registry(self) -> ClassificationRegistrySchemaView<'a> {
        ClassificationRegistrySchemaView::new(&self.schema.classification_registry)
    }
}

pub(super) fn write_stellar_state_schema(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    state: &StellarStateSchema,
) -> fmt::Result {
    if !is_canonical_wire_prefix(prefix) || !state.is_canonical() {
        return Err(fmt::Error);
    }
    let prefix = format!("{prefix}.stellar_state");
    writeln!(f, "{prefix}.schema={}", canonical_text(state.schema_id))?;
    write_state_rules(f, &prefix, state)?;
    write_coordinate_registry(f, &prefix, &state.state_coordinate_registry)?;
    write_sector_registry(f, &prefix, &state.interaction_sector_registry)?;
    write_regime_registry(f, &prefix, &state.physical_regime_registry)?;
    write_classification_registry(f, &prefix, &state.classification_registry)
}

fn write_state_rules(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    state: &StellarStateSchema,
) -> fmt::Result {
    for (field, rule) in [
        ("value_authority_rule", state.value_authority_rule),
        ("realization_rule", state.realization_rule),
        ("component_membership_rule", state.component_membership_rule),
        ("initialization_rule", state.initialization_rule),
        ("dispatch_candidate_rule", state.dispatch_candidate_rule),
        ("transition_rule", state.transition_rule),
        ("trajectory_rule", state.trajectory_rule),
        ("lineage_rule", state.lineage_rule),
        ("composition_rule", state.composition_rule),
        ("balance_rule", state.balance_rule),
        ("angular_momentum_rule", state.angular_momentum_rule),
        ("field_topology_rule", state.field_topology_rule),
        ("radiation_rule", state.radiation_rule),
        ("geometry_rule", state.geometry_rule),
        ("unresolved_interior_rule", state.unresolved_interior_rule),
        ("numeric_rule", state.numeric_rule),
        (
            "classification_feedback_rule",
            state.classification_feedback_rule,
        ),
    ] {
        writeln!(f, "{prefix}.{field}={}", rule.id())?;
    }
    Ok(())
}

fn write_coordinate_registry(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    registry: &StateCoordinateRegistrySchema,
) -> fmt::Result {
    let prefix = format!("{prefix}.state_coordinate_registry");
    writeln!(f, "{prefix}.schema={}", canonical_text(registry.schema_id))?;
    for (field, rule) in [
        ("membership_rule", registry.membership_rule.id()),
        ("identity_rule", registry.identity_rule.id()),
        ("dimension_rule", registry.dimension_rule.id()),
        ("index_rule", registry.index_rule.id()),
        ("tensor_rule", registry.tensor_rule.id()),
        ("normalization_rule", registry.normalization_rule.id()),
        ("reference_rule", registry.reference_rule.id()),
        ("completeness_rule", registry.completeness_rule.id()),
        ("collision_rule", registry.collision_rule.id()),
        ("extension_rule", registry.extension_rule.id()),
        ("capacity_rule", registry.capacity_rule.id()),
        ("ordinal_rule", registry.ordinal_rule.id()),
    ] {
        writeln!(f, "{prefix}.{field}={rule}")?;
    }
    let basis = &registry.dimension_basis_registry;
    let basis_prefix = format!("{prefix}.dimension_basis_registry");
    writeln!(
        f,
        "{basis_prefix}.schema={}",
        canonical_text(basis.schema_id)
    )?;
    for (field, rule) in [
        ("membership_rule", basis.membership_rule.id()),
        ("identity_rule", basis.identity_rule.id()),
        ("cardinality_rule", basis.cardinality_rule.id()),
        ("exponent_encoding_rule", basis.exponent_encoding_rule.id()),
        ("extension_rule", basis.extension_rule.id()),
        ("capacity_rule", basis.capacity_rule.id()),
        ("ordinal_rule", basis.ordinal_rule.id()),
    ] {
        writeln!(f, "{basis_prefix}.{field}={rule}")?;
    }
    Ok(())
}

fn write_sector_registry(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    registry: &InteractionSectorRegistrySchema,
) -> fmt::Result {
    let prefix = format!("{prefix}.interaction_sector_registry");
    writeln!(f, "{prefix}.schema={}", canonical_text(registry.schema_id))?;
    for (field, rule) in [
        ("membership_rule", registry.membership_rule.id()),
        ("identity_rule", registry.identity_rule.id()),
        ("admission_rule", registry.admission_rule.id()),
        ("field_rule", registry.field_rule.id()),
        ("conservation_rule", registry.conservation_rule.id()),
        ("coupling_rule", registry.coupling_rule.id()),
        ("validity_rule", registry.validity_rule.id()),
        ("dimension_basis_rule", registry.dimension_basis_rule.id()),
        ("chaos_rule", registry.chaos_rule.id()),
        ("execution_rule", registry.execution_rule.id()),
        ("absence_rule", registry.absence_rule.id()),
        ("extension_rule", registry.extension_rule.id()),
        ("collision_rule", registry.collision_rule.id()),
        ("capacity_rule", registry.capacity_rule.id()),
        ("ordinal_rule", registry.ordinal_rule.id()),
    ] {
        writeln!(f, "{prefix}.{field}={rule}")?;
    }
    Ok(())
}

fn write_regime_registry(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    registry: &PhysicalRegimeRegistrySchema,
) -> fmt::Result {
    let prefix = format!("{prefix}.physical_regime_registry");
    writeln!(f, "{prefix}.schema={}", canonical_text(registry.schema_id))?;
    for (field, rule) in [
        ("membership_rule", registry.membership_rule.id()),
        (
            "predicate_admission_rule",
            registry.predicate_admission_rule.id(),
        ),
        ("identity_rule", registry.identity_rule.id()),
        ("dependency_rule", registry.dependency_rule.id()),
        ("forbidden_input_rule", registry.forbidden_input_rule.id()),
        ("alias_rule", registry.alias_rule.id()),
        ("applicability_rule", registry.applicability_rule.id()),
        ("compatibility_rule", registry.compatibility_rule.id()),
        ("symmetry_rule", registry.symmetry_rule.id()),
        ("composition_rule", registry.composition_rule.id()),
        ("ambiguity_rule", registry.ambiguity_rule.id()),
        ("dispatch_rule", registry.dispatch_rule.id()),
        ("capacity_rule", registry.capacity_rule.id()),
        ("ordinal_rule", registry.ordinal_rule.id()),
    ] {
        writeln!(f, "{prefix}.{field}={rule}")?;
    }
    Ok(())
}

fn write_classification_registry(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    registry: &ClassificationRegistrySchema,
) -> fmt::Result {
    let prefix = format!("{prefix}.classification_registry");
    writeln!(f, "{prefix}.schema={}", canonical_text(registry.schema_id))?;
    for (field, rule) in [
        ("membership_rule", registry.membership_rule.id()),
        ("identity_rule", registry.identity_rule.id()),
        ("display_name_rule", registry.display_name_rule.id()),
        ("cardinality_rule", registry.cardinality_rule.id()),
        ("unclassified_rule", registry.unclassified_rule.id()),
        ("boundary_rule", registry.boundary_rule.id()),
        ("causal_authority_rule", registry.causal_authority_rule.id()),
        ("selector_rule", registry.selector_rule.id()),
        ("mutation_rule", registry.mutation_rule.id()),
        ("viewer_rule", registry.viewer_rule.id()),
        ("version_rule", registry.version_rule.id()),
        ("capacity_rule", registry.capacity_rule.id()),
        ("ordinal_rule", registry.ordinal_rule.id()),
    ] {
        writeln!(f, "{prefix}.{field}={rule}")?;
    }
    Ok(())
}
