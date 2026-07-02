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
//! world's canonical hash in a pinned order. Honest limits held here: there is no field-to-cognition
//! or cognition-to-field data flow yet (temperature is not a percept, and no chosen action moves a
//! coordinate), and the world hash does not yet fold the being lifecycle (genomes, ages, affect) or
//! the dialogue log, so the composite is not a complete canonical hash of all being state until those
//! later increments land. The field layer is one field (temperature) so far, the pattern the moisture,
//! wind, and resource fields follow.

use crate::located::{LocationIndex, OccupantId};
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
}

impl Runner {
    /// A field-only runner over a field with the given reserved calibrations (no cognition world).
    pub fn new(field: Field, calib: FieldCalib) -> Runner {
        Runner {
            clock: 0,
            field,
            calib,
            index: LocationIndex::new(),
            body_temp: BTreeMap::new(),
            world: None,
        }
    }

    /// A composed runner that owns a cognition [`World`] and ticks it as a fixed sub-phase after the
    /// field phases. The caller constructs and calibrates the world (fail-loud on any unset reserved
    /// value, per the world's own manifest discipline); this runner adds no authored number, no new
    /// RNG draw, and no new phase, so the composite reproduces bit for bit exactly as each side already
    /// does.
    pub fn with_world(field: Field, calib: FieldCalib, world: World) -> Runner {
        Runner {
            clock: 0,
            field,
            calib,
            index: LocationIndex::new(),
            body_temp: BTreeMap::new(),
            world: Some(world),
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

    /// One canonical tick, in a pinned within-tick order: step the field, let each located being
    /// exchange heat with its cell (Newton convective coupling toward the local field temperature,
    /// beings walked in canonical id order), then tick the composed cognition world as the fixed next
    /// sub-phase. The field phases run first so that a later field-to-cognition coupling reads the
    /// same-tick thermal state; no input batch crosses the seam yet.
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
        if let Some(world) = self.world.as_mut() {
            world.tick(&[]);
        }
        self.clock += 1;
    }

    /// The canonical state hash: the clock, the field, and every located being's temperature in id
    /// order, then (for a composed runner) the world's canonical hash folded in a pinned position. A
    /// run reproduces this bit for bit; it is independent of thread count and camera. The world hash
    /// is left byte-identical to [`crate::world::World::state_hash`]; a field-only runner omits it.
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
        h.finish()
    }
}
