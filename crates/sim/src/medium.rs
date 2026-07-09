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

//! The medium respiration coupling (design Part 15, Part 20, Part 35, Part 41; R-MEDIUM; Principles 3,
//! 9, 11). The organism side of the medium substrate: a being exchanges its respirable-gas reserve with
//! the ambient medium through the resolved Fick membrane law ([`civsim_physics::laws::membrane_gas_flux`]),
//! so respiration is a physical consequence of a body's anatomy meeting a medium, never an authored
//! affinity.
//!
//! What is authored is physics: the gas-exchange law (the physics floor), and a being's exchange area,
//! which is the development-weighted sum over its organs of their `bio.respiratory_surface` composition
//! (the same composition-derived sum the metabolic reserves use, [`crate::homeostasis::Homeostasis::new`]).
//! What is not authored is the outcome: a being with a large respiratory surface in a medium rich in the
//! respirable species breathes and thrives; the same body in a poor medium off-gasses and suffocates;
//! and a body with no respiratory organ presents no surface, takes up nothing, and dies whatever the
//! medium. Nothing reads a medium label: a gill in water and a lung in air are the same coupling over
//! different respirable contents, so aquatic, terrestrial, and exotic-medium life are one mechanism
//! (Principle 9). The medium-specialisation of a gill (its failure in air through the dissolved-versus-
//! partial-pressure partition, Henry's law) is the deferred refinement the proposal names; this pass
//! models the shared gas exchange and the no-surface-no-breath consequence.
//!
//! Everything here is integer fixed-point and draws no randomness (Principle 3). The transfer
//! coefficient, the flux cap, and the respiration axis rates and hypoxia floor are reserved with their
//! basis and are the owner's to set (Principle 11); the values here are labelled development fixtures.

use civsim_core::Fixed;
use civsim_physics::laws;
use civsim_world::{Coord3, TileMap};

use crate::anatomy::{BodyPlan, BodyPlanRegistry};
use crate::homeostasis::{Homeostasis, HomeostaticAxisDef, HomeostaticRegistry, RESPIRATION};

/// The biology-floor axis id a respiratory organ carries its gas-exchange surface on (the physics
/// biology floor, `crates/physics/data/biology_floor.toml`). A tissue with none of it is not a
/// respiratory surface, the substrate's absence convention.
pub const RESPIRATORY_SURFACE: &str = "bio.respiratory_surface";

/// A being's total respiratory exchange area: the development-weighted sum over its organs of their
/// `bio.respiratory_surface` composition. The same composition-derived shape the metabolic reserve
/// capacities use, so a body's ability to breathe follows its anatomy: a body with no respiratory organ
/// presents zero area and cannot exchange gas with any medium.
pub fn exchange_area(plan: &BodyPlan, organs: &BodyPlanRegistry) -> Fixed {
    let mut sum = Fixed::ZERO;
    for organ in &plan.organs {
        let surface = organs
            .organ_composition(organ.kind)
            .map(|c| c.component(RESPIRATORY_SURFACE))
            .unwrap_or(Fixed::ZERO);
        let area = organ
            .development
            .checked_mul(surface)
            .unwrap_or(Fixed::ZERO);
        sum = sum.saturating_add(area);
    }
    sum
}

/// One tick of respiration: the being exchanges the respirable species with its ambient medium through
/// the Fick membrane law, applying the signed flux to its [`RESPIRATION`] reserve. Physics in: the flux
/// is `k * A * (c_medium - c_internal)`, where `k` is the transfer coefficient, `A` the being's exchange
/// area (from its organs), `c_medium` the ambient medium's `fluid.respirable_content`, and `c_internal`
/// the being's current reserve amount (in the same normalised concentration units, so the comparison is
/// physical). The metabolic draw on the reserve is applied separately by
/// [`Homeostasis::metabolize`], so a rich medium replenishes what metabolism spends and a poor one does
/// not. Nothing here reads a medium label; only its respirable content.
pub fn respire(
    homeo: &mut Homeostasis,
    exchange_area: Fixed,
    transfer_k: Fixed,
    medium_content: Fixed,
    flux_cap: Fixed,
) {
    let c_internal = homeo.amount(RESPIRATION);
    let flux = laws::membrane_gas_flux(
        transfer_k,
        exchange_area,
        medium_content,
        c_internal,
        flux_cap,
    );
    homeo.adjust(RESPIRATION, flux);
}

/// A labelled DEVELOPMENT FIXTURE registry: a single respiration axis, a unit-capacity oxygen buffer
/// that drains by metabolism and is replenished by the medium coupling. Not owner values. The axis is
/// non-backed (unit capacity, [`crate::homeostasis::Homeostasis::new`]): the anatomy dependence is in
/// the exchange area (the uptake rate), not the buffer size, so a being's ability to breathe follows its
/// respiratory organ while the buffer stays a normalised reserve. RESERVED, with basis: the base and
/// exertion drains are the resting and working oxygen demand of Part 20 mapped onto the base tick, and
/// the death floor is the hypoxic reserve fraction at which the body fails (a being dies well before the
/// buffer is empty), set as a labelled fixture here.
pub fn dev_respiration() -> HomeostaticRegistry {
    HomeostaticRegistry {
        axes: vec![HomeostaticAxisDef {
            id: RESPIRATION,
            name: "respiration".to_string(),
            backing_component: None,
            capacity_per_mass: Fixed::ONE,
            base_drain: Fixed::from_ratio(1, 50),
            exertion_drain: Fixed::from_ratio(1, 50),
            death_floor: Fixed::from_ratio(1, 2),
        }],
    }
}

