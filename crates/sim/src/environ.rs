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

//! The environmental field stack (base-level liveliness step 2; design Part 15, Part 18; Principles 3,
//! 8, 9, 11). A data-driven stack of pinned-stencil scalar fields on the shared map grid, stepped in
//! canonical field order after the temperature field and folded into the runner's `state_hash`.
//!
//! HYDROLOGY: a per-cell water-depth field, sourced by precipitation (moisture crossing the saturation
//! vapour pressure, so cold humid cells rain), sunk by evaporation (the Dalton bulk flux, so hot cells
//! dry), and transported downhill over the frozen worldgen elevation (a downhill-flux advection, so
//! water pools in basins). PRIMARY PRODUCTIVITY: a per-cell standing PRODUCER BIOMASS, the food supply
//! a grazer eats, derived as the Liebig minimum ([`civsim_physics::laws::net_nutrition`]) over water,
//! light, temperature, and soil, then written into the [`crate::locomotion::ResourceField`] as the
//! `bio.energy_density` supply. It is NOT a dead-zone cutoff: the limiting factor sets the continuous
//! productivity through the Liebig product, never an `if water < X { barren }` gate (Principle 8).
//!
//! BIOSPHERE-READY (owner directive): the productivity is the DEFAULT abstract source of the per-cell
//! producer biomass; the standing biomass is a value a source WRITES INTO, so the living-biosphere
//! addendum arc replaces the abstract Liebig source with real producer occupants' biomass with no
//! rewrite (`biomass_from` is the seam). Base-level liveliness step 3 makes the biomass a standing
//! `Stock` that regrows toward this productivity and depletes when grazed; here it sits at the
//! productivity (the abstract producer at its carrying capacity).
//!
//! Every stencil is a pinned integer fold in canonical row-major order, the same shape as
//! [`crate::runner::Field::step`] and the GPU field kernel, so it is bit-identical across threads and
//! machine and ports unchanged to a CubeCL `#[cube]` kernel (Principle 3). The downhill routing's
//! target is a deterministic lowest-neighbour choice, ties broken in a fixed neighbour order, so the
//! advection carries no thread-schedule dependence. Every forcing constant is reserved fail-loud with
//! basis (Principle 11); the biome-to-water and productivity rules key off the physical fields
//! (elevation, moisture, temperature, latitude), never a biome or race label (Principles 8, 9).

use civsim_core::{Fixed, StateHasher};
use civsim_physics::laws;
use civsim_world::{Coord3, TileMap};

use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::edibility::Composition;
use crate::locomotion::ResourceField;
use crate::physiology::{ENERGY_DENSITY, WATER_FRACTION};
use crate::runner::Field;
use crate::stocks::Stock;

/// A scalar field on the flat bounded map, Q32.32, row-major (`idx = y * width + x`), the shape the
/// temperature [`Field`] and the GPU field kernel use. The membership of the environmental stack is
/// data (which fields exist); each field is this fixed representation (Principle 11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalarField {
    width: i32,
    height: i32,
    cells: Vec<Fixed>,
}

