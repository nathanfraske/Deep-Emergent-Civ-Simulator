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
//! policy. Each tick, after the field step and the body-thermal exchange, a pure comfort-band map
//! ([`comfort_fraction`]) turns each being's absolute core temperature into a temperature homeostatic
//! reserve in `[0, 1]`, per being from its own reserved comfort band, and the being's controller reads
//! that reserve and issues a movement affordance; the beings' new coordinates re-sync the located index
//! so the next tick's thermal exchange reads where they moved. This closes the loop from physics to
//! physiology to behaviour to physics with no authored heading. In this first increment the controller
//! reads temperature only as a scalar comfort, with no directional thermal percept, so an uncomfortable
//! being explores (an undirected, seed-keyed heading) and a comfortable one rests, and directed
//! thermotaxis is an emergent consequence of moving-while-uncomfortable under survival rather than a
//! wired rule. The comfort band's set point and half-range are reserved per-race physiology (Part 20),
//! the beings' controllers are expressed and (in the full engine) selected by survival, and the
//! composite hash folds each being's position, reserves, and controller state after the world fold.
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

use crate::anatomy::BodyPlan;
use crate::controller::ControllerLayout;
use crate::homeostasis::{AffordanceRegistry, HomeostaticRegistry, TEMPERATURE};
use crate::located::{LocationIndex, OccupantId};
use crate::locomotion::{self, LocomotionParams, ResourceField, Terrain, Walker};
use crate::world::World;
use civsim_core::{Fixed, StableId, StateHasher};
use civsim_world::{Coord3, TileMap};
use std::collections::BTreeMap;

/// The reserved field-layer calibrations. There is deliberately no `Default`: on a canonical run
/// these are read from the manifest and are fail-loud if unset, and a test must name each as a
/// labelled fixture. None is an agent-set number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldCalib {
    /// The per-tick diffusion (conduction) coefficient, dimensionless, in `[0, 0.25)` for the
    /// four-neighbour stencil's stability bound. Basis: the medium's thermal diffusivity over the
    /// cell size and the base tick, kept below the explicit stability limit.
    pub diffusion: Fixed,
    /// The per-tick relaxation rate of a cell toward its baseline (the solar and biome forcing), in
    /// `[0, 1]`. Basis: the day-night and seasonal forcing timescale over the base tick.
    pub relaxation: Fixed,
    /// The per-tick body-to-environment convective coupling, in `[0, 1]`. Basis: the fluids-floor
    /// convective coefficient and the body surface-to-thermal-mass ratio (`law.convective_flux`),
    /// expressed as the discrete Newton-cooling rate.
    pub exchange: Fixed,
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
    layout: ControllerLayout,
    params: LocomotionParams,
    resources: ResourceField,
    seed: u64,
}

impl Embodiment {
    /// A new, empty embodiment over a temperature-bearing physiology registry, an affordance registry,
    /// the movement parameters, a controller hidden width (zero for a reaction norm), and a locomotion
    /// seed. The controller layout is derived from the two registries, so a caller builds or expresses
    /// its beings' controllers against [`Embodiment::layout`]. The resource field starts empty: this
    /// first increment gives the beings no directional thermal percept, so thermotaxis is emergent
    /// rather than sensed.
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
            layout,
            params,
            resources: ResourceField::new(),
            seed,
        }
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
            world: None,
            embodiment: None,
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
            world: Some(world),
            embodiment: None,
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
        for w in &embodiment.walkers {
            let init = embodiment
                .thermal
                .get(&w.id)
                .map(|b| b.initial_temp)
                .unwrap_or(Fixed::ZERO);
            body_temp.insert(w.id, init);
            index.place(OccupantId::being(w.id), w.coord());
        }
        Runner {
            clock: 0,
            field,
            calib,
            index,
            body_temp,
            world: None,
            embodiment: Some(embodiment),
        }
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

    /// The coupled embodied-being population, if any (a pure read, for tests and rendering).
    pub fn embodiment(&self) -> Option<&Embodiment> {
        self.embodiment.as_ref()
    }

    /// One canonical tick, in a pinned within-tick order: step the field, let each located being
    /// exchange heat with its cell (Newton convective coupling toward the local field temperature,
    /// beings walked in canonical id order), run the embodiment coupling sub-phase (comfort-band map to
    /// evolved-controller locomotion to index re-sync), then tick the composed cognition world as the
    /// fixed final sub-phase. The field phases run first so the embodiment coupling reads the same-tick
    /// thermal state; the cognition world runs last and shares no data across its seam yet.
    pub fn step(&mut self) {
        self.field.step(&self.calib);
        let ids: Vec<StableId> = self.body_temp.keys().copied().collect();
        for id in ids {
            if let Some(coord) = self.index.coord_of(OccupantId::being(id)) {
                let env = self.field.at(coord.x, coord.y);
                let bt = self.body_temp[&id];
                let next = bt + self.calib.exchange.mul(env - bt);
                self.body_temp.insert(id, next);
            }
        }
        if self.embodiment.is_some() {
            self.step_embodiment();
        }
        if let Some(world) = self.world.as_mut() {
            world.tick(&[]);
        }
        self.clock += 1;
    }

    /// The embodiment coupling sub-phase: map each being's field-driven core temperature to its
    /// temperature reserve through the comfort-band map (physics to physiology), let its evolved
    /// controller drive one step of locomotion (physiology to behaviour), and re-sync the located index
    /// to the beings' new coordinates (behaviour to physics). The comfort-band map is pure, and the
    /// locomotion draws its exploration heading from the being-and-tick-keyed RNG, so the sub-phase
    /// authors no heading and reproduces bit for bit. Distinct fields of the runner are borrowed
    /// disjointly, so the field, the body-temperature map, the located index, and the embodiment are
    /// touched without contention.
    fn step_embodiment(&mut self) {
        let (width, height) = self.field.dims();
        let terrain = BoundedPlane { width, height };
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
        // (2) Physiology to behaviour: the evolved controllers drive one locomotion step over the being
        // slice (move-or-rest, and when moving with no directional percept an undirected explore).
        locomotion::step(
            &mut emb.walkers,
            &emb.homeo,
            &emb.layout,
            &emb.afford,
            &terrain,
            &emb.resources,
            &emb.params,
            emb.seed,
            self.clock,
        );
        // (3) Behaviour to physics: the beings' new coordinates re-sync the located index, so next
        // tick's thermal exchange reads where they moved.
        for w in emb.walkers.iter() {
            self.index.place(OccupantId::being(w.id), w.coord());
        }
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
            }
        }
        h.finish()
    }
}
