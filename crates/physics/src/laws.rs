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

//! The closed-form fixed-point law kernels of the two floors (build phase 2).
//!
//! Each kernel is the fixed Rust of an [`crate::InteractionLaw`]: a pure function of
//! the participating entities' axis values and the law's reserved constants, reporting
//! a measured consequence, never a verdict. The reserved constants (caps, the
//! gravitational acceleration, the per-substance modelling data) are parameters, so the
//! kernel itself bakes no fabricated value; the determinism harness below supplies dev
//! fixtures, and the owner's set numbers reach a kernel through the calibration manifest
//! when the engine wires it. The only module constants are engine mechanics, not owner
//! realism values: the unit-bridge ratios fixed by the pinned canonical scales (the
//! megapascal is the owner's R-UNITS-PIN pressure pin), the overflow-safe saturation
//! ratios forced by Q32.32, and the mathematical constants. This is the Principle-11
//! engine-mechanics exemption, the same one the RNG phase numbers take.
//!
//! Every kernel obeys the discipline the two red passes hardened the proposals to: a
//! product or quotient that can exceed the Q32.32 ceiling is formed with checked
//! arithmetic and routed to its physical limit rather than wrapping; a saturation guard
//! is ordered before the operation it protects; and a cross-class reduction is the
//! order-independent [`Fixed::saturating_sum`] or a min-fold, so the result is
//! invariant to how the work is partitioned across threads.

use civsim_core::Fixed;

const ZERO: Fixed = Fixed::ZERO;
const ONE: Fixed = Fixed::ONE;

// Unit-bridge ratios, fixed by the pinned canonical scales (a scale definition, not an
// owner realism value): pascal to megapascal, joule-per-cubic-metre to its mega form,
// joule to kilojoule, newton-metre to kilonewton-metre, watt to kilowatt.
const C_PA: Fixed = Fixed::from_int(1_000_000);
const C_VOL: Fixed = Fixed::from_int(1_000_000);
const C_KJ: Fixed = Fixed::from_int(1_000);
const C_KNM: Fixed = Fixed::from_int(1_000);
const C_KW: Fixed = Fixed::from_int(1_000);

/// One half, the kinetic-energy coefficient, exact in Q32.32 (bit pattern `1 << 31`).
const HALF: Fixed = Fixed::from_bits(1 << 31);

/// The overflow-safe saturation ratio for the squared Hill term: the largest `r` with
/// `r^2` below the representable ceiling (2^31), so the guard `r > R_SAT_N2` provably
/// precedes the square. Forced by Q32.32, not a reserved realism value.
const R_SAT_N2: Fixed = Fixed::from_int(46340);
/// The overflow-safe saturation ratio for the cubed Hill term (`1290^3` fits, `1291^3`
/// wraps), so the guard precedes the cube.
const R_SAT_N3: Fixed = Fixed::from_int(1290);
/// The overflow-safe ceiling for squaring a velocity in [`kinetic_energy`]:
/// `sqrt(2^31) = 46340`, beyond which `v * v` would wrap.
const V2_MAX: Fixed = Fixed::from_int(46340);

/// Pi squared, the Euler buckling coefficient, rendered to Q32.32 by the exact decimal
/// reader (a mathematical constant).
fn pi_squared() -> Fixed {
    Fixed::from_decimal_str("9.86960440108936").expect("pi squared literal is valid")
}

/// One over the square root of three, the von Mises shear-yield ratio (a mathematical
/// constant).
fn von_mises() -> Fixed {
    Fixed::from_decimal_str("0.57735026918963").expect("von Mises literal is valid")
}

/// A small-integer power `r^n` for `n` in 1..=3, formed by repeated checked multiply, so
/// an overflow is `None` rather than a silent wrap.
fn pow_int(r: Fixed, n: u8) -> Option<Fixed> {
    match n {
        1 => Some(r),
        2 => r.checked_mul(r),
        3 => r.checked_mul(r).and_then(|r2| r2.checked_mul(r)),
        _ => None,
    }
}

fn r_sat(n: u8) -> Fixed {
    match n {
        2 => R_SAT_N2,
        _ => R_SAT_N3,
    }
}

// === Biology (R-PHYS-BIO): net nutrition, harm, edibility ===

/// Per-nutrient-class satisfaction in [0, 1]. A `requirement` of `None` (the class is
/// not required) or zero is fully satisfied (it never lowers the Liebig minimum); an
/// abundant supply against a tiny requirement saturates to one rather than wrapping a
/// false zero (the wave-0 NEW-DET-3 fix).
pub fn satisfaction(supply: Fixed, assimilation: Fixed, requirement: Option<Fixed>) -> Fixed {
    let req = match requirement {
        None => return ONE,
        Some(r) if r == ZERO => return ONE,
        Some(r) => r,
    };
    let num = match supply.checked_mul(assimilation) {
        // Both factors are non-negative fractions, so an overflowing product is abundant supply:
        // route to full satisfaction, the same extreme the divide-overflow below reaches, not to zero.
        Some(x) => x,
        None => return ONE,
    };
    match num.checked_div(req) {
        Some(s) => s.clamp(ZERO, ONE),
        None => ONE,
    }
}

/// Net nutrition: the Liebig minimum across the classes (the limiting nutrient). The
/// min-fold is associative and commutative, so the result is order-independent.
pub fn net_nutrition(classes: &[(Fixed, Fixed, Option<Fixed>)]) -> Fixed {
    classes
        .iter()
        .fold(ONE, |acc, &(s, a, r)| acc.min(satisfaction(s, a, r)))
}

/// Per-toxin-class harm in [0, harm_cap] by the integer-Hill dose response. A
/// not-applicable tolerance (`None`) skips the class (zero harm); a present tolerance of
/// zero, or a dose-to-tolerance ratio beyond the representable range, routes to the
/// maximum-harm cap (the wave-0 NEW-DET-2 fix); and the saturation guard is ordered
/// before the power so `r^n` never wraps (the NEW-DET-1 fix). `n` is the per-(class,
/// consumer) integer exponent.
pub fn harm_class(dose: Fixed, tolerance: Option<Fixed>, n: u8, harm_cap: Fixed) -> Fixed {
    let tol = match tolerance {
        None => return ZERO,
        Some(t) => t,
    };
    if dose == ZERO {
        return ZERO;
    }
    let r = match dose.checked_div(tol) {
        Some(r) => r,
        None => return harm_cap,
    };
    if n >= 2 && r > r_sat(n) {
        return harm_cap;
    }
    let rn = match pow_int(r, n) {
        Some(p) => p,
        None => return harm_cap,
    };
    match rn.checked_add(ONE) {
        Some(den) => match rn.checked_div(den) {
            Some(h) => h.clamp(ZERO, harm_cap),
            None => harm_cap,
        },
        None => harm_cap,
    }
}

/// Net harm: the order-independent saturating sum of the per-class harms, capped.
pub fn net_harm(
    classes: &[(Fixed, Option<Fixed>, u8)],
    harm_cap: Fixed,
    total_cap: Fixed,
) -> Fixed {
    Fixed::saturating_sum(
        classes
            .iter()
            .map(|&(d, t, n)| harm_class(d, t, n, harm_cap)),
    )
    .min(total_cap)
}

/// The measured edibility tuple. The law reports only measured quantities; the
/// gain-versus-danger valuation lives in the agent layer, and the medicinal value is a
/// reserved relational refinement, so neither is baked here. The margin is the aggregate
/// safety ratio, formed with a checked divide so a near-clean meal saturates to
/// `margin_cap` rather than wrapping (the wave-0 NEW-DET-5 fix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edibility {
    /// Net nutrition in [0, 1].
    pub net_nutrition: Fixed,
    /// Net harm in [0, total_cap].
    pub net_harm: Fixed,
    /// The aggregate safety margin, capped at `margin_cap`.
    pub margin: Fixed,
}

/// Compute the edibility tuple from the measured nutrition and harm and the aggregate
/// tolerance and dose.
pub fn edibility(
    net_nutrition: Fixed,
    net_harm: Fixed,
    tolerance_aggregate: Fixed,
    dose_aggregate: Fixed,
    margin_cap: Fixed,
) -> Edibility {
    let margin = if dose_aggregate == ZERO {
        margin_cap
    } else {
        match tolerance_aggregate.checked_div(dose_aggregate) {
            Some(m) => m.min(margin_cap),
            None => margin_cap,
        }
    };
    Edibility {
        net_nutrition,
        net_harm,
        margin,
    }
}

// === Mechanics (R-PHYS-MECH) ===

/// Contact pressure in megapascals: force over the bearing area, brought to the MPa
/// scale. A zero or sub-floor area routes to the maximum pressure (a fully concentrated
/// load), the correct physical limit.
pub fn contact_pressure(force: Fixed, contact_area: Fixed, p_max: Fixed) -> Fixed {
    let den = match contact_area.checked_mul(C_PA) {
        Some(d) => d,
        // A contact area so large it overflows spreads the force to a negligible pressure: route to
        // zero, not the maximum-pressure cap (which is the zero-area extreme just below).
        None => return ZERO,
    };
    // The divisor's zero-boundary is a declared physical-limit-at-zero (R-UNITS-PIN floor invariant, the
    // slice-3 backstop): a zero contact area is the concentrated point load, which reads the material's
    // maximum pressure `p_max`, a physical limit rather than a value riding the storage epsilon.
    // `guarded_checked_div` returns `p_max` at that boundary and keeps the law's own overflow cap on `None`.
    // The wiring is byte-neutral on the PHYSICAL DOMAIN: a contact area is a non-negative physical quantity, so
    // `den = contact_area * C_PA >= 0` and `den <= ZERO` there is exactly the prior `den == ZERO` guard. That
    // domain invariant, which the byte-neutrality rests on, is now code-enforced rather than only asserted in
    // prose (a mis-declared negative area fails loud in debug rather than silently reading `p_max`); off the
    // physical domain the cap is a fail-safe, not the prior negative pressure.
    debug_assert!(
        den >= ZERO,
        "contact_pressure: a contact area is a non-negative physical quantity; the floor-invariant wiring's \
         byte-neutrality rests on it"
    );
    match civsim_units::guard::guarded_checked_div(
        force,
        den,
        civsim_units::guard::ZeroGuard::LimitAtZero(p_max),
    ) {
        Some(p) => p.min(p_max),
        None => p_max,
    }
}

/// Cut or penetration depth, capped at the artifact reach. Gated on the contact pressure
/// exceeding the target hardness, then the delivered work over the specific cutting
/// energy and swept area, staged so the energy-per-depth product is never formed before
/// the division reduces it (the wave-1 MECH-RT2-01 fix). A vast resistance product
/// routes to a negligible (zero) cut; a tiny resistance routes to the full reach.
pub fn cut_penetrate(
    pressure: Fixed,
    hardness: Fixed,
    delivered_energy: Fixed,
    specific_cut_energy: Fixed,
    contact_area: Fixed,
    d_max: Fixed,
) -> Fixed {
    if pressure <= hardness {
        return ZERO;
    }
    let den = match specific_cut_energy.checked_mul(contact_area) {
        Some(d) => d,
        None => return ZERO,
    };
    if den == ZERO {
        return d_max;
    }
    let d1 = match delivered_energy.checked_div(den) {
        Some(x) => x,
        None => return d_max,
    };
    match d1.checked_div(C_VOL) {
        Some(x) => x.min(d_max),
        None => d_max,
    }
}

/// Bending stress (MPa) and the collapse margin (collapse when the margin is below
/// zero). Staged as force over (section modulus times the MPa bridge) then times the
/// span, so the large bending moment is never formed (the wave-1 MECH-RT2-02 fix).
pub fn bend_stress(
    force: Fixed,
    section_modulus: Fixed,
    span: Fixed,
    yield_strength: Fixed,
    stress_max: Fixed,
) -> (Fixed, Fixed) {
    let den = match section_modulus.checked_mul(C_PA) {
        Some(d) => d,
        None => return (ZERO, yield_strength),
    };
    let sigma = if den == ZERO {
        stress_max
    } else {
        match force.checked_div(den).and_then(|s| s.checked_mul(span)) {
            Some(s) => s.min(stress_max),
            None => stress_max,
        }
    };
    (sigma, sat_sub(yield_strength, sigma))
}

/// Axial stress (MPa) and the collapse margin: force over cross-section.
pub fn axial_stress(
    force: Fixed,
    cross_section: Fixed,
    yield_strength: Fixed,
    stress_max: Fixed,
) -> (Fixed, Fixed) {
    let den = match cross_section.checked_mul(C_PA) {
        Some(d) => d,
        None => return (ZERO, yield_strength),
    };
    let sigma = if den == ZERO {
        stress_max
    } else {
        match force.checked_div(den) {
            Some(s) => s.min(stress_max),
            None => stress_max,
        }
    };
    (sigma, sat_sub(yield_strength, sigma))
}

/// The dual fracture criterion: a stress margin against the fracture strength and an
/// energy margin against the available fracture energy over the crack area. Fracture
/// initiates when either margin is below zero. No division, so no zero divisor.
pub fn fracture_onset(
    applied_stress: Fixed,
    fracture_strength: Fixed,
    fracture_energy: Fixed,
    crack_area: Fixed,
    delivered_energy: Fixed,
    energy_max: Fixed,
) -> (Fixed, Fixed) {
    let stress_margin = sat_sub(fracture_strength, applied_stress);
    let g_avail = match fracture_energy.checked_mul(crack_area) {
        Some(g) => g.min(energy_max),
        None => energy_max,
    };
    (stress_margin, sat_sub(g_avail, delivered_energy))
}

/// The force a material STRESS raises over a bearing AREA, in newtons (the megapascal-to-newton bridge). A
/// strength stress is stored on the MEGAPASCAL scale (`stored = Pa / 1e6`, the convention `mat.fracture_strength`
/// and its siblings carry), so the force in newtons is the stress over the area promoted by the `C_PA` bridge,
/// exactly as [`contact_pressure`] and [`axial_stress`] apply `C_PA` in the other direction. Without the bridge
/// the force would be a factor of a million too small, and the actuator work below (compared against a
/// joule-scale Griffith resistance) would read a million times too weak. `force_max` bounds the representable
/// force. A zero stress or zero area raises no force.
pub fn stress_force(stress: Fixed, area: Fixed, force_max: Fixed) -> Fixed {
    if stress <= ZERO || area <= ZERO {
        return ZERO;
    }
    match stress.checked_mul(area).and_then(|sa| sa.checked_mul(C_PA)) {
        Some(f) => f.min(force_max),
        None => force_max,
    }
}

/// The actuator-work law (the stroke-rate / limb-biomechanics substrate). The kinetic energy a mass-bearing
/// contact delivers equals the WORK the actuator does bringing the acting part to speed: force times the
/// distance the force acts over (`W = F d`, the floor's work-energy relation), a Joule. This is the delivered
/// energy DIRECTLY, retiring the swing-speed intermediate that only round-trips to it: substituting the swing
/// speed `v = sqrt(2 F d / m)` into `1/2 m v^2` cancels the mass and returns `F d`, so the mass a body swings
/// sets the tip speed but not the delivered energy (a heavier part swings slower for the same work). `force`
/// is the actuating force in NEWTONS, formed from the acting material's strength over its cross-section by
/// [`stress_force`] (which applies the megapascal-to-newton `C_PA` bridge, so the resulting energy is on the
/// joule scale the Griffith resistance is on). `distance` is the stroke the force acts over (the acting part's
/// own grown `mech.stroke_length`, an m), grown independently of the segment length so their ratio is per-body
/// data, never a fixed one. The conversion efficiency is one, a lossless floor idealization (the
/// energy-conservation ceiling, like a frictionless limit); a per-material toughness derating is the disclosed,
/// physics-derivable refinement, not an authored world value. `energy_max` is the representability cap the
/// product saturates at. A zero force (no actuating strength) or zero stroke yields zero energy (the absence
/// convention: an actuator with no strength delivers no blow).
pub fn actuator_work(force: Fixed, distance: Fixed, energy_max: Fixed) -> Fixed {
    if force <= ZERO || distance <= ZERO {
        return ZERO;
    }
    match force.checked_mul(distance) {
        Some(w) => w.min(energy_max),
        None => energy_max,
    }
}

/// The elastic-recoil delivered energy (J), the elastic analog of the rigid actuator work [`actuator_work`]: the
/// elastic STRAIN ENERGY a springy actuator stores up to yield and releases in a recoil blow (a whip tip, a
/// trap-jaw latch, a ballistic spring). It is the MODULUS OF RESILIENCE `yield^2 / (2 E)` (the elastic
/// strain-energy density up to yield, the area under the linear elastic stress-strain curve; Gere and Timoshenko,
/// Mechanics of Materials) times the strained VOLUME, on the joule scale `F d` and the Griffith fracture energy
/// are on, so the run-all-gate-to-zero delivered-energy set (the stroke-rate step-2 substrate) combines it with
/// the rigid `F d` on one currency.
///
/// `yield_strength` and `elastic_modulus` are the material's `mat.yield_strength` and `mat.elastic_modulus`, both
/// MEGAPASCAL-stored (`stored = Pa / 1e6`), so `yield^2 / (2 E)` carries one net megapascal (a stress, an energy
/// density on the MJ/m^3 scale): applying the SAME `C_PA` megapascal-to-pascal bridge [`stress_force`] lands it on
/// J/m^3, and the `volume` product on Joules, with no new constant. `volume` is the strained elastic-element
/// volume (m^3), the actuator's own grown geometry. `energy_max` bounds the representable energy. A part with no
/// yield strength, no elastic modulus, or no volume stores no elastic energy and reads ZERO (the absence
/// convention): a rigid or fluid actuator self-gates, so the elastic kernel contributes nothing until a world
/// grows a springy tissue. The conversion efficiency is one, a lossless floor idealization (the same
/// energy-conservation ceiling [`actuator_work`] makes); a per-material hysteresis-damping derating is the
/// disclosed, physics-derivable refinement, not an authored world value.
pub fn elastic_recoil_energy(
    yield_strength: Fixed,
    elastic_modulus: Fixed,
    volume: Fixed,
    energy_max: Fixed,
) -> Fixed {
    if yield_strength <= ZERO || elastic_modulus <= ZERO || volume <= ZERO {
        return ZERO;
    }
    // The modulus of resilience `yield^2 / (2 E)`, the elastic strain-energy density up to yield (a stress, MJ/m^3
    // in the megapascal-stored scale). Each step guards its overflow to the representability ceiling, never wraps.
    let two_e = match elastic_modulus.checked_mul(Fixed::from_int(2)) {
        Some(v) => v,
        None => return energy_max,
    };
    let resilience = match yield_strength
        .checked_mul(yield_strength)
        .and_then(|y2| y2.checked_div(two_e))
    {
        Some(v) => v,
        None => return energy_max,
    };
    // `resilience[MPa] * C_PA` lands on J/m^3, `* volume` on J (the same bridge `stress_force` applies).
    match resilience
        .checked_mul(C_PA)
        .and_then(|density| density.checked_mul(volume))
    {
        Some(e) => e.min(energy_max),
        None => energy_max,
    }
}

/// Delivered kinetic energy on the kilojoule scale. The half is applied before the
/// mass-velocity-squared product and the scale bridge is applied before the squared
/// velocity, so a representable energy is never pre-saturated (the wave-1 fix); the
/// squared velocity is guarded against its overflow-safe ceiling.
pub fn kinetic_energy(mass: Fixed, velocity: Fixed, energy_max: Fixed) -> Fixed {
    if sat_abs(velocity) > V2_MAX {
        return energy_max;
    }
    let v2 = match velocity.checked_mul(velocity) {
        Some(x) => x,
        None => return energy_max,
    };
    let mh = match mass.checked_mul(HALF) {
        Some(x) => x,
        None => return energy_max,
    };
    let scaled = match mh.checked_div(C_KJ) {
        Some(x) => x,
        None => return energy_max,
    };
    match scaled.checked_mul(v2) {
        Some(k) => k.min(energy_max),
        None => energy_max,
    }
}

/// Reduced-mass impulse `mu * v * (1 + e)`, where the reduced mass `m1 / (1 + m1/m2)` is
/// formed so the large `m1 * m2` product never appears (the wave-1 fix). A massless or
/// vanishingly small relative target transfers nothing.
pub fn impulse(
    striker_mass: Fixed,
    target_mass: Fixed,
    velocity: Fixed,
    restitution: Fixed,
    impulse_max: Fixed,
) -> Fixed {
    if target_mass == ZERO {
        return ZERO;
    }
    let ratio = match striker_mass.checked_div(target_mass) {
        Some(r) => r,
        None => return ZERO,
    };
    let denom = match ratio.checked_add(ONE) {
        Some(d) => d,
        None => return ZERO,
    };
    let mu = match striker_mass.checked_div(denom) {
        Some(m) => m,
        None => return ZERO,
    };
    let one_plus_e = match ONE.checked_add(restitution) {
        Some(x) => x,
        None => return impulse_max,
    };
    match mu
        .checked_mul(velocity)
        .and_then(|p| p.checked_mul(one_plus_e))
    {
        Some(j) => j.min(impulse_max),
        None => impulse_max,
    }
}

/// A lever's measured outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lever {
    /// Torque on the kilonewton-metre scale.
    pub torque: Fixed,
    /// Mechanical advantage, the dimensionless arm ratio.
    pub mechanical_advantage: Fixed,
    /// Output force.
    pub output_force: Fixed,
}

