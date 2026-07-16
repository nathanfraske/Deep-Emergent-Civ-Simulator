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

//! The MOON PRIMITIVES (task #75, the standalone first slice of the moon arc), the derive-first geometry every
//! branch of the moon dispatch shares, built ahead of the branch wiring as a DORMANT substrate: nothing here is
//! wired into a pinned run path, so the two run pins hold bit-exact. The moon arc is a three-branch dispatch
//! (circumplanetary-disk co-accretion, giant-impact debris, capture; see `docs/working/MOON_ARC_SCOPE.md`), and
//! all three branches close on the SAME tidal-survival filter: a candidate moon is kept only if it sits inside the
//! stable fraction of the planet's Hill radius, outside the Roche disruption radius, and does not recede past the
//! stability bound (or decay inside the Roche limit) over the system age. This module is that filter's geometry,
//! four pure fixed-point functions:
//!
//! - The HILL RADIUS `R_H = a * (M_planet / (3*M_star))^(1/3)`, the reach of the planet's gravity against the
//!   star's tide and the outer scale a bound moon must fit inside. It is delegated to the already-proven
//!   [`crate::astro::hill_radius_au`] (consume the banked machinery, never re-derive it): the mass ratio is folded
//!   once from the two cited reference anchors, and the fold reproduces Earth's ~0.0098 AU and Jupiter's ~0.355 AU
//!   without a fit.
//!
//! - The ROCHE LIMIT (rigid-body form) `d = R_planet * (2*rho_planet / rho_moon)^(1/3)`, the distance inside which
//!   the planet's tide overcomes a rigid moon's self-gravity and shears it apart, the inner floor of the survival
//!   band. The classical FLUID form `d = 2.44 * R_planet * (rho_planet / rho_moon)^(1/3)`, for a strengthless
//!   satellite that deforms into a tidal ellipsoid, is provided as the documented alternative
//!   [`roche_limit_fluid`]. For the Earth-Moon densities the rigid form gives ~1.49 Earth radii and the fluid form
//!   ~2.9 Earth radii, the two ends of the real disruption band (a solid moon survives closer in than a rubble
//!   pile); which end a survival filter reads is a per-moon rigidity question, so both are exposed.
//!
//! - The STABLE-ORBIT FRACTION of the Hill radius, [`stable_orbit_fraction`], the outer stability bound from the
//!   N-body fits of Domingos, Winter & Yokoyama (2006): ~0.4895 R_H for a prograde circular satellite and
//!   ~0.9309 R_H for a retrograde one (a retrograde moon is stable to roughly twice the prograde reach). The full
//!   eccentricity-aware fit [`stable_orbit_fraction_ecc`] and the semi-major-axis band [`stable_semimajor_axis`]
//!   carry the planet-eccentricity and satellite-eccentricity terms the fetch supplies, so a moon around an
//!   eccentric planet is a data row (at moderate planet eccentricity the retrograde bound falls toward ~0.7 R_H).
//!
//! - The TIDAL RECESSION RATE `da/dt = 3*(k2/Q)*(m/M)*(R/a)^5 * n * a`, the slow expansion (or, inside corotation,
//!   decay) of the orbit as the moon raises a tide on the planet, [`tidal_recession_rate`]. The Earth-Moon
//!   parameters reproduce the ~3.82 cm/yr present lunar recession anchor. `k2` (the planet's degree-2 tidal Love
//!   number) and `Q` (the tidal quality factor) are CALLER INPUTS, per-world data supplied at the survival
//!   filter's call site (reserved-with-basis there), never authored inside this kernel: a stiff dry planet, a
//!   dissipative ocean world, and a differentiated icy moon differ only in the `k2`/`Q` numbers passed in.
//!
//! Admit-the-alien (a prime directive): every physical input is a per-body datum on the argument list, so a denser
//! moon, a retrograde capture, a heavier star, a more dissipative planet, or a different body plan each flow
//! through the same law as different numbers, never a new code path. Determinism (Principle 3, Principle 10):
//! fixed-point throughout, the pinned [`Fixed::cbrt`], [`Fixed::ln`], and [`Fixed::exp`]; the wide-magnitude
//! recession rate (whose `(R/a)^5` factor is ~1e-9, near the Q32.32 floor, and would lose most of its bits in a
//! direct product) is assembled in LOG-SPACE and exponentiated once (the [`crate::astro::isolation_mass_earth`]
//! and [`crate::giants`] precedent), so no unrepresentable intermediate forms. A degenerate input (a non-positive
//! mass, radius, density, or a value past the representable range) fails soft to `None`, never a fabricated value.
//!
//! The value-authoring line (Principle 6). The only inline numbers in the kernels are the exact integers of the
//! standard algebra (the 2 and 3 of the Roche cube-root argument, the 3 and 5 of the recession form). The Domingos
//! stability fractions and their eccentricity coefficients (0.4895, 0.9309, 1.0305, 0.2738, 1.0764, 0.9812) and
//! the classical fluid Roche coefficient (2.44) are CITED literature values (PIPELINE_FETCHES.md sections 7 and 8),
//! carried as deserialized decimal strings through [`cited`], never authored. `k2` and `Q` are caller inputs.

