//! Exact stage and refusal-frontier projections.

mod dimensional;

pub use dimensional::{
    DimensionOnlyTermScene, DimensionalAttemptScene, DimensionalCensusScene,
    DimensionalPhenomenonScene,
};

use super::TranscriptScene;
use crate::RefusalView;

/// Read-only physical-vocabulary partitions in descriptor, target, law order.
pub type PhysicalVocabularyPartitions<'a> = (&'a [[u8; 32]], &'a [[u8; 32]], &'a [[u8; 32]]);

/// Producer/watchdog result digests followed by their resource digests.
pub type PhysicalVocabularyCheckerDigests = (([u8; 32], [u8; 32]), ([u8; 32], [u8; 32]));

/// Charge-conjugation and mass-transport producer and watchdog digests.
pub type ChargedProfileCheckerEvidenceDigests = ([u8; 32], [u8; 32], [u8; 32], [u8; 32]);

/// Solver, normalization, threshold, uncertainty, and conservation producer
/// and watchdog digests in paired order.
pub type NeutralBoundProfileCheckerEvidenceDigests = [[u8; 32]; 10];

/// Constituent threshold channel, open decay family, mass interval, and
/// constituent threshold interval digests.
pub type NeutralBoundProfileChannelDigests = ([u8; 32], [u8; 32], [u8; 32], [u8; 32]);

/// Curvature, conservation, confinement, applicability, validity, and
/// asymptotic producer and watchdog digests in paired order.
pub type StrongProfileCheckerEvidenceDigests = [[u8; 32]; 12];

/// One canonical stage status in fixed pipeline order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageScene {
    id: &'static str,
    status_id: &'static str,
    entered_event_ordinal: Option<u64>,
    terminal_event_ordinal: Option<u64>,
}

impl StageScene {
    /// Stable stage identity.
    pub const fn id(self) -> &'static str {
        self.id
    }

    /// Stable reached, refused, or not-reached identity.
    pub const fn status_id(self) -> &'static str {
        self.status_id
    }

    /// Event that entered this stage, when present.
    pub const fn entered_event_ordinal(self) -> Option<u64> {
        self.entered_event_ordinal
    }

    /// Event that reached or refused this stage, when present.
    pub const fn terminal_event_ordinal(self) -> Option<u64> {
        self.terminal_event_ordinal
    }
}

/// One blocked derive-first species attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesAttemptScene<'a> {
    id: &'a str,
    status_id: &'static str,
    input_ids: Vec<&'a str>,
    open_proof_ids: Vec<&'a str>,
}

impl<'a> SpeciesAttemptScene<'a> {
    /// Stable derivation-attempt identity.
    pub const fn id(&self) -> &'a str {
        self.id
    }

    /// Exact attempt status.
    pub const fn status_id(&self) -> &'static str {
        self.status_id
    }

    /// Inputs consulted by this attempt in canonical order.
    pub fn input_ids(&self) -> &[&'a str] {
        &self.input_ids
    }

    /// Proofs still open after this attempt.
    pub fn open_proof_ids(&self) -> &[&'a str] {
        &self.open_proof_ids
    }
}

/// Read-only projection of the repository physical-registry frontier.
///
/// These fields report the planet-owned authority receipts, the bounded local
/// members, and the exact open global frontier. They cannot admit a root,
/// create membership, or alter the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalRootAdmissionScene<'a> {
    identity_sha256: [u8; 32],
    tier_id: &'a str,
    provenance_tag: &'a str,
    route_id: &'a str,
}

impl<'a> PhysicalRootAdmissionScene<'a> {
    /// Exact identity of the admitted root.
    pub const fn identity_sha256(&self) -> [u8; 32] {
        self.identity_sha256
    }

    /// Ledger tier attached to this admitted root.
    pub const fn tier_id(&self) -> &'a str {
        self.tier_id
    }

    /// Canonical provenance tag attached to this admitted root.
    pub const fn provenance_tag(&self) -> &'a str {
        self.provenance_tag
    }

    /// Admission route used for this root.
    pub const fn route_id(&self) -> &'a str {
        self.route_id
    }
}

/// Read-only projection of the one locally closed primitive profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPrimitiveProfileScene<'a> {
    identity: (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ),
    dynamical_identity: (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ),
    member_properties: (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str),
    counts: (u32, u32, u32),
    admissions: Vec<PhysicalRootAdmissionScene<'a>>,
    member_sha256: [u8; 32],
    receipt_sha256: [u8; 32],
    evidence_sha256: ([u8; 32], [u8; 32], [u8; 32], [u8; 32]),
    repository_catalog_sha256: [u8; 32],
    protocol_checker_result_sha256: ([u8; 32], [u8; 32]),
    protocol_statuses: (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str),
}

impl<'a> RepositoryPrimitiveProfileScene<'a> {
    /// Receipt, claim, profile, theory, residual slot, and checker identities.
    pub const fn identity(
        &self,
    ) -> (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ) {
        self.identity
    }

    /// Member, symmetry, field, operator, state, sector, validity, and excluded term.
    pub const fn dynamical_identity(
        &self,
    ) -> (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ) {
        self.dynamical_identity
    }

    /// Helicity, statistics, charge, current, stability, and transition labels.
    pub const fn member_properties(
        &self,
    ) -> (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str) {
        self.member_properties
    }

    /// Artifact count, complete quadratic basis count, and excluded operator count.
    pub const fn counts(&self) -> (u32, u32, u32) {
        self.counts
    }

    /// Identity-keyed tier, provenance, and route rows for profile artifacts.
    pub fn admissions(&self) -> &[PhysicalRootAdmissionScene<'a>] {
        &self.admissions
    }

    /// Identity of the one locally closed physical member.
    pub const fn member_sha256(&self) -> [u8; 32] {
        self.member_sha256
    }

    /// Digest of the paired profile projection receipt.
    pub const fn receipt_sha256(&self) -> [u8; 32] {
        self.receipt_sha256
    }

    /// Symmetry action, derivation catalog, coverage, and protocol digests.
    pub const fn evidence_sha256(&self) -> ([u8; 32], [u8; 32], [u8; 32], [u8; 32]) {
        self.evidence_sha256
    }

    /// Registered premise-catalog digest scanned before irreducible admission.
    pub const fn repository_catalog_sha256(&self) -> [u8; 32] {
        self.repository_catalog_sha256
    }

    /// Independent protocol producer and watchdog result digests.
    pub const fn protocol_checker_result_sha256(&self) -> ([u8; 32], [u8; 32]) {
        self.protocol_checker_result_sha256
    }

    /// Derive-first, Pi, Gap, Chaos, Residual, and residual-slot statuses.
    pub const fn protocol_statuses(
        &self,
    ) -> (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str) {
        self.protocol_statuses
    }
}

/// Read-only projection of the locally closed charged conjugation orbit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryChargedProfileScene<'a> {
    identity: (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ),
    counts: (u32, usize),
    admissions: Vec<PhysicalRootAdmissionScene<'a>>,
    member_sha256: Vec<[u8; 32]>,
    receipt_sha256: [u8; 32],
    checker_evidence_sha256: ChargedProfileCheckerEvidenceDigests,
    protocol_statuses: (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str),
}

impl<'a> RepositoryChargedProfileScene<'a> {
    /// Receipt, claim, profile, theory, residual slot, and checker identities.
    pub const fn identity(
        &self,
    ) -> (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ) {
        self.identity
    }

    /// Artifact and member counts for the closed conjugation orbit.
    pub const fn counts(&self) -> (u32, usize) {
        self.counts
    }

    /// Identity-keyed tier, provenance, and route rows for profile artifacts.
    pub fn admissions(&self) -> &[PhysicalRootAdmissionScene<'a>] {
        &self.admissions
    }

    /// Sorted identities of both locally closed charged members.
    pub fn member_sha256(&self) -> &[[u8; 32]] {
        &self.member_sha256
    }

    /// Digest of the paired charged-profile projection receipt.
    pub const fn receipt_sha256(&self) -> [u8; 32] {
        self.receipt_sha256
    }

    /// Charge-conjugation and mass-transport producer and watchdog digests.
    pub const fn checker_evidence_sha256(&self) -> ChargedProfileCheckerEvidenceDigests {
        self.checker_evidence_sha256
    }

    /// Derive-first, Pi, Gap, Chaos, Residual, and residual-slot statuses.
    pub const fn protocol_statuses(
        &self,
    ) -> (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str) {
        self.protocol_statuses
    }
}

/// Read-only projection of one locally closed neutral two-body bound profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryNeutralBoundProfileScene<'a> {
    identity: (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ),
    counts: (u32, usize),
    admissions: Vec<PhysicalRootAdmissionScene<'a>>,
    member_sha256: [u8; 32],
    receipt_sha256: [u8; 32],
    checker_evidence_sha256: NeutralBoundProfileCheckerEvidenceDigests,
    channel_digests: NeutralBoundProfileChannelDigests,
    dispositions: (&'a str, &'a str),
    scope: (bool, bool),
    protocol_statuses: (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str),
}

impl<'a> RepositoryNeutralBoundProfileScene<'a> {
    /// Receipt, claim, profile, theory, residual slot, and checker identities.
    pub const fn identity(
        &self,
    ) -> (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ) {
        self.identity
    }

    /// Artifact count and the one admitted local member.
    pub const fn counts(&self) -> (u32, usize) {
        self.counts
    }

    /// Identity-keyed tier, provenance, and route rows for profile artifacts.
    pub fn admissions(&self) -> &[PhysicalRootAdmissionScene<'a>] {
        &self.admissions
    }

