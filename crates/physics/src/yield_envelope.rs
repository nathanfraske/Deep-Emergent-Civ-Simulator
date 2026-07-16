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

//! THE YIELD-STRENGTH ENVELOPE: the strength of a lid against depth, and the brittle branch that floors it.
//!
//! WHY. `T_e`, the elastic thickness, is the sole unsupplied input to the flexural rigidity `D`, and it FALLS
//! OUT of this envelope rather than being read off a proxy: the BRITTLE curve (frictional sliding, this module)
//! meets the DUCTILE curve (creep, on the world's own derived strain rate) at a depth the world's own physics
//! sets. The famous limiting isotherm was never an input and never will be; it is the Earth-olivine INSTANCE of
//! this construction, and a hindcast checks against the measured `T_e`-versus-age DATASET, never a statistic.
//!
//! THE CONDITIONING AXIS IS NORMAL STRESS FIRST, MATERIAL SECOND, which inverts the usual instinct and is the
//! whole reason Byerlee is worth having. Friction's near material-independence is a HIGH-STRESS property, so the
//! axis a lid moves along is `sigma_n = rho * g * z`: its own density, its own gravity, its own depth.

use civsim_core::Fixed;

const ZERO: Fixed = Fixed::ZERO;

/// The BRANCH POINT of Byerlee's law (megapascals): the normal stress above which friction becomes nearly
/// independent of rock type, and below which it does not.
///
/// This is the source's own structure, not a threshold anyone chose. Byerlee 1978's abstract states it directly:
/// at low normal stress the shear stress varies widely between experiments because friction is STRONGLY
/// DEPENDENT ON SURFACE ROUGHNESS, while at high normal stress that effect diminishes and friction becomes
/// NEARLY INDEPENDENT OF ROCK TYPE. So the branch point IS the universality floor; they are one boundary.
pub fn byerlee_branch_point_mpa() -> Fixed {
    Fixed::from_int(200)
}

/// The high-stress branch's cohesion intercept (megapascals).
///
/// THE UNIT TRAP, AND WHY THIS ROW PROVES ITS OWN UNITS. Byerlee's law is published in KILOBARS: reading
/// `tau = 0.5 + 0.6 sigma` as megapascals is a silent 100x error, and it has convicted at least one widely read
/// reference that prints the intercept in MPa. The half-kilobar intercept is 50 MPa, and the row does not have
/// to take that on faith: THE TWO BRANCHES ARE CONTINUOUS AT THE BRANCH POINT ONLY IF THE INTERCEPT IS 50 MPa,
/// since `0.85 * 200 = 170 = 50 + 0.6 * 200`. That identity is this module's self-test
/// ([`tests::the_branches_are_continuous_which_is_the_rows_own_unit_proof`]): a mis-converted intercept breaks
/// continuity by a factor of 100 and the test fires. The row carries its own arithmetic referee, so the unit
/// cannot silently rot.
pub fn byerlee_cohesion_mpa() -> Fixed {
    Fixed::from_int(50)
}

/// The LOW-stress branch's friction coefficient, `tau = 0.85 * sigma_n`.
///
/// SHIPS WITH A WIDE BAND, and the band is the point rather than a caveat. This is a CENTRAL FIT THROUGH
/// ROUGHNESS-SCATTERED DATA: below the branch point the shear stress varies widely between experiments because
/// surface roughness dominates, so `0.85` is where the scatter centres, never a tight constant. A caller in this
/// regime that reports a single number without the band is reporting a fit as a measurement.
pub fn byerlee_low_stress_coefficient() -> Fixed {
    Fixed::from_ratio(85, 100)
}

/// The HIGH-stress branch's friction coefficient, `tau = 50 MPa + 0.6 * sigma_n`. This is the near
/// material-independent regime: one friction law for every rock, which is the property the whole branch is
/// worth having for.
pub fn byerlee_high_stress_coefficient() -> Fixed {
    Fixed::from_ratio(6, 10)
}

