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

//! The evaluator: a memoised, bottom-up, interval-valued fold that measures a design against physics.
//!
//! [`evaluate_node`] is a pure function of `(physics registry, evaluation substrate, node)`. It has no
//! RNG and no float: it is a memoised fold, so evaluating with a warm cache is bit-for-bit identical to
//! evaluating with none ([`evaluate_uncached`] proves it). A [`NodeBody::Leaf`] runs the relevant
//! physics law kernels over its form geometry, its material [`Substance`](civsim_physics::Substance),
//! and its join, through a small axis-presence dispatch that is BLIND to any material or race id (it
//! keys on which axes the material and geometry carry, never on a name). A [`NodeBody::Composite`]
//! recurses into its children (resolved by content id from the [`Memo`] store, cache-keyed by that id),
//! folds their vectors by each interface axis's combinator, reads the whole-system proxies, and charges
//! a graded interface-mismatch penalty through a [`PenaltyCurve`] (a graded penalty inside the adaptable
//! range, floored beyond it, so a mismatch is softened by an emergent adapter rather than hard-rejected).
//!
//! The viability is a DIMENSIONLESS safety fraction: each structural criterion reports one minus its
//! utilization (`1 - stress/strength`, `1 - energy/toughness`), so the collapse boundary is a safety of
//! zero, defined by the material's own yield and fracture data (`compose.viability_threshold` derives
//! to this boundary). A safety below zero is past collapse; a wide safety interval is an unpinnable
//! coupling. The two criteria trade off by material: a hard-brittle stone wins the stress fraction and
//! loses the energy fraction, a tough-ductile bronze the reverse, so the same design ranks differently
//! from the material vectors alone.

use crate::combinator::{CombinatorKernel, CombinatorRegistry};
use crate::form::{FormRegistry, JoinRegistry};
use crate::interface::{gather_slot, InterfaceRegistry, PortSlot, PortVector};
use crate::interval::{sat_add, sat_mul, sat_sub, Interval};
use crate::node::{CompositionNode, NodeBody};
use crate::proxy::{ProxyRegistry, ProxyWeights};
use civsim_core::Fixed;
use civsim_physics::{laws, AxisRange, Dimension, PhysicsRegistry, QuantityAxis, Substance};
use std::collections::BTreeMap;

/// A piecewise-linear penalty curve: sorted `(mismatch, penalty)` points, linearly interpolated,
/// CLAMPED to the end points outside the range. This is the compose-local mirror of the simulation's
/// `decision::Curve` (compose sits below `civsim-sim` in the crate graph, so it cannot depend on it);
/// the shape and the clamp-at-ends semantics are the same. The clamp is what makes the penalty FLOOR
/// beyond the adaptable range rather than diverge: past the last point the penalty holds at the last
/// `y`, so a large mismatch is a bounded penalty an adapter can pay, never a hard rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenaltyCurve {
    points: Vec<(Fixed, Fixed)>,
}

impl PenaltyCurve {
    /// Build a curve from points, sorted by x.
    pub fn new(points: impl IntoIterator<Item = (Fixed, Fixed)>) -> Self {
        let mut points: Vec<(Fixed, Fixed)> = points.into_iter().collect();
        points.sort_by_key(|(x, _)| *x);
        PenaltyCurve { points }
    }

    /// Evaluate the penalty at a mismatch, clamped to the end points. An empty curve reads zero (no
    /// penalty), so an unspecified curve never fabricates a rejection.
    pub fn eval(&self, x: Fixed) -> Fixed {
        match self.points.first() {
            None => Fixed::ZERO,
            Some(&(x0, y0)) if x <= x0 => y0,
            _ => {
                let &(xn, yn) = self.points.last().unwrap();
                if x >= xn {
                    return yn;
                }
                for win in self.points.windows(2) {
                    let (x0, y0) = win[0];
                    let (x1, y1) = win[1];
                    if x >= x0 && x <= x1 {
                        if x1 == x0 {
                            return y0;
                        }
                        let frac = (x - x0).div(x1 - x0);
                        return y0 + (y1 - y0).mul(frac);
                    }
                }
                yn
            }
        }
    }

    /// The adaptable range: the x-span the curve grades over, before it floors. Derives from the
    /// interface axis's physics range width; the penalty depths (the y-values) are reserved-with-basis.
    pub fn adaptable_range(&self) -> Fixed {
        match (self.points.first(), self.points.last()) {
            (Some(&(x0, _)), Some(&(xn, _))) => sat_sub(xn, x0),
            _ => Fixed::ZERO,
        }
    }
}