    /// Identity of the locally closed neutral bound member.
    pub const fn member_sha256(&self) -> [u8; 32] {
        self.member_sha256
    }

    /// Digest of the paired neutral-bound-profile receipt.
    pub const fn receipt_sha256(&self) -> [u8; 32] {
        self.receipt_sha256
    }

    /// Independent solver, normalization, threshold, uncertainty, and
    /// conservation evidence in producer and watchdog pairs.
    pub const fn checker_evidence_sha256(&self) -> NeutralBoundProfileCheckerEvidenceDigests {
        self.checker_evidence_sha256
    }

    /// Threshold channel, open decay family, and exact interval digests.
    pub const fn channel_digests(&self) -> NeutralBoundProfileChannelDigests {
        self.channel_digests
    }

    /// Constituent-binding and open-decay dispositions.
    pub const fn dispositions(&self) -> (&'a str, &'a str) {
        self.dispositions
    }

    /// Conditioned-support authority and global-stability claim flags.
    pub const fn scope(&self) -> (bool, bool) {
        self.scope
    }

    /// Derive-first, Pi, Gap, Chaos, Residual, and residual-slot statuses.
    pub const fn protocol_statuses(
        &self,
    ) -> (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str) {
        self.protocol_statuses
    }
}

/// Read-only projection of the claim-local confining interaction profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryStrongProfileScene<'a> {
    identity: (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ),
    counts: (u32, u32),
    admissions: Vec<PhysicalRootAdmissionScene<'a>>,
    identity_digests: [[u8; 32]; 4],
    receipt_digests: ([u8; 32], [u8; 32]),
    checker_evidence_sha256: StrongProfileCheckerEvidenceDigests,
    scope: (bool, bool, bool, bool, bool, bool, &'a str),
    protocol_statuses: (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str),
}

impl<'a> RepositoryStrongProfileScene<'a> {
    /// Receipt, claim, profile, theory, residual slot, and checker identities.
    pub const fn identity(
        &self,
    ) -> (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ) {
        self.identity
    }

    /// Artifact count and zero-member boundary.
    pub const fn counts(&self) -> (u32, u32) {
        self.counts
    }

    /// Identity-keyed tier, provenance, and route rows for profile artifacts.
    pub fn admissions(&self) -> &[PhysicalRootAdmissionScene<'a>] {
        &self.admissions
    }

    /// Profile-root, sector, carrier-family, and constraint-law identities.
    pub const fn identity_digests(&self) -> [[u8; 32]; 4] {
        self.identity_digests
    }

    /// Paired profile receipt and documentary evidence-custody digest.
    pub const fn receipt_digests(&self) -> ([u8; 32], [u8; 32]) {
        self.receipt_digests
    }

    /// Independent categorical evidence in producer and watchdog pairs.
    pub const fn checker_evidence_sha256(&self) -> StrongProfileCheckerEvidenceDigests {
        self.checker_evidence_sha256
    }

    /// Coverage, membership, support, theorem, boundary, and authority flags.
    pub const fn scope(&self) -> (bool, bool, bool, bool, bool, bool, &'a str) {
        self.scope
    }

    /// Derive-first, Pi, Gap, Chaos, Residual, and residual-slot statuses.
    pub const fn protocol_statuses(
        &self,
    ) -> (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str) {
        self.protocol_statuses
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPhysicalRegistryFrontierScene<'a> {
    registry_schema_id: &'a str,
    proof_graph_schema_id: &'a str,
    root_receipt_schema_id: &'a str,
    root_claim_id: &'a str,
    root_canary_suite_id: &'a str,
    root_decision_id: &'a str,
    root_input_sha256: [u8; 32],
    root_result_sha256: [u8; 32],
    root_producer_result_sha256: [u8; 32],
    root_watchdog_result_sha256: [u8; 32],
    root_producer_resource_sha256: [u8; 32],
    root_watchdog_resource_sha256: [u8; 32],
    root_canary_sha256: [u8; 32],
    root_receipt_sha256: [u8; 32],
    root_producer_id: &'a str,
    root_watchdog_id: &'a str,
    scalar_coordinate_count: u32,
    membership_neutral_mass_projection_count: u32,
    admitted_root_count: u32,
    admitted_artifact_count: u32,
    root_admissions: Vec<PhysicalRootAdmissionScene<'a>>,
    membership_authority: bool,
    primitive_profile: RepositoryPrimitiveProfileScene<'a>,
    charged_profile: RepositoryChargedProfileScene<'a>,
    neutral_bound_profile: RepositoryNeutralBoundProfileScene<'a>,
    strong_profile: RepositoryStrongProfileScene<'a>,
    vocabulary_receipt_schema_id: &'a str,
    vocabulary_claim_id: &'a str,
    vocabulary_producer_id: &'a str,
    vocabulary_watchdog_id: &'a str,
    vocabulary_descriptor_role_identities: Vec<[u8; 32]>,
    vocabulary_relation_target_identities: Vec<[u8; 32]>,
    vocabulary_constraint_law_identities: Vec<[u8; 32]>,
    vocabulary_counts: (u32, u32, u32, u32),
    vocabulary_scope: (bool, bool, bool),
    vocabulary_checker_result_sha256: ([u8; 32], [u8; 32]),
    vocabulary_checker_resource_sha256: ([u8; 32], [u8; 32]),
    vocabulary_receipt_sha256: [u8; 32],
    refusal_code: &'a str,
    registry_member_count: u32,
    registry_coverage_claim: bool,
    registry_authority_effect: &'a str,
    open_obligations: Vec<&'static str>,
}

impl<'a> RepositoryPhysicalRegistryFrontierScene<'a> {
    /// Physical-registry and proof-graph schema identities.
    pub const fn registry_schema_identity(&self) -> (&'a str, &'a str) {
        (self.registry_schema_id, self.proof_graph_schema_id)
    }

    /// Receipt schema and scientific claim identity sealed by the root pair.
    pub const fn receipt_identity(&self) -> (&'a str, &'a str) {
        (self.root_receipt_schema_id, self.root_claim_id)
    }

    /// Mutation-canary suite bound into the root-pair receipt.
    pub const fn canary_suite_id(&self) -> &'a str {
        self.root_canary_suite_id
    }

    /// Exact producer-watchdog agreement decision.
    pub const fn decision_id(&self) -> &'a str {
        self.root_decision_id
    }

    /// Digest of the canonical root-pair input.
    pub const fn input_sha256(&self) -> [u8; 32] {
        self.root_input_sha256
    }

    /// Digest of the canonical agreed root-pair result.
    pub const fn result_sha256(&self) -> [u8; 32] {
        self.root_result_sha256
    }

    /// Independently computed producer and watchdog result digests.
    pub const fn checker_result_sha256(&self) -> ([u8; 32], [u8; 32]) {
        (
            self.root_producer_result_sha256,
            self.root_watchdog_result_sha256,
        )
    }

    /// Independently encoded producer and watchdog resource-contract digests.
    pub const fn checker_resource_sha256(&self) -> ([u8; 32], [u8; 32]) {
        (
            self.root_producer_resource_sha256,
            self.root_watchdog_resource_sha256,
        )
    }

    /// Digest of the mutation-canary manifest bound into the pair receipt.
    pub const fn canary_sha256(&self) -> [u8; 32] {
        self.root_canary_sha256
    }

    /// Digest of the complete planet-owned root-pair receipt.
    pub const fn receipt_sha256(&self) -> [u8; 32] {
        self.root_receipt_sha256
    }

    /// Schema, claim, and independent checker identities for the bounded
    /// physical-vocabulary partition.
    pub const fn vocabulary_identity(&self) -> (&'a str, &'a str, &'a str, &'a str) {
        (
            self.vocabulary_receipt_schema_id,
            self.vocabulary_claim_id,
            self.vocabulary_producer_id,
            self.vocabulary_watchdog_id,
        )
    }

    /// Descriptor roles, relation targets, and constraint laws classified from
    /// the admitted roots. These are read-only receipt payloads.
    pub fn vocabulary_partitions(&self) -> PhysicalVocabularyPartitions<'_> {
        (
            &self.vocabulary_descriptor_role_identities,
            &self.vocabulary_relation_target_identities,
            &self.vocabulary_constraint_law_identities,
        )
    }

    /// Root, descriptor-role, relation-target, and constraint-law counts.
    pub const fn vocabulary_counts(&self) -> (u32, u32, u32, u32) {
        self.vocabulary_counts
    }

    /// Current-input completeness, global coverage, and membership authority.
    pub const fn vocabulary_scope(&self) -> (bool, bool, bool) {
        self.vocabulary_scope
    }

    /// Independent vocabulary checker result and resource-contract digests.
    pub const fn vocabulary_checker_digests(&self) -> PhysicalVocabularyCheckerDigests {
        (
            self.vocabulary_checker_result_sha256,
            self.vocabulary_checker_resource_sha256,
        )
    }

    /// Digest of the exact bounded vocabulary agreement receipt.
    pub const fn vocabulary_receipt_sha256(&self) -> [u8; 32] {
        self.vocabulary_receipt_sha256
    }

    /// Independent producer and watchdog implementation identities.
    pub const fn checker_ids(&self) -> (&'a str, &'a str) {
        (self.root_producer_id, self.root_watchdog_id)
    }

    /// Number of admitted scalar-coordinate roots.
    pub const fn scalar_coordinate_count(&self) -> u32 {
        self.scalar_coordinate_count
    }

    /// Number of admitted membership-neutral mass projections.
    pub const fn membership_neutral_mass_projection_count(&self) -> u32 {
        self.membership_neutral_mass_projection_count
    }

    /// Total admitted roots covered by the receipt.
    pub const fn admitted_root_count(&self) -> u32 {
        self.admitted_root_count
    }

    /// Total admitted artifacts across floor roots and the local profile.
    pub const fn admitted_artifact_count(&self) -> u32 {
        self.admitted_artifact_count
    }

    /// The one locally closed primitive profile and its evidence.
    pub const fn primitive_profile(&self) -> &RepositoryPrimitiveProfileScene<'a> {
        &self.primitive_profile
    }

    /// The locally closed two-member charge-conjugation orbit and its evidence.
    pub const fn charged_profile(&self) -> &RepositoryChargedProfileScene<'a> {
        &self.charged_profile
    }

    /// The locally closed neutral two-body profile and its bounded evidence.
    pub const fn neutral_bound_profile(&self) -> &RepositoryNeutralBoundProfileScene<'a> {
        &self.neutral_bound_profile
    }

    /// The claim-local confining profile and its non-membership boundary.
    pub const fn strong_profile(&self) -> &RepositoryStrongProfileScene<'a> {
        &self.strong_profile
    }

    /// Exact identity-keyed tier, provenance, and route census for admitted roots.
    pub fn root_admissions(&self) -> &[PhysicalRootAdmissionScene<'a>] {
        &self.root_admissions
    }

    /// Whether the projected roots carry physical species membership authority.
    pub const fn membership_authority(&self) -> bool {
        self.membership_authority
    }

    /// Exact physical-registry refusal after the admitted roots are inspected.
    pub const fn refusal_code(&self) -> &'a str {
        self.refusal_code
    }

    /// Number of physical species members proven after root inspection.
    pub const fn registry_member_count(&self) -> u32 {
        self.registry_member_count
    }

    /// Whether the physical registry claims complete species coverage.
    pub const fn registry_coverage_claim(&self) -> bool {
        self.registry_coverage_claim
    }

    /// Authority effect of the physical-registry result.
    pub const fn registry_authority_effect(&self) -> &'a str {
        self.registry_authority_effect
    }

    /// Ordered obligations that remain open beyond the local registry closure.
    pub fn open_obligations(&self) -> &[&'static str] {
        &self.open_obligations
    }
}