/// The content of one medium class, the two axes a [`MediumField`] cell carries that a body reads
/// against its own anatomy: the respirable content (the `c_medium` the Fick respiration law reads) and
/// the density (the `rho_medium` the buoyancy law reads). A sample is data, not a label: `medium.water`
/// and `medium.air` are the same pair over different axis values (Principle 9), so the folding rule that
/// assigns a sample to a cell keys off physics, never a medium name. The medium's own temperature is not
/// carried here: a cell's temperature is the map tile's worldgen temperature (see [`MediumField::from_map`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediumSample {
    /// The respirable content of the medium (the `c_medium` term of the Fick respiration law).
    pub respirable: Fixed,
    /// The density of the medium (the `rho_medium` term of the buoyancy law).
    pub density: Fixed,
}

/// A per-cell ambient-medium field: the medium each map cell holds, as its respirable content and its
/// density, in the row-major layout the temperature field uses ([`crate::runner::Field`]). A being reads
/// the medium of the cell it stands in, so respiration (and, a later increment, buoyancy) is located:
/// moving from a water cell to an air cell changes the medium a body exchanges with. The medium is data,
/// not a label: air, water, lava, and a magical fluid are the same field over different axis values
/// (Principle 9).
#[derive(Clone, Debug)]
pub struct MediumField {
    width: i32,
    height: i32,
    respirable: Vec<Fixed>,
    density: Vec<Fixed>,
    temperature: Vec<Fixed>,
}

impl MediumField {
    /// A field from explicit per-cell respirable content, density, and temperature (row-major,
    /// `width * height` each). The temperature is the medium's, the state a being's core temperature
    /// exchanges toward (a lava cell is lethally hot, a void cell cold), read by [`in_medium_temperature`].
    pub fn new(
        width: i32,
        height: i32,
        respirable: Vec<Fixed>,
        density: Vec<Fixed>,
        temperature: Vec<Fixed>,
    ) -> MediumField {
        assert!(width > 0 && height > 0, "a field has positive extent");
        let n = (width as usize) * (height as usize);
        assert_eq!(respirable.len(), n, "respirable is width*height long");
        assert_eq!(density.len(), n, "density is width*height long");
        assert_eq!(temperature.len(), n, "temperature is width*height long");
        MediumField {
            width,
            height,
            respirable,
            density,
            temperature,
        }
    }

    /// A uniform medium filling the whole field (one medium everywhere).
    pub fn uniform(
        width: i32,
        height: i32,
        respirable: Fixed,
        density: Fixed,
        temperature: Fixed,
    ) -> MediumField {
        let n = (width as usize) * (height as usize);
        MediumField::new(
            width,
            height,
            vec![respirable; n],
            vec![density; n],
            vec![temperature; n],
        )
    }

    /// A per-cell field folded from a generated world's tiles (real-world unification step 4; owner
    /// ruling 2026-07-04): each cell holds the `submerged` medium below the reserved `submersion_elevation`
    /// and the `emergent` medium at or above it. The rule keys off the same normalised `[0, ONE)` elevation
    /// field the biome classifier reads (`crates/world/src/terrain.rs`), so a waterline is a physical
    /// threshold rather than an `if biome == Ocean` label branch (Principle 9), and the membership stays
    /// data: a world whose two media are the same [`MediumSample`] regresses to a uniform field, and a
    /// world of exotic media is the same fold over different axis values. Pure integer fixed-point over the
    /// deterministic worldgen map, no floating point and no randomness (Principle 3), walked in the field's
    /// row-major (y then x) order, so the same map and threshold fold identically on any machine and thread
    /// count.
    ///
    /// The per-cell temperature is the tile's own worldgen temperature, the same source
    /// [`crate::runner::Field::from_map`] seeds its baseline from. It is carried for the deferred in-medium
    /// thermal exchange and is unread on the running tick this increment: the canonical diffused
    /// [`crate::runner::Field`] is authoritative for a being's ambient temperature, and only respiration
    /// (which reads [`Self::respirable_at`] alone) is wired to the medium field here, so the field's
    /// temperature vector and the canonical field cannot desync in observed state.
    pub fn from_map(
        map: &TileMap,
        submersion_elevation: Fixed,
        submerged: MediumSample,
        emergent: MediumSample,
    ) -> MediumField {
        let topo = map.topo();
        let (w, h) = (topo.width, topo.height);
        let n = (w.max(0) as usize) * (h.max(0) as usize);
        let mut respirable = Vec::with_capacity(n);
        let mut density = Vec::with_capacity(n);
        let mut temperature = Vec::with_capacity(n);
        for y in 0..h {
            for x in 0..w {
                let tile = map
                    .tile(Coord3::new(x, y, 0))
                    .expect("every in-bounds cell has a tile");
                let sample = if tile.elevation() < submersion_elevation {
                    submerged
                } else {
                    emergent
                };
                respirable.push(sample.respirable);
                density.push(sample.density);
                temperature.push(tile.temperature());
            }
        }
        MediumField::new(w, h, respirable, density, temperature)
    }

    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }

    /// The respirable content of the medium at a cell. Off the field there is no medium and this reads
    /// zero (no medium, no breath), the substrate's absence convention.
    pub fn respirable_at(&self, x: i32, y: i32) -> Fixed {
        self.idx(x, y)
            .map(|i| self.respirable[i])
            .unwrap_or(Fixed::ZERO)
    }

    /// The density of the medium at a cell (zero off the field). The buoyancy increment reads this
    /// against a being's body density through the resolved `law.buoyant_force`.
    pub fn density_at(&self, x: i32, y: i32) -> Fixed {
        self.idx(x, y)
            .map(|i| self.density[i])
            .unwrap_or(Fixed::ZERO)
    }

    /// The temperature of the medium at a cell. Off the field there is no medium; this reads zero, which
    /// a caller distinguishes from a real zero-temperature medium by also checking [`Self::contains`]
    /// (the absence convention, as with the other reads).
    pub fn temperature_at(&self, x: i32, y: i32) -> Fixed {
        self.idx(x, y)
            .map(|i| self.temperature[i])
            .unwrap_or(Fixed::ZERO)
    }

    /// Whether a cell is on the field (holds a medium at all).
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.idx(x, y).is_some()
    }
}

