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

//! # The derivation registry (tasks #43 and #46, the derived-output-is-live gate)
//!
//! The constructor gate (`scripts/constructor_gate.py`) proves no authored value sits on the
//! canonical path. It cannot prove a DERIVED value is alive: a derivation can obey the gate and then
//! produce a constant, and nothing catches it. That is the `soil_baseline` bug (a 330x sweep of the
//! photosynthesis constants left the state hash byte-identical) and the identically-zero Dalton
//! evaporation closed in the hydrology re-baseline.
//!
//! This registry is the data-defined membership of the derived-output-is-live gate: one row per
//! tracked derivation, naming the derivation, its category ([`DerivationCategory`]: a retired-floor
//! replacement or a new derived output), the input source it derives from ([`InputSource`]: a
//! manifest scalar, a data-defined driver parameter, or a resident-state field), the scenario that
//! arms it on the run path, and the source site that carries its `@derives[id]:` annotation. The
//! liveness principle (perturb the input, assert the derived value responds at its site) applies to
//! ANY derived output regardless of its input source, so the gate covers all three (task #46, the
//! broadening past the original retired-floor-plus-manifest-key shape). The MECHANISM (the gate that
//! perturbs an input and asserts the derived value responds) is fixed Rust; the MEMBERSHIP is data
//! and grows with the world, sibling to the value, semantic, institution-function, and provenance
//! substrates. Each row's `id` matches the `@derives[id]:` token at its site, and the cross-check
//! (in the gate's test) asserts every annotated site has a row and every row a site, the ratchet the
//! constructor gate already models.
//!
//! The membership and its cross-check are the first surface. The site-local liveness probe (perturb
//! the declared input, assert the derived output responds at its site) is the pass/fail gate below: a
//! live-but-byte-neutral derivation whose effect lands in a downstream deadband is a coverage gap the
//! separate coverage signal reports, never a liveness failure (the gate's ruling).

use civsim_core::Fixed;
use civsim_foundation::calibration::CalibrationManifest;

/// Whether a derivation replaced an authored floor or is a new derived output. The liveness principle
/// (perturb the input, assert the output responds) applies to both; the category only records which
/// class a row is, so the retired-floor cases (the soil_baseline-bug class) stay named (task #46).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DerivationCategory {
    /// A derived value that REPLACED an authored floor: the class behind the soil_baseline bug.
    RetiredFloor,
    /// A derived output that retired no floor (new physics or a new derivation), for example the
    /// convection subsystem's stepped column state.
    NewDerivation,
}

impl DerivationCategory {
    /// Parse the canonical table's category token. Panics on a malformed token (canonical data, a
    /// construction invariant, not a runtime input).
    fn parse(token: &str) -> DerivationCategory {
        match token {
            "retired-floor" => DerivationCategory::RetiredFloor,
            "new-derivation" => DerivationCategory::NewDerivation,
            other => panic!("unknown derivation category '{other}'"),
        }
    }
}

/// Where a derivation's perturbable input lives. The gate perturbs whichever source and asserts the
/// derived output responds, so the liveness principle covers a manifest scalar, a data-defined driver
/// parameter, or a resident simulation-state field alike (task #46). The declared source names what
/// the probe perturbs, so the coverage signal never mistakes a driver-param or field input for a
/// missing manifest key (the phantom `decompose.decomposer_rate` bug the broadening closes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputSource {
    /// A reserved manifest scalar, perturbed via `require_fixed(key)`.
    ManifestKey(String),
    /// A named parameter on a data-defined driver row (for example a `DecomposerDriver` param).
    DriverParam {
        /// The driver the parameter lives on, in prose (a `DecomposerDriver` kind, say `life`).
        driver: String,
        /// The parameter name the probe perturbs (for example `biomass_reference`).
        param: String,
    },
    /// A field on a resident simulation-state struct (for example a `ColumnParams` field).
    ResidentField {
        /// The struct the field lives on, in prose (for example `ColumnParams`).
        holder: String,
        /// The field name the probe perturbs (for example `heat_production`).
        field: String,
    },
}

impl InputSource {
    /// Parse the canonical table's compact input encoding: `manifest:<key>`,
    /// `driver:<driver>/<param>`, or `field:<holder>/<field>`. Panics on a malformed spec (canonical
    /// data, a construction invariant, not a runtime input).
    fn parse(spec: &str) -> InputSource {
        if let Some(key) = spec.strip_prefix("manifest:") {
            InputSource::ManifestKey(key.to_string())
        } else if let Some(rest) = spec.strip_prefix("driver:") {
            let (driver, param) = rest
                .split_once('/')
                .unwrap_or_else(|| panic!("malformed driver input spec '{spec}'"));
            InputSource::DriverParam {
                driver: driver.to_string(),
                param: param.to_string(),
            }
        } else if let Some(rest) = spec.strip_prefix("field:") {
            let (holder, field) = rest
                .split_once('/')
                .unwrap_or_else(|| panic!("malformed field input spec '{spec}'"));
            InputSource::ResidentField {
                holder: holder.to_string(),
                field: field.to_string(),
            }
        } else {
            panic!("input spec '{spec}' has no source prefix (manifest:/driver:/field:)")
        }
    }

    /// A short label of the source kind, for the coverage report.
    pub fn kind(&self) -> &'static str {
        match self {
            InputSource::ManifestKey(_) => "manifest-key",
            InputSource::DriverParam { .. } => "driver-param",
            InputSource::ResidentField { .. } => "resident-field",
        }
    }
}

