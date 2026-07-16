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

//! THE LID GEOTHERM: temperature on a DEPTH axis, `T(z)`, through the conductive lid.
//!
//! WHY THIS EXISTS. The flexural rigidity `D = E T_e^3 / (12 (1 - nu^2))` fills the ~100 to 1000 km relief band
//! between the convective provinces and the crater rows: mountain belts, foreland basins, volcano moats,
//! peripheral bulges. `E` and `nu` derive from the banked elastic moduli, `g` and the densities derive, and
//! `T_e`, the elastic lid thickness, was the SOLE unsupplied input. Until it derives, the middle band is empty
//! and a rendered world is broad province swells plus tiny crater specks with nothing in between. `T_e` cannot
//! be located without a temperature profile against depth, and this module is that profile.
//!
//! THE DEFECT THIS MODULE CLOSES, recorded because the rule it produced binds everything: the coarseness ruling
//! asserted "the derived elastic lid thickness out of the thermal state", the scope documents propagated that
//! sentence, and `flexure.rs` carried it verbatim. No derived elastic lid ever existed. `ColumnState` is
//! `{temperature, convecting}`, one LUMPED scalar per column, and nothing in the engine carried temperature on
//! a depth axis, so a mechanical lid base could not be located against a profile that was not there. The claim
//! passed every review because its verb was "derives from" rather than "wire" or "route". The premise line now
//! covers the whole class: any claim that X is already derived, carried, or owned by the engine is an
//! IMPLEMENTATION-STATUS claim and is verified against code state.
//!
//! NO NEW STATE AXIS. `T(z)` is a DERIVED EVALUATOR, never stored data. `ColumnState` stays
//! `{temperature, convecting}`. The three quantities that pin the profile already exist: the surface
//! temperature (the cold reference the column loses heat toward), the column's lumped scalar (which IS the
//! convecting interior's potential temperature), and the lid thickness between them
//! ([`crate::laws::thermal_boundary_layer`], `L = d * Ra^(-1/3)`, the SAME derivation the convective driving
//! stress shears over, so the stress and the geotherm cannot disagree about how thick the lid is).
//!
//! TWO FORMS, dispatched on what the world's lid is rather than on a named regime:
//! [`halfspace_geotherm`] where a lid has an AGE (it cools from the surface down, the profile a growing erf),
//! and [`steady_conductive_geotherm`] where it does not (a stagnant lid in equilibrium between the surface
//! above and the convecting interior below, with its own radiogenic heat bending the profile). A stagnant-lid
//! world has no plate ages to read, so it takes the steady form; a mobile-lid world's plates carry ages and
//! take the half-space form. Neither is selected by an authored threshold.
//!
//! ADMIT THE ALIEN. Every input is a per-world datum or a derived quantity, so an ice shell is a DATA ROW
//! rather than a rewrite. The conductivity is where the classes truly differ, and the difference is NOT
//! presence-versus-absence of temperature dependence, which is a framing this arc carried until the fetch
//! corrected it. LATTICE CONDUCTIVITY DECLINES AS 1/T IN BOTH silicate and ice, from the same phonon Umklapp
//! scattering, and that shared physics is precisely why the SHAPE was derivable before any constant arrived.
//! The per-class difference is the ANCHOR CONSTANTS plus a RADIATIVE RECOVERY that silicates gain above roughly
//! 1200 to 1500 K and ice has no analogue for. Ice's measured row is `k = 612/T` over 30 to 273 K (Carnahan
//! 2021): the 1/T form CONFIRMED by a fetch that could only confirm or refute it, because the shape was fixed
//! before the number had a home.
//!
//! THE CLASS-KEYED ROW DOES NOT EXIST YET (`ColumnParams::thermal_conductivity` is a single scalar), and the
//! convicting instance is sharper than one world against another: it is WITHIN ONE SHELL. The 1/T law means the
//! cold top of an icy shell conducts several times better than its warm base, so even an honest PER-MATERIAL
//! CONSTANT fails inside a single Europa, and the shell-thickness verdicts this arc feeds are exactly what that
//! error would move. This module reads `k` as the caller's per-world datum rather than reaching for a rock
//! value, so `k(material, T)` lands as data with no change to any function here.
//!
//! REGOLITH IS A DIFFERENT ROW, kept apart deliberately: its conductivity is orders of magnitude lower under
//! vacuum, and it is scoped to the DIURNAL THERMAL SKIN rather than the lithospheric geotherm. Airless-surface
//! physics must never leak into `T_e`.
//!
//! WHAT THIS MODULE DOES NOT DO. It does not derive `T_e`. `T_e` falls out of the YIELD-STRENGTH ENVELOPE, the
//! brittle curve (Byerlee, pressure-dependent) meeting the ductile curve (creep, on the world's own derived
//! strain rate), and both branches are later steps in this arc.
//!
//! THE HINDCAST TARGET IS A DATASET, NEVER A SUMMARY STATISTIC (owner ruling, after the limiting isotherm was
//! fetched and did not survive contact with its own sources). The derived Earth `T_e` is checked against the
//! MEASURED `T_e`-versus-age data directly, each compiled entry carrying its AGE CONVENTION and its LOADING
//! ENVIRONMENT (oceanic interior loads are the primary set; trench loads are a separate tagged environment,
//! because they diverge). No isotherm enters this arc, as an input or as a target.
//!
//! WHY THE ISOTHERM DIED, recorded because it is the silent-parameter class living inside the LITERATURE rather
//! than inside our code, which is a place this project had not yet thought to look. A "limiting isotherm" is
//! not a property of the lithosphere. It is a property of the lithosphere JOINED TO AN AGE CONVENTION: the SAME
//! measurements imply 550 to 600 C against thermal age and 350 to 450 C against isochron age (McNutt 1984, via
//! Calmant et al. 1990), and trench loads land near 340 C again. A single number quoted without its convention
//! is a statistic with a hidden conditioning variable, so it could never have been a target; it would have
//! validated whichever convention it was silently born under. The classical commentary value is 450 +/- 150 C,
//! in CELSIUS, and it may appear as commentary only, with that rider, and nowhere else.
//!
//! (This arc's own prose carried "~600 K" through three documents and into this file, which is 327 C, BELOW
//! every measured band; Calmant et al. state plainly that "no estimate is close to the 600 C isotherm". The
//! error entered as a ruling's summary statistic, propagated verbatim through scope docs into code, and was
//! caught by a fetch agent reading the primaries. Hence the standing rule: hindcast targets name DATASETS.)
//!
//! Nothing in this module authors a scalar.

