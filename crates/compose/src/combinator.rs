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

//! The combinator substrate: how a composite folds its children's port intervals on one axis.
//!
//! A [`CombinatorRegistry`] maps a data-defined topology key ([`CombinatorKey`]) to one of the four
//! fixed-Rust fold kernels ([`CombinatorKernel`]). The split is the same discipline the physics
//! substrate's [`Fold`](civsim_physics::Fold) uses: the fold MATH is a closed, small set of
//! order-defined reductions in Rust (it grows only when a new order-independent reduction is
//! introduced), while the MEMBERSHIP (which interface topology folds by which kernel) is data that
//! grows with the world (Principle 11). An interface axis names its topology by a [`CombinatorKey`];
//! the registry resolves the key to the kernel that folds that axis across a composite's children.
//!
//! The four dev-seeded kernels are each grounded in a physical way capacities combine:
//!
//! - [`CombinatorKernel::LimitingMin`]: the weakest load path. A load runs through a series of
//!   members and the chain is as strong as its weakest link, so the fold is the elementwise interval
//!   minimum. Commutative, order-independent.
//! - [`CombinatorKernel::SaturatingSum`]: redundant capacity. Parallel members each carry a share, so
//!   their capacities add, saturating at the representable limit. Commutative, order-independent, and
//!   EXACT for an additive conserved quantity (the `i128` bit-sum never rounds).
//! - [`CombinatorKernel::ConservedBudget`]: a demand that must fit an envelope or supply. The children's
//!   draws on a shared budget add (like the sum), and the viability check later confirms the total fits
//!   the matching offer. Commutative, order-independent, exact.
//! - [`CombinatorKernel::EfficiencyProduct`]: loss compounds down a chain. Each stage passes a fraction,
//!   so the fractions multiply. Fixed-point multiply is NON-ASSOCIATIVE (each product rounds), so the
//!   fold is order-SENSITIVE and MUST run in a canonical order; the evaluator folds it in ascending
//!   child content-id order so the result is deterministic and assembly-order-independent.

use crate::interval::Interval;
use civsim_core::Fixed;
use std::collections::BTreeMap;

/// A topology key: a data-defined name for a way children's ports connect on an axis. A newtype, so
/// the set of topologies grows with the data rather than being a closed Rust enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CombinatorKey(pub u32);

/// The fixed-Rust fold kernels. This is a closed enum on purpose, the same call the physics substrate
/// makes for [`Fold`](civsim_physics::Fold): these are the order-defined reductions the evaluator
/// performs, and the set grows only when a new physically-grounded reduction is added. The
/// data-driven part is the [`CombinatorRegistry`] membership, not this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombinatorKernel {
    /// The weakest load path: the elementwise interval minimum.
    LimitingMin,
    /// Redundant capacity: the saturating interval sum.
    SaturatingSum,
    /// A demand that must fit an envelope: the saturating interval sum of the draws.
    ConservedBudget,
    /// Loss compounds down a chain: the interval product, folded in canonical order.
    EfficiencyProduct,
}

impl CombinatorKernel {
    /// Whether this kernel is commutative (order-independent). Only [`CombinatorKernel::EfficiencyProduct`]
    /// is not, because fixed-point multiply rounds at each step.
    #[inline]
    pub fn is_commutative(self) -> bool {
        !matches!(self, CombinatorKernel::EfficiencyProduct)
    }

    /// Whether this kernel conserves an additive quantity exactly (parent equals the sum of children
    /// to the bit). True for the two summing kernels.
    #[inline]
    pub fn is_additive(self) -> bool {
        matches!(
            self,
            CombinatorKernel::SaturatingSum | CombinatorKernel::ConservedBudget
        )
    }

