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

//! Parked compatibility for the retired biology and civilization physics surface.
//!
//! The root `civsim-physics` package owns only the abiotic substrate. This crate keeps old parked
//! callers source-compatible: it re-exports the abiotic surface, restores the retired law kernels,
//! validates the retired graph contracts, and makes [`PhysicsRegistry::ground`] return the former
//! combined registry. Canonical packages cannot depend on this nested workspace.

pub mod floor_provenance;
pub mod graph;
pub mod laws;

pub use civsim_physics_abiotic::*;

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::path::Path;

/// The legacy registry facade. All representation and abiotic validation remain in the root package;
/// this wrapper adds only the retired graph contracts and embedded biology data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhysicsRegistry(civsim_physics_abiotic::PhysicsRegistry);

impl PhysicsRegistry {
    /// An empty compatibility registry.
    pub fn new() -> Self {
        Self(civsim_physics_abiotic::PhysicsRegistry::new())
    }

    /// Parse a registry with both abiotic and retired-domain kernel contracts available.
    pub fn from_toml_str(s: &str) -> Result<Self, PhysicsError> {
        let mut inner = civsim_physics_abiotic::PhysicsRegistry::new();
        inner.extend_from_toml_str_with_kernel_contracts(s, graph::legacy_kernel_contract)?;
        Ok(Self(inner))
    }

    /// Load a registry file with both contract sets available.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PhysicsError> {
        let text = std::fs::read_to_string(path).map_err(|e| PhysicsError::Io(e.to_string()))?;
        Self::from_toml_str(&text)
    }

    /// The former combined ground registry: the root abiotic floor, retired shared catalog rows,
    /// the parked biology floor, and the four organism-facing ground substances.
    pub fn ground() -> Result<Self, PhysicsError> {
        let mut inner = civsim_physics_abiotic::PhysicsRegistry::ground()?;
        inner.extend_from_toml_str_with_kernel_contracts(
            include_str!("../data/legacy_shared_floor.toml"),
            graph::legacy_kernel_contract,
        )?;
        inner.extend_from_toml_str_with_kernel_contracts(
            include_str!("../data/biology_floor.toml"),
            graph::legacy_kernel_contract,
        )?;
        inner.extend_from_toml_str_with_kernel_contracts(
            include_str!("../data/biology_ground_floor.toml"),
            graph::legacy_kernel_contract,
        )?;
        Ok(Self(inner))
    }

    /// Extend with a root or retired-domain floor.
    pub fn extend_from_toml_str(&mut self, s: &str) -> Result<(), PhysicsError> {
        self.0
            .extend_from_toml_str_with_kernel_contracts(s, graph::legacy_kernel_contract)
    }

    /// Extend from a file path.
    pub fn extend(&mut self, path: impl AsRef<Path>) -> Result<(), PhysicsError> {
        let text = std::fs::read_to_string(path).map_err(|e| PhysicsError::Io(e.to_string()))?;
        self.extend_from_toml_str(&text)
    }

    /// Borrow the underlying abiotic representation.
    pub fn as_abiotic(&self) -> &civsim_physics_abiotic::PhysicsRegistry {
        &self.0
    }

    /// Consume the facade and return the underlying representation.
    pub fn into_abiotic(self) -> civsim_physics_abiotic::PhysicsRegistry {
        self.0
    }

    /// The derived tier of a law.
    pub fn derived_tier(&self, law_id: &str) -> Option<u32> {
        self.0.derived_tier(law_id)
    }

    /// Every derived law tier in sorted id order.
    pub fn derived_tiers(&self) -> BTreeMap<String, u32> {
        self.0.derived_tiers()
    }

    /// An axis by id.
    pub fn axis(&self, id: &str) -> Option<&QuantityAxis> {
        self.0.axis(id)
    }

    /// A law by id.
    pub fn law(&self, id: &str) -> Option<&InteractionLaw> {
        self.0.law(id)
    }

    /// A substance by id.
    pub fn substance(&self, id: &str) -> Option<&Substance> {
        self.0.substance(id)
    }

    /// The axes in sorted id order.
    pub fn axes(&self) -> impl Iterator<Item = &QuantityAxis> + '_ {
        self.0.axes()
    }

    /// The laws in sorted id order.
    pub fn laws(&self) -> impl Iterator<Item = &InteractionLaw> + '_ {
        self.0.laws()
    }

    /// The substances in sorted id order.
    pub fn substances(&self) -> impl Iterator<Item = &Substance> + '_ {
        self.0.substances()
    }

    /// Reserved axis ids in sorted order.
    pub fn reserved_axis_ids(&self) -> Vec<&str> {
        self.0.reserved_axis_ids()
    }

    /// The deterministic content id of the combined registry.
    pub fn content_id(&self) -> u128 {
        self.0.content_id()
    }

    /// Number of axes.
    pub fn axis_count(&self) -> usize {
        self.0.axis_count()
    }

    /// Number of laws.
    pub fn law_count(&self) -> usize {
        self.0.law_count()
    }

    /// Number of substances.
    pub fn substance_count(&self) -> usize {
        self.0.substance_count()
    }
}

impl Deref for PhysicsRegistry {
    type Target = civsim_physics_abiotic::PhysicsRegistry;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PhysicsRegistry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<civsim_physics_abiotic::PhysicsRegistry> for PhysicsRegistry {
    fn from(value: civsim_physics_abiotic::PhysicsRegistry) -> Self {
        Self(value)
    }
}

impl From<PhysicsRegistry> for civsim_physics_abiotic::PhysicsRegistry {
    fn from(value: PhysicsRegistry) -> Self {
        value.0
    }
}
