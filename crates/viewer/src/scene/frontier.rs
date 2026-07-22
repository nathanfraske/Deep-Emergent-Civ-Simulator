//! Exact stage and refusal-frontier projections.

use super::TranscriptScene;
use crate::RefusalView;

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

/// Summary of the attached exact dimensional census.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionalCensusScene<'a> {
    computed: bool,
    representation_schema_id: Option<&'a str>,
    floor_binding_schema_id: Option<&'a str>,
    floor_binding_sha256: Option<&'a str>,
    base_dimension_ids: Vec<&'a str>,
    structure_schema_id: Option<&'a str>,
    variable_count: usize,
    phenomenon_count: usize,
    coverage_gap_ids: Vec<&'a str>,
    error_code: Option<&'a str>,
    error_detail: Option<&'a str>,
}

impl<'a> DimensionalCensusScene<'a> {
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

    /// Base dimensions in canonical schema order.
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
        self.phenomenon_count
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

/// Exact non-admitting species derivation analysis attached to Stage 1.
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
    candidate_member_count: Option<usize>,
    verified_support_member_count: Option<usize>,
    value_payload_present: Option<bool>,
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

    /// Candidate and verified positive-support member counts.
    pub const fn support_counts(&self) -> (Option<usize>, Option<usize>) {
        (
            self.candidate_member_count,
            self.verified_support_member_count,
        )
    }

    /// Whether a physical value or residual-slot claim is present.
    pub const fn admission_claims(&self) -> (Option<bool>, Option<bool>) {
        (self.value_payload_present, self.residual_slot_claim)
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

    /// All seven stages in their canonical order.
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
                                    .map(|view| DimensionalCensusScene {
                                        computed: view.is_computed(),
                                        representation_schema_id: view.representation_schema_id(),
                                        floor_binding_schema_id: view.floor_binding_schema_id(),
                                        floor_binding_sha256: view.floor_binding_sha256(),
                                        base_dimension_ids: view.base_dimension_ids().to_vec(),
                                        structure_schema_id: view.structure_schema_id(),
                                        variable_count: view.variables().len(),
                                        phenomenon_count: view.phenomena().len(),
                                        coverage_gap_ids: borrowed_ids(view.coverage_gap_ids()),
                                        error_code: view.error_code(),
                                        error_detail: view.error_detail(),
                                    });
                                let species_derivation = analysis
                                    .species_derivation_analysis_view()
                                    .map(|view| SpeciesDerivationScene {
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
                                        candidate_member_count: view.candidate_member_count(),
                                        verified_support_member_count: view
                                            .verified_support_member_count(),
                                        value_payload_present: view.value_payload_present(),
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
}