/// One tick of respiration for a being located at `pos` in an ambient-medium field: read the medium of
/// its cell and exchange gas with it ([`respire`]). Off the field a being finds no medium and takes up
/// nothing (it suffocates on its buffer). This is the located form the running world uses; the unlocated
/// [`respire`] takes the medium content directly, for isolation tests and callers that already know the
/// medium.
pub fn respire_at(
    homeo: &mut Homeostasis,
    plan: &BodyPlan,
    organs: &BodyPlanRegistry,
    field: &MediumField,
    pos: Coord3,
    transfer_k: Fixed,
    flux_cap: Fixed,
) {
    let content = field.respirable_at(pos.x, pos.y);
    let area = exchange_area(plan, organs);
    respire(homeo, area, transfer_k, content, flux_cap);
}

/// The mechanical-floor axis a tissue carries its density on (`mat.density`, kg/m^3). An organ that
/// carries none of it does not contribute a density (the absence convention).
pub const TISSUE_DENSITY: &str = "mat.density";

/// The baseline body density (kg/m^3) for a body whose organs declare no density: water, a body being
/// mostly water. RESERVED fixture, basis: the density of water and of soft tissue (CRC ~1000 to 1060);
/// the labelled value here is not owner canon.
const BODY_DENSITY_BASELINE: Fixed = Fixed::from_int(1000);

/// A being's body density (kg/m^3): the development-weighted average over its organs of their
/// `mat.density` composition, or the water baseline if no organ declares a density. This is the
/// first-cut organ-composition proxy (the whole-body tissue-and-covering mixture density is the noted
/// refinement); it is enough to float a fat or gas-filled body and sink a dense, mineralised one. The
/// same composition-derived shape the reserves and the exchange area use, so buoyancy follows anatomy.
pub fn body_density(plan: &BodyPlan, organs: &BodyPlanRegistry) -> Fixed {
    let mut weighted = Fixed::ZERO;
    let mut total_dev = Fixed::ZERO;
    for organ in &plan.organs {
        let density = organs
            .organ_composition(organ.kind)
            .map(|c| c.component(TISSUE_DENSITY))
            .unwrap_or(Fixed::ZERO);
        if density > Fixed::ZERO {
            let contribution = organ
                .development
                .checked_mul(density)
                .unwrap_or(Fixed::ZERO);
            weighted = weighted.saturating_add(contribution);
            total_dev = total_dev.saturating_add(organ.development);
        }
    }
    if total_dev <= Fixed::ZERO {
        return BODY_DENSITY_BASELINE;
    }
    weighted
        .checked_div(total_dev)
        .unwrap_or(BODY_DENSITY_BASELINE)
}

/// The signed net buoyant acceleration of a body in a medium, through the resolved physics kernels: the
/// medium's upward push ([`civsim_physics::laws::buoyant_force`], rho_medium * g * V) less the body's
/// weight ([`civsim_physics::laws::weight`], rho_body * g over the same unit volume), so the sign is the
/// density difference: positive floats, negative sinks, zero is neutral. A body denser than its medium
/// sinks; a lighter one floats. No medium label: the same law over air, water, and lava, so a dense body
/// that sinks in water floats on lava and plummets in air, from the physics alone (Principle 9).
pub fn buoyancy(body_density: Fixed, medium_density: Fixed, gravity: Fixed) -> Fixed {
    let cap = Fixed::from_int(1_000_000_000);
    let up = laws::buoyant_force(medium_density, gravity, Fixed::ONE, cap);
    let down = laws::weight(body_density, gravity, cap);
    // up and down are each rho * g over a unit volume, bounded well below the cap for physical
    // densities and gravity, so their difference is representable.
    up - down
}

