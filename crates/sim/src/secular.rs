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

//! THE SECULAR-PERTURBATION SPECTRUM (R-CELESTIAL-SECULAR, task #44): from an assembled multi-planet
//! system this derives the classical Laplace-Lagrange secular eigenfrequencies, the `g` (eccentricity /
//! pericenter) modes and the `s` (inclination / node) modes, that ARE the Milankovitch-class forcing a
//! climate module reads. The mode table is the archive object; nothing here is looked up or interpolated,
//! every number falls out of the masses and semimajor axes the assembly delivers plus the cited law
//! constants.
//!
//! THE METHOD (the R-CELESTIAL-SECULAR ruling, `docs/working/CAPSTONE_RESEARCH_RESOLUTIONS.md` section 6).
//! Classical Laplace-Lagrange in the eccentricity and inclination vector variables. The `A` (eccentricity /
//! pericenter) and `B` (inclination / node) matrices are built from Laplace coefficients the ENGINE
//! COMPUTES ([`laplace_coefficient`], the convergent hypergeometric series), never cited. The mass- and
//! frequency-weighted similarity transform ([`symmetrize`]) makes the eigenproblem REAL SYMMETRIC, so the
//! frequencies are real, the fold is deterministic, and it is fixed-point safe; a deterministic Jacobi
//! rotation sweep ([`jacobi_eigenvalues`]) reads the eigenvalues off the diagonal. The Gap-Law hooks are
//! native: near-degenerate eigenfrequencies and secular resonances are CARRIED as flags rather than
//! asserted, and the validity domain excludes mean-motion-resonance proximity. Close-in planets trip the
//! relativity smallness flag from their orbital elements alone, so the general-relativistic pericenter
//! precession joins the `A` diagonal as a DERIVED correction (the Mercury case is that auto-flag, not a
//! special case).
//!
//! THE MATRIX ELEMENT DEFINITIONS (verbatim, `docs/working/CAPSTONE_FETCH_VALUES_2.md` item 5, from Murray
//! & Dermott 1999, Solar System Dynamics, section 7.7, as reproduced in Camargo, Winter & Foryta 2018,
//! arXiv:1806.03122, Eqs. 1 to 3, with the standard companion `B` result). Writing
//! `f_jk = (n_j / 4) [m_k / (m_c + m_j)] alpha_jk alphabar_jk` for the shared positive prefactor:
//!   `A_jj = + sum_{k != j} f_jk b_{3/2}^{(1)}(alpha_jk)` (diagonal positive, first Laplace coefficient),
//!   `A_jk = -            f_jk b_{3/2}^{(2)}(alpha_jk)` (off-diagonal negative, second Laplace coefficient),
//!   `B_jj = - sum_{k != j} f_jk b_{3/2}^{(1)}(alpha_jk)` (diagonal negative, first coefficient),
//!   `B_jk = +            f_jk b_{3/2}^{(1)}(alpha_jk)` (off-diagonal positive, first coefficient).
//! So `A_jj = -B_jj`, while the off-diagonals differ in sign and in which coefficient enters (`b^{(2)}` for
//! `A_jk`, `b^{(1)}` for `B_jk`). The alpha convention (Camargo et al., verbatim): for the ordered pair
//! `(j, k)` with `j` the inner body (`a_j < a_k`), `alpha_jk = alphabar_jk = a_j / a_k`; with `j` the outer
//! body (`a_j > a_k`), `alpha_jk = a_k / a_j` and `alphabar_jk = 1`. The argument is always below one. The
//! sign conventions are the classic hand-built Laplace-Lagrange defect and are taken exactly as fetched.
//!
//! THE WORKING UNIT. The `A` and `B` elements carry the unit of the mean motion. This module works the whole
//! eigenproblem in ARCSECONDS PER YEAR, the physics-natural scale at which the secular frequencies of a
//! planetary system are order one to a few tens (the solar-system `g` and `s` sit near 3 to 30 arcsec/yr),
//! so every matrix entry and every eigenvalue lands in the high-resolution middle of the Q32.32 grid rather
//! than near its `~2.3e-10` floor (the same range discipline the isolation mass uses). The derived mode
//! periods are then `1296000 / |frequency|` years, the kyr-to-Myr Milankovitch band.
//!
//! THE INPUT CONTRACT. [`secular_spectrum`] consumes a slice of [`SecularBody`] (mass in Earth masses, orbit
//! in AU, post-assembly eccentricity) plus the central-star mass in solar masses. This is exactly the data
//! `planetary_assembly::SystemPlanet { orbit_au, mass_earth }` carries, so the assembly maps in one to one
//! (`SecularBody::circular(p.mass_earth, p.orbit_au)`, the assembly leaving orbits circular at this pass, so
//! the eccentricity is zero); the module is kept off the `planetary_assembly` file (an impacts agent owns
//! it) and reads its contract rather than its internals.
//!
//! THE HONEST LIMITS. The eigen-FREQUENCIES (the `g` and `s` mode table this exports as the climate forcing)
//! depend only on the masses and the semimajor axes, so they are derived in full from the assembly's output.
//! The mode AMPLITUDES need the post-assembly eccentricity and inclination vectors projected onto the
//! eigenvectors; the present assembly leaves the system circular and coplanar, so those amplitudes are
//! identically zero until a fragmentation pass carries non-zero `e` and `i`, and this module exports the
//! frequency table (the load-bearing forcing spectrum) with the amplitude projection named as the follow-on.
//! The validity domain is the secular one: it excludes mean-motion-resonance proximity (flagged here, mapped
//! authoritatively by the assembly's stability object when that is wired) and degrades as a pair approaches
//! `alpha -> 1` (the Laplace series then needs more terms; such tight pairs are near-resonant and flagged).

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;

use crate::astro::{ASTRONOMICAL_UNIT_M, SOLAR_MASS_KG};

/// Earth's mass in kilograms, the IAU 2015 Resolution B3 nominal terrestrial mass (`M_earth = 5.9722e24`
/// kg, from the nominal `GM_earth` and the CODATA `G`). A cited REFERENCE ANCHOR paired with the solar mass
/// ([`SOLAR_MASS_KG`]) to form the Earth-to-sun mass ratio the matrix elements need, never a per-world value.
pub const EARTH_MASS_KG: &str = "5.9722e24";

/// The Julian year in seconds (`365.25 * 86400 = 31557600`), the IAU definitional year. A unit-conversion
/// constant (exact by definition), used only to express the speed of light in AU per year.
pub const SECONDS_PER_JULIAN_YEAR: &str = "31557600";

/// Arcseconds in a full turn (`360 * 3600 = 1296000`), used to turn a frequency in radians per year into
/// arcseconds per year and to turn a frequency back into a period in years. Exact by definition.
const ARCSEC_PER_TURN: i32 = 1_296_000;

