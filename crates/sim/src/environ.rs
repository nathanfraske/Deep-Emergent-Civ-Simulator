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
use std::collections::{BTreeMap, BTreeSet};

use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::edibility::Composition;
use crate::locomotion::ResourceField;
use crate::material::{EarthworkField, SoilNutrientField};
use crate::physiology::{ENERGY_DENSITY, SALINITY, WATER_FRACTION};
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

    /// Draw up to `want` down from a cell's standing value (the extract-and-deplete sibling of the read): a
    /// producer's draw on the located water depth as it fixes biomass. Clamped to what is present (never
    /// negative), returns what was removed. This makes water a real draw-down sink; the hydrology is an OPEN
    /// reservoir (precipitation source, evaporation and edge-outflow sink), so a draw is dimensionally clean
    /// against the standing depth but sits OUTSIDE the conserved-matter ledger, an honest limit.
    pub fn take(&mut self, x: i32, y: i32, want: Fixed) -> Fixed {
        if x < 0 || y < 0 || x >= self.width || y >= self.height || want <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        let i = self.idx(x, y);
        let taken = want.clamp(Fixed::ZERO, self.cells[i]);
        self.cells[i] -= taken;
        taken
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
    /// The salt weathering source (base-level liveliness step 4): the salt mass a cell gains per tick as
    /// salt leaches from rock and soil. Small; over many ticks it accumulates in endorheic basins (which
    /// route salt nowhere) while throughflow washes it from well-drained cells, so salt flats emerge
    /// where water evaporates faster than it drains.
    pub salt_weathering: Fixed,
    /// The salt holding cap (base-level liveliness step 4): the maximum salt mass a cell retains, so an
    /// endorheic basin's salt saturates rather than accumulating without bound. Bounds the salinity
    /// concentration a fully-evaporated basin reaches.
    pub salt_cap: Fixed,
    /// The salinity dose scale (base-level liveliness step 4): the multiplier from a cell's salt
    /// concentration (salt over standing water plus the reference water) to the toxin dose the harm law
    /// reads on the `bio.salinity` class. Sets how lethal a given concentration is against a being's
    /// heritable tolerance.
    pub salinity_scale: Fixed,
    /// The salinity reference water (base-level liveliness step 4): the water added to a cell's standing
    /// depth when forming the salinity concentration `salt / (water + reference)`, so a bone-dry salt
    /// flat reads a high but finite concentration and a well-watered cell dilutes its salt. It sets the
    /// water scale at which salt is diluted to harmlessness, so a genome-reachable tolerance can resist a
    /// dry flat; larger dilutes salt faster (a gentler gradient), smaller makes a dry flat more lethal.
    pub reference_water: Fixed,
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
            salt_weathering: m.require_fixed("salinity.weathering_rate")?,
            salt_cap: m.require_fixed("salinity.salt_cap")?,
            salinity_scale: m.require_fixed("salinity.dose_scale")?,
            reference_water: m.require_fixed("salinity.reference_water")?,
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
            salt_weathering: Fixed::from_ratio(1, 100),
            salt_cap: Fixed::from_int(2),
            salinity_scale: Fixed::from_int(1),
            reference_water: Fixed::from_int(1),
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
    /// The standing SALT mass per cell (base-level liveliness step 4): sourced by weathering, advected
    /// downhill with the water routing, and capped, so salt accumulates in endorheic basins and washes
    /// from well-drained cells. The salinity DOSE a being suffers is the concentration `salt / (water +
    /// floor)` scaled, high where a basin has evaporated its water and left salt (a salt flat) and low
    /// where fresh water flows through, written as the `bio.salinity` toxin class.
    salt: ScalarField,
    /// Static per-cell worldgen inputs (row-major): the moisture the precipitation reads and the
    /// latitude light the productivity reads. The frozen worldgen elevation is kept too (base level per
    /// cell), so the downhill routing can be recomputed against the terrain a being has dug or mounded
    /// (material-substrate item 5); with no earthwork it is the same base that seeded `downhill`, so the
    /// routing is unchanged and the run is byte-identical.
    moisture: Vec<Fixed>,
    light: Vec<Fixed>,
    elevation: Vec<Fixed>,
    /// The downhill target index of each cell: the lowest-elevation of its four neighbours (ties broken
    /// in the fixed order up, down, left, right), or the cell itself when no neighbour is strictly lower
    /// (a basin, which retains its water). Seeded from the frozen worldgen elevation and recomputed from
    /// the effective elevation (base plus earthwork delta) whenever digging has reshaped the terrain
    /// ([`Self::recouple_terrain`]), so a dug pit becomes a basin that pools its water and salt.
    downhill: Vec<usize>,
    /// The per-cell SOIL FERTILITY supply (material-substrate arc, cascade item 8, slice C2): the extra
    /// soil-nutrient supply, over the uniform `soil_baseline`, that the matter cycle's deposited nutrient
    /// mass contributes to a cell's productivity soil factor. Derived each tick from the embodiment's soil
    /// nutrient store by [`Self::set_fertility_from`] and read by [`Self::step_productivity`], so a cell
    /// where a carcass rotted grows more where soil is the limiting factor (the matter cycle closes into
    /// the food web). All-zero until the matter cycle deposits and the runner fills it, so a scenario with
    /// no matter cycle armed reads the plain baseline and the productivity (and its hash) is unchanged. Not
    /// folded into `state_hash`: it is a pure derived read of the embodiment's soil store (which folds
    /// itself), and its effect enters the hash through the `capacity` it shifts.
    fertility: Vec<Fixed>,
    /// The located PRODUCER biomass per cell, seeded once at world build from the living biosphere (the
    /// biosphere-into-run arc). Where a real producer organism stands, its biomass is the food CAPACITY here
    /// instead of the abstract climate productivity, so the founders graze real located plants (patchy, with
    /// species identity) rather than a uniform number field. All-zero until the biosphere seeds it, so a run
    /// with no biosphere reads the plain climate productivity and its hash is unchanged. Not folded into
    /// `state_hash` (like `fertility`): its effect enters the hash through the `capacity` it sets.
    producer: Vec<Fixed>,
    /// The evolved abiotic SOURCE id LIST each producer cell draws on (an empty list where no producer
    /// stands), seeded once from the biosphere. A producer closes on ONE OR MORE abiotic sources (light,
    /// water, a soil nutrient, an alien gradient), and its productivity is the Liebig MINIMUM over the set
    /// (each source a potential limiting factor, no authored priority, Principle 8). The run consults a
    /// data-defined [`AbioticSourceRegistry`] to learn which field each id reads and whether it is a
    /// depletable stock, so the extract path never switches on the integer id (which would re-author a
    /// closed Earth source enum). A single-source producer is capped by that one source exactly as a scalar
    /// would (the min over a singleton). Not folded into `state_hash` like `producer`/`fertility`: its
    /// effect enters through the `capacity` the extract beat shapes.
    producer_source: Vec<Vec<u16>>,
    /// The standing-food COMPOSITION of the producer on each cell (chemistry arc, T3): the fixed
    /// per-unit-biomass nutrient simplex (summing to one) a producer's food carries, seeded once from the
    /// biosphere ([`crate::genesis::WorldGenesis::producer_compositions`]) and normalised here. `None` where
    /// no producer composition is seeded, in which case the standing food is the single `bio.energy_density`
    /// class exactly as before (byte-identical). Where `Some`, `regrow_supply` writes each food axis's supply
    /// as the logistic biomass VOLUME times that axis's density, and reads the remaining volume back as the
    /// Liebig MINIMUM over the axes of `supply / density`, so a grazer that ate one axis shrinks the whole
    /// plant and the composition stays a single scalar stock (never N independent stocks). Not folded into
    /// `state_hash`: its effect enters through the food supplies it shapes, which the [`ResourceField`] folds.
    producer_food: Vec<Option<BTreeMap<String, Fixed>>>,
}