/// The signed net buoyant acceleration for a being located at `pos`: its body density (from its organs)
/// against the density of the medium in its cell, through [`buoyancy`]. Off the field there is no medium
/// (zero density), so a body there reads a downward acceleration (it falls).
pub fn buoyancy_at(
    plan: &BodyPlan,
    organs: &BodyPlanRegistry,
    field: &MediumField,
    pos: Coord3,
    gravity: Fixed,
) -> Fixed {
    let medium_density = field.density_at(pos.x, pos.y);
    buoyancy(body_density(plan, organs), medium_density, gravity)
}

/// One tick of in-medium thermal exchange: a being's core temperature relaxes toward the temperature of
/// the medium it sits in, `new = prev + rate * (medium_temp - prev)`, the discrete form of Newton's law
/// of cooling. Physics in: a body in a hot medium (lava) heats toward it and one in a cold medium cools,
/// at the reserved exchange rate, so a lethal-hot or lethal-cold medium drives the body out of its
/// comfort band ([`crate::runner::comfort_fraction`]) and kills it, with no authored rule. The exchange
/// is a scalar relaxation toward the medium temperature: it carries no spatial direction and treats a
/// hot and a cold medium as mirror images, so it steers no heading (Principle 9).
pub fn in_medium_temperature(
    prev_body_temp: Fixed,
    medium_temp: Fixed,
    exchange_rate: Fixed,
) -> Fixed {
    let rate = exchange_rate.clamp(Fixed::ZERO, Fixed::ONE);
    // Temperatures are held in the floor's bounded range, so the signed gap is representable.
    let gap = medium_temp - prev_body_temp;
    let step = rate.checked_mul(gap).unwrap_or(Fixed::ZERO);
    prev_body_temp.saturating_add(step)
}

