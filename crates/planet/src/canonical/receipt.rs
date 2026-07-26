use super::{
    requirement_analysis::{RequirementAnalysis, RequirementAnalysisPayload},
    stellar_birth_dimensions::StellarBirthDimensionalCensusArtifact,
    stellar_birth_species::write_species_derivation_analysis,
    stellar_birth_structure::write_stellar_birth_structure,
    transcript::canonical_text,
    EventId, RealizationId, RepresentationStatus, RunEventKind, RunTranscript, Stage,
    TranscriptError,
};
use std::{cmp::Ordering, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefusalCode {
    FloorCatalogMismatch,
    FloorMagnitudeUnavailable,
    MissingStageRequirement,
    PipelineIncomplete,
    TranscriptInvariantViolation,
    RepresentationUnavailable,
}

/// One unresolved proof leaf and its canonical ordered closure obligations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OpenRequirement {
    requirement_id: String,
    obligations: Vec<String>,
    analyses: Vec<RequirementAnalysis>,
}

impl OpenRequirement {
    pub(crate) fn new(requirement_id: &str, obligations: &[&str]) -> Self {
        Self {
            requirement_id: requirement_id.to_owned(),
            obligations: obligations
                .iter()
                .map(|obligation| (*obligation).to_owned())
                .collect(),
            analyses: Vec::new(),
        }
    }

    pub(crate) fn with_analyses(
        requirement_id: &str,
        obligations: &[&str],
        analyses: Vec<RequirementAnalysis>,
    ) -> Self {
        let mut requirement = Self::new(requirement_id, obligations);
        requirement.analyses = analyses;
        requirement
    }

    pub fn requirement_id(&self) -> &str {
        &self.requirement_id
    }

    pub fn obligations(&self) -> &[String] {
        &self.obligations
    }

    pub fn analyses(&self) -> &[RequirementAnalysis] {
        &self.analyses
    }
}

impl RefusalCode {
    pub const fn id(self) -> &'static str {
        match self {
            Self::FloorCatalogMismatch => "floor_catalog_mismatch",
            Self::FloorMagnitudeUnavailable => "floor_magnitude_unavailable",
            Self::MissingStageRequirement => "missing_stage_requirement",
            Self::PipelineIncomplete => "pipeline_incomplete",
            Self::TranscriptInvariantViolation => "transcript_invariant_violation",
            Self::RepresentationUnavailable => "representation_unavailable",
        }
    }
}

/// Structured reason a canonical run stopped.
///
/// `detail` is a readable explanation. Verification uses `code`, `stage`,
/// `requirement_id`, and the ordered open frontier rather than parsing that
/// prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    code: RefusalCode,
    stage: Option<Stage>,
    requirement_id: Option<String>,
    open_requirements: Vec<OpenRequirement>,
    detail: String,
}

impl Refusal {
    pub(crate) fn floor_catalog_mismatch(detail: String) -> Self {
        Self {
            code: RefusalCode::FloorCatalogMismatch,
            stage: None,
            requirement_id: Some("repository.absolute_floor_catalog".into()),
            open_requirements: Vec::new(),
            detail,
        }
    }

    pub(crate) fn floor_magnitude_unavailable(detail: String) -> Self {
        Self {
            code: RefusalCode::FloorMagnitudeUnavailable,
            stage: None,
            requirement_id: Some("repository.absolute_floor_magnitudes".into()),
            open_requirements: Vec::new(),
            detail,
        }
    }

    #[cfg(test)]
    pub(crate) fn missing_stage_requirement(stage: Stage, requirement_id: &str) -> Self {
        Self {
            code: RefusalCode::MissingStageRequirement,
            stage: Some(stage),
            requirement_id: Some(requirement_id.to_owned()),
            open_requirements: Vec::new(),
            detail: format!(
                "stage '{}' requires derived or admitted absolute-floor measure '{requirement_id}'",
                stage.id()
            ),
        }
    }

    pub(crate) fn missing_stage_requirement_frontier(
        stage: Stage,
        requirement_id: &str,
        open_requirements: Vec<OpenRequirement>,
    ) -> Self {
        Self {
            code: RefusalCode::MissingStageRequirement,
            stage: Some(stage),
            requirement_id: Some(requirement_id.to_owned()),
            detail: format!(
                "stage '{}' requires derived or admitted absolute-floor measure '{requirement_id}'; {} leaf requirement(s) remain open",
                stage.id(),
                open_requirements.len()
            ),
            open_requirements,
        }
    }

