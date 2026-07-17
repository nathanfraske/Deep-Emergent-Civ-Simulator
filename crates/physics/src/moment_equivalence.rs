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

//! THE MOMENT-EQUIVALENCE: the uniform elastic plate that carries the yield-limited plate's bending moment at
//! the same curvature (McNutt and Menard 1982; Watts and Burov 2003).
//!
//! # WHY THE MODULE IS NAMED FOR THE CONSTRUCTION AND NOT FOR `T_e`
//!
//! The arc called this slice "the payoff, `T_e`". The module is not named `elastic_thickness`, and the reason is
//! the owner's fourth ruling: THE COMPARISON HAPPENS IN RIGIDITY SPACE, so `T_e` is not this module's canonical
//! output. Naming the module for `T_e` would enshrine the demoted symbol at the top of the file and teach every
//! reader that the thickness is the object. The object is the moment equivalence, and its output is a RIGIDITY.
//!
//! # THE CANONICAL OUTPUT IS `D_eq`, AND `T_e` IS A DISPLAY STATISTIC
//!
//! Stated here because the slice was told to think carefully about which quantity is primary and say why.
//!
//! A flexure observation constrains a DEFLECTION PROFILE. From the profile the fit object is the flexural
//! parameter and hence the RIGIDITY `D`: the plate equation contains `D`, the wavelength is
//! `alpha = (4 D / (delta_rho g))^(1/4)`, and nothing in the deflection knows what `E` the plate has. `T_e` is
//! then obtained by re-expressing that `D` through an ASSUMED modulus pair, and the literature's pair is
//! `E = 80 GPa`, `nu = 0.25`: McNutt and Menard's Table 1 prints them under the heading "Assumed values for
//! physical parameters", and Watts and Burov assume the same pair (their Fig. 5 caption). Since `T_e` is
//! proportional to `(1/E)^(1/3)`, an engine that DERIVES its own world's Young's modulus and then compares its
//! `T_e` against a published `T_e` is comparing its own plate against a fictitious 80 GPa plate, and the
//! mismatch enters as a silent factor of `(E_row / E_world)^(1/3)`.
//!
//! So the comparison is done in rigidity space, each side owning its own moduli: the engine derives `D_eq` at the
//! world's own `E` and `nu` ([`equivalent_rigidity`]); a hindcast row's published `T_e` is converted BACK to
//! `D_row` through the ROW'S OWN stated pair; and `D_eq` is scored against `D_row`. That is like against like.
//!
//! Two further facts settle it. `D` is the quantity with downstream consequence: the flexure kernel reads `D`
//! ([`crate::flexure::flexural_rigidity`], [`crate::flexure::flexural_parameter`]), and no engine physics
//! anywhere reads `T_e`. And `T_e` exists to talk to the literature, which is a REPORTING need rather than a
//! physical one. Hence [`equivalent_rigidity`] is canonical and [`elastic_thickness_km`] is a display statistic
//! that takes `E` and `nu` as EXPLICIT arguments, so the convention it carries can never be silent.
//!
//! # THE CONSTRUCTION, AND WHERE EACH PIECE IS CITED
//!
//! The bending moment of the yield-limited plate is the depth integral of the fibre stress about the neutral
//! plane (McNutt and Menard 1982 eq. 3, `docs/working/TE_CONSTRUCTION_FETCH.md` section 2.2):
//!
//! `M = integral over depth of sigma_f(z) * (z - z_n) dz`
//!
//! The fibre stress is BOUNDED BY the yield envelope rather than equal to it: in the un-yielded core the plate is
//! elastic, and where the elastic stress would exceed the envelope the fibre yields and the stress is capped at
//! the envelope. The neutral surface `z_n` is fixed by the ZERO-NET-AXIAL-FORCE condition, the source's own
//! sentence being "the fibre stresses must sum to zero over the thickness of the plate" (their p. 365, read
//! against their eq. 2 `N = integral of delta-sigma dz`). Neither primary prints the solve for `z_n`; the fetch
//! grades that step an INFERENCE from what the source prints, and this module inherits that grade
//! ([`neutral_surface_depth_km`]).
//!
//! The equivalent plate then carries that moment at that curvature, which solves to
//! `T_e = (12 (1 - nu^2) M / (E K))^(1/3)` (Watts and Burov 2003 eq. 2, which prints McNutt and Menard's eqs. 4
//! and 5 already solved). `(1 - nu^2)` sits in the DENOMINATOR of `D`, hence in the NUMERATOR of the cube root.
//!
//! # THE SIGN CONVENTION, STATED IN FULL BECAUSE NEITHER PRIMARY STATES IT
//!
//! The fetch (section 6.1) established by back-solving the printed equations, and corroborated against a figure
//! axis, that BOTH papers use TENSION-POSITIVE stress and NEITHER says so. A consumer on the geological
//! compression-positive convention silently swaps the strong and weak branches of an envelope whose whole content
//! is that rock is about three times stronger in compression than in tension. So this module states its
//! convention completely, and encodes the one piece a caller could get wrong in a TYPE ([`FibreCurvature`]).
//!
//! - STRESS is TENSION-POSITIVE. A compressive differential stress is negative.
//! - `z` is POSITIVE DOWNWARD (depth).
//! - `w` is POSITIVE UPWARD, and `K = d2w/dx2`. This is McNutt and Menard's own convention (their p. 365).
//! - THE FIBRE STRESS IS `sigma_f(z) = [E / (1 - nu^2)] * K * (z - z_n)`, capped by the envelope.
//!
//! THE FIBRE-STRESS SIGN IS DERIVED RATHER THAN COPIED, because it is the one line a paraphrase cannot carry.
//! For a curve of curvature `kappa` with the centre of curvature above it, a fibre a height `y` ABOVE the neutral
//! axis is nearer the centre and therefore shorter, so `epsilon(y) = -y * kappa`. With `z` measured DOWNWARD a
//! point at depth `z` sits at height `y = z_n - z`, so `epsilon(z) = +(z - z_n) * K` and the plane-strain fibre
//! stress is `[E / (1 - nu^2)] * (z - z_n) * K`. Sanity: a plate SAGGING under a load is concave up (`K > 0`) and
//! must be COMPRESSED at its top; at the top `z < z_n` so `(z - z_n) < 0` and the stress is negative, which is
//! compression under the tension-positive convention. It checks.
//!
//! THE CONVENTION IS REFEREED BY THE PRIMARY'S OWN SENTENCE, not by this reasoning alone. McNutt and Menard print
//! (p. 367): "a plate with negative curvature (concave downward) will appear to have a smaller `T_e` than a plate
//! with positive curvature". Under the convention above, `K < 0` puts the TENSILE fibre at the SHALLOW top, where
//! the brittle envelope is weak in tension (about 90 MPa at the surface against about 283 MPa in compression), so
//! the yielded moment is smaller and `T_e` is smaller. The construction reproduces the source's own asymmetry
//! from the sign alone, and `the_curvature_sign_reproduces_the_primarys_asymmetry` asserts it.
//!
//! ON `M = -D K`. McNutt and Menard's eq. 4 prints `M = -D K`, and with the fibre stress above this module's `M`
//! (their eq. 3 verbatim) evaluates to `+D K` in the elastic limit. That is a difference in the sign convention
//! attached to the SYMBOL `M`, never a difference in physics: both give the same POSITIVE rigidity, since one
//! reads `D = -M/K` and the other `D = M/K` over moments of opposite sign. The rigidity is convention-free, which
//! is one more reason it is the canonical output, and [`equivalent_rigidity`] returns it positive from a moment
//! and curvature of matching sign.
//!
//! # TWO GEOMETRIES, ONE SCALAR YIELD LAW: THE LINE LOAD AND THE AXISYMMETRIC POINT LOAD
//!
//! `M = -D K` IS THE LINE-LOAD (cylindrical-bending, plane-strain) FORM AND IT IS NOT GENERAL. McNutt and
//! Menard's Appendix A prints, for the axisymmetric case, `M = -D (d2w/dr2 + (nu/r) dw/dr)` while the reported
//! curvature is the Laplacian `K = d2w/dr2 + (1/r) dw/dr`: the `nu/r` and the `1/r` DIFFER, so the moment is not
//! `-D K` for a circular load (fetch section 3, verified against the page at 230 and 500 dpi). An earlier slice
//! REFUSED the circular load rather than mixing the two geometries, and that refusal was CORRECT: this module
//! still never applies the line-load algebra to an axisymmetric geometry.
//!
//! What the primary-source fetch then settled, and what this module now ships beside the line load, is the true
//! axisymmetric construction. Two findings make it a GEOMETRY change rather than a new yield law. First, McNutt
//! and Menard solve the TRUE AXISYMMETRIC PLATE in `ker`/`kei` Bessel functions (their "cylindrical load" is an
//! axisymmetric disc, their word for the two-dimensional case is "rectangular"), and all three of their published
//! seamount rows back-solve onto the circular constants. Second, the fibre YIELD LAW is UNIAXIAL in both
//! primaries: the words "biaxial", "von Mises", "Tresca" and "hoop" appear nowhere in their yield formulation,
//! whose measure is the scalar differential stress `sigma_h - sigma_v`. So the earlier note that the axisymmetric
//! case "needs a two-dimensional yield surface" was the one thing the primaries refute: they use the same scalar
//! envelope this module already integrates, over the axisymmetric geometry. Building a biaxial surface would ship
//! a model the rows never used.
//!
//! So the axisymmetric path ([`solve_point_load`]) changes only the GEOMETRY: the deflection is `ker`/`kei`
//! (McNutt and Menard eq. A8), the DRIVING curvature at the first zero crossing is
//! `kappa_r + nu kappa_theta = d2w/dr2 + (nu/r) dw/dr` (the `M` operator; see
//! [`point_load_curvature_at_first_zero_crossing`]), and the rigidity is `D_eq = M_yield / kappa_eff`, which
//! recovers `D` in the elastic limit exactly as the line load does. The scalar yield envelope, the neutral-surface
//! solve, and the moment integral are all reused unchanged. Whether the uniaxial envelope is adequate at seamount
//! curvatures is a MEASURED cost, not an assumed one: at the hindcast curvatures (4 to 8 by 1e-8 per metre) the
//! competent plate is NEAR-ELASTIC on this engine's own envelope, the yielding ratio `T_e(YSE)/T_e(elastic)`
//! sitting within about six per cent of one (`the_uniaxial_cost_is_measured_at_the_hindcast_curvatures`), so the
//! plate yields only modestly and the two-dimensional yield surface stays the high-curvature refinement it always
//! was rather than a hindcast one. The exact departure is reported by that measurement rather than assumed away,
//! which is the shortcut-validity rule: the uniaxial law's cost is its measured departure from elastic, never a
//! claim of zero. The comparison against the oceanic `T_e`-versus-age rows stays in RIGIDITY SPACE (the rows are
//! `T_e(elastic)` fits, never moment-equivalence outputs), which is another slice's concern.
//!
//! THE ERRATUM THE AXISYMMETRIC FORM CARRIES. McNutt and Menard print the Laplacian curvature coefficient at the
//! first zero crossing as `-0.0289`; their own printed definition applied to their own printed deflection gives
//! `ker(x_0) = -0.0388994`, confirmed by two libraries and a finite-difference twin with the line-load controls
//! reproducing, so their published seamount curvatures run about 26 per cent low, cause UNATTRIBUTED. This module
//! recomputes the coefficient from the series ([`point_load_reported_curvature_coefficient`]) and never copies
//! the printed number, carrying the published value only for a consumer of the paper's own curvature-derived
//! constants (see [`mcnutt_menard_published_laplacian_coefficient`]). The finite disc is approximated by the
//! point load within the primary's own 2 per cent ([`disc_point_rigidity_band`]).
//!
//! # THE STRAIN RATE IS THE LOAD'S OWN, AND IT ARRIVES WITH ITS CHORD
//!
//! `T_e` is a chord over LOAD TIMESCALE, so the flexural envelope evaluates at the LOAD'S OWN strain rate.
//! [`crate::laws::convective_strain_rate`] is the MANTLE-AND-THERMAL chord and its own doc forbids this consumer
//! by name; nothing here reads it. The rate is not derivable from a static load magnitude (it is a property of
//! the load's emplacement history, which is why McNutt and Menard's eq. 16 reaches for the plate VELOCITY to get
//! one), so it is a REQUIRED INPUT and it is bundled into [`LoadChord`] beside the load timescale it came from.
//! A caller therefore cannot supply a rate without declaring the timescale that conditions it, which is the chord
//! made structural rather than remembered.
//!
//! THE SINGLE RATE IS THE PRIMARIES' OWN CONVENTION, and it is a DEFAULT TAKEN rather than a derivation. A
//! bending plate's strain rate is `epsilon-dot(z) = -(z - z_n) dK/dt`, which VARIES with depth and vanishes at
//! the neutral surface; all three sources in the fetch instead use one depth-independent rate for the whole
//! column (McNutt and Menard 1e-16 per second, Watts and Burov 1e-14 and 1e-15). This module follows them and
//! says so, rather than departing from both primaries inside a slice whose job is to reproduce them.
//!
//! # THE UNIT CONTRACT
//!
//! Unlike the flexure kernel this module is NOT unit-agnostic, because it consumes rows that are not: the brittle
//! branch is stated in MEGAPASCALS ([`crate::yield_envelope`]) and the creep rows in megapascals and gigapascals
//! ([`crate::creep_rows`]). The module's own system is THE FLEXURE KERNEL'S OWN documented coherent system,
//! `{length = km, mass = 1e12 kg, time = s}`, which induces stress in GIGAPASCALS, density in `1000 kg/m^3` (so a
//! 3300 kg/m^3 mantle reads 3.3), and gravity in `km/s^2` (so 9.8 m/s^2 reads 0.0098). It induces curvature in
//! `km^-1`, moment per unit length in `GPa km^2`, and rigidity in `GPa km^3`.
//!
//! THE SYSTEM IS CHOSEN FOR RANGE AND THE CHOICE IS FORCED. An Earth-like plate reads `K ~ 5e-4 km^-1`,
//! `M ~ 177 GPa km^2`, `D ~ 3.5e5 GPa km^3`, all comfortably inside the Q32.32 window. The same construction in
//! MEGAPASCALS puts `E * T_e^3` at 5.1e9 for a 40 km plate, which OVERFLOWS the window (about 2.1e9) inside the
//! landed [`crate::flexure::flexural_rigidity`] before this module ever sees it; in raw SI, `D ~ 3.5e23 N m`
//! overflows outright. So gigapascals is the currency that lets the landed kernel be consumed unmodified.
//!
//! Conversions to the consumed rows happen ONCE each, at the boundary, in [`LithosphereEnvelope`]: kilometres to
//! metres for the lithostatic axis, and megapascals to gigapascals on the way out of the brittle and ductile
//! rows.
//!
//! # HONEST LIMITS
//!
//! - THE MOMENT INTEGRAL DOES NOT ALWAYS SELF-TRUNCATE, and this contradicts the premise it was specified under.
//!   See [`bending_moment`], which measures the tail rather than assuming it dies. Where the tail lives, the
//!   integral is bounded at the CONDUCTIVE-LID BASE, derived from the world's own Rayleigh number and refereed
//!   against the convective stress scale ([`ConductiveLidBase`], [`referee_conductive_lid_base`]), because below
//!   that depth the mantle overturns and a static load's stresses are not sustained across it.
//! - THE LID REFEREE IS ONE-SIDED. It convicts a lid base too SHALLOW to have reached the convective stress
//!   scale and is blind to one too DEEP, since deeper material is weaker and passes more easily. A two-sided
//!   check needs a band around the crossing, and a band is a tolerance someone chose.
//! - `V*` IS A SPAN AND THE ENVELOPE ONLY ANSWERS WHERE THE SPAN CANNOT REACH THE ANSWER. The bracket is the
//!   covering determinations where the source's chords reach, and the table's own extremes where they do not
//!   ([`crate::creep_rows::ActivationVolumeBracket`]); [`LithosphereEnvelope`] reports a strength only where both
//!   ends agree to the bit. In the SHALLOW column they always do, which is what lets a lid be sampled from its
//!   own surface, and it is asserted rather than assumed.
//! - THE DEEP COLUMN IS THE COUPLED CONSEQUENCE, and it is a real one rather than a caveat. Today one `V*`
//!   determination is in play per fixture, so every bracket is degenerate and the whole column answers. BANKING
//!   H&K'S TABLE 2 IN FULL WOULD CHANGE THAT: several of its nine determinations cover any given lid pressure and
//!   they disagree by a factor of several in the resulting strength (its full -2 to 27 span is worth about six,
//!   41 MPa against 253, at 60 km on the Earth-like fixture below), so wherever the ductile branch binds the two
//!   ends part company and the envelope reports nothing. That is the primary declining to choose, carried
//!   faithfully rather than papered over, and what it costs is a deep ductile strength. Whether the deep envelope
//!   should then report a BAND (and `T_e` with it) rather than refuse is a ruling this module does not make.
//! - The curvature is read at ONE point, so the reported rigidity is a per-point quantity. McNutt and Menard
//!   state that "`D_eff` varies as the curvature changes along the profile" (their p. 369), so a single rigidity
//!   per load is a MATCHING CONVENTION shared with the literature rather than a property of the plate. Curvature
//!   is also a second derivative and the primary calls it "notoriously unstable", bounding its own method to
//!   curvatures above 1e-7 per metre.
//! - The envelope is an UPPER envelope with a declared band. The primaries find the laboratory flow law makes the
//!   plate too strong to fit the flexure data, wanting an effective activation energy near 418 kJ/mol against the
//!   530 the banked Hirth and Kohlstedt rows carry, and they flag that assigning the whole gap to `Q` was their
//!   own choice. The H&K rows are lab-measured and are not retuned; a derived rigidity that runs STIFF against a
//!   hindcast row is that declared band behaving.
//! - A DECOUPLED LID IS NOT MODELLED. Where a weak ductile layer separates two strong ones, bending stress does
//!   not transfer and `T_e` follows the Kirchhoff sum `(sum of h_l^3)^(1/3)` (Watts and Burov eq. 3), so two
//!   decoupled 20 km layers give about 25 km rather than 40. This module integrates one continuous column, which
//!   is the single-layer oceanic case the primaries call the simple one. A world whose lid decouples is
//!   over-stiffened by this construction, and detecting decoupling is not attempted here.

use crate::creep_rows::{
    ductile_strength_mpa, CreepCandidate, CreepConditions, CreepRefusal, VolumeEnd,
};
use crate::yield_envelope::{lithostatic_normal_stress_mpa, FrictionLaw};
use civsim_core::Fixed;

const ZERO: Fixed = Fixed::ZERO;

/// Megapascals per gigapascal: the ONE unit bridge between the landed rows' currency and this module's.
/// A dimensionless ratio between two named units, the same status as the `1000` that carries kilojoules to
/// joules inside the creep rows, never an authored physical value.
const MPA_PER_GPA: i32 = 1000;

/// Metres per kilometre: the other unit bridge, for the lithostatic axis, which is stated in metres.
const M_PER_KM: i32 = 1000;

/// The number of halvings the neutral-surface bisection takes.
///
/// AN ENGINE-CONVERGENCE BOUND, NOT WORLD CONTENT, and the same count and the same reasoning as the composite
/// creep bisection in [`crate::creep_rows`] and the eutectic bisection in `melting.rs`. The bracket is DERIVED
/// (the envelope's own depth domain), Q32.32 resolves `2^-32`, and 52 halvings drive a bracket of order a few
/// hundred kilometres to about `1e-13` km, far below the last bit the type can hold, so further steps change no
/// bit. The count is chosen PAST the point where it can move a result, which is what makes it a bound rather
/// than a tolerance.
const BISECTION_STEPS: u32 = 52;

/// The maximum number of fixed-point iterations the per-load solve takes before it reports non-convergence.
///
/// A COMPUTATIONAL BOUND, and the one place this module's structure differs from a bisection: the map
/// `D -> D_new` is iterated rather than bracketed, per the owner's ruling that the loop "carries its own
/// convergence test", and a fixed-point iteration has no derived step count the way a bisection does. So this
/// caps the walk and NON-CONVERGENCE IS REPORTED rather than papered over
/// ([`MomentEquivalenceRefusal::LoadExceedsElasticSupport`]). It cannot silently move a result: a solve that
/// hits the cap returns a refusal, never a number.
const MAX_FIXED_POINT_ITERATIONS: u32 = 200;

/// An opaque, caller-owned identifier for the LOAD CLASS a `T_e` was drawn from.
///
/// THE MODULE NEVER BRANCHES ON IT. It is carried so that a rigidity or a thickness cannot be quoted without the
/// population it belongs to, which is the hindcast's own LOADING ENVIRONMENT SPLIT (oceanic interior loads are
/// one set; trench loads are a separate tagged environment, because they diverge). Modelling the class as a
/// closed Rust enum would author a fixed list of load kinds into the engine, which is the templating defect
/// (Principle 8): the membership of the class registry is data and grows with the world, and the mechanism here
/// is fixed. So the class arrives as an identifier this module carries and never reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoadClassId(pub u64);

/// THE CHORD A `T_e` IS DRAWN OVER, carried with every output.
///
/// `T_e` is not a property of the lithosphere. It is the lithosphere JOINED TO A LOAD, so every value this
/// module reports carries the load's own conditioning: the CLASS and the TIMESCALE. The strain rate rides here
/// rather than as a loose argument precisely so it cannot be supplied without the timescale that conditions it.
#[derive(Clone, Copy, Debug)]
pub struct LoadChord {
    /// The load class, carried and never interpreted (see [`LoadClassId`]).
    pub class: LoadClassId,
    /// The LOAD'S OWN timescale (seconds): the time over which the load was emplaced and the plate bent to the
    /// curvature being read. This is the chord's second endpoint, and a `T_e` quoted without it is a statistic
    /// with a hidden conditioning variable.
    pub timescale_s: Fixed,
    /// `ln(epsilon-dot)` for the LOAD'S OWN strain rate, in reciprocal seconds, IN LOG SPACE.
    ///
    /// THE LOG IS THE ONLY REPRESENTABLE FORM, and the reason is [`crate::creep_rows::CreepConditions`]'s own: a
    /// lid's strain rate is about 1e-15 per second and Q32.32 resolves about 2.3e-10, so a bare `Fixed` rate
    /// rounds to zero and a law fed that zero returns an infinite strength with no symptom. Build it with
    /// [`crate::creep_rows::ln_scientific`], which reaches the logarithm without passing through the
    /// unrepresentable value.
    ///
    /// THIS IS THE LOAD'S RATE, NEVER THE MANTLE'S. See the module header.
    pub ln_strain_rate_per_s: Fixed,
}

/// THE CURVATURE, CARRYING ITS DEFLECTION CONVENTION IN ITS TYPE.
///
/// THE DEFENCE THIS EXISTS FOR, and it is the kilobar defence wearing flexure's coat. `T_e` depends on the SIGN
/// of the curvature rather than on its magnitude alone, because the yield envelope is asymmetric between tension and
/// compression. And the two conventions in play here DISAGREE ABOUT THAT SIGN: McNutt and Menard measure `w`
/// POSITIVE UPWARD (their p. 365), while the landed flexure kernel ([`crate::flexure::line_load_deflection`])
/// measures it POSITIVE DOWNWARD in the Turcotte and Schubert convention, its own doc calling the value under
/// the load "the downward moat". So `d2w/dx2` taken from the kernel has the OPPOSITE SIGN to the curvature this
/// construction's equations are written in, and a bare `Fixed` passed between them would invert the envelope's
/// asymmetry with every number still looking reasonable.
///
/// So the convention is a TYPE. There is no constructor from a bare number: a caller must name which deflection
/// convention their second derivative was taken in, and the flip happens in exactly one place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FibreCurvature {
    /// `d2w/dx2` in `km^-1`, normalized to the UPWARD-deflection convention (McNutt and Menard's own).
    upward: Fixed,
}

impl FibreCurvature {
    /// From `d2w/dx2` where `w` is POSITIVE UPWARD: McNutt and Menard's own convention, carried through.
    pub fn from_upward_deflection(second_derivative_per_km: Fixed) -> Self {
        FibreCurvature {
            upward: second_derivative_per_km,
        }
    }

    /// From `d2w/dx2` where `w` is POSITIVE DOWNWARD: the Turcotte and Schubert convention the landed flexure
    /// kernel returns ([`crate::flexure::line_load_deflection`]). The sign flips HERE, once.
    pub fn from_downward_deflection(second_derivative_per_km: Fixed) -> Self {
        FibreCurvature {
            upward: ZERO - second_derivative_per_km,
        }
    }

    /// The curvature in the upward-deflection convention, `km^-1`.
    pub fn upward_per_km(self) -> Fixed {
        self.upward
    }
}

/// WHICH ANDERSONIAN FAULTING SENSE a fibre fails in. Derived from the fibre's own stress state rather than
/// supplied: a fibre in horizontal tension fails by NORMAL faulting, one in horizontal compression by THRUST
/// faulting. This is a fact about the physics (the vertical stress is lithostatic, so it is the greatest
/// principal stress in extension and the least in compression), never a world-content choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultingSense {
    /// Horizontal compression: the vertical stress is the LEAST compressive principal stress.
    Thrust,
    /// Horizontal extension: the vertical stress is the GREATEST compressive principal stress.
    Normal,
}

/// The differential stress a brittle law reports at a state, mirroring the shape of
/// [`crate::yield_envelope::ShearStrength`] for the same reason and deliberately NOT reusing that type.
///
/// THE TYPES ARE KEPT APART BECAUSE THE QUANTITIES ARE DIFFERENT. `ShearStrength` carries the SHEAR stress on a
/// fault plane; this carries the DIFFERENTIAL stress on a horizontal fibre, which is what the moment integral
/// eats. Putting a differential stress inside a type named `ShearStrength` would be one name bound to two
/// quantities, which is the exact defect the arc's own `T_e`/`T_mech` rename was called to retire. Same shape,
/// different quantity, separate type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DifferentialStrength {
    /// A determined magnitude (megapascals).
    Determined(Fixed),
    /// THE BRACKET: no branch of the friction law is licensed at this state, so the two fits are reported
    /// spanned and ordered with NO point chosen between them. Inherited from the friction row's own gap
    /// structure: rock is a crossover law and never brackets, ice's branches do not meet and it does.
    Bracket { low: Fixed, high: Fixed },
}

/// THE MOHR-COULOMB RESOLUTION: a friction law's `tau = S0 + mu * sigma_n` resolved onto optimally oriented
/// faults, giving the DIFFERENTIAL STRESS at first yielding and the fault-normal stress it yields at.
///
/// This is the step McNutt and Menard take between Byerlee's law and their envelope, and it is why their eqs. 7
/// and 8 are in differential stress while eq. 6 is in shear. The moment integral eats fibre stress, so the
/// resolution is not optional: handing it a shear strength would be a quantity error wearing a plausible number.
///
/// THE ALGEBRA. The failure line is tangent to the Mohr circle when `(mu c + S0) / sqrt(1 + mu^2) = R`, with `c`
/// the circle centre and `R` its radius, which rearranges to `sigma_1 = Q sigma_3 + C S0` for
/// `Q = (sqrt(1+mu^2) + mu) / (sqrt(1+mu^2) - mu)` and `C = 2 / (sqrt(1+mu^2) - mu)`. The tangency point sits at
/// `sigma_n = c - R sin(phi)` with `sin(phi) = mu / sqrt(1+mu^2)`.
///
/// THIS IS REFEREED AGAINST THE PRIMARY'S OWN PRINTED COEFFICIENTS, which is what makes it a reading rather than
/// a derivation this module marks its own homework on. With `mu = 0.6` and `S0 = 80 MPa` the algebra returns
/// `-2.119 rho g z - 282.6` in compression and `+0.679 rho g z + 90.6` in tension, against McNutt and Menard's
/// printed eqs. 7 and 8: `-2.17 rho g z - 283` and `+0.68 rho g z + 89`. See
/// `the_mohr_coulomb_resolution_reproduces_the_primarys_printed_envelope`.
///
/// THE SHALLOW TENSILE LIMIT IS THE SOURCE'S OWN AND IT RIDES ALONG. At zero depth the normal-faulting branch
/// returns a nonzero differential strength (about 90 MPa for `mu = 0.6`, `S0 = 80 MPa`), because the Coulomb
/// construction places the least principal stress in TENSION there, where Coulomb friction does not describe the
/// rock and a tensile failure criterion would. McNutt and Menard's printed eq. 8 carries exactly this artifact
/// (its intercept is their 89 MPa), so reproducing it is fidelity to the source rather than a defect introduced
/// here; a world whose shallowest kilometres matter needs a tensile criterion this envelope does not have.
///
/// Returns the differential-stress MAGNITUDE (megapascals, non-negative) and the fault-normal stress at failure
/// (megapascals), the latter being what selects the friction law's branch. `None` on a negative vertical stress
/// or an out-of-range intermediate, never a fabricated strength.
pub fn mohr_coulomb_differential_mpa(
    friction_coefficient: Fixed,
    cohesion_mpa: Fixed,
    vertical_stress_mpa: Fixed,
    sense: FaultingSense,
) -> Option<(Fixed, Fixed)> {
    if vertical_stress_mpa < ZERO || friction_coefficient < ZERO || cohesion_mpa < ZERO {
        return None;
    }
    let mu = friction_coefficient;
    let root = Fixed::ONE.checked_add(mu.checked_mul(mu)?)?.sqrt();
    let denom = root.checked_sub(mu)?;
    if denom <= ZERO {
        return None;
    }
    let q = root.checked_add(mu)?.checked_div(denom)?;
    let c_coeff = Fixed::from_int(2).checked_div(denom)?;
    let c_s0 = c_coeff.checked_mul(cohesion_mpa)?;

    // The two principal stresses at failure, as COMPRESSION-POSITIVE magnitudes. The tension-positive sign is
    // applied by the caller, which is where the fibre's own sense is known.
    let (sigma_1, sigma_3) = match sense {
        // Thrust: the vertical stress is the LEAST compressive, so sigma_3 = rho g z.
        FaultingSense::Thrust => {
            let s1 = q.checked_mul(vertical_stress_mpa)?.checked_add(c_s0)?;
            (s1, vertical_stress_mpa)
        }
        // Normal: the vertical stress is the GREATEST compressive, so sigma_1 = rho g z and sigma_3 follows by
        // inverting the same relation.
        FaultingSense::Normal => {
            let s3 = vertical_stress_mpa.checked_sub(c_s0)?.checked_div(q)?;
            (vertical_stress_mpa, s3)
        }
    };
    let differential = sigma_1.checked_sub(sigma_3)?;
    if differential < ZERO {
        return None;
    }
    // The fault-normal stress at the tangency point, which is the axis the friction law states its domains on.
    let centre = sigma_1
        .checked_add(sigma_3)?
        .checked_div(Fixed::from_int(2))?;
    let radius = differential.checked_div(Fixed::from_int(2))?;
    let sin_phi = mu.checked_div(root)?;
    let normal_at_failure = centre.checked_sub(radius.checked_mul(sin_phi)?)?;
    Some((differential, normal_at_failure))
}

/// THE BRITTLE BRANCH IN DIFFERENTIAL STRESS: a friction law resolved onto optimal faults, with the branch
/// selected by the law's OWN STATED DOMAIN on the fault-normal stress at failure.
///
/// # WHY THE BRANCH CANNOT BE READ OFF THE DEPTH
///
/// A friction law states its branches on the FAULT-NORMAL stress, and the fault-normal stress at failure is
/// itself an output of the resolution (it depends on `mu`, which is the very thing the branch selects). So each
/// branch is tried and each is asked whether ITS OWN tangency landed inside ITS OWN stated domain. That is the
/// landed row's own philosophy carried one step further: domains are stated, and a branch is licensed exactly
/// where it says it is.
///
/// # WHERE SEVERAL BRANCHES ARE LICENSED, THE SMALLEST CIRCLE WINS, AND THAT IS DERIVED
///
/// A rock fails at FIRST yielding, so among the licensed branches the operative one is the SMALLEST Mohr circle,
/// which is the smallest differential stress. Nothing is chosen: the minimum is what "first yielding" means. For
/// Byerlee's rock law both branches are self-consistent over a narrow window of vertical stress (about 104 to
/// 121 MPa in thrust), where they differ by about one percent, and the minimum picks the operative one across it
/// with no discontinuity worth the name.
///
/// # WHERE NO BRANCH IS LICENSED, IT BRACKETS
///
/// That is ice, and it is why the landed row split its domains in the first place: Beeman's low fit is licensed
/// at or below 5 MPa and the high fit at or above 10, and the two do not cross inside the gap. Both fits are
/// reported, spanned and ordered, with no point chosen. A consumer that needs a number there is asking the
/// calibration for something it does not have.
///
/// `None` on a negative vertical stress or an out-of-range intermediate.
pub fn brittle_differential_mpa(
    law: &FrictionLaw,
    vertical_stress_mpa: Fixed,
    sense: FaultingSense,
) -> Option<DifferentialStrength> {
    let (low_d, low_n) = mohr_coulomb_differential_mpa(
        law.low_coefficient,
        law.low_cohesion,
        vertical_stress_mpa,
        sense,
    )?;
    let (high_d, high_n) = mohr_coulomb_differential_mpa(
        law.high_coefficient,
        law.high_cohesion,
        vertical_stress_mpa,
        sense,
    )?;
    let low_licensed = low_n < law.low_domain_max;
    let high_licensed = high_n >= law.high_domain_min;
    // THE LOW BRANCH'S ROUGHNESS BAND, where the material has a measured one AND the fault-normal stress sits in its
    // scatter domain. Byerlee's low fit is a central line through roughness scatter below his 50-bar boundary; above
    // it the intermediate fit is tight, so the band is REGIME-SCOPED, never smeared across the whole low branch (that
    // would overstate the scatter forty times in depth). In the roughness regime the honest strength is the INTERVAL
    // the scatter spans, resolved through the SAME Mohr-Coulomb construction at the band's edges. At or above the
    // boundary the intermediate residual band applies once a source pins it; until then it is `None`, the central fit
    // alone (band-pending, absence as datum, never a fabricated zero width). Strength rises with both coefficient and
    // cohesion, so the lo-lo edge is the weak end and hi-hi the strong one; ordered here so a consumer reads low first.
    let low_band = match law.low_stress_band {
        Some(b) => {
            // The regime is keyed on the CENTRAL fit's fault-normal stress: below the scatter boundary, the roughness
            // cloud's coefficient-and-cohesion edges; at or above, the intermediate residual around the central fit
            // (its cohesion is the central low cohesion), or `None` while that residual read is pending.
            let edges = if low_n < b.scatter_domain_max {
                Some((
                    b.coefficient_lo,
                    b.cohesion_lo,
                    b.coefficient_hi,
                    b.cohesion_hi,
                ))
            } else {
                b.intermediate_band
                    .map(|(clo, chi)| (clo, law.low_cohesion, chi, law.low_cohesion))
            };
            match edges {
                Some((clo, coh_lo, chi, coh_hi)) => {
                    let (weak, _) =
                        mohr_coulomb_differential_mpa(clo, coh_lo, vertical_stress_mpa, sense)?;
                    let (strong, _) =
                        mohr_coulomb_differential_mpa(chi, coh_hi, vertical_stress_mpa, sense)?;
                    Some(if weak <= strong {
                        (weak, strong)
                    } else {
                        (strong, weak)
                    })
                }
                None => None,
            }
        }
        None => None,
    };
    // A (lo, hi) interval collapses to a determined strength when its ends coincide, else it is the ordered bracket.
    let interval = |lo: Fixed, hi: Fixed| {
        if lo == hi {
            DifferentialStrength::Determined(lo)
        } else {
            DifferentialStrength::Bracket { low: lo, high: hi }
        }
    };
    match (low_licensed, high_licensed) {
        // First yielding is the smallest circle, so the minimum of the licensed branches is operative. Where the
        // low branch carries a roughness band, "first yielding" is the interval-min of that band with the high
        // POINT, endpoint-wise. A bracket-versus-scalar min is exact regardless of correlation, so no covariance
        // question arises here (that would need two brackets meeting).
        (true, true) => Some(match low_band {
            Some((lo, hi)) => interval(lo.min(high_d), hi.min(high_d)),
            None => DifferentialStrength::Determined(low_d.min(high_d)),
        }),
        // Only the low branch is licensed: the roughness-scattered regime, where the band (once set) IS the strength.
        (true, false) => Some(match low_band {
            Some((lo, hi)) => interval(lo, hi),
            None => DifferentialStrength::Determined(low_d),
        }),
        (false, true) => Some(DifferentialStrength::Determined(high_d)),
        // Neither fit is in its stated domain: report the envelope the two span, choose nothing.
        (false, false) => Some(if low_d <= high_d {
            DifferentialStrength::Bracket {
                low: low_d,
                high: high_d,
            }
        } else {
            DifferentialStrength::Bracket {
                low: high_d,
                high: low_d,
            }
        }),
    }
}