use civsim_core::Fixed;

const ZERO: Fixed = Fixed::ZERO;

/// The STEADY CONDUCTIVE geotherm through a stagnant lid: the temperature at depth `z` in a lid spanning from
/// `surface_temperature` at `z = 0` to `interior_temperature` at `z = lid_thickness`, with the lid's own
/// radiogenic heat production bending the profile.
///
/// THE FORM. Steady one-dimensional conduction with a uniform volumetric source `A = density * heat_production`
/// obeys `k d2T/dz2 + A = 0`. Solving with the two temperatures as boundary conditions gives
///
/// `T(z) = T_s + (T_i - T_s) * (z/L) + (A / (2k)) * z * (L - z)`.
///
/// The first two terms are the straight conductive ramp between the boundaries; the third is the source's
/// symmetric parabola, zero at both ends and bulging the interior of the lid HOTTER than the ramp (heat made
/// inside the lid has to escape through it, so the profile bows up). With no heat production the parabola
/// vanishes and the profile is the pure ramp, which is the correct limit rather than a special case. The `2` is
/// the integration's own constant, not a knob.
///
/// UNIT-AGNOSTIC over a coherent set, like the flexure kernel: the caller supplies one consistent system and
/// `A / (2k) * z * (L - z)` lands in the same temperature unit as the boundaries. Raw SI is the caller's hazard
/// to manage, since `k` in W/(m K) against a lid in metres puts the source term through a wide dynamic range.
///
/// `None` on a non-positive lid thickness (no lid, so no profile through one) or a fixed-point overflow. A `z`
/// outside `[0, L]` is NOT clamped: the caller asking for a depth below the lid is asking the wrong question,
/// and the honest answer is the formula's own continuation rather than a silently pinned boundary value.
/// Deterministic fixed-point.
pub fn steady_conductive_geotherm(
    surface_temperature: Fixed,
    interior_temperature: Fixed,
    lid_thickness: Fixed,
    depth: Fixed,
    density: Fixed,
    heat_production: Fixed,
    thermal_conductivity: Fixed,
) -> Option<Fixed> {
    if lid_thickness <= ZERO {
        return None;
    }
    // The conductive ramp: T_s + (T_i - T_s) * (z / L).
    let contrast = interior_temperature.checked_sub(surface_temperature)?;
    let fraction = depth.checked_div(lid_thickness)?;
    let ramp = surface_temperature.checked_add(contrast.checked_mul(fraction)?)?;
    // The radiogenic source's parabola: (A / (2k)) * z * (L - z), with A = rho * H.
    if thermal_conductivity <= ZERO {
        // A non-conducting lid carries no conductive profile at all; the source term is undefined against it.
        // The ramp still stands (it is a pure boundary interpolation), so return it rather than fabricate.
        return Some(ramp);
    }
    let source = density.checked_mul(heat_production)?;
    if source == ZERO {
        return Some(ramp);
    }
    let two_k = thermal_conductivity.checked_mul(Fixed::from_int(2))?;
    let span = lid_thickness.checked_sub(depth)?;
    let bulge = source
        .checked_div(two_k)?
        .checked_mul(depth)?
        .checked_mul(span)?;
    ramp.checked_add(bulge)
}