/// The reserved-with-basis evaluation parameters the caller supplies: the interface-mismatch penalty
/// curve (`compose.interface_penalty_curve`) and the per-proxy criticality weights
/// (`compose.emergent_proxy_weights`). Neither is fabricated in this crate; the caller reads them from
/// the calibration manifest (fail-loud if reserved) and passes them here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalParams {
    /// The interface-mismatch penalty curve.
    pub penalty_curve: PenaltyCurve,
    /// The per-proxy criticality weights.
    pub proxy_weights: ProxyWeights,
}

/// What can go wrong wiring the evaluation substrate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeError {
    /// A proxy in the registry has no criticality weight (the fail-loud sentinel: a proxy must never
    /// contribute nothing silently).
    UnweightedProxy(u32),
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComposeError::UnweightedProxy(id) => write!(
                f,
                "proxy {id} has no criticality weight; compose.emergent_proxy_weights must set one before evaluation (never fabricate a value)"
            ),
        }
    }
}

impl std::error::Error for ComposeError {}

/// The result of evaluating a node: its achieved interface vector and its viability safety interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeEval {
    /// The achieved interface vector (the physics-computed offers and the declared demands, folded).
    pub vector: PortVector,
    /// The viability as a dimensionless safety interval. The lower bound is what the promotion gate
    /// compares to the derived collapse boundary (a safety of zero).
    pub viability: Interval,
}

/// The evaluation substrate and cache: the content-addressed node store (for resolving a composite's
/// children), the non-authoritative eval cache, the interface and combinator and proxy registries, the
/// form-and-join floor, and the reserved eval params. Two peoples with different interface substrates
/// are two different `Memo`s, so the same intent stream yields a different library for each.
#[derive(Debug, Clone)]
pub struct Memo {
    store: BTreeMap<u128, CompositionNode>,
    evals: BTreeMap<u128, NodeEval>,
    interface: InterfaceRegistry,
    combinators: CombinatorRegistry,
    proxies: ProxyRegistry,
    forms: FormRegistry,
    joins: JoinRegistry,
    params: EvalParams,
}

impl Memo {
    /// Build the evaluation substrate. Fails loud if a proxy has no criticality weight, so a reserved
    /// value is never silently defaulted.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        interface: InterfaceRegistry,
        combinators: CombinatorRegistry,
        proxies: ProxyRegistry,
        forms: FormRegistry,
        joins: JoinRegistry,
        params: EvalParams,
    ) -> Result<Self, ComposeError> {
        if let Some(id) = params.proxy_weights.first_unweighted(&proxies) {
            return Err(ComposeError::UnweightedProxy(id.0));
        }
        Ok(Memo {
            store: BTreeMap::new(),
            evals: BTreeMap::new(),
            interface,
            combinators,
            proxies,
            forms,
            joins,
            params,
        })
    }

    /// Store a node by its content id, so a composite that references it can resolve it. Returns the
    /// id. Children must be inserted before the composite that references them (a bottom-up build).
    pub fn insert(&mut self, node: CompositionNode) -> u128 {
        let id = node.content_id();
        self.store.insert(id, node);
        id
    }

    /// A stored node by content id.
    pub fn node(&self, id: u128) -> Option<&CompositionNode> {
        self.store.get(&id)
    }

    /// The interface substrate this memo evaluates against.
    pub fn interface(&self) -> &InterfaceRegistry {
        &self.interface
    }

    /// Clear the non-authoritative eval cache (the store and substrate are kept). Used to prove the
    /// cache is an optimization: a cold re-evaluation must reproduce the warm result bit-for-bit.
    pub fn clear_cache(&mut self) {
        self.evals.clear();
    }
}

/// Evaluate a node with memoisation. The result for a content id is computed once and reused; because
/// the fold is a pure function of the content, the cached result equals the recomputed one to the bit
/// (see [`evaluate_uncached`]).
pub fn evaluate_node(reg: &PhysicsRegistry, memo: &mut Memo, node: &CompositionNode) -> NodeEval {
    compute(reg, memo, node, true)
}