    pub(crate) fn pipeline_incomplete(stage: Stage, detail: &str) -> Self {
        Self {
            code: RefusalCode::PipelineIncomplete,
            stage: Some(stage),
            requirement_id: Some(format!("stage.{}.physical_closure", stage.id())),
            open_requirements: Vec::new(),
            detail: detail.to_owned(),
        }
    }

    pub(crate) fn transcript_invariant(detail: String) -> Self {
        Self {
            code: RefusalCode::TranscriptInvariantViolation,
            stage: None,
            requirement_id: Some("canonical.run_transcript".into()),
            open_requirements: Vec::new(),
            detail,
        }
    }

    pub(crate) fn representation_unavailable(detail: String) -> Self {
        Self {
            code: RefusalCode::RepresentationUnavailable,
            stage: None,
            requirement_id: Some("repository.si_representation_projection".into()),
            open_requirements: Vec::new(),
            detail,
        }
    }

    pub const fn code(&self) -> RefusalCode {
        self.code
    }

    pub const fn stage(&self) -> Option<Stage> {
        self.stage
    }

    pub fn requirement_id(&self) -> Option<&str> {
        self.requirement_id.as_deref()
    }

    pub fn open_requirements(&self) -> &[OpenRequirement] {
        &self.open_requirements
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub(super) fn attach_stage(&mut self, expected: Option<Stage>) -> Result<(), Option<Stage>> {
        match (self.stage, expected) {
            (None, stage) => {
                self.stage = stage;
                Ok(())
            }
            (found, expected) if found == expected => Ok(()),
            (found, _) => Err(found),
        }
    }

    pub(super) fn canonical_cmp(&self, other: &Self) -> Ordering {
        self.code
            .cmp(&other.code)
            .then_with(|| self.stage.cmp(&other.stage))
            .then_with(|| self.requirement_id.cmp(&other.requirement_id))
            .then_with(|| self.open_requirements.cmp(&other.open_requirements))
            .then_with(|| self.detail.cmp(&other.detail))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    NotReached,
    Refused,
    Reached,
}

impl StageStatus {
    pub const fn id(self) -> &'static str {
        match self {
            Self::NotReached => "not_reached",
            Self::Refused => "refused",
            Self::Reached => "reached",
        }
    }
}

/// Stage summary whose event identities point into the concrete transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageReceipt {
    stage: Stage,
    status: StageStatus,
    entered_event: Option<EventId>,
    terminal_event: Option<EventId>,
}

impl StageReceipt {
    pub const fn stage(&self) -> Stage {
        self.stage
    }

    pub const fn status(&self) -> StageStatus {
        self.status
    }

    pub const fn entered_event(&self) -> Option<EventId> {
        self.entered_event
    }