/// One planet handed to the secular solver: the projection of `planetary_assembly::SystemPlanet` the theory
/// reads. The mass and the semimajor axis set the eigenfrequencies; the eccentricity enters only the
/// general-relativistic pericenter correction (and, in the follow-on, the mode amplitudes).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SecularBody {
    /// The mass in Earth masses (the assembly's `mass_earth`).
    pub mass_earth: Fixed,
    /// The semimajor axis in AU (the assembly's `orbit_au`).
    pub orbit_au: Fixed,
    /// The post-assembly eccentricity (dimensionless). The giant-impact assembly leaves orbits circular at
    /// this pass, so the assembly adapter supplies zero; a hindcast row (Mercury) supplies the cited value.
    pub ecc: Fixed,
}

impl SecularBody {
    /// A body on a circular orbit, the assembly's output contract (`e = 0`). This is the one-to-one adapter
    /// from `planetary_assembly::SystemPlanet { mass_earth, orbit_au }`.
    pub fn circular(mass_earth: Fixed, orbit_au: Fixed) -> Self {
        SecularBody {
            mass_earth,
            orbit_au,
            ecc: Fixed::ZERO,
        }
    }
}

/// Which family a mode belongs to: the eccentricity/pericenter `g` modes (from `A`) or the inclination/node
/// `s` modes (from `B`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModeFamily {
    /// The eccentricity / longitude-of-pericenter modes (`A` matrix eigenvalues, the `g` frequencies).
    Eccentricity,
    /// The inclination / longitude-of-ascending-node modes (`B` matrix eigenvalues, the `s` frequencies).
    Inclination,
}

/// One secular mode: a derived eigenfrequency and its period. The frequency is in arcseconds per year, the
/// period in years (`1296000 / |frequency|`, zero when the frequency is below the resolvable floor, which is
/// the physical secular-constant / invariable-plane mode).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SecularMode {
    /// The mode's index in the sorted spectrum.
    pub index: usize,
    /// The secular eigenfrequency in arcseconds per year (real, by the symmetric fold).
    pub frequency_arcsec_per_yr: Fixed,
    /// The secular period in years, `1296000 / |frequency|`. Zero flags a frequency at or below the
    /// resolvable floor (the invariable-plane node mode, or a mode whose period exceeds the representable
    /// range): a secular constant the climate module reads as no periodic forcing on this mode.
    pub period_years: Fixed,
}

/// A Gap-Law flag on the spectrum: a condition the secular theory CARRIES and surfaces rather than asserts,
/// so a downstream reader escalates rather than trusting a value in a regime where the theory thins.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GapFlag {
    /// Two eigenfrequencies of the same family sit within the near-degeneracy fraction of one another. The
    /// secular solution mixes the two modes and the split is at the edge of the fixed-point resolution;
    /// carried, not asserted.
    NearDegenerate {
        /// The family the two near-degenerate modes belong to.
        family: ModeFamily,
        /// The lower mode index.
        a: usize,
        /// The higher mode index.
        b: usize,
        /// The absolute separation in arcseconds per year.
        separation_arcsec_per_yr: Fixed,
    },
    /// An eccentricity frequency and an inclination frequency (or two of one family) nearly cancel, a
    /// low-order secular resonance whose argument librates slowly; carried, not asserted.
    SecularResonance {
        /// The eccentricity `g` mode index.
        g_index: usize,
        /// The inclination `s` mode index.
        s_index: usize,
        /// The near-zero combination `g - s` in arcseconds per year.
        combination_arcsec_per_yr: Fixed,
    },
    /// An adjacent pair sits near a low-order mean-motion commensurability. The Laplace-Lagrange secular
    /// theory is OUTSIDE its validity domain there (the assembly's stability object maps this
    /// authoritatively); carried as an escalation, not silently trusted.
    MmrProximity {
        /// The inner planet index.
        inner: usize,
        /// The outer planet index.
        outer: usize,
        /// The commensurability numerator `p` (outer:inner period near `p:q`).
        order_p: u32,
        /// The commensurability denominator `q`.
        order_q: u32,
        /// The fractional distance of the period ratio from the exact commensurability.
        residual: Fixed,
    },
    /// A planet's orbital speed trips the relativity smallness flag, so its general-relativistic pericenter
    /// precession is a material fraction of its secular rate. The correction is ALWAYS added to the `A`
    /// diagonal (it is derived); this flag reports that it matters for this planet.
    RelativitySignificant {
        /// The planet index.
        planet: usize,
        /// The smallness parameter `(v/c)^2` (dimensionless).
        smallness: Fixed,
        /// The general-relativistic pericenter precession added to the diagonal, in arcseconds per year.
        gr_arcsec_per_yr: Fixed,
    },
}

/// The derived secular spectrum: the archive object a climate / Milankovitch module reads. The `g` modes are
/// the eccentricity/pericenter frequencies, the `s` modes the inclination/node frequencies, each with its
/// period; the Gap-Law flags carry the regimes where the theory thins; the per-planet GR corrections and the
/// Jacobi diagnostics record how the spectrum was produced.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SecularSpectrum {
    /// The eccentricity / pericenter modes (`A` eigenvalues), sorted ascending by frequency.
    pub g_modes: Vec<SecularMode>,
    /// The inclination / node modes (`B` eigenvalues), sorted ascending by frequency.
    pub s_modes: Vec<SecularMode>,
    /// The Gap-Law flags carried on the spectrum.
    pub gap_flags: Vec<GapFlag>,
    /// The general-relativistic pericenter precession added to each planet's `A` diagonal, in arcseconds per
    /// year, indexed by input planet order.
    pub gr_corrections_arcsec_per_yr: Vec<Fixed>,
    /// The Jacobi sweeps the `A` eigensolve ran (a determinism / accuracy bound).
    pub jacobi_sweeps_g: u32,
    /// The Jacobi sweeps the `B` eigensolve ran.
    pub jacobi_sweeps_s: u32,
    /// The `A` off-diagonal residual (root of the sum of squares) at the last sweep, in arcseconds per year.
    pub offdiag_residual_g: Fixed,
    /// The `B` off-diagonal residual at the last sweep.
    pub offdiag_residual_s: Fixed,
}