/// Evaluate a node WITHOUT consulting or populating the eval cache. Children are still resolved from
/// the store, but every result is recomputed. This is the cache-soundness oracle: it must equal
/// [`evaluate_node`] to the bit.
pub fn evaluate_uncached(
    reg: &PhysicsRegistry,
    memo: &mut Memo,
    node: &CompositionNode,
) -> NodeEval {
    compute(reg, memo, node, false)
}

fn compute(
    reg: &PhysicsRegistry,
    memo: &mut Memo,
    node: &CompositionNode,
    use_cache: bool,
) -> NodeEval {
    if use_cache {
        if let Some(cached) = memo.evals.get(&node.id) {
            return cached.clone();
        }
    }
    let ev = match &node.body {
        NodeBody::Leaf { .. } => eval_leaf(reg, memo, node),
        NodeBody::Composite { children, .. } => {
            // Resolve and evaluate each child, cloning it out of the store first so the store borrow is
            // released before the recursive &mut call. A referenced child that was never inserted is a
            // caller contract violation (the build must be bottom-up); it contributes an empty vector
            // and a collapsed viability so the omission surfaces as a non-viable design, never a panic.
            let mut child_evals: Vec<(u128, NodeEval)> = Vec::with_capacity(children.len());
            for cref in children {
                match memo.node(cref.target).cloned() {
                    Some(child) => {
                        let cev = compute(reg, memo, &child, use_cache);
                        child_evals.push((cref.target, cev));
                    }
                    None => {
                        child_evals.push((
                            cref.target,
                            NodeEval {
                                vector: memo.interface.empty_vector(),
                                viability: Interval::point(Fixed::from_int(-1)),
                            },
                        ));
                    }
                }
            }
            eval_composite(memo, node, &child_evals)
        }
    };
    if use_cache {
        memo.evals.insert(node.id, ev.clone());
    }
    ev
}

// === The leaf: run the physics over form geometry and material, blind to any id ===

fn eval_leaf(reg: &PhysicsRegistry, memo: &Memo, node: &CompositionNode) -> NodeEval {
    let (primitives, material, joining) = match &node.body {
        NodeBody::Leaf {
            primitives,
            material,
            joining,
        } => (primitives, *material, *joining),
        NodeBody::Composite { .. } => unreachable!("eval_leaf on a composite"),
    };

    let caps = Caps::derive(reg);
    let substance = find_substance(reg, material);

    // Geometry: the per-axis saturating sum over the leaf's primitives (a single-primitive leaf is the
    // common case). edge_radius is carried by the form floor for the cutting dispatch; the structural
    // dispatch here reads contact_area, section_modulus, and arm_length.
    let contact_area = geo(&memo.forms, primitives, "mech.contact_area");
    let section_modulus = geo(&memo.forms, primitives, "mech.section_modulus");
    let arm_length = geo(&memo.forms, primitives, "mech.arm_length");

    // Material axis values, read by axis id (never by substance name): the identity-blind dispatch.
    let yield_strength = mat(substance, "mat.yield_strength");
    let fracture_strength = mat(substance, "mat.fracture_strength");
    let fracture_energy = mat(substance, "mat.fracture_energy");
    let elastic_modulus = mat(substance, "mat.elastic_modulus");

    // The design's declared parameters, in fixed order: the reference-load bounds, the declared
    // envelope mass, and the delivered-impact-energy bounds. A missing param reads zero.
    let load_lo = param(node, 0);
    let load_hi = param(node, 1);
    let mass = param(node, 2);
    let energy_lo = param(node, 3);
    let energy_hi = param(node, 4);

    // The applied bending stress at the two reference loads (higher load, higher stress). bend_stress
    // returns (sigma, yield_margin); the kernel takes only Fixed.
    let (sigma_lo, _) = laws::bend_stress(
        load_lo,
        section_modulus,
        arm_length,
        yield_strength,
        caps.pressure,
    );
    let (sigma_hi, _) = laws::bend_stress(
        load_hi,
        section_modulus,
        arm_length,
        yield_strength,
        caps.pressure,
    );

    // The dual fracture criterion at each load. fracture_onset returns (stress_margin, energy_margin):
    // the stress margin is fracture_strength - sigma, the energy margin is
    // fracture_energy*crack_area - delivered_energy. Low load gives the high margins.
    let (frac_stress_hi_margin, energy_margin_hi) = laws::fracture_onset(
        sigma_lo,
        fracture_strength,
        fracture_energy,
        contact_area,
        energy_lo,
        caps.energy,
    );
    let (frac_stress_lo_margin, energy_margin_lo) = laws::fracture_onset(
        sigma_hi,
        fracture_strength,
        fracture_energy,
        contact_area,
        energy_hi,
        caps.energy,
    );

    // The absorbable energy capacity (for the energy safety fraction).
    let energy_capacity = sat_mul(fracture_energy, contact_area).min(caps.energy);

    // Dimensionless safety fractions: one minus utilization. The collapse boundary is a safety of zero,
    // defined by the material's own strength and toughness (this is what compose.viability_threshold
    // derives to). Higher load lowers safety, so the interval low end is the high-load evaluation.
    let stress_safety = Interval::new(
        safety(sigma_hi, fracture_strength),
        safety(sigma_lo, fracture_strength),
    );
    let energy_safety = Interval::new(
        safety(energy_hi, energy_capacity),
        safety(energy_lo, energy_capacity),
    );

    // Viability is the weakest of the structural safety fractions (the limiting criterion). The two
    // trade off by material, so the ranking diverges from the material vectors alone.
    let viability = stress_safety.min_with(energy_safety);

    // The achieved interface vector: fill each slot by role, so a substrate that does not carry a role
    // simply has no such slot (the exotic axis is present only for the people that expose it).
    let join_eff = memo.joins.efficiency(joining);
    let mut vector = memo.interface.empty_vector();
    let slots: Vec<PortSlot> = vector.slots().to_vec();
    for (i, slot) in slots.iter().enumerate() {
        let role = memo
            .interface
            .axis_at(i)
            .map(|a| a.role.as_str())
            .unwrap_or("");
        let interval = match role {
            // The structural-margin offer (the raw fracture stress margin from the law, MPa).
            "margin" => Interval::new(frac_stress_lo_margin, frac_stress_hi_margin),
            // The toughness offer (the raw absorbable-energy margin from the law).
            "toughness" => Interval::new(energy_margin_lo, energy_margin_hi),
            // The declared envelope-mass demand.
            "budget" => Interval::point(mass),
            // The chain transmission efficiency the join offers.
            "chain_efficiency" => Interval::point(join_eff),
            // The exotic stiffness offer (present only under the exotic substrate).
            "resonance_input" => Interval::point(elastic_modulus),
            _ => slot.interval,
        };
        vector.slots_mut()[i] = PortSlot {
            axis: slot.axis,
            direction: slot.direction,
            interval,
        };
    }

    NodeEval { vector, viability }
}

