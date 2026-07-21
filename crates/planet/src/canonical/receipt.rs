use super::{
    transcript::canonical_text, EventId, RealizationId, RunEventKind, RunTranscript, Stage,
    TranscriptError,
};
use std::{cmp::Ordering, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefusalCode {
    AbsoluteFloorRequired,
    FloorCatalogMismatch,
    FloorMagnitudeUnavailable,
    MissingStageRequirement,
    PipelineIncomplete,
    TranscriptInvariantViolation,
}

impl RefusalCode {
    pub const fn id(self) -> &'static str {
        match self {
            Self::AbsoluteFloorRequired => "absolute_floor_required",
            Self::FloorCatalogMismatch => "floor_catalog_mismatch",
            Self::FloorMagnitudeUnavailable => "floor_magnitude_unavailable",
            Self::MissingStageRequirement => "missing_stage_requirement",
            Self::PipelineIncomplete => "pipeline_incomplete",
            Self::TranscriptInvariantViolation => "transcript_invariant_violation",
        }
    }
}

/// Structured reason a canonical run stopped.
///
/// `detail` is a readable explanation. Verification uses `code`, `stage`, and
/// `requirement_id` rather than parsing that prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    code: RefusalCode,
    stage: Option<Stage>,
    requirement_id: Option<String>,
    detail: String,
}

impl Refusal {
    pub(crate) fn absolute_floor_required() -> Self {
        Self {
            code: RefusalCode::AbsoluteFloorRequired,
            stage: None,
            requirement_id: Some("absolute_physics_floor".into()),
            detail: "run_planet requires a validated absolute physics floor".into(),
        }
    }

    pub(crate) fn floor_catalog_mismatch(detail: String) -> Self {
        Self {
            code: RefusalCode::FloorCatalogMismatch,
            stage: None,
            requirement_id: Some("repository.absolute_floor_catalog".into()),
            detail,
        }
    }

    pub(crate) fn floor_magnitude_unavailable(detail: String) -> Self {
        Self {
            code: RefusalCode::FloorMagnitudeUnavailable,
            stage: None,
            requirement_id: Some("repository.absolute_floor_magnitudes".into()),
            detail,
        }
    }

    pub(crate) fn missing_stage_requirement(stage: Stage, requirement_id: &str) -> Self {
        Self {
            code: RefusalCode::MissingStageRequirement,
            stage: Some(stage),
            requirement_id: Some(requirement_id.to_owned()),
            detail: format!(
                "stage '{}' requires derived or admitted absolute-floor measure '{requirement_id}'",
                stage.id()
            ),
        }
    }

    pub(crate) fn pipeline_incomplete(stage: Stage, detail: &str) -> Self {
        Self {
            code: RefusalCode::PipelineIncomplete,
            stage: Some(stage),
            requirement_id: Some(format!("stage.{}.physical_closure", stage.id())),
            detail: detail.to_owned(),
        }
    }

    pub(crate) fn transcript_invariant(detail: String) -> Self {
        Self {
            code: RefusalCode::TranscriptInvariantViolation,
            stage: None,
            requirement_id: Some("canonical.run_transcript".into()),
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
        let mut transcript = RunTranscript::empty(absolute_floor_entries);
        transcript
            .refuse(None, refusals)
            .expect("a nonempty preflight refusal closes an empty transcript");
        Self::from_transcript(absolute_floor_entries, transcript)
    }

    /// Build a refusal after the canonical run has entered one physical stage.
    #[cfg(test)]
    pub(crate) fn refused_at(
        absolute_floor_entries: usize,
        stage: Stage,
        refusals: Vec<Refusal>,
    ) -> Self {
        let mut transcript = RunTranscript::empty(absolute_floor_entries);
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
        self.refusals.is_empty()
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
        writeln!(f, "receipt=civsim.planet.run.v5")?;
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
        for refusal in &self.refusals {
            writeln!(
                f,
                "refusal.{}.requirement={}",
                refusal.code.id(),
                canonical_text(refusal.requirement_id().unwrap_or(""))
            )?;
            writeln!(
                f,
                "refusal.{}.detail={}",
                refusal.code.id(),
                canonical_text(&refusal.detail)
            )?;
        }
        write!(f, "{}", self.transcript)
    }
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
}
