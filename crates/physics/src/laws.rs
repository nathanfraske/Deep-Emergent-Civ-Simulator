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
    let num = supply.checked_mul(assimilation).unwrap_or(ZERO);
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
        None => return p_max,
    };
    if den == ZERO {
        return p_max;
    }
    match force.checked_div(den) {
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
    (sigma, yield_strength - sigma)
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
    (sigma, yield_strength - sigma)
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
    let stress_margin = fracture_strength - applied_stress;
    let g_avail = match fracture_energy.checked_mul(crack_area) {
        Some(g) => g.min(energy_max),
        None => energy_max,
    };
    (stress_margin, g_avail - delivered_energy)
}

/// Delivered kinetic energy on the kilojoule scale. The half is applied before the
/// mass-velocity-squared product and the scale bridge is applied before the squared
/// velocity, so a representable energy is never pre-saturated (the wave-1 fix); the
/// squared velocity is guarded against its overflow-safe ceiling.
pub fn kinetic_energy(mass: Fixed, velocity: Fixed, energy_max: Fixed) -> Fixed {
    if velocity.abs() > V2_MAX {
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
            Some(m) => m,
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
    let slip_margin = f_s_max - tangential;
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
        let net = (input_power - power_loss).max(ZERO);
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
    match pi_squared().checked_mul(ei) {
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
    let fs = match force.checked_mul(distance) {
        Some(x) => x,
        None => return wear_max,
    };
    let kfs = match wear_coefficient_scaled.checked_mul(fs) {
        Some(x) => x,
        None => return wear_max,
    };
    let unscaled = match kfs.checked_div(coefficient_scale) {
        Some(x) => x,
        None => return wear_max,
    };
    if hardness == ZERO {
        return wear_max;
    }
    match unscaled.checked_div(hardness) {
        Some(v) => v.min(wear_max),
        None => wear_max,
    }
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
    let delta_t = (hot_temperature - cold_temperature).abs();
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
    if capacity == ZERO {
        return rise_max;
    }
    match energy.checked_div(capacity) {
        Some(dt) => dt.min(rise_max),
        None => rise_max,
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
    let delta_t = transition_temperature - start_temperature;
    let sensible =
        sensible_energy(mass, specific_heat, delta_t, energy_max).clamp(ZERO, energy_max);
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
    let sigma = match modulus.checked_mul(strain_effective) {
        Some(x) => x.clamp(-stress_max, stress_max),
        None => return (stress_max, true),
    };
    let fractured = sigma.abs() >= fracture_strength;
    (sigma, fractured)
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
    if velocity.abs() > V2_MAX {
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
fn aero_force(coefficient: Fixed, density: Fixed, area: Fixed, velocity: Fixed, f_max: Fixed) -> Fixed {
    if velocity.abs() > V2_MAX {
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
pub fn drag_force(drag_coefficient: Fixed, density: Fixed, area: Fixed, velocity: Fixed, f_max: Fixed) -> Fixed {
    aero_force(drag_coefficient, density, area, velocity, f_max)
}

/// Aerodynamic lift (1/2) Cl rho A v^2 (N), the reduced-order lumped-coefficient lift that floors a
/// wing, a gliding creature, a sail, and the lift half of a ballistic arc.
pub fn aerodynamic_lift(lift_coefficient: Fixed, density: Fixed, area: Fixed, velocity: Fixed, f_max: Fixed) -> Fixed {
    aero_force(lift_coefficient, density, area, velocity, f_max)
}

/// Reynolds number Re = rho*|v|*L/mu (dimensionless), a laminar/turbulent regime gate. The transition
/// Reynolds number is a reserved consumer constant, kept out of the kernel. Zero speed reads zero, an
/// inviscid fluid reads the cap.
pub fn reynolds_number(density: Fixed, velocity: Fixed, length: Fixed, viscosity: Fixed, re_max: Fixed) -> Fixed {
    let speed = velocity.abs();
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
    let re = match rv.checked_div(viscosity) {
        Some(x) => x,
        None => return re_max,
    };
    match re.checked_mul(length) {
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
    let dt = (hot - cold).abs();
    let ha = match h.checked_mul(area) {
        Some(x) => x,
        None => return flux_max,
    };
    match ha.checked_mul(dt) {
        Some(x) => x.min(flux_max),
        None => flux_max,
    }
}

/// Hagen-Poiseuille laminar flow Q = pi*dP*r^4/(8*mu*L) (m^3/s). The driving pressure is bridged to
/// pascals and divided down before the four radius multiplies, so the underflowing r^4 shrinks an
/// already-reduced base; an underflow is the correct choked (zero) direction. A frictionless or
/// zero-length channel reads the cap; zero radius or pressure reads zero.
pub fn poiseuille_flow(dp: Fixed, radius: Fixed, viscosity: Fixed, length: Fixed, q_max: Fixed) -> Fixed {
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
    let mut b = match pa.checked_div(viscosity).and_then(|x| x.checked_div(length)).and_then(|x| x.checked_div(Fixed::from_int(8))) {
        Some(x) => x,
        None => return q_max,
    };
    for _ in 0..4 {
        b = match b.checked_mul(radius) {
            Some(x) => x,
            None => return ZERO,
        };
    }
    match b.checked_mul(Fixed::from_ratio(355, 113)) {
        Some(q) => q.min(q_max),
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

/// Ideal-gas density rho = P/(R_s*T) (kg/m^3), the coupling that lets the temperature field drive the
/// density field. The pressure is bridged to pascals. A zero or sub-floor R_s*T reads the dense cap.
pub fn ideal_gas_density(pressure: Fixed, temperature: Fixed, gas_constant: Fixed, rho_min: Fixed, rho_max: Fixed) -> Fixed {
    let pa = match pressure.checked_mul(C_PA) {
        Some(x) => x,
        None => return rho_max,
    };
    let rt = match gas_constant.checked_mul(temperature) {
        Some(x) => x,
        None => return rho_max,
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
/// up when the parcel is warmer, using the ideal-gas 1/T thermal expansion. Zero ambient reads zero.
pub fn thermal_buoyancy(t_parcel: Fixed, t_ambient: Fixed, gravity: Fixed, a_max: Fixed) -> Fixed {
    if t_ambient <= ZERO {
        return ZERO;
    }
    let dt = t_parcel - t_ambient;
    let ratio = match dt.checked_div(t_ambient) {
        Some(x) => x,
        None => return ZERO,
    };
    let lo = ZERO - a_max;
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
pub fn saturation_vapor_pressure(temperature: Fixed, slope: Fixed, t_ref: Fixed, e_ref: Fixed, es_cap: Fixed) -> Fixed {
    let dt = temperature - t_ref;
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
    (e_ref + term).clamp(ZERO, es_cap)
}

/// Evaporation mass flux E = (a + b*|u|)*(e_s - e_a) (kg/(m^2*s)), the Dalton bulk aerodynamic proxy.
/// Returns the evaporation source when the vapour-pressure deficit is positive; a non-positive deficit
/// is the condensation case and reads zero here (the sink is the caller's sign-flipped difference).
pub fn evaporation_rate(e_ambient: Fixed, e_saturation: Fixed, wind: Fixed, a_still: Fixed, b_wind: Fixed, e_max: Fixed) -> Fixed {
    let vpd = e_saturation - e_ambient;
    if vpd <= ZERO {
        return ZERO;
    }
    let wind_fn = match b_wind.checked_mul(wind.abs()) {
        Some(x) => a_still.saturating_add(x),
        None => a_still,
    };
    match wind_fn.checked_mul(vpd) {
        Some(e) => e.min(e_max),
        None => e_max,
    }
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
}
