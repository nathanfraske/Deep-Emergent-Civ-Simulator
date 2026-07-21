//! Canonical, append-only provenance transcript for one planet run.
//!
//! The transcript is a readable record stream rather than an opaque digest.
//! Its current producer serializes the noncausal SI representation contract,
//! every measured magnitude in the sealed physical floor, and each causal
//! execution derivation before concrete stage events. Later stage adapters
//! extend the same stream with generated contingency, written state, body
//! lineage, and exact conservation transfers.

use super::{
    accounting::{ExactScaledValue, ExactValueRecord, LawAncestry, TransferReceipt},
    floor_magnitudes::AuditedFloorView,
    receipt::Refusal,
    BodyId, ContingencyDrawId, EventId, RealizationId, ReservoirId, Stage,
};
use civsim_ledger::{AbsolutePhysicsFloor, Provenance, Tier};
use civsim_units::constants::si_representation_magnitudes;
use civsim_units::fundamentals::{
    ATOMIC_VOLUME_CONVERSION, GAS_CONSTANT, REPRESENTATION_DEFINITIONS, SI_BASE_DIMENSION_IDS,
    SI_REPRESENTATION_SCHEMA_ID, STEFAN_BOLTZMANN, VACUUM_PERMITTIVITY,
};
use civsim_units::physics_floor::PHYSICAL_FLOOR_LEN;
use std::{fmt, num::TryFromIntError};

/// Stable schema identity for the first concrete transcript format.
pub const RUN_TRANSCRIPT_SCHEMA_ID: &str = "civsim.planet.transcript.v3";

pub(super) fn canonical_text(value: &str) -> CanonicalText<'_> {
    CanonicalText(value)
}

pub(super) struct CanonicalText<'a>(&'a str);

impl fmt::Display for CanonicalText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        for character in self.0.chars() {
            match character {
                '\"' => f.write_str("\\\"")?,
                '\\' => f.write_str("\\\\")?,
                '\n' => f.write_str("\\n")?,
                '\r' => f.write_str("\\r")?,
                '\t' => f.write_str("\\t")?,
                control if control.is_control() => {
                    write!(f, "\\u{{{:06X}}}", u32::from(control))?;
                }
                printable => write!(f, "{printable}")?,
            }
        }
        f.write_str("\"")
    }
}

/// Explicit transcript format identity and compatibility version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscriptSchema {
    id: &'static str,
    major: u16,
    minor: u16,
}

impl TranscriptSchema {
    pub const V3: Self = Self {
        id: RUN_TRANSCRIPT_SCHEMA_ID,
        major: 3,
        minor: 0,
    };

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub const fn major(self) -> u16 {
        self.major
    }

    pub const fn minor(self) -> u16 {
        self.minor
    }
}

/// One noncausal value in the versioned SI representation contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationValueRecord {
    kind_id: &'static str,
    symbol: &'static str,
    unit: &'static str,
    dimension: civsim_units::fundamentals::SiDimension,
    value: ExactScaledValue,
    source_id: Option<&'static str>,
    source_sha256: Option<&'static str>,
    source_anchor: Option<&'static str>,
    source_decimal: Option<&'static str>,
    formula: Option<&'static str>,
    inputs: Vec<&'static str>,
}

impl RepresentationValueRecord {
    pub const fn kind_id(&self) -> &'static str {
        self.kind_id
    }

    pub const fn symbol(&self) -> &'static str {
        self.symbol
    }

    pub const fn value(&self) -> ExactScaledValue {
        self.value
    }
}

/// Complete representation receipt required to replay physical quantities.
/// It deliberately has no tier or provenance field because SI definitions are
/// conventions, not causal facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationReceipt {
    schema_id: &'static str,
    values: Vec<RepresentationValueRecord>,
}

impl RepresentationReceipt {
    fn sealed() -> Result<Self, TranscriptError> {
        let magnitudes = si_representation_magnitudes().map_err(|error| {
            TranscriptError::FloorShapeMismatch {
                detail: format!("SI representation projection failed: {error}"),
            }
        })?;
        let mut values = Vec::with_capacity(REPRESENTATION_DEFINITIONS.len() + 3);
        for definition in REPRESENTATION_DEFINITIONS {
            let value = magnitudes.get(definition.symbol).ok_or_else(|| {
                TranscriptError::FloorShapeMismatch {
                    detail: format!(
                        "SI representation definition '{}' has no projected value",
                        definition.symbol
                    ),
                }
            })?;
            values.push(RepresentationValueRecord {
                kind_id: "exact_definition",
                symbol: definition.symbol,
                unit: definition.unit,
                dimension: definition.dimension,
                value: ExactScaledValue::from_repository_constant(value),
                source_id: Some(definition.source_id),
                source_sha256: Some(definition.source_sha256),
                source_anchor: Some(definition.source_anchor),
                source_decimal: Some(definition.value),
                formula: None,
                inputs: Vec::new(),
            });
        }
        for derived in [STEFAN_BOLTZMANN, GAS_CONSTANT, ATOMIC_VOLUME_CONVERSION] {
            let value = magnitudes.get(derived.symbol).ok_or_else(|| {
                TranscriptError::FloorShapeMismatch {
                    detail: format!(
                        "SI representation derivation '{}' has no projected value",
                        derived.symbol
                    ),
                }
            })?;
            values.push(RepresentationValueRecord {
                kind_id: "derived_definition",
                symbol: derived.symbol,
                unit: derived.unit,
                dimension: derived.dimension,
                value: ExactScaledValue::from_repository_constant(value),
                source_id: None,
                source_sha256: None,
                source_anchor: None,
                source_decimal: None,
                formula: Some(derived.formula),
                inputs: derived.fundamentals.to_vec(),
            });
        }
        Ok(Self {
            schema_id: SI_REPRESENTATION_SCHEMA_ID,
            values,
        })
    }

    pub const fn schema_id(&self) -> &'static str {
        self.schema_id
    }

    pub fn values(&self) -> &[RepresentationValueRecord] {
        &self.values
    }
}

/// The subject of a generated `[W]` value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GeneratedSubjectId {
    Realization(RealizationId),
    Body(BodyId),
    Reservoir(ReservoirId),
}