/// Case count and complete transcript digest for one law-premise canary suite.
pub type LawPremiseCanaryEvidence = (u32, [u8; 32]);

/// Producer and watchdog canary evidence in that order.
pub type LawPremiseCanaryEvidencePair = [LawPremiseCanaryEvidence; 2];

/// Read-only projection of one claim-scoped derived-coordinate premise.
///
/// This is diagnostic evidence adjacent to the registry. It cannot become a
/// registry input, admit a species, or alter canonical execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryLawPremiseFrontierScene<'a> {
    identity: (&'a str, &'a str, &'a str, &'a str),
    output_symbol: &'a str,
    output_bits: i128,
    output_scale_bits: u32,
    output_projection_receipt_sha256: [u8; 32],
    canonical_relation_sha256: [u8; 32],
    checker_identities: (&'a str, &'a str, &'a str),
    checker_result_sha256: ([u8; 32], [u8; 32]),
    canary_transcript_ids: (&'a str, &'a str),
    canary_evidence: LawPremiseCanaryEvidencePair,
    claim_identity_sha256: [u8; 32],
    role_identity_sha256: [u8; 32],
    content_identity_sha256: [u8; 32],
    upstream_capability_sha256: [u8; 32],
    applicability_receipt_sha256: [u8; 32],
    validity_receipt_sha256: [u8; 32],
    ancestry_receipt_sha256: [u8; 32],
    pair_receipt_sha256: [u8; 32],
    capability_sha256: [u8; 32],
    ledger_classification: (&'a str, &'a str, &'a str),
    scope: (bool, bool, bool, &'a str),
}

impl<'a> RepositoryLawPremiseFrontierScene<'a> {
    /// Premise, artifact, pair-receipt, and canary-suite identities.
    pub const fn identity(&self) -> (&'a str, &'a str, &'a str, &'a str) {
        self.identity
    }

    /// Exact scaled execution coordinate emitted by the sealed relation.
    pub const fn output_coordinate(&self) -> (&'a str, i128, u32) {
        (self.output_symbol, self.output_bits, self.output_scale_bits)
    }

    /// Output projection receipt and canonical relation digest.
    pub const fn derivation_digests(&self) -> ([u8; 32], [u8; 32]) {
        (
            self.output_projection_receipt_sha256,
            self.canonical_relation_sha256,
        )
    }

    /// Producer, watchdog, and explicit agreement-decision identities.
    pub const fn checker_identities(&self) -> (&'a str, &'a str, &'a str) {
        self.checker_identities
    }

    /// Producer and watchdog complete-result digests.
    pub const fn checker_result_digests(&self) -> ([u8; 32], [u8; 32]) {
        self.checker_result_sha256
    }

    /// Independent producer and watchdog canary transcript identities.
    pub const fn canary_transcript_ids(&self) -> (&'a str, &'a str) {
        self.canary_transcript_ids
    }

    /// Producer and watchdog live-canary case counts and transcript digests.
    pub const fn canary_evidence(&self) -> LawPremiseCanaryEvidencePair {
        self.canary_evidence
    }

    /// Claim, role, content, upstream, applicability, validity, and ancestry digests.
    pub const fn claim_scope_digests(&self) -> [[u8; 32]; 7] {
        [
            self.claim_identity_sha256,
            self.role_identity_sha256,
            self.content_identity_sha256,
            self.upstream_capability_sha256,
            self.applicability_receipt_sha256,
            self.validity_receipt_sha256,
            self.ancestry_receipt_sha256,
        ]
    }

    /// Pair receipt and private capability digests.
    pub const fn authority_digests(&self) -> ([u8; 32], [u8; 32]) {
        (self.pair_receipt_sha256, self.capability_sha256)
    }

    /// Universal tier, derived provenance tag, and derived proof kind.
    pub const fn ledger_classification(&self) -> (&'a str, &'a str, &'a str) {
        self.ledger_classification
    }

    /// Exact-claim coverage, species authority, global coverage, and effect.
    pub const fn scope(&self) -> (bool, bool, bool, &'a str) {
        self.scope
    }
}

/// Read-only derive-first route over admitted law-premise capabilities.
///
/// The scene reports one replayed derived route and the exact next blocker. It
/// cannot supply a target, rule, protocol receipt, or authority capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPremiseAdmissionFrontierScene<'a> {
    identity: (&'a str, &'a str, &'a str),
    checker_result_sha256: ([u8; 32], [u8; 32]),
    derived_identity_sha256: ([u8; 32], [u8; 32], [u8; 32]),
    counts: (u32, u32, u32, u32),
    decisions: (&'a str, &'a str),
    scope: (bool, bool, bool, bool, &'a str),
}

impl<'a> RepositoryPremiseAdmissionFrontierScene<'a> {
    /// Route schema plus independent producer and watchdog identities.
    pub const fn identity(&self) -> (&'a str, &'a str, &'a str) {
        self.identity
    }

    /// Complete producer and watchdog result digests.
    pub const fn checker_result_digests(&self) -> ([u8; 32], [u8; 32]) {
        self.checker_result_sha256
    }

    /// Claim, role, and content identities of the replayed derived route.
    pub const fn derived_identity_digests(&self) -> ([u8; 32], [u8; 32], [u8; 32]) {
        self.derived_identity_sha256
    }

    /// Derived routes, descriptor roles, relation targets, and constraint laws.
    pub const fn counts(&self) -> (u32, u32, u32, u32) {
        self.counts
    }

    /// Replayed derived decision and the exact next-target blocker.
    pub const fn decisions(&self) -> (&'a str, &'a str) {
        self.decisions
    }

    /// Derivation completeness, protocol start, premise authority, species
    /// authority, and authority effect.
    pub const fn scope(&self) -> (bool, bool, bool, bool, &'a str) {
        self.scope
    }
}

/// Exact partial species derivation analysis attached to Stage 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesDerivationScene<'a> {
    computed: bool,
    floor_binding_schema_id: Option<&'a str>,
    floor_binding_sha256: Option<&'a str>,
    structure_schema_id: Option<&'a str>,
    species_registry_schema_id: Option<&'a str>,
    stellar_state_schema_id: Option<&'a str>,
    reducer_law_id: Option<&'a str>,
    floor_anchor_id: Option<&'a str>,
    floor_anchor_symbol: Option<&'a str>,
    floor_anchor_bits: Option<i128>,
    floor_anchor_scale_bits: Option<u32>,
    floor_anchor_membership_authority: Option<bool>,
    floor_anchor_role: Option<&'a str>,
    law_premise_frontier: Option<RepositoryLawPremiseFrontierScene<'a>>,
    premise_admission_frontier: Option<RepositoryPremiseAdmissionFrontierScene<'a>>,
    physical_registry_frontier: Option<RepositoryPhysicalRegistryFrontierScene<'a>>,
    frontier_source_id: Option<&'a str>,
    frontier_scope_id: Option<&'a str>,
    frontier_completeness_claim: Option<bool>,
    candidate_member_count: Option<usize>,
    verified_support_member_count: Option<usize>,
    species_support_value_payload_present: Option<bool>,
    residual_slot_claim: Option<bool>,
    derive_first_status_id: Option<&'static str>,
    buckingham_pi_status_id: Option<&'static str>,
    gap_law_status_id: Option<&'static str>,
    chaos_protocol_status_id: Option<&'static str>,
    residual_law_status_id: Option<&'static str>,
    unique_residual_slot_status_id: Option<&'static str>,
    open_proof_ids: Vec<&'a str>,
    attempts: Vec<SpeciesAttemptScene<'a>>,
    error_code: Option<&'a str>,
    error_detail: Option<&'a str>,
}

impl<'a> SpeciesDerivationScene<'a> {
    /// Whether the analysis artifact passed its semantic checker.
    pub const fn is_computed(&self) -> bool {
        self.computed
    }

    /// Bound physical-floor authority schema and digest.
    pub const fn floor_binding(&self) -> (Option<&'a str>, Option<&'a str>) {
        (self.floor_binding_schema_id, self.floor_binding_sha256)
    }

    /// Bound structure, species-registry, and stellar-state schema identities.
    pub const fn schema_bindings(&self) -> (Option<&'a str>, Option<&'a str>, Option<&'a str>) {
        (
            self.structure_schema_id,
            self.species_registry_schema_id,
            self.stellar_state_schema_id,
        )
    }

    /// Exact reducer law identity, without granting production authority.
    pub const fn reducer_law_id(&self) -> Option<&'a str> {
        self.reducer_law_id
    }

    /// Exact floor mass-coordinate anchor fields.
    pub const fn floor_anchor(
        &self,
    ) -> (Option<&'a str>, Option<&'a str>, Option<i128>, Option<u32>) {
        (
            self.floor_anchor_id,
            self.floor_anchor_symbol,
            self.floor_anchor_bits,
            self.floor_anchor_scale_bits,
        )
    }

    /// Whether the floor anchor grants membership authority, plus its role.
    pub const fn floor_anchor_authority(&self) -> (Option<bool>, Option<&'a str>) {
        (
            self.floor_anchor_membership_authority,
            self.floor_anchor_role,
        )
    }

    /// Repository physical-registry receipt and exact remaining frontier.
    pub const fn physical_registry_frontier(
        &self,
    ) -> Option<&RepositoryPhysicalRegistryFrontierScene<'a>> {
        self.physical_registry_frontier.as_ref()
    }

    /// Sealed claim-scoped coordinate premise adjacent to the registry.
    pub const fn law_premise_frontier(&self) -> Option<&RepositoryLawPremiseFrontierScene<'a>> {
        self.law_premise_frontier.as_ref()
    }

    /// Paired derive-first route and exact next-target blocker.
    pub const fn premise_admission_frontier(
        &self,
    ) -> Option<&RepositoryPremiseAdmissionFrontierScene<'a>> {
        self.premise_admission_frontier.as_ref()
    }

    /// Live source, bounded scope, and explicit absence of a completeness claim.
    pub const fn frontier_contract(&self) -> (Option<&'a str>, Option<&'a str>, Option<bool>) {
        (
            self.frontier_source_id,
            self.frontier_scope_id,
            self.frontier_completeness_claim,
        )
    }

    /// Candidate and verified positive-support member counts.
    pub const fn support_counts(&self) -> (Option<usize>, Option<usize>) {
        (
            self.candidate_member_count,
            self.verified_support_member_count,
        )
    }

    /// Whether a species-support value or residual-slot claim is present.
    pub const fn admission_claims(&self) -> (Option<bool>, Option<bool>) {
        (
            self.species_support_value_payload_present,
            self.residual_slot_claim,
        )
    }

    /// Ordered derive-first, Buckingham Pi, Gap, Chaos, Residual, and slot status.
    pub const fn law_status_ids(&self) -> [Option<&'static str>; 6] {
        [
            self.derive_first_status_id,
            self.buckingham_pi_status_id,
            self.gap_law_status_id,
            self.chaos_protocol_status_id,
            self.residual_law_status_id,
            self.unique_residual_slot_status_id,
        ]
    }

    /// Proofs that remain open in canonical order.
    pub fn open_proof_ids(&self) -> &[&'a str] {
        &self.open_proof_ids
    }

    /// Blocked derivation attempts in canonical order.
    pub fn attempts(&self) -> &[SpeciesAttemptScene<'a>] {
        &self.attempts
    }

    /// Typed checker error when the analysis is invalid.
    pub const fn error_code(&self) -> Option<&'a str> {
        self.error_code
    }

    /// Checker detail when the analysis is invalid.
    pub const fn error_detail(&self) -> Option<&'a str> {
        self.error_detail
    }
}

