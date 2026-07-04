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

//! Derived resting metabolism and body-to-medium thermal exchange (design Part 15, Part 20, Part 35,
//! Part 41; R-METABOLIZE; Principles 3, 9, 11). The substrate that frees the authored
//! `base_metabolic_drain`, `exertion_drain_coupling`, and `field.body_exchange` scalars: the drain a
//! body pays at rest and the rate its core temperature couples to a medium DERIVE from the body's mass
//! and tissue against the physics floor, never a per-axis authored number.
//!
//! What is authored is physics. The resting metabolic power is Kleiber's law `P = a * m^(3/4)`
//! ([`civsim_physics::laws::basal_metabolic_rate`]): the 3/4 exponent is a universal physics affordance
//! (West, Brown, and Enquist's fractal-network derivation, holding across taxa; Principle 9 permits
//! authored physics), evaluated by the exact two-square-root fixed-point identity so no exp/ln is
//! touched. The thermoregulatory replacement is the resolved convective and radiant heat-loss over the
//! body's exposed surface ([`civsim_physics::laws::resting_heat_loss`]). The body-to-medium coupling is
//! `h * A / (m * c)`, the discrete Newton-cooling rate from the medium coefficient, the surface area,
//! and the body's thermal mass.
//!
//! What is not authored is the outcome. A being's exchange area, thermal mass, and reserve energy are
//! composition-derived reads over its organs (the same development-weighted-sum shape the metabolic
//! reserves and the respiratory surface use, [`crate::homeostasis`], [`crate::medium`]), so two bodies
//! diverge in their derived drain and coupling from their composition, mass, medium, and temperature
//! alone. Nothing here reads a race identity: a hot-set-point body in a cold medium and its mirror
//! differ because their temperatures differ, not because of a label (Principle 9).
//!
//! Everything is integer, fixed-point, and draws no randomness (Principle 3). The owner anchors (the
//! Kleiber coefficient `a`, the normalized-body-mass-to-kilograms bridge, the medium convective
//! coefficient `h`, the surface emissivity, and the Stefan-Boltzmann constant) are reserved with their
//! basis and are the owner's to set ([`MetabolicAnchors::from_manifest`]); the values in
//! [`MetabolicAnchors::dev_fixture`] are labelled development fixtures, never owner canon. The caps
//! below are representability bounds forced by Q32.32 (the engine-mechanics exemption the law kernels
//! and `medium.rs` take), not owner realism values.
//!
//! Two honest limits stand. First, the exact reconciliation of the reserve's stored energy (the biology
//! floor's `bio.energy_density` in kJ/g, the reserve capacity, and the body mass) to joules comparable
//! with a watt-tick spend is the R-UNITS-PIN owner units bridge: the mechanism derives the drain, the
//! absolute scale is the owner's anchors and the floor's units. Second, and this is a genuine
//! cross-tier difference rather than a defect, the base drain is NONLINEAR in mass (`m^(3/4)`) and in
//! temperature (`T^4` through the radiant loss), so the drain of a pool over a size-and-temperature
//! distribution is NOT the drain of the mean size (a Jensen gap): a coarse aggregate tier that reads
//! the mean body loses the convexity. This is the honest cross-tier difference `docs/design.md:2803`
//! already declares for a nonlinear law output over additive quantities. Do NOT silently substitute the
//! mean; the R-TIER-CONSIST reconciliation (carry a size-distribution moment into the pool-tier drain,
//! or accept and document the gap) is the named follow-on, not resolved here.

use civsim_core::Fixed;
use civsim_physics::laws;

use crate::anatomy::{BodyPlan, BodyPlanRegistry};
use crate::calibration::{CalibrationError, CalibrationManifest};

/// The biology-floor axis a tissue carries its body-to-medium exchange surface on
/// (`crates/physics/data/biology_floor.toml`), the area the heat-loss and coupling laws integrate over.
/// A tissue with none of it presents no exchange surface (the absence convention).
pub const CONVECTIVE_SURFACE: &str = "bio.convective_surface";
/// The mechanical-floor axis a tissue carries its density on (`mat.density`, kg/m^3), reused from
/// [`crate::medium`].
pub const TISSUE_DENSITY: &str = "mat.density";
/// The mechanical-floor axis a tissue carries its specific heat on (`therm.specific_heat`, J/(kg*K)),
/// the per-unit-mass heat capacity the body's thermal mass reads.
pub const TISSUE_SPECIFIC_HEAT: &str = "therm.specific_heat";
/// The biology-floor axis a tissue carries its gross energy density on (`bio.energy_density`), the
/// reserve's per-unit specific energy.
pub const ENERGY_DENSITY: &str = "bio.energy_density";
/// The mechanical-floor axis a tissue carries its material strength on (`mat.fracture_strength`, the
/// same axis the individual-tier [`crate::body::Body::strength`] reads, design Part 35), the strength
/// per unit of the tissue the whole-body work force integrates over. A tissue with none of it provides
/// no muscle force (the absence convention).
pub const MUSCLE_STRENGTH: &str = "mat.fracture_strength";

