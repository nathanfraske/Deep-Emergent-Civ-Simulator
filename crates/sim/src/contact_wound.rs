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

//! The contact wound resolve (hunt-kill strike arc, piece 2): the geometry-derived contact area and the wound
//! a delivered energy inflicts on a struck [`crate::morphogen::Segment`] against that segment's OWN material,
//! computed from the physics floor. It is the sibling of the contact-energy-transfer registry
//! ([`crate::contact_transfer`]): piece 1 resolves HOW MUCH energy a contact delivers (by the acting part's own
//! channel and body), piece 2 resolves the WOUND that energy makes (against the struck part's own material and
//! geometry). Both are pure and off the run path; the strike wire consumes them, so this is byte-neutral by
//! construction.
//!
//! The strike framing panel caught the seam this resolve fixes (seam 3). The pre-strike wound path took a
//! contact area AND a categorical damage mode (CUT, PIERCE, BLUNT, BURN) as PASSED parameters the caller chose,
//! so the wound-shape was a selector authored one level down rather than a fact of the acting and struck bodies.
//! The gate ruled the physical quantity is the energy the contact concentrates over its area against the struck
//! segment's own resistance: `failure_tolerance = fracture_energy * contact_area` (the energy to fully fracture
//! the contacted patch), and the wound is the fraction of that tolerance the delivered energy reaches. The
//! contact area is DERIVED from the acting part's own geometry (its presented bearing patch), and the "mode" is
//! a DESCRIPTION of where the strike falls on that continuous area axis, never a category passed in: a
//! concentrated (small-area) contact reaches a large fraction and cuts deep, a spread (large-area) contact
//! reaches a small fraction and bruises, by the same law. CUT, PIERCE, and BLUNT are narration labels for
//! regions of the contact-area continuum this fraction already spans; a thermal or other non-mechanical channel
//! is a different transfer kernel (piece 1), never a region of this area axis. So the mode emerges from geometry
//! and is never a per-part branch (Principles 8, 9, 11).
//!
//! This resolve is the one currency the run path already fractures matter by: the energy limb of
//! [`civsim_physics::laws::fracture_onset`] (delivered energy against `mat.fracture_energy` times the struck
//! area), the same criterion [`crate::runner::Runner::strike_underfoot`] fractures a cell's constituents with,
//! so a struck being is wounded by exactly the law a struck rock is broken by, keyed on each material's own
//! resistance. It admits the alien as data: the resistance is the struck segment's OWN `mat.fracture_energy`
//! (a tougher body is harder to wound because its material reads tougher, never a per-species number), and the
//! delivered energy arrives through piece 1's channel kernel (a massless energy-being wounds through a different
//! kernel, a data row), so nothing here reads a kingdom, chemistry, or body plan. The severity is a scale-free
//! FRACTION in `[0, ONE]`; converting it to the run-path damage accumulator's units (and writing it) is the
//! held final step of the arc, sequenced with the merged accumulator, not this substrate.
//!
//! FLAGGED FOLLOW-ON (surfaced by the section-9 audit): the DEFENDER-resistance law is a single mechanical
//! Griffith fracture (delivered energy against `mat.fracture_energy` times contact area), NOT yet kernel-
//! dispatched the way piece 1's DELIVERY law is ([`crate::contact_transfer::TransferKernel`]). So a being whose
//! integrity resists by a non-Griffith law (a field, plasma, or mana body, the same alien piece 1's channel set
//! anticipates on the attack side) would today need a code change rather than a data row, and the
//! absence convention defaults a body carrying no `mat.fracture_energy` to maximally fragile. The symmetric fix
//! is a resistance-kernel registry, the sibling of the transfer-kernel registry, so a non-mechanical integrity
//! law is a data row plus one floor kernel; it is a floor extension coupled to the non-kinetic delivery kernels,
//! not this first cut (the floor carries only the Griffith law today, and the delivery set carries only kinetic).

use civsim_core::Fixed;

use crate::morphogen::Segment;

/// The physics-floor geometry axis a segment's presented bearing (contact) area is read from. The same axis
/// [`crate::runner::Runner::strike_underfoot`] reads a wielded tool's struck face from, so an acting body part
/// and a wielded tool present a contact patch by the identical convention.
pub const CONTACT_AREA_AXIS: &str = "mech.contact_area";

/// The physics-floor material axis a struck segment's fracture resistance (its Griffith energy per crack area,
/// J/m^2) is read from. The same axis the run-path matter fracture reads, so a struck being resists a wound by
/// the identical law a struck rock resists breaking.
pub const FRACTURE_ENERGY_AXIS: &str = "mat.fracture_energy";

/// The contact area an acting segment presents, DERIVED from the segment's own grown geometry: its
/// [`CONTACT_AREA_AXIS`] bearing patch. Keyed on the acting part's own body (a part grows a smaller or larger
/// bearing patch, so it concentrates or spreads a contact by its own geometry, never a passed number). A segment
/// that carries no contact-area axis presents zero (the substrate-absence convention the [`Segment`] accessors
/// use), a perfectly concentrated point contact, which the wound resolve routes to a full-tolerance wound below.
pub fn presented_contact_area(acting: &Segment) -> Fixed {
    acting.geo(CONTACT_AREA_AXIS)
}