/// The reserved calibration the secular solver needs. Each field is a numerical or diagnostic bound, not a
/// physics-floor value; the physical constants the solver USES (the speed of light, the Earth and solar
/// masses, the astronomical unit, pi) are cited anchors, not held here. The values below are DEV FIXTURES
/// for exercising and testing the mechanism; every one is reserved for the owner's calibration with the
/// basis noted at its field, and reaches a production run through the calibration manifest, never inline.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SecularCalib {
    /// The maximum Laplace-coefficient series terms. Basis: the series converges geometrically in
    /// `alpha^2`, so the term count needed for fixed-point accuracy rises as `alpha -> 1`; this is a
    /// per-call budget / accuracy bound, the point past which a tighter pair is near-resonant and flagged.
    pub laplace_max_iters: u32,
    /// The Laplace-series term-size cutoff: iteration stops once a term falls below this. Basis: the
    /// fixed-point resolution, the magnitude below which a further term cannot move the accumulated value.
    pub laplace_term_epsilon: Fixed,
    /// The maximum Jacobi sweeps. Basis: cyclic Jacobi converges quadratically, so a small fixed number of
    /// sweeps drives the off-diagonal below the fixed-point noise for the target system size `N`; a
    /// determinism / accuracy bound set by that `N`.
    pub jacobi_max_sweeps: u32,
    /// The Jacobi off-diagonal early-exit tolerance (root sum of squares, arcsec/yr). Basis: the
    /// fixed-point noise floor of the accumulated rotations; below it the off-diagonal is at the grid.
    pub jacobi_offdiag_epsilon: Fixed,
    /// The near-degeneracy fraction: two same-family frequencies within this fraction of the larger are
    /// flagged near-degenerate. Basis: the eigensolver's fixed-point resolution and the secular libration
    /// width, the fractional split below which the two modes are not cleanly separated.
    pub near_degeneracy_fraction: Fixed,
    /// The secular-resonance cutoff (arcsec/yr): a `g - s` combination smaller than this is flagged a
    /// resonance. Basis: the libration width of the secular argument, the near-zero band within which the
    /// argument circulates slowly enough to matter.
    pub secular_resonance_epsilon: Fixed,
    /// The largest commensurability order `p + q` scanned for mean-motion-resonance proximity. Basis: the
    /// order to which the resonance-overlap width the assembly's stability object maps stays material.
    pub mmr_max_order: u32,
    /// The mean-motion-resonance proximity fraction: a period ratio within this fraction of a low-order
    /// commensurability is flagged outside the validity domain. Basis: the resonance-overlap half-width the
    /// assembly's stability object already computes, matched to it for consistency.
    pub mmr_proximity_fraction: Fixed,
    /// The relativity smallness flag: a planet whose `(v/c)^2` exceeds this has its GR pericenter precession
    /// flagged material. Basis: the fraction of the secular pericenter rate GR contributes, above which it
    /// is reported significant (the correction is added regardless; this only tags it).
    pub relativity_smallness_flag: Fixed,
}

impl SecularCalib {
    /// DEV FIXTURES (not canon): values that exercise the mechanism in tests and demonstrations. Every field
    /// is reserved for the owner's calibration (see the field docs for each basis); these defaults are the
    /// development-profile scaffolding, exactly as `run_world` loads dev fixtures, never the owner's numbers.
    pub fn dev_fixtures() -> Self {
        SecularCalib {
            // Ample for alpha up to ~0.9 at the fixed-point grid; a tighter pair is flagged near-resonant.
            laplace_max_iters: 512,
            // One part in ~1e9, a few grid units on an order-one term.
            laplace_term_epsilon: Fixed::from_ratio(1, 1_000_000_000),
            // Cyclic Jacobi reaches the grid in well under this for N up to a few tens.
            jacobi_max_sweeps: 60,
            // A few grid units on an arcsec/yr matrix; the off-diagonal is at the fixed-point noise there.
            jacobi_offdiag_epsilon: Fixed::from_ratio(1, 1_000_000),
            // One part in a thousand: a split tighter than this is at the resolution edge.
            near_degeneracy_fraction: Fixed::from_ratio(1, 1000),
            // A hundredth of an arcsec/yr near-cancellation is a slow secular argument.
            secular_resonance_epsilon: Fixed::from_ratio(1, 100),
            // Scan commensurabilities to fifth order.
            mmr_max_order: 5,
            // Within a percent of an exact period commensurability is resonance-adjacent.
            mmr_proximity_fraction: Fixed::from_ratio(1, 100),
            // `(v/c)^2` above ~1e-8 (Mercury sits at ~2.5e-8) is a material GR precession.
            relativity_smallness_flag: Fixed::from_ratio(1, 100_000_000),
        }
    }
}

/// Round a wide rational to the Q32.32 grid (the astro.rs idiom), or `None` on out-of-range.
fn fixed_from_bigrat(r: &BigRat) -> Option<Fixed> {
    Fixed::from_bits_i128(r.round_to_scale(Fixed::FRAC_BITS)?)
}

/// The Earth-to-sun mass ratio (`M_earth / M_sun ~ 3.003e-6`), computed once in exact rational arithmetic
/// from the two cited anchors, then rounded to the grid. The dimensionless bridge from Earth masses to the
/// solar-mass units the mass ratios `m_k / (m_c + m_j)` are formed in.
fn earth_to_sun_mass_ratio() -> Option<Fixed> {
    let earth = BigRat::from_decimal_str(EARTH_MASS_KG).ok()?;
    let sun = BigRat::from_decimal_str(SOLAR_MASS_KG).ok()?;
    fixed_from_bigrat(&earth.div(&sun))
}

/// The speed of light in AU per year (`~63241`), from the CODATA `c`, the Julian year, and the astronomical
/// unit, computed once in exact rational arithmetic. Used by the derived GR precession; `c^2` overflows the
/// grid, so the caller divides by `c` twice rather than squaring it.
fn speed_of_light_au_per_yr() -> Option<Fixed> {
    let c = BigRat::from_decimal_str(civsim_units::fundamentals::SPEED_OF_LIGHT.value).ok()?;
    let year = BigRat::from_decimal_str(SECONDS_PER_JULIAN_YEAR).ok()?;
    let au = BigRat::from_decimal_str(ASTRONOMICAL_UNIT_M).ok()?;
    fixed_from_bigrat(&c.mul(&year).div(&au))
}

/// Arcseconds per radian (`648000 / pi ~ 206264.8`), from the pinned deterministic pi.
fn arcsec_per_rad() -> Fixed {
    Fixed::from_int(ARCSEC_PER_TURN / 2)
        .checked_div(Fixed::PI)
        .unwrap()
}

