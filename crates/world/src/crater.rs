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

//! The crater-scaling law, the ONE derived relation that turns an impact into a crater: how much mass a
//! strike excavates, how wide the bowl it opens, and how much of that mass is thrown out as ejecta. The
//! excavated ejecta mass is the source term the ballistic fan ([`crate::ballistic::ejecta_fan`]) launches and
//! the redistribution operator ([`crate::redistribute`]) then deposits, so this law is the front of the
//! impact chain and the ejecta blanket is its conservative back.
//!
//! The design is the coupling-parameter (point-source) scaling of Holsapple and Schmidt, the same
//! dimensionless-group framework the runout law uses one level down: the crater is a function of a few
//! dimensionless PI-GROUPS assembled from the impactor's own physical state and the target's own material
//! state, never of a named impact category or a Terran calibration. The groups are
//!
//!   pi2 = g a / U^2      the gravity-scaled size (how strongly gravity resists opening the bowl),
//!   pi3 = Y / (rho_t U^2) the strength ratio (how strongly the target's cohesion resists it),
//!   pi4 = rho_t / rho_i   the target-to-impactor density ratio,
//!
//! with `a` the impactor radius, `U` the impact speed, `g` the surface gravity, `Y` the target strength, and
//! `rho_t`, `rho_i` the target and impactor densities. The two named regimes, a big strengthless basin that
//! only gravity limits and a small crater that only cohesion limits, EMERGE as the two ends of one combined
//! group across the parameter space; the law never selects between them by reading a size or a KIND.
//!
//! The cratering efficiency (excavated mass over impactor mass, in the target's own density units) is
//!
//!   piV = rho_t V / m = K1 { pi2 pi4^p1 + [ K2 pi3 pi4^p2 ]^s }^(-3 mu / (2 + mu)),
//!
//! with the exponents `p1 = (6 nu - 2 - mu)/(3 mu)`, `p2 = (6 nu - 2)/(3 mu)`, `s = (2 + mu)/2` all built
//! from the two coupling exponents `mu` (the velocity coupling: the momentum limit is `mu = 1/3`, the energy
//! limit `mu = 2/3`, competent rock near `0.55`) and `nu` (the density coupling, near `0.4`). The crater
//! opens as a bowl of volume `V = piV m / rho_t`, and its rim diameter follows from that volume and the
//! transient bowl's depth-to-diameter ratio, `V = (pi/8) (h/D) D^3`, so `D = a cbrt( 32 piV / (3 pi4 (h/D)) )`
//! with the impactor mass never formed (the a^3 in the impactor's own volume cancels the volume's cube root),
//! which is what keeps the whole law inside the fixed-point range even for a planet-scale strike. The ejecta
//! that escapes the rim is a fraction of the excavated mass, so the ejecta-to-impactor mass ratio is
//! `f_eject piV`, a dimensionless number the wide-magnitude caller scales by its own impactor mass to feed
//! the fan.
//!
//! Admit-the-alien (a prime directive): every input is the impactor's or the target's own datum. An iron
//! bolide into an ice shell, a comet into a silicate crust, or a strike on a low-gravity mana world are each
//! a different set of numbers through the same law, not a new code path. Nothing about one chemistry, one
//! body plan, or one gravity is wired in; `g`, `Y`, and the densities are read, never assumed.
//!
//! The value-authoring line (Principle 11): the law is fixed Rust. The coupling constants (`mu`, `nu`, `K1`,
//! `K2`, the bowl aspect ratio, and the ejecta fraction) are PER-MATERIAL data, read as a [`CraterCoupling`]
//! the way the runout law reads its friction, never authored inline. Each is reserved for the owner's
//! calibration with its basis given on the [`CraterCoupling`] fields, cited to the coupling-parameter
//! literature (Holsapple 1993, Annu. Rev. Earth Planet. Sci. 21:333; Schmidt and Housen 1987; Melosh 1989,
//! Impact Cratering).
//!
//! Determinism (Principle 3, Principle 10): every quantity is fixed-point, the fractional powers are the
//! pinned [`Fixed::powf`] and [`Fixed::cbrt`], and the arithmetic is staged so no physical intermediate
//! reaches the rails, so the crater is a pure function of the impact and the material, worker-invariant. A
//! non-physical or degenerate input (a non-positive size, speed, or density, or a strengthless zero-gravity
//! target whose bowl is unbounded) returns `None`, the caller's signal, never a fabricated crater.

use civsim_core::Fixed;