/// BYERLEE'S LAW: the shear stress (megapascals) required to slide a fault at a given normal stress
/// (megapascals). The BRITTLE branch of the yield-strength envelope.
///
/// `tau = 0.85 * sigma_n` below the branch point; `tau = 50 + 0.6 * sigma_n` above it.
///
/// THE ALIEN-ADMISSION QUESTION, answered plainly rather than as advertised. Byerlee's material independence is
/// the reason one friction law can serve every silicate lid, and it is REAL, but it is a HIGH-STRESS property.
/// Below the branch point, roughness dominates and the universality does not hold. THE FLOOR THEREFORE INVERTS
/// THE GIFT: the worlds that never reach the universal regime are exactly the SMALL, LOW-GRAVITY ones the rule
/// was meant to admit, plus the shallowest kilometres of every world, including this one. A thin lid on a low-`g`
/// body may live entirely in the roughness-scattered regime, where "one law for every rock" is simply false.
/// [`byerlee_is_in_the_universal_regime`] answers that question for a caller rather than letting it assume.
///
/// NAMED EXCEPTION BAND: montmorillonite and vermiculite gouge collapse friction FAR BELOW both branches. So
/// hydrated and altered fault materials are conditioned separately, which matters for exactly the
/// water-processed crusts a hydrosphere makes. This function is the intact-rock law; a caller with an altered
/// gouge must not read it.
///
/// `None` on a negative normal stress (a fault cannot be pulled apart and slid at once) or overflow.
/// Unit-agnostic in neither direction: `sigma_n` is MEGAPASCALS, because the intercept is, and the source's
/// kilobars were converted ONCE, here, with the continuity identity standing guard over the conversion.
pub fn byerlee_shear_strength_mpa(normal_stress_mpa: Fixed) -> Option<Fixed> {
    if normal_stress_mpa < ZERO {
        return None;
    }
    if normal_stress_mpa < byerlee_branch_point_mpa() {
        return byerlee_low_stress_coefficient().checked_mul(normal_stress_mpa);
    }
    byerlee_high_stress_coefficient()
        .checked_mul(normal_stress_mpa)?
        .checked_add(byerlee_cohesion_mpa())
}

/// Whether a lid at this normal stress sits in the regime where Byerlee's material independence HOLDS.
///
/// A caller asks this before leaning on "one friction law for every rock". `false` means the answer is
/// roughness-dominated and the strength carries a wide band, and it is the honest answer for a thin lid on a
/// small body. This is the question the alien-admission claim depends on, so it is a function rather than an
/// assumption at a call site.
pub fn byerlee_is_in_the_universal_regime(normal_stress_mpa: Fixed) -> bool {
    normal_stress_mpa >= byerlee_branch_point_mpa()
}

