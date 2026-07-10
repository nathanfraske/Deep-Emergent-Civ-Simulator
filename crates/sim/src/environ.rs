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
            // The saturation tangent's reference temperature DERIVES from the world's mean surface
            // temperature (the habitable-band midpoint the tangent is most accurate around), the same
            // absolute-temperature value the temperature field is centred on (Field::from_map_absolute),
            // rather than a duplicate reserved scalar (derive-vs-author, Principle 6; the retired
            // hydrology.saturation_t_ref duplicated it). The tangent's slope and value at t_ref stay the
            // owner's reserved calibration, now anchored at the world's own mean temperature.
            sat_t_ref: m.require_fixed("climate.mean_surface_temperature")?,
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
    /// The standing-food COMPOSITION of the producer on each cell (chemistry arc, T3; magnitudes CORRECTED-T3):
    /// the fixed per-unit-biomass axis vector a producer's food carries at its REAL magnitudes (no longer a
    /// sum-to-one simplex, so an energy-dense plant feeds more than a woody one), seeded once from the
    /// biosphere ([`crate::genesis::WorldGenesis::producer_compositions`]). `None` where
    /// no producer composition is seeded, in which case the standing food is the single `bio.energy_density`
    /// class exactly as before (byte-identical). Where `Some`, `regrow_supply` writes each food axis's supply
    /// as the logistic biomass VOLUME times that axis's density, and reads the remaining volume back as the
    /// Liebig MINIMUM over the axes of `supply / density`, so a grazer that ate one axis shrinks the whole
    /// plant and the composition stays a single scalar stock (never N independent stocks). Not folded into
    /// `state_hash`: its effect enters through the food supplies it shapes, which the [`ResourceField`] folds.
    producer_food: Vec<Option<BTreeMap<String, Fixed>>>,
    /// The WORLD-DECLARED located scalar energy fields keyed by field-kind id (Arc 5, the data-defined field
    /// set): the intrinsic per-cell scalar an alien source reads through [`AbioticField::DataScalar`] (a
    /// chemosynthetic redox potential, a geothermal flux, a mana field). Each is a [`ScalarField`] on the same
    /// grid, read (and, if the binding depletes, drawn down) through the SAME point mechanism as water, so a new
    /// energy field is a data row plus a binding, never a code change (Principle 8, 11). EMPTY on every Terran
    /// world (which uses only the intrinsic light/water/soil backings), so both its extract path and its hash
    /// fold are byte-identical to the pre-Arc-5 run. Folded into `state_hash` in id order after salt: an empty
    /// collection folds NOTHING (byte-neutral), while a world that declares and depletes an alien field folds it,
    /// so divergence in it is caught exactly as water and salt are.
    data_fields: BTreeMap<u16, ScalarField>,
}

/// The VALUE BACKING of an abiotic source: which located field its available energy is read from (decoupled
/// from the READ-SHAPE, [`ReadShape`], that says HOW the draw is derived, and from whether it DEPLETES,
/// [`AbioticBinding::depletes`], so a renewable light-flux and a finite water-stock are the SAME mechanism with
/// different data, FINDING-1). The three named members are the engine's INTRINSIC floor subsystems, each a
/// per-cell located quantity stepped by a fixed physics stencil (light, hydrology, soil nutrient); they are a
/// bounded engine set, documented implemented-not-exhaustive (an environment-owned nodal or graph-keyed backing
/// would be a bounded-Rust addition), never asserted closed. [`AbioticField::DataScalar`] is the OPEN arm that
/// realizes Arc 5's data-defined field set (Principle 8, 11): a world-declared located scalar (a chemosynthetic,
/// geothermal, or mana energy field) is a `ScalarField` row in [`EnvironFields::data_fields`] plus a binding
/// naming its id, never a new enum variant, so the alien is a DATA ROW rather than a rewrite of the dispatch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AbioticField {
    /// The LIGHT field (the latitude-light rate). Renewable by default (a world sets `depletes` if it models
    /// a consumable light budget).
    Light,
    /// The WATER field (the hydrology depth).
    Water,
    /// The located SOIL-NUTRIENT field, read and drawn by class.
    Soil,
    /// A WORLD-DECLARED located scalar field (Arc 5): the intrinsic per-cell scalar the world adds to
    /// [`EnvironFields::data_fields`] under this id, read and (if it depletes) drawn down through the SAME point
    /// mechanism as water. The OPEN membership: a chemosynthetic, geothermal, or mana source is this data row
    /// plus a binding, no new enum variant (Principle 8, 11). A source whose available draw is a between-quantity
    /// DIFFERENCE (a redox donor/acceptor) or a between-cell GRADIENT still reads scalar field(s), not a new
    /// backing kind, but through a different [`ReadShape`] operator; the difference case ALSO needs the binding
    /// to name more than one field (an ordered field/role list), a binding-arity extension the later segment
    /// adds alongside the `ReadShape::Difference` variant. Segment 1 provisions the single-field point read.
    DataScalar(u16),
}

/// The SPATIAL READ-SHAPE of an abiotic source (Arc 5): HOW its available draw is derived from the located
/// field(s) its binding names, decoupled from the value BACKING ([`AbioticField`]). The section-10 blind panel
/// identified this as the real generalization axis: the pre-Arc-5 dispatch read the value AT the producer's own
/// cell and thereby AUTHORED point-locality as the definition of a supply, foreclosing a source whose energy is
/// a between-quantity difference (a redox chemolithotroph draws on a donor/acceptor potential DIFFERENCE, not a
/// value at one cell) or a between-cell gradient. Here a point read is ONE operator among a data-selected set,
/// never the definition of a supply. Each variant is a fixed-Rust physics operator (the mechanism, the same
/// class the field stencils already use); which one a source uses is data (Principle 11). Segment 1 implements
/// `Point` only (the byte-neutral Earth read); the difference and gradient operators are the following segments.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ReadShape {
    /// The value AT the producer's own cell (the point read of the backing). The Earth default, byte-identical
    /// to the pre-Arc-5 match.
    #[default]
    Point,
}

