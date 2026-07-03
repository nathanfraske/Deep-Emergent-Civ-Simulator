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

//! # civsim-compose: the emergent technology-composition evaluator (design Part 41)
//!
//! This crate is the EMERGENT side of the technology substrate, kept in its own crate OUTSIDE
//! `crates/physics` on purpose (Principle 9). Physics is the one authored layer; composition is what
//! emerges from it. Making the boundary a crate boundary means the authored-physics hand cannot reach
//! across it: nothing in `civsim-physics` depends on this crate, and the evaluator's kernel path takes
//! only [`Fixed`](civsim_core::Fixed) values, never a material or race id. A leaf runs the physics law
//! kernels over form geometry and a material's [`Substance`](civsim_physics::Substance) vector through
//! an axis-presence dispatch, so the same design over two divergent materials produces a divergent
//! viability from the material vectors alone.
//!
//! The pieces:
//!
//! - [`interval`]: the fixed-point [`Interval`] every port quantity is carried in, with saturating,
//!   bound-propagating combine operations (no wrap, no float, no RNG).
//! - [`form`]: the form-and-join floor, a data registry of geometric primitives over the wave-1
//!   geometric axes and the ways they are joined (labelled dev seed, owner-extensible membership).
//! - [`node`]: the [`CompositionNode`], with a PURE-CONTENT [`CompositionNode::content_id`] (a
//!   `StateHasher` digest over the canonical body, master seed and intent omitted), matching the
//!   physics `Substance` content-id pattern for cross-world deduplication.
//! - [`interface`]: the interface substrate, an OPEN DATA REGISTRY of interface axes bound to physics
//!   quantity axes, the fixed-width [`PortVector`] every design carries. A people's exotic axis is a
//!   different substrate, so a different library emerges under one physics (Principle 8).
//! - [`combinator`]: the data-defined combinator registry keyed by interface topology, dev-seeded with
//!   four physics-grounded fold kernels (the fold math fixed Rust, the membership data).
//! - [`proxy`]: the whole-system-proxy registry, closed-form integer measures of a composite's emergent
//!   behaviour. This registry is where the evaluator's reach is bounded.
//! - [`eval`]: [`evaluate_node`], the memoised bottom-up fold. Evaluating with a warm cache equals
//!   evaluating with none, to the bit ([`evaluate_uncached`]).
//! - [`promote`]: the three-gate promotion predicate (viability, transmission stability, reuse
//!   compression), producing the per-culture promoted-primitive library.
//!
//! Every owner number the evaluator needs is reserved fail-loud, never fabricated: the viability
//! threshold derives to the physics collapse boundary, the interface penalty curve and the proxy
//! criticality weights are reserved-with-basis and supplied by the caller.

pub mod combinator;
pub mod eval;
pub mod form;
pub mod interface;
pub mod interval;
pub mod node;
pub mod promote;
pub mod proxy;

pub use combinator::{CombinatorDef, CombinatorKernel, CombinatorKey, CombinatorRegistry};
pub use eval::{
    evaluate_node, evaluate_uncached, ComposeError, EvalParams, Memo, NodeEval, PenaltyCurve,
};
pub use form::{FormDef, FormId, FormRegistry, JoinDef, JoinId, JoinRegistry, FORM_AXES};
pub use interface::{
    Direction, InterfaceAxisDef, InterfaceAxisId, InterfaceRegistry, PortSlot, PortVector,
};
pub use interval::Interval;
pub use node::{ComponentRef, CompositionNode, IntentRef, NodeBody, TransformId};
pub use promote::{
    drift_similarity_radius, is_stabilised, promote, promoted_library, stability_span,
    DesignEvidence, Promotion, PromotionParams,
};
pub use proxy::{ProxyDef, ProxyId, ProxyKernel, ProxyRegistry, ProxyWeights};

/// A convenience builder for the labelled DEV-SEED evaluation substrate a people evaluates against: the
/// four-axis base interface, the four combinators, the three proxies, and the form-and-join floor.
/// This is a fixture, not production membership; the owner sets the real registries and the reserved
/// params. `params` carries the reserved-with-basis penalty curve and proxy weights the caller must
/// supply (fail-loud if a proxy is unweighted).
pub fn dev_memo(interface: InterfaceRegistry, params: EvalParams) -> Result<Memo, ComposeError> {
    Memo::new(
        interface,
        CombinatorRegistry::dev_seed(),
        ProxyRegistry::dev_seed(),
        FormRegistry::dev_seed(),
        JoinRegistry::dev_seed(),
        params,
    )
}