impl ScalarField {
    /// A field of `width * height` cells at a uniform value.
    pub fn uniform(width: i32, height: i32, value: Fixed) -> ScalarField {
        assert!(width > 0 && height > 0, "a field has positive extent");
        ScalarField {
            width,
            height,
            cells: vec![value; (width as usize) * (height as usize)],
        }
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    /// The value at a cell (row-major, in bounds by construction of the caller).
    pub fn at(&self, x: i32, y: i32) -> Fixed {
        self.cells[self.idx(x, y)]
    }

    /// The field extent.
    pub fn dims(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// Fold the field into a hash in canonical row-major order (its contribution to `state_hash`).
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_u32(self.width.max(0) as u32);
        h.write_u32(self.height.max(0) as u32);
        for &c in &self.cells {
            h.write_fixed(c);
        }
    }
}

/// The reserved forcing constants the environmental stack reads, fail-loud from the manifest under a
/// Calibrated run or a labelled dev fixture in a test. Every value is reserved-with-basis (Principle
/// 11): the saturation-curve affine tangent, the precipitation and evaporation coefficients, the
/// downhill routing fraction, and the productivity per-factor requirements and soil baseline.
#[derive(Clone, Copy, Debug)]
pub struct EnvironCalib {
    // Saturation vapour pressure e_s = e_ref + slope * (T - T_ref), the affine Clausius-Clapeyron
    // tangent (laws::saturation_vapor_pressure).
    pub sat_slope: Fixed,
    pub sat_t_ref: Fixed,
    pub sat_e_ref: Fixed,
    pub sat_es_cap: Fixed,
    /// The precipitation rate: the fraction of the moisture-over-saturation excess that condenses to
    /// water depth per tick.
    pub precip_rate: Fixed,
    // Evaporation E = (a + b*|u|)*(e_s - e_a) (laws::evaporation_rate), wind zero this step.
    pub evap_a_still: Fixed,
    pub evap_b_wind: Fixed,
    pub evap_max: Fixed,
    /// The downhill routing fraction: the fraction of a cell's water that flows to its lowest
    /// neighbour per tick (a cell with no lower neighbour, a basin, retains its water).
    pub routing_rate: Fixed,
    /// The basin holding capacity: the maximum standing water a cell retains, so a basin fills to a
    /// lake and does not grow without bound (evaporation does not yet scale with depth, the noted
    /// limit). The excess over this cap is shed, the coarse stand-in for a full basin overflowing.
    pub max_water_depth: Fixed,
    // The productivity per-factor requirements (the Liebig satisfaction denominators): a cell meets a
    // factor when its supply reaches the requirement.
    pub water_req: Fixed,
    pub light_req: Fixed,
    pub temp_req: Fixed,
    pub soil_req: Fixed,
    /// The uniform soil-nutrient supply (a baseline until the step-4 soil field lands).
    pub soil_baseline: Fixed,
    /// The producer-biomass regrowth rate (base-level liveliness step 3): the logistic regeneration
    /// coefficient the standing food stock regrows toward the productivity capacity at each tick
    /// ([`crate::stocks::Stock`]). Larger regrows a grazed patch faster and raises the carrying
    /// capacity; smaller makes food scarcer.
    pub regen_rate: Fixed,
    /// The colonization propagule floor (base-level liveliness step 3): the small standing biomass a
    /// viable-but-empty cell establishes so logistic regrowth can bootstrap from nothing (a dry-start
    /// world greens as water arrives, and a grazed-out patch slowly recovers rather than dying forever).
    /// Small relative to a cell's capacity, the seed-rain a viable cell receives.
    pub colonization: Fixed,
}

impl EnvironCalib {
    /// The environmental calibration read fail-loud from the manifest (Principle 11): every forcing
    /// constant is a reserved value that refuses to build while unset.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<EnvironCalib, CalibrationError> {
        Ok(EnvironCalib {
            sat_slope: m.require_fixed("hydrology.saturation_slope")?,
            sat_t_ref: m.require_fixed("hydrology.saturation_t_ref")?,
            sat_e_ref: m.require_fixed("hydrology.saturation_e_ref")?,
            sat_es_cap: m.require_fixed("hydrology.saturation_cap")?,
            precip_rate: m.require_fixed("hydrology.precipitation_rate")?,
            evap_a_still: m.require_fixed("hydrology.evaporation_still")?,
            evap_b_wind: m.require_fixed("hydrology.evaporation_wind")?,
            evap_max: m.require_fixed("hydrology.evaporation_cap")?,
            routing_rate: m.require_fixed("hydrology.routing_rate")?,
            max_water_depth: m.require_fixed("hydrology.max_water_depth")?,
            water_req: m.require_fixed("productivity.water_requirement")?,
            light_req: m.require_fixed("productivity.light_requirement")?,
            temp_req: m.require_fixed("productivity.temperature_requirement")?,
            soil_req: m.require_fixed("productivity.soil_requirement")?,
            soil_baseline: m.require_fixed("productivity.soil_baseline")?,
            regen_rate: m.require_fixed("productivity.regen_rate")?,
            colonization: m.require_fixed("productivity.colonization")?,
        })
    }

