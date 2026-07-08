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

//! The DECOMPOSER-DRIVER substrate: what makes matter rot, expressed as world-carried data so that
//! decomposition emerges from a world's life and conditions rather than from an engine law that all
//! warm matter decays (Principle 8).
//!
//! The material substrate ([`crate::material`]) and its matter cycle ([`crate::runner::Runner`]'s
//! `step_matter_cycle`) already break a cell's organic matter down over time, gated below the
//! substance's own thermal barrier (a frozen remains is preserved), and conserve the lost mass into
//! the ground. That beat as first written decayed EVERY warm cell of organic matter at the substance's
//! own rate unconditionally: a sterile, bone-dry, or airless cell rotted exactly as fast as a warm,
//! wet, colonised one. That unconditionality is an authored universal, the steering defect this
//! substrate removes: it inserts one per-cell ACTIVITY factor, in the unit interval, between the
//! substance's maximum decomposition susceptibility (its `bio.decomposition_rate`, now read as a
//! ceiling) and the volume that breaks down this tick, so the rate a cell realises is set by
//! the world at that cell, never by the engine.
//!
//! The activity is the per-cell contribution of a data-defined set of DRIVER ROWS ([`DecomposerDriver`]),
//! each binding one of a fixed kernel set ([`DecomposerKernelId`]) to reserved parameters keyed by name,
//! combined by a data-defined [`CombineMode`] (the default gates the drivers against each other, so a
//! sterile cell does not decay even under favorable conditions; the alternative makes them independent
//! sufficient drivers). The mechanism is fixed Rust and the membership, parameters, and combine mode are
//! data (Principle 11), sibling to the transform-kernel substrate ([`crate::trace::TransformKernelId`]), the
//! value, semantic, and institution-function substrates. Two kernels are defined:
//!
//! - The CONDITIONS kernel is the abiotic microbial-activity PROXY: a Liebig minimum of saturating
//!   responses over a world-declared set of condition AXES ([`ConditionAxis`], Arc 5 T6), each a
//!   [`ConditionSource`] and a reference scale. The Earth-aerobic fixture declares three: MOISTURE (the
//!   precipitation and soil-moisture the environment field carries, not standing water, since waterlogging
//!   suppresses decomposition through the oxygen term), RESPIRABLE (the same respirable content the
//!   combustion beat reads, so a submerged or sealed cell throttles and anoxia falls out for free), and
//!   WARMTH above the substance's own thermal barrier. Any one response at zero yields activity zero, so a
//!   dry, airless, or barely thawed cell does not decay. The axis SET is data, not the engine's: an
//!   anaerobic world drops the respirable axis, a world whose decay needs a different solvent declares its
//!   own, so oxygen is not an engine-universal decomposition gate but one axis a world may or may not carry.
//! - The LIFE kernel is the emergent branch: a saturating response on the per-cell standing DECOMPOSER
//!   BIOMASS a world's own ecology deposits ([`DecomposerStockField`]). It is zero where no decomposer
//!   life is present, so under the default combine a warm, wet, airy but STERILE cell does not decay, the
//!   case an abiotic proxy cannot express. The biomass source, a generated species whose decomposer role is
//!   read off its diet exactly as a carnivore is read from its prey, is deferred to the biosphere-wiring
//!   slice; until then the stock is hand-seeded (a test) or filled by worldgen, the honest opt-in seam.
//!
//! A world uses either kernel or both, so neither is an engine universal: a life-driven world arms a
//! Life row and its sterile cells preserve; a conditions-driven world arms a Conditions row (the sanctioned
//! abiotic PROXY for a world that models no decomposer species, where a warm, wet, airy cell decays because
//! unmodelled microbes are assumed present, and only a dry or airless cell preserves); a world that arms
//! neither (the default) sees no decomposer registry at all, its matter cycle runs unchanged if armed, and
//! the run is byte-identical. When a world arms BOTH, the [`CombineMode`] is its explicit choice: the default
//! [`CombineMode::All`] gates the drivers against each other, so a Life row makes a sterile cell preserve
//! even beside a favorable Conditions row (decomposer life gated by conditions), while [`CombineMode::Any`]
//! makes them independent alternatives (biotic OR abiotic decay). The physics barrier gate stays in the
//! caller and is never touched here, so falsifiability-by-physics survives: only the rate ABOVE the gate is
//! modulated.
//!
//! This module is off the canonical run path: no constructor arms a decomposer registry, so the
//! canonical `state_hash` is unchanged whether or not this substrate exists.

