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

//! The canonical simulation runner: the physics field layer over the located world (design Part 5.4,
//! Part 5.5, and the map program's phase 4). This is the first piece of the true runner, and it is
//! held to the canonical standard, not the harness standard: it carries ZERO authored numbers. Every
//! calibration it needs is a reserved value ([`FieldCalib`]) the caller supplies from the manifest,
//! which fails loud if unset (Principle 11, the reserved-value discipline); there is no `Default` and
//! no `dev_default`, so no fabricated number can reach canonical state. Tests supply the calibrations
//! and the field baseline as clearly-labelled fixtures, which is the only sanctioned use of an
//! authored number.
//!
//! What it does: it holds a canonical fixed-point temperature [`Field`] over the flat bounded map and
//! the [`crate::located::LocationIndex`] of who stands where, and each tick it steps the field (a
//! pinned integer diffusion-and-relaxation stencil, the Part 5.5 GPU workload, bit-identical on the
//! CPU and, once authored as a CubeCL `#[cube]` kernel, on the GPU) and lets each located being
//! exchange heat with its cell (the discrete Newton-cooling form of `law.convective_flux`, the body
//! core-temperature exchange the body arc deferred until a located world existed, which now exists).
//! Everything walks in canonical id or coordinate order and reads no camera, so a run reproduces bit
//! for bit and is thread-count invariant (Principles 3 and 10).
//!
//! The agent cognition tick is composed onto this spine: a runner built with [`Runner::with_world`]
//! owns a [`crate::world::World`] and runs [`crate::world::World::tick`] as a fixed sub-phase after
//! the field phases, in a pinned within-tick order (the field phases first, then cognition), so that a
//! later field-to-cognition coupling reads the same-tick thermal state. The two sides carry disjoint
//! mutable state and share only [`StableId`], so the composition is additive rather than a merge of
//! contended state, and the composite [`Runner::state_hash`] folds the field-side hash and then the
//! world's canonical hash in a pinned order. Honest limits held here for the cognition path: the
//! cognition [`World`] reads no field state yet (a being's temperature is not a percept to the
//! dialogue tier, and no dialogue move relocates a being), and the world hash does not yet fold the
//! being lifecycle (genomes, ages, affect) or the dialogue log, so the composite is not a complete
//! canonical hash of all being state until those later increments land. The field layer is one field
//! (temperature) so far, the pattern the moisture, wind, and resource fields follow.
//!
//! A second sub-phase couples the field to embodied behaviour on the evolved-controller substrate, the
//! physics-in-and-behaviour-out path (Part 8.4, R-BEHAVIOR-EVOLVE). A runner built with
//! [`Runner::with_embodiment`] owns an [`Embodiment`]: a population of located beings whose movement is
//! driven by their evolved controllers ([`crate::locomotion`], [`crate::controller`]), not an authored
//! policy. Each tick, after the field step and the body-thermal exchange, three pure reads feed the
//! being's controller: a comfort-band map ([`comfort_fraction`]) turns its absolute core temperature
//! into a temperature homeostatic reserve in `[0, 1]` (its physiological state, per being from its own
//! reserved comfort band); the raw temperature gradient at its cell ([`Field::gradient_at`], the unit
//! direction toward warmer surroundings, what a thermoreceptor senses) is its directional percept; and
//! the signed deviation of its core temperature from its set point ([`signed_deviation`], too hot
//! positive, too cold negative) is its interoceptive thermoreceptor, the bit the even comfort reserve
//! cannot carry. The controller reads all three and issues a movement affordance; the beings' new
//! coordinates re-sync the located index so the next tick's thermal exchange reads where they moved.
//! This closes the loop from physics to physiology to behaviour to physics with no authored heading:
//! the gradient and the signed deviation are physical quantities, not a policy, and how a being
//! combines "am I too hot or too cold" (its signed thermoreceptor) with "which way is warmer" (the
//! gradient) is its evolved controller's, selected by survival. A being whose controller has evolved
//! to climb the gradient reaches warmth directly; one that has not explores (an undirected, seed-keyed
//! heading), and directed thermotaxis is the emergent consequence rather than a wired rule. The
//! comfort band's set point and half-range are reserved per-race physiology (Part 20); the composite
//! hash folds each being's position, reserves, and controller state after the world fold. The signed
//! thermoreceptor is what lets a controller flee lethal heat as well as seek warmth: the two demand
//! opposite gradient-following signs, which the even reserve cannot gate but the signed percept can,
//! and combining the signed bit with the gradient to flee is a product a recurrent controller
//! represents. That selection wires it, hot-fleeing emerging without an authored heading, is proven in
//! the [`evolve`](mod@crate::evolve) module. Honest limit: the signed percept lifts the linear-warmth-seeking ceiling, but
//! the bidirectional gating needs the recurrent controller, so a reaction norm still cannot solve a
//! world with both lethal-hot and lethal-cold regions.
//!
//! The steering boundary the canonical runner holds (Principle 9): the world phases it drives are the
//! emergent, data-driven ones (belief, dialogue, gossip, language), and the emergent-behaviour source
//! for a located being is the evolved controller (Part 8.4), not an authored policy. The world's
//! `decide` phase runs an AUTHORED drive-and-action repertoire only when one is installed
//! ([`crate::world::World::set_behaviour`]); that is the sentient deliberative tier of Part 8.1, which
//! Part 8.4 names as steering at the level of behaviour, and it must not ride the canonical spine as
//! if it were emergent. [`Runner::with_world`] therefore refuses a world that carries an authored
//! repertoire, so the authored path is quarantined off the canonical-emergent runner until the
//! deliberative tier is properly built on the evolved substrate.

use crate::anatomy::{BodyPlan, BodyPlanRegistry};
use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::controller::{Controller, ControllerLayout};
use crate::edibility::{Physiology, ToleranceRegistry};
use crate::environ::{EnvironCalib, EnvironFields};
use crate::homeostasis::{
    is_harm_tick, AffordanceId, AffordanceRegistry, DerivedDrain, Homeostasis, HomeostaticAxisId,
    HomeostaticRegistry, CONDITION, CRAFT, DIG, ENERGY, EXTRACT, GEOPHAGE, GRASP, INTEGRITY,
    RELEASE, RESPIRATION, TEMPERATURE,
};
use crate::learn::{
    avoidance_gradient, feature_observations, HarmLearningCalib, BENIGN, HARMS, HARM_ATTR,
};
use crate::located::{LocationIndex, OccupantId};
use crate::locomotion::{self, LocomotionParams, ResourceField, Terrain, Walker};
use crate::material::{
    CombustionCalib, CraftParams, EarthworkField, ExtractionParams, FireField, MaterialField,
    WieldedTool,
};
use crate::medium;
use crate::morphogen::{express_program, grow, Structure};
use crate::percept::PerceptRegistry;
use crate::physiology::{
    self, base_drain_from, body_exchange_rate_from, derive_body_exchange_rate,
    derive_exertion_coupling, MetabolicAnchors,
};
use crate::scenario::ScenarioResolution;
use crate::world::{PlaceId, Stimulus, TickInput, World};
use civsim_compose::FunctionLawRegistry;
use civsim_core::schedule::{run_serial, schedule, Access, ResourceId, SystemId};
use civsim_core::{Fixed, StableId, StateHasher};
use civsim_physics::laws;
use civsim_physics::PhysicsRegistry;
use civsim_world::{Coord3, TileMap};
use std::collections::{BTreeMap, BTreeSet};

// The runner tick's phases as deterministic-scheduler systems over the resources they contend for
// (design Part 57). The resource ids name the field, the body-temperature map, the located index,
// and the cognition world; the system ids are the canonical order the scheduler tie-breaks on.
const RES_FIELD: ResourceId = ResourceId(0);
const RES_BODY: ResourceId = ResourceId(1);
const RES_INDEX: ResourceId = ResourceId(2);
const RES_WORLD: ResourceId = ResourceId(3);
// The union population a being that is at once an Embodiment walker and a World mind belongs to
// (real-world unification, step 2). Declared as a write of BOTH the embodiment coupling and the
// cognition world, so once a shared StableId carries a body and a mind the scheduler serializes the
// two systems (in canonical SystemId order, matching the pinned step_inner order) rather than
// co-batching them as it safely does while world and embodiment share no being.
const RES_BEING: ResourceId = ResourceId(4);
const SYS_FIELD: SystemId = SystemId(0);
const SYS_BODY: SystemId = SystemId(1);
const SYS_EMBODIMENT: SystemId = SystemId(2);
const SYS_WORLD: SystemId = SystemId(3);

// Base-level liveliness step 5 (conversation liveliness): the movement coupling and an environment-
// sourced belief. The runner republishes each being's live cell into the cognition world each tick so
// gossip and converse cluster by where a being stands, and injects a first-order belief about the salt-
// flat hazard it discovers underfoot, so a fact found in one place rides gossip and a migrant into
// another band's cell.

/// The offset a live cell coordinate is mapped into a conversational [`PlaceId`] by, `base + y*width + x`,
/// so a cell-derived conversational place never collides with a small dawn-band `PlaceId` (the frozen
/// lineage ids). The mapping is a stable function of the coordinate (determinism, R-RNG-COORD).
const CELL_PLACE_BASE: u32 = 1_000_000;

/// The base tick-input ordinal an experientially-learned feature observation carries (harm-learning arc
/// slice b), one per present feature channel, high so it orders after any external input to the same
/// mind (determinism: the tick sorts inputs by mind then ordinal). Replaces the retired hazard ordinal:
/// the being now observes the raw features it senses rather than a single injected hazard.
const LEARN_ORDINAL_BASE: u32 = 1_000_000;

// The arc-scoped, default-generous promotion policy (base-level liveliness §4): promotion is the
// RESOLUTION KNOB on the story, not a scarce optimization, so it defaults GENEROUS. A being whose
// survival is at stake (a metabolic reserve or its condition worn low) is in a narrative arc (a struggle
// to eat, a battle with a lethal salt flat), and the runner promotes it, and every being sharing its
// live cell (the talk-hole guard: a promoted being needs promoted partners or it neither gossips nor
// converses), to the individual move-by-move dialogue tier for the duration of that arc, restricting it
// back to the aggregate when its arc resolves (its reserves recover). Emergent (Principle 9: the arc is
// the being's own state, never a scripted hero), deterministic (a canonical-order threshold and a
// stable ranking, no RNG), and cheap (promote/restrict is exact and conserved, design Part 54).

/// The reserved calibrations of the base-level liveliness surfacing policy (§4 and step 5): the numbers
/// that gate and weight the two run-path story hooks, the environment-sourced hazard belief and the
/// arc-scoped promotion. Each value gates or weights world content (a belief that propagates, which
/// beings converse), so none is a hardcoded inline constant: the mechanism is fixed Rust, the magnitudes
/// are reserved-with-basis in the manifest (Principle 11), read fail-loud by [`Self::from_manifest`]. The
/// labelled dev fixture [`Self::dev_default`] stands the same numbers up for the test and harness paths
/// that construct a runner without a manifest, so those paths are unchanged.
#[derive(Clone, Copy, Debug)]
pub struct LivelinessCalib {
    /// The survival-margin level below which a being is in an arc and is promoted to the individual tier.
    /// Manifest home `promotion.stress_threshold`; basis: the fraction of a reserve at which a being is
    /// meaningfully struggling, the generous default half a reserve.
    pub promotion_stress_threshold: Fixed,
    /// The maximum beings promoted to the individual tier at once, a performance bound. Manifest home
    /// `promotion.budget`; basis: the per-tick individual-dialogue cost the frame budget allows,
    /// defaulting high (liveliness over frugality, the owner ruling) so the aggregate tier absorbs the rest.
    pub promotion_budget: usize,
}

impl LivelinessCalib {
    /// Read the surfacing-policy values fail-loud from the manifest (Principle 11): a reserved value
    /// left unset refuses to build rather than running on a fabricated default. The budget is stored as a
    /// fixed-point count and truncated to its integer part. The belief-formation calibrations moved to
    /// [`HarmLearningCalib`] when the injected hazard belief was retired for the associative learner.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<LivelinessCalib, CalibrationError> {
        let budget = m.require_fixed("promotion.budget")?;
        Ok(LivelinessCalib {
            promotion_stress_threshold: m.require_fixed("promotion.stress_threshold")?,
            promotion_budget: (budget.to_bits() >> Fixed::FRAC_BITS).max(0) as usize,
        })
    }

    /// A labelled DEVELOPMENT FIXTURE standing up the same magnitudes the manifest would carry, for the
    /// test and harness paths that build a runner without a manifest. Half a reserve is the generous
    /// stress default, and a budget of 64 the high default the aggregate tier makes affordable.
    pub fn dev_default() -> LivelinessCalib {
        LivelinessCalib {
            promotion_stress_threshold: Fixed::from_bits(1i64 << (Fixed::FRAC_BITS - 1)), // 1/2
            promotion_budget: 64,
        }
    }
}

/// A declared access from resource-id slices (a small local convenience over [`Access::new`]).
fn access(reads: &[ResourceId], writes: &[ResourceId]) -> Access {
    Access {
        reads: reads.iter().copied().collect::<BTreeSet<_>>(),
        writes: writes.iter().copied().collect::<BTreeSet<_>>(),
    }
}

/// The reserved field-layer calibrations. There is deliberately no `Default`: on a canonical run
/// these are read from the manifest and are fail-loud if unset, and a test must name each as a
/// labelled fixture. None is an agent-set number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldCalib {
    /// The per-tick diffusion (conduction) coefficient, dimensionless, in `[0, 0.25]` for the
    /// four-neighbour stencil's stability bound. The bound is inclusive: the von Neumann limit is
    /// `alpha * dt / dx^2 <= 1/4`, and [`derive_field_diffusion`] clamps to [`STENCIL_STABILITY_BOUND`]
    /// (exactly 1/4) inclusively. Basis: the medium's thermal diffusivity over the cell size and the
    /// base tick, kept at or below the explicit stability limit.
    pub diffusion: Fixed,
    /// The per-tick relaxation rate of a cell toward its baseline (the solar and biome forcing), in
    /// `[0, 1]`. Basis: the day-night and seasonal forcing timescale over the base tick.
    pub relaxation: Fixed,
    /// The per-tick body-to-environment convective coupling, in `[0, 1]`. Basis: the fluids-floor
    /// convective coefficient and the body surface-to-thermal-mass ratio (`law.convective_flux`),
    /// expressed as the discrete Newton-cooling rate.
    pub exchange: Fixed,
}

impl FieldCalib {
    /// The field calibrations read from the calibration manifest, fail-loud if any is still reserved
    /// (Principle 11, the reserved-value discipline): `field.diffusion`, `field.relaxation`, and
    /// `field.body_exchange`. This is the sanctioned way to obtain a [`FieldCalib`] on a canonical run;
    /// there is deliberately no default, so an unset value refuses to run rather than fabricating a
    /// number. A test may instead name each as a labelled fixture.
    pub fn from_manifest(manifest: &CalibrationManifest) -> Result<FieldCalib, CalibrationError> {
        Ok(FieldCalib {
            diffusion: manifest.require_fixed("field.diffusion")?,
            relaxation: manifest.require_fixed("field.relaxation")?,
            exchange: manifest.require_fixed("field.body_exchange")?,
        })
    }

    /// The field calibrations with the diffusion coefficient DERIVED from the world's selected medium
    /// (design Part 5.4/5.5; the owner's ruling that the medium is the lever and the diffusivity is
    /// physics). The medium's three thermal axes (`conductivity`, `density`, `specific_heat`) are read
    /// from its `require_map` profile (`medium.{name}`), the cell size from the reserved
    /// `field.cell_size`, and the timestep from `time.base_tick_seconds`; the diffusion coefficient is
    /// then [`derive_field_diffusion`] of those, so the field's conduction rate is not a free scalar
    /// but a consequence of which substance fills the world. The relaxation and body-exchange
    /// calibrations are read as before. Fail-loud throughout: while the medium profile or the cell
    /// size is reserved this refuses to run, so no fabricated diffusivity reaches canonical state
    /// (Principle 11). The medium selection is the caller's (the scenario resolves `medium.{name}`);
    /// this reads no medium label, only the thermal axes, so a world of air and a world of water
    /// diverge from their physics, never a branch (Principle 9).
    pub fn from_manifest_with_medium(
        manifest: &CalibrationManifest,
        medium_id: &str,
    ) -> Result<FieldCalib, CalibrationError> {
        let profile = manifest.require_map(medium_id)?;
        let axis = |name: &str| -> Result<Fixed, CalibrationError> {
            profile
                .get(name)
                .copied()
                .ok_or_else(|| CalibrationError::BadValue {
                    id: medium_id.to_string(),
                    detail: format!("medium profile is missing the '{name}' thermal axis"),
                })
        };
        let conductivity = axis("conductivity")?;
        let density = axis("density")?;
        let specific_heat = axis("specific_heat")?;
        let cell_size = manifest.require_fixed("field.cell_size")?;
        let dt = manifest.require_fixed("time.base_tick_seconds")?;
        Ok(FieldCalib {
            diffusion: derive_field_diffusion(conductivity, density, specific_heat, cell_size, dt),
            relaxation: manifest.require_fixed("field.relaxation")?,
            exchange: manifest.require_fixed("field.body_exchange")?,
        })
    }

    /// The field calibrations for a resolved scenario: the world-build path's field constructor
    /// (design Part 5.4/5.5). The diffusion coefficient DERIVES from the scenario's selected medium
    /// through [`FieldCalib::from_manifest_with_medium`], so a world's field conducts at its medium's
    /// physics rate (`k/(rho*c)`) and the free-scalar `field.diffusion` an `[environment]` block may
    /// push is retired on this path entirely: the medium is the lever and the diffusivity is physics
    /// (the owner's ruling). A scenario that names no medium is the documented default temperate air,
    /// which resolves to the `medium.air` physics profile
    /// ([`ScenarioResolution::medium_manifest_id`]), so even an air-default world derives its diffusion
    /// from air's real k/rho/c rather than a fabricated number, and no world on this path reads a free
    /// diffusion scalar (Principle 9, Principle 11). The relaxation and body-exchange calibrations are
    /// read from the manifest as before. Fail-loud throughout: a reserved or missing medium profile,
    /// cell size, or timestep refuses to build, so no fabricated calibration reaches canonical state.
    pub fn from_resolution(
        manifest: &CalibrationManifest,
        resolution: &ScenarioResolution,
    ) -> Result<FieldCalib, CalibrationError> {
        FieldCalib::from_manifest_with_medium(manifest, resolution.medium_manifest_id())
    }
}

/// The explicit two-dimensional four-neighbour diffusion stencil's stability bound, `1/4`: an
/// explicit forward-Euler diffusion step is stable only for `alpha * dt / dx^2 <= 1/4` on this
/// stencil (the von Neumann stability limit; Press et al., Numerical Recipes). This is a numerics law
/// constant, not world content: it is the mathematics of the discretization, so it is fixed in code
/// rather than reserved (Principle 11 governs world content, not the stencil's own stability limit).
const STENCIL_STABILITY_BOUND: Fixed = Fixed::from_bits(1 << (Fixed::FRAC_BITS - 2));

/// The representability cap on a derived thermal diffusivity (m^2/s) passed to
/// [`laws::thermal_diffusivity`]. No real medium's diffusivity approaches one square metre per second
/// (silver, among the highest, is about 1.7e-4), so a cap of one is a pure overflow guard that never
/// binds on a real substance; it exists so a degenerate zero-heat-capacity medium saturates rather
/// than dividing unbounded.
const DIFFUSIVITY_MAX: Fixed = Fixed::ONE;

/// Derive the field's dimensionless diffusion coefficient from a medium's thermal properties (design
/// Part 5.4/5.5): the medium's thermal diffusivity `alpha = k / (rho * c)` (through
/// [`laws::thermal_diffusivity`]) times the timestep over the squared cell size, `alpha * dt / dx^2`,
/// the explicit-stencil coefficient, clamped to the four-neighbour stencil's stability bound
/// ([`STENCIL_STABILITY_BOUND`]). A canonical cell size keeps the physical coefficient well below the
/// bound (heat does not conduct across a map cell in one base tick), so the clamp is a stability rail
/// rather than the operating point; it guarantees the derived value can never destabilize the stencil
/// regardless of the medium selected. Pure fixed-point and deterministic: the physics divide, one
/// multiply, one divide, and a clamp, no float and no RNG. A zero cell size (a degenerate scale)
/// reads a zero coefficient rather than dividing by zero. Reads no medium label, only its three
/// thermal axes, so two media diverge from their physics alone (Principle 9).
pub fn derive_field_diffusion(
    conductivity: Fixed,
    density: Fixed,
    specific_heat: Fixed,
    cell_size: Fixed,
    dt: Fixed,
) -> Fixed {
    let alpha = laws::thermal_diffusivity(conductivity, density, specific_heat, DIFFUSIVITY_MAX);
    let cell_area = cell_size.mul(cell_size);
    if cell_area == Fixed::ZERO {
        return Fixed::ZERO;
    }
    let coefficient = alpha.mul(dt).div(cell_area);
    coefficient.clamp(Fixed::ZERO, STENCIL_STABILITY_BOUND)
}

/// A canonical scalar temperature field over the flat bounded map, Q32.32 on the `therm.temperature`
/// axis. The boundary is clamped (zero-flux Neumann): an edge cell's missing neighbour is itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    width: i32,
    height: i32,
    temp: Vec<Fixed>,
    baseline: Vec<Fixed>,
}

impl Field {
    /// A field from explicit per-cell baseline temperatures (row-major, `width * height` long). The
    /// initial field equals the baseline. Tests use this with a labelled fixture; a calibrated run
    /// uses [`Field::from_map`].
    pub fn new(width: i32, height: i32, baseline: Vec<Fixed>) -> Field {
        assert!(width > 0 && height > 0, "a field has positive extent");
        assert_eq!(
            baseline.len(),
            (width as usize) * (height as usize),
            "the baseline covers every cell"
        );
        Field {
            width,
            height,
            temp: baseline.clone(),
            baseline,
        }
    }