/// One derivation the liveness gate tracks: a derived value (a retired-floor replacement or a new
/// derived output) and the metadata the gate reads to probe it. All fields are data, so a new
/// derivation is a new row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derivation {
    /// The stable id, matching the `@derives[id]:` token at the site.
    pub id: String,
    /// What the derivation produces, in prose (the derived quantity).
    pub derived: String,
    /// Whether the derivation replaced an authored floor or is a new derived output.
    pub category: DerivationCategory,
    /// The input sources the derivation reads, the values the gate perturbs to prove the output
    /// responds. Empty is not allowed: a derivation with no declared input has nothing to perturb, so
    /// the gate could not prove it alive.
    pub inputs: Vec<InputSource>,
    /// The run scenario that arms the derivation on the canonical path.
    pub scenario: String,
    /// The source file carrying this derivation's `@derives[id]:` annotation, repo-relative.
    pub site: String,
}

/// The registry of retired-floor derivations. Ordered by registration, so any canonical walk over
/// it is deterministic (the same discipline the quantity and unit registries follow). The built-in
/// membership is [`DerivationRegistry::canonical`]; a world may register more.
#[derive(Clone, Debug, Default)]
pub struct DerivationRegistry {
    entries: Vec<Derivation>,
}

impl DerivationRegistry {
    /// An empty registry.
    pub fn new() -> DerivationRegistry {
        DerivationRegistry::default()
    }

    /// Register a derivation. Panics on a duplicate id or an empty input list, so the registry
    /// cannot carry a row the gate could not probe or two rows for one id.
    pub fn register(&mut self, d: Derivation) {
        assert!(
            !d.inputs.is_empty(),
            "derivation '{}' declares no input to perturb",
            d.id
        );
        assert!(
            !self.entries.iter().any(|e| e.id == d.id),
            "duplicate derivation id '{}'",
            d.id
        );
        self.entries.push(d);
    }

    /// Walk the derivations in canonical registration order.
    pub fn iter(&self) -> impl Iterator<Item = &Derivation> {
        self.entries.iter()
    }

    /// The number of registered derivations.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The registered ids, in canonical order.
    pub fn ids(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.id.as_str()).collect()
    }

    /// The built-in membership: every retired-floor derivation the codebase carries today, each
    /// keyed to its `@derives[id]:` site. This is the data the gate reads; extending it is adding a
    /// row, never changing the gate.
    pub fn canonical() -> DerivationRegistry {
        let mut r = DerivationRegistry::new();
        for (id, derived, inputs, scenario, site, category) in CANONICAL {
            r.register(Derivation {
                id: id.to_string(),
                derived: derived.to_string(),
                category: DerivationCategory::parse(category),
                inputs: inputs.iter().map(|s| InputSource::parse(s)).collect(),
                scenario: scenario.to_string(),
                site: site.to_string(),
            });
        }
        r
    }
}

/// The built-in derivation rows as data: `(id, derived, inputs, scenario, site, category)`. Each id
/// matches an `@derives[id]:` annotation at its site. Each input is the compact source encoding
/// [`InputSource::parse`] reads (`manifest:<key>`, `driver:<driver>/<param>`, or
/// `field:<holder>/<field>`); the liveness probe perturbs one and asserts the derived output moves.
/// The category is `retired-floor` (replaced an authored floor) or `new-derivation` (task #46).
/// One built-in derivation row as data: `(id, derived, inputs, scenario, site, category)`.
type CanonicalRow = (
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static str,
    &'static str,
);