/// The energy that fully fractures the contacted patch: the struck material's fracture energy over the contact
/// area (`fracture_energy * contact_area`, the Griffith energy limb of [`civsim_physics::laws::fracture_onset`]), capped at the
/// physics-floor representability ceiling. This is the DENOMINATOR the wound fraction is taken against, exposed
/// on its own so the eventual (held) damage-accumulator write shares the exact tolerance the fraction used. A
/// zero product (no declared resistance or a point contact) is zero tolerance: any delivered energy fully wounds
/// it, matching the run-path target-absence convention (a constituent with no fracture energy is shattered by
/// any blow). An overflowing product routes to the ceiling, a large but finite tolerance.
pub fn failure_tolerance(fracture_energy: Fixed, contact_area: Fixed, energy_max: Fixed) -> Fixed {
    match fracture_energy.checked_mul(contact_area) {
        Some(g) => g.min(energy_max),
        None => energy_max,
    }
}

/// The wound a delivered energy makes on a struck patch: the FRACTION of the patch's [`failure_tolerance`] the
/// delivered energy reaches, in `[0, ONE]`. One is a full fracture (the delivered energy meets or exceeds the
/// tolerance, the boundary the energy limb of [`civsim_physics::laws::fracture_onset`] fires at); a smaller value is a partial
/// wound. Pure and scale-free: the caller supplies `delivered_energy` (piece 1's resolved contact energy) on the
/// same scale as the struck material's fracture energy, and this returns the dimensionless severity, so the
/// substrate carries no unit bridge of its own (converting the fraction to the run-path damage accumulator's
/// units is the held final step). Zero tolerance routes to a full wound for any positive delivered energy (the
/// absence convention) and to no wound for zero energy.
pub fn wound_fraction(
    delivered_energy: Fixed,
    contact_area: Fixed,
    fracture_energy: Fixed,
    energy_max: Fixed,
) -> Fixed {
    let tol = failure_tolerance(fracture_energy, contact_area, energy_max);
    if tol <= Fixed::ZERO {
        return if delivered_energy > Fixed::ZERO {
            Fixed::ONE
        } else {
            Fixed::ZERO
        };
    }
    match delivered_energy.checked_div(tol) {
        Some(f) => f.clamp(Fixed::ZERO, Fixed::ONE),
        // A delivered energy so large the ratio overflows is far past the tolerance: a full wound.
        None => Fixed::ONE,
    }
}

