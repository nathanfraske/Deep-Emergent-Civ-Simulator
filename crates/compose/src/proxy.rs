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

//! The whole-system-proxy substrate: the closed-form measures of a composite's emergent behaviour.
//!
//! A [`ProxyRegistry`] is an OPEN DATA REGISTRY (the owner's decision, baked in): the membership of
//! whole-system proxies is data with a labelled dev seed and owner-set membership. Each [`ProxyDef`]
//! binds a proxy id to one of the fixed-Rust [`ProxyKernel`] formulas, closed-form integer readings of
//! the aggregated port vector in physics-axis units (a resonance from stiffness against mass, a
//! thermal balance, a control-loop stability). THIS registry is where the evaluator's reach is bounded:
//! a whole-system behaviour the substrate carries no proxy for is invisible to the evaluator, and a
//! people whose interface substrate does not expose a proxy's input ports gets no technology that
//! depends on it. That is the design intent, not a limitation to paper over.
//!
//! A proxy reads the aggregated vector by ROLE (the LawPort role-to-axis pattern), so it is blind to
//! which interface-axis id carries the role and to any material or race. A proxy whose required ports
//! are absent returns `None` (inactive), so the same proxy set produces a richer library for a people
//! that exposes more ports. The criticality WEIGHT of each proxy (how hard its violation hits
//! viability) is `compose.emergent_proxy_weights`, a reserved-with-basis value the caller supplies
//! through [`ProxyWeights`]; it is never fabricated here.

use crate::interface::{InterfaceRegistry, PortVector};
use crate::interval::{sat_sub, Interval};
use civsim_core::Fixed;
use std::collections::BTreeMap;

/// A proxy id: a stable handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProxyId(pub u32);

/// The fixed-Rust proxy formulas. A closed enum, the same discipline the physics law kernels use: the
/// FORMULAS are hand-authored physics, the MEMBERSHIP (which proxies are active in a world) is the
/// [`ProxyRegistry`] data. Each returns a signed margin interval, or `None` when a required input port
/// is not exposed by the interface substrate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyKernel {
    /// A resonance index: the square root of the aggregated stiffness set against the square root of
    /// the aggregated envelope mass (`sqrt(k) - sqrt(m)`, a natural-frequency proxy). NONLINEAR in the
    /// additive mass, so it is tier-resolution-dependent by construction (a proxy read over the lump
    /// mass differs from one summed per child). Requires a `resonance_input` port and a `budget` port.
    Resonance,
    /// A thermal balance: heat offered against heat demanded. Requires a `thermal` port; latent in a
    /// substrate that exposes none (the reach bound in action).
    ThermalBalance,
    /// A control-loop stability margin: the through-chain transmission efficiency against a floor. A
    /// chain too lossy to carry a control signal is marginally stable. Requires a `chain_efficiency`
    /// port.
    ControlLoopStability,
}

impl ProxyKernel {
    /// The roles this kernel reads. A proxy is active only when every role resolves to a port in the
    /// interface substrate.
    pub fn required_roles(self) -> &'static [&'static str] {
        match self {
            ProxyKernel::Resonance => &["resonance_input", "budget"],
            ProxyKernel::ThermalBalance => &["thermal"],
            ProxyKernel::ControlLoopStability => &["chain_efficiency"],
        }
    }

    /// The signed margin interval this proxy reports over an aggregated vector, or `None` if a
    /// required port is absent (the proxy is inactive under this substrate). A non-negative margin is
    /// in-band; a negative margin is a whole-system violation the weighted penalty charges. `refs`
    /// carries the reserved-with-basis reference levels the kernels subtract (the resonance
    /// natural-frequency floor and the control-loop efficiency floor), supplied by the caller from
    /// the manifest, never fabricated here.
    pub fn margin(
        self,
        reg: &InterfaceRegistry,
        v: &PortVector,
        refs: &ProxyRefs,
    ) -> Option<Interval> {
        let read = |role: &str| -> Option<Interval> {
            let slot = reg.slot_of_role(role)?;
            Some(v.interval_at(slot))
        };
        match self {
            ProxyKernel::Resonance => {
                let k = read("resonance_input")?;
                let m = read("budget")?;
                // The natural frequency omega = sqrt(k/m), a dimensionally coherent resonance measure,
                // against the reserved resonance floor. (The earlier sqrt(k) - sqrt(m) subtracted
                // incommensurate roots: a stiffness root minus a mass root has no coherent unit.) The
                // ratio is nonlinear in the additive mass, so the tier-resolution dependence lives here.
                // The interval extremes take the worst-case ratio: lowest stiffness over highest mass
                // for the low bound, highest over lowest for the high bound.
                let omega_lo = sqrt_ratio(k.lo, m.hi);
                let omega_hi = sqrt_ratio(k.hi, m.lo);
                let lo = sat_sub(omega_lo, refs.resonance_floor);
                let hi = sat_sub(omega_hi, refs.resonance_floor);
                Some(Interval::new(lo, hi))
            }
            ProxyKernel::ThermalBalance => {
                let t = read("thermal")?;
                // Heat balance around zero: the offered-minus-demanded margin already carried on the
                // thermal port. Linear, so tier-stable.
                Some(t)
            }
            ProxyKernel::ControlLoopStability => {
                let e = read("chain_efficiency")?;
                // The chain must pass MORE than the reserved efficiency floor to carry a control
                // signal; the margin is the efficiency above that floor, so a lossy chain (below the
                // floor) reads negative and the weighted penalty charges. Returning the raw efficiency
                // against an implicit zero was structurally inert: an efficiency in [0, 1] is never
                // negative, so no lossy chain ever registered.
                let lo = sat_sub(e.lo, refs.control_efficiency_floor);
                let hi = sat_sub(e.hi, refs.control_efficiency_floor);
                Some(Interval::new(lo, hi))
            }
        }
    }
}