const CANONICAL: &[CanonicalRow] = &[
    (
        "world_time_cadence",
        "a world's DAY cadence in ticks (the rotation-derived beat: aging, drift, the diurnal calendar)",
        // Repointed off the celestial.rs passthrough (OrbitalElements reads the period straight
        // through, a vacuous identity probe) to the substantive downstream derivation, AND
        // differentiated from clock_calendar_cell: this row floors the ROTATION period (the day),
        // clock_calendar_cell the ORBITAL period (the year), so the two temporal cadences are distinct
        // derivations sharing the ticks_from_seconds kernel (gate ruling, #168).
        &["manifest:world.rotation_period_seconds"],
        "default",
        "parked/crates/foundation/src/clock.rs",
        "retired-floor",
    ),
    (
        "locomotion_speed_cell",
        "movement speed in tiles per tick and the cell edge in metres",
        &["manifest:locomotion.base_speed_m_per_s"],
        "default",
        "parked/crates/sim/src/locomotion.rs",
        "retired-floor",
    ),
    (
        "clock_calendar_cell",
        "a world's year, day, and season in ticks, and the cell area",
        &["manifest:world.orbital_period_seconds"],
        "default",
        "parked/crates/foundation/src/clock.rs",
        "retired-floor",
    ),
    (
        "hydrology_water",
        "local water presence, rainfall, evaporation, and runoff",
        // Repointed off the retired `hydrology.saturation_t_ref` (Arc-2 derive-vs-author, in no
        // profile, a probe on it fails-loud) to a live reserved key the water balance reads: the
        // condensation rate scales the water added (`precip = precip_rate * excess` in step_hydrology),
        // so perturbing it moves the derived water output (gate ruling, #168).
        &["manifest:hydrology.precipitation_rate"],
        "full",
        "parked/crates/sim/src/environ.rs",
        "retired-floor",
    ),
    (
        "productivity_capacity",
        "per-cell biomass productivity and carrying capacity",
        &["manifest:productivity.soil_requirement"],
        "full",
        "parked/crates/sim/src/environ.rs",
        "retired-floor",
    ),
    (
        "metabolic_rate",
        "a being's metabolic rate, energy drain, and heat loss",
        // The real manifest scalar the derivation reads (`MetabolicAnchors::from_manifest`), corrected
        // from the field name `kleiber_a` so the declared source names the perturbed key (task #46).
        &["manifest:metabolism.kleiber_coefficient"],
        "full",
        "parked/crates/sim/src/physiology.rs",
        "retired-floor",
    ),
    (
        "decomposition_recovery",
        "soil-nutrient recovery through the matter cycle",
        // Repointed off the phantom manifest key `decompose.decomposer_rate` (read by no require_fixed,
        // in no profile) to the real input source: the `biomass_reference` parameter on a
        // DecomposerDriver Life row, a data-defined driver-param, not a manifest scalar (task #46, the
        // driver-param case the broadening turns from Unwired into a real probe).
        &["driver:life/biomass_reference"],
        "full",
        "parked/crates/foundation/src/decompose.rs",
        "retired-floor",
    ),
    (
        "weathering_soil_nutrient",
        "abiotic mineral-weathering soil-nutrient supply (rock into soil nutrient before any biomass)",
        &["manifest:weathering.mineral_dissolution_rate"],
        "living",
        "parked/crates/sim/src/environ.rs",
        "retired-floor",
    ),
    (
        "carbon_fixation_rate",
        "per-cell carbon-fixation rate / net primary productivity",
        &["manifest:photosynthesis.light_saturation"],
        "living",
        "parked/crates/sim/src/environ.rs",
        "retired-floor",
    ),
    (
        "column_convection",
        "the interior column's stepped thermal state (the convection subsystem's derived temperature)",
        // A NEW derived output that retired no floor (new physics), covered now the gate is broadened
        // beyond retired-floor (task #46): the input is a ColumnParams field, a resident-state value,
        // not a manifest scalar. Perturbing the heat production moves the stepped column temperature.
        &["field:ColumnParams/heat_production"],
        // The subsystem is not yet armed on any run scenario (the column-wiring slice is held for A's
        // GeodynamicColumn contract), so the probe is site-local over a synthetic column. `default` is
        // the scenario it will run under once wired; the field is documentation, unused by the probe.
        "default",
        "crates/planet-substrate/src/geodynamics.rs",
        "new-derivation",
    ),
];

// --- The site-local liveness probe (task #43 slice 2): the pass/fail gate ---
//
// The registry above is the membership; this is the mechanism. For a wired derivation, the probe
// evaluates the derived output at a fixed representative situation twice: once from the base
// manifest, and once with the derivation's declared reserved input perturbed. A LIVE derivation's
// output responds (the two readings differ); a DEAD one holds the same value regardless of its input,
// the soil_baseline bug (a large sweep of the photosynthesis constants left the derived output, and so
// the state hash, byte-identical). The gate keys on the SITE-LOCAL value, not a distant hash
// consequence, so a live-but-byte-neutral derivation (its effect landing in a downstream deadband on
// the pinned scenarios) is never mistaken for dead (the gate's ruling; that coverage gap is the
// separate softer signal). The perturbation is a structural test factor, never a world value, and
// never enters the sim: the gate is tooling that reads run state and authors nothing on the run path.

/// The two site-local readings of a derived output: its value from the base manifest, and its value
/// with the declared reserved input perturbed. The gate compares them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProbeReading {
    /// The derived output at the representative situation, base manifest.
    pub baseline: Fixed,
    /// The derived output at the same situation, with the declared input perturbed.
    pub perturbed: Fixed,
}

/// The liveness verdict for one derivation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Liveness {
    /// The derived output responded to its declared input: alive.
    Live,
    /// The derived output was identical under the perturbation: dead (the soil_baseline bug).
    Dead,
}

/// The pass/fail rule: a reading whose two values differ is live; identical values are dead. Exact
/// fixed-point equality (Principle 3), so a response of a single ULP already reads live and only a
/// constant output reads dead.
pub fn assess(reading: &ProbeReading) -> Liveness {
    if reading.baseline == reading.perturbed {
        Liveness::Dead
    } else {
        Liveness::Live
    }
}

/// A site-local liveness probe: given the base manifest, it returns the two readings of one
/// derivation's output. The closure owns the mapping from the derivation's declared reserved key to
/// the kernel input it perturbs, so the mechanism ([`assess`]) stays derivation-agnostic. Returns an
/// error string if the manifest cannot supply the derivation's constants (fail-loud, never a silent
/// pass).
pub type LivenessProbe = fn(&CalibrationManifest) -> Result<ProbeReading, String>;

/// The wired probe for a registry id, or `None` where a derivation is registered (so the cross-check
/// covers it) but its site-local probe is not yet wired. A `None` is an honest coverage gap in the
/// probe wiring, reported as such, never a false [`Liveness::Dead`]. The flagship carbon-fixation case
/// (the soil_baseline bug's own site) is wired; the remaining derivations' probes follow as data-plus-
/// closure, each a kernel call reachable from this crate (the sim crate is downstream of core,
/// physics, and world).
pub fn liveness_probe(id: &str) -> Option<LivenessProbe> {
    match id {
        "carbon_fixation_rate" => Some(probe_carbon_fixation_rate),
        // The two temporal cadences are distinct derivations sharing the `ticks_from_seconds` kernel:
        // the calendar row floors the ORBITAL period (the year), the world-time row the ROTATION
        // period (the day). Different inputs, so different probes (gate ruling, #168).
        "clock_calendar_cell" => Some(probe_year_cadence),
        "world_time_cadence" => Some(probe_day_cadence),
        "metabolic_rate" => Some(probe_metabolic_rate),
        "productivity_capacity" => Some(probe_productivity_capacity),
        // The two broadening proof cases (task #46): a driver-param input and a resident-field input.
        "decomposition_recovery" => Some(probe_decomposition_recovery),
        "column_convection" => Some(probe_column_convection),
        _ => None,
    }
}

