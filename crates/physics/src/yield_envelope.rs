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

/// A FRICTION LAW as its source states it: two linear branches, EACH ON ITS OWN STATED DOMAIN.
///
/// THE SHAPE IS THE FINDING. A friction law is not "a coefficient plus a branch point"; it is two fits, each
/// licensed over an interval the experiments covered, and WHETHER THOSE INTERVALS MEET IS A PROPERTY OF THE
/// MATERIAL, not of the form. Rock's branches MEET (a continuity crossover at 200 MPa, self-tested by the
/// identity below). Ice's DO NOT: Beeman's low-stress law is licensed at or below 5 MPa and the high-stress law
/// at or above 10, so there is a GAP between them where neither fit is in its domain. Encoding a single
/// `branch_point` would have forced ice to invent one.
///
/// So the row carries `low_domain_max` and `high_domain_min` SEPARATELY. When they are equal the law is a
/// crossover; when they differ it has a gap, and [`shear_strength_mpa`] returns a BRACKET there rather than a
/// number. The structure admits both without either being a special case.
#[derive(Clone, Copy, Debug)]
pub struct FrictionLaw {
    /// The low-stress branch's coefficient: `tau = low_cohesion + low_coefficient * sigma_n`.
    pub low_coefficient: Fixed,
    /// The low-stress branch's cohesion intercept (megapascals).
    pub low_cohesion: Fixed,
    /// The upper limit of the STATED DOMAIN of the low-stress branch (megapascals).
    pub low_domain_max: Fixed,
    /// The high-stress branch's coefficient: `tau = high_cohesion + high_coefficient * sigma_n`.
    pub high_coefficient: Fixed,
    /// The high-stress branch's cohesion intercept (megapascals).
    pub high_cohesion: Fixed,
    /// The lower limit of the STATED DOMAIN of the high-stress branch (megapascals). EQUAL to `low_domain_max`
    /// for a crossover law (rock); GREATER for a law with a gap (ice).
    pub high_domain_min: Fixed,
    /// The low-stress regime's ROUGHNESS AND COHESION BAND, the scatter the central low fit is a line through.
    ///
    /// MEASURED FROM THE PRIMARY where a material's scatter is sourced, `None` where no source has pinned it yet.
    /// Byerlee's own abstract says the low branch scatters WIDELY between experiments because friction there is
    /// dominated by surface roughness, so a single `low_coefficient` reports a fit as a measurement (the sin
    /// [`byerlee_low_stress_coefficient`] names). `Some` carries the source's own envelope, at which point
    /// [`crate::moment_equivalence::brittle_differential_mpa`] emits a BRACKET wherever the low branch is operative
    /// AND the fault-normal stress sits inside the band's scatter domain, and the interval-of-mins carries it to
    /// the `T_e` band with no new composition. `None` is the honest unset state for a material whose scatter no
    /// source has pinned: the low branch emits its central fit alone, never a fabricated zero band. Rock carries
    /// Byerlee 1978's measured low-pressure cloud; ice carries `None`, since Beeman's scatter is its own read
    /// against Beeman's own domain boundaries rather than Byerlee's inherited.
    pub low_stress_band: Option<LowStressBand>,
}