/// THE YIELD ENVELOPE the moment integral bounds its fibre stress by: the differential stress the material
/// sustains at a depth, in TENSION and in COMPRESSION separately, as non-negative MAGNITUDES in GIGAPASCALS.
///
/// A TRAIT RATHER THAN A STRUCT, because the envelope's MEMBERSHIP is world data while the moment-equivalence
/// MECHANISM is fixed Rust. [`LithosphereEnvelope`] is the silicate-or-ice lid assembly (brittle above, ductile
/// below); a world whose lid is something this arc has not met supplies its own implementation and every
/// function in this module serves it unchanged. That is the alien admitted as a data row rather than a rewrite.
///
/// THE ASYMMETRY IS THE WHOLE POINT of separating the two senses. Rock is about three times stronger in
/// compression than in tension, which is what makes `T_e` depend on the SIGN of the curvature rather than on its
/// magnitude alone.
pub trait YieldEnvelope {
    /// The differential stress the material sustains in TENSION at this depth (gigapascals, non-negative).
    /// `None` where the envelope cannot answer, which refuses the whole construction rather than guessing.
    fn tensile_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed>;
    /// The differential stress the material sustains in COMPRESSION at this depth (gigapascals, non-negative).
    fn compressive_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed>;
    /// The maximum depth (km) over which this envelope is VALID.
    ///
    /// THIS IS THE ENVELOPE'S OWN DOMAIN, NOT A FLOOR THIS MODULE AUTHORS. It is the depth below which the
    /// caller's envelope stops describing anything (physically, the base of the lid: below it the interior
    /// convects and carries no long-term fibre stress). The moment integral runs over it and REPORTS whether the
    /// answer depended on it ([`MomentReading::self_truncated`]), so a domain-limited moment can never be
    /// mistaken for a converged one.
    ///
    /// THE TRAIT TAKES THE DOMAIN ON TRUST AND THE LANDED ASSEMBLY DOES NOT. [`LithosphereEnvelope`] DERIVES
    /// this from the world's own Rayleigh number ([`ConductiveLidBase`]) and cannot be built with a declared
    /// one. The trait stays open because a world whose lid base is set by something this arc has not met (an ice
    /// shell floored by its own ocean, say) implements it directly, and that is the alien arriving as a data row
    /// rather than a rewrite.
    fn domain_max_depth_km(&self) -> Fixed;
}

/// THE SAMPLED ENVELOPE: a yield envelope evaluated once onto a uniform depth grid.
///
/// WHY IT EXISTS. The envelope is the expensive part (each ductile reading is a composite creep bisection),
/// while the neutral-surface solve and the fixed point each re-integrate the same column many times over. The
/// envelope does NOT depend on the curvature or on the neutral surface (only the ELASTIC stress does), so it is
/// sampled once and reused, which turns an otherwise cubic walk into one linear sampling plus cheap arithmetic.
///
/// THE GRID RESOLUTION IS THE CALLER'S DECLARED SAMPLING, never a constant this module hides. It is visible as
/// [`EnvelopeProfile::step_km`], and a caller that wants it derived doubles the grid until the reported moment
/// stops moving by more than the last bit the type can hold, which is what
/// `the_moment_is_converged_in_the_grid_it_is_sampled_on` does.
#[derive(Clone, Debug)]
pub struct EnvelopeProfile {
    step_km: Fixed,
    /// The tensile yield magnitude (GPa) at `z = i * step_km`, `i` in `0..=steps`.
    tensile_gpa: Vec<Fixed>,
    /// The compressive yield magnitude (GPa) at the same nodes.
    compressive_gpa: Vec<Fixed>,
    /// THE SUFFIX MAXIMUM: `suffix_max_gpa[i]` is the greatest yield magnitude, in either sense, at any node at
    /// or below `i`. It is what makes the moment integral's tail bound RIGOROUS rather than extrapolated: the
    /// fibre stress is clamped into `[-compressive, +tensile]`, so `|sigma_f(z)|` is at most this at every
    /// remaining depth, and the whole remaining tail is bounded by it times the greatest remaining lever arm
    /// times the remaining depth. Computed once, backwards, at sampling time.
    suffix_max_gpa: Vec<Fixed>,
}

impl EnvelopeProfile {
    /// Sample an envelope onto a uniform grid of `steps` intervals over `[0, domain_max_depth_km]`.
    ///
    /// `None` on a zero step count or a non-positive domain, or where the envelope refuses at any node (which
    /// refuses the profile rather than interpolating across a hole).
    pub fn sample(envelope: &dyn YieldEnvelope, steps: u32) -> Option<Self> {
        if steps == 0 {
            return None;
        }
        let domain = envelope.domain_max_depth_km();
        if domain <= ZERO {
            return None;
        }
        let step = domain.checked_div(Fixed::from_int(i32::try_from(steps).ok()?))?;
        if step <= ZERO {
            return None;
        }
        let mut tensile = Vec::with_capacity(steps as usize + 1);
        let mut compressive = Vec::with_capacity(steps as usize + 1);
        for i in 0..=steps {
            let z = step.checked_mul(Fixed::from_int(i32::try_from(i).ok()?))?;
            let t = envelope.tensile_yield_gpa(z)?;
            let c = envelope.compressive_yield_gpa(z)?;
            if t < ZERO || c < ZERO {
                return None;
            }
            tensile.push(t);
            compressive.push(c);
        }
        // The suffix maximum, backwards: the greatest strength the envelope still has at or below each node.
        let mut suffix = vec![ZERO; tensile.len()];
        let mut running = ZERO;
        for i in (0..tensile.len()).rev() {
            running = running.max(tensile[i]).max(compressive[i]);
            suffix[i] = running;
        }
        Some(EnvelopeProfile {
            step_km: step,
            tensile_gpa: tensile,
            compressive_gpa: compressive,
            suffix_max_gpa: suffix,
        })
    }

    /// The grid spacing (km).
    pub fn step_km(&self) -> Fixed {
        self.step_km
    }

    /// The number of grid intervals.
    pub fn steps(&self) -> usize {
        self.tensile_gpa.len().saturating_sub(1)
    }

    /// The depth of node `i` (km).
    pub fn depth_km(&self, i: usize) -> Fixed {
        self.step_km * Fixed::from_int(i as i32)
    }

    /// The deepest node (km), which is the envelope's declared domain as the grid realizes it.
    pub fn domain_max_depth_km(&self) -> Fixed {
        self.depth_km(self.steps())
    }
}

/// `E / (1 - nu^2)`, the plane-strain modulus the fibre stress carries (GPa).
///
/// Its presence is what makes the purely elastic plate return `T_e = H` exactly, because the same factor sits in
/// `D`'s denominator and cancels through the cube root. Dropping it would put a `(1 - nu^2)^(1/3)` error into
/// every thickness (about 2 percent at `nu = 0.25`), which is small enough to look like noise and is exactly why
/// the elastic-limit identity is a test rather than a comment.
fn plane_strain_modulus_gpa(youngs_modulus_gpa: Fixed, poisson_ratio: Fixed) -> Option<Fixed> {
    if youngs_modulus_gpa <= ZERO {
        return None;
    }
    let nu2 = poisson_ratio.checked_mul(poisson_ratio)?;
    let one_minus_nu2 = Fixed::ONE.checked_sub(nu2)?;
    if one_minus_nu2 <= ZERO {
        return None;
    }
    youngs_modulus_gpa.checked_div(one_minus_nu2)
}

/// The fibre stress at one node (GPa) under the tension-positive convention: the ELASTIC stress capped by the
/// envelope's own strength in whichever sense the fibre is loaded.
///
/// `sigma_elastic = [E / (1 - nu^2)] * K * (z - z_n)`, then clamped into `[-compressive, +tensile]`. This is the
/// "bounded by the yield envelope" of the source's own sentence: the un-yielded core is elastic and carries the
/// moduli, and only the yielded fibres read the envelope.
fn fibre_stress_gpa(
    profile: &EnvelopeProfile,
    node: usize,
    curvature: FibreCurvature,
    neutral_depth_km: Fixed,
    plane_strain: Fixed,
) -> Option<Fixed> {
    let z = profile.depth_km(node);
    let lever = z.checked_sub(neutral_depth_km)?;
    let elastic = plane_strain
        .checked_mul(curvature.upward_per_km())?
        .checked_mul(lever)?;
    let tensile = *profile.tensile_gpa.get(node)?;
    let compressive = *profile.compressive_gpa.get(node)?;
    Some(elastic.clamp(ZERO - compressive, tensile))
}

/// The trapezoidal net axial force `N = integral of sigma_f dz` (`GPa km`) over the profile, which is the
/// quantity the neutral surface zeroes.
fn axial_force(
    profile: &EnvelopeProfile,
    curvature: FibreCurvature,
    neutral_depth_km: Fixed,
    plane_strain: Fixed,
) -> Option<Fixed> {
    let n = profile.steps();
    let mut acc = ZERO;
    for i in 0..=n {
        let s = fibre_stress_gpa(profile, i, curvature, neutral_depth_km, plane_strain)?;
        // Trapezoid: the endpoints carry half weight.
        let w = if i == 0 || i == n {
            s.checked_div(Fixed::from_int(2))?
        } else {
            s
        };
        acc = acc.checked_add(w)?;
    }
    acc.checked_mul(profile.step_km)
}

/// What the moment integral found, including whether it needed the envelope's whole domain to find it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MomentReading {
    /// The bending moment per unit length (`GPa km^2`), tension-positive convention,
    /// `M = integral of sigma_f (z - z_n) dz`.
    pub moment: Fixed,
    /// The neutral surface the moment was taken about (km).
    pub neutral_depth_km: Fixed,
    /// WHETHER THE INTEGRAL SELF-TRUNCATED on its own residue budget, and it STAYS as the honest flag now that
    /// the domain is derived rather than declared. `true` means the integrand died inside the envelope's domain,
    /// so the moment is INDEPENDENT of where that domain sits. `false` means the integral ran to the domain edge
    /// with the tail still above the budget, so THE MOMENT'S SECOND PARENT IS THE DOMAIN, and for the landed lid
    /// assembly that domain is the derived conductive-lid base ([`ConductiveLidBase`]). The dependence is real
    /// either way; the flag is what keeps it from being absorbed silently.
    pub self_truncated: bool,
    /// The depth (km) at which the integrand's remaining tail fell below the residue budget, where it did.
    pub truncation_depth_km: Option<Fixed>,
    /// The magnitude of the contribution the last integrated interval added (`GPa km^2`), which is the MEASURED
    /// tail rather than an assumed one.
    pub final_interval_contribution: Fixed,
}

/// THE BENDING MOMENT of the yield-limited plate about its neutral surface: McNutt and Menard's eq. 3, evaluated
/// on a sampled envelope.
///
/// # THE RESIDUE BUDGET, DERIVED, AND THE PREMISE IT DISPROVED
///
/// The slice was specified under a premise: that ductile strength decays exponentially with depth, so the
/// integrand's tail is bounded and the integration SELF-TRUNCATES where the remaining tail falls below the
/// integral's own residue budget. The budget is derived here exactly as ruled, and it is the project's own
/// currency for a tolerance: THE ACCUMULATOR'S OWN RESOLUTION, `Fixed::EPSILON`, one part in `2^32`.
///
/// The tail is BOUNDED RATHER THAN EXTRAPOLATED, which is what makes the stop provable. The fibre stress is
/// clamped into `[-compressive, +tensile]`, so `|sigma_f|` is at most the envelope's SUFFIX MAXIMUM at every
/// remaining depth ([`EnvelopeProfile`]), and
///
/// `|integral from Z to D of sigma_f (z - z_n) dz| <= suffix_max(Z) * max|z - z_n| * (D - Z)`
///
/// is an INEQUALITY the envelope itself supplies. The integration stops where that bound can no longer move the
/// accumulated moment by one representable unit. Nothing is authored: no depth, no floor, no stress threshold.
/// The budget is read off the representation, the bound is computed from the caller's own envelope, and the stop
/// is where further work provably changes no bit.
///
/// THE PREMISE IS FALSE FOR THE CREEP ENVELOPE, and the integral measures that rather than assuming its way past
/// it. A power-law creep row has a STRENGTH FLOOR: `sigma = (epsilon-dot / A)^(1/n) * exp((E* + P V*)/(n R T))`
/// tends to `(epsilon-dot / A)^(1/n)` as temperature rises without bound, which for the banked dry-olivine row at
/// 1e-15 per second is about 2 pascals. It is tiny, it never reaches zero, and the lever arm `(z - z_n)` grows
/// linearly, so the integrand tends to a linearly GROWING function and the integral diverges. It is worse than
/// that for a saturating geotherm: [`crate::geotherm::halfspace_geotherm`] tends to the interior temperature as
/// the error function saturates, and with `T` fixed the `P V*` term makes the deep material STRONGER with depth,
/// so the integrand turns and climbs. McNutt and Menard's own integral converges only because their geotherm
/// (their eq. 11) is LINEAR in depth and therefore unphysically hot below the lid, which drives their creep
/// strength to zero. Ours is not.
///
/// So the honest structure is: the budget truncates where the integrand does die (a brittle-capped or
/// synthetic envelope), and where it does not, the integral runs to the envelope's own domain and says so
/// through [`MomentReading::self_truncated`]. THE FLAG STAYS, and it stays honest in both directions: where the
/// integrand dies before the domain, the moment is independent of where the domain sits and the flag says so;
/// where it does not, the domain IS the answer's second parent and the flag says that instead of absorbing it.
///
/// THE DOMAIN IS NO LONGER A DECLARED NUMBER, which is the half this function used to have to confess. For the
/// lid assembly it is the CONDUCTIVE-LID BASE, derived from the world's own Rayleigh number and refereed against
/// the convective stress scale ([`ConductiveLidBase`], [`referee_conductive_lid_base`]), and the justification
/// is physical rather than proximal: below `delta` the mantle overturns, so a static load's stresses are not
/// sustained there and there is no fibre stress left to integrate. This function still asks only the
/// [`YieldEnvelope`] trait for a domain, so a world whose lid is something this arc has not met supplies its own
/// and is served unchanged.
///
/// `None` on an out-of-range intermediate or a degenerate modulus.
pub fn bending_moment(
    profile: &EnvelopeProfile,
    curvature: FibreCurvature,
    neutral_depth_km: Fixed,
    youngs_modulus_gpa: Fixed,
    poisson_ratio: Fixed,
) -> Option<MomentReading> {
    let plane_strain = plane_strain_modulus_gpa(youngs_modulus_gpa, poisson_ratio)?;
    let n = profile.steps();
    if n == 0 {
        return None;
    }
    let integrand = |i: usize| -> Option<Fixed> {
        let s = fibre_stress_gpa(profile, i, curvature, neutral_depth_km, plane_strain)?;
        let lever = profile.depth_km(i).checked_sub(neutral_depth_km)?;
        s.checked_mul(lever)
    };

    let mut acc = ZERO;
    let mut truncation_depth: Option<Fixed> = None;
    let mut final_contribution = ZERO;
    let domain = profile.domain_max_depth_km();

    for i in 0..n {
        let a = integrand(i)?;
        let b = integrand(i + 1)?;
        let contribution = a
            .checked_add(b)?
            .checked_div(Fixed::from_int(2))?
            .checked_mul(profile.step_km)?;
        acc = acc.checked_add(contribution)?;
        final_contribution = contribution.abs();

        // THE SELF-TRUNCATION TEST, on the derived residue budget, with a RIGOROUS tail bound. The fibre stress
        // is clamped into `[-compressive, +tensile]`, so at every remaining depth `|sigma_f|` is at most the
        // envelope's SUFFIX MAXIMUM; the lever arm `|z - z_n|` is at most its greatest remaining value; and the
        // remaining depth is what it is. Their product bounds the whole remaining tail:
        //
        //   |integral from z_i to D of sigma_f (z - z_n) dz| <= suffix_max(z_i) * max|z - z_n| * (D - z_i)
        //
        // Stop when that bound cannot move the accumulated moment by ONE REPRESENTABLE UNIT (`Fixed::EPSILON`,
        // the accumulator's own resolution). Nothing is authored: the bound is computed from the envelope the
        // caller supplied and the budget is read off the representation.
        //
        // THE BOUND IS AN INEQUALITY, NOT AN EXTRAPOLATION, which matters more than it looks. An earlier form
        // bounded the tail geometrically from the integrand's OBSERVED decay ratio, and it was wrong twice over:
        // it could not tell a dead tail from the integrand's legitimate interior zero AT THE NEUTRAL SURFACE, so
        // it truncated at `z_n` and returned exactly half the moment; and a decay ratio creeping toward one from
        // below would have under-bounded a live tail. The suffix maximum has neither failure: it looks at the
        // whole remaining domain rather than at one local ratio, so it cannot be fooled by a zero it is standing
        // on, and it cannot under-report a tail that revives.
        // The final interval is the domain edge, not a tail: there is nothing past it to bound, so the bound is
        // trivially zero and truncating on it would report "self-truncated" for every envelope ever integrated,
        // including the ones whose tails are alive. Reaching the edge is the DOMAIN-LIMITED case and must read
        // as such.
        if i + 1 == n {
            break;
        }
        let z_i = profile.depth_km(i + 1);
        let remaining = domain.checked_sub(z_i)?;
        let suffix_max = *profile.suffix_max_gpa.get(i + 1)?;
        let max_lever = z_i
            .checked_sub(neutral_depth_km)?
            .abs()
            .max(domain.checked_sub(neutral_depth_km)?.abs());
        let tail_bound = suffix_max
            .checked_mul(max_lever)
            .and_then(|x| x.checked_mul(remaining));
        if let Some(bound) = tail_bound {
            if bound <= Fixed::EPSILON {
                truncation_depth = Some(z_i);
                break;
            }
        }
    }

    Some(MomentReading {
        moment: acc,
        neutral_depth_km,
        self_truncated: truncation_depth.is_some(),
        truncation_depth_km: truncation_depth,
        final_interval_contribution: final_contribution,
    })
}

/// Why the moment-equivalence construction refused. Every variant is a refusal to answer, never a degraded
/// number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MomentEquivalenceRefusal {
    /// THE LOAD EXCEEDS WHAT THE ENVELOPE CAN ELASTICALLY CARRY: the per-load fixed point did not converge.
    ///
    /// NOTHING IS BUILT HERE FOR THIS BRANCH, per the owner's ruling. It routes to the SUPPORT-BOUND AND
    /// VISCOUS-RELAXATION branch that already exists, `civsim_sim::deeptime::relax_to_support_bound` ("THE
    /// SUPPORT-BOUND COLLAPSE": the isostatic relief relaxed to `sigma_y / (rho g)` with the excess crust flowed
    /// to the lows, mass conserved to the bit). That module is DOWNSTREAM of this crate, so this refusal NAMES
    /// the branch and stops. The routing is a later slice's wiring; no support bound is recomputed here.
    LoadExceedsElasticSupport,
    /// THE FIXED POINT DID NOT CONVERGE, a NUMERICAL state, distinct from [`Self::LoadExceedsElasticSupport`]
    /// which is a PHYSICAL one. The iteration was exhausted without the D-trajectory ever going non-positive, so
    /// the load is NOT known to exceed elastic support; the map simply did not settle within one ulp. In
    /// deterministic fixed-point arithmetic a near-marginal load presents exactly this way, oscillating in a
    /// limit cycle between adjacent representable rigidities wider than the absolute one-ulp convergence test can
    /// close. Filing that as `LoadExceedsElasticSupport` would route a numerical residual to the support-bound
    /// branch and relax topography a converged solve would have carried, so the two are split. The field carries
    /// the FINAL delta (the last step's `|D_new - D|`), surfaced per the residual discipline so the non-closure
    /// is inspectable rather than invisible; a delta a few ulp wide is the two-cycle signature.
    FixedPointDidNotConverge { final_delta_gpa_km3: Fixed },
    /// The curvature read at the evaluation point was zero, so the moment equivalence has no rigidity to report
    /// (`D = M / K` is undefined at zero curvature, and an unbent plate reveals nothing about its own strength).
    ZeroCurvature,
    /// The zero-net-axial-force condition has no root inside the envelope's depth domain, so the neutral surface
    /// is not locatable against this envelope. Escalate rather than pin it at mid-plate: the primaries' own
    /// envelope is asymmetric, so a pinned mid-plate neutral surface is an assumption they do not make and their
    /// own text contradicts.
    NeutralSurfaceNotBracketed,
    /// The envelope refused, or the profile could not be sampled.
    EnvelopeRefused,
    /// A BANDED SOLVE'S TWO EDGES DISAGREED ABOUT WHETHER THE PLATE HOLDS: one edge of the `V*` band converged to
    /// a rigidity and the other hit [`Self::LoadExceedsElasticSupport`]. That is a FINDING, not an average: the
    /// source's own scatter in `V*` straddles the boundary between an elastically supported load and one that
    /// exceeds it, so the load's support is UNDECIDED across the band and the honest report is that it is, never a
    /// rigidity that silently picked the surviving edge. The field names which edge converged.
    BandEdgeSupportDisagrees { low_edge_converged: bool },
    /// A BANDED SOLVE'S RIGIDITY BAND CAME OUT UNORDERED: the low `V*` edge returned a rigidity ABOVE the high
    /// edge, contradicting the monotonicity the interval-arithmetic license rests on. Never observed in the banked
    /// data or the adversarial sweep, and if it ever fires it is the license itself failing, which the ruling says
    /// is a stop rather than a swap. Carries both rigidities so the violation is inspectable.
    BandRigidityUnordered {
        low_gpa_km3: Fixed,
        high_gpa_km3: Fixed,
    },
    /// The fixed-point arithmetic left the representable window.
    NotRepresentable,
}