use std::collections::BTreeMap;

use civsim_core::{Fixed, StateHasher};
use civsim_world::Coord3;

/// How a registry with MORE THAN ONE armed driver row combines their per-cell contributions into a single
/// activity. This is DATA a world sets (Principle 11), not an engine-hardcoded rule, because whether biotic
/// and abiotic decomposition are alternatives or a joint requirement is a modelling choice a world makes,
/// and hardcoding either would author an outcome one level down. A single-row registry reads the same under
/// both modes (the fold reduces to that one row).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombineMode {
    /// A cell decomposes only where ALL armed drivers permit: the activity is the MINIMUM over the rows, the
    /// same worst-limiting-axis discipline the Conditions kernel uses within itself. This is the
    /// emergence-preserving default: with a Life row armed, a sterile cell (a zero Life contribution) does
    /// not decay even under favorable conditions, so "decomposition needs decomposer life" holds even when a
    /// Conditions row is armed beside it, and a colonised but bone-dry cell does not decay either
    /// (decomposer life gated by the conditions it needs to work).
    All,
    /// A cell decomposes where ANY armed driver drives it: the activity is the MAXIMUM over the rows, so
    /// biotic decay (a Life row) and abiotic decay (a Conditions proxy) are alternatives, either sufficient.
    /// A world that means "matter rots from decomposer life OR from favorable chemistry alone" arms this
    /// explicitly; under it a sterile cell can decay through the Conditions row, the world's stated choice.
    Any,
}

/// The fixed set of DECOMPOSER KERNELS. Each is a general shape for how a per-cell decomposition
/// activity arises; the concrete responses are parameterised by reserved data on the [`DecomposerDriver`]
/// row that binds the kernel. Extending the set is a Rust change (a new kernel needs new inputs and a
/// new response), while a world's CHOICE of kernels and their parameters is data (Principle 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecomposerKernelId {
    /// The abiotic CONDITIONS proxy: a Liebig minimum of saturating responses over the cell's moisture,
    /// oxygen, and warmth above the substance's thermal barrier. A surrogate for the microbial activity a
    /// world without a modelled decomposer species would carry, so a dry, airless, or barely thawed cell
    /// preserves its matter.
    Conditions,
    /// The emergent LIFE driver: a saturating response on the per-cell standing decomposer biomass. Zero
    /// at sterility, so decay is driven by the presence of decomposer life, not by favorable conditions
    /// alone.
    Life,
}

/// A CONDITION SOURCE (Arc 5 T6): the kind of environmental quantity a Conditions axis reads. This is the
/// fixed-vocabulary-plus-data-binding boundary the abiotic [`crate::environ::AbioticField`] draws: the engine
/// knows how to READ each source (the caller supplies its per-cell value), while WHICH sources gate a given
/// substance's decomposition, and their reference scales, are the world's data. Extending the set is a Rust
/// change (a new physical quantity needs a new reader), the same bounded cost `AbioticField` has; a world's
/// CHOICE of which axes gate its decomposition is data, so an anaerobic world drops the respirable axis, a
/// world whose decay needs a solvent other than water declares that axis, all without a new kernel variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionSource {
    /// The cell's moisture (a wetness fraction the environ field carries).
    Moisture,
    /// The cell's respirable-medium concentration (the oxygen surrogate the medium substrate carries).
    Respirable,
    /// The cell's warmth ABOVE the substance's own thermal barrier (a derived quantity,
    /// [`warmth_above_barrier`]), so the axis is zero at the freeze gate and rises over the reference span.
    WarmthAboveBarrier,
}