    /// The field seeded from a generated map's per-tile temperatures (the baseline it relaxes toward).
    /// The map's worldgen calibration is the caller's concern: owner-set on a canonical run, a
    /// labelled fixture in a test. This function fabricates nothing.
    pub fn from_map(map: &TileMap) -> Field {
        let topo = map.topo();
        let (w, h) = (topo.width, topo.height);
        let mut baseline = Vec::with_capacity((w as usize) * (h as usize));
        for y in 0..h {
            for x in 0..w {
                let t = map
                    .tile(Coord3::new(x, y, 0))
                    .map(|t| t.temperature)
                    .expect("every in-bounds cell has a tile");
                baseline.push(t);
            }
        }
        Field::new(w, h, baseline)
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    /// The temperature at a cell.
    pub fn at(&self, x: i32, y: i32) -> Fixed {
        self.temp[self.idx(x, y)]
    }

    /// The field extent.
    pub fn dims(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// The raw temperature gradient at a cell: the central difference over the four clamped neighbours
    /// (the same zero-flux Neumann boundary [`Field::step`] uses), `(at(x+1,y) - at(x-1,y), at(x,y+1) -
    /// at(x,y-1))`. Positive components point toward warmer cells. This is pure integer subtraction (no
    /// `Fixed::mul`, no division, no RNG), the same per-cell stencil class as [`Field::step`] and
    /// `crates/gpu`'s field kernel, so it is bit-identical on every machine and thread count and ports
    /// unchanged to a CubeCL `#[cube]` kernel. A cell off the field reads a zero gradient. The caller
    /// normalises to a unit direction (a cheap per-being step), keeping this kernel add-and-subtract
    /// only. It is a physical field quantity, not a heading: what a thermoreceptor senses, not a policy.
    pub fn gradient_at(&self, x: i32, y: i32) -> (Fixed, Fixed) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return (Fixed::ZERO, Fixed::ZERO);
        }
        let xl = if x > 0 { x - 1 } else { x };
        let xr = if x < self.width - 1 { x + 1 } else { x };
        let yu = if y > 0 { y - 1 } else { y };
        let yd = if y < self.height - 1 { y + 1 } else { y };
        (
            self.at(xr, y) - self.at(xl, y),
            self.at(x, yd) - self.at(x, yu),
        )
    }

    /// One canonical step: the fixed-point diffusion-and-relaxation stencil. Each product is the
    /// pinned `Fixed::mul` (floor), the neighbour sum is exact integer addition, and the clamped
    /// boundary is deterministic, so the step is bit-identical on every machine and thread count and
    /// ports unchanged to a CubeCL `#[cube]` kernel.
    pub fn step(&mut self, c: &FieldCalib) {
        let (w, h) = (self.width, self.height);
        let mut next = self.temp.clone();
        for y in 0..h {
            for x in 0..w {
                let i = self.idx(x, y);
                let cur = self.temp[i];
                let up = self.temp[self.idx(x, if y > 0 { y - 1 } else { y })];
                let dn = self.temp[self.idx(x, if y < h - 1 { y + 1 } else { y })];
                let lf = self.temp[self.idx(if x > 0 { x - 1 } else { x }, y)];
                let rt = self.temp[self.idx(if x < w - 1 { x + 1 } else { x }, y)];
                let lap = up + dn + lf + rt - Fixed::from_int(4).mul(cur);
                let relax = self.baseline[i] - cur;
                next[i] = cur + c.diffusion.mul(lap) + c.relaxation.mul(relax);
            }
        }
        self.temp = next;
    }

    fn hash(&self, h: &mut StateHasher) {
        h.write_i64(self.width as i64);
        h.write_i64(self.height as i64);
        for t in &self.temp {
            h.write_fixed(*t);
        }
    }
}

/// A located being's reserved thermal physiology: the viable core-temperature band the comfort-band
/// map reads, and the being's core temperature at spawn. Per being, so a world differentiates the band
/// by race (Principle 11): the mechanism is fixed, these values are data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeingThermal {
    /// The set point of the viable core-temperature band, the temperature at which comfort is full.
    /// RESERVED. Basis: the race's homeostatic core-temperature set point (Part 20 physiology).
    pub setpoint: Fixed,
    /// The half-range of the survivable band: comfort falls linearly from full at the set point to zero
    /// a half-range away, and a being carried a full half-range past the set point has fallen through
    /// its temperature floor and dies. RESERVED. Basis: the race's survivable core-temperature
    /// half-range around the set point (Part 20 death conditions).
    pub half_band: Fixed,
    /// The being's absolute core temperature at spawn, a physical state (not a reserved calibration),
    /// on the `therm.temperature` axis, from which the field-driven exchange proceeds.
    pub initial_temp: Fixed,
}

/// The comfort-band map: an absolute core temperature and a viable band to a temperature homeostatic
/// reserve fraction in `[0, 1]`. Full comfort (`ONE`) at the set point, falling linearly to zero a
/// half-range away and clamped there, so it is even in the deviation from the set point (a temperature
/// the same distance above or below the set point yields the same comfort) and authors no direction. A
/// pure fixed-point function: no RNG, no camera, and no notion of what a being should do about being
/// cold. A degenerate zero half-range reads as comfortable only exactly at the set point.
pub fn comfort_fraction(body_temp: Fixed, band: &BeingThermal) -> Fixed {
    let dev = (body_temp - band.setpoint).abs();
    if band.half_band <= Fixed::ZERO {
        return if dev == Fixed::ZERO {
            Fixed::ONE
        } else {
            Fixed::ZERO
        };
    }
    Fixed::ONE - dev.div(band.half_band).clamp(Fixed::ZERO, Fixed::ONE)
}

/// The signed thermoreceptor: an absolute core temperature and a viable band to a signed deviation in
/// `[-1, 1]`, positive when the body is above its set point (too hot), negative when below (too cold),
/// zero at the set point. This is the odd (sign-preserving) counterpart of the even [`comfort_fraction`]:
/// where comfort collapses hot and cold to one magnitude, this carries the bit that distinguishes them,
/// the raw interoceptive percept a being needs to tell overheating from freezing. It is scaled by the
/// half-range and clamped, so it saturates to `+1`/`-1` at the lethal edges, and it is exactly odd in
/// the deviation (a body the same distance above or below the set point reads equal and opposite),
/// so it authors no direction and favours neither hot nor cold (Principle 9). A pure fixed-point
/// function: no RNG, no camera. A degenerate zero half-range reads as the pure sign of the deviation.
pub fn signed_deviation(body_temp: Fixed, band: &BeingThermal) -> Fixed {
    let dev = body_temp - band.setpoint;
    if band.half_band <= Fixed::ZERO {
        return match dev.to_bits().cmp(&0) {
            std::cmp::Ordering::Greater => Fixed::ONE,
            std::cmp::Ordering::Less => Fixed::from_int(-1),
            std::cmp::Ordering::Equal => Fixed::ZERO,
        };
    }
    dev.div(band.half_band)
        .clamp(Fixed::from_int(-1), Fixed::ONE)
}

/// Normalise a raw gradient to a unit direction, the same way the known-source percept does (parity
/// with `crate::locomotion` source_dirs): divide by the magnitude and clamp to `[-1, 1]`. A flat
/// gradient (zero magnitude) reads as no direction, so the being has no thermal heading to follow and
/// explores for that axis. This is the cheap per-being step that keeps [`Field::gradient_at`] itself
/// add-and-subtract only for the GPU path.
fn unit(dx: Fixed, dy: Fixed) -> (Fixed, Fixed) {
    let dist = (dx.mul(dx) + dy.mul(dy)).sqrt();
    if dist > Fixed::ZERO {
        let lo = Fixed::from_int(-1);
        (
            dx.div(dist).clamp(lo, Fixed::ONE),
            dy.div(dist).clamp(lo, Fixed::ONE),
        )
    } else {
        (Fixed::ZERO, Fixed::ZERO)
    }
}

/// A bounded open plane the size of the field: every in-bounds tile is passable at unit cost, and a
/// tile off the field is impassable, so a being stays on the field its thermal exchange reads. Pure
/// physics (a passability-and-cost gate), no route and no behaviour; the map-backed terrain with real
/// biomes and elevation is the located-world increment that follows.
struct BoundedPlane {
    width: i32,
    height: i32,
}

impl Terrain for BoundedPlane {
    fn passable(&self, c: Coord3, _body: &BodyPlan) -> bool {
        c.x >= 0 && c.x < self.width && c.y >= 0 && c.y < self.height
    }

    fn cost(&self, _c: Coord3) -> Fixed {
        Fixed::ONE
    }
}

/// A representability cap on the respiration Fick flux (a normalised-concentration bound), the
/// engine-mechanics exemption the physiology and medium kernels take (matching
/// [`crate::physiology`]'s `FLUX_MAX`), not an owner value: it exists so a degenerate input saturates
/// rather than overflowing, and it never binds on a physical medium.
const RESPIRATION_FLUX_MAX: Fixed = Fixed::from_int(1_000_000_000);

/// The per-embodiment physiology configuration that makes the anatomy-derived metabolism LIVE
/// (R-METABOLIZE, design Part 15, Part 20, Part 35, Part 41; Principles 9, 11). Installed on an
/// [`Embodiment`] through [`Embodiment::set_physiology`], it switches the embodiment's beings from the
/// labelled scalar `metabolize` to the physics-derived producers: the per-being resting drain
/// ([`derive_base_drain`], the Kleiber basal rate plus the thermoregulatory replacement), the exertion
/// coupling ([`derive_exertion_coupling`]), the body-to-medium exchange rate ([`derive_body_exchange_rate`]),
/// and, where the physiology registry carries a [`RESPIRATION`] axis, medium respiration
/// ([`crate::medium::respire_at`]). So two beings with different body plans diverge in survival from their
/// anatomy alone, with no race or label branch (Principle 9). The mechanism is fixed Rust; the organ
/// registry, the anchors, and the medium are data (Principle 11).
///
/// The medium is a spatially-varying per-cell [`crate::medium::MediumField`] (real-world unification step
/// 4): a being reads the medium of the cell it stands in, so a body in a water cell respires that cell's
/// content and one in an air cell that cell's, from the same coupling over different axis values. A
/// single-medium world folds to a uniform field (the regression), so the earlier uniform behaviour is the
/// one-sample case of this one.
#[derive(Clone, Debug)]
pub struct EmbodiedPhysiology {
    /// The organ registry a being's tissue composition (convective surface, specific heat, energy
    /// density, respiratory surface) is read against, the same registry [`crate::homeostasis::Homeostasis::new`]
    /// sizes the reserves from.
    organs: BodyPlanRegistry,
    /// The reserved owner metabolic anchors (Kleiber coefficient, kilogram bridge, medium convective
    /// coefficient, emissivity, Stefan-Boltzmann), read fail-loud from the manifest or a labelled
    /// dev fixture.
    anchors: MetabolicAnchors,
    /// The per-cell ambient-medium field, the `c_medium` the Fick respiration law reads at a being's
    /// coordinate ([`crate::medium::respire_at`]). Folded from the worldgen map (`medium.water` below the
    /// reserved submersion elevation, `medium.air` above) or a labelled uniform dev fixture.
    medium: medium::MediumField,
    /// The reserved Fick membrane transfer coefficient `k` the respiration law reads. RESERVED owner
    /// value (`metabolism.respiration_transfer_coefficient`), a labelled fixture in tests.
    respiration_transfer_k: Fixed,
    /// The base tick length in seconds the drain derivation integrates over (`time.base_tick_seconds`).
    tick_seconds: Fixed,
}

impl EmbodiedPhysiology {
    /// The physiology configuration read from the calibration manifest, fail-loud if any input is still
    /// reserved (Principle 11): the metabolic anchors ([`MetabolicAnchors::from_manifest`]), the per-cell
    /// medium field folded from the worldgen map ([`medium_field_from_manifest`], reading the two medium
    /// profiles' `respirable_content` and `density` axes and the reserved submersion elevation), the
    /// reserved respiration transfer coefficient, and the base tick length. The submerged and emergent
    /// medium ids are the caller's (the scenario resolves them: a grounded world holds `medium.water`
    /// below the waterline and `medium.air` above; a single-medium world passes the same id for both and
    /// folds to a uniform field). This is the sanctioned canonical sourcing; a test may instead build a
    /// labelled fixture with [`EmbodiedPhysiology::dev_fixture`].
    pub fn from_manifest(
        manifest: &CalibrationManifest,
        organs: BodyPlanRegistry,
        map: &TileMap,
        submerged_medium_id: &str,
        emergent_medium_id: &str,
    ) -> Result<EmbodiedPhysiology, CalibrationError> {
        let anchors = MetabolicAnchors::from_manifest(manifest)?;
        let medium =
            medium_field_from_manifest(manifest, map, submerged_medium_id, emergent_medium_id)?;
        Ok(EmbodiedPhysiology {
            organs,
            anchors,
            medium,
            respiration_transfer_k: manifest
                .require_fixed("metabolism.respiration_transfer_coefficient")?,
            tick_seconds: manifest.require_fixed("time.base_tick_seconds")?,
        })
    }

    /// A labelled DEVELOPMENT FIXTURE physiology (dev-fixture anchors, a caller-supplied medium field, a
    /// unit transfer coefficient, a one-second base tick), for tests and examples only; a canonical run
    /// reads [`EmbodiedPhysiology::from_manifest`]. Not owner canon. The caller builds the medium field
    /// (a [`crate::medium::MediumField::uniform`] for a one-medium fixture) sized to cover the beings'
    /// coordinates, since a being off the field finds no medium and cannot breathe.
    pub fn dev_fixture(
        organs: BodyPlanRegistry,
        medium: medium::MediumField,
    ) -> EmbodiedPhysiology {
        EmbodiedPhysiology {
            organs,
            anchors: MetabolicAnchors::dev_fixture(),
            medium,
            respiration_transfer_k: Fixed::ONE,
            tick_seconds: Fixed::ONE,
        }
    }
}

/// Fold the per-cell medium field from the worldgen map and the manifest (real-world unification step 4;
/// owner ruling 2026-07-04), fail-loud if the submersion elevation or either medium profile is still
/// reserved (Principle 11). Reads the reserved submersion elevation and each medium profile's
/// `respirable_content` and `density` axes, then assigns each cell its medium by physical elevation alone
/// ([`crate::medium::MediumField::from_map`]): the submerged medium below the threshold, the emergent
/// above, no biome-label branch (Principle 9). The submerged and emergent ids may be the same, folding to
/// a uniform field.
fn medium_field_from_manifest(
    manifest: &CalibrationManifest,
    map: &TileMap,
    submerged_medium_id: &str,
    emergent_medium_id: &str,
) -> Result<medium::MediumField, CalibrationError> {
    let submersion = manifest.require_fixed("medium.submersion_elevation")?;
    let submerged = medium_sample(manifest, submerged_medium_id)?;
    let emergent = medium_sample(manifest, emergent_medium_id)?;
    Ok(medium::MediumField::from_map(
        map, submersion, submerged, emergent,
    ))
}

/// Read one medium profile's [`crate::medium::MediumSample`] (the `respirable_content` and `density`
/// axes) from the manifest, fail-loud if the profile is reserved or missing either axis.
fn medium_sample(
    manifest: &CalibrationManifest,
    medium_id: &str,
) -> Result<medium::MediumSample, CalibrationError> {
    let profile = manifest.require_map(medium_id)?;
    let axis = |name: &str| -> Result<Fixed, CalibrationError> {
        profile
            .get(name)
            .copied()
            .ok_or_else(|| CalibrationError::BadValue {
                id: medium_id.to_string(),
                detail: format!("medium profile is missing the '{name}' axis"),
            })
    };
    Ok(medium::MediumSample {
        respirable: axis("respirable_content")?,
        density: axis("density")?,
    })
}

/// Standard gravity, the reference gravitational acceleration (NIST standard gravity 9.80665 m/s^2,
/// the terran default `mech.gravitational_acceleration` cites), the datum the carry-load weight physics
/// reads (material-substrate arc, cascade item 3). A cited physical constant, not a reserved tunable; a
/// per-world gravity override rides Part 40, a documented follow-on.
fn standard_gravity() -> Fixed {
    Fixed::from_ratio(980665, 100000)
}

/// The physics force ceiling the weight law saturates at: the `mech.force` axis maximum (1e8 N, the
/// mechanical floor's declared force range). A representability cap, not an authored quantity.
const FORCE_CEILING: Fixed = Fixed::from_int(100_000_000);

/// A being's whole-body muscle force, its carry capacity: a GROWN body sums its grown tissue's muscle
/// strength scaled to body mass, a catalog body reads it off its organs, the same read the exertion
/// drain uses ([`being_derived_drains`]). Blind to any kind or race id (Principle 9).
fn being_muscle_force(w: &Walker, phys: &EmbodiedPhysiology) -> Fixed {
    match &w.structure {
        Some(s) => s
            .composition_sum(physiology::MUSCLE_STRENGTH)
            .checked_mul(physiology::body_mass_kg(&w.body, &phys.anchors))
            .unwrap_or(Fixed::ZERO),
        None => physiology::whole_body_muscle_force(&w.body, &phys.organs, &phys.anchors),
    }
}

/// Build a being's per-axis DERIVED drain vector (R-METABOLIZE) from its anatomy against the installed
/// physiology. The metabolic axis (the one backed by the `bio.energy_density` floor axis, keyed off the
/// floor id rather than a hardcoded axis constant, so the choice is data not a special case) drains at
/// the Kleiber basal rate plus the thermoregulatory replacement ([`derive_base_drain`], read against the
/// live `ambient` and the being's `setpoint`) with a work-derived exertion coupling
/// ([`derive_exertion_coupling`]); every other axis keeps its authored per-axis rate from the registry
/// (water lost slower, an oxygen demand, or the zero-drain derived axes temperature and integrity), so
/// only the energy metabolism derives and the rest stay the owner's per-axis calibration. Pure
/// fixed-point, no RNG, and no identity read: two beings diverge from their body plans alone (Principle
/// 9). The exertion inputs are the being's full-exertion ground speed (a body-plan-derived velocity)
/// and its whole-body muscle work force ([`physiology::whole_body_muscle_force`], the Part 35 datum
/// that retires the earlier normalized-mass proxy): the force a body exerts follows its muscle anatomy,
/// so two bodies of equal mass but different muscle endowment now drain differently under exertion, and
/// a body with no muscle tissue exerts no force (the absence convention, not a mass-sized default).
fn being_derived_drains(
    emb: &Embodiment,
    phys: &EmbodiedPhysiology,
    w: &Walker,
    ambient: Fixed,
    setpoint: Fixed,
) -> BTreeMap<HomeostaticAxisId, DerivedDrain> {
    let mut map = BTreeMap::new();
    for axis in &emb.homeo.axes {
        let drain = if axis.backing_component.as_deref() == Some(physiology::ENERGY_DENSITY) {
            let cap = w.homeostasis.capacity(axis.id);
            // The composition scalars the derived drain reads: a GROWN body reads its own grown tissue
            // directly (emergent-anatomy Step 3, the metabolic and derived-physiology grow), a catalog body
            // its organs, so a fully grown body metabolizes and thermoregulates off its grown tissue and
            // needs no catalog organs. The energy density and exposed surface both follow the body a being
            // actually carries; the muscle force is the grown strength summed over the tissue, scaled to mass.
            let (energy_density, surface, force) = match &w.structure {
                Some(s) => (
                    s.whole_body_energy_density(),
                    s.composition_sum(physiology::CONVECTIVE_SURFACE),
                    s.composition_sum(physiology::MUSCLE_STRENGTH)
                        .checked_mul(physiology::body_mass_kg(&w.body, &phys.anchors))
                        .unwrap_or(Fixed::ZERO),
                ),
                None => (
                    physiology::whole_body_energy_density(&w.body, &phys.organs),
                    physiology::whole_body_surface(&w.body, &phys.organs),
                    physiology::whole_body_muscle_force(&w.body, &phys.organs, &phys.anchors),
                ),
            };
            let base = base_drain_from(
                &w.body,
                cap,
                energy_density,
                surface,
                ambient,
                setpoint,
                phys.anchors.medium_h,
                phys.tick_seconds,
                &phys.anchors,
            );
            // A grown body's exertion velocity reads its grown limb; a catalog body's the registry mode
            // (emergent-anatomy Step 2), so the derived drain follows the body a being actually carries.
            let velocity = match &w.structure {
                Some(s) => locomotion::locomotion_speed_structure(
                    s,
                    w.body.temperament.activity,
                    Fixed::ONE,
                    &emb.params,
                ),
                None => {
                    locomotion::locomotion_speed(&w.body, &phys.organs, Fixed::ONE, &emb.params)
                }
            };
            let exertion = derive_exertion_coupling(
                &w.body,
                cap,
                energy_density,
                force,
                velocity,
                phys.tick_seconds,
                &phys.anchors,
            );
            DerivedDrain { base, exertion }
        } else {
            DerivedDrain {
                base: axis.base_drain,
                exertion: axis.exertion_drain,
            }
        };
        map.insert(axis.id, drain);
    }
    map
}

/// The body-to-medium thermal exchange rate for a being, reading its GROWN tissue's exposed surface and
/// specific heat directly when it carries a grown structure (emergent-anatomy Step 3, the derived-physiology
/// grow), and its catalog organs otherwise. So a fully grown body couples to the medium off its own tissue,
/// with no catalog organs; a catalog body is byte-identical to the prior read.
fn walker_exchange_rate(
    body: &BodyPlan,
    structure: &Option<Structure>,
    phys: &EmbodiedPhysiology,
) -> Fixed {
    match structure {
        Some(s) => body_exchange_rate_from(
            body,
            s.composition_sum(physiology::CONVECTIVE_SURFACE),
            s.composition_mean(physiology::TISSUE_SPECIFIC_HEAT),
            phys.anchors.medium_h,
            phys.tick_seconds,
            &phys.anchors,
        ),
        None => derive_body_exchange_rate(
            body,
            &phys.organs,
            phys.anchors.medium_h,
            phys.tick_seconds,
            &phys.anchors,
        ),
    }
}