/// THE NEUTRAL SURFACE, from the ZERO-NET-AXIAL-FORCE condition.
///
/// The source's sentence is "the fibre stresses must sum to zero over the thickness of the plate", which read
/// against its eq. 2 (`N = integral of delta-sigma dz`) is `N = 0`, and that is what fixes `z_n`.
///
/// THIS STEP IS AN INFERENCE AND CARRIES THAT GRADE. Neither primary prints an equation solving for `z_n`; the
/// fetch grades it a sound inference from what the source prints (its section 2.4, the same grade the H&K
/// verification gave its own chord reading), and this module does not upgrade it. The fetch could not reach
/// Goetze and Evans 1979, which is where the solve would most likely be printed.
///
/// # WHY IT IS NOT PINNED AT MID-PLATE
///
/// McNutt and Menard's worked illustration sets `z_n = 20 km` for `H = 40 km` and their text mentions "the
/// observation that the neutral axis of bending occurs at mid-plate depths", which reads like a licence to pin
/// it. It is not one. Their envelope is strongly ASYMMETRIC between tension and compression, so a yielded stress
/// profile cannot be symmetric about mid-plate and still integrate to zero; and they treat `z_n` as a model
/// OUTPUT that a change of envelope RELOCATES, reporting that under an elevated-pore-pressure envelope "the
/// neutral axis of bending for 100 Myr old lithosphere lies below 40 km". A quantity an envelope moves is not a
/// constant. A consumer that pins `z_n = H/2` has adopted an assumption the primaries do not make.
///
/// # THE SOLVE
///
/// `N(z_n)` is MONOTONE in `z_n`, which is what makes bisection valid rather than merely convenient: at each
/// depth the elastic stress `[E/(1-nu^2)] K (z - z_n)` is strictly monotone in `z_n`, and clamping a monotone
/// function into a fixed band leaves it monotone, so the integral of the clamped stresses inherits it. The
/// direction is set by the SIGN of the curvature, and the solve reads it off the bracket rather than assuming
/// one. The bracket is the envelope's own domain `[0, domain]`, and where `N` does not change sign across it
/// there is no root to find and the construction refuses.
///
/// The lever arm dominates the integral ("the greatest contribution to `M` comes from stress differences at
/// large `(z - z_n)`", their p. 380), so an error in `z_n` is an error in `M`, and `T_e` goes as `M^(1/3)`. This
/// is load-bearing arithmetic, which is why it is solved rather than assumed.
pub fn neutral_surface_depth_km(
    profile: &EnvelopeProfile,
    curvature: FibreCurvature,
    youngs_modulus_gpa: Fixed,
    poisson_ratio: Fixed,
) -> Result<Fixed, MomentEquivalenceRefusal> {
    let plane_strain = plane_strain_modulus_gpa(youngs_modulus_gpa, poisson_ratio)
        .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;
    let mut lo = ZERO;
    let mut hi = profile.domain_max_depth_km();
    if hi <= ZERO {
        return Err(MomentEquivalenceRefusal::EnvelopeRefused);
    }
    let n_lo = axial_force(profile, curvature, lo, plane_strain)
        .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;
    let n_hi = axial_force(profile, curvature, hi, plane_strain)
        .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;
    if n_lo == ZERO {
        return Ok(lo);
    }
    if n_hi == ZERO {
        return Ok(hi);
    }
    // No sign change means no root in the envelope's own domain.
    if (n_lo > ZERO) == (n_hi > ZERO) {
        return Err(MomentEquivalenceRefusal::NeutralSurfaceNotBracketed);
    }
    let lo_positive = n_lo > ZERO;
    for _ in 0..BISECTION_STEPS {
        let mid = lo
            .checked_add(hi)
            .and_then(|s| s.checked_div(Fixed::from_int(2)))
            .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;
        if mid <= lo || mid >= hi {
            break;
        }
        let n_mid = axial_force(profile, curvature, mid, plane_strain)
            .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;
        if (n_mid > ZERO) == lo_positive {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(lo)
}

/// THE MOMENT-EQUIVALENT FLEXURAL RIGIDITY, and THE CANONICAL OUTPUT of this module: the rigidity of the uniform
/// elastic plate that carries the moment `M` at the curvature `K`.
///
/// `D_eq = M / K` (`GPa km^3`), which is McNutt and Menard's eq. 4 under this module's own stated sign
/// convention for `M` (their printed form is `M = -D K`; see the module header for why that is a convention on
/// the symbol and not a difference in physics). The rigidity is POSITIVE for a physical plate, since `M` and `K`
/// share sign, and it is CONVENTION-FREE in a way `T_e` is not: it carries no assumed modulus.
///
/// This is the object a hindcast comparison uses. A published `T_e` is converted BACK to a rigidity through the
/// ROW's own stated `(E, nu)`, and the two rigidities are compared. Comparing thicknesses instead would import
/// the row's assumed 80 GPa into a world that derives its own modulus.
///
/// `None` on zero curvature (an unbent plate reveals no rigidity) or an out-of-range quotient.
pub fn equivalent_rigidity(moment_gpa_km2: Fixed, curvature: FibreCurvature) -> Option<Fixed> {
    let k = curvature.upward_per_km();
    if k == ZERO {
        return None;
    }
    moment_gpa_km2.checked_div(k)
}

/// `T_e` (km), THE DISPLAY STATISTIC: the rigidity re-expressed as a thickness through a DECLARED modulus pair.
///
/// `T_e = (12 (1 - nu^2) D / E)^(1/3)` (Watts and Burov 2003 eq. 2, rearranged from the rigidity; `(1 - nu^2)` is
/// in `D`'s denominator and therefore in the cube root's numerator).
///
/// THE MODULI ARE EXPLICIT ARGUMENTS ON PURPOSE. Every published `T_e` is conditioned on an ASSUMED `E` and `nu`
/// and the literature never says so at the point of quotation, which the fetch called the sharpest of its
/// convention findings: `T_e` is proportional to `(1/E)^(1/3)`, so a thickness quoted without its pair is a
/// chord with its endpoints dropped. Requiring the pair here means the engine side declares the world's own
/// derived moduli and the data side declares the row's stated ones, and neither can borrow the other's silently.
///
/// Watts's own gloss on what the number means, and it is the reason this is a statistic rather than a depth:
/// "`T_e(YSE)` is not the actual thickness of the plate. Rather, it is a 'condensed' thickness that reflects the
/// 'integrated' strength of the flexed, competent, plate."
///
/// THE DIVISION BY `E` HAPPENS FIRST, which is range hygiene rather than algebra: `12 (1 - nu^2) D` reaches
/// 5.1e6 for an Earth-like plate and grows as the cube of the thickness, so a stiff world would overflow the
/// Q32.32 window in an intermediate that the reordering never forms.
///
/// THE CUBE ROOT is [`Fixed::powf`], the same route [`crate::laws::thermal_boundary_layer`] takes for its own
/// cube root, and it carries that function's series accuracy rather than the exactness `Fixed::sqrt` would give.
/// `None` on a non-positive rigidity or modulus, or `|nu| >= 1`.
pub fn elastic_thickness_km(
    rigidity_gpa_km3: Fixed,
    youngs_modulus_gpa: Fixed,
    poisson_ratio: Fixed,
) -> Option<Fixed> {
    if rigidity_gpa_km3 <= ZERO || youngs_modulus_gpa <= ZERO {
        return None;
    }
    let nu2 = poisson_ratio.checked_mul(poisson_ratio)?;
    let one_minus_nu2 = Fixed::ONE.checked_sub(nu2)?;
    if one_minus_nu2 <= ZERO {
        return None;
    }
    let cube = rigidity_gpa_km3
        .checked_div(youngs_modulus_gpa)?
        .checked_mul(Fixed::from_int(12))?
        .checked_mul(one_minus_nu2)?;
    if cube <= ZERO {
        return None;
    }
    Some(cube.powf(Fixed::from_ratio(1, 3)))
}

/// THE LINE-LOAD CURVATURE AT THE FIRST ZERO CROSSING, analytic.
///
/// # WHY THE FIRST ZERO CROSSING AND NOT THE PEAK
///
/// The owner's ruling, and the primary's read location. Three reasons, each decisive on its own. The deflection
/// vanishes there, so the axial-load term drops out of the moment identity even when axial loading is
/// appreciable ("because `w(x0) = 0`, even if axial loading `N` is appreciable, it will not be a factor",
/// McNutt and Menard p. 369). It is the one point on the profile where elastic and elastic-plastic models of the
/// SAME profile agree on the curvature, while on the outer trench wall they differ by a factor of two, so a
/// construction that derives `T_e` FROM a rheology must not read its input where the answer depends on the
/// rheology assumed. And the hindcast rows were built at this convention, so reading the peak instead would
/// compare against them through an undeclared location mismatch that biases `T_e` low, curvature being highest
/// at the peak and `T_e` falling as curvature rises.
///
/// # THE ALGEBRA
///
/// The kernel's line-load profile is `w = w0 e^(-X)(cos X + sin X)` with `X = x / alpha` and
/// `w0 = V0 alpha^3 / (8 D)` ([`crate::flexure::line_load_deflection`]). It vanishes where `cos X + sin X = 0`,
/// that is `X = 3 pi / 4`, the kernel's own documented zero crossing. Differentiating twice gives
/// `d2w/dx2 = -(2 w0 / alpha^2) e^(-X) (cos X - sin X)`, and at `X = 3 pi / 4` the bracket is `-sqrt(2)`, so
///
/// `d2w/dx2 (x0) = 2 sqrt(2) e^(-3 pi / 4) w0 / alpha^2`
///
/// IN THE KERNEL'S DOWNWARD-DEFLECTION CONVENTION. It is returned as a [`FibreCurvature`] built through
/// [`FibreCurvature::from_downward_deflection`], so the flip into the construction's upward convention happens in
/// the type and not in a reader's head. The result is NEGATIVE in the upward convention for a downward load,
/// which is concave-downward: the same sign the primary's own fitted trench curvatures carry.
///
/// The `2`, the `sqrt(2)`, the `3 pi / 4` and the `8` are the formula's own pure numbers, the same status as
/// `pi`. `None` on a non-positive `alpha` or `d`, or an out-of-range intermediate.
pub fn line_load_curvature_at_first_zero_crossing(
    v0: Fixed,
    alpha_km: Fixed,
    rigidity_gpa_km3: Fixed,
) -> Option<FibreCurvature> {
    if alpha_km <= ZERO || rigidity_gpa_km3 <= ZERO {
        return None;
    }
    // w0 = V0 alpha^3 / (8 D)
    let a3 = alpha_km
        .checked_mul(alpha_km)
        .and_then(|a2| a2.checked_mul(alpha_km))?;
    let eight_d = Fixed::from_int(8).checked_mul(rigidity_gpa_km3)?;
    let w0 = v0.checked_mul(a3).and_then(|x| x.checked_div(eight_d))?;
    // 2 sqrt(2) e^(-3 pi / 4) w0 / alpha^2
    let three_pi_over_four = Fixed::PI
        .checked_mul(Fixed::from_int(3))?
        .checked_div(Fixed::from_int(4))?;
    let decay = (ZERO - three_pi_over_four).exp();
    let two_root_two = Fixed::from_int(2).checked_mul(Fixed::from_int(2).sqrt())?;
    let a2 = alpha_km.checked_mul(alpha_km)?;
    let k_down = two_root_two
        .checked_mul(decay)?
        .checked_mul(w0)?
        .checked_div(a2)?;
    Some(FibreCurvature::from_downward_deflection(k_down))
}

/// A CONVERGED PER-LOAD MOMENT EQUIVALENCE, carrying its chord.
#[derive(Clone, Copy, Debug)]
pub struct MomentEquivalentPlate {
    /// THE CANONICAL OUTPUT: the moment-equivalent flexural rigidity (`GPa km^3`). See the module header for why
    /// this is primary and `T_e` is not.
    pub rigidity_gpa_km3: Fixed,
    /// The curvature the equivalence was read at (the first zero crossing of the deflection).
    pub curvature: FibreCurvature,
    /// The neutral surface the moment was taken about (km).
    pub neutral_depth_km: Fixed,
    /// The moment reading, including whether the integral self-truncated or was domain-limited.
    pub moment: MomentReading,
    /// THE CHORD. Every output carries the load class and the load timescale it was drawn over.
    pub chord: LoadChord,
    /// The number of fixed-point iterations taken.
    pub iterations: u32,
}

impl MomentEquivalentPlate {
    /// `T_e` (km) at a DECLARED modulus pair. A display statistic; see [`elastic_thickness_km`].
    pub fn elastic_thickness_km(
        &self,
        youngs_modulus_gpa: Fixed,
        poisson_ratio: Fixed,
    ) -> Option<Fixed> {
        elastic_thickness_km(self.rigidity_gpa_km3, youngs_modulus_gpa, poisson_ratio)
    }
}

/// THE PER-LOAD FIXED POINT: trial rigidity, deflection profile, curvature at the first zero crossing, moment
/// equivalence, iterate.
///
/// # THE LOOP IS THE CONSTRUCTION
///
/// `T_e` sets `D`, `D` sets the deflection, the deflection sets the curvature, and the curvature sets `T_e`. That
/// circle is a scalar fixed point per load, and it is the reason no reference bending is ever chosen: THE LOAD
/// SUPPLIES ITS OWN CURVATURE THROUGH THE SOLVE, so silent-curvature authorship dies structurally rather than by
/// discipline.
///
/// THE ITERATE IS THE RIGIDITY, NOT THE THICKNESS, following the canonical-output ruling. The two are equivalent
/// (the map between them is monotone at a fixed modulus pair), and iterating the rigidity keeps the demoted
/// display statistic out of the loop entirely.
///
/// # THE INITIAL TRIAL IS DERIVED
///
/// The walk starts at the FULLY ELASTIC rigidity of the envelope's own declared domain,
/// `D0 = E * domain^3 / (12 (1 - nu^2))`, which is the stiffest the column could possibly be: a plate that yields
/// nowhere. Every yielded plate is weaker, so the walk descends from a bound the envelope itself supplies rather
/// than from a number anyone chose.
///
/// # TWO WAYS TO FAIL, KEPT APART
///
/// The walk has two distinct exits, and conflating them would route a numerical residual to a physical branch.
/// The PHYSICAL exit is a trial rigidity that has fallen to zero or below (`d_new <= 0`): the envelope holds no
/// bending moment against the curvature the load imposes, so the load exceeds what it can elastically carry. That
/// is [`MomentEquivalenceRefusal::LoadExceedsElasticSupport`], and it routes to the SUPPORT-BOUND AND
/// VISCOUS-RELAXATION branch that already exists (`civsim_sim::deeptime::relax_to_support_bound`); nothing is
/// built here for that branch and nothing is routed here, since that module is downstream of this crate. The
/// NUMERICAL exit is iteration exhaustion with the rigidity still positive: the map has not settled to the last
/// bit inside the budget. That is [`MomentEquivalenceRefusal::FixedPointDidNotConverge`], which carries the final
/// step size `final_delta_gpa_km3` so a caller sees how close it came rather than reading a numerical stall as a
/// physical support bound.
///
/// THE LICENSE (ratified on review): a stalled end is a NUMERICAL EVENT TO SURFACE, never an open interval side.
/// Turning a solver stall into epistemic half-knowledge (a band the caller treats as physical spread) would
/// launder arithmetic into physics, so the numerical variant propagates as itself and never becomes a band edge or
/// a support-straddle.
///
/// CONVERGENCE IS TESTED AT THE LAST BIT: the walk stops when successive rigidities differ by at most
/// `Fixed::EPSILON`, the accumulator's own resolution, which is the same currency as every other tolerance here.
///
/// `delta_rho` is the density contrast in `1000 kg/m^3` and `gravity` is in `km/s^2` (the kernel's own coherent
/// system); `v0` is the line-load intensity in `GPa km`.
pub fn solve_line_load(
    profile: &EnvelopeProfile,
    youngs_modulus_gpa: Fixed,
    poisson_ratio: Fixed,
    delta_rho: Fixed,
    gravity: Fixed,
    v0: Fixed,
    chord: LoadChord,
) -> Result<MomentEquivalentPlate, MomentEquivalenceRefusal> {
    let domain = profile.domain_max_depth_km();
    if domain <= ZERO {
        return Err(MomentEquivalenceRefusal::EnvelopeRefused);
    }
    // The derived initial trial: the fully elastic rigidity of the envelope's own domain, the stiffest the
    // column could be.
    let mut d = crate::flexure::flexural_rigidity(youngs_modulus_gpa, poisson_ratio, domain)
        .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;

    let mut last_delta = Fixed::MAX; // sentinel until the first step computes one
    for iteration in 1..=MAX_FIXED_POINT_ITERATIONS {
        let alpha = crate::flexure::flexural_parameter(d, delta_rho, gravity)
            .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;
        let curvature = line_load_curvature_at_first_zero_crossing(v0, alpha, d)
            .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;
        if curvature.upward_per_km() == ZERO {
            return Err(MomentEquivalenceRefusal::ZeroCurvature);
        }
        let z_n = neutral_surface_depth_km(profile, curvature, youngs_modulus_gpa, poisson_ratio)?;
        let reading = bending_moment(profile, curvature, z_n, youngs_modulus_gpa, poisson_ratio)
            .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;
        let d_new = equivalent_rigidity(reading.moment, curvature)
            .ok_or(MomentEquivalenceRefusal::ZeroCurvature)?;
        if d_new <= ZERO {
            return Err(MomentEquivalenceRefusal::LoadExceedsElasticSupport);
        }
        let delta = d_new
            .checked_sub(d)
            .ok_or(MomentEquivalenceRefusal::NotRepresentable)?
            .abs();
        d = d_new;
        last_delta = delta;
        if delta <= Fixed::EPSILON {
            return Ok(MomentEquivalentPlate {
                rigidity_gpa_km3: d,
                curvature,
                neutral_depth_km: z_n,
                moment: reading,
                chord,
                iterations: iteration,
            });
        }
    }
    Err(MomentEquivalenceRefusal::FixedPointDidNotConverge {
        final_delta_gpa_km3: last_delta,
    })
}

/// The first zero crossing of the axisymmetric point-load deflection, `r/l = 3.91467`, which is the first zero
/// of `kei` (Abramowitz and Stegun 1965; McNutt and Menard 1982 p. 387). A CITED root of a special function, the
/// same status as a mathematical constant, and the axisymmetric sibling of the line load's own `x = 3 pi / 4`.
/// The deflection `w(r) = -(P l^2 / 2 pi D) kei(r/l)` vanishes here, so the axial-load term drops out and the
/// curvature is rheology-insensitive, exactly the property the line load reads at its own zero crossing.
fn point_load_first_zero_crossing() -> Fixed {
    Fixed::from_ratio(391467, 100000)
}

/// THE POINT-LOAD-TO-DISC EQUIVALENCE, the axisymmetric form's own accuracy band, as a fraction (0.02 = 2 per
/// cent). McNutt and Menard 1982 (pp. 389-390) show the finite cylindrical (disc) load's asymptotic factor `C_c`
/// is within 2 per cent of the point load's `C_p` at the nodal ring and beyond, so `M` and `K` from the
/// point-load approximation apply to a distributed disc within 2 per cent. A CITED number from the primary, the
/// cost of approximating the finite disc by the point load, never an authored tolerance. It ships as the band on
/// the axisymmetric rigidity ([`disc_point_rigidity_band`]).
fn disc_point_equivalence_band() -> Fixed {
    Fixed::from_ratio(2, 100)
}

/// McNutt and Menard's PUBLISHED Laplacian-curvature coefficient at the first zero crossing, `-0.0289`, carried
/// so both values ride together per the re-derivation standard (RUNBOOK section 11). Their own printed
/// definition of `K` applied to their own printed deflection (A8) gives `ker(x_0) = -0.0388994` instead
/// ([`point_load_reported_curvature_coefficient`]), confirmed by [`crate::flexure::kelvin_ker`] and
/// independently by scipy and mpmath plus a finite-difference twin, WITH the line-load controls reproducing to
/// 0.05 per cent. So their published seamount curvatures run about 26 per cent LOW and a correct coefficient
/// raises them 34 per cent; the cause is UNATTRIBUTED, no natural reading at `x_0` producing `0.0289`. This
/// module ships the RE-DERIVED value inherently (the curvature reads `ker(x_0)` from the series, never the
/// printed constant); the published value is carried only so a consumer of the paper's own `C_2` or Table 3
/// seamount curvatures can apply the correction.
pub fn mcnutt_menard_published_laplacian_coefficient() -> Fixed {
    Fixed::from_ratio(-289, 10000)
}

/// THE RE-DERIVED LAPLACIAN CURVATURE COEFFICIENT at the first zero crossing, `ker(x_0) = -0.0388994`, computed
/// from [`crate::flexure::kelvin_ker`] rather than copied from the page. It is the coefficient in the REPORTED
/// (Laplacian) curvature `K = -(P / 2 pi D) ker(x_0)`, the one the seamount literature quotes as "the curvature".
///
/// This module's canonical output uses the DRIVING curvature instead (the `M` operator, carrying `nu/r`; see
/// [`point_load_curvature_at_first_zero_crossing`]), because the moment equivalence needs the curvature that
/// drives the radial fibre stress. The Laplacian coefficient is exposed for the ERRATUM record and for a
/// consumer comparing an engine rigidity against a published Laplacian curvature: that consumer inherits the
/// paper's 26 per cent unless it recomputes from this value, which is what
/// [`mcnutt_menard_published_laplacian_coefficient`] documents.
pub fn point_load_reported_curvature_coefficient() -> Fixed {
    crate::flexure::kelvin_ker(point_load_first_zero_crossing())
}

/// THE AXISYMMETRIC POINT-LOAD DRIVING CURVATURE AT THE FIRST ZERO CROSSING, analytic, carrying its deflection
/// convention in the returned [`FibreCurvature`].
///
/// # WHICH CURVATURE, AND WHY IT IS NOT THE LINE LOAD'S
///
/// A circular load does NOT obey `M = -D K`. McNutt and Menard's Appendix A gives, for the axisymmetric plate,
/// `M = -D (d2w/dr2 + (nu/r) dw/dr)` while the reported curvature is the Laplacian `K = d2w/dr2 + (1/r) dw/dr`:
/// the `nu/r` and the `1/r` differ, so the moment is not `-D K` (fetch section 3, verified against the page at
/// 230 and 500 dpi). This slice's earlier refusal to apply the line-load algebra was CORRECT and is confirmed
/// from the primary; what the fetch settled is that the fibre YIELD LAW stays uniaxial (no biaxial surface, no
/// von Mises, no Tresca, no hoop stress in either primary's yield integral), so only the GEOMETRY changes, not
/// the scalar envelope this module already integrates.
///
/// So the driving curvature is `kappa_eff = kappa_r + nu kappa_theta = d2w/dr2 + (nu/r) dw/dr`, the operator in
/// `M`. This is the curvature that multiplies `(z - z_n)` in the radial fibre stress `[E/(1-nu^2)] kappa_eff
/// (z - z_n)`, so feeding it to [`bending_moment`] and [`equivalent_rigidity`] gives `D_eq = M_yield / kappa_eff`,
/// and in the elastic limit that recovers `D` exactly (the same identity the line load satisfies). Reading the
/// Laplacian `K` instead would give `D_eq = M/K`, which is wrong by the ratio `kappa_eff / K` (about 1.136 at
/// `nu = 0.25`) and would not recover `D` in the elastic limit.
///
/// # THE ALGEBRA
///
/// With `w(r) = -(P l^2 / 2 pi D) kei(r/l)`, `x = r/l`, the two curvatures at the zero crossing `x_0` are
/// `kappa_r = -(P / 2 pi D) kei''(x_0)` and `kappa_theta = -(P / 2 pi D) (1/x_0) kei'(x_0)`. Using the Kelvin
/// identity `kei'' = ker - kei'/x`,
///
/// `kappa_eff = -(P / 2 pi D) [ker(x_0) - (1 - nu) (1/x_0) kei'(x_0)]`,
///
/// whose bracket is McNutt and Menard's own eq. A10: it evaluates to `-0.04420` at `nu = 0.25`, giving
/// `M(x_0)/P = -0.007035`, their printed `-0.00704` (A9). The bracket carries `(1 - nu)`, so `nu` is baked into
/// the seamount constant just as the fetch found. `ker` and `kei'` are read from
/// [`crate::flexure::kelvin_ker`] and [`crate::flexure::kelvin_kei_prime`]; `x_0` is the cited zero crossing.
///
/// The result is in the deflection's DOWNWARD-positive convention (`w > 0` under the load, since `kei(0) < 0`),
/// so it is built through [`FibreCurvature::from_downward_deflection`] and the flip into the construction's
/// upward convention happens in the type, exactly as the line load does. A downward load reads a negative
/// (concave-down) curvature in the upward convention. `p` is the point-load weight (`GPa km^2` in the kernel's
/// coherent system) and `rigidity_gpa_km3` the trial rigidity; the curvature is INDEPENDENT of the density
/// contrast and gravity, because the flexural length `l` cancels between the deflection's `l^2` amplitude and the
/// `1/l^2` of the second derivative (unlike the line load, whose curvature scales with its own length scale).
///
/// `None` on a non-positive rigidity or an out-of-range intermediate.
pub fn point_load_curvature_at_first_zero_crossing(
    p: Fixed,
    rigidity_gpa_km3: Fixed,
    poisson_ratio: Fixed,
) -> Option<FibreCurvature> {
    if rigidity_gpa_km3 <= ZERO {
        return None;
    }
    let x0 = point_load_first_zero_crossing();
    let ker_x0 = crate::flexure::kelvin_ker(x0);
    let kei_prime_x0 = crate::flexure::kelvin_kei_prime(x0);
    // bracket = ker(x0) - (1 - nu) (1/x0) kei'(x0), the M operator's coefficient (McNutt and Menard A10).
    let one_minus_nu = Fixed::ONE.checked_sub(poisson_ratio)?;
    let hoop = one_minus_nu.checked_mul(kei_prime_x0)?.checked_div(x0)?;
    let bracket = ker_x0.checked_sub(hoop)?;
    // kappa_eff = -(P / (2 pi D)) * bracket, in the downward-positive deflection convention.
    let two_pi_d = Fixed::from_int(2)
        .checked_mul(Fixed::PI)?
        .checked_mul(rigidity_gpa_km3)?;
    let coeff = p.checked_div(two_pi_d)?;
    let kappa_eff_down = (ZERO - coeff).checked_mul(bracket)?;
    Some(FibreCurvature::from_downward_deflection(kappa_eff_down))
}

/// THE REPORTED (LAPLACIAN) CURVATURE AT THE FIRST ZERO CROSSING, `K = -(P / 2 pi D) ker(x_0)`, the axisymmetric
/// plate's rheology-insensitive observable and the quantity the seamount literature calls "the curvature".
///
/// This is NOT the curvature the moment equivalence drives its fibre stress with (that is
/// [`point_load_curvature_at_first_zero_crossing`], the `nu/r` operator). It is exposed so a hindcast comparison
/// against a PUBLISHED Laplacian curvature has a like quantity to compare, and it ships the RE-DERIVED
/// coefficient `ker(x_0) = -0.0388994` rather than the paper's printed `-0.0289`
/// ([`mcnutt_menard_published_laplacian_coefficient`]). Returned in the same downward convention and
/// [`FibreCurvature`] type as the driving curvature. `None` on a non-positive rigidity or an out-of-range
/// intermediate.
pub fn point_load_reported_curvature_at_first_zero_crossing(
    p: Fixed,
    rigidity_gpa_km3: Fixed,
) -> Option<FibreCurvature> {
    if rigidity_gpa_km3 <= ZERO {
        return None;
    }
    let ker_x0 = point_load_reported_curvature_coefficient();
    let two_pi_d = Fixed::from_int(2)
        .checked_mul(Fixed::PI)?
        .checked_mul(rigidity_gpa_km3)?;
    let coeff = p.checked_div(two_pi_d)?;
    // K = -(P / 2 pi D) ker(x0), downward-positive convention.
    let k_down = (ZERO - coeff).checked_mul(ker_x0)?;
    Some(FibreCurvature::from_downward_deflection(k_down))
}

/// THE AXISYMMETRIC (POINT-LOAD) PER-LOAD FIXED POINT, the sibling of [`solve_line_load`] with the geometry
/// changed and the scalar yield envelope unchanged.
///
/// # THE LOOP IS THE SAME, THE CURVATURE IS AXISYMMETRIC
///
/// Trial rigidity `D`, the driving curvature at the first zero crossing
/// ([`point_load_curvature_at_first_zero_crossing`], carrying `nu/r`), the neutral surface, the yield-limited
/// moment, `D_eq = M / kappa_eff`, iterate. The load supplies its own curvature through the solve, so no
/// reference bending is chosen, exactly as for the line load. The fixed point in curvature space seeks
/// `M_yield(kappa) = |bracket| P / (2 pi)`: a load whose yield-limited moment scale exceeds what the envelope can
/// carry crushes the trial rigidity to zero and reports [`MomentEquivalenceRefusal::LoadExceedsElasticSupport`],
/// the physical exit; a load that merely fails to settle to the last bit inside the iteration budget reports
/// [`MomentEquivalenceRefusal::FixedPointDidNotConverge`] carrying its final step size, the numerical one. The
/// two exits are kept apart for the same reason [`solve_line_load`] keeps them apart.
///
/// # NO DENSITY OR GRAVITY, AND WHY
///
/// The point-load curvature is `-(P / 2 pi D) bracket`, independent of the flexural length `l`, so this solve
/// takes neither the density contrast nor the gravity that [`solve_line_load`] needs for its `alpha`. The length
/// scale cancels between the deflection's `l^2` amplitude and the `1/l^2` of its curvature, a real difference
/// between the two geometries rather than an omission.
///
/// # THE INITIAL TRIAL IS DERIVED
///
/// The walk starts at the fully elastic rigidity of the envelope's own domain, the stiffest the column could be,
/// the same bound [`solve_line_load`] uses. `p` is the point-load weight in `GPa km^2` (the kernel's coherent
/// system). The disc-point 2 per cent band the axisymmetric form ships is [`disc_point_rigidity_band`], applied
/// to the converged rigidity by a consumer.
pub fn solve_point_load(
    profile: &EnvelopeProfile,
    youngs_modulus_gpa: Fixed,
    poisson_ratio: Fixed,
    p: Fixed,
    chord: LoadChord,
) -> Result<MomentEquivalentPlate, MomentEquivalenceRefusal> {
    let domain = profile.domain_max_depth_km();
    if domain <= ZERO {
        return Err(MomentEquivalenceRefusal::EnvelopeRefused);
    }
    let mut d = crate::flexure::flexural_rigidity(youngs_modulus_gpa, poisson_ratio, domain)
        .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;

    let mut last_delta = Fixed::MAX; // sentinel until the first step computes one
    for iteration in 1..=MAX_FIXED_POINT_ITERATIONS {
        let curvature = point_load_curvature_at_first_zero_crossing(p, d, poisson_ratio)
            .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;
        if curvature.upward_per_km() == ZERO {
            return Err(MomentEquivalenceRefusal::ZeroCurvature);
        }
        let z_n = neutral_surface_depth_km(profile, curvature, youngs_modulus_gpa, poisson_ratio)?;
        let reading = bending_moment(profile, curvature, z_n, youngs_modulus_gpa, poisson_ratio)
            .ok_or(MomentEquivalenceRefusal::NotRepresentable)?;
        let d_new = equivalent_rigidity(reading.moment, curvature)
            .ok_or(MomentEquivalenceRefusal::ZeroCurvature)?;
        if d_new <= ZERO {
            return Err(MomentEquivalenceRefusal::LoadExceedsElasticSupport);
        }
        let delta = d_new
            .checked_sub(d)
            .ok_or(MomentEquivalenceRefusal::NotRepresentable)?
            .abs();
        d = d_new;
        last_delta = delta;
        if delta <= Fixed::EPSILON {
            return Ok(MomentEquivalentPlate {
                rigidity_gpa_km3: d,
                curvature,
                neutral_depth_km: z_n,
                moment: reading,
                chord,
                iterations: iteration,
            });
        }
    }
    Err(MomentEquivalenceRefusal::FixedPointDidNotConverge {
        final_delta_gpa_km3: last_delta,
    })
}

/// THE DISC-POINT ACCURACY BAND on an axisymmetric rigidity: `[D (1 - b), D (1 + b)]` with `b` the primary's own
/// 2 per cent point-load-to-disc equivalence ([`disc_point_equivalence_band`]).
///
/// This is the band the axisymmetric FORM ships, the cost of approximating the finite disc by the point load,
/// cited from McNutt and Menard pp. 389-390 rather than chosen. It is a SEPARATE band from the `V*` rigidity band
/// ([`RigidityBand`] from [`solve_line_load_banded`]): that one is the source's activation-volume scatter, this
/// one is the geometry approximation. A consumer comparing an axisymmetric rigidity against a hindcast should
/// widen by this band before testing overlap. `C_1`'s own unexplained 2 per cent (fetch section 5.2) is a
/// DECLARED RESIDUAL that ships beside it rather than absorbed into it: it conditions the paper's published
/// moment constant, which this module does not consume, so it is documented rather than folded in here.
///
/// `None` on a non-positive rigidity.
pub fn disc_point_rigidity_band(rigidity_gpa_km3: Fixed) -> Option<RigidityBand> {
    if rigidity_gpa_km3 <= ZERO {
        return None;
    }
    let band = disc_point_equivalence_band();
    let lo = rigidity_gpa_km3.checked_mul(Fixed::ONE.checked_sub(band)?)?;
    let hi = rigidity_gpa_km3.checked_mul(Fixed::ONE.checked_add(band)?)?;
    RigidityBand::new(lo, hi)
}

/// A RIGIDITY BAND: an ordered interval of moment-equivalent flexural rigidity (`GPa km^3`), `low <= high`.
///
/// # THE HONEST CARRIER OF A PRIMARY THAT DECLINED TO CHOOSE
///
/// The dry-dislocation `V*` is not one number; H&K's Table 2 gives NINE determinations that disagree by a factor
/// of several precisely where the ductile branch binds, and the primary declines to collapse them. A refusal
/// there would claim the engine knows nothing; the determinations say the strength lies within a MEASURED spread.
/// So the moment equivalence over a deep-binding column is an interval, and this is its rigidity. A DEGENERATE
/// band (`low == high`) is the shallow column, where the brittle branch floors the envelope identically across the
/// span and the interval collapses to the point the settled view already reports.
///
/// # THE GAP LAW FINISHES THE COMPOSITION, AND IT IS NOT REBUILT HERE
///
/// A downstream verdict that FLIPS across `[low, high]` is NEAR-DEGENERATE (it carries and escalates); one
/// insensitive to the band PROCEEDS on either edge. That is the project's Gap Law, and its machinery is the
/// `civsim_materials::verdict` typestate (`dispose` reads a `delta` against a resolution and returns `Decided`
/// or `Escalate`). This crate is BELOW `materials` in the layering (`core -> physics -> materials -> sim`) and
/// cannot reach it, so this ships the band and the [`Self::overlaps`] and [`Self::is_degenerate`] queries a
/// sim-layer consumer feeds to that machinery. Building a second Gap Law here would be the twin-provider defect
/// the project keeps convicting; the band is the input, never a re-implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RigidityBand {
    low_gpa_km3: Fixed,
    high_gpa_km3: Fixed,
}

impl RigidityBand {
    /// Build a band from two rigidities in either order; the smaller becomes `low`. `None` on a non-positive
    /// rigidity, which is not a plate.
    pub fn new(a_gpa_km3: Fixed, b_gpa_km3: Fixed) -> Option<Self> {
        if a_gpa_km3 <= ZERO || b_gpa_km3 <= ZERO {
            return None;
        }
        Some(RigidityBand {
            low_gpa_km3: a_gpa_km3.min(b_gpa_km3),
            high_gpa_km3: a_gpa_km3.max(b_gpa_km3),
        })
    }

    /// A PUBLISHED HINDCAST ROW'S OWN RIGIDITY SCATTER, built from its published elastic-thickness interval through
    /// the ROW'S OWN modulus pair.
    ///
    /// This is the like-against-like the module header demands: a published `T_e` is conditioned on an ASSUMED
    /// `(E, nu)` the literature never states at the point of quotation, and `T_e ~ (1/E)^(1/3)`, so comparing a
    /// derived `T_e` against a published `T_e` compares the world's plate against a fictitious 80 GPa one. The row
    /// is converted BACK to a rigidity through its own pair here, so the engine's band (built at the world's own
    /// pair) and the row's band are both rigidities and the comparison imports no one's modulus into the other.
    ///
    /// `te_low_km` and `te_high_km` are the row's published thickness interval (its central value plus or minus
    /// its own scatter); `D = E H^3 / (12 (1 - nu^2))` is monotone in `H`, so the thickness interval maps to a
    /// rigidity interval order-preserving. `None` where the flexural rigidity refuses at either endpoint.
    pub fn from_hindcast_thickness_interval(
        te_low_km: Fixed,
        te_high_km: Fixed,
        youngs_modulus_gpa: Fixed,
        poisson_ratio: Fixed,
    ) -> Option<Self> {
        let d_lo = crate::flexure::flexural_rigidity(youngs_modulus_gpa, poisson_ratio, te_low_km)?;
        let d_hi =
            crate::flexure::flexural_rigidity(youngs_modulus_gpa, poisson_ratio, te_high_km)?;
        RigidityBand::new(d_lo, d_hi)
    }

    /// The lower edge (`GPa km^3`): the weakest plate the source's `V*` span permits.
    pub fn low(self) -> Fixed {
        self.low_gpa_km3
    }

    /// The upper edge (`GPa km^3`): the strongest plate the source's `V*` span permits.
    pub fn high(self) -> Fixed {
        self.high_gpa_km3
    }

    /// Whether the band has zero width, which is the shallow column where the span could not move the answer and
    /// the interval is really a point.
    pub fn is_degenerate(self) -> bool {
        self.low_gpa_km3 == self.high_gpa_km3
    }

    /// BAND-AWARE OVERLAP against another rigidity band: whether the two intervals intersect.
    ///
    /// This is the hindcast comparison the ruling calls for, and it is NEVER point equality. A derived rigidity
    /// band and a published row's own scatter band AGREE when they overlap, because a point-equality test would
    /// convict the source's own `V*` spread as a modelling error. The test is exact interval intersection, with no
    /// authored tolerance: the width on each side is the source's own (the engine's from the `V*` determinations,
    /// the row's from its published uncertainty), so the only thing chosen is the question, never a slack.
    pub fn overlaps(self, other: RigidityBand) -> bool {
        self.low_gpa_km3 <= other.high_gpa_km3 && other.low_gpa_km3 <= self.high_gpa_km3
    }
}

/// A BANDED PER-LOAD MOMENT EQUIVALENCE: the fixed point run at BOTH edges of the creep rows' `V*` span, carrying
/// the two converged plates and their shared chord.
///
/// # TWO CONVERGENT SOLVES, NEVER AN AVERAGE
///
/// `V*` is a span, so `D_eq` is a span, and the two edges are found by running the whole per-load fixed point
/// twice: once on the [`EnvelopeEdge::low`] envelope and once on [`EnvelopeEdge::high`]. Each is a full convergent
/// solve with its own curvature, neutral surface, and moment. If ONE edge converges and the other does not, that
/// is a FINDING ([`MomentEquivalenceRefusal::BandEdgeSupportDisagrees`]): the source's scatter straddles the
/// boundary between a load the plate holds and one it does not, and the honest report is that it straddles, never
/// the surviving edge's number passed off as the answer.
#[derive(Clone, Copy, Debug)]
pub struct BandedMomentEquivalentPlate {
    /// The plate at the `V*` LOW edge: the weakest ductile branch, hence the lower rigidity.
    low: MomentEquivalentPlate,
    /// The plate at the `V*` HIGH edge: the strongest ductile branch, hence the higher rigidity.
    high: MomentEquivalentPlate,
    /// THE LID REFEREE'S VERDICT, RIDING THE PLATE rather than computed and discarded. The pure solve leaves this
    /// `None`; [`BandedMomentEquivalentPlate::with_lid_referee`] attaches it from the envelope and a declared
    /// convective stress. `None` means the plate was not refereed, or the referee refused at the lid base. When a
    /// verdict is present, [`BandedMomentEquivalentPlate::te_bias`] maps it to the three-valued [`TeBias`] a
    /// truncated moment integral implies, so a `T_e` that the lid may or does bias low travels with the flag that
    /// says which.
    lid_referee: Option<LidReferee>,
}

/// THE `T_e` BIAS a lid referee's verdict implies, THREE-VALUED because the verdict is. A `T_e` read from a moment
/// integral bounded at the derived lid base is biased LOW when that lid is too shallow (the integral was cut short
/// of the deeper column it should have summed). The verdict maps to this bias one for one, so the ambiguity in the
/// verdict SURFACES here rather than collapsing to clean: a two-valued flag on a three-valued verdict would have to
/// fold the straddle into "unflagged", which launders ambiguity into a clean answer, the wrong direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeBias {
    /// The lid is confirmed at both `V*` ends: the moment integral reached the mechanical boundary, so `T_e` is not
    /// biased by a truncated lid.
    Unbiased,
    /// The `V*` bracket STRADDLES the convective stress: one end says the lid truncated the integral and the other
    /// does not, so the high edge of the `T_e` band carries the suspicion and the low edge does not. Reported as
    /// possibly biased low rather than collapsed to either answer.
    PossiblyBiasedLow,
    /// The strength exceeds the convective stress at BOTH ends: the lid base is derived too shallow, the integral
    /// was cut short, and `T_e` reads low. The one-sidedness (blind to a too-DEEP lid) is the referee's own limit.
    BiasedLow,
}

impl BandedMomentEquivalentPlate {
    /// THE CANONICAL BANDED OUTPUT: the rigidity band `[D(V*_low), D(V*_high)]` (`GPa km^3`).
    pub fn rigidity_band(&self) -> RigidityBand {
        RigidityBand {
            low_gpa_km3: self.low.rigidity_gpa_km3,
            high_gpa_km3: self.high.rigidity_gpa_km3,
        }
    }

    /// The `V*` LOW-edge plate (weaker, lower rigidity).
    pub fn low_edge(&self) -> &MomentEquivalentPlate {
        &self.low
    }

    /// The `V*` HIGH-edge plate (stronger, higher rigidity).
    pub fn high_edge(&self) -> &MomentEquivalentPlate {
        &self.high
    }

    /// Whether the band collapsed to a point: the shallow column, where the two edges converged to the same
    /// rigidity to the bit because the brittle branch floored the envelope identically across the span.
    pub fn is_degenerate(&self) -> bool {
        self.rigidity_band().is_degenerate()
    }

    /// The DISPLAY-THICKNESS band (km) at a DECLARED modulus pair, low edge below high. A statistic, carrying its
    /// pair for the reason [`elastic_thickness_km`] takes them explicitly: a `T_e` without its pair is a chord
    /// with its endpoints dropped. `None` where either edge's thickness refuses.
    pub fn elastic_thickness_band_km(
        &self,
        youngs_modulus_gpa: Fixed,
        poisson_ratio: Fixed,
    ) -> Option<(Fixed, Fixed)> {
        let lo = self
            .low
            .elastic_thickness_km(youngs_modulus_gpa, poisson_ratio)?;
        let hi = self
            .high
            .elastic_thickness_km(youngs_modulus_gpa, poisson_ratio)?;
        Some((lo, hi))
    }

    /// ATTACH THE LID REFEREE'S VERDICT to this plate, so a verdict that was being computed and discarded now
    /// rides the output. Runs [`referee_conductive_lid_base`] against the same `envelope` this band was solved on
    /// and the caller's declared `convective_stress_mpa` (in the ductile branch's own megapascals, explicit for
    /// the reason the referee itself takes it explicitly: the stress crosses a crate boundary that cannot infer
    /// its unit). Where the referee refuses at the lid base the field stays `None`, honest that no verdict was
    /// reached rather than inventing one. A builder that consumes and returns `self`, so the plate stays immutable
    /// once refereed.
    pub fn with_lid_referee(
        mut self,
        envelope: &LithosphereEnvelope<'_>,
        convective_stress_mpa: Fixed,
    ) -> Self {
        self.lid_referee = referee_conductive_lid_base(envelope, convective_stress_mpa);
        self
    }

    /// THE LID REFEREE'S VERDICT if one was attached, else `None`. `None` distinguishes a plate that was never
    /// refereed, or whose referee refused at the lid base, from any verdict reached.
    pub fn lid_referee(&self) -> Option<&LidReferee> {
        self.lid_referee.as_ref()
    }

    /// THE `T_e` BIAS the attached verdict implies, three-valued to match the referee's three-valued verdict, or
    /// `None` when no verdict is attached (the plate was not refereed, or the referee refused at the lid base). A
    /// `Confirmed` lid is [`TeBias::Unbiased`]; a bracket that STRADDLES the convective stress is
    /// [`TeBias::PossiblyBiasedLow`], so the `V*` ends' disagreement about whether the lid truncated the integral
    /// surfaces rather than laundering to clean; strength exceeding the stress at both ends is [`TeBias::BiasedLow`],
    /// the too-shallow-lid conviction. A two-valued flag would have to collapse the straddle into unflagged, the
    /// wrong direction, so the bias carries the verdict's own three values and ambiguity is reported, never hidden.
    pub fn te_bias(&self) -> Option<TeBias> {
        self.lid_referee.map(|referee| match referee.verdict {
            LidVerdict::Confirmed => TeBias::Unbiased,
            LidVerdict::BracketStraddlesConvectiveStress => TeBias::PossiblyBiasedLow,
            LidVerdict::StrengthExceedsConvectiveStress => TeBias::BiasedLow,
        })
    }
}

/// THE BANDED LINE-LOAD SOLVE: run the per-load fixed point at BOTH edges of the creep rows' `V*` span and carry
/// the result as a rigidity band.
///
/// # WHERE THE SETTLED VIEW REFUSES, THIS BANDS
///
/// A DEEP-binding column cannot be sampled through [`LithosphereEnvelope`]'s own [`YieldEnvelope`] impl, because
/// that refuses wherever the ductile ends disagree (the settled view declining to collapse the span). This samples
/// the [`EnvelopeEdge::low`] and [`EnvelopeEdge::high`] profiles instead, each a well-defined surface, and solves
/// each. Where the column is shallow the two edges coincide and the band is degenerate, which is the settled
/// view's answer read as a zero-width interval.
///
/// # THE LICENSE, CHECKED RATHER THAN TRUSTED
///
/// The band is ordered because the moment is MONOTONE in the strength profile and the high `V*` edge is pointwise
/// stronger, so `D(V*_high) >= D(V*_low)`. This is verified in the banked data and an adversarial family of
/// envelopes (`the_edge_envelopes_are_pointwise_ordered_low_below_high`,
/// `the_banded_solve_orders_its_two_edges`), and it is ALSO re-checked at runtime: a solve returning `D_low >
/// D_high` is [`MomentEquivalenceRefusal::BandRigidityUnordered`], a stop rather than a silent swap, because the
/// interval-arithmetic license failing is a finding.
///
/// `steps` is the caller's declared grid sampling, the same convention [`EnvelopeProfile`] carries. The moduli,
/// restoring term, load, and chord are [`solve_line_load`]'s own.
// This mirrors `solve_line_load`'s own seven-argument shape and adds only the sampling grid, so the argument
// list is the single-edge solve's plus one rather than a bundle worth a parameter struct.
#[allow(clippy::too_many_arguments)]
pub fn solve_line_load_banded(
    envelope: &LithosphereEnvelope<'_>,
    steps: u32,
    youngs_modulus_gpa: Fixed,
    poisson_ratio: Fixed,
    delta_rho: Fixed,
    gravity: Fixed,
    v0: Fixed,
    chord: LoadChord,
) -> Result<BandedMomentEquivalentPlate, MomentEquivalenceRefusal> {
    // Sample each edge of the envelope band onto its own profile. A refusal here is the envelope declining to
    // describe a column, which refuses the whole solve rather than one edge.
    let profile_low = EnvelopeProfile::sample(&EnvelopeEdge::low(envelope), steps)
        .ok_or(MomentEquivalenceRefusal::EnvelopeRefused)?;
    let profile_high = EnvelopeProfile::sample(&EnvelopeEdge::high(envelope), steps)
        .ok_or(MomentEquivalenceRefusal::EnvelopeRefused)?;

    let solve = |profile: &EnvelopeProfile| {
        solve_line_load(
            profile,
            youngs_modulus_gpa,
            poisson_ratio,
            delta_rho,
            gravity,
            v0,
            chord,
        )
    };
    let low = solve(&profile_low);
    let high = solve(&profile_high);
    combine_band_edges(low, high)
}

/// COMBINE THE TWO EDGE SOLVES into a rigidity band or a finding. Pure, so every arm is testable directly,
/// including the ones an end-to-end load sweep cannot reach (see `the_band_edge_combinator_reports_each_finding`).
///
/// # THE FINDING ARMS, AND WHY THEY ARE HERE EVEN WHERE A LOAD SWEEP CANNOT REACH THEM
///
/// The ruling is explicit: two convergent solves, and if one edge converges and the other does not that is a
/// FINDING, never an average. So the combinator reports:
///
/// - BOTH Ok and ordered: the band `[D_low, D_high]`.
/// - BOTH Ok but `D_low > D_high`: [`MomentEquivalenceRefusal::BandRigidityUnordered`], the monotonicity license
///   failing, which the ruling says is a stop rather than a silent swap.
/// - EXACTLY ONE Ok, the other [`MomentEquivalenceRefusal::LoadExceedsElasticSupport`]: the source's `V*` scatter
///   straddles the support boundary, reported as [`MomentEquivalenceRefusal::BandEdgeSupportDisagrees`] naming
///   which edge held, never the surviving edge's number.
/// - BOTH `LoadExceedsElasticSupport`: the whole span agrees the load is not held, which routes to the
///   support-bound branch and is honest rather than a straddle.
/// - Any other refusal on an edge: structural (a degenerate modulus, an unrepresentable intermediate), propagated.
///
/// ON THE BANKED EARTH-LIKE ENVELOPE the straddle arm is UNREACHED by a load sweep: the flexure arithmetic leaves
/// the Q32.32 window (`NotRepresentable`) at both edges within one load step of each other, before either edge's
/// physical support limit is crossed alone. That is a representability ceiling masking the physical boundary, a
/// stated blindness of the end-to-end path, not of this combinator, whose arms are put on trial here with
/// synthetic edge results.
fn combine_band_edges(
    low: Result<MomentEquivalentPlate, MomentEquivalenceRefusal>,
    high: Result<MomentEquivalentPlate, MomentEquivalenceRefusal>,
) -> Result<BandedMomentEquivalentPlate, MomentEquivalenceRefusal> {
    match (low, high) {
        (Ok(low), Ok(high)) => {
            // THE ORDERING, CHECKED. A larger `V*` is a stronger ductile branch is a larger moment is a larger
            // rigidity; if that fails here the interval-arithmetic license has failed and this stops.
            if low.rigidity_gpa_km3 > high.rigidity_gpa_km3 {
                return Err(MomentEquivalenceRefusal::BandRigidityUnordered {
                    low_gpa_km3: low.rigidity_gpa_km3,
                    high_gpa_km3: high.rigidity_gpa_km3,
                });
            }
            Ok(BandedMomentEquivalentPlate {
                low,
                high,
                lid_referee: None,
            })
        }
        // BOTH edges say the load is not elastically held: honest and not a band-straddle. The whole span agrees
        // the load exceeds support, so the support-bound branch is where it routes, at both edges.
        (
            Err(MomentEquivalenceRefusal::LoadExceedsElasticSupport),
            Err(MomentEquivalenceRefusal::LoadExceedsElasticSupport),
        ) => Err(MomentEquivalenceRefusal::LoadExceedsElasticSupport),
        // EXACTLY ONE edge holds the load: the source's `V*` scatter straddles the support boundary, which is a
        // finding rather than an average.
        (Ok(_), Err(MomentEquivalenceRefusal::LoadExceedsElasticSupport)) => {
            Err(MomentEquivalenceRefusal::BandEdgeSupportDisagrees {
                low_edge_converged: true,
            })
        }
        (Err(MomentEquivalenceRefusal::LoadExceedsElasticSupport), Ok(_)) => {
            Err(MomentEquivalenceRefusal::BandEdgeSupportDisagrees {
                low_edge_converged: false,
            })
        }
        // Any other refusal (a degenerate modulus, an unrepresentable intermediate, a zero curvature) is
        // structural rather than a band finding, so it propagates. The low edge's is reported where both carry
        // one, since it is sampled first.
        (Err(e), _) | (_, Err(e)) => Err(e),
    }
}