impl GeneratedSubjectId {
    /// Stable transcript variant name, independent of Rust debug formatting.
    pub const fn kind_id(&self) -> &'static str {
        match self {
            Self::Realization(_) => "realization",
            Self::Body(_) => "body",
            Self::Reservoir(_) => "reservoir",
        }
    }

    /// Opaque generated identity carried by this subject.
    pub fn identity(&self) -> &str {
        match self {
            Self::Realization(realization) => realization.as_str(),
            Self::Body(body) => body.as_str(),
            Self::Reservoir(reservoir) => reservoir.as_str(),
        }
    }
}

/// One `[X]` draw generated from a named admitted physical measure.
///
/// The record carries a measure, sampler law, and draw coordinate. It has no
/// caller seed field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContingencyEvent {
    stage: Stage,
    measure_id: String,
    sampler: LawAncestry,
    draw_coordinate: u64,
    outcome: ExactValueRecord,
    tier: Tier,
    provenance: Provenance,
}

impl ContingencyEvent {
    pub const fn stage(&self) -> Stage {
        self.stage
    }

    pub fn measure_id(&self) -> &str {
        &self.measure_id
    }

    pub const fn sampler(&self) -> &LawAncestry {
        &self.sampler
    }

    pub const fn draw_coordinate(&self) -> u64 {
        self.draw_coordinate
    }

    pub const fn outcome(&self) -> &ExactValueRecord {
        &self.outcome
    }

    pub const fn tier(&self) -> Tier {
        self.tier
    }

    pub const fn provenance(&self) -> Provenance {
        self.provenance
    }
}

/// Concrete payload of a generated `[W]` event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WrittenStateRecord {
    RealizationIdentity {
        identity: RealizationId,
        source_draw: ContingencyDrawId,
    },
    BodyBirth {
        body: BodyId,
        parents: Vec<BodyId>,
        law: LawAncestry,
        input_events: Vec<EventId>,
    },
    Value {
        subject: GeneratedSubjectId,
        field_id: String,
        value: Box<ExactValueRecord>,
        law: LawAncestry,
        input_events: Vec<EventId>,
    },
}

/// One generated `[W]` history record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenStateEvent {
    stage: Stage,
    tier: Tier,
    provenance: Provenance,
    record: WrittenStateRecord,
}

impl WrittenStateEvent {
    pub const fn stage(&self) -> Stage {
        self.stage
    }

    pub const fn tier(&self) -> Tier {
        self.tier
    }

    pub const fn provenance(&self) -> Provenance {
        self.provenance
    }

    pub const fn record(&self) -> &WrittenStateRecord {
        &self.record
    }
}

/// Concrete event kinds carried by the transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEventKind {
    FloorValue(ExactValueRecord),
    DerivedValue(ExactValueRecord),
    StageEntered {
        stage: Stage,
    },
    StageReached {
        stage: Stage,
    },
    Contingency(ContingencyEvent),
    WrittenState(WrittenStateEvent),
    ConservationTransfer(TransferReceipt),
    Refused {
        stage: Option<Stage>,
        refusals: Vec<Refusal>,
    },
}

impl RunEventKind {
    pub const fn id(&self) -> &'static str {
        match self {
            Self::FloorValue(_) => "floor_value",
            Self::DerivedValue(_) => "derived_value",
            Self::StageEntered { .. } => "stage_entered",
            Self::StageReached { .. } => "stage_reached",
            Self::Contingency(_) => "contingency",
            Self::WrittenState(_) => "written_state",
            Self::ConservationTransfer(_) => "conservation_transfer",
            Self::Refused { .. } => "refused",
        }
    }
}

/// One event at a stable ordinal in a [`RunTranscript`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunEvent {
    id: EventId,
    kind: RunEventKind,
}

impl RunEvent {
    pub const fn id(&self) -> EventId {
        self.id
    }

    pub const fn kind(&self) -> &RunEventKind {
        &self.kind
    }

    /// Generated draw identity when this event is a contingency record.
    pub fn contingency_draw_id(&self) -> Option<ContingencyDrawId> {
        matches!(self.kind, RunEventKind::Contingency(_))
            .then(|| ContingencyDrawId::generated(self.id))
    }
}

/// One complete append-only causal record through completion or refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTranscript {
    schema: TranscriptSchema,
    representation: RepresentationReceipt,
    declared_floor_entries: usize,
    events: Vec<RunEvent>,
    next_stage_index: usize,
    active_stage: Option<Stage>,
    closed: bool,
}

impl RunTranscript {
    pub(super) fn empty(declared_floor_entries: usize) -> Self {
        Self {
            schema: TranscriptSchema::V3,
            representation: RepresentationReceipt::sealed()
                .expect("the sealed SI representation must project"),
            declared_floor_entries,
            events: Vec::new(),
            next_stage_index: 0,
            active_stage: None,
            closed: false,
        }
    }