/// The embodied-being population coupled to the field on the evolved-controller substrate (Part 8.4,
/// R-BEHAVIOR-EVOLVE). It owns the located beings ([`Walker`], each with its evolved controller), the
/// data-defined physiology and affordance registries and the controller layout derived from them, the
/// movement-physics parameters, the resource field the beings perceive, per-being reserved thermal
/// bands, and the locomotion RNG seed. The mechanism is fixed Rust; the controllers, the registries,
/// the bands, and the parameters are data (Principle 11). The runner ticks this as a sub-phase after
/// the field, and the embodiment never reaches back into the field beyond the coordinates it publishes.
pub struct Embodiment {
    walkers: Vec<Walker>,
    thermal: BTreeMap<StableId, BeingThermal>,
    homeo: HomeostaticRegistry,
    afford: AffordanceRegistry,
    /// The organ registry a being's part kinds are looked up in, so an affordance and the ground speed are
    /// DERIVED from each part's grown geometry and material through the function-law dispatch, blind to any
    /// kind or race id (emergent-anatomy step one). A labelled dev fixture by default ([`Embodiment::new`]);
    /// the world-build installs the world's own registry ([`Embodiment::set_organs`]), the same one the
    /// physiology reads, so the two agree.
    organs: BodyPlanRegistry,
    layout: ControllerLayout,
    params: LocomotionParams,
    resources: ResourceField,
    seed: u64,
    /// The anatomy-derived physiology, when installed ([`Embodiment::set_physiology`]). `Some` switches
    /// the embodiment's beings onto the R-METABOLIZE producers (derived drain, body-medium exchange,
    /// respiration); `None` keeps the labelled scalar metabolize (the evolve harness and the existing
    /// thermal fixtures), so installing the physiology is opt-in and disturbs no existing caller.
    physiology: Option<EmbodiedPhysiology>,
    /// The toxin-tolerance registry a newborn's heritable tolerance is expressed from at the lifecycle
    /// pairing beat (base-level liveliness step 4). Empty by default (no tolerance, the harm sink inert),
    /// set by the world-build ([`Embodiment::set_tolerances`]) so a child inherits its parents' salt (or
    /// dust) resistance through its own genome, the same way the founder step expresses it.
    tolerances: ToleranceRegistry,
    /// The perceived-feature registry the beings sense underfoot (harm-learning arc slice a). EMPTY by
    /// default, so the controller layout carries no feature block and every run hash is unchanged; the
    /// world-build installs a non-empty registry ([`Embodiment::set_percepts`], which rebuilds the
    /// layout to feed the feature block) to opt a world into the feature percept. The membership is the
    /// biology-floor's data, so a new sensible feature is a data edit, never a code change.
    percepts: PerceptRegistry,
    /// The located material substrate the world is made of (material-substrate arc, cascade item 1):
    /// a per-cell mixture of physics substances by volume. EMPTY by default, so a scenario that
    /// declares no material layer folds nothing into `state_hash` and replays bit-for-bit (the opt-in
    /// empty default). The world-build populates it ([`Embodiment::set_material`]) to opt a world into
    /// matter; nothing on the run path reads its derived hardness or density yet (that arrives with the
    /// extraction contest), so this slice folds the substrate into the canonical state without a
    /// consumer.
    material: MaterialField,
    /// The physics registry a carried or worked load's weight and (later) hardness are DERIVED against
    /// (material-substrate arc, cascade item 3): the world material registry
    /// ([`civsim_physics::PhysicsRegistry::ground`]). `None` by default, so an embodiment that declares
    /// no material registry cannot pick up matter (the carry actions no-op) and every existing scenario
    /// is unchanged. Opt-in via [`Embodiment::set_material_registry`], the first run-path consumer of the
    /// derived material properties.
    material_registry: Option<PhysicsRegistry>,
    /// The reserved parameters of the extraction contest (material-substrate arc, cascade item 4): the
    /// working area a being's force presses over and the pressure cap. `None` by default, so an embodiment
    /// that declares no extraction parameters cannot mine (the extract action no-ops) and every existing
    /// scenario is unchanged. Opt-in via [`Embodiment::set_extraction_params`].
    extraction: Option<ExtractionParams>,
    /// The reserved parameters of the crafting contest (material-substrate arc, cascade item 4, knapping):
    /// the working-edge area a knapped tool presents and the volume of carried matter it consumes. `None`
    /// by default, so an embodiment that declares no crafting parameters cannot make a tool (the craft
    /// action no-ops) and every existing scenario is unchanged. Opt-in via [`Embodiment::set_craft_params`].
    craft: Option<CraftParams>,
    /// The per-column earthwork delta a being's digging has made to the terrain (material-substrate arc,
    /// cascade item 5, modifiable terrain). EMPTY by default, so a scenario where nothing digs folds no
    /// bytes into `state_hash` and stays byte-identical (the opt-in empty-default). Digging lowers a column
    /// (a pit) and yields spoil; a deposit raises it (a mound), reshaping the terrain the physics reads.
    earthwork: EarthworkField,
    /// The per-cell fire intensity a combustion beat sources over the burning cells (material-substrate arc,
    /// cascade item 6, live fire): the combustion energy each cell of combustible matter releases this tick.
    /// UNLIT by default, so a scenario with no combustion armed (or no combustible substance hot enough)
    /// folds no bytes into `state_hash` and stays byte-identical (the opt-in empty-default). Recomputed each
    /// tick from the fuel present and the cell temperature, so it always reflects the current combustion.
    fire: FireField,
}

impl Embodiment {
    /// A new, empty embodiment over a temperature-bearing physiology registry, an affordance registry,
    /// the movement parameters, a controller hidden width (zero for a reaction norm), and a locomotion
    /// seed. The controller layout is derived from the two registries, so a caller builds or expresses
    /// its beings' controllers against [`Embodiment::layout`]. The resource field starts empty because
    /// temperature is a diffuse field with no discrete source tile to remember; the being's directional
    /// thermal percept is instead the live temperature gradient the runner reads from the field each
    /// tick ([`Field::gradient_at`]), so thermotaxis is sensed yet still emergent (the controller must
    /// evolve to follow the gradient).
    ///
    /// The registry must carry the [`TEMPERATURE`] axis, and that axis must not self-drain (its base
    /// and exertion draws must be zero), because the reserve is set each tick from the body core
    /// temperature rather than metabolised. Both are fail-loud here: the silent-zero hazard (an
    /// unregistered axis would make the comfort-band map a no-op) and the double-drain hazard (a
    /// self-draining axis would double-count the reserve the map already sets).
    pub fn new(
        homeo: HomeostaticRegistry,
        afford: AffordanceRegistry,
        params: LocomotionParams,
        hidden: usize,
        seed: u64,
    ) -> Embodiment {
        let axis = homeo.axis(TEMPERATURE).expect(
            "the embodiment physiology registry must carry the TEMPERATURE axis, or the comfort-band \
             map would write a reserve that is never read (the silent-zero hazard)",
        );
        assert!(
            axis.base_drain == Fixed::ZERO && axis.exertion_drain == Fixed::ZERO,
            "the TEMPERATURE axis must not self-drain: its reserve is set each tick from the body core \
             temperature, so a nonzero metabolic draw would double-count (the double-drain hazard)"
        );
        let layout = ControllerLayout::new(&homeo, &afford, hidden);
        Embodiment {
            walkers: Vec::new(),
            thermal: BTreeMap::new(),
            homeo,
            afford,
            organs: BodyPlanRegistry::dev_default(),
            layout,
            params,
            resources: ResourceField::new(),
            seed,
            physiology: None,
            tolerances: ToleranceRegistry::default(),
            percepts: PerceptRegistry::empty(),
            material: MaterialField::new(),
            material_registry: None,
            extraction: None,
            craft: None,
            earthwork: EarthworkField::new(),
            fire: FireField::new(),
        }
    }

    /// Install the toxin-tolerance registry (base-level liveliness step 4), so the lifecycle pairing
    /// expresses a newborn's heritable per-toxin-class tolerance from its own genome exactly as the
    /// founder step does. Set before the embodiment is handed to the runner; without it a newborn carries
    /// no tolerance (the harm sink stays inert for it).
    pub fn set_tolerances(&mut self, tolerances: ToleranceRegistry) {
        self.tolerances = tolerances;
    }

    /// Install the perceived-feature registry and REBUILD the controller layout to feed its feature
    /// block (harm-learning arc slice a). Set BEFORE the embodiment's beings are built, exactly like
    /// [`set_organs`] and [`set_physiology`]: the beings' controllers are expressed against
    /// [`Embodiment::layout`], so a percept added after they exist would leave their weight vectors the
    /// wrong length. With an empty registry this is a no-op that leaves the layout and every run hash
    /// unchanged (opt-in). The new feature weights a founder then expresses are zero (unseeded
    /// channels), so the percept has no behavioural effect until selection lifts a weight, the emergent
    /// pattern.
    pub fn set_percepts(&mut self, percepts: PerceptRegistry) {
        self.layout = ControllerLayout::with_percepts(
            &self.homeo,
            &self.afford,
            &percepts,
            self.layout.hidden(),
        );
        self.percepts = percepts;
    }

    /// Install the organ registry an affordance and the ground speed are derived against (emergent-anatomy
    /// step one). Set before the embodiment is handed to the runner, to the same [`BodyPlanRegistry`] the
    /// physiology is built from, so the affordance derive, the speed derive, and the metabolic producers
    /// all read one registry. Without it the embodiment keeps the labelled dev-fixture registry from
    /// [`Embodiment::new`].
    pub fn set_organs(&mut self, organs: BodyPlanRegistry) {
        self.organs = organs;
    }

    /// Install the anatomy-derived physiology (R-METABOLIZE) on this embodiment, so its beings drain,
    /// couple to the medium thermally, and (where a [`RESPIRATION`] axis is present) respire from their
    /// body plan and tissue against the physics rather than the labelled scalar drains. Set before the
    /// embodiment is handed to [`Runner::with_embodiment`], which reads it to seed each being's derived
    /// body-to-medium exchange rate. Opt-in: an embodiment without it keeps the scalar path unchanged.
    pub fn set_physiology(&mut self, physiology: EmbodiedPhysiology) {
        self.physiology = Some(physiology);
    }

    /// Install the located material substrate the world is made of (material-substrate arc, cascade
    /// item 1): the per-cell substance mixture the ground is built from. Opt-in, like [`set_percepts`]
    /// and [`set_physiology`]: an embodiment without it keeps an empty material layer, so folding the
    /// substrate into `state_hash` is byte-identical and every existing scenario replays bit-for-bit.
    /// A populated layer becomes canonical dynamic state (matter is moved, deposited, and consumed as
    /// the cascade wires up), so it folds into the hash from here.
    pub fn set_material(&mut self, material: MaterialField) {
        self.material = material;
    }

    /// The located material substrate, for reading (a pure read; the derived hardness and density a
    /// contest works against are read against a [`civsim_physics::PhysicsRegistry`] the caller supplies).
    /// The run-path write accessor (extraction, deposit, decay) arrives with its first consumer, so the
    /// mutation stays an id-sorted sequential draw off hashed state.
    pub fn material(&self) -> &MaterialField {
        &self.material
    }

    /// The earthwork delta field, for reading how digging has reshaped the terrain (material-substrate arc,
    /// cascade item 5): the per-column elevation change from the worldgen baseline.
    pub fn earthwork(&self) -> &EarthworkField {
        &self.earthwork
    }

    /// The fire field, for reading which cells are burning and how hard (material-substrate arc, cascade
    /// item 6): the per-cell combustion energy released this tick, sourced by the runner's combustion beat.
    pub fn fire(&self) -> &FireField {
        &self.fire
    }

    /// Install the world material registry a carried load's weight is derived against (material-substrate
    /// arc, cascade item 3): the ground registry ([`civsim_physics::PhysicsRegistry::ground`]). Opt-in;
    /// without it the carry actions no-op and every existing scenario is unchanged. This is the first
    /// run-path consumer of a substance's derived physical properties.
    pub fn set_material_registry(&mut self, registry: PhysicsRegistry) {
        self.material_registry = Some(registry);
    }

    /// Install the reserved extraction parameters a mining contest reads (material-substrate arc, cascade
    /// item 4): the working area a being's force presses over and the pressure cap
    /// ([`ExtractionParams`]). Opt-in; without it the extract action no-ops and every existing scenario is
    /// unchanged.
    pub fn set_extraction_params(&mut self, params: ExtractionParams) {
        self.extraction = Some(params);
    }

    /// Install the reserved crafting parameters a knapping contest reads (material-substrate arc, cascade
    /// item 4): the working-edge area a made tool presents and the carried volume it consumes
    /// ([`CraftParams`]). Opt-in; without it the craft action no-ops and every existing scenario is
    /// unchanged.
    pub fn set_craft_params(&mut self, params: CraftParams) {
        self.craft = Some(params);
    }

    /// The volume of a substance the being `walker_id` could take from the ground at `coord`, bounded by
    /// three limits with no randomness: what it wants, what the cell holds, and what its remaining carry
    /// headroom bears (its grown whole-body muscle force minus the weight it already carries, over the
    /// substance's weight per unit volume). A pure read, so a caller can size a pick-up before it moves
    /// matter. Zero when the embodiment declares no material registry or physiology, the being is
    /// unknown, the substance is weightless or unregistered (its weight cannot be derived, so it cannot
    /// be lifted), or no headroom remains.
    fn pickup_amount(
        &self,
        walker_id: StableId,
        coord: Coord3,
        substance: &str,
        want: Fixed,
    ) -> Fixed {
        let (Some(reg), Some(phys)) = (self.material_registry.as_ref(), self.physiology.as_ref())
        else {
            return Fixed::ZERO;
        };
        let Some(w) = self.walkers.iter().find(|w| w.id == walker_id) else {
            return Fixed::ZERO;
        };
        let gravity = standard_gravity();
        let capacity = being_muscle_force(w, phys);
        let carried = w.carried.weight(reg, gravity, FORCE_CEILING);
        let headroom = capacity - carried;
        if headroom <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        // The substance's weight per unit volume: its density times gravity. A substance the registry
        // carries no density for cannot be weighed, so it cannot be lifted (returns zero).
        let density = reg
            .substance(substance)
            .and_then(|s| s.vector.get("mat.density").copied())
            .unwrap_or(Fixed::ZERO);
        let unit_weight = density.checked_mul(gravity).unwrap_or(Fixed::ZERO);
        if unit_weight <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        let fits = headroom.checked_div(unit_weight).unwrap_or(Fixed::ZERO);
        want.min(fits)
            .min(self.material.volume(coord, substance))
            .max(Fixed::ZERO)
    }

    /// Pick up matter from the ground into the being's carried load (material-substrate arc, cascade item
    /// 3, the hinge): the being takes as much of `substance` at `coord` as its grown strength bears
    /// against the load's derived weight ([`Embodiment::pickup_amount`]), an id-keyed sequential draw off
    /// hashed state with no fresh randomness. The carry limit is grown whole-body muscle force versus
    /// physics-derived weight, never a per-race carry table (Principle 9). Returns the volume taken.
    pub fn pick_up(
        &mut self,
        walker_id: StableId,
        coord: Coord3,
        substance: &str,
        want: Fixed,
    ) -> Fixed {
        let take = self.pickup_amount(walker_id, coord, substance, want);
        if take <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        let taken = self.material.take(coord, substance, take);
        if let Some(w) = self.walkers.iter_mut().find(|w| w.id == walker_id) {
            w.carried.add(substance, taken);
        }
        taken
    }

    /// Put matter down from the being's carried load onto the ground (material-substrate arc, cascade
    /// item 3): the being deposits up to `want` of `substance` at `coord`, the inverse of
    /// [`Embodiment::pick_up`]. Dropping is always permitted (no capacity gate). Returns the volume set
    /// down, which persists at the coordinate with its substance identity.
    pub fn put_down(
        &mut self,
        walker_id: StableId,
        coord: Coord3,
        substance: &str,
        want: Fixed,
    ) -> Fixed {
        let Some(w) = self.walkers.iter_mut().find(|w| w.id == walker_id) else {
            return Fixed::ZERO;
        };
        let dropped = w.carried.take(substance, want);
        self.material.deposit(coord, substance, dropped);
        dropped
    }

    /// Enact a being's decided grasp (material-substrate arc, cascade item 3, the driver): pick the matter
    /// the being stands on up into its carried load, each substance in canonical id order bounded by the
    /// being's remaining strength headroom ([`Embodiment::pick_up`]). The evolved controller made the
    /// decision (a grasp output that won the tick); this is the physics that follows, lifting as much loose
    /// matter as the body bears and no more. A being on a void cell, or one already carrying to capacity,
    /// lifts nothing. The want per substance is the cell's whole standing volume, so the strength-versus-
    /// weight bound, not an authored rate, is what limits the lift; item 4's extraction contest will gate
    /// WHICH substance yields by the fracture hardness, where this generic carry gates only by weight.
    /// Returns the total volume lifted. The id-ordered walk is a deterministic tie-break over a shared
    /// cell, never a per-substance preference (no race, kind, or role read; Principle 9).
    pub fn grasp_underfoot(&mut self, walker_id: StableId) -> Fixed {
        let Some(coord) = self
            .walkers
            .iter()
            .find(|w| w.id == walker_id)
            .map(|w| w.coord())
        else {
            return Fixed::ZERO;
        };
        // The substances present, in canonical id (BTreeMap) order, snapshotted so the pick-up loop can
        // mutate the cell without aliasing the read.
        let substances: Vec<String> = match self.material.cell(coord) {
            Some(mix) => mix.substances().map(|(s, _)| s.clone()).collect(),
            None => return Fixed::ZERO,
        };
        let mut lifted = Fixed::ZERO;
        for substance in &substances {
            let want = self.material.volume(coord, substance);
            lifted += self.pick_up(walker_id, coord, substance, want);
        }
        lifted
    }

    /// Enact a being's decided EXTRACT (material-substrate arc, cascade item 4, the extraction contest):
    /// break the bonded matter the being stands on loose and take it into the carried load, but only if the
    /// being's contact pressure clears the cell's FRACTURE-gating hardness. The being's grown whole-body
    /// muscle force ([`being_muscle_force`]) pressed over its reserved working area is a contact pressure
    /// ([`laws::contact_pressure`]); if that pressure does not exceed the cell's fracture hardness (the
    /// hardest constituent's fracture strength, [`MaterialField::fracture_hardness`]) the rock holds and
    /// nothing is taken, so a being too weak to fracture granite mines none of it however much it can lift.
    /// Above the gate the matter is loose and the being takes as much as its strength bears against the
    /// load's weight, the item-3 carry bound ([`Embodiment::grasp_underfoot`]), so extraction is fracture
    /// THEN carry. This is the distinction from a bare grasp: grasp lifts already-loose matter (weight gate
    /// only), extract must first break the bond (the fracture gate). All physics against substance data:
    /// the force is derived, the hardness is the substance floor's, and the working area is reserved with a
    /// basis, no race, kind, or role read (Principles 8, 9). Returns the volume extracted. Opt-in: an
    /// embodiment with no extraction parameters, material registry, or physiology extracts nothing.
    ///
    /// The yield AMOUNT is the strength-bounded carry here (the fracture STRENGTH gates whether the rock
    /// breaks); a later slice sizes the per-stroke yield by the delivered work over the substance's cutting
    /// energy ([`crate::material::extraction_yield`], built and proven), once the mineable substances carry
    /// a cited `mat.specific_cut_energy`.
    pub fn extract_underfoot(&mut self, walker_id: StableId) -> Fixed {
        let (Some(params), Some(reg), Some(phys)) = (
            self.extraction.as_ref(),
            self.material_registry.as_ref(),
            self.physiology.as_ref(),
        ) else {
            return Fixed::ZERO;
        };
        let Some(w) = self.walkers.iter().find(|w| w.id == walker_id) else {
            return Fixed::ZERO;
        };
        let coord = w.coord();
        let force = being_muscle_force(w, phys);
        // The tool the being wields, if any (crafting, the tool multiplies the affordance): its working
        // geometry and material, snapshotted so the being read ends before the mutable take below. A
        // wielded tool presses the same force over its own (smaller) contact area, and blunts at its own
        // indentation hardness; a bare being uses its reserved working area with no material cap. This is
        // the one place the extraction contest reads the tool, so a crafted point breaks rock a fist cannot.
        let (area, tool_hardness) = match &w.wielded {
            Some(t) => {
                let h = reg
                    .substance(&t.substance)
                    .and_then(|s| s.vector.get("mat.indentation_hardness").copied())
                    .unwrap_or(Fixed::ZERO);
                (t.contact_area, Some(h))
            }
            None => (params.working_area, None),
        };
        let fracture = self.material.fracture_hardness(coord, reg);
        // The fracture gate: the being's contact pressure must clear the cell's fracture-gating hardness to
        // break any matter loose. A cell with no fracture resistance (loose soil, void) reads zero and any
        // positive pressure breaks it (Principle 8: physics, no bonded-versus-loose tag).
        let pressure = laws::contact_pressure(force, area, params.pressure_max);
        // A wielded tool blunts at its own hardness: however concentrated, a soft tool cannot carry a
        // pressure above the material it is made of, so a soft point cannot exceed a hard rock's resistance
        // (the same cap the weapon and cut reads apply). A tool whose substance declares no hardness reads
        // zero and blunts to no pressure (the absence convention: a tool must declare its hardness to work
        // matter). A bare being (None) carries no material cap here yet: its working-surface hardness is an
        // anatomy-arc follow-on, so its contest is unchanged.
        let effective = match tool_hardness {
            Some(h) => pressure.min(h),
            None => pressure,
        };
        if effective <= fracture {
            return Fixed::ZERO;
        }
        self.grasp_underfoot(walker_id)
    }

    /// Enact a being's decided GEOPHAGE (material-substrate arc, cascade item 4, INGEST-FOR-COMPOSITION):
    /// eat the matter the being stands on for any reserve backed by a substance the cell holds. For each
    /// homeostatic axis whose backing substance is present in the cell, the being draws it through the SAME
    /// edibility satisfaction the forage ingest uses ([`laws::satisfaction`] over the being's own
    /// assimilation and requirement), bounded by the room left in the reserve, grossed up by the ingest
    /// efficiency to the mass removed, taken from the cell, and the assimilated part deposited in the
    /// reserve. So a being with a mineral deficit standing on that mineral refills from it and a full one
    /// draws nothing: this is the need-side complement to the harm-learning read, the same cell composition
    /// another being learns to AVOID for a harm, this one SEEKS for a nutrient it lacks, and it is what
    /// makes a mined or standing mineral worth something (the payoff that lets mineral-seeking, and the
    /// mining that reaches a bonded mineral, emerge under selection). Reads only the axis backing
    /// substances, the cell's matter, and the being's own physiology, no race, kind, or role (Principle 9).
    /// Returns the total assimilated. Naturally opt-in: an empty material field (every existing scenario)
    /// holds no substance, so the supply is zero and nothing is drawn, and only a being that decided the
    /// geophage affordance (in a geophage fixture) reaches here at all.
    pub fn geophage(&mut self, walker_id: StableId) -> Fixed {
        let Some(coord) = self
            .walkers
            .iter()
            .find(|w| w.id == walker_id)
            .map(|w| w.coord())
        else {
            return Fixed::ZERO;
        };
        let eta = self.params.ingest_efficiency;
        let mut gained = Fixed::ZERO;
        // The substances the being eats this bite, for the toxin harm below (a set so a substance that feeds
        // two reserves is not counted twice against its own toxicity).
        let mut eaten: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for i in 0..self.homeo.axes.len() {
            let Some(substance) = self.homeo.axes[i].backing_component.clone() else {
                continue; // an axis with no backing substance is not fed by matter
            };
            let axis_id = self.homeo.axes[i].id;
            let supply = self.material.volume(coord, &substance);
            if supply <= Fixed::ZERO {
                continue; // the cell holds none of this substance
            }
            // Size the bite from the being's own physiology and the room its reserve has left (immutable).
            let Some(w) = self.walkers.iter().find(|w| w.id == walker_id) else {
                return gained;
            };
            let frac = laws::satisfaction(
                supply,
                w.physiology.assimilation(&substance),
                w.physiology.requirement(&substance),
            );
            let cap = w.homeostasis.capacity(axis_id);
            let room = cap - w.homeostasis.amount(axis_id);
            let target_gain = frac.checked_mul(cap).unwrap_or(cap).min(room);
            if target_gain <= Fixed::ZERO {
                continue; // the reserve is full: draw nothing, deplete nothing
            }
            let gross = if eta > Fixed::ZERO {
                target_gain.checked_div(eta).unwrap_or(target_gain)
            } else {
                target_gain
            };
            // Take the gross bite from the cell and deposit the assimilated part in the reserve (each field
            // mutated on its own, so the cell loses the bite and the being gains the bite times the
            // efficiency, conservation-honest as the forage ingest is).
            let taken = self.material.take(coord, &substance, gross);
            let gain = taken.checked_mul(eta).unwrap_or(taken);
            if let Some(w) = self.walkers.iter_mut().find(|w| w.id == walker_id) {
                w.homeostasis.ingest(axis_id, gain);
            }
            gained += gain;
            if taken > Fixed::ZERO {
                eaten.insert(substance);
            }
        }
        // The HARM-HALF (the symmetric completion of ingestion): the toxins in what was eaten harm the eater
        // against its OWN inherited per-toxin-class tolerance, the same edibility harm side the exposure read
        // applies, so eating a poison for its nutrient costs and a food that sickens ONE eater feeds another
        // safely. Read only the classes the being carries a tolerance for (never the substance's mat.* axes),
        // dose each from the eaten substance's registry concentration, and net_harm against the being's
        // tolerance and Hill exponent, per consumer, no per-substance poison label (Principle 9). Applied to
        // CONDITION (a no-op for a being with no CONDITION axis), so the felt harm feeds the existing
        // harm-learning loop: a being can now learn "this food sickens me", not only "this place harms me".
        let harm = match self.material_registry.as_ref() {
            Some(reg) if !eaten.is_empty() => {
                match self.walkers.iter().find(|w| w.id == walker_id) {
                    Some(w) => {
                        let mut classes: Vec<(Fixed, Option<Fixed>, u8)> = Vec::new();
                        for class in w.physiology.tolerances.keys() {
                            let mut dose = Fixed::ZERO;
                            for substance in &eaten {
                                if let Some(sub) = reg.substance(substance) {
                                    dose = dose.saturating_add(
                                        sub.vector.get(class).copied().unwrap_or(Fixed::ZERO),
                                    );
                                }
                            }
                            if dose > Fixed::ZERO {
                                classes.push((
                                    dose,
                                    w.physiology.tolerance(class),
                                    w.physiology.hill_exp(class),
                                ));
                            }
                        }
                        laws::net_harm(
                            &classes,
                            self.params.harm_caps.harm_cap,
                            self.params.harm_caps.total_harm_cap,
                        )
                    }
                    None => Fixed::ZERO,
                }
            }
            _ => Fixed::ZERO,
        };
        if harm > Fixed::ZERO {
            if let Some(w) = self.walkers.iter_mut().find(|w| w.id == walker_id) {
                w.homeostasis.adjust(CONDITION, Fixed::ZERO - harm);
            }
        }
        gained
    }

