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
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::PhysicsRegistry;
use civsim_world::{Coord3, TileMap};
use std::collections::{BTreeMap, BTreeSet};

use crate::edibility::Composition;
use crate::locomotion::ResourceField;
use crate::physiology::{ENERGY_DENSITY, SALINITY, WATER_FRACTION};
use crate::runner::Field;
use civsim_foundation::calibration::{CalibrationError, CalibrationManifest};
use civsim_foundation::material::{EarthworkField, SoilNutrientField};
use civsim_foundation::stocks::Stock;

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
    // tangent (laws::saturation_vapor_pressure). On a Kelvin world (one that declares a
    // `hydrology.vapor_source_temperature`) `sat_slope` and `sat_e_ref` DERIVE from the volatile's
    // measured latent heat and molar mass ([`derive_saturation_index_tangent`]), no longer two
    // independently-authored numbers; the normalised dev fixture keeps its labelled affine pair.
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
    /// ([`civsim_foundation::stocks::Stock`]). Larger regrows a grazed patch faster and raises the carrying
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

/// The molar gas constant `R = N_A * k_B` DERIVED from the CODATA fundamentals (never an authored decimal),
/// the same float-free composite compute the Stefan-Boltzmann sigma uses
/// ([`crate::physiology::derived_stefan_boltzmann`]): the units-crate evaluates the relation EXACTLY as a
/// rational and rounds ONCE to a fixed-point scale, and this projects it to the sim's Q32.32 `Fixed`. A pure,
/// deterministic, memoized load constant, so it perturbs no canonical result. `R` sits comfortably inside
/// Q32.32 (~8.314), so the projection is exact to the canonical resolution.
pub fn derived_gas_constant() -> Fixed {
    use std::sync::OnceLock;
    static R: OnceLock<Fixed> = OnceLock::new();
    *R.get_or_init(|| {
        let representation = civsim_units::constants::si_representation_magnitudes()
            .expect("the SI representation view must derive");
        let value = representation
            .get(civsim_units::fundamentals::GAS_CONSTANT.symbol)
            .expect("the SI representation view must contain the molar gas constant");
        let bits = i64::try_from(value.bits()).expect("R at its derived scale fits i64");
        let q32 = civsim_units::rescale_bits(bits, value.scale_bits(), Fixed::FRAC_BITS)
            .expect("R rescale to Q32.32 must not overflow");
        Fixed::from_bits(q32)
    })
}

/// Parse a chemical formula string (`"H2O"`, `"CO2"`, `"CH4"`) into its element counts, so a volatile's molar
/// mass derives from its OWN declared formula plus the periodic table rather than a hardcoded molecule (an
/// alien volatile is a data row, Principle 11). Each element is one uppercase letter, optional lowercase
/// letters (a two-letter symbol such as `He` or `Cl`), and an optional decimal count (default one). Returns
/// `None` on a malformed or empty formula.
fn parse_formula(formula: &str) -> Option<BTreeMap<String, u32>> {
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let chars: Vec<char> = formula.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_uppercase() {
            return None;
        }
        let mut symbol = String::new();
        symbol.push(chars[i]);
        i += 1;
        while i < chars.len() && chars[i].is_ascii_lowercase() {
            symbol.push(chars[i]);
            i += 1;
        }
        let mut digits = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() {
            digits.push(chars[i]);
            i += 1;
        }
        let count: u32 = if digits.is_empty() {
            1
        } else {
            digits.parse().ok()?
        };
        *counts.entry(symbol).or_insert(0) += count;
    }
    if counts.is_empty() {
        None
    } else {
        Some(counts)
    }
}

/// The moisture-index saturation tangent `(slope, e_ref)` DERIVED for the world's condensing volatile, the
/// affine pair the hydrology's [`laws::saturation_vapor_pressure`] reads. It retires the double-authored
/// `hydrology.saturation_slope` and `hydrology.saturation_e_ref`: both encode the SAME Clausius-Clapeyron
/// relation, so authoring both authors it twice. Here both DERIVE from the volatile's MEASURED specific latent
/// heat of vaporization (the floor `therm.latent_heat`), its molar mass (the periodic table over its own
/// formula), the DERIVED molar gas constant ([`derived_gas_constant`], `R = N_A*k_B`), and two per-world
/// reference temperatures.
///
/// The hydrology precipitation and evaporation compare `e_s` against the worldgen `[0, 1]` moisture field, so
/// `e_s` is a dimensionless SATURATION INDEX `e_s(T)/e_s(T_source)` (the local saturation as a fraction of the
/// world's warm vapour source), not an absolute pressure. So the derivation stays in that index space:
/// `e_ref = e_s(T_ref)/e_s(T_source) = exp((L/R_v)*(1/T_source - 1/T_ref))` (the constant-latent-heat
/// Clausius-Clapeyron saturation ratio, `T_source > T_ref` so the exponent is negative and `e_ref < 1`), and
/// `slope = L*e_ref/(R_v*T_ref^2)` (the named [`laws::saturation_slope_from_latent_heat`], the tangent of that
/// index at `T_ref`). `R_v = R/M` is the specific gas constant. `t_ref` is the world's mean surface temperature
/// (Kelvin), `t_source` its warm vapour-source temperature (the moisture-index normalization). Returns `None`
/// if the temperatures are degenerate or the floor lacks the volatile or a primitive.
pub fn derive_saturation_index_tangent(t_ref: Fixed, t_source: Fixed) -> Option<(Fixed, Fixed)> {
    if t_ref <= Fixed::ZERO || t_source <= Fixed::ZERO {
        return None;
    }
    let floor = PhysicsRegistry::ground().ok()?;
    // The condensing volatile is keyed by the floor substance id, the same "water" the surface-cooling latent
    // heat reads (run_world derive_surface_cooling); a data-defined condensing-species selector is the alien
    // follow-on, consistent with that existing read.
    let water = floor.substance("water")?;
    // The measured specific latent heat of vaporization L (floor therm.latent_heat, stored kJ/kg -> J/kg).
    let latent_kj = water.vector.get("therm.latent_heat").copied()?;
    let l_vap = latent_kj.checked_mul(Fixed::from_int(1000))?;
    // The specific gas constant R_v = R / M: the DERIVED molar gas constant over the volatile's molar mass,
    // itself derived from the volatile's OWN formula plus the periodic table (never an authored molar mass).
    // molar_mass is g/mol, so R_v = R * 1000 / M_gmol yields J/(kg*K).
    let counts = parse_formula(&water.formula)?;
    let periodic = PeriodicTable::standard().ok()?;
    let m_gmol = periodic.molar_mass(&counts).ok()?;
    if m_gmol <= Fixed::ZERO {
        return None;
    }
    let r = derived_gas_constant();
    let r_v = r.checked_mul(Fixed::from_int(1000))?.checked_div(m_gmol)?;
    if r_v <= Fixed::ZERO {
        return None;
    }
    // e_ref = e_s(T_ref)/e_s(T_source) = exp((L/R_v)*(1/T_source - 1/T_ref)), the constant-L Clausius-Clapeyron
    // saturation index at the reference temperature.
    let l_over_rv = l_vap.checked_div(r_v)?;
    let inv_source = Fixed::ONE.checked_div(t_source)?;
    let inv_ref = Fixed::ONE.checked_div(t_ref)?;
    let exponent = l_over_rv.checked_mul(inv_source - inv_ref)?;
    let e_ref = exponent.exp();
    // slope = L*e_ref/(R_v*T_ref^2), the tangent of that index at T_ref (the named Clausius-Clapeyron slope law).
    let slope = laws::saturation_slope_from_latent_heat(l_vap, t_ref, e_ref, r_v);
    Some((slope, e_ref))
}

