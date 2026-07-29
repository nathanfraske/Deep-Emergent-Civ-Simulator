//! Open interaction-sector admission and conservation contract.
//!
//! A sector identity supplies no law or value. This schema states the proof
//! boundary a future familiar or unfamiliar sector must cross before a run is
//! sealed.

pub(super) const INTERACTION_SECTOR_REGISTRY_SCHEMA_ID: &str =
    "civsim.planet.stellar-interaction-sector-registry.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorMembershipRule {
    DerivedOrImmutablePresealAdmittedOnly,
}

impl SectorMembershipRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::DerivedOrImmutablePresealAdmittedOnly => {
                "derived_or_immutable_preseal_admitted_only"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorIdentityRule {
    CanonicalCompleteAdmittedSectorArtifactDigest,
}

impl SectorIdentityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CanonicalCompleteAdmittedSectorArtifactDigest => {
                "canonical_law_dependency_basis_field_charge_coupling_validity_conservation_digest"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorAdmissionRule {
    DeriveFirstPiGapChaosResidualUniqueOrRefusal,
}

impl SectorAdmissionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::DeriveFirstPiGapChaosResidualUniqueOrRefusal => {
                "derive_first_buckingham_pi_gap_chaos_residual_unique_or_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorFieldRule {
    CompleteCoordinateSourceCurrentAndChargeBinding,
}

impl SectorFieldRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CompleteCoordinateSourceCurrentAndChargeBinding => {
                "complete_coordinate_source_current_and_charge_binding"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorConservationRule {
    GlobalCrossSectorTransferLedgerBalances,
}

impl SectorConservationRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::GlobalCrossSectorTransferLedgerBalances => {
                "global_cross_sector_transfer_ledger_balances"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorCouplingRule {
    DerivedOrPresealAdmittedCouplingAncestry,
}

impl SectorCouplingRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::DerivedOrPresealAdmittedCouplingAncestry => {
                "derived_or_preseal_admitted_coupling_ancestry"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorValidityRule {
    CausalPredicateAdmissionAndDomainProofRequired,
}

impl SectorValidityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CausalPredicateAdmissionAndDomainProofRequired => {
                "causal_predicate_admission_and_domain_proof_required"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorDimensionBasisRule {
    ActiveBasisOrVersionedPresealExtension,
}

impl SectorDimensionBasisRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ActiveBasisOrVersionedPresealExtension => {
                "active_basis_or_versioned_preseal_extension"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorChaosRule {
    GapLawChaosProtocolPerDynamicalRegime,
}

impl SectorChaosRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::GapLawChaosProtocolPerDynamicalRegime => {
                "gap_law_chaos_protocol_per_dynamical_regime"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorExecutionRule {
    IdentityBlindCommonExecutionPath,
}

impl SectorExecutionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::IdentityBlindCommonExecutionPath => "identity_blind_common_execution_path",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorAbsenceRule {
    AbsentFromSealedCausalAuthorityIsNamedRefusal,
}

impl SectorAbsenceRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::AbsentFromSealedCausalAuthorityIsNamedRefusal => {
                "absent_from_sealed_causal_authority_is_named_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorExtensionRule {
    NoActiveRunRegistryExtension,
}

impl SectorExtensionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::NoActiveRunRegistryExtension => "no_active_run_registry_extension",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorCollisionRule {
    CompleteArtifactCollisionIsNamedRefusal,
}

impl SectorCollisionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CompleteArtifactCollisionIsNamedRefusal => {
                "complete_artifact_collision_is_named_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorCapacityRule {
    EngineLimitIsNamedRefusal,
}

impl SectorCapacityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::EngineLimitIsNamedRefusal => "engine_limit_is_named_refusal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SectorOrdinalRule {
    SerializationOnly,
}

impl SectorOrdinalRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::SerializationOnly => "serialization_only_never_identity_or_dispatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InteractionSectorRegistrySchema {
    pub(crate) schema_id: &'static str,
    pub(crate) membership_rule: SectorMembershipRule,
    pub(crate) identity_rule: SectorIdentityRule,
    pub(crate) admission_rule: SectorAdmissionRule,
    pub(crate) field_rule: SectorFieldRule,
    pub(crate) conservation_rule: SectorConservationRule,
    pub(crate) coupling_rule: SectorCouplingRule,
    pub(crate) validity_rule: SectorValidityRule,
    pub(crate) dimension_basis_rule: SectorDimensionBasisRule,
    pub(crate) chaos_rule: SectorChaosRule,
    pub(crate) execution_rule: SectorExecutionRule,
    pub(crate) absence_rule: SectorAbsenceRule,
    pub(crate) extension_rule: SectorExtensionRule,
    pub(crate) collision_rule: SectorCollisionRule,
    pub(crate) capacity_rule: SectorCapacityRule,
    pub(crate) ordinal_rule: SectorOrdinalRule,
}

impl InteractionSectorRegistrySchema {
    pub(super) fn is_canonical(&self) -> bool {
        self == &canonical_interaction_sector_registry_schema()
    }
}

pub(super) const fn canonical_interaction_sector_registry_schema() -> InteractionSectorRegistrySchema
{
    InteractionSectorRegistrySchema {
        schema_id: INTERACTION_SECTOR_REGISTRY_SCHEMA_ID,
        membership_rule: SectorMembershipRule::DerivedOrImmutablePresealAdmittedOnly,
        identity_rule: SectorIdentityRule::CanonicalCompleteAdmittedSectorArtifactDigest,
        admission_rule: SectorAdmissionRule::DeriveFirstPiGapChaosResidualUniqueOrRefusal,
        field_rule: SectorFieldRule::CompleteCoordinateSourceCurrentAndChargeBinding,
        conservation_rule: SectorConservationRule::GlobalCrossSectorTransferLedgerBalances,
        coupling_rule: SectorCouplingRule::DerivedOrPresealAdmittedCouplingAncestry,
        validity_rule: SectorValidityRule::CausalPredicateAdmissionAndDomainProofRequired,
        dimension_basis_rule: SectorDimensionBasisRule::ActiveBasisOrVersionedPresealExtension,
        chaos_rule: SectorChaosRule::GapLawChaosProtocolPerDynamicalRegime,
        execution_rule: SectorExecutionRule::IdentityBlindCommonExecutionPath,
        absence_rule: SectorAbsenceRule::AbsentFromSealedCausalAuthorityIsNamedRefusal,
        extension_rule: SectorExtensionRule::NoActiveRunRegistryExtension,
        collision_rule: SectorCollisionRule::CompleteArtifactCollisionIsNamedRefusal,
        capacity_rule: SectorCapacityRule::EngineLimitIsNamedRefusal,
        ordinal_rule: SectorOrdinalRule::SerializationOnly,
    }
}

/// Read-only view of the interaction-sector admission contract.
#[derive(Debug, Clone, Copy)]
pub struct InteractionSectorRegistrySchemaView<'a> {
    schema: &'a InteractionSectorRegistrySchema,
}

impl<'a> InteractionSectorRegistrySchemaView<'a> {
    pub(in crate::canonical) const fn new(schema: &'a InteractionSectorRegistrySchema) -> Self {
        Self { schema }
    }

