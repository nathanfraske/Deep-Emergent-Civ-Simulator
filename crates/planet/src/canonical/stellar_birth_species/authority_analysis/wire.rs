//! Canonical text encoding for species derivation exhaustion.

use super::{validate_analysis, SpeciesDerivationAnalysisArtifact};
use crate::canonical::transcript::canonical_text;
use std::fmt;

pub(in crate::canonical) fn write_species_derivation_analysis(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    artifact: &SpeciesDerivationAnalysisArtifact,
) -> fmt::Result {
    match artifact {
        SpeciesDerivationAnalysisArtifact::Invalid(invalid) => {
            writeln!(f, "{prefix}.error.code={}", invalid.error_code)?;
            writeln!(
                f,
                "{prefix}.error.detail={}",
                canonical_text(&invalid.detail)
            )
        }
        SpeciesDerivationAnalysisArtifact::Computed(analysis) => {
            validate_analysis(analysis).map_err(|_| fmt::Error)?;
            write_schema_bindings(f, prefix, analysis)?;
            write_floor_anchor(f, prefix, analysis)?;
            writeln!(
                f,
                "{prefix}.candidate_member_count={}",
                analysis.candidate_member_count
            )?;
            writeln!(
                f,
                "{prefix}.verified_support_member_count={}",
                analysis.verified_support_member_count
            )?;
            writeln!(
                f,
                "{prefix}.value_payload_present={}",
                analysis.value_payload_present
            )?;
            writeln!(
                f,
                "{prefix}.residual_slot_claim={}",
                analysis.residual_slot_claim
            )?;
            write_protocol_statuses(f, prefix, analysis)?;
            write_string_list(f, prefix, "open_proof", &analysis.open_proof_ids)?;
            writeln!(f, "{prefix}.attempt_count={}", analysis.attempts.len())?;
            for (index, attempt) in analysis.attempts.iter().enumerate() {
                let attempt_prefix = format!("{prefix}.attempt.{index:04}");
                writeln!(f, "{attempt_prefix}.id={}", canonical_text(attempt.id))?;
                writeln!(f, "{attempt_prefix}.status={}", attempt.status.id())?;
                write_string_list(f, &attempt_prefix, "input", &attempt.input_ids)?;
                write_string_list(f, &attempt_prefix, "open_proof", &attempt.open_proof_ids)?;
            }
            Ok(())
        }
    }
}

fn write_schema_bindings(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    analysis: &super::SpeciesDerivationAnalysis,
) -> fmt::Result {
    for (field, value) in [
        ("floor_binding.schema", analysis.floor_binding_schema_id),
        ("structure_schema", analysis.structure_schema_id),
        (
            "species_registry_schema",
            analysis.species_registry_schema_id,
        ),
        ("stellar_state_schema", analysis.stellar_state_schema_id),
        (
            "state_coordinate_registry_schema",
            analysis.state_coordinate_registry_schema_id,
        ),
        (
            "interaction_sector_registry_schema",
            analysis.interaction_sector_registry_schema_id,
        ),
        (
            "physical_regime_registry_schema",
            analysis.physical_regime_registry_schema_id,
        ),
        ("reducer_law_id", analysis.reducer_law_id),
    ] {
        writeln!(f, "{prefix}.{field}={}", canonical_text(value))?;
    }
    writeln!(
        f,
        "{prefix}.floor_binding.sha256={}",
        analysis.floor_binding_sha256
    )
}

fn write_floor_anchor(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    analysis: &super::SpeciesDerivationAnalysis,
) -> fmt::Result {
    writeln!(
        f,
        "{prefix}.floor_anchor.id={}",
        canonical_text(analysis.floor_mass_anchor.id)
    )?;
    writeln!(
        f,
        "{prefix}.floor_anchor.symbol={}",
        canonical_text(analysis.floor_mass_anchor.symbol)
    )?;
    writeln!(
        f,
        "{prefix}.floor_anchor.bits={}",
        analysis.floor_mass_anchor.bits
    )?;
    writeln!(
        f,
        "{prefix}.floor_anchor.scale_bits={}",
        analysis.floor_mass_anchor.scale_bits
    )?;
    writeln!(
        f,
        "{prefix}.floor_anchor.role={}",
        analysis.floor_mass_anchor.role
    )?;
    writeln!(
        f,
        "{prefix}.floor_anchor.membership_authority={}",
        analysis.floor_mass_anchor.membership_authority
    )
}

fn write_protocol_statuses(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    analysis: &super::SpeciesDerivationAnalysis,
) -> fmt::Result {
    for (field, status) in [
        ("derive_first", analysis.derive_first_status),
        ("buckingham_pi", analysis.buckingham_pi_status),
        ("gap_law", analysis.gap_law_status),
        ("chaos", analysis.chaos_protocol_status),
        ("residual_law", analysis.residual_law_status),
        ("unique_residual_slot", analysis.unique_residual_slot_status),
    ] {
        writeln!(f, "{prefix}.protocol.{field}={}", status.id())?;
    }
    Ok(())
}

fn write_string_list(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    field: &str,
    values: &[String],
) -> fmt::Result {
    writeln!(f, "{prefix}.{field}_count={}", values.len())?;
    for (index, value) in values.iter().enumerate() {
        writeln!(f, "{prefix}.{field}.{index:04}={}", canonical_text(value))?;
    }
    Ok(())
}
