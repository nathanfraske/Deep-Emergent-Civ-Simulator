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

//! Observer-only boundary for immutable canonical run artifacts.
//!
//! This crate borrows sealed [`civsim_planet::PlanetObservation`] tokens and
//! immutable [`civsim_planet::PlanetSnapshot`] values. It does not create,
//! advance, repair, or complete planetary state. Rendering remains unavailable
//! until the snapshot schema and an explicit input transport are wired.

#![forbid(unsafe_code)]

use std::fmt;

/// A read-only handle to a completed planet snapshot.
#[derive(Debug, Clone, Copy)]
pub struct SnapshotView<'a> {
    snapshot: &'a civsim_planet::PlanetSnapshot,
}

impl<'a> SnapshotView<'a> {
    /// Borrow a completed snapshot without copying or mutating canonical state.
    pub const fn new(snapshot: &'a civsim_planet::PlanetSnapshot) -> Self {
        Self { snapshot }
    }

    /// Read the stable generated identity supplied by the completed snapshot.
    pub fn realization_id(&self) -> &str {
        self.snapshot.realization_id()
    }
}

/// Create the observer-side read handle for a completed snapshot.
pub const fn observe(snapshot: &civsim_planet::PlanetSnapshot) -> SnapshotView<'_> {
    SnapshotView::new(snapshot)
}

/// A read-only view of a canonical refusal receipt.
#[derive(Debug, Clone, Copy)]
pub struct RefusalView<'a> {
    receipt: &'a civsim_planet::RunReceipt,
}

impl<'a> RefusalView<'a> {
    const fn new(receipt: &'a civsim_planet::RunReceipt) -> Self {
        Self { receipt }
    }

    /// Preserve the typed receipt for audit inspection without text parsing.
    pub const fn receipt(self) -> &'a civsim_planet::RunReceipt {
        self.receipt
    }
}

/// Read-only projection of either a completed snapshot or a refusal.
#[derive(Debug, Clone, Copy)]
pub struct ObservationView<'a> {
    observation: civsim_planet::PlanetObservation<'a>,
}

impl<'a> ObservationView<'a> {
    /// Borrow the planet-owned observation token without changing the run.
    pub const fn new(observation: civsim_planet::PlanetObservation<'a>) -> Self {
        Self { observation }
    }

    /// Completed state view, when and only when the canonical run completed.
    pub fn snapshot(self) -> Option<SnapshotView<'a>> {
        self.observation.snapshot().map(SnapshotView::new)
    }

    /// Refusal view, when and only when the canonical run refused.
    pub fn refusal(self) -> Option<RefusalView<'a>> {
        self.observation.refusal_receipt().map(RefusalView::new)
    }

    /// Whether the canonical outcome completed.
    pub const fn is_complete(self) -> bool {
        self.observation.is_complete()
    }
}

/// Adapt a planet-owned observation token without acquiring causal authority.
pub const fn observe_run(observation: civsim_planet::PlanetObservation<'_>) -> ObservationView<'_> {
    ObservationView::new(observation)
}

/// Visible refusal used while no canonical snapshot input is connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotInputNotWired;

impl fmt::Display for SnapshotInputNotWired {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("canonical PlanetSnapshot input is not wired")
    }
}

impl std::error::Error for SnapshotInputNotWired {}

/// Return the startup refusal for the current, deliberately unwired binary.
pub const fn startup_refusal() -> SnapshotInputNotWired {
    SnapshotInputNotWired
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_boundary_borrows_the_planet_snapshot() {
        let observer: for<'a> fn(&'a civsim_planet::PlanetSnapshot) -> SnapshotView<'a> = observe;
        let _ = observer;
    }

    #[test]
    fn run_observer_accepts_only_the_sealed_planet_projection() {
        let observer: for<'a> fn(civsim_planet::PlanetObservation<'a>) -> ObservationView<'a> =
            observe_run;
        let _ = observer;
    }

    #[test]
    fn startup_remains_fail_closed_without_snapshot_input() {
        assert_eq!(startup_refusal(), SnapshotInputNotWired);
    }
}
