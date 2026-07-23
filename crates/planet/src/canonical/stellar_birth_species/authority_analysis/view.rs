//! Public read-only views over species derivation exhaustion.

use super::{
    SpeciesDerivationAnalysis, SpeciesDerivationAnalysisArtifact, SpeciesDerivationAttempt,
};

/// Read-only view of the authority analysis attached to the open joint measure.
#[derive(Debug, Clone, Copy)]
pub struct SpeciesDerivationAnalysisView<'a> {
    artifact: &'a SpeciesDerivationAnalysisArtifact,
}

/// Read-only view of one blocked derive-first attempt.
#[derive(Debug, Clone, Copy)]
pub struct SpeciesDerivationAttemptView<'a> {
    attempt: &'a SpeciesDerivationAttempt,
}

impl<'a> SpeciesDerivationAnalysisView<'a> {
    pub(in crate::canonical) const fn new(artifact: &'a SpeciesDerivationAnalysisArtifact) -> Self {
        Self { artifact }
    }

    fn computed(self) -> Option<&'a SpeciesDerivationAnalysis> {
        match self.artifact {
            SpeciesDerivationAnalysisArtifact::Computed(analysis) => Some(analysis),
            SpeciesDerivationAnalysisArtifact::Invalid(_) => None,
        }
    }

    pub fn is_computed(self) -> bool {
        self.computed().is_some()
    }

    pub fn floor_binding_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.floor_binding_schema_id)
    }

    pub fn floor_binding_sha256(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.floor_binding_sha256.as_str())
    }

    pub fn structure_schema_id(self) -> Option<&'a str> {
        self.computed().map(|analysis| analysis.structure_schema_id)
    }

    pub fn species_registry_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.species_registry_schema_id)
    }

    pub fn stellar_state_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.stellar_state_schema_id)
    }

    pub fn state_coordinate_registry_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.state_coordinate_registry_schema_id)
    }

    pub fn interaction_sector_registry_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.interaction_sector_registry_schema_id)
    }

    pub fn physical_regime_registry_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_regime_registry_schema_id)
    }

    pub fn reducer_law_id(self) -> Option<&'a str> {
        self.computed().map(|analysis| analysis.reducer_law_id)
    }

    pub fn floor_anchor_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.id)
    }

    pub fn floor_anchor_symbol(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.symbol)
    }

    pub fn floor_anchor_bits(self) -> Option<i128> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.bits)
    }

    pub fn floor_anchor_scale_bits(self) -> Option<u32> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.scale_bits)
    }

    pub fn floor_anchor_membership_authority(self) -> Option<bool> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.membership_authority)
    }

    pub fn floor_anchor_role(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.role)
    }

    pub fn candidate_member_count(self) -> Option<usize> {
        self.computed()
            .map(|analysis| analysis.candidate_member_count)
    }

    pub fn verified_support_member_count(self) -> Option<usize> {
        self.computed()
            .map(|analysis| analysis.verified_support_member_count)
    }

    pub fn value_payload_present(self) -> Option<bool> {
        self.computed()
            .map(|analysis| analysis.value_payload_present)
    }

    pub fn residual_slot_claim(self) -> Option<bool> {
        self.computed().map(|analysis| analysis.residual_slot_claim)
    }

    pub fn derive_first_status_id(self) -> Option<&'static str> {
        self.computed()
            .map(|analysis| analysis.derive_first_status.id())
    }

    pub fn buckingham_pi_status_id(self) -> Option<&'static str> {
        self.computed()
            .map(|analysis| analysis.buckingham_pi_status.id())
    }

    pub fn gap_law_status_id(self) -> Option<&'static str> {
        self.computed().map(|analysis| analysis.gap_law_status.id())
    }

    pub fn chaos_protocol_status_id(self) -> Option<&'static str> {
        self.computed()
            .map(|analysis| analysis.chaos_protocol_status.id())
    }

    pub fn residual_law_status_id(self) -> Option<&'static str> {
        self.computed()
            .map(|analysis| analysis.residual_law_status.id())
    }

    pub fn unique_residual_slot_status_id(self) -> Option<&'static str> {
        self.computed()
            .map(|analysis| analysis.unique_residual_slot_status.id())
    }

    pub fn open_proof_ids(self) -> &'a [String] {
        self.computed()
            .map_or(&[], |analysis| analysis.open_proof_ids.as_slice())
    }

    pub fn attempts(self) -> impl ExactSizeIterator<Item = SpeciesDerivationAttemptView<'a>> + 'a {
        let attempts: &'a [SpeciesDerivationAttempt] = self
            .computed()
            .map_or(&[], |analysis| analysis.attempts.as_slice());
        attempts
            .iter()
            .map(|attempt| SpeciesDerivationAttemptView { attempt })
    }

    pub fn error_code(self) -> Option<&'a str> {
        match self.artifact {
            SpeciesDerivationAnalysisArtifact::Computed(_) => None,
            SpeciesDerivationAnalysisArtifact::Invalid(invalid) => Some(invalid.error_code),
        }
    }

    pub fn error_detail(self) -> Option<&'a str> {
        match self.artifact {
            SpeciesDerivationAnalysisArtifact::Computed(_) => None,
            SpeciesDerivationAnalysisArtifact::Invalid(invalid) => Some(&invalid.detail),
        }
    }
}

impl<'a> SpeciesDerivationAttemptView<'a> {
    pub fn id(self) -> &'a str {
        self.attempt.id
    }

    pub fn status_id(self) -> &'static str {
        self.attempt.status.id()
    }

    pub fn input_ids(self) -> &'a [String] {
        &self.attempt.input_ids
    }

    pub fn open_proof_ids(self) -> &'a [String] {
        &self.attempt.open_proof_ids
    }
}