/// Which run FIELD an abiotic source reads: the field IDENTITY, named explicitly so a source binds to the
/// field it actually draws on, never a variant that conflates identity with depletion behaviour (FINDING-1).
/// Whether the source DEPLETES the field is separate data ([`AbioticBinding::depletes`]), so a renewable
/// light-flux and a finite water-stock are the SAME mechanism with different data, and (with Arc 5's
/// data-defined field set) an alien source, geothermal or a redox gradient or a mana field, is a new field
/// handle plus the environ field it reads rather than a rewrite of the extract dispatch (Principle 11).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AbioticField {
    /// The LIGHT field (the latitude-light rate). Renewable by default (a world sets `depletes` if it models
    /// a consumable light budget).
    Light,
    /// The WATER field (the hydrology depth).
    Water,
    /// The located SOIL-NUTRIENT field, read and drawn by class.
    Soil,
}

/// One AVAILABILITY BAND on an abiotic source (chemistry arc, Arc 5 T1): the source is present in a region
/// only where the region's environment reads inside the HALF-OPEN interval `[min, max)` on env axis `axis`
/// (an ORDINAL into `EnvProfile::fields`, exactly as `Niche::suitability` reads its axes, so a band never
/// names "moisture" in Rust, only in the world's own row comment). `min` is inclusive, `max` is exclusive
/// (so a `min` band and a `max` band on the same axis partition it with no overlap at the boundary, as the
/// wet-versus-dry source split needs); `max = None` is an open upper bound. This is what lets a
/// lightless cave, an anaerobic world, or a mana world derive its OWN abiotic set from its own environment,
/// never the Earth "light always, water where moist" logic.
#[derive(Clone, Debug)]
pub struct AxisBand {
    pub axis: usize,
    pub min: Fixed,
    pub max: Option<Fixed>,
}

/// The availability rule of an abiotic source: it is present in a region iff EVERY band passes (a Liebig
/// conjunction, like the niche). An empty rule is always available (an unbounded renewable flux, like light
/// above ground). Data (Principle 11): a world declares each source's presence condition as bands over its
/// own env axes, never a hardcoded per-source predicate.
#[derive(Clone, Debug, Default)]
pub struct AbioticAvailability {
    pub bands: Vec<AxisBand>,
}

/// The data-defined binding of one abiotic source id to the run field it reads, whether it depletes that
/// field, and the physics-floor class it supplies. Membership is data; a world's own sources are its own rows.
#[derive(Clone, Debug)]
pub struct AbioticBinding {
    /// The run field this source reads (the field identity, decoupled from the depletion behaviour below).
    pub field: AbioticField,
    /// Whether drawing on this source DEPLETES its field's stock (a finite stock) or leaves it untouched (a
    /// renewable flux). Decoupled from the field so the flux-versus-stock choice is the world's data, not the
    /// engine's per-variant assumption (FINDING-1): a world can declare a renewable nutrient spring (Soil,
    /// `depletes = false`) as readily as a finite one. Depletion draws only a field that HAS a located stock
    /// to draw (Water, Soil); on Light the flag is presently inert because there is no light-stock field to
    /// deplete (a consumable light budget would be a new field under Arc 5's data-defined field set).
    pub depletes: bool,
    /// The physics-floor nutrient class this field supplies (used only for the soil field; empty otherwise).
    pub class: String,
    /// Where in a region this source is PRESENT (Arc 5 T1): the availability rule over the region's env axes.
    /// Empty (the default) means always present, so a source inserted without a rule keeps the old behaviour.
    pub availability: AbioticAvailability,
}

/// The abiotic-source binding registry (Principle 11): the extract mechanism is fixed Rust; the membership
/// (which evolved source id binds to which field and class) is DATA, so the run path never authors a closed
/// Earth source enum. Carries the reserved extract-deplete conversions, surfaced with basis, never set here.
#[derive(Clone, Debug, Default)]
pub struct AbioticSourceRegistry {
    bindings: BTreeMap<u16, AbioticBinding>,
    /// RESERVED (surfaced, not set): the standing biomass a unit of drawn located stock supports. Basis: the
    /// reciprocal of the soil `fertility_scale`, so biomass fixed reconciles with the mass decay returns.
    pub biomass_per_stock: Fixed,
    /// RESERVED: the fraction of the supported biomass a producer sequesters from its stock per tick. Basis:
    /// the nutrient turnover the standing biomass holds; set so a grazed cell depletes over a plausible span.
    pub draw_fraction: Fixed,
    /// RESERVED: the located stock a physical WEATHERING source (rock to nutrient) deposits per producer cell
    /// per tick, the bootstrap that seeds a bare soil before any corpse decomposes. Basis: the rock-weathering
    /// release rate the material data implies; set so a virgin world greens slowly.
    pub weathering_rate: Fixed,
}

impl AbioticSourceRegistry {
    /// The binding for a source id, if the world defines one.
    pub fn binding(&self, id: u16) -> Option<&AbioticBinding> {
        self.bindings.get(&id)
    }

    /// Bind a source id to a field, whether it depletes that field, and the class it supplies (data; called
    /// at the biosphere-arming site). A renewable flux passes `depletes = false`, a finite stock `true`. The
    /// source is ALWAYS present (empty availability); use [`Self::insert_available`] for a conditional source.
    pub fn insert(&mut self, id: u16, field: AbioticField, depletes: bool, class: &str) {
        self.insert_available(id, field, depletes, class, AbioticAvailability::default());
    }

    /// Bind a source id with an AVAILABILITY rule (Arc 5 T1): the source is present in a region only where the
    /// rule's bands pass over the region's env axes, so a world derives its own abiotic set from its own
    /// environment (water where wet, geothermal where hot, mana where the ley-field reads) rather than the
    /// Earth "light always, water where moist" hardcode.
    pub fn insert_available(
        &mut self,
        id: u16,
        field: AbioticField,
        depletes: bool,
        class: &str,
        availability: AbioticAvailability,
    ) {
        self.bindings.insert(
            id,
            AbioticBinding {
                field,
                depletes,
                class: class.to_string(),
                availability,
            },
        );
    }