use civsim_core::Fixed;

use crate::astro::hill_radius_au;

/// Round-trip a CITED literature constant (a Domingos, Winter & Yokoyama 2006 stability coefficient, or the
/// classical fluid Roche coefficient) from its published decimal text to `Fixed`. This is the deserialization
/// discipline the whole codebase uses for cited data: the number is a printed value carried as its exact string
/// and read losslessly to the fixed-point grid, never an authored inline constant. Every caller passes a
/// compile-time literal that appears verbatim at the use site, so the cited value stays auditable there. It
/// `expect`s a well-formed literal, which the call sites are by construction (a malformed-literal test guards it).
fn cited(decimal: &str) -> Fixed {
    Fixed::from_decimal_str(decimal)
        .expect("a cited literature constant is a well-formed decimal string")
}

/// The HILL RADIUS in AU, `R_H = a * (M_planet / (3*M_star))^(1/3)`: the reach of the planet's own gravity against
/// the star's tide, and the outer scale within which a moon can stay bound. `a_au` the planet's orbit, the moon's
/// planet mass `m_planet_earth` in Earth masses, `m_star_solar` the star mass in solar masses. Delegated to the
/// proven [`crate::astro::hill_radius_au`] so there is one source of truth for the mass-ratio fold (the two cited
/// reference anchors [`crate::astro::EARTH_MASS_KG`] / [`crate::astro::SOLAR_MASS_KG`], folded once): the shared
/// primitive the moon branches read is the same ruler the embryo field and the planet spacing are built on. `None`
/// on a non-positive input (fail-soft, never a fabricated radius).
pub fn hill_radius(a_au: Fixed, m_planet_earth: Fixed, m_star_solar: Fixed) -> Option<Fixed> {
    hill_radius_au(a_au, m_planet_earth, m_star_solar)
}

/// The ROCHE LIMIT (rigid-body form) in the length unit of `r_planet`,
/// `d = R_planet * (2*rho_planet / rho_moon)^(1/3)`: the distance inside which the planet's tidal field overcomes
/// a rigid, self-gravitating moon's own gravity and pulls it apart, the inner floor of the survival band. A moon
/// derived (or captured) inside this radius is disrupted rather than retained. `rho_moon` the moon bulk density,
/// `rho_planet` the planet bulk density (any consistent density unit; only their ratio enters), `r_planet` the
/// planet radius (the output carries its length unit).
///
/// This is the RIGID form (a moon held together by material strength, deforming little). The classical FLUID form
/// (a strengthless moon that deforms into a tidal ellipsoid) is [`roche_limit_fluid`], ~1.64x larger. For the
/// Earth-Moon densities (5514 vs 3344 kg/m^3) the rigid form gives ~1.49 Earth radii (~9480 km); the ~2.9 Earth
/// radii figure often quoted for the Earth-Moon Roche limit is the FLUID value. `None` on a non-positive density
/// or radius, or a value past the representable range.
pub fn roche_limit(rho_moon: Fixed, rho_planet: Fixed, r_planet: Fixed) -> Option<Fixed> {
    if rho_moon <= Fixed::ZERO || rho_planet <= Fixed::ZERO || r_planet <= Fixed::ZERO {
        return None;
    }
    // (2 * rho_planet / rho_moon)^(1/3), the density contrast under the exact cube root; the 2 and 3 are the
    // standard algebra of the rigid Roche derivation, not authored parameters.
    let density_argument = Fixed::from_int(2)
        .checked_mul(rho_planet)?
        .checked_div(rho_moon)?;
    if density_argument <= Fixed::ZERO {
        return None;
    }
    r_planet.checked_mul(density_argument.cbrt())
}

