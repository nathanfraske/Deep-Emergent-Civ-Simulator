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
use civsim_world::Coord3;

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
}

impl MediumField {
    /// A field from explicit per-cell respirable content and density (row-major, `width * height` each).
    pub fn new(
        width: i32,
        height: i32,
        respirable: Vec<Fixed>,
        density: Vec<Fixed>,
    ) -> MediumField {
        assert!(width > 0 && height > 0, "a field has positive extent");
        let n = (width as usize) * (height as usize);
        assert_eq!(respirable.len(), n, "respirable is width*height long");
        assert_eq!(density.len(), n, "density is width*height long");
        MediumField {
            width,
            height,
            respirable,
            density,
        }
    }

    /// A uniform medium filling the whole field (one medium everywhere).
    pub fn uniform(width: i32, height: i32, respirable: Fixed, density: Fixed) -> MediumField {
        let n = (width as usize) * (height as usize);
        MediumField::new(width, height, vec![respirable; n], vec![density; n])
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
        )
    }

    /// Run the located respiration-plus-metabolism loop at a cell, returning whether the being ended
    /// alive after the given ticks.
    fn survive_at(plan: &BodyPlan, organs: &BodyPlanRegistry, pos: Coord3, ticks: u32) -> bool {
        let reg = dev_respiration();
        let field = two_region_field();
        let cap = Fixed::from_int(1000);
        let mut h = Homeostasis::new(&reg, plan, organs);
        for _ in 0..ticks {
            respire_at(&mut h, plan, organs, &field, pos, Fixed::ONE, cap);
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
        assert!(
            survive_at(&plan, &organs, Coord3::ground(0, 0), 500),
            "in the rich cell it breathes and survives"
        );
        assert!(
            !survive_at(&plan, &organs, Coord3::ground(1, 0), 500),
            "in the poor cell the same body suffocates"
        );
    }

    #[test]
    fn off_the_field_there_is_no_medium_to_breathe() {
        // A being off the field finds no medium (respirable content zero) and suffocates on its buffer,
        // whatever its anatomy. The field's absence convention: no medium, no breath.
        let (organs, gill) = registry_with_gill();
        let plan = body(vec![organ(gill, (1, 1))]);
        assert!(
            !survive_at(&plan, &organs, Coord3::ground(99, 99), 500),
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
        let run = || survive_at(&plan, &organs, Coord3::ground(0, 0), 200);
        assert_eq!(
            run(),
            run(),
            "the same body, field, and cell replay bit for bit"
        );
    }
}
