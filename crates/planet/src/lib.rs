// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The canonical abiotic planet and stellar-system pipeline.
//!
//! This crate owns planetary construction and evolution. Its only causal
//! input is the admitted absolute physics floor. A separate versioned SI
//! representation contract encodes quantities without becoming a physical
//! degree of freedom. The seven provenance types and
//! four tiers account for that floor; they do not authorize caller values. The
//! crate has no implicit world, no development-value fallback, and no
//! dependency on the parked living-world stack. An incomplete physical closure
//! returns a structured refusal receipt.

pub mod canonical;

pub use canonical::{
    audited_substrate_ledger, preflight, readiness_receipt, run_planet,
    sealed_absolute_physics_floor, AuditedCatalogError, BodyId, ConservationBalance,
    ConservationHolderId, ContingencyDrawId, ContingencyEvent, EventId, ExactScaledValue,
    ExactValueRecord, GeneratedSubjectId, LawAncestry, MeasurementEvidence, PlanetRunOutcome,
    PlanetSnapshot, RealizationId, Refusal, RefusalCode, RepresentationReceipt,
    RepresentationValueRecord, ReservoirId, RunEvent, RunEventKind, RunReceipt, RunTranscript,
    SealedFloorError, Stage, StageReceipt, StageStatus, TranscriptError, TranscriptSchema,
    TransferLeg, TransferOperation, TransferReceipt, WrittenStateEvent, WrittenStateRecord,
    RUN_TRANSCRIPT_SCHEMA_ID,
};