/// Resolve the wound one acting segment's delivered energy makes on a struck segment, reading the contact area
/// off the ACTING part's own geometry and the resistance off the STRUCK part's own material. A convenience over
/// [`presented_contact_area`] and [`wound_fraction`] that proves the full geometry-and-material read against real
/// [`Segment`]s. Returns the severity FRACTION in `[0, ONE]`; the (held) accumulator write consumes it. Blind to
/// any id: it reads only the two segments' own physics, so it wounds a being, a plant, or a boulder by the same
/// call, "this was combat" a description of which segment was struck, never a branch (Principle 8).
pub fn resolve_wound(
    acting: &Segment,
    struck: &Segment,
    delivered_energy: Fixed,
    energy_max: Fixed,
) -> Fixed {
    let contact_area = presented_contact_area(acting);
    let fracture_energy = struck.mat(FRACTURE_ENERGY_AXIS);
    wound_fraction(delivered_energy, contact_area, fracture_energy, energy_max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_physics::laws;
    use std::collections::BTreeMap;

    /// A segment carrying a single geometry axis, for the acting-part contact-area read.
    fn seg_geo(axis: &str, v: Fixed) -> Segment {
        let mut geometry = BTreeMap::new();
        geometry.insert(axis.to_string(), v);
        Segment {
            parent: None,
            depth: 0,
            geometry,
            material: BTreeMap::new(),
            damage: Fixed::ZERO,
        }
    }

    /// A segment carrying a single material axis, for the struck-part resistance read.
    fn seg_mat(axis: &str, v: Fixed) -> Segment {
        let mut material = BTreeMap::new();
        material.insert(axis.to_string(), v);
        Segment {
            parent: None,
            depth: 0,
            geometry: BTreeMap::new(),
            material,
            damage: Fixed::ZERO,
        }
    }

    fn cap() -> Fixed {
        Fixed::from_int(1_000_000)
    }

    #[test]
    fn contact_area_is_read_off_the_acting_segments_own_geometry() {
        let acting = seg_geo(CONTACT_AREA_AXIS, Fixed::from_ratio(1, 100));
        assert_eq!(presented_contact_area(&acting), Fixed::from_ratio(1, 100));
        // A segment with no contact-area axis presents zero (the absence convention): a point contact.
        let bare = seg_geo("mech.arm_length", Fixed::from_int(1));
        assert_eq!(presented_contact_area(&bare), Fixed::ZERO);
    }

    #[test]
    fn failure_tolerance_is_fracture_energy_over_the_contact_area() {
        let fe = Fixed::from_int(50);
        let area = Fixed::from_ratio(1, 10);
        // The tolerance is the Griffith energy limb's g_avail: fracture_energy * contact_area, capped.
        assert_eq!(
            failure_tolerance(fe, area, cap()),
            fe.checked_mul(area).unwrap().min(cap()),
        );
        // No declared resistance is zero tolerance (shattered by any blow, the absence convention).
        assert_eq!(failure_tolerance(Fixed::ZERO, area, cap()), Fixed::ZERO);
        // A point contact (zero area) is zero tolerance too.
        assert_eq!(failure_tolerance(fe, Fixed::ZERO, cap()), Fixed::ZERO);
    }

    #[test]
    fn the_wound_is_the_fraction_of_the_tolerance_the_energy_reaches() {
        let fe = Fixed::from_int(10);
        let area = Fixed::from_int(1);
        let tol = failure_tolerance(fe, area, cap()); // 10
                                                      // Half the tolerance delivered is a half-severity wound.
        let half = wound_fraction(Fixed::from_int(5), area, fe, cap());
        assert_eq!(half, Fixed::from_ratio(1, 2));
        // The full tolerance (and beyond) is a full wound, clamped at ONE, matching the fracture_onset boundary.
        assert_eq!(wound_fraction(tol, area, fe, cap()), Fixed::ONE);
        assert_eq!(
            wound_fraction(
                tol.checked_mul(Fixed::from_int(3)).unwrap(),
                area,
                fe,
                cap()
            ),
            Fixed::ONE
        );
        // No delivered energy is no wound.
        assert_eq!(wound_fraction(Fixed::ZERO, area, fe, cap()), Fixed::ZERO);
    }

    #[test]
    fn the_full_wound_boundary_matches_the_floor_fracture_criterion() {
        // The severity reaches ONE exactly when the delivered energy passes the energy limb of fracture_onset
        // (its energy margin crosses below zero): the wound resolve and the floor criterion agree on the
        // fracture boundary, one currency.
        let fe = Fixed::from_int(10);
        let area = Fixed::from_int(1);
        let just_under = Fixed::from_ratio(999, 100); // 9.99 < tolerance 10
        let just_over = Fixed::from_ratio(1001, 100); // 10.01 > tolerance 10
        let margin = |e: Fixed| {
            let (_, energy_margin) = laws::fracture_onset(Fixed::ZERO, fe, fe, area, e, cap());
            energy_margin
        };
        assert!(
            margin(just_under) > Fixed::ZERO
                && wound_fraction(just_under, area, fe, cap()) < Fixed::ONE
        );
        assert!(
            margin(just_over) < Fixed::ZERO
                && wound_fraction(just_over, area, fe, cap()) == Fixed::ONE
        );
    }

    #[test]
    fn a_concentrated_contact_wounds_deeper_than_a_spread_one_by_geometry_alone() {
        // Same delivered energy, same struck material: a smaller presented contact area concentrates the energy
        // and reaches a larger fraction of the tolerance, so the wound-shape (deep-and-narrow versus
        // shallow-and-wide) EMERGES from the geometry-derived area, never a passed mode.
        let fe = Fixed::from_int(20);
        let energy = Fixed::from_int(5);
        let sharp = wound_fraction(energy, Fixed::from_ratio(1, 100), fe, cap());
        let blunt = wound_fraction(energy, Fixed::from_ratio(1, 2), fe, cap());
        assert!(sharp > blunt && blunt > Fixed::ZERO);
    }

    #[test]
    fn a_tougher_struck_material_takes_a_lesser_wound_from_the_same_blow() {
        // Same acting geometry and delivered energy: the struck segment's OWN fracture energy sets how much it
        // is wounded, so a tougher body resists by its own material and never by a per-species number.
        let acting = seg_geo(CONTACT_AREA_AXIS, Fixed::from_int(1));
        let energy = Fixed::from_int(5);
        let soft = seg_mat(FRACTURE_ENERGY_AXIS, Fixed::from_int(10));
        let tough = seg_mat(FRACTURE_ENERGY_AXIS, Fixed::from_int(200));
        let w_soft = resolve_wound(&acting, &soft, energy, cap());
        let w_tough = resolve_wound(&acting, &tough, energy, cap());
        assert!(w_soft > w_tough && w_tough > Fixed::ZERO);
    }

    #[test]
    fn the_resolve_is_deterministic_and_blind_to_the_struck_segments_kind() {
        // Identical inputs give the identical bit-exact fraction (Principle 3), and the call reads only the two
        // segments' own physics, so a being and a boulder with the same struck material take the same wound.
        let acting = seg_geo(CONTACT_AREA_AXIS, Fixed::from_int(1));
        let struck = seg_mat(FRACTURE_ENERGY_AXIS, Fixed::from_int(30));
        let energy = Fixed::from_int(4);
        let a = resolve_wound(&acting, &struck, energy, cap());
        let b = resolve_wound(&acting, &struck, energy, cap());
        assert_eq!(a, b);
        assert!(a > Fixed::ZERO && a < Fixed::ONE);
    }
}