    /// Enact a being's decided CRAFT (material-substrate arc, cascade item 4, knapping): shape the matter
    /// the being carries into a wielded tool. It consumes the reserved tool volume of the FIRST substance
    /// the being carries enough of (canonical id order, a deterministic tie-break over a mixture, never a
    /// per-substance preference) and wields a tool of that substance with the reserved working-edge area,
    /// replacing any prior tool. So a being that has mined and carried stone can shape it into a point, and
    /// the tool it makes is only as good as the stone it worked (a hard stone a hard tool, a soft stone a
    /// soft one, the tool's function derived from its material and geometry by the crafting seam's cut read,
    /// never a recipe catalog). Reads only the carried substance ids and the reserved geometry, no race,
    /// kind, or role (Principles 8, 9). Returns true if a tool was made. Opt-in: an embodiment with no
    /// crafting parameters, or a being carrying too little of any one substance, makes nothing.
    pub fn craft_from_carried(&mut self, walker_id: StableId) -> bool {
        let Some(params) = self.craft else {
            return false;
        };
        // The first substance the being carries enough of to shape a tool, in canonical id order.
        let Some(substance) = self
            .walkers
            .iter()
            .find(|w| w.id == walker_id)
            .and_then(|w| {
                w.carried
                    .substances()
                    .find(|(_, &vol)| vol >= params.tool_volume)
                    .map(|(s, _)| s.clone())
            })
        else {
            return false; // not carrying enough of any one substance to make a tool
        };
        if let Some(w) = self.walkers.iter_mut().find(|w| w.id == walker_id) {
            w.carried.take(&substance, params.tool_volume);
            w.wielded = Some(WieldedTool {
                contact_area: params.edge_area,
                substance,
            });
        }
        true
    }

    /// Enact a being's decided DIG (material-substrate arc, cascade item 5, modifiable terrain): excavate
    /// the ground underfoot in the SAME extraction fracture contest ([`Embodiment::extract_underfoot`], the
    /// fracture gate plus taking the spoil into the carried load) AND lower the column by the volume removed,
    /// so a pit forms where matter left. This is the distinction from a bare extract: extract takes matter,
    /// dig also reshapes the terrain, the earthwork delta being the elevation bookkeeping the physics will
    /// read (a dug pit pools water). The column drops by the spoil volume (conservation of the ground, a unit
    /// cell area), so what leaves the cell is both carried off and gone from the terrain's height. Reads only
    /// the being's coordinate and the extraction contest, no race, kind, or role (Principle 9). Returns the
    /// volume excavated. Opt-in: an embodiment with no extraction parameters digs nothing (the fracture
    /// contest no-ops), so the earthwork stays empty and the run is byte-identical.
    pub fn dig_underfoot(&mut self, walker_id: StableId) -> Fixed {
        let Some(column) = self.walkers.iter().find(|w| w.id == walker_id).map(|w| {
            let c = w.coord();
            Coord3::ground(c.x, c.y)
        }) else {
            return Fixed::ZERO;
        };
        let spoil = self.extract_underfoot(walker_id);
        if spoil > Fixed::ZERO {
            self.earthwork.adjust(column, Fixed::ZERO - spoil);
        }
        spoil
    }

    /// Enact a being's decided RELEASE (material-substrate arc, cascade item 5, modifiable terrain, the
    /// deposit-and-mound half): set the carried load down onto the ground underfoot (the inverse of
    /// [`Embodiment::grasp_underfoot`]) AND raise the column by the volume deposited, so a mound rises where
    /// matter was set down. Each carried substance is deposited in canonical id order (a deterministic draw)
    /// and the column is raised by the total, conservation-symmetric with [`Embodiment::dig_underfoot`]
    /// lowering it: what a being digs from a pit here and carries to there raises a mound there, so terracing
    /// EMERGES from the dig and release primitives with no mound verb. Reads only the being's carried load
    /// and coordinate, no race, kind, or role (Principle 9). Returns the volume deposited. Opt-in: a being
    /// carrying nothing sets nothing down and the terrain is unchanged.
    pub fn release_underfoot(&mut self, walker_id: StableId) -> Fixed {
        let Some((coord, column, substances)) =
            self.walkers.iter().find(|w| w.id == walker_id).map(|w| {
                let c = w.coord();
                let subs: Vec<String> = w.carried.substances().map(|(s, _)| s.clone()).collect();
                (c, Coord3::ground(c.x, c.y), subs)
            })
        else {
            return Fixed::ZERO;
        };
        let mut deposited = Fixed::ZERO;
        for substance in &substances {
            let want = self
                .walkers
                .iter()
                .find(|w| w.id == walker_id)
                .map(|w| w.carried.volume(substance))
                .unwrap_or(Fixed::ZERO);
            deposited += self.put_down(walker_id, coord, substance, want);
        }
        if deposited > Fixed::ZERO {
            self.earthwork.adjust(column, deposited);
        }
        deposited
    }

    /// The per-tile resource field the beings perceive and ingest, for mutation (base-level liveliness
    /// step 2): the environmental stack writes the standing producer-biomass supply into it each tick
    /// before the embodiment step reads it. Crate-internal; the runner owns the write path.
    pub(crate) fn resources_mut(&mut self) -> &mut ResourceField {
        &mut self.resources
    }

    /// The controller layout derived from this embodiment's registries, against which a caller builds
    /// or expresses its beings' controllers (their dimensions must match).
    pub fn layout(&self) -> &ControllerLayout {
        &self.layout
    }

    /// Add a located being with its evolved controller and its reserved thermal band. The being's
    /// temperature reserve is seeded from its spawn core temperature through the comfort-band map, so it
    /// begins physiologically consistent with the field it stands in.
    pub fn add(&mut self, mut walker: Walker, band: BeingThermal) {
        walker
            .homeostasis
            .set_level(TEMPERATURE, comfort_fraction(band.initial_temp, &band));
        self.thermal.insert(walker.id, band);
        self.walkers.push(walker);
    }

    /// The located beings, for reading and rendering (a pure read).
    pub fn walkers(&self) -> &[Walker] {
        &self.walkers
    }

    /// The standing resource field the grazers deplete and the environment regrows (a pure read, for the
    /// carrying-capacity reader; base-level liveliness step 3).
    pub fn resources(&self) -> &ResourceField {
        &self.resources
    }
}

/// The data the unified runner needs to embody a newborn mind at the lifecycle-pairing beat (real-world
/// unification, step 3c): the reserved comfort band a newborn thermoregulates within, and the spawn
/// coordinate each dawn band's [`PlaceId`] maps to. Everything else a newborn body needs (its body plan
/// and genes, its genome) is read from the [`World`] at the birth, and the organ and homeostatic
/// registries from the installed [`Embodiment`], so this kit carries only the two dawn-assembly inputs
/// that live on neither side. Armed by the world-build ([`Runner::arm_lifecycle`]); without it a newborn
/// stays a bodiless mind (a run that never armed the kit does not embody births).
#[derive(Clone, Debug)]
pub struct LifecycleKit {
    /// The comfort band a newborn is born into (the reserved set point and half-band, born at the set
    /// point). Uniform across the dawn founders this arc, so a newborn inherits the same band.
    thermal: BeingThermal,
    /// The spawn coordinate each dawn band's [`PlaceId`] maps to (`PlaceId` stays frozen at the dawn
    /// band, owner ruling 2026-07-04), so a newborn spawns at its band's site. A newborn whose place is
    /// not in this map is not embodied (a defensive skip, never a fabricated coordinate).
    spawn_by_place: BTreeMap<PlaceId, Coord3>,
}

impl LifecycleKit {
    /// The lifecycle kit from its comfort band and per-band spawn map. The world-build builds this from
    /// the same reserved thermal band and band placement the dawn assembly reads.
    pub fn new(thermal: BeingThermal, spawn_by_place: BTreeMap<PlaceId, Coord3>) -> LifecycleKit {
        LifecycleKit {
            thermal,
            spawn_by_place,
        }
    }
}

/// The canonical runner: the temperature field, the located population, and their deterministic
/// coupling. Constructed with an explicit [`FieldCalib`] (no authored default).
pub struct Runner {
    clock: u64,
    field: Field,
    calib: FieldCalib,
    index: LocationIndex,
    /// Per-located-being body temperature, absolute Q32.32 on the `therm.temperature` axis. Held here
    /// as the thermal state the field drives; the body-arc harm mapping from a temperature outside a
    /// race's comfort band is a reserved consumer (the two-sided band the body arc deferred).
    body_temp: BTreeMap<StableId, Fixed>,
    /// The per-being DERIVED body-to-medium exchange rate `h * A / (m * c)` per tick
    /// ([`crate::physiology::derive_body_exchange_rate`]), when the caller has supplied it. A being with
    /// an entry couples to its cell at its own derived rate (a high-surface, low-thermal-mass body
    /// faster, a compact dense one slower); a being with no entry falls back to the labelled-fixture
    /// [`FieldCalib::exchange`] override. This frees the authored `field.body_exchange` scalar on the
    /// canonical path while keeping the field-fixture fallback for beings placed without a body.
    body_exchange_rate: BTreeMap<StableId, Fixed>,
    /// The cognition world composed onto this spine, ticked as a fixed sub-phase after the field
    /// phases. `None` for a field-only runner ([`Runner::new`]), `Some` for the composed runner
    /// ([`Runner::with_world`]). The world carries disjoint mutable state, its own seed, and its own
    /// canonical hash; this runner never reaches into it beyond ticking it and folding its hash.
    world: Option<World>,
    /// The embodied-being population coupled to the field, ticked as a fixed sub-phase after the
    /// body-thermal exchange and before the cognition world. `None` for a runner without embodied
    /// beings, `Some` for the coupled runner ([`Runner::with_embodiment`]). Its beings share the
    /// runner's `body_temp` and `index` (they are the located population), and the coupling reads the
    /// field only through the comfort-band map, writing back only the beings' coordinates.
    embodiment: Option<Embodiment>,
    /// The lifecycle-pairing kit (real-world unification, step 3c), armed on the unified path so a
    /// [`World`] birth mints a paired body and a death retires it. `None` on every other path (and until
    /// [`Runner::arm_lifecycle`] is called), so a world-only, embodiment-only, or unarmed runner never
    /// embodies a newborn and the reconciliation beat is a no-op.
    lifecycle: Option<LifecycleKit>,
    /// The environmental field stack (base-level liveliness step 2), armed on the run path so hydrology
    /// and primary productivity advance each tick after the temperature field and write the standing
    /// producer biomass into the embodiment's resource field. `None` on a runner without it, so the
    /// temperature-only paths are unchanged. Stepped inside [`Runner::step_field`], folded into
    /// `state_hash`.
    environ: Option<(EnvironFields, EnvironCalib)>,
    /// The reserved calibrations of the combustion beat (material-substrate arc, cascade item 6, live fire),
    /// armed opt-in. `None` on a runner without it, so no combustion runs and every existing scenario is
    /// byte-identical; armed via [`Runner::set_combustion`], the beat then sources the embodiment's fire
    /// field from the combustible matter hot enough to burn. Off the calibrated worldbuild path until a later
    /// slice wires it, exactly like the extraction and craft params.
    combustion: Option<CombustionCalib>,
    /// The set of beings this runner promoted to the individual dialogue tier through the arc-scoped
    /// promotion policy last tick (base-level liveliness §4). The policy owns only this set: each tick it
    /// promotes the new arc set and restricts the beings in this set that left the arc, so a promotion set
    /// by any other path (a test harness, a scripted scene) is never clobbered. Not folded into
    /// `state_hash`: it is a derived function of the reserves and cells the hash already covers, and the
    /// promotions themselves live in the world's own canonical state.
    arc_promoted: BTreeSet<StableId>,
    /// Non-canonical observability: the reserve axis whose depletion killed each locomotion death since
    /// the last drain, so the run harness can report cause of death (which reserve ran out). A pure read
    /// of a dying being's own homeostasis through `dead_axis`, drained by `take_obs_deaths`, and NOT
    /// folded into `state_hash` (observation, not canonical state), so it never perturbs the run.
    obs_deaths: Vec<HomeostaticAxisId>,
    /// The reserved calibrations of the base-level liveliness surfacing policy (the hazard-belief and
    /// arc-promotion magnitudes). Initialized to the labelled dev fixture in every constructor so the
    /// test and harness paths are unchanged; [`build_dawn_runner`](crate::worldbuild::build_dawn_runner)
    /// overrides it fail-loud from the manifest through [`Runner::set_liveliness`].
    liveliness: LivelinessCalib,
    /// The reserved calibrations of the experiential associative learner (harm-learning arc slice b):
    /// the harm-noise floor, the feature granularity, and the two harm likelihoods the belief-formation
    /// weight reads. Initialized to the labelled dev fixture in every constructor; the world-build
    /// overrides it fail-loud from the manifest through [`Runner::set_harm_learning`]. They REPLACE the
    /// retired `hazard_dose_threshold`/`hazard_weight`, which authored the belief a being now forms for
    /// itself.
    harm_learning: HarmLearningCalib,
}

impl Runner {
    /// A field-only runner over a field with the given reserved calibrations (no cognition world, no
    /// embodied beings).
    pub fn new(field: Field, calib: FieldCalib) -> Runner {
        Runner {
            clock: 0,
            field,
            calib,
            index: LocationIndex::new(),
            body_temp: BTreeMap::new(),
            body_exchange_rate: BTreeMap::new(),
            world: None,
            embodiment: None,
            lifecycle: None,
            environ: None,
            combustion: None,
            arc_promoted: BTreeSet::new(),
            obs_deaths: Vec::new(),
            liveliness: LivelinessCalib::dev_default(),
            harm_learning: HarmLearningCalib::dev_default(),
        }
    }

    /// A composed runner that owns a cognition [`World`] and ticks it as a fixed sub-phase after the
    /// field phases. The caller constructs and calibrates the world (fail-loud on any unset reserved
    /// value, per the world's own manifest discipline); this runner adds no authored number, no new
    /// RNG draw, and no new phase, so the composite reproduces bit for bit exactly as each side already
    /// does.
    ///
    /// The canonical steering boundary is fail-loud here (Principle 9): the world must not carry an
    /// authored decision repertoire ([`crate::world::World::set_behaviour`]). That repertoire is the
    /// sentient deliberative tier of Part 8.1, an authored action-and-drive policy Part 8.4 names as
    /// steering at the level of behaviour, and the canonical-emergent runner's behaviour source is the
    /// evolved controller, never an authored policy. A world with one installed is rejected so the
    /// authored path cannot ride the canonical spine as if it were emergent.
    pub fn with_world(field: Field, calib: FieldCalib, world: World) -> Runner {
        assert!(
            !world.has_behaviour(),
            "the canonical runner refuses a world carrying an authored decision repertoire: that is \
             the sentient deliberative tier (Part 8.1), steering at the level of behaviour (Part 8.4), \
             and the canonical-emergent behaviour source is the evolved controller, not an authored \
             policy"
        );
        Runner {
            clock: 0,
            field,
            calib,
            index: LocationIndex::new(),
            body_temp: BTreeMap::new(),
            body_exchange_rate: BTreeMap::new(),
            world: Some(world),
            embodiment: None,
            lifecycle: None,
            environ: None,
            combustion: None,
            arc_promoted: BTreeSet::new(),
            obs_deaths: Vec::new(),
            liveliness: LivelinessCalib::dev_default(),
            harm_learning: HarmLearningCalib::dev_default(),
        }
    }

    /// A runner coupled to an embodied-being population on the evolved-controller substrate (Part 8.4,
    /// R-BEHAVIOR-EVOLVE): its beings are the located population, their movement driven by their evolved
    /// controllers, and the field drives their physiology through the comfort-band map each tick. The
    /// beings' spawn coordinates and core temperatures seed the located index and the body-temperature
    /// map, so the field-thermal exchange and the coupling act on the same population. This adds no
    /// authored heading and no new RNG phase (locomotion's exploration keys on the existing
    /// [`civsim_core::Phase::EXPLORE`]), so the coupled runner reproduces bit for bit.
    ///
    /// The behaviour source is the evolved controller, never an authored policy, so the Principle 9
    /// steering boundary holds by construction: there is no drive-and-action repertoire on this path,
    /// only the heritable controller each being carries.
    pub fn with_embodiment(field: Field, calib: FieldCalib, embodiment: Embodiment) -> Runner {
        let mut body_temp = BTreeMap::new();
        let mut index = LocationIndex::new();
        let mut body_exchange_rate = BTreeMap::new();
        for w in &embodiment.walkers {
            let init = embodiment
                .thermal
                .get(&w.id)
                .map(|b| b.initial_temp)
                .unwrap_or(Fixed::ZERO);
            body_temp.insert(w.id, init);
            index.place(OccupantId::being(w.id), w.coord());
            // Seed the being's DERIVED body-to-medium exchange rate h*A/(m*c) once, when an
            // anatomy-derived physiology is installed. It is static (a pure function of the body plan
            // and the medium coefficient), so it is set here rather than recomputed each tick, and
            // phase_body_exchange then couples the being at its own surface-and-thermal-mass rate rather
            // than the labelled FieldCalib.exchange scalar (Principle 9: divergence from anatomy).
            if let Some(phys) = &embodiment.physiology {
                let rate = walker_exchange_rate(&w.body, &w.structure, phys);
                body_exchange_rate.insert(w.id, rate);
            }
        }
        Runner {
            clock: 0,
            field,
            calib,
            index,
            body_temp,
            body_exchange_rate,
            world: None,
            embodiment: Some(embodiment),
            lifecycle: None,
            environ: None,
            combustion: None,
            arc_promoted: BTreeSet::new(),
            obs_deaths: Vec::new(),
            liveliness: LivelinessCalib::dev_default(),
            harm_learning: HarmLearningCalib::dev_default(),
        }
    }

    /// The unified real world (real-world unification, step 2): one runner carrying BOTH a cognition
    /// [`World`] of minds and an [`Embodiment`] of located, metabolizing bodies, so a founder whose
    /// [`StableId`] owns an entry in both is at once a culture-forming mind and a thermoregulating body
    /// on the field. This is the first constructor to break the mutual exclusion the two run paths held
    /// (`with_world` forced `embodiment = None`, `with_embodiment` forced `world = None`); it composes
    /// them under one shared id space, which the caller (`build_dawn_runner`) guarantees by minting
    /// every id from the world's one [`crate::world::Registry`] and reusing those ids for the walkers,
    /// never a second registry.
    ///
    /// The canonical steering boundary survives verbatim: the world must carry no authored decision
    /// repertoire (Principle 9, Part 8.4), the same fail-loud assert `with_world` makes, so the unified
    /// path cannot smuggle the authored deliberative tier onto the emergent spine. The embodiment side
    /// seeds exactly as `with_embodiment` does (the body-temperature map, the located index, and each
    /// being's derived body-to-medium exchange rate), so a shared being is seeded on both halves.
    ///
    /// Determinism: the two systems now share the [`RES_BEING`] resource so the scheduler serializes
    /// them in the pinned order (see [`Runner::tick_systems`]); `state_hash` already folds both halves
    /// (the body-temperature map id-sorted, the world hash, and each walker id-sorted), and a shared
    /// being appears in all three deterministically. The two clocks differ by one within a tick: the
    /// embodiment (locomotion) draws key on the runner clock pre-increment (tick K) while the world
    /// draws key on the world clock, incremented at the start of `World::tick` (K+1). This offset is
    /// harmless because a being's body draws (`Phase::EXPLORE`) and its mind draws (LANGUAGE,
    /// MATE_CHOICE, CONVERSE, and the rest) are discriminated by their `Phase`, so the two streams never
    /// collide at either clock, and both clocks are deterministic functions of the tick count, so replay
    /// and worker-count independence hold. Post-tick the two clocks agree (each advances once per tick).
    pub fn with_world_and_embodiment(
        field: Field,
        calib: FieldCalib,
        world: World,
        embodiment: Embodiment,
    ) -> Runner {
        assert!(
            !world.has_behaviour(),
            "the canonical runner refuses a world carrying an authored decision repertoire: that is \
             the sentient deliberative tier (Part 8.1), steering at the level of behaviour (Part 8.4), \
             and the canonical-emergent behaviour source is the evolved controller, not an authored \
             policy"
        );
        // Seed the embodiment side exactly as with_embodiment does, so a shared being is seeded on both
        // halves: the body-temperature map, the located index, and each being's derived exchange rate.
        let mut body_temp = BTreeMap::new();
        let mut index = LocationIndex::new();
        let mut body_exchange_rate = BTreeMap::new();
        for w in &embodiment.walkers {
            let init = embodiment
                .thermal
                .get(&w.id)
                .map(|b| b.initial_temp)
                .unwrap_or(Fixed::ZERO);
            body_temp.insert(w.id, init);
            index.place(OccupantId::being(w.id), w.coord());
            if let Some(phys) = &embodiment.physiology {
                let rate = walker_exchange_rate(&w.body, &w.structure, phys);
                body_exchange_rate.insert(w.id, rate);
            }
        }
        Runner {
            clock: 0,
            field,
            calib,
            index,
            body_temp,
            body_exchange_rate,
            world: Some(world),
            embodiment: Some(embodiment),
            lifecycle: None,
            environ: None,
            combustion: None,
            arc_promoted: BTreeSet::new(),
            obs_deaths: Vec::new(),
            liveliness: LivelinessCalib::dev_default(),
            harm_learning: HarmLearningCalib::dev_default(),
        }
    }