/// The lever law. Mechanical advantage is the arm ratio computed directly, then the
/// output force, so an equal-arm lever reads advantage one rather than the saturated
/// value the product-first form produced (the wave-1 MECH-RT2-03 fix); torque is on its
/// own kilonewton-metre scale, the bridge applied before the arm multiply.
pub fn lever(
    force: Fixed,
    effort_arm: Fixed,
    load_arm: Fixed,
    force_max: Fixed,
    advantage_max: Fixed,
    torque_max: Fixed,
) -> Lever {
    let mechanical_advantage = if load_arm == ZERO {
        advantage_max
    } else {
        match effort_arm.checked_div(load_arm) {
            // Cap the arm ratio at advantage_max on the success path too, matching the zero-load and
            // overflow branches, so a representable but very high ratio still honours the bound.
            Some(m) => m.min(advantage_max),
            None => advantage_max,
        }
    };
    let output_force = match force.checked_mul(mechanical_advantage) {
        Some(f) => f.min(force_max),
        None => force_max,
    };
    let torque = match force
        .checked_div(C_KNM)
        .and_then(|f| f.checked_mul(effort_arm))
    {
        Some(t) => t.min(torque_max),
        None => torque_max,
    };
    Lever {
        torque,
        mechanical_advantage,
        output_force,
    }
}

/// A friction interface's measured outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Friction {
    /// The kinetic friction force.
    pub kinetic_force: Fixed,
    /// The slip margin (slips when below zero).
    pub slip_margin: Fixed,
    /// The power lost to friction, on the kilowatt scale.
    pub power_loss: Fixed,
    /// The transmission efficiency in [0, 1].
    pub efficiency: Fixed,
}

/// Coulomb friction and mechanical efficiency. Efficiency is the standard transmitted
/// over input power ratio, a measured consequence, with the value judgment left to the
/// agent layer.
#[allow(clippy::too_many_arguments)]
pub fn friction(
    static_coefficient: Fixed,
    kinetic_coefficient: Fixed,
    normal: Fixed,
    tangential: Fixed,
    slip_velocity: Fixed,
    input_power: Fixed,
    force_max: Fixed,
    power_max: Fixed,
) -> Friction {
    let f_s_max = match static_coefficient.checked_mul(normal) {
        Some(f) => f,
        None => force_max,
    };
    let slip_margin = sat_sub(f_s_max, tangential);
    let kinetic_force = match kinetic_coefficient.checked_mul(normal) {
        Some(f) => f,
        None => force_max,
    };
    let power_loss = match kinetic_force
        .checked_div(C_KW)
        .and_then(|f| f.checked_mul(slip_velocity))
    {
        Some(p) => p.min(power_max),
        None => power_max,
    };
    let efficiency = if input_power == ZERO {
        ZERO
    } else {
        let net = sat_sub(input_power, power_loss).max(ZERO);
        match net.checked_div(input_power) {
            Some(e) => e.clamp(ZERO, ONE),
            None => ONE,
        }
    };
    Friction {
        kinetic_force,
        slip_margin,
        power_loss,
        efficiency,
    }
}

/// Reach: the additive geometric extent of an implement's segments, by the
/// order-independent saturating sum.
pub fn reach(segment_lengths: &[Fixed]) -> Fixed {
    Fixed::saturating_sum(segment_lengths.iter().copied())
}

/// Weight: the load force from mass and the shared gravitational acceleration (`F = m
/// g`). The gravitational acceleration is the owner's reserved value (terran 9.81 with a
/// per-world override), passed in.
pub fn weight(mass: Fixed, gravity: Fixed, force_max: Fixed) -> Fixed {
    match mass.checked_mul(gravity) {
        Some(f) => f.min(force_max),
        None => force_max,
    }
}

/// Mechanical power on the kilowatt scale: force times velocity, the bridge applied
/// before the multiply.
pub fn power(force: Fixed, velocity: Fixed, power_max: Fixed) -> Fixed {
    match force
        .checked_div(C_KW)
        .and_then(|f| f.checked_mul(velocity))
    {
        Some(p) => p.min(power_max),
        None => power_max,
    }
}

/// Mechanical power on the WATT scale: force times velocity with no kilowatt bridge, the
/// SI-watt sibling of [`power`]. This is the scale the metabolism bridge
/// ([`metabolic_drain_fraction`]) and the basal rate ([`basal_metabolic_rate`]) work in, so a
/// derived exertion drain and the resting drain share one power scale rather than differing by the
/// kilowatt factor. An overflowing product routes to the reserved power cap.
pub fn power_watts(force: Fixed, velocity: Fixed, power_max: Fixed) -> Fixed {
    match force.checked_mul(velocity) {
        Some(p) => p.min(power_max),
        None => power_max,
    }
}

// === Materials (R-PHYS-MECH) ===

/// The Euler critical buckling load. The slenderness square is guarded before it is
/// formed, so it cannot wrap to a negative value and invert the law (the wave-1
/// RT-MAT-01 fix); an extremely slender column routes to zero load (buckling governs), a
/// zero-length stub to the maximum force (strength governs), each the correct direction.
pub fn euler_buckle(
    modulus: Fixed,
    second_moment: Fixed,
    effective_length_factor: Fixed,
    length: Fixed,
    force_max: Fixed,
) -> Fixed {
    let le = match effective_length_factor.checked_mul(length) {
        Some(x) => x,
        None => return ZERO,
    };
    let lsq = match le.checked_mul(le) {
        Some(x) => x,
        None => return ZERO,
    };
    if lsq == ZERO {
        return force_max;
    }
    let r = match second_moment.checked_div(lsq) {
        Some(x) => x,
        None => return force_max,
    };
    let ei = match modulus.checked_mul(r) {
        Some(x) => x,
        None => return force_max,
    };
    // `modulus` is on the megapascal scale (stored = Pa / 1e6); the buckling load is a newton
    // force, so promote the product to pascals with the C_PA bridge, applied last (after the
    // reducing r divide) so a representable load is not capped early.
    let base = match pi_squared().checked_mul(ei) {
        Some(x) => x,
        None => return force_max,
    };
    match base.checked_mul(C_PA) {
        Some(p) => p.min(force_max),
        None => force_max,
    }
}

/// Shear (or torsional) stress and the margin against the shear strength. An anisotropic
/// or brittle substance carries an independent shear strength; an isotropic ductile one
/// derives it from yield by the von Mises ratio.
pub fn shear(
    shear_force: Fixed,
    shear_area: Fixed,
    independent_shear_strength: Option<Fixed>,
    yield_strength: Fixed,
    stress_max: Fixed,
) -> (Fixed, Fixed) {
    let tau_material = match independent_shear_strength {
        Some(t) => t,
        None => yield_strength
            .checked_mul(von_mises())
            .unwrap_or(stress_max),
    };
    let den = match shear_area.checked_mul(C_PA) {
        Some(d) => d,
        None => return (ZERO, tau_material),
    };
    let tau_applied = if den == ZERO {
        stress_max
    } else {
        match shear_force.checked_div(den) {
            Some(s) => s.min(stress_max),
            None => stress_max,
        }
    };
    (tau_applied, tau_material - tau_applied)
}

/// Archard worn volume: `K F s / H`, with the wear coefficient carried at its own scale
/// and divided back out (the scale and its split-range representation are a reserved
/// seam). A zero hardness abrades without bound.
#[allow(clippy::too_many_arguments)]
pub fn wear(
    wear_coefficient_scaled: Fixed,
    coefficient_scale: Fixed,
    force: Fixed,
    distance: Fixed,
    hardness: Fixed,
    wear_max: Fixed,
) -> Fixed {
    if hardness == ZERO || coefficient_scale == ZERO {
        // A zero hardness abrades without bound; an open (zero) coefficient scale is an
        // unconfigured coefficient. Both route to the reserved ceiling.
        return wear_max;
    }
    // V = K_scaled*F*s/(scale*H), evaluated in i128 raw bits so the only cap is on the true
    // result. The reduced coefficient K = K_scaled/scale is never materialised as a Fixed (that
    // would floor a sub-2^-32 Archard coefficient toward zero and lose the low bits of a
    // mild-wear coefficient); instead the scale divides the grown K_scaled*F, keeping full
    // precision, and the whole chain stays in i128, which the declared ranges never exhaust. The
    // two divisions truncate toward zero, matching the Fixed floor-division convention.
    let ks = wear_coefficient_scaled.to_bits() as i128;
    let sc = coefficient_scale.to_bits() as i128; // != 0 (guarded)
    let f = force.to_bits() as i128;
    let s = distance.to_bits() as i128;
    let h = hardness.to_bits() as i128; // != 0 (guarded)
    let wmb = wear_max.to_bits() as i128;
    // n1 = K_scaled*F (raw), reduced by the scale before the slide distance grows it.
    let n1 = match ks.checked_mul(f) {
        Some(x) => x,
        None => return wear_max,
    };
    let n2 = n1 / sc; // = K*F as a Fixed raw, full precision
    let n3 = match n2.checked_mul(s) {
        Some(x) => x,
        None => return wear_max,
    };
    // `hardness` is on the megapascal scale (stored = Pa / 1e6); the wear volume is SI cubic
    // metres, so divide by the pascal hardness (h promoted by the 1e6 C_PA bridge), not the raw
    // megapascal value, or the volume comes out 1e6 too large.
    let v = n3 / (h * 1_000_000);
    if v >= wmb {
        wear_max
    } else {
        Fixed::from_bits(v as i64)
    }
}

/// Energy (kilojoule scale) to abrade the Archard worn volume away, so a wear insult accrues in the
/// same currency as a fracture tolerance. From the cut model's own identity, inverting
/// [`cut_penetrate`] (`depth = delivered_energy / (specific_cut_energy * contact_area) / C_VOL`, and
/// `depth * contact_area` is the swept volume `V`): the delivered work to remove a swept volume `V`
/// is `V * specific_cut_energy * C_VOL`. That is the SAME kilojoule scale as `fracture_energy *
/// crack_area` in [`fracture_onset`], so a wear increment and a fracture tolerance are directly
/// commensurate with NO free per-insult weight: the commensuration is the floor's own cut work,
/// keyed on the being's own `specific_cut_energy`. `energy_max` caps the result; `C_VOL` exceeds one,
/// so an intermediate that overflows the representable range means the true energy already exceeds
/// any sane `energy_max` and routes to the cap.
#[allow(clippy::too_many_arguments)]
pub fn wear_energy(
    wear_coefficient_scaled: Fixed,
    coefficient_scale: Fixed,
    force: Fixed,
    distance: Fixed,
    hardness: Fixed,
    specific_cut_energy: Fixed,
    wear_max: Fixed,
    energy_max: Fixed,
) -> Fixed {
    // No load or no slide, no abrasive wear energy (Archard wear is proportional to force times
    // distance). This guard also means a body at REST wears nothing regardless of its hardness,
    // rather than inheriting `wear`'s zero-hardness "abrades without bound" volume convention when
    // there is no drive to abrade it (an unset zero-hardness material is a fail-loud manifest concern,
    // not a per-tick maximum).
    if force <= ZERO || distance <= ZERO {
        return ZERO;
    }
    let v = wear(
        wear_coefficient_scaled,
        coefficient_scale,
        force,
        distance,
        hardness,
        wear_max,
    );
    let e = v
        .checked_mul(specific_cut_energy)
        .and_then(|x| x.checked_mul(C_VOL))
        .unwrap_or(energy_max);
    e.min(energy_max)
}

// === Energy and thermal (R-PHYS-MECH, the energy sub-domain) ===

/// Steady conductive heat flux by Fourier's law, reassociated so the only multiply that
/// can wrap is the last, routed to the maximum flux; a zero conduction path is infinite
/// conductance (the maximum flux), and the gradient is taken as an absolute value.
pub fn conduction(
    conductivity: Fixed,
    area: Fixed,
    hot_temperature: Fixed,
    cold_temperature: Fixed,
    path_length: Fixed,
    max_flux: Fixed,
) -> Fixed {
    let delta_t = sat_abs(sat_sub(hot_temperature, cold_temperature));
    let k_dt = match conductivity.checked_mul(delta_t) {
        Some(x) => x,
        None => return max_flux,
    };
    if path_length == ZERO {
        return max_flux;
    }
    let geometry = match area.checked_div(path_length) {
        Some(x) => x,
        None => return max_flux,
    };
    match k_dt.checked_mul(geometry) {
        Some(f) => f.min(max_flux),
        None => max_flux,
    }
}

/// The sensible-heat energy to effect a temperature change: `m c dT`.
pub fn sensible_energy(
    mass: Fixed,
    specific_heat: Fixed,
    delta_t: Fixed,
    energy_max: Fixed,
) -> Fixed {
    // The law reports a non-negative sensible energy over [0, E_MAX] (a cooling contributes no
    // positive sensible heat), so a non-positive gradient reads zero. This also keeps the overflow
    // branch below sign-correct: it is reached only for a positive gradient, so the positive cap is
    // the right extreme rather than a sign-blind one.
    if delta_t <= ZERO {
        return ZERO;
    }
    let capacity = match mass.checked_mul(specific_heat) {
        Some(c) => c,
        None => return energy_max,
    };
    match capacity.checked_mul(delta_t) {
        Some(q) => q.min(energy_max),
        None => energy_max,
    }
}

/// The temperature rise from a delivered energy: `Q / (m c)`. An overflowed heat
/// capacity is an enormous thermal mass and gives a near-zero rise (the wave-1 F1 fix,
/// distinct from the massless case which gives the maximum swing).
pub fn sensible_rise(mass: Fixed, specific_heat: Fixed, energy: Fixed, rise_max: Fixed) -> Fixed {
    let capacity = match mass.checked_mul(specific_heat) {
        Some(c) => c,
        None => return ZERO,
    };
    // The divisor's zero-boundary is a declared physical-limit-at-zero (the floor invariant, slice-3 backstop):
    // a zero thermal capacity (a massless body) takes the maximum temperature swing `rise_max`, a physical
    // limit rather than the storage epsilon; the law keeps its overflow cap on `None`. Byte-neutral on the
    // PHYSICAL DOMAIN: `capacity = mass * specific_heat >= 0`, so `capacity <= ZERO` is exactly the prior
    // `capacity == ZERO` guard. That invariant is code-enforced below rather than only asserted in prose.
    debug_assert!(
        capacity >= ZERO,
        "sensible_rise: a thermal capacity (mass * specific_heat) is non-negative; the floor-invariant \
         wiring's byte-neutrality rests on it"
    );
    match civsim_units::guard::guarded_checked_div(
        energy,
        capacity,
        civsim_units::guard::ZeroGuard::LimitAtZero(rise_max),
    ) {
        Some(dt) => dt.min(rise_max),
        None => rise_max,
    }
}

/// The thermal diffusivity of a medium, `alpha = k / (rho * c)` (m^2/s): the conductivity `k` over
/// the volumetric heat capacity, the density `rho` times the specific heat `c` (Incropera and DeWitt,
/// Fundamentals of Heat and Mass Transfer). It is the material property that sets how fast a
/// temperature field conducts toward uniform, the physics the discrete field stencil's diffusion
/// coefficient is derived from: a medium is the lever (which substance fills the world), and its
/// diffusivity is this physics, not a free scalar. The same reassociation as `sensible_rise` (which
/// is `Q / (m*c)`): the volumetric heat capacity is formed first, so an overflow there reads as an
/// enormous heat capacity and a near-zero diffusivity, a zero heat capacity saturates to the cap, and
/// a divide overflow saturates to the cap. Deterministic: pinned `checked_mul` and `checked_div`, no
/// float. Nothing here reads a medium label; only its three thermal axes.
pub fn thermal_diffusivity(
    conductivity: Fixed,
    density: Fixed,
    specific_heat: Fixed,
    alpha_max: Fixed,
) -> Fixed {
    let capacity = match density.checked_mul(specific_heat) {
        Some(c) => c,
        None => return ZERO,
    };
    if capacity == ZERO {
        return alpha_max;
    }
    match conductivity.checked_div(capacity) {
        Some(a) => a.clamp(ZERO, alpha_max),
        None => alpha_max,
    }
}

/// The combined sensible-plus-latent energy of a phase transition, combined by an
/// order-independent saturating sum then capped, so two saturated terms cannot sum past
/// the declared interval (the wave-1 F7 fix).
pub fn phase_change_energy(
    mass: Fixed,
    specific_heat: Fixed,
    transition_temperature: Fixed,
    start_temperature: Fixed,
    latent_heat: Fixed,
    energy_max: Fixed,
) -> Fixed {
    let delta_t = sat_sub(transition_temperature, start_temperature);
    // `sensible_energy` is in joules (specific_heat is J/(kg*K)), while the latent term is in
    // kilojoules (latent_heat is kJ/kg); bridge the sensible term to kilojoules with C_KJ before
    // the sum so the two addends share one scale.
    let sensible_j =
        sensible_energy(mass, specific_heat, delta_t, energy_max).clamp(ZERO, energy_max);
    let sensible = match sensible_j.checked_div(C_KJ) {
        Some(x) => x,
        None => ZERO,
    };
    let latent = match mass.checked_mul(latent_heat) {
        Some(e) => e.min(energy_max),
        None => energy_max,
    };
    Fixed::saturating_sum([sensible, latent]).min(energy_max)
}

/// The limiting reactant of combustion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limiter {
    /// The source was below the ignition temperature.
    NotIgnited,
    /// Fuel-limited (oxidiser abundant or self-oxidising).
    Fuel,
    /// Oxidiser-limited.
    Oxidiser,
    /// Stoichiometrically balanced.
    Balanced,
}

/// The measured outputs of combustion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Combustion {
    /// The released energy.
    pub energy: Fixed,
    /// Whether the ignition temperature was crossed.
    pub ignited: bool,
    /// Which reactant limited the burn.
    pub limiter: Limiter,
}

/// Combustion: the released energy gated on the ignition crossing, with the limiting
/// reactant the lesser of the fuel and the oxidiser-limited fuel. A super-abundant
/// oxidiser (the division overflows) reads fuel-limited rather than the mislabel the
/// first form produced (the wave-1 NEW-L4-LIMITER fix). A zero oxidiser demand is a
/// self-oxidising substance.
#[allow(clippy::too_many_arguments)]
pub fn combustion(
    fuel_value: Fixed,
    oxidiser_demand: Fixed,
    ignition_temperature: Fixed,
    fuel_mass: Fixed,
    oxidiser_mass: Fixed,
    source_temperature: Fixed,
    energy_max: Fixed,
) -> Combustion {
    if source_temperature < ignition_temperature {
        return Combustion {
            energy: ZERO,
            ignited: false,
            limiter: Limiter::NotIgnited,
        };
    }
    let (ox_fuel, ox_overflow) = if oxidiser_demand == ZERO {
        (fuel_mass, true)
    } else {
        match oxidiser_mass.checked_div(oxidiser_demand) {
            Some(of) => (of, false),
            None => (fuel_mass, true),
        }
    };
    let burned = ox_fuel.min(fuel_mass);
    let limiter = if oxidiser_demand == ZERO || ox_overflow || ox_fuel > fuel_mass {
        Limiter::Fuel
    } else if ox_fuel < fuel_mass {
        Limiter::Oxidiser
    } else {
        Limiter::Balanced
    };
    let energy = match burned.checked_mul(fuel_value) {
        Some(e) => e.min(energy_max),
        None => energy_max,
    };
    Combustion {
        energy,
        ignited: true,
        limiter,
    }
}

/// Constrained thermal stress and the fracture crossing. The reducing constraint is
/// folded into the strain before the modulus multiply, so a constrained stress is not
/// pre-saturated (the wave-1 F3 fix); fracture is the absolute stress crossing the
/// fracture strength.
pub fn thermal_stress(
    modulus: Fixed,
    expansion_coefficient: Fixed,
    delta_t: Fixed,
    constraint: Fixed,
    fracture_strength: Fixed,
    stress_max: Fixed,
) -> (Fixed, bool) {
    let s1 = match expansion_coefficient.checked_mul(delta_t) {
        Some(x) => x,
        None => return (stress_max, true),
    };
    let strain_effective = match s1.checked_mul(constraint) {
        Some(x) => x,
        None => return (stress_max, true),
    };
    // `modulus` is megapascals (stored = Pa / 1e6) and `expansion_coefficient` is ppm/K (stored =
    // strain-per-K x 1e6); their product cancels the two prefixes to leave pascals, so descale by
    // C_PA to the megapascal that `fracture_strength` and `stress_max` are on, or a mild heating
    // reads 1e6 too high and fractures spuriously.
    let sigma_pa = match modulus.checked_mul(strain_effective) {
        Some(x) => x,
        None => return (stress_max, true),
    };
    let sigma = match sigma_pa.checked_div(C_PA) {
        Some(x) => x.clamp(-stress_max, stress_max),
        None => return (stress_max, true),
    };
    let fractured = sigma.abs() >= fracture_strength;
    (sigma, fractured)
}

/// Saturating absolute value: `Fixed::MIN.abs()` would panic (i64::MIN negation), so the extreme
/// magnitude routes to the ceiling rather than panicking, keeping the kernels total.
#[inline]
fn sat_abs(v: Fixed) -> Fixed {
    if v == Fixed::MIN {
        Fixed::MAX
    } else {
        v.abs()
    }
}

/// Saturating difference in i128, so a subtraction of two saturated sums cannot panic or wrap; an
/// out-of-range result routes to the signed extreme.
#[inline]
fn sat_sub(a: Fixed, b: Fixed) -> Fixed {
    let d = (a.to_bits() as i128) - (b.to_bits() as i128);
    Fixed::from_bits_i128(d).unwrap_or(if d < 0 { Fixed::MIN } else { Fixed::MAX })
}

