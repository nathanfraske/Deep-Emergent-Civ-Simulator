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

//! Emergent locomotion driven by an evolved controller (design Part 8, Part 9, Part 13, Part 20,
//! Part 25; R-BEHAVIOR-EVOLVE; Principles 8, 9, 10).
//!
//! What is authored here is physics, and only physics. A body's capacity to move is its morphology
//! ([`crate::anatomy::BodyPlan`]): a body with no locomotion organ is rooted and never moves,
//! whatever its kingdom, so a rooted tree stays put while a body that bears the organ moves, even an
//! autotroph, so a walking tree walks. Whether a body has that organ is itself an emergent
//! morphological outcome, not a rule keyed on being a plant. Its ground speed comes from its size,
//! its activity, and the terrain (passability and cost, read through [`Terrain`]). Its needs are the
//! homeostatic reserves that drain by metabolism ([`crate::homeostasis`]); its options are the
//! affordances its morphology permits. All of that is physics.
//!
//! What is not authored is the behaviour: which affordance the being issues, and where it aims it.
//! That is the evolved controller ([`crate::controller`]), a heritable mapping from the being's
//! reserves and percept to an affordance, expressed from its genome and (under the pre-dawn epoch)
//! selected by whether it keeps the body alive. Nobody writes "seek water when dry": each tick the
//! being perceives the sources within its sensory range and remembers them ([`Walker::known`]),
//! reads its own reserves, and its controller decides. A being that has evolved the adaptive coupling
//! walks up the gradient to a known source and ingests it; one that has not starves. This is the
//! retirement of the authored decision menu that the prior slice flagged: the drives-and-actions
//! policy is gone from this path, replaced by the expressed controller (the [`crate::decision`]
//! utility layer remains the shape of the sentient, deliberative tier above, which the controller
//! underlies rather than replaces).
//!
//! Non-omniscience stands: a being knows only what it has perceived (a small true sensory range) or
//! remembered, so it cannot head for a source it has never seen; when its controller wants to move
//! but has no known gradient to follow, it explores, a heading drawn from counter-based RNG keyed on
//! the being and the tick ([`civsim_core::Phase::EXPLORE`]), discovering the world by moving through
//! it. The mechanism is fixed Rust and fully deterministic: beings are walked in stable-id order,
//! position is exact fixed-point (a subtile fractional coordinate), the controller evaluation and the
//! metabolism draw no randomness, and every choice keys on the seed, the being, and the tick, never
//! on the camera (Principle 10). What the movement physics needs is reserved with its basis in
//! [`LocomotionParams`] and defaulted only by a labelled development fixture.
//!
//! Honest limits. Perception is a range gate, not yet line of sight or the full belief store of Part
//! 9, and knowledge is never forgotten or shared. Movement is straight-line with a passability gate
//! rather than routing around an obstacle (the pathfinding of Part 13). The reaction-norm controller
//! cannot gate a response on internal state through a product (it moves toward a known source
//! whenever away from it, whatever the reserve, and ingests underfoot when the reserve is low); the
//! recurrent controller lifts that ceiling (both are [`crate::controller`]). Intake is measured, not
//! authored: a being ingests the matter underfoot by reading the tile's [`Composition`] on each
//! homeostatic axis's backing class through the resolved edibility floor's satisfaction measure
//! (R-PHYS-BIO, [`crate::edibility`], `civsim_physics::laws::satisfaction`) against its own
//! [`Physiology`], so the deposit derives from the tile's composition and the being's physiology
//! rather than a reserved fraction. Toxin classes are read but not yet applied to a reserve here (no
//! harm sink at the mass-only Walker tier); the harm coupling is the named follow-on.

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::{DrawKey, Fixed, Phase, StableId, StateHasher};
use civsim_physics::{laws, DepletionCharacter};
use civsim_world::Coord3;

use civsim_compose::{derive_capabilities, CapabilityCaps, CapabilityRefs, FunctionLawRegistry};

use crate::anatomy::{BodyPlan, BodyPlanRegistry};
use crate::controller::{Controller, ControllerLayout};
use crate::conviction_experience::ConvictionExperience;
use crate::edibility::{Composition, FloorCaps, Physiology};
use crate::homeostasis::{
    AffordanceId, AffordanceRegistry, DerivedDrain, DrawTerm, Homeostasis, HomeostaticAxisId,
    HomeostaticRegistry, ReserveMemory, CONDITION, CRAFT, DIG, EXTRACT, GEOPHAGE, GRASP, INGEST,
    MOVE, RELEASE, SHELTER, STRIKE,
};
use crate::learn::{EligibilityTrace, SequenceStep};
use crate::material::{MaterialField, SubstanceMix, WieldedTool};
use crate::material_percept::MaterialPerceptRegistry;
use crate::morphogen::Structure;
use crate::percept::PerceptRegistry;

/// The reserved parameters of the movement physics. The mechanism that reads them is fixed; these
/// numbers are the owner's to set, surfaced with a basis, never fabricated (Principle 11). The
/// development fixture below lets the module run and be tested now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LocomotionParams {
    /// Tiles per tick a maximal, fully active body crosses on flat, open ground. RESERVED. Basis:
    /// a real walking speed (about 1.4 m/s) divided by the tile edge in metres, at the one-second
    /// base tick, so a person crosses roughly one tile per second on open ground.
    pub base_speed: Fixed,
    /// How much terrain cost above the open-ground baseline slows movement (speed is divided by
    /// `1 + terrain_penalty * (cost - 1)`). RESERVED. Basis: the slowdown of real difficult ground
    /// (broken, steep, or wet terrain) relative to open ground.
    pub terrain_penalty: Fixed,
    /// How much a carried load slows movement (material-substrate arc, cascade item 3): the ground speed
    /// is divided by `1 + load_penalty * carried_weight / carry_capacity`, so a being carrying a load at
    /// its whole-body muscle-force limit moves at `1 / (1 + load_penalty)` of its unladen speed, and an
    /// unladen being is unaffected (the divisor is one). RESERVED. Basis: the fractional slowdown real
    /// load carriage incurs at the limit of what a body can bear (the load-carriage and march-speed
    /// literature on speed versus the carried fraction of a body's capacity); a labelled dev fixture
    /// through [`Self::dev_default`] until set.
    pub load_penalty: Fixed,
    /// The lowest activity factor, so even a sluggish body creeps rather than freezing (the
    /// temperament activity axis scales speed between this floor and one). RESERVED. Basis: the
    /// ratio of a slow gait to a brisk one.
    pub activity_floor: Fixed,
    /// How far, in whole tiles, a being perceives a source: the true sensory range within which it
    /// comes to know of a resource, so knowledge is earned by nearness, not read from the map.
    /// RESERVED. Basis: the perception range the being's sensory morphology and acuity imply
    /// (Part 9); small, a handful of tiles, not the whole world.
    pub sense_range: i64,
    /// How many ticks an exploration heading holds before it is redrawn, so a searching being keeps
    /// a direction rather than jittering in place. RESERVED. Basis: the persistence of a real
    /// search path before it turns.
    pub explore_persistence: u64,
    /// The trophic ingest efficiency: the fraction of the biomass a bite removes from a tile's standing
    /// stock that becomes the being's reserve, the rest lost to handling and respiration (the
    /// [`crate::stocks::flow`] transfer efficiency; base-level liveliness step 3). A being's reserve
    /// gain is conservation-honest against the tile's loss: the tile loses the gross bite, the being
    /// gains that times this efficiency. RESERVED. Basis: the ecological trophic transfer efficiency of
    /// a bite becoming consumer tissue (the Lindeman ~10 percent figure, or the fraction the owner sets
    /// for the world's producers); it sets how hard grazing depletes the food relative to the reserve
    /// gained, so it, the regrowth rate, and the metabolic drain together fix the carrying capacity.
    /// Its manifest home is `locomotion.ingest_efficiency` once the locomotion parameters read fail-loud
    /// from the manifest; the dev harness stands up a labelled fixture through [`Self::dev_default`].
    pub ingest_efficiency: Fixed,
    /// R-UNITS-PIN (the reserve joule-scale reconciliation): the physical energy content a unit of standing
    /// food carries, bridging the abstract climate-productivity supply to the reserve's physical joules the
    /// Kleiber drain is paid in, so the forage INGEST fills a reserve by the food's PHYSICAL content
    /// ([`crate::physiology::physical_intake`]) rather than a saturating fill. RESERVED, dev-set: the
    /// absolute scale is the owner's units anchor, and a real producer's own `bio.energy_density` supersedes
    /// it per cell once the standing food carries its composition (T3). Basis: set so a being's forage intake
    /// at a plausible standing density offsets its Kleiber drain (the calibration target: the dev world
    /// survives). Alien-clean: it scales the food's content, never assuming a chemistry (a mana or redox food
    /// carries its own content the same way).
    pub food_energy_density: Fixed,
    /// The floor caps the environmental-harm sink reads (base-level liveliness step 4): the per-class and
    /// aggregate harm ceilings the dose-response ([`civsim_physics::laws::net_harm`]) clamps to. RESERVED
    /// (their home is [`crate::edibility::FloorCaps`], the floor's reserved harm caps); the dev harness
    /// stands up a labelled fixture. A being with no toxin tolerance takes no harm regardless of the caps,
    /// so this is inert until a physiology carries a tolerance.
    pub harm_caps: FloorCaps,
    /// The condition recovery rate (base-level liveliness step 4): the fraction of the CONDITION reserve
    /// a being heals per tick, so environmental harm is a race between damage and healing. A being whose
    /// per-tick harm is below this heals faster than it is worn (it lives on the gradient); one whose harm
    /// is above it declines to death. This is what makes a salt flat livable to a heritable halophile and
    /// lethal to a naive lineage, and lets a being that leaves a toxic cell recover. RESERVED. Basis: the
    /// physiological repair rate of the condition reserve; its manifest home is
    /// `physiology.condition_recovery` once the locomotion parameters read the manifest, a labelled dev
    /// fixture until then.
    pub condition_recovery: Fixed,
    /// The reserved-with-basis reference levels the LOCOMOTE function law measures a limb against
    /// (emergent-anatomy step one), so the movement physics reads a body's mobility from its grown limb
    /// physics rather than a mode-id proxy. RESERVED at their `capability.*` manifest homes (see
    /// [`civsim_compose::CapabilityRefs`]); the dev harness stands up a labelled fixture. Shared with the
    /// affordance derive so the MOVE gate and the ground speed read the same capability.
    pub capability_refs: CapabilityRefs,
    /// The physics saturation ceilings the LOCOMOTE law clamps to, derived from the mechanical floor's own
    /// axis ranges (the same derive-from-floor-range discipline [`civsim_compose::CapabilityCaps::derive`]
    /// uses); a labelled dev fixture until the parameters read the floor registry.
    pub capability_caps: CapabilityCaps,
    /// The reference leg length at which a limb's stride saturates, in metres (`locomotion.reference_leg_length`,
    /// the `mech.arm_length` axis). RESERVED. Basis: the leg length of a maximal-stride body, the arm length
    /// at which the stride factor reaches one; a fraction of it gives a proportionally shorter stride. This
    /// (with the limb's own LOCOMOTE strength) replaces the `sqrt(body_mass)` allometric proxy: the ground
    /// speed now reads the grown limb rather than a mass power law, so a longer, stouter limb moves faster.
    /// A labelled dev fixture until the parameters read the manifest.
    pub reference_leg_length: Fixed,
}

impl LocomotionParams {
    /// A labelled DEVELOPMENT FIXTURE, not owner values, so locomotion runs and can be tested now.
    pub fn dev_default() -> LocomotionParams {
        LocomotionParams {
            base_speed: Fixed::from_ratio(1, 1),
            terrain_penalty: Fixed::from_ratio(1, 1),
            // A being carrying a load at its full muscle-force capacity moves at half speed (divisor 2);
            // a labelled dev fixture, not an owner value.
            load_penalty: Fixed::from_ratio(1, 1),
            activity_floor: Fixed::from_ratio(1, 4),
            sense_range: 4,
            explore_persistence: 6,
            // A labelled fixture (not owner canon): a half-efficient bite, so grazing depletes the food
            // at twice the reserve gained, giving carrying capacity teeth while a lineage can still
            // subsist. A canonical run reads the reserved trophic efficiency.
            ingest_efficiency: Fixed::from_ratio(1, 2),
            // R-UNITS-PIN dev-set (a labelled fixture, not owner canon): the physical energy a unit of
            // standing food carries, the reconciliation of the abstract productivity supply to the reserve's
            // Kleiber joules. Calibrated so a foraging being's intake offsets its drain and the dev world
            // survives; the owner sets the canonical units anchor.
            food_energy_density: Fixed::from_int(3000),
            // The labelled floor harm caps (base-level liveliness step 4); a canonical run reads the
            // reserved FloorCaps.
            harm_caps: FloorCaps::dev_default(),
            // A labelled fixture: the CONDITION reserve heals a quarter per tick, fast enough that a
            // heritable halophile outpaces a salt flat's harm while a naive lineage is worn through.
            condition_recovery: Fixed::from_ratio(1, 4),
            // The labelled LOCOMOTE references and floor ceilings the affordance MOVE gate reads
            // (emergent-anatomy step one); a canonical run reads the reserved capability references and
            // derives the caps from the floor registry. The ceilings are the mechanical floor's own
            // pressure and length range hi (150000 MPa, 100 m), matching the compose crate's derive so a
            // limb reads the same capability in the affordance path as in the individual-tier body path.
            capability_refs: CapabilityRefs::dev_refs(),
            capability_caps: CapabilityCaps {
                pressure: Fixed::from_int(150_000),
                depth: Fixed::from_int(100),
            },
            // A labelled fixture: a half-metre maximal-stride leg, so the dev-fixture walk limb (arm_length
            // 0.3 m) strides at six-tenths of the reference and a longer swim/slither limb saturates it.
            reference_leg_length: Fixed::from_ratio(1, 2),
        }
    }
}

/// The world's terrain, read by the movement physics. The world implements this over its map; the
/// module stays world-agnostic. Passability is body-aware, so a body that can swim crosses water a
/// walker cannot: physics gating a body against the ground, never a scripted route.
pub trait Terrain {
    /// Whether a body may enter this tile. A tile off the map is not passable.
    fn passable(&self, coord: Coord3, body: &BodyPlan) -> bool;

    /// The movement cost multiplier of a tile, at least one on open ground and higher on difficult
    /// ground (slope, mud, undergrowth). A pure physical property of the tile.
    fn cost(&self, coord: Coord3) -> Fixed;
}

/// The matter that really sits on each tile, the world's ground truth: a per-tile [`Composition`]
/// (its supply over the nutrient classes, its dose over the toxin classes), keyed by the biology-
/// floor class ids. Whether a tile is a source of a given homeostatic axis is DERIVED, never stored:
/// it is a source of an axis exactly when its composition carries a nonzero supply on that axis's
/// backing class (`water tiles bear bio.water_fraction`, forage tiles bear `bio.energy_density`),
/// read against a [`HomeostaticRegistry`]. The module never hardcodes which axis a tile restores;
/// the pairing emerges from the composition and the registry's backing classes (Principles 9 and
/// 11). The world builds this from its content and the edibility floor; a being does not get to read
/// it, it can only perceive the tiles near it (see [`step`]).
#[derive(Clone, Debug, Default)]
pub struct ResourceField {
    /// The DENSE ground layer (`z == 0`), row-major `y*w+x`, sized to `w*h` once the grid dimensions are
    /// known ([`set_dims`](Self::set_dims)). `None` is an unoccupied cell (the per-cell occupied flag, so
    /// an unwritten cell folds NOTHING into the hash, exactly as an absent `BTreeMap` key did). The hot
    /// per-cell-per-tick path (regrow, take, supply) indexes this in O(1) with no tree descent, which is
    /// the whole point: `regrow_supply` writes every cell every tick, so the field is dense, and a
    /// `BTreeMap<Coord3>` paid an O(log n) tree walk plus poor locality per cell.
    dense: Vec<Option<Composition>>,
    width: i32,
    height: i32,
    /// The dormant sparse overflow: non-ground cells (`z != 0`), out-of-bounds or negative reads (a
    /// grazer's sense range walks off the grid edge), and the unsized fixture path (a bare `new()` written
    /// before any `set_dims`). Empty on every canonical armed run, so it costs no tree descent there; it
    /// keeps the field total for the nominally 2.5D `Coord3` and for the tests that seed sparse coords.
    overflow: BTreeMap<Coord3, Composition>,
    /// The per-ground-cell REAL-COMPOSITION marker (CORRECTED-T3), keyed PER CLASS: the set of nutrient classes on
    /// this cell whose standing supply is a real producer-composition magnitude (already the physical content at the
    /// plant's own per-substance density), as opposed to the abstract climate-productivity default the forage INGEST
    /// bridges to physical content through the reserved `food_energy_density` anchor. Set each tick by
    /// [`crate::environ::EnvironFields::regrow_supply`] to the cell's producer-composition axes (the `producer_food`
    /// keys), which EXCLUDES the always-written water mirror and the salinity dose, so those non-composition axes keep
    /// the anchor bridge on a producer cell exactly as on a bare cell (the §9 correctness lens caught that a per-cell
    /// bool over-scoped the supersession onto the water axis). INGEST reads it per class: a class in the set is eaten
    /// at `content = supply` directly; any other class (water, or a class the composition does not carry) keeps the
    /// anchor. NOT folded into the state hash (a per-tick derived read of the environ's producer_food, whose effect
    /// enters through the hashed supply values it does not itself change): an all-empty marker (every cell on a run
    /// that seeds no producer food) leaves the INGEST path byte-identical, so the four tracked scenarios hold.
    /// Sparse-safe: an off-grid or unsized read carries no class, so it takes the anchor branch.
    real_composition: Vec<Vec<String>>,
}

impl ResourceField {
    /// An empty, unsized field. Writes fall to the overflow until [`set_dims`](Self::set_dims) sizes the
    /// dense layer.
    pub fn new() -> ResourceField {
        ResourceField::default()
    }