/// One CONDITIONS axis (Arc 5 T6): a source and the reference scale at which it stops limiting. A Conditions
/// row is a Liebig conjunction of these, so a world builds the axis set its own decomposition chemistry needs
/// rather than the hardcoded Earth-aerobic moisture-oxygen-warmth triad.
#[derive(Clone, Debug)]
pub struct ConditionAxis {
    pub source: ConditionSource,
    /// The value at or above which this axis no longer limits decomposition (RESERVED, its basis on the row
    /// constructor). A saturating response rises from zero to full as the source reaches this reference.
    pub reference: Fixed,
}

/// A DECOMPOSER-DRIVER row: a data binding of one [`DecomposerKernelId`] to its reserved parameters,
/// keyed by name (the accepted [`crate::trace::TransformKind`] shape). A registry is a sequence of these
/// rows, and a cell's decomposition activity is the clamped saturating sum of their contributions.
#[derive(Clone, Debug)]
pub struct DecomposerDriver {
    /// The kernel this row's contribution dispatches to.
    pub kernel: DecomposerKernelId,
    /// The CONDITIONS axes this row folds (Arc 5 T6), used only by the Conditions kernel; empty for a Life
    /// row. A world declares its own axis set here, so decomposition's environmental gates are data, not the
    /// hardcoded moisture-oxygen-warmth triad.
    pub axes: Vec<ConditionAxis>,
    /// The kernel's reserved parameters, keyed by name. An absent parameter reads zero (the substrate
    /// absence convention). The Conditions kernel reads `moisture_saturation` (the moisture at or above
    /// which moisture no longer limits, RESERVED, basis the field-capacity moisture of soil-decomposition
    /// kinetics), `oxygen_reference` (the respirable content at or above which decomposition is fully
    /// aerobic, RESERVED, basis the open-air respirable concentration the medium substrate treats as
    /// full), and `warmth_span` (the temperature span above the substance's own barrier over which
    /// activity rises from zero to full, RESERVED, basis a linear surrogate for the Q10 temperature
    /// sensitivity of microbial respiration). The Life kernel reads `biomass_reference` (the standing
    /// decomposer biomass at which decomposition proceeds at the substance's full susceptibility rate,
    /// RESERVED, basis the reference decomposer standing crop of decomposition ecology).
    pub params: BTreeMap<String, Fixed>,
}

impl DecomposerDriver {
    /// A parameter by name; an absent one reads zero (the substrate absence convention).
    pub fn param(&self, name: &str) -> Fixed {
        self.params.get(name).copied().unwrap_or(Fixed::ZERO)
    }

    /// The Earth-aerobic CONDITIONS driver (Arc 5 T6): the moisture saturation, the oxygen reference, and the
    /// warmth span, as the three-axis Terran fixture. A world with a different decomposition chemistry builds
    /// its own axis set with [`Self::conditions_from`]; this convenience keeps the common triad terse and
    /// byte-identical to the pre-T6 hardcoded kernel.
    pub fn conditions(
        moisture_saturation: Fixed,
        oxygen_reference: Fixed,
        warmth_span: Fixed,
    ) -> DecomposerDriver {
        DecomposerDriver::conditions_from(vec![
            ConditionAxis {
                source: ConditionSource::Moisture,
                reference: moisture_saturation,
            },
            ConditionAxis {
                source: ConditionSource::Respirable,
                reference: oxygen_reference,
            },
            ConditionAxis {
                source: ConditionSource::WarmthAboveBarrier,
                reference: warmth_span,
            },
        ])
    }

    /// A CONDITIONS driver over a world-declared axis set (Arc 5 T6): the row's activity is the Liebig minimum
    /// of the axes' saturating responses. An anaerobic world drops the respirable axis; a world whose decay
    /// needs a solvent declares that axis; a world with no conditions gate at all passes an empty set (which
    /// never limits, the always-active row).
    pub fn conditions_from(axes: Vec<ConditionAxis>) -> DecomposerDriver {
        DecomposerDriver {
            kernel: DecomposerKernelId::Conditions,
            axes,
            params: BTreeMap::new(),
        }
    }

