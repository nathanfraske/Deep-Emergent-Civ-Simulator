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
}