/// A representability cap for the basal metabolic rate (W). Engine-mechanics bound, not an owner value.
const RATE_MAX: Fixed = Fixed::from_int(1_000_000_000);
/// A representability cap for the thermoregulatory heat-loss flux (W). Engine-mechanics bound.
const FLUX_MAX: Fixed = Fixed::from_int(1_000_000_000);
/// A representability cap for the mechanical work power (W, the `laws::power_watts` scale, matching the
/// watt-scale basal rate the exertion coupling is summed with). Engine-mechanics.
const POWER_MAX: Fixed = Fixed::from_int(1_000_000_000);
/// The drain-fraction cap: a reserve cannot lose more than its whole capacity in one tick, so the
/// derived fraction is bounded to one. A physical bound, not an owner value.
const FRAC_MAX: Fixed = Fixed::ONE;

/// The reserved owner anchors the derived metabolism needs, surfaced with their basis and fail-loud in
/// the manifest, never fabricated (Principle 11). The kernels are fixed Rust; these are the owner's to
/// set. Read on a canonical run through [`MetabolicAnchors::from_manifest`]; the dev fixture is a
/// labelled test stand-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetabolicAnchors {
    /// The Kleiber coefficient `a` in `P = a * m^(3/4)` (W per kg^(3/4)). RESERVED owner anchor.
    pub kleiber_a: Fixed,
    /// The kilograms a body carries at `body_mass = 1` (the normalized-trait-to-kilograms bridge). An
    /// R-UNITS-PIN bridge, NOT derivable. RESERVED owner anchor.
    pub body_mass_kg_scale: Fixed,
    /// The medium convective coefficient `h` (W/(m^2*K)) for the body-to-medium exchange, a fluids-floor
    /// medium datum. RESERVED owner anchor.
    pub medium_h: Fixed,
    /// The body-surface emissivity for the radiant thermoregulatory term, dimensionless in [0, 1]. A
    /// surface property (~0.95 for biological tissue). RESERVED.
    pub emissivity: Fixed,
    /// The Stefan-Boltzmann constant sigma (W/(m^2*K^4)), a universal physical constant passed like the
    /// other physics constants the radiant law reads. RESERVED (a CODATA constant, an authored
    /// Principle-9 physics affordance).
    pub sigma: Fixed,
}

impl MetabolicAnchors {
    /// The anchors read from the calibration manifest, fail-loud if any is still reserved (Principle 11,
    /// the reserved-value discipline). This is the sanctioned way to obtain the anchors on a canonical
    /// run; there is no default, so an unset value refuses to run rather than fabricating a number.
    pub fn from_manifest(
        manifest: &CalibrationManifest,
    ) -> Result<MetabolicAnchors, CalibrationError> {
        Ok(MetabolicAnchors {
            kleiber_a: manifest.require_fixed("metabolism.kleiber_coefficient")?,
            body_mass_kg_scale: manifest.require_fixed("metabolism.body_mass_kg_scale")?,
            medium_h: manifest.require_fixed("metabolism.medium_convective_coefficient")?,
            emissivity: manifest.require_fixed("metabolism.surface_emissivity")?,
            sigma: manifest.require_fixed("metabolism.stefan_boltzmann")?,
        })
    }

    /// A labelled DEVELOPMENT FIXTURE, not owner canon: a plausible temperate-mammal Kleiber coefficient,
    /// a mid-size kilogram bridge, an air convective coefficient, a biological-tissue emissivity, and the
    /// CODATA Stefan-Boltzmann constant. For tests and examples only; a canonical run reads
    /// [`MetabolicAnchors::from_manifest`].
    pub fn dev_fixture() -> MetabolicAnchors {
        MetabolicAnchors {
            kleiber_a: Fixed::from_ratio(1, 100),
            body_mass_kg_scale: Fixed::from_int(100),
            medium_h: Fixed::from_int(10),
            emissivity: Fixed::from_ratio(95, 100),
            sigma: Fixed::from_ratio(567, 10_000_000_000),
        }
    }
}