/// The located form: a being at `pos` exchanges heat with the medium of its cell. Off the field there is
/// no medium to exchange with, so the body temperature holds (unlike an on-field cell, whose real medium
/// temperature it relaxes toward).
pub fn in_medium_temperature_at(
    prev_body_temp: Fixed,
    field: &MediumField,
    pos: Coord3,
    exchange_rate: Fixed,
) -> Fixed {
    if !field.contains(pos.x, pos.y) {
        return prev_body_temp;
    }
    in_medium_temperature(
        prev_body_temp,
        field.temperature_at(pos.x, pos.y),
        exchange_rate,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{BodyPlan, OrganKindDef, Part, Temperament, TissueComposition};

    fn temperament() -> Temperament {
        Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(1, 2),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        }
    }

    /// A registry whose organ set holds one respiratory organ (a gill, a full gas-exchange surface) at
    /// a known id, alongside the default organs.
    fn registry_with_gill() -> (BodyPlanRegistry, u16) {
        let mut reg = BodyPlanRegistry::dev_default();
        let id = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id,
            name: "gill".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(RESPIRATORY_SURFACE, Fixed::ONE)]),
        });
        (reg, id)
    }

    /// A body bearing the given organs.
    fn body(organs: Vec<Part>) -> BodyPlan {
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
            locomotion: vec![1],
            organs,
            temperament: temperament(),
        }
    }

    fn organ(kind: u16, dev: (i64, i64)) -> Part {
        Part {
            kind,
            development: Fixed::from_ratio(dev.0, dev.1),
        }
    }

    /// Run the respiration-plus-metabolism loop in a medium of the given respirable content, returning
    /// how many ticks the being survived (capped) and whether it ended alive.
    fn survive(
        plan: &BodyPlan,
        organs: &BodyPlanRegistry,
        content: Fixed,
        ticks: u32,
    ) -> (u32, bool) {
        let reg = dev_respiration();
        let area = exchange_area(plan, organs);
        let k = Fixed::ONE;
        let cap = Fixed::from_int(1000);
        let mut h = Homeostasis::new(&reg, plan, organs);
        let mut lived = 0;
        for _ in 0..ticks {
            respire(&mut h, area, k, content, cap);
            if !h.metabolize(&reg, Fixed::ZERO) {
                break;
            }
            lived += 1;
        }
        (lived, h.is_alive(&reg))
    }

    #[test]
    fn a_gilled_being_breathes_and_survives_in_a_rich_medium() {
        let (organs, gill) = registry_with_gill();
        let plan = body(vec![organ(gill, (1, 1))]);
        let (lived, alive) = survive(&plan, &organs, Fixed::ONE, 500);
        assert_eq!(
            lived, 500,
            "a full respiratory surface in a rich medium keeps breathing"
        );
        assert!(alive, "and ends the run alive");
    }

    #[test]
    fn the_same_being_suffocates_in_a_poor_medium() {
        // The identical body and organ: only the medium's respirable content differs, and the outcome
        // flips. The affinity is not a label on the being; it is the physics of the medium it is in.
        let (organs, gill) = registry_with_gill();
        let plan = body(vec![organ(gill, (1, 1))]);
        let (lived, alive) = survive(&plan, &organs, Fixed::from_ratio(1, 5), 500);
        assert!(
            !alive,
            "a medium too poor in the respirable species suffocates it"
        );
        assert!(lived < 500, "it dies before the cap");
    }

    #[test]
    fn a_body_with_no_respiratory_organ_suffocates_even_in_a_rich_medium() {
        // The no-surface-no-breath consequence: a body that presents no exchange area cannot take gas
        // from even the richest medium. This is the physical basis the biosphere viability cull leans on
        // for a would-be air-breather with no lung, the sibling of the no-energy-organ case (F3).
        let (organs, _gill) = registry_with_gill();
        let plan = body(vec![organ(0, (1, 1))]); // one fat-body: no respiratory surface
        assert_eq!(
            exchange_area(&plan, &organs),
            Fixed::ZERO,
            "no respiratory organ, no area"
        );
        let (lived, alive) = survive(&plan, &organs, Fixed::ONE, 500);
        assert!(!alive, "it suffocates despite the rich medium");
        assert!(
            lived > 0 && lived < 500,
            "it lives on its buffer a while, then dies"
        );
    }

    #[test]
    fn a_larger_respiratory_surface_takes_up_more() {
        // The uptake rate follows the anatomy: at the same medium and internal deficit, a larger
        // exchange area takes up more gas. Physics in, no authored preference.
        let (organs, gill) = registry_with_gill();
        let reg = dev_respiration();
        let cap = Fixed::from_int(1000);
        let content = Fixed::ONE;
        let big = body(vec![organ(gill, (1, 1))]); // full gill
        let small = body(vec![organ(gill, (1, 4))]); // quarter gill
        let mut hb = Homeostasis::new(&reg, &big, &organs);
        let mut hs = Homeostasis::new(&reg, &small, &organs);
        // Drain both to the same low level, then let each take one breath.
        hb.adjust(RESPIRATION, Fixed::ZERO - Fixed::from_ratio(1, 2));
        hs.adjust(RESPIRATION, Fixed::ZERO - Fixed::from_ratio(1, 2));
        respire(
            &mut hb,
            exchange_area(&big, &organs),
            Fixed::ONE,
            content,
            cap,
        );
        respire(
            &mut hs,
            exchange_area(&small, &organs),
            Fixed::ONE,
            content,
            cap,
        );
        assert!(
            hb.amount(RESPIRATION) > hs.amount(RESPIRATION),
            "the larger surface replenishes more from the same medium"
        );
    }

    #[test]
    fn respiration_is_deterministic() {
        let (organs, gill) = registry_with_gill();
        let plan = body(vec![organ(gill, (3, 4))]);
        let run = || survive(&plan, &organs, Fixed::from_ratio(3, 4), 200);
        assert_eq!(run(), run(), "the same body and medium replay bit for bit");
    }

    /// A two-cell field: cell (0,0) is a rich medium, cell (1,0) is a poor one, each a uniform density.
    fn two_region_field() -> MediumField {
        MediumField::new(
            2,
            1,
            vec![Fixed::ONE, Fixed::from_ratio(1, 5)], // rich then poor respirable content
            vec![Fixed::from_int(998), Fixed::from_ratio(1225, 1000)], // water then air density
            vec![Fixed::from_int(290), Fixed::from_int(290)], // both temperate
        )
    }

    /// Run the located respiration-plus-metabolism loop at a cell of a given field, returning whether
    /// the being ended alive after the given ticks.
    fn survive_at(
        plan: &BodyPlan,
        organs: &BodyPlanRegistry,
        field: &MediumField,
        pos: Coord3,
        ticks: u32,
    ) -> bool {
        let reg = dev_respiration();
        let cap = Fixed::from_int(1000);
        let mut h = Homeostasis::new(&reg, plan, organs);
        for _ in 0..ticks {
            respire_at(&mut h, plan, organs, field, pos, Fixed::ONE, cap);
            if !h.metabolize(&reg, Fixed::ZERO) {
                break;
            }
        }
        h.is_alive(&reg)
    }

    #[test]
    fn a_being_breathes_the_medium_of_the_cell_it_stands_in() {
        // The same body in the same field: standing in the rich cell it survives, in the poor cell it
        // suffocates. Respiration is located: the medium is the cell's, not the being's label.
        let (organs, gill) = registry_with_gill();
        let plan = body(vec![organ(gill, (1, 1))]);
        let field = two_region_field();
        assert!(
            survive_at(&plan, &organs, &field, Coord3::ground(0, 0), 500),
            "in the rich cell it breathes and survives"
        );
        assert!(
            !survive_at(&plan, &organs, &field, Coord3::ground(1, 0), 500),
            "in the poor cell the same body suffocates"
        );
    }

    #[test]
    fn amphibious_life_emerges_a_body_viable_in_two_media() {
        // Nothing tags a being aquatic or terrestrial. A body whose respiratory organ exchanges the
        // respirable species in two media both rich enough for it is viable in both cells: amphibious
        // life is a consequence of the anatomy meeting the media, not an authored both-medium band.
        let (organs, gill) = registry_with_gill();
        let plan = body(vec![organ(gill, (1, 1))]);
        // Two respirable-rich media of different density: a dense water and a light air.
        let field = MediumField::new(
            2,
            1,
            vec![Fixed::ONE, Fixed::from_ratio(9, 10)],
            vec![Fixed::from_int(998), Fixed::from_ratio(1225, 1000)],
            vec![Fixed::from_int(290), Fixed::from_int(290)],
        );
        assert!(
            survive_at(&plan, &organs, &field, Coord3::ground(0, 0), 500),
            "the same body breathes the dense medium"
        );
        assert!(
            survive_at(&plan, &organs, &field, Coord3::ground(1, 0), 500),
            "and the light medium, viable in both: amphibious, no flag"
        );
    }

    #[test]
    fn off_the_field_there_is_no_medium_to_breathe() {
        // A being off the field finds no medium (respirable content zero) and suffocates on its buffer,
        // whatever its anatomy. The field's absence convention: no medium, no breath.
        let (organs, gill) = registry_with_gill();
        let plan = body(vec![organ(gill, (1, 1))]);
        let field = two_region_field();
        assert!(
            !survive_at(&plan, &organs, &field, Coord3::ground(99, 99), 500),
            "off the field there is no medium to exchange with"
        );
    }

    #[test]
    fn the_field_reads_back_its_medium_and_zero_off_the_edge() {
        let field = two_region_field();
        assert_eq!(field.respirable_at(0, 0), Fixed::ONE);
        assert_eq!(field.respirable_at(1, 0), Fixed::from_ratio(1, 5));
        assert_eq!(
            field.density_at(0, 0),
            Fixed::from_int(998),
            "the water cell's density"
        );
        assert_eq!(
            field.density_at(1, 0),
            Fixed::from_ratio(1225, 1000),
            "the air cell's density"
        );
        assert_eq!(
            field.respirable_at(-1, 0),
            Fixed::ZERO,
            "off the field, no medium"
        );
        assert_eq!(
            field.density_at(5, 5),
            Fixed::ZERO,
            "off the field, no density"
        );
    }

    #[test]
    fn located_respiration_is_deterministic() {
        let (organs, gill) = registry_with_gill();
        let plan = body(vec![organ(gill, (1, 1))]);
        let field = two_region_field();
        let run = || survive_at(&plan, &organs, &field, Coord3::ground(0, 0), 200);
        assert_eq!(
            run(),
            run(),
            "the same body, field, and cell replay bit for bit"
        );
    }

    // Per-cell medium from a generated map (real-world unification step 4): the folding rule keys off the
    // physical elevation the biome classifier reads, water below the submersion threshold and air above,
    // with no biome-label branch.

    use civsim_world::{BiomeSet, FlatBounded, TileMap, WorldgenParams};

    fn generated_map(seed: u64) -> TileMap {
        let topo = FlatBounded::new(12, 9, 1);
        TileMap::generate(
            seed,
            topo,
            &BiomeSet::dev_default(),
            &WorldgenParams::dev_default(),
        )
    }

    fn water() -> MediumSample {
        MediumSample {
            respirable: Fixed::from_ratio(3, 10),
            density: Fixed::from_int(1000),
        }
    }

    fn air() -> MediumSample {
        MediumSample {
            respirable: Fixed::from_int(9),
            density: Fixed::from_ratio(12, 10),
        }
    }

    #[test]
    fn from_map_assigns_the_submerged_medium_below_the_threshold_and_the_emergent_above() {
        // Every cell reads back the medium its physical elevation earns: below the submersion elevation
        // the submerged (water) sample, at or above it the emergent (air) sample. The rule is the
        // elevation field alone, the same field the biome classifier reads, never a biome label. The
        // threshold is the midpoint of this map's own elevation range, so both classes are non-empty
        // whatever the seed generates (the split is real, not an artefact of a fixed cut).
        let map = generated_map(0xB10E);
        let topo = map.topo();
        let (mut lo, mut hi) = (Fixed::from_int(2), Fixed::ZERO - Fixed::from_int(2));
        for y in 0..topo.height {
            for x in 0..topo.width {
                let e = map.tile(Coord3::new(x, y, 0)).unwrap().elevation();
                lo = lo.min(e);
                hi = hi.max(e);
            }
        }
        assert!(
            hi > lo,
            "the fractal map has a spread of elevations to split"
        );
        let submersion = (lo + hi).checked_div(Fixed::from_int(2)).unwrap();
        let field = MediumField::from_map(&map, submersion, water(), air());
        let mut saw_water = false;
        let mut saw_air = false;
        for y in 0..topo.height {
            for x in 0..topo.width {
                let elev = map.tile(Coord3::new(x, y, 0)).unwrap().elevation();
                let (expect_respirable, expect_density) = if elev < submersion {
                    saw_water = true;
                    (water().respirable, water().density)
                } else {
                    saw_air = true;
                    (air().respirable, air().density)
                };
                assert_eq!(field.respirable_at(x, y), expect_respirable);
                assert_eq!(field.density_at(x, y), expect_density);
                assert_eq!(
                    field.temperature_at(x, y),
                    map.tile(Coord3::new(x, y, 0)).unwrap().temperature(),
                    "the cell's temperature is the map tile's worldgen temperature"
                );
            }
        }
        assert!(
            saw_water && saw_air,
            "the midpoint threshold splits the map into both submerged and emergent cells"
        );
    }

    #[test]
    fn a_single_medium_world_folds_to_a_uniform_field() {
        // The regression the owner ruling names: when the two media are the same sample, every cell reads
        // that one medium whatever its elevation, so a single-medium world is a uniform field.
        let map = generated_map(0x5EA);
        let only = water();
        let field = MediumField::from_map(&map, Fixed::from_ratio(40, 100), only, only);
        let topo = map.topo();
        for y in 0..topo.height {
            for x in 0..topo.width {
                assert_eq!(field.respirable_at(x, y), only.respirable);
                assert_eq!(field.density_at(x, y), only.density);
            }
        }
    }

    #[test]
    fn from_map_is_deterministic() {
        let run = || {
            MediumField::from_map(
                &generated_map(0x11),
                Fixed::from_ratio(40, 100),
                water(),
                air(),
            )
        };
        let a = run();
        let b = run();
        let topo = generated_map(0x11).topo();
        for y in 0..topo.height {
            for x in 0..topo.width {
                assert_eq!(a.respirable_at(x, y), b.respirable_at(x, y));
                assert_eq!(a.density_at(x, y), b.density_at(x, y));
                assert_eq!(a.temperature_at(x, y), b.temperature_at(x, y));
            }
        }
    }

    /// A registry with a light organ (density 900, a gas-filled float sac) and a dense organ
    /// (density 1900, a mineral ballast) at known ids.
    fn registry_with_density_organs() -> (BodyPlanRegistry, u16, u16) {
        let mut reg = BodyPlanRegistry::dev_default();
        let light = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: light,
            name: "float-sac".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(TISSUE_DENSITY, Fixed::from_int(900))]),
        });
        let dense = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: dense,
            name: "ballast".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(TISSUE_DENSITY, Fixed::from_int(1900))]),
        });
        (reg, light, dense)
    }

    fn g() -> Fixed {
        Fixed::from_ratio(981, 100)
    }

    #[test]
    fn body_density_is_the_organ_composition_average() {
        let (organs, light, dense) = registry_with_density_organs();
        assert_eq!(
            body_density(&body(vec![organ(light, (1, 1))]), &organs),
            Fixed::from_int(900)
        );
        assert_eq!(
            body_density(&body(vec![organ(dense, (1, 1))]), &organs),
            Fixed::from_int(1900)
        );
        // A body whose organs declare no density falls back to the water baseline.
        let (gill_reg, gill) = registry_with_gill();
        assert_eq!(
            body_density(&body(vec![organ(gill, (1, 1))]), &gill_reg),
            Fixed::from_int(1000),
            "no density-bearing organ, the water baseline"
        );
    }

    #[test]
    fn a_light_body_floats_and_a_dense_body_sinks_in_water() {
        let (organs, light, dense) = registry_with_density_organs();
        let water = Fixed::from_int(998);
        assert!(
            buoyancy(
                body_density(&body(vec![organ(light, (1, 1))]), &organs),
                water,
                g()
            ) > Fixed::ZERO,
            "a body lighter than water floats"
        );
        assert!(
            buoyancy(
                body_density(&body(vec![organ(dense, (1, 1))]), &organs),
                water,
                g()
            ) < Fixed::ZERO,
            "a body denser than water sinks"
        );
    }

    #[test]
    fn the_same_dense_body_floats_on_lava_but_falls_in_air() {
        // The identical body: only the medium's density differs, and float or sink flips. Buoyancy is
        // the physics of the medium the body is in, not a label on the body.
        let (organs, _light, dense) = registry_with_density_organs();
        let plan = body(vec![organ(dense, (1, 1))]); // body density 1900
        let field = MediumField::new(
            3,
            1,
            vec![Fixed::ZERO, Fixed::ZERO, Fixed::ZERO], // respirable content irrelevant to buoyancy
            vec![
                Fixed::from_int(998),          // water: the dense body sinks
                Fixed::from_int(3000),         // lava: denser than the body, it floats
                Fixed::from_ratio(1225, 1000), // air: far lighter, it falls
            ],
            vec![
                Fixed::from_int(290),
                Fixed::from_int(1500),
                Fixed::from_int(290),
            ], // temperature irrelevant to buoyancy
        );
        assert!(
            buoyancy_at(&plan, &organs, &field, Coord3::ground(0, 0), g()) < Fixed::ZERO,
            "sinks in water"
        );
        assert!(
            buoyancy_at(&plan, &organs, &field, Coord3::ground(1, 0), g()) > Fixed::ZERO,
            "floats on the denser lava"
        );
        assert!(
            buoyancy_at(&plan, &organs, &field, Coord3::ground(2, 0), g()) < Fixed::ZERO,
            "falls in thin air"
        );
    }

    #[test]
    fn buoyancy_is_deterministic() {
        let (organs, light, _dense) = registry_with_density_organs();
        let run = || {
            buoyancy(
                body_density(&body(vec![organ(light, (1, 1))]), &organs),
                Fixed::from_int(998),
                g(),
            )
            .to_bits()
        };
        assert_eq!(run(), run(), "the same body and medium replay bit for bit");
    }

    // In-medium thermal exchange (increment 2d, part A): a being's core temperature relaxes toward the
    // temperature of the medium it sits in, so lethal-hot and lethal-cold media are real. All physics,
    // no steering: the exchange carries no heading and treats hot and cold as mirror images.

    use crate::runner::{comfort_fraction, BeingThermal};

    /// A reserved thermal band fixture: a viable body-temperature band around a set point (the same
    /// shape the thermal coupling uses). Not owner canon.
    fn band() -> BeingThermal {
        BeingThermal {
            setpoint: Fixed::from_int(37),
            half_band: Fixed::from_int(8),
            initial_temp: Fixed::from_int(37),
        }
    }

    #[test]
    fn a_body_heats_toward_a_hot_medium_and_cools_toward_a_cold_one() {
        let rate = Fixed::from_ratio(1, 10);
        let start = Fixed::from_int(37);
        assert!(
            in_medium_temperature(start, Fixed::from_int(1200), rate) > start,
            "a hot medium heats the body toward it"
        );
        assert!(
            in_medium_temperature(start, Fixed::from_int(-50), rate) < start,
            "a cold medium cools the body toward it"
        );
        assert_eq!(
            in_medium_temperature(start, start, rate),
            start,
            "a medium at the body temperature drives no exchange (equilibrium)"
        );
    }

    #[test]
    fn a_hot_and_a_cold_medium_are_mirror_image_treatments() {
        // The anti-steer: an equal temperature gap above and below the body drives an equal and
        // opposite exchange, so the mechanism favours neither hot nor cold and bakes in no direction.
        let rate = Fixed::from_ratio(1, 5);
        let start = Fixed::from_int(37);
        let d = Fixed::from_int(200);
        let up = in_medium_temperature(start, start + d, rate) - start;
        let down = in_medium_temperature(start, start - d, rate) - start;
        assert_eq!(
            up,
            Fixed::ZERO - down,
            "hot and cold media are symmetric: no authored preference"
        );
    }

    #[test]
    fn a_lethal_hot_medium_kills_and_a_temperate_one_does_not() {
        // Physics in, death out: a body held in a lava medium heats out of its comfort band and dies
        // (comfort falls to zero), while a body in a temperate medium holds and lives. No authored rule
        // says lava is lethal; the thermal exchange plus the comfort band make it so.
        let rate = Fixed::from_ratio(1, 10);
        let band = band();

        let lava = Fixed::from_int(1200);
        let hot_body = in_medium_temperature(band.initial_temp, lava, rate);
        assert_eq!(
            comfort_fraction(hot_body, &band),
            Fixed::ZERO,
            "a lava medium drives the body past its band: lethal"
        );

        let temperate = band.setpoint;
        let held_body = in_medium_temperature(band.initial_temp, temperate, rate);
        assert!(
            comfort_fraction(held_body, &band) > Fixed::ZERO,
            "a temperate medium holds the body in its band: survivable"
        );

        // A lethal-cold medium is lethal too, the symmetric case.
        let void = Fixed::from_int(-200);
        let cold_body = in_medium_temperature(band.initial_temp, void, rate);
        assert_eq!(
            comfort_fraction(cold_body, &band),
            Fixed::ZERO,
            "a lethal-cold medium is lethal too, symmetrically"
        );
    }

    #[test]
    fn located_thermal_exchange_reads_the_cell_and_holds_off_the_field() {
        // A one-cell lava field. On the cell the body heats toward the lava; off the field there is no
        // medium and the body temperature holds.
        let rate = Fixed::from_ratio(1, 10);
        let start = Fixed::from_int(37);
        let field = MediumField::uniform(
            1,
            1,
            Fixed::ZERO,
            Fixed::from_int(3000),
            Fixed::from_int(1200),
        );
        assert!(
            in_medium_temperature_at(start, &field, Coord3::ground(0, 0), rate) > start,
            "on the lava cell the body heats"
        );
        assert_eq!(
            in_medium_temperature_at(start, &field, Coord3::ground(9, 9), rate),
            start,
            "off the field there is no medium, the body temperature holds"
        );
    }

    #[test]
    fn in_medium_thermal_exchange_is_deterministic() {
        let rate = Fixed::from_ratio(3, 10);
        let run =
            || in_medium_temperature(Fixed::from_int(37), Fixed::from_int(900), rate).to_bits();
        assert_eq!(run(), run(), "the same body and medium replay bit for bit");
    }
}