    /// Build the exact structural and magnitude record for the sealed floor.
    pub(super) fn from_audited_floor(
        floor: &AbsolutePhysicsFloor,
        view: &AuditedFloorView<'_>,
    ) -> Result<Self, TranscriptError> {
        let mut transcript = Self::empty(floor.len());
        let entries: Vec<_> = floor.entries().collect();
        if entries.len() != PHYSICAL_FLOOR_LEN {
            return Err(TranscriptError::FloorShapeMismatch {
                detail: format!(
                    "audited floor has {} entries but the sealed physical-invariant table has {}",
                    entries.len(),
                    PHYSICAL_FLOOR_LEN
                ),
            });
        }

        let definition = |symbol| {
            view.execution
                .physical_invariant_definition(symbol)
                .ok_or_else(|| TranscriptError::FloorShapeMismatch {
                    detail: format!(
                        "sealed execution capability has no physical metadata for '{symbol}'"
                    ),
                })
        };
        let physical_values = [
            (
                *definition("alpha")?,
                view.magnitudes.fine_structure.symbol(),
                ExactScaledValue::from_audited(view.magnitudes.fine_structure),
            ),
            (
                *definition("G")?,
                view.magnitudes.gravitational_constant.symbol(),
                ExactScaledValue::from_audited(view.magnitudes.gravitational_constant),
            ),
            (
                *definition("m_e")?,
                view.magnitudes.electron_mass.symbol(),
                ExactScaledValue::from_audited(view.magnitudes.electron_mass),
            ),
        ];

        for (index, (constant, bound_symbol, value)) in physical_values.into_iter().enumerate() {
            let entry = entries[index];
            let expected_id = format!("fundamental.{}", constant.symbol);
            if entry.id != expected_id
                || bound_symbol != constant.symbol
                || entry.tier != Tier::Universal
                || entry.provenance != Provenance::Measured
                || !entry.inputs.is_empty()
            {
                return Err(TranscriptError::FloorShapeMismatch {
                    detail: format!(
                        "floor entry {index} '{}' does not match sealed measured identity '{expected_id}'",
                        entry.id
                    ),
                });
            }
            let exhaustion =
                floor
                    .receipt(&entry.id)
                    .ok_or_else(|| TranscriptError::FloorShapeMismatch {
                        detail: format!(
                            "floor entry '{}' has no derive-first exhaustion receipt",
                            entry.id
                        ),
                    })?;
            transcript.append(RunEventKind::FloorValue(
                ExactValueRecord::sealed_measured_floor(
                    entry.id.clone(),
                    &constant,
                    value,
                    entry.tier,
                    entry.provenance,
                    exhaustion,
                ),
            ))?;
        }

        let eps0_inputs = vec![
            "representation.e".to_owned(),
            "fundamental.alpha".to_owned(),
            "representation.h".to_owned(),
            "representation.c".to_owned(),
        ];
        let eps0_ancestry = LawAncestry::sealed_execution_derivation(
            "units.execution.eps_0.definition".into(),
            VACUUM_PERMITTIVITY.formula,
            eps0_inputs,
        );
        transcript.append(RunEventKind::DerivedValue(
            ExactValueRecord::sealed_derived_value(
                "derived.eps_0".into(),
                &VACUUM_PERMITTIVITY,
                ExactScaledValue::from_audited(view.execution.vacuum_permittivity),
                Tier::Universal,
                Provenance::Derived,
                eps0_ancestry,
            ),
        ))?;

        Ok(transcript)
    }

    /// Schema governing the readable record stream.
    pub const fn schema(&self) -> TranscriptSchema {
        self.schema
    }

    /// Versioned, noncausal SI encoding contract carried by this bitstream.
    pub const fn representation(&self) -> &RepresentationReceipt {
        &self.representation
    }

    /// Floor-entry count declared at the run boundary.
    pub const fn declared_floor_entries(&self) -> usize {
        self.declared_floor_entries
    }

    /// Events in their canonical append order.
    pub fn events(&self) -> &[RunEvent] {
        &self.events
    }

    /// Whether completion or a refusal has closed the stream.
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// Generated realization identity, when a prior `[X]` measure produced it.
    pub fn realization_id(&self) -> Option<&RealizationId> {
        self.events.iter().find_map(|event| match &event.kind {
            RunEventKind::WrittenState(WrittenStateEvent {
                record: WrittenStateRecord::RealizationIdentity { identity, .. },
                ..
            }) => Some(identity),
            _ => None,
        })
    }

    /// Contingency events in append order.
    pub fn contingency_draws(&self) -> impl Iterator<Item = ContingencyDrawId> + '_ {
        self.events.iter().filter_map(RunEvent::contingency_draw_id)
    }

    pub(super) fn enter_stage(&mut self, stage: Stage) -> Result<EventId, TranscriptError> {
        self.require_open()?;
        if let Some(active) = self.active_stage {
            return Err(TranscriptError::StageAlreadyActive { active });
        }
        let expected = Stage::ALL.get(self.next_stage_index).copied();
        if expected != Some(stage) {
            return Err(TranscriptError::OutOfOrderStage {
                expected,
                found: stage,
            });
        }
        let event = self.append(RunEventKind::StageEntered { stage })?;
        self.active_stage = Some(stage);
        Ok(event)
    }

    #[cfg(test)]
    pub(super) fn reach_active_stage(&mut self) -> Result<EventId, TranscriptError> {
        self.require_open()?;
        let stage = self
            .active_stage
            .ok_or(TranscriptError::ActiveStageRequired)?;
        let event = self.append(RunEventKind::StageReached { stage })?;
        self.active_stage = None;
        self.next_stage_index += 1;
        if self.next_stage_index == Stage::ALL.len() {
            self.closed = true;
        }
        Ok(event)
    }

    pub(super) fn refuse(
        &mut self,
        stage: Option<Stage>,
        mut refusals: Vec<Refusal>,
    ) -> Result<EventId, TranscriptError> {
        self.require_open()?;
        if refusals.is_empty() {
            return Err(TranscriptError::EmptyRefusal);
        }
        match (stage, self.active_stage) {
            (None, None) if self.next_stage_index == 0 => {}
            (Some(found), Some(active)) if found == active => {}
            (found, active) => {
                return Err(TranscriptError::RefusalStageMismatch { active, found });
            }
        }
        for refusal in &mut refusals {
            if let Err(found) = refusal.attach_stage(stage) {
                return Err(TranscriptError::RefusalContextMismatch {
                    expected: stage,
                    found,
                });
            }
        }
        refusals.sort_by(|left, right| left.canonical_cmp(right));
        let event = self.append(RunEventKind::Refused { stage, refusals })?;
        self.active_stage = None;
        self.closed = true;
        Ok(event)
    }

    /// Generate `[W]` realization identity only from a prior `[X]` event.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "armed for the first admitted physical contingency measure"
        )
    )]
    pub(super) fn record_realization(
        &mut self,
        source_draw: ContingencyDrawId,
    ) -> Result<RealizationId, TranscriptError> {
        self.require_open()?;
        let stage = self
            .active_stage
            .ok_or(TranscriptError::ActiveStageRequired)?;
        let source_index = usize::try_from(source_draw.event().ordinal())
            .map_err(TranscriptError::EventCountOverflow)?;
        if !matches!(
            self.events.get(source_index),
            Some(RunEvent {
                kind: RunEventKind::Contingency(_),
                ..
            })
        ) {
            return Err(TranscriptError::MissingContingencyMeasure { source_draw });
        }
        if self.realization_id().is_some() {
            return Err(TranscriptError::DuplicateRealizationIdentity);
        }
        let identity = RealizationId::generated(source_draw);
        self.append(RunEventKind::WrittenState(WrittenStateEvent {
            stage,
            tier: Tier::Contingency,
            provenance: Provenance::WrittenState,
            record: WrittenStateRecord::RealizationIdentity {
                identity: identity.clone(),
                source_draw,
            },
        }))?;
        Ok(identity)
    }

    fn require_open(&self) -> Result<(), TranscriptError> {
        if self.closed {
            Err(TranscriptError::Closed)
        } else {
            Ok(())
        }
    }

    fn append(&mut self, kind: RunEventKind) -> Result<EventId, TranscriptError> {
        self.require_open()?;
        let ordinal =
            u64::try_from(self.events.len()).map_err(TranscriptError::EventCountOverflow)?;
        let id = EventId::generated(ordinal);
        self.events.push(RunEvent { id, kind });
        Ok(id)
    }
}