    pub const fn terminal_event(&self) -> Option<EventId> {
        self.terminal_event
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReceipt {
    absolute_floor_entries: usize,
    stages: Vec<StageReceipt>,
    refusals: Vec<Refusal>,
    transcript: RunTranscript,
}

impl RunReceipt {
    pub(crate) fn refused(absolute_floor_entries: usize, refusals: Vec<Refusal>) -> Self {
        let transcript = RunTranscript::empty(absolute_floor_entries);
        Self::refused_after_transcript_initialization(absolute_floor_entries, transcript, refusals)
    }

    fn refused_after_transcript_initialization(
        absolute_floor_entries: usize,
        transcript: Result<RunTranscript, TranscriptError>,
        refusals: Vec<Refusal>,
    ) -> Self {
        match transcript {
            Ok(mut transcript) => {
                transcript
                    .refuse(None, refusals)
                    .expect("a nonempty preflight refusal closes an empty transcript");
                Self::from_transcript(absolute_floor_entries, transcript)
            }
            Err(error) => Self::representation_unavailable_with_refusals(
                absolute_floor_entries,
                error.to_string(),
                refusals,
            ),
        }
    }

    /// Terminal receipt for a failed SI representation projection. It never
    /// re-enters projection or carries any projected physical value.
    pub(crate) fn representation_unavailable(
        absolute_floor_entries: usize,
        detail: String,
    ) -> Self {
        Self::representation_unavailable_with_refusals(absolute_floor_entries, detail, Vec::new())
    }

    fn representation_unavailable_with_refusals(
        absolute_floor_entries: usize,
        detail: String,
        refusals: Vec<Refusal>,
    ) -> Self {
        Self::from_transcript(
            absolute_floor_entries,
            RunTranscript::representation_unavailable_with_refusals(
                absolute_floor_entries,
                detail,
                refusals,
            ),
        )
    }

    /// Build a refusal after the canonical run has entered one physical stage.
    #[cfg(test)]
    pub(crate) fn refused_at(
        absolute_floor_entries: usize,
        stage: Stage,
        refusals: Vec<Refusal>,
    ) -> Self {
        let mut transcript = match RunTranscript::empty(absolute_floor_entries) {
            Ok(transcript) => transcript,
            Err(error) => {
                return Self::representation_unavailable(absolute_floor_entries, error.to_string());
            }
        };
        for prior in Stage::ALL {
            transcript
                .enter_stage(prior)
                .expect("stages are appended in canonical order");
            if prior == stage {
                transcript
                    .refuse(Some(stage), refusals)
                    .expect("the active stage can append a refusal");
                break;
            }
            transcript
                .reach_active_stage()
                .expect("an entered prior stage can be reached");
        }
        Self::from_transcript(absolute_floor_entries, transcript)
    }

    pub(crate) fn refused_with_transcript(
        absolute_floor_entries: usize,
        mut transcript: RunTranscript,
        stage: Option<Stage>,
        refusals: Vec<Refusal>,
    ) -> Result<Self, TranscriptError> {
        transcript.refuse(stage, refusals)?;
        Ok(Self::from_transcript(absolute_floor_entries, transcript))
    }

    fn from_transcript(absolute_floor_entries: usize, transcript: RunTranscript) -> Self {
        debug_assert!(transcript.is_closed());
        let mut stages: Vec<_> = Stage::ALL
            .into_iter()
            .map(|stage| StageReceipt {
                stage,
                status: StageStatus::NotReached,
                entered_event: None,
                terminal_event: None,
            })
            .collect();
        let mut refusals = Vec::new();
        for event in transcript.events() {
            match event.kind() {
                RunEventKind::StageEntered { stage } => {
                    let receipt = &mut stages[stage_index(*stage)];
                    receipt.entered_event = Some(event.id());
                }
                RunEventKind::StageReached { stage } => {
                    let receipt = &mut stages[stage_index(*stage)];
                    receipt.status = StageStatus::Reached;
                    receipt.terminal_event = Some(event.id());
                }
                RunEventKind::Refused {
                    stage: Some(stage),
                    refusals: event_refusals,
                } => {
                    let receipt = &mut stages[stage_index(*stage)];
                    receipt.status = StageStatus::Refused;
                    receipt.terminal_event = Some(event.id());
                    refusals.extend(event_refusals.iter().cloned());
                }
                RunEventKind::Refused {
                    stage: None,
                    refusals: event_refusals,
                } => refusals.extend(event_refusals.iter().cloned()),
                _ => {}
            }
        }
        Self {
            absolute_floor_entries,
            stages,
            refusals,
            transcript,
        }
    }

    pub const fn absolute_floor_entries(&self) -> usize {
        self.absolute_floor_entries
    }

    pub fn stages(&self) -> &[StageReceipt] {
        &self.stages
    }

    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }

    pub const fn transcript(&self) -> &RunTranscript {
        &self.transcript
    }

    pub fn realization_id(&self) -> Option<&RealizationId> {
        self.transcript.realization_id()
    }

    pub fn is_complete(&self) -> bool {
        self.transcript.representation().status() == RepresentationStatus::Available
            && self.refusals.is_empty()
            && self
                .stages
                .iter()
                .all(|stage| stage.status == StageStatus::Reached)
    }
}

fn stage_index(stage: Stage) -> usize {
    Stage::ALL
        .iter()
        .position(|candidate| *candidate == stage)
        .expect("every canonical stage has an index")
}

impl fmt::Display for RunReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "receipt=civsim.planet.run.v12")?;
        writeln!(f, "complete={}", self.is_complete())?;
        writeln!(
            f,
            "generated.written.realization_id={}",
            self.realization_id()
                .map(RealizationId::as_str)
                .unwrap_or("<none>")
        )?;
        writeln!(
            f,
            "generated.contingency.count={}",
            self.transcript.contingency_draws().count()
        )?;
        writeln!(f, "absolute_floor_entries={}", self.absolute_floor_entries)?;
        for stage in &self.stages {
            writeln!(f, "stage.{}={}", stage.stage.id(), stage.status.id())?;
            writeln!(
                f,
                "stage.{}.entered_event={}",
                stage.stage.id(),
                stage
                    .entered_event
                    .map(|event| event.to_string())
                    .unwrap_or_else(|| "<none>".into())
            )?;
            writeln!(
                f,
                "stage.{}.terminal_event={}",
                stage.stage.id(),
                stage
                    .terminal_event
                    .map(|event| event.to_string())
                    .unwrap_or_else(|| "<none>".into())
            )?;
        }
        writeln!(f, "refusal_count={}", self.refusals.len())?;
        for (index, refusal) in self.refusals.iter().enumerate() {
            let prefix = format!("refusal.{index:04}");
            writeln!(f, "{prefix}.code={}", refusal.code.id())?;
            writeln!(
                f,
                "{prefix}.requirement={}",
                canonical_text(refusal.requirement_id().unwrap_or(""))
            )?;
            write_open_requirements(f, &prefix, &refusal.open_requirements)?;
            writeln!(f, "{prefix}.detail={}", canonical_text(&refusal.detail))?;
        }
        write!(f, "{}", self.transcript)
    }
}