/// The ids whose site-local probe is not yet wired, each with the reason its gap stands (an honest
/// coverage gap, never a false [`Liveness::Dead`]). The reason is data, so a newly wired probe drops
/// its row from this table and a newly registered derivation adds one. The CI harness asserts every
/// unwired id carries a reason here, so no gap is silent.
pub fn unwired_gap_reason(id: &str) -> Option<&'static str> {
    match id {
        "locomotion_speed_cell" => Some(
            "the grown-limb speed kernel needs a representative BodyPlan and organ registry; \
             wireable with a dev body, deferred as bulky scaffolding",
        ),
        "weathering_soil_nutrient" => Some(
            "the deposit rate (`base_rate * wet`) is computed inline in the `weather_minerals` grid \
             loop; no pure kernel to call site-locally without a run-path refactor or a single-cell \
             Field situation, deferred",
        ),
        "hydrology_water" => Some(
            "`step_hydrology` is a private `&mut self` grid method; the input is repointed to the \
             live `hydrology.precipitation_rate` (source-verified to move the water balance), and \
             the probe is wireable via a minimal Field situation once the step is exposed, deferred",
        ),
        _ => None,
    }
}

/// The perturbation the site-local probes apply to a declared input: double it. A structural test
/// factor (never a world value, never on the run path); for the site-local assertion the magnitude
/// only has to move the output past its own ULP, which any non-degenerate factor does.
const PROBE_PERTURBATION: i32 = 2;

/// The flagship probe: the per-cell carbon-fixation rate must respond to `photosynthesis.light_saturation`.
/// The situation is a fertile daylit cell (every non-light Liebig factor saturated), so the light term
/// is limiting and the light-saturation constant reaches the derived rate; doubling it must move the
/// output. This is the exact site of the soil_baseline bug: with the soil factor barren the same kernel
/// goes dead (its output zero regardless of the constants), the case the slice's proof-of-fire test
/// reproduces.
fn probe_carbon_fixation_rate(m: &CalibrationManifest) -> Result<ProbeReading, String> {
    use crate::environ::{carbon_fixation_rate, PhotosynthesisCalib};
    let base = PhotosynthesisCalib::from_manifest(m)
        .map_err(|e| format!("carbon_fixation_rate probe: {e}"))?;
    // A representative daylit, fertile situation: full insolation under the Earth stellar constant
    // (1361 W/m^2, the flux the derivation's own doc-comment cites), the temperature at the enzyme
    // optimum, and water and soil supplied well past their requirements, so every non-light factor
    // saturates and the light-saturation constant reaches the derived rate. Situation scaffolding for
    // the two-point read; it never enters the sim.
    let insolation = Fixed::ONE;
    let solar_constant = Fixed::from_int(1361);
    let temperature = base.temp_optimum;
    let evaporative_demand = Fixed::ONE;
    let water_supply = Fixed::from_int(1000);
    let soil_fertility = Fixed::from_int(1000);
    let soil_requirement = Fixed::ONE;
    let read = |photo: &PhotosynthesisCalib| {
        carbon_fixation_rate(
            insolation,
            solar_constant,
            temperature,
            evaporative_demand,
            water_supply,
            soil_fertility,
            soil_requirement,
            photo,
        )
    };
    let baseline = read(&base);
    let mut perturbed_calib = base;
    perturbed_calib.light_saturation = base
        .light_saturation
        .checked_mul(Fixed::from_int(PROBE_PERTURBATION))
        .ok_or_else(|| {
            "carbon_fixation_rate probe: light_saturation perturbation overflowed".to_string()
        })?;
    let perturbed = read(&perturbed_calib);
    Ok(ProbeReading {
        baseline,
        perturbed,
    })
}

/// The shared tick-cadence probe body: floor a temporal period over the base tick and assert the tick
/// count responds to the period. `ticks_from_seconds` is a non-trivial floored division (distinct from
/// the raw period read the world-time row was repointed off), so doubling the period moves the count.
/// The count is a `u64`, read into [`Fixed`] for the two-point comparison, exact for any realistic
/// cadence (well under the fixed-point integer range). `label` names the cadence in error text.
fn probe_tick_cadence(
    m: &CalibrationManifest,
    label: &str,
    period: Fixed,
) -> Result<ProbeReading, String> {
    use civsim_foundation::clock::{base_tick_seconds_fixed, ticks_from_seconds};
    let base_tick =
        base_tick_seconds_fixed(m).map_err(|e| format!("{label} cadence probe: {e}"))?;
    let cadence_ticks = |seconds: Fixed| -> Result<Fixed, String> {
        let ticks = ticks_from_seconds(seconds, base_tick)
            .map_err(|e| format!("{label} cadence probe: {e:?}"))?;
        i32::try_from(ticks).map(Fixed::from_int).map_err(|_| {
            format!("{label} cadence probe: the tick count exceeds the probe's read range")
        })
    };
    let baseline = cadence_ticks(period)?;
    let perturbed_period = period
        .checked_mul(Fixed::from_int(PROBE_PERTURBATION))
        .ok_or_else(|| format!("{label} cadence probe: period perturbation overflowed"))?;
    let perturbed = cadence_ticks(perturbed_period)?;
    Ok(ProbeReading {
        baseline,
        perturbed,
    })
}