/// Why a canonical transcript could not append a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptError {
    Closed,
    FloorShapeMismatch {
        detail: String,
    },
    OutOfOrderStage {
        expected: Option<Stage>,
        found: Stage,
    },
    StageAlreadyActive {
        active: Stage,
    },
    ActiveStageRequired,
    EmptyRefusal,
    RefusalStageMismatch {
        active: Option<Stage>,
        found: Option<Stage>,
    },
    RefusalContextMismatch {
        expected: Option<Stage>,
        found: Option<Stage>,
    },
    MissingContingencyMeasure {
        source_draw: ContingencyDrawId,
    },
    DuplicateRealizationIdentity,
    EventCountOverflow(TryFromIntError),
}

impl fmt::Display for TranscriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => f.write_str("the canonical transcript is already closed"),
            Self::FloorShapeMismatch { detail } => {
                write!(f, "sealed floor transcript mismatch: {detail}")
            }
            Self::OutOfOrderStage { expected, found } => match expected {
                Some(expected) => write!(
                    f,
                    "stage '{}' cannot enter before canonical stage '{}'",
                    found.id(),
                    expected.id()
                ),
                None => write!(f, "stage '{}' follows the completed pipeline", found.id()),
            },
            Self::StageAlreadyActive { active } => {
                write!(f, "stage '{}' is already active", active.id())
            }
            Self::ActiveStageRequired => {
                f.write_str("a generated event requires an active canonical stage")
            }
            Self::EmptyRefusal => f.write_str("a refusal event requires at least one reason"),
            Self::RefusalStageMismatch { active, found } => write!(
                f,
                "refusal stage {:?} does not match active stage {:?}",
                found.map(Stage::id),
                active.map(Stage::id)
            ),
            Self::RefusalContextMismatch { expected, found } => write!(
                f,
                "refusal reason stage {:?} does not match transcript stage {:?}",
                found.map(Stage::id),
                expected.map(Stage::id)
            ),
            Self::MissingContingencyMeasure { source_draw } => write!(
                f,
                "written realization identity requires recorded contingency draw '{source_draw}'"
            ),
            Self::DuplicateRealizationIdentity => {
                f.write_str("the transcript already contains a realization identity")
            }
            Self::EventCountOverflow(error) => write!(f, "event ordinal overflow: {error}"),
        }
    }
}

impl std::error::Error for TranscriptError {}

impl fmt::Display for RunTranscript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "transcript={}", self.schema.id())?;
        writeln!(f, "schema.major={}", self.schema.major())?;
        writeln!(f, "schema.minor={}", self.schema.minor())?;
        writeln!(
            f,
            "representation.schema={}",
            canonical_text(self.representation.schema_id())
        )?;
        writeln!(
            f,
            "representation.base_dimension_count={}",
            SI_BASE_DIMENSION_IDS.len()
        )?;
        for (index, id) in SI_BASE_DIMENSION_IDS.iter().enumerate() {
            writeln!(
                f,
                "representation.base_dimension.{index:04}={}",
                canonical_text(id)
            )?;
        }
        writeln!(
            f,
            "representation.value_count={}",
            self.representation.values().len()
        )?;
        for (index, value) in self.representation.values().iter().enumerate() {
            write_representation_value(f, &format!("representation.value.{index:04}"), value)?;
        }
        writeln!(f, "declared_floor_entries={}", self.declared_floor_entries)?;
        writeln!(f, "event_count={}", self.events.len())?;
        writeln!(f, "closed={}", self.closed)?;
        for (index, event) in self.events.iter().enumerate() {
            let prefix = format!("event.{index:04}");
            writeln!(f, "{prefix}.id={}", event.id)?;
            writeln!(f, "{prefix}.kind={}", event.kind.id())?;
            write_event(f, &prefix, &event.kind)?;
        }
        Ok(())
    }
}

fn write_representation_value(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    record: &RepresentationValueRecord,
) -> fmt::Result {
    writeln!(f, "{prefix}.kind={}", record.kind_id)?;
    writeln!(f, "{prefix}.symbol={}", canonical_text(record.symbol))?;
    writeln!(f, "{prefix}.unit={}", canonical_text(record.unit))?;
    let [length, mass, time, current, temperature, amount, luminous_intensity] =
        record.dimension.exponents();
    writeln!(f, "{prefix}.dimension.length={length}")?;
    writeln!(f, "{prefix}.dimension.mass={mass}")?;
    writeln!(f, "{prefix}.dimension.time={time}")?;
    writeln!(f, "{prefix}.dimension.current={current}")?;
    writeln!(f, "{prefix}.dimension.temperature={temperature}")?;
    writeln!(f, "{prefix}.dimension.amount={amount}")?;
    writeln!(
        f,
        "{prefix}.dimension.luminous_intensity={luminous_intensity}"
    )?;
    write_scaled(f, &format!("{prefix}.value"), record.value)?;
    writeln!(
        f,
        "{prefix}.source_id={}",
        canonical_text(record.source_id.unwrap_or(""))
    )?;
    writeln!(
        f,
        "{prefix}.source_sha256={}",
        canonical_text(record.source_sha256.unwrap_or(""))
    )?;
    writeln!(
        f,
        "{prefix}.source_anchor={}",
        canonical_text(record.source_anchor.unwrap_or(""))
    )?;
    writeln!(
        f,
        "{prefix}.source_decimal={}",
        canonical_text(record.source_decimal.unwrap_or(""))
    )?;
    writeln!(
        f,
        "{prefix}.formula={}",
        canonical_text(record.formula.unwrap_or(""))
    )?;
    writeln!(f, "{prefix}.input_count={}", record.inputs.len())?;
    for (index, input) in record.inputs.iter().enumerate() {
        writeln!(f, "{prefix}.input.{index:04}={}", canonical_text(input))?;
    }
    Ok(())
}