    /// A labelled DEVELOPMENT FIXTURE calibration (not owner canon), for tests and the run harness. The
    /// values exercise the mechanism: cold humid cells rain, hot cells evaporate, water routes downhill,
    /// and productivity tracks the water-light-temperature-soil Liebig product.
    pub fn dev_fixture() -> EnvironCalib {
        EnvironCalib {
            sat_slope: Fixed::from_ratio(1, 2),
            sat_t_ref: Fixed::from_ratio(1, 2),
            sat_e_ref: Fixed::from_ratio(1, 2),
            sat_es_cap: Fixed::from_int(2),
            precip_rate: Fixed::from_ratio(1, 5),
            evap_a_still: Fixed::from_ratio(1, 10),
            evap_b_wind: Fixed::ZERO,
            evap_max: Fixed::from_int(1),
            routing_rate: Fixed::from_ratio(1, 4),
            max_water_depth: Fixed::from_int(4),
            water_req: Fixed::from_ratio(1, 2),
            light_req: Fixed::from_ratio(1, 2),
            temp_req: Fixed::from_ratio(1, 2),
            soil_req: Fixed::from_ratio(1, 2),
            soil_baseline: Fixed::from_int(1),
            regen_rate: Fixed::from_ratio(1, 4),
            colonization: Fixed::from_ratio(1, 20),
        }
    }
}

/// The environmental field stack over one map (base-level liveliness step 2): the dynamic water-depth
/// field and the derived standing producer biomass, plus the static per-cell inputs (elevation,
/// moisture, latitude light) folded from the worldgen tiles and the precomputed downhill target. The
/// temperature comes from the runner's diffused [`Field`] each step, so the stack reads the same-tick
/// thermal state.
#[derive(Clone, Debug)]
pub struct EnvironFields {
    width: i32,
    height: i32,
    /// The dynamic water depth per cell (the hydrology field).
    water: ScalarField,
    /// The producer-biomass CAPACITY per cell: the primary productivity (the Liebig ceiling the standing
    /// food stock regrows toward), derived each tick from the water-light-temperature-soil product. The
    /// standing stock that feeds a grazer lives in the [`ResourceField`], persisting and
    /// depleting there; this field is the moving target it regrows toward (base-level liveliness step 3).
    /// Biosphere-ready: the addendum replaces this abstract Liebig capacity with real producer biomass.
    capacity: ScalarField,
    /// Static per-cell worldgen inputs (row-major): the moisture the precipitation reads and the
    /// latitude light the productivity reads. The frozen elevation feeds the precomputed `downhill`
    /// target and is not stored past construction.
    moisture: Vec<Fixed>,
    light: Vec<Fixed>,
    /// The precomputed downhill target index of each cell: the lowest-elevation of its four neighbours
    /// (ties broken in the fixed order up, down, left, right), or the cell itself when no neighbour is
    /// strictly lower (a basin, which retains its water). Static, since elevation is frozen.
    downhill: Vec<usize>,
}