// === The composite: fold children by combinator, apply proxies and the interface penalty ===

fn eval_composite(
    memo: &Memo,
    node: &CompositionNode,
    child_evals: &[(u128, NodeEval)],
) -> NodeEval {
    let assembly_join = match &node.body {
        NodeBody::Composite { assembly_join, .. } => *assembly_join,
        NodeBody::Leaf { .. } => unreachable!("eval_composite on a leaf"),
    };
    let child_vectors: Vec<(u128, PortVector)> = child_evals
        .iter()
        .map(|(id, ev)| (*id, ev.vector.clone()))
        .collect();

    // Fold each interface slot across the children by that axis's combinator kernel. The kernel sorts
    // by child content id, so the non-associative EfficiencyProduct is order-independent.
    let mut slots: Vec<PortSlot> = Vec::with_capacity(memo.interface.width());
    let assembly_eff = memo.joins.efficiency(assembly_join);
    for (i, axis) in memo.interface.axes().enumerate() {
        let kernel = memo
            .combinators
            .kernel(axis.combinator)
            .unwrap_or(CombinatorKernel::LimitingMin);
        let gathered = gather_slot(&child_vectors, i);
        let mut agg = kernel.fold(&gathered);
        // The assembly's own join is one more loss stage on a transmission chain.
        if matches!(kernel, CombinatorKernel::EfficiencyProduct) {
            agg = agg * Interval::point(assembly_eff);
        }
        slots.push(PortSlot {
            axis: axis.id,
            direction: axis.direction,
            interval: agg,
        });
    }
    let vector = PortVector::from_slots(slots);

    // Base viability: the weakest child (the limiting load path across the assembly).
    let mut viability = child_evals
        .iter()
        .map(|(_, ev)| ev.viability)
        .reduce(|a, b| a.min_with(b))
        .unwrap_or(Interval::point(Fixed::ONE));

    // The interface-mismatch penalty: the ConservedBudget over-envelope shortfall, normalized by the
    // declared budget, graded through the curve inside the adaptable range and floored beyond it.
    let mismatch = envelope_mismatch(memo, node, &vector);
    let penalty = memo.params.penalty_curve.eval(mismatch);
    viability = viability - Interval::point(penalty);

    // The whole-system proxies: a proxy whose ports are absent is inactive; an active proxy with a
    // negative margin charges its criticality-weighted shortfall against viability.
    for def in memo.proxies.defs() {
        if let Some(margin) = def.kernel.margin(&memo.interface, &vector) {
            if margin.lo < Fixed::ZERO {
                let weight = memo.params.proxy_weights.get(def.id).unwrap_or(Fixed::ZERO);
                let shortfall = sat_sub(Fixed::ZERO, margin.lo);
                viability = viability - Interval::point(sat_mul(weight, shortfall));
            }
        }
    }

    NodeEval { vector, viability }
}