/// THE CONDUCTIVE-LID BASE `delta`, CARRYING ITS DERIVATION IN ITS TYPE: the depth below which the interior
/// convects, which is the moment integral's own domain.
///
/// # WHY THE INTEGRAL NEEDS A DOMAIN AT ALL, WHICH IS THE PREMISE THAT DIED
///
/// The slice below was specified under the claim that ductile strength decays exponentially with depth, so the
/// moment integrand's tail is bounded and the integration self-truncates. THAT CLAIM IS FALSE, and
/// [`bending_moment`] measures its falsity rather than assuming past it: a power-law creep row has a STRENGTH
/// FLOOR (`sigma` tends to `(epsilon-dot / A)^(1/n)`, about 2 pascals for the banked dry-olivine row at 1e-15
/// per second, never zero), the lever arm grows LINEARLY, so the integrand tends to a linearly GROWING function
/// and the integral DIVERGES. Under [`crate::geotherm::halfspace_geotherm`], whose temperature saturates at the
/// interior, it is worse: with `T` fixed the `P V*` term makes deep material STRONGER with depth, the integrand
/// turns and CLIMBS, and about 13 percent of the moment sits in the 200 to 300 km tail, rising. McNutt and
/// Menard's own integral converges only because THEIR GEOTHERM IS LINEAR in depth and therefore unphysically hot
/// below the lid, which drives their creep strength to zero. Ours is not, so ours needs a real domain.
///
/// # WHY THIS DEPTH, AND WHY THE EARLIER REFUSAL WAS RIGHT
///
/// This slice REFUSED to reach for [`crate::laws::thermal_boundary_layer`] when it first landed, on the ground
/// that "reaching for the nearest available depth is the same defect as reaching for the nearest available
/// strain rate". THAT REFUSAL WAS CORRECT AND IS NOT OVERTURNED HERE. Nearest and justified were different
/// properties, and the refusal was waiting for the justification rather than for a second opinion. The
/// justification has since arrived, and it is a fact about the physics rather than about the call graph:
/// `delta` IS THE CONDUCTIVE-CONVECTIVE BOUNDARY, and BELOW IT A STATIC LOAD'S STRESSES ARE NOT SUSTAINED,
/// because the material there overturns. A bending moment is the integral of a stress the column HOLDS, and
/// mantle that convects away holds nothing on a load's timescale. So the domain is not the nearest depth that
/// happened to be derivable; it is the depth at which the quantity being integrated stops existing.
///
/// # AND IT IS NOT ASSERTED, IT IS REFEREED
///
/// `delta` here is derived from the THERMAL structure alone (`d / Ra^(1/3)`: buoyancy, viscosity, diffusivity,
/// depth). Nothing in that derivation knows what creep is. So the choice is CHECKED against the stress scale
/// that defines the same boundary mechanically, by [`referee_conductive_lid_base`]: at `delta` the ductile
/// strength AT THE LOAD'S OWN RATE must have fallen to the convective driving stress
/// ([`crate::laws::convective_stress`]), which is the same competition lid mobilization already emerges from.
/// Two routes to one boundary, and their agreement is evidence rather than a restatement.
///
/// THE TYPE IS THE DEFENCE, and it is [`FibreCurvature`]'s pattern wearing the lid's coat: there is no
/// constructor from a bare depth, so a caller cannot DECLARE a lid base, only derive one. A declared lid is
/// exactly what the honest-limits note used to have to confess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConductiveLidBase {
    depth_km: Fixed,
}

impl ConductiveLidBase {
    /// DERIVE the lid base from the world's own convecting layer: `delta = d / Ra^(1/3)`, through
    /// [`crate::laws::thermal_boundary_layer`], which is the SAME derivation the convective driving stress
    /// shears over and the lid geotherm spans, so the three cannot disagree about how thick the lid is.
    ///
    /// `convecting_depth_km` is the convecting layer's own depth and `rayleigh` its Rayleigh number, both the
    /// world's. A NON-CONVECTING layer (a non-positive Rayleigh number) reads back the whole layer, which is the
    /// law's own documented convention and is the physics: with no convection there is no conductive-convective
    /// boundary, the whole layer conducts, and the lid is all of it. That case is honest rather than degenerate,
    /// and the moment integral over it is bounded by the layer's own base.
    ///
    /// `None` on a non-positive layer depth, or where the derived lid rounds away to nothing: a lid with no
    /// thickness has no column to integrate, and reporting one would be reporting a plate that is not there.
    pub fn from_rayleigh(convecting_depth_km: Fixed, rayleigh: Fixed) -> Option<Self> {
        if convecting_depth_km <= ZERO {
            return None;
        }
        let depth_km = crate::laws::thermal_boundary_layer(convecting_depth_km, rayleigh);
        if depth_km <= ZERO {
            return None;
        }
        Some(ConductiveLidBase { depth_km })
    }

    /// The lid base `delta` (km).
    pub fn depth_km(self) -> Fixed {
        self.depth_km
    }
}

/// THE LID'S YIELD ENVELOPE: the brittle branch above, the ductile branch below, assembled from the landed rows.
///
/// # WHAT IT CONSUMES RATHER THAN REBUILDS
///
/// The lithostatic axis is [`crate::yield_envelope::lithostatic_normal_stress_mpa`], the friction row is a
/// [`FrictionLaw`] (rock's or ice's, the caller's own), and the ductile branch is
/// [`crate::creep_rows::ductile_strength_mpa`] over the caller's admitted creep candidates. The geotherm arrives
/// as an EVALUATOR so that either landed form serves without this module dispatching on a named regime:
/// [`crate::geotherm::halfspace_geotherm`] where a lid has an age,
/// [`crate::geotherm::steady_conductive_geotherm`] where it does not.
///
/// # THE ENVELOPE IS THE LESSER OF THE TWO BRANCHES
///
/// At each depth the material fails by whichever mechanism is weaker, so the envelope is `min(brittle, ductile)`
/// in each sense. The brittle branch is ASYMMETRIC (about three times stronger in compression) and the ductile
/// branch is SYMMETRIC, which is the primaries' own structure: their eqs. 7 and 8 separate tension from
/// compression while their creep branches are stated in differential-stress magnitude.
///
/// # THE ALIEN IS A DATA ROW
///
/// Every input is per-world or per-material: the friction row keys on the material (an ice shell passes
/// [`crate::yield_envelope::ice_friction_law`], whose branches change near 5 to 10 MPa rather than 200 and which
/// brackets in its own gap), the density and gravity are the body's, the geotherm is the caller's closure, and
/// the creep rows are the material's. Nothing here is silicate-shaped.
///
/// # THE STRAIN RATE IS THE LOAD'S
///
/// It arrives inside [`LoadChord`] and is passed through to the creep rows unmodified. This module never reads
/// [`crate::laws::convective_strain_rate`], which is the mantle-and-thermal chord and forbidden to this consumer
/// by its own doc. [`referee_conductive_lid_base`] reads that law's SIBLING, the convective STRESS, which is a
/// different act: the stress locates the boundary, the rate would have evaluated the strength, and only the
/// second is the defect this arc evicted.
///
/// # THE DOMAIN IS DERIVED, AND THE `V*` SPAN STOPS AT THE ENVELOPE
///
/// The lid base is [`ConductiveLidBase`], derived from the world's Rayleigh number and refereed against the
/// convective stress scale, so no caller declares where this envelope stops. And because `V*` is a span the
/// primary declines to collapse, `yield_in_sense` evaluates the ductile branch at BOTH ends of it and reports a
/// strength only where they agree to the bit.
pub struct LithosphereEnvelope<'a> {
    /// The material's own friction row.
    pub friction: FrictionLaw,
    /// The lid's density (kg/m^3), for the lithostatic axis, which is stated in raw SI.
    pub density_kg_m3: Fixed,
    /// The body's surface gravity (m/s^2), for the lithostatic axis, which is stated in raw SI.
    pub gravity_m_s2: Fixed,
    /// The geotherm as an evaluator: depth in KILOMETRES to temperature in KELVIN. `None` where the geotherm
    /// itself refuses, which refuses the envelope rather than fabricating a temperature.
    pub geotherm_k: &'a dyn Fn(Fixed) -> Option<Fixed>,
    /// The creep candidates the ductile branch solves over.
    pub creep: &'a [CreepCandidate<'a>],
    /// The load's chord, which carries the strain rate the envelope evaluates at.
    pub chord: LoadChord,
    /// THE CONDUCTIVE-LID BASE, DERIVED, which is the depth below which this envelope stops describing anything:
    /// the interior below convects and carries no long-term fibre stress.
    ///
    /// IT USED TO BE A DECLARED `Fixed` AND THAT WAS THE ARC'S ONE ASTERISK. A bare depth here meant the moment
    /// depended on a number the caller named, so "nothing in the arc authors a scalar" held only as far as the
    /// caller's own discipline. [`ConductiveLidBase`] has no constructor from a bare depth, so the dependence is
    /// now on the world's own Rayleigh number and layer depth, and the asterisk is gone rather than documented.
    pub lid_base: ConductiveLidBase,
}

/// What the ductile branch reported at a depth, including the two representability edges the creep module names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuctileReading {
    /// A determined creep strength (megapascals).
    Determined(Fixed),
    /// The creep strength ran past `Fixed::MAX`: a cold lid at a geological rate, which is the flow law saying
    /// creep is IRRELEVANT there. The envelope's brittle branch floors it, exactly as the creep module's own doc
    /// states, so the ductile branch simply does not bind.
    AboveRepresentable,
    /// The creep strength ran below `Fixed::EPSILON`: a hot lid, where the material sustains nothing the type can
    /// hold. The envelope reads zero, which is the honest answer rather than a rounded-up floor.
    BelowRepresentable,
    /// The creep rows refused (no admitted row, no chord covering the pressure, a grade the exponent gate
    /// rejects). Refused rather than substituted.
    Refused(CreepRefusal),
}

impl LithosphereEnvelope<'_> {
    /// The lithostatic vertical stress at a depth (MPa), through the landed axis. The kilometre-to-metre
    /// conversion happens HERE, once, at the boundary with a row stated in metres.
    fn vertical_stress_mpa(&self, depth_km: Fixed) -> Option<Fixed> {
        let depth_m = depth_km.checked_mul(Fixed::from_int(M_PER_KM))?;
        lithostatic_normal_stress_mpa(self.density_kg_m3, self.gravity_m_s2, depth_m)
    }

    /// The ductile branch at a depth, at ONE END of the creep rows' `V*` bracket: the composite at the LOAD's
    /// strain rate, the geotherm's temperature, and the lithostatic pressure. The megapascal-to-gigapascal
    /// conversion for the pressure happens HERE, once, at the boundary with rows whose pressure currency is
    /// gigapascals.
    ///
    /// THE END IS NAMED BY THE CALLER because `V*` is a span the primary declines to collapse
    /// ([`crate::creep_rows::ActivationVolumeBracket`]). [`Self::yield_in_sense`] reads BOTH ends and reports no
    /// single number where they disagree, which is what lets the shallow column be sampled from the surface
    /// (where no banked chord reaches) without anyone inventing a determination for it.
    pub fn ductile(&self, depth_km: Fixed, end: VolumeEnd) -> DuctileReading {
        let t_k = match (self.geotherm_k)(depth_km) {
            Some(t) => t,
            None => return DuctileReading::Refused(CreepRefusal::ConditionOutOfDomain),
        };
        let p_mpa = match self.vertical_stress_mpa(depth_km) {
            Some(p) => p,
            None => return DuctileReading::Refused(CreepRefusal::NotRepresentable),
        };
        let p_gpa = match p_mpa.checked_div(Fixed::from_int(MPA_PER_GPA)) {
            Some(p) => p,
            None => return DuctileReading::Refused(CreepRefusal::NotRepresentable),
        };
        let conditions = CreepConditions {
            ln_strain_rate_per_s: self.chord.ln_strain_rate_per_s,
            temperature_k: t_k,
            pressure_gpa: p_gpa,
            grain_size_um: None,
            water: None,
        };
        match ductile_strength_mpa(self.creep, conditions, end) {
            Ok(s) => DuctileReading::Determined(s),
            Err(CreepRefusal::StressNotRepresentable { ln_stress_mpa }) => {
                if ln_stress_mpa > Fixed::MAX.ln() {
                    DuctileReading::AboveRepresentable
                } else {
                    DuctileReading::BelowRepresentable
                }
            }
            Err(other) => DuctileReading::Refused(other),
        }
    }

    /// The BRITTLE branch at a depth, in one sense (megapascals). Exposed so a caller can see which branch bound
    /// the envelope rather than inferring it from the minimum.
    pub fn brittle(&self, depth_km: Fixed, sense: FaultingSense) -> Option<DifferentialStrength> {
        let sigma_v = self.vertical_stress_mpa(depth_km)?;
        brittle_differential_mpa(&self.friction, sigma_v, sense)
    }

    /// The envelope in one sense (GPa): the lesser of the brittle and ductile branches, PROVEN INVARIANT across
    /// the creep rows' `V*` bracket rather than read at an end someone picked.
    ///
    /// # THE BRACKET REACHES HERE, AND HERE IS WHERE IT IS SETTLED OR REPORTED
    ///
    /// `V*` is a span ([`crate::creep_rows::ActivationVolumeBracket`]), so the ductile branch is a span, so
    /// `min(brittle, ductile)` is evaluated at BOTH ENDS and the two are compared. Where they agree the envelope
    /// reports the number, and the agreement is a PROOF that the span could not have moved it. Where they
    /// disagree the span has reached the answer and there is no single number to report, so this refuses, which
    /// is the same treatment the brittle branch's own gap already gets a few lines above.
    ///
    /// THE SHALLOW COLUMN IS WHY THIS IS NOT A REFUSAL EVERYWHERE. The banked chords start at 0.3 GPa, so from
    /// the surface down to about nine kilometres on an Earth-like world the bracket is the table's whole span
    /// ([`crate::creep_rows::VolumeConstraint::UnconstrainedBySource`]). It costs nothing there: `P V*` tops out
    /// near 8 kJ/mol against `E*`'s 530, the cold shallow rock is not creeping at a geological rate at either
    /// end, and the brittle branch floors the envelope identically both ways. That is what lets a real envelope
    /// be sampled FROM THE SURFACE, which is the full-column solve this unblocks, and it is asserted rather than
    /// asserted-about (`the_shallow_envelope_is_invariant_across_the_v_star_bracket`).
    ///
    /// # WHERE THE ENDS DISAGREE THIS REFUSES, AND THE BANDED VIEW IS ITS SIBLING
    ///
    /// This is the SETTLED view: it reports a strength only where the span could not have moved it. The DEEP
    /// column, where the ductile branch binds and the two ends part company, is served instead by
    /// [`LithosphereEnvelope::edge_yield`] through an [`EnvelopeEdge`], which reads ONE edge of the
    /// interval-of-mins rather than demanding the two
    /// agree. Refusing here and banding there are the two halves of the same ruling: honesty changes what the
    /// answer LOOKS LIKE (a point becomes an interval), never whether there is one.
    fn yield_in_sense(&self, depth_km: Fixed, sense: FaultingSense) -> Option<Fixed> {
        if depth_km < ZERO {
            return None;
        }
        let brittle_mpa = match self.brittle(depth_km, sense)? {
            DifferentialStrength::Determined(d) => d,
            // No branch of the friction law is licensed here, so the brittle strength is a band and the envelope
            // has no single number to report. Refuse rather than pick a point inside a gap the calibration says
            // nothing about.
            DifferentialStrength::Bracket { .. } => return None,
        };
        let at_end = |end: VolumeEnd| -> Option<Fixed> {
            match self.ductile(depth_km, end) {
                DuctileReading::Determined(d) => Some(brittle_mpa.min(d)),
                // Creep is irrelevant here; the brittle branch floors the envelope.
                DuctileReading::AboveRepresentable => Some(brittle_mpa),
                // The material sustains nothing representable.
                DuctileReading::BelowRepresentable => Some(ZERO),
                DuctileReading::Refused(_) => None,
            }
        };
        let low = at_end(VolumeEnd::Low)?;
        let high = at_end(VolumeEnd::High)?;
        // THE INVARIANCE, CHECKED RATHER THAN TRUSTED. Where the two ends of the span disagree, the source's own
        // scatter in `V*` has reached the envelope and no single strength is licensed.
        if low != high {
            return None;
        }
        low.checked_div(Fixed::from_int(MPA_PER_GPA))
    }

    /// THE ENVELOPE IN ONE SENSE AT ONE EDGE of the `V*` bracket (GPa): `min(brittle, ductile)` with the ductile
    /// limb read at the named end and NO comparison against the other.
    ///
    /// # THIS IS THE INTERVAL'S EDGE, AND IT IS THE SURFACE THE BANDED CONSTRUCTION INTEGRATES
    ///
    /// `V*` is a span the primary declines to collapse ([`crate::creep_rows::ActivationVolumeBracket`]), so the
    /// envelope's `min(brittle, ductile)` is a span too: `[min(brittle, ductile_low), min(brittle, ductile_high)]`.
    /// [`Self::yield_in_sense`] asks whether that interval is degenerate (the two ends agree) and reports the
    /// number where it is; this asks what the answer IS at one named end of it, so a caller can integrate the LOW
    /// edge and the HIGH edge and carry the moment as an interval.
    ///
    /// # WHY THE TWO ENDS ARE AN ORDERED INTERVAL RATHER THAN TWO UNRELATED NUMBERS
    ///
    /// At a non-negative pressure a larger `V*` raises `E* + P V*`, which lowers every creep row's rate at a given
    /// stress and so RAISES the stress the target rate needs: the ductile strength is monotone increasing in `V*`.
    /// So `min(brittle, ductile_high) >= min(brittle, ductile_low)` at every depth, which is the pointwise ordering
    /// the moment integral's own monotonicity then propagates into an ordered rigidity band
    /// (`the_edge_envelopes_are_pointwise_ordered_low_below_high`).
    ///
    /// # THE BRITTLE GAP IS NOT THE DUCTILE SPAN
    ///
    /// A [`DifferentialStrength::Bracket`] from the friction row is a DIFFERENT band: the friction law's own
    /// missing domain (ice between its two fits), which a `V*` interval cannot carry and which no end of the
    /// ductile span settles. So this refuses there exactly as [`Self::yield_in_sense`] does, rather than pretending
    /// an edge of the creep span speaks for the friction gap.
    pub fn edge_yield(
        &self,
        depth_km: Fixed,
        sense: FaultingSense,
        end: VolumeEnd,
    ) -> Option<Fixed> {
        if depth_km < ZERO {
            return None;
        }
        // BAND-NEVER-REFUSE, SYMMETRICALLY. The ductile limb is read at the named `end` below; the brittle limb
        // is read at the SAME end here, so the interval-of-mins composes over both limbs rather than going silent
        // the moment the brittle limb is itself a bracket. It brackets for ice: Beeman's laws leave a gap from
        // 5 to 10 MPa, which at Europa's gravity is roughly four to eight kilometres down, the heart of an ice
        // shell's brittle zone. Refusing there composed the envelope for every rock lid and silenced every icy one
        // at mid-shell, Terran bias expressed as an unhandled match arm. `VolumeEnd` (Low the weaker end, High the
        // stronger) carries the same sense for both limbs, so the mirrored end parameter is the whole fix.
        //
        // THE INDEPENDENCE LICENSE (ratified on review): endpoint-wise min, low with low and high with high, is the
        // EXACT interval arithmetic for the min of INDEPENDENT intervals, so this composes rather than averages. The
        // two ignorance sources here are independent: the Beeman friction gap is a friction-calibration bracket, the
        // V* spread is the creep rows' own scatter, and neither conditions the other. The boundary is named so it is
        // not crossed unseen: the day two CORRELATED brackets meet in a min, the covariance rule applies instead of
        // endpoint-wise min, and this arm would be wrong for that case.
        let brittle_mpa = match self.brittle(depth_km, sense)? {
            DifferentialStrength::Determined(d) => d,
            DifferentialStrength::Bracket { low, high } => match end {
                VolumeEnd::Low => low,
                VolumeEnd::High => high,
            },
        };
        let mpa = match self.ductile(depth_km, end) {
            DuctileReading::Determined(d) => brittle_mpa.min(d),
            // Creep is irrelevant here; the brittle branch floors the envelope.
            DuctileReading::AboveRepresentable => brittle_mpa,
            // The material sustains nothing representable.
            DuctileReading::BelowRepresentable => ZERO,
            DuctileReading::Refused(_) => return None,
        };
        mpa.checked_div(Fixed::from_int(MPA_PER_GPA))
    }
}

/// ONE EDGE of a [`LithosphereEnvelope`]'s `V*` band, as a [`YieldEnvelope`] in its own right.
///
/// # WHY A WRAPPER RATHER THAN A FLAG ON THE ENVELOPE
///
/// The moment integral consumes a [`YieldEnvelope`] and samples it onto a profile. A DEEP column's envelope
/// refuses through [`LithosphereEnvelope`]'s own trait impl wherever the ductile ends disagree, so it cannot be
/// sampled at all: that is the settled view declining to collapse the band. This wrapper is the BANDED view. It
/// reads one named end of the interval-of-mins through [`LithosphereEnvelope::edge_yield`], so the profile at the
/// [`VolumeEnd::Low`] edge and the profile at the [`VolumeEnd::High`] edge are the two edges of the envelope band,
/// each a well-defined surface with no disagreement to refuse over.
///
/// The construction then integrates each edge (interval arithmetic on a monotone integrand) and carries the
/// result as a rigidity band. The wrapper owns no new physics; it selects which end of an already-banded quantity
/// a given integration reads.
pub struct EnvelopeEdge<'a> {
    envelope: &'a LithosphereEnvelope<'a>,
    end: VolumeEnd,
}

impl<'a> EnvelopeEdge<'a> {
    /// View the LOW edge of the envelope's `V*` band: the smallest `V*`, hence the WEAKEST the ductile branch can
    /// be at a positive pressure, hence the lower edge of the strength interval.
    pub fn low(envelope: &'a LithosphereEnvelope<'a>) -> Self {
        EnvelopeEdge {
            envelope,
            end: VolumeEnd::Low,
        }
    }

    /// View the HIGH edge: the largest `V*`, the STRONGEST the ductile branch can be at a positive pressure.
    pub fn high(envelope: &'a LithosphereEnvelope<'a>) -> Self {
        EnvelopeEdge {
            envelope,
            end: VolumeEnd::High,
        }
    }
}

impl YieldEnvelope for EnvelopeEdge<'_> {
    fn tensile_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed> {
        self.envelope
            .edge_yield(depth_km, FaultingSense::Normal, self.end)
    }

    fn compressive_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed> {
        self.envelope
            .edge_yield(depth_km, FaultingSense::Thrust, self.end)
    }

    fn domain_max_depth_km(&self) -> Fixed {
        self.envelope.lid_base.depth_km()
    }
}

impl YieldEnvelope for LithosphereEnvelope<'_> {
    fn tensile_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed> {
        self.yield_in_sense(depth_km, FaultingSense::Normal)
    }

    fn compressive_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed> {
        self.yield_in_sense(depth_km, FaultingSense::Thrust)
    }

    fn domain_max_depth_km(&self) -> Fixed {
        self.lid_base.depth_km()
    }
}

/// WHAT THE LID REFEREE FOUND: the ductile strength at the lid base, at the load's own rate, set beside the
/// convective driving stress that defines the boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LidReferee {
    /// The verdict.
    pub verdict: LidVerdict,
    /// The lid base the verdict is about (km).
    pub lid_base_km: Fixed,
    /// The ductile branch's reading at the lid base at the LOW end of the `V*` bracket, MEASURED rather than
    /// summarized, so a caller sees the margin rather than the verdict alone.
    pub strength_low: DuctileReading,
    /// The same at the HIGH end. Identical to `strength_low` where the bracket is degenerate.
    pub strength_high: DuctileReading,
    /// The convective driving stress the strengths were refereed against (MPa), as the caller declared it.
    pub convective_stress_mpa: Fixed,
}

/// THE LID REFEREE'S VERDICT.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LidVerdict {
    /// The ductile strength at the lid base has fallen TO OR BELOW the convective driving stress at BOTH ends of
    /// the `V*` bracket. The thermally derived lid and the mechanical boundary agree, and the lid choice is
    /// refereed rather than asserted.
    Confirmed,
    /// The strength at the lid base is ABOVE the convective stress at both ends: the material there still
    /// sustains more than the interior flow can shear, so this envelope's strength has not reached the
    /// boundary's own stress scale by the depth the Rayleigh number puts it at. A FINDING, reported rather than
    /// absorbed, and the two derivations disagreeing is a fact about the world's data rather than a fault here.
    StrengthExceedsConvectiveStress,
    /// The `V*` bracket's two ends STRADDLE the convective stress, so the source's own span does not settle the
    /// question. Reported rather than collapsed to whichever end was read first.
    BracketStraddlesConvectiveStress,
}

/// REFEREE THE DERIVED LID BASE against the stress scale that defines the boundary: at `delta`, the ductile
/// strength AT THE LOAD'S OWN RATE must have fallen to the convective driving stress.
///
/// # WHY THIS IS EVIDENCE AND NOT A RESTATEMENT
///
/// [`ConductiveLidBase`] derives `delta` from the THERMAL structure alone (`d / Ra^(1/3)`: buoyancy, viscosity,
/// diffusivity, layer depth). Nothing in that expression knows what creep is, what `E*` is, or what rate the
/// load imposes. This check asks the MECHANICAL question at that depth, through the creep rows, and it is the
/// same competition that lid mobilization already emerges from: [`crate::laws::convective_stress`]'s own doc
/// states that "the lid mobilizes LOCALLY where the convective stress exceeds the yield strength". So the lid
/// base is where the lid's strength stops exceeding it, and two independent routes to one boundary either agree
/// or produce a finding.
///
/// # THE RATE IS THE LOAD'S, AND THE SIBLING LAW IS STILL FORBIDDEN
///
/// [`crate::laws::convective_strain_rate`] is [`crate::laws::convective_stress`]'s sibling and is THE
/// MANTLE-AND-THERMAL CHORD; its own doc forbids this consumer by name. Nothing here reads it. The strength this
/// referees is the envelope's own, evaluated at the LOAD's rate out of [`LoadChord`], which is the chord `T_e`
/// is drawn over. Refereeing the lid against the convective STRESS is not the same act as evaluating the lid's
/// strength at the convective RATE, and the difference is the whole distinction this arc evicted a defect over:
/// the stress is a property of the boundary being located, the rate is a property of the load doing the bending.
///
/// # THE STRESS ARRIVES IN A DECLARED UNIT BECAUSE IT CANNOT BE INFERRED
///
/// `laws::convective_stress` is unit-agnostic over a consistent set, and the engine's own caller composes it out
/// of REPRESENTABLE-SCALED inputs (`civsim_sim::geodynamics::ColumnParams` declares that its viscosity and depth
/// are scaled and that its own scale system is an open conflict). So a stress read from there carries no unit
/// this module could infer. It is therefore an EXPLICIT ARGUMENT IN MEGAPASCALS, the ductile branch's own
/// currency, exactly as [`elastic_thickness_km`] takes its modulus pair explicitly and for the same reason: a
/// convention that cannot be stated silently cannot be got wrong silently.
///
/// # THE CHECK IS ONE-SIDED, AND THAT IS STATED RATHER THAN HIDDEN
///
/// "Has fallen to" is `<=`, which convicts a lid base that is TOO SHALLOW (the material there is still strong)
/// and is BLIND to one that is TOO DEEP (deeper material is weaker still, so it passes more easily). Making it
/// two-sided would need a band around the crossing, a band is a tolerance, and a tolerance is a number someone
/// chose. So the one-sided form is what the physics licenses with nothing authored, and the blindness is named
/// here rather than left for a reader to find.
///
/// `None` where the ductile branch refuses at the lid base, which refuses the referee rather than guessing a
/// verdict.
/// WHETHER A DUCTILE READING HAS FALLEN TO THE CONVECTIVE-STRESS SCALE: at or below it.
///
/// A NAMED FUNCTION RATHER THAN A CLOSURE, because its two representability arms each carry a physical claim
/// that a fixture reaching only the ordinary arm would never put on trial. Both survived a mutation run as an
/// inline closure, which is what named them.
///
/// - A `Determined` strength answers the question directly.
/// - `AboveRepresentable` is a strength past `Fixed::MAX` MEGAPASCALS, which is the flow law saying creep is
///   irrelevant at this depth. It is astronomically above any convective driving stress, so it has NOT fallen,
///   and this is the reading a lid base derived far too shallow returns: the referee's whole purpose.
/// - `BelowRepresentable` is a strength under `Fixed::EPSILON` megapascals, which is below any positive stress,
///   so it HAS fallen.
/// - A refusal answers nothing, and refuses the referee rather than guessing.
fn strength_has_fallen_to(reading: DuctileReading, convective_stress_mpa: Fixed) -> Option<bool> {
    match reading {
        DuctileReading::Determined(s) => Some(s <= convective_stress_mpa),
        DuctileReading::AboveRepresentable => Some(false),
        DuctileReading::BelowRepresentable => Some(true),
        DuctileReading::Refused(_) => None,
    }
}

pub fn referee_conductive_lid_base(
    envelope: &LithosphereEnvelope<'_>,
    convective_stress_mpa: Fixed,
) -> Option<LidReferee> {
    let lid_base_km = envelope.lid_base.depth_km();
    let strength_low = envelope.ductile(lid_base_km, VolumeEnd::Low);
    let strength_high = envelope.ductile(lid_base_km, VolumeEnd::High);
    let fallen = |reading: DuctileReading| strength_has_fallen_to(reading, convective_stress_mpa);
    let verdict = match (fallen(strength_low)?, fallen(strength_high)?) {
        (true, true) => LidVerdict::Confirmed,
        (false, false) => LidVerdict::StrengthExceedsConvectiveStress,
        _ => LidVerdict::BracketStraddlesConvectiveStress,
    };
    Some(LidReferee {
        verdict,
        lid_base_km,
        strength_low,
        strength_high,
        convective_stress_mpa,
    })
}

/// A LITHOSPHERIC COLUMN'S OWN DATA, enough to derive its elastic thickness. This is the glue the render's flexural
/// middle needs: it composes the geotherm, the friction row, the creep rows, and the derived conductive lid into a
/// [`LithosphereEnvelope`] and solves the banded moment equivalence for the `T_e` band.
///
/// # THE ALIEN IS A DATA ROW
///
/// Every input is per-material or per-body: `friction` and `creep` are the MATERIAL's (a silicate lid passes
/// [`crate::yield_envelope::rock_friction_law`] with olivine creep; a Europa shell passes
/// [`crate::yield_envelope::ice_friction_law`] with ice creep rows once they are built), and the geotherm,
/// densities, and gravity are the WORLD's. Nothing here is silicate-shaped: there is no material enum and no
/// rock-versus-ice dispatch, so the material is a data row, which is material-agnosticism BY CONSTRUCTION. What is
/// demonstrated today is the silicate deriving; the ice shell REFUSES (no ice creep row is built yet), so the
/// deriving-alien half of "one code path" awaits the ice rows rather than being shown here.
///
/// # THE TYPED REFUSAL IS A FEATURE GATE
///
/// [`Self::elastic_thickness_band_km`] returns `None` where the column cannot yield a `T_e`: a material with no
/// admitted creep row (its ductile branch refuses, which is the honest state of a material whose creep rows are not
/// yet built), a geotherm that refuses, or a load the envelope cannot elastically carry. A consumer renders ZERO
/// flexure with a stated reason there, never a fabricated relief. This is the honest empty middle, not a painted
/// one.
pub struct LidColumn<'a> {
    /// The material's friction row.
    pub friction: FrictionLaw,
    /// The material's creep candidates (the ductile branch). An empty or all-refusing set refuses the column.
    pub creep: &'a [CreepCandidate<'a>],
    /// The steady conductive geotherm's inputs (see [`crate::geotherm::steady_conductive_geotherm`]).
    pub surface_temperature_k: Fixed,
    /// The interior (potential) temperature (K).
    pub interior_temperature_k: Fixed,
    /// The lid density (kg/m^3), for both the geotherm and the envelope's lithostatic axis (raw SI).
    pub density_kg_m3: Fixed,
    /// The radiogenic heat production the geotherm carries.
    pub heat_production: Fixed,
    /// The thermal conductivity the geotherm carries.
    pub thermal_conductivity: Fixed,
    /// The body's surface gravity (m/s^2), for the envelope's lithostatic axis (raw SI).
    pub gravity_m_s2: Fixed,
    /// The convecting layer's depth (km), for the DERIVED conductive lid base.
    pub convecting_depth_km: Fixed,
    /// The world's Rayleigh number, for the DERIVED conductive lid base.
    pub rayleigh: Fixed,
    /// The load's chord (its strain rate and timescale), which the envelope evaluates creep at.
    pub chord: LoadChord,
}