impl EnvironFields {
    /// Build the stack from a generated map: seed the water field empty, fold the static elevation,
    /// moisture, and latitude-light per cell, and precompute the downhill routing target. A pure
    /// deterministic fold over the worldgen tiles (Principle 3), keyed off the physical fields, never a
    /// biome label (Principles 8, 9).
    pub fn from_map(map: &TileMap) -> EnvironFields {
        let topo = map.topo();
        let (w, h) = (topo.width, topo.height);
        let n = (w.max(0) as usize) * (h.max(0) as usize);
        let mut elevation = Vec::with_capacity(n);
        let mut moisture = Vec::with_capacity(n);
        let mut light = Vec::with_capacity(n);
        for y in 0..h {
            for x in 0..w {
                let tile = map
                    .tile(Coord3::new(x, y, 0))
                    .expect("every in-bounds cell has a tile");
                elevation.push(tile.elevation);
                moisture.push(tile.moisture);
                light.push(latitude_light(y, h));
            }
        }
        let downhill = compute_downhill(&elevation, w, h);
        EnvironFields {
            width: w,
            height: h,
            water: ScalarField::uniform(w, h, Fixed::ZERO),
            capacity: ScalarField::uniform(w, h, Fixed::ZERO),
            moisture,
            light,
            downhill,
        }
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    /// The water depth at a cell (a per-cell field read, for the reader and the resource loop).
    pub fn water_at(&self, x: i32, y: i32) -> Fixed {
        self.water.at(x, y)
    }

    /// The producer-biomass CAPACITY (the primary productivity, the ceiling the standing food stock
    /// regrows toward) at a cell. The standing supply that feeds a grazer lives in the
    /// [`ResourceField`] this writes into; this is the productivity potential, for the field reader.
    pub fn capacity_at(&self, x: i32, y: i32) -> Fixed {
        self.capacity.at(x, y)
    }

    /// The field extent.
    pub fn dims(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// One canonical step of the environmental stack (base-level liveliness step 2): advance the
    /// hydrology (precipitation, evaporation, downhill routing) against the same-tick diffused
    /// temperature, then derive the primary-productivity CAPACITY (the ceiling the standing food stock
    /// regrows toward; base-level liveliness step 3 moved the standing stock into the [`ResourceField`]
    /// and this field to the capacity). Pinned integer folds in canonical row-major order, double-
    /// buffered, so the step is bit-identical across threads and replays (Principle 3). The temperature
    /// is the runner's diffused [`Field`], sized to the same grid. The standing stock itself is regrown
    /// and grazed through [`Self::regrow_supply`] against this capacity, not here.
    pub fn step(&mut self, temp: &Field, calib: &EnvironCalib) {
        self.step_hydrology(temp, calib);
        self.step_productivity(temp, calib);
    }

    /// The hydrology stencil: for each cell compute its sourced water (old + precipitation - evaporation,
    /// clamped non-negative), then route a fraction downhill to the precomputed lowest neighbour,
    /// double-buffered so the advection is order-independent and conservative (a cell keeps what it does
    /// not send; a basin sends nothing).
    fn step_hydrology(&mut self, temp: &Field, calib: &EnvironCalib) {
        let (w, h) = (self.width, self.height);
        let n = (w as usize) * (h as usize);
        // (1) Precipitation and evaporation, pointwise into a sourced buffer.
        let mut sourced = vec![Fixed::ZERO; n];
        for y in 0..h {
            for x in 0..w {
                let i = self.idx(x, y);
                let t = temp.at(x, y);
                let e_s = laws::saturation_vapor_pressure(
                    t,
                    calib.sat_slope,
                    calib.sat_t_ref,
                    calib.sat_e_ref,
                    calib.sat_es_cap,
                );
                let moist = self.moisture[i];
                // Precipitation: the moisture over saturation condenses (cold cells, low e_s).
                let excess = moist - e_s;
                let precip = if excess > Fixed::ZERO {
                    calib.precip_rate.mul(excess)
                } else {
                    Fixed::ZERO
                };
                // Evaporation: the Dalton flux over the vapour-pressure deficit (hot cells, high e_s).
                let evap = laws::evaporation_rate(
                    moist,
                    e_s,
                    Fixed::ZERO,
                    calib.evap_a_still,
                    calib.evap_b_wind,
                    calib.evap_max,
                );
                let after = (self.water.cells[i].saturating_add(precip) - evap).max(Fixed::ZERO);
                sourced[i] = after;
            }
        }
        // (2) Downhill routing (advection), double-buffered: each cell keeps its retained water and
        // receives the outflow of every higher neighbour that routes to it.
        let mut next = vec![Fixed::ZERO; n];
        for i in 0..n {
            let out = if self.downhill[i] != i {
                calib.routing_rate.mul(sourced[i])
            } else {
                Fixed::ZERO
            };
            // The retained water stays; the outflow moves to the downhill target. Addition is exact and
            // order-independent (a Fixed sum), so the scatter is deterministic.
            next[i] = next[i].saturating_add(sourced[i] - out);
            if out > Fixed::ZERO {
                let j = self.downhill[i];
                next[j] = next[j].saturating_add(out);
            }
        }
        // Cap each cell at the basin holding capacity, so a filled basin becomes a lake rather than
        // growing without bound (the excess is shed, the coarse stand-in for a full basin overflowing).
        for c in next.iter_mut() {
            *c = (*c).min(calib.max_water_depth);
        }
        self.water.cells = next;
    }

    /// The productivity derivation: set each cell's biomass CAPACITY to the Liebig minimum over water,
    /// light, temperature, and soil (`biomass_from`, the abstract-source seam the biosphere addendum
    /// replaces with real producers). The limiting factor sets the continuous productivity, no dead-zone
    /// cutoff (Principle 8). This is the ceiling; [`Self::regrow_supply`] regrows the standing stock
    /// toward it and grazing depletes it (base-level liveliness step 3).
    fn step_productivity(&mut self, temp: &Field, calib: &EnvironCalib) {
        let (w, h) = (self.width, self.height);
        for y in 0..h {
            for x in 0..w {
                let i = self.idx(x, y);
                self.capacity.cells[i] = biomass_from(
                    self.water.cells[i],
                    self.light[i],
                    temp.at(x, y),
                    calib.soil_baseline,
                    calib,
                );
            }
        }
    }

    /// Regrow the standing food stock and refresh the drinkable water supply in the resource field the
    /// grazers read (base-level liveliness step 3). This is the living resource loop, run each tick after
    /// the productivity capacity is derived and before the grazers act:
    ///
    /// FOOD is a PERSISTENT, GRAZABLE stock. Each cell's standing `bio.energy_density` supply carries
    /// over between ticks (grazing depleted it last tick); this reads it back, regrows it toward the
    /// cell's productivity capacity as a Part-15 logistic [`Stock`] (regen and over-harvest collapse for
    /// free), and writes the regrown amount. A colonization propagule floor seeds a viable-but-empty
    /// cell so logistic regrowth can bootstrap from nothing (a dry-start world greens as water arrives,
    /// and a grazed-out patch recovers rather than dying forever). So the standing amount, not the
    /// capacity, is what a grazer eats and depletes: a half-grazed patch feeds half through the same
    /// Liebig math (Principle 8), and the population settles where its draw meets what the land regrows.
    ///
    /// WATER is a DRINKABLE MIRROR of the standing water depth, refreshed (overwritten) each tick, so a
    /// being drinks where the hydrology has pooled water but drinking does not measurably drain a
    /// standing body of water at this scale (the honest limit: the hydrology field is not a drinking
    /// sink). Both supplies key off the biology-floor class strings (`bio.energy_density`,
    /// `bio.water_fraction`), never a biome or race id (Principle 9): the SOURCE binding (which physical
    /// field feeds which consumable class) is the one concrete seam, and a world's alien fluid enters
    /// the same way as data. Walks canonical row-major order; a pure deterministic fold (Principle 3).
    pub fn regrow_supply(&self, resource: &mut ResourceField, calib: &EnvironCalib) {
        let (w, h) = (self.width, self.height);
        for y in 0..h {
            for x in 0..w {
                let coord = Coord3::ground(x, y);
                let cap = self.capacity.at(x, y);
                // The persistent standing food stock (post-graze), regrown toward the capacity.
                let standing = resource.supply(coord, ENERGY_DENSITY);
                let mut stock = Stock::new(standing, cap, calib.regen_rate);
                if cap > Fixed::ZERO {
                    // Colonization: a viable-but-empty cell gets a propagule floor so logistic regrowth
                    // (zero from an empty stock) can begin; the floor never exceeds the cell's capacity.
                    let floor = calib.colonization.min(cap);
                    if stock.amount() < floor {
                        stock.deposit(floor - stock.amount());
                    }
                }
                stock.step(Fixed::ZERO); // logistic regrow toward capacity (grazing already applied)
                let food = stock.amount();
                // The drinkable water supply is the standing depth, clamped to a bounded [0, ONE] supply
                // the satisfaction measure consumes; refreshed each tick, so drinking does not deplete it.
                let water = self.water.at(x, y).min(Fixed::ONE);
                resource.set(
                    coord,
                    Composition {
                        nutrients: [
                            (ENERGY_DENSITY.to_string(), food),
                            (WATER_FRACTION.to_string(), water),
                        ]
                        .into_iter()
                        .collect(),
                        toxins: Default::default(),
                    },
                );
            }
        }
    }

    /// Fold the dynamic environmental fields into a hash in canonical field order (water then
    /// productivity capacity), the stack's contribution to the runner's `state_hash`. A field omitted
    /// here would pass replay while hiding divergence, so both dynamic fields fold; the static inputs are
    /// a pure function of the map and need not fold. The standing food stock lives in the
    /// [`ResourceField`] and is folded there (base-level liveliness step 3).
    pub fn hash_into(&self, h: &mut StateHasher) {
        self.water.hash_into(h);
        self.capacity.hash_into(h);
    }
}

/// The per-cell standing producer biomass from the environmental factors (the abstract Liebig source,
/// base-level liveliness step 2): the Liebig minimum over the water, light, temperature, and soil
/// satisfactions, each `supply / requirement` clamped to `[0, 1]` (`laws::satisfaction`), so the
/// limiting factor sets the continuous productivity in `[0, 1]` with no dead-zone cutoff (Principle 8).
/// This is the `biomass_from` seam the living-biosphere addendum arc replaces with real producer
/// occupants' per-cell biomass, so the food supply becomes flora-sourced with no rewrite.
pub fn biomass_from(
    water: Fixed,
    light: Fixed,
    temperature: Fixed,
    soil: Fixed,
    calib: &EnvironCalib,
) -> Fixed {
    laws::net_nutrition(&[
        (water, Fixed::ONE, Some(calib.water_req)),
        (light, Fixed::ONE, Some(calib.light_req)),
        (temperature, Fixed::ONE, Some(calib.temp_req)),
        (soil, Fixed::ONE, Some(calib.soil_req)),
    ])
}

/// The latitude light factor at a row: full at the equator, falling to zero at the poles, `1 - |y -
/// mid| / mid` clamped to `[0, 1]`, the same latitude gradient the worldgen temperature blend uses. A
/// pure function of the row (Principle 9: no label).
fn latitude_light(y: i32, height: i32) -> Fixed {
    let mid = height / 2;
    if mid <= 0 {
        return Fixed::ONE;
    }
    let dist = (y - mid).abs();
    (Fixed::ONE - Fixed::from_ratio(dist as i64, mid as i64)).clamp(Fixed::ZERO, Fixed::ONE)
}

/// Precompute each cell's downhill routing target: the index of the strictly-lowest of its four
/// neighbours (ties and no-lower-neighbour resolved deterministically), or the cell itself when no
/// neighbour is strictly lower (a basin). A pure fold over the frozen elevation, so the routing carries
/// no thread-schedule dependence. The neighbour scan order (up, down, left, right) is the fixed
/// tie-break, and only a strictly-lower elevation replaces the incumbent, so the choice is deterministic.
fn compute_downhill(elevation: &[Fixed], w: i32, h: i32) -> Vec<usize> {
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    let mut downhill = vec![0usize; elevation.len()];
    for y in 0..h {
        for x in 0..w {
            let i = idx(x, y);
            let here = elevation[i];
            let mut best = i;
            let mut best_elev = here;
            // Fixed neighbour order: up, down, left, right. Only a strictly-lower neighbour wins, so a
            // flat plateau routes nowhere (a basin), and the first-lowest in this order breaks a tie.
            let neighbours = [(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)];
            for (nx, ny) in neighbours {
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let ni = idx(nx, ny);
                if elevation[ni] < best_elev {
                    best_elev = elevation[ni];
                    best = ni;
                }
            }
            downhill[i] = best;
        }
    }
    downhill
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_world::{BiomeSet, FlatBounded, WorldgenParams};

    /// A hand-built stack over an explicit elevation grid (row-major tenths), moisture and light
    /// uniform, water empty, the downhill target computed from the elevation. For isolating the
    /// hydrology and routing mechanics without a generated map's noise.
    fn stack_of(w: i32, h: i32, elev_tenths: &[i64], moisture: Fixed) -> EnvironFields {
        let elevation: Vec<Fixed> = elev_tenths
            .iter()
            .map(|&t| Fixed::from_ratio(t, 10))
            .collect();
        let downhill = compute_downhill(&elevation, w, h);
        EnvironFields {
            width: w,
            height: h,
            water: ScalarField::uniform(w, h, Fixed::ZERO),
            capacity: ScalarField::uniform(w, h, Fixed::ZERO),
            moisture: vec![moisture; elev_tenths.len()],
            light: vec![Fixed::ONE; elev_tenths.len()],
            downhill,
        }
    }

    /// A routing-only calibration: no precipitation, no evaporation, so a step advects the standing
    /// water alone.
    fn routing_only() -> EnvironCalib {
        EnvironCalib {
            precip_rate: Fixed::ZERO,
            evap_a_still: Fixed::ZERO,
            evap_b_wind: Fixed::ZERO,
            ..EnvironCalib::dev_fixture()
        }
    }

    fn a_map(seed: u64) -> TileMap {
        TileMap::generate(
            seed,
            FlatBounded::new(16, 12, 1),
            &BiomeSet::dev_default(),
            &WorldgenParams::dev_default(),
        )
    }

    #[test]
    fn latitude_light_is_full_at_the_equator_and_zero_at_the_poles() {
        // A five-row column: the middle (equator) is full light, the edges (poles) dark.
        assert_eq!(
            latitude_light(2, 5),
            Fixed::ONE,
            "equator row is full light"
        );
        assert!(
            latitude_light(0, 5) < latitude_light(2, 5),
            "the pole is darker"
        );
        assert!(
            latitude_light(1, 5) > latitude_light(0, 5),
            "light rises toward the equator"
        );
    }

    #[test]
    fn biomass_is_the_liebig_minimum_of_the_factors_never_a_cutoff() {
        let c = EnvironCalib::dev_fixture(); // each requirement is 1/2
                                             // All factors met: productivity is high (every satisfaction saturates to one).
        let rich = biomass_from(Fixed::ONE, Fixed::ONE, Fixed::ONE, Fixed::ONE, &c);
        assert_eq!(rich, Fixed::ONE, "a well-supplied cell is fully productive");
        // The limiting factor (dry) sets the productivity CONTINUOUSLY, not a barren cutoff: a cell with
        // a quarter of the water requirement yields half, in proportion, never zero-by-gate.
        let dry = biomass_from(
            Fixed::from_ratio(1, 4),
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE,
            &c,
        );
        assert_eq!(
            dry,
            Fixed::from_ratio(1, 2),
            "the limiting factor sets productivity continuously (Liebig, not a cutoff)"
        );
        assert!(dry < rich, "less of the limiting factor, less productivity");
        // No water at all: the limiting factor drives productivity to zero (a desert), emergent from the
        // Liebig minimum, not an authored dead-zone test.
        assert_eq!(
            biomass_from(Fixed::ZERO, Fixed::ONE, Fixed::ONE, Fixed::ONE, &c),
            Fixed::ZERO,
            "a cell with no water grows nothing"
        );
    }

    #[test]
    fn downhill_points_to_the_lowest_neighbour_and_a_basin_retains() {
        // A 3x3 bowl: the centre (index 4) is the lowest, the rim higher. Every rim cell routes toward a
        // lower neighbour, and the centre (no lower neighbour) is a basin that retains.
        let elev = [5, 4, 5, 4, 1, 4, 5, 4, 5];
        let dh = compute_downhill(&elev.map(|e| Fixed::from_ratio(e, 10)), 3, 3);
        assert_eq!(
            dh[4], 4,
            "the basin centre has no lower neighbour, it retains"
        );
        // Each edge-centre cell (1,3,5,7) routes to the basin centre (4), its lowest neighbour.
        for &i in &[1usize, 3, 5, 7] {
            assert_eq!(dh[i], 4, "cell {i} routes downhill to the basin centre");
        }
    }

    #[test]
    fn routing_moves_water_downhill_into_a_basin() {
        // Put water on the four edge-centre cells of a 3x3 bowl; a routing-only step moves a fraction of
        // each into the centre basin, and total water is conserved (nothing evaporates or precipitates).
        let mut s = stack_of(3, 3, &[5, 4, 5, 4, 1, 4, 5, 4, 5], Fixed::ZERO);
        for &i in &[1usize, 3, 5, 7] {
            s.water.cells[i] = Fixed::ONE;
        }
        let before: Fixed = s
            .water
            .cells
            .iter()
            .copied()
            .fold(Fixed::ZERO, |a, b| a + b);
        let temp = Field::new(3, 3, vec![Fixed::ZERO; 9]);
        s.step_hydrology(&temp, &routing_only());
        let after: Fixed = s
            .water
            .cells
            .iter()
            .copied()
            .fold(Fixed::ZERO, |a, b| a + b);
        assert_eq!(
            before, after,
            "routing conserves water (no source, no sink)"
        );
        assert!(
            s.water.cells[4] > Fixed::ZERO,
            "water flowed downhill into the basin centre: {:?}",
            s.water.cells[4]
        );
    }

    #[test]
    fn the_step_is_deterministic_and_replays() {
        // The whole stack step (precipitation, evaporation, routing, productivity) over a generated map
        // replays bit for bit across two identical runs.
        let map = a_map(0xB0);
        let calib = EnvironCalib::dev_fixture();
        let run = || {
            let mut e = EnvironFields::from_map(&map);
            let temp = Field::from_map(&map);
            for _ in 0..20 {
                e.step(&temp, &calib);
            }
            let mut h = StateHasher::new();
            e.hash_into(&mut h);
            h.finish()
        };
        assert_eq!(run(), run(), "the environmental stack replays bit for bit");
    }

    #[test]
    fn stepping_a_wet_world_produces_water_and_a_productivity_capacity() {
        // Over a generated map with real moisture and temperature, stepping the stack accumulates water
        // where the climate condenses it and derives a producer-biomass capacity, then the resource loop
        // grows a standing food supply the grazers read toward that capacity.
        let map = a_map(0x5EED);
        let mut e = EnvironFields::from_map(&map);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let mut resource = ResourceField::new();
        for _ in 0..40 {
            e.step(&temp, &calib);
            e.regrow_supply(&mut resource, &calib);
        }
        let (w, h) = e.dims();
        let total_water: Fixed = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| e.water_at(x, y))
            .fold(Fixed::ZERO, |a, b| a + b);
        let total_capacity: Fixed = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| e.capacity_at(x, y))
            .fold(Fixed::ZERO, |a, b| a + b);
        assert!(
            total_water > Fixed::ZERO,
            "the climate condensed some standing water"
        );
        assert!(
            total_capacity > Fixed::ZERO,
            "some cells carry a producer-biomass capacity"
        );
        // The resource loop grew a standing food supply toward the capacity (colonization plus logistic
        // regrowth bootstrapped it from an empty field as the water arrived).
        assert!(
            resource.total_supply(ENERGY_DENSITY) > Fixed::ZERO,
            "the resource loop grew a standing food supply the grazers read"
        );

        // A cell with a capacity carries a standing energy-density supply the edibility path reads.
        let productive = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .find(|&(x, y)| e.capacity_at(x, y) > Fixed::ZERO);
        if let Some((x, y)) = productive {
            assert!(
                resource.supply(Coord3::ground(x, y), ENERGY_DENSITY) > Fixed::ZERO,
                "the productive cell supplies energy the grazers read"
            );
        }
    }

