// Copyright 2026 Nathan M. Fraske
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Retired organism and civilization law kernels plus the active abiotic law surface.

pub use civsim_physics_abiotic::laws::*;

use civsim_core::Fixed;

const ZERO: Fixed = Fixed::ZERO;
const ONE: Fixed = Fixed::ONE;
const R_SAT_N2: Fixed = Fixed::from_int(46340);
const R_SAT_N3: Fixed = Fixed::from_int(1290);

/// Compatibility name for the retired tool-use callers.
pub fn shear(
    shear_force: Fixed,
    shear_area: Fixed,
    independent_shear_strength: Option<Fixed>,
    yield_strength: Fixed,
    stress_max: Fixed,
) -> (Fixed, Fixed) {
    civsim_physics_abiotic::laws::shear_stress(
        shear_force,
        shear_area,
        independent_shear_strength,
        yield_strength,
        stress_max,
    )
}

/// Compatibility name for retired callers that predate the explicit force suffix.
pub fn weight(mass: Fixed, gravity: Fixed, force_max: Fixed) -> Fixed {
    civsim_physics_abiotic::laws::weight_force(mass, gravity, force_max)
}

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

/// Per-nutrient-class satisfaction in `[0, 1]`.
pub fn satisfaction(supply: Fixed, assimilation: Fixed, requirement: Option<Fixed>) -> Fixed {
    let req = match requirement {
        None => return ONE,
        Some(r) if r == ZERO => return ONE,
        Some(r) => r,
    };
    let num = match supply.checked_mul(assimilation) {
        Some(x) => x,
        None => return ONE,
    };
    match num.checked_div(req) {
        Some(s) => s.clamp(ZERO, ONE),
        None => ONE,
    }
}

/// The Liebig minimum across nutrient classes.
pub fn net_nutrition(classes: &[(Fixed, Fixed, Option<Fixed>)]) -> Fixed {
    classes
        .iter()
        .fold(ONE, |acc, &(s, a, r)| acc.min(satisfaction(s, a, r)))
}

/// Per-toxin-class integer-Hill harm.
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

/// The saturating sum of per-class harm.
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

/// The retired measured edibility tuple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edibility {
    pub net_nutrition: Fixed,
    pub net_harm: Fixed,
    pub margin: Fixed,
}

/// Compute the retired edibility tuple.
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

/// A retired monotone sensory response family.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ResponseLaw {
    Linear,
    Power,
    LogCompressive,
}

/// Transduce a received magnitude into an internal activation.
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

/// A retired sensory discrimination family.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum DiscriminationLaw {
    AbsoluteStep,
    WeberRelative,
}

/// Quantize an activation through the selected discrimination law.
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

/// Basal metabolic rate, `a * m^(3/4)`.
pub fn basal_metabolic_rate(mass: Fixed, coeff_a: Fixed, rate_max: Fixed) -> Fixed {
    if mass <= ZERO {
        return ZERO;
    }
    let root = mass.sqrt();
    let inner = match mass.checked_mul(root) {
        Some(x) => x,
        None => return rate_max,
    };
    let m34 = inner.sqrt();
    match coeff_a.checked_mul(m34) {
        Some(p) => p.min(rate_max),
        None => rate_max,
    }
}

/// Resting convective plus radiant heat loss.
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

/// Bridge resting power to the fraction of an energy reserve spent per tick.
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
        None => return ZERO,
    };
    if stored <= ZERO {
        return frac_max;
    }
    let spent = match power.checked_mul(tick_seconds) {
        Some(x) => x,
        None => return frac_max,
    };
    match spent.checked_div(stored) {
        Some(f) => f.clamp(ZERO, frac_max),
        None => frac_max,
    }
}

/// Reversible Michaelis-Menten uptake flux.
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
    let sh = stock.powf(hill);
    let kmh = km.powf(hill);
    let denom = kmh.saturating_add(sh);
    let saturation = if denom > ZERO {
        sh.checked_div(denom).unwrap_or(ZERO)
    } else {
        ZERO
    };
    let kt = boltzmann_k.checked_mul(temperature);
    let drive = match kt {
        Some(kt) if kt > ZERO && carrier_charge > ZERO => {
            let scaled = carrier_charge
                .checked_mul(emf)
                .and_then(|qe| qe.checked_div(kt));
            match scaled {
                Some(s) => ONE - (ZERO - s).exp(),
                None => ONE,
            }
        }
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
    raw.clamp(ZERO, stock)
}

/// Retired dependency-integration parse cost.
pub fn parse_cost(domain_extent: Fixed, memory_capacity: Fixed, cost_max: Fixed) -> Fixed {
    if domain_extent <= ZERO {
        return ZERO;
    }
    let den = domain_extent.saturating_add(memory_capacity.max(ZERO));
    if den <= ZERO {
        return cost_max.max(ZERO);
    }
    let frac = match domain_extent.checked_div(den) {
        Some(f) => f,
        None => return cost_max.max(ZERO),
    };
    match cost_max.checked_mul(frac) {
        Some(c) => c.clamp(ZERO, cost_max.max(ZERO)),
        None => cost_max.max(ZERO),
    }
}

/// Retired multiplicative harmony tilt.
pub fn harmony_tilt(cost_reduction: Fixed, temperature: Fixed, tilt_max: Fixed) -> Fixed {
    if cost_reduction <= ZERO {
        return ONE;
    }
    if temperature <= ZERO {
        return tilt_max.max(ONE);
    }
    let z = match cost_reduction.checked_div(temperature) {
        Some(z) => z,
        None => return tilt_max.max(ONE),
    };
    z.exp().clamp(ONE, tilt_max.max(ONE))
}