    /// Size the dense ground layer once the grid dimensions are known (from the environ stack, at the top
    /// of `regrow_supply`). Idempotent: re-sizing to the same dimensions is a no-op. Any overflow cell that
    /// now falls in-bounds ground is migrated into the dense layer, so a cell written before sizing is not
    /// stranded (on the canonical path the overflow is empty here, so the migration is a no-op).
    pub fn set_dims(&mut self, width: i32, height: i32) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        let n = (width.max(0) as usize) * (height.max(0) as usize);
        self.dense = (0..n).map(|_| None).collect();
        self.real_composition = vec![Vec::new(); n];
        let migrate: Vec<Coord3> = self
            .overflow
            .keys()
            .filter(|c| self.dense_index(**c).is_some())
            .copied()
            .collect();
        for c in migrate {
            if let (Some(comp), Some(i)) = (self.overflow.remove(&c), self.dense_index(c)) {
                self.dense[i] = Some(comp);
            }
        }
    }

    /// The flat dense index of a ground cell inside the sized grid, or `None` for anything the overflow
    /// owns: a non-ground cell, an out-of-bounds or negative coordinate, or an unsized field. Never wraps.
    #[inline]
    fn dense_index(&self, coord: Coord3) -> Option<usize> {
        if coord.z == 0
            && self.width > 0
            && coord.x >= 0
            && coord.y >= 0
            && coord.x < self.width
            && coord.y < self.height
        {
            Some((coord.y as usize) * (self.width as usize) + (coord.x as usize))
        } else {
            None
        }
    }

    /// Record the matter composition on a tile (overwriting any prior).
    pub fn set(&mut self, coord: Coord3, comp: Composition) {
        match self.dense_index(coord) {
            Some(i) => self.dense[i] = Some(comp),
            None => {
                self.overflow.insert(coord, comp);
            }
        }
    }

    /// A mutable handle to a tile's composition, inserting an empty one if the tile has none. The regrow
    /// write-back updates a cell's food, water, and salinity through this; a dense ground cell is one
    /// indexed slot (no tree descent, no per-cell heap churn), an off-grid cell falls to the overflow.
    pub fn composition_mut(&mut self, coord: Coord3) -> &mut Composition {
        match self.dense_index(coord) {
            Some(i) => self.dense[i].get_or_insert_with(Composition::default),
            None => self.overflow.entry(coord).or_default(),
        }
    }

    /// Set a ground cell's REAL-COMPOSITION class set (CORRECTED-T3): the nutrient classes whose standing supply is
    /// a real producer-composition magnitude on this cell, so the forage INGEST eats each of them at
    /// `content = supply` rather than bridging through the `food_energy_density` anchor. Set each tick by
    /// `regrow_supply` to the cell's `producer_food` axes (empty for a bare cell), which excludes the water mirror and
    /// salinity dose, so those keep the anchor. An off-grid or unsized cell is ignored (its read carries no class).
    pub fn set_real_composition(&mut self, coord: Coord3, classes: impl Iterator<Item = String>) {
        if let Some(i) = self.dense_index(coord) {
            if i < self.real_composition.len() {
                self.real_composition[i].clear();
                self.real_composition[i].extend(classes);
            }
        }
    }

    /// Whether a ground cell's standing supply of `class` is a real producer-composition magnitude (CORRECTED-T3).
    /// `false` for an off-grid, unsized, or unmarked cell, or for a class the cell's composition does not carry (the
    /// water mirror and any non-composition axis), so those keep the anchor bridge.
    pub fn is_real_composition(&self, coord: Coord3, class: &str) -> bool {
        self.dense_index(coord)
            .and_then(|i| self.real_composition.get(i))
            .is_some_and(|cs| cs.iter().any(|c| c == class))
    }

    /// A mutable handle to an EXISTING tile's composition, without creating one: the graze draw, so a
    /// graze on an unwritten tile is a no-op. `None` when the tile has no composition.
    fn composition_get_mut(&mut self, coord: Coord3) -> Option<&mut Composition> {
        match self.dense_index(coord) {
            Some(i) => self.dense[i].as_mut(),
            None => self.overflow.get_mut(&coord),
        }
    }

    /// The matter composition on a tile, if any. An off-grid or unwritten tile reads `None`.
    pub fn composition(&self, coord: Coord3) -> Option<&Composition> {
        match self.dense_index(coord) {
            Some(i) => self.dense[i].as_ref(),
            None => self.overflow.get(&coord),
        }
    }

    /// Every occupied cell's composition, in no particular order (for the order-free sum and any queries).
    fn cells(&self) -> impl Iterator<Item = &Composition> {
        self.dense
            .iter()
            .filter_map(|c| c.as_ref())
            .chain(self.overflow.values())
    }

    /// Whether a coordinate bears a source of an axis: does its composition carry a NONZERO supply on
    /// the axis's backing class. A present zero is absence (the substrate convention), so it is not a
    /// source. Reads only the axis's backing-class string and the tile composition, never a race,
    /// species, or kind identifier (Principle 9).
    pub fn source(
        &self,
        axis: HomeostaticAxisId,
        coord: Coord3,
        homeo: &HomeostaticRegistry,
    ) -> bool {
        let Some(class) = homeo
            .axis(axis)
            .and_then(|a| a.backing_component.as_deref())
        else {
            return false;
        };
        self.composition(coord)
            .is_some_and(|c| c.nutrient(class) > Fixed::ZERO)
    }

    /// The registered axes this field carries a source for anywhere, in the registry's canonical
    /// order: a backed axis some tile's composition carries a nonzero supply of.
    pub fn axes(&self, homeo: &HomeostaticRegistry) -> Vec<HomeostaticAxisId> {
        homeo
            .axes
            .iter()
            .filter(|def| {
                def.backing_component
                    .as_deref()
                    .is_some_and(|class| self.cells().any(|c| c.nutrient(class) > Fixed::ZERO))
            })
            .map(|def| def.id)
            .collect()
    }

    /// The registered axes whose source is on a given tile, in the registry's canonical order (what a
    /// being can ingest where it stands): a backed axis the tile's composition carries a nonzero
    /// supply of.
    pub fn axes_here(&self, coord: Coord3, homeo: &HomeostaticRegistry) -> Vec<HomeostaticAxisId> {
        let Some(comp) = self.composition(coord) else {
            return Vec::new();
        };
        homeo
            .axes
            .iter()
            .filter(|def| {
                def.backing_component
                    .as_deref()
                    .is_some_and(|class| comp.nutrient(class) > Fixed::ZERO)
            })
            .map(|def| def.id)
            .collect()
    }

    /// The standing supply of one nutrient class on a tile (base-level liveliness step 3): the amount a
    /// grazer reads before it bites. An absent tile or class reads as zero (the substrate absence
    /// convention). Keyed off the class string alone, never a race or kind id (Principle 9).
    pub fn supply(&self, coord: Coord3, class: &str) -> Fixed {
        self.composition(coord)
            .map(|c| c.nutrient(class))
            .unwrap_or(Fixed::ZERO)
    }

    /// Remove up to `want` of one nutrient class from a tile's standing supply, returning what was
    /// removed (never more than is present, never negative), the grazing draw the ingest arm
    /// makes on the living resource loop (base-level liveliness step 3). A depleted tile feeds the next
    /// being less, so competition is the id-sorted walk's sequential draw with no new randomness, and a
    /// grazed-out tile empties and beings move on. Reads and writes only the class string's supply, no
    /// identity (Principle 9); a tile or class with no supply is a no-op returning zero.
    pub fn take(&mut self, coord: Coord3, class: &str, want: Fixed) -> Fixed {
        let Some(comp) = self.composition_get_mut(coord) else {
            return Fixed::ZERO;
        };
        let Some(supply) = comp.nutrients.get_mut(class) else {
            return Fixed::ZERO;
        };
        let taken = want.clamp(Fixed::ZERO, *supply);
        *supply -= taken;
        taken
    }

    /// The total standing supply of one nutrient class over every tile (base-level liveliness step 3):
    /// the whole map's grazable stock of that class, for the carrying-capacity reader. A pure read of
    /// hashed state.
    pub fn total_supply(&self, class: &str) -> Fixed {
        Fixed::saturating_sum(self.cells().map(|c| c.nutrient(class)))
    }

    /// Fold one cell into the hash exactly as the old `BTreeMap<Coord3>` walk did: the coordinate as three
    /// `i64`s, then nutrients then toxins, each `Composition` map in string-sorted (`BTreeMap`) order with
    /// each class byte through `write_u32`. `Composition` is unchanged, so this is byte-identical.
    fn fold_cell(h: &mut StateHasher, coord: Coord3, comp: &Composition) {
        h.write_i64(coord.x as i64);
        h.write_i64(coord.y as i64);
        h.write_i64(coord.z as i64);
        for (class, supply) in &comp.nutrients {
            for b in class.as_bytes() {
                h.write_u32(*b as u32);
            }
            h.write_fixed(*supply);
        }
        for (class, dose) in &comp.toxins {
            for b in class.as_bytes() {
                h.write_u32(*b as u32);
            }
            h.write_fixed(*dose);
        }
    }

    /// Fold the standing resource supplies into a hash in canonical `Coord3` order (base-level liveliness
    /// step 3). The old form walked a `BTreeMap<Coord3>`, which orders x-primary, then y, then z. The dense
    /// ground layer reproduces that byte-for-byte with a STRIDED x-major walk (`for x { for y }`) at
    /// snapshot time only (the hot paths stay sequential `y*w+x`), skipping unoccupied slots so an absent
    /// cell folds nothing. When the overflow is empty (every canonical run) that walk is the whole fold;
    /// otherwise all cells are merged into full `Coord3` order for the rare non-ground / fixture case.
    pub fn hash_into(&self, h: &mut StateHasher) {
        let w = self.width.max(0) as usize;
        if self.overflow.is_empty() {
            for x in 0..self.width {
                for y in 0..self.height {
                    let i = (y as usize) * w + (x as usize);
                    if let Some(comp) = &self.dense[i] {
                        Self::fold_cell(h, Coord3::ground(x, y), comp);
                    }
                }
            }
        } else {
            let mut all: Vec<(Coord3, &Composition)> = Vec::new();
            for x in 0..self.width {
                for y in 0..self.height {
                    let i = (y as usize) * w + (x as usize);
                    if let Some(comp) = &self.dense[i] {
                        all.push((Coord3::ground(x, y), comp));
                    }
                }
            }
            for (coord, comp) in &self.overflow {
                all.push((*coord, comp));
            }
            all.sort_by_key(|(c, _)| *c);
            for (coord, comp) in all {
                Self::fold_cell(h, coord, comp);
            }
        }
    }
}

/// A being that occupies the map and can walk: its stable id, its exact position in fractional tile
/// coordinates, its body plan (the physics of how it moves), its homeostatic reserves (its needs,
/// draining by metabolism), its expressed behaviour controller (its evolved policy) and the hidden
/// state the controller carries, its own remembered knowledge of where sources of each axis are (a
/// belief earned by perceiving them, not a copy of the world), and whether it is still alive.
#[derive(Clone, Debug)]
pub struct Walker {
    /// The stable id, the canonical order beings are walked in.
    pub id: StableId,
    /// Position along the world x axis, in tiles, fractional.
    pub x: Fixed,
    /// Position along the world y axis, in tiles, fractional.
    pub y: Fixed,
    /// The body plan the movement physics reads (mass, activity, whether it has locomotion at all).
    pub body: BodyPlan,
    /// The grown body structure, when the being's body was GROWN from its genome (emergent-anatomy Step 2)
    /// rather than drawn from a catalog. When `Some`, the run reads its affordances and ground speed from
    /// the grown segments' physics DIRECTLY ([`Structure`], via `afforded_structure` /
    /// `locomotion_speed_structure`), with no organs registry and no kind id; when `None`, the being carries
    /// a catalog body read against the shared registry. Materialised once (the LOD split: the aggregate
    /// `body` is the digest, the structure the full grown graph); it is not folded into `state_hash`
    /// directly (a static derived body), so its effect on the run folds only through the dynamic state it
    /// drives (position, reserves), the same as the catalog body.
    pub structure: Option<Structure>,
    /// The accumulated WHOLE-BODY wound damage of a CATALOG being (predation-integration slice): the coarse
    /// branch of the unified wound law, for a being that carries no grown `Structure` (so a per-Segment
    /// `Segment.damage` has nowhere to land). A strike adds a wound fraction here; the being's whole-body
    /// integrity is one minus this (clamped), set into the INTEGRITY axis so the SAME one unified cull removes
    /// it, no new death path. ZERO by default and only ever nonzero under an armed predator, so a being that is
    /// never struck folds nothing into `state_hash` (opt-in, hash-neutral by default), exactly like `structure`.
    pub whole_body_damage: Fixed,
    /// The heritable reproductive controller of a MODELED-LINEAGE being (creature-selection step 2, the
    /// in-run reproduction and behaviour-selection substrate): the controller weights this being contributes
    /// to an offspring under the creature reproduction beat. `None` for a being that is NOT a reproducing
    /// lineage member: a founder (whose reproduction runs through the `World` and its genome), and an authored
    /// environmental GIVEN like the ambush predator, which carries an authored `controller` that drives its
    /// behaviour but NO heritable material, so it structurally cannot contribute to an offspring and fails the
    /// pairing predicate ([`Walker::carries_lineage`]) for the SAME reason it cannot adapt (it has no heritable
    /// controller to pass on), never an authored id tag. `None` by default, so a being that carries no lineage
    /// folds nothing into `state_hash` (opt-in, hash-neutral by default) and the reproduction beat never sees
    /// it, exactly like `structure` and `whole_body_damage`.
    pub lineage: Option<Controller>,
    /// The homeostatic reserves: the being's needs as physical states of its body.
    pub homeostasis: Homeostasis,
    /// The consumer physiology the ingest measure reads: its per-class requirement and assimilation
    /// over the biology-floor classes, per-instance like the body and reserves. What a bite of the
    /// matter underfoot is worth to this being is measured against this, never authored.
    pub physiology: Physiology,
    /// The expressed behaviour controller, the being's evolved policy.
    pub controller: Controller,
    /// The controller's carried hidden state (empty for a reaction norm).
    pub hidden: Vec<Fixed>,
    /// What this being knows: the tiles bearing a source of each axis that it has perceived. It
    /// navigates by this, not by the world, so it can only head for a source it has come to know.
    pub known: BTreeMap<HomeostaticAxisId, BTreeSet<Coord3>>,
    /// The being's memory of its reserve levels at the previous tick, the substrate of the
    /// interoceptive DELTA percept (harm-learning arc slice a). Empty until the harm-learning path
    /// snapshots it, so a being in a world that declares no percepts carries an empty memory that folds
    /// nothing into `state_hash` (the delta percept is opt-in, hash-neutral by default).
    pub reserve_memory: ReserveMemory,
    /// The matter the being carries (material-substrate arc, cascade item 3): a mixture of substances by
    /// volume, bound to the carrier. EMPTY by default, so a being that carries nothing folds nothing
    /// into `state_hash` (the carry substrate is opt-in, hash-neutral by default). Pick-up moves matter
    /// from the ground into it, bounded by the being's grown strength against the load's derived weight;
    /// put-down deposits it back, and the carried weight feeds locomotion cost so an over-laden being
    /// slows (both wired in the run-path slice that follows this substrate).
    pub carried: SubstanceMix,
    /// The tool the being wields, if any (material-substrate arc, cascade item 4, crafting). `None` by
    /// default, so a being wielding nothing folds nothing into `state_hash` (the wielded slot is opt-in,
    /// hash-neutral by default) and its extraction contest uses its bare working surface. A wielded tool
    /// supplies a smaller contact area and its own material to the extraction and cut contests, so a
    /// crafted point breaks harder rock than a bare limb (the tool multiplies the affordance).
    pub wielded: Option<WieldedTool>,
    /// The being's ELIGIBILITY TRACE (ideation / experiential-discovery arc, piece 1, slice 1c): the short,
    /// recency-decayed memory of the primitive sequences it recently executed, so a reserve rise felt after
    /// an action can credit the sequence that produced it ([`crate::learn::EligibilityTrace`]). EMPTY by
    /// default, so a being in a world with no reward learning armed folds nothing into `state_hash` (the
    /// reward trace is opt-in, hash-neutral by default). Populated, decayed, and credited only where the
    /// runner arms the reward learner.
    pub eligibility_trace: EligibilityTrace,
    /// The being's HARM eligibility trace (the being-percept keystone, step 6): the short, recency-decayed
    /// memory of the being-signals it recently PERCEIVED, so a harm felt after perceiving another being can
    /// credit the signals that preceded it (the lagged predator-approach cue, the distal association the
    /// same-tick learner cannot form). Distinct from [`Self::eligibility_trace`], the reward/action trace:
    /// this records being-signal subjects and decays by the reserved harm eligibility latency. EMPTY by
    /// default, so a being in a world with the being-percept off folds nothing into `state_hash` (opt-in,
    /// hash-neutral by default), exactly as the reward trace is.
    pub harm_eligibility_trace: EligibilityTrace,
    /// The being's CONVICTION-EXPERIENCE record (Branch 1 of the learned experience-to-conviction coupling,
    /// `docs/working/OWNER_DECISIONS_LOG.md` R2/R4): the per-conviction leaky signed accumulator that learns,
    /// by correlation over the being's own life, which conviction its felt experience bears on
    /// ([`crate::conviction_experience::ConvictionExperience`]). EMPTY by default, so a being in a world with
    /// no felt-conviction learner armed folds nothing into `state_hash` (opt-in, hash-neutral by default).
    /// Folded each tick from the being's own felt reserve swings and its held convictions only where the runner
    /// arms the learner; it is inert (records, changes no conviction and no behaviour: Branch 2 consumes it).
    pub conviction_experience: ConvictionExperience,
    /// The affordance the being ACTED on this tick, the head of the sequence its eligibility trace records
    /// (ideation arc, piece 1, slice 1c). Set each tick from the being's own decision and `None` on a tick
    /// it took no matter action; transient (re-derived every tick, never folded into `state_hash`), the
    /// source the reward-credit pass reads to record what the being just did.
    pub decided_affordance: Option<AffordanceId>,
    /// The full step the being ACTED on this tick (social-learning arc, piece 3, material granularity): the
    /// primitive plus the target's affordance channel and quantized perceived value, when the action came
    /// from a discovery proposal or a deliberated plan (which carry the target); a plain primitive with a
    /// zero target when the base controller decided it (which names no target). `None` on a tick it took no
    /// matter action. Transient like [`decided_affordance`](Walker::decided_affordance) (re-derived every
    /// tick, never folded into `state_hash`); the granular reward credit keys the belief on this step's
    /// grain through [`crate::learn::step_belief_subject`], so a technique specialises to the target it acted
    /// on. Its `primitive` always equals `decided_affordance`; it carries the target the primitive alone
    /// cannot.
    pub decided_step: Option<SequenceStep>,
    /// The matter the being INGESTED this tick, the perceived composition of what it ate (social-learning
    /// arc, piece 1, nutrition learning): the per-substance volume [`crate::runner::Embodiment::geophage`]
    /// took into the being this tick, keyed by material-floor substance id. EMPTY on a tick it ate nothing.
    /// Transient like [`decided_affordance`](Walker::decided_affordance): cleared at the top of the
    /// embodiment step and re-derived by the bite, NEVER folded into `state_hash` (its only reach into canon
    /// is the reward belief the nutrition credit re-earns from it, which folds through the mind, and only
    /// when nutrition learning is armed). It is the eaten-side sibling of the ground-side material percept:
    /// where the trace credit reads the composition of the cell UNDERFOOT, this reads the composition of
    /// what was EATEN, so a being learns which foods nourish it rather than which place it stood on.
    pub ate: SubstanceMix,
    /// The candidate action the being PROPOSES this tick, sampled from its binding graph by the discovery
    /// loop (ideation arc, piece 2, slice 2c). `None` unless the runner arms the discovery loop and the world
    /// installs the affordance-percept registry, so it stays `None` and folds nothing by default (opt-in,
    /// byte-identical). Set by the runner's discovery pass each tick and folded into `state_hash`, the
    /// hypothesis a being is about to test.
    pub proposed_action: Option<SequenceStep>,
    /// The being's heritable EXPLORATION propensity (ideation arc, piece 2, slice 2c-2): the rate in
    /// `[0, 1]` at which it ACTS on the action it proposed. FOUNDER-ZERO: a founder reads zero and never
    /// enacts a proposal, so it is byte-identical, and only a mutant whose propensity is lifted off zero
    /// tries the untried, so exploration EMERGES by selection rather than being switched on (Principle 9).
    /// Expressed from a heritable `Channel::Exploration` at the birth path in a follow-on; this slice proves
    /// the enactment gate with a PRIMED value, exactly as the appetitive weight was proved primed. Folds
    /// into `state_hash` when positive.
    pub exploration: Fixed,
    /// The being's recent PREDICTION-ERROR magnitude, its surprise (ideation arc, piece 3, slice 3b): how far
    /// the outcome of its last enacted action defied its forward-model prediction, in `[0, 1]`. It MODULATES
    /// the exploration gate multiplicatively (a surprised being enacts its proposals more, a well-predicting
    /// one less), so exploration rises where the world is least understood and falls as predictions come
    /// true. Dynamic state, not heritable, updated each tick from `forward_model::prediction_error` on the
    /// last-enacted action at the one-tick interoceptive lag and decayed otherwise; ZERO by default (no
    /// discovery loop, or nothing enacted yet), so it folds nothing and an opted-out run is byte-identical.
    /// Because the modulation is MULTIPLICATIVE on the heritable propensity, a founder (zero propensity) never
    /// explores however surprised, so founder-zero holds. Folds into `state_hash` when positive.
    pub surprise: Fixed,
    /// The being's heritable DELIBERATION weight (ideation arc, piece 4, slice 4b): the rate in `[0, 1]` at
    /// which it ACTS on the believed-best action its planner recalled toward a goal, rather than only on what
    /// it perceives underfoot. FOUNDER-ZERO: a founder reads zero and never deliberates, so goal-directed
    /// pursuit EMERGES by selection rather than being switched on (Principle 9), never a coded "when idle,
    /// plan". The deliberation analogue of `exploration`: where exploration tries the untried, deliberation
    /// exploits the best-believed, and the two are independent heritable drives. Proved with a PRIMED value
    /// here, expressed from a heritable channel at the birth path in a follow-on. Folds into `state_hash`
    /// when positive.
    pub deliberation: Fixed,
    /// The being's heritable SOCIAL-LEARNING weight (social-learning arc, piece 2, observe-and-imitate): the
    /// rate in `[0, 1]` at which its discovery proposal is BIASED toward an action it perceived a co-located
    /// being enact (carried in `observed_actions`). FOUNDER-ZERO: a founder reads zero and ignores what it
    /// sees, so imitation EMERGES by selection rather than being switched on (Principle 9). Expressed from
    /// [`crate::genome::Channel::SocialLearning`] at the birth path exactly as `exploration` is; tests prime
    /// it directly. Folds into `state_hash` when positive.
    pub social_learning: Fixed,
    /// The being's TRANSIENT observed-action prior (social-learning arc, piece 2): the set of primitive ids
    /// of the actions co-located OTHER beings enacted-and-ate last tick, the valence-free ActionTrace this
    /// being perceived. Read by the discovery sampler to tip the draw toward a demonstrated action (scaled by
    /// `social_learning`), NEVER a belief: it biases what the being tries, and its own felt reward stays the
    /// sole gate to a committed belief (copy-and-verify). Rebuilt each tick from co-located eating (empty
    /// where a being saw no one eat), so it is a one-tick memory, not a store. Populated only when the world
    /// arms observe-and-imitate; empty otherwise, so it folds nothing into `state_hash` (opt-in default) and
    /// leaves an opted-out run byte-identical. Folded (in sorted id order) only when non-empty.
    pub observed_actions: BTreeSet<u16>,
    /// Whether the being is alive. A being whose reserve falls through its floor dies and stops.
    pub alive: bool,
}