// === Fluids, weather, and acoustics (R-PHYS-W2, wave 2) ===
//
// All kernels are closed-form integer over the Fixed ops, staged so a product that could exceed the
// Q32.32 ceiling is either checked (routing to the reserved physical-limit cap) or reduced before it
// forms. Caps are reserved representability limits passed by the caller. `Fixed::sqrt` is the exact
// deterministic integer isqrt; the speed of sound uses it after a divide, and takes the megapascal
// bridge as a factor of one thousand (the square root of the pinned MPa scale) so the pascal-scale
// modulus never materialises.

/// Hydrostatic pressure P = rho*g*h (MPa). The megapascal bridge is applied before the depth multiply
/// so a deep column is not pre-saturated. A zero column reads zero.
pub fn hydrostatic_pressure(density: Fixed, gravity: Fixed, height: Fixed, p_max: Fixed) -> Fixed {
    if height <= ZERO {
        return ZERO;
    }
    let rho_g = match density.checked_mul(gravity) {
        Some(x) => x,
        None => return p_max,
    };
    let per_m = match rho_g.checked_div(C_PA) {
        Some(x) => x,
        None => return p_max,
    };
    match per_m.checked_mul(height) {
        Some(p) => p.min(p_max),
        None => p_max,
    }
}

/// Buoyant force F = rho*g*V (N), Archimedes. The float-versus-sink comparison to the weight is the
/// agent layer, not this law.
pub fn buoyant_force(density: Fixed, gravity: Fixed, volume: Fixed, f_max: Fixed) -> Fixed {
    let rho_g = match density.checked_mul(gravity) {
        Some(x) => x,
        None => return f_max,
    };
    match rho_g.checked_mul(volume) {
        Some(f) => f.min(f_max),
        None => f_max,
    }
}

/// Dynamic (stagnation) pressure q = (1/2) rho v^2 (MPa). The half and the MPa bridge are applied to
/// density before the squared velocity (the kinetic-energy staging); a velocity past the overflow-safe
/// square ceiling routes to the cap.
pub fn dynamic_pressure(density: Fixed, velocity: Fixed, p_max: Fixed) -> Fixed {
    if sat_abs(velocity) > V2_MAX {
        return p_max;
    }
    let v2 = match velocity.checked_mul(velocity) {
        Some(x) => x,
        None => return p_max,
    };
    let rh = match density.checked_mul(HALF) {
        Some(x) => x,
        None => return p_max,
    };
    let coeff = match rh.checked_div(C_PA) {
        Some(x) => x,
        None => return p_max,
    };
    match coeff.checked_mul(v2) {
        Some(q) => q.min(p_max),
        None => p_max,
    }
}

/// Aerodynamic force (1/2) C rho A v^2 (N), shared by drag (C = drag coefficient) and lift (C = lift
/// coefficient); the two differ only in the coefficient. The coefficient product is built before the
/// squared velocity.
fn aero_force(
    coefficient: Fixed,
    density: Fixed,
    area: Fixed,
    velocity: Fixed,
    f_max: Fixed,
) -> Fixed {
    if sat_abs(velocity) > V2_MAX {
        return f_max;
    }
    let v2 = match velocity.checked_mul(velocity) {
        Some(x) => x,
        None => return f_max,
    };
    let c1 = match density.checked_mul(HALF) {
        Some(x) => x,
        None => return f_max,
    };
    let c2 = match c1.checked_mul(coefficient) {
        Some(x) => x,
        None => return f_max,
    };
    let c3 = match c2.checked_mul(area) {
        Some(x) => x,
        None => return f_max,
    };
    match c3.checked_mul(v2) {
        Some(f) => f.min(f_max),
        None => f_max,
    }
}

/// Drag force (1/2) Cd rho A v^2 (N).
pub fn drag_force(
    drag_coefficient: Fixed,
    density: Fixed,
    area: Fixed,
    velocity: Fixed,
    f_max: Fixed,
) -> Fixed {
    aero_force(drag_coefficient, density, area, velocity, f_max)
}

/// Aerodynamic lift (1/2) Cl rho A v^2 (N), the reduced-order lumped-coefficient lift that floors a
/// wing, a gliding creature, a sail, and the lift half of a ballistic arc.
pub fn aerodynamic_lift(
    lift_coefficient: Fixed,
    density: Fixed,
    area: Fixed,
    velocity: Fixed,
    f_max: Fixed,
) -> Fixed {
    aero_force(lift_coefficient, density, area, velocity, f_max)
}

/// Reynolds number Re = rho*|v|*L/mu (dimensionless), a laminar/turbulent regime gate. The transition
/// Reynolds number is a reserved consumer constant, kept out of the kernel. Zero speed reads zero, an
/// inviscid fluid reads the cap.
pub fn reynolds_number(
    density: Fixed,
    velocity: Fixed,
    length: Fixed,
    viscosity: Fixed,
    re_max: Fixed,
) -> Fixed {
    let speed = sat_abs(velocity);
    if speed == ZERO {
        return ZERO;
    }
    if viscosity == ZERO {
        return re_max;
    }
    let rv = match density.checked_mul(speed) {
        Some(x) => x,
        None => return re_max,
    };
    // Multiply the characteristic length in before dividing by the (possibly tiny) viscosity:
    // dividing first sends `rho*v/mu` past the ceiling for a small channel even when the true
    // Re (with a sub-metre length) is representable. If `rho*v*L` overflows, the Reynolds number
    // is genuinely out of range and the cap is the right extreme.
    let rvl = match rv.checked_mul(length) {
        Some(x) => x,
        None => return re_max,
    };
    match rvl.checked_div(viscosity) {
        Some(x) => x.min(re_max),
        None => re_max,
    }
}

/// Young-Laplace curvature pressure dP = 2*gamma/r (MPa). Zero radius reads the cap (infinite
/// curvature). Divide-only, so no overflow product forms.
pub fn laplace_pressure(surface_tension: Fixed, radius: Fixed, p_max: Fixed) -> Fixed {
    if radius <= ZERO {
        return p_max;
    }
    let two_g = match surface_tension.checked_mul(Fixed::from_int(2)) {
        Some(x) => x,
        None => return p_max,
    };
    let pa = match two_g.checked_div(radius) {
        Some(x) => x,
        None => return p_max,
    };
    match pa.checked_div(C_PA) {
        Some(x) => x.min(p_max),
        None => p_max,
    }
}

/// Volumetric strain dV/V = dP/K (dimensionless). Zero bulk modulus reads the cap. No product.
pub fn compressibility(pressure: Fixed, bulk_modulus: Fixed, strain_max: Fixed) -> Fixed {
    match pressure.checked_div(bulk_modulus) {
        Some(s) => s.clamp(ZERO, strain_max),
        None => strain_max,
    }
}

/// Newton convective cooling q = h*A*|T_hot - T_cold| (W), the body arc's convective exchange. The
/// absolute-value gradient matches conduction.
pub fn convective_flux(h: Fixed, area: Fixed, hot: Fixed, cold: Fixed, flux_max: Fixed) -> Fixed {
    let dt = sat_abs(sat_sub(hot, cold));
    let ha = match h.checked_mul(area) {
        Some(x) => x,
        None => return flux_max,
    };
    match ha.checked_mul(dt) {
        Some(x) => x.min(flux_max),
        None => flux_max,
    }
}

/// Fick's-law membrane gas exchange in the mass-transfer (Sherwood) form: J = k*A*(c_medium -
/// c_internal) (kg/s), the signed rate at which a respiratory surface exchanges the respirable species
/// with the medium it sits in (R-MEDIUM). Positive is uptake from a richer medium; negative is loss to
/// a poorer one (a water-breather in air off-gassing and suffocating). The concentration difference is
/// a signed saturating subtract over `fluid.respirable_content` (both ports read that one axis, as
/// `convective_flux` differences one temperature axis), so equal concentrations are zero flux
/// (equilibrium, no authored preference) and the sign is the exchange direction. The magnitude is
/// capped at the reserved representability limit; a zero coefficient or area (no exchange surface) reads
/// zero. Nothing here reads a medium label: only the respirable content of the medium the surface sits
/// in, so a gill in water and a lung in air are the same kernel over different concentrations
/// (Principle 9).
pub fn membrane_gas_flux(
    coefficient: Fixed,
    area: Fixed,
    c_medium: Fixed,
    c_internal: Fixed,
    flux_max: Fixed,
) -> Fixed {
    let lo = ZERO - flux_max;
    let dc = sat_sub(c_medium, c_internal);
    let ka = match coefficient.checked_mul(area) {
        Some(x) => x,
        None => return if dc < ZERO { lo } else { flux_max },
    };
    match ka.checked_mul(dc) {
        Some(j) => j.clamp(lo, flux_max),
        None => {
            if dc < ZERO {
                lo
            } else {
                flux_max
            }
        }
    }
}

/// Hagen-Poiseuille laminar flow Q = pi*dP*r^4/(8*mu*L) (m^3/s). The driving pressure is bridged to
/// pascals, then the radius multiplies and the viscosity, length, and 8 divides are interleaved so
/// no intermediate overflows a representable flow or underflows a capillary to zero. A frictionless
/// or zero-length channel reads the cap; zero radius or pressure reads zero.
pub fn poiseuille_flow(
    dp: Fixed,
    radius: Fixed,
    viscosity: Fixed,
    length: Fixed,
    q_max: Fixed,
) -> Fixed {
    if radius <= ZERO || dp <= ZERO {
        return ZERO;
    }
    if viscosity == ZERO || length == ZERO {
        return q_max;
    }
    let pa = match dp.checked_mul(C_PA) {
        Some(x) => x,
        None => return q_max,
    };
    // Interleave the four radius multiplies with the viscosity, length, and 8 divides so the
    // running value tracks the (bounded) true flow. Dividing by the tiny viscosity up front sends
    // `dp/mu` past the ceiling for a representable flow, and applying r^4 up front underflows a
    // capillary to zero; alternating grow and shrink keeps every intermediate near the result.
    // A genuinely out-of-range flow (a large radius) still overflows to the cap.
    let q = pa
        .checked_mul(radius)
        .and_then(|x| x.checked_div(viscosity))
        .and_then(|x| x.checked_mul(radius))
        .and_then(|x| x.checked_div(length))
        .and_then(|x| x.checked_mul(radius))
        .and_then(|x| x.checked_div(Fixed::from_int(8)))
        .and_then(|x| x.checked_mul(radius))
        .and_then(|x| x.checked_mul(Fixed::PI));
    match q {
        Some(x) => x.min(q_max),
        None => q_max,
    }
}

/// Speed of sound c = sqrt(K/rho) (m/s). The modulus stays on the megapascal scale and the pascal
/// bridge is taken as a factor of one thousand (the square root of the MPa scale) after the root, so
/// the pascal-scale modulus (which would overflow for water) never forms. Zero density reads the cap.
pub fn speed_of_sound(bulk_modulus: Fixed, density: Fixed, c_max: Fixed) -> Fixed {
    if density <= ZERO {
        return c_max;
    }
    let ratio = match bulk_modulus.checked_div(density) {
        Some(x) => x,
        None => return c_max,
    };
    match ratio.sqrt().checked_mul(Fixed::from_int(1000)) {
        Some(c) => c.min(c_max),
        None => c_max,
    }
}

/// Stokes thermoviscous sound absorption alpha = reference * f^2 (1/m), the frequency-squared law that
/// makes a medium absorb high frequencies over distance (an authored universal physics affordance,
/// Principle 9). Report the linear absorption coefficient; its path attenuation is the existing
/// [`optical_depth`] (report the measured indicator, defer the transcendental transform), so no new
/// path kernel is introduced. The square is staged as `reference*frequency` then `*frequency`, so the
/// tiny per-square-frequency reference shrinks the product before the second frequency grows it and
/// the unrepresentable f^2 never forms; the two staged [`Fixed::checked_mul`] carry the overflow
/// guard, routing a genuinely unrepresentable product to the cap. There is no frequency-alone early
/// return: a small reference keeps `reference*f*f` representable well past any single-multiply
/// ceiling, so gating on the frequency alone would over-saturate a low-reference medium at a high
/// frequency it still absorbs finitely. A non-positive frequency has no absorption and reads zero.
pub fn acoustic_absorption(reference: Fixed, frequency: Fixed, alpha_max: Fixed) -> Fixed {
    if frequency <= ZERO {
        return ZERO;
    }
    // Interleave the two frequency multiplies around the reference (the poiseuille grow/shrink
    // discipline): reference*frequency stays tiny, then the second *frequency lifts it to the
    // absorption scale, so no intermediate exceeds the representable ceiling for a physical reference.
    // Only a product that truly overflows i64 routes to the cap.
    match reference
        .checked_mul(frequency)
        .and_then(|x| x.checked_mul(frequency))
    {
        Some(a) => a.clamp(ZERO, alpha_max),
        None => alpha_max,
    }
}

/// Quarter-wave closed-open tube resonance f_n = (2n-1)*c/(4L) (Hz), the source-filter formant law (an
/// authored universal physics affordance, Principle 9): a tube closed at one end and open at the other
/// resonates on the odd harmonics of c/(4L), the standing-wave series a vocal tract (or a stopped horn)
/// imposes on a sound speed c and a resonator length L. Stage c/L first, then apply the odd multiplier
/// (2n-1) and the quarter-wave divide by four, so the large intermediate (2n-1)*c never forms; a zero
/// or near-zero length overflows the divide and reads the cap (the frequency grows without bound as L
/// shrinks), and a zero sound speed reads zero (no medium, no resonance, the speed-of-sound zero-guard).
/// A non-positive mode number has no resonance and reads zero.
pub fn tube_resonance(
    harmonic: Fixed,
    speed_of_sound: Fixed,
    resonator_length: Fixed,
    freq_max: Fixed,
) -> Fixed {
    if speed_of_sound <= ZERO {
        return ZERO;
    }
    if resonator_length <= ZERO {
        return freq_max;
    }
    // The odd multiplier (2n-1) of the closed-open quarter-wave series; a non-positive mode has none.
    let odd = sat_sub(harmonic.saturating_add(harmonic), ONE);
    if odd <= ZERO {
        return ZERO;
    }
    let c_over_l = match speed_of_sound.checked_div(resonator_length) {
        Some(x) => x,
        None => return freq_max,
    };
    let scaled = match c_over_l.checked_mul(odd) {
        Some(x) => x,
        None => return freq_max,
    };
    match scaled.checked_div(Fixed::from_int(4)) {
        Some(f) => f.min(freq_max),
        None => freq_max,
    }
}

/// Ideal-gas density rho = P/(R_s*T) (kg/m^3), the coupling that lets the temperature field drive the
/// density field. The pressure is bridged to pascals. A zero or sub-floor R_s*T reads the dense cap.
pub fn ideal_gas_density(
    pressure: Fixed,
    temperature: Fixed,
    gas_constant: Fixed,
    rho_min: Fixed,
    rho_max: Fixed,
) -> Fixed {
    let pa = match pressure.checked_mul(C_PA) {
        Some(x) => x,
        None => return rho_max,
    };
    let rt = match gas_constant.checked_mul(temperature) {
        Some(x) => x,
        // rho = P/(R*T): an overflowing R*T denominator drives the density toward zero, so route to
        // the minimum, not the maximum. A vanishing R*T (below) is the dense extreme.
        None => return rho_min,
    };
    if rt <= ZERO {
        return rho_max;
    }
    match pa.checked_div(rt) {
        Some(r) => r.clamp(rho_min, rho_max),
        None => rho_max,
    }
}

/// Boussinesq natural-convection acceleration a = g*(T_parcel - T_ambient)/T_ambient (m/s^2), signed
/// up when the parcel is warmer, using the ideal-gas 1/T thermal expansion. The `1/T_ambient` divisor's
/// floor DERIVES from the physics, not an owner value (the R-UNITS-PIN floor invariant): T_ambient is an
/// ABSOLUTE temperature, so `T_ambient > 0` is the third-law physical floor (absolute zero is unreachable),
/// which is why the divide by T_ambient is safe below without riding the storage epsilon. A non-positive
/// T_ambient is non-physical, and the ZERO it returns is the ABSENCE convention (no ambient medium, no
/// buoyant coupling), a declared physical-limit-at-zero rather than a fabricated substitute.
pub fn thermal_buoyancy(t_parcel: Fixed, t_ambient: Fixed, gravity: Fixed, a_max: Fixed) -> Fixed {
    // The declared physical-limit-at-zero: the third-law floor is T_ambient > 0, so a non-positive absolute
    // temperature is off the physical domain and reads the absence convention (no buoyancy).
    if t_ambient <= ZERO {
        return ZERO;
    }
    let lo = sat_sub(ZERO, a_max);
    let dt = sat_sub(t_parcel, t_ambient);
    let ratio = match dt.checked_div(t_ambient) {
        Some(x) => x,
        // A huge |dt|/T is a large signed acceleration: route by the sign of dt, matching the
        // multiply-overflow branch below, rather than to zero (which reads as no buoyancy).
        None => return if dt < ZERO { lo } else { a_max },
    };
    match ratio.checked_mul(gravity) {
        Some(a) => a.clamp(lo, a_max),
        None => {
            if dt < ZERO {
                lo
            } else {
                a_max
            }
        }
    }
}

/// Saturation vapour pressure e_s = e_ref + slope*(T - T_ref) (MPa), the affine tangent to the
/// Clausius-Clapeyron curve over the simulated band (the exact exp/log curve is deferred to
/// R-GPU-CANON-PIN). Clamped to [0, cap]; valid within about twenty kelvin of the reference.
pub fn saturation_vapor_pressure(
    temperature: Fixed,
    slope: Fixed,
    t_ref: Fixed,
    e_ref: Fixed,
    es_cap: Fixed,
) -> Fixed {
    let dt = sat_sub(temperature, t_ref);
    let term = match slope.checked_mul(dt) {
        Some(x) => x,
        None => {
            if dt < ZERO {
                return ZERO;
            } else {
                return es_cap;
            }
        }
    };
    e_ref.saturating_add(term).clamp(ZERO, es_cap)
}

/// Evaporation mass flux E = (a + b*|u|)*(e_s - e_a) (kg/(m^2*s)), the Dalton bulk aerodynamic proxy.
/// Returns the evaporation source when the vapour-pressure deficit is positive; a non-positive deficit
/// is the condensation case and reads zero here (the sink is the caller's sign-flipped difference).
pub fn evaporation_rate(
    e_ambient: Fixed,
    e_saturation: Fixed,
    wind: Fixed,
    a_still: Fixed,
    b_wind: Fixed,
    e_max: Fixed,
) -> Fixed {
    let vpd = sat_sub(e_saturation, e_ambient);
    if vpd <= ZERO {
        return ZERO;
    }
    let wind_fn = match b_wind.checked_mul(sat_abs(wind)) {
        Some(x) => a_still.saturating_add(x),
        // An overflowing wind term is an unbounded transfer coefficient over a positive deficit, so
        // the evaporation saturates at the cap, not back down to the still-air baseline.
        None => return e_max,
    };
    match wind_fn.checked_mul(vpd) {
        Some(e) => e.min(e_max),
        None => e_max,
    }
}

// === Chemistry (R-PHYS-W2, wave 2) ===

/// Reaction enthalpy delta_h = sum(product formation enthalpies) - sum(reactants) per kg, and whether
/// the barrier is crossed (`temperature >= barrier`, the generalization of combustion's ignition
/// gate). The caller forms the mass-weighted sums by `Fixed::saturating_sum` (order-independent);
/// this kernel takes them. A negative delta_h is exothermic. Which reactions occur emerges from the
/// sign over the substance vectors, never an authored recipe (Hess's law).
pub fn reaction(
    products_sum: Fixed,
    reactants_sum: Fixed,
    temperature: Fixed,
    barrier: Fixed,
) -> (Fixed, bool) {
    // Saturating in i128: the sums are order-independent saturating_sums bounded only by i64, so the
    // difference of opposite-signed extremes must not panic or wrap.
    (sat_sub(products_sum, reactants_sum), temperature >= barrier)
}

/// Corrosion driving margin (a rate proxy): the oxidiser-minus-material potential, times the
/// material susceptibility, times a monotone acidity factor. A thermodynamically uphill pairing
/// (non-positive driving) does not attack. Reports the driving margin; the exponential Tafel rate is
/// deferred. The pairing emerges from the measured potentials, not an authored table (this is the
/// wave-2 corrosion the R-WOUND corrosion mode and R-FLUID corrosion were flagged against).
pub fn corrosion(
    fluid_potential: Fixed,
    material_potential: Fixed,
    susceptibility: Fixed,
    acidity_factor: Fixed,
    corrosion_max: Fixed,
) -> Fixed {
    let driving = sat_sub(fluid_potential, material_potential);
    if driving <= ZERO {
        return ZERO;
    }
    let r1 = match driving.checked_mul(susceptibility) {
        Some(x) => x,
        None => return corrosion_max,
    };
    // `acidity_factor` is the pH (0 most acidic, 14 most basic); acid attack rises as pH falls, so
    // the aggressiveness is the distance below the pH ceiling. The 14 is the definitional pH scale
    // maximum (the chem.acidity axis range), a scale bound, not a fabricated realism value.
    let aggressiveness = sat_sub(Fixed::from_int(14), acidity_factor).max(ZERO);
    match r1.checked_mul(aggressiveness) {
        Some(x) => x.min(corrosion_max),
        None => corrosion_max,
    }
}