/// The reserved-with-basis reference levels the whole-system proxy margins subtract (design Part 41).
/// Supplied by the caller from the calibration manifest (fail-loud while reserved), never fabricated
/// in this crate; a margin subtracts the relevant floor so a below-floor design registers as a
/// whole-system violation rather than being structurally inert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProxyRefs {
    /// The control-loop through-chain efficiency floor (`compose.control_efficiency_floor`): the
    /// minimum transmission efficiency below which a chain cannot carry a control signal. Basis: the
    /// fraction of a signal a chain must pass to close a control loop, the loss beyond which feedback
    /// is unreliable; below it the [`ProxyKernel::ControlLoopStability`] margin goes negative.
    pub control_efficiency_floor: Fixed,
    /// The resonance natural-frequency floor (`compose.resonance_floor`): the minimum
    /// `omega = sqrt(k/m)` below which a design is too slack to hold its form. Basis: the lowest
    /// natural frequency a load-bearing assembly must clear so it does not resonate destructively
    /// under its service loads; below it the [`ProxyKernel::Resonance`] margin goes negative.
    pub resonance_floor: Fixed,
}

/// A non-negative square root that is total on any input (a negative reads zero), so the proxy fold
/// never panics on a signed margin.
#[inline]
fn sqrt_nonneg(v: Fixed) -> Fixed {
    if v <= Fixed::ZERO {
        Fixed::ZERO
    } else {
        v.sqrt()
    }
}

/// The natural frequency `omega = sqrt(num/den)`, total on any input. A non-positive numerator
/// (no stiffness) reads zero; a non-positive denominator (no mass) reads the saturating maximum
/// (an unbounded natural frequency), so the proxy never divides by zero or roots a negative.
#[inline]
fn sqrt_ratio(num: Fixed, den: Fixed) -> Fixed {
    if num <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    if den <= Fixed::ZERO {
        return Fixed::MAX;
    }
    match num.checked_div(den) {
        Some(r) => sqrt_nonneg(r),
        None => Fixed::MAX,
    }
}

/// One proxy entry: an id, a name, and the kernel it computes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyDef {
    /// The proxy id.
    pub id: ProxyId,
    /// The human-readable name.
    pub name: String,
    /// The formula.
    pub kernel: ProxyKernel,
}

/// The whole-system-proxy catalogue. Membership is data; the reach ceiling is this set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProxyRegistry {
    defs: BTreeMap<u32, ProxyDef>,
}

impl ProxyRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        ProxyRegistry::default()
    }

    /// Add a proxy. Returns the id.
    pub fn insert(&mut self, def: ProxyDef) -> ProxyId {
        let id = def.id;
        self.defs.insert(id.0, def);
        id
    }

    /// The proxies, in id order.
    pub fn defs(&self) -> impl Iterator<Item = &ProxyDef> + '_ {
        self.defs.values()
    }

    /// Number of proxies.
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// The resonance-proxy id.
    pub const ID_RESONANCE: ProxyId = ProxyId(0);
    /// The thermal-balance-proxy id.
    pub const ID_THERMAL: ProxyId = ProxyId(1);
    /// The control-loop-stability-proxy id.
    pub const ID_CONTROL: ProxyId = ProxyId(2);

    /// A labelled DEV SEED of the three whole-system proxies. Not owner-authored production
    /// membership. The reach ceiling of a world is which proxies its owner has admitted here.
    pub fn dev_seed() -> Self {
        let mut reg = ProxyRegistry::new();
        reg.insert(ProxyDef {
            id: Self::ID_RESONANCE,
            name: "resonance".to_string(),
            kernel: ProxyKernel::Resonance,
        });
        reg.insert(ProxyDef {
            id: Self::ID_THERMAL,
            name: "thermal_balance".to_string(),
            kernel: ProxyKernel::ThermalBalance,
        });
        reg.insert(ProxyDef {
            id: Self::ID_CONTROL,
            name: "control_loop_stability".to_string(),
            kernel: ProxyKernel::ControlLoopStability,
        });
        reg
    }
}

/// The per-proxy criticality weights (`compose.emergent_proxy_weights`), how hard each proxy's
/// violation hits viability. Reserved-with-basis and supplied by the caller; never fabricated in this
/// crate. A proxy with no weight is a construction error (fail-loud), so a proxy cannot silently
/// contribute nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProxyWeights {
    weights: BTreeMap<u32, Fixed>,
}

impl ProxyWeights {
    /// An empty weight set.
    pub fn new() -> Self {
        ProxyWeights::default()
    }

    /// Set a proxy's criticality weight.
    pub fn set(&mut self, id: ProxyId, weight: Fixed) -> &mut Self {
        self.weights.insert(id.0, weight);
        self
    }

    /// The weight of a proxy, or `None` if unset (the fail-loud sentinel the evaluator honours).
    pub fn get(&self, id: ProxyId) -> Option<Fixed> {
        self.weights.get(&id.0).copied()
    }

    /// Confirm every proxy in a registry carries a weight. Returns the id of the first proxy with no
    /// weight, so the caller can fail loud rather than let a proxy contribute nothing silently.
    pub fn first_unweighted(&self, reg: &ProxyRegistry) -> Option<ProxyId> {
        reg.defs()
            .find(|d| !self.weights.contains_key(&d.id.0))
            .map(|d| d.id)
    }
}