pub(super) fn write_open_requirements(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    requirements: &[OpenRequirement],
) -> fmt::Result {
    writeln!(f, "{prefix}.open_requirement_count={}", requirements.len())?;
    for (open_index, requirement) in requirements.iter().enumerate() {
        let open_prefix = format!("{prefix}.open_requirement.{open_index:04}");
        writeln!(
            f,
            "{open_prefix}.id={}",
            canonical_text(requirement.requirement_id())
        )?;
        writeln!(
            f,
            "{open_prefix}.obligation_count={}",
            requirement.obligations().len()
        )?;
        for (obligation_index, obligation) in requirement.obligations().iter().enumerate() {
            writeln!(
                f,
                "{open_prefix}.obligation.{obligation_index:04}={}",
                canonical_text(obligation)
            )?;
        }
        writeln!(
            f,
            "{open_prefix}.analysis_count={}",
            requirement.analyses().len()
        )?;
        for (analysis_index, analysis) in requirement.analyses().iter().enumerate() {
            write_requirement_analysis(
                f,
                &format!("{open_prefix}.analysis.{analysis_index:04}"),
                analysis,
            )?;
        }
    }
    Ok(())
}

fn write_requirement_analysis(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    analysis: &RequirementAnalysis,
) -> fmt::Result {
    writeln!(f, "{prefix}.kind={}", analysis.kind_id())?;
    writeln!(
        f,
        "{prefix}.schema={}",
        canonical_text(analysis.schema_id())
    )?;
    writeln!(
        f,
        "{prefix}.checker={}",
        canonical_text(analysis.checker_id())
    )?;
    writeln!(f, "{prefix}.status={}", analysis.status_id())?;
    writeln!(
        f,
        "{prefix}.closure_effect={}",
        analysis.closure_effect_id()
    )?;
    writeln!(f, "{prefix}.coverage_claim={}", analysis.coverage_claim())?;
    match &analysis.payload {
        RequirementAnalysisPayload::ExactDimensionalCensus(census) => {
            write_dimensional_census(f, prefix, census)
        }
        RequirementAnalysisPayload::SpeciesStateDerivation(analysis) => {
            write_species_derivation_analysis(f, prefix, analysis)
        }
    }
}