/// Ideal Carnot efficiency eta = 1 - Tc/Th, the maximum thermodynamic efficiency (the ideal end of
/// the heat-to-work ceiling; the real irreversible cycle is deferred). A non-positive gradient yields
/// zero.
pub fn carnot_limit(hot: Fixed, cold: Fixed) -> Fixed {
    if hot <= cold || hot <= ZERO {
        return ZERO;
    }
    let ratio = match cold.checked_div(hot) {
        Some(x) => x,
        None => return ZERO,
    };
    sat_sub(Fixed::ONE, ratio).clamp(ZERO, Fixed::ONE)
}

/// Dissolution leach fraction: the fraction of a solute extracted into a solvent, its solute affinity
/// times the solvent aggressiveness, clamped to `[0, 1]`. The soak-and-leach of medicine and
/// preparation (detox, tincture, decoction); the time-resolved Noyes-Whitney rate is deferred.
pub fn dissolution(solute_affinity: Fixed, solvent_aggressiveness: Fixed) -> Fixed {
    match solute_affinity.checked_mul(solvent_aggressiveness) {
        Some(x) => x.clamp(ZERO, Fixed::ONE),
        None => Fixed::ONE,
    }
}

// === Optics and signal (R-PHYS-W2, wave 2) ===

/// Radiant heat exchange j = emissivity*sigma*(T_hot^4 - T_cold^4) (W), Stefan-Boltzmann, absorbing
/// the wave-1 radiant-heat ceiling. Sigma is interleaved with the four temperature multiplies so no
/// intermediate fourth power materialises; the emissive power reaches the Q32.32 ceiling near
/// T ~ 14000 K (blue-star and plasma), above which a surface routes to the cap (an honest Tier-0
/// limit; a forge and a solar surface are well within). A cooler surface than its surroundings emits
/// nothing net here (the absorption side is the caller's).
pub fn radiant_emission(
    emissivity: Fixed,
    area: Fixed,
    t_hot: Fixed,
    t_cold: Fixed,
    sigma: Fixed,
    flux_max: Fixed,
) -> Fixed {
    let fourth = |t: Fixed| {
        sigma
            .checked_mul(t)
            .and_then(|x| x.checked_mul(t))
            .and_then(|x| x.checked_mul(t))
            .and_then(|x| x.checked_mul(t))
    };
    let e_hot = match fourth(t_hot) {
        Some(x) => x,
        None => return flux_max,
    };
    let e_cold = match fourth(t_cold) {
        Some(x) => x,
        None => return flux_max,
    };
    if e_hot < e_cold {
        return ZERO;
    }
    let net = e_hot - e_cold;
    match net
        .checked_mul(emissivity)
        .and_then(|x| x.checked_mul(area))
    {
        Some(q) => q.min(flux_max),
        None => flux_max,
    }
}

/// The Tier-2 radiant heat exchange (R-UNITS-PIN slice 4): the same Stefan-Boltzmann law as
/// [`radiant_emission`], but with sigma entering at its FULL derived precision (a fine `(bits, scale)` pair
/// from the fundamentals) instead of the roughly eight-significant-bit Q32.32 truncation `radiant_emission`
/// carries (`244 x 2^-32`). The precision-critical term `sigma * (T_hot^4 - T_cold^4)` is computed in ONE
/// wide accumulator (the slice-1 `WideAccum`, i256): the two quartics are formed and subtracted EXACTLY (the
/// difference-of-quartics cancellation is lossless, unlike forming each quartic in Q32.32 and subtracting the
/// two rounded values), sigma multiplies in at its fine scale, and the chain rounds ONCE to Q32.32. That
/// net radiant power then scales by emissivity and area in Q32.32, exactly as `radiant_emission` does (both
/// are O(1)-range factors the canonical fixed-point holds without loss), and the same [`FLUX_MAX`] cap
/// applies. A surface cooler than its surroundings (`t_hot < t_cold`) emits nothing net, and a plasma-hot
/// surface whose net term overruns the Q32.32 range routes to the cap, both the same zero-branch and
/// representability-cap semantics as `radiant_emission` (a directional match on the caps, which sit well above
/// physiological temperatures, not a bit-identical threshold with the interleaved form).
///
/// The wide accumulator holds the whole envelope: `sigma * T^4` at the planner's scales reaches about 210
/// bits (the gate's hardware validation), inside i256, and only the final round to Q32.32 can overflow i64
/// (the plasma cap). Keeping emissivity and area as a Q32.32 tail rather than folding them into the wide
/// chain is deliberate: the full `sigma * T^4 * emissivity * area` at Q32.32 scales would exceed i256, and
/// coarsening the inputs to fit would trade their precision for sigma's, so the lift isolates sigma's
/// correction to the term that carries it. Deterministic and float-free (the wide path is integer-only).
pub fn radiant_emission_tier2(
    emissivity: Fixed,
    area: Fixed,
    t_hot: Fixed,
    t_cold: Fixed,
    sigma_bits: i64,
    sigma_scale: u32,
    flux_max: Fixed,
) -> Fixed {
    use civsim_units::plan::{evaluate, LawExpr};
    // sigma * (T_hot^4 - T_cold^4): input 0 sigma (fine scale), 1 T_hot, 2 T_cold (Q32.32). Sigma folds into
    // the difference-of-quartics chain as a scalar leaf (the wide accumulator multiplies a scalar mantissa in).
    let expr = LawExpr::Mul(
        Box::new(LawExpr::Sub(
            Box::new(LawExpr::Powi(Box::new(LawExpr::Input(1)), 4)),
            Box::new(LawExpr::Powi(Box::new(LawExpr::Input(2)), 4)),
        )),
        Box::new(LawExpr::Input(0)),
    );
    let net = match evaluate(
        &expr,
        &|q| match q {
            0 => (sigma_bits, sigma_scale),
            1 => (t_hot.to_bits(), Fixed::FRAC_BITS),
            _ => (t_cold.to_bits(), Fixed::FRAC_BITS),
        },
        Fixed::FRAC_BITS,
    ) {
        // The net radiant power at Q32.32; an i64 overflow (a plasma-hot surface) routes to the cap.
        Some(bits) => Fixed::from_bits(bits),
        None => return flux_max,
    };
    if net <= ZERO {
        // Cooler than the surroundings: no net emission (the `e_hot < e_cold` branch of `radiant_emission`).
        return ZERO;
    }
    match net
        .checked_mul(emissivity)
        .and_then(|x| x.checked_mul(area))
    {
        Some(q) => q.min(flux_max),
        None => flux_max,
    }
}

/// Wien peak wavelength lambda = b/T (m), grounding colour-from-temperature (a hot forge glows). Zero
/// temperature reads the long-wavelength cap.
pub fn wien_peak(temperature: Fixed, wien_b: Fixed, wavelength_max: Fixed) -> Fixed {
    match wien_b.checked_div(temperature) {
        Some(x) => x.min(wavelength_max),
        None => wavelength_max,
    }
}

/// Inverse-square irradiance E = P/(4*pi*r^2) (W/m^2), the geometric-spreading half of a stimulus's
/// spatial reach (light or sound). A distant source (the r^2 or 4*pi*r^2 product past the ceiling) is
/// negligible (zero); a source at zero distance reads the cap.
pub fn inverse_square_falloff(
    power: Fixed,
    distance: Fixed,
    four_pi: Fixed,
    irrad_max: Fixed,
) -> Fixed {
    let r2 = match distance.checked_mul(distance) {
        Some(x) => x,
        None => return ZERO,
    };
    let denom = match four_pi.checked_mul(r2) {
        Some(x) => x,
        None => return ZERO,
    };
    // The divisor's zero-boundary is a declared physical-limit-at-zero (the floor invariant, slice-3 backstop):
    // a zero distance (the source at the point) reads the irradiance cap `irrad_max`, a physical limit rather
    // than the storage epsilon. Byte-neutral (`denom = 4*pi*r^2 >= 0`, so the boundary is `== ZERO`); the law
    // keeps its overflow cap on `None`.
    match civsim_units::guard::guarded_checked_div(
        power,
        denom,
        civsim_units::guard::ZeroGuard::LimitAtZero(irrad_max),
    ) {
        Some(e) => e.min(irrad_max),
        None => irrad_max,
    }
}

/// General geometric spreading `E = power / (sphere_coeff * distance^(D-1))`, the
/// dimensionality-parameterized form of a stimulus's geometric spatial reach. A point source's
/// intensity spreads over the surface of a `(D-1)`-sphere of radius `distance` in `D`-dimensional
/// space, whose area is `sphere_coeff * distance^(D-1)`; `sphere_coeff` is the `(D-1)`-sphere surface
/// coefficient the caller supplies (`4*pi` for a 3D bulk, `2*pi` for a 2D surface, the duct coefficient
/// for a 1D line). At `dimensionality == 3` with `sphere_coeff == 4*pi` this reproduces
/// [`inverse_square_falloff`] exactly (byte-identical: the same `distance^2` and the same divide); at
/// `dimensionality == 2` it is `1/distance`; at `dimensionality == 1` the exponent is zero, so there is
/// no radial spreading (a duct). The dimensionality DERIVES from the geometry of the space the signal
/// traverses and is never an authored per-channel constant (the reach-substrate value-authoring rule).
/// A source so distant that the staged product overflows is negligible (zero); a source at zero
/// distance, or any zero denominator, reads the cap.
pub fn geometric_spread(
    power: Fixed,
    distance: Fixed,
    dimensionality: u32,
    sphere_coeff: Fixed,
    irrad_max: Fixed,
) -> Fixed {
    // distance^(D-1): the signal spreads over the surface of a (D-1)-sphere of radius = distance. The
    // staged multiply carries the same overflow-to-zero discipline as inverse_square_falloff, so a
    // distant source is negligible rather than wrapping. At D = 3 this yields distance^2 exactly, so
    // the divide below matches inverse_square_falloff bit for bit.
    let mut spread = Fixed::ONE;
    for _ in 0..dimensionality.saturating_sub(1) {
        spread = match spread.checked_mul(distance) {
            Some(x) => x,
            None => return ZERO,
        };
    }
    let denom = match sphere_coeff.checked_mul(spread) {
        Some(x) => x,
        None => return ZERO,
    };
    if denom == ZERO {
        return irrad_max;
    }
    match power.checked_div(denom) {
        Some(e) => e.min(irrad_max),
        None => irrad_max,
    }
}

/// The monotone response law a being's sensory channel transduces a received magnitude by: a physics-floor
/// family of established sensory psychophysics (Principle 9), where the mechanism is fixed Rust and the
/// SELECTION and its parameters are the being's own data (Principle 11). A lineage whose sense compresses,
/// expands, or responds linearly is a different variant or a different shape value, a data row, never a code
/// rewrite. [`ResponseLaw::Linear`] is the degenerate default: [`transduce`] under it reproduces
/// `magnitude * gain` bit-for-bit, so the family strictly generalizes a plain linear sensitivity.
///
/// SCOPE and its flagged limits (the slice-2 audit named these): the family is the MONOTONE, unbounded
/// responses (linear, power-law expansive or compressive, logarithmic), and [`transduce`] clamps every one
/// to `activation_max`, so the ceiling is a hard clip rather than a smooth saturation. Two response shapes
/// real receptors exhibit are NOT yet in the family, so they are flagged floor extensions (a new variant
/// plus its law, the strict-generalization pattern), not authored elsewhere: a SATURATING response
/// (Naka-Rushton or Hill, `activation = gain * m^n / (k^n + m^n)`, the dominant real transducer nonlinearity
/// and the natural shape for a finite-ceiling mana or redox receptor), and a NON-MONOTONE tuned or band-pass
/// response (a receptor with a preferred magnitude, peaking then falling). Until those variants land, a
/// saturating or tuned sense is not a data row under this family; the admit-the-alien claim holds for the
/// monotone shapes only.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ResponseLaw {
    /// Linear: `activation = gain * magnitude`. The degenerate default, bit-identical to a plain
    /// multiplicative sensitivity.
    Linear,
    /// Stevens power law: `activation = gain * magnitude^shape`. `shape < 1` compresses (a
    /// diminishing-returns sense), `shape > 1` expands. Near `shape = 1` it approximates Linear (through the
    /// transcendental path, not bit-identically, so Linear is the default for the exact linear case).
    Power,
    /// Fechner logarithm: `activation = gain * ln(1 + shape * magnitude)`. Compresses a wide dynamic range,
    /// zero at zero magnitude, `shape` sets the compression.
    LogCompressive,
}

/// Transduce a received magnitude into an internal activation through a being's own monotone response law,
/// clamped to `[0, activation_max]`. Pure and deterministic (the fixed-point `ln`/`powf` are the pinned
/// integer transcendentals). The response SHAPE is the being's data, never the mechanism: `Linear` with a
/// gain is the degenerate default and reproduces `magnitude * gain` bit-for-bit (a strict generalization),
/// so a logarithmic, power-law, or (with a threshold the caller applies) thresholded sense is a data row,
/// not a rewrite. A non-positive magnitude has no percept and reads zero.
pub fn transduce(
    magnitude: Fixed,
    law: ResponseLaw,
    gain: Fixed,
    shape: Fixed,
    activation_max: Fixed,
) -> Fixed {
    if magnitude <= ZERO {
        return ZERO;
    }
    let raw = match law {
        ResponseLaw::Linear => magnitude.checked_mul(gain).unwrap_or(activation_max),
        ResponseLaw::Power => match magnitude.powf(shape).checked_mul(gain) {
            Some(a) => a,
            None => activation_max,
        },
        ResponseLaw::LogCompressive => {
            let scaled = match shape.checked_mul(magnitude) {
                Some(x) => x,
                None => return activation_max,
            };
            // The Fechner argument `1 + shape*magnitude` stays above zero by the law's own DOMAIN, derived
            // from the physics rather than set: `magnitude > 0` (guarded above) and `shape >= 0` (the
            // monotone-compressive contract, "monotone shapes only" per this law's doc), so the argument is at
            // or above one. That derived floor, not the storage epsilon, bounds the log (Principle 10, the
            // R-UNITS-PIN floor invariant): `guarded_ln` clamps the argument up to the derived floor ONE, so it
            // is byte-neutral on the contract (the argument is already >= 1, so the clamp is a no-op and the log
            // is exact) and fail-safe if a mis-declared negative shape ever drove the argument below the domain,
            // rather than the silent `ln(arg<=0) -> Fixed::MIN` sentinel it rode before. No value is authored:
            // the floor is the physics of the compressive law.
            let arg = Fixed::ONE + scaled;
            match civsim_units::guard::guarded_ln(
                arg,
                civsim_units::guard::ZeroGuard::Floor(Fixed::ONE),
            )
            .checked_mul(gain)
            {
                Some(a) => a,
                None => activation_max,
            }
        }
    };
    raw.clamp(ZERO, activation_max)
}

/// The discrimination law a being quantizes a transduced activation into a discrete perceptual bucket by: a
/// physics-floor family for how finely a being tells two signals apart (Principle 9), the SELECTION and the
/// step its own data (Principle 11). [`DiscriminationLaw::AbsoluteStep`] is the degenerate default:
/// [`discriminate`] under it reproduces a uniform floor quantization bit-for-bit, strictly generalizing an
/// absolute just-noticeable difference. The bucket is the stable key a downstream per-feature belief is
/// minted from, so which signals count as the same perceived kind derives from the being's own sense, never
/// an authored taxonomy.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum DiscriminationLaw {
    /// A uniform absolute step: `bucket = floor(activation / step)`. Equal intervals; the degenerate
    /// default.
    AbsoluteStep,
    /// A Weber-relative step: equal RATIOS, not equal intervals (a magnitude-relative just-noticeable
    /// difference). `bucket = floor(ln(activation) / ln(1 + step))`, so a fixed fractional change spans one
    /// bucket at any magnitude.
    WeberRelative,
}

/// Quantize a transduced activation into a discrete perceptual bucket through a being's own discrimination
/// law. Deterministic. A non-positive step (a misconfiguration) reads bucket zero, the same fail-safe the
/// percept subsystem's uniform bucket uses. `AbsoluteStep` reproduces `floor(activation / step)` bit-for-bit
/// (a strict generalization of the absolute just-noticeable difference).
pub fn discriminate(activation: Fixed, law: DiscriminationLaw, step: Fixed) -> i64 {
    if step.to_bits() <= 0 {
        return 0;
    }
    match law {
        DiscriminationLaw::AbsoluteStep => activation
            .checked_div(step)
            .map(|q| q.to_int() as i64)
            .unwrap_or(0),
        DiscriminationLaw::WeberRelative => {
            if activation <= ZERO {
                return 0;
            }
            let den = (Fixed::ONE + step).ln();
            if den.to_bits() <= 0 {
                return 0;
            }
            activation
                .ln()
                .checked_div(den)
                .map(|q| q.to_int() as i64)
                .unwrap_or(0)
        }
    }
}

/// Split an incident radiant flux at an interface into (reflected, absorbed, transmitted), each a
/// bounded fraction of the incident so no overflow forms; the absorbed is the residual (R+T+A=1),
/// clamped non-negative. The light-field gating of Part 5 and the surface half of perception
/// attenuation.
pub fn interface_split(
    incident: Fixed,
    reflectance: Fixed,
    transmittance: Fixed,
) -> (Fixed, Fixed, Fixed) {
    // The fractions are physical partitions in [0, 1]; clamping keeps a reflected/transmitted term in
    // [0, incident] so the residual subtraction cannot overflow on an out-of-domain negative input.
    let reflectance = reflectance.clamp(ZERO, Fixed::ONE);
    // Reflectance and transmittance share one unit budget (R + T <= 1), so clamp transmittance to the
    // fraction reflectance leaves. Otherwise a physically-impossible R + T > 1 pair would return a
    // triple summing to more than the incident flux, creating energy from nothing.
    let transmittance = transmittance.clamp(ZERO, sat_sub(Fixed::ONE, reflectance));
    let reflected = incident
        .checked_mul(reflectance)
        .unwrap_or(incident)
        .min(incident);
    let transmitted = incident
        .checked_mul(transmittance)
        .unwrap_or(ZERO)
        .min(incident);
    let absorbed = sat_sub(sat_sub(incident, reflected), transmitted).clamp(ZERO, incident);
    (reflected, absorbed, transmitted)
}

/// Optical depth tau = alpha*path (dimensionless), the medium-attenuation half of a stimulus's reach;
/// the transmitted fraction exp(-tau) is transcendental and deferred (report the measured indicator,
/// defer the transform).
pub fn optical_depth(absorption_coefficient: Fixed, path: Fixed, tau_max: Fixed) -> Fixed {
    match absorption_coefficient.checked_mul(path) {
        Some(x) => x.min(tau_max),
        None => tau_max,
    }
}

/// Refractive contrast n2/n1 and whether total internal reflection is possible (n1 > n2), a measured
/// condition and not a verdict. The angle-resolved Snell law needs sin and is deferred.
pub fn refractive_contrast(n1: Fixed, n2: Fixed, contrast_max: Fixed) -> (Fixed, bool) {
    if n1 <= ZERO {
        return (contrast_max, false);
    }
    let contrast = match n2.checked_div(n1) {
        Some(x) => x.min(contrast_max),
        None => contrast_max,
    };
    (contrast, n1 > n2)
}

/// Radiative-equilibrium temperature T_eq = (E_abs/(emissivity*sigma))^(1/4) (K), the inverse of the
/// forward emission law, the term that SETS a surface's temperature from absorbed irradiance. The
/// fourth root is two nested integer square roots, so the unrepresentable T^4 never forms. A
/// non-positive absorbed flux reads zero; a non-emitter never equilibrates and reads the cap.
pub fn radiative_equilibrium(
    absorbed_irradiance: Fixed,
    emissivity: Fixed,
    sigma: Fixed,
    t_max: Fixed,
) -> Fixed {
    if absorbed_irradiance <= ZERO {
        return ZERO;
    }
    // The denominator root is sqrt(emissivity*sigma), formed as sqrt(emissivity)*sqrt(sigma) rather
    // than sqrt of the product: sigma is only about eight fixed-point bits, so emissivity*sigma
    // underflows to zero for a low emissivity and would spuriously return the cap, while each factor
    // roots cleanly first.
    let den_sqrt = match emissivity.sqrt().checked_mul(sigma.sqrt()) {
        Some(x) => x,
        None => return t_max,
    };
    if den_sqrt == ZERO {
        return t_max;
    }
    let num_sqrt = absorbed_irradiance.sqrt();
    let t2 = match num_sqrt.checked_div(den_sqrt) {
        Some(x) => x,
        None => return t_max,
    };
    t2.sqrt().min(t_max)
}

// === Metabolism (R-METABOLIZE): resting metabolic power and the drain bridge ===
//
// The resting-metabolism kernels that free the authored base_metabolic_drain, exertion_drain_coupling,
// and field.body_exchange scalars: the drain a body pays derives from its mass and tissue against the
// physics, not from a per-axis authored number. Every kernel is total, integer, and overflow-capped in
// the house style (checked arithmetic routing an out-of-range product to its physical limit, caps the
// reserved representability bounds passed by the caller). Nothing here reads an identity: two bodies
// diverge from mass, composition, medium, and temperature alone (Principle 9).