/// The impactor's physical state at the strike, supplied as DATA (never a named bolide kind): its radius, its
/// speed, and its bulk density. The mass is deliberately NOT taken; the law works in the dimensionless
/// groups and returns the ejecta as a mass RATIO, so a planet-scale impactor whose mass overflows fixed point
/// still passes through. All three are per-event physical values the law reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Impactor {
    /// The impactor radius `a` (world length units). Its own volume's cube root cancels the crater volume's,
    /// so the radius, not the mass, is what sets the crater's absolute size.
    pub radius: Fixed,
    /// The impact speed `U` at the surface (world length over time units), the closing speed the encounter
    /// geometry and the world's gravity deliver.
    pub velocity: Fixed,
    /// The impactor bulk density `rho_i` (mass over length-cubed units), read for the density ratio `pi4`.
    pub density: Fixed,
}

/// The target's material and world state the crater law reads, each a floor axis or a per-world datum (never
/// authored here): the surface gravity that resists the excavation, the effective strength (cohesion) that
/// also resists it, and the target bulk density the efficiency is measured in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    /// The surface gravity `g` (`mech.gravitational_acceleration`, or a derived `g = G M / R^2`), the
    /// body force that limits a large bowl. A higher `g` opens a smaller crater at fixed impact.
    pub gravity: Fixed,
    /// The effective target strength `Y` (the material's cohesive yield, a pressure), the cohesion that
    /// limits a small crater. Zero is a strengthless (fully granular or fluid) target, the pure gravity
    /// regime; the law handles it as the vanishing strength term, not a special case.
    pub strength: Fixed,
    /// The target bulk density `rho_t` (mass over length-cubed units), the units the excavated mass and the
    /// efficiency `piV` are measured in.
    pub density: Fixed,
}

/// The per-material coupling constants the crater law reads (never authored inline): the two coupling
/// exponents and the fit coefficients of the point-source scaling. Each is reserved for the owner's
/// calibration with its basis, cited to the coupling-parameter literature (Holsapple 1993; Schmidt and Housen
/// 1987; Melosh 1989). A world's material carries its own row, so a soft ice, a competent basalt, and an
/// alien substrate differ by data, not by code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CraterCoupling {
    /// The velocity-coupling exponent `mu`. Basis: the material's momentum-to-energy coupling regime, bounded
    /// by the momentum limit `mu = 1/3` and the energy limit `mu = 2/3`; a competent silicate sits near
    /// `0.55`, dry sand near `0.41` (Holsapple 1993, Table 1). Reserved, not fabricated.
    pub velocity_exponent: Fixed,
    /// The density-coupling exponent `nu`, near `0.4`. Basis: the point-source density exponent of the
    /// coupling parameter (Holsapple 1993; Schmidt and Housen 1987). Reserved.
    pub density_exponent: Fixed,
    /// The efficiency coefficient `K1`, the intercept of `piV` against the combined coupling group. Basis:
    /// the material's fitted cratering-efficiency intercept, which also absorbs the `pi2` convention (radius
    /// versus diameter, the point-source geometric factor), so it is calibrated jointly with the group form
    /// (Holsapple 1993, Table 1). Reserved.
    pub efficiency_coefficient: Fixed,
    /// The strength-term coefficient `K2`, of order one. Basis: the fitted weight of the strength group in
    /// the combined coupling group (Holsapple 1993). Reserved.
    pub strength_coefficient: Fixed,
    /// The transient bowl's depth-to-diameter ratio `h/D`. Basis: the transient crater's aspect before
    /// collapse, near `0.2` to `0.3` for a simple bowl (Melosh 1989); it converts the excavated volume to a
    /// rim diameter. Reserved.
    pub bowl_aspect: Fixed,
    /// The escaping-ejecta fraction `f_eject`: the part of the excavated mass thrown beyond the rim, as
    /// opposed to the breccia that slumps back into the bowl. Basis: the ejecta-versus-fallback partition of
    /// the transient crater, near `0.4` to `0.5` (Melosh 1989). Reserved.
    pub eject_fraction: Fixed,
}

