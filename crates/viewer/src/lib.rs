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

//! Observer-only boundary for completed canonical planet snapshots.
//!
//! This crate borrows immutable [`civsim_planet::PlanetSnapshot`] values. It
//! does not create, advance, repair, or complete planetary state. Rendering
//! remains unavailable until the snapshot schema and an explicit input
//! transport are wired.

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
    fn startup_remains_fail_closed_without_snapshot_input() {
        assert_eq!(startup_refusal(), SnapshotInputNotWired);
    }
}