    pub fn schema_id(self) -> &'static str {
        self.schema.schema_id
    }
    pub fn membership_rule_id(self) -> &'static str {
        self.schema.membership_rule.id()
    }
    pub fn identity_rule_id(self) -> &'static str {
        self.schema.identity_rule.id()
    }
    pub fn admission_rule_id(self) -> &'static str {
        self.schema.admission_rule.id()
    }
    pub fn field_rule_id(self) -> &'static str {
        self.schema.field_rule.id()
    }
    pub fn conservation_rule_id(self) -> &'static str {
        self.schema.conservation_rule.id()
    }
    pub fn coupling_rule_id(self) -> &'static str {
        self.schema.coupling_rule.id()
    }
    pub fn validity_rule_id(self) -> &'static str {
        self.schema.validity_rule.id()
    }
    pub fn dimension_basis_rule_id(self) -> &'static str {
        self.schema.dimension_basis_rule.id()
    }
    pub fn chaos_rule_id(self) -> &'static str {
        self.schema.chaos_rule.id()
    }
    pub fn execution_rule_id(self) -> &'static str {
        self.schema.execution_rule.id()
    }
    pub fn absence_rule_id(self) -> &'static str {
        self.schema.absence_rule.id()
    }
    pub fn extension_rule_id(self) -> &'static str {
        self.schema.extension_rule.id()
    }
    pub fn collision_rule_id(self) -> &'static str {
        self.schema.collision_rule.id()
    }
    pub fn capacity_rule_id(self) -> &'static str {
        self.schema.capacity_rule.id()
    }
    pub fn ordinal_rule_id(self) -> &'static str {
        self.schema.ordinal_rule.id()
    }
}