    /// Fold a set of `(content_id, interval)` children into one interval. The children are sorted by
    /// content id ascending before the fold, so the non-associative [`CombinatorKernel::EfficiencyProduct`]
    /// is deterministic and assembly-order-independent; the commutative kernels are unaffected by the
    /// ordering but use the same path so there is one fold routine. An empty child set folds to the
    /// kernel's identity: `[0, 0]` for the sums and the limiting min (nothing to carry), `[1, 1]` for
    /// the product (a lossless empty chain).
    pub fn fold(self, children: &[(u128, Interval)]) -> Interval {
        if children.is_empty() {
            return match self {
                CombinatorKernel::EfficiencyProduct => Interval::point(Fixed::ONE),
                _ => Interval::ZERO,
            };
        }
        let mut ordered: Vec<(u128, Interval)> = children.to_vec();
        ordered.sort_by_key(|(id, _)| *id);
        let mut iter = ordered.into_iter().map(|(_, iv)| iv);
        let first = iter.next().unwrap();
        match self {
            CombinatorKernel::LimitingMin => iter.fold(first, |acc, iv| acc.min_with(iv)),
            CombinatorKernel::SaturatingSum | CombinatorKernel::ConservedBudget => {
                iter.fold(first, |acc, iv| acc + iv)
            }
            CombinatorKernel::EfficiencyProduct => iter.fold(first, |acc, iv| acc * iv),
        }
    }
}

/// One combinator entry: a topology key, a name, and the kernel it folds by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinatorDef {
    /// The topology key.
    pub key: CombinatorKey,
    /// The human-readable name.
    pub name: String,
    /// The fold kernel.
    pub kernel: CombinatorKernel,
}

/// The combinator catalogue: topology key to fold kernel. Membership is data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CombinatorRegistry {
    defs: BTreeMap<u32, CombinatorDef>,
}

impl CombinatorRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        CombinatorRegistry::default()
    }

    /// Add a combinator. Returns the key.
    pub fn insert(&mut self, def: CombinatorDef) -> CombinatorKey {
        let key = def.key;
        self.defs.insert(key.0, def);
        key
    }

    /// The kernel a topology key resolves to.
    pub fn kernel(&self, key: CombinatorKey) -> Option<CombinatorKernel> {
        self.defs.get(&key.0).map(|d| d.kernel)
    }

    /// The entries, in key order.
    pub fn defs(&self) -> impl Iterator<Item = &CombinatorDef> + '_ {
        self.defs.values()
    }

    /// The stable dev-seed key for each kernel, so the interface dev seed can wire an axis to a
    /// topology without hardcoding a number.
    pub const KEY_LIMITING_MIN: CombinatorKey = CombinatorKey(0);
    /// The saturating-sum topology key.
    pub const KEY_SATURATING_SUM: CombinatorKey = CombinatorKey(1);
    /// The conserved-budget topology key.
    pub const KEY_CONSERVED_BUDGET: CombinatorKey = CombinatorKey(2);
    /// The efficiency-product topology key.
    pub const KEY_EFFICIENCY_PRODUCT: CombinatorKey = CombinatorKey(3);

    /// A labelled DEV SEED wiring the four physics-grounded topology keys to their kernels. The
    /// membership (which topologies exist) is data; a new topology is a new entry, not a code change.
    pub fn dev_seed() -> Self {
        let mut reg = CombinatorRegistry::new();
        reg.insert(CombinatorDef {
            key: Self::KEY_LIMITING_MIN,
            name: "series_load_path".to_string(),
            kernel: CombinatorKernel::LimitingMin,
        });
        reg.insert(CombinatorDef {
            key: Self::KEY_SATURATING_SUM,
            name: "parallel_capacity".to_string(),
            kernel: CombinatorKernel::SaturatingSum,
        });
        reg.insert(CombinatorDef {
            key: Self::KEY_CONSERVED_BUDGET,
            name: "shared_envelope".to_string(),
            kernel: CombinatorKernel::ConservedBudget,
        });
        reg.insert(CombinatorDef {
            key: Self::KEY_EFFICIENCY_PRODUCT,
            name: "transmission_chain".to_string(),
            kernel: CombinatorKernel::EfficiencyProduct,
        });
        reg
    }
}