/// The LOW-STRESS ROUGHNESS BAND, and where it ends: the scatter the central low fit is a line through, carried
/// as the interval its edges span and SCOPED to the regime it belongs to. MEASURED from the primary rather than
/// reserved, because the source states the envelope in its own ink.
///
/// THE SCOPE IS PART OF THE VALUE, and this is the seam that a fixed band hides. Byerlee 1978 separates three
/// pressure regimes, and the wide scatter is the LOW-pressure one: normal stress up to 50 bars (5 MPa, his Fig. 3),
/// where "the coefficient of friction can be as low as 0.3 and as high as 10" (p. 618), the cloud his abstract
/// calls friction "varying widely with surface roughness". Above that his intermediate maximum-friction data
/// (Fig. 5) have "much less scatter and can be approximated by tau = 0.85 sigma_n". So a band laid across the
/// whole sub-200-MPa low branch would overstate the scatter forty times in depth (5 MPa sits near 150 m on Earth,
/// the 200 MPa universality floor near 6 km). `scatter_domain_max` is that boundary, a measured regime line with
/// the same citizenship as the 200 MPa floor already encoded.
///
/// The coefficient edges are the roughness-scattered friction; the cohesion edges are the roughness-induced
/// apparent cohesion from asperity interlock (megapascals). Both edges ride so an asymmetric envelope is
/// representable, since a central fit need not sit at the midpoint of its own scatter.
#[derive(Clone, Copy, Debug)]
pub struct LowStressBand {
    /// The low edge of the roughness-scattered friction coefficient, BELOW `scatter_domain_max`.
    pub coefficient_lo: Fixed,
    /// The high edge of the roughness-scattered friction coefficient, below `scatter_domain_max`.
    pub coefficient_hi: Fixed,
    /// The low edge of the roughness-induced cohesion (megapascals).
    pub cohesion_lo: Fixed,
    /// The high edge of the roughness-induced cohesion (megapascals).
    pub cohesion_hi: Fixed,
    /// The normal-stress boundary (megapascals) between the roughness-scattered regime (below, where the
    /// coefficient band applies) and the intermediate regime (at or above, where the central low fit governs).
    /// Byerlee's own 50 bars = 5 MPa, the low-pressure limit of Fig. 3 and the onset of the tight Fig. 5 fit.
    pub scatter_domain_max: Fixed,
    /// The intermediate regime's residual coefficient band `(lo, hi)`, at or above `scatter_domain_max`. `None` is
    /// the EXPLICIT band-pending state, absence carried as a datum rather than an implicit zero-width band: the
    /// central fit governs there until a Figure-5 read lands. Byerlee calls that cloud "much less scatter" without
    /// a number, so it is drawn in ink but unquantified in text and awaits a figure digitization.
    pub intermediate_band: Option<(Fixed, Fixed)>,
}

/// The shear strength a friction law reports at a normal stress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShearStrength {
    /// A determined strength (megapascals): the normal stress sits inside one branch's stated domain.
    Determined(Fixed),
    /// THE BRACKET, for a normal stress inside a GAP the source does not constrain: the envelope spanned by the
    /// two fits, both evaluated outside their stated domains, with NO point chosen between them.
    ///
    /// This is DERIVED INFORMATION, the cited laws' own envelope, and it contains ZERO authored scalars. The
    /// alternative was to splice the branches at some stress inside the gap, which would author a boundary where
    /// the calibration says nothing (and for ice would invent a DISCONTINUITY of several MPa, since the two fits
    /// do not meet: their naive mathematical crossing sits near 21 MPa, outside BOTH stated domains). That is the
    /// n-gap finding wearing friction's coat: a scalar authored inside a classifier.
    ///
    /// Consumers eat bands natively. A verdict that FLIPS across this bracket is near-degenerate and surfaces as
    /// such, which is the machinery working rather than a gap in it.
    Bracket { low: Fixed, high: Fixed },
}

/// ROCK's friction law (Byerlee 1978), a CROSSOVER law: the two branches meet at 200 MPa.
///
/// The branch point is the source's own structure, not a threshold anyone chose. Byerlee's abstract states it
/// directly: at low normal stress the shear stress varies widely between experiments because friction is
/// STRONGLY DEPENDENT ON SURFACE ROUGHNESS, while at high normal stress that effect diminishes and friction
/// becomes NEARLY INDEPENDENT OF ROCK TYPE. So the branch point IS the universality floor; they are one boundary.
///
/// The low branch (`0.85 sigma_n`, no cohesion) is a CENTRAL FIT THROUGH ROUGHNESS-SCATTERED DATA and ships with
/// a wide band. The high branch (`50 + 0.6 sigma_n`) is the near material-independent regime.
pub fn rock_friction_law() -> FrictionLaw {
    FrictionLaw {
        low_coefficient: Fixed::from_ratio(85, 100),
        low_cohesion: ZERO,
        low_domain_max: Fixed::from_int(200),
        high_coefficient: Fixed::from_ratio(6, 10),
        high_cohesion: Fixed::from_int(50),
        high_domain_min: Fixed::from_int(200),
        // MEASURED from Byerlee 1978 (vendored, sha256 995adf14a816e517bd037936d45fa4237e312c0aa1f278781525bd6b202d2306):
        // the low-pressure roughness cloud, coefficient 0.3 to 10 (p. 618 text and Fig. 3), no cohesion
        // (through-origin, Barton's JRC form), scoped to his own 50-bar = 5 MPa low-pressure boundary. Above it the
        // intermediate maximum-friction fit is tight; that residual band is `None`, the explicit Figure-5-read-pending
        // state, so the central 0.85 governs there until the figure digitization lands.
        low_stress_band: Some(LowStressBand {
            coefficient_lo: Fixed::from_ratio(3, 10),
            coefficient_hi: Fixed::from_int(10),
            cohesion_lo: ZERO,
            cohesion_hi: ZERO,
            scatter_domain_max: Fixed::from_int(5),
            intermediate_band: None,
        }),
    }
}