/// Basal (resting) metabolic rate P = a * m^(3/4) (W), Kleiber's law over body mass. The 3/4 exponent
/// is an authored universal physics affordance (West, Brown, and Enquist's fractal-network derivation
/// holds across taxa; Principle 9 permits authored physics), evaluated by the EXACT two-square-root
/// fixed-point identity m^(3/4) = sqrt(m * sqrt(m)): `m^(1/2)` then `m * m^(1/2) = m^(3/2)` then its
/// square root, so no exp/ln is touched and the result is bit-identical on every machine (both roots
/// are the exact deterministic integer isqrt). The coefficient `a` is the caller's reserved owner
/// anchor. Zero (or negative) mass has no metabolism and reads zero; an out-of-range product routes to
/// the reserved rate cap.
pub fn basal_metabolic_rate(mass: Fixed, coeff_a: Fixed, rate_max: Fixed) -> Fixed {
    if mass <= ZERO {
        return ZERO;
    }
    // m^(3/4) = sqrt(m * sqrt(m)): the two exact square roots of the identity, no transcendental.
    let root = mass.sqrt(); // m^(1/2)
    let inner = match mass.checked_mul(root) {
        Some(x) => x, // m^(3/2)
        None => return rate_max,
    };
    let m34 = inner.sqrt(); // m^(3/4)
    match coeff_a.checked_mul(m34) {
        Some(p) => p.min(rate_max),
        None => rate_max,
    }
}

/// The resting thermoregulatory heat-loss power (W): the order-independent saturating sum of the Newton
/// convective flux ([`convective_flux`]) and the Stefan-Boltzmann radiant emission ([`radiant_emission`])
/// over the body's exposed surface area, the power a body must replace by metabolism to hold its core
/// temperature against the medium. It reuses the two resolved heat-transport kernels unchanged, reads
/// only the body and medium temperatures, the surface area, and the two surface constants (`h`,
/// emissivity, sigma), and takes no identity, so a hot body in a cold medium and its temperature mirror
/// diverge from temperature alone (Principle 9). Capped at the reserved flux limit; a body at the medium
/// temperature loses nothing (equilibrium).
#[allow(clippy::too_many_arguments)]
pub fn resting_heat_loss(
    h: Fixed,
    area: Fixed,
    body_temp: Fixed,
    medium_temp: Fixed,
    emissivity: Fixed,
    sigma_bits: i64,
    sigma_scale: u32,
    flux_max: Fixed,
) -> Fixed {
    let convective = convective_flux(h, area, body_temp, medium_temp, flux_max);
    // The radiant term takes sigma at its full derived scale (the Tier-2 lift, R-UNITS-PIN slice 4): sigma
    // enters at full precision instead of the Q32.32 truncation, its precision-critical `sigma*(T_hot^4 -
    // T_cold^4)` computed in one wide accumulator and rounded once.
    let radiant = radiant_emission_tier2(
        emissivity,
        area,
        body_temp,
        medium_temp,
        sigma_bits,
        sigma_scale,
        flux_max,
    );
    Fixed::saturating_sum([convective, radiant]).min(flux_max)
}

/// Bridge a resting metabolic power (W) to a fraction of the energy reserve drained per tick. The
/// resting demand is the order-independent saturating sum of the basal rate and the thermoregulatory
/// replacement (`basal + heat_loss`, W); the energy the reserve holds is `energy_capacity *
/// energy_density` (the reserve's energy-storing tissue times its per-unit energy content, J); the
/// fraction is the energy spent this tick over the energy stored, `(power * tick_seconds) / stored`,
/// with the spent energy formed before the divide (a modest per-tick joule figure) so a representable
/// fraction is never pre-saturated. A zero-power demand drains nothing; a zero-energy store (no reserve
/// tissue) drains fully (the cap); an out-of-range spend routes to the cap. Clamped to `[0, frac_max]`.
pub fn metabolic_drain_fraction(
    basal: Fixed,
    heat_loss: Fixed,
    energy_capacity: Fixed,
    energy_density: Fixed,
    tick_seconds: Fixed,
    frac_max: Fixed,
) -> Fixed {
    let power = Fixed::saturating_sum([basal, heat_loss]);
    if power <= ZERO {
        return ZERO;
    }
    let stored = match energy_capacity.checked_mul(energy_density) {
        Some(e) => e,
        // A store so large it overflows is effectively inexhaustible over one tick: negligible fraction.
        None => return ZERO,
    };
    if stored <= ZERO {
        return frac_max;
    }
    let spent = match power.checked_mul(tick_seconds) {
        Some(x) => x, // the joules spent this tick
        None => return frac_max,
    };
    match spent.checked_div(stored) {
        Some(f) => f.clamp(ZERO, frac_max),
        None => frac_max,
    }
}

// === Electricity and magnetism (R-PHYS-W3, wave 3) ===
//
// The reserved constants (the Coulomb coefficient on its x1e9 scale, the vacuum permeability MU_0,
// the tick duration DT, and each coil's turn count and turn density) are the caller's, passed in, not
// fabricated inline. Every zero divisor and every overflow routes to the physical extreme (a
// coincident charge or a short to a cap, an open to a cap). The two induction laws are the only place
// the substrate takes a time derivative: a first-order finite difference over a resident state axis's
// prior-tick sample, deterministic and tick-rate-dependent.

/// Coulomb force F = k*|q1|*|q2|/r^2 (N), with the attractive/repulsive condition tracked separately
/// (like signs repel). Coincident charges route to the cap; a distant pair is negligible. The
/// Coulomb coefficient is passed on its reserved x1e9 output scale.
pub fn coulomb_force(
    q1: Fixed,
    q2: Fixed,
    r: Fixed,
    k_coulomb: Fixed,
    f_max: Fixed,
) -> (Fixed, bool) {
    let repulsive = (q1 > ZERO) == (q2 > ZERO);
    if r <= ZERO {
        return (f_max, repulsive);
    }
    // F = k*|q1|*|q2|/r^2, evaluated in i128 raw bits so the only cap is on the true force. Each
    // charge magnitude is reduced by the separation before the product (a = |q1|/r, b = |q2|/r,
    // base = a*b), keeping every intermediate inside i128 across the declared ranges; because the
    // reduction happens in i128 rather than a Fixed, no representable in-range force routes to the
    // ceiling regardless of where the (reserved) charge scale is later set. A genuinely huge force
    // (charges so large or separations so small that the i128 product overflows, or a result at or
    // above the reserved ceiling) still routes to f_max. Inputs are Fixed, so every raw magnitude is
    // bounded by i64::MAX and the `<<32` never overflows i128; the checked multiplies catch the rest.
    let q1b = sat_abs(q1).to_bits() as i128;
    let q2b = sat_abs(q2).to_bits() as i128;
    let rb = r.to_bits() as i128; // > 0 (guarded)
    let kb = k_coulomb.to_bits() as i128;
    let fmb = f_max.to_bits() as i128;
    let a = (q1b << 32) / rb; // |q1|/r as a Fixed raw, full precision
    let b = (q2b << 32) / rb; // |q2|/r as a Fixed raw
    let base = match a.checked_mul(b) {
        Some(x) => x >> 32, // |q1||q2|/r^2 as a Fixed raw
        None => return (f_max, repulsive),
    };
    let force = match base.checked_mul(kb) {
        Some(x) => x >> 32,
        None => return (f_max, repulsive),
    };
    if force >= fmb {
        (f_max, repulsive)
    } else {
        (Fixed::from_bits(force as i64), repulsive)
    }
}

/// Ohm's law V = I*R (V), reported as a non-negative magnitude over [0, V_MAX] (the resistance is a
/// magnitude, so the current's sign carries no meaning here), which also keeps the overflow cap
/// sign-correct.
pub fn ohm_voltage(current: Fixed, resistance: Fixed, v_max: Fixed) -> Fixed {
    match sat_abs(current).checked_mul(resistance) {
        Some(v) => v.min(v_max),
        None => v_max,
    }
}

/// Circuit current I = emf / r_total (A), a magnitude; a zero total resistance is a short (the cap).
/// The caller forms r_total as the order-independent `saturating_sum` of the series resistances.
pub fn circuit_current(emf: Fixed, r_total: Fixed, i_max: Fixed) -> Fixed {
    if r_total <= ZERO {
        return i_max;
    }
    match sat_abs(emf).checked_div(r_total) {
        Some(i) => i.min(i_max),
        None => i_max,
    }
}

/// Joule power P = I*V (W), the dissipated power (which feeds `law.sensible_heat`, so a wire heats).
pub fn power_dissipation(current: Fixed, voltage: Fixed, power_max: Fixed) -> Fixed {
    match sat_abs(current).checked_mul(sat_abs(voltage)) {
        Some(p) => p.min(power_max),
        None => power_max,
    }
}

/// Capacitor stored energy U = (1/2) C V^2 (J). The capacitance is halved first and each product is
/// a checked multiply, so no raw V^2 forms and an overflow (a large C at a large V) routes to the cap;
/// there is no voltage-only guard, which would spuriously cap a small capacitor at a high voltage.
pub fn capacitor_energy(capacitance: Fixed, voltage: Fixed, e_max: Fixed) -> Fixed {
    let half_c = match capacitance.checked_mul(HALF) {
        Some(x) => x,
        None => return e_max,
    };
    let t = match half_c.checked_mul(voltage) {
        Some(x) => x,
        None => return e_max,
    };
    match t.checked_mul(voltage) {
        Some(u) => u.min(e_max),
        None => e_max,
    }
}

/// Galvanic cell EMF = E_cathode - E_anode (V), signed, from the volt-promoted electrode potentials;
/// the unification law that closes the loop the wave-2 corrosion driving margin opened as a proxy.
pub fn battery_emf(cathode: Fixed, anode: Fixed) -> Fixed {
    sat_sub(cathode, anode)
}

/// The standard cell potential at the cell TEMPERATURE (V): `E0(T) = E0_ref + (dE0/dT) * (T - T_ref)`, the
/// linear temperature coefficient of the couple's standard potential (its reaction-entropy term), so a redox
/// couple's standard drive shifts with the cell temperature rather than being frozen at the reference. `dE0/dT`
/// is per-couple data (its own reserved axis); at `dE0/dT = 0` this is the reference potential unchanged. The
/// caller passes the result as the `standard_emf` of [`nernst_emf`]. Deterministic fixed-point.
pub fn standard_potential_at_temperature(
    e0_ref: Fixed,
    de0_dt: Fixed,
    temperature: Fixed,
    t_ref: Fixed,
) -> Fixed {
    let dt = sat_sub(temperature, t_ref);
    e0_ref.saturating_add(de0_dt.checked_mul(dt).unwrap_or(ZERO))
}

/// The NERNST-adjusted galvanic EMF (V): the (temperature-adjusted) standard cell EMF corrected for the
/// couple's ACTUAL activities, `E = E_standard + (k_B*T/q) * (ln a_donor + ln a_acceptor)`, so a redox
/// source's drive FALLS as its donor and acceptor deplete and crosses zero at (and would reverse below) the
/// couple's OWN equilibrium, rather than reading spontaneity forever at the standard state (the
/// concentration-independent defect of the bare `battery_emf`). The thermal factor is the PER-PARTICLE form
/// `k_B*T/q` from the Boltzmann constant `boltzmann_k` (a floor fundamental), the cell temperature `T`, and
/// the couple's CARRIER CHARGE `q = n*e` (its own per-couple datum, sibling of `chem.electron_count`), NOT the
/// molar `RT/(nF)`, so no molar gas constant or Faraday constant enters and the `R = N_A*k_B` / `F = N_A*e`
/// composite drift is avoided. `standard_emf` is `battery_emf(acceptor, donor)` after
/// [`standard_potential_at_temperature`]; the activities are the gamma-adjusted concentrations relative to the
/// standard state (unit activity is the standard state), formed by the caller from its activity-coefficient
/// data. At unit activities (`ln 1 = 0`) this reduces exactly to the standard EMF. A depleted species
/// (non-positive activity) has no real log and no free energy: the drive collapses to zero (no reactant, no
/// yield, no flux). Deterministic fixed-point (`Fixed::ln`, integer-only and pinned, so the redox yield
/// replays bit-identically and is worker-invariant).
pub fn nernst_emf(
    standard_emf: Fixed,
    donor_activity: Fixed,
    acceptor_activity: Fixed,
    boltzmann_k: Fixed,
    temperature: Fixed,
    carrier_charge: Fixed,
) -> Fixed {
    if carrier_charge <= ZERO || boltzmann_k <= ZERO || temperature <= ZERO {
        return standard_emf; // no charge carrier or no thermal scale: no concentration adjustment
    }
    let kt = match boltzmann_k.checked_mul(temperature) {
        Some(x) => x,
        None => return standard_emf,
    };
    let kt_over_q = match kt.checked_div(carrier_charge) {
        Some(x) => x,
        None => return standard_emf,
    };
    // A depleted species (non-positive activity) has no reactant and so no drive: the couple's EMF collapses
    // to the equilibrium boundary, which the flux and the zero-clamped yield read as no life. (`-ln(activity)`
    // diverges as the stock vanishes; the zero boundary stands in without an authored magnitude in unknown
    // units.)
    if donor_activity <= ZERO || acceptor_activity <= ZERO {
        return ZERO;
    }
    // E = E_standard + (k_B*T/q) * (ln a_donor + ln a_acceptor).
    let ln_sum = donor_activity.ln().saturating_add(acceptor_activity.ln());
    let adj = kt_over_q.checked_mul(ln_sum).unwrap_or_else(|| {
        if ln_sum < ZERO {
            sat_sub(ZERO, kt_over_q)
        } else {
            kt_over_q
        }
    });
    standard_emf.saturating_add(adj)
}

/// The reversible MICHAELIS-MENTEN uptake flux (per tick, in the source's stock units): the substrate-
/// saturating Hill term times the reversible thermodynamic drive,
/// `v = Vmax * (S^h / (Km^h + S^h)) * drive`, with `drive = 1 - exp(-q E / (k_B*T))` the free-energy factor of
/// the couple's EMF `E` (`dG = -qE` per reaction event, `q = n*e` the carrier charge): forward (`drive` toward
/// one) when the reaction releases free energy (`E > 0`), zero at equilibrium (`E = 0`), and negative below
/// it, so a source powers NO life below its own (Nernst-shifted) equilibrium. The STRUCTURAL conservation
/// clamp `min(v, S)` is applied here (a draw never exceeds the present stock, `v <= S` is not free), and the
/// flux is floored at zero (no reverse uptake). The Hill exponent `h` is the cooperativity (`h = 1` the plain
/// Monod `S/(Km+S)`); `Km` the half-saturation stock is per-source-class kinetics data; and `Vmax` is the
/// maximum specific uptake the CALLER derives from the being's own catalyst tissue (`Vmax = kcat * catalyst`,
/// the emergent-throughput architecture, no authored efficiency scalar), passed in here. The thermal factor
/// uses the same per-particle `k_B*T/q` as [`nernst_emf`]. Deterministic fixed-point (`Fixed::powf`/`exp`,
/// integer-only and pinned).
#[allow(clippy::too_many_arguments)]
pub fn reversible_uptake_flux(
    stock: Fixed,
    vmax: Fixed,
    km: Fixed,
    hill: Fixed,
    emf: Fixed,
    boltzmann_k: Fixed,
    temperature: Fixed,
    carrier_charge: Fixed,
) -> Fixed {
    if stock <= ZERO || vmax <= ZERO {
        return ZERO;
    }
    // The Hill-saturating substrate term S^h / (Km^h + S^h), in [0, 1). A zero stock is zero (no draw).
    let sh = stock.powf(hill);
    let kmh = km.powf(hill);
    let denom = kmh.saturating_add(sh);
    let saturation = if denom > ZERO {
        sh.checked_div(denom).unwrap_or(ZERO)
    } else {
        ZERO
    };
    // The reversible thermodynamic drive 1 - exp(-q E / (k_B*T)): one far forward, zero at equilibrium,
    // negative below it (floored to zero by the clamp: no life below the couple's own equilibrium).
    let kt = boltzmann_k.checked_mul(temperature);
    let drive = match kt {
        Some(kt) if kt > ZERO && carrier_charge > ZERO => {
            let scaled = carrier_charge
                .checked_mul(emf)
                .and_then(|qe| qe.checked_div(kt));
            match scaled {
                Some(s) => ONE - (ZERO - s).exp(),
                // An overflowing exponent means an enormous forward drive: saturate to one.
                None => ONE,
            }
        }
        // No thermal scale given: forward at full when the standard drive is spontaneous.
        _ => {
            if emf > ZERO {
                ONE
            } else {
                ZERO
            }
        }
    };
    let raw = vmax
        .checked_mul(saturation)
        .unwrap_or(vmax)
        .checked_mul(drive)
        .unwrap_or(ZERO);
    // min(v, S) conservation clamp plus the no-reverse-uptake floor.
    raw.clamp(ZERO, stock)
}

/// Element resistance R = rho*L/A (Ohm), the measured geometric consequence of the material and shape;
/// a vanishing cross-section is an open (the cap).
pub fn resistance(resistivity: Fixed, length: Fixed, area: Fixed, r_max: Fixed) -> Fixed {
    if area <= ZERO {
        return r_max;
    }
    // Divide the length by the area before the resistivity multiply (reduce before grow), so an
    // in-range resistance whose rho*length would overflow the ceiling is computed rather than capped.
    let geometry = match length.checked_div(area) {
        Some(x) => x,
        None => return r_max,
    };
    match resistivity.checked_mul(geometry) {
        Some(r) => r.min(r_max),
        None => r_max,
    }
}

/// Solenoid field B = mu_0 * mu_r * n * I (T), with mu_0 applied early so the large relative
/// permeability does not overflow. The nonlinear B-H saturation loop is deferred.
pub fn solenoid_field(
    permeability: Fixed,
    current: Fixed,
    turn_density: Fixed,
    mu_0: Fixed,
    b_max: Fixed,
) -> Fixed {
    // The flux-density axis is a non-negative magnitude, and the other factors are non-negative, so
    // take the current's magnitude: the field strength does not carry the current's sign here, and
    // the overflow cap stays sign-correct.
    let ni = match turn_density.checked_mul(sat_abs(current)) {
        Some(x) => x,
        None => return b_max,
    };
    let b0 = match ni.checked_mul(mu_0) {
        Some(x) => x,
        None => return b_max,
    };
    match b0.checked_mul(permeability) {
        Some(b) => b.min(b_max),
        None => b_max,
    }
}

/// Flux linkage Phi = B*A (Wb), the resident magnetic-flux state `law.faraday_emf` differentiates.
/// Flux is a non-negative magnitude over [0, PHI_MAX], consistent with `solenoid_field`'s magnitude
/// flux density and the floor's interval bound; the Lenz-law sign `faraday_emf` recovers comes from
/// the signed tick-to-tick difference of two non-negative flux samples, not from the flux itself. A
/// non-negative product bounds cleanly and an overflow is a large flux, so it routes to the cap.
pub fn flux_linkage(flux_density: Fixed, area: Fixed, phi_max: Fixed) -> Fixed {
    match flux_density.checked_mul(area) {
        Some(p) => p.clamp(ZERO, phi_max),
        None => phi_max,
    }
}

/// Force on a current-carrying conductor F = B*I*L (N), the motor, relay, and telegraph-sounder force.
pub fn motor_force(flux_density: Fixed, current: Fixed, length: Fixed, f_max: Fixed) -> Fixed {
    let bi = match flux_density.checked_mul(current) {
        Some(x) => x,
        None => return f_max,
    };
    match bi.checked_mul(length) {
        Some(f) => sat_abs(f).min(f_max),
        None => f_max,
    }
}

/// Lorentz force on a moving charge F = |q|*v*B (N).
pub fn lorentz_force(charge: Fixed, velocity: Fixed, flux_density: Fixed, f_max: Fixed) -> Fixed {
    let qv = match sat_abs(charge).checked_mul(sat_abs(velocity)) {
        Some(x) => x,
        None => return f_max,
    };
    match qv.checked_mul(sat_abs(flux_density)) {
        Some(f) => f.min(f_max),
        None => f_max,
    }
}

/// Magnetic dipole maximum torque tau = m*B (N*m); the sin(theta) angular factor is deferred, so this
/// is the perpendicular-orientation envelope (the compass, galvanometer, and motor torque).
pub fn dipole_torque(moment: Fixed, flux_density: Fixed, torque_max: Fixed) -> Fixed {
    match sat_abs(moment).checked_mul(sat_abs(flux_density)) {
        Some(t) => t.min(torque_max),
        None => torque_max,
    }
}

/// Faraday induced EMF = -N * dPhi/DT (V), signed by Lenz's law, the per-tick flux delta. The caller
/// threads the prior-tick flux (canonical state) and the fixed tick duration DT.
pub fn faraday_emf(
    flux_now: Fixed,
    flux_prev: Fixed,
    turns: Fixed,
    dt: Fixed,
    v_max: Fixed,
) -> Fixed {
    if dt <= ZERO {
        return ZERO;
    }
    let dphi = sat_sub(flux_now, flux_prev);
    let rate = dphi
        .checked_div(dt)
        .unwrap_or(if dphi < ZERO { Fixed::MIN } else { Fixed::MAX });
    let prod = rate
        .checked_mul(turns)
        .unwrap_or(if rate < ZERO { Fixed::MIN } else { Fixed::MAX });
    // Lenz: the EMF opposes the change, so negate the flux-rate term.
    sat_sub(ZERO, prod).clamp(ZERO - v_max, v_max)
}

/// Inductive EMF = -L * dI/DT (V), signed; the self back-EMF, or the mutual step-up with
/// M = k*sqrt(L1*L2) formed by the caller. The transformer and choke, and the closing half of the
/// R-COMMS inductance gap.
pub fn inductive_emf(
    inductance: Fixed,
    current_now: Fixed,
    current_prev: Fixed,
    dt: Fixed,
    v_max: Fixed,
) -> Fixed {
    if dt <= ZERO {
        return ZERO;
    }
    let di = sat_sub(current_now, current_prev);
    let rate = di
        .checked_div(dt)
        .unwrap_or(if di < ZERO { Fixed::MIN } else { Fixed::MAX });
    let prod =
        rate.checked_mul(inductance)
            .unwrap_or(if rate < ZERO { Fixed::MIN } else { Fixed::MAX });
    sat_sub(ZERO, prod).clamp(ZERO - v_max, v_max)
}