fn write_dimensional_census(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    artifact: &StellarBirthDimensionalCensusArtifact,
) -> fmt::Result {
    use super::stellar_birth_dimensions::StellarBirthDimensionalCensusArtifact::{
        Computed, Invalid,
    };

    match artifact {
        Invalid(census) => {
            writeln!(f, "{prefix}.error.code={}", census.error_code)?;
            writeln!(
                f,
                "{prefix}.error.detail={}",
                canonical_text(&census.detail)
            )
        }
        Computed(census) => {
            writeln!(
                f,
                "{prefix}.representation_schema={}",
                canonical_text(census.representation_schema_id)
            )?;
            writeln!(
                f,
                "{prefix}.floor_binding.schema={}",
                canonical_text(census.floor_binding_schema_id)
            )?;
            writeln!(
                f,
                "{prefix}.floor_binding.sha256={}",
                census.floor_binding_sha256
            )?;
            writeln!(
                f,
                "{prefix}.base_dimension_count={}",
                census.base_dimension_ids.len()
            )?;
            for (index, id) in census.base_dimension_ids.iter().enumerate() {
                writeln!(
                    f,
                    "{prefix}.base_dimension.{index:04}={}",
                    canonical_text(id)
                )?;
            }
            write_stellar_birth_structure(f, prefix, &census.structure)?;
            writeln!(f, "{prefix}.variable_count={}", census.variables.len())?;
            for (index, variable) in census.variables.iter().enumerate() {
                let variable_prefix = format!("{prefix}.variable.{index:04}");
                writeln!(f, "{variable_prefix}.id={}", canonical_text(&variable.id))?;
                writeln!(f, "{variable_prefix}.role={}", variable.role.id())?;
                writeln!(f, "{variable_prefix}.carrier={}", variable.carrier.id())?;
                writeln!(
                    f,
                    "{variable_prefix}.coupling_group={}",
                    canonical_text(&variable.coupling_group_id)
                )?;
                write_dimension(f, &variable_prefix, variable.dimension)?;
            }
            writeln!(f, "{prefix}.phenomenon_count={}", census.phenomena.len())?;
            for (index, phenomenon) in census.phenomena.iter().enumerate() {
                let phenomenon_prefix = format!("{prefix}.phenomenon.{index:04}");
                writeln!(
                    f,
                    "{phenomenon_prefix}.id={}",
                    canonical_text(&phenomenon.phenomenon_id)
                )?;
                writeln!(
                    f,
                    "{phenomenon_prefix}.coverage_complete={}",
                    phenomenon.coverage_complete
                )?;
                writeln!(
                    f,
                    "{phenomenon_prefix}.matrix.orientation={}",
                    canonical_text(phenomenon.matrix_orientation)
                )?;
                write_string_list(f, &phenomenon_prefix, "input", &phenomenon.input_ids)?;
                write_string_list(f, &phenomenon_prefix, "output", &phenomenon.output_ids)?;
                writeln!(
                    f,
                    "{phenomenon_prefix}.matrix.column_count={}",
                    phenomenon.matrix_columns.len()
                )?;
                for (column_index, column) in phenomenon.matrix_columns.iter().enumerate() {
                    let column_prefix =
                        format!("{phenomenon_prefix}.matrix.column.{column_index:04}");
                    writeln!(f, "{column_prefix}.id={}", canonical_text(&column.id))?;
                    write_dimension(f, &column_prefix, column.dimension)?;
                }
                writeln!(f, "{phenomenon_prefix}.rank={}", phenomenon.rank)?;
                writeln!(f, "{phenomenon_prefix}.nullity={}", phenomenon.nullity())?;
                write_index_list(
                    f,
                    &phenomenon_prefix,
                    "pivot_column",
                    &phenomenon.pivot_columns,
                )?;
                write_index_list(
                    f,
                    &phenomenon_prefix,
                    "free_column",
                    &phenomenon.free_columns,
                )?;
                writeln!(
                    f,
                    "{phenomenon_prefix}.null_vector_count={}",
                    phenomenon.null_space_basis.len()
                )?;
                for (vector_index, vector) in phenomenon.null_space_basis.iter().enumerate() {
                    let vector_prefix =
                        format!("{phenomenon_prefix}.null_vector.{vector_index:04}");
                    writeln!(f, "{vector_prefix}.coefficient_count={}", vector.len())?;
                    for (coefficient_index, coefficient) in vector.iter().enumerate() {
                        writeln!(
                            f,
                            "{vector_prefix}.coefficient.{coefficient_index:04}={coefficient}"
                        )?;
                    }
                }
                writeln!(
                    f,
                    "{phenomenon_prefix}.derivation_attempt_count={}",
                    phenomenon.derivation_attempts.len()
                )?;
                for (attempt_index, attempt) in phenomenon.derivation_attempts.iter().enumerate() {
                    let attempt_prefix =
                        format!("{phenomenon_prefix}.derivation_attempt.{attempt_index:04}");
                    writeln!(
                        f,
                        "{attempt_prefix}.id={}",
                        canonical_text(&attempt.attempt_id)
                    )?;
                    writeln!(
                        f,
                        "{attempt_prefix}.law_id={}",
                        canonical_text(&attempt.law_id)
                    )?;
                    writeln!(f, "{attempt_prefix}.status={}", attempt.status.id())?;
                    write_string_list(f, &attempt_prefix, "input", &attempt.input_ids)?;
                    writeln!(
                        f,
                        "{attempt_prefix}.output={}",
                        canonical_text(&attempt.output_id)
                    )?;
                    writeln!(
                        f,
                        "{attempt_prefix}.dimension_only_projection_count={}",
                        attempt.dimension_only_projection.len()
                    )?;
                    for (projection_index, exponent) in
                        attempt.dimension_only_projection.iter().enumerate()
                    {
                        let projection_prefix = format!(
                            "{attempt_prefix}.dimension_only_projection.{projection_index:04}"
                        );
                        writeln!(f, "{projection_prefix}.numerator={}", exponent.numerator)?;
                        writeln!(
                            f,
                            "{projection_prefix}.denominator={}",
                            exponent.denominator
                        )?;
                    }
                    write_string_list(
                        f,
                        &attempt_prefix,
                        "dimension_only_support",
                        &attempt.dimension_only_support_ids,
                    )?;
                    write_string_list(
                        f,
                        &attempt_prefix,
                        "missing_dependency",
                        &attempt.missing_dependency_ids,
                    )?;
                    write_string_list(
                        f,
                        &attempt_prefix,
                        "dropped_mechanism",
                        &attempt.dropped_mechanism_ids,
                    )?;
                }
            }
            write_string_list(f, prefix, "coverage_gap", &census.coverage_gap_ids)
        }
    }
}