    /// The emergent LIFE driver: the reference decomposer biomass at which activity saturates to full.
    pub fn life(biomass_reference: Fixed) -> DecomposerDriver {
        DecomposerDriver {
            kernel: DecomposerKernelId::Life,
            axes: Vec::new(),
            params: BTreeMap::from([("biomass_reference".to_string(), biomass_reference)]),
        }
    }

    /// This row's per-cell contribution to the decomposition activity, in `[0, 1]`. A Conditions row folds its
    /// world-declared axes against the cell's `profile` (a source-to-value slice the caller builds); a Life
    /// row reads the cell's standing decomposer biomass. The unused inputs for a given kernel are ignored, so
    /// the caller passes all of them once.
    fn contribution(&self, profile: &[(ConditionSource, Fixed)], life_stock: Fixed) -> Fixed {
        match self.kernel {
            DecomposerKernelId::Conditions => {
                // The Liebig minimum of the axes' saturating responses: the worst-limiting axis sets the
                // activity, the order-independent form the biosphere's niche suitability and the floor's
                // net_nutrition already use. An axis whose source the profile does not carry is NON-LIMITING
                // (skipped): a world that supplies no value for an axis makes no claim it limits, the same
                // open-air convention the runner's `unwrap_or(ONE)` already applied. An empty axis set never
                // limits (activity full), the always-active conditions row.
                let mut activity = Fixed::ONE;
                for axis in &self.axes {
                    if let Some((_, value)) = profile.iter().find(|(s, _)| *s == axis.source) {
                        activity = activity.min(saturating_response(*value, axis.reference));
                    }
                }
                activity
            }
            DecomposerKernelId::Life => {
                // Zero at sterility (no decomposer biomass), rising to full at the reference standing crop.
                saturating_response(life_stock, self.param("biomass_reference"))
            }
        }
    }
}

/// The warmth of a cell ABOVE a substance's own thermal barrier (Arc 5 T6, extracted from the Conditions
/// kernel so a caller builds the [`ConditionSource::WarmthAboveBarrier`] profile value once): zero at or
/// below the barrier (the caller's freeze gate), rising with the excess. A fixed Rust physics derivation,
/// not data. The difference is a SATURATING subtraction over the raw bits (the same discipline
/// `organic_salience` uses for its negation), so an extreme temperature-minus-barrier cannot overflow i64
/// and panic under the release profile's overflow checks; a non-positive excess reads zero.
pub fn warmth_above_barrier(temperature: Fixed, barrier: Fixed) -> Fixed {
    Fixed::from_bits(temperature.to_bits().saturating_sub(barrier.to_bits())).max(Fixed::ZERO)
}

/// A saturating response in `[0, 1]`: zero at a zero value, rising linearly to full as the value reaches
/// the reference, and flat at full above it. A non-positive reference is a knife-edge (full if any value
/// is present, else zero), matching the zero-breadth case of the biosphere's niche response. The single
/// divide has a numerator clamped non-negative and, where it exceeds the reference, is capped at full, so
/// no product precedes the clamp and the value stays in the unit interval.
fn saturating_response(value: Fixed, reference: Fixed) -> Fixed {
    let value = value.max(Fixed::ZERO);
    if reference <= Fixed::ZERO {
        if value > Fixed::ZERO {
            Fixed::ONE
        } else {
            Fixed::ZERO
        }
    } else {
        value
            .checked_div(reference)
            .unwrap_or(Fixed::ONE)
            .min(Fixed::ONE)
    }
}

/// The DECOMPOSER-DRIVER REGISTRY: the data-defined set of driver rows a world arms, and the [`CombineMode`]
/// by which their contributions combine when more than one is armed. Its one entry point,
/// [`Self::activity_at`], returns a cell's decomposition activity in `[0, 1]`. Empty by default; a runner
/// with no registry armed passes a full activity of one to the matter cycle (today's unconditional rate),
/// and an armed-but-empty registry passes zero (a decomposition gate with no driver present preserves
/// everything), so arming the registry is the deliberate flip from the authored universal to the emergent
/// driver.
#[derive(Clone, Debug)]
pub struct DecomposerDriverRegistry {
    rows: Vec<DecomposerDriver>,
    combine: CombineMode,
}