    /// Arm the lifecycle-pairing kit (real-world unification, step 3c), so a [`World`] birth mints a
    /// paired body and a death retires it at the reconciliation beat. The world-build calls this after
    /// [`Runner::with_world_and_embodiment`] with the reserved comfort band and the per-band spawn map
    /// the dawn assembly already built. Without it the unified runner ticks minds and bodies but never
    /// embodies a newborn (the pre-3c behaviour), so arming is opt-in and additive.
    pub fn arm_lifecycle(&mut self, kit: LifecycleKit) {
        self.lifecycle = Some(kit);
    }

    /// Arm the environmental field stack (base-level liveliness step 2): hydrology and primary
    /// productivity advance each tick after the temperature field, and the standing producer biomass is
    /// written into the embodiment's resource field so the grazers have supply. Folded into `state_hash`.
    /// Without it a runner is temperature-only, exactly as before.
    pub fn set_environ(&mut self, fields: EnvironFields, calib: EnvironCalib) {
        self.environ = Some((fields, calib));
    }

    /// Arm the combustion beat (material-substrate arc, cascade item 6, live fire): each tick, the
    /// combustible matter a cell holds that stands at or above its ignition temperature burns, consuming a
    /// bounded fraction of its fuel and lighting the embodiment's fire field. Opt-in: a runner left unarmed
    /// runs no combustion, so every existing scenario is byte-identical. Reserved calibrations
    /// ([`CombustionCalib`]); off the calibrated worldbuild path until a later slice wires it.
    pub fn set_combustion(&mut self, calib: CombustionCalib) {
        self.combustion = Some(calib);
    }

    /// Arm the reserved calibrations of the base-level liveliness surfacing policy (the hazard-belief and
    /// arc-promotion magnitudes), overriding the labelled dev fixture the constructors install. The dawn
    /// build reads these fail-loud from the manifest (Principle 11); a runner left unarmed keeps the dev
    /// fixture, so the test and harness paths are unchanged.
    pub fn set_liveliness(&mut self, calib: LivelinessCalib) {
        self.liveliness = calib;
    }

    /// Arm the reserved calibrations of the experiential associative learner (harm-learning arc slice b),
    /// overriding the labelled dev fixture the constructors install. The dawn build reads these fail-loud
    /// from the manifest (Principle 11); a runner left unarmed keeps the dev fixture, so the test and
    /// harness paths are unchanged.
    pub fn set_harm_learning(&mut self, calib: HarmLearningCalib) {
        self.harm_learning = calib;
    }

    /// The environmental field stack, if armed (a pure read, for the field-state reader and tests).
    pub fn environ(&self) -> Option<&EnvironFields> {
        self.environ.as_ref().map(|(f, _)| f)
    }

    /// Place a being on the map at a coordinate with an initial body temperature.
    pub fn place_being(&mut self, id: StableId, coord: Coord3, body_temp: Fixed) {
        self.index.place(OccupantId::being(id), coord);
        self.body_temp.insert(id, body_temp);
    }

    /// The current tick.
    pub fn clock(&self) -> u64 {
        self.clock
    }

    /// The field, for reading and rendering (a pure read, never a write to canon).
    pub fn field(&self) -> &Field {
        &self.field
    }

    /// A located being's body temperature.
    pub fn body_temp(&self, id: StableId) -> Option<Fixed> {
        self.body_temp.get(&id).copied()
    }

    /// The composed cognition world, if any (a pure read, for tests and rendering).
    pub fn world(&self) -> Option<&World> {
        self.world.as_ref()
    }

    /// The composed cognition world for mutation (a calibration override applied after assembly, for
    /// example a life-cadence override so a multi-generation run fits a test budget). This is not part
    /// of the tick path; the deterministic scheduler reads the world when it steps, so a calibration
    /// set here before stepping is reproducible.
    pub fn world_mut(&mut self) -> Option<&mut World> {
        self.world.as_mut()
    }

    /// The coupled embodied-being population, if any (a pure read, for tests and rendering).
    /// Non-canonical observability: drain and return the reserve axis behind each locomotion death since
    /// the last call, for the run harness's cause-of-death reader. Not part of canonical state or
    /// `state_hash`, so draining it never affects the run.
    pub fn take_obs_deaths(&mut self) -> Vec<HomeostaticAxisId> {
        std::mem::take(&mut self.obs_deaths)
    }

    pub fn embodiment(&self) -> Option<&Embodiment> {
        self.embodiment.as_ref()
    }

    /// The embodiment for mutation (a caller driving a matter action directly, or a test): the runner owns
    /// it, so this is the write handle beside the read [`Runner::embodiment`].
    pub fn embodiment_mut(&mut self) -> Option<&mut Embodiment> {
        self.embodiment.as_mut()
    }

    /// One canonical tick, in a pinned within-tick order: step the field, let each located being
    /// exchange heat with its cell (Newton convective coupling toward the local field temperature,
    /// beings walked in canonical id order), run the embodiment coupling sub-phase (comfort-band map to
    /// evolved-controller locomotion to index re-sync), then tick the composed cognition world as the
    /// fixed final sub-phase. The field phases run first so the embodiment coupling reads the same-tick
    /// thermal state; the cognition world runs last and shares no data across its seam yet.
    pub fn step(&mut self) {
        self.step_inner(&[]);
    }

    /// Like [`step`](Self::step), but feeds the composed cognition world a batch of tick
    /// inputs (the observations that drive its beings to form beliefs) rather than the
    /// empty batch. The field and embodiment sub-phases are untouched, since they carry no
    /// cognition inputs; only the final cognition sub-phase receives the batch. This exists
    /// so the determinism harness can keep the converse phase, and therefore the CommandKey
    /// barrier, exercised over a non-empty dialogue-move set rather than an empty one
    /// (R-HARNESS-COVER, R-CMD-ORDER); a runner with no world simply ignores the inputs.
    pub fn step_with_world_inputs(&mut self, world_inputs: &[TickInput]) {
        self.step_inner(world_inputs);
    }

    /// The shared body of [`step`](Self::step): step the field, exchange body heat, run the
    /// embodiment coupling, then tick the composed cognition world with `world_inputs` as
    /// the fixed final sub-phase. Kept private so the empty-batch and driven-batch entry
    /// points cannot diverge.
    /// Step the temperature field and, when an environmental stack is armed, advance it against the
    /// same-tick field and write the standing producer biomass into the embodiment's resource field
    /// (base-level liveliness step 2). Shared by the pinned order ([`Runner::step_inner`]) and the
    /// scheduled order (the `SYS_FIELD` phase), so both advance the field and its environment identically
    /// before the body and embodiment phases read them. The environment step is a pure deterministic fold
    /// (Principle 3); the supply write keys off the physical productivity, no label (Principles 8, 9).
    fn step_field(&mut self) {
        self.field.step(&self.calib);
        if let Some((env, calib)) = self.environ.as_mut() {
            let calib = *calib;
            env.step(&self.field, &calib);
        }
        // Regrow the standing food stock toward the freshly-derived productivity capacity and refresh the
        // drinkable water supply in the embodiment's resource field (base-level liveliness step 3), before
        // the embodiment step grazes it. The stock persists in the resource field, so this reads back last
        // tick's grazed amount and regrows it; the RES_FIELD read-after-write already serializes this
        // SYS_FIELD write before the SYS_EMBODIMENT graze in the scheduled order (matching the pinned one).
        if let Some((env, calib)) = self.environ.as_ref() {
            if let Some(emb) = self.embodiment.as_mut() {
                env.regrow_supply(emb.resources_mut(), calib);
            }
        }
        self.step_combustion();
    }

    /// The combustion beat (material-substrate arc, cascade item 6, live fire): the combustible matter each
    /// cell holds that stands at or above its ignition temperature burns through the resolved combustion law,
    /// consuming a bounded fraction of its fuel from the material field and lighting the fire field with the
    /// released energy. Run inside [`Runner::step_field`] after the temperature advances, so both tick orders
    /// (pinned and scheduled) source the fire from the settled temperature identically. A pure deterministic
    /// fold in canonical cell-and-substance order (Principle 3); the outcome keys off the substance's own
    /// combustion axes (fuel value, ignition temperature, oxidiser demand) and the cell temperature, no race,
    /// kind, or role (Principles 8, 9). Opt-in: a runner with no combustion calib, no material registry, or no
    /// combustible substance hot enough burns nothing, so the fire field stays empty and folds no bytes.
    ///
    /// The fire field is rebuilt from empty each tick, so a cell that runs out of fuel or cools below its
    /// ignition temperature drops out. The oxidiser is not yet supplied (self-oxidising fuels burn on the
    /// current data; an oxygen-demanding fuel reads zero oxidiser and so does not burn), so the medium-oxygen
    /// gate that makes fire need air is the next slice.
    fn step_combustion(&mut self) {
        let Some(calib) = self.combustion else {
            return;
        };
        // Read phase: over every cell of combustible matter, compute this tick's burn (consumed fuel volume
        // and released energy) against the settled temperature. Snapshotted so the material take below does
        // not alias the read. Borrows the embodiment and the field immutably (disjoint fields of the runner).
        let burns: Vec<(Coord3, String, Fixed, Fixed)> = {
            let Some(emb) = self.embodiment.as_ref() else {
                return;
            };
            let Some(reg) = emb.material_registry.as_ref() else {
                return;
            };
            let axis = |substance: &str, id: &str| -> Option<Fixed> {
                reg.substance(substance)
                    .and_then(|s| s.vector.get(id).copied())
            };
            let mut out = Vec::new();
            for (cell, mix) in emb.material.cells() {
                let temperature = self.field.at(cell.x, cell.y);
                for (substance, &volume) in mix.substances() {
                    // Only a substance carrying a fuel value is combustible; others (rock, soil) are skipped.
                    let Some(fuel_value) = axis(substance, "therm.fuel_value") else {
                        continue;
                    };
                    if fuel_value <= Fixed::ZERO {
                        continue;
                    }
                    let ignition =
                        axis(substance, "therm.ignition_temperature").unwrap_or(Fixed::ZERO);
                    let oxidiser_demand =
                        axis(substance, "therm.oxidiser_demand").unwrap_or(Fixed::ZERO);
                    let density = axis(substance, "mat.density").unwrap_or(Fixed::ZERO);
                    if density <= Fixed::ZERO {
                        continue; // no mass conversion without a density
                    }
                    // The fuel mass present, and the bounded fraction that can combust this tick (the reserved
                    // burn rate: a fuel burns down over a timescale, not instantly).
                    let fuel_mass = match volume.checked_mul(density) {
                        Some(m) => m,
                        None => continue,
                    };
                    let burnable = fuel_mass
                        .checked_mul(calib.burn_rate)
                        .unwrap_or(Fixed::ZERO);
                    if burnable <= Fixed::ZERO {
                        continue;
                    }
                    // The resolved combustion law gates on the ignition crossing. The oxidiser is not supplied
                    // yet (zero), so a self-oxidising fuel (zero demand) burns and an oxygen-demanding one reads
                    // oxidiser-limited to nothing, the honest limit until the medium-oxygen slice.
                    let c = laws::combustion(
                        fuel_value,
                        oxidiser_demand,
                        ignition,
                        burnable,
                        Fixed::ZERO,
                        temperature,
                        calib.energy_cap,
                    );
                    if !c.ignited || c.energy <= Fixed::ZERO {
                        continue;
                    }
                    // The burned mass follows from the released energy over the fuel value (the law's own
                    // relation, exact below the energy cap); the consumed volume converts it back through the
                    // substance density.
                    let burned_mass = c.energy.checked_div(fuel_value).unwrap_or(Fixed::ZERO);
                    let burned_volume = burned_mass.checked_div(density).unwrap_or(Fixed::ZERO);
                    if burned_volume <= Fixed::ZERO {
                        continue;
                    }
                    out.push((*cell, substance.clone(), burned_volume, c.energy));
                }
            }
            out
        };
        // Apply phase: consume the burned fuel and rebuild the fire field with this tick's released energy per
        // cell (a cell with two combustible substances sums their release). Borrows the embodiment mutably.
        let Some(emb) = self.embodiment.as_mut() else {
            return;
        };
        let mut per_cell: BTreeMap<Coord3, Fixed> = BTreeMap::new();
        for (cell, substance, burned_volume, energy) in burns {
            emb.material.take(cell, &substance, burned_volume);
            let entry = per_cell.entry(cell).or_insert(Fixed::ZERO);
            *entry = entry.saturating_add(energy);
        }
        let mut fire = FireField::new();
        for (cell, energy) in per_cell {
            fire.set(cell, energy);
        }
        emb.fire = fire;
    }

    /// Recouple the hydrology routing to the terrain this tick's digging reshaped (material-substrate item
    /// 5): after the embodiment step has moved matter and adjusted the earthwork, rebuild the environmental
    /// stack's downhill targets from the effective elevation, so next tick's hydrology and salinity route
    /// water and salt into a dug pit and off a mound. A pure deterministic fold ([`EnvironFields::recouple_terrain`]),
    /// worker-count independent, run identically after the embodiment phase in the pinned and scheduled
    /// orders. Opt-in and crucible-safe: the recompute is skipped on an empty earthwork, so a run in which
    /// nothing digs keeps the seeded worldgen routing and stays byte-identical.
    fn recouple_hydrology(&mut self) {
        if let (Some((env, _)), Some(emb)) = (self.environ.as_mut(), self.embodiment.as_ref()) {
            env.recouple_terrain(emb.earthwork());
        }
    }

    fn step_inner(&mut self, world_inputs: &[TickInput]) {
        self.step_field();
        self.phase_body_exchange();
        if self.embodiment.is_some() {
            self.step_embodiment();
        }
        self.recouple_hydrology();
        // Base-level liveliness step 5: publish each moved being's live cell into the world (so gossip
        // clusters by where it stands) and inject the environment-sourced hazard belief, then tick the
        // world with the merged batch. Runs after the embodiment moved the beings, matching the scheduled
        // order (SYS_EMBODIMENT before SYS_WORLD), so both orders publish post-movement cells.
        let inputs = self.couple_conversation(world_inputs);
        if let Some(world) = self.world.as_mut() {
            world.tick(&inputs);
        }
        self.reconcile_lifecycle();
        self.clock += 1;
    }

    /// The conversation-movement coupling and the experiential-learning belief source (harm-learning arc
    /// slice b). Republishes each located being's live cell into the cognition world as a conversational
    /// [`PlaceId`] (`CELL_PLACE_BASE + y*width + x`, a stable function of the coordinate), so gossip and
    /// converse cluster by where a being stands now rather than its frozen dawn band, and builds the
    /// being's OWN feature observations: for each present feature of the cell it stands on, one piece of
    /// evidence toward "this feature harms me" (a harm tick, its own interoceptive reserve fall) or "this
    /// feature is benign" (a harm-free tick), so the being forms the belief for itself rather than the
    /// run injecting it. Returns the caller's `world_inputs` merged with those observations (the learned
    /// ones last, at a high ordinal, so the tick's canonical mind-then-ordinal sort is deterministic).
    /// Reads the embodiment and world (immutably) before the mutable world publish, and draws no
    /// randomness, so it replays and is worker-count invariant. A runner with no embodiment publishes
    /// nothing and returns the inputs unchanged; a world that declares no percepts observes no features,
    /// so the learner is inert and the run is unchanged.
    fn couple_conversation(&mut self, world_inputs: &[TickInput]) -> Vec<TickInput> {
        // The learner calibrations (Copy), read once so the borrow of the embodiment below does not
        // conflict with the read.
        let harm_learn = self.harm_learning;
        let mut cells: BTreeMap<StableId, PlaceId> = BTreeMap::new();
        let mut env_inputs: Vec<TickInput> = Vec::new();
        // Per-being stress (the lower of its energy and condition margins) and its cell, for the
        // arc-scoped promotion policy: a being whose stress is high is in a survival arc.
        let mut stress: BTreeMap<StableId, Fixed> = BTreeMap::new();
        if let Some(emb) = self.embodiment.as_ref() {
            let (width, _) = self.field.dims();
            for w in emb.walkers() {
                let c = w.coord();
                let cell = CELL_PLACE_BASE.wrapping_add((c.y.max(0) * width + c.x.max(0)) as u32);
                cells.insert(w.id, cell);
                // The survival margin: the lower of the energy and condition reserve levels, counting an
                // axis only when the being carries that reserve (its capacity is positive), so a
                // being whose registry lacks the axis reads a full margin rather than a false zero. A being
                // whose margin is below the stress threshold is in a narrative arc (starving, or worn by a
                // hazard); a thermal-only fixture carries neither axis, so it yields a full margin and
                // promotes no one.
                let axis_margin = |axis| {
                    if w.homeostasis.capacity(axis) > Fixed::ZERO {
                        w.homeostasis.level(axis)
                    } else {
                        Fixed::ONE
                    }
                };
                let margin = axis_margin(ENERGY).min(axis_margin(CONDITION));
                stress.insert(w.id, margin);
                // Experiential associative learning (harm-learning arc slice b): the being forms the
                // belief "this feature harms me" for ITSELF, replacing the injected hazard Observe. It
                // felt harm this tick if any reserve fell beyond the metabolic-drain noise floor (its
                // OWN interoceptive delta, from the reserve-memory snapshot taken at the top of this
                // tick's embodiment step), and it senses the raw features of the cell it stands on. For
                // each present feature it contributes one piece of evidence toward HARMS (a harm tick)
                // or BENIGN (a harm-free tick), keyed on a per-feature belief subject, scaled by its own
                // heritable belief plasticity. Nothing reads a dose threshold, a hazard label, or a race
                // id: the sign is the reserve falling, the subject a raw quantized percept, so "this
                // ground harms me" emerges from the correlation (Principles 8, 9). Inert where the world
                // declares no percepts (an empty feature vector yields no observation), so an opted-out
                // run is unchanged.
                let harm = emb.homeo.axes.iter().any(|axis| {
                    is_harm_tick(
                        w.reserve_memory.delta(axis.id, &w.homeostasis),
                        harm_learn.harm_noise_floor,
                    )
                });
                let features = emb.percepts.perceive(emb.resources.composition(c));
                let plasticity = self
                    .world
                    .as_ref()
                    .and_then(|world| world.mind(w.id))
                    .map(|m| m.plasticity)
                    .unwrap_or(Fixed::ONE);
                for (k, obs) in feature_observations(harm, &features, plasticity, &harm_learn)
                    .into_iter()
                    .enumerate()
                {
                    env_inputs.push(TickInput {
                        mind: w.id,
                        ordinal: LEARN_ORDINAL_BASE + k as u32,
                        stim: Stimulus::Observe {
                            subject: obs.subject,
                            attr: HARM_ATTR,
                            hyps: vec![HARMS, BENIGN],
                            toward: obs.toward,
                            weight: obs.weight,
                            from: w.id,
                        },
                    });
                }
            }
        }
        // The promoted set: every being sharing a cell that holds a being in a survival arc (the
        // talk-hole guard promotes whole co-located groups), capped at the generous budget by keeping the
        // most-stressed cells. A pure deterministic function of the reserves and the cells (no RNG).
        let promote_set = self.arc_promotion_set(&cells, &stress);
        if let Some(world) = self.world.as_mut() {
            world.set_conversational_cells(cells);
            // The beings already promoted before this policy touches anything, minus the set the policy
            // itself held last tick, are the ones promoted by some OTHER path (a test harness, a scripted
            // scene). The policy must never claim ownership of those, so it never restricts them.
            let externally_owned: BTreeSet<StableId> = world
                .promoted_ids()
                .into_iter()
                .filter(|id| !self.arc_promoted.contains(id))
                .collect();
            // Apply the arc-scoped promotion, touching only the set this policy owns. Promote the new arc
            // set, then restrict every being the policy promoted last tick that has left the arc (its arc
            // resolved). A being promoted by another path is never in `arc_promoted`, so it is left
            // untouched, and a being still in the arc is not restricted.
            for &id in &promote_set {
                world.promote(id);
            }
            for &id in &self.arc_promoted {
                if !promote_set.contains(&id) {
                    world.restrict(id);
                }
            }
            // The policy now owns exactly the arc set it chose, minus any being another path already held
            // promoted (which stays that path's to restrict), so it can never later clobber an external
            // promotion that happened to coincide with a survival arc.
            self.arc_promoted = promote_set.difference(&externally_owned).copied().collect();
        } else {
            // No world to promote into: the policy owns nothing this tick.
            self.arc_promoted = BTreeSet::new();
        }
        if env_inputs.is_empty() {
            world_inputs.to_vec()
        } else {
            let mut merged = world_inputs.to_vec();
            merged.extend(env_inputs);
            merged
        }
    }

    /// The arc-scoped promotion set (base-level liveliness §4): the beings to promote to the individual
    /// dialogue tier this tick, from the per-being survival stress and the live conversational cells. A
    /// cell is "in an arc" when any being in it is below the stress threshold; the whole cell is promoted
    /// together (the talk-hole guard, so a promoted being has promoted partners to converse with). When
    /// more beings sit in arc-cells than the generous budget allows, the most-stressed cells win (a stable
    /// ranking by the cell's lowest margin, then its id), so the choice is deterministic and camera-free
    /// (Principle 10). Returns the promoted ids as a canonical set.
    fn arc_promotion_set(
        &self,
        cells: &BTreeMap<StableId, PlaceId>,
        stress: &BTreeMap<StableId, Fixed>,
    ) -> BTreeSet<StableId> {
        // Group beings by cell, and find each cell's lowest margin (its stress). Canonical (cell, id).
        let mut by_cell: BTreeMap<PlaceId, Vec<StableId>> = BTreeMap::new();
        for (&id, &cell) in cells {
            by_cell.entry(cell).or_default().push(id);
        }
        // Cells that hold at least one being in a survival arc, with the cell's lowest margin.
        let mut arc_cells: Vec<(Fixed, PlaceId, Vec<StableId>)> = by_cell
            .into_iter()
            .filter_map(|(cell, ids)| {
                let lowest = ids
                    .iter()
                    .map(|id| stress.get(id).copied().unwrap_or(Fixed::ONE))
                    .min()
                    .unwrap_or(Fixed::ONE);
                (lowest < self.liveliness.promotion_stress_threshold).then_some((lowest, cell, ids))
            })
            .collect();
        // Most-stressed cell first (lowest margin), ties broken by cell id, so the budget selection is
        // a deterministic, stable order.
        arc_cells.sort_by(|a, b| a.0.to_bits().cmp(&b.0.to_bits()).then(a.1.cmp(&b.1)));
        let mut promoted = BTreeSet::new();
        for (_, _, ids) in arc_cells {
            if promoted.len() >= self.liveliness.promotion_budget {
                break;
            }
            for id in ids {
                promoted.insert(id);
            }
        }
        promoted
    }