/// A being's whole-body convective exchange area: the development-weighted sum over its organs of their
/// `bio.convective_surface` composition. The same composition-derived shape [`crate::medium::exchange_area`]
/// uses for the respiratory surface, so a body's ability to shed heat follows its anatomy: a body with no
/// exchange-surface tissue presents zero area and couples to no medium convectively. The sum is the
/// order-independent [`Fixed::saturating_add`], so it is invariant to organ order.
pub fn whole_body_surface(plan: &BodyPlan, organs: &BodyPlanRegistry) -> Fixed {
    let mut sum = Fixed::ZERO;
    for organ in &plan.organs {
        let surface = organs
            .organ_composition(organ.kind)
            .map(|c| c.component(CONVECTIVE_SURFACE))
            .unwrap_or(Fixed::ZERO);
        let area = organ
            .development
            .checked_mul(surface)
            .unwrap_or(Fixed::ZERO);
        sum = sum.saturating_add(area);
    }
    sum
}

/// A being's whole-body muscle work force (design Part 35, real-world unification step 5): the
/// development-weighted sum over its organs of their `mat.fracture_strength` composition, times the
/// body's mass in kilograms, mirroring the individual-tier [`crate::body::Body::strength`] (muscle mass
/// times material strength) at the body-plan tier so the two tiers stay dimensionally consistent (owner
/// ruling 2026-07-04). This replaces the raw `body_mass` proxy the exertion coupling read: a body's
/// exertion drain now follows its actual muscle endowment scaled by its size, so two bodies of equal mass
/// but different muscle composition exert different force, and two bodies of equal muscle composition but
/// different mass exert different force too (the mass scaling the raw proxy carried is kept, not dropped).
/// It reads the composition axis, never a specific tissue-material id or a race label (Principle 9): a
/// body whose tissue declares no strength reads ZERO (the absence convention its siblings use), not a
/// mass-sized default, so the exertion coupling falls to its no-force branch rather than a hidden proxy.
/// The sum is the order-independent [`Fixed::saturating_add`], so it is invariant to organ order, and the
/// mass bridge is the reserved kilogram scale the metabolic derivations already read.
pub fn whole_body_muscle_force(
    plan: &BodyPlan,
    organs: &BodyPlanRegistry,
    anchors: &MetabolicAnchors,
) -> Fixed {
    let mut sum = Fixed::ZERO;
    for organ in &plan.organs {
        let strength = organs
            .organ_composition(organ.kind)
            .map(|c| c.component(MUSCLE_STRENGTH))
            .unwrap_or(Fixed::ZERO);
        let force = organ
            .development
            .checked_mul(strength)
            .unwrap_or(Fixed::ZERO);
        sum = sum.saturating_add(force);
    }
    sum.checked_mul(body_mass_kg(plan, anchors))
        .unwrap_or(Fixed::ZERO)
}

/// A being's whole-body specific heat (J/(kg*K)): the development-weighted average over its organs of
/// their `therm.specific_heat` composition, or ZERO if no organ declares one (the absence convention
/// its siblings [`whole_body_surface`] and [`whole_body_energy_density`] use). The same
/// composition-average shape [`crate::medium::body_density`] uses, so the body's thermal mass follows
/// its tissue rather than a hidden terran-water default: a body whose tissue carries no specific heat
/// has no defined thermal mass, and the body-to-medium coupling then falls to its own
/// no-thermal-mass branch ([`derive_body_exchange_rate`]) rather than converging on the specific heat
/// of water (Principle 9: no terran constant on the content path). Order-independent (saturating
/// sums, one checked division).
pub fn whole_body_specific_heat(plan: &BodyPlan, organs: &BodyPlanRegistry) -> Fixed {
    let mut weighted = Fixed::ZERO;
    let mut total_dev = Fixed::ZERO;
    for organ in &plan.organs {
        let c = organs
            .organ_composition(organ.kind)
            .map(|comp| comp.component(TISSUE_SPECIFIC_HEAT))
            .unwrap_or(Fixed::ZERO);
        if c > Fixed::ZERO {
            let contribution = organ.development.checked_mul(c).unwrap_or(Fixed::ZERO);
            weighted = weighted.saturating_add(contribution);
            total_dev = total_dev.saturating_add(organ.development);
        }
    }
    if total_dev <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    weighted.checked_div(total_dev).unwrap_or(Fixed::ZERO)
}