impl Default for DecomposerDriverRegistry {
    fn default() -> DecomposerDriverRegistry {
        DecomposerDriverRegistry::new()
    }
}

impl DecomposerDriverRegistry {
    /// An empty registry combining by the emergence-preserving [`CombineMode::All`] default: no driver rows,
    /// so [`Self::activity_at`] is zero everywhere until rows are armed.
    pub fn new() -> DecomposerDriverRegistry {
        DecomposerDriverRegistry {
            rows: Vec::new(),
            combine: CombineMode::All,
        }
    }

    /// Set the combine mode (data): [`CombineMode::All`] (the default, drivers gate each other) or
    /// [`CombineMode::Any`] (drivers are alternatives). Consumes and returns self for terse construction.
    pub fn with_combine(mut self, combine: CombineMode) -> DecomposerDriverRegistry {
        self.combine = combine;
        self
    }

    /// The registry's combine mode.
    pub fn combine(&self) -> CombineMode {
        self.combine
    }

    /// Add a driver row.
    pub fn push(&mut self, driver: DecomposerDriver) {
        self.rows.push(driver);
    }

    /// The number of driver rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether no driver row is armed.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// A cell's decomposition ACTIVITY in `[0, 1]`: each row's contribution combined by the registry's
    /// [`CombineMode`]. Under [`CombineMode::All`] the activity is the MINIMUM over the rows (a cell decays
    /// only where every armed driver permits, so a Life row's zero at a sterile cell forces the activity to
    /// zero however favorable the conditions); under [`CombineMode::Any`] it is the MAXIMUM (any driver is
    /// sufficient). An empty registry reads zero (no driver present preserves everything). Both min and max
    /// over per-row contributions each already in `[0, 1]` are order-independent and stay in `[0, 1]`, so the
    /// result is deterministic regardless of row order; the final clamp is a defensive guard. The caller
    /// multiplies the substance's rate by this before the volume breaks down, so a factor of one reproduces
    /// the unconditional rate and a factor of zero preserves the matter.
    pub fn activity_at(&self, profile: &[(ConditionSource, Fixed)], life_stock: Fixed) -> Fixed {
        let mut contributions = self
            .rows
            .iter()
            .map(|row| row.contribution(profile, life_stock));
        let combined = match contributions.next() {
            None => return Fixed::ZERO,
            Some(first) => match self.combine {
                CombineMode::All => contributions.fold(first, |acc, c| acc.min(c)),
                CombineMode::Any => contributions.fold(first, |acc, c| acc.max(c)),
            },
        };
        combined.clamp(Fixed::ZERO, Fixed::ONE)
    }

    /// A labelled NON-CANONICAL dev fixture: a registry arming both a Conditions row and a Life row with
    /// stand-in parameters, combined by the [`CombineMode::All`] default, so a sterile cell does not decay
    /// even though the conditions are favorable (the Life row gates it). Not a set of decided values; the
    /// reserved parameters are the owner's to calibrate (basis on each row constructor and in
    /// `calibration/reserved.toml`).
    pub fn dev_fixture() -> DecomposerDriverRegistry {
        let mut reg = DecomposerDriverRegistry::new();
        // Moisture saturates at a quarter of the field's full moisture, oxygen at the open-air full
        // concentration (one), warmth over a ten-degree span above the barrier: illustrative, not decided.
        reg.push(DecomposerDriver::conditions(
            Fixed::from_ratio(1, 4),
            Fixed::ONE,
            Fixed::from_int(10),
        ));
        // A reference standing decomposer crop of one biomass unit per cell: illustrative, not decided.
        reg.push(DecomposerDriver::life(Fixed::ONE));
        reg
    }
}