fn write_event(f: &mut fmt::Formatter<'_>, prefix: &str, kind: &RunEventKind) -> fmt::Result {
    match kind {
        RunEventKind::FloorValue(record) | RunEventKind::DerivedValue(record) => {
            write_value(f, prefix, record)
        }
        RunEventKind::StageEntered { stage } | RunEventKind::StageReached { stage } => {
            writeln!(f, "{prefix}.stage={}", stage.id())
        }
        RunEventKind::Contingency(event) => {
            writeln!(f, "{prefix}.stage={}", event.stage().id())?;
            writeln!(f, "{prefix}.tier={}", event.tier().id())?;
            writeln!(
                f,
                "{prefix}.provenance={}",
                event
                    .provenance()
                    .bracket_tag()
                    .expect("canonical provenance has a bracket tag")
            )?;
            writeln!(
                f,
                "{prefix}.measure_id={}",
                canonical_text(event.measure_id())
            )?;
            writeln!(f, "{prefix}.draw_coordinate={}", event.draw_coordinate())?;
            write_law(f, &format!("{prefix}.sampler"), event.sampler())?;
            write_value(f, &format!("{prefix}.outcome"), event.outcome())
        }
        RunEventKind::WrittenState(event) => {
            writeln!(f, "{prefix}.stage={}", event.stage().id())?;
            writeln!(f, "{prefix}.tier={}", event.tier().id())?;
            writeln!(
                f,
                "{prefix}.provenance={}",
                event
                    .provenance()
                    .bracket_tag()
                    .expect("canonical provenance has a bracket tag")
            )?;
            write_written_state(f, prefix, event.record())
        }
        RunEventKind::ConservationTransfer(receipt) => {
            writeln!(f, "{prefix}.stage={}", receipt.stage().id())?;
            writeln!(f, "{prefix}.tier={}", receipt.tier().id())?;
            writeln!(
                f,
                "{prefix}.provenance={}",
                receipt
                    .provenance()
                    .bracket_tag()
                    .expect("canonical provenance has a bracket tag")
            )?;
            writeln!(
                f,
                "{prefix}.quantity_id={}",
                canonical_text(receipt.quantity_id())
            )?;
            writeln!(f, "{prefix}.unit_id={}", canonical_text(receipt.unit_id()))?;
            writeln!(f, "{prefix}.operation={}", receipt.operation().id())?;
            write_transfer_legs(f, &format!("{prefix}.source"), receipt.sources())?;
            write_transfer_legs(f, &format!("{prefix}.destination"), receipt.destinations())?;
            let balance = receipt.balance();
            write_scaled(f, &format!("{prefix}.balance.before"), balance.before())?;
            write_scaled(f, &format!("{prefix}.balance.debited"), balance.debited())?;
            write_scaled(f, &format!("{prefix}.balance.credited"), balance.credited())?;
            write_scaled(
                f,
                &format!("{prefix}.balance.boundary_net"),
                balance.boundary_net(),
            )?;
            write_scaled(f, &format!("{prefix}.balance.after"), balance.after())?;
            write_scaled(f, &format!("{prefix}.balance.residual"), balance.residual())?;
            write_law(f, &format!("{prefix}.law"), receipt.law())?;
            write_event_ids(f, &format!("{prefix}.input_event"), receipt.input_events())
        }
        RunEventKind::Refused { stage, refusals } => {
            writeln!(
                f,
                "{prefix}.stage={}",
                stage.map(Stage::id).unwrap_or("<preflight>")
            )?;
            writeln!(f, "{prefix}.reason_count={}", refusals.len())?;
            for (index, refusal) in refusals.iter().enumerate() {
                let refusal_prefix = format!("{prefix}.reason.{index:04}");
                writeln!(f, "{refusal_prefix}.code={}", refusal.code().id())?;
                writeln!(
                    f,
                    "{refusal_prefix}.stage={}",
                    refusal.stage().map(Stage::id).unwrap_or("<preflight>")
                )?;
                writeln!(
                    f,
                    "{refusal_prefix}.requirement_id={}",
                    canonical_text(refusal.requirement_id().unwrap_or(""))
                )?;
                writeln!(
                    f,
                    "{refusal_prefix}.detail={}",
                    canonical_text(refusal.detail())
                )?;
            }
            Ok(())
        }
    }
}