/// Inductor stored energy U = (1/2) L I^2 (J), the magnetic dual of the capacitor energy. The
/// inductance is halved first and each product is a checked multiply, so an overflow routes to the
/// cap (no raw I^2, and no current-only guard that would spuriously cap a small inductor at a high
/// current).
pub fn inductor_energy(inductance: Fixed, current: Fixed, e_max: Fixed) -> Fixed {
    let half_l = match inductance.checked_mul(HALF) {
        Some(x) => x,
        None => return e_max,
    };
    let t = match half_l.checked_mul(current) {
        Some(x) => x,
        None => return e_max,
    };
    match t.checked_mul(current) {
        Some(u) => u.min(e_max),
        None => e_max,
    }
}

// === Language processing cost (R-LANG-TYPOLOGY, the word-order harmony floor) ===
//
// The two direction-NEUTRAL kernels the sim-side word-order harmony tilt derives from
// (crates/sim/src/typology.rs owns the branching-consistency mapping that turns a grammar into an
// extent). Both are LABEL-BLIND and DIRECTION-BLIND: they see only a scalar domain extent and a
// scalar cost reduction, never a word-order value, so they cannot privilege one linear order over
// its mirror and they author no attractor (Principle 9). What they reward is CONSISTENCY (a shorter
// dependency-integration domain costs less to hold), never a specific direction. Each is a pure
// closed-form Fixed function, saturation-capped in the house idiom, and total on adversarial input.

/// The dependency-integration parse cost of holding a linearization domain in working memory
/// (Hawkins 1983/2004's processing account of the branching-direction anchor; Gibson 1998 dependency
/// locality): a monotone-increasing, saturating function of how much material a head must hold before
/// it is integrated (`domain_extent`), SOFTENED by the parser's working-memory capacity. The
/// integer-Hill saturating form `extent / (extent + memory)` (the same dose-response shape
/// [`harm_class`] uses) scaled by the reserved `cost_max`: a zero extent is zero cost, an unbounded
/// extent saturates at `cost_max`, and at `extent == memory` the cost is half of `cost_max`. A larger
/// memory capacity shifts the half-cost point outward, so the same domain costs a higher-capacity
/// parser less (the per-race softening). Direction-blind: `domain_extent` is a magnitude, never a
/// word-order value.
pub fn parse_cost(domain_extent: Fixed, memory_capacity: Fixed, cost_max: Fixed) -> Fixed {
    if domain_extent <= ZERO {
        return ZERO;
    }
    // den = extent + memory (both taken non-negative), saturating: a saturated sum is a huge
    // denominator, handled by the divide (frac routes toward the extent/extent = one limit).
    let den = domain_extent.saturating_add(memory_capacity.max(ZERO));
    if den <= ZERO {
        // den >= extent > 0 always holds, so this is unreachable; guard so the divide is total and a
        // degenerate denominator routes to the full cost rather than a wrap.
        return cost_max.max(ZERO);
    }
    // frac = extent / den in (0, 1]; against a fixed memory, frac -> one as the extent grows.
    let frac = match domain_extent.checked_div(den) {
        Some(f) => f,
        None => return cost_max.max(ZERO),
    };
    // Scale the [0, 1] cost fraction by the reserved ceiling, capped rather than wrapped.
    match cost_max.checked_mul(frac) {
        Some(c) => c.clamp(ZERO, cost_max.max(ZERO)),
        None => cost_max.max(ZERO),
    }
}