    /// The body-thermal exchange phase: every located being pulls its core temperature toward its
    /// cell's field temperature (the discrete Newton-cooling coupling), beings walked in canonical id
    /// order. Reads the field and the located index, writes the body temperatures.
    fn phase_body_exchange(&mut self) {
        let ids: Vec<StableId> = self.body_temp.keys().copied().collect();
        for id in ids {
            if let Some(coord) = self.index.coord_of(OccupantId::being(id)) {
                let env = self.field.at(coord.x, coord.y);
                let bt = self.body_temp[&id];
                // The being's own DERIVED coupling rate h*A/(m*c) when supplied, else the labelled
                // FieldCalib.exchange fixture override (a being placed without a body).
                let rate = self
                    .body_exchange_rate
                    .get(&id)
                    .copied()
                    .unwrap_or(self.calib.exchange);
                let next = bt + rate.mul(env - bt);
                self.body_temp.insert(id, next);
            }
        }
    }

    /// Set a located being's DERIVED body-to-medium exchange rate `h * A / (m * c)` per tick
    /// ([`crate::physiology::derive_body_exchange_rate`]), so its core temperature couples to its cell at
    /// a rate its own surface and thermal mass set rather than the shared [`FieldCalib::exchange`] scalar.
    /// A being with no rate set falls back to that fixture override. The rate is a fraction in `[0, 1]`
    /// (the derivation clamps it); a caller passes what the physics derivation returns.
    pub fn set_body_exchange_rate(&mut self, id: StableId, rate: Fixed) {
        self.body_exchange_rate.insert(id, rate);
    }

    /// The runner's tick phases declared as deterministic-scheduler systems over the resources they
    /// touch (design Part 57): the field step writes the field; the body-thermal exchange reads the
    /// field and the located index and writes the body temperatures; the embodiment coupling reads the
    /// field and writes the body temperatures, the index, the union being population, and the field
    /// (it recouples the hydrology routing to this tick's digging, material-substrate item 5); the
    /// cognition world reads and writes the world and the union being population. Only the phases this
    /// runner runs are declared, so a field-only runner declares two systems and a fully composed one
    /// declares four.
    ///
    /// The embodiment coupling and the cognition world both write [`RES_BEING`] (real-world
    /// unification, step 2): while world and embodiment share no being (a world-only or
    /// embodiment-only runner) that write is uncontended and changes no batching, so those paths
    /// schedule exactly as before. Once a shared [`StableId`] carries a body and a mind, the write-write
    /// conflict on [`RES_BEING`] forces the scheduler to serialize the two systems in canonical
    /// [`SystemId`] order (`SYS_EMBODIMENT` before `SYS_WORLD`), which is the pinned `step_inner` order,
    /// so the two beats that both touch a shared being cannot be reordered and the composite stays
    /// bit-identical.
    pub fn tick_systems(&self) -> BTreeMap<SystemId, Access> {
        let mut sys = BTreeMap::new();
        sys.insert(SYS_FIELD, access(&[], &[RES_FIELD]));
        sys.insert(SYS_BODY, access(&[RES_FIELD, RES_INDEX], &[RES_BODY]));
        if self.embodiment.is_some() {
            sys.insert(
                SYS_EMBODIMENT,
                access(&[RES_FIELD], &[RES_FIELD, RES_BODY, RES_INDEX, RES_BEING]),
            );
        }
        if self.world.is_some() {
            sys.insert(
                SYS_WORLD,
                access(&[RES_WORLD, RES_BEING], &[RES_WORLD, RES_BEING]),
            );
        }
        sys
    }

    /// Run one tick phase by its [`SystemId`], the dispatch the scheduled executor drives.
    fn run_phase(&mut self, sid: SystemId, world_inputs: &[TickInput]) {
        if sid == SYS_FIELD {
            self.step_field();
        } else if sid == SYS_BODY {
            self.phase_body_exchange();
        } else if sid == SYS_EMBODIMENT {
            self.step_embodiment();
            // Recouple the hydrology to this tick's digging, exactly as step_inner does after
            // step_embodiment (material-substrate item 5): a pure fold touching only the environmental
            // downhill routing, which no other phase reads this tick, so the placement is order-safe and
            // the scheduled and pinned orders stay bit-identical.
            self.recouple_hydrology();
        } else if sid == SYS_WORLD {
            // The conversation-movement coupling and env belief source run here (base-level liveliness
            // step 5), after SYS_EMBODIMENT (serialized by the RES_BEING edge), exactly as step_inner runs
            // them after step_embodiment, so the scheduled and pinned orders publish identical cells and
            // inject identical env observations.
            let inputs = self.couple_conversation(world_inputs);
            if let Some(world) = self.world.as_mut() {
                world.tick(&inputs);
            }
        }
    }

    /// One tick run through the deterministic scheduler (design Part 57): the phases are declared as
    /// systems over their resources, the scheduler derives conflict-free batches from the
    /// declarations, and the flattened schedule runs them. When no being is shared, the cognition
    /// world shares no resource with the field phases, so the scheduler places the world tick in the
    /// first batch alongside the field step (a parallelisable pair); when a being is shared, the
    /// [`RES_BEING`] write the world and the embodiment coupling both declare serializes those two
    /// systems in the pinned order (real-world unification, step 2). Either way the result is
    /// bit-identical to the pinned-order [`step`](Self::step): the reordered or serialized phases do
    /// not conflict on any resource, and the counter RNG is draw-keyed rather than sequential
    /// (R-RNG-COORD), so the reorder cannot change any draw. This is the runner as the scheduler's
    /// first real tick, proven equivalent to the hand-pinned order.
    pub fn step_scheduled(&mut self, world_inputs: &[TickInput]) {
        let sch = schedule(&self.tick_systems());
        run_serial(&sch, |sid| self.run_phase(sid, world_inputs));
        // The lifecycle pairing runs after the scheduled phases exactly as it does after the pinned
        // order in step_inner: it is a pure deterministic function of the post-tick world and embodiment
        // state (worker-count independent), so both tick entry points reconcile identically and stay
        // bit-identical (real-world unification, step 3c).
        self.reconcile_lifecycle();
        self.clock += 1;
    }

    /// The embodiment coupling sub-phase: sense each being's thermal comfort gradient from the field
    /// (physics to percept), map its field-driven core temperature to its temperature reserve through
    /// the comfort-band map (physics to physiology), let its evolved controller drive one step of
    /// locomotion (physiology and percept to behaviour), and re-sync the located index to the beings'
    /// new coordinates (behaviour to physics). The comfort gradient and the comfort-band map are pure,
    /// and the locomotion draws its exploration heading from the being-and-tick-keyed RNG, so the
    /// sub-phase authors no heading and reproduces bit for bit. Distinct fields of the runner are
    /// borrowed disjointly, so the field, the body-temperature map, the located index, and the
    /// embodiment are touched without contention.
    fn step_embodiment(&mut self) {
        let (width, height) = self.field.dims();
        let terrain = BoundedPlane { width, height };
        // (0) Physics to percept: each being senses the raw temperature gradient at its cell, the unit
        // direction toward warmer surroundings (what a thermoreceptor senses), as the TEMPERATURE axis's
        // directional percept. Read from the field (immutable) before the mutable embodiment borrow; a
        // pure field quantity, drawing no RNG. It is a percept, not a heading: a controller must evolve
        // to act on it, and how it combines it with its comfort reserve is selection's to wire.
        let harm_gran = self.harm_learning.feature_granularity;
        let field_dirs: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>> =
            match self.embodiment.as_ref() {
                Some(emb) => {
                    // The belief-avoidance gradient (harm-learning arc slice c) routes into the CONDITION
                    // axis's direction slot, so a being that has learned some ground harms it senses a
                    // unit direction away from it. Only where the registry carries CONDITION.
                    let condition_reg = emb.homeo.axis(CONDITION).is_some();
                    emb.walkers
                        .iter()
                        .map(|w| {
                            let coord = w.coord();
                            let (gx, gy) = self.field.gradient_at(coord.x, coord.y);
                            let mut dirs = BTreeMap::from([(TEMPERATURE, unit(gx, gy))]);
                            // The avoidance percept: the belief-derived expected-harm gradient into
                            // CONDITION's dead direction slot, present only when the world carries the
                            // being's mind. A zero gradient (no learned harm nearby) is not inserted, so a
                            // being with no harmful belief is unchanged; and the evolved
                            // CONDITION-dir-to-heading weight (founding-zero) must be lifted by selection
                            // before the gradient moves the being, so avoidance emerges (Principle 9).
                            if condition_reg {
                                if let Some(world) = self.world.as_ref() {
                                    if let Some(mind) = world.mind(w.id) {
                                        let raw = avoidance_gradient(
                                            mind,
                                            coord,
                                            &emb.resources,
                                            &emb.percepts,
                                            emb.params.sense_range,
                                            harm_gran,
                                            world.belief_params(),
                                        );
                                        let (ax, ay) = unit(raw.0, raw.1);
                                        if ax != Fixed::ZERO || ay != Fixed::ZERO {
                                            dirs.insert(CONDITION, (ax, ay));
                                        }
                                    }
                                }
                            }
                            (w.id, dirs)
                        })
                        .collect()
                }
                None => return,
            };
        // (0b) Physics to percept, the signed thermoreceptor: each being senses the signed deviation of
        // its core temperature from its own comfort set point (too hot positive, too cold negative), a
        // raw interoceptive scalar in [-1, 1], fed into the TEMPERATURE axis's signed input slot. This is
        // the bit the even comfort reserve cannot carry; it is a percept, not a heading (it says the body
        // is too hot, not which way to flee), so a controller must combine it with the gradient percept
        // to act, and selection wires that. Read before the mutable embodiment borrow, drawing no RNG.
        let field_signed: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, Fixed>> =
            match self.embodiment.as_ref() {
                Some(emb) => emb
                    .walkers
                    .iter()
                    .filter_map(|w| {
                        let bt = *self.body_temp.get(&w.id)?;
                        let band = emb.thermal.get(&w.id)?;
                        Some((
                            w.id,
                            BTreeMap::from([(TEMPERATURE, signed_deviation(bt, band))]),
                        ))
                    })
                    .collect(),
                None => return,
            };
        // (0c) Physics to physiology, the anatomy-derived metabolism (R-METABOLIZE): when a physiology
        // is installed, build each being's per-axis DERIVED drain so its survival follows its body plan,
        // mass, tissue, medium, and temperature rather than the axis defs' authored scalars. The
        // metabolic (energy-density-backed) axis drains at the Kleiber basal rate plus the
        // thermoregulatory replacement, read against the being's LIVE post-exchange core temperature as
        // the effective ambient and its own comfort set point (so the base drain tracks the medium each
        // tick, unlike the static exchange rate), plus a work-derived exertion coupling; every other
        // axis keeps its authored per-axis rate. Keyed off the floor axis id, no race branch (Principle
        // 11). An empty map (no physiology) leaves locomotion on the scalar metabolize. Read before the
        // mutable borrow, drawing no RNG.
        let drains: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, DerivedDrain>> =
            match self.embodiment.as_ref() {
                Some(emb) => match &emb.physiology {
                    Some(phys) => emb
                        .walkers
                        .iter()
                        .map(|w| {
                            let ambient = self.body_temp.get(&w.id).copied();
                            let setpoint = emb.thermal.get(&w.id).map(|b| b.setpoint);
                            let (ambient, setpoint) = match (ambient, setpoint) {
                                (Some(a), Some(s)) => (a, s),
                                (Some(a), None) => (a, a),
                                (None, Some(s)) => (s, s),
                                (None, None) => (Fixed::ZERO, Fixed::ZERO),
                            };
                            (w.id, being_derived_drains(emb, phys, w, ambient, setpoint))
                        })
                        .collect(),
                    None => BTreeMap::new(),
                },
                None => return,
            };
        let Some(emb) = self.embodiment.as_mut() else {
            return;
        };
        // (1) Physics to physiology: the comfort-band map turns each being's core temperature into its
        // temperature reserve, per being from its own reserved band. No behaviour, no RNG.
        for w in emb.walkers.iter_mut() {
            if let (Some(&bt), Some(band)) = (self.body_temp.get(&w.id), emb.thermal.get(&w.id)) {
                w.homeostasis
                    .set_level(TEMPERATURE, comfort_fraction(bt, band));
            }
        }
        // (1a) Physics to physiology, whole-body coherence (emergent-anatomy Step 3, the viability cull):
        // a GROWN body's derived integrity reserve is set each tick to the greatest capability its grown
        // segments read on ANY function law ([`crate::morphogen::Structure::whole_body_viability`]), a pure
        // physics read. A body that reads no viable function is inert matter no life function can run on, so
        // its integrity falls to the floor and it dies through the SAME reserve-floor cull as any other
        // death (`metabolize` below reads `is_alive` over every axis, then `reconcile_lifecycle` retires the
        // body), with no predicate that inspects morphology to reject it (Principle 8). Gated on the registry
        // carrying the integrity axis and the walker carrying a grown structure, so a catalog body and a
        // registry without integrity (every existing run scenario) are untouched: the cull is opt-in and
        // hash-neutral until a race both grows its body and declares the integrity axis.
        if emb.homeo.axis(INTEGRITY).is_some() {
            let fns = FunctionLawRegistry::dev_seed();
            let refs = &emb.params.capability_refs;
            let caps = &emb.params.capability_caps;
            for w in emb.walkers.iter_mut() {
                if let Some(s) = w.structure.as_ref() {
                    let viability = s.whole_body_viability(&fns, refs, caps);
                    w.homeostasis.set_level(INTEGRITY, viability);
                }
            }
        }
        // (1b) Physics to physiology, medium respiration (R-MEDIUM): when a physiology is installed and
        // the registry carries a RESPIRATION axis, each being exchanges its respirable-gas reserve with
        // the medium of the cell it stands in, through the Fick membrane law over its respiratory exchange
        // area, in canonical id order (the walkers are id-sorted on the prior locomotion step). Breathe in
        // before this tick's metabolic draw (matching the medium coupling's tested order), so the death
        // check inside locomotion accounts for the tick's uptake. A body with no respiratory surface takes
        // up nothing and suffocates on its buffer, whatever the medium (Principle 9), and a being off the
        // field finds no medium and suffocates on its buffer likewise. The medium is now per-cell
        // ([`medium::respire_at`] reading the being's coordinate), so a body in a water cell respires that
        // cell's content and one in an air cell that cell's; a single-medium world folds to a uniform field.
        if let Some(phys) = emb.physiology.as_ref() {
            if emb.homeo.axis(RESPIRATION).is_some() {
                for w in emb.walkers.iter_mut() {
                    let coord = w.coord();
                    medium::respire_at(
                        &mut w.homeostasis,
                        &w.body,
                        &phys.organs,
                        &phys.medium,
                        coord,
                        phys.respiration_transfer_k,
                        RESPIRATION_FLUX_MAX,
                    );
                }
            }
        }
        // (2) Physiology and percept to behaviour: the evolved controllers drive one locomotion step
        // over the being slice, reading the temperature gradient in the TEMPERATURE direction slot and
        // the signed thermoreceptor in its signed slot. A controller that has evolved to gate its
        // gradient-following on the signed bit climbs toward comfort from either side; one that has not
        // explores or, wired for one side only, walks into danger on the other. The per-being derived
        // drain (when a physiology is installed) is applied through metabolize_derived inside the step.
        // Material-substrate item 3: the per-being carried-load speed penalty. A load factor (>= 1)
        // divides a laden being's ground speed (`1 + load_penalty * carried_weight / carry_capacity`),
        // so a being near its strength limit moves slowest. Empty unless the embodiment declares a
        // material registry and a being carries a load, so an opted-out run inserts no factor and every
        // existing scenario is byte-identical.
        let load_factors: BTreeMap<StableId, Fixed> =
            match (&emb.material_registry, &emb.physiology) {
                (Some(reg), Some(phys)) => emb
                    .walkers
                    .iter()
                    .filter_map(|w| {
                        if w.carried.is_empty() {
                            return None;
                        }
                        let capacity = being_muscle_force(w, phys);
                        if capacity <= Fixed::ZERO {
                            return None;
                        }
                        let weight = w.carried.weight(reg, standard_gravity(), FORCE_CEILING);
                        let ratio = weight.checked_div(capacity).unwrap_or(Fixed::ZERO);
                        let factor = Fixed::ONE
                            + emb
                                .params
                                .load_penalty
                                .checked_mul(ratio)
                                .unwrap_or(Fixed::ZERO);
                        Some((w.id, factor))
                    })
                    .collect(),
                _ => BTreeMap::new(),
            };
        // Material-substrate items 3 and 4, the drivers: the per-being matter decisions this step records,
        // keyed by id, each the decided affordance (GRASP or EXTRACT) and its evolved activation for a being
        // whose controller chose to act on the matter underfoot. Empty unless such an output wins a being's
        // decision, so an opted-out run (no grasp or extract affordance, no such weight) records nothing and
        // enacts nothing.
        let mut deferred_actions: BTreeMap<StableId, (AffordanceId, Fixed)> = BTreeMap::new();
        locomotion::step_with_field_dirs(
            &mut emb.walkers,
            &emb.homeo,
            &emb.layout,
            &emb.afford,
            &emb.organs,
            &terrain,
            &mut emb.resources,
            &emb.params,
            emb.seed,
            self.clock,
            &field_dirs,
            &field_signed,
            &drains,
            &emb.percepts,
            &load_factors,
            &mut deferred_actions,
        );
        // (2b) Behaviour to matter: enact the matter decisions in id order (the map is id-keyed, so the
        // draw off the shared cell is deterministic), dispatched by the decided affordance: GRASP lifts the
        // loose matter underfoot bounded by strength (item 3, the driver), EXTRACT breaks bonded matter
        // loose in a fracture contest and takes it (item 4). A being that decided neither, or an embodiment
        // with no material registry, moves no matter, so an opted-out run is byte-identical through here.
        // The being did not move this tick (its matter action won its decision over MOVE), so its cell is
        // where it stood when it decided.
        for (&id, &(affordance, activation)) in deferred_actions.iter() {
            if activation > Fixed::ZERO {
                match affordance {
                    GRASP => {
                        emb.grasp_underfoot(id);
                    }
                    EXTRACT => {
                        emb.extract_underfoot(id);
                    }
                    GEOPHAGE => {
                        emb.geophage(id);
                    }
                    CRAFT => {
                        emb.craft_from_carried(id);
                    }
                    DIG => {
                        emb.dig_underfoot(id);
                    }
                    RELEASE => {
                        emb.release_underfoot(id);
                    }
                    _ => {}
                }
            }
        }
        // (3) Behaviour to physics: the beings' new coordinates re-sync the located index, so next
        // tick's thermal exchange reads where they moved.
        for w in emb.walkers.iter() {
            self.index.place(OccupantId::being(w.id), w.coord());
        }
    }

    /// Reconcile the embodied population against the cognition world after a tick (real-world
    /// unification, step 3c: lifecycle pairing). Births in [`crate::world::World`] reproduction and
    /// deaths in world mortality and in locomotion each happen on their own half; this beat re-pairs the
    /// two so a shared being's body and mind stay in lockstep before [`Runner::state_hash`] folds, so no
    /// dead being's body keeps metabolizing and a child of embodied parents is itself embodied. It is a
    /// pure deterministic function of the post-tick world and embodiment state (no RNG: a newborn's body
    /// plan, genome-expressed controller, and comfort band are all deterministic), walked in canonical
    /// id order, so it replays bit for bit and is independent of the field-worker width. It runs only on
    /// the unified path (both world and embodiment present) and identically after the pinned
    /// ([`Runner::step`]) and scheduled ([`Runner::step_scheduled`]) orders.
    ///
    /// The reconciliations, in order: (1) a body that died in locomotion (`alive = false`) propagates
    /// its death to the world, so a starved or suffocated body ends the whole being; (2) every body
    /// whose mind is gone from the world (world mortality culled it, or step 1 just did) is retired;
    /// (3) every newborn mind whose race carries a body plan and has no body yet is embodied, its body
    /// expressed from its race and genome as the dawn assembly expresses a founder. A mind whose race
    /// carries no body plan stays a bodiless mind (owner ruling 2026-07-04), so the pairing is optional.
    fn reconcile_lifecycle(&mut self) {
        if self.world.is_none() || self.embodiment.is_none() {
            return;
        }
        // (1) Locomotion deaths propagate to the world. Record each death's cause (the reserve axis that
        // fell to its death floor) on the non-folded observability log before removal, a pure read of the
        // dying being's own homeostasis, never canonical state.
        let dead: Vec<(StableId, Option<HomeostaticAxisId>)> = {
            let emb = self.embodiment.as_ref().unwrap();
            emb.walkers
                .iter()
                .filter(|w| !w.alive)
                .map(|w| (w.id, w.homeostasis.dead_axis(&emb.homeo)))
                .collect()
        };
        if !dead.is_empty() {
            for (_, cause) in &dead {
                if let Some(axis) = cause {
                    self.obs_deaths.push(*axis);
                }
            }
            let world = self.world.as_mut().unwrap();
            for (id, _) in &dead {
                world.remove_being(*id);
            }
        }
        // (2) Retire every body whose mind is gone from the world, in canonical id order.
        let live_minds: BTreeSet<StableId> = self
            .world
            .as_ref()
            .unwrap()
            .being_ids()
            .into_iter()
            .collect();
        let retire: Vec<StableId> = self
            .embodiment
            .as_ref()
            .unwrap()
            .walkers
            .iter()
            .map(|w| w.id)
            .filter(|id| !live_minds.contains(id))
            .collect();
        for id in retire {
            self.retire_body(id);
        }
        // (3) Embody every newborn (a world mind whose race carries a body plan and has no body yet), in
        // canonical id order. Requires the lifecycle kit; without it a newborn stays a bodiless mind.
        if self.lifecycle.is_none() {
            return;
        }
        let embodied: BTreeSet<StableId> = self
            .embodiment
            .as_ref()
            .unwrap()
            .walkers
            .iter()
            .map(|w| w.id)
            .collect();
        let newborns: Vec<StableId> = {
            let world = self.world.as_ref().unwrap();
            world
                .being_ids()
                .into_iter()
                .filter(|id| !embodied.contains(id))
                .filter(|&id| {
                    world
                        .race_of(id)
                        .and_then(|rid| world.race(rid))
                        // A race founds embodied members if it declares a catalog body OR a developmental
                        // program: a fully grown race (Step 3, the metabolic-tier grow) needs no catalog body.
                        .map(|race| race.body.is_some() || race.morphogen.is_some())
                        .unwrap_or(false)
                })
                .collect()
        };
        for id in newborns {
            self.embody_newborn(id);
        }
    }