/// A redox source's yield DERIVATION from the floor (Arc 5 segment 3, the depth-independent CORE): the biomass a
/// chemolithotroph fixes per unit stock DERIVES from the galvanic EMF of its electron-donor/electron-acceptor
/// couple, rather than a world-declared raw conversion. The two STANDARD POTENTIALS are floor data (the substances'
/// own `chem.standard_potential`, the authored physics floor, P9-legal); the EMF is the floor law
/// [`civsim_physics::laws::battery_emf`] (`E_acceptor - E_donor`, the galvanic cell EMF), clamped at zero AT
/// STANDARD CONCENTRATIONS (a couple whose STANDARD EMF is non-positive releases no free energy and powers no life
/// in the standard state); the yield is that EMF times the RESERVED [`AbioticSourceRegistry::emf_to_biomass`]
/// coupling. So a redox source's conversion is neither the soil-derived global nor a fabricated per-source number:
/// it is DERIVED from the floor and the couple (Principle 6, 11), and any redox chemistry (Terran vent
/// sulfide/oxygen, an alien couple) is a data row of two potentials.
///
/// DEPTH EXTENSION, FLAGGED not wired (the owner's standard-EMF-versus-Nernst ruling): this CORE uses the STANDARD
/// EMF, the base BOTH depths share. Full-Nernst adds a concentration adjustment to the EMF, the reaction quotient
/// `Q` formed from the donor/acceptor `DataScalar` field CONCENTRATIONS times the thermal factor `RT/nF`, a clean
/// parameterized add-on around this same core (a `nernst`-adjusted EMF replacing the standard EMF in the yield),
/// not built until the owner rules the depth. HONEST LIMIT of the standard-EMF core: the zero-clamp reads
/// spontaneity at the STANDARD state, so a couple that is standard-non-spontaneous but concentration-driven
/// spontaneous (or the reverse) is judged only by its standard EMF until the Nernst extension re-evaluates it at
/// the actual field concentrations; and the reserved coupling folds the per-couple electron count `n` (from
/// `dG = -n*F*EMF`) into one registry-global, so two couples differing only in `n` share a yield-per-volt until a
/// per-couple `n` (a couple-data refinement) is added. Both are the owner's yield-model-depth call, surfaced.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RedoxEmf {
    /// The electron DONOR's (reductant's) standard reduction potential (floor `chem.standard_potential`).
    pub donor_potential: Fixed,
    /// The electron ACCEPTOR's (oxidant's) standard reduction potential (floor `chem.standard_potential`).
    pub acceptor_potential: Fixed,
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
    /// The value backing this source reads (which located field, decoupled from the depletion behaviour below
    /// and from the read-shape). See [`AbioticField`].
    pub field: AbioticField,
    /// HOW the available draw is derived from the backing (Arc 5): a point read of the value at the producer's
    /// cell, or (later segments) a between-quantity difference or a between-cell gradient. Decoupled from the
    /// backing so a source's spatial shape is the world's data, not baked into the read. Defaults to
    /// [`ReadShape::Point`], the byte-neutral Earth read. See [`ReadShape`].
    pub read_shape: ReadShape,
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
    /// PER-SOURCE stock-to-biomass CONVERSION (Arc 5 segment 2): the biomass a unit of THIS field's stock
    /// supports, the cap conversion the extract beat applies in Pass 1. `None` falls back to the registry-global
    /// [`AbioticSourceRegistry::biomass_per_stock`], so a source that does not set it keeps the old behaviour
    /// (byte-neutral). Decoupled per-source because a joule of redox free energy and a mole of soil nutrient are
    /// dimensionally incommensurable: one global (whose basis is a SOIL fertility scale) cannot honestly convert
    /// both (the section-10 panel finding). RESERVED per source, surfaced not fabricated: a world sets it, or for
    /// a redox source it DERIVES from the floor `law.battery_emf` yield (segment 3, gated on the owner's
    /// EMF-to-biomass coupling); the basis is that field's own floor units. HONEST LIMIT (opt-out, not yet
    /// closed): an ALIEN `DataScalar` source that OMITS its conversion silently borrows the soil-derived global,
    /// re-applying the incommensurable Terran scale it is meant to escape. The seam is closed only when the source
    /// declares its own conversion (here) or segment 3 derives it from the floor; a future segment may REQUIRE an
    /// alien source to declare or derive rather than defaulting to soil, which stays byte-neutral (no Earth source
    /// is a `DataScalar`).
    pub biomass_per_stock: Option<Fixed>,
    /// PER-SOURCE STOICHIOMETRIC drawdown (Arc 5 segment 2): the units of THIS field's stock CONSUMED per unit
    /// of supported biomass, the amount Pass 2 draws down. `None` falls back to the reciprocal of the effective
    /// conversion (`draw_biomass / biomass_per_stock`), today's implicit 1:1 behaviour (byte-neutral). Decoupled
    /// per-source because a real redox reaction consumes its reactants in a reaction-specific ratio, not 1:1
    /// (the section-10 panel finding): a producer closing on a donor AND an acceptor (each its own source id,
    /// Liebig-min co-limited by segment 1) draws each down by its OWN coefficient. RESERVED per source with the
    /// same basis as the conversion; `Some(0)` models a catalyst (enabling but not consumed).
    pub stock_per_biomass: Option<Fixed>,
    /// A redox source's yield DERIVATION from the floor (Arc 5 segment 3): when `Some`, this source's stock-to-
    /// biomass conversion is DERIVED from the couple's galvanic EMF times the reserved coupling (see [`RedoxEmf`]),
    /// taking precedence over the per-source [`Self::biomass_per_stock`] and the registry-global, so a redox
    /// source's power is neither soil-borrowed nor fabricated but read from the floor. `None` (every Terran source)
    /// keeps the segment-2 behaviour, so the run is byte-identical.
    pub redox_emf: Option<RedoxEmf>,
}