/// ICE's friction law (Beeman et al. 1988), a GAP law: the branches do NOT meet.
///
/// `tau = 0.55 sigma_n + 1.0 MPa` at or below 5 MPa; `tau = 0.20 sigma_n + 8.3 MPa` at or above 10 MPa, measured
/// at 77 to 115 K with friction independent of temperature and of velocity over the tested decade.
///
/// THE GAP IS REAL AND IT IS WHY THIS ROW EXISTS. Between 5 and 10 MPa neither fit is licensed, and the two do
/// not cross inside the gap: their naive mathematical crossing is near 21 MPa, outside BOTH domains, and at any
/// splice inside the gap they disagree by several MPa. A single spliced boundary would author a discontinuous
/// strength law out of thin air. So [`shear_strength_mpa`] returns the bracket there.
///
/// THE HEADLINE, from the source: ice's frictional strength sits well below every rock, so ICE LIDS ARE WEAK ON
/// BOTH BRANCHES of their law. And the branch structure changes near 5 to 10 MPa rather than 200, which is why
/// handing an ice shell the ROCK branch point would quietly un-do the material conditioning this row exists for.
pub fn ice_friction_law() -> FrictionLaw {
    FrictionLaw {
        low_coefficient: Fixed::from_ratio(55, 100),
        low_cohesion: Fixed::ONE,
        low_domain_max: Fixed::from_int(5),
        high_coefficient: Fixed::from_ratio(20, 100),
        high_cohesion: Fixed::from_ratio(83, 10),
        high_domain_min: Fixed::from_int(10),
        // `None` by ratified design: ice keeps Beeman's OWN domain boundaries rather than inheriting Byerlee's, so
        // its low-fit scatter is its own read against Beeman's own regimes, unbuilt until that source is read.
        low_stress_band: None,
    }
}

/// The BRANCH POINT of ROCK's law (megapascals). Retained as the rock row's accessor; a caller with a material
/// reads [`FrictionLaw`] instead of this, because a hardcoded branch point IS the defect this row was split to
/// retire: a conditioning function that hardcodes an instance teaches every reader that conditioning is handled,
/// which is worse than a wrong value.
pub fn byerlee_branch_point_mpa() -> Fixed {
    rock_friction_law().low_domain_max
}

/// The high-stress branch's cohesion intercept (megapascals).
///
/// THE UNIT TRAP, AND WHY THIS ROW PROVES ITS OWN UNITS. Byerlee's law is published in KILOBARS: reading
/// `tau = 0.5 + 0.6 sigma` as megapascals is a silent 100x error, and it has convicted at least one widely read
/// reference that prints the intercept in MPa. The half-kilobar intercept is 50 MPa, and the row does not have
/// to take that on faith: THE TWO BRANCHES ARE CONTINUOUS AT THE BRANCH POINT ONLY IF THE INTERCEPT IS 50 MPa,
/// since `0.85 * 200 = 170 = 50 + 0.6 * 200`. That identity is this module's self-test
/// (`tests::the_branches_are_continuous_which_is_the_rows_own_unit_proof`): a mis-converted intercept breaks
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
    match shear_strength_mpa(&rock_friction_law(), normal_stress_mpa)? {
        ShearStrength::Determined(t) => Some(t),
        // Rock is a crossover law, so it has no gap and cannot bracket. Unreachable by construction, and the
        // continuity self-test proves the domains meet.
        ShearStrength::Bracket { .. } => None,
    }
}