impl Walker {
    /// A walker placed at the centre of a tile with the given reserves and controller, no knowledge
    /// yet: it has seen nothing and must perceive or explore to learn the world.
    pub fn new(
        id: StableId,
        tile: Coord3,
        body: BodyPlan,
        homeostasis: Homeostasis,
        physiology: Physiology,
        controller: Controller,
    ) -> Walker {
        let hidden = controller.fresh_hidden();
        Walker {
            id,
            x: Fixed::from_int(tile.x) + HALF,
            y: Fixed::from_int(tile.y) + HALF,
            body,
            structure: None,
            whole_body_damage: Fixed::ZERO,
            lineage: None,
            homeostasis,
            physiology,
            controller,
            hidden,
            known: BTreeMap::new(),
            reserve_memory: ReserveMemory::new(),
            carried: SubstanceMix::new(),
            wielded: None,
            eligibility_trace: EligibilityTrace::new(),
            harm_eligibility_trace: EligibilityTrace::new(),
            conviction_experience: ConvictionExperience::new(),
            decided_affordance: None,
            decided_step: None,
            ate: SubstanceMix::new(),
            proposed_action: None,
            exploration: Fixed::ZERO,
            surprise: Fixed::ZERO,
            deliberation: Fixed::ZERO,
            social_learning: Fixed::ZERO,
            observed_actions: BTreeSet::new(),
            alive: true,
        }
    }

    /// Attach a GROWN body structure to this walker (emergent-anatomy Step 2): the run then reads the
    /// being's affordances and ground speed from the grown segments' physics directly, rather than from the
    /// catalog `body` against the shared organs registry. The aggregate `body` stays as the LOD-0 digest the
    /// metabolism reads. A builder, so the founder and newborn embodiment can grow a body from the genome
    /// and hand it here without changing [`Walker::new`]'s many callers.
    pub fn with_structure(mut self, structure: Structure) -> Walker {
        self.structure = Some(structure);
        self
    }

    /// Mark this being as a MODELED-LINEAGE member carrying the heritable reproductive controller `lineage`
    /// (creature-selection step 2). A builder, so `spawn_creatures` can hand a creature its heritable
    /// controller without changing [`Walker::new`]'s many callers; a being minted without it (a founder body,
    /// the authored predator) carries no lineage and cannot enter the creature reproduction beat.
    pub fn with_lineage(mut self, lineage: Controller) -> Walker {
        self.lineage = Some(lineage);
        self
    }

    /// The STRUCTURAL pairing predicate (creature-selection step 2, replacing the `PREDATOR_ID_BIT` id tag):
    /// whether this being carries heritable reproductive material, so it can contribute to an offspring under
    /// the creature reproduction beat. It reads what the being IS (does it carry a heritable controller),
    /// never its id or a species tag, so the authored predator is excluded for the same structural reason it
    /// cannot adapt: it carries no `lineage`. A pure read.
    pub fn carries_lineage(&self) -> bool {
        self.lineage.is_some()
    }

    /// The tile the being currently stands on.
    pub fn coord(&self) -> Coord3 {
        Coord3::ground(floor_i32(self.x), floor_i32(self.y))
    }

    /// Record that this being now knows of a source of `axis` at `coord`.
    pub fn learn(&mut self, axis: HomeostaticAxisId, coord: Coord3) {
        self.known.entry(axis).or_default().insert(coord);
    }

    /// The nearest source of `axis` this being knows of, to where it stands, by squared distance
    /// with a canonical tie-break. `None` if it knows of none.
    fn nearest_known(&self, axis: HomeostaticAxisId) -> Option<Coord3> {
        let from = self.coord();
        self.known.get(&axis)?.iter().copied().min_by_key(|c| {
            let dx = (c.x - from.x) as i64;
            let dy = (c.y - from.y) as i64;
            (dx * dx + dy * dy, c.x, c.y)
        })
    }
}

/// One-half, the tile centre offset.
const HALF: Fixed = Fixed::from_bits(1i64 << 31);
/// The smallest squared heading magnitude that counts as a directional signal; below it the being
/// has no gradient to follow and explores instead.
const HEADING_EPS: Fixed = Fixed::from_bits(1i64 << 22); // ~1e-3

/// Floor a fractional tile coordinate to its integer tile (arithmetic shift floors negatives too;
/// Q32.32 fixed point).
fn floor_i32(v: Fixed) -> i32 {
    (v.to_bits() >> 32) as i32
}

/// The physics of a body's ground speed on a tile, in tiles per tick. It rises with body size (an
/// allometric square-root of mass, larger bodies taking longer strides), scales with the temperament
/// activity axis between the reserved floor and one, and is divided down by terrain cost above open
/// ground. A body with no locomotion organ does not move. Whether the being has the reserves to move
/// is the metabolism's concern, not this pure physical speed.
///
/// The size factor is DERIVED from the body's grown limbs (emergent-anatomy step one), not from an
/// allometric mass power law: across the body's locomotion modes the strongest LOCOMOTE limb (one that
/// bears a reference propulsive load without buckling, read from its section modulus, length, and yield
/// through [`civsim_compose::derive_capabilities`]) sets both the stride (its `mech.arm_length` over the
/// reserved reference leg length) and the push (its LOCOMOTE capability, one minus the bending
/// utilization). This retires the `sqrt(body_mass)` proxy: a longer, stouter limb strides farther and
/// pushes off harder, so a strong-limbed lineage disperses faster than a weak-limbed one, by physics
/// rather than by mass, blind to any kind or race id. Per-being limb variation (and so per-being speed)
/// returns when step two grows the limb geometry per body.
pub fn locomotion_speed(
    body: &BodyPlan,
    organs: &BodyPlanRegistry,
    terrain_cost: Fixed,
    p: &LocomotionParams,
) -> Fixed {
    // The strongest locomotor limb the body bears: its LOCOMOTE capability (structural push) and its leg
    // length (stride), a pure physics read over the organ registry, blind to any kind or race id.
    let fns = FunctionLawRegistry::dev_seed();
    let mut best_cap = Fixed::ZERO;
    let mut stride_leg = Fixed::ZERO;
    for &m in &body.locomotion {
        if let Some(k) = organs.locomotion.iter().find(|k| k.id == m) {
            let geo = |axis: &str| k.geo(axis);
            let mat = |axis: &str| k.mat(axis);
            let cap = derive_capabilities(&fns, &geo, &mat, &p.capability_refs, &p.capability_caps)
                .score(FunctionLawRegistry::ID_LOCOMOTE);
            if cap > best_cap {
                best_cap = cap;
                stride_leg = k.geo("mech.arm_length");
            }
        }
    }
    if best_cap <= Fixed::ZERO {
        return Fixed::ZERO; // no limb bears a propulsive load: rooted, by physics not by a mode id
    }
    // Stride length from the grown limb over the reserved reference leg length, clamped to [0, 1], scaled
    // by the limb's structural push (its LOCOMOTE capability): the grown-limb size factor that retires
    // sqrt(body_mass).
    let stride = if p.reference_leg_length > Fixed::ZERO {
        stride_leg
            .div(p.reference_leg_length)
            .clamp(Fixed::ZERO, Fixed::ONE)
    } else {
        Fixed::ZERO
    };
    let size = stride.mul(best_cap);
    // Activity factor between the reserved floor and one.
    let activity = p.activity_floor
        + (Fixed::ONE - p.activity_floor)
            .mul(body.temperament.activity.clamp(Fixed::ZERO, Fixed::ONE));
    // Terrain divisor: 1 + terrain_penalty * (cost - 1), never below one.
    let over = if terrain_cost > Fixed::ONE {
        terrain_cost - Fixed::ONE
    } else {
        Fixed::ZERO
    };
    let divisor = Fixed::ONE + p.terrain_penalty.mul(over);
    let raw = p.base_speed.mul(size).mul(activity);
    let speed = if divisor > Fixed::ZERO {
        raw.div(divisor)
    } else {
        raw
    };
    speed.clamp(Fixed::ZERO, p.base_speed)
}

/// The ground speed of a GROWN body, read from its [`Structure`] directly (emergent-anatomy Step 2): the
/// same grown-limb physics as [`locomotion_speed`], but the stride and the structural push come from the
/// structure's strongest LOCOMOTE segment ([`Structure::best_locomotor_stride`]) rather than a catalog mode
/// looked up in the organs registry, so a body no catalog contains moves exactly as fast as its grown limb
/// bears. The activity factor reads the being's temperament activity (carried on the LOD-0 digest), so the
/// caller supplies it. A structure whose every segment reads zero LOCOMOTE is rooted and does not move.
pub fn locomotion_speed_structure(
    structure: &Structure,
    activity: Fixed,
    terrain_cost: Fixed,
    p: &LocomotionParams,
) -> Fixed {
    let fns = FunctionLawRegistry::dev_seed();
    let (best_cap, stride_leg) =
        structure.best_locomotor_stride(&fns, &p.capability_refs, &p.capability_caps);
    if best_cap <= Fixed::ZERO {
        return Fixed::ZERO; // no grown limb bears a propulsive load: rooted, by physics
    }
    let stride = if p.reference_leg_length > Fixed::ZERO {
        stride_leg
            .div(p.reference_leg_length)
            .clamp(Fixed::ZERO, Fixed::ONE)
    } else {
        Fixed::ZERO
    };
    let size = stride.mul(best_cap);
    let activity = p.activity_floor
        + (Fixed::ONE - p.activity_floor).mul(activity.clamp(Fixed::ZERO, Fixed::ONE));
    let over = if terrain_cost > Fixed::ONE {
        terrain_cost - Fixed::ONE
    } else {
        Fixed::ZERO
    };
    let divisor = Fixed::ONE + p.terrain_penalty.mul(over);
    let raw = p.base_speed.mul(size).mul(activity);
    let speed = if divisor > Fixed::ZERO {
        raw.div(divisor)
    } else {
        raw
    };
    speed.clamp(Fixed::ZERO, p.base_speed)
}

/// Perceive the world within the being's sensory range: for each axis the field carries, any source
/// tile within `sense_range` tiles of where the being stands is learned. This is the being seeing
/// what is near it; it learns nothing about tiles beyond its senses.
fn perceive(w: &mut Walker, resources: &ResourceField, homeo: &HomeostaticRegistry, range: i64) {
    let here = w.coord();
    let axes = resources.axes(homeo);
    for axis in axes {
        for dy in -range..=range {
            for dx in -range..=range {
                let c = Coord3::ground(here.x + dx as i32, here.y + dy as i32);
                if resources.source(axis, c, homeo) {
                    w.learn(axis, c);
                }
            }
        }
    }
}

/// The unit direction from a being to the nearest known source of each axis it knows of. A source
/// on the being's own tile reads as a zero direction (there is nowhere to go for it); the being
/// tells that case apart through the separate here-flag the percept carries.
fn source_dirs(w: &Walker) -> BTreeMap<HomeostaticAxisId, (Fixed, Fixed)> {
    let mut m = BTreeMap::new();
    let axes: Vec<HomeostaticAxisId> = w.known.keys().copied().collect();
    for axis in axes {
        if let Some(c) = w.nearest_known(axis) {
            let tx = Fixed::from_int(c.x) + HALF;
            let ty = Fixed::from_int(c.y) + HALF;
            let dx = tx - w.x;
            let dy = ty - w.y;
            let dist = (dx.mul(dx) + dy.mul(dy)).sqrt();
            if dist > Fixed::ZERO {
                let ux = dx.div(dist).clamp(Fixed::from_int(-1), Fixed::ONE);
                let uy = dy.div(dist).clamp(Fixed::from_int(-1), Fixed::ONE);
                m.insert(axis, (ux, uy));
            } else {
                m.insert(axis, (Fixed::ZERO, Fixed::ZERO));
            }
        }
    }
    m
}

/// Advance every being one tick of controller-driven locomotion. Each perceives nearby sources into
/// its memory, reads its reserves and its percept, and its controller decides which affordance to
/// issue: moving (toward a known source it is drawn to, or exploring when it has no gradient) or
/// ingesting the matter underfoot. Then its metabolism drains its reserves, more when it exerted
/// itself, and a being whose reserve falls through its floor dies. Deterministic: beings are walked
/// in stable-id order, the controller and metabolism draw no randomness, exploration keys on
/// `(seed, being, tick)`, and every step is exact fixed-point. Returns the number of beings that
/// moved this tick.
#[allow(clippy::too_many_arguments)]
pub fn step<T: Terrain>(
    walkers: &mut [Walker],
    homeo: &HomeostaticRegistry,
    layout: &ControllerLayout,
    afford: &AffordanceRegistry,
    terrain: &T,
    resources: &ResourceField,
    p: &LocomotionParams,
    seed: u64,
    tick: u64,
) -> usize {
    // The field-less entry does NOT deplete the caller's resource field: it is the scoring and fixture
    // path (the evolve proxy, the movement tests), whose resource field is a re-seeded fixture, not a
    // living stock. It grazes a throwaway copy so the ingest deposit is measured identically while the
    // caller's field is left intact; the live run path drives the depleting `step_with_field_dirs`
    // directly with a `&mut ResourceField` (base-level liveliness step 3). Being the labelled-fixture
    // entry, it supplies the labelled-fixture organ registry the affordance and speed derives read; the
    // live run path passes the world's own registry from the runner (emergent-anatomy step one).
    let mut scratch = resources.clone();
    let organs = BodyPlanRegistry::dev_default();
    step_with_field_dirs(
        walkers,
        homeo,
        layout,
        afford,
        &organs,
        terrain,
        &mut scratch,
        p,
        seed,
        tick,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &BTreeMap::new(),
        // The field-less fixture path installs no physiology, so the INGEST uses the pre-grounding measure.
        None,
        // The field-less scoring/fixture path senses no features (the evolve proxy and movement tests
        // run without the percept substrate), so the controller feature block, if any, reads zero.
        &PerceptRegistry::empty(),
        // The field-less fixture path senses no material either (it carries no material field), so the
        // material-feature block, if any, reads zero.
        &MaterialPerceptRegistry::empty(),
        &MaterialField::new(),
        // No carried-load penalty on the field-less fixture path (nothing picks matter up here).
        &BTreeMap::new(),
        // No appetitive belief percept on the field-less fixture path (the layout carries no appetitive
        // block here), so the controller's appetitive input, if any, reads zero.
        &BTreeMap::new(),
        // No attraction-direction percept on the field-less fixture path (the layout carries no attraction
        // block here), so the controller's attraction input, if any, reads zero.
        &BTreeMap::new(),
        // No conviction percept on the field-less fixture path (the layout carries no conviction block here),
        // so the controller's conviction input, if any, reads zero.
        &BTreeMap::new(),
        // No being-directed percept on the field-less fixture path (the layout carries no being block here),
        // so the controller's being input, if any, reads zero.
        &BTreeMap::new(),
        // The field-less fixture path enacts no grasp (it carries no material field); the sink is
        // discarded, so a decided grasp on this path is inert.
        &mut BTreeMap::new(),
    )
}

