//! Independent mechanical evidence for one irreducible theory profile.
//!
//! This pair scans the registered premise catalog and occupied residual slots.
//! It also classifies this request type as categorical and nondynamical because
//! the type has no numeric-basis, trajectory, time-step, or realization fields.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CatalogSeed {
    pub(super) claim_identity: [u8; 32],
    pub(super) role_identity: [u8; 32],
    pub(super) content_identity: [u8; 32],
    pub(super) capability_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CatalogRule {
    pub(super) claim_identity: [u8; 32],
    pub(super) conclusion_role_identity: [u8; 32],
    pub(super) conclusion_content_identity: [u8; 32],
    pub(super) capability_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProfileProtocolInput {
    pub(super) claim_identity: [u8; 32],
    pub(super) role_identity: [u8; 32],
    pub(super) content_identity: [u8; 32],
    pub(super) profile_input_sha256: [u8; 32],
    pub(super) source_custody_sha256: [u8; 32],
    pub(super) applicability_receipt_sha256: [u8; 32],
    pub(super) validity_receipt_sha256: [u8; 32],
    pub(super) derivation_catalog_sha256: [u8; 32],
    pub(super) repository_seeds: Vec<CatalogSeed>,
    pub(super) repository_rules: Vec<CatalogRule>,
    pub(super) residual_slot_id: String,
    pub(super) occupied_residual_slots: Vec<String>,
    pub(super) owner_admission_record: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProfileProtocolAssessment {
    pub(super) repository_catalog_sha256: [u8; 32],
    pub(super) repository_seed_count: u32,
    pub(super) repository_rule_count: u32,
    pub(super) target_scoped_seed_count: u32,
    pub(super) target_scoped_rule_count: u32,
    pub(super) numeric_basis_count: u32,
    pub(super) trajectory_coordinate_count: u32,
    pub(super) occupied_slot_count: u32,
    pub(super) occupied_slot_inventory_sha256: [u8; 32],
    pub(super) residual_slot_collision: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProfileProtocolCheckerReceipts {
    pub(super) coverage_sha256: [u8; 32],
    pub(super) buckingham_pi_sha256: [u8; 32],
    pub(super) gap_law_sha256: [u8; 32],
    pub(super) chaos_protocol_sha256: [u8; 32],
    pub(super) residual_law_sha256: [u8; 32],
    pub(super) residual_slot_sha256: [u8; 32],
    pub(super) owner_admission_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProfileProtocolCheckerOutput {
    pub(super) assessment: ProfileProtocolAssessment,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) receipts: ProfileProtocolCheckerReceipts,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProfileProtocolPairEvidence {
    pub(super) assessment: ProfileProtocolAssessment,
    pub(super) producer_receipts: ProfileProtocolCheckerReceipts,
    pub(super) watchdog_receipts: ProfileProtocolCheckerReceipts,
    pub(super) producer_result_sha256: [u8; 32],
    pub(super) watchdog_result_sha256: [u8; 32],
}

pub(super) fn inspect_forward(
    input: &ProfileProtocolInput,
) -> Result<ProfileProtocolCheckerOutput, &'static str> {
    producer::inspect(input)
}

pub(super) fn inspect_reverse(
    input: &ProfileProtocolInput,
) -> Result<ProfileProtocolCheckerOutput, &'static str> {
    watchdog::inspect(input)
}

#[cfg(test)]
pub(super) fn inspect(
    input: &ProfileProtocolInput,
) -> Result<ProfileProtocolPairEvidence, &'static str> {
    let produced = producer::inspect(input)?;
    let watched = watchdog::inspect(input)?;
    if produced.assessment != watched.assessment
        || produced.canonical_bytes != watched.canonical_bytes
    {
        return Err("profile_protocol_checker_disagreement");
    }
    if produced.assessment.target_scoped_seed_count != 0
        || produced.assessment.target_scoped_rule_count != 0
        || produced.assessment.numeric_basis_count != 0
        || produced.assessment.trajectory_coordinate_count != 0
    {
        return Err("profile_protocol_scope_invalid");
    }
    if produced.assessment.residual_slot_collision {
        return Err("profile_protocol_residual_slot_collision");
    }
    let producer_receipts = produced.receipts;
    let watchdog_receipts = watched.receipts;
    let producer_result_sha256 = civsim_units::digest::sha256(&produced.canonical_bytes);
    let watchdog_result_sha256 = civsim_units::digest::sha256(&watched.canonical_bytes);
    if producer_result_sha256 == [0; 32]
        || producer_result_sha256 != watchdog_result_sha256
        || receipts_invalid(producer_receipts)
        || receipts_invalid(watchdog_receipts)
        || producer_receipts == watchdog_receipts
    {
        return Err("profile_protocol_receipt_invalid");
    }
    Ok(ProfileProtocolPairEvidence {
        assessment: produced.assessment,
        producer_receipts,
        watchdog_receipts,
        producer_result_sha256,
        watchdog_result_sha256,
    })
}

#[cfg(test)]
fn receipts_invalid(receipts: ProfileProtocolCheckerReceipts) -> bool {
    let values = [
        receipts.coverage_sha256,
        receipts.buckingham_pi_sha256,
        receipts.gap_law_sha256,
        receipts.chaos_protocol_sha256,
        receipts.residual_law_sha256,
        receipts.residual_slot_sha256,
        receipts.owner_admission_sha256,
    ];
    values.contains(&[0; 32])
        || values
            .iter()
            .enumerate()
            .any(|(index, digest)| values[index + 1..].contains(digest))
}