/// The classical fluid Roche coefficient ~2.44 (a strengthless satellite deforming into a tidal ellipsoid raises
/// the disruption radius over the rigid case), a CITED standard value read through [`cited`].
fn fluid_roche_coefficient() -> Fixed {
    cited("2.44")
}

/// The ROCHE LIMIT (fluid-body form) in the length unit of `r_planet`,
/// `d = 2.44 * R_planet * (rho_planet / rho_moon)^(1/3)`: the disruption radius for a strengthless moon that a
/// planet's tide deforms into an elongated ellipsoid before shearing it apart, the OUTER end of the real
/// disruption band and the documented alternative to the rigid [`roche_limit`]. For the Earth-Moon densities this
/// gives ~2.9 Earth radii (~18300 km), the commonly quoted Earth-Moon Roche value. A survival filter reads the
/// rigid form for a coherent solid moon and this form for a rubble pile; which applies is a per-moon rigidity
/// question, so both ends are exposed. `None` on a non-positive density or radius, or an out-of-range result.
pub fn roche_limit_fluid(rho_moon: Fixed, rho_planet: Fixed, r_planet: Fixed) -> Option<Fixed> {
    if rho_moon <= Fixed::ZERO || rho_planet <= Fixed::ZERO || r_planet <= Fixed::ZERO {
        return None;
    }
    let density_ratio = rho_planet.checked_div(rho_moon)?;
    if density_ratio <= Fixed::ZERO {
        return None;
    }
    fluid_roche_coefficient()
        .checked_mul(r_planet)?
        .checked_mul(density_ratio.cbrt())
}

/// The prograde and retrograde CIRCULAR-orbit stability fractions of the Hill radius, the lead coefficients of the
/// Domingos, Winter & Yokoyama (2006) fits (PIPELINE_FETCHES.md section 7), cited as decimal strings.
fn prograde_circular_fraction() -> Fixed {
    cited("0.4895")
}
fn retrograde_circular_fraction() -> Fixed {
    cited("0.9309")
}

/// The STABLE-ORBIT FRACTION of the Hill radius for a circular satellite orbit: the fraction of `R_H` out to which
/// a satellite's orbit is stable against the star's perturbation, from the N-body fits of Domingos, Winter &
/// Yokoyama (2006). `prograde` selects the sense: ~0.4895 R_H for a prograde satellite and ~0.9309 R_H for a
/// retrograde one, so a retrograde moon is stable to roughly twice the prograde reach (the standard
/// prograde/retrograde asymmetry; a retrograde captured moon like Triton can sit far out). These are cited
/// literature constants for zero eccentricity; [`stable_orbit_fraction_ecc`] carries the eccentricity terms.
pub fn stable_orbit_fraction(prograde: bool) -> Fixed {
    if prograde {
        prograde_circular_fraction()
    } else {
        retrograde_circular_fraction()
    }
}

/// The Domingos (2006) eccentricity coefficients: the prograde fit is
/// `0.4895 * (1 - 1.0305*e_planet - 0.2738*e_sat)` and the retrograde fit
/// `0.9309 * (1 - 1.0764*e_planet - 0.9812*e_sat)` (PIPELINE_FETCHES.md section 7), each coefficient cited.
fn prograde_e_planet_coeff() -> Fixed {
    cited("1.0305")
}
fn prograde_e_sat_coeff() -> Fixed {
    cited("0.2738")
}
fn retrograde_e_planet_coeff() -> Fixed {
    cited("1.0764")
}
fn retrograde_e_sat_coeff() -> Fixed {
    cited("0.9812")
}