/// THE MATERIAL-KEYED SHEAR STRENGTH: a friction law's strength at a normal stress, keyed on the MATERIAL'S OWN
/// ROW rather than on rock's numbers wearing a constant's clothes.
///
/// Below the low branch's stated maximum, the low fit. At or above the high branch's stated minimum, the high
/// fit. BETWEEN THEM, if the law has a gap, the BRACKET spanned by both fits with no point chosen
/// ([`ShearStrength::Bracket`]). A crossover law (rock) has no between, so it always determines.
///
/// WHY THIS REPLACED A HARDCODED 200. The predicate that told a caller whether Byerlee's universality holds was
/// itself hardcoding rock's branch point, so an ice shell would have been handed the ROCK boundary, silently
/// un-doing the material conditioning the brittle branch had just gained. Ice's law changes branch near 5 to 10
/// MPa, forty times lower. A conditioning function that hardcodes an instance is wrong machinery wearing the
/// uniform of the fix.
///
/// `None` on a negative normal stress or overflow.
pub fn shear_strength_mpa(law: &FrictionLaw, normal_stress_mpa: Fixed) -> Option<ShearStrength> {
    if normal_stress_mpa < ZERO {
        return None;
    }
    let low = law
        .low_coefficient
        .checked_mul(normal_stress_mpa)?
        .checked_add(law.low_cohesion)?;
    let high = law
        .high_coefficient
        .checked_mul(normal_stress_mpa)?
        .checked_add(law.high_cohesion)?;
    if normal_stress_mpa < law.low_domain_max {
        return Some(ShearStrength::Determined(low));
    }
    if normal_stress_mpa >= law.high_domain_min {
        return Some(ShearStrength::Determined(high));
    }
    // Inside the gap: neither fit is licensed here. Report the envelope the two cited fits span, ordered, with
    // no point chosen between them.
    if low <= high {
        Some(ShearStrength::Bracket { low, high })
    } else {
        Some(ShearStrength::Bracket {
            low: high,
            high: low,
        })
    }
}

/// Whether a material at this normal stress sits in the regime where its friction law's HIGH branch is
/// licensed: for rock, where Byerlee's near material-independence holds.
///
/// KEYED ON THE MATERIAL'S OWN ROW. Rock reaches its universal regime at 200 MPa; ice's high branch begins near
/// 10 MPa. Answering this question with rock's number for an ice shell is the defect this signature exists to
/// make impossible: the material is an argument, so a caller cannot forget to supply one.
pub fn is_in_high_branch_regime(law: &FrictionLaw, normal_stress_mpa: Fixed) -> bool {
    normal_stress_mpa >= law.high_domain_min
}