/// A Laplace coefficient `b_s^{(j)}(alpha) = (1/pi) integral_0^{2pi} cos(j psi) [1 - 2 alpha cos psi +
/// alpha^2]^{-s} d psi`, computed by its convergent hypergeometric series (Murray & Dermott 1999 Eq. 6.68 /
/// Brouwer & Clemence): with `x = alpha^2`,
///   `b_s^{(j)}(alpha) = C_j alpha^j sum_{m>=0} t_m`, `t_0 = 1`,
///   `t_{m+1} = t_m [(s + m)(s + j + m) / ((j + 1 + m)(1 + m))] x`, `C_j = 2 (s)_j / j!`,
/// where `(s)_j = s (s+1) ... (s+j-1)` is the rising factorial. The series converges for `alpha < 1`,
/// geometrically in `x = alpha^2` asymptotically, so it is slowest as `alpha -> 1`; iteration stops when a
/// term falls below `calib.laplace_term_epsilon` or after `calib.laplace_max_iters` terms (both reserved).
/// `None` on `alpha >= 1` (the argument must be below one, the convention guarantees this) or an overflow.
fn laplace_coefficient(s: Fixed, j: u32, alpha: Fixed, calib: &SecularCalib) -> Option<Fixed> {
    if alpha <= Fixed::ZERO || alpha >= Fixed::ONE {
        return None;
    }
    let x = alpha.checked_mul(alpha)?; // alpha^2

    // Leading coefficient C_j = 2 (s)_j / j!.
    let mut c_j = Fixed::from_int(2);
    let mut factorial: i64 = 1;
    for i in 0..j {
        c_j = c_j.checked_mul(s.checked_add(Fixed::from_int(i as i32))?)?; // multiply by (s + i)
        factorial = factorial.checked_mul((i as i64) + 1)?;
    }
    c_j = c_j.checked_div(Fixed::from_int(factorial as i32))?;

    // alpha^j.
    let mut alpha_pow = Fixed::ONE;
    for _ in 0..j {
        alpha_pow = alpha_pow.checked_mul(alpha)?;
    }
    let lead = c_j.checked_mul(alpha_pow)?;

    // The hypergeometric sum sum_{m>=0} t_m, t_0 = 1.
    let mut sum = Fixed::ONE;
    let mut term = Fixed::ONE;
    let j_fx = Fixed::from_int(j as i32);
    for m in 0..calib.laplace_max_iters {
        let m_fx = Fixed::from_int(m as i32);
        let num = s
            .checked_add(m_fx)?
            .checked_mul(s.checked_add(j_fx)?.checked_add(m_fx)?)?; // (s+m)(s+j+m)
        let den = j_fx
            .checked_add(Fixed::ONE)?
            .checked_add(m_fx)?
            .checked_mul(Fixed::ONE.checked_add(m_fx)?)?; // (j+1+m)(1+m)
        term = term.checked_mul(num)?.checked_div(den)?.checked_mul(x)?;
        sum = sum.checked_add(term)?;
        if term.abs() < calib.laplace_term_epsilon {
            break;
        }
    }
    lead.checked_mul(sum)
}

/// The mean motion in radians per year, `n = 2 pi sqrt(M_total / a^3)` with the total mass in solar masses
/// and the semimajor axis in AU (Kepler's third law in astronomical units: `n^2 a^3 = G M_total` with
/// `G M_sun = 4 pi^2 AU^3 / yr^2`). `None` on a non-positive semimajor axis or an overflow.
fn mean_motion_rad_per_yr(orbit_au: Fixed, total_mass_solar: Fixed) -> Option<Fixed> {
    if orbit_au <= Fixed::ZERO || total_mass_solar <= Fixed::ZERO {
        return None;
    }
    let a3 = orbit_au.checked_mul(orbit_au)?.checked_mul(orbit_au)?;
    let ratio = total_mass_solar.checked_div(a3)?;
    let two_pi = Fixed::PI.checked_mul(Fixed::from_int(2))?;
    two_pi.checked_mul(ratio.sqrt())
}

/// The general-relativistic pericenter precession `d(pomega)/dt = 3 n^3 a^2 / (c^2 (1 - e^2))`, in
/// arcseconds per year, and the smallness parameter `(v/c)^2 = (n a / c)^2`. Derived from the orbital
/// elements alone (Murray & Dermott 1999 section 7 / the standard first-post-Newtonian result); for Mercury
/// (`a = 0.3871` AU, `e = 0.2056`) this returns about 43 arcsec per century, the classic test. To keep every
/// intermediate on the Q32.32 grid the factor `n^3 a^2` is formed as `n (n a)^2` (a moderate speed squared,
/// never a large cube) and the division by the ~63241 AU/yr light speed is done TWICE rather than by a `c^2`
/// that overflows the grid. `None` on `e >= 1` or an overflow (an extreme close-in body, escalated).
fn gr_precession(
    n_rad: Fixed,
    a_au: Fixed,
    ecc: Fixed,
    c_au_yr: Fixed,
    arcsec_per_rad: Fixed,
) -> Option<(Fixed, Fixed)> {
    let one_minus_e2 = Fixed::ONE.checked_sub(ecc.checked_mul(ecc)?)?;
    if one_minus_e2 <= Fixed::ZERO {
        return None;
    }
    let v = n_rad.checked_mul(a_au)?; // orbital speed in AU/yr
    let v2 = v.checked_mul(v)?;
    // 3 n v^2 / (1 - e^2), then / c / c: rate in radians per year.
    let rate_rad = Fixed::from_int(3)
        .checked_mul(n_rad)?
        .checked_mul(v2)?
        .checked_div(one_minus_e2)?
        .checked_div(c_au_yr)?
        .checked_div(c_au_yr)?;
    let gr_arcsec = rate_rad.checked_mul(arcsec_per_rad)?;
    // Smallness (v/c)^2 = (v/c)(v/c), formed by dividing before squaring so it never overflows.
    let v_over_c = v.checked_div(c_au_yr)?;
    let smallness = v_over_c.checked_mul(v_over_c)?;
    Some((gr_arcsec, smallness))
}