    /// A labelled DEVELOPMENT FIXTURE reproducing the Earth abiotic triad exactly (Arc 5 T1), so a canonical
    /// Earth-terrain world derives byte-identical `Region.abiotic` sets to the pre-arc hardcode: id 0 LIGHT
    /// always present (a renewable flux, no bands); id 1 WATER present where the moisture axis (ordinal 1 in
    /// the dev env vector `[elevation, moisture, temperature, soil]`) reads at or above 0.3 (a depletable
    /// stock); id 2 a dryland SOIL nutrient present where drier (moisture below 0.3), supplying
    /// `bio.organic_residue`. Carries the same reserved extract-deplete scalars the run previously set inline.
    /// Not owner canon; an alien world declares its own sources and presence bands.
    pub fn earth_dev() -> AbioticSourceRegistry {
        let mut r = AbioticSourceRegistry::default();
        let moisture_axis = 1usize;
        r.insert_available(
            0,
            AbioticField::Light,
            false,
            "",
            AbioticAvailability::default(),
        );
        r.insert_available(
            1,
            AbioticField::Water,
            true,
            "",
            AbioticAvailability {
                bands: vec![AxisBand {
                    axis: moisture_axis,
                    min: Fixed::from_ratio(3, 10),
                    max: None,
                }],
            },
        );
        r.insert_available(
            2,
            AbioticField::Soil,
            true,
            "bio.organic_residue",
            AbioticAvailability {
                bands: vec![AxisBand {
                    axis: moisture_axis,
                    min: Fixed::ZERO,
                    max: Some(Fixed::from_ratio(3, 10)),
                }],
            },
        );
        r.biomass_per_stock = Fixed::from_int(4);
        r.draw_fraction = Fixed::from_ratio(1, 20);
        r.weathering_rate = Fixed::from_ratio(1, 100);
        r
    }

    /// The abiotic source ids PRESENT in a region whose env reads `fields` (Arc 5 T1): every source whose
    /// availability bands all pass, in canonical id order. Replaces the Earth-hardcoded {light, water, soil}
    /// derivation with a data read, so a world's own declared sources and presence conditions decide its
    /// regions' abiotic sets. Fail-loud on a band referencing an axis the env does not carry (a config error),
    /// naming the offending source, rather than silently dropping it.
    pub fn available_in(&self, fields: &[Fixed]) -> BTreeSet<u16> {
        let mut present = BTreeSet::new();
        for (id, binding) in &self.bindings {
            let ok = binding.availability.bands.iter().all(|band| {
                let value = *fields.get(band.axis).unwrap_or_else(|| {
                    panic!(
                        "abiotic source {id}: availability band on env axis {} but the region carries only {} axes",
                        band.axis,
                        fields.len()
                    )
                });
                value >= band.min && band.max.is_none_or(|m| value < m)
            });
            if ok {
                present.insert(*id);
            }
        }
        present
    }
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
            salt: ScalarField::uniform(w, h, Fixed::ZERO),
            moisture,
            light,
            elevation,
            downhill,
            fertility: vec![Fixed::ZERO; n],
            producer: vec![Fixed::ZERO; n],
            producer_source: vec![Vec::new(); n],
            producer_food: vec![None; n],
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

