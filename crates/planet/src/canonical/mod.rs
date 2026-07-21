//! The fail-closed canonical front door.

mod accounting;
mod catalog;
mod floor_magnitudes;
mod identity;
mod pipeline;
mod preflight;
mod receipt;
mod snapshot;
mod stage;
mod star_disk_system;
mod stellar_birth_measure;
mod transcript;

pub use accounting::{
    ConservationBalance, ConservationHolderId, ExactScaledValue, ExactValueRecord, LawAncestry,
    MeasurementEvidence, TransferLeg, TransferOperation, TransferReceipt,
};
pub use catalog::{
    audited_substrate_ledger, sealed_absolute_physics_floor, AuditedCatalogError, SealedFloorError,
};
pub use civsim_ledger::{
    AbsolutePhysicsFloor, DerivationExhaustionReceipt, FloorAdmissionError, GapLawReceipt,
    ResidualLawReceipt,
};
pub use identity::{BodyId, ContingencyDrawId, EventId, RealizationId, ReservoirId};
pub use pipeline::{readiness_receipt, run_planet, PlanetRunOutcome};
pub use preflight::preflight;
pub use receipt::{Refusal, RefusalCode, RunReceipt, StageReceipt, StageStatus};
pub use snapshot::PlanetSnapshot;
pub use stage::Stage;
pub use transcript::{
    ContingencyEvent, GeneratedSubjectId, RepresentationReceipt, RepresentationValueRecord,
    RunEvent, RunEventKind, RunTranscript, TranscriptError, TranscriptSchema, WrittenStateEvent,
    WrittenStateRecord, RUN_TRANSCRIPT_SCHEMA_ID,
};