/// The `A` (eccentricity) and `B` (inclination) matrices in arcseconds per year, the per-planet mean motions
/// in radians per year, the per-planet GR corrections (already folded into the `A` diagonal), and the
/// smallness parameters. Built strictly from the fetched Murray & Dermott element definitions.
#[allow(clippy::type_complexity)]
fn build_matrices(
    bodies: &[SecularBody],
    star_mass_solar: Fixed,
    calib: &SecularCalib,
) -> Option<(
    Vec<Vec<Fixed>>,
    Vec<Vec<Fixed>>,
    Vec<Fixed>,
    Vec<Fixed>,
    Vec<Fixed>,
)> {
    let n = bodies.len();
    let e2s = earth_to_sun_mass_ratio()?;
    let c_au_yr = speed_of_light_au_per_yr()?;
    let apr = arcsec_per_rad();
    let s = Fixed::from_ratio(3, 2); // the secular theory's s = 3/2

    // Per-planet mass in solar units and mean motion (rad/yr, then arcsec/yr for the matrix prefactor).
    let mut mass_solar = Vec::with_capacity(n);
    let mut n_rad = Vec::with_capacity(n);
    let mut n_asec = Vec::with_capacity(n);
    for b in bodies {
        let m = b.mass_earth.checked_mul(e2s)?;
        let nr = mean_motion_rad_per_yr(b.orbit_au, star_mass_solar.checked_add(m)?)?;
        mass_solar.push(m);
        n_asec.push(nr.checked_mul(apr)?);
        n_rad.push(nr);
    }

    let mut a_mat = vec![vec![Fixed::ZERO; n]; n];
    let mut b_mat = vec![vec![Fixed::ZERO; n]; n];
    for j in 0..n {
        let prefactor = n_asec[j].checked_div(Fixed::from_int(4))?; // n_j / 4 in arcsec/yr
        for k in 0..n {
            if k == j {
                continue;
            }
            let (aj, ak) = (bodies[j].orbit_au, bodies[k].orbit_au);
            if aj == ak {
                return None; // degenerate orbits: outside the secular ordering
            }
            // The Camargo alpha / alphabar convention (verbatim): j inner vs j outer.
            let (alpha, alphabar) = if aj < ak {
                (aj.checked_div(ak)?, aj.checked_div(ak)?)
            } else {
                (ak.checked_div(aj)?, Fixed::ONE)
            };
            // f_jk = (n_j / 4) [m_k / (m_c + m_j)] alpha alphabar, the shared positive prefactor.
            let mass_ratio =
                mass_solar[k].checked_div(star_mass_solar.checked_add(mass_solar[j])?)?;
            let f = prefactor
                .checked_mul(mass_ratio)?
                .checked_mul(alpha)?
                .checked_mul(alphabar)?;
            let b1 = laplace_coefficient(s, 1, alpha, calib)?;
            let b2 = laplace_coefficient(s, 2, alpha, calib)?;
            let f_b1 = f.checked_mul(b1)?;
            let f_b2 = f.checked_mul(b2)?;
            // A_jj = + sum f_jk b^(1); A_jk = - f_jk b^(2).
            a_mat[j][j] = a_mat[j][j].checked_add(f_b1)?;
            a_mat[j][k] = Fixed::ZERO.checked_sub(f_b2)?;
            // B_jj = - sum f_jk b^(1); B_jk = + f_jk b^(1).
            b_mat[j][j] = b_mat[j][j].checked_sub(f_b1)?;
            b_mat[j][k] = f_b1;
        }
    }

    // The derived GR pericenter correction joins the A diagonal (pericenter only; the node B is untouched at
    // this order). Always added (it is derived); the smallness parameter drives the significance flag.
    let mut gr = Vec::with_capacity(n);
    let mut smallness = Vec::with_capacity(n);
    for j in 0..n {
        let (gr_asec, small) =
            gr_precession(n_rad[j], bodies[j].orbit_au, bodies[j].ecc, c_au_yr, apr)?;
        a_mat[j][j] = a_mat[j][j].checked_add(gr_asec)?;
        gr.push(gr_asec);
        smallness.push(small);
    }

    Some((a_mat, b_mat, n_rad, gr, smallness))
}

/// The mass- and frequency-weighted diagonal similarity that renders a Laplace-Lagrange matrix REAL
/// SYMMETRIC. Scaling row and column `j` by `d_j = sqrt(Lambda_j)` with the circular Poincare action
/// `Lambda_j = m_j n_j a_j^2` (here `m_j` in Earth masses, since the common Earth-to-sun factor cancels in
/// every ratio `d_j / d_k`, keeping the weights order one) turns `M` into `M_sym[j][k] = M[j][k] d_j / d_k`,
/// which is symmetric because the fetched element definitions give `M[j][k] d_k^2 = M[k][j] d_j^2` exactly.
/// A similarity transform preserves the eigenvalues, so the symmetric matrix has the same (now provably
/// real) spectrum as `M`. `None` on a non-positive weight or an overflow.
fn symmetrize(
    m: &[Vec<Fixed>],
    bodies: &[SecularBody],
    n_rad: &[Fixed],
) -> Option<Vec<Vec<Fixed>>> {
    let n = m.len();
    let mut d = Vec::with_capacity(n);
    for j in 0..n {
        let a2 = bodies[j].orbit_au.checked_mul(bodies[j].orbit_au)?;
        let lambda = bodies[j]
            .mass_earth
            .checked_mul(n_rad[j])?
            .checked_mul(a2)?;
        if lambda <= Fixed::ZERO {
            return None;
        }
        d.push(lambda.sqrt());
    }
    let mut sym = vec![vec![Fixed::ZERO; n]; n];
    for j in 0..n {
        for k in 0..n {
            sym[j][k] = m[j][k].checked_mul(d[j])?.checked_div(d[k])?;
        }
    }
    Some(sym)
}

/// The root of the sum of squares of the strict upper off-diagonal, the Jacobi convergence measure.
fn off_diagonal_norm(m: &[Vec<Fixed>]) -> Fixed {
    let mut acc = Fixed::ZERO;
    for (p, row) in m.iter().enumerate() {
        for &val in row.iter().skip(p + 1) {
            acc += val.checked_mul(val).unwrap_or(Fixed::MAX);
        }
    }
    acc.sqrt()
}