/// The HALF-SPACE COOLING geotherm: the temperature at depth `z` in a lid of age `age` cooling from a surface
/// held at `surface_temperature` into an interior at `interior_temperature`.
///
/// THE FORM. A half-space initially at `T_i`, its surface dropped to `T_s` at `t = 0` and held there, conducts
/// heat out with the profile
///
/// `T(z, t) = T_s + (T_i - T_s) * erf( z / (2 sqrt(kappa t)) )`.
///
/// This is the exact solution of the diffusion equation for that boundary condition, so the only numbers in it
/// are the `2` from the similarity variable and the `erf` itself ([`Fixed::erf`], Abramowitz and Stegun 7.1.26,
/// shared with Ewald's real-space sum). At `z = 0` the erf vanishes and the profile reads the surface; deep down
/// the erf saturates to one and it reads the interior. The thermal thickness grows as `sqrt(kappa t)`, which is
/// why an older lid is a thicker one, and it is the same square root the oceanic `T_e`-versus-age hindcast row
/// is stated against.
///
/// THE APPROXIMATION'S HONEST LIMIT, measured rather than assumed: the A&S 7.1.26 coefficients sum to
/// 0.999999999 rather than exactly one, so `erf(0)` returns about 1.16e-9 (five bits in Q32.32) instead of a
/// clean zero. The surface boundary condition is therefore honoured to about `1.16e-9 * (T_i - T_s)`, which for
/// a 1300 K contrast is a couple of microkelvin. That is far inside the fit's own stated 1.5e-7 maximum error
/// and physically nothing, but it is a residual rather than an exactness, and the tests assert it as such
/// instead of pretending the boundary is pinned. [`steady_conductive_geotherm`] has no such residual: its
/// boundaries are exact by construction, since it interpolates them directly.
///
/// WHEN THIS FORM APPLIES: where a lid HAS an age, because it was created at a definite time and has been
/// cooling since (a plate born at a ridge). A stagnant lid has no such clock and takes
/// [`steady_conductive_geotherm`] instead. The dispatch is on whether the world's lid carries an age, never on
/// a named tectonic regime chosen by a threshold.
///
/// `None` on a non-positive age or diffusivity (no cooling clock, so no profile) or a fixed-point overflow.
/// Deterministic fixed-point.
pub fn halfspace_geotherm(
    surface_temperature: Fixed,
    interior_temperature: Fixed,
    thermal_diffusivity: Fixed,
    age: Fixed,
    depth: Fixed,
) -> Option<Fixed> {
    if age <= ZERO || thermal_diffusivity <= ZERO {
        return None;
    }
    // The similarity variable eta = z / (2 sqrt(kappa t)).
    let diffusion_length = thermal_diffusivity.checked_mul(age)?.sqrt();
    if diffusion_length <= ZERO {
        return None;
    }
    let two_length = diffusion_length.checked_mul(Fixed::from_int(2))?;
    let eta = depth.checked_div(two_length)?;
    let contrast = interior_temperature.checked_sub(surface_temperature)?;
    surface_temperature.checked_add(contrast.checked_mul(eta.erf())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(n: i64) -> Fixed {
        Fixed::from_int(n as i32)
    }

    #[test]
    fn the_steady_lid_spans_its_two_boundaries() {
        // With no radiogenic source the profile is the pure conductive ramp, so it reads the surface at the top
        // and the interior at the lid base exactly: the boundary conditions are honoured, not approximated.
        let t = |z| {
            steady_conductive_geotherm(f(300), f(1600), f(100), z, f(3300), ZERO, f(3))
                .expect("a profile")
        };
        assert_eq!(
            t(ZERO),
            f(300),
            "the top of the lid is the surface temperature"
        );
        assert_eq!(
            t(f(100)),
            f(1600),
            "the base of the lid is the interior potential temperature"
        );
        let mid = t(f(50));
        assert_eq!(
            mid,
            f(950),
            "with no source the profile is the straight ramp, so the midpoint is the mean"
        );
    }

    #[test]
    fn radiogenic_heat_bows_the_lid_hotter_than_the_ramp() {
        // Heat made INSIDE the lid must escape THROUGH it, so the interior of the lid runs hotter than the
        // straight ramp between the same boundaries. The bulge is zero at both ends (it cannot move a boundary
        // condition) and positive in between.
        let cold = steady_conductive_geotherm(f(300), f(1600), f(100), f(50), f(3300), ZERO, f(3))
            .unwrap();
        let hot = steady_conductive_geotherm(
            f(300),
            f(1600),
            f(100),
            f(50),
            f(3300),
            Fixed::from_ratio(1, 100_000),
            f(3),
        )
        .unwrap();
        assert!(
            hot > cold,
            "a lid making its own heat is hotter inside than the source-free ramp"
        );
        let at_top = steady_conductive_geotherm(
            f(300),
            f(1600),
            f(100),
            ZERO,
            f(3300),
            Fixed::from_ratio(1, 100_000),
            f(3),
        )
        .unwrap();
        let at_base = steady_conductive_geotherm(
            f(300),
            f(1600),
            f(100),
            f(100),
            f(3300),
            Fixed::from_ratio(1, 100_000),
            f(3),
        )
        .unwrap();
        assert_eq!(
            at_top,
            f(300),
            "the source cannot move the surface boundary"
        );
        assert_eq!(
            at_base,
            f(1600),
            "the source cannot move the interior boundary"
        );
    }

    #[test]
    fn a_lidless_column_has_no_profile() {
        // Fail-soft, never a fabricated temperature: no lid means no conductive profile through one.
        assert!(
            steady_conductive_geotherm(f(300), f(1600), ZERO, f(10), f(3300), ZERO, f(3)).is_none()
        );
        assert!(
            steady_conductive_geotherm(f(300), f(1600), f(-5), f(10), f(3300), ZERO, f(3))
                .is_none()
        );
    }

    #[test]
    fn the_halfspace_reads_its_boundaries_and_thickens_with_age() {
        // erf(0) = 0 so the surface is exact; erf saturates so the deep profile approaches the interior.
        let kappa = Fixed::from_ratio(1, 1_000_000);
        let young = f(1);
        let at_surface = halfspace_geotherm(f(300), f(1600), kappa, young, ZERO).unwrap();
        // NOT exact, and the reason is documented on the function: A&S 7.1.26's coefficients sum to
        // 0.999999999, so erf(0) is ~1.16e-9 rather than a clean zero, and the surface reads
        // `T_s + 1.16e-9 * (T_i - T_s)`, about two microkelvin high on this 1300 K contrast. Asserted as the
        // residual it is; widening this to `assert_eq` would be a lie and tightening it would fail for a real reason.
        let surface_residual = (at_surface - f(300)).abs();
        assert!(
            surface_residual < Fixed::from_ratio(1, 1000),
            "the surface of a cooling half-space reads the surface temperature to the fit's residual, got {at_surface:?}"
        );

        // THE AGE IS THE WHOLE POINT: at a FIXED depth, an OLDER lid has cooled further, so it is COLDER there.
        // That is the sqrt(kappa t) thickening, the same growth the oceanic T_e-versus-age hindcast is stated
        // against, and it is what makes the age a real input rather than decoration.
        // The probe must sit where the similarity variable eta = z / (2 sqrt(kappa t)) is of order one, which is
        // where the profile varies. Deeper than that and BOTH ages saturate erf to one and read the
        // interior, so the comparison would pass or fail on nothing. With kappa = 1e-6, z = 5e-3 puts eta at
        // ~2.5 for the young lid and ~0.25 for the old one: on opposite sides of the transition, so the age is
        // doing real work here rather than being decoration the test cannot see.
        let probe = Fixed::from_ratio(1, 200);
        let old = halfspace_geotherm(f(300), f(1600), kappa, f(100), probe).unwrap();
        let fresh = halfspace_geotherm(f(300), f(1600), kappa, f(1), probe).unwrap();
        assert!(
            old < fresh,
            "an older lid has cooled further at the same depth, got old={old:?} fresh={fresh:?}"
        );
        assert!(
            fresh > f(1500),
            "the young lid is still near its interior temperature at this depth, got {fresh:?}"
        );
        assert!(
            old < f(800),
            "the old lid has cooled substantially at this depth, got {old:?}"
        );
    }

    #[test]
    fn a_lid_with_no_clock_has_no_halfspace_profile() {
        // A stagnant lid has no cooling age, so this form REFUSES rather than inventing one. The caller takes
        // the steady conductive form instead; the dispatch is on the world's own lid, never on a named regime.
        let kappa = Fixed::from_ratio(1, 1_000_000);
        assert!(halfspace_geotherm(f(300), f(1600), kappa, ZERO, f(1)).is_none());
        assert!(halfspace_geotherm(f(300), f(1600), ZERO, f(10), f(1)).is_none());
    }

    #[test]
    fn the_error_function_twins_against_an_independent_quadrature_over_the_geotherm_range() {
        // THE NUMERICAL TWIN, and it is required because VALIDATION DOES NOT TRANSFER ACROSS DOMAINS. The A&S
        // 7.1.26 fit was twinned over EWALD's real-space argument range (`alpha r`); the geotherm exercises a
        // DIFFERENT range, the similarity variable `eta = z / (2 sqrt(kappa t))`, which runs from zero through
        // the order-one transition where the profile varies and out toward saturation. Reusing Ewald's
        // validation here would be borrowing evidence from a range this consumer never visits.
        //
        // The twin is INDEPENDENT by construction: composite Simpson quadrature of erf's OWN DEFINITION,
        // `erf(x) = (2/sqrt(pi)) * integral_0^x exp(-t^2) dt`. It shares no code with the fitted series (only
        // `exp` and `sqrt`), so agreement is real evidence rather than a series checked against itself, which is
        // the circular-validation trap. The `2/sqrt(pi)` is DERIVED from `Fixed::PI`, never typed in.
        let two_over_sqrt_pi = Fixed::from_int(2) / Fixed::PI.sqrt();
        let quad_erf = |x: Fixed| -> Fixed {
            // Simpson needs an even panel count; the integrand exp(-t^2) is smooth, so Simpson's h^4 accuracy
            // at 64 panels sits far inside the fit's own stated 1.5e-7 bound, which is what makes the fit (and
            // not the quadrature) the thing under test.
            let n = 64i32;
            let h = x / Fixed::from_int(n);
            let f = |t: Fixed| (ZERO - t * t).exp();
            let mut acc = f(ZERO) + f(x);
            for i in 1..n {
                let t = h * Fixed::from_int(i);
                let w = if i % 2 == 1 { 4 } else { 2 };
                acc += Fixed::from_int(w) * f(t);
            }
            two_over_sqrt_pi * (h / Fixed::from_int(3)) * acc
        };
        // The geotherm's own eta range: the order-one transition band the lid profile lives in, plus the
        // saturating tail. Each probe is differenced against the independent quadrature.
        let tol = Fixed::from_ratio(1, 100_000);
        for (num, den) in [(1, 4), (1, 2), (1, 1), (3, 2), (2, 1), (5, 2), (3, 1)] {
            let x = Fixed::from_ratio(num, den);
            let fit = x.erf();
            let quad = quad_erf(x);
            let err = (fit - quad).abs();
            assert!(
                err < tol,
                "the A&S fit and the independent quadrature agree at eta={x:?}: fit={fit:?} quad={quad:?} err={err:?}"
            );
        }
    }

    #[test]
    fn the_error_function_matches_its_known_values() {
        // The shared Fixed::erf against the standard table: erf(0) = 0, erf(1) ~ 0.8427, erf(2) ~ 0.9953.
        // A&S 7.1.26's stated maximum error is 1.5e-7; the tolerance here is loose enough to sit above the
        // fixed-point floor and tight enough to catch a wrong series.
        let tol = Fixed::from_ratio(1, 1000);
        // erf(0) = 0 ANALYTICALLY, but the fit's coefficients sum to 0.999999999, so it returns ~1.16e-9 (five
        // bits in Q32.32). Pinned at the residual it carries: this test is the one that measured it, and
        // the half-space geotherm's surface boundary inherits exactly this much error and says so.
        let e0 = ZERO.erf();
        assert!(
            e0.abs() < Fixed::from_ratio(15, 100_000_000),
            "erf(0) is zero to the fit's own 1.5e-7 bound, got {e0:?}"
        );
        let e1 = Fixed::ONE.erf();
        let want1 = Fixed::from_ratio(8427, 10_000);
        assert!((e1 - want1).abs() < tol, "erf(1) ~ 0.8427, got {e1:?}");
        let e2 = f(2).erf();
        let want2 = Fixed::from_ratio(9953, 10_000);
        assert!((e2 - want2).abs() < tol, "erf(2) ~ 0.9953, got {e2:?}");
        // Odd symmetry, inherited from erfc's reflection.
        let neg = (ZERO - Fixed::ONE).erf();
        assert!((neg + want1).abs() < tol, "erf(-1) = -erf(1), got {neg:?}");
    }
}