/// One non-admitting analysis attached to an open proof leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisScene<'a> {
    kind_id: &'static str,
    schema_id: &'static str,
    checker_id: &'static str,
    status_id: &'static str,
    closure_effect_id: &'static str,
    coverage_claim: bool,
    dimensional_census: Option<DimensionalCensusScene<'a>>,
    species_derivation: Option<SpeciesDerivationScene<'a>>,
}

impl<'a> AnalysisScene<'a> {
    /// Stable analysis kind.
    pub const fn kind_id(&self) -> &'static str {
        self.kind_id
    }

    /// Analysis schema and checker identities.
    pub const fn schema_and_checker(&self) -> (&'static str, &'static str) {
        (self.schema_id, self.checker_id)
    }

    /// Analysis status and declared closure effect.
    pub const fn status_and_closure(&self) -> (&'static str, &'static str) {
        (self.status_id, self.closure_effect_id)
    }

    /// Whether this non-admitting analysis claims complete coverage.
    pub const fn coverage_claim(&self) -> bool {
        self.coverage_claim
    }

    /// Dimensional census payload when this is that analysis kind.
    pub const fn dimensional_census(&self) -> Option<&DimensionalCensusScene<'a>> {
        self.dimensional_census.as_ref()
    }

    /// Species derivation payload when this is that analysis kind.
    pub const fn species_derivation(&self) -> Option<&SpeciesDerivationScene<'a>> {
        self.species_derivation.as_ref()
    }
}

/// One exact open proof leaf and its ordered obligations and analyses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRequirementScene<'a> {
    requirement_id: &'a str,
    obligations: Vec<&'a str>,
    analyses: Vec<AnalysisScene<'a>>,
}

impl<'a> OpenRequirementScene<'a> {
    /// Stable proof-leaf identity.
    pub const fn requirement_id(&self) -> &'a str {
        self.requirement_id
    }

    /// Ordered closure obligations.
    pub fn obligations(&self) -> &[&'a str] {
        &self.obligations
    }

    /// Ordered non-admitting analyses attached to this leaf.
    pub fn analyses(&self) -> &[AnalysisScene<'a>] {
        &self.analyses
    }
}

/// One structured refusal and its exact open frontier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalReasonScene<'a> {
    index: usize,
    code_id: &'static str,
    stage_id: Option<&'static str>,
    requirement_id: Option<&'a str>,
    detail: &'a str,
    open_requirements: Vec<OpenRequirementScene<'a>>,
}

impl<'a> RefusalReasonScene<'a> {
    /// Position in the receipt's canonical refusal order.
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Stable typed refusal code.
    pub const fn code_id(&self) -> &'static str {
        self.code_id
    }

    /// Stage that refused, when physical execution had entered one.
    pub const fn stage_id(&self) -> Option<&'static str> {
        self.stage_id
    }

    /// Root missing requirement.
    pub const fn requirement_id(&self) -> Option<&'a str> {
        self.requirement_id
    }

    /// Human-readable receipt detail.
    pub const fn detail(&self) -> &'a str {
        self.detail
    }

    /// Exact ordered proof frontier beneath this refusal.
    pub fn open_requirements(&self) -> &[OpenRequirementScene<'a>] {
        &self.open_requirements
    }
}

/// Immutable observer projection of a canonical refusal receipt.
#[derive(Debug, Clone, Copy)]
pub struct RefusalScene<'a> {
    receipt: &'a civsim_planet::RunReceipt,
}

impl<'a> RefusalScene<'a> {
    pub(crate) const fn new(view: RefusalView<'a>) -> Self {
        Self {
            receipt: view.receipt(),
        }
    }

    /// Number of admitted floor entries carried by the run boundary.
    pub const fn absolute_floor_entries(self) -> usize {
        self.receipt.absolute_floor_entries()
    }