impl EnvironCalib {
    /// The environmental calibration read fail-loud from the manifest (Principle 11): every forcing
    /// constant is a reserved value that refuses to build while unset.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<EnvironCalib, CalibrationError> {
        // The saturation tangent's reference temperature DERIVES from the world's mean surface
        // temperature (the habitable-band midpoint the tangent is most accurate around), the same
        // absolute-temperature value the temperature field is centred on (Field::from_map_absolute),
        // rather than a duplicate reserved scalar (derive-vs-author, Principle 6; the retired
        // hydrology.saturation_t_ref duplicated it).
        let sat_t_ref = m.require_fixed("climate.mean_surface_temperature")?;
        // The tangent's SLOPE and E_REF DERIVE from the condensing volatile's measured latent heat and
        // molar mass on the Kelvin path, retiring the double-authored `hydrology.saturation_slope` and
        // `hydrology.saturation_e_ref` (both encoded the SAME Clausius-Clapeyron relation, so authoring
        // both authored it twice). The opt-in signal AND the moisture-index normalization reference is the
        // per-world `hydrology.vapor_source_temperature` (the warmest saturated-air cell, T_ref + range/2);
        // a normalised dev fixture that declares none keeps its labelled affine pair, byte-identical.
        let (sat_slope, sat_e_ref) = match m.require_fixed("hydrology.vapor_source_temperature") {
            Ok(t_source) => {
                let tangent = derive_saturation_index_tangent(sat_t_ref, t_source);
                tangent.ok_or_else(|| CalibrationError::BadValue {
                    id: "hydrology.vapor_source_temperature".to_string(),
                    detail:
                        "the condensing volatile's saturation index failed to derive from the floor"
                            .to_string(),
                })?
            }
            Err(_) => (
                m.require_fixed("hydrology.saturation_slope")?,
                m.require_fixed("hydrology.saturation_e_ref")?,
            ),
        };
        Ok(EnvironCalib {
            sat_slope,
            sat_t_ref,
            sat_e_ref,
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
            // The uniform baseline soil supply DERIVES from the soil requirement rather than a duplicate
            // reserved scalar: its basis declares it MUST equal `soil_requirement` so bare soil is exactly
            // non-limiting at baseline (soil / requirement = 1), before the matter-cycle fertility field
            // differentiates the supply per cell (derive-vs-author, Principle 6; the same pattern as the
            // retired `hydrology.saturation_t_ref` twin). The field stays overridable for the fertility test.
            soil_baseline: m.require_fixed("productivity.soil_requirement")?,
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
            // Equals soil_req (the invariant: bare soil is exactly non-limiting at baseline).
            soil_baseline: Fixed::from_ratio(1, 2),
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
    /// biosphere ([`crate::genesis::LivingWorld::producer_compositions`]). `None` where
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
    /// The world's DATA sky for the diurnal insolation drive (day-night arc), or `None` when the cycle is
    /// UNARMED. Opt-in like the living scenario: while `None` the `light` field keeps the static build-time
    /// latitude map, so a run that never arms the cycle is byte-identical and the four determinism pins hold;
    /// while `Some`, [`Self::step`] recomputes the light each tick from the sun-angle law over this sky.
    sky: Option<DiurnalSky>,
    /// The private diurnal PHASE COUNTER: the number of environ steps since arming, advanced once per tick in
    /// the insolation step. It is FIREWALLED, never exposed to the percept or any behavioural substrate, so a
    /// diurnal rhythm can only reach a being through the cycling light and temperature MAGNITUDES it perceives
    /// and never by reading the clock (the steering line: entrainment emerges, it is not templated). Not folded
    /// into the state hash: its effect enters through the light field, which drives the hashed productivity
    /// capacity; on an unarmed run it never advances, so the pins hold.
    diurnal_tick: u64,
    /// The per-cell SOLAR-FORCING BASELINE temperature the diurnal drive computes each armed tick (the
    /// radiative-equilibrium temperature of the absorbed insolation plus the world's back-radiation floor,
    /// day-night arc, night-floor form (2)). The runner copies it into the temperature `Field`'s relaxation
    /// baseline, and the field's existing relaxation-plus-diffusion produces the emergent surface swing and
    /// thermal lag. EMPTY when the cycle is unarmed (no copy happens, the field baseline stands, byte-identical).
    solar_baseline: Vec<Fixed>,
    /// The per-cell THERMAL-INERTIA FACTOR the diurnal drive computes each armed tick (follow-on 2): the ratio
    /// of the dry substrate's volumetric heat capacity to the cell's own, blended from the cell's real water
    /// fraction and freeze state ([`SurfaceThermal::inertia_factor`]). The runner copies it into the temperature
    /// `Field`, whose relaxation-plus-diffusion step scales by it, so a water-laden cell lags and swings less
    /// than dry land (the ocean effect), emergent from the cell's own water content. A dry cell reads one, so
    /// its dynamics are unchanged. EMPTY when the cycle is unarmed (the field's inertia stays absent, and its
    /// step is byte-identical to the uniform pre-follow-on kernel).
    solar_inertia: Vec<Fixed>,
    /// The per-cell EVAPORATION mass flux the hydrology step computed LAST tick (kg/(m^2*s)), stored so the
    /// diurnal surface balance can read it as the latent cooling flux one tick later. `step_insolation` runs
    /// before `step_hydrology`, so the current-tick flux is not yet available; the one-tick lag (the same lag the
    /// thermal-inertia freeze-read already carries) is invisible under the field's relaxation timescale, and the
    /// deterministic replay preserves it. EMPTY until the hydrology step has run once (the balance then reads zero
    /// latent cooling, the honest first-tick transient), and read only when surface cooling is armed, so it is
    /// byte-neutral on an unarmed run.
    evaporation: Vec<Fixed>,
    /// The world's SURFACE TURBULENT-COOLING data, or `None` when it is UNARMED (the default). While `None`, the
    /// diurnal surface balance keeps only its radiative loss (the pre-arc `radiative_equilibrium`, reached exactly
    /// through the `h = 0, q_latent = 0` short-circuit of [`civsim_physics::laws::surface_balance_temperature`]),
    /// so a run that never arms it is byte-identical and the pins hold. While `Some`, the balance adds the sensible
    /// and latent surface cooling, so the surface temperature emerges from the full turbulent balance rather than
    /// running hot on radiation alone. Armed by a scenario (the living world) that supplies its air medium's
    /// convective coefficient and latent heat.
    surface_cooling: Option<SurfaceCooling>,
    /// The world's PHOTOSYNTHESIS calibration, or `None` when it is UNARMED (the default). While `None`,
    /// [`Self::step_productivity`] keeps the abstract-producer Liebig `biomass_from` path, so a run that never
    /// arms it is byte-identical and the four determinism pins hold. While `Some` (and the diurnal sky is armed,
    /// so the stellar-constant flux anchor is present), the per-cell productivity DERIVES from photosynthesis
    /// ([`carbon_fixation_rate`]): the light-response over the real insolation flux, the carbon-fixing enzyme's
    /// thermal-performance tent, the water-use-efficiency coupling to the evaporation flux, and the soil-nutrient
    /// limitation over the matter-cycle fertility, retiring the authored `productivity.*_requirement` and the flat
    /// `soil_baseline` on the armed path (the photosynthesis-to-productivity arc, #156). Armed by the living world.
    photosynthesis: Option<PhotosynthesisCalib>,
}

/// The SURFACE TURBULENT-COOLING data a world supplies to arm the diurnal surface balance's latent and sensible
/// terms: the air medium's convective coefficient `h` (the sensible flux `h*(T - T_air)`, `fluid.convective_coefficient`)
/// and the latent heat of vaporization `L_vap` (the latent flux `E * L_vap`, the cited `1/metabolism.water_loss_per_joule`).
/// Both are world DATA read from the floor and the manifest, never authored here; `None` on `EnvironFields` leaves the
/// balance radiative-only and byte-identical (Principle 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceCooling {
    /// The air medium's convective heat-transfer coefficient `h` (W/(m^2*K)), `fluid.convective_coefficient`.
    pub convective_h: Fixed,
    /// The latent heat of vaporization `L_vap` (J/kg), the cited `1/metabolism.water_loss_per_joule`.
    pub latent_heat: Fixed,
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
    /// The NERNST concentration term (the depth extension, now built): the couple's CARRIER CHARGE `q = n*e`
    /// (Coulombs, on the `elec.charge` floor axis, a per-couple datum), the electrons the reaction transfers
    /// times the elementary charge. ZERO is the OPT-OUT: at `q = 0` the Nernst reduces to the standard EMF
    /// (`nernst_emf` returns `standard_emf`), so a couple that declares no carrier charge keeps the
    /// concentration-INDEPENDENT standard behaviour and is byte-identical. A world arms the concentration
    /// dependence by declaring `q > 0` (via [`AbioticSourceRegistry::set_source_nernst`]). RESERVED.
    pub carrier_charge: Fixed,
    /// The ACCEPTOR's concentration field (its located stock), read at the cell for the Nernst activity. `None`
    /// reads unit acceptor activity (an abundant/buffered acceptor), reproducing the standard EMF's implicit
    /// unit-activity acceptor. The DONOR activity is the source's own bound stock field (the fuel that
    /// depletes). Data (Principle 11): a world names which field carries the acceptor.
    pub acceptor_field: Option<AbioticField>,
    /// The activity COEFFICIENTS (dimensionless) for the donor and acceptor, the gamma of the Nernst activity
    /// `a = gamma * concentration` (a data-defined activity registry, non-Terran override). ONE is the ideal
    /// reference (`a = concentration`), the byte-neutral default. RESERVED per couple.
    pub gamma_donor: Fixed,
    /// The acceptor's activity coefficient (see [`Self::gamma_donor`]).
    pub gamma_acceptor: Fixed,
    /// The temperature COEFFICIENT of the standard potential `dE0/dT` (V/K), the couple's reaction-entropy
    /// term, and the reference temperature `t_ref` (K) it is measured at, so `E0(T)` shifts with the cell
    /// temperature (`standard_potential_at_temperature`). ZERO `de0_dt` is the temperature-independent default.
    /// RESERVED per couple.
    pub de0_dt: Fixed,
    /// The reference temperature the standard potential and `dE0/dT` are measured at (K). See [`Self::de0_dt`].
    pub t_ref: Fixed,
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

/// The per-source-class KINETICS of a redox uptake (the reversible-Michaelis-Menten draw, phase-2 increment 2):
/// the catalytic turnover `kcat`, the half-saturation stock `Km`, the Hill cooperativity `h`, and the composition
/// CLASS whose amount is the being's catalyst tissue (so `Vmax = kcat * catalyst`). A high-affinity oligotroph and
/// a low-affinity copiotroph are data rows, not one authored kinetics. The catalyst class is DATA (Principle 11):
/// it defaults to `bio.protein` (enzymes are proteins, Principle 4's causal-primitive-plus-correlating-proxy), but
/// a world names its own, so a silicon or mineral-catalyst alien whose catalyst is NOT protein is a data row. The
/// admit-alien catalyst AXIS (a first-class catalyst datum rather than a borrowed composition class) is coupled to
/// R-SOURCE-VECTOR (the shared source-vector substrate), flagged there, not baked here.
#[derive(Clone, Debug)]
pub struct RedoxKinetics {
    /// The catalytic turnover `kcat` (per tick), the maximum specific uptake per unit catalyst tissue. RESERVED,
    /// basis the enzyme's turnover number. `Vmax = kcat * catalyst_tissue`, the emergent-throughput architecture
    /// (no authored efficiency scalar; the throughput derives from the being's own catalyst amount).
    pub kcat: Fixed,
    /// The half-saturation stock `Km` (in the source's stock units): the substrate at which the uptake is half of
    /// `Vmax`. RESERVED, basis the transporter's affinity for the source class it draws.
    pub km: Fixed,
    /// The Hill cooperativity exponent `h` (dimensionless): 1 the plain Monod, above 1 cooperative uptake.
    /// RESERVED, basis the couple's cooperativity (1 unless the uptake is known cooperative).
    pub hill: Fixed,
    /// The producer-composition CLASS whose amount is the catalyst tissue (`Vmax = kcat * producer_food[class]`).
    /// DATA: `bio.protein` by default (the protein-fraction proxy), a world names its own for an alien catalyst.
    pub catalyst_class: String,
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
    /// The reversible-Michaelis-Menten uptake KINETICS (phase-2 increment 2): when `Some` on an armed redox
    /// source, the pass-2 draw is the [`civsim_physics::laws::reversible_uptake_flux`] (Hill-saturating, driven by
    /// the Nernst EMF, `min(v, S)` conserved, `Vmax = kcat * catalyst tissue`) rather than the capacity-
    /// proportional draw. `None` (every Terran source and every unarmed redox source) keeps the segment-2 draw,
    /// so the run is byte-identical. See [`RedoxKinetics`].
    pub kinetics: Option<RedoxKinetics>,
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
    /// RESERVED (the Nernst thermal factor, surfaced not fabricated): the BOLTZMANN constant `k_B` (J/K), read
    /// as a calibration value rather than a floor-toml axis (option B, so this does not grow the floor or race
    /// the concurrent floor-reconciliation sweep). It sets the per-particle thermal factor `k_B*T/q` of the
    /// Nernst EMF. Basis: the CODATA value `1.380649e-23 J/K`, scaled to the engine units. Defaults to the
    /// fail-loud sentinel (zero): while it is unset, an ARMED Nernst source (carrier charge above zero) refuses
    /// to run rather than fabricating a thermal factor; an unarmed or non-redox source never reads it, so every
    /// Terran run stays byte-identical. The graduation of `k_B` to a proper floor fundamental (with `R` and
    /// sigma re-derived from it) is the flagged floor-reconciliation follow-on, not this arc.
    pub boltzmann_k: Fixed,
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
                kinetics: None,
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
                // The Nernst concentration term is OPT-OUT by default: q = 0 reduces to the standard EMF, so a
                // source armed only via this setter keeps the concentration-independent standard behaviour and
                // is byte-identical. Ideal activity coefficients, no temperature coefficient.
                carrier_charge: Fixed::ZERO,
                acceptor_field: None,
                gamma_donor: Fixed::ONE,
                gamma_acceptor: Fixed::ONE,
                de0_dt: Fixed::ZERO,
                t_ref: Fixed::ZERO,
            });
        }
    }

    /// Arm the NERNST concentration dependence on an already-redox source (the depth extension): the couple's
    /// carrier charge `q = n*e` (on `elec.charge`), the field carrying the ACCEPTOR concentration (`None` for a
    /// unit-activity buffered acceptor), the donor/acceptor activity coefficients (gamma, `ONE` ideal), and the
    /// standard-potential temperature coefficient `dE0/dT` with its reference temperature. With `q > 0` the
    /// source's yield reads the couple's ACTUAL located concentrations, so its drive falls as the couple
    /// depletes and crosses zero at its own equilibrium; `q = 0` (the default) is the standard EMF. A no-op if
    /// the id is unbound or not a redox source (arm [`Self::set_source_redox`] first). All data (Principle 11).
    #[allow(clippy::too_many_arguments)]
    pub fn set_source_nernst(
        &mut self,
        id: u16,
        carrier_charge: Fixed,
        acceptor_field: Option<AbioticField>,
        gamma_donor: Fixed,
        gamma_acceptor: Fixed,
        de0_dt: Fixed,
        t_ref: Fixed,
    ) {
        if let Some(b) = self.bindings.get_mut(&id) {
            if let Some(rx) = b.redox_emf.as_mut() {
                rx.carrier_charge = carrier_charge;
                rx.acceptor_field = acceptor_field;
                rx.gamma_donor = gamma_donor;
                rx.gamma_acceptor = gamma_acceptor;
                rx.de0_dt = de0_dt;
                rx.t_ref = t_ref;
            }
        }
    }

    /// Arm the reversible-Michaelis-Menten uptake KINETICS on a source (phase-2 increment 2): the catalytic
    /// turnover `kcat`, the half-saturation `km`, the Hill exponent `hill`, and the producer-composition class
    /// whose amount is the catalyst tissue (`Vmax = kcat * producer_food[catalyst_class]`, default `bio.protein`).
    /// With this set on an armed redox source, its pass-2 draw is the reversible flux; unset, the draw is the
    /// segment-2 capacity-proportional draw (byte-identical). A no-op if the id is unbound. All data (Principle 11).
    pub fn set_source_kinetics(
        &mut self,
        id: u16,
        kcat: Fixed,
        km: Fixed,
        hill: Fixed,
        catalyst_class: &str,
    ) {
        if let Some(b) = self.bindings.get_mut(&id) {
            b.kinetics = Some(RedoxKinetics {
                kcat,
                km,
                hill,
                catalyst_class: catalyst_class.to_string(),
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

    /// Set the reserved Boltzmann constant `k_B` the Nernst thermal factor reads (option B: a calibration value,
    /// not a floor axis). A world that arms a Nernst redox source sets it from CODATA; a Terran world leaves it
    /// at the fail-loud sentinel and never reads it.
    pub fn set_boltzmann_k(&mut self, k: Fixed) {
        self.boltzmann_k = k;
    }

    /// The NERNST-adjusted redox stock-to-biomass conversion at a cell (the concentration-dependent depth
    /// extension): for an ARMED redox source (carrier charge above zero) it returns `Some(bps)` computed from
    /// the couple's ACTUAL activities through [`civsim_physics::laws::nernst_emf`], so the drive FALLS as the
    /// couple depletes and crosses zero at its own equilibrium, clamped at zero and times `emf_to_biomass`. The
    /// activities are the raw donor and acceptor concentrations at the cell each times its `gamma`; the standard
    /// potential first shifts with the cell temperature (`dE0/dT`). Returns `None` for a non-redox source or an
    /// UNARMED redox source (carrier charge zero), whose conversion stays the standard [`Self::effective_conversion`]
    /// (byte-identical). Fails loud if `emf_to_biomass` or `k_B` is unset while armed (the sentinel discipline).
    fn redox_conversion_at(
        &self,
        binding: &AbioticBinding,
        donor_conc: Fixed,
        acceptor_conc: Fixed,
        temperature: Fixed,
    ) -> Option<Fixed> {
        let emf = self.redox_nernst_emf(binding, donor_conc, acceptor_conc, temperature)?;
        Some(emf.checked_mul(self.emf_to_biomass).unwrap_or(Fixed::MAX))
    }

    /// The clamped NERNST EMF (volts) of an ARMED redox source at a cell, the shared drive both the pass-1 yield
    /// ([`Self::redox_conversion_at`]) and the pass-2 reversible-flux draw ([`Self::redox_draw_flux`]) read: the
    /// standard potential shifted for temperature (`dE0/dT`), corrected for the couple's ACTUAL activities (raw
    /// concentrations times `gamma`), clamped at zero (no life below its own equilibrium). Returns `None` for a
    /// non-redox or UNARMED redox source (carrier charge zero). Fails loud if `emf_to_biomass` or `k_B` is unset
    /// while armed (the sentinel discipline).
    fn redox_nernst_emf(
        &self,
        binding: &AbioticBinding,
        donor_conc: Fixed,
        acceptor_conc: Fixed,
        temperature: Fixed,
    ) -> Option<Fixed> {
        let rx = binding.redox_emf.as_ref()?;
        if rx.carrier_charge <= Fixed::ZERO {
            return None; // unarmed: the standard concentration-independent EMF via effective_conversion
        }
        assert!(
            self.emf_to_biomass > Fixed::ZERO,
            "redox source: emf_to_biomass coupling reserved value is unset (would starve every redox producer)"
        );
        assert!(
            self.boltzmann_k > Fixed::ZERO,
            "Nernst redox source: boltzmann_k reserved value is unset (would fabricate the thermal factor)"
        );
        let a_donor = rx.gamma_donor.checked_mul(donor_conc).unwrap_or(donor_conc);
        let a_acceptor = rx
            .gamma_acceptor
            .checked_mul(acceptor_conc)
            .unwrap_or(acceptor_conc);
        // E0(T), then the Nernst concentration correction, clamped at zero (no life below equilibrium).
        let e0_t = laws::standard_potential_at_temperature(
            laws::battery_emf(rx.acceptor_potential, rx.donor_potential),
            rx.de0_dt,
            temperature,
            rx.t_ref,
        );
        Some(
            laws::nernst_emf(
                e0_t,
                a_donor,
                a_acceptor,
                self.boltzmann_k,
                temperature,
                rx.carrier_charge,
            )
            .max(Fixed::ZERO),
        )
    }

    /// The pass-2 reversible-Michaelis-Menten uptake DRAW of an armed redox source WITH kinetics (phase-2
    /// increment 2): the [`civsim_physics::laws::reversible_uptake_flux`] over the source's own stock, driven by
    /// the couple's Nernst EMF, with `Vmax = kcat * catalyst_tissue` (the emergent throughput, no authored
    /// efficiency scalar), the Hill saturation, and the structural `min(v, S)` conservation clamp. Returns `None`
    /// for a source that is non-redox, unarmed, or carries no kinetics, whose draw stays the segment-2
    /// capacity-proportional draw (byte-identical). `catalyst_tissue` is the being's catalyst amount (its
    /// `catalyst_class` composition), read by the caller from the producer at the cell.
    fn redox_draw_flux(
        &self,
        binding: &AbioticBinding,
        stock: Fixed,
        acceptor_conc: Fixed,
        temperature: Fixed,
        catalyst_tissue: Fixed,
    ) -> Option<Fixed> {
        let kin = binding.kinetics.as_ref()?;
        let rx = binding.redox_emf.as_ref()?;
        let emf = self.redox_nernst_emf(binding, stock, acceptor_conc, temperature)?;
        let vmax = kin.kcat.checked_mul(catalyst_tissue).unwrap_or(Fixed::ZERO);
        Some(laws::reversible_uptake_flux(
            stock,
            vmax,
            kin.km,
            kin.hill,
            emf,
            self.boltzmann_k,
            temperature,
            rx.carrier_charge,
        ))
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
            sky: None,
            diurnal_tick: 0,
            solar_baseline: Vec::new(),
            solar_inertia: Vec::new(),
            evaporation: Vec::new(),
            surface_cooling: None,
            photosynthesis: None,
        }
    }

    /// Arm the DERIVED PHOTOSYNTHESIS productivity (the photosynthesis-to-productivity arc, #156, opt-in) with the
    /// world's producer photosynthesis calibration, so [`Self::step_productivity`] derives each cell's carbon-
    /// fixation rate ([`carbon_fixation_rate`]) instead of the abstract-producer Liebig `biomass_from`. Unarmed
    /// (the default) the productivity stays the Liebig interim and the run is byte-identical, so the four
    /// determinism pins hold; a scenario that wants the real derivation (the living world) calls this AND
    /// [`Self::arm_diurnal`] (the derivation reads the sky's stellar-constant flux anchor). Where the sky is
    /// unarmed the derivation has no flux anchor, so the Liebig path stands (the clean degrade).
    pub fn arm_photosynthesis(&mut self, photo: PhotosynthesisCalib) {
        self.photosynthesis = Some(photo);
    }

    /// Arm the DIURNAL insolation cycle (day-night arc, opt-in) with the world's data sky, so [`Self::step`]
    /// recomputes the `light` field each tick from the sun-angle law instead of holding the static latitude map.
    /// Unarmed (the default) the light stays static and the run is byte-identical, so the determinism pins hold;
    /// this is armed only by a scenario that wants the cycle, like the living world. The phase counter restarts
    /// at zero on arming (a deterministic per-scenario dawn), never read by any behavioural substrate.
    pub fn arm_diurnal(&mut self, sky: DiurnalSky) {
        self.sky = Some(sky);
        self.diurnal_tick = 0;
    }

    /// Arm the SURFACE TURBULENT COOLING of the diurnal balance (opt-in) with the world's air convective
    /// coefficient and latent heat, so the surface temperature emerges from the full balance (radiative plus
    /// sensible plus latent) instead of running hot on radiation alone. Unarmed (the default) the balance is
    /// radiative-only and byte-identical, so the pins hold; a scenario that wants the turbulent cooling (the living
    /// world) calls this. Independent of [`Self::arm_diurnal`]: the cooling terms only reach the baseline while the
    /// diurnal cycle is also armed (the balance runs in `step_insolation`), so arming this without the sky is inert.
    pub fn arm_surface_cooling(&mut self, cooling: SurfaceCooling) {
        self.surface_cooling = Some(cooling);
    }

    /// Recompute the `light` field from the diurnal sun-angle law (day-night arc), advancing the private phase
    /// counter one tick. A no-op when the cycle is unarmed (`sky` is `None`), so an unarmed run never touches the
    /// light and is byte-identical. The phase is the counter modulo the sidereal rotation period, the orbital
    /// phase the counter modulo the orbital period, and each cell's light is [`insolation_at`] over the world's
    /// star-list. Deterministic (a pure fold over the counter and the cell coordinate), so it replays.
    fn step_insolation(&mut self, temp: &Field) {
        let Some(sky) = self.sky.clone() else {
            return; // the cycle is unarmed: the static latitude light stands, byte-identical.
        };
        let (w, h) = (self.width, self.height);
        let diurnal_phase = Fixed::from_ratio(
            (self.diurnal_tick % sky.rotation_period_ticks) as i64,
            sky.rotation_period_ticks as i64,
        );
        let orbital_phase = Fixed::from_ratio(
            (self.diurnal_tick % sky.orbital_period_ticks) as i64,
            sky.orbital_period_ticks as i64,
        );
        let n = (w.max(0) as usize) * (h.max(0) as usize);
        if self.solar_baseline.len() != n {
            self.solar_baseline = vec![Fixed::ZERO; n];
        }
        if self.solar_inertia.len() != n {
            self.solar_inertia = vec![Fixed::ONE; n];
        }
        for y in 0..h {
            for x in 0..w {
                let i = self.idx(x, y);
                let insol = insolation_at(x, y, w, h, diurnal_phase, orbital_phase, &sky);
                self.light[i] = insol;
                // The heat baseline (night-floor form (2)): the absorbed irradiance is the normalised daylit
                // insolation scaled to physical watts by the stellar constant and REDUCED by the world's
                // shortwave albedo (the reflected fraction is not absorbed), plus the world's back-radiation
                // floor (longwave, not reflected) so the night side relaxes toward a retained temperature
                // (airless zero, Earth mild, Venus high) rather than absolute zero. The radiative-equilibrium
                // law returns the surface temperature of that absorbed flux; the field's relaxation-plus-
                // diffusion then produces the emergent swing and lag. The albedo is the uniform planetary Bond
                // value (per-world data, zero for the reference world so it stays byte-identical); the per-cell,
                // per-material albedo from the floor `opt.reflectance` is the named follow-on.
                let solar_absorbed_fraction = (Fixed::ONE - sky.albedo).max(Fixed::ZERO);
                let absorbed = insol
                    .checked_mul(sky.solar_constant)
                    .unwrap_or(Fixed::MAX)
                    .checked_mul(solar_absorbed_fraction)
                    .unwrap_or(Fixed::MAX)
                    .saturating_add(sky.back_radiation);
                // The surface energy balance: the absorbed flux set against radiative emission and, when surface
                // turbulent cooling is armed, the sensible loss to the air reference and the latent loss from last
                // tick's evaporation. Unarmed (`surface_cooling` None) the sensible and latent terms are zero, so
                // `surface_balance_temperature` returns the same `radiative_equilibrium` as before, byte-identical.
                // The air reference `T_air = (absorbed/sigma)^(1/4)` is the effective radiating temperature of the
                // same absorbed flux (emissivity one, Option A), independent of the surface it exchanges with; the
                // latent flux reads the PREVIOUS tick's evaporation (this step runs before `step_hydrology`), a
                // one-tick lag invisible under the field's relaxation.
                let (cooling_h, t_air, q_latent) = match &self.surface_cooling {
                    Some(sc) => {
                        let t_air = civsim_physics::laws::radiative_equilibrium(
                            absorbed,
                            Fixed::ONE,
                            sky.sigma,
                            sky.t_max,
                        );
                        let e_prev = self.evaporation.get(i).copied().unwrap_or(Fixed::ZERO);
                        let q_latent = e_prev.checked_mul(sc.latent_heat).unwrap_or(Fixed::MAX);
                        (sc.convective_h, t_air, q_latent)
                    }
                    None => (Fixed::ZERO, Fixed::ZERO, Fixed::ZERO),
                };
                self.solar_baseline[i] = civsim_physics::laws::surface_balance_temperature(
                    absorbed,
                    sky.emissivity,
                    sky.sigma,
                    sky.t_max,
                    cooling_h,
                    t_air,
                    q_latent,
                );
                // The per-material THERMAL INERTIA (follow-on 2): the cell's own soil moisture and standing-water
                // depth, with its current temperature (the freeze read), blend the substrate with water or ice,
                // and the factor (one for bone-dry land, below one for damp or water-laden cells) scales how fast
                // the field relaxes and diffuses, so a water-laden cell lags and swings less than dry land. A dry
                // cell reads one, so its dynamics are unchanged. The frozen state reads the field's current
                // temperature, a one-tick lag the deterministic replay preserves.
                self.solar_inertia[i] = sky.surface.inertia_factor(
                    self.water.at(x, y),
                    self.moisture[i],
                    temp.at(x, y),
                );
            }
        }
        self.diurnal_tick = self.diurnal_tick.saturating_add(1);
    }

    /// Whether the diurnal insolation cycle is armed (day-night arc), so the runner knows to copy
    /// [`Self::solar_baseline_at`] into the temperature field's relaxation baseline each tick.
    pub fn is_diurnal_armed(&self) -> bool {
        self.sky.is_some()
    }

    /// Copy the armed diurnal SOLAR BASELINE into the temperature field's relaxation baseline (day-night arc,
    /// the heat coupling). A no-op when the cycle is unarmed, so an unarmed run leaves the field's baseline
    /// untouched and is byte-identical. Called by the runner each tick after [`Self::step`] has computed the
    /// baseline, keeping the runner edit to one line; the field then relaxes toward the cycling baseline and the
    /// diurnal swing and thermal lag emerge from its own relaxation-plus-diffusion (bounded by the maximum
    /// principle), never authored here.
    pub fn apply_diurnal_baseline(&self, field: &mut Field) {
        if !self.is_diurnal_armed() {
            return;
        }
        for y in 0..self.height {
            for x in 0..self.width {
                field.set_baseline_at(x, y, self.solar_baseline_at(x, y));
            }
        }
    }

    /// The per-cell solar-forcing baseline TEMPERATURE the armed diurnal drive computed this tick (day-night
    /// arc), for the runner to copy into the temperature field's relaxation baseline. Reads zero for an
    /// off-grid cell or an unarmed run (the field baseline then stands unchanged).
    pub fn solar_baseline_at(&self, x: i32, y: i32) -> Fixed {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return Fixed::ZERO;
        }
        self.solar_baseline
            .get(self.idx(x, y))
            .copied()
            .unwrap_or(Fixed::ZERO)
    }

    /// Copy the armed diurnal per-cell THERMAL-INERTIA factor into the temperature field (follow-on 2), so the
    /// field's relaxation-plus-diffusion step scales by it and a water-laden cell lags and swings less than dry
    /// land. A no-op when the cycle is unarmed (the field's inertia stays absent and its step is byte-identical
    /// to the uniform pre-follow-on kernel). Called by the runner each tick after [`Self::step`], the sibling of
    /// [`Self::apply_diurnal_baseline`]; a dry cell's factor is one, so only water-laden cells change.
    pub fn apply_diurnal_inertia(&self, field: &mut Field) {
        if !self.is_diurnal_armed() {
            return;
        }
        field.set_inertia_from(&self.solar_inertia);
    }

    /// The per-cell thermal-inertia factor the armed diurnal drive computed this tick (follow-on 2): one for
    /// dry land, below one for a water-laden cell (slower, lagging). Reads one for an off-grid cell or an
    /// unarmed run (no slowing, the byte-neutral baseline).
    pub fn solar_inertia_at(&self, x: i32, y: i32) -> Fixed {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return Fixed::ONE;
        }
        self.solar_inertia
            .get(self.idx(x, y))
            .copied()
            .unwrap_or(Fixed::ONE)
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
    /// ([`civsim_foundation::decompose`]). Reads nothing dynamic and mutates nothing, so exposing it changes no state
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
                let delta = earthwork.total_delta(Coord3::ground(x, y));
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
        self.step_insolation(temp);
        self.step_hydrology(temp, calib);
        self.step_salinity(calib);
        self.step_productivity(temp, calib);
    }

    /// The salinity stencil (base-level liveliness step 4): weather salt into every cell, then advect it
    /// downhill with the water routing (the same precomputed lowest-neighbour targets the hydrology uses),
    /// double-buffered and conservative except at map-edge outflow, then cap. Salt accumulates in
    /// endorheic basins (which route to themselves, so they retain all their salt) and washes from
    /// well-drained cells, so a basin whose water evaporates concentrates its salt into a salt flat. The
    /// concentration a being suffers is derived in [`Self::salinity_dose`] from this salt and the standing
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
    // @derives[hydrology_water]: local water presence, rainfall, evaporation, runoff <- Clausius-Clapeyron saturation(local temperature) + Dalton evaporation + condensation where moisture exceeds saturation + downhill routing to the lowest neighbour. Water is NOT authored per cell; it falls out of temperature and terrain.
    fn step_hydrology(&mut self, temp: &Field, calib: &EnvironCalib) {
        let (w, h) = (self.width, self.height);
        let n = (w as usize) * (h as usize);
        // (1) Precipitation and evaporation, pointwise into a sourced buffer.
        let mut sourced = vec![Fixed::ZERO; n];
        // Store this tick's evaporation flux per cell so the diurnal surface balance can read it NEXT tick as the
        // latent cooling flux (the one-tick lag; `step_insolation` runs before this step). Sized on first use.
        if self.evaporation.len() != n {
            self.evaporation = vec![Fixed::ZERO; n];
        }
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
                // Precipitation: the moisture over saturation condenses (cold cells, low e_s). FOLLOW-ON (flagged,
                // not faked): this branch still compares the ground `moisture_content` to `e_s` directly, the same
                // substance-quantity conflation the evaporation coupling below retires. A correct precipitation
                // driver needs a DYNAMIC `fluid.vapor_pressure` transport state (that evaporation feeds and
                // condensation draws from), which does not exist yet (`self.moisture` is a static worldgen input),
                // the deeper water-cycle arc; left as-is here under the gate's scope ruling.
                let excess = moist - e_s;
                let precip = if excess > Fixed::ZERO {
                    calib.precip_rate.mul(excess)
                } else {
                    Fixed::ZERO
                };
                // Evaporation: the Dalton flux over the vapour-pressure deficit. The ambient vapour pressure is
                // DERIVED from the ground moisture (the soil-moisture-availability, or alpha, evapotranspiration
                // proxy): the ground wetness fraction sets the near-surface relative humidity, so `e_ambient` is the
                // saturation vapour pressure scaled by wetness, `moist * e_s`, and the deficit
                // `e_s - e_ambient = e_s * (1 - moist)` is a proper vapour-pressure deficit in MPa, non-negative,
                // scaling with dryness (bone-dry evaporates at the full deficit, saturated ground evaporates
                // nothing). This turns `fluid.moisture_content` (dimensionless) into the `fluid.vapor_pressure` (MPa)
                // the Dalton law's `e_ambient` port is floor-bound to read, retiring the mismatch that fed a wetness
                // fraction into a pressure port and zeroed the evaporation everywhere.
                let e_ambient = moist.checked_mul(e_s).unwrap_or(Fixed::ZERO);
                let evap = laws::evaporation_rate(
                    e_ambient,
                    e_s,
                    Fixed::ZERO,
                    calib.evap_a_still,
                    calib.evap_b_wind,
                    calib.evap_max,
                );
                self.evaporation[i] = evap;
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

    /// The ABIOTIC mineral-weathering floor (the matter-cycle completion, #156): rock dissolves to soil nutrient
    /// MAP-WIDE at a rate that DERIVES from each cell's own WETNESS, times a reserved base mineral-dissolution
    /// rate. Chemical weathering is hydrolysis, so it scales with the standing water a cell holds relative to its
    /// basin capacity (`water / max_water_depth`, clamped): a WET marsh cell weathers strongly, a dry ridge
    /// barely. The base rate is mineral-agnostic and reserved-with-basis, because the worldgen carries no
    /// per-cell lithology to read the parent-rock composition from (the geology arc is the flagged follow-on);
    /// the temperature (Arrhenius) coupling is likewise the deferred follow-on, like the thermal tent's `exp`, so
    /// a warm marsh does not gate on it. So the soil is fertile from GEOLOGY before any biomass (primary
    /// succession on weathered rock), and the biotic loop (plants grow, die, decompose back to soil) then
    /// sustains it. Deposits the `bio.organic_residue` class the producer soil-draw and the fertility read both
    /// key on. A legitimate out-of-ledger boundary source (rock into nutrient), run OUTSIDE the
    /// `step_matter_cycle` conservation bracket (the decomposition legs stay conservative; runner.rs). Pure
    /// deterministic fold in canonical row-major order (Principle 3); a zero base or zero basin capacity is a
    /// no-op, so an unarmed run is byte-identical.
    // @derives[weathering_soil_nutrient]: abiotic mineral-weathering soil-nutrient supply (the matter-cycle completion, rock -> soil nutrient before any biomass) <- a reserved mineral-agnostic base dissolution rate x the cell's own WETNESS (standing water / basin capacity), the hydrolysis coupling; a wet marsh cell weathers strongly and a dry cell not at all. The base is reserved (no per-cell lithology exists to read the parent-rock composition from, the geology arc the follow-on) and the temperature-Arrhenius coupling is deferred, but the wetness scaling DERIVES from the water field, not authored per cell.
    pub fn weather_minerals(
        &self,
        soil: &mut SoilNutrientField,
        calib: &EnvironCalib,
        base_rate: Fixed,
        class: &str,
    ) {
        if base_rate <= Fixed::ZERO || calib.max_water_depth <= Fixed::ZERO {
            return;
        }
        for y in 0..self.height {
            for x in 0..self.width {
                let wet = self
                    .water
                    .at(x, y)
                    .div(calib.max_water_depth)
                    .clamp(Fixed::ZERO, Fixed::ONE);
                let rate = base_rate.checked_mul(wet).unwrap_or(Fixed::MAX);
                if rate > Fixed::ZERO {
                    soil.deposit(Coord3::ground(x, y), class, rate);
                }
            }
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
    /// biosphere-BALANCE calibration the `dawn_harness.rs` T3 owner-gate still holds (whether the grazers THRIVE on
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
        temp: &Field,
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
                    // The stock-to-biomass conversion resolves the Arc-5 precedence (the Nernst redox derivation
                    // over the segment-2 per-source value over the registry-global; byte-neutral for Earth, which
                    // declares none of the first two): an ARMED Nernst redox source reads its couple's ACTUAL
                    // concentrations at the cell (donor = this source's own supply, acceptor = its named field or
                    // unit activity, the cell temperature driving the k_B*T/q factor and the dE0/dT shift), so its
                    // yield FALLS as the couple depletes and crosses zero at its own equilibrium; an unarmed redox
                    // source uses the standard concentration-independent EMF, a per-source override its own rate,
                    // else the soil-derived global (each byte-identical to before).
                    let bps = {
                        let temperature = temp.at(x, y);
                        let acceptor_conc =
                            match binding.redox_emf.as_ref().and_then(|rx| rx.acceptor_field) {
                                Some(AbioticField::Light) => self.light[i],
                                Some(AbioticField::Water) => self.water.at(x, y),
                                Some(AbioticField::Soil) => soil.mass(coord, &binding.class),
                                Some(AbioticField::DataScalar(fid)) => self
                                    .data_fields
                                    .get(&fid)
                                    .map(|f| f.at(x, y))
                                    .unwrap_or(Fixed::ONE),
                                None => Fixed::ONE, // a buffered/abundant acceptor: unit activity (the standard case)
                            };
                        registry
                            .redox_conversion_at(binding, supply, acceptor_conc, temperature)
                            .unwrap_or_else(|| registry.effective_conversion(binding))
                    };
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
                    // The reversible-flux DRAW (increment 2): an armed redox source WITH kinetics draws its own
                    // uptake flux (Hill-saturating, driven by the couple's Nernst EMF, Vmax = kcat * the being's
                    // catalyst tissue, min(v, S) conserved) over its own stock, so a depleting couple's draw
                    // saturates and falls with the drive. Any other source (non-redox, unarmed, or no kinetics)
                    // keeps the segment-2 capacity-proportional draw exactly (byte-identical). The catalyst tissue
                    // is the producer's own composition amount for the kinetics' named class (default bio.protein,
                    // a per-source datum so an alien names its own catalyst class; the fuller catalyst axis is the
                    // flagged R-SOURCE-VECTOR follow-on). The per-couple clamp (the Nernst EMF zeroed below the
                    // couple's own equilibrium) is the correct form for these INDEPENDENT couples; the net-across-a-
                    // thermodynamically-coupled-set clamp is a follow-on for when shared-intermediate coupling is
                    // modelled (there is no coupling substrate yet).
                    let draw_amt = {
                        let temperature = temp.at(x, y);
                        let acceptor_conc =
                            match binding.redox_emf.as_ref().and_then(|rx| rx.acceptor_field) {
                                Some(AbioticField::Light) => self.light[i],
                                Some(AbioticField::Water) => self.water.at(x, y),
                                Some(AbioticField::Soil) => soil.mass(coord, &binding.class),
                                Some(AbioticField::DataScalar(fid)) => self
                                    .data_fields
                                    .get(&fid)
                                    .map(|f| f.at(x, y))
                                    .unwrap_or(Fixed::ONE),
                                None => Fixed::ONE,
                            };
                        let stock = match binding.field {
                            AbioticField::Light => Fixed::ZERO,
                            AbioticField::Soil => soil.mass(coord, &binding.class),
                            AbioticField::Water => self.water.at(x, y),
                            AbioticField::DataScalar(fid) => self
                                .data_fields
                                .get(&fid)
                                .map(|f| f.at(x, y))
                                .unwrap_or(Fixed::ZERO),
                        };
                        let catalyst_tissue = binding
                            .kinetics
                            .as_ref()
                            .map(|kin| {
                                self.producer_food[i]
                                    .as_ref()
                                    .and_then(|f| f.get(&kin.catalyst_class).copied())
                                    .unwrap_or(Fixed::ZERO)
                            })
                            .unwrap_or(Fixed::ZERO);
                        match registry.redox_draw_flux(
                            binding,
                            stock,
                            acceptor_conc,
                            temperature,
                            catalyst_tissue,
                        ) {
                            Some(flux) => flux,
                            None => {
                                let bps = registry.effective_conversion(binding);
                                match binding.stock_per_biomass {
                                    Some(spb) => {
                                        draw_biomass.checked_mul(spb).unwrap_or(Fixed::ZERO)
                                    }
                                    None => draw_biomass.checked_div(bps).unwrap_or(Fixed::ZERO),
                                }
                            }
                        }
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
    // @derives[productivity_capacity]: per-cell biomass productivity / carrying capacity <- when photosynthesis is armed, the DERIVED carbon-fixation rate (carbon_fixation_rate: the light-response over the real insolation flux, the enzyme thermal tent, the water-use-efficiency coupling to evaporation, and the soil-nutrient limitation over matter-cycle fertility); unarmed, the abstract-producer Liebig minimum over (water, light, temperature, soil) with soil = soil_baseline + matter-cycle fertility (the interim the derivation retires). Productivity is NOT authored per cell.
    fn step_productivity(&mut self, temp: &Field, calib: &EnvironCalib) {
        let (w, h) = (self.width, self.height);
        // The DERIVED photosynthesis path (armed, #156): read the calibration and the stellar-constant flux anchor
        // out as owned Copy values BEFORE the loop, so the per-cell field reads do not hold a borrow of `self`
        // across the `self.capacity.cells[i]` write. `None` unless BOTH the photosynthesis and the diurnal sky are
        // armed (the derivation needs the flux anchor), so an unarmed run keeps the Liebig `biomass_from` path and
        // stays byte-identical.
        let derived = match (&self.photosynthesis, &self.sky) {
            (Some(photo), Some(sky)) => Some((*photo, sky.solar_constant)),
            _ => None,
        };
        for y in 0..h {
            for x in 0..w {
                let i = self.idx(x, y);
                let climate = match derived {
                    // The armed derivation: the carbon-fixation rate over the real insolation flux (normalised
                    // light times the stellar constant), the enzyme thermal tent, the water-use-efficiency
                    // coupling to the evaporation flux, and the soil-nutrient limitation over the matter-cycle
                    // fertility. Retires the flat `soil_baseline` (soil supply IS the fertility) and the authored
                    // water/light/temperature requirements on this path.
                    // a-prime joules bridge (#42): the carbon-fixation FLUX (W/m^2) integrated over the cell area
                    // (and the 1 s tick, folded as unity) yields the cell's per-tick fixed-carbon ENERGY in joules,
                    // so the standing food a grazer eats and its Kleiber drain compare in the same joules rather
                    // than a flux-versus-energy scale gap papered over by the retired `food_energy_density` anchor.
                    Some((photo, solar_constant)) => carbon_fixation_rate(
                        self.light[i],
                        solar_constant,
                        temp.at(x, y),
                        self.evaporation.get(i).copied().unwrap_or(Fixed::ZERO),
                        self.water.cells[i],
                        self.fertility[i],
                        calib.soil_req,
                        &photo,
                    )
                    .checked_mul(photo.cell_area_m2)
                    .unwrap_or(Fixed::MAX),
                    // The unarmed Liebig interim (byte-identical): the abstract-producer minimum over the four
                    // satisfactions, the soil supply the flat baseline plus the matter-cycle fertility.
                    None => biomass_from(
                        self.water.cells[i],
                        self.light[i],
                        temp.at(x, y),
                        calib.soil_baseline.saturating_add(self.fertility[i]),
                        calib,
                    ),
                };
                self.capacity.cells[i] = match derived {
                    // ARMED: the derived carbon-fixation rate IS the productivity everywhere (the land's
                    // photosynthetic potential), so a located producer's ENERGY derives from its own cell's
                    // fixation like any cell; its nutrient COMPOSITION still rides `producer_food` /
                    // `set_real_composition` in `regrow_supply`. The static `producer[i]` biomass is retired as
                    // the energy source on this path (retiring the static-T3 energy the derivation replaces).
                    Some(_) => climate,
                    // UNARMED (byte-identical): where a real producer organism stands (biosphere-into-run), its
                    // located biomass is the food ceiling; elsewhere the abstract climate productivity is the
                    // baseline (the stand-in for unmodelled background vegetation). An all-zero producer takes
                    // the climate branch on every cell, so the capacity and its hash are byte-unchanged.
                    None => {
                        if self.producer[i] > Fixed::ZERO {
                            self.producer[i]
                        } else {
                            climate
                        }
                    }
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
        // Whether the DERIVED photosynthesis productivity is armed (and the sky supplies the flux anchor): on the
        // armed path the food capacity is an absolute fixed-energy value, so a bare cell's energy food is marked
        // real (eaten at `content = supply`) rather than double-scaled through the `food_energy_density` anchor.
        let photosynthesis_armed = self.photosynthesis.is_some() && self.sky.is_some();
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
                // CORRECTED-T3: mark, PER CLASS, which of this cell's nutrient classes carry a real producer
                // composition (their supply is already the physical content at the plant's own per-substance
                // density), so the forage INGEST eats each at `content = supply` rather than double-scaling through
                // the food_energy_density anchor. The marked set is exactly the producer_food axes: it EXCLUDES the
                // water mirror and the salinity dose written below, so those non-composition axes keep the anchor on
                // a producer cell just as on a bare cell (a per-cell flag would wrongly strip the anchor off the
                // water axis, the §9 correctness lens's catch). A cell with no producer_food marks nothing
                // (byte-identical for a run that seeds no producer food, so the four tracked pins hold).
                match food {
                    Some(fc) => resource.set_real_composition(coord, fc.keys().cloned()),
                    // ARMED (the derived productivity, #156): a cell with no located producer still grows its food
                    // toward the DERIVED carbon-fixation capacity, which is already an absolute fixed-energy value,
                    // so mark its energy class real so the forage INGEST eats it at `content = supply` and does NOT
                    // double-scale through the `food_energy_density` anchor (retiring that scalar on the armed
                    // energy path; the water and salinity axes below stay unmarked and keep the anchor).
                    None if photosynthesis_armed => resource
                        .set_real_composition(coord, std::iter::once(ENERGY_DENSITY.to_string())),
                    None => resource.set_real_composition(coord, std::iter::empty()),
                }
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

/// The DERIVED photosynthesis calibration: the measured per-producer constants the carbon-fixation derivation
/// reads (the photosynthesis-to-productivity arc, #156). Each is a per-substance MEASURED datum surfaced
/// reserved-with-basis (Principle 11), refutable in a lab without this simulator, never an ecology knob; a
/// different world's producer, or an evolved lineage, is a data row (admit-the-alien). These REPLACE the authored
/// abstract-producer half-saturation constants (`productivity.water/light/temperature_requirement`).
#[derive(Clone, Copy, Debug)]
pub struct PhotosynthesisCalib {
    /// The photosynthetic QUANTUM YIELD (the initial slope of the light-response curve): the fraction of
    /// incident radiant energy fixed as chemical energy at low, light-limited irradiance. RESERVED. Basis: the
    /// measured maximum quantum efficiency of the producer's photosystem (a C3 leaf fixes on the order of a few
    /// percent of incident energy at low light; Taiz and Zeiger, Plant Physiology).
    pub quantum_yield: Fixed,
    /// The LIGHT-SATURATION IRRADIANCE (in the same physical flux units as `insolation * solar_constant`): the
    /// irradiance at which fixation stops rising with light (the enzyme- or CO2-limited plateau begins).
    /// RESERVED. Basis: the measured light-saturation point of the producer's photosynthesis, well below full sun
    /// for C3 vegetation (Taiz and Zeiger; Larcher, Physiological Plant Ecology).
    pub light_saturation: Fixed,
    /// The carbon-fixing enzyme's THERMAL OPTIMUM (an absolute temperature): where the fixation rate peaks.
    /// RESERVED. Basis: the measured optimum temperature of the producer's carboxylating enzyme (about 298 K for
    /// temperate C3 vegetation; Berry and Bjorkman 1980, Annu. Rev. Plant Physiol.).
    pub temp_optimum: Fixed,
    /// The carbon-fixing enzyme's THERMAL BREADTH (the half-width of its thermal-performance curve): the
    /// temperature departure from the optimum at which fixation falls to zero. RESERVED. Basis: the measured
    /// thermal breadth of the producer's photosynthetic temperature response (Berry and Bjorkman 1980; Larcher).
    /// The piecewise-linear TENT over (optimum, breadth) approximates the measured thermal-performance curve; the
    /// exact Johnson-Lewin activation-times-deactivation form is reserved for when the `Fixed::exp` kernel is
    /// canon-pinned (R-GPU-CANON-PIN), the flagged follow-on.
    pub temp_breadth: Fixed,
    /// The WATER-USE EFFICIENCY coupling (the water a producer must transpire per unit fixation at its potential
    /// rate): the coefficient turning the cell's evaporative demand into the water requirement. RESERVED. Basis:
    /// the measured transpiration efficiency of the producer (carbon fixed per water transpired; the water-use-
    /// efficiency literature). The derived water requirement is `water_use_efficiency * evaporative_demand`,
    /// retiring the flat `productivity.water_requirement`.
    pub water_use_efficiency: Fixed,
    /// The CELL AREA (m^2): the spatial scale over which the fixation FLUX is integrated to yield the cell's
    /// per-tick fixed-carbon ENERGY (`E_food = fixation_flux [W/m^2] * cell_area [m^2] * tick [s]`, the a-prime
    /// joules bridge, #42), so the standing food a grazer eats and its Kleiber drain compare in the same joules.
    /// RESERVED, a per-world SPATIAL datum (not a photosynthesis property, carried here as its only consumer).
    /// Basis: the tile edge squared, and the tile edge is fixed by one real creature's walking speed and the tick
    /// (`tile_edge = real_speed / base_speed` at the 1 s base tick, so about 1.4 m and a cell of about 2 m^2;
    /// `locomotion.rs:88`). The tick is the 1 s base, folded in as unity; a world with a different tick carries it
    /// separately, the flagged follow-on.
    pub cell_area_m2: Fixed,
}

impl PhotosynthesisCalib {
    /// A LABELLED DEV FIXTURE standing up Earth-like C3 magnitudes for the tests and harness paths, not owner
    /// values: a quantum yield of a twentieth (a few percent of incident energy fixed at low light), a light
    /// saturation of 300 (well below the ~1361 stellar constant, so C3 saturates below full sun), a thermal
    /// optimum of 298 K and a breadth of 30 K (the temperate-C3 photosynthetic temperature response), and a unit
    /// water-use efficiency. The shipped values read from the manifest at arming; these only run in tests.
    pub fn dev_fixture() -> PhotosynthesisCalib {
        PhotosynthesisCalib {
            quantum_yield: Fixed::from_ratio(1, 20),
            light_saturation: Fixed::from_int(300),
            temp_optimum: Fixed::from_int(298),
            temp_breadth: Fixed::from_int(30),
            water_use_efficiency: Fixed::ONE,
            cell_area_m2: Fixed::from_int(2),
        }
    }

    /// Read the photosynthesis calibration fail-loud from the manifest (Principle 11): each measured constant is
    /// a reserved value that refuses to build while unset, never a silent default. The manifest home is the
    /// `photosynthesis.*` keys, surfaced with their basis in the reserved manifest.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<PhotosynthesisCalib, CalibrationError> {
        Ok(PhotosynthesisCalib {
            quantum_yield: m.require_fixed("photosynthesis.quantum_yield")?,
            light_saturation: m.require_fixed("photosynthesis.light_saturation")?,
            temp_optimum: m.require_fixed("photosynthesis.temperature_optimum")?,
            temp_breadth: m.require_fixed("photosynthesis.temperature_breadth")?,
            water_use_efficiency: m.require_fixed("photosynthesis.water_use_efficiency")?,
            cell_area_m2: m.require_fixed("photosynthesis.cell_area_m2")?,
        })
    }
}

/// The DERIVED per-cell CARBON-FIXATION RATE (net primary productivity), the photosynthesis substrate that
/// RETIRES the authored Liebig `biomass_from`. It is the Liebig MINIMUM (the limiting factor sets the rate, the
/// same min-fold the biome-fit uses) over four unit-interval factors, scaled by the light-saturated ABSOLUTE
/// rate `p_max`. The rate `p_max` is the product of the quantum yield and the light saturation, so the absolute
/// scale DERIVES from the stellar-constant flux anchor and the measured efficiency, with no owner net-primary-
/// productivity number. The light factor is `I / (I + light_saturation)`, the saturating light limitation, with
/// the real irradiance `I` the normalised insolation times the solar constant, so at low light the rate follows
/// the quantum-yield slope and at high light it saturates to `p_max`. The temperature factor is the piecewise-
/// linear TENT `max(0, 1 - |T - optimum| / breadth)` approximating the carbon-fixing enzyme's measured thermal-
/// performance. The water factor is the water limitation whose requirement DERIVES from the evaporative demand
/// and the measured water-use-efficiency, retiring the flat `productivity.water_requirement`. The soil factor is
/// the nutrient limitation over the matter-cycle fertility, retiring the flat `soil_baseline` (the lithology-
/// mineral derivation the follow-on). Every input is floor physics (the insolation flux, the surface temperature,
/// the evaporative demand, the matter-cycle fertility) or a measured per-producer constant, so no free ecology
/// knob enters.
// @derives[carbon_fixation_rate]: per-cell carbon-fixation rate / net primary productivity <- the photosynthesis light-response
//   (quantum yield x light-saturation over the real insolation flux = normalised light x the solar_constant
//   floor-unit pin, itself derivable from L/(4 pi d^2)), the carbon-fixing enzyme thermal-performance tent
//   (measured optimum, breadth), the water limitation (water-use-efficiency x evaporative demand), and the
//   soil-nutrient limitation (matter-cycle fertility). Retires the authored productivity.*_requirement and the
//   flat soil_baseline; the measured photosynthetic constants are reserved-with-basis.
#[allow(clippy::too_many_arguments)]
pub fn carbon_fixation_rate(
    insolation_normalised: Fixed,
    solar_constant: Fixed,
    temperature: Fixed,
    evaporative_demand: Fixed,
    water_supply: Fixed,
    soil_fertility: Fixed,
    soil_requirement: Fixed,
    photo: &PhotosynthesisCalib,
) -> Fixed {
    // The real irradiance: the normalised daylit insolation scaled by the stellar-constant flux anchor.
    let irradiance = insolation_normalised
        .checked_mul(solar_constant)
        .unwrap_or(Fixed::MAX);
    // The absolute light-saturated rate: the measured quantum yield times the light-saturation irradiance, so the
    // absolute scale derives from the flux anchor and the measured efficiency (no owner NPP number).
    let p_max = photo
        .quantum_yield
        .checked_mul(photo.light_saturation)
        .unwrap_or(Fixed::MAX);
    // light_fraction = I / (I + I_sat), the saturating light limitation in [0, 1].
    let denom = irradiance.saturating_add(photo.light_saturation);
    let light_fraction = if denom > Fixed::ZERO {
        irradiance.div(denom).clamp(Fixed::ZERO, Fixed::ONE)
    } else {
        Fixed::ZERO
    };
    // temp_factor: the piecewise-linear tent over the enzyme optimum and breadth (a zero breadth reads a
    // knife-edge, non-optimum temperatures fixing nothing).
    let temp_factor = if photo.temp_breadth > Fixed::ZERO {
        let dist = (temperature - photo.temp_optimum).abs();
        (Fixed::ONE - dist.div(photo.temp_breadth)).clamp(Fixed::ZERO, Fixed::ONE)
    } else if temperature == photo.temp_optimum {
        Fixed::ONE
    } else {
        Fixed::ZERO
    };
    // water_factor: the water limitation, its requirement derived from the evaporative demand and the WUE.
    let water_req = photo
        .water_use_efficiency
        .checked_mul(evaporative_demand)
        .unwrap_or(Fixed::MAX);
    let water_factor = laws::satisfaction(water_supply, Fixed::ONE, Some(water_req));
    // soil_factor: the nutrient limitation over the matter-cycle fertility.
    let soil_factor = laws::satisfaction(soil_fertility, Fixed::ONE, Some(soil_requirement));
    // The Liebig minimum over the four factors, scaled by the absolute light-saturated rate.
    let limiting = light_fraction
        .min(temp_factor)
        .min(water_factor)
        .min(soil_factor);
    p_max.checked_mul(limiting).unwrap_or(Fixed::ZERO)
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

/// A world's DATA sky for the diurnal insolation drive (Arc, day-night). One entry per star, plus the world's
/// axial tilt and its rotation and orbital cadences in ticks. The default is the ZERO-OBLIQUITY SINGLE-STAR
/// REFERENCE world (one star of unit luminosity, tilt 0), NOT Mirror: Mirror is Earth at its real 23.4-degree
/// obliquity (real seasons on top of day-night), a data-row override that sets `obliquity` and per-star data.
/// A tidally-locked, high-obliquity, or binary-star world is likewise a data row (Principles 8, 11). The
/// membership is data and grows with the world; the sun-angle law is fixed Rust.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Star {
    /// The star's radiant flux delivered to the world (its luminosity attenuated by the inverse-square of its
    /// orbital distance), the L_s in the insolation sum. RESERVED, owner-set from real astronomical data; the
    /// reference world uses unit flux so the daylit peak is 1.
    pub luminosity: Fixed,
    /// The star's own orbital-phase offset in `[0, 1)`, so a binary or trinary system's suns rise and set on
    /// their own cadences rather than sharing one world phase. Zero for the single-star reference.
    pub phase_offset: Fixed,
}

/// The world's SURFACE-MATERIAL thermal data for the per-material heat effect (day-night arc, follow-on 2):
/// the volumetric heat capacities (`rho * c_p`, the `mat.density * therm.specific_heat` floor product) of the
/// three surface thermal materials a cell blends by its REAL state, and the water freezing temperature. Fixed
/// Rust mechanism, data membership (Principle 11): the mechanism blends and derives the per-cell thermal
/// inertia, these values are per-world data. The membership is deliberately the three the cell's own state can
/// distinguish without a lookup: the dry SUBSTRATE, standing WATER, and its frozen form ICE, blended by the
/// cell's water fraction and freeze state (the derive-from-real-state form the gate ruled). The full per-cell
/// lithology (rock versus sand versus clay varying the dry substrate) is a separate flagged substrate arc
/// (the geodynamics per-column lithology field), not faked here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceThermal {
    /// The dry substrate's volumetric heat capacity `rho * c_p` (J/(m^3*K)). The REFERENCE material: a
    /// bone-dry cell reads exactly this, so its inertia factor is one and its heat dynamics are unchanged.
    /// RESERVED, owner-set from the floor `mat.density * therm.specific_heat` of the world's default dry
    /// surface material (dev fixture: dry mineral soil, ~1.2e6, CRC soil-property tables).
    pub substrate: Fixed,
    /// Standing WATER's volumetric heat capacity `rho * c_p` (J/(m^3*K)). Larger than the substrate, so a
    /// water-laden cell (an ocean or a filled basin) has a higher thermal inertia and lags: the ocean's small
    /// diurnal swing versus dry land's large one, emergent from the cell's own water content. RESERVED, from
    /// the floor water profile (dev fixture: ~4.182e6, water rho 1000 * c_p 4182, CRC).
    pub water: Fixed,
    /// ICE's volumetric heat capacity `rho * c_p` (J/(m^3*K)), read for a cell whose water has frozen (its
    /// temperature is below `freeze_temp`), so a frozen surface lags by ice's inertia rather than water's.
    /// RESERVED, from the floor ice profile (dev fixture: ~1.919e6, ice rho 917 * c_p 2093, CRC).
    pub ice: Fixed,
    /// The water FREEZING temperature (K): below it a cell's standing water is ice (the `therm.melting_temperature`
    /// floor phase boundary, read as data, not an authored branch). RESERVED, from the floor water melting point
    /// (dev fixture: 273.15 K, Mirror's real value).
    pub freeze_temp: Fixed,
    /// The standing-water depth at which a cell reads HALF water inertia, the half-saturation of the water
    /// fraction `w = depth / (depth + water_reference)` (a Michaelis saturation, the same shape the salinity
    /// dilution uses): a dry cell (no standing water) is all substrate, a deep basin or ocean saturates toward
    /// all water. RESERVED, owner-set against the hydrology's standing-water depth scale (the depth marking a
    /// water-dominated cell); dev fixture at unit depth. Its own value, not the salinity dilution reference (a
    /// different quantity), so the thermal fraction keys off its own basis.
    pub water_reference: Fixed,
}

impl SurfaceThermal {
    /// The LABELLED dev-fixture surface thermal materials (Earth-like), the reserved values surfaced for the
    /// owner, not decided here: the dry-soil, water, and ice volumetric heat capacities, the water freezing
    /// point, and the standing-water half-saturation depth. A calibrated world reads the heat capacities from
    /// the floor `mat.density * therm.specific_heat` per material; an alien world (a methane ocean, an ammonia
    /// crust) is a data row.
    pub fn dev_fixture() -> SurfaceThermal {
        SurfaceThermal {
            substrate: Fixed::from_int(1_200_000),
            water: Fixed::from_int(4_182_000),
            ice: Fixed::from_int(1_919_000),
            freeze_temp: Fixed::from_ratio(27315, 100),
            water_reference: Fixed::ONE,
        }
    }

    /// The per-cell THERMAL INERTIA FACTOR (day-night arc, follow-on 2): the ratio of the dry substrate's
    /// volumetric heat capacity to the cell's own, `substrate / c_cell`, where the cell's heat capacity blends
    /// the substrate with its water (or ice, when frozen) by the cell's water FRACTION,
    /// `c_cell = (1 - w) * substrate + w * c_liquid`. The fraction combines the cell's TWO real water signals:
    /// its worldgen soil MOISTURE (`[0, 1]`, damp soil already holds thermal mass) and the Michaelis saturation
    /// of its standing-water DEPTH (`depth / (depth + water_reference)`, a lake or ocean over the top),
    /// `w = moisture + (1 - moisture) * standing`, so standing water saturates the remaining dry fraction: dry
    /// desert reads near zero (all substrate), damp soil lags, and a filled basin saturates toward all water. A
    /// bone-dry cell (`moisture = 0`, no standing water) reads exactly one, so scaling its heat dynamics by this
    /// factor leaves them unchanged; a water-laden cell reads below one, so its relaxation and diffusion slow
    /// and it lags and swings less (the ocean and damp-soil effect), emergent from the cell's real water content
    /// and temperature and the floor material data, no authored lookup. The factor is clamped at one (a dry cell
    /// is the fastest, the lowest heat capacity, which the material ordering guarantees and which also holds the
    /// diffusion stencil inside its stability bound). A pure fixed-point function of the cell's own state; the
    /// freeze read keys off the material's floor melting temperature, so a frozen surface lags by ice, never by
    /// a label (Principles 3, 8, 9).
    pub fn inertia_factor(&self, water_depth: Fixed, moisture: Fixed, cell_temp: Fixed) -> Fixed {
        // The standing-water saturation, the Michaelis fraction of the depth, in `[0, 1)`.
        let depth = water_depth.max(Fixed::ZERO);
        let denom = depth.saturating_add(self.water_reference);
        let standing = if denom > Fixed::ZERO {
            depth.div(denom).clamp(Fixed::ZERO, Fixed::ONE)
        } else {
            Fixed::ZERO
        };
        // The combined water fraction: soil moisture, with standing water saturating the remaining dry
        // fraction. A bone-dry cell (no moisture, no standing water) is zero, so its factor is exactly one.
        let moist = moisture.clamp(Fixed::ZERO, Fixed::ONE);
        let w = moist
            .saturating_add(
                (Fixed::ONE - moist)
                    .checked_mul(standing)
                    .unwrap_or(standing),
            )
            .clamp(Fixed::ZERO, Fixed::ONE);
        let c_liquid = if cell_temp < self.freeze_temp {
            self.ice
        } else {
            self.water
        };
        let dry = Fixed::ONE - w;
        let c_cell = dry
            .checked_mul(self.substrate)
            .unwrap_or(self.substrate)
            .saturating_add(w.checked_mul(c_liquid).unwrap_or(c_liquid));
        if c_cell <= Fixed::ZERO {
            return Fixed::ONE; // a degenerate zero heat capacity: no slowing (the substrate-absence convention).
        }
        self.substrate.div(c_cell).min(Fixed::ONE)
    }
}

/// The world's DATA sky: the star-list and the orbital geometry the diurnal insolation reads. Fixed Rust law,
/// data membership; the default is the zero-obliquity single-star reference world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiurnalSky {
    /// Ticks per SIDEREAL rotation (one spin against the fixed stars), the diurnal phase period.
    pub rotation_period_ticks: u64,
    /// Ticks per orbit, so the SYNODIC (solar) day derives from the sidereal spin minus the orbital advance,
    /// and the tidally-locked case (rotation equals orbit) comes out as a permanent day face.
    pub orbital_period_ticks: u64,
    /// The world's axial tilt in radians; 0 is the reference world (no seasons, poles dark). The declination
    /// derives from this and the orbital phase, so a tilted world gets seasons and polar day/night as data.
    pub obliquity: Fixed,
    /// The world's orbital ECCENTRICITY (the shape of its ellipse), 0 for a circular orbit. The delivered
    /// flux varies over the year by the inverse square of the orbit distance, which a Kepler solve derives
    /// from this and the orbital phase; 0 is the reference world (a circle, no distance variation, byte-neutral).
    /// Per-world data (Principle 11): Mirror carries Earth's near-circular ~0.0167, an eccentric world its own.
    pub eccentricity: Fixed,
    /// The PERIHELION PHASE: the orbital phase (in the seasonal cycle) at which the world is closest to its
    /// star, so the distance minimum need not coincide with the seasonal reference. This is the precession
    /// term: it decouples perihelion from the seasons, setting whether the eccentricity brightening falls in
    /// one hemisphere's summer or its winter (the seasonal-amplitude asymmetry). 0 puts perihelion at the
    /// reference equinox (the pre-precession behaviour, byte-neutral). Per-world data: Mirror carries Earth's
    /// value (perihelion ~13 days after the December solstice), an eccentric world its own; the slow drift of
    /// this phase over deep time is the Milankovitch precession follow-on.
    pub perihelion_phase: Fixed,
    /// The data star-list. One unit-luminosity star at zero phase offset is the single-star reference.
    pub stars: Vec<Star>,
    /// The physical stellar flux scale (W/m^2) that turns the normalised daylit insolation into an absorbed
    /// irradiance for the radiative surface-temperature baseline, so the heat path reads physical watts while
    /// the light path stays the normalised `[0, 1]` productivity signal. DERIVED (genesis-forward Stage 1)
    /// from the scenario-set stellar inputs through [`crate::astro::stellar_flux`]: `L_sun *
    /// (M_star/M_sun)^exponent / (4*pi*d^2)`. Mirror is a solar-mass star at one AU, so this is Earth's real
    /// ~1361.17 W/m^2; an alien world's mass, distance, or exponent lands its own value.
    pub solar_constant: Fixed,
    /// The per-world atmospheric BACK-RADIATION (downwelling longwave) floor (W/m^2), the night-side irradiance
    /// a surface still absorbs when the star is down, so the night baseline is `radiative_eq(back_radiation)`
    /// rather than absolute zero and the Moon-Earth-Venus diurnal-swing spectrum EMERGES from this one datum
    /// (airless 0, Earth mild, thick-atmosphere high). RESERVED, owner-set (Mirror = Earth's real downwelling
    /// longwave). The full derivation from the atmosphere's greenhouse optical depth is the flagged follow-on.
    pub back_radiation: Fixed,
    /// The per-world SHORTWAVE ALBEDO (the planetary Bond albedo), the fraction of the incoming stellar flux
    /// reflected rather than absorbed, so the absorbed solar term is `insolation * solar_constant * (1 -
    /// albedo)` (the longwave back-radiation floor is unaffected, it is not reflected). Without it a bare
    /// world absorbs the full stellar flux and equilibrates far too hot (a single-star world at 1361 W/m^2 with
    /// no reflection sits ~23 K above Earth's mean); Earth's ~0.30 Bond albedo is what lands its equilibrium at
    /// ~288 K. RESERVED per-world DATA (Principle 11): Mirror carries Earth's measured Bond albedo, an icy
    /// world a higher one, a dark world near zero. The reference world sets it to ZERO (a bare blackbody), so a
    /// world that does not declare an albedo is byte-identical to the pre-albedo baseline. The full per-cell,
    /// per-material albedo (ice, ocean, desert reflecting differently, from the floor `opt.reflectance`) is the
    /// named follow-on; this is the uniform planetary value.
    pub albedo: Fixed,
    /// The surface EMISSIVITY the radiative-equilibrium baseline reads. INTERIM uniform (a flagged
    /// uniform-absorption limit): the per-material emissivity from the floor `opt.emissivity` (so ice, rock,
    /// water, and an alien crust equilibrate and lag differently) is the named immediate follow-on.
    pub emissivity: Fixed,
    /// The Stefan-Boltzmann constant sigma the radiative-equilibrium law reads, a universal physical constant
    /// DERIVED from the CODATA fundamentals ([`crate::physiology::derived_stefan_boltzmann`]), the same in
    /// every world (not a per-world reserved dev fixture).
    pub sigma: Fixed,
    /// The representability cap the radiative-equilibrium kernel clamps its output temperature to.
    pub t_max: Fixed,
    /// The world's SURFACE-MATERIAL thermal data (follow-on 2): the volumetric heat capacities the per-cell
    /// thermal-inertia factor blends by a cell's real water fraction and freeze state, so ice, water, and dry
    /// substrate lag and swing differently. Per-world data; the reference uses the dev fixture.
    pub surface: SurfaceThermal,
}

impl DiurnalSky {
    /// The zero-obliquity SINGLE-STAR REFERENCE world (not Mirror): one unit-luminosity star, no tilt. A clean
    /// pure-diurnal cycle. `rotation_period_ticks` is the day length in ticks (from the world's own rotation
    /// period through the seconds-to-ticks bridge); `orbital_period_ticks` its year.
    pub fn reference(rotation_period_ticks: u64, orbital_period_ticks: u64) -> DiurnalSky {
        // The STELLAR-SOURCE inputs, labelled dev fixtures surfaced for the owner (the same scenario-set
        // standing as the obliquity and albedo below): the star's mass as a fraction of the sun, the world's
        // orbital distance in AU, and the mass-luminosity exponent (a reserved closure-residue). The reference
        // world is a solar-mass star at one AU, so `solar_constant` DERIVES from these through the stellar-flux
        // kernel (Earth's real ~1361.17 W/m^2) rather than being an authored number. An alien world sets a
        // different mass, distance, or exponent (admit-the-alien: the derivation is fixed, the inputs are data).
        let star_mass_ratio = Fixed::ONE;
        let orbital_distance_au = Fixed::ONE;
        let mass_luminosity_exponent = Fixed::from_ratio(35, 10);
        let solar_constant = crate::astro::stellar_flux(
            star_mass_ratio,
            mass_luminosity_exponent,
            orbital_distance_au,
        )
        .expect("the stellar flux derives for a solar-mass star at one AU");
        DiurnalSky {
            rotation_period_ticks: rotation_period_ticks.max(1),
            orbital_period_ticks: orbital_period_ticks.max(1),
            obliquity: Fixed::ZERO,
            // The reference world is a CIRCULAR orbit (eccentricity 0): the distance never varies, so the
            // orbital-distance flux factor is exactly 1 and the reference world is byte-identical to the
            // pre-eccentricity baseline. Mirror overrides this with Earth's near-circular value.
            eccentricity: Fixed::ZERO,
            // The reference world's perihelion sits at the seasonal reference (phase 0): no precession offset,
            // so the perihelion-vs-solstice term is inert and byte-neutral. Mirror sets Earth's real phase.
            perihelion_phase: Fixed::ZERO,
            stars: vec![Star {
                luminosity: Fixed::ONE,
                phase_offset: Fixed::ZERO,
            }],
            // The stellar flux DERIVES (above) from the scenario-set stellar inputs, retiring the old inline
            // 1361 literal. The remaining LABELLED DEV FIXTURES (Earth-like), reserved and surfaced for the
            // owner, not decided here: a mild atmospheric back-radiation floor, a uniform surface emissivity
            // (the per-material floor read is the follow-on), and a representability cap. Mirror sets these to
            // Earth's real values; an airless world sets back_radiation to zero for a Moon-like plunge. Sigma
            // is NOT among them: it DERIVES from the CODATA fundamentals, universal, not reserved.
            solar_constant,
            back_radiation: Fixed::from_int(300),
            // The reference world is a bare blackbody (no reflection), so an unarmed-albedo world is
            // byte-identical to the pre-albedo baseline. Mirror overrides this with Earth's Bond albedo.
            albedo: Fixed::ZERO,
            emissivity: Fixed::from_ratio(95, 100),
            sigma: crate::physiology::derived_stefan_boltzmann(),
            t_max: Fixed::from_int(500),
            surface: SurfaceThermal::dev_fixture(),
        }
    }

    /// The MIRROR world's sky: Earth's real single sun and, above all, Earth's real axial tilt, so REAL
    /// SEASONS ride on top of the day-night cycle through the declination term already in the sun-angle law.
    /// Identical to `reference` except the obliquity is set to Earth's measured 23.44 degrees (0.4091 rad):
    /// the tilt that gives the sub-solar latitude its yearly swing, midnight sun and polar night at the poles,
    /// and the summer/winter insolation contrast at mid-latitudes. This is per-world DATA (Principle 11,
    /// the locked per-world-outcome rule): Mirror carries Earth's real value, an alien world sets its own,
    /// and nothing is authored globally. The heat parameters remain the labelled Earth-like dev fixtures the
    /// reference uses, reserved for the owner's calibration; the per-material emissivity/thermal-inertia read
    /// is the sibling follow-on. `rotation_period_ticks` is Mirror's day, `orbital_period_ticks` its year.
    pub fn mirror(rotation_period_ticks: u64, orbital_period_ticks: u64) -> DiurnalSky {
        DiurnalSky {
            // Earth's real obliquity, 23.44 degrees = 0.4091 radians. Per-world data, not a global author:
            // this is Mirror's measured tilt, surfaced for the owner as the reserved world datum.
            obliquity: Fixed::from_ratio(4091, 10_000),
            // Earth's measured planetary Bond albedo, 0.306 (the fraction of incoming sunlight reflected;
            // NASA Earth Fact Sheet, Bond albedo 0.306). Per-world data: this is what drops Mirror's absorbed
            // solar flux to 0.694 of the incident and lands its radiative-equilibrium mean near Earth's real
            // ~288 K instead of the ~311 K a bare (albedo-zero) world reaches. Surfaced for the owner as the
            // reserved world datum, cited not fabricated.
            albedo: Fixed::from_ratio(306, 1000),
            // Earth's measured orbital eccentricity, ~0.0167 (a near-circular ellipse). Per-world data: this
            // is what makes Mirror's delivered flux ~3.4% higher at perihelion than at aphelion over the year,
            // the distance modulation a Kepler solve derives. Surfaced for the owner as the reserved world
            // datum, cited not fabricated.
            eccentricity: Fixed::from_ratio(167, 10_000),
            // Earth's perihelion phase. Perihelion (~January 4) falls ~13 days after the December solstice,
            // which in this law's declination convention (0 the March equinox, 3/4 the December solstice) sits
            // at orbital phase ~0.75 + 13/365 ~ 0.787. So Mirror's closest approach is in the northern winter,
            // the real precession offset that makes the southern hemisphere's summer the more extreme one.
            // Per-world data surfaced with its basis (the exact ecliptic-longitude convention is owner-refinable).
            perihelion_phase: Fixed::from_ratio(787, 1000),
            ..DiurnalSky::reference(rotation_period_ticks, orbital_period_ticks)
        }
    }
}

/// The fixed iteration cap for the Kepler eccentric-anomaly fixed-point solve. An engine-accuracy /
/// determinism bound (a fixed integer count, not a world value): the fixed-point `E = M + e*sin(E)` converges
/// geometrically at rate `e`, so at the represented eccentricity range this is far more than enough to reach
/// the `Fixed` epsilon (Earth's e ~ 0.017 converges in ~3 steps; even e ~ 0.9 well inside the cap). Fixed, so
/// the solve is deterministic and worker-invariant, never an unbounded until-converged loop.
const KEPLER_ITERS: usize = 16;

/// The orbital-distance flux factor `(a/d)^2` at an orbital phase, DERIVED from the world's eccentricity by a
/// Kepler solve: the delivered irradiance scales as the inverse square of the star distance, and the distance
/// varies over an eccentric orbit as `d/a = 1 - e*cos(E)`, with the eccentric anomaly `E` solving Kepler's
/// equation `M = E - e*sin(E)` for the mean anomaly `M = 2*pi*orbital_phase` (perihelion at phase 0; the
/// perihelion-longitude phase relative to the seasons is the precession follow-on). A CIRCULAR orbit
/// (`eccentricity <= 0`) returns exactly 1, so a world that does not declare an eccentricity is byte-identical
/// to the pre-eccentricity baseline. A degenerate non-closed orbit (`d/a <= 0`, `e >= 1`) returns 1 rather
/// than diverge. Deterministic fixed-point CORDIC trig under a fixed iteration cap.
fn orbital_distance_factor(
    orbital_phase: Fixed,
    eccentricity: Fixed,
    perihelion_phase: Fixed,
) -> Fixed {
    if eccentricity <= Fixed::ZERO {
        return Fixed::ONE;
    }
    let two_pi = Fixed::PI.saturating_add(Fixed::PI);
    // The mean anomaly is measured FROM perihelion: at orbital_phase == perihelion_phase the world is closest
    // (M = 0). Subtracting the perihelion phase is the precession offset, decoupling the distance minimum from
    // the seasonal reference the declination keys to. A zero perihelion phase reduces to M = 2*pi*orbital_phase.
    let mean_anomaly = two_pi
        .checked_mul(orbital_phase - perihelion_phase)
        .unwrap_or(Fixed::ZERO);
    // Kepler's equation by fixed-point iteration E_{n+1} = M + e*sin(E_n), a fixed cap for determinism.
    let mut e_anom = mean_anomaly;
    for _ in 0..KEPLER_ITERS {
        e_anom = mean_anomaly.saturating_add(
            eccentricity
                .checked_mul(e_anom.sin())
                .unwrap_or(Fixed::ZERO),
        );
    }
    // d/a = 1 - e*cos(E); factor = (a/d)^2 = 1 / (1 - e*cos(E))^2.
    let d_over_a = Fixed::ONE
        - eccentricity
            .checked_mul(e_anom.cos())
            .unwrap_or(Fixed::ZERO);
    if d_over_a <= Fixed::ZERO {
        return Fixed::ONE;
    }
    d_over_a
        .checked_mul(d_over_a)
        .and_then(|d2| Fixed::ONE.checked_div(d2))
        .unwrap_or(Fixed::ONE)
}

/// The instantaneous insolation at a cell (day-night sun-angle law): the sum over the world's data star-list of
/// each star's flux times the clamped cosine of the sun's zenith angle,
/// `insolation = sum_s L_s * max(0, cos theta_s)`, with
/// `cos theta_s = sin(lat) sin(decl) + cos(lat) cos(decl) cos(hour)` (the standard solar-zenith geometry). The
/// latitude derives from the row (equator 0, poles +/- pi/2), the hour angle is the SYNODIC solar angle
/// `2*pi*(phase + longitude - orbital_phase + star_offset)` from the per-cell longitude (the column) and the
/// diurnal and orbital phases (so a tidally-locked world's day face is fixed), and the declination
/// `obliquity * sin(2*pi*orbital_phase)` gives seasons and polar day/night from the world's tilt. At the
/// zero-obliquity single-star reference this reduces to `cos(lat) cos(hour)`: a clean day-night swing that is
/// correctly dark at the poles (no tilt, the sun never clears the horizon there). A pure function of the cell,
/// the phases, and the world's own sky data (Principles 8, 9: no label, no authored outcome); deterministic
/// fixed-point CORDIC trig.
fn insolation_at(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    diurnal_phase: Fixed,
    orbital_phase: Fixed,
    sky: &DiurnalSky,
) -> Fixed {
    let mid = height / 2;
    if mid <= 0 || width <= 0 {
        // A degenerate strip has no latitude structure: read the summed daylit flux so a test fixture is lit.
        return sky
            .stars
            .iter()
            .fold(Fixed::ZERO, |a, s| a.saturating_add(s.luminosity));
    }
    // Latitude in [-pi/2, pi/2]: the equator row (mid) is 0, the poles are +/- pi/2.
    let lat = Fixed::from_ratio((mid - y) as i64, mid as i64)
        .checked_mul(Fixed::HALF_PI)
        .unwrap_or(Fixed::ZERO);
    let longitude = Fixed::from_ratio(x as i64, width as i64); // [0, 1) around the world
    let two_pi = Fixed::PI.saturating_add(Fixed::PI);
    // The declination from the world's tilt and its orbital position (0 at the reference world).
    let decl = sky
        .obliquity
        .checked_mul(
            two_pi
                .checked_mul(orbital_phase)
                .unwrap_or(Fixed::ZERO)
                .sin(),
        )
        .unwrap_or(Fixed::ZERO);
    let (sin_lat, cos_lat) = lat.sin_cos();
    let (sin_decl, cos_decl) = decl.sin_cos();
    let mut total = Fixed::ZERO;
    for star in &sky.stars {
        // The synodic (solar) hour angle: the sidereal diurnal phase plus the cell's longitude, minus the
        // orbital advance (so successive noons track the star, not the fixed stars), plus the star's own offset.
        let hour_frac = diurnal_phase
            .saturating_add(longitude)
            .saturating_add(star.phase_offset)
            - orbital_phase;
        let hour = two_pi.checked_mul(hour_frac).unwrap_or(Fixed::ZERO);
        // cos(zenith) = sin(lat)sin(decl) + cos(lat)cos(decl)cos(hour).
        let term_pole = sin_lat.checked_mul(sin_decl).unwrap_or(Fixed::ZERO);
        let term_day = cos_lat
            .checked_mul(cos_decl)
            .unwrap_or(Fixed::ZERO)
            .checked_mul(hour.cos())
            .unwrap_or(Fixed::ZERO);
        let cos_zenith = term_pole.saturating_add(term_day);
        // max(0, cos zenith): the night side (sun below the horizon) delivers no flux.
        let lit = cos_zenith.max(Fixed::ZERO);
        total = total.saturating_add(star.luminosity.checked_mul(lit).unwrap_or(Fixed::ZERO));
    }
    // Scale by the orbital-distance flux factor (a/d)^2 (exactly 1 for a circular orbit, so byte-neutral
    // there): an eccentric world receives more flux at perihelion and less at aphelion over its year.
    total
        .checked_mul(orbital_distance_factor(
            orbital_phase,
            sky.eccentricity,
            sky.perihelion_phase,
        ))
        .unwrap_or(total)
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
            sky: None,
            diurnal_tick: 0,
            solar_baseline: Vec::new(),
            solar_inertia: Vec::new(),
            evaporation: Vec::new(),
            surface_cooling: None,
            photosynthesis: None,
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
    fn baseline_soil_equals_the_soil_requirement_the_derived_invariant() {
        // soil_baseline DERIVES from soil_requirement (its basis: MUST equal it so bare soil is exactly
        // non-limiting at baseline). The manifest path reads soil_requirement for both; this locks the same
        // invariant on the code dev-fixture path, so the two cannot drift apart.
        let f = EnvironCalib::dev_fixture();
        assert_eq!(f.soil_baseline, f.soil_req);
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
    fn the_sun_angle_law_swings_day_to_night_at_the_equator_and_leaves_the_poles_dark() {
        // The zero-obliquity single-star reference sky: one unit star, no tilt, a 100-tick day.
        let sky = DiurnalSky::reference(100, 36500);
        let (w, h) = (10, 5); // equator row is y=2 (mid); poles are y=0 and y=4.
        let orbital = Fixed::ZERO; // one day is a negligible slice of the year; hold the orbital phase.
                                   // At the equator, column x=0, diurnal phase 0 puts the sun overhead (hour angle 0): full flux ~1.
        let noon = insolation_at(0, 2, w, h, Fixed::ZERO, orbital, &sky);
        assert!(
            noon > Fixed::from_ratio(9, 10),
            "the equator at local noon is fully lit, got {noon:?}"
        );
        // Half a rotation later the same cell faces away (hour angle pi): the night side reads zero.
        let midnight = insolation_at(0, 2, w, h, Fixed::from_ratio(1, 2), orbital, &sky);
        assert_eq!(
            midnight,
            Fixed::ZERO,
            "the equator at local midnight is dark, got {midnight:?}"
        );
        // Dawn/dusk (quarter turn, hour angle pi/2) is the terminator: near zero, below noon.
        let dusk = insolation_at(0, 2, w, h, Fixed::from_ratio(1, 4), orbital, &sky);
        assert!(
            dusk < noon && dusk <= Fixed::from_ratio(1, 100),
            "the terminator is dim, got {dusk:?}"
        );
        // A pole under zero obliquity never clears the horizon: dark at every phase (physically correct here).
        // cos(pi/2) is a sub-part-per-billion fixed-point CORDIC residual rather than exact zero, so the pole is
        // negligibly (not bit-exactly) lit; bound it rather than assert an exact zero the trig cannot deliver.
        let eps = Fixed::from_ratio(1, 1_000_000);
        for p in [0, 1, 2, 3] {
            let pole = insolation_at(0, 0, w, h, Fixed::from_ratio(p, 4), orbital, &sky);
            assert!(
                pole < eps,
                "a zero-tilt pole is dark at phase {p}/4, got {pole:?}"
            );
        }
    }

    #[test]
    fn a_circular_orbit_has_a_unit_distance_factor_so_the_reference_is_byte_neutral() {
        // Eccentricity 0 is a circle: the distance never varies, so the orbital-distance flux factor is
        // EXACTLY 1 at every phase and every perihelion phase, and a world that does not declare an
        // eccentricity is byte-identical to the pre-eccentricity baseline.
        for p in [0, 1, 2, 3, 7] {
            let factor = orbital_distance_factor(
                Fixed::from_ratio(p, 8),
                Fixed::ZERO,
                Fixed::from_ratio(3, 8),
            );
            assert_eq!(
                factor,
                Fixed::ONE,
                "a circular orbit's distance factor is exactly 1 at phase {p}/8, got {factor:?}"
            );
        }
    }

    #[test]
    fn an_eccentric_orbit_is_brighter_at_perihelion_and_dimmer_at_aphelion() {
        // With the perihelion at orbital phase 0, phase 0 is closest (more flux) and phase 1/2 farthest (less).
        // For Earth's e ~ 0.0167 the perihelion factor is ~(1/(1-e))^2 ~ 1.034 and aphelion ~(1/(1+e))^2 ~
        // 0.967, the ~3.4% swing. The ordering and rough magnitude are what Kepler's geometry asserts.
        let e = Fixed::from_ratio(167, 10_000);
        let perihelion = orbital_distance_factor(Fixed::ZERO, e, Fixed::ZERO);
        let aphelion = orbital_distance_factor(Fixed::from_ratio(1, 2), e, Fixed::ZERO);
        assert!(
            perihelion > Fixed::ONE && aphelion < Fixed::ONE,
            "perihelion brightens ({perihelion:?}) and aphelion dims ({aphelion:?})"
        );
        assert!(
            perihelion > aphelion,
            "the world is brighter at perihelion than aphelion"
        );
        // The perihelion factor is close to (1/(1-e))^2 ~ 1.034 (within a coarse fixed-point tolerance).
        assert!(
            (perihelion.to_f64_lossy() - 1.034).abs() < 0.01,
            "the perihelion factor is ~1.034, got {}",
            perihelion.to_f64_lossy()
        );
    }

    #[test]
    fn precession_moves_the_distance_minimum_to_the_declared_perihelion_phase() {
        // The perihelion phase decouples the closest approach from the seasonal reference: the distance factor
        // peaks at orbital_phase == perihelion_phase, wherever that is set, not at phase 0. With perihelion at
        // 0.3, orbital phase 0.3 is the max and phase 0 is no longer it; half a year past perihelion is the min.
        let e = Fixed::from_ratio(167, 10_000);
        let peri_phase = Fixed::from_ratio(3, 10);
        let at_perihelion = orbital_distance_factor(peri_phase, e, peri_phase);
        let at_phase_zero = orbital_distance_factor(Fixed::ZERO, e, peri_phase);
        let half_year_later = orbital_distance_factor(
            peri_phase.saturating_add(Fixed::from_ratio(1, 2)),
            e,
            peri_phase,
        );
        assert!(
            at_perihelion > at_phase_zero,
            "the max flux is at the declared perihelion phase, not at phase 0"
        );
        assert!(
            at_perihelion > Fixed::ONE && half_year_later < Fixed::ONE,
            "perihelion brightens and half a year later (aphelion) dims"
        );
        // Mirror carries a real, nonzero perihelion phase (Earth's ~0.787), so its closest approach is off the
        // seasonal reference: the precession offset is armed, not inert.
        let mirror = DiurnalSky::mirror(100, 36500);
        assert!(
            mirror.perihelion_phase > Fixed::from_ratio(7, 10),
            "Mirror's perihelion phase is set to Earth's ~0.787, got {:?}",
            mirror.perihelion_phase
        );
        let mirror_peri = orbital_distance_factor(
            mirror.perihelion_phase,
            mirror.eccentricity,
            mirror.perihelion_phase,
        );
        let mirror_apo = orbital_distance_factor(
            mirror
                .perihelion_phase
                .saturating_add(Fixed::from_ratio(1, 2)),
            mirror.eccentricity,
            mirror.perihelion_phase,
        );
        assert!(
            mirror_peri > mirror_apo,
            "Mirror is brighter at its perihelion than half a year later"
        );
    }

    #[test]
    fn the_circular_reference_reads_identically_across_the_year() {
        // The zero-eccentricity reference reads the same insolation at any two orbital phases at matched noon
        // geometry: no distance modulation, and (both equinoxes) no season, so the sky is byte-identical.
        let flat = DiurnalSky::reference(100, 36500);
        let (w, h) = (10, 5);
        let a = insolation_at(0, 2, w, h, Fixed::ZERO, Fixed::ZERO, &flat);
        let b = insolation_at(
            0,
            2,
            w,
            h,
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 2),
            &flat,
        );
        assert_eq!(
            a, b,
            "the circular reference reads identically at two equinox noons"
        );
    }

    #[test]
    fn the_mirror_tilt_rides_real_seasons_on_the_day_night_cycle() {
        // Mirror's sky carries Earth's real 23.44-degree tilt, so the declination term swings the sub-solar
        // latitude across the year and REAL SEASONS emerge on top of the day-night cycle. Contrast it with the
        // zero-tilt reference at the same orbital phase: the tilt is the only difference, so any pole effect is
        // the season and not the day. North pole is y=0, south pole is y=h-1 (lat = (mid - y)/mid * pi/2).
        let mirror = DiurnalSky::mirror(100, 36500);
        let flat = DiurnalSky::reference(100, 36500);
        let (w, h) = (10, 5); // equator row y=2 (mid); north pole y=0, south pole y=4.

        // Northern summer solstice: orbital phase 1/4 puts sin(2*pi*phase)=1, declination = +obliquity, so the
        // sub-solar point is at +23.44 latitude and the NORTH POLE sees MIDNIGHT SUN: lit at every hour of the
        // rotation. The zero-tilt pole at the same phase stays dark (no season without tilt).
        let summer = Fixed::from_ratio(1, 4);
        for p in [0, 1, 2, 3] {
            let phase = Fixed::from_ratio(p, 4);
            let north_mirror = insolation_at(0, 0, w, h, phase, summer, &mirror);
            let north_flat = insolation_at(0, 0, w, h, phase, summer, &flat);
            assert!(
                north_mirror > Fixed::from_ratio(3, 10),
                "the tilted north pole has midnight sun at the summer solstice, phase {p}/4, got {north_mirror:?}"
            );
            let eps = Fixed::from_ratio(1, 1_000_000);
            assert!(
                north_flat < eps,
                "the zero-tilt pole stays dark at the same phase {p}/4, got {north_flat:?}"
            );
        }

        // Half a year on (orbital phase 3/4, sin = -1), the declination flips to -obliquity: the north pole is
        // in POLAR NIGHT (dark at every hour) while the south pole now takes the midnight sun. Seasons reverse.
        let winter = Fixed::from_ratio(3, 4);
        for p in [0, 1, 2, 3] {
            let phase = Fixed::from_ratio(p, 4);
            let north = insolation_at(0, 0, w, h, phase, winter, &mirror);
            assert_eq!(
                north,
                Fixed::ZERO,
                "the tilted north pole is in polar night half a year later, phase {p}/4, got {north:?}"
            );
        }
        let south_winter = insolation_at(0, 4, w, h, Fixed::ZERO, winter, &mirror);
        assert!(
            south_winter > Fixed::from_ratio(3, 10),
            "the opposite pole takes the midnight sun when the seasons reverse, got {south_winter:?}"
        );

        // The season does not abolish the day: at the equator the diurnal swing still runs through the tilt.
        // Local noon is the SYNODIC angle (diurnal phase = orbital phase, hour angle 0), midnight half a turn on.
        let equator_noon = insolation_at(0, 2, w, h, summer, summer, &mirror);
        let equator_midnight = insolation_at(
            0,
            2,
            w,
            h,
            summer.saturating_add(Fixed::from_ratio(1, 2)),
            summer,
            &mirror,
        );
        assert!(
            equator_noon > equator_midnight,
            "the day-night swing survives the season at the equator ({equator_noon:?} vs {equator_midnight:?})"
        );
    }

    #[test]
    fn the_armed_diurnal_cycle_warms_the_day_baseline_and_holds_a_night_floor_above_absolute_zero()
    {
        // Arm the reference sky and step one tick, then read the solar-forcing baseline the runner would copy
        // into the temperature field. The equator at dawn (tick 0, hour angle 0) is fully daylit, so its
        // baseline is the radiative-equilibrium temperature of the full stellar flux plus back-radiation; a
        // dark cell (a zero-tilt pole) reads the radiative-equilibrium of the back-radiation floor alone, which
        // is well above absolute zero (the night-floor form (2), never 0 K).
        let map = a_map(0x5A1A17);
        let mut e = EnvironFields::from_map(&map);
        let h = e.height;
        e.arm_diurnal(DiurnalSky::reference(100, 36500));
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        e.step(&temp, &calib);
        let mid = h / 2;
        let day = e.solar_baseline_at(0, mid); // equator, local noon at tick 0
        let night = e.solar_baseline_at(0, 0); // a zero-tilt pole: no direct sun, back-radiation only
        assert!(
            day > Fixed::from_int(250),
            "the daylit baseline is a warm surface temperature (K), got {day:?}"
        );
        assert!(
            night > Fixed::from_int(50),
            "the dark-side baseline holds a back-radiation floor well above absolute zero, got {night:?}"
        );
        assert!(
            day > night,
            "the daylit side is warmer than the dark side ({day:?} vs {night:?})"
        );
    }

    #[test]
    fn armed_surface_cooling_lowers_the_daylit_baseline_below_the_radiative_only_balance() {
        // Slice 2: with surface turbulent cooling ARMED, the diurnal balance adds the sensible loss to the
        // effective-radiating-temperature air reference (and, after a tick, the latent loss), so the daylit
        // baseline is cooler than the radiative-only balance an unarmed run keeps. Unarmed is byte-identical to
        // the pre-arc `radiative_equilibrium` (the diurnal test above exercises that path through the short
        // circuit); this shows the armed terms bite. The reference sky's emissivity is 0.95, so the surface sits
        // above the effective radiating temperature and the sensible term cools it.
        let map = a_map(0x5A1A17);
        let sky = DiurnalSky::reference(100, 36500);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let mut bare = EnvironFields::from_map(&map);
        bare.arm_diurnal(sky.clone());
        bare.step(&temp, &calib);
        let mut cooled = EnvironFields::from_map(&map);
        cooled.arm_diurnal(sky);
        cooled.arm_surface_cooling(SurfaceCooling {
            convective_h: Fixed::from_int(30),
            latent_heat: Fixed::from_int(2_400_000),
        });
        cooled.step(&temp, &calib);
        let mid = bare.height / 2;
        let day_bare = bare.solar_baseline_at(0, mid);
        let day_cooled = cooled.solar_baseline_at(0, mid);
        assert!(
            day_cooled < day_bare,
            "armed surface cooling lowers the daylit baseline: cooled {day_cooled:?} < radiative-only {day_bare:?}"
        );
        assert!(
            day_cooled > Fixed::from_int(250),
            "the cooled baseline is still a warm surface temperature, not collapsed: {day_cooled:?}"
        );
    }

    #[test]
    fn surface_thermal_inertia_makes_water_lag_and_freezing_shifts_to_ice() {
        // The per-material thermal inertia (follow-on 2): a DRY cell reads exactly one, so its heat dynamics
        // are unchanged (the byte-neutral baseline); a WATER-laden cell reads below one, so it lags; and a
        // frozen cell reads ice's inertia (a smaller heat capacity than liquid water), so it lags LESS than the
        // same cell unfrozen. All emergent from the cell's own water depth and temperature, no label.
        let s = SurfaceThermal::dev_fixture();
        let warm = Fixed::from_int(300); // above freezing: standing water is liquid
        let cold = Fixed::from_int(260); // below 273.15 K: the water is ice
        let deep = Fixed::from_int(50); // a deep basin or ocean, far past the half-saturation depth
        let dry_soil = Fixed::ZERO; // no soil moisture
        let full_soil = Fixed::ONE; // saturated soil

        let dry = s.inertia_factor(Fixed::ZERO, dry_soil, warm);
        assert_eq!(
            dry,
            Fixed::ONE,
            "a bone-dry cell reads exactly one (its dynamics are unchanged), got {dry:?}"
        );
        // Soil moisture alone (no standing water) already lags: damp soil holds thermal mass.
        let damp = s.inertia_factor(Fixed::ZERO, Fixed::from_ratio(1, 2), warm);
        assert!(
            damp < dry,
            "damp soil lags below dry land even with no standing water (damp {damp:?}, dry {dry:?})"
        );
        let wet = s.inertia_factor(deep, full_soil, warm);
        assert!(
            wet < damp,
            "a water-laden cell lags more than merely damp soil (water {wet:?}, damp {damp:?})"
        );
        let frozen = s.inertia_factor(deep, full_soil, cold);
        assert!(
            wet < frozen && frozen < Fixed::ONE,
            "ice's smaller heat capacity lags less than liquid water but still more than dry land (liquid {wet:?}, ice {frozen:?})"
        );
        // Monotone in soil moisture: more moisture, more inertia, more lag.
        let a_little = s.inertia_factor(Fixed::ZERO, Fixed::from_ratio(1, 4), warm);
        assert!(
            damp < a_little && a_little < Fixed::ONE,
            "more soil moisture lags more (quarter-moist {a_little:?}, half-moist {damp:?})"
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
    fn carbon_fixation_derives_from_the_photosynthesis_physics_and_the_measured_constants() {
        // The derived carbon-fixation rate (the photosynthesis-to-productivity arc, #156): productivity is a
        // measured rate from the cell's own physical fields and the producer's measured enzyme data, not an
        // authored per-cell number. Every assertion checks a physics behaviour, none a fitted magnitude.
        let p = PhotosynthesisCalib::dev_fixture(); // C3-like: quantum yield 1/20, saturation 300, opt 298 K, breadth 30 K
        let sun = Fixed::from_int(1361); // the stellar-constant flux anchor (Earth's measured solar constant)
        let opt = Fixed::from_int(298); // the enzyme thermal optimum
                                        // At the optimum temperature, well-watered and fertile, fixation RISES with insolation and SATURATES
                                        // (the light-response curve): the increment from a tenth to a half of full sun exceeds the increment from
                                        // a half to full sun, so the curve is concave, never linear.
        let fix = |insol: Fixed, temp: Fixed, evap: Fixed, water: Fixed, soil: Fixed| {
            carbon_fixation_rate(
                insol,
                sun,
                temp,
                evap,
                water,
                soil,
                Fixed::from_ratio(1, 2),
                &p,
            )
        };
        let full = Fixed::ONE;
        let ample = Fixed::from_int(10); // abundant water and soil supply, so light and temperature limit
        let lo = fix(Fixed::from_ratio(1, 10), opt, Fixed::ZERO, ample, ample);
        let mid = fix(Fixed::from_ratio(1, 2), opt, Fixed::ZERO, ample, ample);
        let hi = fix(full, opt, Fixed::ZERO, ample, ample);
        assert!(
            lo < mid && mid < hi,
            "fixation rises with insolation: {lo:?} {mid:?} {hi:?}"
        );
        assert!(
            (mid - lo) > (hi - mid),
            "the light-response saturates (concave), not linear: {lo:?} {mid:?} {hi:?}"
        );
        // The temperature TENT peaks at the enzyme optimum and falls away: the optimum fixes more than a cell 20 K
        // colder, and a cell a full breadth (30 K) off the optimum fixes NOTHING (the tent floor).
        let at_opt = fix(full, opt, Fixed::ZERO, ample, ample);
        let cold = fix(full, Fixed::from_int(278), Fixed::ZERO, ample, ample);
        let too_hot = fix(full, Fixed::from_int(328), Fixed::ZERO, ample, ample);
        assert!(
            cold < at_opt,
            "the optimum fixes more than a colder cell: {cold:?} vs {at_opt:?}"
        );
        assert_eq!(
            too_hot,
            Fixed::ZERO,
            "a cell a full thermal breadth past the optimum fixes nothing (the enzyme is out of band)"
        );
        // The WATER limitation: a higher evaporative demand raises the derived water requirement, so a cell with
        // the same water supply but a drier atmosphere fixes LESS (the water-use-efficiency coupling), never a
        // flat authored requirement.
        let humid = fix(full, opt, Fixed::from_int(1), Fixed::ONE, ample);
        let arid = fix(full, opt, Fixed::from_int(20), Fixed::ONE, ample);
        assert!(
            arid < humid,
            "a drier atmosphere (higher evaporative demand) fixes less: {arid:?} vs {humid:?}"
        );
        // The SOIL-nutrient limitation over the matter-cycle fertility: a nutrient-poor cell fixes less than a
        // fertile one (the Liebig soil factor, retiring the flat soil_baseline).
        let poor = fix(full, opt, Fixed::ZERO, ample, Fixed::from_ratio(1, 10));
        let fertile = fix(full, opt, Fixed::ZERO, ample, ample);
        assert!(
            poor < fertile,
            "a nutrient-poor cell fixes less: {poor:?} vs {fertile:?}"
        );
        // Determinism (Principle 3): a pure function of its physical inputs, bit-identical on replay.
        assert_eq!(
            hi,
            fix(full, opt, Fixed::ZERO, ample, ample),
            "the derivation is deterministic"
        );
    }

    #[test]
    fn mineral_weathering_scales_soil_nutrient_by_cell_wetness() {
        // The ABIOTIC weathering floor (#156, the matter-cycle completion): rock weathers to soil nutrient
        // MAP-WIDE at a rate that DERIVES from each cell's own wetness (hydrolysis), so a wet marsh cell weathers
        // strongly and a fully-dry cell not at all. This is what breaks the soil-bootstrap deadlock (fertility
        // positive from geology before any biomass). A zero base is a no-op (byte-neutral off), and the fold is
        // deterministic.
        let map = a_map(0x5EED);
        let temp = Field::from_map(&map);
        let calib = EnvironCalib::dev_fixture();
        let mut e = EnvironFields::from_map(&map);
        for _ in 0..40 {
            e.step(&temp, &calib); // accumulate water so some cells are wet and some stay dry
        }
        let (w, h) = e.dims();
        let class = "bio.organic_residue";

        // Zero base deposits nothing (the unarmed run is byte-identical).
        let mut soil_off = SoilNutrientField::new();
        e.weather_minerals(&mut soil_off, &calib, Fixed::ZERO, class);
        assert_eq!(
            soil_off.cell_totals().count(),
            0,
            "a zero weathering base deposits nothing (byte-neutral off)"
        );

        // Armed: a wet cell carries weathered nutrient, a fully-dry cell carries none (wetness-scaled).
        let base = Fixed::from_int(100);
        let mut soil = SoilNutrientField::new();
        e.weather_minerals(&mut soil, &calib, base, class);
        let wet = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .find(|&(x, y)| e.water_at(x, y) > Fixed::ZERO)
            .expect("the wet world offers a wet cell to weather");
        assert!(
            soil.mass(Coord3::ground(wet.0, wet.1), class) > Fixed::ZERO,
            "a wet cell weathers soil nutrient from geology"
        );
        if let Some((dx, dy)) = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .find(|&(x, y)| e.water_at(x, y) == Fixed::ZERO)
        {
            assert_eq!(
                soil.mass(Coord3::ground(dx, dy), class),
                Fixed::ZERO,
                "a fully-dry cell weathers nothing (the hydrolysis coupling)"
            );
        }

        // Deterministic: a second identical pass reproduces the wet cell's deposit exactly.
        let mut soil2 = SoilNutrientField::new();
        e.weather_minerals(&mut soil2, &calib, base, class);
        assert_eq!(
            soil.mass(Coord3::ground(wet.0, wet.1), class),
            soil2.mass(Coord3::ground(wet.0, wet.1), class),
            "weathering is a deterministic fold"
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
            {
                let tf = Field::new(
                    e.width,
                    e.height,
                    vec![Fixed::from_int(300); (e.width * e.height) as usize],
                );
                e.extract_producers(&mut soil, &reg, &tf)
            };
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
            {
                let tf = Field::new(
                    e.width,
                    e.height,
                    vec![Fixed::from_int(300); (e.width * e.height) as usize],
                );
                e.extract_producers(&mut soil, &reg, &tf)
            };
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
        {
            let tf = Field::new(
                e.width,
                e.height,
                vec![Fixed::from_int(300); (e.width * e.height) as usize],
            );
            e.extract_producers(&mut soil, &reg, &tf)
        }; // panics: field 99 was never declared
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
            {
                let tf = Field::new(
                    e.width,
                    e.height,
                    vec![Fixed::from_int(300); (e.width * e.height) as usize],
                );
                e.extract_producers(&mut soil, &reg, &tf)
            };
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
        {
            let tf = Field::new(
                e.width,
                e.height,
                vec![Fixed::from_int(300); (e.width * e.height) as usize],
            );
            e.extract_producers(&mut soil, &reg, &tf)
        };

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
        {
            let tf = Field::new(
                e.width,
                e.height,
                vec![Fixed::from_int(300); (e.width * e.height) as usize],
            );
            e.extract_producers(&mut soil, &reg, &tf)
        };

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
    fn a_nernst_armed_redox_source_yield_falls_as_the_couple_depletes() {
        // The concentration-dependent depth extension (the corrected Nernst): an ARMED redox source (carrier
        // charge above zero) reads its couple's ACTUAL donor concentration, so BELOW the standard state
        // (activity under one) its per-unit yield is LOWER than the concentration-independent standard EMF, and
        // AT unit activity it reduces EXACTLY to the standard EMF. This is the fix for the standard-EMF defect:
        // a depleting couple's drive falls rather than reading spontaneity forever.
        let map = a_map(0x5EDD00);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let field_id: u16 = 20;
        let source_id: u16 = 200;
        let donor = Fixed::from_ratio(2, 10);
        let acceptor = Fixed::from_ratio(8, 10);
        let coupling = Fixed::from_int(2);
        let kb = Fixed::from_ratio(257, 10_000); // k_B*T/q ~ 0.0257 at unit T and unit carrier charge

        let run = |stock: Fixed, armed: bool| -> Fixed {
            let mut reg = AbioticSourceRegistry::default();
            reg.insert(source_id, AbioticField::DataScalar(field_id), false, "");
            reg.biomass_per_stock = Fixed::from_int(4);
            reg.draw_fraction = Fixed::ZERO;
            reg.weathering_rate = Fixed::ZERO;
            reg.emf_to_biomass = coupling;
            reg.set_source_redox(source_id, donor, acceptor);
            if armed {
                reg.set_boltzmann_k(kb);
                // q = 1, ideal gamma, no temperature coefficient: the donor is the source's own stock, the
                // acceptor buffered at unit activity, so only the donor concentration shifts the EMF here.
                reg.set_source_nernst(
                    source_id,
                    Fixed::ONE,
                    None,
                    Fixed::ONE,
                    Fixed::ONE,
                    Fixed::ZERO,
                    Fixed::ZERO,
                );
            }
            let mut e = EnvironFields::from_map(&map);
            let (w, h) = e.dims();
            e.set_producer(&[(cell, Fixed::from_int(1000))]);
            e.set_producer_source(&[(cell, vec![source_id])]);
            e.set_data_field(field_id, ScalarField::uniform(w, h, stock));
            let temp = Field::from_map(&map);
            e.step(&temp, &calib);
            let mut soil = SoilNutrientField::new();
            // A unit-temperature field so the thermal factor k_B*T/q equals kb exactly.
            let tf = Field::new(
                e.width,
                e.height,
                vec![Fixed::ONE; (e.width * e.height) as usize],
            );
            e.extract_producers(&mut soil, &reg, &tf);
            e.capacity_at(cell.x, cell.y)
        };

        // At unit donor activity (stock = 1) the Nernst reduces exactly to the standard EMF: identical yield.
        assert_eq!(
            run(Fixed::ONE, true),
            run(Fixed::ONE, false),
            "at unit activity the armed Nernst yield equals the standard-EMF yield (reduces exactly)"
        );
        // Below the standard state (stock = 1/2, activity < 1) the armed drive FALLS below the standard EMF.
        let low_armed = run(Fixed::from_ratio(1, 2), true);
        let low_std = run(Fixed::from_ratio(1, 2), false);
        assert!(
            low_armed < low_std && low_armed > Fixed::ZERO,
            "a depleting couple's Nernst yield falls below the standard EMF but still powers some life \
             (armed {low_armed:?}, standard {low_std:?})"
        );
    }

    #[test]
    fn a_nernst_redox_source_with_kinetics_draws_the_reversible_flux_from_its_catalyst_tissue() {
        // Phase-2 increment 2: an armed redox source WITH kinetics draws the reversible-Michaelis-Menten uptake
        // flux, Vmax = kcat * the being's OWN catalyst tissue (its named composition class), min(v, S) conserved.
        // A producer WITH catalyst tissue draws its stock down; one WITHOUT (no catalyst, Vmax = 0) draws nothing;
        // the draw never exceeds the present stock (the conservation clamp).
        let map = a_map(0x5EDD02);
        let calib = EnvironCalib::dev_fixture();
        let cell = Coord3::ground(1, 1);
        let field_id: u16 = 21;
        let source_id: u16 = 201;
        let donor = Fixed::from_ratio(2, 10);
        let acceptor = Fixed::from_ratio(8, 10);

        let run = |stock: Fixed, protein: Option<Fixed>| -> Fixed {
            let mut reg = AbioticSourceRegistry::default();
            reg.insert(source_id, AbioticField::DataScalar(field_id), true, ""); // depletes: pass 2 draws it
            reg.biomass_per_stock = Fixed::from_int(4);
            reg.draw_fraction = Fixed::from_ratio(1, 2);
            reg.weathering_rate = Fixed::ZERO;
            reg.emf_to_biomass = Fixed::from_int(2);
            reg.set_boltzmann_k(Fixed::from_ratio(257, 10_000));
            reg.set_source_redox(source_id, donor, acceptor);
            reg.set_source_nernst(
                source_id,
                Fixed::ONE,
                None,
                Fixed::ONE,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::ZERO,
            );
            reg.set_source_kinetics(
                source_id,
                Fixed::from_int(4),
                Fixed::ONE,
                Fixed::ONE,
                "bio.protein",
            );
            let mut e = EnvironFields::from_map(&map);
            let (w, h) = e.dims();
            e.set_producer(&[(cell, Fixed::from_int(1000))]);
            e.set_producer_source(&[(cell, vec![source_id])]);
            if let Some(p) = protein {
                let mut comp = BTreeMap::new();
                comp.insert("bio.protein".to_string(), p);
                e.set_producer_food(&[(cell, comp)]);
            }
            e.set_data_field(field_id, ScalarField::uniform(w, h, stock));
            let temp = Field::from_map(&map);
            e.step(&temp, &calib);
            let mut soil = SoilNutrientField::new();
            let tf = Field::new(
                e.width,
                e.height,
                vec![Fixed::ONE; (e.width * e.height) as usize],
            );
            e.extract_producers(&mut soil, &reg, &tf);
            stock - e.data_field_at(field_id, cell.x, cell.y).unwrap() // the amount drawn
        };

        // A producer WITH catalyst tissue draws the flux; one WITHOUT draws nothing (Vmax = kcat * 0 = 0).
        let drawn_with = run(Fixed::from_int(10), Some(Fixed::from_ratio(1, 2)));
        let drawn_without = run(Fixed::from_int(10), None);
        assert!(
            drawn_with > Fixed::ZERO,
            "a producer with catalyst tissue draws the reversible flux, got {drawn_with:?}"
        );
        assert_eq!(
            drawn_without,
            Fixed::ZERO,
            "a producer with no catalyst tissue draws nothing (its Vmax is zero), got {drawn_without:?}"
        );

        // Conservation: with a tiny stock the draw never exceeds it (the structural min(v, S) clamp).
        let tiny = Fixed::from_ratio(1, 1000);
        let drawn_tiny = run(tiny, Some(Fixed::from_ratio(1, 2)));
        assert!(
            drawn_tiny <= tiny && drawn_tiny > Fixed::ZERO,
            "the flux draws some but never more than the present stock (drawn {drawn_tiny:?}, stock {tiny:?})"
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
        {
            let tf = Field::new(
                e.width,
                e.height,
                vec![Fixed::from_int(300); (e.width * e.height) as usize],
            );
            e.extract_producers(&mut soil, &reg, &tf)
        };

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
            {
                let tf = Field::new(
                    e.width,
                    e.height,
                    vec![Fixed::from_int(300); (e.width * e.height) as usize],
                );
                e.extract_producers(&mut soil, &reg, &tf)
            };
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
        {
            let tf = Field::new(
                e.width,
                e.height,
                vec![Fixed::from_int(300); (e.width * e.height) as usize],
            );
            e.extract_producers(&mut soil, &reg, &tf)
        }; // panics: coupling unset
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
        {
            let tf = Field::new(
                e.width,
                e.height,
                vec![Fixed::from_int(300); (e.width * e.height) as usize],
            );
            e.extract_producers(&mut soil, &reg, &tf)
        };

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

    #[test]
    fn the_gas_constant_derives_from_the_fundamentals() {
        // R = N_A * k_B derives from the CODATA fundamentals (never an authored decimal), landing near
        // 8.314462618 J/(mol*K) at Q32.32 (the value fits comfortably inside the representable window).
        let r = derived_gas_constant();
        assert!(
            r > Fixed::from_ratio(8310, 1000) && r < Fixed::from_ratio(8320, 1000),
            "R derives to ~8.314 J/(mol*K), got {r:?}"
        );
    }

    #[test]
    fn a_chemical_formula_parses_to_its_element_counts() {
        // The volatile's molar mass derives from its OWN formula (an alien volatile is a data row), so the
        // parser must count H2O, a two-letter symbol, and a bare (count-one) element correctly.
        let water = parse_formula("H2O").expect("H2O parses");
        assert_eq!(water.get("H"), Some(&2));
        assert_eq!(water.get("O"), Some(&1));
        let calcite = parse_formula("CaCO3").expect("CaCO3 parses (two-letter symbol)");
        assert_eq!(calcite.get("Ca"), Some(&1));
        assert_eq!(calcite.get("C"), Some(&1));
        assert_eq!(calcite.get("O"), Some(&3));
        assert!(parse_formula("").is_none(), "an empty formula is rejected");
        assert!(parse_formula("2O").is_none(), "a leading digit is rejected");
    }

    #[test]
    fn the_saturation_index_tangent_derives_from_the_floor_volatile() {
        // The moisture-index saturation tangent DERIVES from water's measured latent heat, its molar mass,
        // and the derived gas constant, at Mirror's reference (288 K) and warm-source (mean + range/2 = 311 K)
        // temperatures. The constant-latent-heat Clausius-Clapeyron index e_ref = e_s(288)/e_s(311) lands near
        // 0.255 (below one, since the reference is colder than the source), and the tangent slope near 0.0164
        // index/K, reproducing the retired authored pair's Clausius-Clapeyron basis (0.20, 0.0131) from the
        // field-consistent 311 K source rather than the old 315 K estimate.
        let t_ref = Fixed::from_int(288);
        let t_source = Fixed::from_int(311);
        let (slope, e_ref) =
            derive_saturation_index_tangent(t_ref, t_source).expect("the water tangent derives");
        assert!(
            e_ref > Fixed::from_ratio(24, 100) && e_ref < Fixed::from_ratio(27, 100),
            "e_ref derives to ~0.255 (a saturation index below one), got {e_ref:?}"
        );
        assert!(
            slope > Fixed::from_ratio(15, 1000) && slope < Fixed::from_ratio(18, 1000),
            "the tangent slope derives to ~0.0164 index/K, got {slope:?}"
        );
        // A WARMER vapour source lowers the index (the reference is a smaller fraction of a warmer saturation),
        // the Clausius-Clapeyron monotonicity, so the mechanism responds to the world's own climate.
        let (_, e_ref_warmer) = derive_saturation_index_tangent(t_ref, Fixed::from_int(330))
            .expect("the warmer-source tangent derives");
        assert!(
            e_ref_warmer < e_ref,
            "a warmer vapour source gives a lower saturation index: {e_ref_warmer:?} < {e_ref:?}"
        );
        // A degenerate temperature yields None rather than a divide-by-zero.
        assert!(derive_saturation_index_tangent(Fixed::ZERO, t_source).is_none());
    }
}