/// The STABLE-ORBIT FRACTION of the Hill radius carrying the ECCENTRICITY terms of the Domingos, Winter &
/// Yokoyama (2006) fit: `a_crit / R_H = f0 * (1 - c_p*e_planet - c_s*e_sat)`, with `(f0, c_p, c_s)` the cited
/// prograde `(0.4895, 1.0305, 0.2738)` or retrograde `(0.9309, 1.0764, 0.9812)`. `e_planet` is the planet's
/// heliocentric eccentricity and `e_sat` the satellite's own eccentricity; both raise the star's reach and shrink
/// the stable region, so an eccentric planet holds moons less far out (at moderate `e_planet` the retrograde bound
/// falls toward ~0.7 R_H, as the fetch notes). Reduces to [`stable_orbit_fraction`] at zero eccentricity. `None`
/// on a negative eccentricity or when the bracket falls to zero or below (the linear fit no longer marks a stable
/// region, so no bound is reported rather than a fabricated negative one).
pub fn stable_orbit_fraction_ecc(prograde: bool, e_planet: Fixed, e_sat: Fixed) -> Option<Fixed> {
    if e_planet < Fixed::ZERO || e_sat < Fixed::ZERO {
        return None;
    }
    let (base, c_planet, c_sat) = if prograde {
        (
            prograde_circular_fraction(),
            prograde_e_planet_coeff(),
            prograde_e_sat_coeff(),
        )
    } else {
        (
            retrograde_circular_fraction(),
            retrograde_e_planet_coeff(),
            retrograde_e_sat_coeff(),
        )
    };
    let bracket = Fixed::ONE
        .checked_sub(c_planet.checked_mul(e_planet)?)?
        .checked_sub(c_sat.checked_mul(e_sat)?)?;
    if bracket <= Fixed::ZERO {
        return None;
    }
    base.checked_mul(bracket)
}

/// The outer STABLE SEMI-MAJOR-AXIS BOUND for a satellite, in the length unit of `r_hill`: the Domingos (2006)
/// stable fraction (with its eccentricity terms) applied to the planet's Hill radius, `a_crit = R_H * fraction`.
/// A moon must sit inside this bound to survive the star's perturbation; combined with the [`roche_limit`] inner
/// floor it is the orbital-stability half of the tidal-survival filter. `r_hill` from [`hill_radius`], `prograde`
/// the orbit sense, `e_planet` and `e_sat` the eccentricities. `None` on a non-positive Hill radius, a negative
/// eccentricity, or an eccentricity so large the stable region vanishes.
pub fn stable_semimajor_axis(
    r_hill: Fixed,
    prograde: bool,
    e_planet: Fixed,
    e_sat: Fixed,
) -> Option<Fixed> {
    if r_hill <= Fixed::ZERO {
        return None;
    }
    let fraction = stable_orbit_fraction_ecc(prograde, e_planet, e_sat)?;
    r_hill.checked_mul(fraction)
}