fn write_value(f: &mut fmt::Formatter<'_>, prefix: &str, record: &ExactValueRecord) -> fmt::Result {
    writeln!(
        f,
        "{prefix}.quantity_id={}",
        canonical_text(record.quantity_id())
    )?;
    writeln!(f, "{prefix}.unit_id={}", canonical_text(record.unit_id()))?;
    let [length, mass, time, current, temperature, amount, luminous_intensity] =
        record.dimension().exponents();
    writeln!(f, "{prefix}.dimension.length={length}")?;
    writeln!(f, "{prefix}.dimension.mass={mass}")?;
    writeln!(f, "{prefix}.dimension.time={time}")?;
    writeln!(f, "{prefix}.dimension.current={current}")?;
    writeln!(f, "{prefix}.dimension.temperature={temperature}")?;
    writeln!(f, "{prefix}.dimension.amount={amount}")?;
    writeln!(
        f,
        "{prefix}.dimension.luminous_intensity={luminous_intensity}"
    )?;
    writeln!(f, "{prefix}.tier={}", record.tier().id())?;
    writeln!(
        f,
        "{prefix}.provenance={}",
        record
            .provenance()
            .bracket_tag()
            .expect("canonical provenance has a bracket tag")
    )?;
    write_scaled(f, &format!("{prefix}.value"), record.value())?;
    match record.measurement() {
        Some(evidence) => {
            writeln!(f, "{prefix}.measurement.present=true")?;
            writeln!(
                f,
                "{prefix}.measurement.source_id={}",
                canonical_text(evidence.source_id())
            )?;
            writeln!(
                f,
                "{prefix}.measurement.source_sha256={}",
                canonical_text(evidence.source_sha256())
            )?;
            writeln!(
                f,
                "{prefix}.measurement.source_anchor={}",
                canonical_text(evidence.source_anchor())
            )?;
            writeln!(
                f,
                "{prefix}.measurement.source_decimal={}",
                canonical_text(evidence.source_decimal())
            )?;
            writeln!(
                f,
                "{prefix}.measurement.uncertainty.kind={}",
                evidence.uncertainty_kind()
            )?;
            writeln!(
                f,
                "{prefix}.measurement.uncertainty.decimal={}",
                canonical_text(evidence.uncertainty_decimal())
            )?;
            writeln!(
                f,
                "{prefix}.measurement.projection.rule_id={}",
                canonical_text(evidence.projection_rule_id())
            )?;
            write_scaled(
                f,
                &format!("{prefix}.measurement.projection.max_abs_error"),
                evidence.projection_max_abs_error(),
            )?;
        }
        None => writeln!(f, "{prefix}.measurement.present=false")?,
    }
    match record.exhaustion() {
        Some(receipt) => {
            writeln!(f, "{prefix}.exhaustion.present=true")?;
            writeln!(
                f,
                "{prefix}.exhaustion.entry_id={}",
                canonical_text(&receipt.entry_id)
            )?;
            writeln!(
                f,
                "{prefix}.exhaustion.phenomenon={}",
                canonical_text(&receipt.phenomenon)
            )?;
            writeln!(
                f,
                "{prefix}.exhaustion.residual_slot={}",
                canonical_text(&receipt.residual_slot)
            )?;
            writeln!(
                f,
                "{prefix}.exhaustion.buckingham_pi_groups={}",
                receipt.buckingham_pi_groups
            )?;
            writeln!(
                f,
                "{prefix}.exhaustion.derivation_attempt_count={}",
                receipt.derivation_attempts.len()
            )?;
            for (index, attempt) in receipt.derivation_attempts.iter().enumerate() {
                writeln!(
                    f,
                    "{prefix}.exhaustion.derivation_attempt.{index:04}={}",
                    canonical_text(attempt)
                )?;
            }
            for (field, evidence) in [
                (
                    "gap.reference_validity",
                    &receipt.gap_law.reference_validity,
                ),
                ("gap.gap_dispatch", &receipt.gap_law.gap_dispatch),
                (
                    "gap.smooth_systematics",
                    &receipt.gap_law.smooth_systematics,
                ),
                ("gap.scale_free_limit", &receipt.gap_law.scale_free_limit),
                ("residual.conservation", &receipt.residual_law.conservation),
                (
                    "residual.disequilibrium",
                    &receipt.residual_law.disequilibrium,
                ),
                (
                    "residual.fluctuation_dissipation",
                    &receipt.residual_law.fluctuation_dissipation,
                ),
                (
                    "residual.dimensional_analysis",
                    &receipt.residual_law.dimensional_analysis,
                ),
            ] {
                writeln!(
                    f,
                    "{prefix}.exhaustion.{field}={}",
                    canonical_text(evidence)
                )?;
            }
        }
        None => writeln!(f, "{prefix}.exhaustion.present=false")?,
    }
    match record.ancestry() {
        Some(ancestry) => write_law(f, &format!("{prefix}.ancestry"), ancestry),
        None => {
            writeln!(f, "{prefix}.ancestry.law_id=<measured-leaf>")?;
            writeln!(f, "{prefix}.ancestry.input_count=0")
        }
    }
}

fn write_scaled(f: &mut fmt::Formatter<'_>, prefix: &str, value: ExactScaledValue) -> fmt::Result {
    writeln!(f, "{prefix}.bits={}", value.bits())?;
    writeln!(f, "{prefix}.scale_bits={}", value.scale_bits())
}

fn write_law(f: &mut fmt::Formatter<'_>, prefix: &str, law: &LawAncestry) -> fmt::Result {
    writeln!(f, "{prefix}.law_id={}", canonical_text(law.law_id()))?;
    writeln!(
        f,
        "{prefix}.expression={}",
        canonical_text(law.expression().unwrap_or(""))
    )?;
    writeln!(
        f,
        "{prefix}.evaluation_id={}",
        canonical_text(law.evaluation_id().unwrap_or(""))
    )?;
    writeln!(f, "{prefix}.input_count={}", law.input_ids().len())?;
    for (index, input) in law.input_ids().iter().enumerate() {
        writeln!(f, "{prefix}.input.{index:04}={}", canonical_text(input))?;
    }
    Ok(())
}

fn write_written_state(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    record: &WrittenStateRecord,
) -> fmt::Result {
    match record {
        WrittenStateRecord::RealizationIdentity {
            identity,
            source_draw,
        } => {
            writeln!(f, "{prefix}.record=realization_identity")?;
            writeln!(f, "{prefix}.identity={identity}")?;
            writeln!(f, "{prefix}.source_draw={source_draw}")
        }
        WrittenStateRecord::BodyBirth {
            body,
            parents,
            law,
            input_events,
        } => {
            writeln!(f, "{prefix}.record=body_birth")?;
            writeln!(f, "{prefix}.body={body}")?;
            writeln!(f, "{prefix}.parent_count={}", parents.len())?;
            for (index, parent) in parents.iter().enumerate() {
                writeln!(f, "{prefix}.parent.{index:04}={parent}")?;
            }
            write_law(f, &format!("{prefix}.law"), law)?;
            write_event_ids(f, &format!("{prefix}.input_event"), input_events)
        }
        WrittenStateRecord::Value {
            subject,
            field_id,
            value,
            law,
            input_events,
        } => {
            writeln!(f, "{prefix}.record=value")?;
            writeln!(f, "{prefix}.subject.kind={}", subject.kind_id())?;
            writeln!(f, "{prefix}.subject.id={}", subject.identity())?;
            writeln!(f, "{prefix}.field_id={}", canonical_text(field_id))?;
            write_value(f, &format!("{prefix}.value"), value)?;
            write_law(f, &format!("{prefix}.law"), law)?;
            write_event_ids(f, &format!("{prefix}.input_event"), input_events)
        }
    }
}