/// The abiotic-source binding registry (Principle 11): the extract mechanism is fixed Rust; the membership
/// (which evolved source id binds to which field and class) is DATA, so the run path never authors a closed
/// Earth source enum. Carries the reserved extract-deplete conversions, surfaced with basis, never set here.
#[derive(Clone, Debug, Default)]
pub struct AbioticSourceRegistry {
    bindings: BTreeMap<u16, AbioticBinding>,
    /// RESERVED (surfaced, not set): the standing biomass a unit of drawn located stock supports. Basis: the
    /// reciprocal of the soil `fertility_scale`, so biomass fixed reconciles with the mass decay returns. This is
    /// the FALLBACK default (Arc 5 segment 2): a source that declares its own [`AbioticBinding::biomass_per_stock`]
    /// converts on its own terms; only a source with `None` borrows this global, so a Terran world is byte-neutral.
    pub biomass_per_stock: Fixed,
    /// RESERVED: the fraction of the supported biomass a producer sequesters from its stock per tick. Basis:
    /// the nutrient turnover the standing biomass holds; set so a grazed cell depletes over a plausible span.
    /// HONEST LIMIT (Arc 5 segment 2 scope): this per-tick TURNOVER fraction stays registry-global even after the
    /// stock conversion and the stoichiometric drawdown became per-source, so two heterogeneous fields on one
    /// world share one turnover rate; a per-source turnover is a later refinement, not built.
    pub draw_fraction: Fixed,
    /// RESERVED: the located stock a physical WEATHERING source (rock to nutrient) deposits per producer cell
    /// per tick, the bootstrap that seeds a bare soil before any corpse decomposes. Basis: the rock-weathering
    /// release rate the material data implies; set so a virgin world greens slowly.
    pub weathering_rate: Fixed,
    /// RESERVED (Arc 5 segment 3, the EMF-to-biomass coupling, surfaced not fabricated, the OWNER's value): the
    /// biomass a redox source supports per unit of galvanic EMF (volt) of its couple, the conversion from the
    /// floor free-energy the couple releases to the biomass it fixes. Basis: a thermodynamic-efficiency bound
    /// (`dG = -n * F * EMF`, so biomass-per-volt folds the electrons transferred `n`, the Faraday constant `F`,
    /// and the fraction of free energy captured as biomass). Defaults to a fail-loud sentinel (zero): a redox
    /// source's derived conversion REFUSES to run while it is unset (asserting, never silently starving), exactly
    /// the sentinel discipline `biomass_per_stock` follows. A world with no redox source never reads it, so it
    /// stays reserved and every Terran run is byte-identical.
    pub emf_to_biomass: Fixed,
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
        self.insert_shaped(id, field, ReadShape::Point, depletes, class, availability);
    }

    /// Bind a source id with an explicit READ-SHAPE (Arc 5): the general inserter [`Self::insert`] and
    /// [`Self::insert_available`] delegate here with [`ReadShape::Point`] (the byte-neutral Earth read); a
    /// source whose available draw is a between-quantity difference or a between-cell gradient passes its own
    /// shape, so the spatial operator is the world's data (Principle 11), never baked into the extract dispatch.
    pub fn insert_shaped(
        &mut self,
        id: u16,
        field: AbioticField,
        read_shape: ReadShape,
        depletes: bool,
        class: &str,
        availability: AbioticAvailability,
    ) {
        self.bindings.insert(
            id,
            AbioticBinding {
                field,
                read_shape,
                depletes,
                class: class.to_string(),
                availability,
                biomass_per_stock: None,
                stock_per_biomass: None,
                redox_emf: None,
            },
        );
    }

    /// Set the PER-SOURCE stock-to-biomass conversion and/or stoichiometric drawdown of an already-bound source
    /// (Arc 5 segment 2). `None` on either leaves that dimension at the registry-global fallback (byte-neutral).
    /// This is how a world declares that a mana or redox field converts and depletes on its OWN terms rather than
    /// borrowing the soil-derived global: a joule of redox free energy and a mole of soil nutrient are
    /// dimensionally incommensurable (Principle 11). A no-op if the id is unbound. FAILS LOUD at config time on a
    /// per-source conversion of zero (a field that supports zero biomass per unit stock is a config error that
    /// would silently starve the producer, exactly the sentinel the global assert guards; `stock_per_biomass` of
    /// zero is NOT an error, it is the legitimate catalyst case, enabling but not consumed).
    pub fn set_source_conversion(
        &mut self,
        id: u16,
        biomass_per_stock: Option<Fixed>,
        stock_per_biomass: Option<Fixed>,
    ) {
        if let Some(v) = biomass_per_stock {
            assert!(
                v > Fixed::ZERO,
                "abiotic source {id}: a per-source biomass_per_stock must be positive (zero would silently \
                 starve the producer); leave it None to use the registry-global fallback"
            );
        }
        if let Some(b) = self.bindings.get_mut(&id) {
            b.biomass_per_stock = biomass_per_stock;
            b.stock_per_biomass = stock_per_biomass;
        }
    }

    /// Declare a source's yield as a REDOX DERIVATION from the floor (Arc 5 segment 3): its stock-to-biomass
    /// conversion is computed from the couple's galvanic EMF (the two floor standard potentials) times the reserved
    /// [`Self::emf_to_biomass`] coupling, rather than a declared raw number (see [`RedoxEmf`]). A no-op if the id is
    /// unbound. The couple's potentials are floor data; the coupling is the owner's reserved value, so the yield is
    /// derived, never fabricated.
    pub fn set_source_redox(&mut self, id: u16, donor_potential: Fixed, acceptor_potential: Fixed) {
        if let Some(b) = self.bindings.get_mut(&id) {
            b.redox_emf = Some(RedoxEmf {
                donor_potential,
                acceptor_potential,
            });
        }
    }

    /// The stock-to-biomass conversion a binding uses, resolving the Arc-5 precedence (segment 3 over 2 over the
    /// global): a REDOX source DERIVES it from the couple's galvanic EMF ([`civsim_physics::laws::battery_emf`],
    /// `E_acceptor - E_donor`, clamped at zero so a non-spontaneous couple powers no life) times the RESERVED
    /// `emf_to_biomass` coupling, failing loud if that coupling is unset (the sentinel discipline, never silently
    /// starving); otherwise the per-source [`AbioticBinding::biomass_per_stock`] if set, else the registry-global.
    /// Byte-neutral for Earth (no redox source, no per-source override, so the global is returned unchanged).
    fn effective_conversion(&self, binding: &AbioticBinding) -> Fixed {
        if let Some(rx) = &binding.redox_emf {
            assert!(
                self.emf_to_biomass > Fixed::ZERO,
                "redox source: emf_to_biomass coupling reserved value is unset (would starve every redox producer)"
            );
            let emf = laws::battery_emf(rx.acceptor_potential, rx.donor_potential).max(Fixed::ZERO);
            return emf.checked_mul(self.emf_to_biomass).unwrap_or(Fixed::MAX);
        }
        binding.biomass_per_stock.unwrap_or(self.biomass_per_stock)
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
                elevation.push(tile.elevation());
                moisture.push(tile.moisture());
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
            data_fields: BTreeMap::new(),
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
    /// world build), carrying the given per-unit-biomass axis vector at its REAL per-axis MAGNITUDES (CORRECTED-T3,
    /// owner-ruled): the food is NOT normalised to a sum-to-one simplex, so a plant's food value reflects how much
    /// its own substances actually carry (an energy-dense fruit feeds more per unit biomass than a woody stem),
    /// rather than every plant reading equally nutritious. A blind panel (verified against source) caught that the
    /// prior simplex authored a flat 1/total nutrition, and that collapsing instead to a plant-side gross
    /// energy-density scalar would author digestibility = 1 for every consumer: so the FULL de-normalised VECTOR is
    /// carried, and the per-consumer usable value EMERGES downstream in [`crate::physiology::physical_intake`],
    /// which folds this real content against the CONSUMER's own assimilation and trophic efficiency through the
    /// SAME `bio.energy_density` reserve bridge (keyed on no axis identity, so a mana-fed or silicon being is a
    /// data row). No fresh UNITS anchor is minted: the existing reserve bridge converts the magnitude, so energy is
    /// conserved across the eat step. This is the food-value MECHANISM (the units question), distinct from the
    /// biosphere-BALANCE calibration the `worldbuild.rs` T3 owner-gate still holds (whether the grazers THRIVE on
    /// these real food values, which the balance work tunes; this foundation makes the value real, it does not
    /// claim the world thrives). The environmental axes regrow writes itself (water, salinity) are EXCLUDED,
    /// so the food never fights the hydrology write. A composition with no positive food axis seeds nothing (the
    /// cell keeps the single energy-density default, byte-identical). Off-grid cells dropped; last occupant wins on
    /// a shared cell. See [`Self::producer_food`].
    pub fn set_producer_food(&mut self, cells: &[(Coord3, BTreeMap<String, Fixed>)]) {
        for f in self.producer_food.iter_mut() {
            *f = None;
        }
        for (cell, comp) in cells {
            if cell.x < 0 || cell.y < 0 || cell.x >= self.width || cell.y >= self.height {
                continue;
            }
            let is_food = |a: &str| a != WATER_FRACTION && a != SALINITY;
            // Carry the real per-axis magnitudes (no division by the total): the plant's own chemistry at full
            // scale, so its food value is what it is materially made of, not a flattened ratio.
            let food_vec: BTreeMap<String, Fixed> = comp
                .iter()
                .filter(|(a, v)| is_food(a) && **v > Fixed::ZERO)
                .map(|(a, v)| (a.clone(), *v))
                .collect();
            if food_vec.is_empty() {
                continue; // no positive food axis: keep the energy-density default
            }
            let i = self.idx(cell.x, cell.y);
            self.producer_food[i] = Some(food_vec);
        }
    }

    /// Declare (or replace) a world's located SCALAR energy field under a field-kind id (Arc 5, the data-defined
    /// field set): the data-row path an [`AbioticField::DataScalar`] binding reads. An alien source (a
    /// chemosynthetic redox potential, a geothermal flux, a mana field) is this call plus a binding, never a new
    /// enum variant (Principle 8, 11). The field must match the environ extent. Seeded at world build (or stepped
    /// by the world's own stencil); read and depleted through the same point mechanism as water. Idempotent on
    /// re-seed. A Terran world calls this never, so its `data_fields` stays empty and its run is byte-identical.
    pub fn set_data_field(&mut self, id: u16, field: ScalarField) {
        assert_eq!(
            field.dims(),
            (self.width, self.height),
            "a data energy field matches the environ extent"
        );
        self.data_fields.insert(id, field);
    }

    /// The value of a world-declared located energy field at a cell (Arc 5), or `None` if the world declared no
    /// field under this id. A pure read for the field reader and tests; mutates nothing.
    pub fn data_field_at(&self, id: u16, x: i32, y: i32) -> Option<Fixed> {
        self.data_fields.get(&id).map(|f| f.at(x, y))
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
                    // The available draw of this source at the cell, derived from its backing under its
                    // READ-SHAPE (Arc 5). Segment 1 implements the POINT read (the value at the producer's own
                    // cell); the difference and gradient operators are the following segments. For the Earth
                    // backings under Point this is byte-identical to the pre-Arc-5 match.
                    let supply = match binding.read_shape {
                        ReadShape::Point => match binding.field {
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
                            // A world-declared located scalar (mana, geothermal, a redox potential): read its
                            // per-cell value. A binding to a field the world never declared is a config error:
                            // fail loud (matching the unbound-source-id panic) rather than silently starve.
                            AbioticField::DataScalar(fid) => self
                                .data_fields
                                .get(&fid)
                                .unwrap_or_else(|| {
                                    panic!(
                                        "nutrient cycle: producer source binds DataScalar field {fid} but the world declared no such field"
                                    )
                                })
                                .at(x, y),
                        },
                    };
                    // The stock-to-biomass conversion resolves the Arc-5 precedence (segment 3 redox derivation
                    // over the segment-2 per-source value over the registry-global; byte-neutral for Earth, which
                    // declares none of the first two): a redox source DERIVES its yield from the couple's floor
                    // EMF, a per-source override converts on its own terms, else the soil-derived global.
                    let bps = registry.effective_conversion(binding);
                    let supported = supply.checked_mul(bps).unwrap_or(Fixed::MAX);
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
                    // The stock DRAWN per unit biomass is per-source STOICHIOMETRY (Arc 5 segment 2): a redox
                    // reaction consumes its reactants in a reaction-specific ratio, not 1:1, so a producer
                    // closing on a donor and an acceptor (each its own source id, Liebig-min co-limited) draws
                    // each by its OWN coefficient. `None` falls back to the reciprocal of this source's effective
                    // conversion (draw_biomass / bps, the same segment-3-or-2-or-global conversion Pass 1 capped
                    // by), today's implicit 1:1 draw (byte-identical for Earth).
                    let bps = registry.effective_conversion(binding);
                    let draw_amt = match binding.stock_per_biomass {
                        Some(spb) => draw_biomass.checked_mul(spb).unwrap_or(Fixed::ZERO),
                        None => draw_biomass.checked_div(bps).unwrap_or(Fixed::ZERO),
                    };
                    match binding.field {
                        AbioticField::Light => {} // light has no located stock to deplete
                        AbioticField::Soil => {
                            soil.take(coord, &binding.class, draw_amt);
                        }
                        AbioticField::Water => {
                            self.water.take(x, y, draw_amt);
                        }
                        // A world-declared located scalar depletes through the same point draw as water. An
                        // undeclared field FAILS LOUD here exactly as the Pass-1 read does (Pass 1 has already
                        // panicked on it, so this is unreachable, but the symmetry removes the asymmetric silent
                        // no-op the audit flagged: if the two passes ever decouple, a missing draw fails loud
                        // rather than silently starving the ledger). A renewable source never enters this loop
                        // (guarded by `depletes` above).
                        AbioticField::DataScalar(fid) => {
                            self.data_fields
                                .get_mut(&fid)
                                .unwrap_or_else(|| {
                                    panic!(
                                        "nutrient cycle: producer source binds DataScalar field {fid} but the world declared no such field"
                                    )
                                })
                                .take(x, y, draw_amt);
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
                // CORRECTED-T3: mark whether this cell's food is a real producer composition (its supply is
                // already the physical energy content at the plant's own bio.energy_density), so the forage
                // INGEST eats it at `content = supply` rather than double-scaling through the food_energy_density
                // anchor. A composition-less cell stays unmarked and keeps the anchor bridge (byte-identical for
                // a run that seeds no producer food, so the four tracked pins hold).
                resource.set_real_composition(coord, food.is_some());
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
        // The world-declared located energy fields (Arc 5), folded in id order (BTreeMap iterates sorted) after
        // salt. An EMPTY collection (every Terran world) folds nothing, so the Earth hash is byte-identical; a
        // world that declares a data field folds it, so divergence in a depleted alien stock is caught exactly
        // as water and salt are. No count prefix is written, so an empty collection contributes zero bytes.
        for f in self.data_fields.values() {
            f.hash_into(h);
        }
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
            data_fields: BTreeMap::new(),
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
    fn a_more_energy_dense_plant_carries_more_food_value_corrected_t3() {
        // CORRECTED-T3 (owner-ruled, blind-panel-verified): the standing food carries the plant's REAL
        // composition magnitude, not a sum-to-one simplex, so an energy-dense plant feeds more per unit biomass
        // than a poor one. Two cells with the SAME producer biomass (hence the same capacity and regrown volume)
        // are seeded with single-axis energy-density compositions of DIFFERENT magnitude (2.0 vs 0.5); the food
        // supply tracks the magnitude ratio (4:1), where the old simplex would have flattened BOTH to 1.0 (every
        // plant equally nutritious). The per-consumer assimilation still emerges later in physical_intake.
        let map = a_map(0xF00D53);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let rich = Coord3::ground(1, 1);
        let poor = Coord3::ground(2, 1);
        let mut e = EnvironFields::from_map(&map);
        // The same biomass at both cells, so their capacity (and thus regrown volume) is identical.
        e.set_producer(&[(rich, Fixed::from_int(1)), (poor, Fixed::from_int(1))]);
        // Single-axis energy-density foods of different magnitude. Under the old simplex both would normalise to
        // {energy_density: 1.0} and read identically; de-normalised they keep 2.0 and 0.5.
        let rich_comp: BTreeMap<String, Fixed> = [(ENERGY_DENSITY.to_string(), Fixed::from_int(2))]
            .into_iter()
            .collect();
        let poor_comp: BTreeMap<String, Fixed> =
            [(ENERGY_DENSITY.to_string(), Fixed::from_ratio(1, 2))]
                .into_iter()
                .collect();
        e.set_producer_food(&[(rich, rich_comp), (poor, poor_comp)]);
        let mut resource = ResourceField::new();
        for _ in 0..30 {
            e.step(&temp, &calib);
            e.regrow_supply(&mut resource, &calib);
        }
        let rich_food = resource.supply(rich, ENERGY_DENSITY);
        let poor_food = resource.supply(poor, ENERGY_DENSITY);
        assert!(
            rich_food > Fixed::ZERO && poor_food > Fixed::ZERO,
            "both plants carry some food"
        );
        // The KEY proof: the energy-dense plant feeds STRICTLY MORE. Under the old simplex both single-axis foods
        // normalised to {energy_density: 1.0} and these would be EQUAL (the flat nutrition the fix removes); with
        // the real magnitudes carried, rich (2.0) clearly outfeeds poor (0.5). Both cells share one capacity
        // (cap 1, asserted below), so the difference is the composition magnitude, nothing else.
        assert_eq!(
            e.capacity_at(rich.x, rich.y),
            e.capacity_at(poor.x, poor.y),
            "the two cells share one productivity capacity, so only the food magnitude can differ"
        );
        assert!(
            rich_food > poor_food.checked_mul(Fixed::from_int(3)).unwrap(),
            "the energy-dense plant feeds far more (about 4x) than the poor one: the real magnitude is preserved, \
             where the old simplex would have made them EQUAL. rich={rich_food:?} poor={poor_food:?}"
        );
        // And the ratio tracks the 4:1 density ratio to within fixed-point rounding (the logistic regrowth over
        // many ticks accrues a sub-part-per-billion drift, so this is a bounded near-equality, not exact).
        let four_poor = poor_food.checked_mul(Fixed::from_int(4)).unwrap();
        let drift = (rich_food - four_poor).abs();
        assert!(
            drift < Fixed::from_ratio(1, 1_000_000),
            "the food tracks the 4:1 magnitude ratio within fixed-point rounding: rich={rich_food:?} \
             4*poor={four_poor:?} drift={drift:?}"
        );
        // END TO END: the differentiated supply reaches the CONSUMER's reserve differentiated. Feed each food
        // to the SAME consumer (identical assimilation, trophic efficiency, body mass, storage density, and
        // ample room) through the real `physiology::physical_intake` fold: the energy-dense plant banks strictly
        // more reserve, so the magnitude is not re-flattened between the environ write and the being's reserve.
        let one = Fixed::from_int(1);
        let room = Fixed::from_int(1000); // ample, so intake is bounded by the food, not the reserve room
        let (_, rich_gain) =
            crate::physiology::physical_intake(rich_food, one, one, one, one, room);
        let (_, poor_gain) =
            crate::physiology::physical_intake(poor_food, one, one, one, one, room);
        assert!(
            rich_gain > poor_gain,
            "the energy-dense plant fills the consumer's reserve MORE through physical_intake (the magnitude \
             survives to the being, not re-flattened): rich_gain={rich_gain:?} poor_gain={poor_gain:?}"
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
    fn a_world_declared_energy_field_is_a_pure_data_row_that_caps_and_depletes() {
        // Arc 5 segment 1 (the OPEN arm): a producer draws energy from a WORLD-DECLARED located scalar field
        // (AbioticField::DataScalar), the alien-as-data proof. No light, water, or soil is involved; the source
        // is a `ScalarField` the world seeded plus a binding naming its id, zero new Rust. Two claims: (1) the
        // declared field's per-cell value CAPS the producer's productivity through the same Liebig math (point
        // read-shape); (2) a depleting binding DRAWS THE FIELD DOWN, so a heavily-worked cell exhausts its alien
        // stock exactly as it would water. This is a chemosynthetic/geothermal/mana energy source as a data row.
        let map = a_map(0x0A11E7);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let field_id: u16 = 7; // an opaque field-kind id, not one of the Earth backings
        let source_id: u16 = 42; // the evolved source id the producer closes on

        // The registry: one source (id 42) bound to the world's declared scalar field (id 7), depleting. No
        // light/water/soil binding exists at all, so nothing Earth-shaped can leak into the result.
        let mut reg = AbioticSourceRegistry::default();
        reg.insert(source_id, AbioticField::DataScalar(field_id), true, "");
        reg.biomass_per_stock = Fixed::from_int(4);
        reg.draw_fraction = Fixed::from_ratio(1, 2); // a heavy draw so depletion is visible in one beat
        reg.weathering_rate = Fixed::ZERO; // no soil bootstrap; the alien field is the only supply

        // A helper: seed a producer with a large biomass (so the pre-cap capacity is high and the alien field is
        // what binds), declare the energy field at the given uniform level, run the extract beat once, and
        // return (capped capacity, remaining field value) at the cell.
        let run = |field_level: Fixed| -> (Fixed, Fixed) {
            let mut e = EnvironFields::from_map(&map);
            let (w, h) = e.dims();
            e.set_producer(&[(cell, Fixed::from_int(1000))]);
            e.set_producer_source(&[(cell, vec![source_id])]);
            e.set_data_field(field_id, ScalarField::uniform(w, h, field_level));
            e.step(&temp, &calib); // sets the pre-cap capacity from the producer biomass
            let mut soil = SoilNutrientField::new();
            e.extract_producers(&mut soil, &reg);
            (
                e.capacity_at(cell.x, cell.y),
                e.data_field_at(field_id, cell.x, cell.y).unwrap(),
            )
        };

        let scarce = Fixed::from_ratio(1, 10);
        let abundant = Fixed::from_int(2);
        let (cap_scarce, remain_scarce) = run(scarce);
        let (cap_abundant, _remain_abundant) = run(abundant);

        // (1) The declared field caps the productivity: a scarcer field supports less biomass, exactly as a
        // scarce water or soil stock would, through the same `supply * biomass_per_stock` Liebig math.
        assert!(
            cap_scarce > Fixed::ZERO,
            "the alien field supports some biomass"
        );
        assert_eq!(
            cap_scarce,
            scarce.checked_mul(reg.biomass_per_stock).unwrap(),
            "the world-declared field's value caps the productivity (point read-shape, Liebig math)"
        );
        assert!(
            cap_abundant > cap_scarce,
            "more of the declared field supports more biomass: {cap_abundant:?} > {cap_scarce:?}"
        );
        // (2) The depleting binding drew the alien field DOWN at the worked cell (it started at `scarce`).
        assert!(
            remain_scarce < scarce,
            "a depleting source draws the world-declared field down, so a worked cell exhausts its alien \
             stock: {remain_scarce:?} < {scarce:?}"
        );
    }

    #[test]
    fn an_empty_data_field_collection_folds_nothing_so_the_earth_hash_is_unchanged() {
        // Byte-neutrality guard for Arc 5 segment 1: the data_fields collection folds into `state_hash` only its
        // members, with no count prefix, so an EMPTY collection (every Terran world) contributes zero bytes and
        // the hash is identical to a stack that carries no such field at all. Declaring one field then changes
        // the hash (the fold is real), proving the empty case is a true no-op rather than an accidental omission.
        let map = a_map(0xE0F0);
        let calib = EnvironCalib::dev_fixture();
        let temp = Field::from_map(&map);
        let mut e = EnvironFields::from_map(&map);
        for _ in 0..8 {
            e.step(&temp, &calib);
        }
        // (1) The DIRECT empty-neutrality proof: the full `hash_into` (which folds the empty data_fields) equals a
        // manual fold of ONLY the pre-Arc-5 dynamic fields (water, capacity, salt) in the same order. So the
        // empty collection contributed ZERO bytes, and the Earth hash is byte-identical to the pre-change fold,
        // not merely "the fold is observable". (The four run_world pins prove this end-to-end; this pins it here.)
        let full = {
            let mut h = StateHasher::new();
            e.hash_into(&mut h);
            h.finish()
        };
        let pre_arc5 = {
            let mut h = StateHasher::new();
            e.water.hash_into(&mut h);
            e.capacity.hash_into(&mut h);
            e.salt.hash_into(&mut h);
            h.finish()
        };
        assert_eq!(
            full, pre_arc5,
            "an empty data_fields collection folds NOTHING: the Arc-5 hash equals the pre-Arc-5 water+capacity+salt \
             fold byte for byte (no count prefix, zero bytes for the empty case)"
        );
        // (2) And the fold is REAL: declaring one field changes the hash, so the empty no-op is a guarantee, not
        // an omission that would silently hide divergence in a depleted alien stock.
        let with_field = {
            let mut e2 = EnvironFields::from_map(&map);
            let (w, h_) = e2.dims();
            e2.set_data_field(3, ScalarField::uniform(w, h_, Fixed::from_int(1)));
            for _ in 0..8 {
                e2.step(&temp, &calib);
            }
            let mut h = StateHasher::new();
            e2.hash_into(&mut h);
            h.finish()
        };
        assert_ne!(
            full, with_field,
            "a declared data field folds into the hash (so a depleted alien stock cannot pass replay while \
             hiding divergence)"
        );
    }

    #[test]
    #[should_panic(expected = "declared no such field")]
    fn a_producer_bound_to_an_undeclared_energy_field_fails_loud() {
        // Arc 5 segment 1 fail-loud guarantee: a producer whose evolved source binds an AbioticField::DataScalar
        // the world never declared is a config error, and the extract read pass PANICS naming the field, rather
        // than silently reading zero and starving the producer (which would mask a broken world spec). Mirrors
        // the unbound-source-id panic. This exercises the guarantee the audit flagged as untested.
        let map = a_map(0xBAD1);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let mut reg = AbioticSourceRegistry::default();
        reg.insert(5, AbioticField::DataScalar(99), false, ""); // field 99 is never declared
        reg.biomass_per_stock = Fixed::from_int(4);
        reg.draw_fraction = Fixed::ZERO;
        reg.weathering_rate = Fixed::ZERO;
        let mut e = EnvironFields::from_map(&map);
        e.set_producer(&[(cell, Fixed::from_int(10))]);
        e.set_producer_source(&[(cell, vec![5])]);
        e.step(&temp, &calib);
        let mut soil = SoilNutrientField::new();
        e.extract_producers(&mut soil, &reg); // panics: field 99 was never declared
    }

    #[test]
    fn a_source_converts_stock_to_biomass_by_its_own_per_source_rate_not_the_global() {
        // Arc 5 segment 2 (per-source conversion): the stock-to-biomass conversion is PER-SOURCE, so a world
        // declares that its alien field converts on its own terms rather than borrowing the soil-derived global
        // (a joule of redox free energy and a mole of soil nutrient are incommensurable). Two runs on the SAME
        // field value differ only in the source's per-source biomass_per_stock, and the capped productivity
        // tracks the per-source rate, not the registry-global.
        let map = a_map(0xC0FFEE);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let field_id: u16 = 7;
        let source_id: u16 = 3;
        let field_level = Fixed::from_ratio(1, 4);

        let run = |per_source_bps: Option<Fixed>| -> Fixed {
            let mut reg = AbioticSourceRegistry::default();
            reg.insert(source_id, AbioticField::DataScalar(field_id), false, "");
            reg.biomass_per_stock = Fixed::from_int(4); // the registry-global (the fallback)
            reg.draw_fraction = Fixed::ZERO; // isolate the cap from the draw
            reg.weathering_rate = Fixed::ZERO;
            reg.set_source_conversion(source_id, per_source_bps, None);
            let mut e = EnvironFields::from_map(&map);
            let (w, h) = e.dims();
            e.set_producer(&[(cell, Fixed::from_int(1000))]);
            e.set_producer_source(&[(cell, vec![source_id])]);
            e.set_data_field(field_id, ScalarField::uniform(w, h, field_level));
            e.step(&temp, &calib);
            let mut soil = SoilNutrientField::new();
            e.extract_producers(&mut soil, &reg);
            e.capacity_at(cell.x, cell.y)
        };

        // None: the source uses the registry-global (4), so the cap is field_level * 4.
        let global_cap = run(None);
        assert_eq!(
            global_cap,
            field_level.checked_mul(Fixed::from_int(4)).unwrap(),
            "with no per-source rate the source falls back to the registry-global conversion (byte-neutral)"
        );
        // A per-source rate of 10 (its OWN floor units) caps at field_level * 10, NOT the global 4: the
        // conversion is per-source, so the incommensurable-global seam is closed.
        let per_source_cap = run(Some(Fixed::from_int(10)));
        assert_eq!(
            per_source_cap,
            field_level.checked_mul(Fixed::from_int(10)).unwrap(),
            "the source converts by its OWN per-source rate, not the soil-derived registry-global"
        );
        assert!(
            per_source_cap > global_cap,
            "the per-source rate overrides the global: {per_source_cap:?} > {global_cap:?}"
        );
    }

    #[test]
    fn the_draw_uses_the_per_source_conversion_not_the_global_when_no_stoichiometry_is_set() {
        // Arc 5 segment 2 (Pass-2 coverage the audit flagged): when a source sets a per-source conversion but NO
        // stoichiometric coefficient, the deplete draw falls back to the reciprocal of THAT source's conversion
        // (draw_biomass / per_source_bps), not the registry-global. Proven by drawing a field down and checking
        // the removed amount equals draw_biomass / per_source_bps, which differs from draw_biomass / global.
        let map = a_map(0xD3A);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let field_id: u16 = 8;
        let source_id: u16 = 4;
        let field_level = Fixed::from_ratio(1, 2);
        let per_source_bps = Fixed::from_int(10);
        let global_bps = Fixed::from_int(4);
        let draw_fraction = Fixed::from_ratio(1, 2);

        let mut reg = AbioticSourceRegistry::default();
        reg.insert(source_id, AbioticField::DataScalar(field_id), true, "");
        reg.biomass_per_stock = global_bps;
        reg.draw_fraction = draw_fraction;
        reg.weathering_rate = Fixed::ZERO;
        reg.set_source_conversion(source_id, Some(per_source_bps), None); // per-source bps, None stoich

        let mut e = EnvironFields::from_map(&map);
        let (w, h) = e.dims();
        e.set_producer(&[(cell, Fixed::from_int(1000))]);
        e.set_producer_source(&[(cell, vec![source_id])]);
        e.set_data_field(field_id, ScalarField::uniform(w, h, field_level));
        e.step(&temp, &calib);
        let mut soil = SoilNutrientField::new();
        e.extract_producers(&mut soil, &reg);

        let cap = e.capacity_at(cell.x, cell.y); // = field_level * per_source_bps
        let draw_biomass = cap.checked_mul(draw_fraction).unwrap();
        let drawn = field_level - e.data_field_at(field_id, cell.x, cell.y).unwrap();
        assert_eq!(
            drawn,
            draw_biomass.checked_div(per_source_bps).unwrap(),
            "the None-stoichiometry draw uses THIS source's per-source conversion (draw_biomass / per_source_bps)"
        );
        assert_ne!(
            drawn,
            draw_biomass.checked_div(global_bps).unwrap(),
            "the draw does NOT use the registry-global conversion (that was the untested Pass-2 branch)"
        );
    }

    #[test]
    #[should_panic(expected = "must be positive")]
    fn a_zero_per_source_conversion_fails_loud_rather_than_silently_starving() {
        // Arc 5 segment 2 fail-loud (the audit's asymmetry finding): a per-source biomass_per_stock of zero is a
        // config error that would silently cap every producer on it to zero biomass, so set_source_conversion
        // panics at config time, symmetric with the registry-global's own unset guard. (A zero stock_per_biomass
        // is NOT an error, it is the catalyst case, tested separately by its absence of a panic here.)
        let mut reg = AbioticSourceRegistry::default();
        reg.insert(1, AbioticField::DataScalar(5), true, "");
        reg.set_source_conversion(1, Some(Fixed::ZERO), None); // panics: zero conversion
    }

    #[test]
    fn a_redox_source_derives_its_yield_from_the_floor_emf_not_a_declared_number() {
        // Arc 5 segment 3 (the depth-independent floor-EMF core): a chemolithotroph's stock-to-biomass conversion
        // DERIVES from the galvanic EMF of its couple (the two floor standard potentials, through the floor law
        // battery_emf = E_acceptor - E_donor) times the reserved emf_to_biomass coupling, rather than a declared
        // raw number or the soil-derived global. Proof: the capped productivity equals supply * (EMF * coupling),
        // matching the floor law exactly, and differs from what the registry-global conversion would give.
        let map = a_map(0x5EDD00);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let field_id: u16 = 20;
        let source_id: u16 = 200;
        let field_level = Fixed::from_ratio(1, 2);
        // A spontaneous couple: acceptor (oxidant) above donor (reductant), so EMF > 0.
        let donor_potential = Fixed::from_ratio(2, 10);
        let acceptor_potential = Fixed::from_ratio(8, 10);
        let coupling = Fixed::from_int(2); // the reserved EMF-to-biomass value (a dev-fixture, owner's in canon)

        let mut reg = AbioticSourceRegistry::default();
        reg.insert(source_id, AbioticField::DataScalar(field_id), false, "");
        reg.biomass_per_stock = Fixed::from_int(4); // the global the redox derivation must OVERRIDE
        reg.draw_fraction = Fixed::ZERO;
        reg.weathering_rate = Fixed::ZERO;
        reg.emf_to_biomass = coupling;
        reg.set_source_redox(source_id, donor_potential, acceptor_potential);

        let mut e = EnvironFields::from_map(&map);
        let (w, h) = e.dims();
        e.set_producer(&[(cell, Fixed::from_int(1000))]);
        e.set_producer_source(&[(cell, vec![source_id])]);
        e.set_data_field(field_id, ScalarField::uniform(w, h, field_level));
        e.step(&temp, &calib);
        let mut soil = SoilNutrientField::new();
        e.extract_producers(&mut soil, &reg);

        let emf = civsim_physics::laws::battery_emf(acceptor_potential, donor_potential); // = 0.6
        let derived_yield = emf.checked_mul(coupling).unwrap();
        let cap = e.capacity_at(cell.x, cell.y);
        assert_eq!(
            cap,
            field_level.checked_mul(derived_yield).unwrap(),
            "the redox source's productivity derives from the floor EMF (E_acceptor - E_donor) times the coupling"
        );
        assert_ne!(
            cap,
            field_level.checked_mul(Fixed::from_int(4)).unwrap(),
            "the derivation OVERRIDES the registry-global conversion (the yield is read from the floor, not declared)"
        );
    }

    #[test]
    fn a_redox_derivation_takes_precedence_over_a_per_source_conversion() {
        // Arc 5 segment 3 precedence: when a source carries BOTH a redox couple AND a per-source biomass_per_stock
        // (segment 2), the redox derivation wins, so the couple's floor EMF sets the yield, not the declared
        // per-source number. Proven by setting a per-source conversion that would give a DIFFERENT cap and showing
        // the realised cap is the EMF-derived one, not the per-source one (and not the global).
        let map = a_map(0x9E11);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let field_id: u16 = 22;
        let source_id: u16 = 202;
        let field_level = Fixed::from_ratio(1, 2);
        let donor_potential = Fixed::from_ratio(2, 10);
        let acceptor_potential = Fixed::from_ratio(8, 10);
        let coupling = Fixed::from_int(2);
        let per_source_bps = Fixed::from_int(7); // a DIFFERENT per-source value the redox must override

        let mut reg = AbioticSourceRegistry::default();
        reg.insert(source_id, AbioticField::DataScalar(field_id), false, "");
        reg.biomass_per_stock = Fixed::from_int(4);
        reg.draw_fraction = Fixed::ZERO;
        reg.weathering_rate = Fixed::ZERO;
        reg.emf_to_biomass = coupling;
        reg.set_source_conversion(source_id, Some(per_source_bps), None); // segment-2 per-source value
        reg.set_source_redox(source_id, donor_potential, acceptor_potential); // segment-3 derivation

        let mut e = EnvironFields::from_map(&map);
        let (w, h) = e.dims();
        e.set_producer(&[(cell, Fixed::from_int(1000))]);
        e.set_producer_source(&[(cell, vec![source_id])]);
        e.set_data_field(field_id, ScalarField::uniform(w, h, field_level));
        e.step(&temp, &calib);
        let mut soil = SoilNutrientField::new();
        e.extract_producers(&mut soil, &reg);

        let emf = civsim_physics::laws::battery_emf(acceptor_potential, donor_potential); // 0.6
        let cap = e.capacity_at(cell.x, cell.y);
        assert_eq!(
            cap,
            field_level.checked_mul(emf.checked_mul(coupling).unwrap()).unwrap(),
            "the redox derivation takes precedence: the cap is EMF-derived, not the per-source conversion"
        );
        assert_ne!(
            cap,
            field_level.checked_mul(per_source_bps).unwrap(),
            "the per-source conversion (segment 2) is OVERRIDDEN by the redox derivation (segment 3)"
        );
    }

    #[test]
    fn a_standard_non_spontaneous_redox_couple_powers_no_life() {
        // Arc 5 segment 3: a couple whose acceptor is not above its donor releases no free energy at the STANDARD
        // state (standard EMF <= 0), so the yield clamps to zero and supports no biomass, emergent from the floor
        // law rather than an authored gate. Covers both the strictly-negative case AND the EMF == 0 boundary
        // (acceptor == donor), and a positive control (acceptor above donor yields biomass). The STANDARD-state
        // reading is the core's honest limit: the Nernst extension (flagged, not wired) would re-judge spontaneity
        // at the actual field concentrations, so a standard-non-spontaneous couple could turn spontaneous there.
        let map = a_map(0xDEAD00);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let field_id: u16 = 21;
        let source_id: u16 = 201;
        let coupling = Fixed::from_int(2);

        // Cap at `cell` for a couple with the given donor/acceptor standard potentials, all else fixed.
        let cap_for = |donor: Fixed, acceptor: Fixed| -> Fixed {
            let mut reg = AbioticSourceRegistry::default();
            reg.insert(source_id, AbioticField::DataScalar(field_id), false, "");
            reg.biomass_per_stock = Fixed::from_int(4);
            reg.draw_fraction = Fixed::ZERO;
            reg.weathering_rate = Fixed::ZERO;
            reg.emf_to_biomass = coupling;
            reg.set_source_redox(source_id, donor, acceptor);
            let mut e = EnvironFields::from_map(&map);
            let (w, h) = e.dims();
            e.set_producer(&[(cell, Fixed::from_int(1000))]);
            e.set_producer_source(&[(cell, vec![source_id])]);
            e.set_data_field(field_id, ScalarField::uniform(w, h, Fixed::from_int(1)));
            e.step(&temp, &calib);
            let mut soil = SoilNutrientField::new();
            e.extract_producers(&mut soil, &reg);
            e.capacity_at(cell.x, cell.y)
        };

        // Strictly negative standard EMF (acceptor 0.2 below donor 0.8): no biomass.
        assert_eq!(
            cap_for(Fixed::from_ratio(8, 10), Fixed::from_ratio(2, 10)),
            Fixed::ZERO,
            "a standard-non-spontaneous couple (acceptor below donor) supports no biomass"
        );
        // The EMF == 0 boundary (acceptor == donor): the clamp includes zero, still no biomass.
        assert_eq!(
            cap_for(Fixed::from_ratio(5, 10), Fixed::from_ratio(5, 10)),
            Fixed::ZERO,
            "the zero-EMF boundary (acceptor == donor) also supports no biomass (the clamp includes zero)"
        );
        // Positive control: a spontaneous couple (acceptor 0.8 above donor 0.2) DOES support biomass, so the zero
        // above is the couple's non-spontaneity, not a dead test.
        assert!(
            cap_for(Fixed::from_ratio(2, 10), Fixed::from_ratio(8, 10)) > Fixed::ZERO,
            "a spontaneous couple (acceptor above donor) supports biomass: the zero result isolates non-spontaneity"
        );
    }

    #[test]
    #[should_panic(expected = "emf_to_biomass coupling reserved value is unset")]
    fn a_redox_source_fails_loud_when_the_coupling_is_unset() {
        // Arc 5 segment 3 reserved-value discipline: the EMF-to-biomass coupling is the OWNER's value, surfaced not
        // fabricated. A redox derivation with the coupling at its fail-loud sentinel (zero, the default) REFUSES to
        // run rather than silently starving every redox producer, exactly as the global conversion's guard does.
        let map = a_map(0xC0DE00);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let mut reg = AbioticSourceRegistry::default();
        reg.insert(9, AbioticField::DataScalar(30), false, "");
        reg.biomass_per_stock = Fixed::from_int(4);
        reg.set_source_redox(9, Fixed::from_ratio(2, 10), Fixed::from_ratio(8, 10));
        // reg.emf_to_biomass left at the default sentinel (zero): the derivation must fail loud.
        let mut e = EnvironFields::from_map(&map);
        let (w, h) = e.dims();
        e.set_producer(&[(cell, Fixed::from_int(10))]);
        e.set_producer_source(&[(cell, vec![9])]);
        e.set_data_field(30, ScalarField::uniform(w, h, Fixed::from_int(1)));
        e.step(&temp, &calib);
        let mut soil = SoilNutrientField::new();
        e.extract_producers(&mut soil, &reg); // panics: coupling unset
    }

    #[test]
    fn a_donor_and_acceptor_are_co_limited_and_drawn_by_their_own_stoichiometry() {
        // Arc 5 segment 2 (per-source stoichiometry) composed with segment 1's Liebig co-limitation: a redox
        // chemolithotroph closes on a reduced DONOR field AND an oxidized ACCEPTOR field, each its OWN source id.
        // Two claims: (1) CO-LIMITATION is already delivered by the Liebig-minimum over the source set (the cap
        // is bound by the scarcer reactant, segment 1); (2) each reactant is DRAWN DOWN by its OWN stoichiometric
        // coefficient, not a shared 1:1 amount, so a reaction that consumes twice as much donor as acceptor is a
        // pure data row (no bespoke difference operator; the redox character is co-limitation + per-source data).
        let map = a_map(0x5ED0C0);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let donor_field: u16 = 10;
        let acceptor_field: u16 = 11;
        let donor_src: u16 = 100;
        let acceptor_src: u16 = 101;

        let mut reg = AbioticSourceRegistry::default();
        reg.insert(donor_src, AbioticField::DataScalar(donor_field), true, "");
        reg.insert(
            acceptor_src,
            AbioticField::DataScalar(acceptor_field),
            true,
            "",
        );
        reg.biomass_per_stock = Fixed::from_int(2);
        reg.draw_fraction = Fixed::from_ratio(1, 2); // a visible per-tick draw
        reg.weathering_rate = Fixed::ZERO;
        // The donor is consumed at twice the acceptor's stoichiometric rate (2:1), each its own per-source data.
        let donor_stoich = Fixed::from_int(1);
        let acceptor_stoich = Fixed::from_ratio(1, 2);
        reg.set_source_conversion(donor_src, None, Some(donor_stoich));
        reg.set_source_conversion(acceptor_src, None, Some(acceptor_stoich));

        // The acceptor is the SCARCER reactant, so it should bind the productivity (co-limitation).
        let donor_level = Fixed::from_int(2);
        let acceptor_level = Fixed::from_ratio(1, 2);
        let mut e = EnvironFields::from_map(&map);
        let (w, h) = e.dims();
        e.set_producer(&[(cell, Fixed::from_int(1000))]);
        e.set_producer_source(&[(cell, vec![donor_src, acceptor_src])]);
        e.set_data_field(donor_field, ScalarField::uniform(w, h, donor_level));
        e.set_data_field(acceptor_field, ScalarField::uniform(w, h, acceptor_level));
        e.step(&temp, &calib);
        let mut soil = SoilNutrientField::new();
        e.extract_producers(&mut soil, &reg);

        // (1) Co-limitation: the cap is the SCARCER (acceptor) supported biomass, not the donor's.
        let cap = e.capacity_at(cell.x, cell.y);
        assert_eq!(
            cap,
            acceptor_level.checked_mul(reg.biomass_per_stock).unwrap(),
            "the scarcer reactant (acceptor) binds the productivity: the Liebig minimum co-limits (segment 1)"
        );
        assert!(
            cap < donor_level.checked_mul(reg.biomass_per_stock).unwrap(),
            "the abundant donor alone would support more; co-limitation holds it to the acceptor"
        );
        // (2) Per-source stoichiometry: each reactant drawn by capacity*draw_fraction*its_own_stoich.
        let draw_biomass = cap.checked_mul(reg.draw_fraction).unwrap();
        let donor_remaining = e.data_field_at(donor_field, cell.x, cell.y).unwrap();
        let acceptor_remaining = e.data_field_at(acceptor_field, cell.x, cell.y).unwrap();
        assert_eq!(
            donor_remaining,
            donor_level - draw_biomass.checked_mul(donor_stoich).unwrap(),
            "the donor is drawn down by its OWN stoichiometric coefficient"
        );
        assert_eq!(
            acceptor_remaining,
            acceptor_level - draw_biomass.checked_mul(acceptor_stoich).unwrap(),
            "the acceptor is drawn down by its OWN coefficient (2:1 vs the donor, not a shared 1:1 amount)"
        );
        let donor_drawn = donor_level - donor_remaining;
        let acceptor_drawn = acceptor_level - acceptor_remaining;
        assert_eq!(
            donor_drawn,
            acceptor_drawn.checked_mul(Fixed::from_int(2)).unwrap(),
            "the donor is consumed at twice the acceptor's rate: reaction-specific stoichiometry, a data row"
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