/// The TIDAL RECESSION RATE `da/dt = 3*(k2/Q)*(m_moon/M_planet)*(R_planet/a)^5 * n * a` (Murray & Dermott 1999,
/// Solar System Dynamics ch. 4; PIPELINE_FETCHES.md section 8): the rate at which a moon's orbit expands as the
/// moon raises a tide on the planet and the planet's spin drags that bulge ahead of the moon, torquing it outward.
/// The Earth-Moon parameters reproduce the ~3.82 cm/yr present lunar recession anchor. The same form runs in
/// reverse (a decaying orbit) when the moon orbits inside the planet's corotation radius, so the sign of the
/// physical evolution is the caller's to read from the corotation comparison; this kernel returns the magnitude.
///
/// `k2` (the planet's degree-2 tidal Love number) and `q_factor` (the tidal quality factor `Q`) are CALLER
/// INPUTS, per-world data (the planet's rigidity and dissipation) supplied and reserved-with-basis at the survival
/// filter's call site, never authored here. The masses `m_moon` and `M_planet` may be in any consistent mass unit
/// (only their ratio enters); `r_planet` and `a` in any consistent length unit; `mean_motion` the orbital mean
/// motion `n` (radians per unit time, `n = sqrt(G*M_planet/a^3)`, which the caller derives so this kernel stays
/// free of `G`). The result is in the length unit of `a` per the time unit of `mean_motion` (pass `a` in metres
/// and `n` in radians per year to read metres per year). The caller should choose units so the rate lands in the
/// representable range rather than near the fixed-point floor.
///
/// The product spans many decades (`(R/a)^5` is ~1e-9 for the Moon, near the Q32.32 floor), so the rate is
/// assembled in LOG-SPACE and exponentiated once (the [`crate::astro::isolation_mass_earth`] precedent), so the
/// small factor never underflows. `None` on a non-positive input or a value past the representable exp ceiling.
pub fn tidal_recession_rate(
    k2: Fixed,
    q_factor: Fixed,
    m_moon: Fixed,
    m_planet: Fixed,
    r_planet: Fixed,
    a: Fixed,
    mean_motion: Fixed,
) -> Option<Fixed> {
    if k2 <= Fixed::ZERO
        || q_factor <= Fixed::ZERO
        || m_moon <= Fixed::ZERO
        || m_planet <= Fixed::ZERO
        || r_planet <= Fixed::ZERO
        || a <= Fixed::ZERO
        || mean_motion <= Fixed::ZERO
    {
        return None;
    }
    // ln(da/dt) = ln 3 + ln(k2) - ln(Q) + ln(m_moon) - ln(M_planet) + 5*(ln R_planet - ln a) + ln(n) + ln(a).
    // Every factor enters as its log, so the (R/a)^5 factor (~1e-9) is carried precisely rather than underflowing
    // a direct fixed-point product; the 3 and 5 are the standard algebra of the tidal form.
    let radius_ratio_log = r_planet.ln().checked_sub(a.ln())?;
    let ln_rate = Fixed::from_int(3)
        .ln()
        .checked_add(k2.ln())?
        .checked_sub(q_factor.ln())?
        .checked_add(m_moon.ln())?
        .checked_sub(m_planet.ln())?
        .checked_add(Fixed::from_int(5).checked_mul(radius_ratio_log)?)?
        .checked_add(mean_motion.ln())?
        .checked_add(a.ln())?;
    // Fail loud past the representable exp ceiling rather than let `exp` saturate (the astro/giants log-space
    // precedent): ln(2^31) = 31*ln 2 is the log of the representation's own maximum, an engine bound.
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_rate >= ln_ceiling {
        return None;
    }
    Some(ln_rate.exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    /// The Hill radius primitive reproduces Earth's real ~0.0098 AU (~1.5e6 km), the shared ruler the moon
    /// branches read, delegated to the proven astro fold. `None` on a non-positive input (fail-soft).
    #[test]
    fn the_hill_radius_matches_earth() {
        let earth = hill_radius(Fixed::ONE, Fixed::ONE, Fixed::ONE).unwrap();
        assert!(
            (earth.to_f64_lossy() - 0.0098).abs() < 0.0005,
            "Earth's Hill radius ~0.0098 AU (~1.5e6 km), got {}",
            earth.to_f64_lossy()
        );
        // ~0.0098 AU * 1.496e8 km/AU ~ 1.47e6 km, the ~1.5e6 km the task names.
        let km = earth.to_f64_lossy() * 1.495_978_707e8;
        assert!((km - 1.5e6).abs() < 1.0e5, "~1.5e6 km, got {} km", km);
        assert!(hill_radius(Fixed::ZERO, Fixed::ONE, Fixed::ONE).is_none());
        assert!(hill_radius(Fixed::ONE, Fixed::ONE, Fixed::ZERO).is_none());
    }

    /// The RIGID Roche limit reproduces the Earth-Moon rigid disruption radius of ~1.49 Earth radii (~9480 km):
    /// `R_E * (2*rho_E/rho_Moon)^(1/3)` with rho_E = 5514, rho_Moon = 3344 kg/m^3, R_E = 6.371e6 m. This is the
    /// rigid-body form the task specifies; the ~2.9 Earth radii figure the task's anchor names is the FLUID value
    /// (checked in the sibling test below), a labelling caught and surfaced, not forced out of the rigid formula.
    #[test]
    fn the_rigid_roche_limit_matches_the_earth_moon_anchor() {
        let r_earth_m = Fixed::from_int(6_371_000);
        let d = roche_limit(
            Fixed::from_int(3344), // rho_moon, kg/m^3
            Fixed::from_int(5514), // rho_planet, kg/m^3
            r_earth_m,
        )
        .unwrap();
        let in_earth_radii = d.to_f64_lossy() / 6_371_000.0;
        assert!(
            (in_earth_radii - 1.49).abs() < 0.03,
            "rigid Earth-Moon Roche ~1.49 R_Earth, got {} R_Earth ({} m)",
            in_earth_radii,
            d.to_f64_lossy()
        );
        assert!(roche_limit(Fixed::ZERO, Fixed::from_int(5514), r_earth_m).is_none());
    }

    /// The FLUID Roche limit reproduces the ~2.9 Earth radii (~18300 km) commonly quoted for the Earth-Moon, and
    /// is strictly larger than the rigid form (a strengthless moon is torn apart farther out than a coherent one).
    /// This is where the task's "~2.9 Earth radii" anchor comes from: the fluid form, not the rigid one.
    #[test]
    fn the_fluid_roche_limit_matches_the_2_9_earth_radii_figure() {
        let r_earth_m = Fixed::from_int(6_371_000);
        let rho_moon = Fixed::from_int(3344);
        let rho_planet = Fixed::from_int(5514);
        let fluid = roche_limit_fluid(rho_moon, rho_planet, r_earth_m).unwrap();
        let in_earth_radii = fluid.to_f64_lossy() / 6_371_000.0;
        assert!(
            (in_earth_radii - 2.9).abs() < 0.05,
            "fluid Earth-Moon Roche ~2.9 R_Earth, got {} R_Earth ({} m)",
            in_earth_radii,
            fluid.to_f64_lossy()
        );
        let rigid = roche_limit(rho_moon, rho_planet, r_earth_m).unwrap();
        assert!(
            fluid > rigid,
            "the fluid limit ({}) exceeds the rigid limit ({})",
            fluid.to_f64_lossy(),
            rigid.to_f64_lossy()
        );
    }

    /// The Domingos stable-orbit fractions: the prograde circular fraction (~0.4895) is BELOW the retrograde
    /// (~0.9309), the ordering that says a retrograde moon is stable to roughly twice the prograde reach. The
    /// cited values themselves are checked against the fetch to a tight tolerance.
    #[test]
    fn the_prograde_stable_fraction_is_below_the_retrograde() {
        let prograde = stable_orbit_fraction(true);
        let retrograde = stable_orbit_fraction(false);
        assert!(
            prograde < retrograde,
            "prograde ({}) below retrograde ({})",
            prograde.to_f64_lossy(),
            retrograde.to_f64_lossy()
        );
        assert!(
            (prograde.to_f64_lossy() - 0.4895).abs() < 1e-4,
            "prograde fraction ~0.4895, got {}",
            prograde.to_f64_lossy()
        );
        assert!(
            (retrograde.to_f64_lossy() - 0.9309).abs() < 1e-4,
            "retrograde fraction ~0.9309, got {}",
            retrograde.to_f64_lossy()
        );
    }

    /// The eccentricity terms shrink the stable region, and reduce to the circular fraction at zero eccentricity.
    /// A moderate planet eccentricity pulls the retrograde bound down toward ~0.7 R_H, the fetch's noted behaviour.
    #[test]
    fn the_eccentricity_terms_shrink_the_stable_fraction() {
        let circular = stable_orbit_fraction_ecc(false, Fixed::ZERO, Fixed::ZERO).unwrap();
        assert_eq!(
            circular,
            stable_orbit_fraction(false),
            "zero eccentricity recovers the circular fraction"
        );
        // Retrograde at planet eccentricity 0.2: 0.9309*(1 - 1.0764*0.2) ~ 0.9309*0.7847 ~ 0.7305.
        let eccentric = stable_orbit_fraction_ecc(false, r(2, 10), Fixed::ZERO).unwrap();
        assert!(
            eccentric < circular,
            "eccentricity shrinks the bound ({} < {})",
            eccentric.to_f64_lossy(),
            circular.to_f64_lossy()
        );
        assert!(
            (eccentric.to_f64_lossy() - 0.73).abs() < 0.02,
            "moderate planet eccentricity pulls the retrograde bound toward ~0.73 R_H, got {}",
            eccentric.to_f64_lossy()
        );
        // A large eccentricity that erases the stable region reports no bound rather than a negative one.
        assert!(stable_orbit_fraction_ecc(true, Fixed::ONE, Fixed::ONE).is_none());
        assert!(stable_orbit_fraction_ecc(true, r(-1, 10), Fixed::ZERO).is_none());
    }

    /// The stable-semi-major-axis band applies the fraction to the Hill radius. For Earth (R_H ~ 0.0098 AU) the
    /// prograde circular bound sits at ~0.48 of that (~0.0048 AU), inside the Hill radius as it must.
    #[test]
    fn the_stable_semimajor_axis_is_a_fraction_of_the_hill_radius() {
        let r_hill = hill_radius(Fixed::ONE, Fixed::ONE, Fixed::ONE).unwrap();
        let band = stable_semimajor_axis(r_hill, true, Fixed::ZERO, Fixed::ZERO).unwrap();
        assert!(
            band < r_hill,
            "the stable bound sits inside the Hill radius"
        );
        let ratio = band.to_f64_lossy() / r_hill.to_f64_lossy();
        assert!(
            (ratio - 0.4895).abs() < 1e-3,
            "the prograde circular band is ~0.4895 R_H, got {}",
            ratio
        );
        assert!(stable_semimajor_axis(Fixed::ZERO, true, Fixed::ZERO, Fixed::ZERO).is_none());
    }

    /// The tidal recession rate reproduces the ~3.82 cm/yr present lunar recession from the Earth-Moon parameters
    /// (k2 ~ 0.30, effective Q ~ 12, m/M ~ 0.0123, R_Earth = 6.371e6 m, a = 3.844e8 m, n = mean motion). Passing
    /// `a` in metres and `n` in radians per year returns metres per year; the model lands ~3.73 cm/yr with the
    /// round effective k2/Q anchors, inside the anchor's own precision band around the measured 3.82 cm/yr.
    #[test]
    fn the_recession_rate_reproduces_the_lunar_anchor() {
        let k2 = r(30, 100); // Earth effective Love number ~0.30 (fetch)
        let q = Fixed::from_int(12); // Earth effective Q ~12 (ocean-dominated, fetch)
        let m_moon = r(123, 10000); // M_Moon/M_Earth ~0.0123 (Earth-mass unit)
        let m_planet = Fixed::ONE; // 1 Earth mass
        let r_planet = Fixed::from_int(6_371_000); // R_Earth, metres
        let a = Fixed::from_int(384_400_000); // Earth-Moon distance, metres
        let n = r(8400, 100); // mean motion ~84.0 rad/yr (2*pi / sidereal month in years)
        let rate_m_per_yr = tidal_recession_rate(k2, q, m_moon, m_planet, r_planet, a, n).unwrap();
        let cm_per_yr = rate_m_per_yr.to_f64_lossy() * 100.0;
        assert!(
            (3.4..=4.2).contains(&cm_per_yr),
            "Earth-Moon recession ~3.82 cm/yr (model ~3.73 with the round k2/Q anchors), got {} cm/yr",
            cm_per_yr
        );
    }

    /// The recession rate rises with `k2/Q` (a more dissipative planet drives the moon out faster) and falls
    /// with the orbit (the `(R/a)^5 * a` = `R^5/a^4` distance dependence), the levers the survival filter reads
    /// over the system age.
    #[test]
    fn the_recession_rate_moves_with_dissipation_and_distance() {
        let k2 = r(30, 100);
        let q = Fixed::from_int(12);
        let m_moon = r(123, 10000);
        let m_planet = Fixed::ONE;
        let r_planet = Fixed::from_int(6_371_000);
        let a = Fixed::from_int(384_400_000);
        let n = r(8400, 100);
        let base = tidal_recession_rate(k2, q, m_moon, m_planet, r_planet, a, n).unwrap();
        // Halving Q (twice as dissipative) drives faster recession.
        let dissipative =
            tidal_recession_rate(k2, Fixed::from_int(6), m_moon, m_planet, r_planet, a, n).unwrap();
        assert!(
            dissipative > base,
            "a more dissipative planet drives faster recession ({} > {})",
            dissipative.to_f64_lossy(),
            base.to_f64_lossy()
        );
        // A moon at twice the distance recedes far slower (the mean motion is held for isolation of the R/a and a
        // factors; physically n would also fall, deepening the drop).
        let farther = tidal_recession_rate(
            k2,
            q,
            m_moon,
            m_planet,
            r_planet,
            Fixed::from_int(768_800_000),
            n,
        )
        .unwrap();
        assert!(
            farther < base,
            "a farther moon recedes slower ({} < {})",
            farther.to_f64_lossy(),
            base.to_f64_lossy()
        );
    }

    /// Determinism (Principle 3): every primitive returns the same bits on repeat, and a degenerate input fails
    /// soft to `None` rather than a fabricated value.
    #[test]
    fn the_primitives_are_deterministic_and_fail_soft() {
        let a = tidal_recession_rate(
            r(30, 100),
            Fixed::from_int(12),
            r(123, 10000),
            Fixed::ONE,
            Fixed::from_int(6_371_000),
            Fixed::from_int(384_400_000),
            r(8400, 100),
        );
        let b = tidal_recession_rate(
            r(30, 100),
            Fixed::from_int(12),
            r(123, 10000),
            Fixed::ONE,
            Fixed::from_int(6_371_000),
            Fixed::from_int(384_400_000),
            r(8400, 100),
        );
        assert_eq!(a, b);
        assert_eq!(
            roche_limit(r(3, 1), r(5, 1), Fixed::from_int(6_371_000)),
            roche_limit(r(3, 1), r(5, 1), Fixed::from_int(6_371_000))
        );
        // Fail-soft on non-positive inputs across the family.
        assert!(roche_limit(Fixed::ZERO, Fixed::ONE, Fixed::ONE).is_none());
        assert!(roche_limit_fluid(Fixed::ONE, Fixed::ZERO, Fixed::ONE).is_none());
        assert!(tidal_recession_rate(
            Fixed::ZERO,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE
        )
        .is_none());
    }

    /// The cited literature constants parse from their exact decimal text (the deserialization discipline): the
    /// `cited` reader never panics on a well-formed value, and the six Domingos coefficients plus the fluid Roche
    /// coefficient land where the fetch prints them.
    #[test]
    fn the_cited_constants_parse_to_their_printed_values() {
        assert!((prograde_circular_fraction().to_f64_lossy() - 0.4895).abs() < 1e-4);
        assert!((retrograde_circular_fraction().to_f64_lossy() - 0.9309).abs() < 1e-4);
        assert!((prograde_e_planet_coeff().to_f64_lossy() - 1.0305).abs() < 1e-4);
        assert!((prograde_e_sat_coeff().to_f64_lossy() - 0.2738).abs() < 1e-4);
        assert!((retrograde_e_planet_coeff().to_f64_lossy() - 1.0764).abs() < 1e-4);
        assert!((retrograde_e_sat_coeff().to_f64_lossy() - 0.9812).abs() < 1e-4);
        assert!((fluid_roche_coefficient().to_f64_lossy() - 2.44).abs() < 1e-4);
    }
}