/// A deterministic real-symmetric eigenvalue solve by CYCLIC JACOBI rotations. Each rotation zeroes one
/// off-diagonal `(p, q)` through the numerically stable angle `t = tan(theta) = sign(theta) / (|theta| +
/// sqrt(theta^2 + 1))`, `theta = (a_qq - a_pp) / (2 a_pq)`, then `c = 1 / sqrt(1 + t^2)`, `s = t c`, so the
/// solve uses only square roots and divisions (no trigonometric series) and is bit-identical on every
/// machine. When `|theta|` would overflow the grid on squaring, the asymptotic `t = 1 / (2 theta)` is used
/// (the far tail of the same formula). Sweeps run in fixed ascending `(p, q)` order, stopping when the
/// off-diagonal norm falls below `calib.jacobi_offdiag_epsilon` or after `calib.jacobi_max_sweeps` sweeps
/// (both reserved: the sweep count is the determinism / accuracy bound). Returns the diagonal (the
/// eigenvalues), the sweeps run, and the final off-diagonal residual. `None` on an overflow.
fn jacobi_eigenvalues(
    matrix: &[Vec<Fixed>],
    calib: &SecularCalib,
) -> Option<(Vec<Fixed>, u32, Fixed)> {
    let n = matrix.len();
    let mut a: Vec<Vec<Fixed>> = matrix.to_vec();
    let two = Fixed::from_int(2);
    let mut sweeps = 0u32;
    let mut residual = off_diagonal_norm(&a);
    for _ in 0..calib.jacobi_max_sweeps {
        if residual < calib.jacobi_offdiag_epsilon {
            break;
        }
        sweeps += 1;
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = a[p][q];
                if apq == Fixed::ZERO {
                    continue;
                }
                // theta = (a_qq - a_pp) / (2 a_pq). When `a_pq` is a grid-level residual against the diagonal
                // gap this quotient leaves the grid: the rotation angle is then ~0, so the element is zeroed
                // directly (the exact tiny rotation to first order, its diagonal correction below the grid).
                let theta = match a[q][q]
                    .checked_sub(a[p][p])
                    .and_then(|gap| two.checked_mul(apq).and_then(|den| gap.checked_div(den)))
                {
                    Some(theta) => theta,
                    None => {
                        a[p][q] = Fixed::ZERO;
                        a[q][p] = Fixed::ZERO;
                        continue;
                    }
                };
                let sign = if theta.to_bits() >= 0 {
                    Fixed::ONE
                } else {
                    Fixed::ZERO.checked_sub(Fixed::ONE)?
                };
                let abs_theta = theta.abs();
                // t = sign / (|theta| + sqrt(theta^2 + 1)); the asymptote t = 1/(2 theta) when theta^2
                // would leave the grid (a well-separated pair, a near-diagonal off-element).
                let t = match theta.checked_mul(theta) {
                    Some(theta2) => {
                        let denom =
                            abs_theta.checked_add(theta2.checked_add(Fixed::ONE)?.sqrt())?;
                        sign.checked_div(denom)?
                    }
                    // `|theta|` is so large that `theta^2` leaves the grid: the off-element is negligible
                    // against the diagonal gap, so the rotation is `t = 1 / (2 theta) ~ 0`. If even `2|theta|`
                    // leaves the grid the rotation is below the grid entirely, so `t = 0` (a direct zeroing of
                    // the residual off-element, the exact tiny rotation to first order).
                    None => match two.checked_mul(abs_theta) {
                        Some(denom) => sign.checked_div(denom)?,
                        None => Fixed::ZERO,
                    },
                };
                let c =
                    Fixed::ONE.checked_div(Fixed::ONE.checked_add(t.checked_mul(t)?)?.sqrt())?;
                let s = t.checked_mul(c)?;
                // Apply the rotation to rows and columns p, q (the matrix stays symmetric).
                let app = a[p][p];
                let aqq = a[q][q];
                let c2 = c.checked_mul(c)?;
                let s2 = s.checked_mul(s)?;
                let two_sc_apq = two.checked_mul(s)?.checked_mul(c)?.checked_mul(apq)?;
                a[p][p] = c2
                    .checked_mul(app)?
                    .checked_sub(two_sc_apq)?
                    .checked_add(s2.checked_mul(aqq)?)?;
                a[q][q] = s2
                    .checked_mul(app)?
                    .checked_add(two_sc_apq)?
                    .checked_add(c2.checked_mul(aqq)?)?;
                a[p][q] = Fixed::ZERO;
                a[q][p] = Fixed::ZERO;
                // The rotation touches rows and columns p and q of every other index, so the index into
                // several rows and columns at once is the clear form here.
                #[allow(clippy::needless_range_loop)]
                for i in 0..n {
                    if i == p || i == q {
                        continue;
                    }
                    let aip = a[i][p];
                    let aiq = a[i][q];
                    let new_ip = c.checked_mul(aip)?.checked_sub(s.checked_mul(aiq)?)?;
                    let new_iq = s.checked_mul(aip)?.checked_add(c.checked_mul(aiq)?)?;
                    a[i][p] = new_ip;
                    a[p][i] = new_ip;
                    a[i][q] = new_iq;
                    a[q][i] = new_iq;
                }
            }
        }
        residual = off_diagonal_norm(&a);
    }
    let eigenvalues = (0..n).map(|i| a[i][i]).collect();
    Some((eigenvalues, sweeps, residual))
}

/// Turn a sorted eigenvalue list (arcsec/yr) into the mode table, each mode carrying its period in years
/// (`1296000 / |frequency|`, zero when the frequency is at or below the resolvable floor, the secular
/// constant / invariable-plane case, or when the period would leave the grid).
fn modes_from_eigenvalues(mut eigenvalues: Vec<Fixed>, floor: Fixed) -> Vec<SecularMode> {
    eigenvalues.sort_by_key(|f| f.to_bits());
    let turn = Fixed::from_int(ARCSEC_PER_TURN);
    eigenvalues
        .into_iter()
        .enumerate()
        .map(|(index, frequency_arcsec_per_yr)| {
            let mag = frequency_arcsec_per_yr.abs();
            let period_years = if mag <= floor {
                Fixed::ZERO
            } else {
                turn.checked_div(mag).unwrap_or(Fixed::ZERO)
            };
            SecularMode {
                index,
                frequency_arcsec_per_yr,
                period_years,
            }
        })
        .collect()
}

/// Reduce a rational `outer / inner` period ratio toward a low-order commensurability, returning the closest
/// `(p, q, fractional residual)` with `p + q <= max_order` and `p > q >= 1`. Used only to FLAG
/// mean-motion-resonance proximity (outside the secular validity domain), never to assert a value.
fn nearest_commensurability(ratio: Fixed, max_order: u32) -> Option<(u32, u32, Fixed)> {
    let mut best: Option<(u32, u32, Fixed)> = None;
    for q in 1..max_order {
        for p in (q + 1)..=(max_order - q) {
            let target = Fixed::from_int(p as i32).checked_div(Fixed::from_int(q as i32))?;
            let residual = ratio
                .checked_sub(target)?
                .abs()
                .checked_div(target)
                .unwrap_or(Fixed::MAX);
            if best.map(|(_, _, r)| residual < r).unwrap_or(true) {
                best = Some((p, q, residual));
            }
        }
    }
    best
}