    /// Append-only transcript projection paired with this refusal.
    pub const fn transcript(self) -> TranscriptScene<'a> {
        TranscriptScene::new(self.receipt)
    }

    /// All stages in the current repository route order.
    pub fn stages(self) -> impl ExactSizeIterator<Item = StageScene> + 'a {
        self.receipt.stages().iter().map(|stage| StageScene {
            id: stage.stage().id(),
            status_id: stage.status().id(),
            entered_event_ordinal: stage.entered_event().map(|event| event.ordinal()),
            terminal_event_ordinal: stage.terminal_event().map(|event| event.ordinal()),
        })
    }

    /// Structured refusals in their canonical receipt order.
    pub fn refusals(self) -> impl ExactSizeIterator<Item = RefusalReasonScene<'a>> + 'a {
        self.receipt
            .refusals()
            .iter()
            .enumerate()
            .map(|(index, refusal)| {
                let open_requirements = refusal
                    .open_requirements()
                    .iter()
                    .map(|requirement| {
                        let analyses = requirement
                            .analyses()
                            .iter()
                            .map(|analysis| {
                                let dimensional_census = analysis
                                    .exact_dimensional_census_view()
                                    .map(|view| {
                                        let phenomena = view
                                            .phenomena()
                                            .map(|phenomenon| {
                                                let attempts = phenomenon
                                                    .derivation_attempts()
                                                    .map(|attempt| {
                                                        DimensionalAttemptScene::new(
                                                            attempt.id(),
                                                            attempt.law_id(),
                                                            attempt.output_id(),
                                                            attempt.status_id(),
                                                            attempt.input_ids(),
                                                            attempt
                                                                .dimension_only_projection(),
                                                            attempt
                                                                .dimension_only_support_ids(),
                                                            attempt.missing_dependency_ids(),
                                                            attempt.dropped_mechanism_ids(),
                                                        )
                                                    })
                                                    .collect();
                                                DimensionalPhenomenonScene::new(
                                                    phenomenon.id(),
                                                    phenomenon.coverage_complete(),
                                                    attempts,
                                                )
                                            })
                                            .collect();
                                        DimensionalCensusScene::new(
                                            view.is_computed(),
                                            view.representation_schema_id(),
                                            view.floor_binding_schema_id(),
                                            view.floor_binding_sha256(),
                                            view.base_dimension_ids().to_vec(),
                                            view.structure_schema_id(),
                                            view.variables().len(),
                                            phenomena,
                                            view.coverage_gap_ids(),
                                            view.error_code(),
                                            view.error_detail(),
                                        )
                                    });
                                let species_derivation = analysis
                                    .species_derivation_analysis_view()
                                    .map(|view| {
                                        let law_premise_frontier = (|| {
                                            Some(RepositoryLawPremiseFrontierScene {
                                                identity: view.law_premise_identity()?,
                                                output_symbol: view
                                                    .law_premise_output_symbol()?,
                                                output_bits: view.law_premise_output_bits()?,
                                                output_scale_bits: view
                                                    .law_premise_output_scale_bits()?,
                                                output_projection_receipt_sha256: view
                                                    .law_premise_output_projection_receipt_sha256(
                                                    )?,
                                                canonical_relation_sha256: view
                                                    .law_premise_relation_sha256()?,
                                                checker_identities: view
                                                    .law_premise_checker_identities()?,
                                                checker_result_sha256: view
                                                    .law_premise_checker_result_sha256()?,
                                                canary_transcript_ids: view
                                                    .law_premise_canary_transcript_ids()?,
                                                canary_evidence: view
                                                    .law_premise_canary_evidence()?,
                                                claim_identity_sha256: view
                                                    .law_premise_claim_identity_sha256()?,
                                                role_identity_sha256: view
                                                    .law_premise_role_identity_sha256()?,
                                                content_identity_sha256: view
                                                    .law_premise_content_identity_sha256()?,
                                                upstream_capability_sha256: view
                                                    .law_premise_upstream_capability_sha256()?,
                                                applicability_receipt_sha256: view
                                                    .law_premise_applicability_receipt_sha256()?,
                                                validity_receipt_sha256: view
                                                    .law_premise_validity_receipt_sha256()?,
                                                ancestry_receipt_sha256: view
                                                    .law_premise_ancestry_receipt_sha256()?,
                                                pair_receipt_sha256: view
                                                    .law_premise_pair_receipt_sha256()?,
                                                capability_sha256: view
                                                    .law_premise_capability_sha256()?,
                                                ledger_classification: view
                                                    .law_premise_ledger_classification()?,
                                                scope: view.law_premise_scope()?,
                                            })
                                        })();
                                        let premise_admission_frontier = (|| {
                                            Some(RepositoryPremiseAdmissionFrontierScene {
                                                identity: view
                                                    .premise_admission_route_identity()?,
                                                checker_result_sha256: view
                                                    .premise_admission_route_result_sha256()?,
                                                derived_identity_sha256: view
                                                    .premise_admission_derived_identity()?,
                                                counts: view
                                                    .premise_admission_route_counts()?,
                                                decisions: view
                                                    .premise_admission_route_decisions()?,
                                                scope: view
                                                    .premise_admission_route_scope()?,
                                            })
                                        })();
                                        let physical_registry_frontier = (|| {
                                            let primitive_semantics =
                                                view.primitive_profile_semantics()?;
                                            Some(RepositoryPhysicalRegistryFrontierScene {
                                                registry_schema_id: view
                                                    .physical_registry_schema_id()?,
                                                proof_graph_schema_id: view
                                                    .physical_registry_proof_graph_schema_id()?,
                                                root_receipt_schema_id: view
                                                    .physical_registry_root_receipt_schema_id()?,
                                                root_claim_id: view
                                                    .physical_registry_root_claim_id()?,
                                                root_canary_suite_id: view
                                                    .physical_registry_root_canary_suite_id()?,
                                                root_decision_id: view
                                                    .physical_registry_root_decision_id()?,
                                                root_input_sha256: view
                                                    .physical_registry_root_input_sha256()?,
                                                root_result_sha256: view
                                                    .physical_registry_root_result_sha256()?,
                                                root_producer_result_sha256: view
                                                    .physical_registry_root_producer_result_sha256()?,
                                                root_watchdog_result_sha256: view
                                                    .physical_registry_root_watchdog_result_sha256()?,
                                                root_producer_resource_sha256: view
                                                    .physical_registry_root_producer_resource_sha256(
                                                    )?,
                                                root_watchdog_resource_sha256: view
                                                    .physical_registry_root_watchdog_resource_sha256(
                                                    )?,
                                                root_canary_sha256: view
                                                    .physical_registry_root_canary_sha256()?,
                                                root_receipt_sha256: view
                                                    .physical_registry_root_receipt_sha256()?,
                                                root_producer_id: view
                                                    .physical_registry_root_producer_id()?,
                                                root_watchdog_id: view
                                                    .physical_registry_root_watchdog_id()?,
                                                scalar_coordinate_count: view
                                                    .physical_registry_scalar_coordinate_count()?,
                                                membership_neutral_mass_projection_count: view
                                                    .physical_registry_membership_neutral_mass_projection_count(
                                                    )?,
                                                admitted_root_count: view
                                                    .physical_registry_admitted_root_count()?,
                                                admitted_artifact_count: view
                                                    .physical_registry_admitted_artifact_count()?,
                                                root_admissions: view
                                                    .physical_registry_root_admissions()
                                                    .map(|admission| {
                                                        PhysicalRootAdmissionScene {
                                                            identity_sha256: admission
                                                                .identity_sha256(),
                                                            tier_id: admission.tier_id(),
                                                            provenance_tag: admission
                                                                .provenance_tag(),
                                                            route_id: admission.route_id(),
                                                        }
                                                    })
                                                    .collect(),
                                                membership_authority: view
                                                    .physical_registry_membership_authority()?,
                                                primitive_profile:
                                                    RepositoryPrimitiveProfileScene {
                                                        identity: view
                                                            .primitive_profile_identity()?,
                                                        dynamical_identity: primitive_semantics
                                                            .dynamical_identity(),
                                                        member_properties: primitive_semantics
                                                            .member_properties(),
                                                        counts: view.primitive_profile_counts()?,
                                                        admissions: view
                                                            .primitive_profile_admissions()
                                                            .map(|admission| {
                                                                PhysicalRootAdmissionScene {
                                                                    identity_sha256: admission
                                                                        .identity_sha256(),
                                                                    tier_id: admission.tier_id(),
                                                                    provenance_tag: admission
                                                                        .provenance_tag(),
                                                                    route_id: admission.route_id(),
                                                                }
                                                            })
                                                            .collect(),
                                                        member_sha256: view
                                                            .primitive_profile_member_sha256()?,
                                                        receipt_sha256: view
                                                            .primitive_profile_receipt_sha256()?,
                                                        evidence_sha256: view
                                                            .primitive_profile_evidence_sha256()?,
                                                        repository_catalog_sha256: view
                                                            .primitive_profile_repository_catalog_sha256(
                                                            )?,
                                                        protocol_checker_result_sha256: view
                                                            .primitive_profile_protocol_checker_result_sha256(
                                                            )?,
                                                        protocol_statuses: view
                                                            .primitive_profile_protocol_statuses()?,
                                                    },
                                                charged_profile:
                                                    RepositoryChargedProfileScene {
                                                        identity: view
                                                            .charged_profile_identity()?,
                                                        counts: view.charged_profile_counts()?,
                                                        admissions: view
                                                            .charged_profile_admissions()
                                                            .map(|admission| {
                                                                PhysicalRootAdmissionScene {
                                                                    identity_sha256: admission
                                                                        .identity_sha256(),
                                                                    tier_id: admission.tier_id(),
                                                                    provenance_tag: admission
                                                                        .provenance_tag(),
                                                                    route_id: admission.route_id(),
                                                                }
                                                            })
                                                            .collect(),
                                                        member_sha256: view
                                                            .charged_profile_member_sha256()?
                                                            .to_vec(),
                                                        receipt_sha256: view
                                                            .charged_profile_receipt_sha256()?,
                                                        checker_evidence_sha256: view
                                                            .charged_profile_checker_evidence_sha256(
                                                            )?,
                                                        protocol_statuses: view
                                                            .charged_profile_protocol_statuses()?,
                                                    },
                                                neutral_bound_profile:
                                                    RepositoryNeutralBoundProfileScene {
                                                        identity: view
                                                            .neutral_bound_profile_identity()?,
                                                        counts: view
                                                            .neutral_bound_profile_counts()?,
                                                        admissions: view
                                                            .neutral_bound_profile_admissions()
                                                            .map(|admission| {
                                                                PhysicalRootAdmissionScene {
                                                                    identity_sha256: admission
                                                                        .identity_sha256(),
                                                                    tier_id: admission.tier_id(),
                                                                    provenance_tag: admission
                                                                        .provenance_tag(),
                                                                    route_id: admission.route_id(),
                                                                }
                                                            })
                                                            .collect(),
                                                        member_sha256: view
                                                            .neutral_bound_profile_member_sha256()?,
                                                        receipt_sha256: view
                                                            .neutral_bound_profile_receipt_sha256()?,
                                                        checker_evidence_sha256: view
                                                            .neutral_bound_profile_checker_evidence_sha256(
                                                            )?,
                                                        channel_digests: view
                                                            .neutral_bound_profile_channel_digests()?,
                                                        dispositions: view
                                                            .neutral_bound_profile_dispositions()?,
                                                        scope: view
                                                            .neutral_bound_profile_scope()?,
                                                        protocol_statuses: view
                                                            .neutral_bound_profile_protocol_statuses(
                                                            )?,
                                                    },
                                                strong_profile:
                                                    RepositoryStrongProfileScene {
                                                        identity: view
                                                            .strong_profile_identity()?,
                                                        counts: view.strong_profile_counts()?,
                                                        admissions: view
                                                            .strong_profile_admissions()
                                                            .map(|admission| {
                                                                PhysicalRootAdmissionScene {
                                                                    identity_sha256: admission
                                                                        .identity_sha256(),
                                                                    tier_id: admission.tier_id(),
                                                                    provenance_tag: admission
                                                                        .provenance_tag(),
                                                                    route_id: admission.route_id(),
                                                                }
                                                            })
                                                            .collect(),
                                                        identity_digests: view
                                                            .strong_profile_identity_digests()?,
                                                        receipt_digests: view
                                                            .strong_profile_receipt_digests()?,
                                                        checker_evidence_sha256: view
                                                            .strong_profile_checker_evidence_sha256(
                                                            )?,
                                                        scope: view.strong_profile_scope()?,
                                                        protocol_statuses: view
                                                            .strong_profile_protocol_statuses()?,
                                                    },
                                                vocabulary_receipt_schema_id: view
                                                    .physical_vocabulary_receipt_schema_id()?,
                                                vocabulary_claim_id: view
                                                    .physical_vocabulary_claim_id()?,
                                                vocabulary_producer_id: view
                                                    .physical_vocabulary_producer_id()?,
                                                vocabulary_watchdog_id: view
                                                    .physical_vocabulary_watchdog_id()?,
                                                vocabulary_descriptor_role_identities: view
                                                    .physical_vocabulary_descriptor_role_identities(
                                                    )?
                                                    .to_vec(),
                                                vocabulary_relation_target_identities: view
                                                    .physical_vocabulary_relation_target_identities(
                                                    )?
                                                    .to_vec(),
                                                vocabulary_constraint_law_identities: view
                                                    .physical_vocabulary_constraint_law_identities(
                                                    )?
                                                    .to_vec(),
                                                vocabulary_counts: view
                                                    .physical_vocabulary_counts()?,
                                                vocabulary_scope: view
                                                    .physical_vocabulary_scope()?,
                                                vocabulary_checker_result_sha256: view
                                                    .physical_vocabulary_checker_result_sha256()?,
                                                vocabulary_checker_resource_sha256: view
                                                    .physical_vocabulary_checker_resource_sha256()?,
                                                vocabulary_receipt_sha256: view
                                                    .physical_vocabulary_receipt_sha256()?,
                                                refusal_code: view
                                                    .physical_registry_refusal_code()?,
                                                registry_member_count: view
                                                    .physical_registry_member_count()?,
                                                registry_coverage_claim: view
                                                    .physical_registry_coverage_claim()?,
                                                registry_authority_effect: view
                                                    .physical_registry_authority_effect()?,
                                                open_obligations: view
                                                    .physical_registry_open_obligations()
                                                    .to_vec(),
                                            })
                                        })();
                                        SpeciesDerivationScene {
                                        computed: view.is_computed(),
                                        floor_binding_schema_id: view.floor_binding_schema_id(),
                                        floor_binding_sha256: view.floor_binding_sha256(),
                                        structure_schema_id: view.structure_schema_id(),
                                        species_registry_schema_id: view
                                            .species_registry_schema_id(),
                                        stellar_state_schema_id: view.stellar_state_schema_id(),
                                        reducer_law_id: view.reducer_law_id(),
                                        floor_anchor_id: view.floor_anchor_id(),
                                        floor_anchor_symbol: view.floor_anchor_symbol(),
                                        floor_anchor_bits: view.floor_anchor_bits(),
                                        floor_anchor_scale_bits: view.floor_anchor_scale_bits(),
                                        floor_anchor_membership_authority: view
                                            .floor_anchor_membership_authority(),
                                        floor_anchor_role: view.floor_anchor_role(),
                                        law_premise_frontier,
                                        premise_admission_frontier,
                                        physical_registry_frontier,
                                        frontier_source_id: view.frontier_source_id(),
                                        frontier_scope_id: view.frontier_scope_id(),
                                        frontier_completeness_claim: view
                                            .frontier_completeness_claim(),
                                        candidate_member_count: view.candidate_member_count(),
                                        verified_support_member_count: view
                                            .verified_support_member_count(),
                                        species_support_value_payload_present: view
                                            .species_support_value_payload_present(),
                                        residual_slot_claim: view.residual_slot_claim(),
                                        derive_first_status_id: view.derive_first_status_id(),
                                        buckingham_pi_status_id: view.buckingham_pi_status_id(),
                                        gap_law_status_id: view.gap_law_status_id(),
                                        chaos_protocol_status_id: view.chaos_protocol_status_id(),
                                        residual_law_status_id: view.residual_law_status_id(),
                                        unique_residual_slot_status_id: view
                                            .unique_residual_slot_status_id(),
                                        open_proof_ids: borrowed_ids(view.open_proof_ids()),
                                        attempts: view
                                            .attempts()
                                            .map(|attempt| SpeciesAttemptScene {
                                                id: attempt.id(),
                                                status_id: attempt.status_id(),
                                                input_ids: borrowed_ids(attempt.input_ids()),
                                                open_proof_ids: borrowed_ids(
                                                    attempt.open_proof_ids(),
                                                ),
                                            })
                                            .collect(),
                                        error_code: view.error_code(),
                                        error_detail: view.error_detail(),
                                    }
                                    });
                                AnalysisScene {
                                    kind_id: analysis.kind_id(),
                                    schema_id: analysis.schema_id(),
                                    checker_id: analysis.checker_id(),
                                    status_id: analysis.status_id(),
                                    closure_effect_id: analysis.closure_effect_id(),
                                    coverage_claim: analysis.coverage_claim(),
                                    dimensional_census,
                                    species_derivation,
                                }
                            })
                            .collect();
                        OpenRequirementScene {
                            requirement_id: requirement.requirement_id(),
                            obligations: borrowed_ids(requirement.obligations()),
                            analyses,
                        }
                    })
                    .collect();
                RefusalReasonScene {
                    index,
                    code_id: refusal.code().id(),
                    stage_id: refusal.stage().map(|stage| stage.id()),
                    requirement_id: refusal.requirement_id(),
                    detail: refusal.detail(),
                    open_requirements,
                }
            })
    }
}