/// The LITHOSTATIC normal stress (megapascals) at a depth: `sigma_n = rho * g * z`, the axis the brittle branch
/// is conditioned on. Density in kg/m^3, gravity in m/s^2, depth in metres; the `1e-6` folds pascals to
/// megapascals so the caller never converts and the branch comparison is always in the intercept's own unit.
///
/// THIS is why the conditioning axis is normal stress first and material second: a world's own density, gravity,
/// and depth place it on Byerlee's curve, and only then does its material matter (and above the branch point,
/// barely at all). A low-`g` world reaches a given stress deeper, so it carries more of its lid in the
/// roughness-scattered regime, which is a real and derived consequence rather than a special case.
pub fn lithostatic_normal_stress_mpa(
    density: Fixed,
    gravity: Fixed,
    depth_m: Fixed,
) -> Option<Fixed> {
    if density < ZERO || gravity < ZERO || depth_m < ZERO {
        return None;
    }
    // rho * g * z in pascals, then to megapascals. Staged so the pascal intermediate (which reaches ~1e9 for a
    // deep lid) never has to be representable on its own: the metres are folded to megametres first.
    let depth_scaled = depth_m.checked_div(Fixed::from_int(1_000_000))?;
    density.checked_mul(gravity)?.checked_mul(depth_scaled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_branches_are_continuous_which_is_the_rows_own_unit_proof() {
        // THE SELF-TEST, and the reason this row cannot silently rot in the wrong unit. Byerlee is published in
        // KILOBARS; the half-kilobar intercept is 50 MPa. The two branches meet at the branch point ONLY if that
        // conversion is right: 0.85 * 200 = 170 = 50 + 0.6 * 200. An intercept mis-read as 0.5 MPa (the exact
        // 100x error that has convicted at least one widely read reference) breaks this by ~49 MPa and fires here.
        let bp = byerlee_branch_point_mpa();
        let from_below = byerlee_low_stress_coefficient().checked_mul(bp).unwrap();
        let from_above = byerlee_shear_strength_mpa(bp).unwrap();
        // THE IDENTITY ITSELF, and it holds BIT-EXACTLY, which is a stronger result than the arithmetic
        // promises: 0.85 and 0.6 each truncate slightly low in Q32.32, and they truncate to the SAME product
        // (both branches land on 730144440200 bits). So the two branches meet with no residue between them.
        assert_eq!(
            from_below, from_above,
            "the branches must meet at the branch point: below={from_below:?} above={from_above:?}"
        );
        // The MEETING VALUE is 170 MPa analytically, but NOT to the bit: both branches sit ~2.8e-8 MPa below it,
        // because neither 0.85 nor 0.6 is exactly representable in Q32.32. Asserted at the representation's own
        // residue rather than at a fictitious exactness. The tolerance does not soften the test's job: the error
        // it exists to catch is the kilobar/megapascal 100x misread, which moves the intercept by ~49 MPa, nine
        // orders of magnitude above this residue.
        let ideal = Fixed::from_int(170);
        let residue = (from_below - ideal).abs();
        assert!(
            residue < Fixed::from_ratio(1, 1000),
            "the branches meet at 170 MPa to the fixed-point residue, got {from_below:?} (residue {residue:?})"
        );
        // The guard's discriminating power, asserted rather than assumed: an intercept misread as 0.5 MPa (the
        // exact 100x error that has convicted a widely read reference) breaks continuity by ~49 MPa, which is
        // ~49000x this tolerance. The identity cannot fail to notice.
        let mis_read = byerlee_high_stress_coefficient()
            .checked_mul(bp)
            .unwrap()
            .checked_add(Fixed::from_ratio(5, 10))
            .unwrap();
        assert!(
            (mis_read - ideal).abs() > Fixed::from_int(49),
            "a kilobar-as-megapascal intercept misses the identity by ~49 MPa, so continuity convicts it"
        );
    }

    #[test]
    fn the_universal_regime_question_is_answered_not_assumed() {
        // The material-independence Byerlee is worth having for is a HIGH-STRESS property, so the honest answer
        // below the branch point is "no". A caller leaning on one-law-for-every-rock must ask.
        assert!(
            !byerlee_is_in_the_universal_regime(Fixed::from_int(50)),
            "a shallow lid is roughness-dominated"
        );
        assert!(
            byerlee_is_in_the_universal_regime(Fixed::from_int(400)),
            "a deep lid is in the universal regime"
        );
        assert!(
            byerlee_is_in_the_universal_regime(byerlee_branch_point_mpa()),
            "the branch point itself is where universality begins"
        );
    }

    #[test]
    fn strength_rises_with_depth_and_the_slope_slackens_past_the_branch() {
        // Physically: deeper rock is harder to slide. And the high-stress branch is SHALLOWER in slope (0.6 vs
        // 0.85), so the envelope bends over at the branch point rather than continuing straight.
        let shallow = byerlee_shear_strength_mpa(Fixed::from_int(100)).unwrap();
        let deep = byerlee_shear_strength_mpa(Fixed::from_int(400)).unwrap();
        assert!(deep > shallow, "a deeper fault is stronger");
        // The slope on each side, measured over the same 50 MPa span.
        let low_a = byerlee_shear_strength_mpa(Fixed::from_int(100)).unwrap();
        let low_b = byerlee_shear_strength_mpa(Fixed::from_int(150)).unwrap();
        let high_a = byerlee_shear_strength_mpa(Fixed::from_int(300)).unwrap();
        let high_b = byerlee_shear_strength_mpa(Fixed::from_int(350)).unwrap();
        assert!(
            (low_b - low_a) > (high_b - high_a),
            "the low-stress branch climbs faster (0.85) than the high-stress one (0.6)"
        );
    }

    #[test]
    fn a_low_gravity_world_carries_more_lid_in_the_roughness_regime() {
        // THE ALIEN CASE, and it is the one the universality claim inverts on. At the SAME depth, a low-g body
        // reaches a lower normal stress, so more of its lid sits below the branch point where friction is
        // roughness-dominated and "one law for every rock" is false. This is derived, not special-cased: it
        // falls out of sigma_n = rho g z alone.
        let rho = Fixed::from_int(2900);
        let depth = Fixed::from_int(10_000); // 10 km
        let earth = lithostatic_normal_stress_mpa(rho, Fixed::from_ratio(981, 100), depth).unwrap();
        let small = lithostatic_normal_stress_mpa(rho, Fixed::from_ratio(163, 100), depth).unwrap(); // lunar-class g
        assert!(
            earth > small,
            "the same depth on a smaller body is a lower normal stress"
        );
        assert!(
            byerlee_is_in_the_universal_regime(earth),
            "10 km into an Earth-gravity crust is past the branch point, got {earth:?} MPa"
        );
        assert!(
            !byerlee_is_in_the_universal_regime(small),
            "10 km into a lunar-gravity crust is still roughness-dominated, got {small:?} MPa"
        );
    }

    #[test]
    fn the_lithostatic_axis_lands_the_known_crustal_magnitude() {
        // A sanity anchor rather than a fit: ~2900 kg/m^3 crust at Earth gravity, 10 km down, is ~285 MPa.
        // If the megapascal fold were wrong this would be off by 1e6 and every branch comparison would be noise.
        let s = lithostatic_normal_stress_mpa(
            Fixed::from_int(2900),
            Fixed::from_ratio(981, 100),
            Fixed::from_int(10_000),
        )
        .unwrap();
        assert!(
            s > Fixed::from_int(250) && s < Fixed::from_int(320),
            "10 km of 2900 kg/m^3 crust at 9.81 m/s^2 is ~285 MPa, got {s:?}"
        );
    }

    #[test]
    fn a_fault_pulled_apart_has_no_friction_law() {
        // Fail-soft, never a fabricated strength: a negative normal stress is not a state this law describes.
        assert!(byerlee_shear_strength_mpa(Fixed::ZERO - Fixed::ONE).is_none());
        assert!(lithostatic_normal_stress_mpa(
            Fixed::from_int(2900),
            Fixed::from_int(10),
            Fixed::ZERO - Fixed::ONE
        )
        .is_none());
        // Zero normal stress: an unloaded fault slides at zero shear. The low branch passes through the origin.
        assert_eq!(byerlee_shear_strength_mpa(ZERO).unwrap(), ZERO);
    }
}