/// The year-cadence probe (`clock_calendar_cell`): the year in ticks must respond to
/// `world.orbital_period_seconds`, the orbital period floored over the base tick.
fn probe_year_cadence(m: &CalibrationManifest) -> Result<ProbeReading, String> {
    use civsim_foundation::clock::orbital_from_manifest;
    let orbit = orbital_from_manifest(m).map_err(|e| format!("year cadence probe: {e}"))?;
    probe_tick_cadence(m, "year", orbit.orbital_period_seconds)
}

/// The day-cadence probe (`world_time_cadence`): the day in ticks must respond to
/// `world.rotation_period_seconds`, the rotation period floored over the base tick. Distinct from the
/// year cadence (a different period), so the two temporal rows are meaningfully separate derivations
/// rather than one probe under two ids (gate ruling, #168). The rotation cadence is a real run-path
/// derivation: `DiurnalSky::rotation_period_ticks` drives the diurnal cycle.
fn probe_day_cadence(m: &CalibrationManifest) -> Result<ProbeReading, String> {
    use civsim_foundation::clock::orbital_from_manifest;
    let orbit = orbital_from_manifest(m).map_err(|e| format!("day cadence probe: {e}"))?;
    probe_tick_cadence(m, "day", orbit.rotation_period_seconds)
}

/// The metabolic-rate probe: a being's basal metabolic rate must respond to
/// `metabolism.kleiber_coefficient`. Kleiber's law P = a * m^(3/4) reads the coefficient as a linear
/// prefactor over the body mass, so doubling it moves the rate at any positive mass. The situation is
/// one representative body mass; the cap is a structural non-clipping ceiling for the two-point read,
/// never a world value.
fn probe_metabolic_rate(m: &CalibrationManifest) -> Result<ProbeReading, String> {
    use crate::physiology::MetabolicAnchors;
    use civsim_physics::laws::basal_metabolic_rate;
    // A cap far above any basal rate the perturbed coefficient reaches at the representative mass, so
    // the probe reads the unclipped Kleiber response rather than a saturated ceiling.
    let rate_cap = Fixed::from_int(1_000_000_000);
    let mass_kg = Fixed::from_int(10);
    let base =
        MetabolicAnchors::from_manifest(m).map_err(|e| format!("metabolic_rate probe: {e}"))?;
    let baseline = basal_metabolic_rate(mass_kg, base.kleiber_a, rate_cap);
    let perturbed_coeff = base
        .kleiber_a
        .checked_mul(Fixed::from_int(PROBE_PERTURBATION))
        .ok_or_else(|| {
            "metabolic_rate probe: kleiber-coefficient perturbation overflowed".to_string()
        })?;
    let perturbed = basal_metabolic_rate(mass_kg, perturbed_coeff, rate_cap);
    Ok(ProbeReading {
        baseline,
        perturbed,
    })
}

/// The productivity-capacity probe: per-cell biomass productivity must respond to
/// `productivity.soil_requirement`. `biomass_from` takes the Liebig minimum over the four factor
/// satisfactions, each supply over its requirement; the situation saturates water, light, and
/// temperature and holds the soil supply at the base requirement so soil is the binding factor.
/// Doubling the soil requirement halves the soil satisfaction, so the minimum (and the biomass)
/// moves. This mirrors the soil_baseline family: a requirement the derived output must track.
fn probe_productivity_capacity(m: &CalibrationManifest) -> Result<ProbeReading, String> {
    use crate::environ::{biomass_from, EnvironCalib};
    let base =
        EnvironCalib::from_manifest(m).map_err(|e| format!("productivity_capacity probe: {e}"))?;
    // Water, light, and temperature supplied well past their requirements (each factor saturates at
    // one), soil held at its own requirement so it is the binding Liebig factor. Situation
    // scaffolding for the two-point read; it never enters the sim.
    let saturated = Fixed::from_int(1000);
    let soil_supply = base.soil_req;
    let read =
        |calib: &EnvironCalib| biomass_from(saturated, saturated, saturated, soil_supply, calib);
    let baseline = read(&base);
    let mut perturbed_calib = base;
    perturbed_calib.soil_req = base
        .soil_req
        .checked_mul(Fixed::from_int(PROBE_PERTURBATION))
        .ok_or_else(|| {
            "productivity_capacity probe: soil-requirement perturbation overflowed".to_string()
        })?;
    let perturbed = read(&perturbed_calib);
    Ok(ProbeReading {
        baseline,
        perturbed,
    })
}

/// The decomposition-recovery probe (task #46, the DRIVER-PARAM proof): the matter-cycle recovery
/// activity must respond to the `biomass_reference` parameter on a `DecomposerDriver` Life row, a
/// data-defined driver-param rather than a manifest scalar (the input source the broadening adds). The
/// situation is a single Life row at a favourable standing stock in the rising region (below the
/// reference), so doubling the reference biomass, the standing crop at which activity saturates, moves
/// the activity. This turns the phantom-input row into a real probe. The manifest is unused: the input
/// lives on the driver, not the manifest.
fn probe_decomposition_recovery(_m: &CalibrationManifest) -> Result<ProbeReading, String> {
    use civsim_foundation::decompose::{DecomposerDriver, DecomposerDriverRegistry};
    // A registry of one Life row, so `activity_at` returns that row's contribution:
    // saturating_response(life_stock, biomass_reference). A stock below the reference keeps the read in
    // the rising region (not saturated at either end). Situation scaffolding; it never enters the sim.
    let life_stock = Fixed::ONE;
    let base_reference = Fixed::from_int(2);
    let activity = |reference: Fixed| {
        let mut reg = DecomposerDriverRegistry::new();
        reg.push(DecomposerDriver::life(reference));
        reg.activity_at(&[], life_stock)
    };
    let baseline = activity(base_reference);
    let perturbed_reference = base_reference
        .checked_mul(Fixed::from_int(PROBE_PERTURBATION))
        .ok_or_else(|| {
            "decomposition_recovery probe: biomass-reference perturbation overflowed".to_string()
        })?;
    let perturbed = activity(perturbed_reference);
    Ok(ProbeReading {
        baseline,
        perturbed,
    })
}

