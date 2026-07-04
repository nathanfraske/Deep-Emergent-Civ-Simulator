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

//! Promotion: the three-gate predicate that turns an evaluated design into a per-culture promoted
//! primitive.
//!
//! A design earns a place in a culture's technology library only when it passes all three gates:
//!
//! 1. VIABILITY: its viability lower bound is at or above the collapse boundary. The boundary
//!    (`compose.viability_threshold`) DERIVES to a safety fraction of zero, the point at which a
//!    structural utilization reaches the material's own yield or fracture capacity (`crate::eval`); an
//!    owner safety factor above zero, if any, is the only reserved-with-basis part. A lower bound below
//!    the boundary is rejected even when the upper bound would pass, so an unpinnable coupling (a wide
//!    interval straddling the boundary) does not promote.
//! 2. TRANSMISSION STABILITY: the design has outrun both loss and drift. It has persisted at least
//!    [`stability_span`] ticks (`ceil(1 / loss_rate)`, so it is not being forgotten) and its live copies
//!    have re-converged within [`drift_similarity_radius`] (`2 * drift_rate`, so the technique has not
//!    diffused into mutually-unrecognizable variants). Both rates are the transmission substrate's own
//!    (`transmission.loss_rate`, `transmission.drift_rate`), supplied by the caller;
//!    `compose.transmission_stability` derives set-equal to them, the same derivation the transmission
//!    substrate already records (`crates/sim/src/transmission.rs`). The math is mirrored here because
//!    compose sits below `civsim-sim` in the crate graph.
//! 3. REUSE COMPRESSION: the design is reused at least `compose.reuse_compression_threshold` times, the
//!    integer surrogate for a description-length decrease (a component that pays for itself as a named
//!    primitive because enough designs build on it).
//!
//! The result over a set of designs is the per-culture promoted-primitive library: the content ids that
//! pass all three gates, in ascending id order (deterministic, observer-independent).

use crate::interval::Interval;
use civsim_core::Fixed;

/// The reserved-with-basis and derived parameters of the three gates, supplied by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionParams {
    /// The viability floor: the DERIVED collapse boundary (a safety of zero) plus any owner safety
    /// factor. `compose.viability_threshold` derives to zero here; a positive value is the reserved
    /// safety margin above the physics boundary.
    pub viability_floor: Fixed,
    /// The below-floor erosion rate `transmission.loss_rate`; `stability_span` is `ceil(1 / loss_rate)`.
    pub loss_rate: Fixed,
    /// The copy-drift rate `transmission.drift_rate`; the similarity radius is `2 * drift_rate`.
    pub drift_rate: Fixed,
    /// The reuse-compression threshold `compose.reuse_compression_threshold`.
    pub reuse_threshold: u32,
}

/// The transmission-and-reuse evidence for one design, the input the promotion gates read beyond the
/// evaluator's viability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignEvidence {
    /// The design's content id.
    pub id: u128,
    /// The evaluated viability safety interval.
    pub viability: Interval,
    /// How many ticks the design has been continuously held (the loss test).
    pub persisted_ticks: u64,
    /// The current live copies' proficiencies (the drift test reads their spread).
    pub copies: Vec<Fixed>,
    /// How many other designs reuse this design as a component (the reuse-compression count).
    pub reuse_count: u32,
}

/// Why a design did or did not promote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Promotion {
    /// Passed all three gates.
    Promoted,
    /// The viability lower bound is below the collapse boundary.
    RejectedViability,
    /// The design has not outrun loss and drift.
    RejectedUnstable,
    /// The design is not reused enough to compress the library.
    RejectedReuse,
}

impl Promotion {
    /// Whether the outcome is a promotion.
    #[inline]
    pub fn is_promoted(self) -> bool {
        matches!(self, Promotion::Promoted)
    }
}

/// The persistence span a design must outrun to escape loss: `ceil(1 / loss_rate)` ticks. A
/// non-positive rate never erodes, so the span is [`u64::MAX`] (a fail-safe; a real loss rate is a set,
/// positive value). Mirrors `transmission::stability_span` from which the rate flows.
pub fn stability_span(loss_rate: Fixed) -> u64 {
    if loss_rate <= Fixed::ZERO {
        return u64::MAX;
    }
    let q = Fixed::ONE.div(loss_rate);
    let whole = q.to_int() as i64;
    let has_frac = q != Fixed::from_int(q.to_int());
    let span = whole + i64::from(has_frac);
    span.max(1) as u64
}

/// The drift-similarity radius two copies of one design may differ by and still count as the same
/// technique: `2 * drift_rate`. Mirrors `transmission::drift_similarity_radius`.
pub fn drift_similarity_radius(drift_rate: Fixed) -> Fixed {
    Fixed::from_int(2).mul(drift_rate)
}

/// Whether a design has stabilised: persisted at least [`stability_span`] ticks and its live copies
/// have re-converged within [`drift_similarity_radius`]. Fewer than two copies cannot show divergence
/// and count as converged.
pub fn is_stabilised(ev: &DesignEvidence, params: &PromotionParams) -> bool {
    if ev.persisted_ticks < stability_span(params.loss_rate) {
        return false;
    }
    if ev.copies.len() < 2 {
        return true;
    }
    let mut lo = ev.copies[0];
    let mut hi = ev.copies[0];
    for &c in &ev.copies[1..] {
        lo = lo.min(c);
        hi = hi.max(c);
    }
    crate::interval::sat_sub(hi, lo) <= drift_similarity_radius(params.drift_rate)
}

/// Apply the three gates in order, returning the outcome (the first failing gate's reason, or a
/// promotion). The order is viability, then stability, then reuse: a design that would collapse is
/// rejected before its transmission is even asked about.
pub fn promote(ev: &DesignEvidence, params: &PromotionParams) -> Promotion {
    if ev.viability.lo < params.viability_floor {
        return Promotion::RejectedViability;
    }
    if !is_stabilised(ev, params) {
        return Promotion::RejectedUnstable;
    }
    if ev.reuse_count < params.reuse_threshold {
        return Promotion::RejectedReuse;
    }
    Promotion::Promoted
}

/// The per-culture promoted-primitive library: the content ids that pass all three gates, in ascending
/// id order (deterministic and observer-independent). Duplicate ids fold to one entry.
pub fn promoted_library(evidence: &[DesignEvidence], params: &PromotionParams) -> Vec<u128> {
    let mut ids: Vec<u128> = evidence
        .iter()
        .filter(|ev| promote(ev, params).is_promoted())
        .map(|ev| ev.id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}