/// The multiplicative harmony tilt a cost reduction earns: `exp(cost_reduction / temperature)`, the
/// softmax weight of the lower-cost (consistent) option relative to the baseline, floored at one and
/// saturating at `tilt_max`. `cost_reduction` is the parse cost a consistent choice AVOIDS (a
/// [`parse_cost`] output), and `temperature` is the softmax scale: a small temperature makes the tilt
/// bite hard, a large one flattens it toward one. A zero (or negative) reduction earns no tilt (the
/// weight floors at one, so the law never pushes a weight below its prior), and the deterministic
/// zero-temperature limit saturates at `tilt_max`. The exponential is the canon-pinned deterministic
/// [`Fixed::exp`] (R-GPU-CANON-PIN), integer-only and bit-identical on every backend; for a large
/// argument it saturates, and the clamp routes that to `tilt_max`. Direction-blind: the argument is a
/// scalar cost, never a word-order value.
pub fn harmony_tilt(cost_reduction: Fixed, temperature: Fixed, tilt_max: Fixed) -> Fixed {
    if cost_reduction <= ZERO {
        return ONE;
    }
    if temperature <= ZERO {
        // exp(reduction / 0+) -> infinity: the hard-max (deterministic) limit saturates at the cap.
        return tilt_max.max(ONE);
    }
    let z = match cost_reduction.checked_div(temperature) {
        Some(z) => z,
        // A reduction-over-temperature past the representable range is the same hard-max limit.
        None => return tilt_max.max(ONE),
    };
    // exp(z) with z >= 0 is >= 1 (and saturates to Fixed::MAX for a large z); clamp to a bounded
    // boost in [ONE, tilt_max] so the tilt never wraps and never falls below one.
    z.exp().clamp(ONE, tilt_max.max(ONE))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Dev fixtures: representable caps for the determinism harness, never canon. The
    // owner's set caps reach a kernel through the calibration manifest when the engine
    // wires it; these only have to be below the Q32.32 ceiling for the harness to run.
    const HARM_CAP: Fixed = Fixed::ONE;
    const F_INT: fn(i32) -> Fixed = Fixed::from_int;

    fn cap(v: i32) -> Fixed {
        Fixed::from_int(v)
    }

    // --- Reach: geometric spreading ---

    #[test]
    fn geometric_spread_reproduces_inverse_square_at_dimension_three() {
        // The exact 4*pi is immaterial to the identity (both kernels receive the same coefficient);
        // a realistic value keeps the fixture honest. 62832/5000 = 12.5664, four pi to four places.
        let four_pi = Fixed::from_ratio(62_832, 5_000);
        let irrad_max = cap(1_000_000);
        for &p in &[
            Fixed::from_int(1),
            Fixed::from_int(100),
            Fixed::from_ratio(1, 2),
        ] {
            for &r in &[Fixed::from_int(1), Fixed::from_int(5), Fixed::from_int(37)] {
                assert_eq!(
                    geometric_spread(p, r, 3, four_pi, irrad_max),
                    inverse_square_falloff(p, r, four_pi, irrad_max),
                    "geometric_spread at D=3 must be byte-identical to inverse_square_falloff",
                );
            }
        }
    }

    #[test]
    fn geometric_spread_is_one_over_r_in_2d_and_flat_in_1d() {
        let two_pi = Fixed::from_ratio(31_416, 5_000);
        let irrad_max = cap(1_000_000);
        let p = Fixed::from_int(100);
        let r = Fixed::from_int(4);
        // 2D surface: E = P / (2*pi*r), the 1/r geometric spreading.
        let denom_2d = two_pi.checked_mul(r).unwrap();
        assert_eq!(
            geometric_spread(p, r, 2, two_pi, irrad_max),
            p.checked_div(denom_2d).unwrap().min(irrad_max),
        );
        // 1D duct: the exponent is zero, so there is no radial spreading; the value does not fall off
        // with distance.
        let line_coeff = Fixed::from_int(2);
        let near = geometric_spread(p, r, 1, line_coeff, irrad_max);
        let far = geometric_spread(p, Fixed::from_int(400), 1, line_coeff, irrad_max);
        assert_eq!(near, p.checked_div(line_coeff).unwrap().min(irrad_max));
        assert_eq!(near, far, "a 1D duct does not attenuate with distance");
    }

    #[test]
    fn geometric_spread_caps_at_zero_distance_and_vanishes_when_distant() {
        let four_pi = Fixed::from_ratio(62_832, 5_000);
        let irrad_max = cap(1_000_000);
        let p = Fixed::from_int(100);
        // Zero distance: the denominator is zero, so the read is the cap.
        assert_eq!(
            geometric_spread(p, Fixed::ZERO, 3, four_pi, irrad_max),
            irrad_max,
        );
        // A source far enough that distance^2 overflows the representable product is negligible (zero),
        // the same overflow-to-zero behaviour as inverse_square_falloff.
        assert_eq!(
            geometric_spread(p, Fixed::from_int(100_000), 3, four_pi, irrad_max),
            Fixed::ZERO,
        );
    }

    // --- Perception: the transduction response family and the discrimination family ---

    #[test]
    fn transduce_linear_default_reproduces_magnitude_times_gain() {
        // The degenerate default is a strict generalization: Linear reproduces `magnitude * gain`
        // bit-for-bit in the non-overflow regime (the shape parameter is ignored), so wiring a plain
        // linear sensitivity through the family changes no bit.
        let cap = cap(1_000_000);
        let shape_ignored = Fixed::from_int(3);
        for &m in &[
            Fixed::from_int(1),
            Fixed::from_int(50),
            Fixed::from_ratio(3, 2),
        ] {
            for &g in &[
                Fixed::from_int(1),
                Fixed::from_int(4),
                Fixed::from_ratio(1, 2),
            ] {
                assert_eq!(
                    transduce(m, ResponseLaw::Linear, g, shape_ignored, cap),
                    m.mul(g).min(cap),
                    "Linear transduction must be byte-identical to magnitude * gain",
                );
            }
        }
        // The clamp bites at the activation ceiling.
        assert_eq!(
            transduce(
                Fixed::from_int(10),
                ResponseLaw::Linear,
                Fixed::from_int(10),
                shape_ignored,
                Fixed::from_int(50)
            ),
            Fixed::from_int(50),
            "the activation is clamped to activation_max",
        );
    }

    #[test]
    fn discriminate_absolute_step_reproduces_the_uniform_bucket() {
        // AbsoluteStep reproduces `floor(activation / step)` bit-for-bit, the same formula (and the same
        // non-positive-step fail-safe) the percept subsystem's feature_bucket uses.
        let step = Fixed::from_ratio(1, 4);
        for &v in &[
            Fixed::ZERO,
            Fixed::from_ratio(1, 8),
            Fixed::from_int(1),
            Fixed::from_ratio(9, 4),
        ] {
            let expected = v.checked_div(step).map(|q| q.to_int() as i64).unwrap_or(0);
            assert_eq!(
                discriminate(v, DiscriminationLaw::AbsoluteStep, step),
                expected
            );
        }
        // A non-positive step reads bucket zero (the misconfiguration fail-safe).
        assert_eq!(
            discriminate(
                Fixed::from_int(5),
                DiscriminationLaw::AbsoluteStep,
                Fixed::ZERO
            ),
            0
        );
    }

    #[test]
    fn transduce_all_laws_are_monotone_and_zero_at_zero() {
        let cap = cap(1_000_000);
        let gain = Fixed::from_int(2);
        // Every law reads zero at zero magnitude (no percept from no signal).
        for law in [
            ResponseLaw::Linear,
            ResponseLaw::Power,
            ResponseLaw::LogCompressive,
        ] {
            assert_eq!(
                transduce(Fixed::ZERO, law, gain, Fixed::from_ratio(1, 2), cap),
                Fixed::ZERO
            );
        }
        // Every law is monotone increasing in the magnitude.
        for law in [
            ResponseLaw::Linear,
            ResponseLaw::Power,
            ResponseLaw::LogCompressive,
        ] {
            let a = transduce(Fixed::from_int(2), law, gain, Fixed::from_ratio(1, 2), cap);
            let b = transduce(Fixed::from_int(8), law, gain, Fixed::from_ratio(1, 2), cap);
            assert!(
                b > a,
                "transduction is monotone increasing in the magnitude"
            );
        }
    }

    #[test]
    fn transduce_power_and_log_compress_a_wide_range() {
        // A compressive law (Stevens power with shape < 1, or Fechner log) grows sub-linearly: doubling
        // the input less than doubles the activation, unlike the linear default.
        let cap = cap(1_000_000);
        let gain = Fixed::ONE;
        let m = Fixed::from_int(16);
        for law in [ResponseLaw::Power, ResponseLaw::LogCompressive] {
            let shape = Fixed::from_ratio(1, 2);
            let at_m = transduce(m, law, gain, shape, cap);
            let at_2m = transduce(m.mul(Fixed::from_int(2)), law, gain, shape, cap);
            assert!(
                at_2m < at_m.mul(Fixed::from_int(2)),
                "a compressive law grows sub-linearly (doubling input less than doubles activation)",
            );
        }
        // The linear default does NOT compress: doubling the input doubles the activation.
        let lin_m = transduce(m, ResponseLaw::Linear, gain, Fixed::ONE, cap);
        let lin_2m = transduce(
            m.mul(Fixed::from_int(2)),
            ResponseLaw::Linear,
            gain,
            Fixed::ONE,
            cap,
        );
        assert_eq!(lin_2m, lin_m.mul(Fixed::from_int(2)));
    }

    #[test]
    fn discriminate_weber_bucket_step_is_bounded_across_magnitude_unlike_absolute() {
        // Weber-relative quantizes on equal RATIOS, so a doubling advances the bucket by a near-constant
        // (bounded) amount at any magnitude (the continuous ratio ln(2)/ln(1+step) is constant; flooring
        // leaves it constant within one bucket). The absolute step instead advances by a GROWING amount at
        // high magnitude. That contrast is the Weber property.
        let step = Fixed::from_ratio(1, 2);
        let weber =
            |v: i32| discriminate(Fixed::from_int(v), DiscriminationLaw::WeberRelative, step);
        let abs = |v: i32| discriminate(Fixed::from_int(v), DiscriminationLaw::AbsoluteStep, step);
        let w_low = weber(8) - weber(4);
        let w_high = weber(256) - weber(128);
        assert!(
            (w_low - w_high).abs() <= 1,
            "the Weber increment per doubling stays near-constant across magnitude (low {w_low}, high {w_high})",
        );
        let a_low = abs(8) - abs(4);
        let a_high = abs(256) - abs(128);
        assert!(
            a_high > a_low,
            "the absolute-step increment per doubling grows with magnitude (low {a_low}, high {a_high})",
        );
        // A non-positive activation reads bucket zero.
        assert_eq!(
            discriminate(Fixed::ZERO, DiscriminationLaw::WeberRelative, step),
            0
        );
    }

    // --- Biology ---

    #[test]
    fn net_nutrition_is_the_limiting_nutrient_and_order_independent() {
        let half = Fixed::from_ratio(1, 2);
        let a = (Fixed::ONE, Fixed::ONE, Some(Fixed::ONE)); // fully satisfied
        let b = (half, Fixed::ONE, Some(Fixed::ONE)); // half satisfied (the limiter)
        let c = (Fixed::ONE, Fixed::ONE, None); // not required, contributes one
        let forward = net_nutrition(&[a, b, c]);
        let reversed = net_nutrition(&[c, b, a]);
        assert_eq!(forward, half, "the minimum is the limiting nutrient");
        assert_eq!(forward, reversed, "the min-fold is order-independent");
    }

    #[test]
    fn abundant_supply_saturates_rather_than_wrapping_a_false_zero() {
        // A one-bit requirement against full supply must read fully satisfied, not the
        // wrapped zero the wave-0 NEW-DET-3 attack produced.
        let tiny = Fixed::from_bits(1);
        assert_eq!(satisfaction(Fixed::ONE, Fixed::ONE, Some(tiny)), Fixed::ONE);
    }

    #[test]
    fn harm_routes_an_out_of_range_ratio_to_the_cap_not_garbage() {
        // dose 38000 against tolerance 1e-6 is a ratio of ~3.8e10, far past the
        // representable ceiling; it must route to the max-harm cap (the NEW-DET-2 fix),
        // not wrap r to a small or negative value.
        let dose = Fixed::from_int(38000);
        let tol = Fixed::from_decimal_str("0.000001").unwrap();
        assert_eq!(harm_class(dose, Some(tol), 3, HARM_CAP), HARM_CAP);
    }

    #[test]
    fn harm_cube_boundary_computes_below_the_overflow_and_caps_above() {
        // At r = 1290 the cube fits and harm is computed; at r = 1291 the guard fires
        // and harm is the cap, never a wrapped value.
        let at = harm_class(Fixed::from_int(1290), Some(Fixed::ONE), 3, HARM_CAP);
        let over = harm_class(Fixed::from_int(1291), Some(Fixed::ONE), 3, HARM_CAP);
        assert!(at < HARM_CAP, "r=1290 computes a sub-cap harm");
        assert_eq!(
            over, HARM_CAP,
            "r=1291 is guarded to the cap before the cube"
        );
    }

    #[test]
    fn the_same_food_is_poison_to_one_consumer_and_safe_to_another() {
        // Race-blindness: one dose, two tolerances, the harm differs purely by the
        // consumer datum, and swapping the consumers swaps the outcomes.
        let dose = Fixed::from_int(10);
        let fragile = Some(Fixed::ONE); // low tolerance: high harm
        let hardy = Some(Fixed::from_int(1000)); // high tolerance: low harm
        let h_fragile = harm_class(dose, fragile, 2, HARM_CAP);
        let h_hardy = harm_class(dose, hardy, 2, HARM_CAP);
        assert!(h_fragile > h_hardy, "the fragile consumer takes more harm");
        // Swapping the two consumer vectors swaps the outputs exactly.
        assert_eq!(harm_class(dose, hardy, 2, HARM_CAP), h_hardy);
        assert_eq!(harm_class(dose, fragile, 2, HARM_CAP), h_fragile);
    }

    #[test]
    fn not_applicable_tolerance_skips_but_zero_tolerance_is_maximally_harmful() {
        assert_eq!(harm_class(Fixed::from_int(5), None, 2, HARM_CAP), ZERO);
        assert_eq!(
            harm_class(Fixed::from_int(5), Some(ZERO), 2, HARM_CAP),
            HARM_CAP
        );
    }

    #[test]
    fn net_harm_sum_is_order_independent() {
        let classes = [
            (Fixed::from_int(2), Some(Fixed::ONE), 2u8),
            (Fixed::from_int(3), Some(Fixed::from_int(2)), 1u8),
            (Fixed::from_int(1), None, 3u8),
        ];
        let mut reversed = classes;
        reversed.reverse();
        assert_eq!(
            net_harm(&classes, HARM_CAP, cap(10)),
            net_harm(&reversed, HARM_CAP, cap(10))
        );
    }

    #[test]
    fn edibility_margin_saturates_on_a_near_clean_meal() {
        // A tiny nonzero aggregate dose against an appreciable tolerance must saturate to
        // the margin cap, not wrap to a small or inverted reading (the NEW-DET-5 fix).
        let tol = Fixed::from_int(5000);
        let dose = Fixed::from_decimal_str("0.000001").unwrap();
        let e = edibility(Fixed::ONE, ZERO, tol, dose, cap(1_000_000));
        assert_eq!(e.margin, cap(1_000_000));
        // A zero dose also reads the cap.
        assert_eq!(
            edibility(Fixed::ONE, ZERO, tol, ZERO, cap(1_000_000)).margin,
            cap(1_000_000)
        );
    }

    // --- Mechanics ---

    #[test]
    fn cut_penetrate_computes_a_real_cut_where_the_product_form_overflowed() {
        // The wave-1 MECH-RT2-01 adversarial case: contact area 0.5, specific cutting
        // energy 5000, delivered energy 1e6. The product u*A*C_VOL = 2.5e9 overflows,
        // but the staged division yields a representable 4e-4 depth, not zero.
        let depth = cut_penetrate(
            Fixed::from_int(200), // pressure above hardness
            Fixed::from_int(100), // hardness
            Fixed::from_int(1_000_000),
            Fixed::from_int(5000),
            Fixed::from_ratio(1, 2),
            cap(10),
        );
        assert_eq!(depth, Fixed::from_decimal_str("0.0004").unwrap());
        assert!(depth > ZERO, "a representable cut is not reported as zero");
    }

    #[test]
    fn cut_penetrate_onset_guard_gives_no_cut_below_hardness() {
        let depth = cut_penetrate(
            Fixed::from_int(50),
            Fixed::from_int(100),
            Fixed::from_int(1_000_000),
            Fixed::from_int(5000),
            Fixed::from_ratio(1, 2),
            cap(10),
        );
        assert_eq!(depth, ZERO);
    }

    #[test]
    fn mace_versus_morningstar_is_one_design_resolved_by_contact_geometry() {
        // Same blow, same materials, different contact area. The blunt mace head spreads
        // the force (large area, low pressure); the morningstar spike concentrates it
        // (small area, high pressure). One law set resolves both.
        let force = Fixed::from_int(2000);
        let mace_area = Fixed::from_ratio(1, 100); // a broad head, 0.01 m^2
        let spike_area = Fixed::from_decimal_str("0.000001").unwrap(); // a fine tip, 1e-6 m^2
        let p_mace = contact_pressure(force, mace_area, cap(200_000));
        let p_spike = contact_pressure(force, spike_area, cap(200_000));
        assert!(
            p_spike > p_mace,
            "the spike concentrates the same force into a higher pressure"
        );
        // The blunt head stays below a hard target's hardness (crush, no pierce); the
        // spike exceeds it (pierce). The distinction is geometry, not two authored types.
        let hardness = Fixed::from_int(150);
        assert!(p_mace < hardness, "the mace crushes rather than pierces");
        assert!(p_spike > hardness, "the morningstar spike pierces");
    }

    #[test]
    fn bend_stress_computes_below_yield_where_the_moment_overflowed() {
        // F=1e8, span=30, Z=1: the moment F*span=3e9 overflows, but the staged form
        // gives 3000 MPa, below a 5000 MPa yield, so the member survives (MECH-RT2-02).
        let (sigma, margin) = bend_stress(
            Fixed::from_int(100_000_000),
            Fixed::from_int(1),
            Fixed::from_int(30),
            Fixed::from_int(5000),
            cap(2_000_000),
        );
        assert_eq!(sigma, Fixed::from_int(3000));
        assert!(margin > ZERO, "the beam survives, not a false collapse");
    }

    #[test]
    fn lever_equal_arms_read_unity_not_a_saturated_advantage() {
        // F=1e8, equal 100 m arms: the product-first form saturated torque and read
        // advantage 0.21; the arm-ratio form reads exactly one (MECH-RT2-03).
        let l = lever(
            Fixed::from_int(100_000_000),
            Fixed::from_int(100),
            Fixed::from_int(100),
            cap(2_000_000_000),
            cap(200_000),
            cap(2_000_000_000),
        );
        assert_eq!(l.mechanical_advantage, Fixed::ONE);
    }

    #[test]
    fn kinetic_energy_is_half_first_and_not_pre_saturated() {
        // m=3000, v=1000: ((3000*0.5)/1000)*1e6 = 1.5e6 kJ, representable, not the cap.
        let ke = kinetic_energy(
            Fixed::from_int(3000),
            Fixed::from_int(1000),
            cap(1_000_000_000),
        );
        assert_eq!(ke, Fixed::from_int(1_500_000));
    }

    #[test]
    fn reduced_mass_impulse_avoids_the_large_product() {
        // Equal masses give a reduced mass of half the mass, computed without forming
        // m1*m2 (which would overflow for large equal masses).
        let big = Fixed::from_int(100_000);
        let j = impulse(big, big, Fixed::from_int(10), ZERO, cap(2_000_000_000));
        // mu = m/2 = 50000; impulse = mu*v*(1+0) = 50000*10 = 5e5.
        assert_eq!(j, Fixed::from_int(500_000));
    }

    #[test]
    fn reach_is_an_order_independent_additive_extent() {
        let segments = [
            Fixed::from_int(1),
            Fixed::from_ratio(1, 2),
            Fixed::from_int(2),
        ];
        let mut reversed = segments;
        reversed.reverse();
        assert_eq!(reach(&segments), reach(&reversed));
        assert_eq!(reach(&segments), Fixed::from_ratio(7, 2));
    }

    // --- Materials ---

    #[test]
    fn euler_buckle_slender_column_routes_to_zero_not_a_wrapped_max() {
        // An extremely slender column (the square would wrap) routes to zero critical
        // load (buckling governs), the correct direction (RT-MAT-01), never a negative
        // wrap inverting the law.
        let p_cr = euler_buckle(
            Fixed::from_int(200_000),
            Fixed::from_int(1),
            Fixed::from_int(1),
            Fixed::from_int(60000), // k_e*L well past the 46340 square ceiling
            cap(2_000_000_000),
        );
        assert_eq!(p_cr, ZERO);
    }

    #[test]
    fn shear_derives_the_von_mises_ratio_when_no_independent_strength() {
        let (_applied, tau_material) = shear(
            ZERO,
            Fixed::from_int(1),
            None,
            Fixed::from_int(1000),
            cap(200_000),
        );
        // tau_material = yield * (1/sqrt 3) ~= 577 MPa.
        assert!(tau_material > Fixed::from_int(576) && tau_material < Fixed::from_int(578));
    }

    // --- Thermal ---

    #[test]
    fn thermal_diffusivity_is_k_over_rho_c_and_separates_two_media() {
        // Real air against real water (Incropera and DeWitt): both diffusivities are positive, small,
        // and representable, and air diffuses heat far faster than water, purely from k/(rho*c). The
        // medium is the lever; the diffusivity is not a free scalar.
        let alpha_max = cap(1);
        // Air: k=0.0262 W/m/K, rho=1.2 kg/m^3, c=1005 J/kg/K -> alpha ~ 2.17e-5 m^2/s.
        let air = thermal_diffusivity(
            Fixed::from_ratio(262, 10_000),
            Fixed::from_ratio(12, 10),
            Fixed::from_int(1005),
            alpha_max,
        );
        // Water: k=0.606, rho=1000, c=4186 -> alpha ~ 1.45e-7 m^2/s.
        let water = thermal_diffusivity(
            Fixed::from_ratio(606, 1000),
            Fixed::from_int(1000),
            Fixed::from_int(4186),
            alpha_max,
        );
        assert!(
            air > ZERO && water > ZERO,
            "both diffusivities are positive"
        );
        assert!(
            air > water,
            "air conducts heat faster than water from k/(rho*c) ({air:?} > {water:?})"
        );
        // A massless (zero heat capacity) medium saturates to the cap; nothing wraps negative.
        assert_eq!(
            thermal_diffusivity(Fixed::from_int(1), ZERO, Fixed::from_int(1), alpha_max),
            alpha_max
        );
    }

    #[test]
    fn conduction_saturates_on_a_zero_path_and_is_finite_otherwise() {
        let max = cap(1_000_000_000);
        // Zero path is infinite conductance, the max flux.
        assert_eq!(
            conduction(
                Fixed::from_int(400),
                Fixed::from_int(1),
                Fixed::from_int(500),
                Fixed::from_int(300),
                ZERO,
                max
            ),
            max
        );
        // A finite path gives a finite, representable flux.
        let f = conduction(
            Fixed::from_int(400),
            Fixed::from_int(1),
            Fixed::from_int(500),
            Fixed::from_int(300),
            Fixed::from_int(2),
            max,
        );
        assert!(f > ZERO && f < max);
    }

    #[test]
    fn combustion_below_ignition_releases_nothing() {
        let c = combustion(
            Fixed::from_int(18000),
            Fixed::from_int(1),
            Fixed::from_int(570),
            Fixed::from_int(1),
            Fixed::from_int(10),
            Fixed::from_int(400), // below ignition
            cap(1_000_000_000),
        );
        assert!(!c.ignited);
        assert_eq!(c.energy, ZERO);
        assert_eq!(c.limiter, Limiter::NotIgnited);
    }

    #[test]
    fn combustion_super_abundant_oxidiser_reads_fuel_limited() {
        // A tiny oxidiser demand against abundant oxidiser overflows the quotient and
        // must read fuel-limited, not the mislabel (NEW-L4-LIMITER).
        let c = combustion(
            Fixed::from_int(18000),
            Fixed::from_decimal_str("0.0001").unwrap(),
            Fixed::from_int(570),
            Fixed::from_int(1),
            Fixed::from_int(1_000_000),
            Fixed::from_int(1000),
            cap(1_000_000_000),
        );
        assert!(c.ignited);
        assert_eq!(c.limiter, Limiter::Fuel);
    }

    #[test]
    fn thermal_stress_folds_the_constraint_before_the_modulus() {
        // A reducing constraint must not pre-saturate a representable stress (F3).
        let (sigma, _fractured) = thermal_stress(
            Fixed::from_int(1000),
            Fixed::from_int(500),
            Fixed::from_int(6000),
            Fixed::from_ratio(1, 10),
            Fixed::from_int(1_000_000_000),
            cap(2_000_000_000),
        );
        assert!(sigma > ZERO && sigma < cap(2_000_000_000));
    }

    #[test]
    fn caps_are_dev_fixtures_only() {
        // A guard so the fixture helpers are exercised and the intent is on record: these
        // caps are test stand-ins for the owner's reserved values, never canon.
        assert_eq!(F_INT(7), Fixed::from_int(7));
        assert_eq!(cap(3), Fixed::from_int(3));
    }

    // --- Metabolism (R-METABOLIZE) ---

    #[test]
    fn basal_rate_reproduces_a_mass_three_quarters_by_the_two_sqrt_identity() {
        // m^(3/4) = sqrt(m * sqrt(m)), exact where m is a perfect fourth power: 16^(3/4) = 8 and
        // 256^(3/4) = 64, reconstructed to the last fixed-point bit by the two integer square roots.
        let a = Fixed::ONE;
        let big = cap(1_000_000);
        assert_eq!(
            basal_metabolic_rate(Fixed::from_int(16), a, big),
            Fixed::from_int(8),
            "16^(3/4) = 8 by the two-sqrt identity"
        );
        assert_eq!(
            basal_metabolic_rate(Fixed::from_int(256), a, big),
            Fixed::from_int(64),
            "256^(3/4) = 64"
        );
        // The coefficient scales the power linearly.
        assert_eq!(
            basal_metabolic_rate(Fixed::from_int(16), Fixed::from_int(3), big),
            Fixed::from_int(24),
            "a scales the rate: 3 * 16^(3/4) = 24"
        );
    }

    #[test]
    fn basal_rate_is_zero_at_zero_mass_and_saturates_to_the_cap() {
        assert_eq!(
            basal_metabolic_rate(ZERO, Fixed::ONE, cap(1_000_000)),
            ZERO,
            "no mass, no metabolism"
        );
        assert_eq!(
            basal_metabolic_rate(Fixed::from_int(256), Fixed::ONE, Fixed::from_int(10)),
            Fixed::from_int(10),
            "64 W against a 10 W cap routes to the cap"
        );
    }

    #[test]
    fn basal_rate_is_monotone_increasing_yet_sublinear() {
        let a = Fixed::ONE;
        let big = cap(1_000_000);
        let small = basal_metabolic_rate(Fixed::from_int(16), a, big); // 8
        let large = basal_metabolic_rate(Fixed::from_int(256), a, big); // 64
        assert!(large > small, "a larger body has the higher resting rate");
        // Sublinear: mass rose 16x (16 -> 256) but the rate rose only 8x (8 -> 64), so the rate is
        // below the linear extrapolation of the smaller body (the Kleiber signature).
        assert!(
            large < small.checked_mul(Fixed::from_int(16)).unwrap(),
            "the rate grows slower than mass: 64 < 8 * 16"
        );
    }

    #[test]
    fn resting_loss_is_the_saturating_sum_of_convection_and_radiation() {
        // The thermoregulatory loss is exactly convective_flux + radiant_emission over the area, the two
        // resolved heat-transport kernels reused unchanged. A body warmer than its medium so both terms
        // are positive.
        let h = Fixed::from_ratio(1, 10);
        let area = Fixed::from_int(2);
        let body = Fixed::from_int(310);
        let medium = Fixed::from_int(280);
        let emissivity = Fixed::from_ratio(95, 100);
        // Sigma at a fine scale (5.67e-8 at scale 55), the value the Tier-2 radiant term consumes.
        let sigma_scale = 55u32;
        let sigma_bits = civsim_units::bignum::BigRat::from_decimal_str("0.0000000567")
            .unwrap()
            .round_to_scale(sigma_scale)
            .unwrap() as i64;
        let big = cap(1_000_000_000);
        let convective = convective_flux(h, area, body, medium, big);
        let radiant =
            radiant_emission_tier2(emissivity, area, body, medium, sigma_bits, sigma_scale, big);
        let want = Fixed::saturating_sum([convective, radiant]).min(big);
        assert_eq!(
            resting_heat_loss(
                h,
                area,
                body,
                medium,
                emissivity,
                sigma_bits,
                sigma_scale,
                big
            ),
            want,
            "resting loss = convective_flux + the Tier-2 radiant term over the area"
        );
        // A body at the medium temperature loses nothing.
        assert_eq!(
            resting_heat_loss(
                h,
                area,
                body,
                body,
                emissivity,
                sigma_bits,
                sigma_scale,
                big
            ),
            ZERO,
            "no gradient, no loss (equilibrium)"
        );
    }

    #[test]
    fn drain_fraction_is_energy_spent_over_energy_stored() {
        // basal 10 W, no heat loss, reserve 100 units at density 1 (stored 100 J), one-second tick:
        // spent 10 J over stored 100 J is a tenth of the reserve per tick.
        let frac = metabolic_drain_fraction(
            Fixed::from_int(10),
            ZERO,
            Fixed::from_int(100),
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE,
        );
        assert_eq!(frac, Fixed::from_ratio(1, 10));
        // A larger store drains a smaller fraction of itself for the same power (the reserve-side half of
        // the Kleiber signature): ten times the stored energy, a tenth of the fraction.
        let bigger = metabolic_drain_fraction(
            Fixed::from_int(10),
            ZERO,
            Fixed::from_int(1000),
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE,
        );
        assert!(bigger < frac, "a larger reserve drains a smaller fraction");
        assert_eq!(bigger, Fixed::from_ratio(1, 100));
        // A zero-energy reserve (no energy tissue) drains fully to the cap; a zero-power demand drains
        // nothing.
        assert_eq!(
            metabolic_drain_fraction(
                Fixed::from_int(10),
                ZERO,
                ZERO,
                Fixed::ONE,
                Fixed::ONE,
                Fixed::ONE
            ),
            Fixed::ONE,
            "no reserve tissue, full drain"
        );
        assert_eq!(
            metabolic_drain_fraction(
                ZERO,
                ZERO,
                Fixed::from_int(100),
                Fixed::ONE,
                Fixed::ONE,
                Fixed::ONE
            ),
            ZERO,
            "no resting power, no drain"
        );
    }

    // --- Hardening: product-before-divide reassociation (the wave-1 discipline, extended) ---

    #[test]
    fn wear_reassociates_the_scale_before_the_product() {
        // In-range axes (force 1e8, slide 10, coefficient 1000 stored x1e6 so K=1e-3, hardness 1 MPa)
        // whose K_scaled*F*s would overflow a Fixed must yield the true wear from the i128 chain, with
        // the SI cubic-metre volume promoting the megapascal hardness to pascals:
        // V = K*F*s/H_Pa = 1e-3*1e8*10/(1*1e6) = 1.0 m^3, not the false cap.
        let wear_max = cap(2_000_000_000);
        let w = wear(
            F_INT(1000),        // wear_coefficient_scaled (K x 1e6)
            F_INT(1_000_000),   // coefficient_scale
            F_INT(100_000_000), // force 1e8
            F_INT(10),          // slide distance
            F_INT(1),           // hardness (MPa)
            wear_max,
        );
        assert!(
            w > Fixed::from_ratio(99, 100) && w < Fixed::from_ratio(101, 100),
            "wear = {w:?} should be the true ~1.0 m^3, not the cap"
        );
    }

    #[test]
    fn coulomb_divides_by_separation_before_the_charge_product() {
        // A distant large-charge pair (q=1e5 each, r=100) whose |q1|*|q2| would overflow must yield
        // the true modest force, not the max cap; and the sign is repulsive for like charges.
        let f_max = cap(2_000_000_000);
        let (f, repulsive) = coulomb_force(
            F_INT(100_000),
            F_INT(100_000),
            F_INT(100),
            Fixed::ONE,
            f_max,
        );
        assert!(
            f > ZERO && f < f_max,
            "force = {f:?} should be the true ~1e6, not the cap"
        );
        assert!(repulsive, "like charges repel");
    }

    #[test]
    fn resistance_divides_by_area_before_the_resistivity() {
        // In-range axes (resistivity 1000, length 2.2e6, area 1e6) whose rho*length would overflow
        // must yield the true ~2200 ohm, not the open-circuit cap.
        let r_max = cap(2_000_000_000);
        let r = resistance(F_INT(1000), F_INT(2_200_000), F_INT(1_000_000), r_max);
        assert!(
            r > F_INT(2000) && r < r_max,
            "resistance = {r:?} should be the true ~2200, not the cap"
        );
    }

    #[test]
    fn coulomb_wide_form_keeps_the_true_force_beyond_the_reserved_charge_range() {
        // The wide i128 evaluation caps only on the true force, so it holds even for an asymmetric
        // large-and-small charge pair whose |q1|/r overflows a Fixed (the reassociated form's
        // false-cap corner). q1 = 2e9, q2 = 1e-6, r = 0.5, k = 1: the true force is
        // k*|q1||q2|/r^2 = 2e9*1e-6/0.25 = 8000, well below the 2e9 ceiling.
        let f_max = cap(2_000_000_000);
        let (f, repulsive) = coulomb_force(
            F_INT(2_000_000_000),
            Fixed::from_ratio(1, 1_000_000),
            Fixed::from_ratio(1, 2),
            Fixed::ONE,
            f_max,
        );
        assert!(
            f > F_INT(7_900) && f < F_INT(8_100),
            "force = {f:?} should be the true ~8000, not the cap"
        );
        assert!(repulsive, "like-signed charges repel");
    }

    #[test]
    fn wear_wide_form_keeps_full_precision_for_a_sub_unit_coefficient() {
        // At the mild-lubricated low end (K_scaled = 0.001 stored x1e6, so true K = 1e-9), the wide
        // i128 evaluation must reconstruct the true wear without losing the low bits, with the SI
        // volume promoting the megapascal hardness to pascals: V = K*F*s/H_Pa =
        // 1e-9 * 1e8 * 10 / (1*1e6) = 1e-6 m^3 (a Fixed-reduced coefficient would floor to zero).
        let wear_max = cap(2_000_000_000);
        let w = wear(
            Fixed::from_ratio(1, 1_000), // wear_coefficient_scaled = 0.001 (true K = 1e-9)
            F_INT(1_000_000),            // coefficient_scale
            F_INT(100_000_000),          // force 1e8
            F_INT(10),                   // slide distance
            F_INT(1),                    // hardness (MPa)
            wear_max,
        );
        assert!(
            w > Fixed::from_ratio(9, 10_000_000) && w < Fixed::from_ratio(11, 10_000_000),
            "wear = {w:?} should be the true ~1e-6 m^3"
        );
    }

    // --- Scale, precision, and reduce-before-grow corrections (blind-audit fixes) ---

    #[test]
    fn euler_buckle_promotes_the_megapascal_modulus_to_a_newton_load() {
        // Iron E = 200000 MPa, I = 1e-6 m^4, K = 1, L = 1: P_cr = pi^2 * E_Pa * I / (KL)^2 =
        // pi^2 * 2e11 * 1e-6 ~ 1.97e6 N. Without the C_PA promotion it read ~1.97 N.
        let p = euler_buckle(
            Fixed::from_int(200_000),
            Fixed::from_ratio(1, 1_000_000),
            Fixed::ONE,
            Fixed::ONE,
            cap(2_000_000_000),
        );
        assert!(
            p > Fixed::from_int(1_900_000) && p < Fixed::from_int(2_050_000),
            "buckling load = {p:?} should be ~1.97e6 N, not ~1.97 N"
        );
    }

    #[test]
    fn thermal_stress_descales_to_megapascals_and_does_not_fracture_on_mild_heating() {
        // Iron E = 200000 MPa, alpha = 12 ppm/K, dT = 1 K, constraint = 1, fracture 500 MPa:
        // sigma = E_Pa * alpha * dT = 2.4e6 Pa = 2.4 MPa, well under fracture. The unbridged kernel
        // left sigma at 2.4e6 (pascals) and fractured spuriously.
        let (sigma, fractured) = thermal_stress(
            Fixed::from_int(200_000),
            Fixed::from_int(12),
            Fixed::ONE,
            Fixed::ONE,
            Fixed::from_int(500),
            cap(2_000_000_000),
        );
        assert!(
            sigma < Fixed::from_int(10),
            "mild sigma = {sigma:?} should be ~2.4 MPa"
        );
        assert!(
            !fractured,
            "a 1 K constrained heating of iron must not fracture"
        );
        // A large gradient still fractures: dT = 300 K gives ~720 MPa > 500.
        let (_s, hot) = thermal_stress(
            Fixed::from_int(200_000),
            Fixed::from_int(12),
            Fixed::from_int(300),
            Fixed::ONE,
            Fixed::from_int(500),
            cap(2_000_000_000),
        );
        assert!(
            hot,
            "a 300 K constrained heating exceeds the fracture strength"
        );
    }

    #[test]
    fn phase_change_energy_bridges_the_sensible_joules_to_kilojoules() {
        // Water, m = 1 kg, c = 4186 J/(kg*K), dT = 10 K, latent = 334 kJ/kg: sensible = 41860 J =
        // 41.86 kJ, plus 334 kJ latent = ~375.86 kJ. The unbridged sum was 41860 + 334.
        let e = phase_change_energy(
            Fixed::ONE,
            Fixed::from_int(4186),
            Fixed::from_int(273),
            Fixed::from_int(263),
            Fixed::from_int(334),
            cap(2_000_000_000),
        );
        assert!(
            e > Fixed::from_int(375) && e < Fixed::from_int(377),
            "phase-change energy = {e:?} should be ~375.86 kJ"
        );
    }

    #[test]
    fn sensible_rise_is_the_energy_over_heat_capacity_and_bounds_the_extremes() {
        // m = 2 kg, c = 1000 J/(kg*K), Q = 1000 J: dT = Q/(m*c) = 1000/2000 = 0.5 K.
        let dt = sensible_rise(
            Fixed::from_int(2),
            Fixed::from_int(1000),
            Fixed::from_int(1000),
            cap(1_000_000),
        );
        assert_eq!(dt, Fixed::from_ratio(1, 2), "dT = {dt:?} should be 0.5 K");
        // A massless body has no heat capacity, so any energy swings it the full reserved rise.
        assert_eq!(
            sensible_rise(
                ZERO,
                Fixed::from_int(1000),
                Fixed::from_int(1000),
                cap(1_000_000)
            ),
            cap(1_000_000),
            "the massless limit is the maximum swing"
        );
        // An overflowing heat capacity is an enormous thermal mass: the rise reads zero, not the
        // cap (the wave-1 F1 fix the kernel comment records).
        assert_eq!(
            sensible_rise(
                Fixed::from_int(2_000_000_000),
                Fixed::from_int(2),
                Fixed::from_int(1000),
                cap(1_000_000),
            ),
            ZERO,
            "an overflowing capacity is a vast mass, a near-zero rise"
        );
    }

    #[test]
    fn poiseuille_flow_keeps_a_representable_flow_off_the_cap() {
        // Air, dp = 1 MPa, r = 0.01 m, mu = 1.78e-5 Pa*s, L = 1 m: Q = pi*dp_Pa*r^4/(8*mu*L) ~ 220
        // m^3/s. The divide-first form overflowed dp/mu and returned the cap.
        let q = poiseuille_flow(
            Fixed::ONE,
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(178, 10_000_000),
            Fixed::ONE,
            cap(2_000_000_000),
        );
        assert!(
            q > Fixed::from_int(150) && q < Fixed::from_int(300),
            "laminar flow = {q:?} should be ~220 m^3/s, not the cap"
        );
    }

    #[test]
    fn reynolds_number_multiplies_length_before_the_viscosity_divide() {
        // rho = 998, v = 1000, L = 1e-3, mu = 1e-4: Re = rho*v*L/mu = 9.98e6, representable. The
        // divide-first form overflowed rho*v/mu and returned the cap.
        let re = reynolds_number(
            Fixed::from_int(998),
            Fixed::from_int(1000),
            Fixed::from_ratio(1, 1000),
            Fixed::from_ratio(1, 10_000),
            cap(2_000_000_000),
        );
        assert!(
            re > Fixed::from_int(9_000_000) && re < Fixed::from_int(11_000_000),
            "Reynolds = {re:?} should be ~9.98e6, not the cap"
        );
    }

    #[test]
    fn radiative_equilibrium_roots_the_factors_so_low_emissivity_does_not_underflow() {
        let sigma = Fixed::from_ratio(567, 10_000_000_000); // 5.67e-8
                                                            // emissivity 0.004, absorbed 1000: T = (1000/(0.004*sigma))^(1/4) ~ 1450 K. Forming
                                                            // emissivity*sigma underflowed to zero and returned the cap.
        let t = radiative_equilibrium(
            Fixed::from_int(1000),
            Fixed::from_ratio(4, 1000),
            sigma,
            Fixed::from_int(100_000),
        );
        assert!(
            t > Fixed::from_int(1200) && t < Fixed::from_int(1700),
            "equilibrium temperature = {t:?} should be ~1450 K, not the cap"
        );
    }

    #[test]
    fn lever_caps_the_mechanical_advantage_on_the_success_path() {
        // effort 100, load 1e-6 gives a raw ratio 1e8; the advantage_max = 50 cap must bind.
        let l = lever(
            Fixed::from_int(100),
            Fixed::from_int(100),
            Fixed::from_ratio(1, 1_000_000),
            cap(2_000_000_000),
            Fixed::from_int(50),
            cap(2_000_000_000),
        );
        assert_eq!(l.mechanical_advantage, Fixed::from_int(50));
    }

    #[test]
    fn interface_split_conserves_flux_when_reflectance_plus_transmittance_exceed_one() {
        // R = T = 0.7 is physically impossible (R + T <= 1); the triple must still sum to the
        // incident flux, not 1.4x it.
        let (r, a, t) = interface_split(
            Fixed::from_int(100),
            Fixed::from_ratio(7, 10),
            Fixed::from_ratio(7, 10),
        );
        assert_eq!(
            r + a + t,
            Fixed::from_int(100),
            "R + A + T must equal the incident flux"
        );
        assert!(
            r > Fixed::from_int(69) && r < Fixed::from_int(71),
            "reflected ~70"
        );
        assert!(
            t > Fixed::from_int(29) && t < Fixed::from_int(31),
            "transmitted ~30"
        );
        assert_eq!(
            a, ZERO,
            "the residual is fully consumed, no negative absorbed"
        );
    }

    // --- Overflow-direction and sign corrections (blind-audit latent-class sweep) ---

    #[test]
    fn overflowing_and_degenerate_branches_route_to_the_correct_physical_extreme() {
        // satisfaction: an overflowing supply*assimilation is abundance, so full satisfaction.
        assert_eq!(
            satisfaction(Fixed::MAX, Fixed::MAX, Some(Fixed::ONE)),
            ONE,
            "an overflowing supply product is fully satisfied, not starving"
        );
        // contact_pressure: an overflowing contact area spreads the force to zero pressure.
        assert_eq!(
            contact_pressure(Fixed::ONE, Fixed::MAX, cap(2_000_000_000)),
            ZERO,
            "a vast contact area gives negligible pressure, not the max"
        );
        // sensible_energy: a cooling (negative gradient) is zero over the [0, E_MAX] law.
        assert_eq!(
            sensible_energy(
                Fixed::ONE,
                Fixed::from_int(4186),
                Fixed::from_int(-10),
                cap(2_000_000_000)
            ),
            ZERO,
            "a negative temperature gradient contributes no positive sensible heat"
        );
        // ideal_gas_density: an overflowing R*T denominator drives density to its minimum.
        assert_eq!(
            ideal_gas_density(
                Fixed::ONE,
                Fixed::from_int(100_000),
                Fixed::from_int(30_000),
                Fixed::from_int(1),
                cap(2_000_000_000),
            ),
            Fixed::from_int(1),
            "a huge R*T gives a vanishing density (rho_min), not rho_max"
        );
    }

    #[test]
    fn em_magnitude_kernels_bound_non_negative() {
        // ohm_voltage is a magnitude: a negative current gives a positive voltage.
        assert_eq!(
            ohm_voltage(Fixed::from_int(-5), Fixed::from_int(10), cap(2_000_000_000)),
            Fixed::from_int(50),
            "V = |I|*R is non-negative"
        );
        // flux_linkage is a non-negative magnitude over [0, phi_max] (consistent with the magnitude
        // flux density and the floor bound): a product over the cap clamps to phi_max, and an
        // out-of-domain negative clamps to the zero floor rather than staying unbounded.
        assert_eq!(
            flux_linkage(
                Fixed::from_int(100),
                Fixed::from_int(10),
                Fixed::from_int(500)
            ),
            Fixed::from_int(500),
            "a large flux clamps to phi_max"
        );
        assert_eq!(
            flux_linkage(
                Fixed::from_int(-100),
                Fixed::from_int(10),
                Fixed::from_int(500)
            ),
            ZERO,
            "an out-of-domain negative flux clamps to the zero floor"
        );
    }

    // --- Hardening: temperature/potential differences saturate rather than panic ---

    #[test]
    fn difference_kernels_saturate_at_the_extremes_without_panicking() {
        // Under overflow-checks (on in debug and release), a raw i64 subtract of MIN/MAX inputs
        // would panic. Every difference-taking kernel must instead route to a defined saturated
        // value. Reaching the assertions at all proves no panic; the ranges prove the result is sane.
        let m = cap(1_000_000_000);
        assert!(convective_flux(F_INT(10), F_INT(10), Fixed::MAX, Fixed::MIN, m) <= m);
        let a_max = cap(100);
        let a = thermal_buoyancy(Fixed::MIN, F_INT(288), F_INT(10), a_max);
        assert!(a >= ZERO - a_max && a <= a_max);
        assert!(saturation_vapor_pressure(Fixed::MIN, Fixed::ONE, F_INT(300), F_INT(1), m) >= ZERO);
        assert_eq!(
            evaporation_rate(Fixed::MAX, Fixed::MIN, ZERO, ZERO, ZERO, m),
            ZERO
        );
        assert!(corrosion(Fixed::MAX, Fixed::MIN, Fixed::ONE, Fixed::ONE, m) <= m);
        let eta = carnot_limit(F_INT(300), Fixed::MIN);
        assert!(eta >= ZERO && eta <= Fixed::ONE);
        assert!(conduction(F_INT(1), F_INT(1), Fixed::MAX, Fixed::MIN, F_INT(1), m) <= m);
        let (margin, _g) = fracture_onset(Fixed::MIN, Fixed::MAX, F_INT(1), F_INT(1), ZERO, m);
        assert!(
            margin > ZERO,
            "MAX strength minus MIN stress saturates positive"
        );
    }

    // --- Language processing cost (R-LANG-TYPOLOGY) ---

    #[test]
    fn parse_cost_is_zero_at_zero_extent_and_below() {
        let m = Fixed::from_int(4);
        let cap = Fixed::ONE;
        assert_eq!(parse_cost(ZERO, m, cap), ZERO, "no domain, no cost");
        assert_eq!(
            parse_cost(Fixed::from_int(-3), m, cap),
            ZERO,
            "a negative extent has no cost"
        );
    }

    #[test]
    fn parse_cost_saturates_at_the_cap_for_an_unbounded_extent() {
        let m = Fixed::from_int(4);
        let cap = Fixed::from_int(7);
        // A huge extent against a finite memory drives extent/(extent+memory) -> one, so the cost
        // reaches the cap rather than wrapping.
        assert_eq!(
            parse_cost(Fixed::MAX, m, cap),
            cap,
            "unbounded extent saturates"
        );
        // And it never exceeds the cap on the way there.
        assert!(parse_cost(Fixed::from_int(1000), m, cap) <= cap);
        assert!(
            parse_cost(Fixed::from_int(1000), m, cap) < cap,
            "still below the cap at a finite extent"
        );
    }

    #[test]
    fn parse_cost_is_monotone_increasing_in_extent() {
        let m = Fixed::from_int(4);
        let cap = Fixed::ONE;
        let c1 = parse_cost(Fixed::from_int(1), m, cap);
        let c2 = parse_cost(Fixed::from_int(2), m, cap);
        let c3 = parse_cost(Fixed::from_int(8), m, cap);
        assert!(c1 < c2 && c2 < c3, "cost rises with the held domain extent");
        // At extent == memory the cost is half the cap (the Hill half-saturation point).
        let half = parse_cost(m, m, cap);
        assert_eq!(
            half,
            Fixed::from_ratio(1, 2),
            "half cost at extent == memory"
        );
    }

    #[test]
    fn parse_cost_is_softened_by_working_memory() {
        let cap = Fixed::ONE;
        let extent = Fixed::from_int(4);
        let small = parse_cost(extent, Fixed::from_int(1), cap);
        let large = parse_cost(extent, Fixed::from_int(16), cap);
        assert!(
            large < small,
            "a larger working-memory capacity lowers the parse cost of the same domain"
        );
    }

    #[test]
    fn parse_cost_caps_rather_than_wraps_at_extremes() {
        // Adversarial extremes route to a bounded [0, cap], never a wrap or panic.
        let cap = Fixed::from_int(5);
        for &e in &[Fixed::MIN, ZERO, Fixed::ONE, Fixed::MAX] {
            for &mem in &[Fixed::MIN, ZERO, Fixed::from_int(3), Fixed::MAX] {
                let c = parse_cost(e, mem, cap);
                assert!(
                    c >= ZERO && c <= cap,
                    "parse_cost stayed in [0, cap] for e and mem"
                );
            }
        }
    }

    #[test]
    fn harmony_tilt_floors_at_one_and_needs_a_reduction() {
        let temp = Fixed::from_ratio(1, 10);
        let cap = Fixed::from_int(64);
        assert_eq!(harmony_tilt(ZERO, temp, cap), ONE, "no reduction, no tilt");
        assert_eq!(
            harmony_tilt(Fixed::from_int(-2), temp, cap),
            ONE,
            "a negative reduction never pushes below one"
        );
        // A positive reduction earns a tilt strictly above one.
        let t = harmony_tilt(Fixed::from_ratio(3, 10), temp, cap);
        assert!(
            t > ONE && t <= cap,
            "a real reduction earns a bounded boost"
        );
    }

    #[test]
    fn harmony_tilt_saturates_at_the_cap_for_an_unbounded_reduction() {
        let tiny_temp = Fixed::from_ratio(1, 1000);
        let cap = Fixed::from_int(32);
        // A large reduction over a tiny temperature drives exp past the representable range: it
        // saturates at the cap rather than wrapping.
        assert_eq!(harmony_tilt(Fixed::from_int(100), tiny_temp, cap), cap);
        // The deterministic zero-temperature limit is the same hard max.
        assert_eq!(harmony_tilt(Fixed::from_ratio(1, 4), ZERO, cap), cap);
    }

    #[test]
    fn harmony_tilt_is_monotone_in_the_reduction() {
        let temp = Fixed::from_ratio(1, 4);
        let cap = Fixed::from_int(1 << 16);
        let a = harmony_tilt(Fixed::from_ratio(1, 10), temp, cap);
        let b = harmony_tilt(Fixed::from_ratio(3, 10), temp, cap);
        let c = harmony_tilt(Fixed::from_ratio(6, 10), temp, cap);
        assert!(a < b && b < c, "a larger avoided cost earns a larger tilt");
    }

    // --- Nernst EMF and reversible uptake flux (redox depth extension) ---

    #[test]
    fn standard_potential_shifts_linearly_with_temperature() {
        // E0(T) = E0_ref + (dE0/dT)(T - T_ref): at the reference temperature it is the reference potential;
        // a positive temperature coefficient raises it above and a lower temperature drops it below.
        let e0 = Fixed::from_ratio(8, 10);
        let de0_dt = Fixed::from_ratio(1, 1000); // +1 mV/K
        let t_ref = Fixed::from_int(298);
        assert_eq!(
            standard_potential_at_temperature(e0, de0_dt, t_ref, t_ref),
            e0,
            "at the reference temperature the potential is unchanged"
        );
        let warmer = standard_potential_at_temperature(e0, de0_dt, Fixed::from_int(308), t_ref);
        let cooler = standard_potential_at_temperature(e0, de0_dt, Fixed::from_int(288), t_ref);
        assert!(
            warmer > e0 && cooler < e0,
            "the standard potential shifts with temperature (warmer {warmer:?}, cooler {cooler:?})"
        );
        assert_eq!(
            standard_potential_at_temperature(e0, Fixed::ZERO, Fixed::from_int(400), t_ref),
            e0,
            "a zero coefficient is temperature-independent"
        );
    }

    #[test]
    fn nernst_emf_reduces_to_standard_at_unit_activity_and_falls_as_the_couple_depletes() {
        // The Nernst EMF corrects the standard cell EMF for the couple's actual activities. At unit activity
        // (the standard state, ln 1 = 0) it is exactly the standard EMF; as the donor and acceptor deplete
        // below the standard state their logs go negative, so the drive falls and eventually crosses zero
        // (no life below the couple's own equilibrium), rather than the concentration-independent standard EMF.
        // The thermal factor is the per-particle k_B*T/q: k_B*T = 0.0257 and q = 1 give k_B*T/q ~ 0.0257 V.
        let e_std = Fixed::from_ratio(8, 10); // +0.8 V standard cell EMF
        let kt = Fixed::from_ratio(257, 10_000); // k_B*T ~ 0.0257 (eV-scale)
        let temp = Fixed::ONE; // unit temperature (kt folded above)
        let q = Fixed::ONE; // carrier charge n*e = 1 in these units
                            // Unit activities: exactly the standard EMF.
        let at_standard = nernst_emf(e_std, Fixed::ONE, Fixed::ONE, kt, temp, q);
        assert_eq!(
            at_standard, e_std,
            "at unit activity the Nernst EMF is the standard EMF, got {at_standard:?}"
        );
        // Depleted below standard: the EMF falls (the drive weakens as the stock is consumed).
        let depleted = nernst_emf(
            e_std,
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(1, 100),
            kt,
            temp,
            q,
        );
        assert!(
            depleted < at_standard,
            "a depleted couple drives less than the standard state (depleted {depleted:?}, standard {at_standard:?})"
        );
        // Richer than standard (activity above one): the EMF rises above the standard EMF.
        let rich = nernst_emf(e_std, Fixed::from_int(10), Fixed::from_int(10), kt, temp, q);
        assert!(
            rich > at_standard,
            "a couple richer than standard drives more (rich {rich:?}, standard {at_standard:?})"
        );
        // A fully depleted species (zero activity) collapses the drive to the equilibrium boundary: no life.
        let empty = nernst_emf(e_std, Fixed::ZERO, Fixed::ONE, kt, temp, q);
        assert_eq!(
            empty,
            Fixed::ZERO,
            "an empty donor collapses the drive to no yield, got {empty:?}"
        );
    }

    #[test]
    fn reversible_uptake_flux_saturates_in_stock_drives_with_emf_and_conserves() {
        // The reversible Michaelis-Menten flux saturates in the substrate (Hill/Monod), scales with the
        // thermodynamic drive of the EMF, is floored at zero below equilibrium, and never exceeds the present
        // stock (the structural min(v, S) conservation clamp). Thermal factor is the per-particle k_B*T/q.
        let kt = Fixed::from_ratio(257, 10_000);
        let temp = Fixed::ONE;
        let q = Fixed::ONE;
        let vmax = Fixed::from_int(2); // the caller derives this from catalyst tissue; a fixture here
        let km = Fixed::ONE;
        let h = Fixed::ONE; // plain Monod S/(Km+S)
        let e_fwd = Fixed::from_ratio(8, 10); // strongly spontaneous

        // Monotone rising in the stock (more substrate, more flux), up to Vmax*drive.
        let low = reversible_uptake_flux(Fixed::from_ratio(1, 2), vmax, km, h, e_fwd, kt, temp, q);
        let high = reversible_uptake_flux(Fixed::from_int(100), vmax, km, h, e_fwd, kt, temp, q);
        assert!(
            high > low && low > Fixed::ZERO,
            "the flux rises and saturates with the stock (low {low:?}, high {high:?})"
        );
        // Drive: a stronger EMF pulls a larger flux at the same stock; a zero EMF (at equilibrium) pulls none.
        let weak = reversible_uptake_flux(
            Fixed::from_int(100),
            vmax,
            km,
            h,
            Fixed::from_ratio(1, 100),
            kt,
            temp,
            q,
        );
        assert!(
            high > weak,
            "a stronger EMF drives a larger flux (strong {high:?}, weak {weak:?})"
        );
        let at_equil =
            reversible_uptake_flux(Fixed::from_int(100), vmax, km, h, Fixed::ZERO, kt, temp, q);
        assert_eq!(
            at_equil,
            Fixed::ZERO,
            "at equilibrium (zero EMF) the flux is zero, got {at_equil:?}"
        );
        // Below equilibrium (negative EMF): no reverse uptake, floored at zero.
        let below = reversible_uptake_flux(
            Fixed::from_int(100),
            vmax,
            km,
            h,
            Fixed::from_ratio(-5, 10),
            kt,
            temp,
            q,
        );
        assert_eq!(
            below,
            Fixed::ZERO,
            "below its equilibrium the source powers no life, got {below:?}"
        );
        // Conservation: with a small stock the draw is capped at the stock (min(v, S)), never more.
        let tiny_stock = Fixed::from_ratio(1, 100);
        let capped = reversible_uptake_flux(tiny_stock, vmax, km, h, e_fwd, kt, temp, q);
        assert!(
            capped <= tiny_stock,
            "the flux never exceeds the present stock (flux {capped:?}, stock {tiny_stock:?})"
        );
    }

    #[test]
    fn elastic_recoil_energy_is_the_resilience_times_volume_and_gates_on_absent_material() {
        // The stroke-rate step-2 elastic kernel law: the delivered recoil energy is the modulus of resilience
        // `yield^2 / (2 E)` (the elastic strain-energy density up to yield) times the strained volume, on the
        // joule scale `F d` is on. Values chosen so the fixed-point arithmetic is exact: yield 200 MPa, modulus
        // 2000 MPa, so resilience = 200^2 / (2*2000) = 40000/4000 = 10 (MPa = MJ/m^3); `* C_PA` (1e6) = 1e7 J/m^3;
        // `* volume` 1 m^3 = 1e7 J, under a 1e8 cap.
        let cap = Fixed::from_int(100_000_000);
        let yield_s = Fixed::from_int(200);
        let modulus = Fixed::from_int(2000);
        let volume = Fixed::ONE;
        let e = elastic_recoil_energy(yield_s, modulus, volume, cap);
        assert_eq!(
            e,
            Fixed::from_int(10_000_000),
            "resilience yield^2/(2E) times C_PA times volume, on the joule scale"
        );
        // A stiffer material of the same yield stores LESS recoil energy (resilience falls as the modulus rises).
        let stiff = elastic_recoil_energy(yield_s, Fixed::from_int(20000), volume, cap);
        assert!(
            stiff < e && stiff > ZERO,
            "a stiffer spring of the same yield stores less recoil energy (stiff {stiff:?} vs {e:?})"
        );
        // A higher-yield material of the same modulus stores MORE (resilience rises with yield squared).
        let tougher = elastic_recoil_energy(Fixed::from_int(400), modulus, volume, cap);
        assert!(
            tougher > e,
            "a higher-yield spring stores more recoil energy (tougher {tougher:?} vs {e:?})"
        );
        // The absence convention: no yield, no modulus, or no volume stores no elastic energy, so a rigid or
        // fluid actuator self-gates and the elastic kernel contributes nothing until a world grows a springy tissue.
        assert_eq!(
            elastic_recoil_energy(ZERO, modulus, volume, cap),
            ZERO,
            "no yield strength: no recoil"
        );
        assert_eq!(
            elastic_recoil_energy(yield_s, ZERO, volume, cap),
            ZERO,
            "no elastic modulus: no recoil"
        );
        assert_eq!(
            elastic_recoil_energy(yield_s, modulus, ZERO, cap),
            ZERO,
            "no strained volume: no recoil"
        );
        // Deterministic (Principle 3).
        assert_eq!(e, elastic_recoil_energy(yield_s, modulus, volume, cap));
    }
}