fn write_dimension(f: &mut fmt::Formatter<'_>, prefix: &str, dimension: [i8; 7]) -> fmt::Result {
    let [length, mass, time, current, temperature, amount, luminous_intensity] = dimension;
    writeln!(f, "{prefix}.dimension.length={length}")?;
    writeln!(f, "{prefix}.dimension.mass={mass}")?;
    writeln!(f, "{prefix}.dimension.time={time}")?;
    writeln!(f, "{prefix}.dimension.current={current}")?;
    writeln!(f, "{prefix}.dimension.temperature={temperature}")?;
    writeln!(f, "{prefix}.dimension.amount={amount}")?;
    writeln!(
        f,
        "{prefix}.dimension.luminous_intensity={luminous_intensity}"
    )
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

fn write_index_list(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    field: &str,
    values: &[usize],
) -> fmt::Result {
    writeln!(f, "{prefix}.{field}_count={}", values.len())?;
    for (index, value) in values.iter().enumerate() {
        writeln!(f, "{prefix}.{field}.{index:04}={value}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stage_refusal_marks_only_the_stage_that_was_entered() {
        let receipt = RunReceipt::refused_at(
            3,
            Stage::StarDiskSystem,
            vec![Refusal::missing_stage_requirement(
                Stage::StarDiskSystem,
                "stellar_birth.realization_measure",
            )],
        );

        assert_eq!(receipt.stages()[0].status(), StageStatus::Refused);
        assert!(receipt.stages()[1..]
            .iter()
            .all(|stage| stage.status() == StageStatus::NotReached));
        assert!(receipt.stages()[0].entered_event().is_some());
        assert!(receipt.stages()[0].terminal_event().is_some());
        assert!(!receipt.is_complete());
    }

    #[test]
    fn a_later_stage_refusal_preserves_concrete_prior_stage_events() {
        let receipt = RunReceipt::refused_at(
            3,
            Stage::GeodynamicsDeepTime,
            vec![Refusal::missing_stage_requirement(
                Stage::GeodynamicsDeepTime,
                "deep_time.physical_termination",
            )],
        );

        assert!(receipt.stages()[..4]
            .iter()
            .all(|stage| stage.status() == StageStatus::Reached));
        assert_eq!(receipt.stages()[4].status(), StageStatus::Refused);
        assert!(receipt.stages()[5..]
            .iter()
            .all(|stage| stage.status() == StageStatus::NotReached));
        assert_eq!(
            receipt
                .transcript()
                .events()
                .iter()
                .filter(|event| matches!(event.kind(), RunEventKind::StageEntered { .. }))
                .count(),
            5
        );
    }

    #[test]
    fn unavailable_representation_is_a_stable_visible_value_free_refusal() {
        let detail = "SI representation projection failed: injected receipt failure";
        let first = RunReceipt::representation_unavailable(3, detail.to_owned());
        let second = RunReceipt::representation_unavailable(3, detail.to_owned());

        assert_eq!(first, second);
        assert_eq!(first.to_string(), second.to_string());
        assert!(!first.is_complete());
        assert_eq!(first.refusals().len(), 1);
        assert_eq!(
            first.refusals()[0].code(),
            RefusalCode::RepresentationUnavailable
        );
        assert_eq!(
            first.refusals()[0].requirement_id(),
            Some("repository.si_representation_projection")
        );
        assert_eq!(first.refusals()[0].detail(), detail);
        assert_eq!(
            first.transcript().representation().status(),
            RepresentationStatus::RepresentationUnavailable
        );
        assert!(first.transcript().representation().values().is_empty());
        assert_eq!(first.transcript().events().len(), 1);
        assert!(matches!(
            first.transcript().events()[0].kind(),
            RunEventKind::Refused {
                stage: None,
                refusals,
            } if refusals.as_slice() == first.refusals()
        ));

        let text = first.to_string();
        assert!(text.contains("refusal.0000.code=representation_unavailable\n"));
        assert!(text.contains(
            "refusal.0000.detail=\"SI representation projection failed: injected receipt failure\"\n"
        ));
        assert!(text.contains(
            "event.0000.reason.0000.detail=\"SI representation projection failed: injected receipt failure\"\n"
        ));
        assert!(text.contains(
            "representation.error=\"SI representation projection failed: injected receipt failure\"\n"
        ));
        assert!(!text.contains("representation.value.0000"));
        assert!(!text.contains(".kind=floor_value\n"));
    }

    #[test]
    fn unavailable_representation_preserves_known_structural_refusals() {
        let representation_detail =
            "SI representation projection failed: injected receipt initialization failure";
        let structural_detail = "injected absolute-floor catalog mismatch";
        let build = || {
            RunReceipt::refused_after_transcript_initialization(
                3,
                Err(TranscriptError::RepresentationUnavailable {
                    detail: representation_detail.to_owned(),
                }),
                vec![Refusal::floor_catalog_mismatch(
                    structural_detail.to_owned(),
                )],
            )
        };

        let first = build();
        let second = build();
        assert_eq!(first, second);
        assert_eq!(
            first
                .refusals()
                .iter()
                .map(Refusal::code)
                .collect::<Vec<_>>(),
            vec![
                RefusalCode::FloorCatalogMismatch,
                RefusalCode::RepresentationUnavailable,
            ]
        );
        assert_eq!(first.refusals()[0].detail(), structural_detail);
        assert_eq!(first.refusals()[1].detail(), representation_detail);
        assert_eq!(first.transcript().events().len(), 1);
        assert!(matches!(
            first.transcript().events()[0].kind(),
            RunEventKind::Refused {
                stage: None,
                refusals,
            } if refusals.as_slice() == first.refusals()
        ));
        assert!(first.transcript().representation().values().is_empty());
        assert!(first.transcript().events().iter().all(|event| !matches!(
            event.kind(),
            RunEventKind::FloorValue(_) | RunEventKind::DerivedValue(_)
        )));

        let text = first.to_string();
        assert!(text.contains("refusal_count=2\n"));
        assert!(text.contains("refusal.0000.code=floor_catalog_mismatch\n"));
        assert!(text.contains("refusal.0001.code=representation_unavailable\n"));
        assert!(text.contains("event.0000.reason.0000.code=floor_catalog_mismatch\n"));
        assert!(text.contains("event.0000.reason.0001.code=representation_unavailable\n"));
        assert!(text.contains("representation.value_count=0\n"));
        assert!(!text.contains("representation.value.0000"));
    }
}