/// THE SECULAR SPECTRUM: from an assembled system derive the Laplace-Lagrange `g` (eccentricity) and `s`
/// (inclination) eigenfrequency tables, the archive object a climate / Milankovitch module reads. Builds the
/// `A` and `B` matrices from the fetched Murray & Dermott definitions (with the derived GR pericenter
/// correction on the `A` diagonal), folds each to real symmetric form by the mass- and frequency-weighted
/// similarity, solves each by deterministic Jacobi, and carries the Gap-Law flags. Returns `None` on fewer
/// than two bodies (no secular coupling), a non-positive star mass, or a numerical overflow (fail-loud).
pub fn secular_spectrum(
    bodies: &[SecularBody],
    star_mass_solar: Fixed,
    calib: &SecularCalib,
) -> Option<SecularSpectrum> {
    if bodies.len() < 2 || star_mass_solar <= Fixed::ZERO {
        return None;
    }

    let (a_mat, b_mat, n_rad, gr, smallness) = build_matrices(bodies, star_mass_solar, calib)?;
    let a_sym = symmetrize(&a_mat, bodies, &n_rad)?;
    let b_sym = symmetrize(&b_mat, bodies, &n_rad)?;
    let (g_vals, sweeps_g, resid_g) = jacobi_eigenvalues(&a_sym, calib)?;
    let (s_vals, sweeps_s, resid_s) = jacobi_eigenvalues(&b_sym, calib)?;

    let g_modes = modes_from_eigenvalues(g_vals, calib.jacobi_offdiag_epsilon);
    let s_modes = modes_from_eigenvalues(s_vals, calib.jacobi_offdiag_epsilon);

    let mut gap_flags = Vec::new();

    // Near-degeneracy within each family (carry, not assert).
    for (family, modes) in [
        (ModeFamily::Eccentricity, &g_modes),
        (ModeFamily::Inclination, &s_modes),
    ] {
        for a in 0..modes.len() {
            for b in (a + 1)..modes.len() {
                let fa = modes[a].frequency_arcsec_per_yr;
                let fb = modes[b].frequency_arcsec_per_yr;
                let sep = fb.checked_sub(fa).unwrap_or(Fixed::MAX).abs();
                let scale = fa.abs().max(fb.abs());
                let bound = scale
                    .checked_mul(calib.near_degeneracy_fraction)
                    .unwrap_or(Fixed::ZERO);
                if sep < bound {
                    gap_flags.push(GapFlag::NearDegenerate {
                        family,
                        a,
                        b,
                        separation_arcsec_per_yr: sep,
                    });
                }
            }
        }
    }

    // Secular resonance: a g and an s nearly cancel (carry).
    for (gi, gm) in g_modes.iter().enumerate() {
        for (si, sm) in s_modes.iter().enumerate() {
            let comb = gm
                .frequency_arcsec_per_yr
                .checked_sub(sm.frequency_arcsec_per_yr)
                .unwrap_or(Fixed::MAX);
            if comb.abs() < calib.secular_resonance_epsilon {
                gap_flags.push(GapFlag::SecularResonance {
                    g_index: gi,
                    s_index: si,
                    combination_arcsec_per_yr: comb,
                });
            }
        }
    }

    // Mean-motion-resonance proximity: outside the validity domain (carry an escalation). Adjacent pairs by
    // period ratio, using the mean motions already derived.
    let mut order: Vec<usize> = (0..bodies.len()).collect();
    order.sort_by(|&i, &j| {
        bodies[i]
            .orbit_au
            .to_bits()
            .cmp(&bodies[j].orbit_au.to_bits())
    });
    for w in order.windows(2) {
        let (inner, outer) = (w[0], w[1]);
        // Period ratio outer:inner = n_inner / n_outer (> 1).
        if let Some(ratio) = n_rad[inner].checked_div(n_rad[outer]) {
            if let Some((p, q, residual)) = nearest_commensurability(ratio, calib.mmr_max_order) {
                if residual < calib.mmr_proximity_fraction {
                    gap_flags.push(GapFlag::MmrProximity {
                        inner,
                        outer,
                        order_p: p,
                        order_q: q,
                        residual,
                    });
                }
            }
        }
    }

    // Relativity significance per planet (the correction is already in A; this tags it).
    for (planet, &small) in smallness.iter().enumerate() {
        if small > calib.relativity_smallness_flag {
            gap_flags.push(GapFlag::RelativitySignificant {
                planet,
                smallness: small,
                gr_arcsec_per_yr: gr[planet],
            });
        }
    }

    Some(SecularSpectrum {
        g_modes,
        s_modes,
        gap_flags,
        gr_corrections_arcsec_per_yr: gr,
        jacobi_sweeps_g: sweeps_g,
        jacobi_sweeps_s: sweeps_s,
        offdiag_residual_g: resid_g,
        offdiag_residual_s: resid_s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn laplace_coefficients_match_the_integral_quadrature() {
        // Reference values from a 1,000,000-point trapezoid of the integral definition
        // b_s^(m)(alpha) = (1/pi) int_0^2pi cos(m psi) (1 - 2 alpha cos psi + alpha^2)^(-s) dpsi.
        let calib = SecularCalib::dev_fixtures();
        let s = Fixed::from_ratio(3, 2);
        let cases = [
            (0.2_f64, 0.6477699752, 0.1611224948),
            (0.3, 1.0744568986, 0.3982561886),
            (0.5, 2.5805000300, 1.5580264438),
        ];
        for (alpha_f, b1_ref, b2_ref) in cases {
            let alpha = Fixed::from_decimal_str(&format!("{alpha_f}")).unwrap();
            let b1 = laplace_coefficient(s, 1, alpha, &calib).unwrap();
            let b2 = laplace_coefficient(s, 2, alpha, &calib).unwrap();
            assert!(
                approx(b1, b1_ref, 1e-6),
                "b_3/2^(1)({alpha_f}) = {} vs {b1_ref}",
                b1.to_f64_lossy()
            );
            assert!(
                approx(b2, b2_ref, 1e-6),
                "b_3/2^(2)({alpha_f}) = {} vs {b2_ref}",
                b2.to_f64_lossy()
            );
        }
    }

    #[test]
    fn two_planet_system_recovers_the_analytic_laplace_lagrange_frequencies() {
        // A Jupiter-Saturn pair (Earth masses, AU), one solar mass. The 2x2 A matrix has closed-form
        // eigenvalues; an independent computation gives g = 3.4392 and 21.6486 arcsec/yr (periods ~376830
        // and ~59865 yr, the secular / Milankovitch band). The full pipeline (build A -> symmetrize ->
        // Jacobi) must recover them, proving the mechanism on the small case.
        let calib = SecularCalib::dev_fixtures();
        let bodies = [
            SecularBody::circular(
                Fixed::from_decimal_str("317.83").unwrap(),
                Fixed::from_decimal_str("5.2044").unwrap(),
            ),
            SecularBody::circular(
                Fixed::from_decimal_str("95.16").unwrap(),
                Fixed::from_decimal_str("9.5826").unwrap(),
            ),
        ];
        let spec = secular_spectrum(&bodies, Fixed::ONE, &calib).unwrap();
        assert_eq!(spec.g_modes.len(), 2);
        let g0 = spec.g_modes[0].frequency_arcsec_per_yr;
        let g1 = spec.g_modes[1].frequency_arcsec_per_yr;
        assert!(
            approx(g0, 3.4392136817, 2e-3),
            "g0 = {} vs 3.4392",
            g0.to_f64_lossy()
        );
        assert!(
            approx(g1, 21.6486245701, 5e-3),
            "g1 = {} vs 21.6486",
            g1.to_f64_lossy()
        );
        // Periods land in the secular band.
        assert!(
            approx(spec.g_modes[1].period_years, 59865.0, 50.0),
            "g1 period = {} vs ~59865 yr",
            spec.g_modes[1].period_years.to_f64_lossy()
        );
    }

    #[test]
    fn the_inclination_matrix_carries_the_invariable_plane_zero_mode() {
        // B has row sums exactly zero (B_jj = - sum B_jk), so (1,...,1) is a null eigenvector: one s
        // frequency is exactly zero, the invariable-plane node mode (a secular constant, period reported
        // zero), and it is not a Gap-Law defect.
        let calib = SecularCalib::dev_fixtures();
        let bodies = [
            SecularBody::circular(
                Fixed::from_decimal_str("317.83").unwrap(),
                Fixed::from_decimal_str("5.2044").unwrap(),
            ),
            SecularBody::circular(
                Fixed::from_decimal_str("95.16").unwrap(),
                Fixed::from_decimal_str("9.5826").unwrap(),
            ),
        ];
        let spec = secular_spectrum(&bodies, Fixed::ONE, &calib).unwrap();
        // The zero mode is the smallest in magnitude (it need not sort first; the nodal modes regress, so
        // the non-zero s frequency is negative and sorts below the zero).
        let zero_mode = spec
            .s_modes
            .iter()
            .min_by_key(|m| m.frequency_arcsec_per_yr.abs().to_bits())
            .unwrap();
        assert!(
            zero_mode.frequency_arcsec_per_yr.abs().to_f64_lossy() < 1e-3,
            "the invariable-plane s mode is ~0, got {}",
            zero_mode.frequency_arcsec_per_yr.abs().to_f64_lossy()
        );
        assert_eq!(
            zero_mode.period_years,
            Fixed::ZERO,
            "the zero mode reports no periodic forcing"
        );
    }

    #[test]
    fn mercury_general_relativistic_precession_recovers_forty_three_arcsec_per_century() {
        // Mercury's orbital elements alone (a = 0.3871 AU, e = 0.2056, one solar mass, planet mass
        // negligible) drive the derived GR pericenter precession. The classic value is ~43 arcsec/century
        // (Murray & Dermott 1999 section 7; the observed anomalous advance is 42.98 +/- 0.04 arcsec/cy).
        let a = Fixed::from_decimal_str("0.3871").unwrap();
        let e = Fixed::from_decimal_str("0.2056").unwrap();
        let n_rad = mean_motion_rad_per_yr(a, Fixed::ONE).unwrap();
        let c = speed_of_light_au_per_yr().unwrap();
        let apr = arcsec_per_rad();
        let (gr_per_yr, smallness) = gr_precession(n_rad, a, e, c, apr).unwrap();
        let gr_per_century = gr_per_yr.to_f64_lossy() * 100.0;
        assert!(
            (gr_per_century - 42.98).abs() < 0.2,
            "Mercury GR precession = {gr_per_century} arcsec/century vs ~43"
        );
        // The smallness parameter (v/c)^2 ~ 2.5e-8 trips the relativity flag.
        assert!(
            smallness.to_f64_lossy() > 1e-8 && smallness.to_f64_lossy() < 1e-7,
            "(v/c)^2 = {} for Mercury",
            smallness.to_f64_lossy()
        );
    }

    #[test]
    fn a_close_in_planet_trips_the_relativity_flag_as_the_auto_case() {
        // The Mercury case handled by the general auto-flag: a close-in body carrying real eccentricity
        // surfaces a RelativitySignificant flag, not a special-cased branch.
        let calib = SecularCalib::dev_fixtures();
        let bodies = [
            SecularBody {
                mass_earth: Fixed::from_decimal_str("0.055").unwrap(),
                orbit_au: Fixed::from_decimal_str("0.3871").unwrap(),
                ecc: Fixed::from_decimal_str("0.2056").unwrap(),
            },
            SecularBody::circular(
                Fixed::from_decimal_str("1.0").unwrap(),
                Fixed::from_decimal_str("1.0").unwrap(),
            ),
        ];
        let spec = secular_spectrum(&bodies, Fixed::ONE, &calib).unwrap();
        assert!(
            spec.gap_flags
                .iter()
                .any(|f| matches!(f, GapFlag::RelativitySignificant { planet: 0, .. })),
            "the close-in body auto-trips the relativity flag: {:?}",
            spec.gap_flags
        );
    }

    #[test]
    fn the_frequencies_are_real_and_the_spectrum_is_bit_deterministic() {
        // The symmetric fold guarantees real eigenvalues (no complex arithmetic exists in the solve), and
        // the same system produces the same spectrum bit-for-bit (Principle 3): every value is Fixed, every
        // op deterministic, the sweep order fixed.
        let calib = SecularCalib::dev_fixtures();
        let bodies = [
            SecularBody::circular(
                Fixed::from_decimal_str("1.0").unwrap(),
                Fixed::from_decimal_str("1.0").unwrap(),
            ),
            SecularBody::circular(
                Fixed::from_decimal_str("0.815").unwrap(),
                Fixed::from_decimal_str("0.723").unwrap(),
            ),
            SecularBody::circular(
                Fixed::from_decimal_str("317.83").unwrap(),
                Fixed::from_decimal_str("5.2044").unwrap(),
            ),
        ];
        let a = secular_spectrum(&bodies, Fixed::ONE, &calib).unwrap();
        let b = secular_spectrum(&bodies, Fixed::ONE, &calib).unwrap();
        assert_eq!(a, b, "same system, same spectrum bit-for-bit");
        // Every frequency is finite and the solve converged well inside the sweep bound.
        for m in a.g_modes.iter().chain(a.s_modes.iter()) {
            assert!(m.frequency_arcsec_per_yr.abs() < Fixed::from_int(1_000_000));
        }
        assert!(a.jacobi_sweeps_g <= calib.jacobi_max_sweeps);
        assert!(a.jacobi_sweeps_s <= calib.jacobi_max_sweeps);
    }

    #[test]
    fn fewer_than_two_bodies_and_a_dead_star_fail_loud() {
        let calib = SecularCalib::dev_fixtures();
        let one = [SecularBody::circular(Fixed::ONE, Fixed::ONE)];
        assert!(secular_spectrum(&one, Fixed::ONE, &calib).is_none());
        let two = [
            SecularBody::circular(Fixed::ONE, Fixed::ONE),
            SecularBody::circular(Fixed::ONE, Fixed::from_int(2)),
        ];
        assert!(secular_spectrum(&two, Fixed::ZERO, &calib).is_none());
    }
}