    #[test]
    fn the_food_stock_regrows_toward_capacity_and_over_harvest_collapses_it() {
        // Base-level liveliness step 3: the standing food supply is a persistent, grazable, logistically
        // regrowing stock. From an empty field over a wet map it bootstraps (colonization + regrowth) and
        // climbs toward the productivity capacity; a sustained heavy graze each tick drives it to collapse
        // (the Part-15 over-harvest feedback), while a light graze settles below capacity without collapse.
        let map = a_map(0xF00D);
        let mut e = EnvironFields::from_map(&map);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();

        // Warm the hydrology and food up: no grazing, so the stock climbs toward its capacity.
        let mut ungrazed = ResourceField::new();
        for _ in 0..60 {
            e.step(&temp, &calib);
            e.regrow_supply(&mut ungrazed, &calib);
        }
        let settled = ungrazed.total_supply(ENERGY_DENSITY);
        assert!(settled > Fixed::ZERO, "the ungrazed food stock grew");

        // Replay the same environment, but graze every cell hard each tick (take most of the supply).
        // The stock cannot keep up, so the standing total falls far below the ungrazed settle point.
        let mut e2 = EnvironFields::from_map(&map);
        let mut grazed = ResourceField::new();
        let (w, h) = e2.dims();
        for _ in 0..60 {
            e2.step(&temp, &calib);
            e2.regrow_supply(&mut grazed, &calib);
            for y in 0..h {
                for x in 0..w {
                    let coord = Coord3::ground(x, y);
                    let here = grazed.supply(coord, ENERGY_DENSITY);
                    // A heavy per-tick draw, above regrowth near the ceiling.
                    grazed.take(coord, ENERGY_DENSITY, here);
                }
            }
        }
        let after_heavy = grazed.total_supply(ENERGY_DENSITY);
        assert!(
            after_heavy < settled,
            "sustained over-harvest holds the food far below the ungrazed level: {after_heavy:?} < {settled:?}"
        );
    }
}