impl LidColumn<'_> {
    /// Derive this column's elastic-thickness BAND (km), low edge below high, or `None` where the envelope refuses.
    /// The band is the `V*` scatter propagated through the moment equivalence. `youngs_modulus_gpa` and
    /// `poisson_ratio` are the column's own moduli (a `T_e` is a chord over them, so they ride explicitly),
    /// `delta_rho` is the density contrast the deflection floats against (in `1000 kg/m^3`), `v0` the line load
    /// (`GPa km`), `steps` the grid sampling. The flexure kernel's `km/s^2` gravity is DERIVED from the body's own
    /// `gravity_m_s2`, never supplied a second time (one gravity, two unit systems).
    pub fn elastic_thickness_band_km(
        &self,
        youngs_modulus_gpa: Fixed,
        poisson_ratio: Fixed,
        delta_rho: Fixed,
        v0: Fixed,
        steps: u32,
    ) -> Option<(Fixed, Fixed)> {
        // ONE LID DERIVATION. The conductive lid base sets BOTH the moment-integral domain AND the depth over which
        // the geotherm ramps from the surface to the interior temperature. The stress and the geotherm must agree
        // about how thick the lid is (the `thermal_boundary_layer` "one derivation" invariant, laws.rs), so the
        // geotherm's ramp IS the derived lid base, never a second free input that could disagree with it. A free
        // ramp length longer than the derived base would read the geotherm too cold at the base and bias `T_e` high.
        let lid_base = ConductiveLidBase::from_rayleigh(self.convecting_depth_km, self.rayleigh)?;
        let lid_thickness_km = lid_base.depth_km();
        let geotherm = |depth_km: Fixed| {
            crate::geotherm::steady_conductive_geotherm(
                self.surface_temperature_k,
                self.interior_temperature_k,
                lid_thickness_km,
                depth_km,
                self.density_kg_m3,
                self.heat_production,
                self.thermal_conductivity,
            )
        };
        // ONE GRAVITY: the flexure kernel's `km/s^2` gravity is the body's own `m/s^2` gravity converted once, never
        // a second free copy that could disagree with the lithostatic axis's.
        let gravity_km_s2 = self.gravity_m_s2.checked_div(Fixed::from_int(M_PER_KM))?;
        let envelope = LithosphereEnvelope {
            friction: self.friction,
            density_kg_m3: self.density_kg_m3,
            gravity_m_s2: self.gravity_m_s2,
            geotherm_k: &geotherm,
            creep: self.creep,
            chord: self.chord,
            lid_base,
        };
        let banded = solve_line_load_banded(
            &envelope,
            steps,
            youngs_modulus_gpa,
            poisson_ratio,
            delta_rho,
            gravity_km_s2,
            v0,
            self.chord,
        )
        .ok()?;
        banded.elastic_thickness_band_km(youngs_modulus_gpa, poisson_ratio)
    }

    /// D_mech (`GPa km^3`), THE MECHANICAL RIGIDITY the field filter runs at: the fully-elastic rigidity of the
    /// lid's own domain, `D = E * domain^3 / (12 (1 - nu^2))` with the domain the conductive lid base. This is the
    /// linear-response rigidity a linear transfer function is only valid at, the `v0 -> 0` limit of the
    /// load-conditioned rigidity [`Self::elastic_thickness_band_km`] reads.
    ///
    /// THE LIMIT HAS A CLOSED ANSWER, so this is a derivation and not an evaluation at zero (which the
    /// load-conditioned solve cannot do: no load, no moment, no `T_e`). As curvature `K -> 0` the yielded skins
    /// shrink as `O(K)`, the top because fibre stress and near-surface brittle strength vanish together, the bottom
    /// because vanishing stress falls below the creep floor, so the elastic core is the FULL lid domain and the
    /// mechanical rigidity is the fully-elastic rigidity of that domain. This is verbatim [`solve_line_load`]'s
    /// derived initial trial, the stiffest the column could be (the `D0` its fixed-point walk starts from), reused
    /// here rather than re-derived. The load-conditioned rigidity can only be softer, so D_mech is the elastic
    /// ceiling, and everything the field filter's linearity excludes routes to the load-conditioned solve instead.
    ///
    /// `None` where the conductive lid base or the rigidity does not resolve.
    pub fn mechanical_rigidity(
        &self,
        youngs_modulus_gpa: Fixed,
        poisson_ratio: Fixed,
    ) -> Option<Fixed> {
        let lid_base = ConductiveLidBase::from_rayleigh(self.convecting_depth_km, self.rayleigh)?;
        crate::flexure::flexural_rigidity(youngs_modulus_gpa, poisson_ratio, lid_base.depth_km())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creep_rows::{
        hk_dry_dislocation, hk_dry_dislocation_activation_volumes,
        hk_table2_activation_volume_determinations, ln_scientific, select_activation_volume,
        ActivationVolume, Modality, VolumeConstraint,
    };
    use crate::geotherm::steady_conductive_geotherm;
    use crate::yield_envelope::{ice_friction_law, rock_friction_law, LowStressBand};

    // McNutt and Menard's Table 1, printed under the heading "Assumed values for physical parameters", and the
    // same pair Watts and Burov assume. These are the LITERATURE's moduli, entered here because the tests
    // reproduce the LITERATURE's own worked numbers; a world's own derived pair is what the engine would pass.
    fn lit_e() -> Fixed {
        Fixed::from_int(80) // E = 8 x 10^10 N m^-2 = 80 GPa
    }
    fn lit_nu() -> Fixed {
        Fixed::from_ratio(1, 4) // nu = 0.25
    }
    fn f64_of(x: Fixed) -> f64 {
        x.to_f64_lossy()
    }

    /// McNutt and Menard's own worked illustration (their p. 366): a plate of thickness `H` with a UNIFORM yield
    /// strength, symmetric in tension and compression, and nothing below it. This is the source's fixture, not a
    /// physical envelope, and it exists so the moment integral and the extraction can be refereed against the
    /// numbers the source printed from it.
    struct UniformYieldEnvelope {
        yield_gpa: Fixed,
        thickness_km: Fixed,
    }
    impl YieldEnvelope for UniformYieldEnvelope {
        fn tensile_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed> {
            if depth_km <= self.thickness_km {
                Some(self.yield_gpa)
            } else {
                Some(ZERO)
            }
        }
        fn compressive_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed> {
            self.tensile_yield_gpa(depth_km)
        }
        fn domain_max_depth_km(&self) -> Fixed {
            self.thickness_km
        }
    }

    /// An envelope with the two senses set INDEPENDENTLY, so the neutral surface has something asymmetric to
    /// respond to. It is the asymmetry, not the magnitude, that moves `z_n` off mid-plate.
    struct AsymmetricEnvelope {
        tensile_gpa: Fixed,
        compressive_gpa: Fixed,
        thickness_km: Fixed,
    }
    impl YieldEnvelope for AsymmetricEnvelope {
        fn tensile_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed> {
            if depth_km <= self.thickness_km {
                Some(self.tensile_gpa)
            } else {
                Some(ZERO)
            }
        }
        fn compressive_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed> {
            if depth_km <= self.thickness_km {
                Some(self.compressive_gpa)
            } else {
                Some(ZERO)
            }
        }
        fn domain_max_depth_km(&self) -> Fixed {
            self.thickness_km
        }
    }

    /// McNutt and Menard's OWN PRINTED BRITTLE ENVELOPE, their eqs. 7 and 8, transcribed as they print them:
    ///
    ///   `Delta-sigma = +0.68 rho g z + 89 MPa`   in tension
    ///   `Delta-sigma = -2.17 rho g z - 283 MPa`  in compression
    ///
    /// DEPTH-DEPENDENT AND ASYMMETRIC, and it has to be BOTH to carry the curvature-sign effect. A depth-uniform
    /// asymmetric envelope is MIRROR-SYMMETRIC about mid-plate: swapping the sign of the curvature swaps which
    /// cap sits on which side, the neutral surface moves to the mirrored depth, and the moment magnitude is
    /// unchanged, so `T_e` does not depend on the sign at all. It is the envelope RISING WITH DEPTH that breaks
    /// the mirror: the weak tensile cap sits at the shallow top where it is weakest, and that is what makes a
    /// concave-down plate read thinner. This fixture is the primary's own envelope, so the test that reads its
    /// printed asymmetry is refereed by the same paper that printed the sentence.
    struct McNuttMenardBrittleEnvelope {
        thickness_km: Fixed,
    }
    impl McNuttMenardBrittleEnvelope {
        fn rho_g_z_mpa(&self, depth_km: Fixed) -> Option<Fixed> {
            lithostatic_normal_stress_mpa(
                Fixed::from_int(3300),
                Fixed::from_ratio(981, 100),
                depth_km.checked_mul(Fixed::from_int(1000))?,
            )
        }
    }
    impl YieldEnvelope for McNuttMenardBrittleEnvelope {
        fn tensile_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed> {
            if depth_km > self.thickness_km {
                return Some(ZERO);
            }
            let rgz = self.rho_g_z_mpa(depth_km)?;
            // 0.68 rho g z + 89, in MPa, then to GPa.
            Fixed::from_ratio(68, 100)
                .checked_mul(rgz)?
                .checked_add(Fixed::from_int(89))?
                .checked_div(Fixed::from_int(1000))
        }
        fn compressive_yield_gpa(&self, depth_km: Fixed) -> Option<Fixed> {
            if depth_km > self.thickness_km {
                return Some(ZERO);
            }
            let rgz = self.rho_g_z_mpa(depth_km)?;
            // 2.17 rho g z + 283, in MPa, then to GPa (the magnitude; the sign is the fibre's).
            Fixed::from_ratio(217, 100)
                .checked_mul(rgz)?
                .checked_add(Fixed::from_int(283))?
                .checked_div(Fixed::from_int(1000))
        }
        fn domain_max_depth_km(&self) -> Fixed {
            self.thickness_km
        }
    }

    /// The 40 km / 500 MPa fixture of the primary's illustration, sampled fine enough that the trapezoid's own
    /// error sits far below the assertions (see `the_moment_is_converged_in_the_grid_it_is_sampled_on`).
    fn mm_illustration_profile(yield_gpa: Fixed) -> EnvelopeProfile {
        EnvelopeProfile::sample(
            &UniformYieldEnvelope {
                yield_gpa,
                thickness_km: Fixed::from_int(40),
            },
            4000,
        )
        .expect("the illustration samples")
    }

    /// `K = -5 x 10^-7 m^-1`, the curvature of the primary's own illustration, in this module's `km^-1`.
    fn mm_illustration_curvature() -> FibreCurvature {
        FibreCurvature::from_upward_deflection(Fixed::from_ratio(-1, 2000))
    }

    #[test]
    fn the_mohr_coulomb_resolution_reproduces_the_primarys_printed_envelope() {
        // THE REFEREE FOR THE BRITTLE RESOLUTION, and it is twin-independent by construction: McNutt and
        // Menard's eqs. 7 and 8 were computed by McNutt and Menard, printed in their own paper, and reached this
        // codebase only as digits on a page. Reproducing them is not this module agreeing with itself.
        //
        //   Delta-sigma = -2.17 rho g z - 283 MPa   (compression, their eq. 7)
        //   Delta-sigma = +0.68 rho g z +  89 MPa   (tension,     their eq. 8)
        //
        // Their stated inputs are Byerlee friction with mu = 0.6 and a cohesion of 80 MPa (their eq. 6, whose
        // printed "600" has lost a decimal point; the fetch convicts that typo from these very equations).
        let mu = Fixed::from_ratio(6, 10);
        let s0 = Fixed::from_int(80);
        let at = |sv: i32, sense| {
            mohr_coulomb_differential_mpa(mu, s0, Fixed::from_int(sv), sense)
                .expect("the resolution evaluates")
                .0
        };

        // Intercepts, read at zero overburden.
        let comp_intercept = f64_of(at(0, FaultingSense::Thrust));
        let tens_intercept = f64_of(at(0, FaultingSense::Normal));
        // Slopes, read over a 100 MPa span of overburden.
        let comp_slope = (f64_of(at(100, FaultingSense::Thrust)) - comp_intercept) / 100.0;
        let tens_slope = (f64_of(at(100, FaultingSense::Normal)) - tens_intercept) / 100.0;

        // THE TOLERANCES ARE THE FETCH'S OWN REPORTED RESIDUALS, not room to be wrong in. The back-solve gives
        // -2.119 rho g z - 282.6 and +0.679 rho g z + 90.6 against the printed -2.17/-283 and +0.68/+89, and the
        // fetch reports that gap rather than resolving it (the paper prints two significant figures on the
        // slopes). The bounds sit just outside those residuals.
        assert!(
            (comp_intercept - 283.0).abs() < 1.0,
            "compression intercept: {comp_intercept} against the primary's printed 283 MPa"
        );
        assert!(
            (comp_slope - 2.17).abs() < 0.06,
            "compression slope: {comp_slope} against the primary's printed 2.17"
        );
        assert!(
            (tens_intercept - 89.0).abs() < 2.0,
            "tension intercept: {tens_intercept} against the primary's printed 89 MPa"
        );
        assert!(
            (tens_slope - 0.68).abs() < 0.01,
            "tension slope: {tens_slope} against the primary's printed 0.68"
        );

        // THE ASYMMETRY IS THE LOAD-BEARING PART, in the source's own words: "the upper plate is significantly
        // stronger in compression as compared to tension". About three times, and the whole curvature-sign
        // dependence rests on it.
        assert!(
            comp_intercept > 3.0 * tens_intercept,
            "compression must be far stronger than tension: {comp_intercept} against {tens_intercept}"
        );

        // THE GUARD'S DISCRIMINATING POWER, asserted rather than assumed. Byerlee (1978)'s half-kilobar cohesion
        // is 50 MPa and McNutt and Menard's is 80; the fetch reports that conflict unresolved and names it as a
        // reading the cited source does not carry. A resolution run at 50 MPa misses the printed intercept by
        // about 106 MPa, a hundred times the tolerance above, so this referee cannot fail to notice which
        // cohesion it was handed.
        let at_50 = f64_of(
            mohr_coulomb_differential_mpa(mu, Fixed::from_int(50), ZERO, FaultingSense::Thrust)
                .expect("evaluates")
                .0,
        );
        assert!(
            (at_50 - 283.0).abs() > 100.0,
            "a 50 MPa cohesion misses the primary's printed intercept decisively, got {at_50}"
        );
    }

    #[test]
    fn the_purely_elastic_plate_returns_its_own_thickness() {
        // THE SHARPEST IDENTITY THE PRIMARY PRINTS, and the one that pins the plane-strain factor. McNutt and
        // Menard state that for the purely elastic plate "we would find from equations (3)-(5) that the base of
        // the plate H equals T_e". So an envelope strong enough that nothing yields must return T_e = H EXACTLY,
        // and the result is independent of E and nu: the plane-strain modulus in the fibre stress cancels
        // against the one inside D through the cube root.
        //
        // THIS IS WHY IT IS A TEST AND NOT A COMMENT. Drop the 1/(1 - nu^2) from the fibre stress and the
        // elastic limit returns H * (1 - nu^2)^(1/3) = 39.15 km against 40, a 2 percent error that looks exactly
        // like quadrature noise and would never be noticed by an assertion on a physical band.
        let profile = mm_illustration_profile(Fixed::from_int(100_000)); // an envelope nothing reaches
        let k = mm_illustration_curvature();
        let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n");
        let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).expect("M");
        let d = equivalent_rigidity(m.moment, k).expect("D");
        let te = elastic_thickness_km(d, lit_e(), lit_nu()).expect("T_e");
        assert!(
            (f64_of(te) - 40.0).abs() < 0.02,
            "the purely elastic plate returns its own thickness: T_e = {} against H = 40",
            f64_of(te)
        );
        // The rigidity is the textbook D(H) at the same moduli, which is the same identity read in the canonical
        // currency: D = E H^3 / (12 (1 - nu^2)) = 455111 GPa km^3.
        let expect_d = 80.0 * 40.0_f64.powi(3) / (12.0 * (1.0 - 0.0625));
        assert!(
            (f64_of(d) - expect_d).abs() < expect_d * 1e-4,
            "D = {} against the textbook D(H) = {expect_d}",
            f64_of(d)
        );
        // AND THE INDEPENDENCE, which is the half that convicts the plane-strain factor: a DIFFERENT modulus
        // pair must return the same 40 km, because E and nu cancel in the elastic limit.
        let soft_e = Fixed::from_int(30);
        let soft_nu = Fixed::from_ratio(35, 100);
        let z_n2 = neutral_surface_depth_km(&profile, k, soft_e, soft_nu).expect("z_n");
        let m2 = bending_moment(&profile, k, z_n2, soft_e, soft_nu).expect("M");
        let d2 = equivalent_rigidity(m2.moment, k).expect("D");
        let te2 = elastic_thickness_km(d2, soft_e, soft_nu).expect("T_e");
        assert!(
            (f64_of(te2) - 40.0).abs() < 0.02,
            "the elastic limit is independent of the modulus pair: T_e = {} at E = 30, nu = 0.35",
            f64_of(te2)
        );
    }

    #[test]
    fn the_elastic_plastic_worked_example_matches_the_primarys_printed_answer() {
        // THE REFEREE FOR THE WHOLE CONSTRUCTION, and it came from outside this codebase. McNutt and Menard's
        // p. 366 illustration: H = 40 km, z_n = 20 km, K = -5 x 10^-7 m^-1, a uniform 500 MPa yield strength.
        // Their printed answer: "a purely elastic plate with the same moment and curvature would be LESS THAN 37
        // KM THICK", against the 40 km the same plate returns when nothing yields.
        //
        // Every piece of the construction is on trial here: the elastic-plastic cap, the plane-strain modulus,
        // the lever arm about the neutral surface, the trapezoid, the rigidity, and the cube root.
        let profile = mm_illustration_profile(Fixed::from_ratio(1, 2)); // 500 MPa = 0.5 GPa
        let k = mm_illustration_curvature();
        let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n");
        let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).expect("M");
        let d = equivalent_rigidity(m.moment, k).expect("D");
        let te = f64_of(elastic_thickness_km(d, lit_e(), lit_nu()).expect("T_e"));

        // THE PRIMARY'S OWN BOUND, verbatim.
        assert!(
            te < 37.0,
            "the primary prints 'less than 37 km' for its own illustration, got {te}"
        );
        // AND THE BOUND IS NOT VACUOUS: the same plate unyielded is 40 km, so 'less than 37' is a real
        // constraint on the yielding, and the construction must land in the narrow band an independent
        // evaluation of the source's own arithmetic gives (36.794 km).
        assert!(
            (te - 36.794).abs() < 0.05,
            "the elastic-plastic illustration lands at 36.794 km, got {te}"
        );
        assert!(
            te > 36.0,
            "the yielding must not swallow the plate: {te} against the 40 km elastic limit"
        );
    }

    #[test]
    fn the_symmetric_envelope_puts_the_neutral_surface_at_mid_plate() {
        // The primary's illustration sets z_n = 20 km for H = 40 km, and the zero-net-axial-force condition must
        // REPRODUCE that rather than be told it. A symmetric envelope has a stress profile odd about mid-plate,
        // so mid-plate is where the fibre stresses sum to zero. This is the solve agreeing with the source's own
        // stated neutral surface on the source's own fixture.
        let profile = mm_illustration_profile(Fixed::from_ratio(1, 2));
        let k = mm_illustration_curvature();
        let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n");
        assert!(
            (f64_of(z_n) - 20.0).abs() < 0.01,
            "a symmetric envelope zeroes the axial force at mid-plate: z_n = {} against the primary's 20 km",
            f64_of(z_n)
        );
    }

    #[test]
    fn the_asymmetric_envelope_moves_the_neutral_surface_off_mid_plate() {
        // THE REASON z_n IS SOLVED AND NOT PINNED. McNutt and Menard's own envelope is strongly asymmetric
        // (about three times stronger in compression), so a yielded profile CANNOT be symmetric about mid-plate
        // and still integrate to zero. Their text treats z_n as a model output an envelope relocates, reporting
        // it "below 40 km" under a different envelope. A consumer that pins z_n = H/2 has adopted an assumption
        // the primaries do not make and their own envelope contradicts, and this test is what would catch it.
        let profile = EnvelopeProfile::sample(
            &AsymmetricEnvelope {
                tensile_gpa: Fixed::from_ratio(1, 10),     // 100 MPa in tension
                compressive_gpa: Fixed::from_ratio(3, 10), // 300 MPa in compression
                thickness_km: Fixed::from_int(40),
            },
            4000,
        )
        .expect("profile");
        let k = mm_illustration_curvature();
        let z_n = f64_of(neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n"));
        assert!(
            (z_n - 20.0).abs() > 1.0,
            "an asymmetric envelope must move the neutral surface off mid-plate, got {z_n}"
        );
        assert!(
            z_n > 0.0 && z_n < 40.0,
            "the neutral surface stays inside the plate, got {z_n}"
        );
        // The axial force it found really is zero, which is the condition itself rather than a proxy for it.
        let plane_strain = plane_strain_modulus_gpa(lit_e(), lit_nu()).unwrap();
        let n = axial_force(
            &profile,
            k,
            Fixed::from_ratio((z_n * 1e6) as i64, 1_000_000),
            plane_strain,
        )
        .expect("N");
        assert!(
            f64_of(n).abs() < 0.01,
            "the solve zeroes the net axial force, got N = {}",
            f64_of(n)
        );
    }

    #[test]
    fn the_curvature_sign_reproduces_the_primarys_asymmetry() {
        // THE SIGN CONVENTION ON TRIAL, refereed by a sentence the primary printed: "Other factors being equal,
        // a plate with negative curvature (concave downward) will appear to have a SMALLER T_e than a plate with
        // positive curvature" (their p. 367).
        //
        // Under this module's stated convention, K < 0 puts the TENSILE fibre at the shallow top, where the
        // envelope is weak in tension, so the yielded moment is smaller and so is T_e. Nothing in the code says
        // that; it falls out of the fibre-stress sign joined to the envelope's asymmetry. If the convention were
        // flipped, this test reverses and fires.
        //
        // THE FIXTURE MUST BE THE PRIMARY'S OWN DEPTH-DEPENDENT ENVELOPE, and finding out why cost a test. A
        // depth-UNIFORM asymmetric envelope is mirror-symmetric about mid-plate, so it reports the SAME T_e for
        // both signs of the curvature and cannot see this effect at all: the sign dependence needs the envelope
        // to RISE WITH DEPTH, which is what puts the weak tensile cap at the shallow top. See the fixture's doc.
        let profile = EnvelopeProfile::sample(
            &McNuttMenardBrittleEnvelope {
                thickness_km: Fixed::from_int(40),
            },
            4000,
        )
        .expect("profile");
        let te_at = |k: FibreCurvature| {
            let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n");
            let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).expect("M");
            let d = equivalent_rigidity(m.moment, k).expect("D");
            f64_of(elastic_thickness_km(d, lit_e(), lit_nu()).expect("T_e"))
        };
        let concave_down = te_at(FibreCurvature::from_upward_deflection(Fixed::from_ratio(
            -1, 2000,
        )));
        let concave_up = te_at(FibreCurvature::from_upward_deflection(Fixed::from_ratio(
            1, 2000,
        )));
        assert!(
            concave_down < concave_up,
            "the primary's own asymmetry: concave-down T_e = {concave_down} must be smaller than concave-up T_e = {concave_up}"
        );
        // And the effect is not a rounding: the two differ by a real margin, which is what makes the sign
        // load-bearing rather than cosmetic.
        assert!(
            concave_up - concave_down > 0.5,
            "the curvature sign moves T_e by a real margin: {concave_down} against {concave_up}"
        );
    }

    #[test]
    fn the_elastic_thickness_falls_as_curvature_rises() {
        // Both primaries state it and the fetch quotes both verbatim. Watts and Burov: "T_e slowly decreases
        // with increasing curvature and, hence, bending stress". McNutt and Menard: "a lithospheric plate which
        // is more sharply bent will appear thinner than an identical plate with lower curvature". The mechanism
        // is moment saturation: past yielding, more curvature adds no more moment, so T_e = (M/K)^(1/3) falls.
        let profile = mm_illustration_profile(Fixed::from_ratio(1, 2));
        let te_at = |num: i64, den: i64| {
            let k = FibreCurvature::from_upward_deflection(Fixed::from_ratio(num, den));
            let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n");
            let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).expect("M");
            let d = equivalent_rigidity(m.moment, k).expect("D");
            f64_of(elastic_thickness_km(d, lit_e(), lit_nu()).expect("T_e"))
        };
        // A monotone ladder over three decades of curvature, all concave-down.
        let gentle = te_at(-1, 200_000); // 1e-8 m^-1: barely bent
        let middling = te_at(-1, 20_000); // 1e-7 m^-1
        let sharp = te_at(-1, 2000); // 5e-7 m^-1, the primary's own illustration
        let sharper = te_at(-1, 500); // 2e-6 m^-1
        assert!(
            gentle > middling && middling > sharp && sharp > sharper,
            "T_e must fall as curvature rises: {gentle} > {middling} > {sharp} > {sharper}"
        );
        // THE LOW-CURVATURE LIMIT IS THE ELASTIC ONE, which is the other half of the same statement: a plate
        // bent gently enough yields nowhere and reads its own thickness. Watts and Burov report the ratio
        // T_e(YSE)/T_e(elastic) reaching 1 for K below 1e-8 m^-1.
        assert!(
            (gentle - 40.0).abs() < 0.1,
            "a barely bent plate reads its own thickness: {gentle} against H = 40"
        );
        // And the sharply bent one is well below it, which is the saturation biting.
        assert!(
            sharper < 30.0,
            "a sharply bent plate reads far thinner: {sharper}"
        );
    }

    #[test]
    fn the_first_zero_crossing_curvature_twins_against_the_landed_kernel() {
        // THE NUMERICAL TWIN, and it is independent by construction: the analytic form here was differentiated
        // by hand from the profile, while the twin takes a SECOND DIFFERENCE of the landed kernel's own
        // `line_load_deflection`. They share no arithmetic, so agreement is evidence rather than a series
        // checked against itself. It convicts a sign error, a lost factor of two, a wrong e^(-3 pi / 4), and a
        // zero crossing read at the wrong x.
        let d = crate::flexure::flexural_rigidity(lit_e(), lit_nu(), Fixed::from_int(40)).unwrap();
        let alpha = crate::flexure::flexural_parameter(
            d,
            Fixed::from_ratio(33, 10),
            Fixed::from_ratio(98, 10000),
        )
        .unwrap();
        let v0 = Fixed::from_int(56);
        let analytic = line_load_curvature_at_first_zero_crossing(v0, alpha, d).expect("K");
        // The kernel's convention is w positive DOWNWARD, so the analytic value in that convention is the
        // negation of the upward one the type carries.
        let analytic_down = -f64_of(analytic.upward_per_km());

        // The zero crossing itself, from the kernel's own documented x0 = 3 pi alpha / 4.
        let a = f64_of(alpha);
        let x0 = 3.0 * std::f64::consts::PI * a / 4.0;
        // FIRST, the kernel really does cross zero there, which is what makes this the right x to read.
        let w_at_x0 = f64_of(
            crate::flexure::line_load_deflection(
                v0,
                alpha,
                d,
                Fixed::from_ratio((x0 * 1e6) as i64, 1_000_000),
            )
            .unwrap(),
        );
        let w_peak = f64_of(crate::flexure::line_load_deflection(v0, alpha, d, ZERO).unwrap());
        assert!(
            w_at_x0.abs() < w_peak.abs() * 0.01,
            "the kernel's deflection vanishes at x0 = 3 pi alpha / 4: w = {w_at_x0} against a peak of {w_peak}"
        );

        // The second difference. The step is 1 km: large enough that the differenced fixed-point values keep
        // several significant digits (a step near the fixed-point floor would cancel catastrophically), small
        // enough that the O(h^2) truncation sits far below the tolerance, since the profile's own length scale
        // is alpha and alpha is tens of kilometres.
        let h = 1.0_f64;
        let w = |x: f64| {
            f64_of(
                crate::flexure::line_load_deflection(
                    v0,
                    alpha,
                    d,
                    Fixed::from_ratio((x * 1e6) as i64, 1_000_000),
                )
                .unwrap(),
            )
        };
        let numeric_down = (w(x0 + h) - 2.0 * w(x0) + w(x0 - h)) / (h * h);
        assert!(
            (numeric_down - analytic_down).abs() < analytic_down.abs() * 0.02,
            "the analytic first-zero-crossing curvature {analytic_down} against the kernel's second difference {numeric_down}"
        );
        // THE SIGN, which is the load-bearing half: a downward line load bends the plate CONCAVE DOWN at its
        // first zero crossing, so the curvature is NEGATIVE in the upward convention the construction reads. The
        // primary's own fitted trench curvatures carry that sign.
        assert!(
            analytic.upward_per_km() < ZERO,
            "a downward load reads a negative (concave-down) curvature at the zero crossing, got {:?}",
            analytic.upward_per_km()
        );
    }

    #[test]
    fn the_curvature_convention_flip_is_the_type_and_not_the_reader() {
        // The two deflection conventions disagree about the sign of the curvature, and T_e depends on that sign.
        // So the flip lives in the constructor and nowhere else, and there is no way to build a FibreCurvature
        // without naming a convention.
        let k = Fixed::from_ratio(1, 1000);
        assert_eq!(
            FibreCurvature::from_upward_deflection(k).upward_per_km(),
            k,
            "the upward convention is the construction's own and passes through"
        );
        assert_eq!(
            FibreCurvature::from_downward_deflection(k).upward_per_km(),
            ZERO - k,
            "the downward convention (the landed kernel's) flips exactly once"
        );
    }

    #[test]
    fn the_moment_integral_self_truncates_on_a_dying_envelope() {
        // THE RULING'S MECHANISM, on an envelope whose premise HOLDS. The primary's illustration has zero
        // strength below 40 km, so past the plate the integrand is exactly zero, the observed decay ratio is
        // zero, the geometric tail bound collapses below one representable unit, and the integration stops
        // without anyone naming a depth.
        let envelope = UniformYieldEnvelope {
            yield_gpa: Fixed::from_ratio(1, 2),
            thickness_km: Fixed::from_int(40),
        };
        // A domain DEEPER than the plate, so there is a dead tail to truncate rather than a coincidence between
        // the plate's base and the grid's end.
        struct DeepDomain(UniformYieldEnvelope);
        impl YieldEnvelope for DeepDomain {
            fn tensile_yield_gpa(&self, z: Fixed) -> Option<Fixed> {
                self.0.tensile_yield_gpa(z)
            }
            fn compressive_yield_gpa(&self, z: Fixed) -> Option<Fixed> {
                self.0.compressive_yield_gpa(z)
            }
            fn domain_max_depth_km(&self) -> Fixed {
                Fixed::from_int(120)
            }
        }
        let profile = EnvelopeProfile::sample(&DeepDomain(envelope), 1200).expect("profile");
        let k = mm_illustration_curvature();
        let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n");
        let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).expect("M");
        assert!(
            m.self_truncated,
            "an envelope with no strength below its plate must self-truncate, reading {m:?}"
        );
        let depth = f64_of(m.truncation_depth_km.expect("a truncation depth"));
        assert!(
            (40.0..60.0).contains(&depth),
            "the truncation lands just past the plate's base, got {depth} km"
        );
        // AND THE TRUNCATION COST NOTHING, which is what the residue budget means and is tested as the property
        // itself: a SELF-TRUNCATED moment must not depend on where the domain was declared, so declaring it
        // twice as deep must return the same number to the bit. This is the exact mirror of
        // `the_moment_integral_reports_domain_limited_when_the_tail_lives`, where the same widening MOVES the
        // answer, and the pair is what separates the two regimes.
        //
        // THE COMPARISON IS AGAINST A WIDER DOMAIN RATHER THAN A TIGHTER ONE, and finding out why cost a test.
        // Differencing this against a profile that stops at the plate's base measures neither the truncation nor
        // the budget: this fixture's envelope JUMPS from 0.5 GPa to zero at 40 km, and a trapezoid smears that
        // discontinuity across one interval, contributing a spurious half a unit that the tighter profile never
        // sees because its grid ends on the jump. That is a quadrature artifact of a fixture with a cliff in it
        // (a real envelope decays to zero continuously), and it has nothing to say about truncation.
        struct DeeperDomain(UniformYieldEnvelope);
        impl YieldEnvelope for DeeperDomain {
            fn tensile_yield_gpa(&self, z: Fixed) -> Option<Fixed> {
                self.0.tensile_yield_gpa(z)
            }
            fn compressive_yield_gpa(&self, z: Fixed) -> Option<Fixed> {
                self.0.compressive_yield_gpa(z)
            }
            fn domain_max_depth_km(&self) -> Fixed {
                Fixed::from_int(240)
            }
        }
        let deeper = EnvelopeProfile::sample(
            &DeeperDomain(UniformYieldEnvelope {
                yield_gpa: Fixed::from_ratio(1, 2),
                thickness_km: Fixed::from_int(40),
            }),
            2400,
        )
        .expect("profile");
        assert_eq!(
            deeper.step_km(),
            profile.step_km(),
            "the two profiles share a grid step, so only the declared domain differs"
        );
        let z_n_deep = neutral_surface_depth_km(&deeper, k, lit_e(), lit_nu()).expect("z_n");
        let m_deep = bending_moment(&deeper, k, z_n_deep, lit_e(), lit_nu()).expect("M");
        assert!(
            m_deep.self_truncated,
            "the deeper domain self-truncates too: {m_deep:?}"
        );
        assert_eq!(
            m_deep.moment,
            m.moment,
            "a self-truncated moment does not depend on the declared domain: {} against {}",
            f64_of(m.moment),
            f64_of(m_deep.moment)
        );
    }

    #[test]
    fn the_moment_integral_reports_domain_limited_when_the_tail_lives() {
        // THE PREMISE'S OTHER SIDE, and the reason the reading carries a flag rather than a promise. An envelope
        // that keeps a floor of strength to the bottom of its domain has an integrand that GROWS with the lever
        // arm, so no geometric tail bound exists, the integration runs to the declared domain, and the moment
        // DEPENDS on where the caller declared it. That dependence is real and it is reported rather than hidden.
        //
        // This is the shape of the creep envelope: a power-law row's strength tends to (eps/A)^(1/n) rather than
        // to zero, so its moment integrand never dies. See this function's own doc.
        struct FlooredEnvelope;
        impl YieldEnvelope for FlooredEnvelope {
            fn tensile_yield_gpa(&self, _z: Fixed) -> Option<Fixed> {
                Some(Fixed::from_ratio(1, 1000)) // a 1 MPa floor that never dies
            }
            fn compressive_yield_gpa(&self, _z: Fixed) -> Option<Fixed> {
                Some(Fixed::from_ratio(1, 1000))
            }
            fn domain_max_depth_km(&self) -> Fixed {
                Fixed::from_int(200)
            }
        }
        let profile = EnvelopeProfile::sample(&FlooredEnvelope, 2000).expect("profile");
        let k = mm_illustration_curvature();
        let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n");
        let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).expect("M");
        assert!(
            !m.self_truncated,
            "an envelope whose strength never dies cannot self-truncate: {m:?}"
        );
        assert_eq!(m.truncation_depth_km, None);
        // THE TAIL IS MEASURED, NOT ASSUMED: the last interval still contributes, which is precisely why the
        // integral could not stop.
        assert!(
            m.final_interval_contribution > Fixed::EPSILON,
            "the final interval still contributes, so the tail is alive: {:?}",
            m.final_interval_contribution
        );
        // AND THE DEPENDENCE IS REAL: declare a deeper domain and the moment MOVES. This is the finding the
        // ruling's premise did not expect, made concrete.
        struct DeeperFloored;
        impl YieldEnvelope for DeeperFloored {
            fn tensile_yield_gpa(&self, _z: Fixed) -> Option<Fixed> {
                Some(Fixed::from_ratio(1, 1000))
            }
            fn compressive_yield_gpa(&self, _z: Fixed) -> Option<Fixed> {
                Some(Fixed::from_ratio(1, 1000))
            }
            fn domain_max_depth_km(&self) -> Fixed {
                Fixed::from_int(400)
            }
        }
        let deeper = EnvelopeProfile::sample(&DeeperFloored, 4000).expect("profile");
        let z_n2 = neutral_surface_depth_km(&deeper, k, lit_e(), lit_nu()).expect("z_n");
        let m2 = bending_moment(&deeper, k, z_n2, lit_e(), lit_nu()).expect("M");
        assert!(
            (f64_of(m2.moment) - f64_of(m.moment)).abs() > 0.1,
            "a deeper declared domain moves a domain-limited moment: {} against {}",
            f64_of(m.moment),
            f64_of(m2.moment)
        );
    }

    #[test]
    fn the_moment_is_converged_in_the_grid_it_is_sampled_on() {
        // THE QUADRATURE'S OWN RESOLUTION, checked rather than declared. The grid is the caller's, so the caller
        // is owed evidence that the answer does not depend on it: doubling the sampling must not move T_e by
        // more than the assertions elsewhere in this file tolerate. This is what licenses the 4000-step fixture.
        let te_at_steps = |steps: u32| {
            let profile = EnvelopeProfile::sample(
                &UniformYieldEnvelope {
                    yield_gpa: Fixed::from_ratio(1, 2),
                    thickness_km: Fixed::from_int(40),
                },
                steps,
            )
            .expect("profile");
            let k = mm_illustration_curvature();
            let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n");
            let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).expect("M");
            let d = equivalent_rigidity(m.moment, k).expect("D");
            f64_of(elastic_thickness_km(d, lit_e(), lit_nu()).expect("T_e"))
        };
        let coarse = te_at_steps(2000);
        let fine = te_at_steps(4000);
        let finer = te_at_steps(8000);
        assert!(
            (fine - coarse).abs() < 0.01 && (finer - fine).abs() < 0.01,
            "the moment is converged in the grid: {coarse} -> {fine} -> {finer}"
        );
    }

    // A V* determination from H&K's Table 2 (Karato and Jung 2002, V* = 14 over 0.3 to 2 GPa), used as a
    // FIXTURE. This module banks no V* determination and this is not a selection it endorses: Table 2 offers
    // nine values from -2 to 27, they fail to overlap because V* is a chord that decreases with pressure, and
    // picking one is a decision the primary declines to make. The assertions below are RELATIONS (which branch
    // binds where), none of which this number can move.
    fn table2_volume_fixture() -> ActivationVolume {
        ActivationVolume {
            cm3_per_mol: Fixed::from_int(14),
            interval_min_gpa: Fixed::from_ratio(3, 10),
            interval_max_gpa: Fixed::from_int(2),
            modality: Modality::Fitted,
        }
    }

    /// THE TABLE'S OWN EXTREMES as a FIXTURE: H&K's Table 2 offers nine determinations from -2 to 27 cm^3/mol,
    /// and this is that span's two ends carrying the banked chord's own lower limit of 0.3 GPa.
    ///
    /// IT BANKS NOTHING AND ENDORSES NOTHING. The nine values fail to overlap because `V*` is a chord that
    /// decreases with pressure, and picking one is a decision the primary declines to make. What this fixture is
    /// for is the OPPOSITE of picking: it is the widest span the source supports, so a test can prove the width
    /// cannot reach the shallow envelope's answer. Every assertion it appears in is a RELATION, and neither
    /// endpoint can move one.
    fn table2_span_fixture() -> [ActivationVolume; 2] {
        let chord = |v: i32| ActivationVolume {
            cm3_per_mol: Fixed::from_int(v),
            interval_min_gpa: Fixed::from_ratio(3, 10),
            interval_max_gpa: Fixed::from_int(2),
            modality: Modality::Fitted,
        };
        [chord(-2), chord(27)]
    }

    fn test_chord() -> LoadChord {
        LoadChord {
            class: LoadClassId(1),
            // A load timescale of about 1 Myr, and the strain rate that goes with it. Both are the TEST's, not
            // the engine's: the engine's caller derives them from the load's own emplacement history.
            timescale_s: Fixed::from_int(31_557_600),
            ln_strain_rate_per_s: ln_scientific(1, 1, -15),
        }
    }

    #[test]
    fn the_lithosphere_envelope_is_brittle_shallow_and_ductile_deep() {
        // THE ASSEMBLY, on the landed rows. "Brittle above, ductile below" is not a layering anyone imposed: it
        // falls out of taking the LESSER of two branches that move in opposite directions with depth, the
        // brittle rising with rho g z and the ductile falling as the geotherm climbs.
        let volumes = [table2_volume_fixture()];
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        // A steady conductive lid with no radiogenic source: the pure ramp from 273 K at the surface to 1600 K
        // at 100 km. Passed as a closure, so either landed geotherm form serves without this module dispatching.
        let geotherm = |z_km: Fixed| {
            steady_conductive_geotherm(
                Fixed::from_int(273),
                Fixed::from_int(1600),
                Fixed::from_int(100),
                z_km,
                Fixed::from_int(3300),
                ZERO,
                Fixed::from_int(3),
            )
        };
        // THE LID BASE IS DERIVED, not declared: a 2890 km convecting layer at Ra = 1e5 puts the conductive
        // boundary layer at `2890 / (1e5)^(1/3)` = about 62 km, which is deep enough to contain the
        // brittle-ductile crossover this test is about. The pair is the FIXTURE'S; what the engine passes is the
        // world's own layer depth and Rayleigh number.
        let lid_base =
            ConductiveLidBase::from_rayleigh(Fixed::from_int(2890), Fixed::from_int(100_000))
                .expect("a convecting layer has a boundary layer");
        assert!(
            (f64_of(lid_base.depth_km()) - 62.3).abs() < 0.5,
            "the derived lid base is d / Ra^(1/3): {} km",
            f64_of(lid_base.depth_km())
        );
        let env = LithosphereEnvelope {
            friction: rock_friction_law(),
            density_kg_m3: Fixed::from_int(3300),
            gravity_m_s2: Fixed::from_ratio(981, 100),
            geotherm_k: &geotherm,
            creep: &creep,
            chord: test_chord(),
            lid_base,
        };

        // SHALLOW (15 km, about 470 K): the creep law says deformation at a geological rate is IRRELEVANT here,
        // running the strength past what the type can hold, and the brittle branch floors the envelope. That is
        // the creep module's own documented edge behaving, not an error.
        let shallow = Fixed::from_int(15);
        // The V* fixture's chord covers this depth, so the bracket is degenerate and both ends read alike.
        for end in [VolumeEnd::Low, VolumeEnd::High] {
            assert_eq!(
                env.ductile(shallow, end),
                DuctileReading::AboveRepresentable,
                "a cold shallow lid is not creeping at a geological rate"
            );
        }
        let shallow_brittle = env
            .brittle(shallow, FaultingSense::Thrust)
            .expect("brittle");
        let shallow_yield = env.compressive_yield_gpa(shallow).expect("envelope");
        match shallow_brittle {
            DifferentialStrength::Determined(b) => assert!(
                (f64_of(shallow_yield) * 1000.0 - f64_of(b)).abs() < 1.0,
                "shallow, the BRITTLE branch is the envelope: {} MPa against {} MPa",
                f64_of(shallow_yield) * 1000.0,
                f64_of(b)
            ),
            other => panic!("rock determines everywhere, got {other:?}"),
        }

        // DEEP (60 km, about 1070 K): the creep law now determines, and it is far WEAKER than the brittle
        // branch, so the ductile branch is the envelope.
        let deep = Fixed::from_int(60);
        let deep_ductile = match env.ductile(deep, VolumeEnd::Low) {
            DuctileReading::Determined(d) => d,
            other => panic!("a hot deep lid creeps at a determinate strength, got {other:?}"),
        };
        let deep_brittle = match env.brittle(deep, FaultingSense::Thrust).expect("brittle") {
            DifferentialStrength::Determined(b) => b,
            other => panic!("rock determines, got {other:?}"),
        };
        assert!(
            deep_ductile < deep_brittle,
            "deep, creep is weaker than friction: {} MPa against {} MPa",
            f64_of(deep_ductile),
            f64_of(deep_brittle)
        );
        let deep_yield = env.compressive_yield_gpa(deep).expect("envelope");
        assert!(
            (f64_of(deep_yield) * 1000.0 - f64_of(deep_ductile)).abs() < 1.0,
            "deep, the DUCTILE branch is the envelope: {} MPa against {} MPa",
            f64_of(deep_yield) * 1000.0,
            f64_of(deep_ductile)
        );

        // AND THE ENVELOPE TURNS OVER, which is the shape the whole construction rests on: strength rises with
        // depth while friction binds, then falls once creep takes over.
        let at = |z: i32| {
            f64_of(
                env.compressive_yield_gpa(Fixed::from_int(z))
                    .expect("envelope"),
            )
        };
        assert!(
            at(10) < at(30),
            "the brittle limb rises with depth: {} then {}",
            at(10),
            at(30)
        );
        assert!(
            at(60) < at(30),
            "the ductile limb falls with depth: {} then {}",
            at(30),
            at(60)
        );
    }

    /// The Earth-like lid the two new checks share: a 2890 km convecting layer, a conductive ramp from 273 K to
    /// 1600 K over 100 km, the dry-olivine row, and the load's own 1e-15 per second. Every number is the
    /// FIXTURE'S; the engine's caller passes the world's.
    fn earth_like_lid<'a>(
        creep: &'a [CreepCandidate<'a>],
        geotherm: &'a dyn Fn(Fixed) -> Option<Fixed>,
        rayleigh: Fixed,
    ) -> LithosphereEnvelope<'a> {
        LithosphereEnvelope {
            // Un-banded rock: this helper isolates the lid, geotherm, and solve machinery, so it carries the
            // central low fit alone. The low-stress roughness band's shallow bracket is a SEPARATE feature, proven
            // by `the_low_stress_roughness_band_widens_the_silicate_te` and exercised through the canonical
            // `rock_friction_law` in the assembler test, not smuggled into every machinery test through the helper.
            friction: FrictionLaw {
                low_stress_band: None,
                ..rock_friction_law()
            },
            density_kg_m3: Fixed::from_int(3300),
            gravity_m_s2: Fixed::from_ratio(981, 100),
            geotherm_k: geotherm,
            creep,
            chord: test_chord(),
            lid_base: ConductiveLidBase::from_rayleigh(Fixed::from_int(2890), rayleigh)
                .expect("a convecting layer has a boundary layer"),
        }
    }

    fn ramp_geotherm(z_km: Fixed) -> Option<Fixed> {
        steady_conductive_geotherm(
            Fixed::from_int(273),
            Fixed::from_int(1600),
            Fixed::from_int(100),
            z_km,
            Fixed::from_int(3300),
            ZERO,
            Fixed::from_int(3),
        )
    }

    #[test]
    fn the_conductive_lid_base_is_derived_and_carries_that_derivation_in_its_type() {
        // THE DOMAIN THE MOMENT INTEGRAL NEEDS, and the reason it needs one: the integrand does NOT die (see
        // `bending_moment`), so the integral is bounded at the conductive-lid base, below which the mantle
        // overturns and a static load's stresses are not sustained.
        //
        // THE TWIN IS EXTERNAL: `2890 / (1e5)^(1/3)` = 2890 / 46.4159 = 62.263 km, a cube root taken outside
        // this codebase and typed here as a literal, against the fixed-point `powf` the law reaches for. The
        // tolerance is that function's own series accuracy, which the module header already names as the price
        // of `powf` over the exact `sqrt`.
        let delta =
            ConductiveLidBase::from_rayleigh(Fixed::from_int(2890), Fixed::from_int(100_000))
                .expect("a convecting layer has a boundary layer");
        assert!(
            (f64_of(delta.depth_km()) - 62.263).abs() < 0.05,
            "delta = d / Ra^(1/3): got {} km against the external 62.263",
            f64_of(delta.depth_km())
        );
        // AND IT IS THE LAW'S OWN NUMBER, not a second copy of the same expression: the driving stress, the lid
        // geotherm, and this domain must agree about lid thickness, so they read ONE derivation.
        assert_eq!(
            delta.depth_km(),
            crate::laws::thermal_boundary_layer(Fixed::from_int(2890), Fixed::from_int(100_000)),
            "the lid base IS the banked boundary layer, to the bit, rather than a reimplementation of it"
        );
        // THE LID THINS AS THE FLOW QUICKENS, which is the scaling's whole content and is what makes the
        // Rayleigh number load-bearing here rather than decorative.
        let vigorous =
            ConductiveLidBase::from_rayleigh(Fixed::from_int(2890), Fixed::from_int(1_000_000))
                .expect("a vigorous layer has a thinner one");
        assert!(
            vigorous.depth_km() < delta.depth_km(),
            "a more vigorous mantle shears over a thinner lid: {} against {}",
            f64_of(vigorous.depth_km()),
            f64_of(delta.depth_km())
        );
        // A LAYER THAT DOES NOT CONVECT IS ITS OWN LID, which is the law's documented convention and is the
        // physics: no convection, no conductive-convective boundary, so the whole layer conducts.
        assert_eq!(
            ConductiveLidBase::from_rayleigh(Fixed::from_int(30), ZERO)
                .expect("a still layer is its own lid")
                .depth_km(),
            Fixed::from_int(30)
        );
        // AND THERE IS NO LID WITHOUT A LAYER: refuse rather than report a plate that is not there.
        assert!(ConductiveLidBase::from_rayleigh(ZERO, Fixed::from_int(100_000)).is_none());
        assert!(
            ConductiveLidBase::from_rayleigh(Fixed::from_int(-1), Fixed::from_int(100)).is_none()
        );
    }

    #[test]
    fn the_shallow_envelope_is_invariant_across_the_v_star_bracket() {
        // THE ASSERTION THAT MAKES THE UNCONSTRAINED BRACKET SAFE, put on trial rather than assumed. The banked
        // `V*` chords start at 0.3 GPa, about nine kilometres down, so a lid sampled FROM THE SURFACE is outside
        // every chord through its whole brittle top and is served the TABLE'S OWN EXTREMES. That span is enormous
        // (-2 to 27 cm^3/mol) and it costs nothing there, because `P V*` tops out near 8 kJ/mol at 0.3 GPa
        // against `E*`'s 530: the cold shallow rock is not creeping at a geological rate at EITHER end, the
        // brittle branch floors the envelope both ways, and the minimum is identical to the bit.
        let volumes = table2_span_fixture();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let geotherm = ramp_geotherm;
        let env = earth_like_lid(&creep, &geotherm, Fixed::from_int(100_000));

        // THE PREMISE FIRST, or the invariance below would be a fact about a degenerate bracket. At 5 km the
        // pressure is about 0.16 GPa, under every chord's 0.3 GPa floor, so the span is the table's own and it is
        // 29 cm^3/mol wide.
        let shallow = Fixed::from_int(5);
        let p_gpa = f64_of(env.vertical_stress_mpa(shallow).expect("lithostatic")) / 1000.0;
        assert!(
            p_gpa < 0.3,
            "5 km sits under the banked chords' 0.3 GPa floor: {p_gpa} GPa"
        );
        let bracket =
            select_activation_volume(&volumes, Fixed::from_ratio((p_gpa * 1e6) as i64, 1_000_000))
                .expect("the table supports its own extremes");
        assert_eq!(
            bracket.constraint(),
            VolumeConstraint::UnconstrainedBySource,
            "no chord reaches 5 km, so the source constrains nothing there"
        );
        assert!(
            !bracket.is_degenerate()
                && f64_of(bracket.at(VolumeEnd::High)) - f64_of(bracket.at(VolumeEnd::Low)) > 28.0,
            "the span really is the table's whole width, or this test proves nothing: [{}, {}]",
            f64_of(bracket.at(VolumeEnd::Low)),
            f64_of(bracket.at(VolumeEnd::High))
        );

        // THE CLAIM. Both ends agree, so the envelope reports a number, and the agreement is the PROOF that the
        // span could not have moved it.
        for z in [0, 1, 3, 5, 8] {
            let z = Fixed::from_int(z);
            for sense in [FaultingSense::Thrust, FaultingSense::Normal] {
                assert_eq!(
                    env.ductile(z, VolumeEnd::Low),
                    DuctileReading::AboveRepresentable,
                    "the shallow column is not creeping at a geological rate at the low end either"
                );
                assert_eq!(
                    env.ductile(z, VolumeEnd::High),
                    DuctileReading::AboveRepresentable
                );
                // WHICH BRANCH WINS is what the bracket cannot change: the envelope IS the brittle branch here.
                let envelope = env
                    .yield_in_sense(z, sense)
                    .expect("both ends agree, so the envelope reports a strength");
                let brittle = match env.brittle(z, sense).expect("rock determines") {
                    DifferentialStrength::Determined(b) => b,
                    other => panic!("rock determines everywhere, got {other:?}"),
                };
                assert_eq!(
                    envelope,
                    brittle
                        .checked_div(Fixed::from_int(MPA_PER_GPA))
                        .expect("to GPa"),
                    "the brittle branch floors the shallow envelope at both ends of the span"
                );
            }
        }

        // AND THE INVARIANCE IS NOT A PROPERTY OF THE CODE ALWAYS SAYING YES, which is the half that makes the
        // check above evidence. DEEP in the same column the ductile branch binds, the same span reaches the
        // answer, the two ends disagree by a factor of several, and the envelope REFUSES rather than choosing an
        // end. The shallow invariance is a fact about the shallow column, and here is the proof it can fail.
        let deep = Fixed::from_int(60);
        let low = match env.ductile(deep, VolumeEnd::Low) {
            DuctileReading::Determined(d) => f64_of(d),
            other => panic!("a hot deep lid creeps at a determinate strength, got {other:?}"),
        };
        let high = match env.ductile(deep, VolumeEnd::High) {
            DuctileReading::Determined(d) => f64_of(d),
            other => panic!("a hot deep lid creeps at a determinate strength, got {other:?}"),
        };
        assert!(
            high > low * 2.0,
            "deep, the span is worth a factor of several in strength: {low} against {high}"
        );
        assert!(
            env.compressive_yield_gpa(deep).is_none(),
            "where the span reaches the answer there is no single strength to report, got {:?}",
            env.compressive_yield_gpa(deep)
        );
    }

    #[test]
    fn the_lid_referee_checks_the_derived_base_against_the_convective_stress_scale() {
        // THE CROSS-CHECK, and why it is evidence rather than a restatement. `ConductiveLidBase` derives delta
        // from the THERMAL structure alone (`d / Ra^(1/3)`: buoyancy, viscosity, diffusivity, depth) and nothing
        // in that expression knows what creep is. This asks the MECHANICAL question at that depth, through the
        // creep rows at the LOAD's own rate, against the stress scale lid mobilization already emerges from.
        //
        // THIS TEST PROVES THE COMPARATOR AND NOT THE PHYSICS, and the division of labour is deliberate: the
        // strength's own arithmetic is refereed against H&K's printed worked examples, which are back-solved from
        // nothing. Here the pivot is the MEASURED strength at the lid base, and the verdict must flip around it,
        // which is what a live check does and a constant cannot.
        let volumes = [table2_volume_fixture()];
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let geotherm = ramp_geotherm;
        let env = earth_like_lid(&creep, &geotherm, Fixed::from_int(100_000));

        // The measured strength at the derived base, which is what the referee reports rather than summarizes.
        let probe =
            referee_conductive_lid_base(&env, Fixed::ONE).expect("the ductile branch answers");
        let at_base = match probe.strength_low {
            DuctileReading::Determined(s) => f64_of(s),
            other => panic!("the lid base creeps at a determinate strength, got {other:?}"),
        };
        assert!(
            at_base > 0.0,
            "the strength at the base is a real number to pivot on: {at_base} MPa"
        );
        assert_eq!(probe.lid_base_km, env.lid_base.depth_km());

        // BELOW the strength, the lid has NOT reached the convective stress scale: a finding, reported.
        let refuted = referee_conductive_lid_base(
            &env,
            Fixed::from_ratio((at_base * 0.5 * 1e6) as i64, 1_000_000),
        )
        .expect("answers");
        assert_eq!(
            refuted.verdict,
            LidVerdict::StrengthExceedsConvectiveStress,
            "a convective stress the lid's own strength exceeds does not confirm the base"
        );
        // ABOVE it, the strength HAS fallen to the scale and the two derivations agree.
        let confirmed = referee_conductive_lid_base(
            &env,
            Fixed::from_ratio((at_base * 2.0 * 1e6) as i64, 1_000_000),
        )
        .expect("answers");
        assert_eq!(
            confirmed.verdict,
            LidVerdict::Confirmed,
            "a convective stress above the lid's strength at its base confirms it"
        );

        // AND THE SPAN CAN STRADDLE, which is the third answer and is honest rather than a fudge: where the V*
        // bracket's two ends land on opposite sides of the stress, the source's own scatter does not settle the
        // question, and saying so beats collapsing to whichever end was read first.
        let wide = table2_span_fixture();
        let creep_wide = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &wide,
        }];
        let env_wide = earth_like_lid(&creep_wide, &geotherm, Fixed::from_int(100_000));
        let straddle = referee_conductive_lid_base(&env_wide, Fixed::ONE).expect("answers");
        let (lo, hi) = match (straddle.strength_low, straddle.strength_high) {
            (DuctileReading::Determined(l), DuctileReading::Determined(h)) => {
                (f64_of(l), f64_of(h))
            }
            other => panic!("both ends determine at the base, got {other:?}"),
        };
        assert!(
            hi > lo,
            "the span is ordered at a positive pressure: {lo}, {hi}"
        );
        let between = Fixed::from_ratio(((lo + hi) / 2.0 * 1e6) as i64, 1_000_000);
        assert_eq!(
            referee_conductive_lid_base(&env_wide, between)
                .expect("answers")
                .verdict,
            LidVerdict::BracketStraddlesConvectiveStress,
            "a stress between the span's two ends leaves the question open, and the referee says so"
        );

        // THE CASE THE REFEREE EXISTS FOR, end to end, and the one a `Determined` fixture never reaches. A
        // VIGOROUS mantle (Ra = 1e9) puts the derived lid base at `2890 / 1000` = 2.9 km, where the rock is
        // about 311 K and IS NOT CREEPING AT A GEOLOGICAL RATE AT ALL: the flow law runs its strength past what
        // the type can hold. No convective stress, however large, confirms a lid base whose rock creep is
        // irrelevant, so the thermal derivation and the mechanical boundary DISAGREE and the referee reports it
        // rather than absorbing it. That disagreement is a fact about this fixture's inconsistent Rayleigh
        // number and geotherm, which is exactly the class of thing the cross-check is here to surface.
        let thin = earth_like_lid(&creep, &geotherm, Fixed::from_int(1_000_000_000));
        assert!(
            f64_of(thin.lid_base.depth_km()) < 3.0,
            "Ra = 1e9 shears over a very thin lid: {} km",
            f64_of(thin.lid_base.depth_km())
        );
        let vigorous = referee_conductive_lid_base(&thin, Fixed::from_int(1000)).expect("answers");
        assert_eq!(vigorous.strength_low, DuctileReading::AboveRepresentable);
        assert_eq!(
            vigorous.verdict,
            LidVerdict::StrengthExceedsConvectiveStress,
            "rock that is not creeping at all has not fallen to any stress scale, however large"
        );
    }

    #[test]
    fn the_full_column_solves_from_the_surface_to_the_derived_lid_base() {
        // THE TWO FINDINGS, CLOSED, IN ONE READING. This is the thing that could not be done before, and each
        // half was blocked by a different one:
        //
        // - THE SURFACE END was blocked by the `V*` chords, which start at 0.3 GPa (about nine kilometres down).
        //   The selection refused above that, so the composite had no admitted row, so the envelope refused, so
        //   the profile could not be sampled AT ALL from the surface. The bracket retires that: outside every
        //   chord the table's own extremes are reported, tagged unconstrained.
        // - THE DEEP END was blocked by the integral having no domain but a declared one, since the integrand
        //   does not die. It is now the derived conductive-lid base, below which the mantle overturns and a
        //   static load's stresses are not sustained.
        let volumes = [table2_volume_fixture()];
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let geotherm = ramp_geotherm;
        let env = earth_like_lid(&creep, &geotherm, Fixed::from_int(100_000));
        let delta = env.lid_base.depth_km();

        // THE LID IS A SKIN ON THE LAYER RATHER THAN THE LAYER, which is what the Rayleigh number buys: a
        // convecting mantle carries heat through its interior efficiently, so the temperature drop concentrates
        // into a thin conductive top. Asserted here because without it this test sits through a "lid" that
        // silently swallowed the whole 2890 km column, which is a mutation run's finding rather than a worry.
        assert!(
            delta < Fixed::from_int(2890),
            "a convecting layer's conductive lid is thinner than the layer: {} km of 2890",
            f64_of(delta)
        );

        // THE SURFACE IS OUTSIDE EVERY CHORD, which is the blocker made concrete rather than recalled.
        assert!(
            f64_of(env.vertical_stress_mpa(ZERO).expect("lithostatic")) / 1000.0 < 0.3,
            "the surface sits under the banked chords' 0.3 GPa floor, which is what used to refuse it"
        );
        assert!(
            env.compressive_yield_gpa(ZERO).is_some() && env.tensile_yield_gpa(ZERO).is_some(),
            "the envelope answers AT THE SURFACE, which is the whole of what B7 unblocks"
        );

        // AND THE WHOLE COLUMN SAMPLES, surface to derived lid base, with no declared depth anywhere in it.
        let steps = 600;
        let profile = EnvelopeProfile::sample(&env, steps).expect("the full column samples");
        // THE GRID REALIZES THE DOMAIN RATHER THAN REPRODUCING IT, and the gap has a derived bound rather than a
        // chosen one. `step = trunc(delta / steps)` truncates, losing under one ULP, and the deepest node is
        // `step * steps`, so the node sits at or below `delta` by at most ONE ULP PER STEP: `steps *
        // Fixed::EPSILON`, read off the representation. Asserting equality here would be asserting that
        // fixed-point division is exact, and asserting a hand-picked epsilon would be a tolerance someone chose.
        //
        // THIS ASSERTION'S OWN BLINDNESS, stated because a mutation run measured it: widening the bound a
        // thousandfold does not fail this test, since the realized gap (about 388 ULP against the bound's 600)
        // sits well inside either. A test that a value lies within a bound cannot tell a derived bound from a
        // looser authored one, BY CONSTRUCTION. What guards that is the derivation being written here where a
        // reader checks it, which is how this project caught an authored `+2` hiding inside a "derived" bound:
        // by reading it, never by a test going red.
        let quantization = Fixed::EPSILON * Fixed::from_int(steps as i32);
        let realized = profile.domain_max_depth_km();
        assert!(
            realized <= delta && delta - realized <= quantization,
            "the profile's deepest node is the derived lid base as its own grid realizes it: {} against {}",
            f64_of(realized),
            f64_of(delta)
        );
        // The envelope turns over inside the column, which is what makes this a real yield envelope rather than
        // one branch: brittle rising with rho g z, then creep taking over as the geotherm climbs.
        let at = |z: i32| {
            f64_of(
                env.compressive_yield_gpa(Fixed::from_int(z))
                    .expect("envelope"),
            )
        };
        assert!(
            at(5) < at(30) && at(60) < at(30),
            "the envelope rises on the brittle limb and falls on the ductile one: {}, {}, {}",
            at(5),
            at(30),
            at(60)
        );

        // THE MOMENT INTEGRATES OVER IT.
        let k = mm_illustration_curvature();
        let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n");
        assert!(
            z_n > ZERO && z_n < delta,
            "the neutral surface is solved inside the derived lid: {} km of {}",
            f64_of(z_n),
            f64_of(delta)
        );
        let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).expect("M");
        let d = equivalent_rigidity(m.moment, k).expect("D");
        assert!(
            d > ZERO,
            "a bent lid has a positive rigidity: {}",
            f64_of(d)
        );

        // AND THE FLAG STAYS HONEST, which is the ruling's own condition. This envelope's integrand does NOT die
        // (a power-law row keeps a strength floor and the lever arm grows), so the integral runs to the domain
        // and says so. The domain is derived now rather than declared, and that is stated rather than absorbed:
        // the moment's second parent is the lid base, and `self_truncated = false` is how a reader learns it.
        assert!(
            !m.self_truncated,
            "the creep envelope's tail does not die, and the reading must not claim otherwise: {m:?}"
        );
        assert_eq!(m.truncation_depth_km, None);
        assert!(
            m.final_interval_contribution > Fixed::EPSILON,
            "the last interval at the lid base still contributes, which is the tail measured rather than assumed"
        );
    }

    #[test]
    fn a_refused_ductile_branch_refuses_the_envelope_rather_than_reading_as_no_strength() {
        // A REFUSAL IS NOT A ZERO, and the distance between them is the whole envelope. A ductile branch that
        // cannot answer (no admitted row, a geotherm that refuses, a pressure past the type) means the envelope
        // has NO ANSWER; reading it as zero strength would report a column that sustains nothing, which is a
        // confident wrong answer in the direction that looks like physics (a hot weak lid) rather than like a
        // bug. Found by a mutation run: this branch was reachable and no test drove it.
        let geotherm = ramp_geotherm;
        // An envelope with NO creep candidates: the composite refuses with `NoAdmittedRow`.
        let env = earth_like_lid(&[], &geotherm, Fixed::from_int(100_000));
        let z = Fixed::from_int(15);
        assert_eq!(
            env.ductile(z, VolumeEnd::Low),
            DuctileReading::Refused(CreepRefusal::NoAdmittedRow),
            "no admitted row, no ductile branch"
        );
        // THE BRITTLE BRANCH DETERMINES HERE, so the refusal below is the ductile branch's own and this test
        // cannot pass because the wrong gate fired.
        assert!(matches!(
            env.brittle(z, FaultingSense::Thrust),
            Some(DifferentialStrength::Determined(_))
        ));
        assert!(
            env.compressive_yield_gpa(z).is_none() && env.tensile_yield_gpa(z).is_none(),
            "a refused ductile branch refuses the envelope, in both senses"
        );
        // AND THE REFUSAL PROPAGATES rather than being interpolated across: a profile cannot be sampled over a
        // column the envelope cannot describe.
        assert!(
            EnvelopeProfile::sample(&env, 100).is_none(),
            "a profile over an envelope that refuses at a node is refused, never patched"
        );
    }

    #[test]
    fn the_lid_referees_two_representability_edges_carry_their_own_physical_claim() {
        // THE TWO ARMS A FIXTURE AT AN ORDINARY DEPTH NEVER REACHES, which is how a mutation run found them:
        // both survived as an inline closure because every fixture's lid base returned a `Determined` strength.
        // Each arm is a real claim about the flow law's own documented edges, so each is asserted rather than
        // left to a fixture that happens to land on it.
        let stress = Fixed::from_int(5);
        // A strength past `Fixed::MAX` MPa is the flow law saying creep is IRRELEVANT here. It is astronomically
        // above any convective stress, so it has not fallen, and no stress can make it so.
        assert_eq!(
            strength_has_fallen_to(DuctileReading::AboveRepresentable, stress),
            Some(false)
        );
        assert_eq!(
            strength_has_fallen_to(DuctileReading::AboveRepresentable, Fixed::MAX),
            Some(false),
            "not even the largest representable stress confirms rock that is not creeping"
        );
        // A strength below `Fixed::EPSILON` MPa is under any positive stress, so it has fallen.
        assert_eq!(
            strength_has_fallen_to(DuctileReading::BelowRepresentable, stress),
            Some(true)
        );
        // The ordinary arm, on both sides of its own boundary, which is where the comparison lives.
        assert_eq!(
            strength_has_fallen_to(DuctileReading::Determined(Fixed::from_int(4)), stress),
            Some(true)
        );
        assert_eq!(
            strength_has_fallen_to(DuctileReading::Determined(Fixed::from_int(6)), stress),
            Some(false)
        );
        assert_eq!(
            strength_has_fallen_to(DuctileReading::Determined(stress), stress),
            Some(true),
            "'has fallen TO the scale' includes reaching it"
        );
        // A refusal answers nothing rather than guessing a verdict.
        assert_eq!(
            strength_has_fallen_to(DuctileReading::Refused(CreepRefusal::NoAdmittedRow), stress),
            None
        );
    }

    #[test]
    fn the_brittle_branch_keys_on_the_materials_own_row_and_ice_brackets_in_its_own_gap() {
        // THE ALIEN CASE, and it is the landed row's own gap structure surviving the resolution onto faults.
        // Rock is a CROSSOVER law: its two branches meet at 200 MPa, so between them there is nothing, and the
        // resolution determines at every state. Ice's branches DO NOT meet (Beeman licenses the low fit at or
        // below 5 MPa and the high fit at or above 10), and the gap survives: there are states where NEITHER
        // fit's tangency lands in its own domain, and there the resolution reports the band the two fits span
        // and chooses nothing.
        let rock = rock_friction_law();
        let ice = ice_friction_law();

        // Rock never brackets, in either faulting sense, across the lid's whole stress range.
        for sv in [0, 25, 50, 100, 150, 200, 400, 800, 1600] {
            for sense in [FaultingSense::Thrust, FaultingSense::Normal] {
                assert!(
                    matches!(
                        brittle_differential_mpa(&rock, Fixed::from_int(sv), sense).unwrap(),
                        DifferentialStrength::Determined(_)
                    ),
                    "rock's crossover law determines everywhere, got a bracket at {sv} MPa, {sense:?}"
                );
            }
        }
        // Ice brackets inside its own gap. In the normal-faulting sense the two fits' tangencies straddle the
        // 5-to-10 MPa gap over a real window of overburden, and neither is licensed there.
        match brittle_differential_mpa(&ice, Fixed::from_int(15), FaultingSense::Normal).unwrap() {
            DifferentialStrength::Bracket { low, high } => {
                assert!(high > low, "the bracket spans the two fits, ordered");
                assert!(
                    f64_of(high) - f64_of(low) > 1.0,
                    "the fits disagree by megapascals inside the gap, which is why no point is chosen: [{}, {}]",
                    f64_of(low),
                    f64_of(high)
                );
            }
            other => panic!(
                "ice at 15 MPa overburden sits in its own gap and must bracket, got {other:?}"
            ),
        }
        // AND THE ENVELOPE REFUSES THERE rather than picking a point: an ice shell has a depth band where its
        // own friction calibration says nothing, and the construction says so instead of inventing a strength.
        let volumes = [table2_volume_fixture()];
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let geotherm = |_z: Fixed| Some(Fixed::from_int(100));
        // A CONDUCTIVE (non-convecting) shell, which is the law's own documented convention doing real work: with
        // no convection there is no conductive-convective boundary, so the lid is THE WHOLE SHELL and the derived
        // base reads back its full 30 km. A shell that convects would read a thin skin instead, and neither case
        // is a number anyone here declares.
        let lid_base = ConductiveLidBase::from_rayleigh(Fixed::from_int(30), ZERO)
            .expect("a non-convecting layer is its own lid");
        assert_eq!(
            lid_base.depth_km(),
            Fixed::from_int(30),
            "a layer that does not convect conducts throughout, so its lid is all of it"
        );
        let shell = LithosphereEnvelope {
            friction: ice_friction_law(),
            density_kg_m3: Fixed::from_int(920),
            gravity_m_s2: Fixed::from_ratio(131, 100), // Europa-class
            geotherm_k: &geotherm,
            creep: &creep,
            chord: test_chord(),
            lid_base,
        };
        // 15 MPa of overburden on a Europa-class shell sits near 12.4 km down.
        assert!(
            shell.tensile_yield_gpa(Fixed::from_int(12)).is_none(),
            "inside its friction row's gap the shell's envelope refuses rather than choosing a point"
        );
        // Ice is weak on BOTH branches, the source's own headline: at the same overburden it yields below rock.
        let ice_deep =
            brittle_differential_mpa(&ice, Fixed::from_int(300), FaultingSense::Thrust).unwrap();
        let rock_deep =
            brittle_differential_mpa(&rock, Fixed::from_int(300), FaultingSense::Thrust).unwrap();
        match (ice_deep, rock_deep) {
            (DifferentialStrength::Determined(i), DifferentialStrength::Determined(r)) => {
                assert!(
                    i < r,
                    "ice yields below rock at the same overburden: {} against {}",
                    f64_of(i),
                    f64_of(r)
                )
            }
            other => panic!("both determine at 300 MPa, got {other:?}"),
        }
    }

    #[test]
    fn first_yielding_takes_the_smallest_circle_where_both_branches_are_licensed() {
        // THE DERIVED SELECTION RULE, on the one window where it can be seen. A friction law states its branches
        // on the FAULT-NORMAL stress, and the tangency that produces that stress depends on the very `mu` the
        // branch selects, so both of Byerlee's branches turn out to be self-consistent over a narrow window of
        // overburden (about 104 to 121 MPa in thrust). Inside it the operative branch is the one that yields
        // FIRST, which is the SMALLEST Mohr circle. Nothing is chosen: the minimum is what first yielding means.
        //
        // The window has two halves and only the far half can convict, which is why this test sits at 120 MPa.
        // Below about 115 the low branch is already the smaller one, so a rule that simply always took the low
        // branch would agree with the minimum and no test could tell them apart. At 120 the HIGH branch is the
        // smaller circle while BOTH are licensed, so taking the minimum and taking the low branch part company.
        let rock = rock_friction_law();
        let sv = Fixed::from_int(120);
        let (low_d, low_n) = mohr_coulomb_differential_mpa(
            rock.low_coefficient,
            rock.low_cohesion,
            sv,
            FaultingSense::Thrust,
        )
        .expect("the low branch resolves");
        let (high_d, high_n) = mohr_coulomb_differential_mpa(
            rock.high_coefficient,
            rock.high_cohesion,
            sv,
            FaultingSense::Thrust,
        )
        .expect("the high branch resolves");
        // The premise the test rests on: both tangencies really are inside their own stated domains here.
        assert!(
            low_n < rock.low_domain_max,
            "the low branch is licensed at 120 MPa: its tangency sits at {} MPa",
            f64_of(low_n)
        );
        assert!(
            high_n >= rock.high_domain_min,
            "the high branch is licensed at 120 MPa: its tangency sits at {} MPa",
            f64_of(high_n)
        );
        // And the high branch is the SMALLER circle here, which is what makes the minimum load-bearing.
        assert!(
            high_d < low_d,
            "at 120 MPa the high branch yields first: {} against {}",
            f64_of(high_d),
            f64_of(low_d)
        );
        match brittle_differential_mpa(&rock, sv, FaultingSense::Thrust).unwrap() {
            DifferentialStrength::Determined(d) => {
                assert_eq!(
                    d, high_d,
                    "first yielding takes the smallest circle, not the low branch"
                );
                assert!(
                    f64_of(low_d) - f64_of(d) > 5.0,
                    "and the two really differ, so the rule is testable: {} against {}",
                    f64_of(d),
                    f64_of(low_d)
                );
            }
            other => panic!("rock determines everywhere, got {other:?}"),
        }
    }

    #[test]
    fn the_tail_bound_is_not_fooled_by_an_envelope_that_revives() {
        // THE RIGOUR CLAIM THE TAIL BOUND MAKES, put on trial. The bound is the envelope's SUFFIX MAXIMUM (the
        // greatest strength at or below a node), not the strength AT that node, and the difference only shows on
        // an envelope that DIES AND COMES BACK: a strong layer, a weak layer, then a strong layer again. That is
        // the decoupled-lid geometry, which this module declares it does not model, but a tail bound that
        // truncated inside the weak layer would silently DELETE the strong layer beneath it and report a
        // confident, wrong, too-small moment.
        //
        // A local maximum would stop in the dead layer. The suffix maximum sees the revival and does not.
        struct RevivingEnvelope;
        impl RevivingEnvelope {
            fn strength(z: Fixed) -> Fixed {
                // Strong to 20 km, dead from 20 to 40, strong again from 40 to 60.
                if z <= Fixed::from_int(20) || z > Fixed::from_int(40) {
                    Fixed::from_ratio(1, 2)
                } else {
                    ZERO
                }
            }
        }
        impl YieldEnvelope for RevivingEnvelope {
            fn tensile_yield_gpa(&self, z: Fixed) -> Option<Fixed> {
                Some(Self::strength(z))
            }
            fn compressive_yield_gpa(&self, z: Fixed) -> Option<Fixed> {
                Some(Self::strength(z))
            }
            fn domain_max_depth_km(&self) -> Fixed {
                Fixed::from_int(60)
            }
        }
        let profile = EnvelopeProfile::sample(&RevivingEnvelope, 6000).expect("profile");
        let k = mm_illustration_curvature();
        let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).expect("z_n");
        let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).expect("M");
        assert!(
            !m.self_truncated,
            "an envelope alive at its domain edge must not truncate in the dead layer above it: {m:?}"
        );
        // AND THE DEEP LAYER REALLY WAS INTEGRATED: the same column with the deep layer removed carries a
        // materially smaller moment, so a bound that stopped in the weak layer would have reported that instead.
        struct TopLayerOnly;
        impl YieldEnvelope for TopLayerOnly {
            fn tensile_yield_gpa(&self, z: Fixed) -> Option<Fixed> {
                if z <= Fixed::from_int(20) {
                    Some(Fixed::from_ratio(1, 2))
                } else {
                    Some(ZERO)
                }
            }
            fn compressive_yield_gpa(&self, z: Fixed) -> Option<Fixed> {
                self.tensile_yield_gpa(z)
            }
            fn domain_max_depth_km(&self) -> Fixed {
                Fixed::from_int(60)
            }
        }
        let top = EnvelopeProfile::sample(&TopLayerOnly, 6000).expect("profile");
        let z_n_top = neutral_surface_depth_km(&top, k, lit_e(), lit_nu()).expect("z_n");
        let m_top = bending_moment(&top, k, z_n_top, lit_e(), lit_nu()).expect("M");
        assert!(
            f64_of(m.moment).abs() > f64_of(m_top.moment).abs() * 1.5,
            "the reviving layer contributes a real moment: {} against the top layer alone {}",
            f64_of(m.moment),
            f64_of(m_top.moment)
        );
        // The top-layer-only column DOES self-truncate, which is the control: its envelope never revives, so the
        // suffix maximum below 20 km is zero and the bound collapses to nothing, which is the honest answer.
        assert!(
            m_top.self_truncated,
            "a column whose envelope never revives self-truncates: {m_top:?}"
        );
    }

    #[test]
    fn the_line_load_fixed_point_converges_and_carries_its_chord() {
        // THE LOOP, end to end, on the primary's own fixture. The load supplies its own curvature through the
        // solve, so no reference bending is chosen anywhere: the only inputs are the envelope, the moduli, the
        // restoring term, and the load.
        let profile = mm_illustration_profile(Fixed::from_ratio(1, 2));
        let chord = test_chord();
        // THE LOAD IS SIZED TO BEND THE PLATE INTO ITS OWN YIELDING, which is the regime the construction exists
        // for. A gentle load converges to the fully elastic ceiling, which is correct and is asserted separately
        // below, but it exercises none of the elastic-plastic machinery: at the curvatures a small load makes
        // (below about 1e-8 per metre) Watts and Burov report the yielded and elastic thicknesses equal.
        let load = Fixed::from_int(80);
        let plate = solve_line_load(
            &profile,
            lit_e(),
            lit_nu(),
            Fixed::from_ratio(33, 10),    // delta_rho, in 1000 kg/m^3
            Fixed::from_ratio(98, 10000), // g, in km/s^2
            load,                         // a line load, in GPa km
            chord,
        )
        .expect("the fixed point converges");

        // THE FIXED POINT IS A FIXED POINT: re-entering the converged rigidity reproduces it, which is the
        // property the loop claims rather than merely the number it stopped at.
        let alpha = crate::flexure::flexural_parameter(
            plate.rigidity_gpa_km3,
            Fixed::from_ratio(33, 10),
            Fixed::from_ratio(98, 10000),
        )
        .unwrap();
        let k = line_load_curvature_at_first_zero_crossing(load, alpha, plate.rigidity_gpa_km3)
            .unwrap();
        let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).unwrap();
        let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).unwrap();
        let d_again = equivalent_rigidity(m.moment, k).unwrap();
        assert!(
            (f64_of(d_again) - f64_of(plate.rigidity_gpa_km3)).abs()
                < f64_of(plate.rigidity_gpa_km3) * 1e-6,
            "re-entering the converged rigidity reproduces it: {} against {}",
            f64_of(d_again),
            f64_of(plate.rigidity_gpa_km3)
        );

        // The curvature it settled at is concave-down (a downward load read at its first zero crossing), and the
        // rigidity is positive and inside the envelope's own elastic ceiling.
        assert!(
            plate.curvature.upward_per_km() < ZERO,
            "the converged curvature is concave-down"
        );
        let elastic_ceiling =
            crate::flexure::flexural_rigidity(lit_e(), lit_nu(), Fixed::from_int(40)).unwrap();
        assert!(
            plate.rigidity_gpa_km3 > ZERO,
            "a converged plate has a positive rigidity, got {}",
            f64_of(plate.rigidity_gpa_km3)
        );
        assert!(
            f64_of(plate.rigidity_gpa_km3) < f64_of(elastic_ceiling) * 0.9,
            "this load really does yield the plate: {} against the unyielded ceiling {}",
            f64_of(plate.rigidity_gpa_km3),
            f64_of(elastic_ceiling)
        );

        // A GENTLE LOAD LANDS AT THE ELASTIC CEILING, which is the same statement read from the other end: a
        // plate bent too little to yield anywhere is its own uniform elastic plate. The tolerance is the
        // trapezoid's own residual (a few parts in 1e7 on a convex integrand), not slack.
        let gentle = solve_line_load(
            &profile,
            lit_e(),
            lit_nu(),
            Fixed::from_ratio(33, 10),
            Fixed::from_ratio(98, 10000),
            Fixed::from_ratio(1, 100),
            chord,
        )
        .expect("a gentle load converges");
        assert!(
            (f64_of(gentle.rigidity_gpa_km3) - f64_of(elastic_ceiling)).abs()
                < f64_of(elastic_ceiling) * 1e-4,
            "a load too gentle to yield reads the unyielded column: {} against {}",
            f64_of(gentle.rigidity_gpa_km3),
            f64_of(elastic_ceiling)
        );

        // THE CHORD RIDES WITH THE ANSWER. A rigidity or a thickness without its load class and load timescale
        // is a statistic with a hidden conditioning variable, so the output carries both.
        assert_eq!(plate.chord.class, LoadClassId(1));
        assert_eq!(plate.chord.timescale_s, Fixed::from_int(31_557_600));
        assert_eq!(plate.chord.ln_strain_rate_per_s, ln_scientific(1, 1, -15));

        // The display statistic is available at a DECLARED pair and refuses nothing here.
        let te = f64_of(plate.elastic_thickness_km(lit_e(), lit_nu()).expect("T_e"));
        assert!(
            te > 0.0 && te <= 40.0,
            "the display thickness is inside the column it was integrated over, got {te}"
        );
    }

    #[test]
    fn the_display_thickness_carries_its_modulus_pair_which_is_the_whole_point() {
        // THE CHORD-STRIKE THE RULING NAMES: T_e is proportional to (1/E)^(1/3), so the SAME rigidity read at a
        // different modulus pair is a different thickness. This is why the rigidity is canonical and the
        // thickness takes its moduli as explicit arguments: a T_e quoted without its pair is a chord with its
        // endpoints dropped, and the literature's pair (80 GPa, 0.25) is ASSUMED rather than measured.
        let d = Fixed::from_int(354_224); // the primary illustration's own rigidity
        let at_literature = f64_of(elastic_thickness_km(d, Fixed::from_int(80), lit_nu()).unwrap());
        let at_stiffer = f64_of(elastic_thickness_km(d, Fixed::from_int(160), lit_nu()).unwrap());
        // Doubling E must divide the reported thickness by 2^(1/3) = 1.26, at the same physical plate.
        let ratio = at_literature / at_stiffer;
        assert!(
            (ratio - 2.0_f64.powf(1.0 / 3.0)).abs() < 0.01,
            "T_e scales as (1/E)^(1/3): the ratio is {ratio} against 2^(1/3)"
        );
        // The size of the trap, stated: a world whose derived modulus is twice the literature's assumed 80 GPa
        // reports a thickness 21 percent smaller for the very same plate. Compared against a published T_e as
        // though the two were alike, that is a 21 percent error with no symptom.
        assert!(
            (at_literature - at_stiffer) / at_literature > 0.2,
            "the modulus mismatch is a fifth of the answer: {at_literature} against {at_stiffer}"
        );
    }

    #[test]
    fn the_point_load_curvature_twins_and_reproduces_the_printed_moment() {
        // THE NUMERICAL TWIN for the axisymmetric driving curvature, independent by construction: the analytic
        // form here reads `ker` and `kei'` from the fixed-point series, while the twin builds the point-load
        // deflection `w(r) = -(P l^2 / 2 pi D) kei(r/l)` from an f64 evaluation of kei's OWN A&S series and takes
        // finite differences of it. They share no arithmetic, so agreement convicts a sign slip, a lost factor,
        // or a wrong bracket. The deflection is differenced in f64 rather than through the fixed-point kelvin_kei
        // because a second difference divides by h^2 and would amplify the representation's quantization.
        let gamma = 0.5772156649_f64;
        let kei_f64 = |x: f64| -> f64 {
            let xh = x / 2.0;
            let xh4 = xh.powi(4);
            let (mut b, mut c) = (1.0_f64, xh * xh);
            let (mut ber, mut bei) = (b, c);
            let mut h = 1.0_f64;
            let mut bei_phi = c * h;
            for k in 1..40 {
                let mb = ((2 * k) * (2 * k - 1)) as f64;
                b = -b * xh4 / (mb * mb);
                ber += b;
                let mc = ((2 * k + 1) * (2 * k)) as f64;
                c = -c * xh4 / (mc * mc);
                bei += c;
                h += 1.0 / (2 * k) as f64 + 1.0 / (2 * k + 1) as f64;
                bei_phi += c * h;
            }
            -(xh.ln() + gamma) * bei - std::f64::consts::FRAC_PI_4 * ber + bei_phi
        };
        // The curvature is length-scale independent, so l = 1 loses nothing; P and D are the shared inputs.
        let (p, d, l, nu) = (1000.0_f64, 400_000.0_f64, 1.0_f64, 0.25_f64);
        let w = |r: f64| -(p * l * l / (2.0 * std::f64::consts::PI * d)) * kei_f64(r / l);
        let x0 = 3.91467_f64;
        let r0 = x0 * l;
        let hh = 1e-3_f64;
        // Central differences at the first zero crossing.
        let kappa_r = (w(r0 + hh) - 2.0 * w(r0) + w(r0 - hh)) / (hh * hh); // d2w/dr2
        let kappa_theta = ((w(r0 + hh) - w(r0 - hh)) / (2.0 * hh)) / r0; // (1/r) dw/dr
        let kappa_eff_twin = kappa_r + nu * kappa_theta; // the M operator, downward convention
        let laplacian_twin = kappa_r + kappa_theta; // the reported Laplacian K, downward convention

        // FIRST, the deflection really vanishes at the read location, which is what makes it the right x.
        assert!(
            w(r0).abs() < w(r0 + l).abs() * 0.02,
            "the point-load deflection vanishes at x0 = 3.91467: w = {} against {}",
            w(r0),
            w(r0 + l)
        );

        // The analytic driving curvature (my construction), converted back to the downward convention.
        let curv = point_load_curvature_at_first_zero_crossing(
            Fixed::from_int(1000),
            Fixed::from_int(400_000),
            lit_nu(),
        )
        .expect("curvature");
        let kappa_eff_analytic_down = -f64_of(curv.upward_per_km());
        assert!(
            (kappa_eff_analytic_down - kappa_eff_twin).abs() < kappa_eff_twin.abs() * 0.02,
            "analytic kappa_eff {kappa_eff_analytic_down} against the finite-difference twin {kappa_eff_twin}"
        );
        // THE SIGN, load-bearing: a downward point load bends the plate concave-down at its first zero crossing,
        // so the curvature is NEGATIVE in the upward convention, the same sign the line load carries.
        assert!(
            curv.upward_per_km() < ZERO,
            "a downward point load reads a negative (concave-down) curvature, got {:?}",
            curv.upward_per_km()
        );

        // THE CONTROL, which is what makes the twin an erratum check rather than a self-comparison:
        // M(x0)/P = bracket/(2 pi) must reproduce McNutt and Menard's printed A9 (-0.00704). M = -D kappa_eff.
        let moment_over_p = -d * kappa_eff_twin / p;
        assert!(
            (moment_over_p - (-0.007035)).abs() < 5e-5,
            "M(x0)/P reproduces the primary's printed A9 (-0.00704): got {moment_over_p}"
        );
        // AND THE ERRATUM: the Laplacian coefficient the twin recovers is ker(x0) = -0.0388994, NOT the paper's
        // printed -0.0289. laplacian = -(P/2 pi D) ker(x0), so ker(x0) = -laplacian * 2 pi D / P.
        let ker_x0_twin = -laplacian_twin * 2.0 * std::f64::consts::PI * d / p;
        assert!(
            (ker_x0_twin - (-0.0388994)).abs() < 5e-4,
            "the twin recovers the re-derived ker(x0) = -0.0388994, got {ker_x0_twin}"
        );
        assert!(
            (ker_x0_twin - (-0.0289)).abs() > 5e-3,
            "and it is decisively NOT the paper's printed -0.0289 (their 26 per cent erratum): {ker_x0_twin}"
        );
        // THE REPORTED (LAPLACIAN) CURVATURE the module exposes is the twin's Laplacian, and it DIFFERS from the
        // driving curvature above by the (1-nu) hoop term: the reported K reads ker(x0) alone, the driving one
        // reads the M operator. Exercising it here ties the exposed quantity to the same finite-difference twin.
        let reported = point_load_reported_curvature_at_first_zero_crossing(
            Fixed::from_int(1000),
            Fixed::from_int(400_000),
        )
        .expect("reported curvature");
        let reported_down = -f64_of(reported.upward_per_km());
        assert!(
            (reported_down - laplacian_twin).abs() < laplacian_twin.abs() * 0.02,
            "the reported Laplacian curvature matches the twin: {reported_down} against {laplacian_twin}"
        );
        assert!(
            (reported_down - kappa_eff_twin).abs() > kappa_eff_twin.abs() * 0.05,
            "and the reported curvature is a DIFFERENT quantity from the driving one: {reported_down} against {kappa_eff_twin}"
        );
    }

    #[test]
    fn the_axisymmetric_curvature_reproduces_the_primarys_printed_seamount_bracket() {
        // THE PRIMARY'S OWN PRINTED CONSTANT, reproduced from the construction. McNutt and Menard's A10 prints
        // the moment bracket [ker(x0) - (1-nu)/x0 kei'(x0)] = -0.04421 at nu = 0.25, giving M(x0)/P = -0.00704
        // (A9). The bracket is `-2 pi D kappa_eff / P`, and nu is baked into it through the (1-nu) factor, which
        // is the fetch's finding that the seamount constant is conditioned on nu.
        let bracket_at = |nu: Fixed| -> f64 {
            // Use P = 1, D = 1 so the bracket reads directly: kappa_eff = -(1/2 pi) bracket.
            let curv = point_load_curvature_at_first_zero_crossing(Fixed::ONE, Fixed::ONE, nu)
                .expect("curvature");
            let kappa_eff_down = -f64_of(curv.upward_per_km());
            -2.0 * std::f64::consts::PI * kappa_eff_down // bracket
        };
        let b25 = bracket_at(Fixed::from_ratio(1, 4));
        assert!(
            (b25 - (-0.04420)).abs() < 5e-4,
            "the bracket at nu = 0.25 reproduces the primary's A10 (-0.04421): got {b25}"
        );
        assert!(
            ((b25 / (2.0 * std::f64::consts::PI)) - (-0.007035)).abs() < 5e-5,
            "M(x0)/P at nu = 0.25 reproduces A9 (-0.00704): got {}",
            b25 / (2.0 * std::f64::consts::PI)
        );
        // THE nu SPREAD the fetch reports: M(x0)/P is -0.007316 at nu = 0, -0.006753 at nu = 0.5. Only nu = 0.25
        // reproduces A9, which is what proves nu sits inside the constant.
        let m0 = bracket_at(ZERO) / (2.0 * std::f64::consts::PI);
        let m5 = bracket_at(Fixed::from_ratio(1, 2)) / (2.0 * std::f64::consts::PI);
        assert!(
            (m0 - (-0.007316)).abs() < 5e-5,
            "M(x0)/P at nu = 0 is -0.007316: got {m0}"
        );
        assert!(
            (m5 - (-0.006753)).abs() < 5e-5,
            "M(x0)/P at nu = 0.5 is -0.006753: got {m5}"
        );
        assert!(
            m0 < b25 / (2.0 * std::f64::consts::PI) && b25 / (2.0 * std::f64::consts::PI) < m5,
            "the moment magnitude falls as nu rises, the (1-nu) dependence: {m0} then {m5}"
        );
    }

    #[test]
    fn the_erratum_carries_both_curvature_values_with_the_correction() {
        // THE RE-DERIVATION STANDARD, at the site. The re-derived Laplacian coefficient is ker(x0) = -0.0388994
        // (from the series), the paper prints -0.0289, and both are carried. The published value is about 26 per
        // cent low in magnitude and a correct one raises it by 34 per cent.
        let rederived = f64_of(point_load_reported_curvature_coefficient());
        let published = f64_of(mcnutt_menard_published_laplacian_coefficient());
        assert!(
            (rederived - (-0.0388994)).abs() < 5e-5,
            "the re-derived coefficient is ker(x0) = -0.0388994, got {rederived}"
        );
        assert!(
            (published - (-0.0289)).abs() < 1e-6,
            "the published coefficient is carried verbatim (-0.0289), got {published}"
        );
        // 26 per cent low: |published| / |rederived| ~ 0.743.
        let ratio = published.abs() / rederived.abs();
        assert!(
            (ratio - 0.743).abs() < 0.01,
            "the published value runs about 26 per cent low: ratio {ratio}"
        );
        // The correction factor a consumer of the paper's own C_2 applies: |rederived| / |published| ~ 1.346.
        let correction = rederived.abs() / published.abs();
        assert!(
            (correction - 1.346).abs() < 0.01,
            "a correct coefficient raises the paper's by 34 per cent: factor {correction}"
        );
    }

    #[test]
    fn the_point_load_fixed_point_recovers_the_elastic_limit_and_yields_under_load() {
        // THE AXISYMMETRIC LOOP, end to end, on the primary's own fixture. Two limits pin it.
        let chord = test_chord();
        // ELASTIC LIMIT: an envelope nothing reaches must return the fully elastic rigidity, so T_e = H = 40, and
        // the answer is independent of the load P (a plate that yields nowhere is its own uniform elastic plate).
        let strong = mm_illustration_profile(Fixed::from_int(100_000));
        let elastic = solve_point_load(&strong, lit_e(), lit_nu(), Fixed::from_int(100), chord)
            .expect("the elastic point-load solve converges");
        let te_elastic = f64_of(
            elastic
                .elastic_thickness_km(lit_e(), lit_nu())
                .expect("T_e"),
        );
        assert!(
            (te_elastic - 40.0).abs() < 0.05,
            "a point load too gentle to yield reads the plate's own thickness: T_e = {te_elastic}"
        );
        let elastic_big =
            solve_point_load(&strong, lit_e(), lit_nu(), Fixed::from_int(5000), chord)
                .expect("converges");
        // Independent of the load to quadrature precision: the elastic moment scales linearly with the
        // curvature, so `M / kappa_eff` is load-free up to the trapezoid's and the neutral-surface bisection's
        // own last-bit rounding (a few parts in 1e6), not a physical load dependence.
        assert!(
            (f64_of(elastic.rigidity_gpa_km3) - f64_of(elastic_big.rigidity_gpa_km3)).abs()
                < f64_of(elastic.rigidity_gpa_km3) * 1e-4,
            "the elastic limit is independent of the load magnitude: {} against {}",
            f64_of(elastic.rigidity_gpa_km3),
            f64_of(elastic_big.rigidity_gpa_km3)
        );

        // YIELDING: a large point load bends the 500 MPa / 40 km plate into its own yielding, so the rigidity
        // sits below the unyielded ceiling and the fixed point is a fixed point. The load is sized to yield the
        // plate while staying inside its support: a still larger load drives the fixed point toward a vanishing
        // rigidity and leaves the Q32.32 window (`NotRepresentable`) before the physical support limit is crossed,
        // the same representability ceiling `solve_line_load_banded` documents for its own edges.
        let profile = mm_illustration_profile(Fixed::from_ratio(1, 2));
        let load = Fixed::from_int(24_000);
        let plate = solve_point_load(&profile, lit_e(), lit_nu(), load, chord)
            .expect("the yielding point-load solve converges");
        let ceiling =
            crate::flexure::flexural_rigidity(lit_e(), lit_nu(), Fixed::from_int(40)).unwrap();
        assert!(
            f64_of(plate.rigidity_gpa_km3) < f64_of(ceiling) * 0.95,
            "this point load yields the plate: {} against the unyielded ceiling {}",
            f64_of(plate.rigidity_gpa_km3),
            f64_of(ceiling)
        );
        assert!(
            plate.curvature.upward_per_km() < ZERO,
            "the converged curvature is concave-down"
        );
        // The fixed point re-enters: recompute the curvature at the converged rigidity and re-solve; the rigidity
        // reproduces, which is the property the loop claims rather than the number it stopped at.
        let k = point_load_curvature_at_first_zero_crossing(load, plate.rigidity_gpa_km3, lit_nu())
            .unwrap();
        let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).unwrap();
        let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).unwrap();
        let d_again = equivalent_rigidity(m.moment, k).unwrap();
        assert!(
            (f64_of(d_again) - f64_of(plate.rigidity_gpa_km3)).abs()
                < f64_of(plate.rigidity_gpa_km3) * 1e-6,
            "re-entering the converged rigidity reproduces it: {} against {}",
            f64_of(d_again),
            f64_of(plate.rigidity_gpa_km3)
        );
        // The chord rides with the answer.
        assert_eq!(plate.chord.class, LoadClassId(1));
    }

    #[test]
    fn the_disc_point_band_widens_by_the_cited_two_percent() {
        // THE FORM'S OWN BAND, cited from the primary (C_c within 2 per cent of C_p, pp. 389-390). It widens the
        // axisymmetric rigidity by 2 per cent either way and is a SEPARATE band from the V* scatter.
        let d = Fixed::from_int(400_000);
        let band = disc_point_rigidity_band(d).expect("band");
        assert!(
            (f64_of(band.low()) - 392_000.0).abs() < 1.0
                && (f64_of(band.high()) - 408_000.0).abs() < 1.0,
            "the disc-point band is [0.98 D, 1.02 D]: [{}, {}]",
            f64_of(band.low()),
            f64_of(band.high())
        );
        // A non-plate refuses rather than reporting a band.
        assert!(disc_point_rigidity_band(ZERO).is_none());
    }

    #[test]
    fn the_uniaxial_cost_is_measured_at_the_hindcast_curvatures() {
        // THE OWNER-RULED SHORTCUT-VALIDITY MEASUREMENT. The uniaxial yield law is what BOTH primaries use, so
        // its cost is the DEPARTURE of the plate from elastic at the hindcast curvatures, measured on this
        // engine's OWN envelope rather than assumed. If the competent plate is effectively elastic there, the
        // biaxial (2-D yield surface) question is moot for the hindcast; if the ratio departs, it reopens with a
        // number attached. The number is reported either way.
        //
        // The measurement is Watts and Burov's ratio T_e(YSE)/T_e(elastic), read here as the moment the plate
        // carries over the moment a FULLY ELASTIC plate of the same competent thickness would carry at the same
        // curvature: R = M_yield / (D(T_mech) kappa). R ~ 1 means the caps do not bind (effectively elastic).
        let volumes = hk_dry_dislocation_activation_volumes();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let geotherm = ramp_geotherm;
        let env = earth_like_banded_lid(&creep, &geotherm);
        // Use the LOW V* edge as a definite surface (the competent layer is shallow, where the V* span cannot
        // move the answer, so the edge equals the settled view there).
        let edge = EnvelopeEdge::low(&env);
        let delta = env.lid_base.depth_km();

        // T_mech: the depth of maximum envelope strength, which is the brittle-ductile crossing (brittle rising
        // meets ductile falling). Scanned on the low edge in the tensile sense (the weaker branch, which yields
        // first and so bounds the competent behaviour).
        let mut t_mech = Fixed::ZERO;
        let mut peak = Fixed::ZERO;
        let mut z = Fixed::from_ratio(1, 2);
        while z < delta {
            if let Some(y) = edge.tensile_yield_gpa(z) {
                if y > peak {
                    peak = y;
                    t_mech = z;
                }
            }
            z = z.checked_add(Fixed::from_ratio(1, 2)).unwrap();
        }
        assert!(
            t_mech > Fixed::from_int(10) && t_mech < delta,
            "the mechanical thickness is a real competent layer inside the lid: {} km of {}",
            f64_of(t_mech),
            f64_of(delta)
        );

        // The competent-layer profile: the low-edge envelope capped at T_mech.
        struct CappedDomain<'a> {
            edge: EnvelopeEdge<'a>,
            cap_km: Fixed,
        }
        impl YieldEnvelope for CappedDomain<'_> {
            fn tensile_yield_gpa(&self, z: Fixed) -> Option<Fixed> {
                self.edge.tensile_yield_gpa(z)
            }
            fn compressive_yield_gpa(&self, z: Fixed) -> Option<Fixed> {
                self.edge.compressive_yield_gpa(z)
            }
            fn domain_max_depth_km(&self) -> Fixed {
                self.cap_km
            }
        }
        let competent = EnvelopeProfile::sample(
            &CappedDomain {
                edge: EnvelopeEdge::low(&env),
                cap_km: t_mech,
            },
            2000,
        )
        .expect("the competent layer samples");
        let d_mech = crate::flexure::flexural_rigidity(lit_e(), lit_nu(), t_mech)
            .expect("the competent elastic rigidity");

        // R(kappa) = M_yield / (D(T_mech) kappa): the moment the competent plate carries over the fully elastic
        // moment at the same curvature, concave-down (the seamount sign). Watts and Burov's T_e ratio is the cube
        // root of this. The curvature is in km^-1: 1e-8 per metre = 1e-5 per km.
        let ratio_at = |num: i64, den: i64| -> f64 {
            let k = FibreCurvature::from_upward_deflection(Fixed::from_ratio(num, den));
            let z_n = neutral_surface_depth_km(&competent, k, lit_e(), lit_nu()).expect("z_n");
            let m = bending_moment(&competent, k, z_n, lit_e(), lit_nu()).expect("M");
            let m_elastic = f64_of(d_mech) * f64_of(k.upward_per_km());
            f64_of(m.moment) / m_elastic
        };
        // THE ELASTIC-LIMIT SANITY: at a vanishing curvature nothing yields, so R must be 1 to the trapezoid's
        // own precision. This is what proves the ratio is measuring yielding rather than a quadrature offset.
        let r_tiny = ratio_at(-1, 100_000_000); // 1e-8 per km, effectively unbent
        assert!(
            (r_tiny - 1.0).abs() < 5e-3,
            "an unbent competent plate carries its full elastic moment: R = {r_tiny}"
        );
        let r_low = ratio_at(-4, 100_000); // 4e-8 per metre
        let r_high = ratio_at(-8, 100_000); // 8e-8 per metre
        let r_sharp = ratio_at(-8, 10_000); // 8e-7 per metre, an order harder
        println!(
            "UNIAXIAL COST: T_mech = {:.1} km, peak envelope = {:.0} MPa | \
             R_moment(4e-8/m)={r_low:.4} (T_e {:.4}), R_moment(8e-8/m)={r_high:.4} (T_e {:.4}), \
             R_moment(8e-7/m)={r_sharp:.4} (T_e {:.4})",
            f64_of(t_mech),
            f64_of(peak) * 1000.0,
            r_low.powf(1.0 / 3.0),
            r_high.powf(1.0 / 3.0),
            r_sharp.powf(1.0 / 3.0),
        );
        // THE MEASUREMENT'S STRUCTURE, asserted; the VERDICT is reported rather than pre-decided. Yielding only
        // ever REDUCES the moment (R <= 1), and MORE curvature yields MORE, so R falls monotonically. If it did
        // not, the measurement would be reading noise rather than yielding.
        assert!(
            r_low <= 1.0 + 5e-3 && r_high <= 1.0 + 5e-3,
            "yielding does not increase the moment: R = {r_low}, {r_high}"
        );
        assert!(
            r_low > r_high && r_high > r_sharp,
            "the ratio falls monotonically as curvature rises: {r_low} > {r_high} > {r_sharp}"
        );
        // AND THE SEAMOUNT REGIME IS NEAR-ELASTIC, distinguishing it from trench-wall curvatures: in Watts and
        // Burov's own T_e convention (the cube root) the seamount ratio sits within about ten per cent of one,
        // while the order-harder plate departs decisively. So the plate yields only modestly at seamount loads,
        // which is what makes the biaxial refinement a high-curvature effect rather than a hindcast one. The
        // exact percentage is the number reopening the question, reported above, not asserted to be zero.
        assert!(
            r_low.powf(1.0 / 3.0) > 0.9,
            "the seamount T_e ratio is near-elastic (within ten per cent of one): {}",
            r_low.powf(1.0 / 3.0)
        );
        assert!(
            r_sharp.powf(1.0 / 3.0) < r_high.powf(1.0 / 3.0) - 0.02,
            "an order-harder plate departs decisively further from elastic: {} against {}",
            r_sharp.powf(1.0 / 3.0),
            r_high.powf(1.0 / 3.0)
        );
    }

    #[test]
    fn the_construction_fails_loud_on_degenerate_inputs() {
        // No fabricated value on a degenerate input: each guard refuses.
        let profile = mm_illustration_profile(Fixed::from_ratio(1, 2));
        let k = mm_illustration_curvature();
        // An unbent plate reveals no rigidity.
        assert!(equivalent_rigidity(
            Fixed::from_int(10),
            FibreCurvature::from_upward_deflection(ZERO)
        )
        .is_none());
        // A degenerate modulus pair.
        assert!(elastic_thickness_km(Fixed::from_int(1000), ZERO, lit_nu()).is_none());
        assert!(elastic_thickness_km(Fixed::from_int(1000), lit_e(), Fixed::ONE).is_none());
        assert!(elastic_thickness_km(ZERO, lit_e(), lit_nu()).is_none());
        assert!(plane_strain_modulus_gpa(lit_e(), Fixed::ONE).is_none());
        assert!(plane_strain_modulus_gpa(ZERO, lit_nu()).is_none());
        // A fault pulled apart has no Coulomb resolution.
        assert!(mohr_coulomb_differential_mpa(
            Fixed::from_ratio(6, 10),
            Fixed::from_int(80),
            ZERO - Fixed::ONE,
            FaultingSense::Thrust
        )
        .is_none());
        // A zero-step or zero-domain profile.
        assert!(EnvelopeProfile::sample(
            &UniformYieldEnvelope {
                yield_gpa: Fixed::ONE,
                thickness_km: Fixed::from_int(40)
            },
            0
        )
        .is_none());
        assert!(EnvelopeProfile::sample(
            &UniformYieldEnvelope {
                yield_gpa: Fixed::ONE,
                thickness_km: ZERO
            },
            10
        )
        .is_none());
        // A curvature of zero has no fixed point to find.
        assert!(bending_moment(&profile, k, Fixed::from_int(20), ZERO, lit_nu()).is_none());
        // AN ENVELOPE WITH NO STRENGTH AT ALL cannot locate a neutral surface: every fibre stress is zero, so
        // the axial force is zero everywhere and there is no sign change to bracket a root with. The
        // construction refuses rather than returning an arbitrary depth.
        let dead = EnvelopeProfile::sample(
            &UniformYieldEnvelope {
                yield_gpa: ZERO,
                thickness_km: Fixed::from_int(40),
            },
            100,
        )
        .expect("profile");
        assert_eq!(
            neutral_surface_depth_km(&dead, k, lit_e(), lit_nu()),
            Ok(ZERO),
            "a strengthless column zeroes the axial force at its own surface, the degenerate root"
        );
    }

    // ===================================================================================================
    // THE DEEP V* BAND: where the ductile branch binds, several of H&K Table 2's determinations cover the
    // lid pressure WHILE DISAGREEING, so the envelope is a visible interval rather than a refusal.
    // ===================================================================================================

    /// The Earth-like lid carrying the BANKED Table 2 dislocation set (eight determinations), the shared fixture
    /// for the deep-band checks. Same body and geotherm as `earth_like_lid`, but the creep candidate reads the
    /// real banked volumes rather than a single fixture, which is what makes the deep bracket a real band.
    fn earth_like_banded_lid<'a>(
        creep: &'a [CreepCandidate<'a>],
        geotherm: &'a dyn Fn(Fixed) -> Option<Fixed>,
    ) -> LithosphereEnvelope<'a> {
        earth_like_lid(creep, geotherm, Fixed::from_int(100_000))
    }

    #[test]
    fn the_edge_envelopes_are_pointwise_ordered_low_below_high() {
        // THE INTERVAL-ARITHMETIC LICENSE, ITS FIRST HALF. A larger V* raises E* + P V*, which lowers the creep
        // rate at a given stress and so raises the strength: the ductile branch is monotone increasing in V*, so
        // the HIGH edge of the envelope band is pointwise at or above the LOW edge. The moment integral's own
        // monotonicity then carries this into an ordered rigidity band; here the pointwise half is put on trial
        // across the whole column, in both senses. A mutation swapping the two ends in `edge_yield` reverses this
        // and fires.
        let volumes = hk_dry_dislocation_activation_volumes();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let geotherm = ramp_geotherm;
        let env = earth_like_banded_lid(&creep, &geotherm);
        let delta = f64_of(env.lid_base.depth_km());

        let mut deep_band_seen = false;
        let mut z = 0.5;
        while z < delta {
            let zf = Fixed::from_ratio((z * 1e6) as i64, 1_000_000);
            for sense in [FaultingSense::Thrust, FaultingSense::Normal] {
                if let (Some(lo), Some(hi)) = (
                    env.edge_yield(zf, sense, VolumeEnd::Low),
                    env.edge_yield(zf, sense, VolumeEnd::High),
                ) {
                    assert!(
                        lo <= hi,
                        "the low V* edge must be at or below the high edge at {z} km, {sense:?}: {} against {}",
                        f64_of(lo),
                        f64_of(hi)
                    );
                    if hi > lo {
                        deep_band_seen = true;
                    }
                }
            }
            z += 0.5;
        }
        // AND THE ORDERING IS NOT VACUOUS: somewhere in the column the two edges do part company, or this
        // test would pass on a degenerate band that never exercised the inequality.
        assert!(
            deep_band_seen,
            "the deep column must open a real band where the two edges differ, or the ordering is untested"
        );
    }

    #[test]
    fn the_deep_band_is_visible_where_the_settled_view_refuses() {
        // THE ANOMALY THIS ARC ANSWERS, made concrete. At a deep lid depth the ductile branch binds and several
        // banked determinations cover the pressure while disagreeing, so the SETTLED view refuses (the two ends
        // are not equal) while the BANDED view reports the interval. Refusing there would claim the engine knows
        // nothing; the determinations say the strength lies within a measured spread, and the band is that spread.
        let volumes = hk_dry_dislocation_activation_volumes();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let geotherm = ramp_geotherm;
        let env = earth_like_banded_lid(&creep, &geotherm);

        let deep = Fixed::from_int(60);
        // THE SETTLED VIEW REFUSES: the V* span has reached the answer, so there is no single strength.
        assert!(
            env.compressive_yield_gpa(deep).is_none() && env.tensile_yield_gpa(deep).is_none(),
            "where the ductile ends disagree the settled view reports nothing"
        );
        // THE BANDED VIEW REPORTS BOTH EDGES, and they are a real interval, ordered, non-degenerate.
        let lo = env
            .edge_yield(deep, FaultingSense::Thrust, VolumeEnd::Low)
            .expect("the low edge is a determinate surface");
        let hi = env
            .edge_yield(deep, FaultingSense::Thrust, VolumeEnd::High)
            .expect("the high edge is a determinate surface");
        assert!(
            hi > lo,
            "the deep band is visible: [{}, {}] GPa",
            f64_of(lo),
            f64_of(hi)
        );
        // THE WIDTH IS WORTH SEEING, not a rounding: the covering set [6, 27] cm^3/mol drives a strength ratio of
        // several. Measured rather than intuited.
        let ratio = f64_of(hi) / f64_of(lo);
        assert!(
            ratio > 2.0,
            "the deep band spans a factor of several in strength: {ratio:.3}x"
        );
    }

    #[test]
    fn the_conditioning_narrows_the_band_and_the_residual_ships() {
        // THE HEADLINE RESULT, measured on the banked table. The band BEFORE conditioning drops each chord's
        // carried pressure endpoints and takes the table's bare V* extremes [-2, 27], which cover the lid only
        // because their endpoints were thrown away. The band AFTER conditioning respects the interval each chord
        // was drawn over, so at the lid pressure only the covering determinations [6, 27] contribute. The residual
        // [6, 27] is REAL disagreement (V* decreases with pressure; the primary predicts the non-overlap) and
        // ships as the band.
        let geotherm = ramp_geotherm;
        let deep = Fixed::from_int(60);
        let strength = |vols: &[ActivationVolume], end: VolumeEnd| {
            let creep = [CreepCandidate {
                row: hk_dry_dislocation(),
                volumes: vols,
            }];
            let env = earth_like_banded_lid(&creep, &geotherm);
            match env.ductile(deep, end) {
                DuctileReading::Determined(d) => f64_of(d),
                other => panic!("a hot deep lid creeps at a determinate strength, got {other:?}"),
            }
        };

        // BEFORE: the bare-endpoints-dropped span. The table's V* extremes (-2 and 27) pasted onto a covering
        // interval, which is exactly the "chord with its endpoints dropped" defect the interval tagging exists to
        // prevent: it drops each chord's real pressure range and treats both extremes as if they applied at the
        // lid.
        let bare = |v: i32| ActivationVolume {
            cm3_per_mol: Fixed::from_int(v),
            interval_min_gpa: Fixed::from_ratio(3, 10),
            interval_max_gpa: Fixed::from_int(2),
            modality: Modality::Fitted,
        };
        let before_lo = strength(&[bare(-2)], VolumeEnd::Low);
        let before_hi = strength(&[bare(27)], VolumeEnd::High);
        let before = before_hi / before_lo;

        // AFTER: the faithful nine determinations, pressure-conditioned by the selection at the lid pressure. The
        // -2 (Bejina, Si self-diffusion over 5 to 10 GPa) drops out because its chord does not reach 1.94 GPa, so
        // respecting the carried interval raises the low edge from -2's strength to 6's. This is the chord
        // discipline paying its dividend: the fields captured to prevent laundering are the legitimate
        // band-narrower.
        let nine = hk_table2_activation_volume_determinations();
        let after_lo = strength(&nine, VolumeEnd::Low);
        let after_hi = strength(&nine, VolumeEnd::High);
        let after = after_hi / after_lo;

        println!(
            "BAND WIDTH at 60 km: before conditioning {before:.3}x ([{before_lo:.2}, {before_hi:.2}] MPa), \
             after conditioning {after:.3}x ([{after_lo:.2}, {after_hi:.2}] MPa)"
        );
        // THE MEASURED NUMBERS, pinned to what the ruling reported (6.11x) and what conditioning leaves (3.71x).
        assert!(
            (before - 6.11).abs() < 0.05,
            "the bare-span band reproduces the ruling's 6.11x, got {before:.3}x"
        );
        assert!(
            (after - 3.71).abs() < 0.05,
            "conditioning on the chords' own pressure narrows it to 3.71x, got {after:.3}x"
        );
        // CONDITIONING NARROWS, AND THE RESIDUAL IS REAL. The band shrinks but does not close: what remains is the
        // primary's own scatter within matching pressures, which ships rather than being averaged away.
        assert!(
            after < before && after > 1.5,
            "conditioning narrows the band to a real residual, not a point: {before:.3}x -> {after:.3}x"
        );
    }

    #[test]
    fn the_rigidity_band_orders_refuses_a_non_plate_and_overlaps_by_intersection() {
        // THE BAND TYPE'S OWN ALGEBRA, and the hindcast comparison it exists for.
        // Ordering: the two ends sort regardless of the order handed in.
        let b =
            RigidityBand::new(Fixed::from_int(300_000), Fixed::from_int(500_000)).expect("band");
        assert_eq!(b.low(), Fixed::from_int(300_000));
        assert_eq!(b.high(), Fixed::from_int(500_000));
        let flipped =
            RigidityBand::new(Fixed::from_int(500_000), Fixed::from_int(300_000)).expect("band");
        assert_eq!(
            b, flipped,
            "the band sorts its ends, so input order cannot move it"
        );
        // A non-plate refuses rather than reporting a zero-rigidity band.
        assert!(RigidityBand::new(ZERO, Fixed::from_int(1000)).is_none());
        assert!(RigidityBand::new(Fixed::from_int(1000), ZERO - Fixed::ONE).is_none());
        // Degeneracy is a zero-width band, which is the shallow column.
        assert!(
            RigidityBand::new(Fixed::from_int(1000), Fixed::from_int(1000))
                .unwrap()
                .is_degenerate()
        );
        assert!(!b.is_degenerate());

        // THE HINDCAST COMPARISON IS OVERLAP, NEVER POINT EQUALITY. Two bands agree when their intervals
        // intersect; a point-equality test would convict the source's own V* spread as a modelling error.
        let engine = RigidityBand::new(Fixed::from_int(300_000), Fixed::from_int(500_000)).unwrap();
        // A hindcast row whose scatter overlaps but whose CENTRE differs: overlap says agree, `==` would not.
        let row_overlapping =
            RigidityBand::new(Fixed::from_int(450_000), Fixed::from_int(700_000)).unwrap();
        assert!(
            engine.overlaps(row_overlapping) && row_overlapping.overlaps(engine),
            "overlapping bands agree, and the relation is symmetric"
        );
        assert_ne!(
            engine, row_overlapping,
            "and they are NOT equal: overlap is not point equality, which is the whole ruling"
        );
        // A row whose scatter is disjoint: no overlap, a real disagreement.
        let row_disjoint =
            RigidityBand::new(Fixed::from_int(600_000), Fixed::from_int(800_000)).unwrap();
        assert!(
            !engine.overlaps(row_disjoint),
            "disjoint bands do not overlap: a real disagreement, reported"
        );
        // Touching at an endpoint counts as overlap (the intervals are closed), which is the honest boundary.
        let row_touch =
            RigidityBand::new(Fixed::from_int(500_000), Fixed::from_int(900_000)).unwrap();
        assert!(
            engine.overlaps(row_touch),
            "closed intervals touching at an edge overlap"
        );

        // THE ROW'S BAND IS BUILT THROUGH ITS OWN MODULUS PAIR, which is the like-against-like the header demands:
        // a published T_e interval becomes a rigidity interval via D = E H^3 / (12 (1 - nu^2)), monotone in H.
        let row_band = RigidityBand::from_hindcast_thickness_interval(
            Fixed::from_int(30),
            Fixed::from_int(40),
            lit_e(),
            lit_nu(),
        )
        .expect("the row converts to a rigidity band");
        // TWIN: D(30) and D(40) at (80, 0.25) computed OUTSIDE this codebase, against the band's ends.
        let d30 = 80.0 * 30.0_f64.powi(3) / (12.0 * (1.0 - 0.0625));
        let d40 = 80.0 * 40.0_f64.powi(3) / (12.0 * (1.0 - 0.0625));
        assert!(
            (f64_of(row_band.low()) - d30).abs() < d30 * 1e-4
                && (f64_of(row_band.high()) - d40).abs() < d40 * 1e-4,
            "the row band is [D(30), D(40)] through its own pair: [{}, {}] against [{d30}, {d40}]",
            f64_of(row_band.low()),
            f64_of(row_band.high())
        );
    }

    #[test]
    fn the_banded_solve_runs_the_fixed_point_at_both_edges_and_carries_the_chord() {
        // TWO CONVERGENT SOLVES, ORDERED. The deep-binding column is solved at both edges of the V* span, each a
        // full fixed point with its own curvature and neutral surface, and the two rigidities are an ordered band
        // because the high V* edge is the stronger plate. Nothing is averaged; the band is what the primary's own
        // scatter licenses, carried.
        let volumes = hk_dry_dislocation_activation_volumes();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let geotherm = ramp_geotherm;
        let env = earth_like_banded_lid(&creep, &geotherm);
        let dr = Fixed::from_ratio(33, 10);
        let g = Fixed::from_ratio(98, 10000);
        // A load that bends this 62 km lid into its own yielding and converges at both edges (measured: both
        // converge well inside the iteration cap and below the elastic ceiling).
        let load = Fixed::from_int(64);
        let banded =
            solve_line_load_banded(&env, 600, lit_e(), lit_nu(), dr, g, load, test_chord())
                .expect("both edges of the V* band converge on this deep lid");

        let band = banded.rigidity_band();
        assert!(
            band.low() < band.high(),
            "the deep column's rigidity is a real band: [{}, {}] GPa km^3",
            f64_of(band.low()),
            f64_of(band.high())
        );
        assert!(
            !banded.is_degenerate(),
            "the deep column opens a band rather than collapsing to a point"
        );
        // BOTH EDGES ARE FIXED POINTS: re-entering each converged rigidity reproduces it, which is the property
        // the solve claims rather than the number it stopped at.
        for plate in [banded.low_edge(), banded.high_edge()] {
            let alpha = crate::flexure::flexural_parameter(plate.rigidity_gpa_km3, dr, g).unwrap();
            let k = line_load_curvature_at_first_zero_crossing(load, alpha, plate.rigidity_gpa_km3)
                .unwrap();
            // Each edge owns its OWN profile; re-derive it to re-enter the fixed point at that edge.
            let end = if plate.rigidity_gpa_km3 == banded.low_edge().rigidity_gpa_km3 {
                VolumeEnd::Low
            } else {
                VolumeEnd::High
            };
            let edge = if end == VolumeEnd::Low {
                EnvelopeEdge::low(&env)
            } else {
                EnvelopeEdge::high(&env)
            };
            let profile = EnvelopeProfile::sample(&edge, 600).unwrap();
            let z_n = neutral_surface_depth_km(&profile, k, lit_e(), lit_nu()).unwrap();
            let m = bending_moment(&profile, k, z_n, lit_e(), lit_nu()).unwrap();
            let d_again = equivalent_rigidity(m.moment, k).unwrap();
            assert!(
                (f64_of(d_again) - f64_of(plate.rigidity_gpa_km3)).abs()
                    < f64_of(plate.rigidity_gpa_km3) * 1e-6,
                "each edge is a fixed point: re-entering {} reproduces it, got {}",
                f64_of(plate.rigidity_gpa_km3),
                f64_of(d_again)
            );
        }

        // THE CHORD RIDES WITH BOTH EDGES: a rigidity band without its load class and timescale is a band with a
        // hidden conditioning variable.
        for plate in [banded.low_edge(), banded.high_edge()] {
            assert_eq!(plate.chord.class, LoadClassId(1));
            assert_eq!(plate.chord.timescale_s, Fixed::from_int(31_557_600));
            assert_eq!(plate.chord.ln_strain_rate_per_s, ln_scientific(1, 1, -15));
        }

        // THE DISPLAY-THICKNESS BAND is available at a declared pair and is ordered the same way.
        let (te_lo, te_hi) = banded
            .elastic_thickness_band_km(lit_e(), lit_nu())
            .expect("the thickness band resolves");
        assert!(
            te_lo < te_hi && f64_of(te_hi) <= 62.3,
            "the display-thickness band is ordered and inside the lid: [{}, {}] km",
            f64_of(te_lo),
            f64_of(te_hi)
        );
    }

    #[test]
    fn the_lid_referee_verdict_rides_the_banded_plate_when_attached() {
        // THE VERDICT RIDES THE OUTPUT rather than being computed and discarded. `referee_conductive_lid_base` was
        // reachable only from tests: it produced a verdict nothing carried. A plate solved against a lid base the
        // referee convicts as too shallow has a moment integral cut short at that base, so its `T_e` reads low, and
        // a consumer needs to see that riding the plate. `with_lid_referee` attaches the verdict; the pure solve
        // leaves it absent. A mutation nulling the attach, or flipping which verdict raises the flag, dies here.
        let volumes = hk_dry_dislocation_activation_volumes();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let geotherm = ramp_geotherm;
        let env = earth_like_banded_lid(&creep, &geotherm);
        let dr = Fixed::from_ratio(33, 10);
        let g = Fixed::from_ratio(98, 10000);
        let load = Fixed::from_int(64);
        let solve = || {
            solve_line_load_banded(&env, 600, lit_e(), lit_nu(), dr, g, load, test_chord())
                .expect("both edges of the V* band converge on this deep lid")
        };

        // THE PURE SOLVE CARRIES NO VERDICT: the field is absent and the bias is None (not Unbiased, since with no
        // referee we cannot say), so the attach below is what raises it and not the solve always speaking.
        let bare = solve();
        assert!(
            bare.lid_referee().is_none(),
            "the pure banded solve attaches no referee"
        );
        assert_eq!(
            bare.te_bias(),
            None,
            "with no verdict attached the bias is unknown, reported None rather than Unbiased"
        );

        // The measured strength at the derived lid base, both V* ends determinate and ordered, is the pivot the two
        // convective stresses below straddle. This is the same probe the standalone referee test pivots on.
        let probe =
            referee_conductive_lid_base(&env, Fixed::ONE).expect("the ductile branch answers");
        let (lo, hi) = match (probe.strength_low, probe.strength_high) {
            (DuctileReading::Determined(l), DuctileReading::Determined(h)) => {
                (f64_of(l), f64_of(h))
            }
            other => panic!(
                "the premise: both V* ends creep at a determinate strength at the base, got {other:?}"
            ),
        };
        assert!(
            0.0 < lo && lo <= hi,
            "the base strengths are positive and ordered: {lo}, {hi} MPa"
        );

        // A CONVECTIVE STRESS THE LID'S STRENGTH EXCEEDS AT BOTH ENDS convicts the base as too shallow: the verdict
        // rides the plate and the bias flag is up. Half the weaker end is below both.
        let below_both = Fixed::from_ratio((lo * 0.5 * 1e6) as i64, 1_000_000);
        let biased = solve().with_lid_referee(&env, below_both);
        assert_eq!(
            biased.lid_referee(),
            referee_conductive_lid_base(&env, below_both).as_ref(),
            "the attached verdict is exactly the referee's own, unaltered by riding the plate"
        );
        assert_eq!(
            biased.lid_referee().expect("attached").verdict,
            LidVerdict::StrengthExceedsConvectiveStress,
            "a stress the lid's strength exceeds at both ends convicts the base"
        );
        assert_eq!(
            biased.te_bias(),
            Some(TeBias::BiasedLow),
            "the convicting verdict reads as biased low on the plate"
        );

        // A CONVECTIVE STRESS ABOVE BOTH ENDS confirms the base: the verdict rides and the bias is Unbiased, so the
        // bias tracks the verdict and not merely the presence of one.
        let above_both = Fixed::from_ratio((hi * 2.0 * 1e6) as i64, 1_000_000);
        let confirmed = solve().with_lid_referee(&env, above_both);
        assert_eq!(
            confirmed.lid_referee().expect("attached").verdict,
            LidVerdict::Confirmed,
            "a stress above the lid's strength at both ends confirms the base"
        );
        assert_eq!(
            confirmed.te_bias(),
            Some(TeBias::Unbiased),
            "a confirmed base is unbiased, though a verdict is attached"
        );

        // A CONVECTIVE STRESS BETWEEN THE TWO ENDS is the straddle: the weaker V* end has fallen to the stress and
        // the stronger has not, so the ends DISAGREE about whether the lid truncated the integral. The amendment is
        // that this surfaces as PossiblyBiasedLow rather than collapsing to unflagged (which a two-valued flag would
        // have done, laundering the ambiguity to clean). The deep band gives distinct base strengths, so a midpoint
        // straddles.
        assert!(
            lo < hi,
            "the deep band's two ends give distinct base strengths, so a midpoint can straddle: {lo}, {hi}"
        );
        let between = Fixed::from_ratio(((lo + hi) / 2.0 * 1e6) as i64, 1_000_000);
        let straddle = solve().with_lid_referee(&env, between);
        assert_eq!(
            straddle.lid_referee().expect("attached").verdict,
            LidVerdict::BracketStraddlesConvectiveStress,
            "a stress between the two ends leaves the question open: the referee straddles"
        );
        assert_eq!(
            straddle.te_bias(),
            Some(TeBias::PossiblyBiasedLow),
            "the straddle surfaces as possibly biased low, never laundered to unflagged"
        );
    }

    #[test]
    fn the_lid_column_assembles_te_for_silicate_and_refuses_ice() {
        // THE ASSEMBLER END TO END, on both a silicate lid and an ice shell, through ONE code path. The silicate
        // column composes rock friction with olivine creep and derives a T_e band; the ice column composes ice
        // friction with NO creep row (none is built yet) and REFUSES, which a render reads as the honest empty
        // middle rather than a fabricated relief. The alien and the Earth-like differ only by their data rows.
        let volumes = hk_dry_dislocation_activation_volumes();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let silicate = LidColumn {
            friction: rock_friction_law(),
            creep: &creep,
            surface_temperature_k: Fixed::from_int(273),
            interior_temperature_k: Fixed::from_int(1600),
            density_kg_m3: Fixed::from_int(3300),
            heat_production: ZERO,
            thermal_conductivity: Fixed::from_int(3),
            gravity_m_s2: Fixed::from_ratio(981, 100),
            convecting_depth_km: Fixed::from_int(2890),
            rayleigh: Fixed::from_int(100_000),
            chord: test_chord(),
        };
        let (lo, hi) = silicate
            .elastic_thickness_band_km(
                lit_e(),
                lit_nu(),
                Fixed::from_ratio(33, 10),
                Fixed::from_int(64),
                600,
            )
            .expect("a silicate lid derives a T_e band on the shipped friction and creep rows");
        // The band is a real elastic thickness INSIDE the derived lid (the ~62 km conductive base), and its edges
        // are ordered. The geotherm now ramps over the SAME derived lid base the moment integral stops at (one
        // derivation), so this is the coherent value, not the too-cold-at-base overestimate a free ramp gave.
        assert!(
            lo < hi && f64_of(lo) > 2.0 && f64_of(hi) < 62.3,
            "the silicate T_e band is a real elastic thickness inside the lid: {} to {} km",
            f64_of(lo),
            f64_of(hi)
        );

        // POSITIVE MATERIAL-AGNOSTIC EVIDENCE, not merely silicate-derives / alien-refuses: a NON-silicate friction
        // row still traverses the derive path. Swapping rock friction for ice friction (Beeman, far weaker) while
        // keeping the rest still derives a T_e band, which proves the friction axis is a free data row rather than a
        // silicate-hardcoded assumption. The ice-friction lid is weaker so its band is lower, but it DERIVES.
        let chimera = LidColumn {
            friction: ice_friction_law(),
            ..silicate
        };
        assert!(
            chimera
                .elastic_thickness_band_km(
                    lit_e(),
                    lit_nu(),
                    Fixed::from_ratio(33, 10),
                    Fixed::from_int(64),
                    600,
                )
                .is_some(),
            "a non-silicate friction row still traverses the derive path: the friction axis is a free input"
        );

        // An ice shell with NO creep row: the ductile branch has no admitted candidate, so the column refuses.
        let no_creep: [CreepCandidate; 0] = [];
        let ice = LidColumn {
            friction: ice_friction_law(),
            creep: &no_creep,
            surface_temperature_k: Fixed::from_int(100),
            interior_temperature_k: Fixed::from_int(260),
            density_kg_m3: Fixed::from_int(920),
            heat_production: ZERO,
            thermal_conductivity: Fixed::from_ratio(22, 10),
            gravity_m_s2: Fixed::from_ratio(131, 100),
            convecting_depth_km: Fixed::from_int(30),
            rayleigh: Fixed::from_int(100_000),
            chord: test_chord(),
        };
        assert!(
            ice.elastic_thickness_band_km(
                lit_e(),
                lit_nu(),
                Fixed::from_ratio(8, 100),
                Fixed::from_int(2),
                600,
            )
            .is_none(),
            "an ice shell with no creep row refuses: the honest empty middle until ice rows land"
        );
    }

    #[test]
    fn the_mechanical_rigidity_is_the_elastic_ceiling_over_the_full_lid_domain() {
        // OPTION B, EXECUTABLE: the field filter's linear-response rigidity is D_mech, the fully-elastic rigidity of
        // the lid domain and the v0 -> 0 limit, so the stiffest the column can be. Two properties pin it. It
        // round-trips to the FULL lid domain: a T_e read back from D_mech through the moduli recovers the conductive
        // lid base, so D_mech is the rigidity of the whole competent lid. And it is the CEILING the load-conditioned
        // rigidity sits under: a plate that yields is softer, never stiffer, so the assembler's T_e-band rigidity
        // stays at or below D_mech. This is the linearity license the transfer function runs on.
        let volumes = hk_dry_dislocation_activation_volumes();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let silicate = LidColumn {
            friction: rock_friction_law(),
            creep: &creep,
            surface_temperature_k: Fixed::from_int(273),
            interior_temperature_k: Fixed::from_int(1600),
            density_kg_m3: Fixed::from_int(3300),
            heat_production: ZERO,
            thermal_conductivity: Fixed::from_int(3),
            gravity_m_s2: Fixed::from_ratio(981, 100),
            convecting_depth_km: Fixed::from_int(2890),
            rayleigh: Fixed::from_int(100_000),
            chord: test_chord(),
        };
        let d_mech = silicate
            .mechanical_rigidity(lit_e(), lit_nu())
            .expect("D_mech resolves");
        assert!(f64_of(d_mech) > 0.0, "D_mech is positive");

        // Round-trip: D_mech's own T_e IS the conductive lid base, the full competent domain.
        let lid_base =
            ConductiveLidBase::from_rayleigh(silicate.convecting_depth_km, silicate.rayleigh)
                .unwrap();
        let te_mech = elastic_thickness_km(d_mech, lit_e(), lit_nu()).unwrap();
        assert!(
            (f64_of(te_mech) - f64_of(lid_base.depth_km())).abs() < 0.05,
            "D_mech round-trips to the full lid domain: T_e {} vs lid base {} km",
            f64_of(te_mech),
            f64_of(lid_base.depth_km())
        );

        // Ceiling: the assembler's load-conditioned rigidity, at its stiffest T_e edge, sits at or below D_mech.
        let (_lo, hi) = silicate
            .elastic_thickness_band_km(
                lit_e(),
                lit_nu(),
                Fixed::from_ratio(33, 10),
                Fixed::from_int(64),
                600,
            )
            .unwrap();
        let d_load_stiffest = crate::flexure::flexural_rigidity(lit_e(), lit_nu(), hi).unwrap();
        assert!(
            d_load_stiffest <= d_mech,
            "D_mech is the elastic ceiling: load-conditioned {} <= mechanical {}",
            f64_of(d_load_stiffest),
            f64_of(d_mech)
        );
    }

    #[test]
    fn the_low_stress_roughness_band_widens_the_silicate_te() {
        // CALL ONE, EXECUTABLE: the roughness band widens the T_e where it applies, and its SCOPE confines how far.
        // Byerlee's low fit is a central line through roughness scatter, so a band over its domain softens the
        // shallow brittle limb at the weak edge and stiffens it at the strong one, and the interval-of-mins in
        // `edge_yield` carries that to the T_e band, low with low and high with high. But the scatter is a REGIME,
        // below Byerlee's own 5 MPa boundary, so a band scoped to 5 MPa touches only the shallowest sliver and moves
        // the T_e far less than one smeared across the whole low branch: the 40x-in-depth correction, executable.
        // The fixture values are a round illustrative band, NOT the canonical Byerlee cloud; the test asserts the
        // widening and its scoping, never a number.
        let volumes = hk_dry_dislocation_activation_volumes();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        // Derive the silicate T_e low edge for a given friction law, all else the Earth-like column held fixed.
        let te_lo = |friction: FrictionLaw| {
            LidColumn {
                friction,
                creep: &creep,
                surface_temperature_k: Fixed::from_int(273),
                interior_temperature_k: Fixed::from_int(1600),
                density_kg_m3: Fixed::from_int(3300),
                heat_production: ZERO,
                thermal_conductivity: Fixed::from_int(3),
                gravity_m_s2: Fixed::from_ratio(981, 100),
                convecting_depth_km: Fixed::from_int(2890),
                rayleigh: Fixed::from_int(100_000),
                chord: test_chord(),
            }
            .elastic_thickness_band_km(
                lit_e(),
                lit_nu(),
                Fixed::from_ratio(33, 10),
                Fixed::from_int(64),
                600,
            )
            .expect("the silicate lid derives a T_e band")
            .0
        };
        // A fixture roughness band (coefficient 0.6 to 1.0, no cohesion), scoped either across the whole low branch
        // or to Byerlee's own 5 MPa boundary. This is a round fixture, not the canonical 0.3-to-10 cloud; its only
        // job is to move the T_e measurably so the mechanism and its scoping are visible.
        let fixture_band = |scatter_domain_max: Fixed| LowStressBand {
            coefficient_lo: Fixed::from_ratio(6, 10),
            coefficient_hi: Fixed::ONE,
            cohesion_lo: Fixed::ZERO,
            cohesion_hi: Fixed::ZERO,
            scatter_domain_max,
            intermediate_band: None,
        };
        let unbanded = te_lo(FrictionLaw {
            low_stress_band: None,
            ..rock_friction_law()
        });
        let broad = te_lo(FrictionLaw {
            low_stress_band: Some(fixture_band(Fixed::from_int(200))),
            ..rock_friction_law()
        });
        let scoped = te_lo(FrictionLaw {
            low_stress_band: Some(fixture_band(Fixed::from_int(5))),
            ..rock_friction_law()
        });

        // The mechanism: a band spanning the low branch drops the T_e low edge below the un-banded one (the softened
        // shallow limb). The scope: the 5 MPa-scoped band widens STRICTLY LESS, since it touches only the shallowest
        // sliver, so its low edge sits above the broad band's and at or below the un-banded one. The same scatter,
        // confined to its own regime, barely moves the number, which is the whole scope correction made executable.
        assert!(
            broad < unbanded,
            "a low-branch-spanning band widens the T_e: {} < {}",
            f64_of(broad),
            f64_of(unbanded)
        );
        assert!(
            broad < scoped && scoped <= unbanded,
            "the 5 MPa scope confines the widening: broad {} < scoped {} <= unbanded {}",
            f64_of(broad),
            f64_of(scoped),
            f64_of(unbanded)
        );
    }

    #[test]
    fn the_banded_solve_is_degenerate_on_an_all_brittle_shallow_column() {
        // HONESTY CHANGES SHAPE, NOT WHETHER THE ENGINE SPEAKS. Where the column is shallow and cold the ductile
        // branch never binds (the flow law says creep is irrelevant at a geological rate), so the envelope is the
        // brittle branch at both V* edges and the band is a POINT. The banded solve returns it, and the two edges
        // agree to the bit, which is the shallow bracket's invariance surviving into the solve.
        let volumes = hk_dry_dislocation_activation_volumes();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        let geotherm = ramp_geotherm;
        // A vigorous mantle (Ra = 7e6) shears over a ~15 km lid; at 15 km on the ramp geotherm the rock is not
        // creeping at 1e-15/s, so the whole column is brittle and V*-independent.
        let env = earth_like_lid(&creep, &geotherm, Fixed::from_int(7_000_000));
        assert!(
            f64_of(env.lid_base.depth_km()) < 16.0,
            "Ra = 7e6 gives a thin lid: {} km",
            f64_of(env.lid_base.depth_km())
        );
        assert_eq!(
            env.ductile(env.lid_base.depth_km(), VolumeEnd::Low),
            DuctileReading::AboveRepresentable,
            "the thin lid's base is not creeping at a geological rate, so the column is all brittle"
        );
        let dr = Fixed::from_ratio(33, 10);
        let g = Fixed::from_ratio(98, 10000);
        let banded = solve_line_load_banded(
            &env,
            400,
            lit_e(),
            lit_nu(),
            dr,
            g,
            Fixed::from_int(4),
            test_chord(),
        )
        .expect("the shallow column converges");
        assert!(
            banded.is_degenerate(),
            "an all-brittle column is V*-independent, so its rigidity band is a point: [{}, {}]",
            f64_of(banded.rigidity_band().low()),
            f64_of(banded.rigidity_band().high())
        );
        assert_eq!(
            banded.low_edge().rigidity_gpa_km3,
            banded.high_edge().rigidity_gpa_km3,
            "the two edges agree to the bit where the span cannot move the answer"
        );
    }

    #[test]
    fn the_band_edge_combinator_reports_each_finding() {
        // THE FINDING ARMS, PUT ON TRIAL DIRECTLY, because a load sweep on the banked envelope cannot reach the
        // straddle (the flexure arithmetic leaves the representable window at both edges within one load step of
        // each other, before either edge's support limit is crossed alone). This is stated as a blindness of the
        // end-to-end path in `solve_line_load_banded`'s own doc; here the pure combinator's arms are exercised
        // with synthetic edge results, so no arm is dead-untested.
        let plate = |d: i64| MomentEquivalentPlate {
            rigidity_gpa_km3: Fixed::from_int(d as i32),
            curvature: FibreCurvature::from_upward_deflection(Fixed::from_ratio(-1, 2000)),
            neutral_depth_km: Fixed::from_int(20),
            moment: MomentReading {
                moment: Fixed::from_int(100),
                neutral_depth_km: Fixed::from_int(20),
                self_truncated: false,
                truncation_depth_km: None,
                final_interval_contribution: Fixed::ONE,
            },
            chord: test_chord(),
            iterations: 5,
        };
        let sup = || Err(MomentEquivalenceRefusal::LoadExceedsElasticSupport);
        let other = || Err(MomentEquivalenceRefusal::ZeroCurvature);

        // BOTH Ok and ordered: a band, low below high.
        let band = combine_band_edges(Ok(plate(300_000)), Ok(plate(500_000))).expect("band");
        assert_eq!(band.rigidity_band().low(), Fixed::from_int(300_000));
        assert_eq!(band.rigidity_band().high(), Fixed::from_int(500_000));

        // BOTH Ok but UNORDERED: the monotonicity license failed, a stop rather than a swap.
        assert_eq!(
            combine_band_edges(Ok(plate(500_000)), Ok(plate(300_000))).unwrap_err(),
            MomentEquivalenceRefusal::BandRigidityUnordered {
                low_gpa_km3: Fixed::from_int(500_000),
                high_gpa_km3: Fixed::from_int(300_000),
            },
            "an unordered band stops rather than silently swapping"
        );

        // EXACTLY ONE holds the load: the straddle finding, naming which edge held.
        assert_eq!(
            combine_band_edges(Ok(plate(300_000)), sup()).unwrap_err(),
            MomentEquivalenceRefusal::BandEdgeSupportDisagrees {
                low_edge_converged: true
            },
            "low held, high did not: a finding naming the low edge"
        );
        assert_eq!(
            combine_band_edges(sup(), Ok(plate(300_000))).unwrap_err(),
            MomentEquivalenceRefusal::BandEdgeSupportDisagrees {
                low_edge_converged: false
            },
            "high held, low did not: a finding naming the high edge"
        );

        // BOTH fail support: honest, routes to the support-bound branch, NOT a straddle.
        assert_eq!(
            combine_band_edges(sup(), sup()).unwrap_err(),
            MomentEquivalenceRefusal::LoadExceedsElasticSupport,
            "the whole span agreeing the load is not held is not a straddle"
        );

        // ANY OTHER refusal is structural and propagates rather than becoming a band finding.
        assert_eq!(
            combine_band_edges(Ok(plate(300_000)), other()).unwrap_err(),
            MomentEquivalenceRefusal::ZeroCurvature,
            "a structural refusal on one edge propagates, never a straddle"
        );
        assert_eq!(
            combine_band_edges(other(), Ok(plate(300_000))).unwrap_err(),
            MomentEquivalenceRefusal::ZeroCurvature
        );

        // A NUMERICAL non-convergence is structural, NOT a support disagreement. This is the Finding-1 guarantee at
        // the combinator: `FixedPointDidNotConverge` is a numerical residual and must never be laundered into
        // `BandEdgeSupportDisagrees` (the physical straddle) the way `LoadExceedsElasticSupport` is. It propagates
        // as itself, carrying its final step size, so a caller reads a numerical stall as a numerical stall.
        let unconverged = || {
            Err(MomentEquivalenceRefusal::FixedPointDidNotConverge {
                final_delta_gpa_km3: Fixed::from_int(7),
            })
        };
        assert_eq!(
            combine_band_edges(Ok(plate(300_000)), unconverged()).unwrap_err(),
            MomentEquivalenceRefusal::FixedPointDidNotConverge {
                final_delta_gpa_km3: Fixed::from_int(7),
            },
            "a numerical non-convergence on one edge propagates as itself, never a support straddle"
        );
        assert_eq!(
            combine_band_edges(unconverged(), Ok(plate(300_000))).unwrap_err(),
            MomentEquivalenceRefusal::FixedPointDidNotConverge {
                final_delta_gpa_km3: Fixed::from_int(7),
            }
        );
    }

    #[test]
    fn the_edge_yield_composes_inside_the_friction_gap_at_both_ends() {
        // BAND NEVER REFUSE, SYMMETRICALLY, on trial. An ice shell has a depth band where the brittle branch is
        // itself a bracket, because Beeman's two friction fits leave a gap from 5 to 10 MPa that the calibration
        // declines to collapse. The settled view refused there at both ends, which silenced every icy lid at
        // mid-shell: Terran bias expressed as an unhandled match arm. `edge_yield` now reads the SAME named end of
        // the brittle bracket that it reads of the ductile limb, so the interval of mins composes over both limbs
        // rather than going None. The Low edge reads the weaker friction limb, the High edge the stronger, and the
        // band stays ordered. A mutation run showed the composition arm was untested.
        let volumes = hk_dry_dislocation_activation_volumes();
        let creep = [CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        }];
        // A cold conductive Europa-class shell, its own lid: the same fixture the friction-gap test uses.
        let geotherm = |_z: Fixed| Some(Fixed::from_int(100));
        let lid_base = ConductiveLidBase::from_rayleigh(Fixed::from_int(30), ZERO)
            .expect("a non-convecting shell is its own lid");
        let shell = LithosphereEnvelope {
            friction: ice_friction_law(),
            density_kg_m3: Fixed::from_int(920),
            gravity_m_s2: Fixed::from_ratio(131, 100),
            geotherm_k: &geotherm,
            creep: &creep,
            chord: test_chord(),
            lid_base,
        };
        // 15 MPa of overburden sits near 12.4 km on this shell, inside its NORMAL-faulting friction gap (the
        // sense the two Beeman fits straddle over 5 to 10 MPa). The brittle branch brackets there.
        let in_gap = Fixed::from_int(12);
        let (low_mpa, high_mpa) = match shell.brittle(in_gap, FaultingSense::Normal) {
            Some(DifferentialStrength::Bracket { low, high }) => (low, high),
            other => panic!(
                "the premise: the friction branch is a bracket at 12 km in the normal sense, got {other:?}"
            ),
        };
        // The cold column does not creep, so the ductile limb is above representable at BOTH ends and the brittle
        // bracket floors the envelope unchanged. This premise licenses the exact equality below: `edge_yield`
        // returns the named brittle limb, converted to GPa, with no ductile minimum biting.
        for end in [VolumeEnd::Low, VolumeEnd::High] {
            assert_eq!(
                shell.ductile(in_gap, end),
                DuctileReading::AboveRepresentable,
                "the premise: the cold shell does not creep at 12 km, {end:?}"
            );
        }
        // THE FIX: the edge no longer refuses in the gap. It composes symmetrically, the Low edge reading the
        // weaker friction limb and the High edge the stronger, each floored by the non-creeping brittle branch.
        let low_edge = shell
            .edge_yield(in_gap, FaultingSense::Normal, VolumeEnd::Low)
            .expect("the low edge composes the gap rather than refusing");
        let high_edge = shell
            .edge_yield(in_gap, FaultingSense::Normal, VolumeEnd::High)
            .expect("the high edge composes the gap rather than refusing");
        assert_eq!(
            low_edge,
            low_mpa.checked_div(Fixed::from_int(MPA_PER_GPA)).unwrap(),
            "the low edge reads the low limb of the friction bracket, in GPa"
        );
        assert_eq!(
            high_edge,
            high_mpa.checked_div(Fixed::from_int(MPA_PER_GPA)).unwrap(),
            "the high edge reads the high limb of the friction bracket, in GPa"
        );
        assert!(
            low_edge <= high_edge,
            "the composed band stays ordered: the weak edge is no stronger than the strong edge"
        );
        // AND WHERE THE FRICTION BRANCH DETERMINES, the edge still answers, so the composition above is the gap
        // being read and not the function simply always speaking. Deep in the shell (about 30 MPa overburden at
        // 25 km, well past the 10 MPa gap top) the brittle branch determines and floors the cold, non-creeping
        // column.
        let determinate = Fixed::from_int(25); // ~30 MPa on a 920 kg/m^3, 1.31 m/s^2 shell, inside the 30 km lid
        assert!(
            matches!(
                shell.brittle(determinate, FaultingSense::Thrust),
                Some(DifferentialStrength::Determined(_))
            ),
            "the premise: the friction branch determines deep in the shell"
        );
        assert!(
            shell
                .edge_yield(determinate, FaultingSense::Thrust, VolumeEnd::Low)
                .is_some(),
            "where the friction branch determines the edge answers"
        );
    }
}