/// Whether a lid at this normal stress sits in the regime where Byerlee's material independence HOLDS.
///
/// A caller asks this before leaning on "one friction law for every rock". `false` means the answer is
/// roughness-dominated and the strength carries a wide band, and it is the honest answer for a thin lid on a
/// small body. This is the question the alien-admission claim depends on, so it is a function rather than an
/// assumption at a call site.
pub fn byerlee_is_in_the_universal_regime(normal_stress_mpa: Fixed) -> bool {
    is_in_high_branch_regime(&rock_friction_law(), normal_stress_mpa)
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
    fn ice_has_a_gap_where_rock_has_a_crossover_and_the_gap_brackets_rather_than_splices() {
        // THE SHAPE FINDING. Rock's branches MEET (a crossover): its domains are one boundary. Ice's DO NOT:
        // Beeman licenses the low fit at or below 5 MPa and the high fit at or above 10, so 5 to 10 is a gap
        // where NEITHER fit is in its domain. The two do not cross inside it either (their naive crossing is
        // near 21 MPa, outside both), so any spliced boundary would author a discontinuity of several MPa out
        // of thin air. The row BRACKETS instead: the envelope the two cited fits span, no point chosen.
        let rock = rock_friction_law();
        let ice = ice_friction_law();
        assert_eq!(
            rock.low_domain_max, rock.high_domain_min,
            "rock is a CROSSOVER law: its two domains meet at one boundary"
        );
        assert!(
            ice.high_domain_min > ice.low_domain_max,
            "ice is a GAP law: its high branch begins above where its low branch ends"
        );
        // Inside ice's gap: a bracket, and a real one (the fits disagree there, which is why no point is chosen).
        match shear_strength_mpa(&ice, Fixed::from_int(7)).unwrap() {
            ShearStrength::Bracket { low, high } => {
                assert!(high > low, "the bracket spans the two fits, ordered");
                assert!(
                    (high - low) > Fixed::ONE,
                    "the fits disagree by MPa inside the gap, which is exactly why splicing would author a jump: [{low:?}, {high:?}]"
                );
            }
            other => panic!("ice at 7 MPa is inside its gap and must bracket, got {other:?}"),
        }
        // Outside the gap, ice determines on each stated domain.
        assert!(matches!(
            shear_strength_mpa(&ice, Fixed::from_int(2)).unwrap(),
            ShearStrength::Determined(_)
        ));
        assert!(matches!(
            shear_strength_mpa(&ice, Fixed::from_int(20)).unwrap(),
            ShearStrength::Determined(_)
        ));
        // Rock NEVER brackets: a crossover law has no between.
        for s in [0, 100, 199, 200, 400] {
            assert!(
                matches!(
                    shear_strength_mpa(&rock, Fixed::from_int(s)).unwrap(),
                    ShearStrength::Determined(_)
                ),
                "rock determines everywhere, including at its crossover, got a bracket at {s} MPa"
            );
        }
    }

    #[test]
    fn the_alien_inversion_holds_across_material_as_well_as_gravity() {
        // THE MATERIAL CASE, completing a test that was valid but MATERIAL-BLIND. The gravity half already
        // proved the inversion falls out of sigma_n = rho g z with no special case. This half proves the
        // predicate is keyed on the MATERIAL'S OWN ROW: handing an ice shell rock's 200 MPa boundary would tell
        // it that it is roughness-dominated where its own law says its high branch began twenty times lower.
        let rock = rock_friction_law();
        let ice = ice_friction_law();
        // The same stress reads OPPOSITELY on the two materials, which is the whole point of the keying.
        let stress = Fixed::from_int(20);
        assert!(
            !is_in_high_branch_regime(&rock, stress),
            "20 MPa is far below rock's 200 MPa branch: rock is still roughness-dominated"
        );
        assert!(
            is_in_high_branch_regime(&ice, stress),
            "20 MPa is well above ice's ~10 MPa branch: ice is already on its high branch"
        );
        // A EUROPA-CLASS SHELL at depth against EARTH ROCK at depth, both through the same lithostatic axis.
        let ice_stress = lithostatic_normal_stress_mpa(
            Fixed::from_int(920),        // ice density
            Fixed::from_ratio(131, 100), // Europa-class gravity
            Fixed::from_int(10_000),     // 10 km into the shell
        )
        .unwrap();
        let rock_stress = lithostatic_normal_stress_mpa(
            Fixed::from_int(2900),
            Fixed::from_ratio(981, 100),
            Fixed::from_int(10_000),
        )
        .unwrap();
        assert!(
            is_in_high_branch_regime(&ice, ice_stress),
            "10 km into a Europa-class shell is past ICE's own branch, got {ice_stress:?} MPa"
        );
        assert!(
            is_in_high_branch_regime(&rock, rock_stress),
            "10 km into Earth rock is past ROCK's own branch, got {rock_stress:?} MPa"
        );
        // AND THE DEFECT THIS REPLACED, asserted so it cannot come back: rock's boundary would MISJUDGE the ice
        // shell, calling it roughness-dominated when its own law has it on the high branch.
        assert!(
            !is_in_high_branch_regime(&rock, ice_stress),
            "rock's row misjudges the ice shell, which is why the predicate must key on the material's own row"
        );
        // ICE IS WEAK ON BOTH BRANCHES, the source's own headline: at the same stress, ice yields below rock.
        let ice_tau = shear_strength_mpa(&ice, Fixed::from_int(300)).unwrap();
        let rock_tau = shear_strength_mpa(&rock, Fixed::from_int(300)).unwrap();
        match (ice_tau, rock_tau) {
            (ShearStrength::Determined(i), ShearStrength::Determined(r)) => assert!(
                i < r,
                "ice's frictional strength sits well below rock's: ice={i:?} rock={r:?}"
            ),
            other => panic!("both determine at 300 MPa, got {other:?}"),
        }
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