/// The derived crater: its excavation efficiency, its rim diameter, its bowl depth, and the mass ratio of the
/// ejecta the fan launches. The efficiency and the ejecta ratio are dimensionless (a wide-magnitude caller
/// multiplies the ejecta ratio by the impactor's own mass); the diameter and depth are in the impactor's
/// length units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crater {
    /// The cratering efficiency `piV = rho_t V / m`, the excavated mass in units of the impactor mass. Large
    /// for a small strengthless crater (an efficient dig), smaller for a big gravity-limited basin.
    pub efficiency: Fixed,
    /// The transient rim diameter `D`, in the impactor's length units.
    pub diameter: Fixed,
    /// The transient bowl depth `h = (h/D) D`, in the impactor's length units.
    pub depth: Fixed,
    /// The escaping-ejecta mass as a fraction of the impactor mass, `f_eject piV`. The wide-magnitude caller
    /// scales this by the impactor's own mass to get the mass the ballistic fan launches; keeping it a ratio
    /// is what lets a planet-scale impactor pass through fixed point.
    pub ejecta_mass_ratio: Fixed,
}

/// Solve the crater-scaling law for an impact of `impactor` into a `target` of the given material `coupling`.
/// Returns the derived [`Crater`], or `None` when an input is non-physical (a non-positive radius, speed, or
/// density) or the target is a degenerate strengthless zero-gravity body whose bowl is unbounded, or on any
/// fixed-point overflow. Deterministic and worker-invariant (the pinned [`Fixed::powf`] and [`Fixed::cbrt`],
/// staged so no physical intermediate rails).
pub fn crater(impactor: Impactor, target: Target, coupling: CraterCoupling) -> Option<Crater> {
    // Physical-domain guard: the groups need positive size, speed, and densities.
    if impactor.radius <= Fixed::ZERO
        || impactor.velocity <= Fixed::ZERO
        || impactor.density <= Fixed::ZERO
        || target.density <= Fixed::ZERO
    {
        return None;
    }
    if target.gravity < Fixed::ZERO || target.strength < Fixed::ZERO {
        return None;
    }

    let two = Fixed::from_int(2);
    let three = Fixed::from_int(3);
    let six = Fixed::from_int(6);
    let mu = coupling.velocity_exponent;
    let nu = coupling.density_exponent;
    if mu <= Fixed::ZERO {
        // The exponents divide by `3 mu`; a non-positive velocity coupling is not a physical material.
        return None;
    }

    // The dimensionless groups, each division staged so no intermediate leaves the fixed-point range (a
    // planetary impactor's g*a and Y/rho_t stay well inside the rails, and U^2 is never formed as one
    // product).
    // pi2 = g a / U^2.
    let pi2 = target
        .gravity
        .checked_mul(impactor.radius)?
        .checked_div(impactor.velocity)?
        .checked_div(impactor.velocity)?;
    // pi3 = Y / (rho_t U^2), as (Y / rho_t) / U / U.
    let pi3 = target
        .strength
        .checked_div(target.density)?
        .checked_div(impactor.velocity)?
        .checked_div(impactor.velocity)?;
    // pi4 = rho_t / rho_i.
    let pi4 = target.density.checked_div(impactor.density)?;

    // The coupling exponents, built from mu and nu (never authored as literals).
    let three_mu = three.checked_mul(mu)?;
    let six_nu_minus_two = six.checked_mul(nu)?.checked_sub(two)?;
    let p1 = six_nu_minus_two.checked_sub(mu)?.checked_div(three_mu)?;
    let p2 = six_nu_minus_two.checked_div(three_mu)?;
    let s = two.checked_add(mu)?.checked_div(two)?;
    // outer = -3 mu / (2 + mu).
    let outer = Fixed::ZERO
        .checked_sub(three_mu)?
        .checked_div(two.checked_add(mu)?)?;

    // The combined coupling group: the gravity term plus the strength term. `powf` guards a non-positive base
    // by returning zero, so a zero-strength target contributes a zero strength term (the pure gravity regime)
    // rather than a special case.
    let gravity_term = pi2.checked_mul(pi4.powf(p1))?;
    let strength_inner = coupling
        .strength_coefficient
        .checked_mul(pi3)?
        .checked_mul(pi4.powf(p2))?;
    let strength_term = strength_inner.powf(s);
    let group = gravity_term.checked_add(strength_term)?;
    if group <= Fixed::ZERO {
        // A strengthless zero-gravity target: nothing resists the excavation, the bowl is unbounded. Signal
        // it rather than fabricate a size.
        return None;
    }

    let efficiency = coupling
        .efficiency_coefficient
        .checked_mul(group.powf(outer))?;
    if efficiency <= Fixed::ZERO {
        return None;
    }

    // D = a * cbrt( 32 piV / (3 pi4 (h/D)) ). The impactor mass never appears: its own volume's a^3 cancels
    // the crater volume's cube root, leaving a dimensionless radicand times the impactor radius.
    let radicand = Fixed::from_int(32)
        .checked_mul(efficiency)?
        .checked_div(three)?
        .checked_div(pi4)?
        .checked_div(coupling.bowl_aspect)?;
    let diameter = impactor.radius.checked_mul(radicand.cbrt())?;
    let depth = coupling.bowl_aspect.checked_mul(diameter)?;
    let ejecta_mass_ratio = coupling.eject_fraction.checked_mul(efficiency)?;

    Some(Crater {
        efficiency,
        diameter,
        depth,
        ejecta_mass_ratio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // An illustrative competent-silicate coupling, the numbers cited to the coupling-parameter literature
    // (Holsapple 1993, Table 1; Melosh 1989) for a rock-like target. These are TEST fixtures standing in for
    // one material's reserved row, not authored floor values; the law reads them, it does not contain them.
    fn rock_coupling() -> CraterCoupling {
        CraterCoupling {
            velocity_exponent: Fixed::from_ratio(55, 100), // mu ~ 0.55, competent rock.
            density_exponent: Fixed::from_ratio(4, 10),    // nu ~ 0.4.
            efficiency_coefficient: Fixed::from_ratio(2, 10), // K1 ~ 0.2 (convention-absorbing intercept).
            strength_coefficient: Fixed::ONE,                 // K2 ~ 1.
            bowl_aspect: Fixed::from_ratio(2, 10),            // h/D ~ 0.2, a simple transient bowl.
            eject_fraction: Fixed::from_ratio(5, 10),         // f_eject ~ 0.5.
        }
    }

    // A Moon-like target: low gravity, modest rock strength, silicate density.
    fn moon_target() -> Target {
        Target {
            gravity: Fixed::from_ratio(162, 100),  // g ~ 1.62 m/s^2.
            strength: Fixed::from_int(10_000_000), // Y ~ 1e7 Pa, competent rock cohesion.
            density: Fixed::from_int(2500),        // rho_t ~ 2500 kg/m^3.
        }
    }

    // A kilometre-class stony impactor (radius 500 m) at 17 km/s.
    fn km_impactor() -> Impactor {
        Impactor {
            radius: Fixed::from_int(500),
            velocity: Fixed::from_int(17_000),
            density: Fixed::from_int(3000),
        }
    }

    #[test]
    fn a_kilometre_impactor_opens_a_crater_of_the_right_grade() {
        let c = crater(km_impactor(), moon_target(), rock_coupling())
            .expect("a physical impact resolves");
        // A ~1 km stony impactor on the Moon opens a crater tens of times its own radius (the canonical
        // crater-to-impactor size ratio ~ 20 to 40). Grade-level: the diameter is in the 5 to 60 km band and
        // the diameter-to-radius ratio is in [15, 60].
        let d = c.diameter.to_f64_lossy();
        assert!(
            (5_000.0..=60_000.0).contains(&d),
            "the crater diameter {d} m is in the kilometre-class grade band"
        );
        let ratio = d / 500.0;
        assert!(
            (15.0..=60.0).contains(&ratio),
            "the diameter-to-impactor-radius ratio {ratio} is the canonical crater grade"
        );
    }

    #[test]
    fn the_fixed_point_efficiency_tracks_an_independent_float_evaluation() {
        // A numerical twin: evaluate the SAME pi-scaling formula in f64 (an independent arithmetic path) and
        // confirm the fixed-point efficiency reproduces it. This validates the fixed-point staging and the
        // pinned ln/exp on the small pi2 argument; it does not (and cannot) validate the physics, which the
        // grade-band and monotonicity tests and the citation carry.
        let c = crater(km_impactor(), moon_target(), rock_coupling()).expect("resolves");
        let (a, u, rho_i) = (500.0_f64, 17_000.0_f64, 3000.0_f64);
        let (g, y, rho_t) = (1.62_f64, 1.0e7_f64, 2500.0_f64);
        let (mu, nu, k1, k2) = (0.55_f64, 0.4_f64, 0.2_f64, 1.0_f64);
        let pi2 = g * a / (u * u);
        let pi3 = y / (rho_t * u * u);
        let pi4 = rho_t / rho_i;
        let p1 = (6.0 * nu - 2.0 - mu) / (3.0 * mu);
        let p2 = (6.0 * nu - 2.0) / (3.0 * mu);
        let s = (2.0 + mu) / 2.0;
        let outer = -3.0 * mu / (2.0 + mu);
        let group = pi2 * pi4.powf(p1) + (k2 * pi3 * pi4.powf(p2)).powf(s);
        let piv = k1 * group.powf(outer);
        let got = c.efficiency.to_f64_lossy();
        let rel = (got - piv).abs() / piv;
        assert!(
            rel < 0.05,
            "the fixed-point efficiency {got} is within 5% of the float twin {piv} (rel {rel})"
        );
    }

    #[test]
    fn a_faster_impact_excavates_more() {
        let slow = crater(
            Impactor {
                velocity: Fixed::from_int(10_000),
                ..km_impactor()
            },
            moon_target(),
            rock_coupling(),
        )
        .expect("resolves");
        let fast = crater(
            Impactor {
                velocity: Fixed::from_int(30_000),
                ..km_impactor()
            },
            moon_target(),
            rock_coupling(),
        )
        .expect("resolves");
        assert!(
            fast.efficiency > slow.efficiency && fast.diameter > slow.diameter,
            "a faster strike digs a larger crater ({} vs {})",
            fast.diameter.to_f64_lossy(),
            slow.diameter.to_f64_lossy()
        );
    }

    #[test]
    fn stronger_gravity_and_stronger_rock_both_shrink_the_crater() {
        let base = crater(km_impactor(), moon_target(), rock_coupling()).expect("resolves");
        let high_g = crater(
            km_impactor(),
            Target {
                gravity: Fixed::from_ratio(981, 100), // Earth-like g resists the bowl.
                ..moon_target()
            },
            rock_coupling(),
        )
        .expect("resolves");
        let strong = crater(
            km_impactor(),
            Target {
                strength: Fixed::from_int(100_000_000), // 1e8 Pa, a much stronger target.
                ..moon_target()
            },
            rock_coupling(),
        )
        .expect("resolves");
        assert!(
            high_g.diameter < base.diameter,
            "higher gravity opens a smaller crater"
        );
        assert!(
            strong.diameter < base.diameter,
            "a stronger target opens a smaller crater"
        );
    }

    #[test]
    fn the_ejecta_ratio_is_the_eject_fraction_of_the_efficiency() {
        let c = crater(km_impactor(), moon_target(), rock_coupling()).expect("resolves");
        let expected = Fixed::from_ratio(5, 10).mul(c.efficiency);
        assert_eq!(
            c.ejecta_mass_ratio, expected,
            "the escaping ejecta is exactly f_eject * piV of the impactor mass"
        );
        // And the bowl geometry is self-consistent: depth = (h/D) * D.
        assert_eq!(c.depth, Fixed::from_ratio(2, 10).mul(c.diameter));
    }

    #[test]
    fn the_alien_is_a_data_row_not_a_new_path() {
        // An iron bolide into a soft ice shell on a low-gravity world: the same law, different numbers, a
        // finite crater. No Terran assumption blocks it.
        let iron_on_ice = crater(
            Impactor {
                radius: Fixed::from_int(200),
                velocity: Fixed::from_int(25_000),
                density: Fixed::from_int(7800), // iron impactor.
            },
            Target {
                gravity: Fixed::from_ratio(50, 100), // a small icy moon, g ~ 0.5.
                strength: Fixed::from_int(1_000_000), // soft ice, ~1e6 Pa.
                density: Fixed::from_int(920),       // water ice.
            },
            CraterCoupling {
                bowl_aspect: Fixed::from_ratio(25, 100),
                eject_fraction: Fixed::from_ratio(4, 10),
                ..rock_coupling()
            },
        )
        .expect("an alien impact still resolves");
        assert!(
            iron_on_ice.diameter > Fixed::ZERO && iron_on_ice.efficiency > Fixed::ZERO,
            "the alien crater is a finite derived crater"
        );
    }

    #[test]
    fn non_physical_and_degenerate_inputs_fail_soft() {
        // Non-positive size, speed, or density: no crater.
        assert!(crater(
            Impactor {
                radius: Fixed::ZERO,
                ..km_impactor()
            },
            moon_target(),
            rock_coupling()
        )
        .is_none());
        assert!(crater(
            Impactor {
                velocity: Fixed::ZERO,
                ..km_impactor()
            },
            moon_target(),
            rock_coupling()
        )
        .is_none());
        // A strengthless zero-gravity target: nothing bounds the bowl, so the law refuses rather than
        // fabricate a size.
        assert!(crater(
            km_impactor(),
            Target {
                gravity: Fixed::ZERO,
                strength: Fixed::ZERO,
                density: Fixed::from_int(2500),
            },
            rock_coupling()
        )
        .is_none());
    }
}