/// The convection-subsystem probe (task #46, the RESIDENT-FIELD proof, a NEW derived output that
/// retired no floor): the stepped column temperature must respond to the `heat_production` field on
/// `ColumnParams`, a resident-state value rather than a manifest scalar. One convection step reads the
/// heat production into the internal-heat balance, so doubling it moves the stepped temperature. This
/// proves the broadened gate covers a derivation beyond the retired-floor class. The manifest is
/// unused: the input lives on the resident column parameters, not the manifest.
fn probe_column_convection(_m: &CalibrationManifest) -> Result<ProbeReading, String> {
    use crate::geodynamics::{convection_step, ColumnParams, ColumnState};
    // A representative warm interior over a cold reference, the same shape the geodynamics tests use.
    // Situation scaffolding for the two-point read; it never enters the sim.
    let state = ColumnState {
        temperature: Fixed::from_int(400),
        convecting: false,
    };
    let params = |heat_production: Fixed| ColumnParams {
        reference_temperature: Fixed::from_int(300),
        density: Fixed::ONE,
        thermal_conductivity: Fixed::from_int(2),
        thermal_expansion_ppm: Fixed::from_int(30),
        gravity: Fixed::from_int(10),
        depth: Fixed::ONE,
        radius: Fixed::ONE,
        viscosity: Fixed::ONE,
        specific_heat: Fixed::from_int(10),
        heat_production,
        ra_crit: Fixed::from_int(2000),
        ra_crit_wavenumber: Fixed::from_ratio(3117, 1000),
        ra_max: Fixed::from_int(1_000_000),
        v_max: Fixed::from_int(1_000_000),
        flux_max: Fixed::from_int(1_000_000),
        stress_max: Fixed::from_int(1_000_000),
        dt: Fixed::ONE,
    };
    let base_heat = Fixed::from_int(100);
    let baseline = convection_step(&state, &params(base_heat)).temperature;
    let perturbed_heat = base_heat
        .checked_mul(Fixed::from_int(PROBE_PERTURBATION))
        .ok_or_else(|| {
            "column_convection probe: heat-production perturbation overflowed".to_string()
        })?;
    let perturbed = convection_step(&state, &params(perturbed_heat)).temperature;
    Ok(ProbeReading {
        baseline,
        perturbed,
    })
}

// --- The coverage signal (task #43 slice 3): the classification beside the pass/fail verdict ---
//
// A green gate must never be read as proof where the probe is absent or weak. Beside the Live/Dead
// verdict, every registered derivation carries a coverage classification, so the gaps stay visible.
// Only Dead fails CI (a derived output gone constant blocks the merge, the soil_baseline regression);
// the other states are surfaced as gaps to close over time, never failures. This extends C's
// live-but-deadband idea to the trivial and unwired cases (the gate's ruling, #168).

/// The coverage classification of one registered derivation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// A wired probe ran and the derived output responded to its input: the pass.
    Live,
    /// A wired probe ran and the derived output held constant under the perturbation: the fail (the
    /// soil_baseline bug). The only state that fails CI.
    Dead,
    /// Registered and cross-checked, but no probe closure yet. An honest gap ([`unwired_gap_reason`]
    /// names why), never a false [`Coverage::Dead`].
    Unwired,
    /// The site-local output is an identity read of the perturbed input, so a probe there is
    /// structurally guaranteed Live and proves nothing (the passthrough case). The standing guard
    /// against a passthrough slipping into the registry as a false Live. No row is Trivial today
    /// (world_time_cadence was repointed off its passthrough), so this is the safety net for the
    /// future, reported as a gap.
    Trivial,
    /// The probe reads Live at its site, but the derivation's effect lands in a downstream deadband
    /// on the four canonical pins, so a scenario-level hash sweep would show it byte-neutral. A
    /// coverage gap reported softly, never a liveness failure: the site-local read already proves the
    /// output is alive. Detecting this needs the pinned scenario runs, out of the site-local harness's
    /// scope, so it is a declared classification (data), surfaced for a future scenario-level signal.
    DeadbandOnPins,
}

/// One row of the coverage report: a registered derivation and its classification. When the state is
/// [`Coverage::Unwired`], `note` carries the gap reason; for a wired probe that failed to read (a
/// fail-loud manifest error), `note` carries the error, and the row is left [`Coverage::Unwired`]
/// rather than a false Dead (a probe that could not run has not proven the output dead).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverageRow {
    /// The derivation id.
    pub id: String,
    /// Its coverage classification.
    pub coverage: Coverage,
    /// The gap reason for a non-Live/Dead state, or a probe error, in prose; empty for a clean
    /// Live/Dead verdict.
    pub note: String,
}