/// A being's whole-body energy density: the development-weighted average over its organs of their
/// `bio.energy_density` composition, the reserve's per-unit specific energy the drain bridge multiplies
/// by the reserve capacity to reach the stored joules. A body with no energy-dense tissue reads zero
/// (no stored energy, so the resting demand drains its reserve fully, the no-energy-organ death the
/// physiology already models). Order-independent.
pub fn whole_body_energy_density(plan: &BodyPlan, organs: &BodyPlanRegistry) -> Fixed {
    let mut weighted = Fixed::ZERO;
    let mut total_dev = Fixed::ZERO;
    for organ in &plan.organs {
        let d = organs
            .organ_composition(organ.kind)
            .map(|comp| comp.component(ENERGY_DENSITY))
            .unwrap_or(Fixed::ZERO);
        if d > Fixed::ZERO {
            let contribution = organ.development.checked_mul(d).unwrap_or(Fixed::ZERO);
            weighted = weighted.saturating_add(contribution);
            total_dev = total_dev.saturating_add(organ.development);
        }
    }
    if total_dev <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    weighted.checked_div(total_dev).unwrap_or(Fixed::ZERO)
}

/// The body's mass in kilograms: the normalized `body_mass` trait times the reserved kilogram bridge.
/// An overflowing product routes to zero (an unrepresentably huge body has no meaningful metabolism
/// here), matching the law kernels' degenerate-input convention.
fn body_mass_kg(plan: &BodyPlan, anchors: &MetabolicAnchors) -> Fixed {
    plan.body_mass
        .checked_mul(anchors.body_mass_kg_scale)
        .unwrap_or(Fixed::ZERO)
}

/// The derived resting drain FRACTION of the energy reserve per tick, composing the physics laws: the
/// Kleiber basal rate over the body mass plus the thermoregulatory heat loss over the whole-body surface
/// (the body held at its resting set point against the ambient medium), bridged to a fraction of the
/// reserve's stored energy. This replaces the authored `base_metabolic_drain`: two bodies diverge from
/// mass, tissue, medium, and temperature alone. `energy_capacity` is the being's energy-reserve capacity
/// (the caller passes `homeostasis.capacity(ENERGY)`); `tick` is the tick length in seconds.
#[allow(clippy::too_many_arguments)]
pub fn derive_base_drain(
    plan: &BodyPlan,
    organs: &BodyPlanRegistry,
    energy_capacity: Fixed,
    ambient_temp: Fixed,
    setpoint: Fixed,
    medium_h: Fixed,
    tick: Fixed,
    anchors: &MetabolicAnchors,
) -> Fixed {
    let mass_kg = body_mass_kg(plan, anchors);
    let basal = laws::basal_metabolic_rate(mass_kg, anchors.kleiber_a, RATE_MAX);
    // At rest the body holds its set point; the thermoregulatory demand is the heat shed to the medium
    // at that core temperature over the body's exposed surface.
    let area = whole_body_surface(plan, organs);
    let heat_loss = laws::resting_heat_loss(
        medium_h,
        area,
        setpoint,
        ambient_temp,
        anchors.emissivity,
        anchors.sigma,
        FLUX_MAX,
    );
    // The reserve's energy-storing mass: the anatomy-derived reserve capacity scaled to the body's
    // physical mass, so the bridge to stored joules scales with size (a larger body stores proportionally
    // more absolute energy). The exact kJ/g-to-joule reconciliation of the reserve units is the
    // R-UNITS-PIN owner calibration (the honest units limit); the mechanism derives, the absolute scale
    // is the owner's anchors and the floor's energy-density units.
    let reserve_mass = energy_capacity.checked_mul(mass_kg).unwrap_or(Fixed::ZERO);
    let energy_density = whole_body_energy_density(plan, organs);
    laws::metabolic_drain_fraction(
        basal,
        heat_loss,
        reserve_mass,
        energy_density,
        tick,
        FRAC_MAX,
    )
}

/// The derived exertion drain coupling: the added fraction of the energy reserve drained per tick per
/// unit of exertion, from the mechanical work power a full-exertion body sustains (`force * velocity`),
/// bridged to a reserve fraction. This replaces the authored `exertion_drain_coupling`;
/// [`crate::homeostasis::Homeostasis::metabolize_derived`] scales it by the being's exertion signal
/// and ADDS it to the base drain, so the two must share one power scale. It reads the work power on
/// the WATT scale ([`civsim_physics::laws::power_watts`]), the same scale the basal rate and the
/// `metabolic_drain_fraction` bridge use, so the base and exertion fractions are commensurate rather
/// than off by the kilowatt factor (the earlier `laws::power` returned kilowatts, making the summed
/// exertion term a thousand times too small).
pub fn derive_exertion_coupling(
    plan: &BodyPlan,
    organs: &BodyPlanRegistry,
    energy_capacity: Fixed,
    force: Fixed,
    velocity: Fixed,
    tick: Fixed,
    anchors: &MetabolicAnchors,
) -> Fixed {
    let work_power = laws::power_watts(force, velocity, POWER_MAX);
    // The same size-scaled reserve-energy bridge as the base drain (see derive_base_drain).
    let reserve_mass = energy_capacity
        .checked_mul(body_mass_kg(plan, anchors))
        .unwrap_or(Fixed::ZERO);
    let energy_density = whole_body_energy_density(plan, organs);
    laws::metabolic_drain_fraction(
        work_power,
        Fixed::ZERO,
        reserve_mass,
        energy_density,
        tick,
        FRAC_MAX,
    )
}