/// As [`step`], but with two additional per-being percept maps, each keyed by stable id then by
/// homeostatic axis. The first, `field_dirs`, is a directional percept a being senses from a physical
/// field rather than from a remembered point source: the temperature gradient the runner supplies for
/// the TEMPERATURE axis (the unit direction toward warmer surroundings at the being's cell), and later
/// a moisture or wind field, merged into that axis's direction slot alongside the known-source percept.
/// The second, `field_signed`, is the scalar signed setpoint-deviation percept for an axis (the raw
/// thermoreceptor: whether the body is too hot or too cold), fed into that axis's signed input slot.
/// Both are percepts, not headings: the controller must evolve to combine them (Principle 9), and
/// neither draws randomness, so determinism and camera-freedom hold. A field direction for an axis
/// overrides the known-source direction for that axis, since the field percept is the live signal for
/// a diffuse quantity that has no discrete source tile; the signed percept has no known-source
/// counterpart and is simply supplied.
///
/// The `drains` map is the per-being anatomy-derived metabolism (R-METABOLIZE): for a being with an
/// entry the tick's drain is applied through [`Homeostasis::metabolize_derived`] over its per-axis
/// [`DerivedDrain`] (the Kleiber basal rate plus the thermoregulatory replacement for the metabolic
/// axis, the authored per-axis rates for the others), so its survival follows its body plan, mass,
/// tissue, medium, and temperature rather than a hardcoded per-axis fraction. A being with no entry
/// (the labelled-fixture path used by the evolve harness and the field-only [`step`]) keeps the scalar
/// [`Homeostasis::metabolize`] over the axis defs' authored drains, so the derived path is retired only
/// where a caller supplies a derived drain. The exertion signal each being computes this tick scales
/// its exertion coupling in both paths, so the reconciliation with locomotion is exact.
/// The R-SOURCE-VECTOR per-reserve DRAW FOLD: fold a feeder reserve's declared draw over its axis SET into
/// that one reserve. Each term reads its source axis's standing supply, bridges it to reserve-fillable
/// content by the term's own unit (a matter term keeps the per-being `food_energy_density` bridge, a feeder
/// its own), fills the reserve bounded by the REMAINING room (each `ingest` raises `amount()`, so a later
/// term in the set sees the reduced room), and depletes the source only where the term's cached floor
/// character is a depletable stock. The deplete step keys on the CONSERVATION LAW of the quantity, never on
/// a source kind: a matter axis and a mixotroph's second axis both deplete, a photon or gravity-gradient
/// flux does not, and each is a data row over the axis registry with no branch added. A matter reserve's
/// singleton draw is NOT routed here (it stays the byte-identical fast path in the INGEST arm); this fold
/// runs only for a reserve that declares an explicit multi-term or feeder draw set.
#[allow(clippy::too_many_arguments)]
pub fn draw_feeder_set(
    draw_set: &[DrawTerm],
    axis_id: HomeostaticAxisId,
    here: Coord3,
    resources: &mut ResourceField,
    homeostasis: &mut Homeostasis,
    physiology: &Physiology,
    storage: &BTreeMap<String, Fixed>,
    body_mass: Fixed,
    eta: Fixed,
    food_energy_density: Fixed,
) {
    for term in draw_set {
        let supply = resources.supply(here, &term.class);
        if supply <= Fixed::ZERO {
            continue; // the cell is no source of this term's axis
        }
        let content = term.unit_bridge.content(supply, food_energy_density);
        let body_c = storage.get(&term.class).copied().unwrap_or(Fixed::ZERO);
        let room = homeostasis.capacity(axis_id) - homeostasis.amount(axis_id);
        let (eaten_content, gain) = crate::physiology::physical_intake(
            content,
            physiology.assimilation(&term.class),
            eta,
            body_mass,
            body_c,
            room,
        );
        if eaten_content <= Fixed::ZERO {
            continue;
        }
        // The deplete step keys on the source axis's floor conservation character, then the reserve gains.
        // A character the fold cannot yet honour FAILS LOUD before the gain: a draw must never silently
        // ingest against an undeclared or unwired character (the reserved-value discipline, Principle 11).
        match &term.depletion {
            DepletionCharacter::DepletableStock => {
                // A located stock: remove the supply the eaten content came from, so the cell depletes by
                // exactly what the reserve drew (mass conserved, the matter case and a mixotroph's tissue).
                let eaten_supply = term.unit_bridge.supply(eaten_content, food_energy_density);
                resources.take(here, &term.class, eaten_supply);
            }
            DepletionCharacter::NonRivalrousFlux => {
                // A renewable flux (a photon or gravity-gradient source): the draw gains but never depletes
                // it, so no `take`. This is the whole feeder generalization, a data row not a code branch.
            }
            DepletionCharacter::Reservoir => {
                // A reservoir's pool draw (a source-and-sink pool) is not yet wired; fail loud rather than
                // gain against a character the fold does not model, so it cannot silently grant a reserve.
                panic!(
                    "R-SOURCE-VECTOR: a reservoir draw is not yet wired (axis '{}'); a feeder must declare a \
                     depletable_stock or non_rivalrous_flux character until the reservoir path is built",
                    term.class
                );
            }
            DepletionCharacter::Reserved { .. } => {
                // An undeclared (reserved) character reaching a draw is a config error: the axis's floor
                // conservation character was never declared. Fail loud (the reserved-value discipline: never
                // a silent use of an unset value). The feeder-arming follow-on adds a validated DrawTerm
                // builder that rejects this at construction; this fold is the backstop.
                panic!(
                    "R-SOURCE-VECTOR: a draw on axis '{}' whose depletion character is undeclared (Reserved) \
                     is a config error; declare the axis's floor conservation character before a feeder draws it",
                    term.class
                );
            }
        }
        homeostasis.ingest(axis_id, gain);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn step_with_field_dirs<T: Terrain>(
    walkers: &mut [Walker],
    homeo: &HomeostaticRegistry,
    layout: &ControllerLayout,
    afford: &AffordanceRegistry,
    organs: &BodyPlanRegistry,
    terrain: &T,
    resources: &mut ResourceField,
    p: &LocomotionParams,
    seed: u64,
    tick: u64,
    field_dirs: &BTreeMap<StableId, BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>>,
    field_signed: &BTreeMap<StableId, BTreeMap<HomeostaticAxisId, Fixed>>,
    drains: &BTreeMap<StableId, BTreeMap<HomeostaticAxisId, DerivedDrain>>,
    // The metabolic anchors (R-PHYS-BIO edibility), `Some` when a physiology is installed: the forage INGEST
    // then fills a reserve by the food's PHYSICAL content, converted through the being's own body mass
    // (`body_mass_kg`) and its storage density on the reserve's backing class (`whole_body_composition_vector`,
    // computed inside from `organs` and `w.body`), the same size-scaled bridge the drain uses. `None` leaves
    // the INGEST on the pre-grounding satisfaction measure (byte-identical for a fixture with no physiology).
    intake_anchors: Option<crate::physiology::MetabolicAnchors>,
    percepts: &PerceptRegistry,
    material_percepts: &MaterialPerceptRegistry,
    material_field: &MaterialField,
    load_factors: &BTreeMap<StableId, Fixed>,
    appetitive: &BTreeMap<StableId, Vec<Fixed>>,
    attraction: &BTreeMap<StableId, Vec<Fixed>>,
    // Each being's own STANCE on each exposed conviction axis (Prereq B for the learned
    // experience-to-conviction coupling, `docs/working/OWNER_DECISIONS_LOG.md` R2), computed by the runner from
    // the being's intrinsic beliefs and written into the controller's conviction block. Empty (a being absent
    // from the map, or an empty map) when the world exposes no conviction (the layout carries no conviction
    // block), so the input is byte-identical to before the block existed. Only an evolved conviction weight
    // turns a stance into a behaviour bias, so a conviction-biased behaviour emerges rather than being authored.
    conviction: &BTreeMap<StableId, Vec<Fixed>>,
    // Each being's unit being-directed percept (the being-percept keystone, step 6): four components, the unit
    // direction AWAY from perceived believed-harm emitters (avoidance dx, dy) and TOWARD perceived
    // believed-reward ones (attraction dx, dy), computed by the runner from what the being perceived and its
    // learned beliefs, written into the controller's being block. Empty (a being absent from the map, or an
    // empty map) when the being-percept feature is off (the layout carries no being block), so the input is
    // byte-identical to before the block existed. Only an evolved FREELY-SIGNED weight turns the direction into
    // approach (predation) or avoidance (fleeing), so the approach/avoid sign emerges rather than being authored.
    being: &BTreeMap<StableId, Vec<Fixed>>,
    deferred_actions: &mut BTreeMap<StableId, (AffordanceId, Fixed)>,
) -> usize {
    walkers.sort_by_key(|w| w.id);
    let mut moved = 0usize;
    for w in walkers.iter_mut() {
        if !w.alive {
            continue;
        }
        // Clear the per-tick INGESTED-matter scratch (social-learning arc, nutrition learning): a being
        // that eats nothing this tick carries an empty `ate`, so the nutrition credit reads a clean slate
        // and a stale bite from an earlier tick never re-earns a belief. The bite below refills it. Not
        // folded into `state_hash` (transient scratch, like `decided_affordance`), so this is byte-neutral.
        w.ate = SubstanceMix::new();
        // Snapshot the reserves at the START of the tick, so this tick's interoceptive delta
        // (`delta(axis) = level_now - level_prev`) reads the NET change the tick then makes, the raw
        // signal both experiential learners read after metabolism. The harm learner (harm-learning arc
        // slice b) keys it to the FEATURE underfoot ("my CONDITION fell on this ground"), so it needs
        // the snapshot wherever the world declares percepts. The reward learner (ideation arc slice 1c)
        // keys it to the ACTION the being executed, carried in its eligibility trace ("this action I did
        // pays off"), so it needs the snapshot wherever that trace is populated, which happens only when
        // the reward learner is armed. Opt-in on both sides: a run that declares no percepts and arms no
        // reward learner carries an empty trace, takes no snapshot, and folds an empty memory into
        // state_hash, so it stays bit-identical. A pure canonical-order snapshot drawing no randomness.
        if !percepts.is_empty() || !material_percepts.is_empty() || !w.eligibility_trace.is_empty()
        {
            w.reserve_memory.snapshot(homeo, &w.homeostasis);
        }
        // Perceive first, so knowledge gained this tick is available to this tick's decision.
        perceive(w, resources, homeo, p.sense_range);
        let here = w.coord();
        // Environmental harm (base-level liveliness step 4): the toxin dose of the cell the being stands
        // on this tick, measured against its OWN heritable tolerances through the dose-response harm law.
        // Captured now (before any movement) as a scalar, applied to the CONDITION reserve below. A being
        // with no tolerance for a class takes no harm from it (the class does not apply); a low-tolerance
        // being on a salt flat accrues harm and dies, a high-tolerance one shrugs it off, so a lineage
        // adapts to the gradient by selection rather than a fixed-dose gate (Principles 8, 9). Reads only
        // the tile toxins and the being's own physiology, no race or kind id.
        let harm = match resources.composition(here) {
            Some(comp) if !comp.toxins.is_empty() => {
                let classes: Vec<(Fixed, Option<Fixed>, u8)> = comp
                    .toxins
                    .iter()
                    .map(|(class, &dose)| {
                        (
                            dose,
                            w.physiology.tolerance(class),
                            w.physiology.hill_exp(class),
                        )
                    })
                    .collect();
                laws::net_harm(&classes, p.harm_caps.harm_cap, p.harm_caps.total_harm_cap)
            }
            _ => Fixed::ZERO,
        };
        let here_axes: BTreeSet<HomeostaticAxisId> =
            resources.axes_here(here, homeo).into_iter().collect();
        let mut dirs = source_dirs(w);
        // Merge the field-derived percept for this being: a directional signal it senses from a
        // physical field (the temperature comfort gradient), overriding the known-source direction for
        // that axis since a diffuse field has no discrete source tile to remember.
        if let Some(fd) = field_dirs.get(&w.id) {
            for (&axis, &d) in fd {
                dirs.insert(axis, d);
            }
        }
        // The signed setpoint-deviation percept for this being (the raw thermoreceptor), empty when
        // none is supplied so the signed input reads zero, as it did before this percept existed.
        let empty_signed = BTreeMap::new();
        let signed = field_signed.get(&w.id).unwrap_or(&empty_signed);
        // The raw perceived-feature vector for the cell the being stands on (harm-learning arc slice a):
        // the amount of each declared substance class underfoot, in registry order. Empty when the
        // world declares no percepts, so the feature block is absent and the input is byte-identical to
        // before the feature substrate existed. A pure physical read of what is here, no threshold and
        // no label (Principles 8, 9).
        let features = percepts.perceive(resources.composition(here));
        // The being's APPETITIVE belief percept for this tick (ideation arc, piece 1, the belief-to-behaviour
        // feedback): its committed reward-belief signal over its affordances, computed by the runner from its
        // own beliefs and written into the controller's appetitive block. Empty when the world does not opt
        // into reward repetition (the layout carries no appetitive block), so the input is byte-identical to
        // before the block existed. Only an evolved appetitive weight turns the signal into repetition.
        let empty_appetitive: Vec<Fixed> = Vec::new();
        let appetite = appetitive.get(&w.id).unwrap_or(&empty_appetitive);
        // The raw MATERIAL-feature vector for the cell the being stands on (the lifetime/demography keystone,
        // pillar 2, trace slice C): the amount of each declared substance in the cell's matter, in registry
        // order. Empty when the world declares no material percepts, so the material block is absent and the
        // input is byte-identical to before the material substrate existed. A pure physical read of the matter
        // here, the opaque signature (no label), the mirror of the biology feature read above (Principles 8, 9).
        let material = material_percepts.perceive(material_field.cell(here));
        // The being's belief-derived ATTRACTION-direction percept for this tick (the lifetime/demography
        // keystone, pillar 2, trace slice C3): the unit direction toward the nearest believed-rewarding
        // material it senses, computed by the runner from its own reward beliefs and written into the
        // controller's dedicated attraction block. Empty when the world does not enable the gradient (the
        // layout carries no attraction block), so the input is byte-identical to before the block existed.
        // Only an evolved attraction weight turns the direction into approach, the mirror of harm avoidance.
        let empty_attraction: Vec<Fixed> = Vec::new();
        let attract = attraction.get(&w.id).unwrap_or(&empty_attraction);
        // The being's own CONVICTION-stance percept for this tick (Prereq B for the learned
        // experience-to-conviction coupling, `docs/working/OWNER_DECISIONS_LOG.md` R2): its signed stance on
        // each exposed conviction axis, computed by the runner from its own intrinsic beliefs and written into
        // the controller's conviction block. Empty when the world exposes no conviction (the layout carries no
        // conviction block), so the input is byte-identical to before the block existed. Only an evolved
        // conviction weight turns a stance into a behaviour bias, so a conviction-biased behaviour emerges.
        let empty_conviction: Vec<Fixed> = Vec::new();
        let convict = conviction.get(&w.id).unwrap_or(&empty_conviction);
        // The being's own BEING-DIRECTED percept for this tick (the being-percept keystone, step 6): its unit
        // avoidance and attraction directions over the beings it perceived, computed by the runner and written
        // into the controller's being block. Empty when the being-percept feature is off (the layout carries no
        // being block), so the input is byte-identical to before the block existed.
        let empty_being: Vec<Fixed> = Vec::new();
        let being_dirs = being.get(&w.id).unwrap_or(&empty_being);
        let input = layout.build_input_full_with_conviction(
            &w.homeostasis,
            &here_axes,
            &dirs,
            signed,
            &features,
            appetite,
            &material,
            attract,
            convict,
            being_dirs,
        );
        let (out, new_hidden) = w.controller.evaluate(&input, &w.hidden);
        w.hidden = new_hidden;
        // A grown body reads its affordances from its own structure's physics directly; a catalog body
        // reads them against the shared organs registry (emergent-anatomy Step 2).
        let afforded = match &w.structure {
            Some(s) => afford.afforded_structure(s, &p.capability_refs, &p.capability_caps),
            None => afford.afforded(&w.body, organs, &p.capability_refs, &p.capability_caps),
        };
        let decision = layout.decide(&out, &afforded);
        // Record the affordance the being acted on this tick, the head of the sequence its reward eligibility
        // trace credits (ideation arc, piece 1, slice 1c). `None` on a tick it took no action; a pure read of
        // its own decision, folded into no hash here (the runner's reward-credit pass reads it).
        w.decided_affordance = decision
            .as_ref()
            .filter(|d| d.activation > Fixed::ZERO)
            .map(|d| d.affordance);
        // The full enacted step (social-learning arc, piece 3): the controller names an affordance but NO
        // target, so its step carries a zero target channel and value; a discovery proposal or a deliberated
        // plan below OVERWRITES this with the target it acted on when it enacts one. Set each tick alongside
        // the primitive so the granular reward credit can key on the target the action was directed at.
        w.decided_step = w.decided_affordance.map(|a| SequenceStep {
            primitive: a.0,
            target_bucket: 0,
            param_bucket: 0,
        });

        let mut exertion = Fixed::ZERO;
        if let Some(d) = decision {
            if d.activation > Fixed::ZERO {
                match d.affordance {
                    MOVE => {
                        let cost = terrain.cost(here);
                        // A grown body's ground speed reads its own strongest grown limb; a catalog body's
                        // reads the mode kind in the shared registry (emergent-anatomy Step 2).
                        let speed = match &w.structure {
                            Some(s) => {
                                locomotion_speed_structure(s, w.body.temperament.activity, cost, p)
                            }
                            None => locomotion_speed(&w.body, organs, cost, p),
                        };
                        // A carried load slows the being (material-substrate arc, cascade item 3): the
                        // per-walker load factor (>= 1) divides the ground speed. It is 1 for an unladen
                        // being (byte-identical) and rises with the fraction of its strength the load
                        // consumes, so an over-laden being moves slower, by physics not by a label.
                        let speed = match load_factors.get(&w.id) {
                            Some(f) if *f > Fixed::ONE => speed.div(*f),
                            _ => speed,
                        };
                        if speed > Fixed::ZERO {
                            let (hx, hy) = d.heading.unwrap_or((Fixed::ZERO, Fixed::ZERO));
                            let mag2 = hx.mul(hx) + hy.mul(hy);
                            let did = if mag2 > HEADING_EPS {
                                walk_dir(w, hx, hy, speed, terrain)
                            } else {
                                // It wants to move but has no known gradient: it explores.
                                explore(w, terrain, speed, p, seed, tick)
                            };
                            if did {
                                moved += 1;
                                exertion = Fixed::ONE;
                            }
                        }
                    }
                    INGEST => {
                        // Take in the matter underfoot, its worth DERIVED FROM PHYSICS not authored AND its
                        // stock DEPLETED (base-level liveliness step 3, R-PHYS-BIO edibility): for each
                        // homeostatic axis backed by a biology-floor class, the being eats the tile's standing
                        // CONTENT of that class and its reserve rises by that content assimilated (its own
                        // per-class assimilation) and passed at the trophic efficiency, CONVERTED through the
                        // being's OWN storage density on the class ([`physiology::physical_intake`], the same
                        // size-scaled reserve bridge the drain uses). So the reserve fills by the food's
                        // PHYSICAL content and not a saturating fill-to-capacity or a biomass fudge, and the
                        // mechanism is ALIEN-CLEAN: it keys on the reserve's backing class and the being's own
                        // composition, never `bio.energy_density`, so a thaumic being fills a mana reserve from
                        // a mana-bearing plant by the SAME call a grazer fills energy from a seed (Principle 9).
                        // The tile loses exactly the content the being takes (conservation-honest), a grazed
                        // tile feeds the next id-ordered being less, and a full reserve draws nothing.
                        let eta = p.ingest_efficiency;
                        match intake_anchors {
                            Some(anchors) => {
                                // The being's own storage density on each class and its mass, its data alone.
                                let body_mass = crate::physiology::body_mass_kg(&w.body, &anchors);
                                let storage = crate::physiology::whole_body_composition_vector(
                                    &w.body, organs,
                                );
                                for axis in &homeo.axes {
                                    if !axis.draw_set.is_empty() {
                                        // A FEEDER RESERVE'S DRAW (R-SOURCE-VECTOR): the per-reserve FOLD
                                        // over its declared draw-axis set (a mixotroph's light-plus-tissue,
                                        // a photovore's flux). No current reserve declares a set, so this
                                        // branch never runs on the pinned scenarios and the four pins hold
                                        // by non-modification of the matter path below. Each term reads its
                                        // source, bridges to content by its own unit, fills bounded by the
                                        // REMAINING room (the live amount() reflects prior terms' ingests),
                                        // and depletes the source only where its floor character is a
                                        // depletable stock, so the deplete step keys on the conservation
                                        // law of the quantity, never on a source kind. The matter singleton
                                        // is the empty-set special case below, kept as today's fast path.
                                        draw_feeder_set(
                                            &axis.draw_set,
                                            axis.id,
                                            here,
                                            resources,
                                            &mut w.homeostasis,
                                            &w.physiology,
                                            &storage,
                                            body_mass,
                                            eta,
                                            p.food_energy_density,
                                        );
                                        continue;
                                    }
                                    let Some(class) = axis.backing_component.as_deref() else {
                                        continue;
                                    };
                                    let supply = resources.supply(here, class);
                                    if supply <= Fixed::ZERO {
                                        continue; // the tile is no source of this axis
                                    }
                                    // The food's PHYSICAL content on the class (CORRECTED-T3), decided PER CLASS.
                                    // Where THIS class's supply on the cell is a real producer-composition
                                    // magnitude, its supply is ALREADY the physical content at the plant's OWN
                                    // per-substance density (the seeding side carries the real magnitude), so eat
                                    // it at `content = supply` directly: the real plant value SUPERSEDES the
                                    // uniform `food_energy_density` anchor, as that anchor's own contract promised.
                                    // Any other class (the always-written water mirror, or a class this cell's
                                    // composition does not carry) keeps the reserved anchor bridge to physical
                                    // content, on a producer cell exactly as on a bare cell (the per-class marker
                                    // is what confines the supersession to the food axes; a per-cell flag would
                                    // wrongly strip the anchor off the water axis too). Skipping the multiply for a
                                    // composition class removes the double-scale; it lowers that axis's absolute
                                    // food scale to the real density, an owner-gated biosphere-balance question
                                    // surfaced, never tuned here.
                                    let content = if resources.is_real_composition(here, class) {
                                        supply
                                    } else {
                                        supply
                                            .checked_mul(p.food_energy_density)
                                            .unwrap_or(Fixed::MAX)
                                    };
                                    let body_c = storage.get(class).copied().unwrap_or(Fixed::ZERO);
                                    let room = w.homeostasis.capacity(axis.id)
                                        - w.homeostasis.amount(axis.id);
                                    let (eaten_content, gain) = crate::physiology::physical_intake(
                                        content,
                                        w.physiology.assimilation(class),
                                        eta,
                                        body_mass,
                                        body_c,
                                        room,
                                    );
                                    if eaten_content <= Fixed::ZERO {
                                        continue;
                                    }
                                    // Remove the standing supply the eaten content came from (content back to
                                    // supply units), so the tile depletes by exactly what the being took. The
                                    // inverse of the content bridge above (CORRECTED-T3), PER CLASS: a
                                    // real-composition class's content IS its supply, so it depletes one-for-one; any
                                    // other class divides back out the food_energy_density anchor.
                                    let eaten_supply = if resources.is_real_composition(here, class)
                                    {
                                        eaten_content
                                    } else {
                                        eaten_content
                                            .checked_div(p.food_energy_density)
                                            .unwrap_or(eaten_content)
                                    };
                                    resources.take(here, class, eaten_supply);
                                    w.homeostasis.ingest(axis.id, gain);
                                }
                            }
                            None => {
                                // No physiology installed: the pre-grounding satisfaction measure stands, so a
                                // fixture without a physiology is byte-identical to before the grounding.
                                for axis in &homeo.axes {
                                    let Some(class) = axis.backing_component.as_deref() else {
                                        continue;
                                    };
                                    let supply = resources.supply(here, class);
                                    if supply <= Fixed::ZERO {
                                        continue;
                                    }
                                    let frac = laws::satisfaction(
                                        supply,
                                        w.physiology.assimilation(class),
                                        w.physiology.requirement(class),
                                    );
                                    let cap = w.homeostasis.capacity(axis.id);
                                    let room = cap - w.homeostasis.amount(axis.id);
                                    let target_gain =
                                        frac.checked_mul(cap).unwrap_or(cap).min(room);
                                    if target_gain <= Fixed::ZERO {
                                        continue;
                                    }
                                    let gross = if eta > Fixed::ZERO {
                                        target_gain.checked_div(eta).unwrap_or(target_gain)
                                    } else {
                                        target_gain
                                    };
                                    let taken = resources.take(here, class, gross);
                                    let gain = taken.checked_mul(eta).unwrap_or(taken);
                                    w.homeostasis.ingest(axis.id, gain);
                                }
                            }
                        }
                        // The tile's toxin classes are NOT a factor in this ingest arm (they neither feed
                        // nor deny a reserve here); they are the environmental-harm sink's concern, applied
                        // once per tick to the CONDITION reserve above (base-level liveliness step 4),
                        // whether or not the being ingests, so exposure harms a being that only passes
                        // through a toxic cell.
                    }
                    GRASP | EXTRACT | GEOPHAGE | CRAFT | DIG | RELEASE | SHELTER | STRIKE => {
                        // The evolved decision to act on matter (material-substrate arc): GRASP picks loose
                        // matter up (item 3, the driver), EXTRACT breaks bonded matter loose in a fracture
                        // contest and takes it (item 4), GEOPHAGE eats the matter underfoot for a reserve that
                        // needs it (item 4, INGEST-FOR-COMPOSITION), CRAFT shapes the carried matter into a
                        // wielded tool (item 4, knapping), DIG and RELEASE lower and raise the terrain (item 5),
                        // SHELTER sets the carried load down overhead as a roof (item 7, the overhead-deposit
                        // technique), STRIKE wounds the Segments occupying the cell with the acting part's
                        // delivered energy (hunt-kill strike arc, the emergent predation payoff). Each records
                        // its decided affordance and activation for the embodiment's post-step enactment pass,
                        // which owns the material field, the carried load, and the registry this function cannot
                        // reach ([`crate::runner::Embodiment::grasp_underfoot`],
                        // [`crate::runner::Embodiment::extract_underfoot`]). Recorded rather than enacted here,
                        // so the decision stays where the evolved controller makes it while the physics stays
                        // where the matter lives. A blank controller expresses zero for these weights, so this
                        // arm never fires for it (the activation would not clear the wins-the-decision bar);
                        // only a being whose weight selection has lifted off zero acts. STRIKE is afforded only
                        // by a body that grows a part reading the PIERCE capability, so a world with no such
                        // body (every run_world scenario) never decides it and is byte-identical.
                        deferred_actions.insert(w.id, (d.affordance, d.activation));
                    }
                    _ => {} // an affordance the engine has no enactment for yet: idle
                }
            }
        }

        // The CONDITION reserve nets this tick's healing against its harm (base-level liveliness step 4),
        // before the metabolism death-check below, so a body worn through its condition floor by exposure
        // dies in the same tick (the emergent reserve-through-floor cull). Healing (a recovery toward
        // full) races the harm: a tolerant being on a salt flat (harm below the recovery) heals faster
        // than it is worn and lives on the gradient, a naive one (harm above the recovery) declines to
        // death, and a being that leaves a toxic cell recovers. The `adjust` clamps to [0, capacity] and
        // is a no-op for a being whose registry carries no CONDITION axis (the thermal-only fixtures), so
        // the sink is inert wherever it does not apply.
        w.homeostasis.adjust(CONDITION, p.condition_recovery - harm);
        // Metabolism drains the reserves every tick (basal, plus the tick's exertion); a being whose
        // reserve falls through its floor dies. When the caller supplies a per-being DERIVED drain
        // (R-METABOLIZE, the anatomy-derived physiology), the drain follows the body's physics through
        // metabolize_derived; otherwise the labelled scalar path over the axis defs' authored drains.
        let alive = match drains.get(&w.id) {
            Some(d) => w.homeostasis.metabolize_derived(homeo, d, exertion),
            None => w.homeostasis.metabolize(homeo, exertion),
        };
        if !alive {
            w.alive = false;
        }
    }
    moved
}

/// The eight headings a searching being can take, unit vectors so a diagonal step covers the same
/// ground as a cardinal one.
fn headings() -> [(Fixed, Fixed); 8] {
    let d = Fixed::from_ratio(7071, 10000); // ~1/sqrt(2)
    let z = Fixed::ZERO;
    let o = Fixed::ONE;
    let n = |v: Fixed| Fixed::ZERO - v;
    [
        (o, z),
        (d, d),
        (z, o),
        (n(d), d),
        (n(o), z),
        (n(d), n(d)),
        (z, n(o)),
        (d, n(d)),
    ]
}

/// Explore: move one step along a heading drawn from counter-based RNG keyed on the being and the
/// exploration period, so the search is a reproducible function of the seed, the being, and the
/// tick, never of the camera. If the drawn heading is blocked, the being rotates through the other
/// headings deterministically and takes the first passable one, so it is not trapped against a wall.
fn explore<T: Terrain>(
    w: &mut Walker,
    terrain: &T,
    speed: Fixed,
    p: &LocomotionParams,
    seed: u64,
    tick: u64,
) -> bool {
    let period = p.explore_persistence.max(1);
    let base = DrawKey::entity(w.id.0, tick / period, Phase::EXPLORE)
        .rng(seed)
        .range_u32(0, 8);
    let dirs = headings();
    for k in 0..8u32 {
        let (dx, dy) = dirs[((base + k) % 8) as usize];
        let nx = w.x + dx.mul(speed);
        let ny = w.y + dy.mul(speed);
        let ncoord = Coord3::ground(floor_i32(nx), floor_i32(ny));
        if terrain.passable(ncoord, &w.body) {
            w.x = nx;
            w.y = ny;
            return true;
        }
    }
    false // hemmed in on every side
}

/// Step a walker one step of `speed` along a heading vector, normalising the heading and entering
/// only a passable tile. Returns whether it moved. A blocked step holds the being in place (routing
/// is Part 13, future).
fn walk_dir<T: Terrain>(w: &mut Walker, hx: Fixed, hy: Fixed, speed: Fixed, terrain: &T) -> bool {
    let mag = (hx.mul(hx) + hy.mul(hy)).sqrt();
    if mag <= Fixed::ZERO {
        return false;
    }
    let ux = hx.div(mag);
    let uy = hy.div(mag);
    let nx = w.x + ux.mul(speed);
    let ny = w.y + uy.mul(speed);
    let ncoord = Coord3::ground(floor_i32(nx), floor_i32(ny));
    if !terrain.passable(ncoord, &w.body) {
        return false;
    }
    w.x = nx;
    w.y = ny;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{BodyPlan, Part, Temperament};
    use crate::controller::ControllerLayout;
    use crate::homeostasis::{
        AffordanceRegistry, HomeostaticAxisDef, HomeostaticRegistry, UnitBridge, ENERGY, WATER,
    };

    const SEED: u64 = 0x10C0;

    struct OpenGround;
    impl Terrain for OpenGround {
        fn passable(&self, _c: Coord3, _b: &BodyPlan) -> bool {
            true
        }
        fn cost(&self, _c: Coord3) -> Fixed {
            Fixed::ONE
        }
    }

    struct Walled;
    impl Terrain for Walled {
        fn passable(&self, c: Coord3, _b: &BodyPlan) -> bool {
            c.x != 5
        }
        fn cost(&self, _c: Coord3) -> Fixed {
            Fixed::ONE
        }
    }

    /// A registry with only a water axis, so movement tests are not confounded by energy starvation
    /// (a labelled test fixture, not owner canon).
    fn water_reg() -> HomeostaticRegistry {
        HomeostaticRegistry {
            axes: vec![HomeostaticAxisDef {
                id: WATER,
                name: "water".to_string(),
                backing_component: Some("bio.water_fraction".to_string()),
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::from_ratio(1, 300),
                exertion_drain: Fixed::from_ratio(1, 400),
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            }],
        }
    }

    fn layout_for(reg: &HomeostaticRegistry) -> ControllerLayout {
        ControllerLayout::new(reg, &AffordanceRegistry::dev_default(), 0)
    }

    /// The biology-floor class the water axis is backed by, in the fixtures.
    const WATER_CLASS: &str = "bio.water_fraction";

    /// A labelled dev-fixture water composition: a tile whose matter carries the given supply on the
    /// water backing class and nothing else. A `water_fraction` of `1/4` reproduces the retired
    /// `intake_yield` fixture (a unit-requirement, unit-assimilation consumer then deposits a quarter
    /// of capacity per bite), so the movement fixtures that do not turn on composition are unchanged.
    fn water_matter(water_fraction: Fixed) -> Composition {
        Composition {
            nutrients: [(WATER_CLASS.to_string(), water_fraction)]
                .into_iter()
                .collect(),
            toxins: BTreeMap::new(),
        }
    }

    /// The standard fixture water tile: a quarter-water composition (the retired-`intake_yield`
    /// equivalent), used where a test only needs a water source and not a specific richness.
    fn water_tile() -> Composition {
        water_matter(Fixed::from_ratio(1, 4))
    }

    /// A taxis controller for a single target axis whose input block starts at `base`: it moves
    /// toward the known source when away from it and ingests the matter underfoot when the reserve is
    /// low. Output layout: [move_act, move_dx, move_dy, ingest_act].
    fn taxis_controller(l: &ControllerLayout, base: usize) -> Controller {
        let n_in = l.n_in();
        let bias = n_in - 1;
        let (lvl, here, dx, dy) = (base, base + 1, base + 2, base + 3);
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        // move_act (output 0): wants to move (bias), suppressed when the source is underfoot.
        w[bias] = Fixed::ONE;
        w[here] = Fixed::from_int(-1);
        // move_dx / move_dy (outputs 1, 2): follow the source direction.
        w[n_in + dx] = Fixed::ONE;
        w[2 * n_in + dy] = Fixed::ONE;
        // ingest_act (output 3): fire when the source is underfoot and the reserve is low.
        w[3 * n_in + here] = Fixed::ONE;
        w[3 * n_in + lvl] = Fixed::from_int(-1);
        Controller::from_weights(n_in, l.n_out(), l.hidden(), w)
    }

    fn mobile_body() -> BodyPlan {
        BodyPlan {
            body_mass: Fixed::from_ratio(1, 2),
            encephalization: Fixed::from_ratio(1, 2),
            diet_breadth: Fixed::from_ratio(1, 2),
            weapons: vec![],
            covering: Part {
                kind: 0,
                development: Fixed::from_ratio(1, 2),
            },
            senses: vec![],
            locomotion: vec![1], // a mobile mode (not the rooted mark 0), so it can walk
            organs: vec![],
            temperament: Temperament {
                boldness: Fixed::from_ratio(1, 2),
                exploration: Fixed::from_ratio(1, 2),
                activity: Fixed::from_ratio(3, 4),
                sociability: Fixed::from_ratio(1, 2),
                aggression: Fixed::from_ratio(1, 4),
            },
        }
    }

    /// A rooted body carries only the rooted mark (kind id 0, which bears no limb geometry, so it reads
    /// no LOCOMOTE capability), so it cannot walk, whatever its kingdom.
    fn rooted_body() -> BodyPlan {
        let mut b = mobile_body();
        b.locomotion = vec![0]; // the rooted mark (kind id 0), which bears no limb geometry
        b
    }

    /// A walking tree: an autotroph body that nonetheless bears a mobile locomotion organ, so it
    /// walks. Mobility is the body, not the kingdom.
    fn walking_tree_body() -> BodyPlan {
        let mut b = mobile_body();
        b.locomotion = vec![3];
        b
    }

    /// A walker with a taxis-for-water controller over the water-only registry, pre-drained so it is
    /// thirsty enough to drink on arrival.
    fn water_walker(
        id: u64,
        tile: Coord3,
        body: BodyPlan,
    ) -> (
        Walker,
        HomeostaticRegistry,
        ControllerLayout,
        AffordanceRegistry,
    ) {
        let reg = water_reg();
        let afford = AffordanceRegistry::dev_default();
        let l = layout_for(&reg);
        let c = taxis_controller(&l, 0); // water is axis 0 in this registry
        let mut homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
        for _ in 0..120 {
            homeo.metabolize(&reg, Fixed::ZERO); // grow thirsty
        }
        let phys = Physiology::dev_for_registry(&reg);
        (
            Walker::new(StableId(id), tile, body, homeo, phys, c),
            reg,
            l,
            afford,
        )
    }

    #[test]
    fn a_rooted_body_never_moves_however_thirsty() {
        let (mut wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), rooted_body());
        wk.learn(WATER, Coord3::ground(2, 0));
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.set(Coord3::ground(2, 0), water_tile());
        let p = LocomotionParams::dev_default();
        let start = ws[0].coord();
        for t in 0..40 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
        }
        assert_eq!(
            ws[0].coord(),
            start,
            "a rooted body stays put whatever its kingdom"
        );
    }

    #[test]
    fn a_walking_tree_walks_because_its_body_can() {
        let (mut wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), walking_tree_body());
        wk.learn(WATER, Coord3::ground(6, 0));
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.set(Coord3::ground(6, 0), water_tile());
        let p = LocomotionParams::dev_default();
        let start = ws[0].coord();
        for t in 0..60 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
        }
        assert_ne!(
            ws[0].coord(),
            start,
            "a walking tree moves: mobility is the body, not the kingdom"
        );
    }

    #[test]
    fn a_being_walks_to_water_it_knows_of_and_drinks() {
        // Walk to a known water tile and drink; and the reserve GAIN from one bite scales with the
        // tile's water composition, because the deposited fraction is the edibility floor's
        // satisfaction over the tile's supply, not an authored constant (R-PHYS-BIO,
        // laws::satisfaction). A richer tile (higher bio.water_fraction) restores more per bite.
        let drink_from = |water_fraction: Fixed| -> (bool, Fixed, Fixed) {
            let (mut wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), mobile_body());
            wk.learn(WATER, Coord3::ground(9, 0)); // it has seen this water before
            let mut ws = vec![wk];
            let mut field = ResourceField::new();
            field.set(Coord3::ground(9, 0), water_matter(water_fraction)); // labelled dev fixture
            let p = LocomotionParams::dev_default();
            for t in 0..80 {
                step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
                if ws[0].coord() == Coord3::ground(9, 0) {
                    // Just arrived, not yet drunk: record the level, take one drink tick, record again.
                    let before_drink = ws[0].homeostasis.level(WATER);
                    step(
                        &mut ws,
                        &reg,
                        &l,
                        &afford,
                        &OpenGround,
                        &field,
                        &p,
                        SEED,
                        100 + t,
                    );
                    let after_drink = ws[0].homeostasis.level(WATER);
                    return (true, before_drink, after_drink);
                }
            }
            (false, Fixed::ZERO, Fixed::ZERO)
        };
        let (reached_poor, before_poor, after_poor) = drink_from(Fixed::from_ratio(1, 10));
        let (reached_rich, before_rich, after_rich) = drink_from(Fixed::from_ratio(4, 10));
        assert!(
            reached_poor && reached_rich,
            "the being walked to the water it knew of"
        );
        assert!(after_poor > before_poor, "and drank, restoring its water");
        // The walk is identical (composition does not affect movement), so the pre-drink levels match
        // and any difference in the post-drink level is the composition-scaled bite.
        assert_eq!(
            before_poor, before_rich,
            "the walk to the tile is unchanged"
        );
        assert!(
            after_rich > after_poor,
            "a richer water tile restores more per bite: the gain scales with the tile's composition"
        );
    }

    #[test]
    fn a_being_does_not_head_for_water_it_has_never_perceived() {
        // Non-omniscience: water sits far, out of sensory range; the being has never seen it, so on
        // its first step it explores rather than making a beeline for water it cannot know of.
        let (wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), mobile_body());
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.set(Coord3::ground(40, 0), water_tile());
        let p = LocomotionParams::dev_default();
        assert!(
            !ws[0].known.contains_key(&WATER),
            "it starts knowing of no water"
        );
        step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, 0);
        assert!(
            ws[0]
                .known
                .get(&WATER)
                .map(|s| s.is_empty())
                .unwrap_or(true),
            "it did not learn of water outside its senses"
        );
        assert!(
            ws[0].coord().x < 5,
            "it did not make a beeline for water it cannot know about"
        );
    }

    #[test]
    fn a_being_discovers_water_by_exploring_then_drinks() {
        // The being knows of no water, but a band of water is reachable. Left to explore, it should
        // come within sensory range of some, learn it, walk to it, and slake its thirst.
        let (wk, reg, l, afford) = water_walker(1, Coord3::ground(4, 4), mobile_body());
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        for x in 6..=10 {
            field.set(Coord3::ground(x, 3), water_tile());
            field.set(Coord3::ground(x, 4), water_tile());
        }
        let p = LocomotionParams::dev_default();
        let mut learned = false;
        let mut drank = false;
        let start_thirst = ws[0].homeostasis.level(WATER);
        for t in 0..600 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
            if ws[0].known.get(&WATER).is_some_and(|s| !s.is_empty()) {
                learned = true;
            }
            if learned && ws[0].homeostasis.level(WATER) > start_thirst {
                drank = true;
                break;
            }
        }
        assert!(
            learned,
            "the being discovered water by exploring, not by reading the map"
        );
        assert!(drank, "and having found it, drank");
    }

    #[test]
    fn perception_is_local_not_global() {
        let (wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), mobile_body());
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.set(Coord3::ground(2, 0), water_tile()); // within sense range of the origin
        field.set(Coord3::ground(40, 0), water_tile()); // far outside it
        let p = LocomotionParams::dev_default();
        step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, 0);
        let known = ws[0].known.get(&WATER).cloned().unwrap_or_default();
        assert!(
            known.contains(&Coord3::ground(2, 0)),
            "it perceived the near water"
        );
        assert!(
            !known.contains(&Coord3::ground(40, 0)),
            "it did not perceive the far water"
        );
    }

    #[test]
    fn a_wall_blocks_a_straight_line_mover() {
        let (mut wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), mobile_body());
        wk.learn(WATER, Coord3::ground(9, 0));
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.set(Coord3::ground(9, 0), water_tile());
        let p = LocomotionParams::dev_default();
        for t in 0..80 {
            step(&mut ws, &reg, &l, &afford, &Walled, &field, &p, SEED, t);
        }
        assert!(
            ws[0].coord().x < 5,
            "the wall stops the straight-line mover short of the water"
        );
    }

    #[test]
    fn locomotion_replays_bit_identically() {
        // One being (id 2) sits on a water Composition tile and drinks each tick; the other (id 1)
        // knows of no water and explores. The run therefore exercises both exploration and ingestion
        // from a Composition tile, and the fingerprint carries the water reserves too, so the replay
        // proves the measured intake is deterministic as well as the movement.
        let run = || {
            let reg = water_reg();
            let afford = AffordanceRegistry::dev_default();
            let l = layout_for(&reg);
            let c = taxis_controller(&l, 0);
            let mut field = ResourceField::new();
            for x in 6..=10 {
                field.set(Coord3::ground(x, 3), water_tile());
            }
            let mk = |id: u64, tile: Coord3, knows_water: bool| {
                let mut h = Homeostasis::from_mass(&reg, Fixed::ONE);
                for _ in 0..80 {
                    h.metabolize(&reg, Fixed::ZERO);
                }
                let phys = Physiology::dev_for_registry(&reg);
                let mut w = Walker::new(StableId(id), tile, mobile_body(), h, phys, c.clone());
                if knows_water {
                    w.learn(WATER, Coord3::ground(8, 3));
                }
                w
            };
            let mut ws = vec![
                mk(2, Coord3::ground(8, 3), true), // starts on water, drinks in place
                mk(1, Coord3::ground(1, 6), false), // knows nothing, explores
            ];
            let p = LocomotionParams::dev_default();
            for t in 0..80 {
                step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
            }
            // After the id-order sort ws[0] is id 1 (the explorer), ws[1] is id 2 (the drinker).
            (
                ws[0].x.to_bits(),
                ws[0].y.to_bits(),
                ws[1].x.to_bits(),
                ws[1].y.to_bits(),
                ws[0].homeostasis.amount(WATER).to_bits(),
                ws[1].homeostasis.amount(WATER).to_bits(),
                ws[1].homeostasis.level(WATER),
            )
        };
        let first = run();
        assert_eq!(
            first,
            run(),
            "the same setup, including exploration and ingestion, replays bit for bit"
        );
        assert!(
            first.6 > Fixed::from_ratio(3, 4),
            "the being on the water tile drank: its reserve stayed above three-quarters, which 80 ticks of pure drain from a pre-drained start could not"
        );
    }

    #[test]
    fn metabolism_kills_an_unfed_being() {
        // With the real dev registry (energy and water) and no sources anywhere, a being that never
        // eats or drinks eventually dies: survival is a physical fact, the fitness Stage 3 selects on.
        let reg = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let l = ControllerLayout::new(&reg, &afford, 0);
        let c = taxis_controller(&l, 4); // water block starts at input 4 in the two-axis layout
        let homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
        let phys = Physiology::dev_for_registry(&reg);
        let mut ws = vec![Walker::new(
            StableId(1),
            Coord3::ground(0, 0),
            mobile_body(),
            homeo,
            phys,
            c,
        )];
        let field = ResourceField::new(); // barren
        let p = LocomotionParams::dev_default();
        let mut died_at = None;
        for t in 0..100_000 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
            if !ws[0].alive {
                died_at = Some(t);
                break;
            }
        }
        assert!(died_at.is_some(), "unfed and unwatered, the being dies");
    }

    /// A locomotion mode kind carrying a section modulus, arm length, and yield strength, so a test can
    /// vary a limb's strength (its section) at a fixed stride (its arm length) and read the effect on
    /// speed. The values are `(section_modulus, arm_length, yield_strength)` as fixed-point.
    fn limb_kind(
        id: u16,
        section: Fixed,
        arm: Fixed,
        yield_strength: Fixed,
    ) -> crate::anatomy::KindDef {
        let mut geometry = BTreeMap::new();
        geometry.insert("mech.section_modulus".to_string(), section);
        geometry.insert("mech.arm_length".to_string(), arm);
        let mut material = BTreeMap::new();
        material.insert("mat.yield_strength".to_string(), yield_strength);
        crate::anatomy::KindDef {
            id,
            name: format!("limb{id}"),
            fantasy: false,
            geometry,
            material,
        }
    }

    /// A registry whose locomotion modes are a stout limb (id 1) and a slender near-yield limb (id 2) of
    /// EQUAL stride (arm length 0.3 m), so a test isolates the limb's structural strength from its length.
    fn strength_registry() -> BodyPlanRegistry {
        let mut reg = BodyPlanRegistry::dev_default();
        reg.locomotion = vec![
            // The rooted mark (kind id 0): no limb geometry, reads no LOCOMOTE capability.
            crate::anatomy::KindDef {
                id: 0,
                name: "rooted".to_string(),
                fantasy: false,
                geometry: BTreeMap::new(),
                material: BTreeMap::new(),
            },
            // A stout limb: a large section modulus, so the reference load raises little bending stress
            // and the LOCOMOTE capability is near one.
            limb_kind(
                1,
                Fixed::from_ratio(1, 10_000),
                Fixed::from_ratio(3, 10),
                Fixed::from_int(150),
            ),
            // A slender limb of the SAME length but a far smaller section, so the same load raises a
            // bending stress near yield and the capability is low: a weaker locomotor.
            limb_kind(
                2,
                Fixed::from_ratio(1, 5_000_000),
                Fixed::from_ratio(3, 10),
                Fixed::from_int(150),
            ),
        ];
        reg
    }

    #[test]
    fn a_stronger_limbed_body_moves_faster_at_equal_stride() {
        // Ground speed now reads the grown limb, not body mass: at an IDENTICAL stride (arm length), the
        // stouter limb reads a greater LOCOMOTE capability (its section bears the propulsive load far from
        // yield) and so pushes the body off faster. The mass power law is retired for a physics read of
        // the limb, so a strong-limbed lineage disperses faster than a weak-limbed one.
        let organs = strength_registry();
        let p = LocomotionParams::dev_default();
        let mut strong = mobile_body();
        strong.locomotion = vec![1]; // the stout limb
        let mut weak = mobile_body();
        weak.locomotion = vec![2]; // the slender limb, same stride
        let vstr = locomotion_speed(&strong, &organs, Fixed::ONE, &p);
        let vw = locomotion_speed(&weak, &organs, Fixed::ONE, &p);
        assert!(
            vstr > vw,
            "the stronger limb moves the body faster at equal stride ({vstr:?} > {vw:?})"
        );
        assert!(
            vw > Fixed::ZERO,
            "the slender limb still bears its load and moves"
        );
    }

    #[test]
    fn a_carried_load_slows_a_moving_being() {
        // Material-substrate item 3, the carried-load locomotion penalty: two identical exploring beings
        // draw the SAME heading sequence (same id and seed), so the only difference is the step SIZE. The
        // being whose load factor exceeds one covers less ground, because the factor divides its ground
        // speed; an empty or unit load factor leaves the walk byte-identical (the opt-out).
        let organs = strength_registry();
        let reg = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let l = ControllerLayout::new(&reg, &afford, 0);
        let p = LocomotionParams::dev_default();
        let n_in = l.n_in();
        let mut wts = vec![Fixed::ZERO; l.weight_count()];
        wts[n_in - 1] = Fixed::ONE; // move_act bias positive, no directional weights: it explores
        let mover = Controller::from_weights(n_in, l.n_out(), l.hidden(), wts);
        let disperse = |factor: Option<Fixed>| -> Fixed {
            let mut body = mobile_body();
            body.locomotion = vec![1];
            let homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
            let phys = Physiology::dev_for_registry(&reg);
            let mut ws = vec![Walker::new(
                StableId(1),
                Coord3::ground(0, 0),
                body,
                homeo,
                phys,
                mover.clone(),
            )];
            let mut field = ResourceField::new();
            let load_factors: BTreeMap<StableId, Fixed> = match factor {
                Some(f) => [(StableId(1), f)].into_iter().collect(),
                None => BTreeMap::new(),
            };
            for t in 0..15u64 {
                step_with_field_dirs(
                    &mut ws,
                    &reg,
                    &l,
                    &afford,
                    &organs,
                    &OpenGround,
                    &mut field,
                    &p,
                    7,
                    t,
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    None,
                    &PerceptRegistry::empty(),
                    &crate::material_percept::MaterialPerceptRegistry::empty(),
                    &crate::material::MaterialField::new(),
                    &load_factors,
                    &BTreeMap::new(), // no appetitive block on the fixture path
                    &BTreeMap::new(), // no attraction block on the fixture path
                    &BTreeMap::new(), // no conviction block on the fixture path
                    &BTreeMap::new(), // no being block on the fixture path
                    &mut BTreeMap::new(),
                );
            }
            let (dx, dy) = (
                ws[0].x - Fixed::from_ratio(1, 2),
                ws[0].y - Fixed::from_ratio(1, 2),
            );
            dx.mul(dx) + dy.mul(dy)
        };
        let unladen = disperse(None);
        let laden = disperse(Some(Fixed::from_int(3))); // divisor 3: a third the ground speed
        assert!(
            laden < unladen,
            "a laden being disperses less than an unladen one ({laden:?} < {unladen:?})"
        );
        assert!(unladen > Fixed::ZERO, "the unladen being does move");
        // A unit load factor is below the penalty threshold, so it leaves the walk byte-identical (the
        // opt-out: an unladen being, whose factor map is empty, is never slowed).
        assert_eq!(
            disperse(None),
            disperse(Some(Fixed::ONE)),
            "a load factor of one leaves the walk unchanged"
        );
    }

    #[test]
    fn a_strong_limbed_lineage_disperses_faster_than_a_weak_limbed_one() {
        // The blind concept-verification on the run-path locomotion step: two beings identical but for
        // their limb's structural strength (the stout section vs the slender near-yield one, at EQUAL
        // stride), each driven by a controller that wants to move but knows of no source, so it explores.
        // Given the same id and seed they draw the IDENTICAL sequence of exploration headings, so the only
        // difference is the step SIZE, the grown-limb speed. The stouter limb reads a greater LOCOMOTE
        // capability, pushes off faster, and ends farther from the origin: with the sqrt(body_mass) proxy
        // retired, movement speed is a physics read of the limb, so a strong-limbed lineage disperses
        // faster, and the property manifests in the run, not only in the pure-speed unit read above.
        let organs = strength_registry();
        let reg = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let l = ControllerLayout::new(&reg, &afford, 0);
        let p = LocomotionParams::dev_default();
        // A controller that always wants to move but authors no heading, so it explores every tick.
        let n_in = l.n_in();
        let mut wts = vec![Fixed::ZERO; l.weight_count()];
        wts[n_in - 1] = Fixed::ONE; // move_act bias positive; no directional weights
        let mover = Controller::from_weights(n_in, l.n_out(), l.hidden(), wts);
        let disperse2 = |mode: u16| -> Fixed {
            let mut body = mobile_body();
            body.locomotion = vec![mode];
            let homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
            let phys = Physiology::dev_for_registry(&reg);
            let mut ws = vec![Walker::new(
                StableId(1),
                Coord3::ground(0, 0),
                body,
                homeo,
                phys,
                mover.clone(),
            )];
            let mut field = ResourceField::new(); // no source: the being explores
            for t in 0..15u64 {
                step_with_field_dirs(
                    &mut ws,
                    &reg,
                    &l,
                    &afford,
                    &organs,
                    &OpenGround,
                    &mut field,
                    &p,
                    SEED,
                    t,
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    None,
                    &crate::percept::PerceptRegistry::empty(),
                    &crate::material_percept::MaterialPerceptRegistry::empty(),
                    &crate::material::MaterialField::new(),
                    &std::collections::BTreeMap::new(),
                    &std::collections::BTreeMap::new(), // no appetitive block on the fixture path
                    &std::collections::BTreeMap::new(), // no attraction block on the fixture path
                    &std::collections::BTreeMap::new(), // no conviction block on the fixture path
                    &std::collections::BTreeMap::new(), // no being block on the fixture path
                    &mut std::collections::BTreeMap::new(),
                );
            }
            let (x, y) = (ws[0].x, ws[0].y);
            x.mul(x) + y.mul(y) // squared displacement from the origin
        };
        let stout = disperse2(1);
        let slender = disperse2(2);
        assert!(
            stout > slender,
            "the strong-limbed being disperses farther from the origin ({stout:?} > {slender:?})"
        );
    }

    #[test]
    fn difficult_terrain_slows_a_body() {
        let organs = BodyPlanRegistry::dev_default();
        let p = LocomotionParams::dev_default();
        let body = mobile_body();
        let open = locomotion_speed(&body, &organs, Fixed::ONE, &p);
        let rough = locomotion_speed(&body, &organs, Fixed::from_int(3), &p);
        assert!(rough < open, "costlier ground slows the body");
    }

    #[test]
    fn the_run_reads_a_grown_structure_when_the_walker_carries_one() {
        // The Step-2 run wiring (slice B2a): when a walker carries a GROWN structure, the run reads its
        // affordances and speed from the grown segments' physics, NOT from the catalog `body`. A being whose
        // catalog body is a walker but whose grown structure is rooted does not move; one whose grown
        // structure bears a limb does. The grown body governs the run, by physics not by the catalog.
        use crate::morphogen::{grow, MorphogenProgram};
        let program = MorphogenProgram::dev_default();
        let mut limbed_params = vec![Fixed::ZERO; program.param_count()];
        limbed_params[1] = Fixed::from_ratio(1, 2); // section_modulus fraction
        limbed_params[2] = Fixed::from_ratio(2, 5); // arm_length fraction
        limbed_params[9] = Fixed::from_ratio(3, 4); // yield_strength fraction
        let limbed = grow(&program, &limbed_params, 0x1, StableId(1));
        let rooted = grow(
            &program,
            &vec![Fixed::ZERO; program.param_count()],
            0x1,
            StableId(1),
        );

        let reg = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let l = ControllerLayout::new(&reg, &afford, 0);
        let n_in = l.n_in();
        let mut wts = vec![Fixed::ZERO; l.weight_count()];
        wts[n_in - 1] = Fixed::ONE; // move_act bias positive: it explores every tick
        let mover = Controller::from_weights(n_in, l.n_out(), l.hidden(), wts);
        let p = LocomotionParams::dev_default();
        let organs = BodyPlanRegistry::dev_default();

        let run = |structure: Structure| -> Coord3 {
            let homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
            let phys = Physiology::dev_for_registry(&reg);
            // The catalog body is a mobile walker (locomotion mode 1); the grown structure overrides it.
            let walker = Walker::new(
                StableId(1),
                Coord3::ground(0, 0),
                mobile_body(),
                homeo,
                phys,
                mover.clone(),
            )
            .with_structure(structure);
            let mut ws = vec![walker];
            let mut field = ResourceField::new();
            for t in 0..10u64 {
                step_with_field_dirs(
                    &mut ws,
                    &reg,
                    &l,
                    &afford,
                    &organs,
                    &OpenGround,
                    &mut field,
                    &p,
                    SEED,
                    t,
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    None,
                    &crate::percept::PerceptRegistry::empty(),
                    &crate::material_percept::MaterialPerceptRegistry::empty(),
                    &crate::material::MaterialField::new(),
                    &std::collections::BTreeMap::new(),
                    &std::collections::BTreeMap::new(), // no appetitive block on the fixture path
                    &std::collections::BTreeMap::new(), // no attraction block on the fixture path
                    &std::collections::BTreeMap::new(), // no conviction block on the fixture path
                    &std::collections::BTreeMap::new(), // no being block on the fixture path
                    &mut std::collections::BTreeMap::new(),
                );
            }
            ws[0].coord()
        };
        assert_ne!(
            run(limbed),
            Coord3::ground(0, 0),
            "a walker carrying a grown limbed structure moves, reading MOVE from the grown limb"
        );
        assert_eq!(
            run(rooted),
            Coord3::ground(0, 0),
            "a walker carrying a grown rooted structure stays put, though its catalog body is a walker"
        );
    }

    #[test]
    fn energy_and_water_both_being_sought_is_the_next_layer() {
        // A sanity check that a two-axis being can be constructed and stepped without panic; the
        // full two-need forage loop is what selection (Stage 3) and the recurrent controller
        // (Stage 4) bring.
        let reg = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let l = ControllerLayout::new(&reg, &afford, 0);
        let c = Controller::zeros(&l);
        let homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
        let phys = Physiology::dev_for_registry(&reg);
        let mut ws = vec![Walker::new(
            StableId(1),
            Coord3::ground(0, 0),
            mobile_body(),
            homeo,
            phys,
            c,
        )];
        let field = ResourceField::new();
        let p = LocomotionParams::dev_default();
        for t in 0..10 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
        }
        assert!(
            ws[0].alive,
            "a two-axis being steps without dying over a short unfed horizon"
        );
        assert_eq!(
            reg.axes.len(),
            2,
            "the dev registry carries both energy and water axes"
        );
        let _ = (ENERGY, WATER);
    }

    #[test]
    fn a_tile_is_a_source_only_where_its_composition_carries_the_backing_component() {
        // Source-of-an-axis is DERIVED, never stored: a tile is a water source exactly when its
        // composition carries a nonzero supply on the water axis's backing class, read against the
        // registry. Nothing tags a tile "water"; the pairing emerges from the composition and the
        // registry's backing classes (Principles 9 and 11).
        let reg = water_reg();
        let mut field = ResourceField::new();
        let wet = Coord3::ground(1, 1);
        let dry = Coord3::ground(2, 2);
        field.set(wet, water_matter(Fixed::from_ratio(1, 2)));
        // A tile whose composition carries only a class no registered axis is backed by is no source.
        field.set(
            dry,
            Composition {
                nutrients: [("bio.energy_density".to_string(), Fixed::ONE)]
                    .into_iter()
                    .collect(),
                toxins: BTreeMap::new(),
            },
        );
        assert!(
            field.source(WATER, wet, &reg),
            "the wet tile carries the water backing class, so it is a water source"
        );
        assert!(
            !field.source(WATER, dry, &reg),
            "the energy-only tile carries no water, so it is not a water source"
        );
        assert!(
            !field.source(WATER, Coord3::ground(9, 9), &reg),
            "an empty tile is no source"
        );
        assert_eq!(
            field.axes_here(wet, &reg),
            vec![WATER],
            "the wet tile affords the water axis"
        );
        assert!(
            field.axes_here(dry, &reg).is_empty(),
            "the energy-only tile affords no registered (water) axis"
        );
        assert_eq!(
            field.axes(&reg),
            vec![WATER],
            "the field carries a water source somewhere"
        );
    }

    #[test]
    fn a_present_zero_backing_component_is_not_a_source() {
        // Presence is a NONZERO supply; a present zero is absence (the substrate convention), so a
        // tile carrying bio.water_fraction = 0 is not a water source.
        let reg = water_reg();
        let mut field = ResourceField::new();
        let c = Coord3::ground(0, 0);
        field.set(c, water_matter(Fixed::ZERO));
        assert!(
            !field.source(WATER, c, &reg),
            "a present-zero water supply is not a source"
        );
        assert!(field.axes_here(c, &reg).is_empty());
        assert!(field.axes(&reg).is_empty());
    }

    #[test]
    fn two_physiologies_ingest_differently_from_one_identical_tile() {
        // THE NON-STEERING TEST. Two beings with distinct physiology stand on ONE identical water
        // tile, both thirsty. They end the tick with different water reserves purely from their own
        // physiology (their per-class requirement over the tile's supply through laws::satisfaction),
        // never from any race, species, or kind identifier: the ingest arm reads only homeo.axes, the
        // backing-class strings, the tile composition, and each being's own physiology (Principle 9).
        // The tile is deliberately RICH relative to a small being's appetite (step-3 depletion): the
        // physiology difference (the grossed-up satisfaction) shows only when the tile can supply more
        // than the being's bite, otherwise both strip a scarce tile to the same floor (still identity-
        // free). A small-appetite grazer on a rich tile is the non-limiting per-bite regime.
        let reg = water_reg();
        let afford = AffordanceRegistry::dev_default();
        let l = layout_for(&reg);
        let c = taxis_controller(&l, 0);
        let tile = Coord3::ground(0, 0);
        // A supply that out-supplies a small being's grossed-up bite, so the requirement difference (not
        // the tile) is the binding constraint and neither reserve saturates in a tick.
        let mut field = ResourceField::new();
        field.set(tile, water_matter(Fixed::from_ratio(1, 4)));

        // Two consumers differing ONLY in their water requirement: an efficient one (low requirement,
        // high satisfaction) and a demanding one (high requirement, low satisfaction). Assimilation is
        // the labelled unit dev fixture in both. Their reserves are small (a fifth of unit mass), so the
        // rich tile out-supplies their bite and the physiology, not the tile, sets the intake.
        let mk = |req: Fixed| {
            let mut homeo = Homeostasis::from_mass(&reg, Fixed::from_ratio(1, 5));
            for _ in 0..200 {
                homeo.metabolize(&reg, Fixed::ZERO); // grow thirsty enough to drink, not die
            }
            let phys = Physiology {
                requirements: [(WATER_CLASS.to_string(), req)].into_iter().collect(),
                assimilation: [(WATER_CLASS.to_string(), Fixed::ONE)]
                    .into_iter()
                    .collect(),
                tolerances: BTreeMap::new(),
                hill: BTreeMap::new(),
            };
            let mut wk = Walker::new(StableId(1), tile, mobile_body(), homeo, phys, c.clone());
            wk.learn(WATER, tile);
            vec![wk]
        };
        let mut efficient = mk(Fixed::from_ratio(1, 2)); // 0.25 supply / 0.5 req -> satisfaction 0.5
        let mut demanding = mk(Fixed::ONE); //             0.25 supply / 1.0 req -> satisfaction 0.25
        let p = LocomotionParams::dev_default();
        step(
            &mut efficient,
            &reg,
            &l,
            &afford,
            &OpenGround,
            &field,
            &p,
            SEED,
            0,
        );
        step(
            &mut demanding,
            &reg,
            &l,
            &afford,
            &OpenGround,
            &field,
            &p,
            SEED,
            0,
        );

        let e = efficient[0].homeostasis.level(WATER);
        let d = demanding[0].homeostasis.level(WATER);
        assert!(
            e > d,
            "the efficient consumer (lower requirement) restores more from the identical tile than the demanding one, purely from its own physiology: {e:?} vs {d:?}"
        );
    }

    /// A one-reserve homeostasis drained to leave ample room, for the R-SOURCE-VECTOR fold fixtures.
    fn drained_energy_reserve() -> (HomeostaticRegistry, Homeostasis) {
        let reg = HomeostaticRegistry {
            axes: vec![HomeostaticAxisDef {
                id: ENERGY,
                name: "energy".to_string(),
                backing_component: Some("bio.energy_density".to_string()),
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::from_ratio(1, 400),
                exertion_drain: Fixed::from_ratio(1, 100),
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            }],
        };
        let mut homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
        for _ in 0..200 {
            homeo.metabolize(&reg, Fixed::ZERO); // open ample room without dying
        }
        (reg, homeo)
    }

    #[test]
    fn mixotrophy_folds_two_source_axes_into_one_reserve_with_zero_mechanism_code() {
        // R-SOURCE-VECTOR HARD-GATE ACCEPTANCE (a): a mixotroph draws ONE reserve from TWO source axes
        // (a photo-and-chemo feeder, here two composition classes). The per-reserve fold sums both bites
        // into the reserve and depletes both, through `draw_feeder_set` exactly as a matter singleton
        // would, differing ONLY in that the draw set has two entries. No branch on any source kind: a
        // mixotroph costs zero mechanism code, a data row of two terms.
        let (_reg, mut homeo) = drained_energy_reserve();
        let class_a = "bio.energy_density";
        let class_b = "bio.protein_fraction";
        let tile = Coord3::ground(0, 0);
        let mut field = ResourceField::new();
        field.set(
            tile,
            Composition {
                nutrients: [
                    (class_a.to_string(), Fixed::from_ratio(1, 4)),
                    (class_b.to_string(), Fixed::from_ratio(1, 4)),
                ]
                .into_iter()
                .collect(),
                toxins: BTreeMap::new(),
            },
        );
        let phys = Physiology {
            requirements: BTreeMap::new(),
            assimilation: [
                (class_a.to_string(), Fixed::ONE),
                (class_b.to_string(), Fixed::ONE),
            ]
            .into_iter()
            .collect(),
            tolerances: BTreeMap::new(),
            hill: BTreeMap::new(),
        };
        // A large storage density on each class makes each bite small relative to the room, so neither
        // term saturates the reserve and both contribute (the fold's sum is visible, not a single bite).
        let storage: BTreeMap<String, Fixed> = [
            (class_a.to_string(), Fixed::from_int(10)),
            (class_b.to_string(), Fixed::from_int(10)),
        ]
        .into_iter()
        .collect();
        let body_mass = Fixed::ONE;
        let eta = Fixed::ONE;
        let fed = Fixed::ONE; // a unit bridge, so content == supply and the bite arithmetic is legible
        let draw_set = vec![
            DrawTerm {
                class: class_a.to_string(),
                unit_bridge: UnitBridge::MatterPerBeing,
                depletion: DepletionCharacter::DepletableStock,
            },
            DrawTerm {
                class: class_b.to_string(),
                unit_bridge: UnitBridge::MatterPerBeing,
                depletion: DepletionCharacter::DepletableStock,
            },
        ];
        // The expected total is the two terms' independent bites with the fold's room threading (the
        // second term sees the room the first left).
        let room0 = homeo.capacity(ENERGY) - homeo.amount(ENERGY);
        let (_, gain_a) = crate::physiology::physical_intake(
            field.supply(tile, class_a),
            phys.assimilation(class_a),
            eta,
            body_mass,
            storage[class_a],
            room0,
        );
        let (_, gain_b) = crate::physiology::physical_intake(
            field.supply(tile, class_b),
            phys.assimilation(class_b),
            eta,
            body_mass,
            storage[class_b],
            room0 - gain_a,
        );
        assert!(
            room0 > gain_a + gain_b,
            "the fixture must leave room for both terms, else the sum is not observable"
        );
        let before = homeo.amount(ENERGY);
        let supply_a0 = field.supply(tile, class_a);
        let supply_b0 = field.supply(tile, class_b);
        draw_feeder_set(
            &draw_set, ENERGY, tile, &mut field, &mut homeo, &phys, &storage, body_mass, eta, fed,
        );
        // The one reserve rose by the SUM of the two terms' bites: mixotrophy IS the fold, no new branch.
        assert_eq!(homeo.amount(ENERGY) - before, gain_a + gain_b);
        assert!(
            gain_a > Fixed::ZERO && gain_b > Fixed::ZERO,
            "both terms contributed"
        );
        // Both source axes depleted (each a depletable stock, the conservation law read off the axis).
        assert!(field.supply(tile, class_a) < supply_a0, "class A depleted");
        assert!(field.supply(tile, class_b) < supply_b0, "class B depleted");
    }

    #[test]
    fn a_flux_feeder_draws_without_depleting_its_source() {
        // R-SOURCE-VECTOR HARD-GATE ACCEPTANCE (b), the draw side: a photovore or gravity-gradient feeder
        // draws a NON-RIVALROUS FLUX. The reserve gains but the source is NOT depleted, because the fold's
        // deplete step keys on the axis's floor character (a flux), never on a source kind. The flux is a
        // data row; adding it edits no code in the fold.
        let (_reg, mut homeo) = drained_energy_reserve();
        let flux = "field.gravity_gradient";
        let tile = Coord3::ground(0, 0);
        let mut field = ResourceField::new();
        field.set(
            tile,
            Composition {
                nutrients: [(flux.to_string(), Fixed::from_ratio(1, 4))]
                    .into_iter()
                    .collect(),
                toxins: BTreeMap::new(),
            },
        );
        let phys = Physiology {
            requirements: BTreeMap::new(),
            assimilation: [(flux.to_string(), Fixed::ONE)].into_iter().collect(),
            tolerances: BTreeMap::new(),
            hill: BTreeMap::new(),
        };
        let storage: BTreeMap<String, Fixed> = [(flux.to_string(), Fixed::from_int(10))]
            .into_iter()
            .collect();
        let draw_set = vec![DrawTerm {
            class: flux.to_string(),
            unit_bridge: UnitBridge::MatterPerBeing,
            depletion: DepletionCharacter::NonRivalrousFlux,
        }];
        let before = homeo.amount(ENERGY);
        let supply0 = field.supply(tile, flux);
        draw_feeder_set(
            &draw_set,
            ENERGY,
            tile,
            &mut field,
            &mut homeo,
            &phys,
            &storage,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE,
        );
        assert!(
            homeo.amount(ENERGY) > before,
            "the flux feeder gained from its source"
        );
        assert_eq!(
            field.supply(tile, flux),
            supply0,
            "a non-rivalrous flux is NOT depleted by the draw (the character read, not a source kind)"
        );
    }

    /// A draw term reading a source axis of the given depletion character, for the fail-loud fold tests.
    fn feeder_field_for(class: &str) -> (ResourceField, Physiology, BTreeMap<String, Fixed>) {
        let tile = Coord3::ground(0, 0);
        let mut field = ResourceField::new();
        field.set(
            tile,
            Composition {
                nutrients: [(class.to_string(), Fixed::from_ratio(1, 4))]
                    .into_iter()
                    .collect(),
                toxins: BTreeMap::new(),
            },
        );
        let phys = Physiology {
            requirements: BTreeMap::new(),
            assimilation: [(class.to_string(), Fixed::ONE)].into_iter().collect(),
            tolerances: BTreeMap::new(),
            hill: BTreeMap::new(),
        };
        let storage: BTreeMap<String, Fixed> = [(class.to_string(), Fixed::from_int(10))]
            .into_iter()
            .collect();
        (field, phys, storage)
    }

    #[test]
    #[should_panic(expected = "undeclared")]
    fn a_draw_on_an_undeclared_reserved_character_fails_loud() {
        // R-SOURCE-VECTOR reserved-value discipline (the section-9 audit's catch): a draw term whose source
        // axis carries an UNDECLARED (Reserved) conservation character must FAIL LOUD in the fold, never
        // silently ingest a reserve against an unset value. The behavioral counterpart to the physics
        // storage test that the sentinel is carried.
        let (_reg, mut homeo) = drained_energy_reserve();
        let class = "field.undeclared";
        let (mut field, phys, storage) = feeder_field_for(class);
        let draw_set = vec![DrawTerm {
            class: class.to_string(),
            unit_bridge: UnitBridge::MatterPerBeing,
            depletion: DepletionCharacter::Reserved {
                basis: "test: never declared".to_string(),
            },
        }];
        draw_feeder_set(
            &draw_set,
            ENERGY,
            Coord3::ground(0, 0),
            &mut field,
            &mut homeo,
            &phys,
            &storage,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE,
        );
    }

    #[test]
    #[should_panic(expected = "reservoir")]
    fn a_reservoir_draw_fails_loud_until_wired() {
        // A reservoir character is declared but the reservoir pool draw is not yet wired: the fold fails
        // loud rather than silently gaining against a character it does not model (the same discipline).
        let (_reg, mut homeo) = drained_energy_reserve();
        let class = "field.reservoir";
        let (mut field, phys, storage) = feeder_field_for(class);
        let draw_set = vec![DrawTerm {
            class: class.to_string(),
            unit_bridge: UnitBridge::MatterPerBeing,
            depletion: DepletionCharacter::Reservoir,
        }];
        draw_feeder_set(
            &draw_set,
            ENERGY,
            Coord3::ground(0, 0),
            &mut field,
            &mut homeo,
            &phys,
            &storage,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE,
        );
    }

    #[test]
    fn grazing_depletes_the_tile_and_competition_is_the_id_sorted_walk() {
        // Base-level liveliness step 3: the run-path ingest (step_with_field_dirs with a &mut resource
        // field) DEPLETES the standing supply, so grazing draws the stock down and the id-sorted walk
        // makes it deterministic competition with no new randomness. Two thirsty beings on one modest
        // water tile eat in id order; the first draws the tile down (here to empty), the second finds
        // less (here none), so the first ends with the larger reserve and the tile's supply has fallen.
        let reg = water_reg();
        let afford = AffordanceRegistry::dev_default();
        let l = layout_for(&reg);
        let c = taxis_controller(&l, 0);
        let tile = Coord3::ground(0, 0);
        let start_supply = Fixed::from_ratio(1, 4);
        let mut field = ResourceField::new();
        field.set(tile, water_matter(start_supply));

        let mk = |id: u64| {
            let mut homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
            for _ in 0..200 {
                homeo.metabolize(&reg, Fixed::ZERO); // thirsty enough to drink, not dead
            }
            let phys = Physiology::dev_for_registry(&reg);
            let mut wk = Walker::new(StableId(id), tile, mobile_body(), homeo, phys, c.clone());
            wk.learn(WATER, tile);
            wk
        };
        // Supplied out of id order on purpose: the step sorts by id, so being 1 eats before being 2.
        let mut ws = vec![mk(2), mk(1)];
        let empty_dirs = BTreeMap::new();
        let empty_signed = BTreeMap::new();
        let empty_drains = BTreeMap::new();
        step_with_field_dirs(
            &mut ws,
            &reg,
            &l,
            &afford,
            &BodyPlanRegistry::dev_default(),
            &OpenGround,
            &mut field,
            &LocomotionParams::dev_default(),
            SEED,
            0,
            &empty_dirs,
            &empty_signed,
            &empty_drains,
            None,
            &crate::percept::PerceptRegistry::empty(),
            &crate::material_percept::MaterialPerceptRegistry::empty(),
            &crate::material::MaterialField::new(),
            &std::collections::BTreeMap::new(),
            &std::collections::BTreeMap::new(), // no appetitive block on the fixture path
            &std::collections::BTreeMap::new(), // no attraction block on the fixture path
            &std::collections::BTreeMap::new(), // no conviction block on the fixture path
            &std::collections::BTreeMap::new(), // no being block on the fixture path
            &mut std::collections::BTreeMap::new(),
        );

        let after = field.supply(tile, WATER_CLASS);
        assert!(
            after < start_supply,
            "grazing depleted the tile's standing supply: {after:?} < {start_supply:?}"
        );
        let level_of = |id: u64| {
            ws.iter()
                .find(|w| w.id == StableId(id))
                .unwrap()
                .homeostasis
                .level(WATER)
        };
        assert!(
            level_of(1) > level_of(2),
            "the first-id being ate before the second saw the depleted tile: {:?} > {:?}",
            level_of(1),
            level_of(2)
        );
    }

    /// A registry carrying only the CONDITION reserve (base-level liveliness step 4), so the salt-harm
    /// sink is exercised without a metabolic-starvation confound: the only way to die is the environmental
    /// harm wearing CONDITION through its floor.
    fn condition_reg() -> HomeostaticRegistry {
        HomeostaticRegistry {
            axes: vec![HomeostaticAxisDef {
                id: CONDITION,
                name: "condition".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            }],
        }
    }

    /// A physiology carrying a salinity tolerance of the given magnitude (Hill exponent two), and no
    /// nutrient requirements, so it neither eats nor starves in the harm test.
    fn salt_physiology(tolerance: Fixed) -> Physiology {
        Physiology {
            requirements: BTreeMap::new(),
            assimilation: BTreeMap::new(),
            tolerances: [(crate::physiology::SALINITY.to_string(), tolerance)]
                .into_iter()
                .collect(),
            hill: [(crate::physiology::SALINITY.to_string(), 2u8)]
                .into_iter()
                .collect(),
        }
    }

    /// A cell composition carrying only a salinity toxin dose.
    fn salt_cell(dose: Fixed) -> Composition {
        Composition {
            nutrients: BTreeMap::new(),
            toxins: [(crate::physiology::SALINITY.to_string(), dose)]
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn a_salt_flat_is_lethal_to_a_naive_lineage_and_livable_to_a_halophile() {
        // Base-level liveliness step 4, THE MILESTONE PROOF: two beings stand on one identical salt flat
        // (a cell dosing bio.salinity), differing ONLY in their heritable salt tolerance. The naive one
        // (low tolerance) accrues harm faster than it heals and is worn through its CONDITION floor to
        // death; the halophile (high tolerance) heals faster than it is harmed and lives on indefinitely.
        // Death is the emergent reserve-through-floor cull, never a fixed-dose exclusion gate (Principle
        // 8), and it keys off each being's own tolerance, never a race or kind id (Principle 9).
        let reg = condition_reg();
        let afford = AffordanceRegistry::dev_default();
        let l = layout_for(&reg);
        let c = Controller::zeros(&l); // idle: it does not move, so it stays on the flat
        let tile = Coord3::ground(0, 0);
        let dose = Fixed::from_int(2); // a fully-evaporated salt flat's dose
        let mut field = ResourceField::new();
        field.set(tile, salt_cell(dose));
        let p = LocomotionParams::dev_default();
        let empty_dirs: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>> =
            BTreeMap::new();
        let empty_signed: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, Fixed>> = BTreeMap::new();
        let empty_drains: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, DerivedDrain>> =
            BTreeMap::new();

        let mk = |tolerance: Fixed| {
            let homeo = Homeostasis::from_mass(&reg, Fixed::ONE); // CONDITION starts full
            Walker::new(
                StableId(1),
                tile,
                mobile_body(),
                homeo,
                salt_physiology(tolerance),
                c.clone(),
            )
        };
        let mut run = |tolerance: Fixed| -> u32 {
            let mut ws = vec![mk(tolerance)];
            let mut survived = 0u32;
            for t in 0..80u32 {
                step_with_field_dirs(
                    &mut ws,
                    &reg,
                    &l,
                    &afford,
                    &BodyPlanRegistry::dev_default(),
                    &OpenGround,
                    &mut field,
                    &p,
                    SEED,
                    t as u64,
                    &empty_dirs,
                    &empty_signed,
                    &empty_drains,
                    None,
                    &crate::percept::PerceptRegistry::empty(),
                    &crate::material_percept::MaterialPerceptRegistry::empty(),
                    &crate::material::MaterialField::new(),
                    &std::collections::BTreeMap::new(),
                    &std::collections::BTreeMap::new(), // no appetitive block on the fixture path
                    &std::collections::BTreeMap::new(), // no attraction block on the fixture path
                    &std::collections::BTreeMap::new(), // no conviction block on the fixture path
                    &std::collections::BTreeMap::new(), // no being block on the fixture path
                    &mut std::collections::BTreeMap::new(),
                );
                if !ws[0].alive {
                    break;
                }
                survived = t + 1;
            }
            survived
        };

        let naive = run(Fixed::from_ratio(1, 5)); // tolerance 0.2, well below the dose
        let halophile = run(Fixed::from_int(5)); // tolerance 5, well above the dose
        assert!(
            naive < 80,
            "the naive lineage is worn through its condition and dies on the salt flat (survived {naive} ticks)"
        );
        assert_eq!(
            halophile, 80,
            "the halophile lives on the salt flat the whole run: its heritable tolerance outpaces the harm"
        );
        assert!(
            halophile > naive,
            "the salt flat is lethal to the naive lineage and livable to the halophile: {halophile} > {naive}"
        );
    }

    #[test]
    fn the_feature_channel_and_interoceptive_delta_are_read_on_the_flat() {
        // Harm-learning arc slice a, THE MILESTONE: a being standing on the salt flat reads a high
        // bio.salinity FEATURE channel in its controller input, and its interoceptive CONDITION DELTA
        // goes negative as the salt wears it. Both are pure reads of already-hashed physical state (the
        // tile's composition and the being's own reserves), draw no randomness, and replay bit for bit.
        // Declaring a percept grows the controller feature block by exactly one input; a world that
        // declares no percepts is untouched (that hash-neutrality is carried by every existing suite
        // staying green with the wiring in place).
        use crate::percept::PerceptRegistry;

        let reg = condition_reg();
        let afford = AffordanceRegistry::dev_default();
        let percepts = PerceptRegistry::dev_salinity();
        let l = ControllerLayout::with_percepts(&reg, &afford, &percepts, 0);
        assert_eq!(
            l.n_features(),
            1,
            "declaring one percept grows the controller feature block by one channel"
        );
        assert_eq!(
            l.n_in(),
            ControllerLayout::new(&reg, &afford, 0).n_in() + 1,
            "the feature block adds exactly one input over the percept-less layout"
        );

        let tile = Coord3::ground(0, 0);
        let dose = Fixed::from_int(2); // a fully-evaporated salt flat's dose
        let mut field = ResourceField::new();
        field.set(tile, salt_cell(dose));

        // The feature the being senses underfoot is the raw salinity dose, and it reaches the controller
        // input at the feature block: a high bio.salinity channel (Principle 9, a raw physical read).
        let features = percepts.perceive(field.composition(tile));
        assert_eq!(
            features,
            vec![dose],
            "the being senses the raw salinity dose as its one feature channel"
        );
        let homeo0 = Homeostasis::from_mass(&reg, Fixed::ONE);
        let input = l.build_input_with_features(
            &homeo0,
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &features,
        );
        assert_eq!(
            input[l.feature_input_base()],
            dose,
            "the salinity feature reaches the controller input at the feature block base"
        );

        // On the run: a naive being idles on the flat (a zeros controller, so it stays), takes salt harm
        // each tick, and its interoceptive CONDITION delta reads the net fall.
        let c = Controller::zeros(&l);
        let p = LocomotionParams::dev_default();
        let empty_dirs: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>> =
            BTreeMap::new();
        let empty_signed: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, Fixed>> = BTreeMap::new();
        let empty_drains: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, DerivedDrain>> =
            BTreeMap::new();
        let run_delta = || -> Fixed {
            let mut ws = vec![Walker::new(
                StableId(1),
                tile,
                mobile_body(),
                Homeostasis::from_mass(&reg, Fixed::ONE),
                salt_physiology(Fixed::from_ratio(1, 5)), // naive: harm outpaces the recovery
                c.clone(),
            )];
            let mut field = field.clone();
            for t in 0..3u64 {
                step_with_field_dirs(
                    &mut ws,
                    &reg,
                    &l,
                    &afford,
                    &BodyPlanRegistry::dev_default(),
                    &OpenGround,
                    &mut field,
                    &p,
                    SEED,
                    t,
                    &empty_dirs,
                    &empty_signed,
                    &empty_drains,
                    None,
                    &percepts,
                    &crate::material_percept::MaterialPerceptRegistry::empty(),
                    &crate::material::MaterialField::new(),
                    &std::collections::BTreeMap::new(),
                    &std::collections::BTreeMap::new(), // no appetitive block on the fixture path
                    &std::collections::BTreeMap::new(), // no attraction block on the fixture path
                    &std::collections::BTreeMap::new(), // no conviction block on the fixture path
                    &std::collections::BTreeMap::new(), // no being block on the fixture path
                    &mut std::collections::BTreeMap::new(),
                );
            }
            // The delta since the start of the last tick: the net CONDITION change the salt harm drove.
            ws[0].reserve_memory.delta(CONDITION, &ws[0].homeostasis)
        };
        let delta_a = run_delta();
        assert!(
            delta_a < Fixed::ZERO,
            "the interoceptive CONDITION delta goes negative as the salt wears the naive being: {delta_a:?}"
        );
        let delta_b = run_delta();
        assert_eq!(
            delta_a, delta_b,
            "the interoceptive delta is deterministic and replays bit for bit"
        );
    }

    #[test]
    fn the_belief_avoidance_gradient_steers_a_being_only_through_an_evolved_weight() {
        // Harm-learning arc slice c, THE MILESTONE: the belief-derived avoidance gradient (a westward
        // percept, as if the being believes the salt to its east harms it) reaches the CONDITION
        // direction slot, and a being whose evolved CONDITION-dir-to-heading weight is non-zero steers
        // AWAY (west) by it, while a being with a blank controller (the founding-zero weight) ignores it
        // and only explores. So avoidance is not authored: it EMERGES exactly when selection lifts the
        // weight off zero (Principle 9). The gradient enters as an input the controller weights, the same
        // way the temperature gradient does; the MOVE arm never subtracts a harm term itself.
        use crate::controller::{forage_taxis_weights, ForageGains};

        let reg = condition_reg();
        let afford = AffordanceRegistry::dev_default();
        let l = ControllerLayout::new(&reg, &afford, 0);
        let n_in = l.n_in();
        let cond_base = l.axis_input_base(CONDITION).unwrap();
        let p = LocomotionParams::dev_default();

        // The avoider: it wants to move (move_bias) and steers its MOVE heading along the CONDITION
        // gradient (CONDITION as a steer axis), so it follows the avoidance percept. MOVE is output 0
        // (act, dx, dy at 0,1,2), INGEST the scalar output at 3.
        let gains = ForageGains {
            move_bias: Fixed::ONE,
            here_suppress: Fixed::ZERO,
            heading_gain: Fixed::ONE,
            ingest_drive: Fixed::ZERO,
        };
        let mut avoider_w = vec![Fixed::ZERO; l.weight_count()];
        for (pid, v) in forage_taxis_weights(&l, 0, 3, &[], &[cond_base], gains) {
            avoider_w[pid.0 as usize] = v;
        }
        let avoider = Controller::from_weights(n_in, l.n_out(), l.hidden(), avoider_w);

        // The blank: it wants to move but has no CONDITION-dir weight, so the same gradient is inert.
        let mut blank_w = vec![Fixed::ZERO; l.weight_count()];
        blank_w[n_in - 1] = Fixed::ONE; // move_act bias only
        let blank = Controller::from_weights(n_in, l.n_out(), l.hidden(), blank_w);

        // A westward avoidance gradient in the CONDITION direction slot (away from believed harm to the
        // east), supplied per being exactly as the runner supplies it from the belief in slice c.
        let west: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>> =
            BTreeMap::from([(
                StableId(1),
                BTreeMap::from([(CONDITION, (Fixed::ZERO - Fixed::ONE, Fixed::ZERO))]),
            )]);
        let empty_signed: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, Fixed>> = BTreeMap::new();
        let empty_drains: BTreeMap<StableId, BTreeMap<HomeostaticAxisId, DerivedDrain>> =
            BTreeMap::new();

        let run_end_x = |controller: &Controller| -> Fixed {
            let mut ws = vec![Walker::new(
                StableId(1),
                Coord3::ground(8, 8),
                mobile_body(),
                Homeostasis::from_mass(&reg, Fixed::ONE),
                Physiology::dev_for_registry(&reg),
                controller.clone(),
            )];
            let mut field = ResourceField::new();
            for t in 0..12u64 {
                step_with_field_dirs(
                    &mut ws,
                    &reg,
                    &l,
                    &afford,
                    &BodyPlanRegistry::dev_default(),
                    &OpenGround,
                    &mut field,
                    &p,
                    SEED,
                    t,
                    &west,
                    &empty_signed,
                    &empty_drains,
                    None,
                    &crate::percept::PerceptRegistry::empty(),
                    &crate::material_percept::MaterialPerceptRegistry::empty(),
                    &crate::material::MaterialField::new(),
                    &std::collections::BTreeMap::new(),
                    &std::collections::BTreeMap::new(), // no appetitive block on the fixture path
                    &std::collections::BTreeMap::new(), // no attraction block on the fixture path
                    &std::collections::BTreeMap::new(), // no conviction block on the fixture path
                    &std::collections::BTreeMap::new(), // no being block on the fixture path
                    &mut std::collections::BTreeMap::new(),
                );
            }
            ws[0].x
        };

        let start_x = Fixed::from_int(8) + HALF;
        let avoider_x = run_end_x(&avoider);
        let blank_x = run_end_x(&blank);
        // The avoider steered west, away from the believed harm to its east.
        assert!(
            avoider_x < start_x,
            "the avoider steers west (away) by the CONDITION gradient: {avoider_x:?} < {start_x:?}"
        );
        // The blank being's founding-zero CONDITION-dir weight leaves the gradient inert, so it does not
        // systematically flee west: avoidance appears only with the evolved weight, never authored.
        assert!(
            avoider_x < blank_x,
            "avoidance emerges from the evolved weight: the avoider ends west of the blank explorer \
             ({avoider_x:?} < {blank_x:?})"
        );
    }
}