fn write_event_ids(f: &mut fmt::Formatter<'_>, prefix: &str, ids: &[EventId]) -> fmt::Result {
    writeln!(f, "{prefix}_count={}", ids.len())?;
    for (index, id) in ids.iter().enumerate() {
        writeln!(f, "{prefix}.{index:04}={id}")?;
    }
    Ok(())
}

fn write_transfer_legs(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    legs: &[super::TransferLeg],
) -> fmt::Result {
    writeln!(f, "{prefix}_count={}", legs.len())?;
    for (index, leg) in legs.iter().enumerate() {
        writeln!(
            f,
            "{prefix}.{index:04}.holder.kind={}",
            leg.holder().kind_id()
        )?;
        writeln!(
            f,
            "{prefix}.{index:04}.holder.id={}",
            leg.holder().identity()
        )?;
        write_scaled(f, &format!("{prefix}.{index:04}.amount"), leg.amount())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::{sealed_absolute_physics_floor, RefusalCode};
    use civsim_units::bignum::BigRat;
    use std::collections::BTreeMap;

    fn audited_floor() -> AbsolutePhysicsFloor {
        sealed_absolute_physics_floor().expect("the physical floor is admissible")
    }

    fn audited_transcript() -> RunTranscript {
        let floor = audited_floor();
        let view = AuditedFloorView::from_floor(&floor).expect("the floor has sealed magnitudes");
        RunTranscript::from_audited_floor(&floor, &view)
            .expect("the sealed floor has one canonical transcript projection")
    }

    #[test]
    fn floor_events_follow_the_declared_catalog_and_event_ordinals() {
        let floor = audited_floor();
        let view = AuditedFloorView::from_floor(&floor).expect("the floor has sealed magnitudes");
        let transcript = RunTranscript::from_audited_floor(&floor, &view)
            .expect("the sealed floor has one canonical transcript projection");
        let recorded_ids: Vec<_> = transcript
            .events()
            .iter()
            .filter_map(|event| match event.kind() {
                RunEventKind::FloorValue(value) => Some(value.quantity_id()),
                _ => None,
            })
            .collect();
        let expected_ids: Vec<_> = floor.entries().map(|entry| entry.id.as_str()).collect();
        assert_eq!(recorded_ids, expected_ids);
        for (index, event) in transcript.events().iter().enumerate() {
            assert_eq!(event.id().ordinal(), index as u64);
        }
        assert!(matches!(
            transcript.events()[PHYSICAL_FLOOR_LEN].kind(),
            RunEventKind::DerivedValue(record) if record.quantity_id() == "derived.eps_0"
        ));
    }

    #[test]
    fn measured_floor_events_carry_replayable_source_and_projection_receipts() {
        let floor = audited_floor();
        let view = AuditedFloorView::from_floor(&floor).expect("the floor has sealed magnitudes");
        let transcript = RunTranscript::from_audited_floor(&floor, &view).unwrap();
        for (symbol, event) in ["alpha", "G", "m_e"].into_iter().zip(transcript.events()) {
            let fundamental = view
                .execution
                .physical_invariant_definition(symbol)
                .expect("every floor coordinate carries capability-gated metadata");
            let RunEventKind::FloorValue(record) = event.kind() else {
                panic!("expected a floor value event")
            };
            let evidence = record
                .measurement()
                .expect("every measured leaf carries source evidence");
            assert_eq!(record.dimension(), fundamental.dimension);
            assert_eq!(evidence.source_id(), fundamental.source_id);
            assert_eq!(evidence.source_sha256(), fundamental.source_sha256);
            assert_eq!(evidence.source_anchor(), fundamental.source_anchor);
            assert_eq!(evidence.source_decimal(), fundamental.value);
            assert_eq!(
                evidence.uncertainty_kind(),
                fundamental.uncertainty.kind_id()
            );
            assert_eq!(
                evidence.uncertainty_decimal(),
                fundamental.uncertainty.decimal()
            );
            assert_eq!(
                evidence.projection_rule_id(),
                super::super::accounting::FLOOR_PROJECTION_RULE_ID
            );
            assert_eq!(
                evidence.projection_max_abs_error().scale_bits(),
                record.value().scale_bits() + 1
            );
            let exhaustion = record
                .exhaustion()
                .expect("every physical leaf carries derive-first exhaustion evidence");
            assert_eq!(exhaustion.entry_id, record.quantity_id());
            assert!(!exhaustion.derivation_attempts.is_empty());
        }
    }

    #[test]
    fn representation_and_eps0_replay_without_promoting_si_definitions_to_measurements() {
        fn pow(value: &BigRat, exponent: u32) -> BigRat {
            (0..exponent).fold(BigRat::from_i64(1), |acc, _| acc.mul(value))
        }

        let transcript = audited_transcript();
        let representation: BTreeMap<_, _> = transcript
            .representation()
            .values()
            .iter()
            .map(|record| (record.symbol(), record.value()))
            .collect();
        let rational = |symbol: &str| {
            let value = representation[symbol];
            BigRat::from_scaled_i128(value.bits(), value.scale_bits())
        };

        assert_eq!(
            transcript.representation().schema_id(),
            SI_REPRESENTATION_SCHEMA_ID
        );
        assert_eq!(transcript.representation().values().len(), 10);
        for record in transcript.representation().values() {
            if record.kind_id() == "exact_definition" {
                assert!(record.source_id.is_some());
                assert!(record.source_sha256.is_some());
                assert!(record.source_anchor.is_some());
                assert!(record.source_decimal.is_some());
                assert!(record.formula.is_none());
            } else {
                assert_eq!(record.kind_id(), "derived_definition");
                assert!(record.source_id.is_none());
                assert!(record.source_decimal.is_none());
                assert!(record.formula.is_some());
            }
        }
        let c = rational("c");
        let kb = rational("k_B");
        let h = rational("h");
        let e = rational("e");
        let na = rational("N_A");
        let pi = civsim_units::compute::pi(90);
        let sigma = BigRat::from_i64(2)
            .mul(&pow(&pi, 5))
            .mul(&pow(&kb, 4))
            .div(&BigRat::from_i64(15).mul(&pow(&h, 3)).mul(&pow(&c, 2)));
        let gas = na.mul(&kb);
        let atomic_volume = pow(&BigRat::from_i64(10), 24).div(&na);

        for (symbol, replayed) in [
            ("sigma", sigma),
            ("R", gas),
            ("A3_per_cm3_mol", atomic_volume),
        ] {
            let recorded = representation[symbol];
            assert_eq!(
                replayed.round_to_scale(recorded.scale_bits()),
                Some(recorded.bits()),
                "{symbol} does not replay from the representation inputs"
            );
        }

        let alpha = transcript
            .events()
            .iter()
            .find_map(|event| match event.kind() {
                RunEventKind::FloorValue(record) if record.quantity_id() == "fundamental.alpha" => {
                    Some(BigRat::from_scaled_i128(
                        record.value().bits(),
                        record.value().scale_bits(),
                    ))
                }
                _ => None,
            })
            .unwrap();
        let eps0 = e
            .mul(&e)
            .div(&BigRat::from_i64(2).mul(&alpha).mul(&h).mul(&c));
        let record = transcript
            .events()
            .iter()
            .find_map(|event| match event.kind() {
                RunEventKind::DerivedValue(record) if record.quantity_id() == "derived.eps_0" => {
                    Some(record)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(record.provenance(), Provenance::Derived);
        assert!(record.measurement().is_none());
        assert_eq!(
            eps0.round_to_scale(record.value().scale_bits()),
            Some(record.value().bits())
        );
        assert_eq!(
            record.ancestry().unwrap().evaluation_id(),
            Some(super::super::accounting::SI_EXECUTION_DERIVATION_ID)
        );
    }

    #[test]
    fn a_stage_cannot_enter_out_of_canonical_order() {
        let mut transcript = audited_transcript();
        let error = transcript
            .enter_stage(Stage::AssemblyComposition)
            .expect_err("Stage 2 cannot precede Stage 1");
        assert_eq!(
            error,
            TranscriptError::OutOfOrderStage {
                expected: Some(Stage::StarDiskSystem),
                found: Stage::AssemblyComposition,
            }
        );
        assert_eq!(transcript.events().len(), PHYSICAL_FLOOR_LEN + 1);
    }

    #[test]
    fn refusal_closes_the_stream_before_downstream_events() {
        let mut transcript = audited_transcript();
        transcript
            .enter_stage(Stage::StarDiskSystem)
            .expect("Stage 1 is the first stage");
        transcript
            .refuse(
                Some(Stage::StarDiskSystem),
                vec![Refusal::missing_stage_requirement(
                    Stage::StarDiskSystem,
                    "stellar_birth.realization_measure",
                )],
            )
            .expect("the active stage can refuse");

        assert_eq!(
            transcript.enter_stage(Stage::AssemblyComposition),
            Err(TranscriptError::Closed)
        );
        assert!(transcript.is_closed());
        assert!(transcript.events().iter().all(|event| !matches!(
            event.kind(),
            RunEventKind::StageEntered {
                stage: Stage::AssemblyComposition
            }
        )));
    }

    #[test]
    fn realization_identity_cannot_exist_before_a_recorded_measure_draw() {
        let mut transcript = audited_transcript();
        transcript
            .enter_stage(Stage::StarDiskSystem)
            .expect("Stage 1 is the first stage");
        let absent_draw = ContingencyDrawId::generated(EventId::generated(0));
        assert_eq!(
            transcript.record_realization(absent_draw),
            Err(TranscriptError::MissingContingencyMeasure {
                source_draw: absent_draw,
            })
        );
        assert!(transcript.realization_id().is_none());
        assert_eq!(transcript.contingency_draws().count(), 0);
    }

    #[test]
    fn the_same_floor_and_refusal_have_a_stable_readable_transcript() {
        fn run() -> RunTranscript {
            let mut transcript = audited_transcript();
            transcript
                .enter_stage(Stage::StarDiskSystem)
                .expect("Stage 1 is the first stage");
            transcript
                .refuse(
                    Some(Stage::StarDiskSystem),
                    vec![Refusal::missing_stage_requirement(
                        Stage::StarDiskSystem,
                        "stellar_birth.realization_measure",
                    )],
                )
                .expect("the active stage can refuse");
            transcript
        }

        let first = run();
        let second = run();
        assert_eq!(first, second);
        assert_eq!(first.to_string(), second.to_string());
        assert!(first.to_string().starts_with(
            "transcript=civsim.planet.transcript.v3\nschema.major=3\nschema.minor=0\n"
        ));
        let last = first.events().last().expect("the refusal is recorded");
        assert!(matches!(
            last.kind(),
            RunEventKind::Refused { refusals, .. }
                if refusals[0].code() == RefusalCode::MissingStageRequirement
                    && refusals[0].requirement_id()
                        == Some("stellar_birth.realization_measure")
        ));
    }

    #[test]
    fn generated_subject_serialization_uses_explicit_stable_fields() {
        let event = EventId::generated(11);
        let body = GeneratedSubjectId::Body(BodyId::generated(event, 0));
        let reservoir = GeneratedSubjectId::Reservoir(ReservoirId::generated(event, 1));

        assert_eq!(body.kind_id(), "body");
        assert_eq!(body.identity(), "body:event:0000000000000011:00000000");
        assert_eq!(reservoir.kind_id(), "reservoir");
        assert_eq!(
            reservoir.identity(),
            "reservoir:event:0000000000000011:00000001"
        );
        assert_ne!(body.identity(), format!("{body:?}"));
        assert_ne!(reservoir.identity(), format!("{reservoir:?}"));
    }

    #[test]
    fn transcript_text_escaping_is_explicit_and_stable() {
        let expected = [
            "\"",
            "alpha",
            "\\n",
            "\\\"",
            "beta",
            "\\\"",
            "\\t",
            "\\\\",
            "\\u{000001}",
            "\"",
        ]
        .concat();
        assert_eq!(
            canonical_text("alpha\n\"beta\"\t\\\u{0001}").to_string(),
            expected
        );
    }
}