    /// Retire a body from the embodiment and every runner-side map it appears in: its walker, comfort
    /// band, body temperature, derived exchange rate, and located-index entry, so a dead being leaves no
    /// half behind (referential integrity, design Part 58). Preserves the relative order of the
    /// surviving walkers, which does not affect [`Runner::state_hash`] (it sorts walkers by id) but keeps
    /// the walk deterministic.
    fn retire_body(&mut self, id: StableId) {
        if let Some(emb) = self.embodiment.as_mut() {
            emb.walkers.retain(|w| w.id != id);
            emb.thermal.remove(&id);
        }
        self.body_temp.remove(&id);
        self.body_exchange_rate.remove(&id);
        self.index.remove(OccupantId::being(id));
    }

    /// Embody a newborn mind: mint a paired body reusing the mind id (never a second registry), its body
    /// plan and genes its race's and its genome its own, expressed exactly as the dawn assembly expresses
    /// a founder, then seed its runner-side state (comfort band, body temperature, located index, derived
    /// exchange rate) as [`Runner::with_world_and_embodiment`] seeds a founder. Everything is gathered
    /// under shared borrows and released before the mutation. A newborn whose place is not in the spawn
    /// map, or a run with no installed physiology, is skipped rather than embodied on a fabricated input.
    fn embody_newborn(&mut self, id: StableId) {
        let gathered = {
            let world = self.world.as_ref().unwrap();
            let emb = self.embodiment.as_ref().unwrap();
            let kit = self.lifecycle.as_ref().unwrap();
            let race = world.race_of(id).and_then(|rid| world.race(rid));
            let place_coord = world
                .place_of(id)
                .and_then(|place| kit.spawn_by_place.get(&place).copied());
            match (race, place_coord, emb.physiology.as_ref()) {
                (Some(race), Some(coord), Some(phys)) => {
                    // Grow the newborn's run-body from its OWN genome (emergent-anatomy Step 2), so a
                    // lineage's evolved morphology governs the child's run affordances and ground speed.
                    // Growth keys on (program, genome, emb.seed, id), a pure function reproduced on replay
                    // and on a two-tier reload where the walker is regrown from the re-minted genome.
                    let structure = match (&race.morphogen, world.genome_of(id)) {
                        (Some(program), Some(genome)) => {
                            let params = express_program(program, &race.genes, genome);
                            Some(grow(program, &params, emb.seed, id))
                        }
                        _ => None,
                    };
                    // The metabolic body and reserves, exactly as the worldbuild founder step: a race with a
                    // catalog body keeps it as the metabolic aggregate (its catalog organs source the
                    // reserves, unchanged); a FULLY GROWN race (no catalog body) sources both from its grown
                    // structure (the digest and the grown tissue), so it needs no catalog body (Step 3, the
                    // metabolic-tier grow).
                    let body_homeo = if let Some(plan) = &race.body {
                        Some((
                            plan.clone(),
                            Homeostasis::new(&emb.homeo, plan, &phys.organs),
                        ))
                    } else {
                        structure
                            .as_ref()
                            .map(|s| (s.digest(), Homeostasis::from_structure(&emb.homeo, s)))
                    };
                    let Some((body, homeostasis)) = body_homeo else {
                        return; // a grown race whose newborn has no genome yet: cannot embody
                    };
                    let controller = match world.genome_of(id) {
                        Some(genome) => Controller::express(&race.genes, genome, &emb.layout),
                        None => Controller::zeros(&emb.layout),
                    };
                    // The newborn's consumer physiology, its heritable per-toxin-class tolerance expressed
                    // from its OWN genome through the embodiment's tolerance registry (base-level liveliness
                    // step 4), so salt (or dust) resistance is inherited and selection carries across
                    // generations. A newborn with no genome falls back to the tolerance-free dev fixture.
                    let physiology = match world.genome_of(id) {
                        Some(genome) => {
                            Physiology::express(&emb.homeo, &emb.tolerances, &race.genes, genome)
                        }
                        None => Physiology::dev_for_registry(&emb.homeo),
                    };
                    let exchange_rate = walker_exchange_rate(&body, &structure, phys);
                    let mut walker =
                        Walker::new(id, coord, body, homeostasis, physiology, controller);
                    if let Some(s) = structure {
                        walker = walker.with_structure(s);
                    }
                    Some((walker, kit.thermal, coord, exchange_rate))
                }
                _ => None,
            }
        };
        let Some((walker, thermal, coord, exchange_rate)) = gathered else {
            return;
        };
        let emb = self.embodiment.as_mut().unwrap();
        emb.add(walker, thermal);
        self.body_temp.insert(id, thermal.initial_temp);
        self.index.place(OccupantId::being(id), coord);
        self.body_exchange_rate.insert(id, exchange_rate);
    }