/// The normalized over-envelope mismatch: how far the aggregated envelope-mass demand exceeds the
/// composite's declared budget, as a fraction of the budget. Zero if it fits or if no budget is
/// declared. This is the ConservedBudget "must fit an envelope" check that drives the interface penalty.
fn envelope_mismatch(memo: &Memo, node: &CompositionNode, vector: &PortVector) -> Fixed {
    let Some(slot) = memo.interface.slot_of_role("budget") else {
        return Fixed::ZERO;
    };
    let budget = param(node, 2);
    if budget <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    let demand = vector.interval_at(slot).hi;
    let over = sat_sub(demand, budget);
    if over <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    over.checked_div(budget).unwrap_or(Fixed::MAX)
}

// === Physics caps derived from the substrate (routing out-of-range to the physical limit) ===

/// The physical-limit caps the leaf kernels route out-of-range results to. Each derives from the
/// physics substrate: the largest set upper bound over the axes of a dimension is that dimension's
/// representable physical limit. A dimension with no set axis falls back to the representable ceiling
/// [`Fixed::MAX`] (the hardware limit, not an owner value), so a cap is never fabricated.
struct Caps {
    pressure: Fixed,
    energy: Fixed,
}

impl Caps {
    fn derive(reg: &PhysicsRegistry) -> Caps {
        Caps {
            pressure: dim_cap(reg, Dimension::PRESSURE),
            energy: dim_cap(reg, Dimension::ENERGY),
        }
    }
}

fn dim_cap(reg: &PhysicsRegistry, dim: Dimension) -> Fixed {
    reg.axes()
        .filter(|a| a.dimension == dim)
        .filter_map(axis_hi)
        .max()
        .unwrap_or(Fixed::MAX)
}

fn axis_hi(a: &QuantityAxis) -> Option<Fixed> {
    match &a.range {
        AxisRange::Set { hi, .. } => Some(*hi),
        AxisRange::Reserved { .. } => None,
    }
}

// === Small helpers ===

/// Find a substance by its content id (a linear scan; the substance set is small and the result is
/// memoised, so the cost is bounded). Blind to the human label: two substances with identical physics
/// resolve to one, the anti-identity-steering guarantee at the leaf.
fn find_substance(reg: &PhysicsRegistry, content_id: u128) -> Option<&Substance> {
    reg.substances().find(|s| s.content_id() == content_id)
}

fn geo(forms: &FormRegistry, primitives: &[crate::form::FormId], axis: &str) -> Fixed {
    let mut acc = Fixed::ZERO;
    for f in primitives {
        if let Some(def) = forms.get(*f) {
            acc = sat_add(acc, def.geo(axis));
        }
    }
    acc
}

fn mat(substance: Option<&Substance>, axis: &str) -> Fixed {
    substance
        .and_then(|s| s.vector.get(axis).copied())
        .unwrap_or(Fixed::ZERO)
}

fn param(node: &CompositionNode, i: usize) -> Fixed {
    node.param.get(i).copied().unwrap_or(Fixed::ZERO)
}

/// One minus utilization, the dimensionless safety fraction. No capacity is fully unsafe; an overflow
/// in the division routes to fully unsafe. A safety below zero is past the collapse boundary.
fn safety(load_effect: Fixed, capacity: Fixed) -> Fixed {
    if capacity <= Fixed::ZERO {
        return Fixed::from_int(-1);
    }
    let util = load_effect
        .checked_div(capacity)
        .unwrap_or(Fixed::from_int(2));
    sat_sub(Fixed::ONE, util)
}