/// The per-cell STANDING DECOMPOSER BIOMASS field: how much decomposer life stands in each cell, per
/// substance it decomposes. The Life kernel reads it; a world's ecology deposits into it. Structured
/// exactly like [`crate::material::SoilNutrientField`], a [`Coord3`]-keyed map of per-substance amounts
/// with the empty-default-folds-nothing discipline, so an unarmed or unseeded field folds no bytes into
/// `state_hash` and a scenario with no decomposer life is byte-identical.
#[derive(Clone, Debug, Default)]
pub struct DecomposerStockField {
    /// The standing decomposer biomass at each cell, keyed by [`Coord3`] then by the substance id the
    /// biomass decomposes. A cell or substance not present holds no biomass (the absence convention).
    cells: BTreeMap<Coord3, BTreeMap<String, Fixed>>,
}

impl DecomposerStockField {
    /// A sterile field: no cell holds any decomposer biomass.
    pub fn new() -> DecomposerStockField {
        DecomposerStockField::default()
    }

    /// Whether no decomposer biomass stands anywhere (the opt-out state a scenario with no decomposer life
    /// stays in, so its `state_hash` fold folds nothing and it replays bit-for-bit).
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Set the standing decomposer biomass for a substance at a cell, replacing what was there (a standing
    /// crop, not an accumulation). A non-positive biomass clears the entry, so an emptied cell stays empty
    /// and folds nothing.
    pub fn seed(&mut self, cell: Coord3, substance: &str, biomass: Fixed) {
        if biomass <= Fixed::ZERO {
            if let Some(m) = self.cells.get_mut(&cell) {
                m.remove(substance);
                if m.is_empty() {
                    self.cells.remove(&cell);
                }
            }
            return;
        }
        self.cells
            .entry(cell)
            .or_default()
            .insert(substance.to_string(), biomass);
    }

    /// The standing decomposer biomass for a substance at a cell; a sterile cell or an absent substance
    /// reads zero.
    pub fn mass(&self, cell: Coord3, substance: &str) -> Fixed {
        self.cells
            .get(&cell)
            .and_then(|m| m.get(substance))
            .copied()
            .unwrap_or(Fixed::ZERO)
    }

    /// Fold the stock field into a hash in canonical (cell, substance, biomass) order, beside
    /// [`crate::material::SoilNutrientField::hash_into`]. An empty field folds nothing, so an opted-out
    /// run is unchanged. The `BTreeMap`s walk in canonical key order, so the fold is reproducible and
    /// thread-invariant.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (cell, substances) in &self.cells {
            for (substance, biomass) in substances {
                h.write_i64(cell.x as i64);
                h.write_i64(cell.y as i64);
                h.write_i64(cell.z as i64);
                for b in substance.as_bytes() {
                    h.write_u32(*b as u32);
                }
                h.write_fixed(*biomass);
            }
        }
    }

    /// A labelled NON-CANONICAL dev fixture: a field seeding a unit of decomposer biomass for one
    /// substance at one cell, for tests and examples.
    pub fn dev_fixture(cell: Coord3, substance: &str) -> DecomposerStockField {
        let mut field = DecomposerStockField::new();
        field.seed(cell, substance, Fixed::ONE);
        field
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A warm, wet, and oxygen-rich cell well above the barrier, the favorable case every kernel test
    // varies one axis away from.
    fn favorable() -> (Fixed, Fixed, Fixed, Fixed) {
        // temperature, barrier, moisture, oxygen
        (
            Fixed::from_int(300),
            Fixed::from_int(273),
            Fixed::ONE,
            Fixed::ONE,
        )
    }

    // The Earth-triad condition profile a test cell presents (Arc 5 T6): the moisture, respirable, and
    // warmth-above-barrier the runner supplies, so the tests exercise the same axes the run does.
    fn profile(
        temperature: Fixed,
        barrier: Fixed,
        moisture: Fixed,
        oxygen: Fixed,
    ) -> Vec<(ConditionSource, Fixed)> {
        vec![
            (ConditionSource::Moisture, moisture),
            (ConditionSource::Respirable, oxygen),
            (
                ConditionSource::WarmthAboveBarrier,
                warmth_above_barrier(temperature, barrier),
            ),
        ]
    }

    #[test]
    fn an_unarmed_registry_reads_zero_activity() {
        // An armed-but-empty registry has no driver, so it preserves everything (activity zero). The
        // runner reads a full activity of one only from the ABSENCE of a registry, not from an empty one.
        let reg = DecomposerDriverRegistry::new();
        let (t, b, m, o) = favorable();
        assert_eq!(
            reg.activity_at(&profile(t, b, m, o), Fixed::ONE),
            Fixed::ZERO
        );
    }

    #[test]
    fn the_life_kernel_is_zero_at_sterility_and_rises_with_biomass() {
        // The emergent falsifier in miniature: a Life-only registry decays nothing where no decomposer
        // biomass stands, and decays where it does, so decay is driven by life not by favorable conditions.
        let mut reg = DecomposerDriverRegistry::new();
        reg.push(DecomposerDriver::life(Fixed::from_int(2)));
        let (t, b, m, o) = favorable();
        let sterile = reg.activity_at(&profile(t, b, m, o), Fixed::ZERO);
        let colonised = reg.activity_at(&profile(t, b, m, o), Fixed::ONE);
        let full = reg.activity_at(&profile(t, b, m, o), Fixed::from_int(2));
        assert_eq!(
            sterile,
            Fixed::ZERO,
            "a sterile cell does not decay under the life kernel however warm, wet, and airy"
        );
        assert!(
            colonised > sterile,
            "standing decomposer biomass raises the activity"
        );
        assert_eq!(
            full,
            Fixed::ONE,
            "at the reference standing crop the activity saturates to the full susceptibility rate"
        );
    }

    #[test]
    fn the_conditions_kernel_is_limited_by_its_worst_axis() {
        // The Liebig minimum: a dry cell, an airless cell, and a barely thawed cell each preserve their
        // matter though the other two axes are favorable, and only the all-favorable cell decays.
        let mut reg = DecomposerDriverRegistry::new();
        reg.push(DecomposerDriver::conditions(
            Fixed::from_ratio(1, 2),
            Fixed::ONE,
            Fixed::from_int(10),
        ));
        let (t, b, m, o) = favorable();
        assert!(
            reg.activity_at(&profile(t, b, m, o), Fixed::ZERO) > Fixed::ZERO,
            "a warm, wet, oxygenated cell decays"
        );
        assert_eq!(
            reg.activity_at(&profile(t, b, Fixed::ZERO, o), Fixed::ZERO),
            Fixed::ZERO,
            "a bone-dry cell does not decay"
        );
        assert_eq!(
            reg.activity_at(&profile(t, b, m, Fixed::ZERO), Fixed::ZERO),
            Fixed::ZERO,
            "an airless cell does not decay"
        );
        assert_eq!(
            reg.activity_at(&profile(b, b, m, o), Fixed::ZERO),
            Fixed::ZERO,
            "a cell exactly at its thawing barrier does not decay"
        );
    }

    #[test]
    fn a_world_that_declares_its_own_condition_axes_is_not_bound_to_the_earth_triad() {
        // Arc 5 T6: the Conditions axis set is world data, not the hardcoded moisture-oxygen-warmth triad, so
        // an ANAEROBIC world (whose decomposition does not need oxygen) drops the respirable axis and its
        // airless cells decay, where the Earth-triad kernel would gate them to zero. Proof: the same airless
        // cell (oxygen zero) reads zero under the Earth triad but full-limited-only-by-moisture-and-warmth
        // under a world that declares just those two axes. No new kernel, only a different data axis set.
        let (t, b, m, _o) = favorable();
        let airless = profile(t, b, m, Fixed::ZERO); // wet and warm, but no oxygen

        let mut terran = DecomposerDriverRegistry::new();
        terran.push(DecomposerDriver::conditions(
            Fixed::from_ratio(1, 2),
            Fixed::ONE,
            Fixed::from_int(10),
        ));
        assert_eq!(
            terran.activity_at(&airless, Fixed::ZERO),
            Fixed::ZERO,
            "under the Earth triad an airless cell does not decay (the respirable axis gates it)"
        );

        let mut anaerobic = DecomposerDriverRegistry::new();
        anaerobic.push(DecomposerDriver::conditions_from(vec![
            ConditionAxis {
                source: ConditionSource::Moisture,
                reference: Fixed::from_ratio(1, 2),
            },
            ConditionAxis {
                source: ConditionSource::WarmthAboveBarrier,
                reference: Fixed::from_int(10),
            },
        ]));
        assert!(
            anaerobic.activity_at(&airless, Fixed::ZERO) > Fixed::ZERO,
            "a world that declares no respirable axis decays an airless but wet, warm cell: oxygen is not an \
             engine-universal gate, it is one axis a world may or may not declare"
        );
    }

    #[test]
    fn the_all_combine_gates_a_sterile_cell_even_under_favorable_conditions() {
        // The emergence-preserving default (CombineMode::All): a registry arming BOTH a Conditions row and a
        // Life row takes the MINIMUM, so a warm, wet, airy but STERILE cell (zero decomposer biomass) does
        // NOT decay, because the Life row's zero gates the favorable Conditions row. This is the seam a
        // blind audit caught: an additive combine let the Conditions row swallow the Life signal.
        let mut reg = DecomposerDriverRegistry::new();
        reg.push(DecomposerDriver::conditions(
            Fixed::from_ratio(1, 2),
            Fixed::ONE,
            Fixed::from_int(10),
        ));
        reg.push(DecomposerDriver::life(Fixed::ONE));
        assert_eq!(
            reg.combine(),
            CombineMode::All,
            "the default gates the drivers"
        );
        let (t, b, m, o) = favorable();
        assert_eq!(
            reg.activity_at(&profile(t, b, m, o), Fixed::ZERO),
            Fixed::ZERO,
            "a sterile cell does not decay under All even with a favorable Conditions row (Life gates it)"
        );
        assert_eq!(
            reg.activity_at(&profile(t, b, m, o), Fixed::ONE),
            Fixed::ONE,
            "a fully colonised, favorable cell decays at the full rate (both drivers permit)"
        );
    }

    #[test]
    fn the_any_combine_lets_either_driver_drive_decay() {
        // The opt-in alternative (CombineMode::Any): the MAXIMUM, so biotic and abiotic decay are
        // independent sufficient drivers. A sterile but favorable cell decays through the Conditions row,
        // the world's explicit choice; the activity never exceeds full.
        let reg = DecomposerDriverRegistry::new().with_combine(CombineMode::Any);
        let mut reg = reg;
        reg.push(DecomposerDriver::conditions(
            Fixed::from_ratio(1, 2),
            Fixed::ONE,
            Fixed::from_int(10),
        ));
        reg.push(DecomposerDriver::life(Fixed::ONE));
        let (t, b, m, o) = favorable();
        assert_eq!(
            reg.activity_at(&profile(t, b, m, o), Fixed::ZERO),
            Fixed::ONE,
            "under Any a favorable Conditions row drives decay even at a sterile cell (the explicit OR choice)"
        );
        assert_eq!(
            reg.activity_at(&profile(t, b, m, o), Fixed::from_int(5)),
            Fixed::ONE,
            "the combined activity never exceeds full"
        );
    }

    #[test]
    fn the_stock_field_is_empty_by_default_and_reads_zero() {
        let field = DecomposerStockField::new();
        assert!(field.is_empty());
        assert_eq!(
            field.mass(Coord3::new(0, 0, 0), "carrion"),
            Fixed::ZERO,
            "a sterile field reads zero biomass"
        );
    }

    #[test]
    fn seeding_and_clearing_the_stock_field() {
        let mut field = DecomposerStockField::new();
        let cell = Coord3::new(1, 2, 0);
        field.seed(cell, "carrion", Fixed::from_int(3));
        assert_eq!(field.mass(cell, "carrion"), Fixed::from_int(3));
        assert!(!field.is_empty());
        // A standing crop replaces rather than accumulates.
        field.seed(cell, "carrion", Fixed::from_int(1));
        assert_eq!(field.mass(cell, "carrion"), Fixed::ONE);
        // A non-positive biomass clears the entry, and the emptied field folds nothing.
        field.seed(cell, "carrion", Fixed::ZERO);
        assert_eq!(field.mass(cell, "carrion"), Fixed::ZERO);
        assert!(
            field.is_empty(),
            "clearing the last entry empties the field"
        );
    }
}