    /// The static worldgen MOISTURE at a cell (the precipitation and soil-moisture proxy, not standing
    /// water depth), a pure read over the frozen moisture input for the decomposition-activity kernel
    /// ([`crate::decompose`]). Reads nothing dynamic and mutates nothing, so exposing it changes no state
    /// and no hash. Bounds are the caller's responsibility, exactly as the reader's [`Self::water_at`].
    pub fn moisture_at(&self, x: i32, y: i32) -> Fixed {
        self.moisture[self.idx(x, y)]
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

    /// Whether a cell is a basin: it routes its water to itself because no neighbour is strictly lower, so
    /// it retains what flows in (a natural bowl or a dug pit). A pure read of the downhill routing, for the
    /// hydrology reader and to observe the terrain coupling (material-substrate item 5): a being that digs a
    /// deep enough pit turns its cell into a basin here, and a mound can lift a cell out of one.
    pub fn is_basin(&self, x: i32, y: i32) -> bool {
        let i = self.idx(x, y);
        self.downhill[i] == i
    }

    /// Recompute the downhill water routing against the terrain a being has reshaped (material-substrate
    /// item 5, the hydrology coupling): the effective elevation is the frozen worldgen base plus the
    /// per-column earthwork delta digging and mounding have accumulated, and the routing is rebuilt from
    /// it by the same pure fold that seeded it ([`compute_downhill`]). So a dug pit that drops a cell below
    /// its neighbours becomes a basin that routes to itself and pools its water and salt, and a mound that
    /// lifts a cell sheds them, the terrain change feeding the physics with no water verb. Opt-in and
    /// crucible-safe: an empty earthwork leaves the base elevation and so the seeded routing untouched
    /// (the fold is pure, base plus zero equals base), so a run in which nothing digs never recomputes and
    /// stays byte-identical; only a reshaped column pays the rebuild. Keyed off the physical elevation, no
    /// label (Principles 3, 8, 9).
    pub fn recouple_terrain(&mut self, earthwork: &EarthworkField) {
        if earthwork.is_empty() {
            return;
        }
        let mut effective = self.elevation.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                let delta = earthwork.delta(Coord3::ground(x, y));
                if delta != Fixed::ZERO {
                    let i = self.idx(x, y);
                    effective[i] = effective[i].saturating_add(delta);
                }
            }
        }
        self.downhill = compute_downhill(&effective, self.width, self.height);
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
        self.step_salinity(calib);
        self.step_productivity(temp, calib);
    }

    /// The salinity stencil (base-level liveliness step 4): weather salt into every cell, then advect it
    /// downhill with the water routing (the same precomputed lowest-neighbour targets the hydrology uses),
    /// double-buffered and conservative except at map-edge outflow, then cap. Salt accumulates in
    /// endorheic basins (which route to themselves, so they retain all their salt) and washes from
    /// well-drained cells, so a basin whose water evaporates concentrates its salt into a salt flat. The
    /// concentration a being suffers is derived in [`Self::salinity_at`] from this salt and the standing
    /// water; salinity does not limit productivity here (it is an animal toxin, the halophile-selection
    /// gradient, not a plant factor). Pinned integer folds in canonical order, so it replays (Principle 3).
    fn step_salinity(&mut self, calib: &EnvironCalib) {
        let (w, h) = (self.width, self.height);
        let n = (w as usize) * (h as usize);
        // (1) Weathering source, pointwise.
        let mut sourced = vec![Fixed::ZERO; n];
        for (dst, &cur) in sourced.iter_mut().zip(self.salt.cells.iter()) {
            *dst = cur.saturating_add(calib.salt_weathering);
        }
        // (2) Downhill advection with the water routing, double-buffered (each cell keeps its retained
        // salt and receives the outflow of every higher neighbour that routes to it). A basin (downhill
        // to self) sends nothing and retains everything.
        let mut next = vec![Fixed::ZERO; n];
        for i in 0..n {
            let out = if self.downhill[i] != i {
                calib.routing_rate.mul(sourced[i])
            } else {
                Fixed::ZERO
            };
            next[i] = next[i].saturating_add(sourced[i] - out);
            if out > Fixed::ZERO {
                let j = self.downhill[i];
                next[j] = next[j].saturating_add(out);
            }
        }
        // (3) Cap each cell's salt at the holding capacity, so a basin saturates rather than growing
        // without bound.
        for c in next.iter_mut() {
            *c = (*c).min(calib.salt_cap);
        }
        self.salt.cells = next;
    }

    /// The standing SALT mass at a cell (base-level liveliness step 4): high in endorheic basins that
    /// retain their salt, low on well-drained slopes that wash it away. A pure per-cell field read for
    /// the field reader. The toxin DOSE a being suffers is derived in [`Self::salinity_dose`] from this
    /// salt and the standing water (the concentration), so a wet basin dilutes its salt and a dry one
    /// concentrates it.
    pub fn salt_at(&self, x: i32, y: i32) -> Fixed {
        self.salt.at(x, y)
    }

    /// The salinity DOSE at a cell (base-level liveliness step 4): the concentration `salt / (water +
    /// reference)` scaled by `salinity.dose_scale`, the dose the harm law reads on the `bio.salinity`
    /// class. High where a basin has evaporated its water and left salt (a salt flat), near zero where
    /// fresh water dilutes it. Pure fixed-point. The `reference_water` keeps the denominator positive for
    /// every set (positive) calibration; should it ever be zero AND the cell bone-dry, the concentration
    /// of a salt-bearing cell is unbounded, so the fallback is the MAXIMAL dose (the deadliest reading),
    /// never a false zero that would report the driest, most salt-saturated cell as safe. The harm law
    /// saturates a maximal dose to its harm cap, so a maximal dose is bounded downstream.
    pub fn salinity_dose(&self, x: i32, y: i32, calib: &EnvironCalib) -> Fixed {
        let salt = self.salt.at(x, y);
        if salt <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        let denom = self.water.at(x, y).saturating_add(calib.reference_water);
        // A zero denominator (a bone-dry cell under a zero reference) means an unbounded concentration:
        // fall back to the maximum, the deadliest reading, matching the mechanism's own invariant that a
        // dry salt flat reads a HIGH concentration (never a false zero).
        let concentration = salt.checked_div(denom).unwrap_or(Fixed::MAX);
        concentration
            .checked_mul(calib.salinity_scale)
            .unwrap_or(concentration)
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

    /// Fill the per-cell soil fertility supply from the matter cycle's soil nutrient store (material-
    /// substrate arc, cascade item 8, slice C2), scaled by the reserved fertility scale (the soil supply
    /// gained per unit deposited nutrient mass). Called by the runner before [`Self::step_productivity`]
    /// when the matter cycle is armed, so a cell where a carcass rotted contributes its nutrient mass to
    /// its productivity soil factor. The store is sparse (only decomposed cells carry nutrient); the
    /// fertility vector is reset to zero and refilled each tick, so a cell that loses its nutrient (or a
    /// scenario that disarms the matter cycle) reads no bonus. Multiple z-levels at a cell column sum into
    /// the same 2D productivity cell. A pure deterministic fold in canonical (cell) order (Principle 3).
    pub fn set_fertility_from(&mut self, soil: &SoilNutrientField, scale: Fixed) {
        for f in self.fertility.iter_mut() {
            *f = Fixed::ZERO;
        }
        for (cell, total) in soil.cell_totals() {
            if cell.x < 0 || cell.y < 0 || cell.x >= self.width || cell.y >= self.height {
                continue;
            }
            let i = self.idx(cell.x, cell.y);
            let supply = total.checked_mul(scale).unwrap_or(Fixed::MAX);
            self.fertility[i] = self.fertility[i].saturating_add(supply);
        }
    }

    /// Seed the located PRODUCER biomass from the living biosphere (the biosphere-into-run arc), once at
    /// world build. Overwrites (idempotent on re-seed); an off-grid cell is dropped. See [`Self::producer`].
    pub fn set_producer(&mut self, cells: &[(Coord3, Fixed)]) {
        for p in self.producer.iter_mut() {
            *p = Fixed::ZERO;
        }
        for &(cell, biomass) in cells {
            if cell.x < 0 || cell.y < 0 || cell.x >= self.width || cell.y >= self.height {
                continue;
            }
            let i = self.idx(cell.x, cell.y);
            self.producer[i] = self.producer[i].saturating_add(biomass);
        }
    }

    /// Seed the evolved abiotic source id LIST of each producer cell from the biosphere (once at world
    /// build). A cell's list is overwritten (last occupant wins on a shared cell, as the scalar seed did).
    /// Off-grid cells are dropped. See [`Self::producer_source`].
    pub fn set_producer_source(&mut self, cells: &[(Coord3, Vec<u16>)]) {
        for s in self.producer_source.iter_mut() {
            s.clear();
        }
        for (cell, ids) in cells {
            if cell.x < 0 || cell.y < 0 || cell.x >= self.width || cell.y >= self.height {
                continue;
            }
            let i = self.idx(cell.x, cell.y);
            self.producer_source[i] = ids.clone();
        }
    }

    /// Seed the standing-food COMPOSITION of each producer cell from the biosphere (chemistry arc, T3, once at
    /// world build), normalising the given per-unit-biomass axis vector to a NUTRIENT SIMPLEX (the food axes
    /// summing to one) so the standing food is the producer's own chemistry scaled by the logistic biomass
    /// volume. The environmental axes regrow writes itself (water, salinity) are EXCLUDED, so the food
    /// composition never fights the hydrology write. A composition with no positive food axis seeds nothing
    /// (the cell keeps the single energy-density default, byte-identical). Off-grid cells dropped; last
    /// occupant wins on a shared cell. See [`Self::producer_food`].
    pub fn set_producer_food(&mut self, cells: &[(Coord3, BTreeMap<String, Fixed>)]) {
        for f in self.producer_food.iter_mut() {
            *f = None;
        }
        for (cell, comp) in cells {
            if cell.x < 0 || cell.y < 0 || cell.x >= self.width || cell.y >= self.height {
                continue;
            }
            let is_food = |a: &str| a != WATER_FRACTION && a != SALINITY;
            let total = comp
                .iter()
                .filter(|(a, v)| is_food(a) && **v > Fixed::ZERO)
                .fold(Fixed::ZERO, |acc, (_, v)| acc.saturating_add(*v));
            if total <= Fixed::ZERO {
                continue; // no positive food axis: keep the energy-density default
            }
            let simplex: BTreeMap<String, Fixed> = comp
                .iter()
                .filter(|(a, v)| is_food(a) && **v > Fixed::ZERO)
                .map(|(a, v)| (a.clone(), v.checked_div(total).unwrap_or(Fixed::ZERO)))
                .filter(|(_, v)| *v > Fixed::ZERO)
                .collect();
            if simplex.is_empty() {
                continue;
            }
            let i = self.idx(cell.x, cell.y);
            self.producer_food[i] = Some(simplex);
        }
    }

    /// The producer EXTRACT-DEPLETE beat (the closed nutrient cycle): each producer draws its food from the
    /// availability of the specific abiotic source its niche EVOLVED to close on (its `producer_source` id,
    /// bound to a field by the data-defined `registry`, never an id switch), the source LIMITS its
    /// productivity ceiling, and a depletable stock (soil, water) is DRAWN DOWN so a heavily-worked cell
    /// depletes rather than reading an infinite well. A physical WEATHERING source (rock to soil nutrient)
    /// seeds a bare soil stock each tick so the cycle bootstraps before any corpse decomposes. Runs after
    /// `step_productivity` set the niche-fit `capacity` and before `regrow_supply`. Sequential row-major
    /// fold (worker-invariant); the soil/water `take`/`deposit` are canonical, so the new state folds
    /// reproducibly. The biomass layer is OPEN; the conserved ledger is soil+material+tissue across decay.
    pub fn extract_producers(
        &mut self,
        soil: &mut SoilNutrientField,
        registry: &AbioticSourceRegistry,
    ) {
        // Fail loud on an unset reserved conversion rather than silently capping every producer to zero.
        assert!(
            registry.biomass_per_stock > Fixed::ZERO,
            "nutrient cycle: biomass_per_stock reserved value is unset (would starve every producer)"
        );
        let (w, h) = (self.width, self.height);
        for y in 0..h {
            for x in 0..w {
                let i = self.idx(x, y);
                if self.producer_source[i].is_empty() {
                    continue; // no producer stands on this cell
                }
                let coord = Coord3::ground(x, y);
                // Pass 1 (the Liebig LAW OF THE MINIMUM, Principle 8): each evolved source is a potential
                // limiting factor. Deposit the physical weathering seed of each soil source, read each
                // source's supply, and cap the productivity to the SCARCEST source's supported biomass, so a
                // producer that closes on light AND water AND a soil nutrient is limited by whichever is
                // least, with no authored priority among them. A single-source producer is capped by that one
                // source exactly as the scalar seed was (the minimum over a singleton is that source).
                let mut min_supported = Fixed::MAX;
                for k in 0..self.producer_source[i].len() {
                    let id = self.producer_source[i][k];
                    // A seeded producer whose evolved source the world never bound is a config error: fail
                    // loud rather than fall through to an uncapped producer (the old infinite-well).
                    let binding = registry.binding(id).unwrap_or_else(|| {
                        panic!("nutrient cycle: producer source id {id} has no registry binding")
                    });
                    let supply = match binding.field {
                        AbioticField::Light => self.light[i],
                        AbioticField::Soil => {
                            // The rock-to-nutrient WEATHERING bootstrap seeds the soil stock (soil-specific;
                            // a general per-field replenishment handle is Arc 5).
                            if registry.weathering_rate > Fixed::ZERO {
                                soil.deposit(coord, &binding.class, registry.weathering_rate);
                            }
                            soil.mass(coord, &binding.class)
                        }
                        AbioticField::Water => self.water.at(x, y),
                    };
                    let supported = supply
                        .checked_mul(registry.biomass_per_stock)
                        .unwrap_or(Fixed::MAX);
                    if supported < min_supported {
                        min_supported = supported;
                    }
                }
                if self.capacity.cells[i] > min_supported {
                    self.capacity.cells[i] = min_supported;
                }
                // Pass 2: draw down each DEPLETING source's field by the realised productivity's share, so a
                // heavily-worked cell depletes rather than reading an infinite well. Whether a source depletes
                // is its own DATA (`binding.depletes`), decoupled from the field (FINDING-1): a renewable flux
                // leaves its field untouched, a finite stock draws down, and the world chooses which. The draw
                // reads the FINAL capped capacity, so the single-source sequence (cap, then draw) is
                // bit-identical to the scalar path it replaces.
                for k in 0..self.producer_source[i].len() {
                    let id = self.producer_source[i][k];
                    let binding = registry.binding(id).unwrap_or_else(|| {
                        panic!("nutrient cycle: producer source id {id} has no registry binding")
                    });
                    if !binding.depletes {
                        continue; // a renewable source leaves its field untouched
                    }
                    let draw_biomass = self.capacity.cells[i]
                        .checked_mul(registry.draw_fraction)
                        .unwrap_or(Fixed::ZERO);
                    let draw_amt = draw_biomass
                        .checked_div(registry.biomass_per_stock)
                        .unwrap_or(Fixed::ZERO);
                    match binding.field {
                        AbioticField::Light => {} // light has no located stock to deplete
                        AbioticField::Soil => {
                            soil.take(coord, &binding.class, draw_amt);
                        }
                        AbioticField::Water => {
                            self.water.take(x, y, draw_amt);
                        }
                    }
                }
            }
        }
    }

    /// The productivity derivation: set each cell's biomass CAPACITY to the Liebig minimum over water,
    /// light, temperature, and soil (`biomass_from`, the abstract-source seam the biosphere addendum
    /// replaces with real producers). The limiting factor sets the continuous productivity, no dead-zone
    /// cutoff (Principle 8). This is the ceiling; [`Self::regrow_supply`] regrows the standing stock
    /// toward it and grazing depletes it (base-level liveliness step 3). The soil supply is the uniform
    /// `soil_baseline` plus the per-cell fertility the matter cycle has deposited ([`Self::set_fertility_from`]),
    /// so a fertilised cell grows more where soil is the limiting factor and the matter cycle closes into
    /// the food web. With no matter cycle armed the fertility is zero and the soil supply is the plain
    /// baseline, so the productivity (and its hash) is unchanged.
    fn step_productivity(&mut self, temp: &Field, calib: &EnvironCalib) {
        let (w, h) = (self.width, self.height);
        for y in 0..h {
            for x in 0..w {
                let i = self.idx(x, y);
                let soil = calib.soil_baseline.saturating_add(self.fertility[i]);
                let climate = biomass_from(
                    self.water.cells[i],
                    self.light[i],
                    temp.at(x, y),
                    soil,
                    calib,
                );
                // Where a real producer organism stands (biosphere-into-run), its located biomass is the food
                // ceiling; elsewhere the abstract climate productivity is the baseline (the stand-in for
                // unmodelled background vegetation). An all-zero producer (no biosphere seeded) takes the
                // climate branch on every cell, so the capacity and its hash are byte-unchanged.
                self.capacity.cells[i] = if self.producer[i] > Fixed::ZERO {
                    self.producer[i]
                } else {
                    climate
                };
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
        // Size the dense ground layer to the environ grid the first time (idempotent), so every per-cell
        // write below lands in an O(1) indexed slot rather than a BTreeMap tree descent.
        resource.set_dims(w, h);
        for y in 0..h {
            for x in 0..w {
                let coord = Coord3::ground(x, y);
                let cap = self.capacity.at(x, y);
                // The drinkable water supply is the standing depth, clamped to a bounded [0, ONE] supply
                // the satisfaction measure consumes; refreshed each tick, so drinking does not deplete it.
                let water = self.water.at(x, y).min(Fixed::ONE);
                // The salinity DOSE (base-level liveliness step 4): the concentration scaled, written as
                // the bio.salinity toxin the harm sink reads against a being's heritable tolerance. A
                // present dose on a dosed cell, absent on a fresh one (the substrate absence convention).
                let salinity = self.salinity_dose(x, y, calib);
                // Read the post-graze standing food, regrow it, and write food, water, and salinity back in
                // ONE tree access, reusing the tile's entry and its already-interned key strings. This is
                // the byte-identical replacement for rebuilding a Composition, two key strings, and two maps
                // per cell every tick: an absent tile is created empty (standing reads zero, matching the
                // old `supply`), and the only classes the field ever bears are these three, so the stored
                // content and its state-hash fold are unchanged. At a large grid this is the whole cost.
                // The producer's fixed food composition on this cell (T3), if the biosphere seeded one; where
                // none, the food is the single energy-density class exactly as before.
                let food = self.producer_food[self.idx(x, y)].as_ref();
                let comp = resource.composition_mut(coord);
                // The standing food VOLUME read back: with a producer composition, the remaining volume is the
                // Liebig MINIMUM over its axes of supply/density (a grazer that ate one axis has shrunk the
                // whole plant, and the axes stay in the composition's fixed ratio, so the stock is a single
                // scalar, never N independent stocks); with none, the food is the single energy-density class
                // and the volume IS its supply (byte-identical to before).
                let standing = match food {
                    Some(fc) => food_volume(comp, fc),
                    None => comp.nutrient(ENERGY_DENSITY),
                };
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
                let volume = stock.amount();
                // Write the food supplies from the regrown VOLUME and the fixed composition (each food axis =
                // volume times its density), so the standing food carries the producer's OWN chemistry a
                // grazer reads; with no composition it is the single energy-density class exactly as before.
                match food {
                    Some(fc) => {
                        for (axis, density) in fc {
                            comp.set_nutrient(
                                axis,
                                volume.checked_mul(*density).unwrap_or(Fixed::MAX),
                            );
                        }
                    }
                    None => comp.set_nutrient(ENERGY_DENSITY, volume),
                }
                comp.set_nutrient(WATER_FRACTION, water);
                comp.set_toxin(SALINITY, salinity);
            }
        }
    }

    /// Fold the dynamic environmental fields into a hash in canonical field order (water, productivity
    /// capacity, then salt), the stack's contribution to the runner's `state_hash`. A field omitted here
    /// would pass replay while hiding divergence, so every dynamic field folds; the static inputs are a
    /// pure function of the map and need not fold. The standing food stock lives in the [`ResourceField`]
    /// and is folded there (base-level liveliness step 3).
    pub fn hash_into(&self, h: &mut StateHasher) {
        self.water.hash_into(h);
        self.capacity.hash_into(h);
        self.salt.hash_into(h);
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

/// The standing food VOLUME implied by a cell's food supplies and its fixed producer composition (T3): the
/// Liebig MINIMUM over the composition's axes of `supply / density`, so the axes stay in the fixed ratio and
/// a grazer that depleted one axis has shrunk the WHOLE plant's volume (never N axes drifting apart into
/// independent stocks). An empty composition or one whose densities are all non-positive reads zero.
fn food_volume(comp: &Composition, food_comp: &BTreeMap<String, Fixed>) -> Fixed {
    let mut vol: Option<Fixed> = None;
    for (axis, density) in food_comp {
        if *density <= Fixed::ZERO {
            continue;
        }
        // The volume this axis's supply implies. An overflow (a large supply over a tiny density) means an
        // effectively unbounded volume, so the axis is NON-LIMITING and reads MAX (the Liebig minimum ignores
        // it); reading ZERO here would wrongly zero the WHOLE cell's standing food off one overflowing axis.
        let axis_vol = comp
            .nutrient(axis)
            .checked_div(*density)
            .unwrap_or(Fixed::MAX);
        vol = Some(match vol {
            Some(cur) => axis_vol.min(cur),
            None => axis_vol,
        });
    }
    vol.unwrap_or(Fixed::ZERO)
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
            salt: ScalarField::uniform(w, h, Fixed::ZERO),
            moisture: vec![moisture; elev_tenths.len()],
            light: vec![Fixed::ONE; elev_tenths.len()],
            elevation,
            downhill,
            fertility: vec![Fixed::ZERO; elev_tenths.len()],
            producer: vec![Fixed::ZERO; elev_tenths.len()],
            producer_source: vec![Vec::new(); elev_tenths.len()],
            producer_food: vec![None; elev_tenths.len()],
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
    fn a_dug_pit_becomes_a_basin_that_pools_its_water() {
        // Material-substrate item 5, the hydrology coupling. On a plane sloping down to the right, every
        // interior cell routes to its lower-x neighbour, so nothing ponds in the middle. Dig the centre a
        // full unit below the plane and recouple the routing to the reshaped terrain: the centre becomes a
        // basin its four neighbours drain into, so water placed on the rim pools in the pit where before it
        // ran off to the right. The terrain change feeds the physics with no water verb.
        let elev = [3, 2, 1, 3, 2, 1, 3, 2, 1]; // sloping down to the right (lower x-elevation eastward)
        let mut s = stack_of(3, 3, &elev, Fixed::ZERO);
        let centre = s.idx(1, 1);
        assert!(
            !s.is_basin(1, 1),
            "on the bare slope the interior routes east, it is no basin"
        );

        // Dig the centre a full unit below the plane (its elevation was 0.2; now -0.8, below every
        // neighbour) and recouple the hydrology to the new terrain.
        let mut ew = EarthworkField::new();
        ew.adjust(Coord3::ground(1, 1), Fixed::ZERO - Fixed::ONE);
        s.recouple_terrain(&ew);
        assert!(
            s.is_basin(1, 1),
            "the dug pit routes to itself, it retains its water"
        );
        for &(x, y) in &[(1, 0), (1, 2), (0, 1), (2, 1)] {
            let i = s.idx(x, y);
            assert_eq!(
                s.downhill[i], centre,
                "the neighbour at ({x},{y}) now drains into the dug pit"
            );
        }

        // Route water: a unit on each of the four neighbours, one routing-only step, and it flows into the
        // pit, which before the dig would have run east off the map.
        for &(x, y) in &[(1, 0), (1, 2), (0, 1), (2, 1)] {
            let i = s.idx(x, y);
            s.water.cells[i] = Fixed::ONE;
        }
        let temp = Field::new(3, 3, vec![Fixed::ZERO; 9]);
        s.step_hydrology(&temp, &routing_only());
        assert!(
            s.water.cells[centre] > Fixed::ZERO,
            "water pooled in the dug pit: {:?}",
            s.water.cells[centre]
        );
    }

    #[test]
    fn an_empty_earthwork_leaves_the_routing_byte_identical() {
        // The crucible-safety guarantee: with nothing dug, recoupling the terrain is a no-op, so the seeded
        // worldgen routing is unchanged. A run in which no being reshapes the ground never recomputes the
        // routing and cannot diverge, so every existing scenario stays byte-identical.
        let mut s = stack_of(3, 3, &[3, 2, 1, 3, 2, 1, 3, 2, 1], Fixed::ZERO);
        let before = s.downhill.clone();
        s.recouple_terrain(&EarthworkField::new());
        assert_eq!(
            s.downhill, before,
            "an empty earthwork changes no routing (the opt-in no-op)"
        );
    }

    #[test]
    fn salt_concentrates_in_an_endorheic_basin() {
        // Base-level liveliness step 4: a 3x3 bowl whose centre is an endorheic basin (routes to itself)
        // and whose rim routes downhill toward it. Weathering salts every cell; the basin retains all its
        // salt and receives the rim's outflow while the rim washes salt toward the centre, so after
        // stepping the basin centre holds far more salt than a well-drained corner, the salt flat.
        let mut s = stack_of(3, 3, &[5, 4, 5, 4, 1, 4, 5, 4, 5], Fixed::ZERO);
        let calib = EnvironCalib::dev_fixture();
        for _ in 0..80 {
            s.step_salinity(&calib);
        }
        let centre = s.salt_at(1, 1);
        let corner = s.salt_at(0, 0);
        assert!(centre > Fixed::ZERO, "the basin accumulated salt");
        assert!(
            centre > corner,
            "salt concentrates in the endorheic basin, not on the well-drained rim: {centre:?} > {corner:?}"
        );
    }

    #[test]
    fn a_bone_dry_salt_flat_never_reads_as_safe_even_with_a_zero_reference() {
        // Regression: salinity_dose must never report the driest, most salt-saturated cell as safe. With
        // the dilution reference defeated (reference_water zero) and a bone-dry salted cell, the salt
        // concentration is unbounded, so the dose is the MAXIMAL reading, not a false zero that would tell
        // a being the deadliest salt flat in the world is harmless.
        let mut s = stack_of(1, 1, &[5], Fixed::ZERO); // water is uniform zero (a bone-dry cell)
        s.salt.cells[0] = Fixed::from_int(2); // salt present on the dry cell
        let mut calib = EnvironCalib::dev_fixture();
        calib.reference_water = Fixed::ZERO; // defeat the dilution guard
        let dry = s.salinity_dose(0, 0, &calib);
        assert!(
            dry > Fixed::ZERO,
            "a dry salt flat is never read as safe (a false zero would omit the toxin entirely): {dry:?}"
        );
        // With a positive reference the same cell reads a finite, smaller dose: dilution is monotone, and
        // the zero-reference fallback is the deadlier limit, not a lesser one.
        calib.reference_water = Fixed::from_int(1);
        let diluted = s.salinity_dose(0, 0, &calib);
        assert!(
            dry > diluted,
            "the zero-reference fallback reads deadlier than a positive dilution: {dry:?} > {diluted:?}"
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
    fn standing_food_carries_the_producers_own_composition_and_grazing_draws_the_whole_volume() {
        // Chemistry arc, T3: a producer's standing food is its OWN composition (not one minted energy class),
        // stored as a single logistic biomass VOLUME with the composition riding as fixed densities. Two
        // proofs: (1) the food a grazer reads stands in the producer's composition RATIO; (2) grazing ONE axis
        // draws the VOLUME, so ALL axes shrink together (the plant loses biomass, not one nutrient leached out
        // of it), the read-back staying a single scalar stock rather than N independent ones.
        let map = a_map(0x7EED);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let mut e = EnvironFields::from_map(&map);
        for _ in 0..30 {
            e.step(&temp, &calib); // warm the hydrology so cells carry a productivity capacity
        }
        let (w, h) = e.dims();
        let cell = (1..h - 1)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .find(|&(x, y)| e.capacity_at(x, y) > Fixed::ZERO)
            .map(|(x, y)| Coord3::ground(x, y))
            .expect("the wet world offers a productive interior cell");
        // The producer is 60% energy, 40% a second nutrient (its fixed composition).
        let comp: BTreeMap<String, Fixed> = [
            (ENERGY_DENSITY.to_string(), Fixed::from_ratio(6, 10)),
            ("bio.protein".to_string(), Fixed::from_ratio(4, 10)),
        ]
        .into_iter()
        .collect();
        e.set_producer_food(&[(cell, comp)]);
        let mut resource = ResourceField::new();
        for _ in 0..20 {
            e.step(&temp, &calib);
            e.regrow_supply(&mut resource, &calib);
        }
        let energy0 = resource.supply(cell, ENERGY_DENSITY);
        let protein0 = resource.supply(cell, "bio.protein");
        assert!(
            energy0 > Fixed::ZERO && protein0 > Fixed::ZERO,
            "the standing food carries both of the producer's own axes, not one minted energy class"
        );
        // Proof (1): the two axes are volume * density, so their ratio is the density ratio exactly (60:40,
        // i.e. energy = 1.5 * protein).
        assert_eq!(
            energy0,
            protein0.checked_mul(Fixed::from_ratio(3, 2)).unwrap(),
            "the food axes stand in the producer's fixed 60:40 composition ratio"
        );
        // Proof (2): graze the ENERGY axis to nothing, then regrow once.
        resource.take(cell, ENERGY_DENSITY, energy0);
        e.step(&temp, &calib);
        e.regrow_supply(&mut resource, &calib);
        let energy1 = resource.supply(cell, ENERGY_DENSITY);
        let protein1 = resource.supply(cell, "bio.protein");
        assert!(
            protein1 < protein0,
            "grazing the energy axis shrank the WHOLE plant: the un-grazed protein axis fell too (a volume \
             draw, not a per-axis leach)"
        );
        // And the composition ratio survives the graze-and-regrow (the axes never drift into independent
        // stocks): the read-back is the single-volume Liebig minimum, so both are volume * density again.
        assert_eq!(
            energy1,
            protein1.checked_mul(Fixed::from_ratio(3, 2)).unwrap(),
            "the food axes stay in the fixed composition ratio after grazing (one volume stock, not N)"
        );
    }

    #[test]
    fn decomposed_soil_fertility_lifts_productivity_where_soil_is_the_limiting_factor() {
        // Material-substrate item 8 slice C2 (the matter cycle closes into the food web): the nutrient the
        // matter cycle deposits into the soil raises a cell's productivity where soil is the limiting Liebig
        // factor. Under a soil-limited calibration (no uniform soil baseline), a wet, lit, warm cell still
        // carries no capacity because soil starves it; fertilising that one cell lifts its capacity above
        // zero, while an equally-viable unfertilised cell stays barren, so the lift is LOCAL to where the
        // matter rotted (no coded fertility rule, just the deposited nutrient relaxing the soil factor).
        let map = a_map(0x5EED);
        let temp = Field::from_map(&map);
        // Soil is the only starved factor: no uniform soil baseline, the other requirements the fixture's.
        let calib = EnvironCalib {
            soil_baseline: Fixed::ZERO,
            ..EnvironCalib::dev_fixture()
        };
        let mut e = EnvironFields::from_map(&map);
        // Accumulate water so some cells are wet (soil the only starved factor there).
        for _ in 0..40 {
            e.step(&temp, &calib);
        }
        let (w, h) = e.dims();
        // With no soil supply, every cell is soil-limited to zero capacity.
        let total_capacity: Fixed = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| e.capacity_at(x, y))
            .fold(Fixed::ZERO, |a, b| a + b);
        assert_eq!(
            total_capacity,
            Fixed::ZERO,
            "with no soil supply every cell is soil-limited to zero productivity"
        );
        // Viable interior cells (wet, warm, off the dark poles), so only soil limits them: one to fertilise,
        // one an unfertilised control.
        let viable: Vec<(i32, i32)> = (1..h - 1)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| e.water_at(x, y) > Fixed::ZERO && temp.at(x, y) > Fixed::ZERO)
            .collect();
        assert!(
            viable.len() >= 2,
            "the wet world offers at least two viable cells to compare"
        );
        let (fx, fy) = viable[0];
        let (ux, uy) = viable[1];
        // Deposit soil nutrient at the fertilised cell (as the matter cycle would) and fill the fertility;
        // a unit scale and a unit mass raise its soil supply to the requirement, so soil no longer starves.
        let mut soil = SoilNutrientField::new();
        soil.deposit(
            Coord3::ground(fx, fy),
            "bio.mineral_ash_fraction",
            Fixed::from_int(1),
        );
        e.set_fertility_from(&soil, Fixed::ONE);
        e.step(&temp, &calib);
        assert!(
            e.capacity_at(fx, fy) > Fixed::ZERO,
            "the fertilised cell now carries productivity (the deposited nutrient relaxed the soil factor)"
        );
        assert_eq!(
            e.capacity_at(ux, uy),
            Fixed::ZERO,
            "an equally-viable unfertilised cell stays barren (the lift is local to the deposit)"
        );
    }

    #[test]
    fn a_regions_abiotic_set_derives_from_data_availability_bands_and_admits_an_alien_world() {
        // Arc 5 T1: which abiotic sources a region offers is a DATA read (each source's availability bands
        // over the region's env axes), not the Earth "light always, water where moist" hardcode. Proof (1):
        // the earth_dev fixture reproduces the exact Earth triad at a moist and a dry region. Proof (2): an
        // ALIEN registry with NO light and its own sources derives its own set with no Earth assumption.
        let earth = AbioticSourceRegistry::earth_dev();
        // Env vector layout [elevation, moisture, temperature, soil]; moisture is ordinal 1.
        let moist = vec![
            Fixed::ONE,
            Fixed::from_ratio(6, 10),
            Fixed::ONE,
            Fixed::ZERO,
        ];
        let dry = vec![
            Fixed::ONE,
            Fixed::from_ratio(1, 10),
            Fixed::ONE,
            Fixed::ZERO,
        ];
        assert_eq!(
            earth.available_in(&moist),
            [0u16, 1].into_iter().collect(),
            "a moist region: light (always) + water (moisture >= 0.3)"
        );
        assert_eq!(
            earth.available_in(&dry),
            [0u16, 2].into_iter().collect(),
            "a dry region: light (always) + the dryland soil source (moisture < 0.3), not water"
        );

        // An alien world: NO light source at all; a geothermal source present only where hot (env axis 2
        // above a threshold); a chemosynthetic source always present. No Earth axis or source is assumed.
        let mut alien = AbioticSourceRegistry::default();
        alien.insert_available(
            10,
            AbioticField::Soil,
            true,
            "chem.redox",
            AbioticAvailability::default(), // always
        );
        alien.insert_available(
            11,
            AbioticField::Water,
            true,
            "",
            AbioticAvailability {
                bands: vec![AxisBand {
                    axis: 2,
                    min: Fixed::from_ratio(8, 10),
                    max: None,
                }],
            }, // geothermal: only where the temperature axis is hot
        );
        let hot = vec![Fixed::ZERO, Fixed::ZERO, Fixed::ONE, Fixed::ZERO];
        let cold = vec![
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::from_ratio(1, 10),
            Fixed::ZERO,
        ];
        assert_eq!(
            alien.available_in(&hot),
            [10u16, 11].into_iter().collect(),
            "a hot alien region: the always-on chemosynthetic source + the geothermal source; NO light"
        );
        assert_eq!(
            alien.available_in(&cold),
            [10u16].into_iter().collect(),
            "a cold alien region: only the chemosynthetic source; the geothermal source is absent, and light \
             never enters because this world declares none"
        );
    }

    #[test]
    fn productivity_is_the_liebig_minimum_over_the_evolved_source_set_not_the_first() {
        // T2 source-vector (the chemistry generalization): a producer closes on a SET of abiotic sources and
        // the extract beat caps its productivity to the SCARCEST of them (the Law of the Minimum), never the
        // first-listed. A cell drawing on an abundant flux AND a scarce soil nutrient is limited by the soil;
        // the same cell drawing on the flux ALONE is not. The soil-limited outcome is identical whether the
        // soil is listed first or last, so the min is over the whole set, not the head of the list.
        let map = a_map(0x50FF);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        // A scarce soil stock: seeded tiny so its supported biomass sits far below the abundant flux's.
        let scarce = Fixed::from_ratio(1, 100);
        // The registry: id 0 an abundant flux (light), id 1 a scarce soil nutrient. The draw is zeroed so
        // this isolates the CAP from the deplete, and there is no weathering bootstrap.
        let mut reg = AbioticSourceRegistry::default();
        reg.insert(0, AbioticField::Light, false, ""); // an abundant renewable flux
        reg.insert(1, AbioticField::Soil, true, "nutrient"); // a scarce depletable soil stock
        reg.biomass_per_stock = Fixed::from_int(4);
        reg.draw_fraction = Fixed::ZERO;
        reg.weathering_rate = Fixed::ZERO;
        // A helper: run the extract beat once on a fresh field with the given source list and return the
        // capped capacity at `cell`. A large producer biomass makes the pre-cap capacity high, so the source
        // set is what binds.
        let run = |ids: Vec<u16>| -> Fixed {
            let mut e = EnvironFields::from_map(&map);
            e.set_producer(&[(cell, Fixed::from_int(1000))]);
            e.set_producer_source(&[(cell, ids)]);
            e.step(&temp, &calib); // sets the pre-cap capacity from the producer biomass
            let mut soil = SoilNutrientField::new();
            soil.deposit(cell, "nutrient", scarce);
            e.extract_producers(&mut soil, &reg);
            e.capacity_at(cell.x, cell.y)
        };
        let flux_only = run(vec![0]);
        let soil_only = run(vec![1]);
        let flux_then_soil = run(vec![0, 1]);
        let soil_then_flux = run(vec![1, 0]);
        assert!(
            flux_only > soil_only,
            "the abundant flux alone supports more biomass than the scarce soil alone"
        );
        assert_eq!(
            flux_then_soil, soil_only,
            "drawing on both, the SCARCE soil binds the productivity (the Liebig minimum, not the flux)"
        );
        assert_eq!(
            soil_then_flux, flux_then_soil,
            "the minimum is over the whole source set: order of the sources does not change the cap"
        );
        assert!(
            flux_then_soil < flux_only,
            "adding a scarcer source can only lower the ceiling, never raise it above the flux-only cap"
        );
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