/// The derived body-to-medium thermal coupling rate per tick: `h * A / (m * c)`, the discrete
/// Newton-cooling rate that governs `new_temp = temp + rate * (medium_temp - temp)`. `h` is the medium
/// convective coefficient (a fluids-floor datum), `A` the whole-body convective surface, and `m * c` the
/// body's thermal mass (its mass in kilograms times its whole-body specific heat). This replaces the
/// authored `field.body_exchange`: a high-surface, low-thermal-mass body couples fast; a compact, dense
/// one couples slowly, from the physics alone. Clamped to `[0, 1]` for the explicit scheme's stability
/// (rate 1 is instant equilibration; a rate above 1 would overshoot). A body with no exchange surface
/// (or in a medium with no coupling) reads zero: no surface, no convective exchange.
pub fn derive_body_exchange_rate(
    plan: &BodyPlan,
    organs: &BodyPlanRegistry,
    medium_h: Fixed,
    tick: Fixed,
    anchors: &MetabolicAnchors,
) -> Fixed {
    let area = whole_body_surface(plan, organs);
    let ha = match medium_h.checked_mul(area) {
        Some(x) => x,
        None => return Fixed::ONE,
    };
    if ha <= Fixed::ZERO {
        // No exchange surface (or no medium coupling): no convective exchange, the body holds its heat.
        return Fixed::ZERO;
    }
    let mass_kg = body_mass_kg(plan, anchors);
    let c = whole_body_specific_heat(plan, organs);
    let mc = match mass_kg.checked_mul(c) {
        Some(x) => x,
        // An enormous thermal mass barely responds over one tick.
        None => return Fixed::ZERO,
    };
    if mc <= Fixed::ZERO {
        // A massless (heat-capacity-less) body equilibrates instantly.
        return Fixed::ONE;
    }
    let per_second = match ha.checked_div(mc) {
        Some(x) => x,
        None => return Fixed::ONE,
    };
    per_second
        .checked_mul(tick)
        .unwrap_or(Fixed::ONE)
        .clamp(Fixed::ZERO, Fixed::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{OrganKindDef, Part, Temperament, TissueComposition};
    use crate::homeostasis::{Homeostasis, HomeostaticRegistry, ENERGY};

    fn temperament() -> Temperament {
        Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(1, 2),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        }
    }

    fn organ(kind: u16, dev: (i64, i64)) -> Part {
        Part {
            kind,
            development: Fixed::from_ratio(dev.0, dev.1),
        }
    }

    /// A body of a given mass bearing the given organs (locomotion irrelevant here).
    fn body(mass: (i64, i64), organs: Vec<Part>) -> BodyPlan {
        BodyPlan {
            body_mass: Fixed::from_ratio(mass.0, mass.1),
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

    /// A registry whose organ set adds a skin/covering tissue bearing convective surface, a
    /// dense-and-heat-capacious tissue (density and specific heat), and an energy tissue, at known ids,
    /// alongside the default organs. Labelled fixtures.
    fn registry() -> (BodyPlanRegistry, u16, u16, u16) {
        let mut reg = BodyPlanRegistry::dev_default();
        let skin = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: skin,
            name: "skin".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(CONVECTIVE_SURFACE, Fixed::from_int(2))]),
        });
        let flesh = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: flesh,
            name: "flesh".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[
                (TISSUE_DENSITY, Fixed::from_int(1000)),
                (TISSUE_SPECIFIC_HEAT, Fixed::from_int(3500)),
            ]),
        });
        let fat = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: fat,
            name: "energy-store".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(ENERGY_DENSITY, Fixed::ONE)]),
        });
        (reg, skin, flesh, fat)
    }

    #[test]
    fn whole_body_surface_follows_the_convective_tissue_and_is_zero_without_it() {
        let (organs, skin, _flesh, fat) = registry();
        // A body with a full skin presents more surface than one with a quarter skin.
        let big = whole_body_surface(&body((1, 2), vec![organ(skin, (1, 1))]), &organs);
        let small = whole_body_surface(&body((1, 2), vec![organ(skin, (1, 4))]), &organs);
        assert!(big > small, "more convective tissue, more exchange area");
        assert_eq!(
            big,
            Fixed::from_int(2),
            "full skin (dev 1 * surface 2) = 2 m^2"
        );
        // A body with only an energy organ (no convective surface) presents zero area.
        assert_eq!(
            whole_body_surface(&body((1, 2), vec![organ(fat, (1, 1))]), &organs),
            Fixed::ZERO,
            "no convective tissue, no exchange area"
        );
    }

    #[test]
    fn whole_body_muscle_force_follows_the_strength_tissue_and_mass_and_is_zero_without_it() {
        // Real-world unification step 5: the work force a body exerts is its development-weighted muscle
        // strength times its mass, mirroring the individual-tier Body::strength (muscle mass times
        // material strength). A muscle-bearing tissue provides force to the extent of its development; a
        // body with none exerts zero (the absence convention); two equal-mass bodies with different
        // muscle endowment exert different force; and two bodies of equal muscle but different mass exert
        // different force (the mass scaling the earlier raw proxy carried, kept by the ruling).
        let anchors = MetabolicAnchors::dev_fixture(); // body_mass_kg_scale = 100
        let mut reg = BodyPlanRegistry::dev_default();
        let muscle = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: muscle,
            name: "muscle".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(MUSCLE_STRENGTH, Fixed::from_int(4))]),
        });
        let energy = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: energy,
            name: "energy".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(ENERGY_DENSITY, Fixed::ONE)]),
        });
        let big =
            whole_body_muscle_force(&body((1, 2), vec![organ(muscle, (1, 1))]), &reg, &anchors);
        let small =
            whole_body_muscle_force(&body((1, 2), vec![organ(muscle, (1, 4))]), &reg, &anchors);
        assert!(big > small, "more muscle development, more work force");
        assert_eq!(
            big,
            Fixed::from_int(200),
            "full muscle (dev 1 * strength 4) times mass (0.5 * 100 kg) = 200"
        );
        assert_eq!(
            whole_body_muscle_force(&body((1, 2), vec![organ(energy, (1, 1))]), &reg, &anchors),
            Fixed::ZERO,
            "no strength tissue, no work force (not a mass-sized default)"
        );
        // Two bodies of equal normalized mass but different muscle endowment exert different force,
        // which the earlier body-mass proxy could not distinguish.
        let strong =
            whole_body_muscle_force(&body((3, 4), vec![organ(muscle, (1, 1))]), &reg, &anchors);
        let weak =
            whole_body_muscle_force(&body((3, 4), vec![organ(muscle, (1, 8))]), &reg, &anchors);
        assert!(
            strong > weak,
            "equal mass, different muscle, different force"
        );
        // Two bodies of equal muscle endowment but different mass exert different force: the mass scaling
        // the ruling keeps, that the earlier composition-only sum had dropped.
        let heavy =
            whole_body_muscle_force(&body((1, 1), vec![organ(muscle, (1, 1))]), &reg, &anchors);
        let light =
            whole_body_muscle_force(&body((1, 4), vec![organ(muscle, (1, 1))]), &reg, &anchors);
        assert!(
            heavy > light,
            "equal muscle, more mass, more force (the mass factor is present)"
        );
    }

    #[test]
    fn whole_body_specific_heat_averages_the_tissue_and_is_zero_without_it() {
        let (organs, skin, flesh, _fat) = registry();
        assert_eq!(
            whole_body_specific_heat(&body((1, 2), vec![organ(flesh, (1, 1))]), &organs),
            Fixed::from_int(3500),
            "one flesh organ carries its specific heat"
        );
        // No tissue declares specific heat (skin carries only surface): the absence convention reads
        // ZERO, not a hidden terran-water default (audit defect 2, Principle 9).
        assert_eq!(
            whole_body_specific_heat(&body((1, 2), vec![organ(skin, (1, 1))]), &organs),
            Fixed::ZERO,
            "no specific-heat tissue reads zero (the absence convention), never the water constant"
        );
    }

    #[test]
    fn two_specific_heat_free_bodies_do_not_converge_on_the_earth_water_value() {
        // Regression (audit defect 2): two distinct bodies that both declare no specific-heat tissue
        // must not both read the same hidden 4186 water value. Under the absence convention both read
        // ZERO thermal mass, so the body-to-medium coupling takes its own no-thermal-mass branch
        // (rate one, instant equilibration) rather than converging on the terran-water constant.
        let (organs, skin, _flesh, _fat) = registry();
        let anchors = MetabolicAnchors::dev_fixture();
        let a = body((1, 2), vec![organ(skin, (1, 1))]);
        let b = body((1, 4), vec![organ(skin, (1, 2))]);
        assert_eq!(whole_body_specific_heat(&a, &organs), Fixed::ZERO);
        assert_eq!(whole_body_specific_heat(&b, &organs), Fixed::ZERO);
        // The coupling is not authored from a hidden water thermal mass; the no-thermal-mass branch
        // reads rate one for both.
        assert_eq!(
            derive_body_exchange_rate(&a, &organs, anchors.medium_h, Fixed::ONE, &anchors),
            Fixed::ONE
        );
        assert_eq!(
            derive_body_exchange_rate(&b, &organs, anchors.medium_h, Fixed::ONE, &anchors),
            Fixed::ONE
        );
    }

    #[test]
    fn a_larger_denser_body_drains_a_smaller_fraction_of_its_reserve() {
        // The Kleiber signature: basal power grows as mass^(3/4) while the energy reserve grows linearly
        // with the body's energy tissue, so a larger, denser body spends a SMALLER fraction of its
        // reserve per tick. Thermoneutral (ambient == set point) to isolate the basal term.
        let (organs, _skin, _flesh, fat) = registry();
        let reg = HomeostaticRegistry::dev_default();
        let anchors = MetabolicAnchors::dev_fixture();
        let setpoint = Fixed::from_int(310);
        let tick = Fixed::ONE;
        // Small body: quarter mass, a quarter energy store.
        let small = body((1, 4), vec![organ(fat, (1, 4))]);
        // Large, denser body: full mass, a full energy store (more energy-dense tissue).
        let large = body((1, 1), vec![organ(fat, (1, 1))]);
        let cap_small = Homeostasis::new(&reg, &small, &organs).capacity(ENERGY);
        let cap_large = Homeostasis::new(&reg, &large, &organs).capacity(ENERGY);
        assert!(
            cap_large > cap_small,
            "the larger body holds the larger reserve"
        );
        let drain_small = derive_base_drain(
            &small,
            &organs,
            cap_small,
            setpoint,
            setpoint,
            anchors.medium_h,
            tick,
            &anchors,
        );
        let drain_large = derive_base_drain(
            &large,
            &organs,
            cap_large,
            setpoint,
            setpoint,
            anchors.medium_h,
            tick,
            &anchors,
        );
        assert!(
            drain_small > Fixed::ZERO && drain_large > Fixed::ZERO,
            "both drain"
        );
        assert!(
            drain_large < drain_small,
            "the larger, denser body drains a smaller fraction (Kleiber): large {drain_large:?} < small {drain_small:?}"
        );
    }

    #[test]
    fn a_colder_medium_drains_more_than_a_warm_one() {
        // The thermoregulatory term: the same body in a colder medium sheds more heat and so pays a
        // larger resting drain than in a temperate medium. Physics in (a temperature gradient), no label.
        let (organs, skin, flesh, fat) = registry();
        let reg = HomeostaticRegistry::dev_default();
        let anchors = MetabolicAnchors::dev_fixture();
        let plan = body(
            (1, 1),
            vec![
                organ(skin, (1, 1)),
                organ(flesh, (1, 1)),
                organ(fat, (1, 1)),
            ],
        );
        let cap = Homeostasis::new(&reg, &plan, &organs).capacity(ENERGY);
        let setpoint = Fixed::from_int(310);
        let cold = derive_base_drain(
            &plan,
            &organs,
            cap,
            Fixed::from_int(250),
            setpoint,
            anchors.medium_h,
            Fixed::ONE,
            &anchors,
        );
        let warm = derive_base_drain(
            &plan,
            &organs,
            cap,
            setpoint,
            setpoint,
            anchors.medium_h,
            Fixed::ONE,
            &anchors,
        );
        assert!(
            cold > warm,
            "a colder medium exacts a larger thermoregulatory drain"
        );
    }

    #[test]
    fn exertion_coupling_adds_a_drain_that_scales_with_work() {
        let (organs, _skin, _flesh, fat) = registry();
        let reg = HomeostaticRegistry::dev_default();
        let anchors = MetabolicAnchors::dev_fixture();
        let plan = body((1, 1), vec![organ(fat, (1, 1))]);
        let cap = Homeostasis::new(&reg, &plan, &organs).capacity(ENERGY);
        // A modest force on the WATT scale (force*velocity, no kilowatt bridge, matching the
        // watt-scale basal drain it is summed with), kept below the full-drain saturation so the
        // scaling with velocity is visible.
        let force = Fixed::ONE;
        let slow =
            derive_exertion_coupling(&plan, &organs, cap, force, Fixed::ONE, Fixed::ONE, &anchors);
        let fast = derive_exertion_coupling(
            &plan,
            &organs,
            cap,
            force,
            Fixed::from_int(4),
            Fixed::ONE,
            &anchors,
        );
        assert!(
            fast > slow,
            "faster work at the same force adds a larger exertion drain ({fast:?} > {slow:?})"
        );
        assert!(
            slow > Fixed::ZERO,
            "work exacts a nonzero exertion coupling"
        );
        assert!(
            fast < FRAC_MAX,
            "the exertion coupling stays below full drain here"
        );
    }

    #[test]
    fn a_high_surface_body_couples_to_the_medium_faster_than_a_compact_one() {
        // h*A/(m*c): a high-surface body couples fast, a low-surface one slowly, and a body with no
        // exchange surface does not couple at all.
        let (organs, skin, flesh, _fat) = registry();
        let anchors = MetabolicAnchors::dev_fixture();
        // High surface: a full skin plus modest flesh.
        let high = body((1, 2), vec![organ(skin, (1, 1)), organ(flesh, (1, 4))]);
        // Compact: the same flesh but a quarter skin (less exposed surface).
        let compact = body((1, 2), vec![organ(skin, (1, 4)), organ(flesh, (1, 4))]);
        let rate_high =
            derive_body_exchange_rate(&high, &organs, anchors.medium_h, Fixed::ONE, &anchors);
        let rate_compact =
            derive_body_exchange_rate(&compact, &organs, anchors.medium_h, Fixed::ONE, &anchors);
        assert!(rate_high > rate_compact, "more surface, faster coupling");
        // No exchange surface: no coupling.
        assert_eq!(
            derive_body_exchange_rate(
                &body((1, 2), vec![organ(flesh, (1, 1))]),
                &organs,
                anchors.medium_h,
                Fixed::ONE,
                &anchors,
            ),
            Fixed::ZERO,
            "no convective surface, no coupling"
        );
    }

    #[test]
    fn anchors_read_from_a_set_manifest_and_fail_loud_when_reserved() {
        // The five owner anchors load from a set manifest, and a reserved one refuses to fabricate.
        let set = r#"
[[reserved]]
id = "metabolism.kleiber_coefficient"
basis = "fixture"
status = "set"
value = "3.4"
unit = "w"
source = "test"
[[reserved]]
id = "metabolism.body_mass_kg_scale"
basis = "fixture"
status = "set"
value = "100"
unit = "kg"
source = "test"
[[reserved]]
id = "metabolism.medium_convective_coefficient"
basis = "fixture"
status = "set"
value = "10"
unit = "h"
source = "test"
[[reserved]]
id = "metabolism.surface_emissivity"
basis = "fixture"
status = "set"
value = "0.95"
unit = "e"
source = "test"
[[reserved]]
id = "metabolism.stefan_boltzmann"
basis = "fixture"
status = "set"
value = "0.0000000567"
unit = "sigma"
source = "test"
"#;
        let m = CalibrationManifest::from_toml_str(set).unwrap();
        let a = MetabolicAnchors::from_manifest(&m).unwrap();
        assert_eq!(a.body_mass_kg_scale, Fixed::from_int(100));
        assert_eq!(a.emissivity, Fixed::from_ratio(95, 100));
        // The shipped anchors are reserved (empty), so a from_manifest read fails loud rather than
        // fabricating a number.
        let reserved = set.replace(
            "id = \"metabolism.kleiber_coefficient\"\nbasis = \"fixture\"\nstatus = \"set\"\nvalue = \"3.4\"",
            "id = \"metabolism.kleiber_coefficient\"\nbasis = \"fixture\"\nstatus = \"reserved\"\nvalue = \"\"",
        );
        let mr = CalibrationManifest::from_toml_str(&reserved).unwrap();
        assert_eq!(
            MetabolicAnchors::from_manifest(&mr).unwrap_err(),
            CalibrationError::Reserved("metabolism.kleiber_coefficient".to_string()),
        );
    }

    #[test]
    fn derived_metabolism_is_deterministic() {
        let (organs, skin, flesh, fat) = registry();
        let reg = HomeostaticRegistry::dev_default();
        let anchors = MetabolicAnchors::dev_fixture();
        let plan = body(
            (3, 4),
            vec![
                organ(skin, (1, 2)),
                organ(flesh, (3, 4)),
                organ(fat, (1, 2)),
            ],
        );
        let cap = Homeostasis::new(&reg, &plan, &organs).capacity(ENERGY);
        let run = || {
            let base = derive_base_drain(
                &plan,
                &organs,
                cap,
                Fixed::from_int(270),
                Fixed::from_int(310),
                anchors.medium_h,
                Fixed::ONE,
                &anchors,
            );
            let rate =
                derive_body_exchange_rate(&plan, &organs, anchors.medium_h, Fixed::ONE, &anchors);
            (base.to_bits(), rate.to_bits())
        };
        assert_eq!(
            run(),
            run(),
            "the same body, medium, and anchors replay bit for bit"
        );
    }
}