fn borrowed_ids(values: &[String]) -> Vec<&str> {
    values.iter().map(String::as_str).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrowed_frontier_fields_preserve_exact_source_order() {
        let values = vec![
            "unfamiliar.proof.z".to_owned(),
            "unfamiliar.proof.a".to_owned(),
            "unfamiliar.proof.z".to_owned(),
        ];

        let first = borrowed_ids(&values);
        let second = borrowed_ids(&values);
        assert_eq!(first, second);
        assert_eq!(
            first,
            [
                "unfamiliar.proof.z",
                "unfamiliar.proof.a",
                "unfamiliar.proof.z"
            ]
        );
    }

    #[test]
    fn repository_law_premise_frontier_is_read_only_and_non_authorizing() {
        let frontier = RepositoryLawPremiseFrontierScene {
            identity: (
                "planet.derived-law-premise-eps0",
                "artifact",
                "receipt",
                "canaries",
            ),
            output_symbol: "eps_0",
            output_bits: 11,
            output_scale_bits: 32,
            output_projection_receipt_sha256: [1; 32],
            canonical_relation_sha256: [2; 32],
            checker_identities: ("producer", "watchdog", "agreed"),
            checker_result_sha256: ([3; 32], [4; 32]),
            canary_transcript_ids: ("producer-canaries", "watchdog-canaries"),
            canary_evidence: [(12, [5; 32]), (12, [15; 32])],
            claim_identity_sha256: [6; 32],
            role_identity_sha256: [7; 32],
            content_identity_sha256: [8; 32],
            upstream_capability_sha256: [9; 32],
            applicability_receipt_sha256: [10; 32],
            validity_receipt_sha256: [11; 32],
            ancestry_receipt_sha256: [12; 32],
            pair_receipt_sha256: [13; 32],
            capability_sha256: [14; 32],
            ledger_classification: ("universal", "[D]", "verified_derived_content"),
            scope: (true, false, false, "none"),
        };

        assert_eq!(frontier.output_coordinate(), ("eps_0", 11, 32));
        assert_eq!(frontier.derivation_digests(), ([1; 32], [2; 32]));
        assert_eq!(
            frontier.checker_identities(),
            ("producer", "watchdog", "agreed")
        );
        assert_eq!(frontier.checker_result_digests(), ([3; 32], [4; 32]));
        assert_eq!(
            frontier.canary_transcript_ids(),
            ("producer-canaries", "watchdog-canaries")
        );
        assert_eq!(frontier.canary_evidence(), [(12, [5; 32]), (12, [15; 32])]);
        assert_eq!(
            frontier.claim_scope_digests(),
            [[6; 32], [7; 32], [8; 32], [9; 32], [10; 32], [11; 32], [12; 32]]
        );
        assert_eq!(frontier.authority_digests(), ([13; 32], [14; 32]));
        assert_eq!(
            frontier.ledger_classification(),
            ("universal", "[D]", "verified_derived_content")
        );
        assert_eq!(frontier.scope(), (true, false, false, "none"));
    }

    #[test]
    fn premise_admission_frontier_exposes_partial_protocol_progress_without_a_control_edge() {
        let frontier = RepositoryPremiseAdmissionFrontierScene {
            identity: ("route.v1", "producer.v1", "watchdog.v1"),
            checker_result_sha256: ([1; 32], [1; 32]),
            derived_identity_sha256: ([2; 32], [3; 32], [4; 32]),
            counts: (1, 107, 125, 5),
            decisions: ("derived", "next_target_not_bound"),
            scope: (false, true, false, false, "none"),
        };

        assert_ne!(frontier.identity().1, frontier.identity().2);
        assert_eq!(frontier.checker_result_digests(), ([1; 32], [1; 32]));
        assert_eq!(
            frontier.derived_identity_digests(),
            ([2; 32], [3; 32], [4; 32])
        );
        assert_eq!(frontier.counts(), (1, 107, 125, 5));
        assert_eq!(frontier.decisions(), ("derived", "next_target_not_bound"));
        assert_eq!(frontier.scope(), (false, true, false, false, "none"));
    }

    #[test]
    fn repository_physical_frontier_preserves_the_read_only_receipt() {
        let frontier = RepositoryPhysicalRegistryFrontierScene {
            registry_schema_id: "civsim.planet.stellar-birth-physical-species-registry.v7",
            proof_graph_schema_id: "civsim.planet.stellar-birth-species-proof-graph.v6",
            root_receipt_schema_id:
                "civsim.planet.stellar-birth-repository-physical-root-receipt.v5",
            root_claim_id: "planet.stellar-species-floor-coordinate-projection",
            root_canary_suite_id:
                "civsim.planet.stellar-birth-repository-physical-root-canaries.v6",
            root_decision_id: "agreed_projected",
            root_input_sha256: [1; 32],
            root_result_sha256: [2; 32],
            root_producer_result_sha256: [2; 32],
            root_watchdog_result_sha256: [2; 32],
            root_producer_resource_sha256: [3; 32],
            root_watchdog_resource_sha256: [3; 32],
            root_canary_sha256: [4; 32],
            root_receipt_sha256: [5; 32],
            root_producer_id: "civsim.planet.stellar-birth-repository-physical-root-producer.v6",
            root_watchdog_id: "civsim.planet.stellar-birth-repository-physical-root-watchdog.v6",
            scalar_coordinate_count: 3,
            membership_neutral_mass_projection_count: 1,
            admitted_root_count: 4,
            admitted_artifact_count: 125,
            root_admissions: (20_u8..24)
                .map(|identity| PhysicalRootAdmissionScene {
                    identity_sha256: [identity; 32],
                    tier_id: "universal",
                    provenance_tag: "[D]",
                    route_id: "derived",
                })
                .collect(),
            membership_authority: false,
            primitive_profile: RepositoryPrimitiveProfileScene {
                identity: (
                    "civsim.planet.primitive-excitation-profile-pair-receipt.v1",
                    "planet.primitive-excitation.unbroken-abelian-null-mode",
                    "primitive-profile.unbroken-abelian-null-excitation.v1",
                    "compact-rank-one-unbroken-abelian-gauge-theory",
                    "planet.primitive-excitation.theory-profile.unbroken-abelian.v1",
                    "profile-producer",
                    "profile-watchdog",
                ),
                dynamical_identity: (
                    "primitive-null-abelian-gauge-excitation",
                    "unbroken-compact-rank-one-abelian-gauge-symmetry",
                    "abelian-gauge-connection-field",
                    "source-free-gauge-wave-operator",
                    "transverse-null-one-excitation-state",
                    "unbroken-abelian-interaction-sector",
                    "local-source-free-linearized-unbroken-sector",
                    "gauge-noninvariant-rest-mass-term",
                ),
                member_properties: (
                    "massless-helicity-pair-minus-one-plus-one",
                    "integer-spin-bose-statistics",
                    "zero-unbroken-abelian-self-charge",
                    "conserved-unbroken-abelian-current-coupling",
                    "stable-within-unbroken-source-free-validity-domain",
                    "no-lower-profile-state-transition-within-validity-domain",
                ),
                counts: (29, 10, 1),
                admissions: (100_u8..129)
                    .map(|identity| PhysicalRootAdmissionScene {
                        identity_sha256: [identity; 32],
                        tier_id: "residue",
                        provenance_tag: if identity == 100 { "[A]" } else { "[D]" },
                        route_id: if identity == 100 {
                            "irreducible"
                        } else {
                            "derived"
                        },
                    })
                    .collect(),
                member_sha256: [13; 32],
                receipt_sha256: [14; 32],
                evidence_sha256: ([15; 32], [16; 32], [17; 32], [18; 32]),
                repository_catalog_sha256: [19; 32],
                protocol_checker_result_sha256: ([20; 32], [20; 32]),
                protocol_statuses: (
                    "executed_open_frontier",
                    "semantic_inapplicability_paired",
                    "executed_and_bound",
                    "nondynamical_inapplicability_paired",
                    "executed_and_bound",
                    "collision_checked_unique",
                ),
            },
            charged_profile: RepositoryChargedProfileScene {
                identity: (
                    "civsim.planet.charged-matter-profile-pair-receipt.v1",
                    "planet.charged-matter.charge-conjugate-massive-spinor-pair",
                    "matter-profile.charge-conjugate-massive-spinor-pair.v1",
                    "local-lorentz-covariant-complex-spinor-under-admitted-rank-one-abelian-sector",
                    "planet.charged-matter.theory-profile.complex-spinor.v1",
                    "charged-profile-producer",
                    "charged-profile-watchdog",
                ),
                counts: (39, 2),
                admissions: (130_u8..169)
                    .map(|identity| PhysicalRootAdmissionScene {
                        identity_sha256: [identity; 32],
                        tier_id: "residue",
                        provenance_tag: if identity == 130 { "[A]" } else { "[D]" },
                        route_id: if identity == 130 {
                            "irreducible"
                        } else {
                            "derived"
                        },
                    })
                    .collect(),
                member_sha256: vec![[21; 32], [22; 32]],
                receipt_sha256: [23; 32],
                checker_evidence_sha256: ([24; 32], [25; 32], [26; 32], [27; 32]),
                protocol_statuses: (
                    "executed_open_frontier",
                    "semantic_inapplicability_paired",
                    "executed_and_bound",
                    "nondynamical_inapplicability_paired",
                    "executed_and_bound",
                    "collision_checked_unique",
                ),
            },
            neutral_bound_profile: RepositoryNeutralBoundProfileScene {
                identity: (
                    "civsim.planet.neutral-bound-profile-pair-receipt.v1",
                    "planet.neutral-bound-state.equal-mass-opposite-charge-central-ground-level",
                    "bound-profile.neutral-equal-mass-opposite-charge-ground-level.v1",
                    "local-leading-nonrelativistic-attractive-rank-one-abelian-two-body-reduction",
                    "planet.neutral-bound-state.leading-central-interaction.v1",
                    "neutral-bound-profile-producer",
                    "neutral-bound-profile-watchdog",
                ),
                counts: (33, 1),
                admissions: (170_u8..203)
                    .map(|identity| PhysicalRootAdmissionScene {
                        identity_sha256: [identity; 32],
                        tier_id: "residue",
                        provenance_tag: if identity == 170 { "[A]" } else { "[D]" },
                        route_id: if identity == 170 {
                            "irreducible"
                        } else {
                            "derived"
                        },
                    })
                    .collect(),
                member_sha256: [28; 32],
                receipt_sha256: [29; 32],
                checker_evidence_sha256: [
                    [30; 32], [31; 32], [32; 32], [33; 32], [34; 32], [35; 32], [36; 32], [37; 32],
                    [38; 32], [39; 32],
                ],
                channel_digests: ([40; 32], [41; 32], [42; 32], [43; 32]),
                dispositions: (
                    "strictly_below_free_constituent_threshold",
                    "energetically_open_neutral_massless_carrier_family",
                ),
                scope: (false, false),
                protocol_statuses: (
                    "executed_open_frontier",
                    "semantic_inapplicability_paired",
                    "executed_and_bound",
                    "nondynamical_inapplicability_paired",
                    "executed_and_bound",
                    "collision_checked_unique",
                ),
            },
            strong_profile: RepositoryStrongProfileScene {
                identity: (
                    "civsim.planet.confining-profile-pair-receipt.v1",
                    "planet.interaction-profile.compact-noncommutative-confining-asymptotic-boundary",
                    "interaction-profile.compact-noncommutative-confining-sector.v1",
                    "compact-noncommutative-local-gauge-sector-with-admitted-confining-boundary",
                    "planet.interaction-profile.compact-noncommutative-confining-boundary.v1",
                    "confining-profile-producer",
                    "confining-profile-watchdog",
                ),
                counts: (20, 0),
                admissions: (203_u8..223)
                    .map(|identity| PhysicalRootAdmissionScene {
                        identity_sha256: [identity; 32],
                        tier_id: "residue",
                        provenance_tag: if identity == 203 { "[A]" } else { "[D]" },
                        route_id: if identity == 203 {
                            "irreducible"
                        } else {
                            "derived"
                        },
                    })
                    .collect(),
                identity_digests: [[44; 32], [45; 32], [46; 32], [47; 32]],
                receipt_digests: ([48; 32], [49; 32]),
                checker_evidence_sha256: [
                    [50; 32], [51; 32], [52; 32], [53; 32], [54; 32], [55; 32], [56; 32],
                    [57; 32], [58; 32], [59; 32], [60; 32], [61; 32],
                ],
                scope: (false, false, false, false, false, true, "none"),
                protocol_statuses: (
                    "executed_open_frontier",
                    "semantic_inapplicability_paired",
                    "executed_and_bound",
                    "nondynamical_inapplicability_paired",
                    "executed_and_bound",
                    "collision_checked_unique",
                ),
            },
            vocabulary_receipt_schema_id:
                "civsim.planet.stellar-birth-physical-vocabulary-agreement.v1",
            vocabulary_claim_id: "civsim.planet.stellar-birth-physical-vocabulary.partition.v1",
            vocabulary_producer_id: "civsim.planet.stellar-birth-physical-vocabulary-producer.v1",
            vocabulary_watchdog_id: "civsim.planet.stellar-birth-physical-vocabulary-watchdog.v1",
            vocabulary_descriptor_role_identities: (30_u8..137)
                .map(|identity| [identity; 32])
                .collect(),
            vocabulary_relation_target_identities: (100_u8..225)
                .map(|identity| [identity; 32])
                .collect(),
            vocabulary_constraint_law_identities: vec![
                [94; 32], [95; 32], [96; 32], [97; 32], [98; 32],
            ],
            vocabulary_counts: (125, 107, 125, 5),
            vocabulary_scope: (true, false, false),
            vocabulary_checker_result_sha256: ([10; 32], [10; 32]),
            vocabulary_checker_resource_sha256: ([11; 32], [11; 32]),
            vocabulary_receipt_sha256: [12; 32],
            refusal_code: "none",
            registry_member_count: 4,
            registry_coverage_claim: false,
            registry_authority_effect: "none",
            open_obligations: vec![
                "complete_global_physical_vocabulary_coverage",
                "complete_registry_closure_domain",
                "conditioned_species_support",
                "confining-sector-constituent-content",
                "multi-constituent-bound-state-spectrum",
                "reaction-network-closure",
            ],
        };

        assert_eq!(
            frontier.registry_schema_identity(),
            (
                "civsim.planet.stellar-birth-physical-species-registry.v7",
                "civsim.planet.stellar-birth-species-proof-graph.v6"
            )
        );
        assert_eq!(
            frontier.receipt_identity(),
            (
                "civsim.planet.stellar-birth-repository-physical-root-receipt.v5",
                "planet.stellar-species-floor-coordinate-projection"
            )
        );
        assert_eq!(
            frontier.canary_suite_id(),
            "civsim.planet.stellar-birth-repository-physical-root-canaries.v6"
        );
        assert_eq!(frontier.decision_id(), "agreed_projected");
        assert_eq!(frontier.input_sha256(), [1; 32]);
        assert_eq!(frontier.result_sha256(), [2; 32]);
        assert_eq!(frontier.checker_result_sha256(), ([2; 32], [2; 32]));
        assert_eq!(frontier.checker_resource_sha256(), ([3; 32], [3; 32]));
        assert_eq!(frontier.canary_sha256(), [4; 32]);
        assert_eq!(frontier.receipt_sha256(), [5; 32]);
        assert_ne!(frontier.checker_ids().0, frontier.checker_ids().1);
        assert_eq!(frontier.scalar_coordinate_count(), 3);
        assert_eq!(frontier.membership_neutral_mass_projection_count(), 1);
        assert_eq!(frontier.admitted_root_count(), 4);
        assert_eq!(frontier.admitted_artifact_count(), 125);
        assert_eq!(frontier.primitive_profile().counts(), (29, 10, 1));
        assert_eq!(frontier.primitive_profile().admissions().len(), 29);
        assert_eq!(
            frontier
                .primitive_profile()
                .admissions()
                .iter()
                .filter(|admission| admission.provenance_tag() == "[A]")
                .count(),
            1
        );
        assert_eq!(frontier.primitive_profile().member_sha256(), [13; 32]);
        assert_eq!(frontier.primitive_profile().receipt_sha256(), [14; 32]);
        assert_eq!(
            frontier.primitive_profile().repository_catalog_sha256(),
            [19; 32]
        );
        assert_eq!(
            frontier
                .primitive_profile()
                .protocol_checker_result_sha256(),
            ([20; 32], [20; 32])
        );
        assert_eq!(
            frontier.primitive_profile().dynamical_identity().0,
            "primitive-null-abelian-gauge-excitation"
        );
        assert_eq!(
            frontier.primitive_profile().member_properties().0,
            "massless-helicity-pair-minus-one-plus-one"
        );
        assert_eq!(
            frontier.primitive_profile().protocol_statuses(),
            (
                "executed_open_frontier",
                "semantic_inapplicability_paired",
                "executed_and_bound",
                "nondynamical_inapplicability_paired",
                "executed_and_bound",
                "collision_checked_unique",
            )
        );
        assert_eq!(frontier.charged_profile().counts(), (39, 2));
        assert_eq!(frontier.charged_profile().admissions().len(), 39);
        assert_eq!(
            frontier
                .charged_profile()
                .admissions()
                .iter()
                .filter(|admission| admission.provenance_tag() == "[A]")
                .count(),
            1
        );
        assert_eq!(
            frontier.charged_profile().member_sha256(),
            &[[21; 32], [22; 32]]
        );
        assert_eq!(frontier.charged_profile().receipt_sha256(), [23; 32]);
        assert_eq!(
            frontier.charged_profile().checker_evidence_sha256(),
            ([24; 32], [25; 32], [26; 32], [27; 32])
        );
        assert_eq!(frontier.neutral_bound_profile().counts(), (33, 1));
        assert_eq!(frontier.neutral_bound_profile().admissions().len(), 33);
        assert_eq!(
            frontier
                .neutral_bound_profile()
                .admissions()
                .iter()
                .filter(|admission| admission.provenance_tag() == "[A]")
                .count(),
            1
        );
        assert_eq!(frontier.neutral_bound_profile().member_sha256(), [28; 32]);
        assert_eq!(frontier.neutral_bound_profile().receipt_sha256(), [29; 32]);
        assert_eq!(
            frontier.neutral_bound_profile().checker_evidence_sha256(),
            [
                [30; 32], [31; 32], [32; 32], [33; 32], [34; 32], [35; 32], [36; 32], [37; 32],
                [38; 32], [39; 32],
            ]
        );
        assert_eq!(
            frontier.neutral_bound_profile().channel_digests(),
            ([40; 32], [41; 32], [42; 32], [43; 32])
        );
        assert_eq!(
            frontier.neutral_bound_profile().dispositions(),
            (
                "strictly_below_free_constituent_threshold",
                "energetically_open_neutral_massless_carrier_family",
            )
        );
        assert_eq!(frontier.neutral_bound_profile().scope(), (false, false));
        assert_eq!(frontier.strong_profile().counts(), (20, 0));
        assert_eq!(frontier.strong_profile().admissions().len(), 20);
        assert_eq!(
            frontier
                .strong_profile()
                .admissions()
                .iter()
                .filter(|admission| admission.provenance_tag() == "[A]")
                .count(),
            1
        );
        assert_eq!(
            frontier.strong_profile().identity_digests(),
            [[44; 32], [45; 32], [46; 32], [47; 32]]
        );
        assert_eq!(
            frontier.strong_profile().receipt_digests(),
            ([48; 32], [49; 32])
        );
        assert_eq!(
            frontier.strong_profile().scope(),
            (false, false, false, false, false, true, "none")
        );
        assert_eq!(frontier.root_admissions().len(), 4);
        assert!(frontier.root_admissions().iter().all(|admission| {
            admission.tier_id() == "universal"
                && admission.provenance_tag() == "[D]"
                && admission.route_id() == "derived"
        }));
        assert!(!frontier.membership_authority());
        assert_eq!(
            frontier.vocabulary_identity(),
            (
                "civsim.planet.stellar-birth-physical-vocabulary-agreement.v1",
                "civsim.planet.stellar-birth-physical-vocabulary.partition.v1",
                "civsim.planet.stellar-birth-physical-vocabulary-producer.v1",
                "civsim.planet.stellar-birth-physical-vocabulary-watchdog.v1",
            )
        );
        assert_eq!(frontier.vocabulary_partitions().0.len(), 107);
        assert_eq!(frontier.vocabulary_partitions().1.len(), 125);
        assert_eq!(frontier.vocabulary_partitions().2.len(), 5);
        assert_eq!(frontier.vocabulary_counts(), (125, 107, 125, 5));
        assert_eq!(frontier.vocabulary_scope(), (true, false, false));
        assert_eq!(
            frontier.vocabulary_checker_digests(),
            (([10; 32], [10; 32]), ([11; 32], [11; 32]))
        );
        assert_eq!(frontier.vocabulary_receipt_sha256(), [12; 32]);
        assert_eq!(frontier.refusal_code(), "none");
        assert_eq!(frontier.registry_member_count(), 4);
        assert!(!frontier.registry_coverage_claim());
        assert_eq!(frontier.registry_authority_effect(), "none");
        assert_eq!(
            frontier.open_obligations(),
            [
                "complete_global_physical_vocabulary_coverage",
                "complete_registry_closure_domain",
                "conditioned_species_support",
                "confining-sector-constituent-content",
                "multi-constituent-bound-state-spectrum",
                "reaction-network-closure",
            ]
        );
    }
}