    /// The canonical state hash: the clock, the field, and every located being's temperature in id
    /// order, then (for a composed runner) the world's canonical hash, then (for a coupled runner) each
    /// embodied being's position, reserves, and controller hidden state in id order. A run reproduces
    /// this bit for bit; it is independent of thread count and camera. The world hash is left
    /// byte-identical to [`crate::world::World::state_hash`]; a runner without a world or an embodiment
    /// omits that side, so a field-only runner's hash is unchanged by this composition.
    pub fn state_hash(&self) -> u128 {
        let mut h = StateHasher::new();
        h.write_u64(self.clock);
        self.field.hash(&mut h);
        // The environmental field stack (base-level liveliness step 2), folded in canonical field order
        // after the temperature field. A field left out here would pass replay while hiding divergence,
        // so the dynamic environmental fields fold with the temperature field.
        if let Some((env, _)) = &self.environ {
            env.hash_into(&mut h);
        }
        for (id, t) in &self.body_temp {
            h.write_stable(*id);
            h.write_fixed(*t);
        }
        if let Some(world) = &self.world {
            let wh = world.state_hash();
            h.write_u64((wh >> 64) as u64);
            h.write_u64(wh as u64);
        }
        if let Some(emb) = &self.embodiment {
            // The standing food-and-water stock the grazers deplete and the environment regrows (base-
            // level liveliness step 3): dynamic state that must fold, or a divergence in the regrow-and-
            // graze loop would pass replay while hiding. Walks canonical (coordinate, class) order.
            emb.resources.hash_into(&mut h);
            // The located material substrate (material-substrate arc, cascade item 1): the per-cell
            // substance mixture the world is made of, folded beside the resource field in canonical
            // (Coord3, substance-id, volume) order. Empty for every scenario that declares no material
            // layer, so folding it writes no bytes and the run is byte-identical (the opt-in default).
            emb.material.hash_into(&mut h);
            // The earthwork delta (material-substrate arc, cascade item 5): the per-column elevation change
            // digging has made, folded after the material layer in canonical (column, delta) order. Empty
            // for every scenario where nothing digs, so it folds no bytes and the run is byte-identical.
            emb.earthwork.hash_into(&mut h);
            // The fire field (material-substrate arc, cascade item 6): the per-cell combustion energy released
            // this tick, folded after the earthwork in canonical (cell, intensity) order. Empty for every
            // scenario with no combustion armed or nothing burning, so it folds no bytes and the run is
            // byte-identical (the opt-in empty-default).
            emb.fire.hash_into(&mut h);
            let mut ordered: Vec<&Walker> = emb.walkers.iter().collect();
            ordered.sort_by_key(|w| w.id);
            for w in ordered {
                h.write_stable(w.id);
                h.write_fixed(w.x);
                h.write_fixed(w.y);
                for axis in &emb.homeo.axes {
                    h.write_fixed(w.homeostasis.level(axis.id));
                }
                for hv in &w.hidden {
                    h.write_fixed(*hv);
                }
                // The interoceptive delta memory (harm-learning arc slice a): new per-being dynamic
                // state, folded in canonical axis order after the hidden state. Empty (never
                // snapshotted) where the world declares no percepts, so it folds nothing and leaves an
                // opted-out run's hash unchanged.
                if !w.reserve_memory.is_empty() {
                    for axis in &emb.homeo.axes {
                        h.write_fixed(w.reserve_memory.prev_level(axis.id));
                    }
                }
                // The carried matter (material-substrate arc, cascade item 3): the load a being bears,
                // per-being dynamic state folded after the reserve memory in canonical (substance-id,
                // volume) order. Empty for a being carrying nothing, so it folds nothing and leaves an
                // opted-out run's hash unchanged.
                if !w.carried.is_empty() {
                    w.carried.hash_into(&mut h);
                }
                // The wielded tool (material-substrate arc, cascade item 4, crafting): the worked object a
                // being bears, folded after the carried matter. `None` for a being wielding nothing, so it
                // folds nothing and leaves an opted-out run's hash unchanged.
                if let Some(tool) = &w.wielded {
                    tool.hash_into(&mut h);
                }
            }
        }
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::CalibrationManifest;
    use crate::scenario::Scenario;

    /// A manifest with the three field calibrations set to labelled fixture values.
    const SET: &str = r#"
[[reserved]]
id = "field.diffusion"
basis = "fixture"
status = "set"
value = "0.125"
unit = "ratio_per_tick"
source = "test"
[[reserved]]
id = "field.relaxation"
basis = "fixture"
status = "set"
value = "0.0625"
unit = "ratio_per_tick"
source = "test"
[[reserved]]
id = "field.body_exchange"
basis = "fixture"
status = "set"
value = "0.25"
unit = "ratio_per_tick"
source = "test"
"#;

    /// A FieldCalib fixture (labelled, not owner canon): a still field (no diffusion or relaxation) so
    /// the body-exchange phase is exercised in isolation, and a fallback exchange rate.
    fn calib() -> FieldCalib {
        FieldCalib {
            diffusion: Fixed::ZERO,
            relaxation: Fixed::ZERO,
            exchange: Fixed::from_ratio(1, 4),
        }
    }

    /// Real air and water thermal profiles (Incropera and DeWitt), the medium `require_map` axes the
    /// derivation reads. Labelled fixtures, not owner canon: a canonical run reads the reserved
    /// `medium.{name}` profile, which is fail-loud until the owner sets it.
    const AIR_K: Fixed = Fixed::from_bits((262 << Fixed::FRAC_BITS) / 10_000); // 0.0262 W/m/K
    const WATER_K: Fixed = Fixed::from_bits((606 << Fixed::FRAC_BITS) / 1_000); // 0.606 W/m/K

    #[test]
    fn field_diffusion_derives_from_the_medium_and_two_media_diverge_under_the_bound() {
        // A fixture cell size and the one-second base tick chosen so both media land representable and
        // sub-bound: at these scales air's derived coefficient is near the stability rail and water's
        // is far below it, purely from k/(rho*c). The medium SELECTION is the lever.
        let cell = Fixed::from_ratio(1, 100); // one-centimetre fixture cell
        let dt = Fixed::ONE; // time.base_tick_seconds = 1
        let air = derive_field_diffusion(
            AIR_K,
            Fixed::from_ratio(12, 10),
            Fixed::from_int(1005),
            cell,
            dt,
        );
        let water = derive_field_diffusion(
            WATER_K,
            Fixed::from_int(1000),
            Fixed::from_int(4186),
            cell,
            dt,
        );
        assert!(air > Fixed::ZERO, "air conducts");
        assert!(water > Fixed::ZERO, "water conducts");
        assert_ne!(
            air, water,
            "the two media give different diffusion coefficients"
        );
        assert!(
            air > water,
            "air conducts heat faster than water from k/(rho*c) ({air:?} > {water:?})"
        );
        assert!(
            air < STENCIL_STABILITY_BOUND && water < STENCIL_STABILITY_BOUND,
            "both derived coefficients stay under the four-neighbour stencil's 0.25 stability bound"
        );
    }

    #[test]
    fn a_pathological_medium_is_clamped_to_the_stencil_bound_not_beyond() {
        // A high-conductivity, tiny-heat-capacity, tiny-cell fixture drives the raw coefficient past
        // the stability bound; the derivation clamps it to the bound rather than destabilizing the
        // stencil, so no medium selection can break the field step.
        let clamped = derive_field_diffusion(
            Fixed::from_int(500),
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(1, 1000),
            Fixed::ONE,
        );
        assert_eq!(
            clamped, STENCIL_STABILITY_BOUND,
            "an unstable raw coefficient is clamped to the stability bound"
        );
    }

    #[test]
    fn the_field_step_reads_the_derived_medium_diffusion() {
        // A hot cell in the middle of a cool row; one step with the medium-derived diffusion spreads
        // heat to the neighbours, and the denser-conducting air spreads more than water in one step,
        // so the field's evolution follows the selected medium.
        let hot_row = || vec![Fixed::ZERO, Fixed::from_int(100), Fixed::ZERO];
        let cell = Fixed::from_ratio(1, 100);
        let dt = Fixed::ONE;
        let air_diff = derive_field_diffusion(
            AIR_K,
            Fixed::from_ratio(12, 10),
            Fixed::from_int(1005),
            cell,
            dt,
        );
        let water_diff = derive_field_diffusion(
            WATER_K,
            Fixed::from_int(1000),
            Fixed::from_int(4186),
            cell,
            dt,
        );
        let field_after = |diff: Fixed| {
            let mut f = Field::new(3, 1, hot_row());
            f.step(&FieldCalib {
                diffusion: diff,
                relaxation: Fixed::ZERO,
                exchange: Fixed::ZERO,
            });
            (f.at(0, 0), f.at(1, 0))
        };
        let (air_edge, air_centre) = field_after(air_diff);
        let (water_edge, _water_centre) = field_after(water_diff);
        assert!(
            air_edge > Fixed::ZERO,
            "the medium-derived diffusion conducted heat into the neighbour"
        );
        assert!(
            air_centre < Fixed::from_int(100),
            "and drew it out of the hot cell"
        );
        assert!(
            air_edge > water_edge,
            "air's faster medium diffusion spreads more heat in one step than water's ({air_edge:?} > {water_edge:?})"
        );
    }

    #[test]
    fn from_manifest_with_medium_fails_loud_while_the_profile_is_reserved_and_derives_once_set() {
        // A manifest whose medium profile is still reserved: the derivation refuses to run rather than
        // fabricating a diffusivity (Principle 11).
        let reserved = format!(
            "{SET}\n[[reserved]]\nid = \"medium.air\"\nbasis = \"b\"\nstatus = \"reserved\"\nvalue = \"\"\nunit = \"medium_profile\"\nsource = \"t\"\n[[reserved]]\nid = \"field.cell_size\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"0.01\"\nunit = \"m\"\nsource = \"t\"\n[[reserved]]\nid = \"time.base_tick_seconds\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"1\"\nunit = \"s\"\nsource = \"t\"\n"
        );
        let m = CalibrationManifest::from_toml_str(&reserved).unwrap();
        assert!(
            FieldCalib::from_manifest_with_medium(&m, "medium.air").is_err(),
            "a reserved medium profile fails loud"
        );

        // Once the owner sets the profile (with the conductivity and specific-heat axes), the field
        // calibration derives its diffusion from it.
        let set = reserved.replace(
            "id = \"medium.air\"\nbasis = \"b\"\nstatus = \"reserved\"\nvalue = \"\"",
            "id = \"medium.air\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"conductivity=0.0262,density=1.2,specific_heat=1005\"",
        );
        let m2 = CalibrationManifest::from_toml_str(&set).unwrap();
        let calib = FieldCalib::from_manifest_with_medium(&m2, "medium.air").unwrap();
        assert!(
            calib.diffusion > Fixed::ZERO && calib.diffusion < STENCIL_STABILITY_BOUND,
            "the derived diffusion is positive and sub-bound ({:?})",
            calib.diffusion
        );
    }

    /// A fixture manifest carrying the real air and water medium profiles (Incropera and DeWitt) set,
    /// a one-centimetre cell and one-second base tick chosen so both derived coefficients land
    /// representable and sub-bound, and a still-field relaxation (zero) so the observable test isolates
    /// diffusion. Labelled fixtures, not owner canon: a canonical run reads the reserved
    /// `medium.{name}` profiles, which are fail-loud until the owner sets them.
    const WIRING_MANIFEST: &str = r#"
[[reserved]]
id = "medium.air"
basis = "fixture: Incropera and DeWitt air near 300 K"
status = "set"
value = "conductivity=0.0262,density=1.2,specific_heat=1005"
unit = "medium_profile"
source = "test"
[[reserved]]
id = "medium.water"
basis = "fixture: Incropera and DeWitt liquid water near 300 K"
status = "set"
value = "conductivity=0.606,density=1000,specific_heat=4186"
unit = "medium_profile"
source = "test"
[[reserved]]
id = "field.cell_size"
basis = "fixture: one-centimetre cell, both media representable and sub-bound"
status = "set"
value = "0.01"
unit = "metres_per_cell"
source = "test"
[[reserved]]
id = "time.base_tick_seconds"
basis = "fixture"
status = "set"
value = "1"
unit = "s"
source = "test"
[[reserved]]
id = "field.relaxation"
basis = "fixture: a still field, so the test isolates medium-derived diffusion"
status = "set"
value = "0"
unit = "ratio_per_tick"
source = "test"
[[reserved]]
id = "field.body_exchange"
basis = "fixture"
status = "set"
value = "0.1"
unit = "ratio_per_tick"
source = "test"
"#;

    /// A hot-spot field: a cool plane with one hot cell at its centre. The baseline carries the spot
    /// and the relaxation coefficient is zero on this path, so a step conducts the spot outward at the
    /// calibration's diffusion rate and nothing pulls it back.
    fn hot_spot_field() -> Field {
        let (w, h) = (5, 5);
        let mut baseline = vec![Fixed::ZERO; (w * h) as usize];
        baseline[(2 * w + 2) as usize] = Fixed::from_int(100);
        Field::new(w, h, baseline)
    }

    #[test]
    fn medium_derived_field_diffusion_is_wired_through_the_world_build_path() {
        // The milestone (design Part 5.4/5.5): two worlds identical but for their ambient medium get
        // DIFFERENT field diffusion coefficients derived from the medium's physics alone, their
        // temperature fields diverge after stepping a hot spot, and the field is bit-identical under
        // the scheduler variant (scheduler == pinned order). The medium is the lever and the
        // diffusivity is physics; the free scalar field.diffusion is retired on this path.
        let manifest = CalibrationManifest::from_toml_str(WIRING_MANIFEST).unwrap();

        // Two scenarios identical but for the medium: one names water, one names none (the documented
        // default temperate air, which resolves to the medium.air physics profile).
        let air_world =
            Scenario::from_toml_str("[scenario]\nid = \"air\"\nname = \"Air\"\n").unwrap();
        let water_world = Scenario::from_toml_str(
            "[scenario]\nid = \"water\"\nname = \"Water\"\nmedium = \"water\"\n",
        )
        .unwrap();
        let air_res = air_world.resolve(&manifest).unwrap();
        let water_res = water_world.resolve(&manifest).unwrap();

        // The air-default world reads the medium.air profile; the water world reads medium.water. No
        // world reads a free diffusion scalar.
        assert_eq!(air_res.medium_manifest_id(), "medium.air");
        assert_eq!(water_res.medium_manifest_id(), "medium.water");

        // The world-build path derives each field calibration from its medium's k/(rho*c).
        let air_calib = FieldCalib::from_resolution(&manifest, &air_res).unwrap();
        let water_calib = FieldCalib::from_resolution(&manifest, &water_res).unwrap();
        assert_ne!(
            air_calib.diffusion, water_calib.diffusion,
            "two media give different diffusion coefficients from the world-build path"
        );
        assert!(
            air_calib.diffusion > water_calib.diffusion,
            "air conducts faster than water from k/(rho*c) alone ({:?} > {:?})",
            air_calib.diffusion,
            water_calib.diffusion
        );
        assert!(
            air_calib.diffusion > Fixed::ZERO
                && water_calib.diffusion > Fixed::ZERO
                && air_calib.diffusion < STENCIL_STABILITY_BOUND
                && water_calib.diffusion < STENCIL_STABILITY_BOUND,
            "both derived coefficients are positive and sub-bound"
        );
        // The relaxation and body-exchange calibrations are the medium-independent manifest reads, so
        // they match: only the diffusion coefficient tracks the medium.
        assert_eq!(air_calib.relaxation, water_calib.relaxation);
        assert_eq!(air_calib.exchange, water_calib.exchange);

        // The two worlds diverge under the hot spot: identical field baselines, medium-derived calibs,
        // so after stepping the temperature-field state hashes must differ (the field is folded into
        // state_hash, so this is the whole-runner canonical hash).
        let mut air_runner = Runner::new(hot_spot_field(), air_calib);
        let mut water_runner = Runner::new(hot_spot_field(), water_calib);
        assert_eq!(
            air_runner.state_hash(),
            water_runner.state_hash(),
            "the two runners start from the same field"
        );
        for _ in 0..8 {
            air_runner.step();
            water_runner.step();
        }
        assert_ne!(
            air_runner.state_hash(),
            water_runner.state_hash(),
            "the medium-derived diffusion diverges the two worlds' temperature fields"
        );

        // The field is bit-identical under the scheduler variant (the field-only runner's version of
        // worker-width invariance: the pinned-order step and the scheduled step must track exactly).
        for calib in [air_calib, water_calib] {
            let mut pinned = Runner::new(hot_spot_field(), calib);
            let mut scheduled = Runner::new(hot_spot_field(), calib);
            for _ in 0..8 {
                pinned.step();
                scheduled.step_scheduled(&[]);
                assert_eq!(
                    scheduled.state_hash(),
                    pinned.state_hash(),
                    "the medium-derived field diverged under the scheduler variant"
                );
            }
        }
    }

    #[test]
    fn per_being_exchange_cools_a_high_surface_body_faster_and_replays_bit_for_bit() {
        use crate::anatomy::{BodyPlan, OrganKindDef, Part, Temperament, TissueComposition};
        use crate::physiology::{
            derive_body_exchange_rate, MetabolicAnchors, CONVECTIVE_SURFACE, TISSUE_SPECIFIC_HEAT,
        };

        // A registry with a skin tissue (convective surface) and a flesh tissue (specific heat).
        let mut organs = crate::anatomy::BodyPlanRegistry::dev_default();
        let skin = organs.organs.len() as u16;
        organs.organs.push(OrganKindDef {
            id: skin,
            name: "skin".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(CONVECTIVE_SURFACE, Fixed::from_int(2))]),
        });
        let flesh = organs.organs.len() as u16;
        organs.organs.push(OrganKindDef {
            id: flesh,
            name: "flesh".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(
                TISSUE_SPECIFIC_HEAT,
                Fixed::from_int(3500),
            )]),
        });
        let temperament = Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(1, 2),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        };
        let make = |skin_dev: (i64, i64)| BodyPlan {
            body_mass: Fixed::from_ratio(1, 2),
            encephalization: Fixed::from_ratio(1, 2),
            diet_breadth: Fixed::from_ratio(1, 2),
            weapons: vec![],
            covering: Part {
                kind: 0,
                development: Fixed::from_ratio(1, 2),
            },
            senses: vec![],
            locomotion: vec![1],
            organs: vec![
                Part {
                    kind: skin,
                    development: Fixed::from_ratio(skin_dev.0, skin_dev.1),
                },
                Part {
                    kind: flesh,
                    development: Fixed::ONE,
                },
            ],
            temperament,
        };
        let anchors = MetabolicAnchors::dev_fixture();
        let high_body = make((1, 1)); // full skin: large surface
        let compact_body = make((1, 8)); // little skin: small surface
        let rate_high =
            derive_body_exchange_rate(&high_body, &organs, anchors.medium_h, Fixed::ONE, &anchors);
        let rate_compact = derive_body_exchange_rate(
            &compact_body,
            &organs,
            anchors.medium_h,
            Fixed::ONE,
            &anchors,
        );
        assert!(
            rate_high > rate_compact,
            "the high-surface body couples faster"
        );

        // Run: a uniform cold field, both beings starting hot in the same cell, each coupled at its own
        // derived rate. The high-surface body cools further toward the cold cell in one step.
        let start = Fixed::from_int(310);
        let cold = Fixed::from_int(250);
        let run = || {
            let field = Field::new(2, 1, vec![cold, cold]);
            let mut r = Runner::new(field, calib());
            let high = StableId(1);
            let compact = StableId(2);
            r.place_being(high, Coord3::ground(0, 0), start);
            r.place_being(compact, Coord3::ground(1, 0), start);
            r.set_body_exchange_rate(high, rate_high);
            r.set_body_exchange_rate(compact, rate_compact);
            r.step();
            (
                r.body_temp(high).unwrap(),
                r.body_temp(compact).unwrap(),
                r.state_hash(),
            )
        };
        let (t_high, t_compact, hash1) = run();
        assert!(
            t_high < start && t_compact < start,
            "both cooled toward the cold cell"
        );
        assert!(
            t_high < t_compact,
            "the high-surface body cooled more: {t_high:?} < {t_compact:?}"
        );
        let (_t2h, _t2c, hash2) = run();
        assert_eq!(hash1, hash2, "the same run replays bit for bit");
    }

    #[test]
    fn field_calib_reads_the_three_values_from_a_set_manifest() {
        let m = CalibrationManifest::from_toml_str(SET).unwrap();
        let c = FieldCalib::from_manifest(&m).unwrap();
        assert_eq!(c.diffusion, Fixed::from_ratio(1, 8));
        assert_eq!(c.relaxation, Fixed::from_ratio(1, 16));
        assert_eq!(c.exchange, Fixed::from_ratio(1, 4));
    }

    #[test]
    fn field_calib_fails_loud_when_a_value_is_reserved() {
        // The shipped manifest holds these reserved (empty), so the loader must refuse rather than
        // fabricate a number (Principle 11). A reserved diffusion entry reproduces that.
        let reserved = r#"
[[reserved]]
id = "field.diffusion"
basis = "fixture"
status = "reserved"
value = ""
unit = "ratio_per_tick"
source = "test"
[[reserved]]
id = "field.relaxation"
basis = "fixture"
status = "set"
value = "0.0625"
unit = "ratio_per_tick"
source = "test"
[[reserved]]
id = "field.body_exchange"
basis = "fixture"
status = "set"
value = "0.25"
unit = "ratio_per_tick"
source = "test"
"#;
        let m = CalibrationManifest::from_toml_str(reserved).unwrap();
        assert_eq!(
            FieldCalib::from_manifest(&m).unwrap_err(),
            CalibrationError::Reserved("field.diffusion".to_string()),
        );
    }

    /// A labelled thermal band fixture (not owner canon): a set point and half-range.
    fn band() -> BeingThermal {
        BeingThermal {
            setpoint: Fixed::from_int(37),
            half_band: Fixed::from_int(8),
            initial_temp: Fixed::from_int(37),
        }
    }

    #[test]
    fn signed_deviation_reads_hot_positive_and_cold_negative() {
        let b = band();
        assert!(
            signed_deviation(b.setpoint + Fixed::from_int(4), &b) > Fixed::ZERO,
            "above the set point reads too hot (positive)"
        );
        assert!(
            signed_deviation(b.setpoint - Fixed::from_int(4), &b) < Fixed::ZERO,
            "below the set point reads too cold (negative)"
        );
        assert_eq!(
            signed_deviation(b.setpoint, &b),
            Fixed::ZERO,
            "at the set point there is no deviation"
        );
    }

    #[test]
    fn signed_deviation_is_odd_in_the_deviation() {
        // The anti-steer: the raw thermoreceptor is exactly odd about the set point, so a body the same
        // distance above and below reads equal and opposite. It favours neither hot nor cold and bakes
        // in no direction (Principle 9), the signed counterpart of comfort_fraction being even.
        let b = band();
        for d in [1i32, 3, 7, 40] {
            let up = signed_deviation(b.setpoint + Fixed::from_int(d), &b);
            let down = signed_deviation(b.setpoint - Fixed::from_int(d), &b);
            assert_eq!(
                up,
                Fixed::ZERO - down,
                "hot and cold are mirror images at d={d}"
            );
        }
    }

    #[test]
    fn signed_deviation_saturates_at_the_band_edges() {
        // At and beyond a full half-range the percept saturates to +/-1, so a lethal-hot medium reads a
        // fully-positive thermoreceptor and a lethal-cold one a fully-negative, the clean bit a
        // controller needs to tell the two dangers apart.
        let b = band();
        assert_eq!(
            signed_deviation(b.setpoint + Fixed::from_int(100), &b),
            Fixed::ONE,
            "far above the band saturates to +1"
        );
        assert_eq!(
            signed_deviation(b.setpoint - Fixed::from_int(100), &b),
            Fixed::from_int(-1),
            "far below the band saturates to -1"
        );
    }

    #[test]
    fn signed_deviation_and_comfort_are_the_odd_and_even_halves() {
        // The two thermoreceptive reads are complementary: comfort is even (magnitude of discomfort),
        // signed deviation is odd (which side). Together they carry both "how far out of band" and "which
        // way", which the even reserve alone cannot, without either one authoring a heading.
        let b = band();
        let hot = b.setpoint + Fixed::from_int(5);
        let cold = b.setpoint - Fixed::from_int(5);
        assert_eq!(
            comfort_fraction(hot, &b),
            comfort_fraction(cold, &b),
            "comfort collapses hot and cold to one magnitude"
        );
        assert_ne!(
            signed_deviation(hot, &b),
            signed_deviation(cold, &b),
            "but the signed thermoreceptor distinguishes them"
        );
    }

    #[test]
    fn a_being_forms_the_harm_belief_through_the_runner_and_the_falsifier_holds() {
        // Harm-learning arc slice d, the run-level acceptance of the FORMATION loop and its falsifier,
        // through the actual Runner tick: a being whose body stands on a salt cell feels its own
        // CONDITION fall, senses the salinity underfoot, and COMMITS the "this feature harms me" belief
        // for itself in its mind, with no injected observation. The falsifier (remove the harm: a fully
        // tolerant being on the identical salt) forms no such belief, so the belief tracks the felt harm,
        // not the mere presence of the substance. This ties the whole learner path end to end through
        // couple_conversation, which the unit tests exercise piecewise.
        use crate::anatomy::{BodyPlan, Part, Temperament};
        use crate::edibility::{Composition, Physiology};
        use crate::evidence::InferenceParams;
        use crate::homeostasis::{HomeostaticAxisDef, HomeostaticRegistry, CONDITION, TEMPERATURE};
        use crate::learn::{feature_subject, HARMS, HARM_ATTR};
        use crate::percept::{feature_bucket, PerceptId, PerceptRegistry};
        use crate::tom::{AccessChannelId, AccessWeights};

        let bp = InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        };
        // A registry with the required non-draining TEMPERATURE axis and the CONDITION reserve the salt
        // harm wears. No draining energy axis, so the being lives on its CONDITION until the salt wears it.
        let reg = HomeostaticRegistry {
            axes: vec![
                HomeostaticAxisDef {
                    id: TEMPERATURE,
                    name: "temperature".to_string(),
                    backing_component: None,
                    capacity_per_mass: Fixed::ONE,
                    base_drain: Fixed::ZERO,
                    exertion_drain: Fixed::ZERO,
                    death_floor: Fixed::ZERO,
                },
                HomeostaticAxisDef {
                    id: CONDITION,
                    // A large condition reserve so the being survives several ticks of salt harm and
                    // accumulates enough correlation evidence to COMMIT the belief before it is worn
                    // through (the formation, not the mortality, is what this test measures; the cull is
                    // proven elsewhere).
                    name: "condition".to_string(),
                    backing_component: None,
                    capacity_per_mass: Fixed::from_int(30),
                    base_drain: Fixed::ZERO,
                    exertion_drain: Fixed::ZERO,
                    death_floor: Fixed::ZERO,
                },
            ],
        };
        let body = || BodyPlan {
            body_mass: Fixed::from_ratio(1, 2),
            encephalization: Fixed::from_ratio(1, 2),
            diet_breadth: Fixed::from_ratio(1, 2),
            weapons: vec![],
            covering: Part {
                kind: 0,
                development: Fixed::from_ratio(1, 2),
            },
            senses: vec![],
            locomotion: vec![1],
            organs: vec![],
            temperament: Temperament {
                boldness: Fixed::from_ratio(1, 2),
                exploration: Fixed::from_ratio(1, 2),
                activity: Fixed::from_ratio(1, 2),
                sociability: Fixed::from_ratio(1, 2),
                aggression: Fixed::from_ratio(1, 4),
            },
        };
        let salt_physiology = |tolerance: Fixed| Physiology {
            requirements: BTreeMap::new(),
            assimilation: BTreeMap::new(),
            tolerances: [(crate::physiology::SALINITY.to_string(), tolerance)]
                .into_iter()
                .collect(),
            hill: [(crate::physiology::SALINITY.to_string(), 2u8)]
                .into_iter()
                .collect(),
        };

        // The salt feature-subject the being should form a belief about: channel 0 (the one declared
        // salinity percept), bucket of the flat's dose under the dev feature granularity. A fully-
        // evaporated salt flat's dose (two), which wears a naive being faster than it heals.
        let dose = Fixed::from_int(2);
        let percepts = PerceptRegistry::dev_salinity();
        assert_eq!(percepts.percepts()[0].id, PerceptId(0));
        let subject = feature_subject(0, feature_bucket(dose, Fixed::ONE));

        // Run the being on the salt for a few ticks (an idle, blank controller, so it stays on the salt),
        // capturing the first HARMS commit before the salt eventually wears it through.
        let run = |tolerance: Fixed| -> Option<crate::evidence::ValueId> {
            let mut world = World::new(
                bp,
                bp,
                AccessWeights::from_pairs([
                    (AccessChannelId(1), Fixed::from_int(4)),
                    (AccessChannelId(3), Fixed::from_int(2)),
                ]),
            );
            let id = world.spawn(Fixed::ONE);
            world.set_place(id, 0);

            let mut emb = Embodiment::new(
                reg.clone(),
                AffordanceRegistry::dev_default(),
                LocomotionParams::dev_default(),
                0,
                0x5A17,
            );
            emb.set_percepts(percepts.clone());
            let blank = Controller::zeros(emb.layout());
            let tile = Coord3::ground(4, 4);
            emb.add(
                Walker::new(
                    id,
                    tile,
                    body(),
                    Homeostasis::from_mass(&reg, Fixed::ONE),
                    salt_physiology(tolerance),
                    blank,
                ),
                band(),
            );
            // The salt cell the being stands on (a bio.salinity toxin dose), and a benign neighbourhood.
            let mut salt_toxins = BTreeMap::new();
            salt_toxins.insert(crate::physiology::SALINITY.to_string(), dose);
            emb.resources_mut().set(
                tile,
                Composition {
                    nutrients: BTreeMap::new(),
                    toxins: salt_toxins,
                },
            );

            let field = Field::new(8, 8, vec![Fixed::from_int(37); 64]);
            let mut runner = Runner::with_world_and_embodiment(field, calib(), world, emb);
            let mut committed = None;
            for _ in 0..10 {
                runner.step();
                match runner.world().and_then(|w| w.mind(id)) {
                    Some(m) => {
                        if let Some(v) = m.belief(subject, HARM_ATTR, &bp) {
                            committed = Some(v);
                        }
                    }
                    None => break, // the being died; keep the last committed value
                }
            }
            committed
        };

        // The naive being (low salt tolerance) is worn by the salt, feels it, and forms the HARMS belief
        // for itself through the runner, with no injected observation.
        assert_eq!(
            run(Fixed::from_ratio(1, 5)),
            Some(HARMS),
            "a naive being on the salt forms the HARMS belief for itself through the runner"
        );
        // The falsifier: a fully tolerant being takes no harm from the identical salt, so it forms no
        // HARMS belief. The belief tracks the felt harm, not the substance's presence.
        assert_ne!(
            run(Fixed::from_int(5)),
            Some(HARMS),
            "remove the harm (full salt tolerance) and no HARMS belief forms: the belief tracks harm"
        );
    }

    #[test]
    fn a_holder_that_avoids_the_hazard_outlives_a_naive_being_on_the_same_harm() {
        // Harm-learning arc slice d, the ADAPTIVE leg: the belief pays off in survival. Two beings sit on
        // the western edge of a salt region (salt to their east). Both hold the HARMS belief about the
        // salt feature; the one whose controller has the evolved CONDITION-dir-to-heading weight steers
        // WEST off the salt (its avoidance percept points away from the believed harm to the east) and
        // keeps its condition, while the one with the founding-zero weight cannot act on the belief, stays
        // on the salt, and is worn. So the belief is adaptive only through the evolved weight, and
        // avoidance is what makes it worth carrying (Principle 9). No authored flight: both hold the same
        // belief; only the evolved weight differs.
        use crate::anatomy::{BodyPlan, Part, Temperament};
        use crate::controller::{forage_taxis_weights, ForageGains};
        use crate::edibility::{Composition, Physiology};
        use crate::evidence::InferenceParams;
        use crate::homeostasis::{HomeostaticAxisDef, HomeostaticRegistry, CONDITION, TEMPERATURE};
        use crate::learn::{feature_subject, HARMS, HARM_ATTR};
        use crate::percept::{feature_bucket, PerceptRegistry};
        use crate::tom::{AccessChannelId, AccessWeights};

        let bp = InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        };
        let reg = HomeostaticRegistry {
            axes: vec![
                HomeostaticAxisDef {
                    id: TEMPERATURE,
                    name: "temperature".to_string(),
                    backing_component: None,
                    capacity_per_mass: Fixed::ONE,
                    base_drain: Fixed::ZERO,
                    exertion_drain: Fixed::ZERO,
                    death_floor: Fixed::ZERO,
                },
                HomeostaticAxisDef {
                    id: CONDITION,
                    name: "condition".to_string(),
                    backing_component: None,
                    capacity_per_mass: Fixed::from_int(30),
                    base_drain: Fixed::ZERO,
                    exertion_drain: Fixed::ZERO,
                    death_floor: Fixed::ZERO,
                },
            ],
        };
        let body = || BodyPlan {
            body_mass: Fixed::from_ratio(1, 2),
            encephalization: Fixed::from_ratio(1, 2),
            diet_breadth: Fixed::from_ratio(1, 2),
            weapons: vec![],
            covering: Part {
                kind: 0,
                development: Fixed::from_ratio(1, 2),
            },
            senses: vec![],
            locomotion: vec![1],
            organs: vec![],
            temperament: Temperament {
                boldness: Fixed::from_ratio(1, 2),
                exploration: Fixed::from_ratio(1, 2),
                activity: Fixed::from_ratio(1, 2),
                sociability: Fixed::from_ratio(1, 2),
                aggression: Fixed::from_ratio(1, 4),
            },
        };
        let naive_physiology = || Physiology {
            requirements: BTreeMap::new(),
            assimilation: BTreeMap::new(),
            tolerances: [(
                crate::physiology::SALINITY.to_string(),
                Fixed::from_ratio(1, 5),
            )]
            .into_iter()
            .collect(),
            hill: [(crate::physiology::SALINITY.to_string(), 2u8)]
                .into_iter()
                .collect(),
        };

        let dose = Fixed::from_int(2);
        let percepts = PerceptRegistry::dev_salinity();
        let subject = feature_subject(0, feature_bucket(dose, Fixed::ONE));

        let mut world = World::new(
            bp,
            bp,
            AccessWeights::from_pairs([
                (AccessChannelId(1), Fixed::from_int(4)),
                (AccessChannelId(3), Fixed::from_int(2)),
            ]),
        );
        let avoider = world.spawn(Fixed::ONE);
        let stayer = world.spawn(Fixed::ONE);
        world.set_place(avoider, 0);
        world.set_place(stayer, 1);

        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x5A17,
        );
        emb.set_percepts(percepts.clone());
        let layout = emb.layout().clone();
        // The avoider's controller: it wants to move and steers its MOVE heading along the CONDITION
        // avoidance gradient (CONDITION as a steer axis), so it acts on the belief. MOVE is output 0
        // (act, dx, dy), INGEST the scalar output at 3.
        let cond_base = layout.axis_input_base(CONDITION).unwrap();
        let gains = ForageGains {
            move_bias: Fixed::ONE,
            here_suppress: Fixed::ZERO,
            heading_gain: Fixed::ONE,
            ingest_drive: Fixed::ZERO,
        };
        let mut avoider_w = vec![Fixed::ZERO; layout.weight_count()];
        for (pid, v) in forage_taxis_weights(&layout, 0, 3, &[], &[cond_base], gains) {
            avoider_w[pid.0 as usize] = v;
        }
        let avoider_ctrl =
            Controller::from_weights(layout.n_in(), layout.n_out(), layout.hidden(), avoider_w);
        let blank = Controller::zeros(&layout);

        // Both start on the western EDGE of a salt region: salt fills every cell with x >= 5, the safe
        // ground is x < 5, and each being stands at x = 5 (on the salt, salt to its east, safe to its
        // west), one row apart so they do not share a cell.
        let av_start = Coord3::ground(5, 3);
        let st_start = Coord3::ground(5, 6);
        emb.add(
            Walker::new(
                avoider,
                av_start,
                body(),
                Homeostasis::from_mass(&reg, Fixed::ONE),
                naive_physiology(),
                avoider_ctrl,
            ),
            band(),
        );
        emb.add(
            Walker::new(
                stayer,
                st_start,
                body(),
                Homeostasis::from_mass(&reg, Fixed::ONE),
                naive_physiology(),
                blank,
            ),
            band(),
        );
        for y in 0..8 {
            for x in 5..8 {
                let mut toxins = BTreeMap::new();
                toxins.insert(crate::physiology::SALINITY.to_string(), dose);
                emb.resources_mut().set(
                    Coord3::ground(x, y),
                    Composition {
                        nutrients: BTreeMap::new(),
                        toxins,
                    },
                );
            }
        }

        let field = Field::new(8, 8, vec![Fixed::from_int(37); 64]);
        let mut runner = Runner::with_world_and_embodiment(field, calib(), world, emb);
        // Both beings already hold the HARMS belief about the salt (they have learned it): seed each once
        // so the leg under test is avoidance-and-survival, not formation (formation is the previous test).
        let seed = |id: StableId| TickInput {
            mind: id,
            ordinal: 0,
            stim: Stimulus::Observe {
                subject,
                attr: HARM_ATTR,
                hyps: vec![HARMS, 0],
                toward: HARMS,
                weight: Fixed::from_int(50),
                from: id,
            },
        };
        runner.step_with_world_inputs(&[seed(avoider), seed(stayer)]);
        for _ in 0..10 {
            runner.step();
        }

        let level = |r: &Runner, id: StableId| -> Fixed {
            r.embodiment()
                .unwrap()
                .walkers()
                .iter()
                .find(|w| w.id == id)
                .map(|w| w.homeostasis.level(CONDITION))
                .unwrap_or(Fixed::ZERO)
        };
        let x_of = |r: &Runner, id: StableId| -> i32 {
            r.embodiment()
                .unwrap()
                .walkers()
                .iter()
                .find(|w| w.id == id)
                .map(|w| w.coord().x)
                .unwrap_or(0)
        };
        // Both hold the same belief, but only the avoider can act on it: it steered west off the salt and
        // kept its condition, while the stayer could not and was worn.
        assert!(
            x_of(&runner, avoider) < 5,
            "the avoider steered west off the salt (x = {})",
            x_of(&runner, avoider)
        );
        assert_eq!(
            x_of(&runner, stayer),
            5,
            "the stayer, unable to act on the belief, stayed on the salt"
        );
        assert!(
            level(&runner, avoider) > level(&runner, stayer),
            "the avoider outlived the stayer on the same harm: condition {:?} > {:?}",
            level(&runner, avoider),
            level(&runner, stayer)
        );
    }

    #[test]
    fn a_learned_feature_harm_belief_rides_the_shipped_gossip_to_a_co_located_naive_being() {
        // Harm-learning arc slice d, the TRANSMISSION leg: because a learned "this feature harms me"
        // belief is an ordinary (subject, attr) frame, it rides the shipped overhearing transmission for
        // free. A holder co-located with a naive being conveys the belief through gossip, so the idea
        // spreads by presence (not by an authored teaching path, not by reading kinship), and it persists
        // only while a holder is present, which is what makes the loop's persistence possible.
        use crate::evidence::InferenceParams;
        use crate::learn::{feature_subject, HARMS, HARM_ATTR};
        use crate::percept::feature_bucket;
        use crate::tom::{AccessChannelId, AccessWeights};
        use crate::world::GossipParams;

        let bp = InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        };
        let mut world = World::new(
            bp,
            bp,
            AccessWeights::from_pairs([
                (AccessChannelId(1), Fixed::from_int(4)),
                (AccessChannelId(3), Fixed::from_int(2)),
            ]),
        );
        world.set_gossip(GossipParams {
            told_weight: Fixed::from_int(3),
            trust_baseline: Fixed::ONE,
            trust_penalty: Fixed::from_ratio(1, 2),
        });
        let holder = world.spawn(Fixed::ONE);
        let naive = world.spawn(Fixed::ONE);
        // Co-located: they share one conversational place, so a speaker's committed beliefs reach the
        // other (the overhearing follow-on).
        world.set_place(holder, 7);
        world.set_place(naive, 7);

        let subject = feature_subject(0, feature_bucket(Fixed::from_int(2), Fixed::ONE));
        // Before any tick, neither being holds the belief.
        assert_eq!(
            world.mind(naive).unwrap().belief(subject, HARM_ATTR, &bp),
            None,
            "the naive being starts with no belief"
        );
        // The holder has LEARNED the salt harms it (a committed feature-harm belief); the naive has not.
        let seed = TickInput {
            mind: holder,
            ordinal: 0,
            stim: Stimulus::Observe {
                subject,
                attr: HARM_ATTR,
                hyps: vec![HARMS, 0],
                toward: HARMS,
                weight: Fixed::from_int(50),
                from: holder,
            },
        };
        // One tick: the holder commits the learned belief, then the shipped overhearing transmission
        // carries it to the co-located naive being in the same tick's gossip beat.
        world.tick(&[seed]);
        assert_eq!(
            world.mind(holder).unwrap().belief(subject, HARM_ATTR, &bp),
            Some(HARMS),
            "the holder holds the learned feature-harm belief"
        );
        assert_eq!(
            world.mind(naive).unwrap().belief(subject, HARM_ATTR, &bp),
            Some(HARMS),
            "the learned belief rode the shipped gossip to the co-located naive being by presence"
        );
    }
}