/// The full coverage report over the canonical registry, read against a base manifest: run each wired
/// probe and classify it Live or Dead, classify each unwired id Unwired with its gap reason, and pass
/// through the declared Trivial / DeadbandOnPins classifications. Deterministic (registry order), the
/// same discipline the registry walk follows. This is the data the CI harness asserts over and prints.
pub fn coverage_report(m: &CalibrationManifest) -> Vec<CoverageRow> {
    let registry = DerivationRegistry::canonical();
    let mut rows = Vec::with_capacity(registry.len());
    for d in registry.iter() {
        let row = match liveness_probe(&d.id) {
            Some(probe) => match probe(m) {
                Ok(reading) => CoverageRow {
                    id: d.id.clone(),
                    coverage: match assess(&reading) {
                        Liveness::Live => Coverage::Live,
                        Liveness::Dead => Coverage::Dead,
                    },
                    note: String::new(),
                },
                // A probe that fails to read (a fail-loud manifest error) has NOT proven the output
                // dead: it is an unwired-for-this-manifest gap, carrying the error, never a false Dead.
                Err(e) => CoverageRow {
                    id: d.id.clone(),
                    coverage: Coverage::Unwired,
                    note: e,
                },
            },
            None => CoverageRow {
                id: d.id.clone(),
                coverage: Coverage::Unwired,
                note: unwired_gap_reason(&d.id)
                    .unwrap_or("no probe wired and no gap reason recorded")
                    .to_string(),
            },
        };
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_canonical_registry_carries_the_annotated_derivations() {
        let r = DerivationRegistry::canonical();
        assert_eq!(r.len(), 10, "one row per @derives[id] site today");
        // Every row declares at least one input to perturb (register enforces it), a scenario, and a
        // site, so the gate has what it needs to probe each.
        for d in r.iter() {
            assert!(!d.inputs.is_empty());
            assert!(!d.scenario.is_empty());
            assert!(d.site.ends_with(".rs"));
        }
    }

    #[test]
    fn the_walk_is_deterministic_registration_order() {
        let r = DerivationRegistry::canonical();
        let ids = r.ids();
        assert_eq!(ids.first(), Some(&"world_time_cadence"));
        assert_eq!(ids.last(), Some(&"column_convection"));
        // A second walk yields the identical order (no hash-map iteration leaks in).
        let again = DerivationRegistry::canonical();
        assert_eq!(again.ids(), ids);
    }

    #[test]
    #[should_panic(expected = "duplicate derivation id")]
    fn a_duplicate_id_panics() {
        let mut r = DerivationRegistry::canonical();
        r.register(Derivation {
            id: "metabolic_rate".to_string(),
            derived: "x".to_string(),
            category: DerivationCategory::RetiredFloor,
            inputs: vec![InputSource::ManifestKey("y".to_string())],
            scenario: "full".to_string(),
            site: "z.rs".to_string(),
        });
    }

    #[test]
    #[should_panic(expected = "declares no input to perturb")]
    fn an_inputless_derivation_panics() {
        let mut r = DerivationRegistry::new();
        r.register(Derivation {
            id: "x".to_string(),
            derived: "x".to_string(),
            category: DerivationCategory::NewDerivation,
            inputs: vec![],
            scenario: "full".to_string(),
            site: "z.rs".to_string(),
        });
    }

    // --- The site-local liveness probe (slice 2) ---

    /// The base manifest the probes read: the labelled dev-fixtures profile, which carries the
    /// photosynthesis constants set (not reserved). The probe never enters the sim, so a fixture
    /// base is correct: it is tooling reading run state.
    const DEV_FIXTURES: &str = include_str!("../../../calibration/profiles/dev-fixtures.toml");

    fn base_manifest() -> CalibrationManifest {
        CalibrationManifest::from_toml_str(DEV_FIXTURES).expect("dev-fixtures profile parses")
    }

    #[test]
    fn every_wired_probe_id_is_a_registered_derivation() {
        // A probe may only exist for an id the registry carries, so the cross-check that guards the
        // membership also guards the probe wiring. (The reverse is allowed: a registered derivation
        // whose probe is not yet wired is an honest coverage gap, a None, never a false Dead.)
        let registry = DerivationRegistry::canonical();
        let ids = registry.ids();
        for d in registry.iter() {
            if liveness_probe(&d.id).is_some() {
                assert!(
                    ids.contains(&d.id.as_str()),
                    "wired probe id '{}' is not a registered derivation",
                    d.id
                );
            }
        }
        // The flagship is wired.
        assert!(liveness_probe("carbon_fixation_rate").is_some());
        // An unregistered id has no probe.
        assert!(liveness_probe("not_a_derivation").is_none());
    }

    #[test]
    fn the_flagship_carbon_fixation_probe_reads_live() {
        // The live derivation: perturbing photosynthesis.light_saturation moves the derived rate at
        // its site, so the probe reads Live. This is the pass side of the gate.
        let m = base_manifest();
        let probe = liveness_probe("carbon_fixation_rate").expect("the flagship probe is wired");
        let reading =
            probe(&m).expect("the fixture manifest supplies the photosynthesis constants");
        assert_ne!(
            reading.baseline, reading.perturbed,
            "a live carbon-fixation rate must respond to its light-saturation input"
        );
        assert_eq!(assess(&reading), Liveness::Live);
    }

    #[test]
    fn the_gate_fires_on_the_soil_baseline_dead_case() {
        // The proof-of-fire: reproduce the exact soil_baseline dead output. With the soil factor
        // barren (fertility zero, the soil-bootstrap deadlock the mineral-weathering arm was added to
        // break), the Liebig minimum zeroes the derived rate regardless of the photosynthesis
        // constants, so perturbing light_saturation moves nothing. assess must read Dead, catching the
        // silent regression the constructor gate cannot see.
        use crate::environ::{carbon_fixation_rate, PhotosynthesisCalib};
        let m = base_manifest();
        let base = PhotosynthesisCalib::from_manifest(&m).expect("photosynthesis constants set");
        let barren_read = |photo: &PhotosynthesisCalib| {
            carbon_fixation_rate(
                Fixed::ONE,            // full insolation
                Fixed::from_int(1361), // stellar constant
                base.temp_optimum,     // optimal temperature
                Fixed::ONE,            // small evaporative demand
                Fixed::from_int(1000), // ample water
                Fixed::ZERO,           // BARREN soil: the dead regime
                Fixed::ONE,            // soil requirement
                photo,
            )
        };
        let mut doubled = base;
        doubled.light_saturation = base.light_saturation.mul(Fixed::from_int(2));
        let dead = ProbeReading {
            baseline: barren_read(&base),
            perturbed: barren_read(&doubled),
        };
        assert_eq!(
            dead.baseline,
            Fixed::ZERO,
            "the barren regime fixes no carbon"
        );
        assert_eq!(
            assess(&dead),
            Liveness::Dead,
            "a derived output constant under its input perturbation must read Dead"
        );
    }

    // --- The remaining wired probes (slice 4) ---

    #[test]
    fn the_two_temporal_cadences_read_live_on_distinct_periods() {
        // The clock row floors the ORBITAL period (the year), the world-time row the ROTATION period
        // (the day): distinct derivations sharing the ticks_from_seconds kernel, each responding to its
        // own period, so both read Live.
        let m = base_manifest();
        for id in ["clock_calendar_cell", "world_time_cadence"] {
            let probe = liveness_probe(id).unwrap_or_else(|| panic!("{id} is wired"));
            let reading =
                probe(&m).expect("the fixture manifest supplies the periods and base tick");
            assert_eq!(
                assess(&reading),
                Liveness::Live,
                "{id} must respond to its period"
            );
        }
    }

    #[test]
    fn the_metabolic_rate_probe_reads_live() {
        // Kleiber's law reads the coefficient as a linear prefactor, so the basal rate responds.
        let m = base_manifest();
        let probe = liveness_probe("metabolic_rate").expect("metabolic_rate is wired");
        let reading = probe(&m).expect("the fixture manifest supplies the Kleiber coefficient");
        assert_eq!(assess(&reading), Liveness::Live);
    }

    #[test]
    fn the_productivity_capacity_probe_reads_live() {
        // With soil the binding Liebig factor, doubling its requirement moves the biomass minimum.
        let m = base_manifest();
        let probe =
            liveness_probe("productivity_capacity").expect("productivity_capacity is wired");
        let reading =
            probe(&m).expect("the fixture manifest supplies the productivity requirements");
        assert_eq!(assess(&reading), Liveness::Live);
    }

    // --- The coverage signal (slice 3) ---

    #[test]
    fn the_coverage_report_has_no_dead_and_every_unwired_carries_a_reason() {
        // The CI shape in miniature: over the canonical registry, no row is Dead (the only failing
        // state), and every Unwired row carries a non-empty gap reason so no gap is silent.
        let m = base_manifest();
        let report = coverage_report(&m);
        assert_eq!(
            report.len(),
            10,
            "one coverage row per registered derivation"
        );
        for row in &report {
            assert_ne!(
                row.coverage,
                Coverage::Dead,
                "{} read Dead: a derived output went constant",
                row.id
            );
            if row.coverage == Coverage::Unwired {
                assert!(
                    !row.note.is_empty(),
                    "unwired row {} must carry a gap reason",
                    row.id
                );
            }
        }
    }

    #[test]
    fn the_wired_ids_read_live_including_the_two_broadening_proof_cases() {
        let m = base_manifest();
        let report = coverage_report(&m);
        let by_id = |id: &str| {
            report
                .iter()
                .find(|r| r.id == id)
                .expect("id present")
                .coverage
        };
        // The manifest-key wired derivations read Live, and so do the two proof cases the broadening
        // adds: decomposition_recovery (a driver-param input) and column_convection (a resident-field
        // input, a NEW derived output that retired no floor). Both were Unwired before task #46.
        for id in [
            "carbon_fixation_rate",
            "clock_calendar_cell",
            "world_time_cadence",
            "metabolic_rate",
            "productivity_capacity",
            "decomposition_recovery",
            "column_convection",
        ] {
            assert_eq!(by_id(id), Coverage::Live, "{id} should read Live");
        }
    }

    #[test]
    fn the_input_source_kinds_cover_the_three_broadened_sources() {
        // The registry now declares manifest-key, driver-param, and resident-field inputs, so the
        // broadening covers all three source kinds (task #46), not manifest keys alone.
        let r = DerivationRegistry::canonical();
        let kind_of = |id: &str| {
            r.iter()
                .find(|d| d.id == id)
                .and_then(|d| d.inputs.first())
                .map(|s| s.kind())
                .expect("id present with an input")
        };
        assert_eq!(kind_of("carbon_fixation_rate"), "manifest-key");
        assert_eq!(kind_of("decomposition_recovery"), "driver-param");
        assert_eq!(kind_of("column_convection"), "resident-field");
        // The new-derivation category is carried, distinct from the retired-floor class.
        let column = r
            .iter()
            .find(|d| d.id == "column_convection")
            .expect("present");
        assert_eq!(column.category, DerivationCategory::NewDerivation);
    }
}
