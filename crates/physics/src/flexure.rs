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

//! The FLEXURE KERNEL (surface pipeline, slice 5): the analytic elastic-plate flexure that fills the
//! wavelength band between the coarse convective provinces (~1000 km) and the fine crater rows, the band a
//! human reads as mountain belts, foreland basins, moats around volcanoes, and peripheral bulges
//! (`docs/working/FLEXURE_ARC_SCOPE.md`). It is a DORMANT physics module: pure, deterministic, fixed-point
//! functions with no run-path consumer yet (the render wiring is a later slice), so both run pins hold
//! trivially.
//!
//! # The mechanism (thin elastic plate over a fluid, derive-first, no free constants)
//!
//! A thin elastic plate over an inviscid fluid substrate obeys
//! `D grad^4(w) + (rho_mantle - rho_infill) g w = q(load)`, where `w` is the deflection, `q` the load, and `D`
//! the flexural rigidity. Everything on the left DERIVES; the kernel authors no physical value. It reads:
//!
//! - `E` (Young's modulus) and `nu` (Poisson's ratio), which the caller derives UPSTREAM from the banked
//!   material moduli by the standard isotropic-elastic relations `E = 9 K G / (3 K + G)` and
//!   `nu = (3 K - 2 G) / (2 (3 K + G))` (`civsim_materials::properties::youngs_modulus_gpa` and
//!   `poisson_ratio`), over the bulk modulus `K` (the ionic lattice-curvature tier
//!   [`crate::lattice_modulus::phase_bulk_modulus_ionic`], `B = (n-1) A / (18 r0^4)`) and the shear modulus
//!   `G = k K` (the Pugh chain `civsim_materials::properties::shear_modulus_gpa`, the same shear route the
//!   support-bound yield uses). Materials depends on physics, so this crate is UPSTREAM of that derivation and
//!   takes `E` and `nu` as inputs rather than importing the (downstream) relation.
//! - `T_e` (the elastic thickness), a PER-LOAD input the caller supplies. The kernel does not author it.
//!
//!   THIS LINE CARRIED A FALSE IMPLEMENTATION-STATUS CLAIM FOR FOUR GENERATIONS OF ITS OWN DESCENDANTS, and it
//!   is recorded rather than quietly swapped, because the claim propagated from here into two scope documents by
//!   transcription. It said `T_e` was supplied "from the deep-time thermal state", which asserted a DERIVED
//!   ELASTIC LID THE CODEBASE NEVER HAD: `ColumnState` is `{temperature, convecting}`, one lumped scalar per
//!   column, and nothing carried temperature on a depth axis. The thermal state is an ANCESTOR of `T_e` (it sets
//!   the geotherm `T(z)`, which sets the ductile branch of the yield envelope), and it was never the CARRIER.
//!
//!   WHAT `T_e` IS (owner ruling 2026-07-16, after one symbol was found bound to two constructions across three
//!   documents): the MOMENT-EQUIVALENT thickness, McNutt 1984. It is the uniform elastic plate that reproduces
//!   the yield-strength envelope's BENDING MOMENT at a given CURVATURE, which is why `T_e` FALLS AS CURVATURE
//!   RISES: more of the real plate yields and the moment saturates. It is therefore NOT a property of the
//!   lithosphere alone and NOT per-world; it is the lithosphere JOINED TO A LOAD, and it is solved per load by a
//!   scalar fixed point (trial `T_e`, elastic deflection, peak curvature, recompute `T_e` from the moment
//!   integral, iterate). Every `T_e` carries its chord fields, LOAD CLASS and LOAD TIMESCALE.
//!
//!   DO NOT CONFUSE IT WITH `T_mech`, the MECHANICAL thickness, which is the crossing of the brittle and ductile
//!   curves and is the depth extent of strength. Both are real and both ship; they are two quantities, and the
//!   defect was one name.
//! - The RESTORING TERM `(rho_mantle - rho_infill) g`: the mantle and infill densities and the surface gravity
//!   are per-world inputs the caller supplies (derived upstream from composition and the body). The kernel
//!   reads the density contrast `delta_rho` and the gravity `g`, never authors them.
//!
//! From these, `flexural_rigidity` computes `D = E T_e^3 / (12 (1 - nu^2))` (Turcotte & Schubert eq. 3-127
//! form, PIPELINE_FETCHES.md section 1) and `flexural_parameter` computes the DERIVED LENGTH SCALE
//! `alpha = (4 D / ((rho_mantle - rho_infill) g))^(1/4)` (T&S eq. 3-127). `alpha` is the whole point: a thin-lid
//! world (small `T_e`, small `D`) flexes at a SHORT wavelength, a thick-lid world at a LONG one. Same formula,
//! different worlds, the conditioning line.
//!
//! # The Green's functions (analytic, cited shapes)
//!
//! The deflection is an analytic convolution of point/line-load Green's functions over the caller's load list.
//!
//! - LINE LOAD (continuous plate), T&S eq. 3-130 / TAFI eq. 4: for a line load `V0` at the line,
//!   `w(x) = (V0 alpha^3 / (8 D)) e^(-x/alpha) (cos(x/alpha) + sin(x/alpha))`, `x` the perpendicular distance.
//!   The maximum deflection under the load is `w0 = V0 alpha^3 / (8 D)`; the decaying oscillation gives the
//!   flexural moat then the forebulge (the zero crossing at `x0 = 3 pi alpha / 4`, the forebulge peak about
//!   4.3 percent of the central depression). This uses only exp/cos/sin, which the fixed-point library carries
//!   exactly, so it is the PROVEN core.
//! - POINT LOAD (axisymmetric, infinite plate), Brotchie & Silvester 1969 / TAFI eq. 6: for a point load `Q0`
//!   at the origin, `w(r) = Q0 (l^2 / (2 pi D)) kei(r/l)`, where `kei` is the zeroth-order Kelvin function, `r`
//!   the radial distance, and `l = (D / (delta_rho g))^(1/4) = alpha / sqrt(2)` the AXISYMMETRIC flexural
//!   length, which is NOT the line-load `alpha` above. `kei(0) = -pi/4`, so the central deflection magnitude is
//!   `Q0 l^2 / (8 D)`, which in the line-load parameter is `Q0 alpha^2 / (16 D)`. It is deliberately NOT
//!   structurally parallel to the line-load `w0`: assuming that parallelism is what welded the two lengths and
//!   made the moat sqrt(2) too wide and 2x too deep here until 2026-07-17. The factor of 4 in `alpha` belongs to
//!   the one-dimensional line-load ODE alone (McNutt and Menard 1982 eq. A8; the TAFI Table 1 point-load row
//!   carries no factor of 4). [`kelvin_kei`] evaluates `kei` from its Abramowitz-Stegun 9.9 ascending series in
//!   fixed point; see its accuracy band below.
//!
//! # No free constants
//!
//! The only literals the kernel carries are the FORMULA'S OWN pure numbers: the `12`, `4`, `8`, `2`, the `1/4`
//! power (a nested square root), `pi` ([`Fixed::PI`]), and the Euler-Mascheroni constant `gamma` ([`euler_gamma`])
//! that the Kelvin/Bessel-`K0` series carries. Each is a dimensionless mathematical constant, the same status as
//! `pi`, not an authored physical value. `E`, `nu`, `T_e`, `delta_rho`, `g`, and the load magnitudes/positions
//! are all caller inputs. No physical value is planted here.
//!
//! # The unit contract (range discipline)
//!
//! The kernel is UNIT-AGNOSTIC: it is pure algebra and treats every input as a plain fixed-point number in
//! whatever single COHERENT unit system the caller chooses; `D` and `alpha` then come out in the induced units.
//! The caller owns the choice, because the Q32.32 window (about +/- 2.1e9, resolution ~2.3e-10) will not hold
//! `D` in raw SI (Earth's `D ~ 1e23 N m` overflows). A coherent system that keeps every intermediate in range
//! for planetary flexure is `{length = km, mass = 1e12 kg, time = s}`, which induces stress in GPa, density in
//! `1000 kg/m^3` (so a `3300 kg/m^3` mantle reads 3.3), and gravity in `km/s^2` (so `9.8 m/s^2` reads 0.0098).
//! In it `D` is in `GPa km^3`, the restoring modulus `delta_rho g` in `GPa/km`, `alpha` and `w` in km, and a
//! line load `V0` in `GPa km`. The tests use exactly this system. The kernel guards every step with checked
//! arithmetic and fails loud (`None`) on an out-of-range intermediate rather than wrapping, so a badly-scaled
//! unit choice is refused, never silently corrupted.
//!
//! # Honest limits
//!
//! The forms are the constant-rigidity, no-end-load solutions: `D` uniform (no lateral rigidity variation), no
//! in-plane (membrane) force, an inviscid substrate (instantaneous isostatic response, no viscoelastic relaxation
//! time). The line load runs parallel to the y-axis ([`LoadKind::LineY`]); an arbitrary line orientation is a
//! caller-side coordinate rotation, not modelled here. The point-load [`kelvin_kei`] is accurate over its
//! validated near-to-mid field and returns zero in the far field where the ascending series would leave the
//! Q32.32 window and `kei` is already negligible (see the constant docs). Superposition is linear, so overlapping
//! loads beyond the small-deflection (`w << T_e`) regime are outside the theory the way real flexure is.

use civsim_core::Fixed;

/// The Euler-Mascheroni constant `gamma = 0.5772156649...`, a dimensionless MATHEMATICAL constant (the same
/// status as `pi`), NOT an authored physical value. It is carried by the ascending series of the modified Bessel
/// function `K_0` and hence of the Kelvin function `kei` (Abramowitz & Stegun 9.9): the log term of `kei` is
/// `-(ln(x/2) + gamma) bei(x)`, not `-ln(x/2) bei(x)`. Omitting it is a real error (it shifts `kei(1)` from the
/// correct `-0.4950` to `-0.3509`). Read to ten decimals through the exact integer ratio, so it lands on the
/// Q32.32 grid with no floating point.
pub fn euler_gamma() -> Fixed {
    Fixed::from_ratio(5_772_156_649, 10_000_000_000)
}

/// The far-field radius cap (in units of `r/alpha`) beyond which [`kelvin_kei`] returns zero. This is a
/// NUMERICAL / representability bound, not a physical value: past it the `ber`/`bei` partial sums of the
/// ascending series grow like `e^(x/sqrt2)` toward the edge of the Q32.32 window, while `kei` itself has already
/// decayed below `~1e-4` (it decays as `e^(-x/sqrt2)`), so the point-load deflection there is negligible and is
/// taken as zero. Twelve `alpha` is far past the point-load forebulge (near `5.6 alpha`) and its decay.
const KEI_SERIES_MAX: i32 = 12;

/// The maximum number of terms summed in the [`kelvin_kei`] ascending series before the loop gives up. A
/// NUMERICAL resolution (the series converges well inside this within the `KEI_SERIES_MAX` domain, breaking early
/// when successive terms fall below the fixed-point floor), the sibling of the Debye integral's interval count.
const KEI_MAX_TERMS: i32 = 40;

/// The FLEXURAL RIGIDITY `D = E T_e^3 / (12 (1 - nu^2))` (Turcotte & Schubert, Geodynamics ch. 3, the
/// moment-curvature relation; PIPELINE_FETCHES.md section 1). `e` is Young's modulus, `t_e` the elastic lid
/// thickness, `nu` Poisson's ratio, all caller inputs in one coherent unit system (see the module unit contract);
/// `D` comes out in `stress * length^3`. The `12` is the formula's own. Fails loud (`None`) on a non-positive
/// modulus or thickness, on `1 - nu^2 <= 0` (`|nu| >= 1`, outside the physical Poisson range), or on any
/// out-of-range fixed-point intermediate, never a fabricated rigidity. Deterministic (Principle 3).
pub fn flexural_rigidity(e: Fixed, nu: Fixed, t_e: Fixed) -> Option<Fixed> {
    if e <= Fixed::ZERO || t_e <= Fixed::ZERO {
        return None;
    }
    // 1 - nu^2, the plane-strain stiffening factor; must be strictly positive (|nu| < 1).
    let nu2 = nu.checked_mul(nu)?;
    let one_minus_nu2 = Fixed::ONE.checked_sub(nu2)?;
    if one_minus_nu2 <= Fixed::ZERO {
        return None;
    }
    // T_e^3 by checked multiplies (not the wrapping powi), the checked-innermost discipline.
    let t3 = t_e.checked_mul(t_e).and_then(|t2| t2.checked_mul(t_e))?;
    let numerator = e.checked_mul(t3)?;
    let denom = Fixed::from_int(12).checked_mul(one_minus_nu2)?;
    numerator.checked_div(denom)
}

/// The FLEXURAL PARAMETER `alpha = (4 D / (delta_rho g))^(1/4)` (Turcotte & Schubert eq. 3-127; PIPELINE_FETCHES.md
/// section 1), the DERIVED length scale the plate bends at, in the caller's length unit. `d` is the flexural
/// rigidity, `delta_rho = rho_mantle - rho_infill` the density contrast the deflection floats against, `g` the
/// surface gravity. The quarter power is computed as a NESTED square root arranged as `alpha = sqrt(2 sqrt(D /
/// (delta_rho g)))`, which is algebraically identical to `(4 D / (delta_rho g))^(1/4)` (the `4` enters as
/// `sqrt(4) = 2` folded into the inner root) but keeps the intermediate `4 D` from overflowing for a very thick
/// lid, a pure range-hygiene rearrangement of the cited form. [`Fixed::sqrt`] is exact to the last bit, so this
/// carries no series error. Fails loud (`None`) on a non-positive `d`, `delta_rho`, or `g`, or on an out-of-range
/// intermediate. Deterministic.
pub fn flexural_parameter(d: Fixed, delta_rho: Fixed, g: Fixed) -> Option<Fixed> {
    if d <= Fixed::ZERO || delta_rho <= Fixed::ZERO || g <= Fixed::ZERO {
        return None;
    }
    let restoring = delta_rho.checked_mul(g)?; // (rho_m - rho_infill) g, the buoyancy modulus
    let ratio = d.checked_div(restoring)?; // D / (delta_rho g) = alpha^4 / 4
    let inner = Fixed::from_int(2).checked_mul(ratio.sqrt())?; // 2 sqrt(ratio) = alpha^2
    Some(inner.sqrt()) // sqrt(alpha^2) = alpha
}

/// The AXISYMMETRIC (point/disc load) flexural length `l = (D / (delta_rho g))^(1/4)`, DISTINCT from the
/// line-load [`flexural_parameter`] `alpha = (4D / (delta_rho g))^(1/4) = sqrt(2) l`. The factor of 4 belongs to
/// the one-dimensional line-load ODE; the axisymmetric plate equation `D grad^4 w + delta_rho g w = 0` has natural
/// length `l`, since `grad^4 kei(r/l) = -(1/l^4) kei(r/l)` cancels the restoring term only when `l^4 = D/(delta_rho g)`
/// (McNutt and Menard 1982 eq. A8; TAFI point-load row, Brotchie and Silvester 1969, no factor of 4). Naming this
/// separately keeps the two length scales from being welded, the exact confusion PIPELINE_FETCHES.md section 1 made.
pub fn flexural_length_axisymmetric(d: Fixed, delta_rho: Fixed, g: Fixed) -> Option<Fixed> {
    if d <= Fixed::ZERO || delta_rho <= Fixed::ZERO || g <= Fixed::ZERO {
        return None;
    }
    let ratio = d.checked_div(delta_rho.checked_mul(g)?)?; // D / (delta_rho g) = l^4
    Some(ratio.sqrt().sqrt()) // (l^4)^(1/4) = l
}

/// The axisymmetric length from the already-computed line-load `alpha`, exactly `l = alpha / sqrt(2)` (since
/// `alpha^4 = 4 l^4`). Used by [`point_load_deflection`] so a caller that has `alpha` need not re-thread the
/// densities, and the conversion happens in one audited place.
fn flexural_length_axisymmetric_from_alpha(alpha: Fixed) -> Option<Fixed> {
    alpha.checked_div(Fixed::from_int(2).sqrt())
}

// ----- The spectral flexural filter (the transfer function on a periodic load) -----

/// The FLEXURAL RESPONSE RATIO `Phi(k) = 1 / (1 + (l k)^4)`, the fraction of an isostatic (Airy) load a plate of
/// axisymmetric flexural length `l` passes into supported relief at wavenumber `k`. It is the spectral form of the
/// plate equation `D grad^4 w + delta_rho g w = load`: in the wavenumber domain `(D k^4 + delta_rho g) W = Load`,
/// so the ratio of the plate deflection to the isostatic deflection `Load / (delta_rho g)` is `delta_rho g /
/// (D k^4 + delta_rho g) = 1 / (1 + (l k)^4)`, since `l^4 = D / (delta_rho g)` ([`flexural_length_axisymmetric`];
/// Turcotte & Schubert, the flexural filter / degree of compensation). A LONG-wavelength load (small `k`, `l k`
/// much less than 1) is fully compensated and passes as full relief (`Phi -> 1`, isostasy); a SHORT-wavelength
/// load (large `k`, `l k` much greater than 1) is held by plate strength and passes almost none (`Phi -> 0`, the
/// lid does not bend at that scale). This is the transfer function that convolves a province thickness-contrast
/// field into the relief the lid supports, evaluated at the linear-response rigidity by passing
/// `l = flexural_length_axisymmetric(D_mech, delta_rho, g)`.
///
/// The characteristic corner is at `l k = 1` (the wavelength `2 pi l`), where `Phi = 1/2`: contrasts longer than
/// this pass more than half their relief, shorter ones less. Dimensionless, in `(0, 1]`. The `k = 0` DC term (the
/// infinite-wavelength mean) returns `Phi = 1` exactly (a uniform load is fully isostatic). Fails loud (`None`) on
/// a non-positive `l`, a negative `k`, or an out-of-range fixed-point intermediate, never a fabricated ratio.
/// Deterministic.
pub fn flexural_response_ratio(flexural_length: Fixed, wavenumber: Fixed) -> Option<Fixed> {
    if flexural_length <= Fixed::ZERO || wavenumber < Fixed::ZERO {
        return None;
    }
    if wavenumber == Fixed::ZERO {
        // The infinite-wavelength (DC) term is fully compensated: a uniform load floats isostatically.
        return Some(Fixed::ONE);
    }
    let lk = flexural_length.checked_mul(wavenumber)?;
    let lk2 = lk.checked_mul(lk)?;
    let lk4 = lk2.checked_mul(lk2)?;
    Fixed::ONE.checked_div(Fixed::ONE + lk4)
}

/// The LINE-LOAD deflection `w(x) = (V0 alpha^3 / (8 D)) e^(-|x|/alpha) (cos(|x|/alpha) + sin(|x|/alpha))`
/// (Turcotte & Schubert eq. 3-130 / TAFI eq. 4, the continuous-plate solution; PIPELINE_FETCHES.md section 1),
/// the flexure at perpendicular distance `perp_dist` from a line load of magnitude `v0`, given the flexural
/// parameter `alpha` and rigidity `d`. The value is signed in the T&S convention: positive is the downward moat
/// under and beside the load, turning negative past the zero crossing at `3 pi alpha / 4` (the upward forebulge).
/// The distance is taken by magnitude, so the profile is symmetric about the line. A zero load gives zero
/// everywhere. The `8` is the formula's own. Fails loud (`None`) on a non-positive `alpha` or `d`, or on an
/// out-of-range intermediate. Deterministic.
pub fn line_load_deflection(v0: Fixed, alpha: Fixed, d: Fixed, perp_dist: Fixed) -> Option<Fixed> {
    if alpha <= Fixed::ZERO || d <= Fixed::ZERO {
        return None;
    }
    // w0 = V0 alpha^3 / (8 D), the maximum deflection under the load.
    let a3 = alpha
        .checked_mul(alpha)
        .and_then(|a2| a2.checked_mul(alpha))?;
    let eight_d = Fixed::from_int(8).checked_mul(d)?;
    let w0 = v0.checked_mul(a3).and_then(|x| x.checked_div(eight_d))?;
    // The dimensionless argument X = |x| / alpha, and the decaying-oscillatory shape e^(-X)(cos X + sin X).
    let big_x = perp_dist.abs().checked_div(alpha)?;
    let decay = (Fixed::ZERO - big_x).exp(); // e^(-X); saturates to 0 far from the load (honest Q32.32 limit)
    let (sin_x, cos_x) = big_x.sin_cos();
    let shape = decay.checked_mul(cos_x.checked_add(sin_x)?)?;
    w0.checked_mul(shape)
}

/// The POINT-LOAD (axisymmetric) deflection `w(r) = Q0 (l^2 / (2 pi D)) kei(r/l)` (Brotchie & Silvester 1969 /
/// TAFI eq. 6, the impulse-response Green's function), the flexure at radial distance `r` from a point load of
/// magnitude `q0`, given rigidity `d`.
///
/// THE LENGTH IN THE FORMULA IS NOT THE ARGUMENT. The caller passes `alpha`, the LINE-LOAD flexural parameter
/// `(4 D / (delta_rho g))^(1/4)`, because that is what every caller already holds; this function converts once
/// to the AXISYMMETRIC length `l = (D / (delta_rho g))^(1/4) = alpha / sqrt(2)`
/// ([`flexural_length_axisymmetric`]) and runs the Green's function on `l`. The factor of 4 belongs to the
/// one-dimensional line-load ODE, never to the axisymmetric plate equation, whose `grad^4 kei(r/l) =
/// -(1/l^4) kei(r/l)` cancels the restoring term only at `l^4 = D / (delta_rho g)` (McNutt and Menard 1982
/// eq. A8; the TAFI Table 1 point-load row, which carries no factor of 4).
///
/// So the value is signed and `kei(0) = -pi/4` gives a central depression of magnitude `Q0 l^2 / (8 D)`, which
/// in the caller's own `alpha` is `Q0 alpha^2 / (16 D)`, HALF what the line-load parameter alone would suggest.
/// `kei` crosses zero and rises into the axisymmetric forebulge farther out. The `2 pi` is the formula's own.
///
/// Stated at this length because the welded form was a real defect here, not a hypothetical one: this function
/// read `kei(r/alpha)` and scaled by `alpha^2` until 2026-07-17, which made the moat sqrt(2) too wide and 2x too
/// deep, from a misread of TAFI Table 1 in PIPELINE_FETCHES.md that welded one `alpha` to both load types.
///
/// Fails loud (`None`) on a non-positive `alpha` or `d`, a negative `r`, or an out-of-range intermediate.
/// Deterministic.
pub fn point_load_deflection(q0: Fixed, alpha: Fixed, d: Fixed, r: Fixed) -> Option<Fixed> {
    if alpha <= Fixed::ZERO || d <= Fixed::ZERO || r < Fixed::ZERO {
        return None;
    }
    // THE AXISYMMETRIC LENGTH IS NOT alpha. The caller passes `alpha = (4D/dpg)^(1/4)`, the LINE-LOAD flexural
    // parameter, and the axisymmetric point-load Green's function `w = -(P l^2/2 pi D) kei(r/l)` runs on
    // `l = (D/dpg)^(1/4) = alpha/sqrt(2)`, verified three ways (the plate ODE `grad^4 kei = -kei` forces
    // `l^4 = D/dpg`; McNutt and Menard 1982 eq. A8; the point-load row of TAFI Table 1, whose parameter carries
    // no factor of 4). The factor of 4 is the 1-D line-load ODE's, not the axisymmetric one's. Reading `kei(r/alpha)`
    // and scaling by `alpha^2` (as this did until 2026-07-17) made the moat sqrt(2) too wide and 2x too deep; the
    // origin was a misread of TAFI Table 1 in PIPELINE_FETCHES.md that welded one `alpha` to both load types.
    // Converting once here, from the parameter every caller already has, keeps the line-load `alpha` from
    // reaching this function under the wrong meaning: this function OWNS the conversion.
    let l = flexural_length_axisymmetric_from_alpha(alpha)?;
    // coefficient Q0 l^2 / (2 pi D)
    let l2 = l.checked_mul(l)?;
    let two_pi_d = Fixed::from_int(2)
        .checked_mul(Fixed::PI)
        .and_then(|x| x.checked_mul(d))?;
    let coef = q0.checked_mul(l2).and_then(|x| x.checked_div(two_pi_d))?;
    let arg = r.checked_div(l)?; // r / l
    coef.checked_mul(kelvin_kei(arg))
}

/// The zeroth-order Kelvin function `kei(x)`, the axisymmetric point-load Green's-function shape (Brotchie &
/// Silvester 1969), evaluated from the Abramowitz & Stegun 9.9 ascending series in fixed point:
///
/// `kei(x) = -(ln(x/2) + gamma) bei(x) - (pi/4) ber(x) + sum_{j>=0} (-1)^j H_{2j+1}/((2j+1)!)^2 (x/2)^(4j+2)`,
///
/// with `ber(x) = sum_j (-1)^j (x/2)^(4j) / ((2j)!)^2`, `bei(x) = sum_j (-1)^j (x/2)^(4j+2) / ((2j+1)!)^2`,
/// `gamma` the Euler-Mascheroni constant, and `H_n` the `n`-th harmonic number. The `ber`, `bei`, and
/// harmonic-weighted sums are accumulated by TERM RECURRENCE (each term from the previous, so no standalone
/// `(x/2)^(4j)` power is ever formed) to keep every intermediate in the Q32.32 window.
///
/// Domain and accuracy: `kei(0) = -pi/4` exactly (returned for `x <= 0`; a negative radius is not meaningful and
/// clamps to the origin). For `0 < x <= KEI_SERIES_MAX` the series is evaluated; it matches tabulated reference
/// values (`kei(1) ~ -0.4950`, `kei(2) ~ -0.2024`) to within the fixed-point floor and reproduces the sign change
/// and small positive forebulge of the true function. Beyond `KEI_SERIES_MAX` it returns zero (the far field
/// where `kei` is negligible and the ascending series would overflow; see the constant). Deterministic.
pub fn kelvin_kei(x: Fixed) -> Fixed {
    let pi_over_4 = Fixed::PI
        .checked_div(Fixed::from_int(4))
        .unwrap_or(Fixed::ZERO);
    if x <= Fixed::ZERO {
        // kei(0) = -pi/4 (ber(0) = 1, bei(0) = 0, and the log term vanishes as x^2 ln x -> 0).
        return Fixed::ZERO - pi_over_4;
    }
    if x > Fixed::from_int(KEI_SERIES_MAX) {
        return Fixed::ZERO;
    }
    let xh = match x.checked_div(Fixed::from_int(2)) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    let xh2 = match xh.checked_mul(xh) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    let xh4 = match xh2.checked_mul(xh2) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    // b_k: the ber term (b_0 = 1); c_k: the bei term (c_0 = (x/2)^2); h: H_{2k+1} (H_1 = 1).
    let mut b = Fixed::ONE;
    let mut c = xh2;
    let mut ber = b;
    let mut bei = c;
    let mut h = Fixed::ONE;
    let mut bei_phi = match c.checked_mul(h) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    let mut k = 1i32;
    while k <= KEI_MAX_TERMS {
        // ber recurrence: b_k = b_{k-1} * (-(x/2)^4 / ((2k)(2k-1))^2)
        let m_ber = (2 * k) * (2 * k - 1);
        let d_ber = match Fixed::from_int(m_ber).checked_mul(Fixed::from_int(m_ber)) {
            Some(v) => v,
            None => break,
        };
        b = match b.checked_mul(xh4).and_then(|x| x.checked_div(d_ber)) {
            Some(v) => Fixed::ZERO - v,
            None => break,
        };
        ber = ber.checked_add(b).unwrap_or(ber);
        // bei recurrence: c_k = c_{k-1} * (-(x/2)^4 / ((2k+1)(2k))^2)
        let m_bei = (2 * k + 1) * (2 * k);
        let d_bei = match Fixed::from_int(m_bei).checked_mul(Fixed::from_int(m_bei)) {
            Some(v) => v,
            None => break,
        };
        c = match c.checked_mul(xh4).and_then(|x| x.checked_div(d_bei)) {
            Some(v) => Fixed::ZERO - v,
            None => break,
        };
        bei = bei.checked_add(c).unwrap_or(bei);
        // H_{2k+1} = H_{2k-1} + 1/(2k) + 1/(2k+1)
        let inv_even = Fixed::ONE
            .checked_div(Fixed::from_int(2 * k))
            .unwrap_or(Fixed::ZERO);
        let inv_odd = Fixed::ONE
            .checked_div(Fixed::from_int(2 * k + 1))
            .unwrap_or(Fixed::ZERO);
        h = h.saturating_add(inv_even).saturating_add(inv_odd);
        if let Some(term) = c.checked_mul(h) {
            bei_phi = bei_phi.checked_add(term).unwrap_or(bei_phi);
        }
        // Converged once both leading increments fall below the fixed-point floor.
        if b.abs() <= Fixed::EPSILON && c.abs() <= Fixed::EPSILON {
            break;
        }
        k += 1;
    }
    // kei = -(ln(x/2) + gamma) bei - (pi/4) ber + bei_phi
    let ln_term = xh.ln().saturating_add(euler_gamma());
    let log_part = Fixed::ZERO - ln_term.mul(bei);
    let pi_part = Fixed::ZERO - pi_over_4.mul(ber);
    log_part.saturating_add(pi_part).saturating_add(bei_phi)
}

/// The zeroth-order Kelvin function `ker(x)`, the SECOND axisymmetric point-load Green's-function shape (the
/// companion of [`kelvin_kei`]), evaluated from the Abramowitz & Stegun 9.9 ascending series in fixed point:
///
/// `ker(x) = -(ln(x/2) + gamma) ber(x) + (pi/4) bei(x) + sum_{k>=1} (-1)^k H_{2k}/((2k)!)^2 (x/2)^(4k)`,
///
/// with `ber` and `bei` exactly as in [`kelvin_kei`], `gamma` the Euler-Mascheroni constant ([`euler_gamma`]),
/// and `H_n` the `n`-th harmonic number. `H_0 = 0`, so the `k = 0` term of the harmonic sum vanishes and the
/// sum starts at `k = 1`. The `ber`, `bei`, and harmonic-weighted sums are accumulated by the same TERM
/// RECURRENCE [`kelvin_kei`] uses, so no standalone `(x/2)^(4k)` power is ever formed.
///
/// # WHY THE AXISYMMETRIC MOMENT EQUIVALENCE NEEDS IT
///
/// The point-load deflection is `w(r) = -(P l^2 / 2 pi D) kei(r/l)` (McNutt and Menard 1982 eq. A8, with
/// `l = (D / (delta_rho g))^(1/4)`). Its LAPLACIAN CURVATURE `K = d2w/dr2 + (1/r) dw/dr` reduces, through the
/// Kelvin identity `kei'' + kei'/x = ker`, to `K = -(P / 2 pi D) ker(r/l)`. So `ker` at the deflection's first
/// zero crossing is the axisymmetric plate's reported curvature, the sibling of the line load's own read at
/// `x = 3 pi / 4`. See `crate::moment_equivalence::point_load_curvature_at_first_zero_crossing`.
///
/// Domain and accuracy: `ker` has a LOGARITHMIC SINGULARITY at the origin (`ker(x) -> +inf` as `x -> 0`), so a
/// non-positive argument has no value and returns [`Fixed::MIN`] as a fail-loud sentinel, the same convention
/// [`Fixed::ln`] uses; a caller guards its domain. For `0 < x <= KEI_SERIES_MAX` the series is evaluated and
/// matches tabulated reference values (`ker(1) ~ 0.286706`, `ker(2) ~ -0.041665`, `ker(3.91467) ~ -0.038899`)
/// to within the fixed-point floor. The last is the value at the first zero crossing that McNutt and Menard's
/// own printed `-0.0289` fails to reproduce (their erratum; see `crate::moment_equivalence`). Beyond
/// `KEI_SERIES_MAX` it returns zero (the far field where `ker` decays as `e^(-x/sqrt2)` and is negligible while
/// the `ber`/`bei` partial sums would leave the Q32.32 window). Deterministic.
pub fn kelvin_ker(x: Fixed) -> Fixed {
    let pi_over_4 = Fixed::PI
        .checked_div(Fixed::from_int(4))
        .unwrap_or(Fixed::ZERO);
    if x <= Fixed::ZERO {
        // ker has no value at or below the origin (a logarithmic singularity at 0), so fail loud.
        return Fixed::MIN;
    }
    if x > Fixed::from_int(KEI_SERIES_MAX) {
        return Fixed::ZERO;
    }
    let xh = match x.checked_div(Fixed::from_int(2)) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    let xh2 = match xh.checked_mul(xh) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    let xh4 = match xh2.checked_mul(xh2) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    // b_k: the ber term (b_0 = 1); c_k: the bei term (c_0 = (x/2)^2).
    let mut b = Fixed::ONE;
    let mut c = xh2;
    let mut ber = b;
    let mut bei = c;
    // ber_phi = sum_{k>=1} b_k * H_{2k}; h_even tracks H_{2k} (H_0 = 0, so the k=0 term is skipped).
    let mut h_even = Fixed::ZERO;
    let mut ber_phi = Fixed::ZERO;
    let mut k = 1i32;
    while k <= KEI_MAX_TERMS {
        // ber recurrence: b_k = b_{k-1} * (-(x/2)^4 / ((2k)(2k-1))^2)
        let m_ber = (2 * k) * (2 * k - 1);
        let d_ber = match Fixed::from_int(m_ber).checked_mul(Fixed::from_int(m_ber)) {
            Some(v) => v,
            None => break,
        };
        b = match b.checked_mul(xh4).and_then(|x| x.checked_div(d_ber)) {
            Some(v) => Fixed::ZERO - v,
            None => break,
        };
        ber = ber.checked_add(b).unwrap_or(ber);
        // bei recurrence: c_k = c_{k-1} * (-(x/2)^4 / ((2k+1)(2k))^2)
        let m_bei = (2 * k + 1) * (2 * k);
        let d_bei = match Fixed::from_int(m_bei).checked_mul(Fixed::from_int(m_bei)) {
            Some(v) => v,
            None => break,
        };
        c = match c.checked_mul(xh4).and_then(|x| x.checked_div(d_bei)) {
            Some(v) => Fixed::ZERO - v,
            None => break,
        };
        bei = bei.checked_add(c).unwrap_or(bei);
        // H_{2k} = H_{2k-2} + 1/(2k-1) + 1/(2k)
        let inv_odd = Fixed::ONE
            .checked_div(Fixed::from_int(2 * k - 1))
            .unwrap_or(Fixed::ZERO);
        let inv_even = Fixed::ONE
            .checked_div(Fixed::from_int(2 * k))
            .unwrap_or(Fixed::ZERO);
        h_even = h_even.saturating_add(inv_odd).saturating_add(inv_even);
        if let Some(term) = b.checked_mul(h_even) {
            ber_phi = ber_phi.checked_add(term).unwrap_or(ber_phi);
        }
        // Converged once both leading increments fall below the fixed-point floor.
        if b.abs() <= Fixed::EPSILON && c.abs() <= Fixed::EPSILON {
            break;
        }
        k += 1;
    }
    // ker = -(ln(x/2) + gamma) ber + (pi/4) bei + ber_phi
    let ln_term = xh.ln().saturating_add(euler_gamma());
    let log_part = Fixed::ZERO - ln_term.mul(ber);
    let pi_part = pi_over_4.mul(bei);
    log_part.saturating_add(pi_part).saturating_add(ber_phi)
}

/// The derivative `kei'(x)` of the zeroth-order Kelvin function, evaluated from the term-by-term derivative of
/// [`kelvin_kei`]'s Abramowitz & Stegun 9.9 series:
///
/// `kei'(x) = -(1/x) bei(x) - (ln(x/2) + gamma) bei'(x) - (pi/4) ber'(x) + [d/dx of the harmonic-weighted sum]`,
///
/// where each power's derivative is `d/dx (x/2)^p = (p / (2 (x/2))) (x/2)^p`, so `ber'(x)` is
/// `sum_{k>=1} (2k / (x/2)) b_k`, `bei'(x)` is `sum_{k>=0} ((2k+1) / (x/2)) c_k`, and the harmonic sum's
/// derivative is `sum_{k>=0} ((2k+1) / (x/2)) c_k H_{2k+1}`, each accumulated from the same `b_k`, `c_k` term
/// recurrence [`kelvin_kei`] uses. The `-(1/x) bei(x)` term is the derivative of the `-(ln(x/2) + gamma)`
/// prefactor, whose `d/dx` is `-(1/x)`.
///
/// # WHY THE AXISYMMETRIC MOMENT EQUIVALENCE NEEDS IT
///
/// The point-load deflection's TANGENTIAL (hoop) curvature is `kappa_theta = (1/r) dw/dr = -(P/2 pi D)(1/x) kei'(x)`,
/// so the radial fibre's driving curvature `kappa_r + nu kappa_theta` (McNutt and Menard 1982's `M` operator,
/// the one carrying `nu/r` against the Laplacian's `1/r`) reads `kei'` at the first zero crossing. See
/// `crate::moment_equivalence::point_load_curvature_at_first_zero_crossing`.
///
/// Domain and accuracy: `kei'(x)` is finite at the origin (the odd-power derivative series vanishes there), and
/// `0` is returned for `x <= 0`, a negative radius being meaningless. For `0 < x <= KEI_SERIES_MAX` the series
/// matches tabulated values (`kei'(1) ~ 0.352370`, `kei'(3.91467) ~ 0.027669`, `kei'(4.93181) ~ 0` at the arch
/// peak where `kei` is extremal) to within the fixed-point floor; beyond `KEI_SERIES_MAX` it returns zero (the
/// far field where `kei'` is negligible). Deterministic.
pub fn kelvin_kei_prime(x: Fixed) -> Fixed {
    let pi_over_4 = Fixed::PI
        .checked_div(Fixed::from_int(4))
        .unwrap_or(Fixed::ZERO);
    if x <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    if x > Fixed::from_int(KEI_SERIES_MAX) {
        return Fixed::ZERO;
    }
    let xh = match x.checked_div(Fixed::from_int(2)) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    let xh2 = match xh.checked_mul(xh) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    let xh4 = match xh2.checked_mul(xh2) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    // b_k: the ber term (b_0 = 1); c_k: the bei term (c_0 = (x/2)^2).
    let mut b = Fixed::ONE;
    let mut c = xh2;
    let mut bei = c;
    // The k=0 contributions to the derivative sums. ber'(0-term) = 0 (the constant b_0 has zero derivative);
    // bei' and the harmonic-sum derivative carry the c_0 term at factor (1/(x/2)) with H_1 = 1.
    let inv_xh = match Fixed::ONE.checked_div(xh) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    let mut berp = Fixed::ZERO;
    let mut beip = c.checked_mul(inv_xh).unwrap_or(Fixed::ZERO); // (1/xh) c_0
    let mut h_odd = Fixed::ONE; // H_{2k+1}, starting at H_1 = 1
    let mut beiphip = beip; // (1/xh) c_0 H_1, and H_1 = 1
    let mut k = 1i32;
    while k <= KEI_MAX_TERMS {
        // ber recurrence and its derivative contribution (2k / xh) b_k.
        let m_ber = (2 * k) * (2 * k - 1);
        let d_ber = match Fixed::from_int(m_ber).checked_mul(Fixed::from_int(m_ber)) {
            Some(v) => v,
            None => break,
        };
        b = match b.checked_mul(xh4).and_then(|x| x.checked_div(d_ber)) {
            Some(v) => Fixed::ZERO - v,
            None => break,
        };
        if let Some(term) = Fixed::from_int(2 * k)
            .checked_mul(b)
            .and_then(|t| t.checked_mul(inv_xh))
        {
            berp = berp.checked_add(term).unwrap_or(berp);
        }
        // bei recurrence and its derivative contribution ((2k+1) / xh) c_k.
        let m_bei = (2 * k + 1) * (2 * k);
        let d_bei = match Fixed::from_int(m_bei).checked_mul(Fixed::from_int(m_bei)) {
            Some(v) => v,
            None => break,
        };
        c = match c.checked_mul(xh4).and_then(|x| x.checked_div(d_bei)) {
            Some(v) => Fixed::ZERO - v,
            None => break,
        };
        bei = bei.checked_add(c).unwrap_or(bei);
        let deriv_factor = match Fixed::from_int(2 * k + 1).checked_mul(inv_xh) {
            Some(v) => v,
            None => break,
        };
        if let Some(term) = c.checked_mul(deriv_factor) {
            beip = beip.checked_add(term).unwrap_or(beip);
        }
        // H_{2k+1} = H_{2k-1} + 1/(2k) + 1/(2k+1)
        let inv_even = Fixed::ONE
            .checked_div(Fixed::from_int(2 * k))
            .unwrap_or(Fixed::ZERO);
        let inv_odd = Fixed::ONE
            .checked_div(Fixed::from_int(2 * k + 1))
            .unwrap_or(Fixed::ZERO);
        h_odd = h_odd.saturating_add(inv_even).saturating_add(inv_odd);
        if let Some(term) = c
            .checked_mul(deriv_factor)
            .and_then(|t| t.checked_mul(h_odd))
        {
            beiphip = beiphip.checked_add(term).unwrap_or(beiphip);
        }
        if b.abs() <= Fixed::EPSILON && c.abs() <= Fixed::EPSILON {
            break;
        }
        k += 1;
    }
    // kei'(x) = -(1/x) bei - (ln(x/2) + gamma) bei' - (pi/4) ber' + [harmonic sum]'
    let inv_x = match Fixed::ONE.checked_div(x) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    let ln_term = xh.ln().saturating_add(euler_gamma());
    let bei_part = Fixed::ZERO - inv_x.mul(bei);
    let log_part = Fixed::ZERO - ln_term.mul(beip);
    let pi_part = Fixed::ZERO - pi_over_4.mul(berp);
    bei_part
        .saturating_add(log_part)
        .saturating_add(pi_part)
        .saturating_add(beiphip)
}

/// The per-world plate inputs the deflection evaluator reads, all caller-supplied in one coherent unit system
/// (see the module unit contract). Every field is DERIVED UPSTREAM; the kernel authors none of them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlateInputs {
    /// Young's modulus `E` (stress unit), the banked isotropic-elastic derivation over the bulk and shear moduli.
    pub youngs_modulus: Fixed,
    /// Poisson's ratio `nu` (dimensionless), the banked isotropic-elastic companion.
    pub poisson_ratio: Fixed,
    /// The MOMENT-EQUIVALENT elastic thickness `T_e` (length unit), PER LOAD: the uniform elastic plate that
    /// reproduces the yield envelope's bending moment at THIS load's own curvature (McNutt 1984). It is not read
    /// off the thermal state and it is not a per-world constant; see the module doc for the correction and for
    /// its distinction from `T_mech`, the brittle-ductile crossing. Still the SOLE unsupplied input to `D`: the
    /// geotherm arc derives it, and until that lands `PlateInputs` has no production caller.
    pub elastic_thickness: Fixed,
    /// The density contrast `delta_rho = rho_mantle - rho_infill` (density unit) the deflection floats against.
    pub density_contrast: Fixed,
    /// The surface gravity `g` (length/time^2).
    pub gravity: Fixed,
}

impl PlateInputs {
    /// The flexural rigidity `D` for these inputs ([`flexural_rigidity`]).
    pub fn rigidity(&self) -> Option<Fixed> {
        flexural_rigidity(
            self.youngs_modulus,
            self.poisson_ratio,
            self.elastic_thickness,
        )
    }

    /// The flexural parameter `alpha` for these inputs ([`flexural_parameter`]).
    pub fn parameter(&self) -> Option<Fixed> {
        flexural_parameter(self.rigidity()?, self.density_contrast, self.gravity)
    }
}

/// Which Green's function a load contributes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadKind {
    /// A LINE load running parallel to the y-axis through `x = Load::x` (a mountain belt, a rift shoulder). Its
    /// deflection is the 1-D [`line_load_deflection`] in the perpendicular distance `|query_x - Load::x|`;
    /// `Load::y` is unused. An arbitrary line orientation is a caller-side coordinate rotation.
    LineY,
    /// A POINT load at `(Load::x, Load::y)` (a volcanic construct, a large crater basin). Its deflection is the
    /// axisymmetric [`point_load_deflection`] in the radial distance to the query point.
    Point,
}

/// One load in the caller's load list: a kind, a magnitude, and a position. For a [`LoadKind::LineY`] the
/// magnitude is the line-load intensity `V0` (force per unit length) and only `x` is read; for a
/// [`LoadKind::Point`] it is the point-load magnitude `Q0` and both `x` and `y` are read. All in the caller's
/// coherent unit system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Load {
    /// Whether this load contributes the line or the point Green's function.
    pub kind: LoadKind,
    /// The load magnitude (`V0` for a line load, `Q0` for a point load).
    pub magnitude: Fixed,
    /// The load position's x coordinate (the line's x for [`LoadKind::LineY`], the point's x for
    /// [`LoadKind::Point`]).
    pub x: Fixed,
    /// The load position's y coordinate (unused for [`LoadKind::LineY`], the point's y for [`LoadKind::Point`]).
    pub y: Fixed,
}

/// The DEFLECTION EVALUATOR: the total plate deflection at query point `(qx, qy)` from the whole load list,
/// given the plate inputs. It computes `D` and `alpha` once, sums the per-load Green's-function contributions
/// (each in its own distance to the query), and returns the signed deflection in the caller's length unit.
///
/// The sum is over `Fixed` values, whose addition is exact and associative, so the result is INDEPENDENT of the
/// order the loads are listed in (the determinism contract). An empty list, or a list of zero-magnitude loads,
/// gives zero. Fails loud (`None`) if the rigidity or parameter is degenerate, or if any load contribution or the
/// running sum leaves the Q32.32 window, never a fabricated deflection. Deterministic (Principle 3).
pub fn deflection_at(inputs: &PlateInputs, loads: &[Load], qx: Fixed, qy: Fixed) -> Option<Fixed> {
    let d = inputs.rigidity()?;
    let alpha = flexural_parameter(d, inputs.density_contrast, inputs.gravity)?;
    let mut total = Fixed::ZERO;
    for load in loads {
        let contribution = match load.kind {
            LoadKind::LineY => {
                let perp = qx.checked_sub(load.x)?;
                line_load_deflection(load.magnitude, alpha, d, perp)?
            }
            LoadKind::Point => {
                let dx = qx.checked_sub(load.x)?;
                let dy = qy.checked_sub(load.y)?;
                let dx2 = dx.checked_mul(dx)?;
                let dy2 = dy.checked_mul(dy)?;
                let r = dx2.checked_add(dy2)?.sqrt();
                point_load_deflection(load.magnitude, alpha, d, r)?
            }
        };
        total = total.checked_add(contribution)?;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A representative COHERENT unit system for the tests (module unit contract): length in km, stress in GPa,
    // density in units of 1000 kg/m^3, gravity in km/s^2. So an Earth-like lithosphere reads:
    //   E = 70 GPa, nu = 0.25, mantle density 3.3, infill (air) density 0 -> delta_rho = 3.3, g = 0.0098 km/s^2.
    // D comes out in GPa km^3, alpha in km, a line load V0 in GPa km, and w in km.
    fn earth_e() -> Fixed {
        Fixed::from_int(70)
    }
    fn earth_nu() -> Fixed {
        Fixed::from_ratio(1, 4)
    }
    fn earth_drho() -> Fixed {
        Fixed::from_ratio(33, 10) // 3300 kg/m^3 mantle over an air infill
    }
    fn earth_g() -> Fixed {
        Fixed::from_ratio(98, 10000) // 9.8 m/s^2 = 0.0098 km/s^2
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn flexural_rigidity_matches_the_turcotte_schubert_form() {
        // D = E T_e^3 / (12 (1 - nu^2)); check against an independent f64 evaluation of the same form.
        let t_e = Fixed::from_int(40); // 40 km lid
        let d = flexural_rigidity(earth_e(), earth_nu(), t_e).expect("rigidity");
        let (e, nu, t) = (70.0_f64, 0.25_f64, 40.0_f64);
        let expect = e * t.powi(3) / (12.0 * (1.0 - nu * nu));
        assert!(
            close(d, expect, expect * 1e-6),
            "D = {} vs expected {expect}",
            d.to_f64_lossy()
        );
    }

    #[test]
    fn flexural_parameter_is_the_derived_length_scale() {
        // alpha = (4 D / (delta_rho g))^(1/4); check against the independent f64 quarter-power.
        let t_e = Fixed::from_int(40);
        let d = flexural_rigidity(earth_e(), earth_nu(), t_e).expect("rigidity");
        let alpha = flexural_parameter(d, earth_drho(), earth_g()).expect("parameter");
        let d_f = d.to_f64_lossy();
        let expect = (4.0 * d_f / (3.3 * 0.0098)).powf(0.25);
        assert!(
            close(alpha, expect, expect * 1e-5),
            "alpha = {} vs expected {expect}",
            alpha.to_f64_lossy()
        );
        // Earth-like: alpha comes out at a few tens of km, the mountain-belt / foreland-basin band.
        assert!(
            (50.0..120.0).contains(&alpha.to_f64_lossy()),
            "alpha in the planetary flexural band, got {}",
            alpha.to_f64_lossy()
        );
    }

    #[test]
    fn thin_and_thick_lids_flex_at_different_wavelengths() {
        // THE HEADLINE (conditioning line): same call, same materials, different lid thickness -> different
        // derived flexural wavelength. A thin lid flexes at a SHORT alpha, a thick lid at a LONG one, and the
        // ratio follows alpha ~ T_e^(3/4) (since alpha ~ D^(1/4) and D ~ T_e^3).
        let thin_t = Fixed::from_int(10);
        let thick_t = Fixed::from_int(40);
        let d_thin = flexural_rigidity(earth_e(), earth_nu(), thin_t).expect("thin D");
        let d_thick = flexural_rigidity(earth_e(), earth_nu(), thick_t).expect("thick D");
        let a_thin = flexural_parameter(d_thin, earth_drho(), earth_g()).expect("thin alpha");
        let a_thick = flexural_parameter(d_thick, earth_drho(), earth_g()).expect("thick alpha");
        assert!(
            a_thin.to_f64_lossy() < a_thick.to_f64_lossy(),
            "thin lid flexes at a shorter wavelength: thin {} vs thick {}",
            a_thin.to_f64_lossy(),
            a_thick.to_f64_lossy()
        );
        // The ratio is (40/10)^(3/4) = 4^0.75 = 2.828, purely from the derived scaling, no authored wavelength.
        let ratio = a_thick.to_f64_lossy() / a_thin.to_f64_lossy();
        assert!(
            (ratio - 4.0_f64.powf(0.75)).abs() < 0.02,
            "the wavelength ratio follows T_e^(3/4), got {ratio}"
        );
    }

    #[test]
    fn a_zero_load_gives_zero_deflection_everywhere() {
        // The other half of the headline: no load, no deflection. An empty list and a zero-magnitude load both
        // give exactly zero at every query point.
        let inputs = PlateInputs {
            youngs_modulus: earth_e(),
            poisson_ratio: earth_nu(),
            elastic_thickness: Fixed::from_int(30),
            density_contrast: earth_drho(),
            gravity: earth_g(),
        };
        for (qx, qy) in [(0, 0), (50, 0), (0, 120), (200, 200)] {
            let empty = deflection_at(&inputs, &[], Fixed::from_int(qx), Fixed::from_int(qy))
                .expect("empty list resolves");
            assert_eq!(empty, Fixed::ZERO, "empty load list deflects nothing");
        }
        let zero_loads = [
            Load {
                kind: LoadKind::Point,
                magnitude: Fixed::ZERO,
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
            Load {
                kind: LoadKind::LineY,
                magnitude: Fixed::ZERO,
                x: Fixed::from_int(10),
                y: Fixed::ZERO,
            },
        ];
        let w = deflection_at(&inputs, &zero_loads, Fixed::from_int(5), Fixed::from_int(5))
            .expect("zero loads resolve");
        assert_eq!(w, Fixed::ZERO, "a zero-magnitude load deflects nothing");
    }

    #[test]
    fn the_line_load_peaks_at_the_load_and_decays_in_an_oscillation() {
        // The line-load Green's function must peak under the load, decay, cross zero into a forebulge, and
        // oscillate at the scale alpha, the shape a foreland basin plus its peripheral bulge is read from.
        let t_e = Fixed::from_int(40);
        let d = flexural_rigidity(earth_e(), earth_nu(), t_e).expect("D");
        let alpha = flexural_parameter(d, earth_drho(), earth_g()).expect("alpha");
        let v0 = Fixed::from_int(20); // a line load in GPa km
        let w0 = line_load_deflection(v0, alpha, d, Fixed::ZERO).expect("w(0)");
        // Peak at the load: w(0) equals the analytic maximum V0 alpha^3 / (8 D) and is the largest value.
        let expect_w0 = v0.to_f64_lossy() * alpha.to_f64_lossy().powi(3) / (8.0 * d.to_f64_lossy());
        assert!(
            close(w0, expect_w0, expect_w0.abs() * 1e-4),
            "w(0) = {} vs analytic {expect_w0}",
            w0.to_f64_lossy()
        );
        // Monotone decay of the depression from the load out to the first zero crossing (x0 = 3 pi alpha / 4).
        let a = alpha.to_f64_lossy();
        let quarter = line_load_deflection(v0, alpha, d, Fixed::from_ratio((a * 0.5) as i64, 1))
            .expect("w(alpha/2)");
        assert!(
            quarter.to_f64_lossy() < w0.to_f64_lossy() && quarter.to_f64_lossy() > 0.0,
            "the depression decays but stays positive before the zero crossing"
        );
        // At x0 = 3 pi alpha / 4 the deflection crosses zero (moat to forebulge).
        let x0 = 3.0 * std::f64::consts::PI * a / 4.0;
        let at_x0 =
            line_load_deflection(v0, alpha, d, Fixed::from_ratio((x0 * 1000.0) as i64, 1000))
                .expect("w(x0)");
        assert!(
            at_x0.to_f64_lossy().abs() < expect_w0 * 0.02,
            "w(3 pi alpha/4) crosses zero, got {}",
            at_x0.to_f64_lossy()
        );
        // The forebulge: past the zero crossing the deflection is negative (upward), peaking at a few percent of
        // the central depression (the fetch quotes ~4.3 percent).
        let mut fore_min = 0.0_f64;
        let mut xx = x0;
        while xx < x0 + 2.0 * a {
            let wv =
                line_load_deflection(v0, alpha, d, Fixed::from_ratio((xx * 1000.0) as i64, 1000))
                    .expect("w in the forebulge");
            fore_min = fore_min.min(wv.to_f64_lossy());
            xx += a * 0.1;
        }
        let fore_frac = -fore_min / expect_w0;
        assert!(
            (0.02..0.06).contains(&fore_frac),
            "the forebulge peaks at a few percent of the central depression, got {fore_frac}"
        );
    }

    #[test]
    fn the_line_load_matches_the_analytic_form_and_conserves_force() {
        // Numerical twin: the fixed-point line-load profile equals the f64 analytic form point by point.
        let t_e = Fixed::from_int(40);
        let d = flexural_rigidity(earth_e(), earth_nu(), t_e).expect("D");
        let alpha = flexural_parameter(d, earth_drho(), earth_g()).expect("alpha");
        let v0 = Fixed::from_int(20);
        let (v, a, dd) = (20.0_f64, alpha.to_f64_lossy(), d.to_f64_lossy());
        let w0_analytic = v * a.powi(3) / (8.0 * dd);
        let mut xx = 0.0_f64;
        while xx < 6.0 * a {
            let fx =
                line_load_deflection(v0, alpha, d, Fixed::from_ratio((xx * 1000.0) as i64, 1000))
                    .expect("w");
            let big_x = xx / a;
            let analytic = w0_analytic * (-big_x).exp() * (big_x.cos() + big_x.sin());
            assert!(
                (fx.to_f64_lossy() - analytic).abs() < w0_analytic.abs() * 2e-3 + 1e-4,
                "fixed-point {} vs analytic {analytic} at x = {xx}",
                fx.to_f64_lossy()
            );
            xx += a * 0.25;
        }
        // INDEPENDENT physical check (force balance): the deflection integrated over the whole plate returns the
        // load divided by the restoring modulus, integral(w dx) = V0 / (delta_rho g). This is the analytic
        // identity (both tails contribute V0 alpha^4 / (8 D), and alpha^4 = 4 D / (delta_rho g)), not a
        // restatement of the deflection formula. Trapezoidal sum over both sides.
        let step = a * 0.02;
        let mut integral = 0.0_f64;
        let mut prev = line_load_deflection(v0, alpha, d, Fixed::ZERO)
            .expect("w(0)")
            .to_f64_lossy();
        let mut xg = step;
        while xg < 40.0 * a {
            let wv =
                line_load_deflection(v0, alpha, d, Fixed::from_ratio((xg * 1000.0) as i64, 1000))
                    .expect("w")
                    .to_f64_lossy();
            integral += 0.5 * (prev + wv) * step; // one side
            prev = wv;
            xg += step;
        }
        let both_sides = 2.0 * integral;
        let expect = v / (3.3 * 0.0098); // V0 / (delta_rho g)
        assert!(
            (both_sides - expect).abs() < expect * 5e-3,
            "integral(w dx) over the plate = {both_sides} vs V0/(delta_rho g) = {expect}"
        );
    }

    #[test]
    fn the_point_load_kei_matches_reference_values() {
        // The Kelvin function kei anchored at independent tabulated values (Abramowitz & Stegun): kei(0) = -pi/4,
        // kei(1) ~ -0.494991, kei(2) ~ -0.202400. These are literature values, not a restatement of the series.
        assert!(
            close(kelvin_kei(Fixed::ZERO), -std::f64::consts::FRAC_PI_4, 1e-7),
            "kei(0) = -pi/4, got {}",
            kelvin_kei(Fixed::ZERO).to_f64_lossy()
        );
        assert!(
            close(kelvin_kei(Fixed::from_int(1)), -0.494991, 5e-4),
            "kei(1) ~ -0.494991, got {}",
            kelvin_kei(Fixed::from_int(1)).to_f64_lossy()
        );
        assert!(
            close(kelvin_kei(Fixed::from_int(2)), -0.202400, 5e-4),
            "kei(2) ~ -0.202400, got {}",
            kelvin_kei(Fixed::from_int(2)).to_f64_lossy()
        );
        // The function crosses zero near x ~ 3.9 and has a small positive forebulge peaking near x ~ 5; check the
        // sign flip from the depression side (kei(3) ~ -0.051) to the forebulge (kei(5) ~ +0.011).
        assert!(
            kelvin_kei(Fixed::from_int(3)).to_f64_lossy() < 0.0,
            "kei is negative on the depression side at 3, got {}",
            kelvin_kei(Fixed::from_int(3)).to_f64_lossy()
        );
        assert!(
            kelvin_kei(Fixed::from_int(5)).to_f64_lossy() > 0.0,
            "kei has risen into its positive forebulge by 5, got {}",
            kelvin_kei(Fixed::from_int(5)).to_f64_lossy()
        );
    }

    #[test]
    fn the_point_load_central_deflection_is_the_analytic_magnitude() {
        // w(0) = Q0 l^2 / (2 pi D) * kei(0) = -Q0 l^2 / (8 D), on the AXISYMMETRIC length l = alpha/sqrt(2), so
        // the central depression is -Q0 alpha^2 / (16 D), HALF the old (wrong) -Q0 alpha^2 / (8 D). This assertion
        // mirrors the code's own amplitude and so does not by itself catch a length-scale error; the zero-crossing
        // test below does, from a physical property the algebra cannot fake.
        let t_e = Fixed::from_int(40);
        let d = flexural_rigidity(earth_e(), earth_nu(), t_e).expect("D");
        let alpha = flexural_parameter(d, earth_drho(), earth_g()).expect("alpha");
        let l = flexural_length_axisymmetric(d, earth_drho(), earth_g()).expect("l");
        let q0 = Fixed::from_int(500);
        let w_center = point_load_deflection(q0, alpha, d, Fixed::ZERO).expect("w(0)");
        let expect = -500.0 * l.to_f64_lossy().powi(2) / (8.0 * d.to_f64_lossy());
        assert!(
            close(w_center, expect, expect.abs() * 1e-4),
            "central point-load deflection {} vs analytic {expect}",
            w_center.to_f64_lossy()
        );
        // Radial decay: the depression magnitude shrinks moving out from the load (before the forebulge).
        let w_out = point_load_deflection(q0, alpha, d, alpha).expect("w(alpha)");
        assert!(
            w_out.to_f64_lossy().abs() < w_center.to_f64_lossy().abs(),
            "the point-load depression decays radially"
        );
    }

    #[test]
    fn the_point_load_first_zero_crossing_is_at_the_axisymmetric_length() {
        // THE NON-CIRCULAR CHECK the amplitude test cannot be: the deflection's first zero crossing is a PHYSICAL
        // property, the first zero of kei, at r/l = 3.91467 where l = (D/dpg)^(1/4). The old code put it at
        // r = 3.91467 alpha, sqrt(2) too far out; the fix puts it at r = 3.91467 l = 3.91467 alpha/sqrt(2). This
        // pins the LENGTH SCALE, not the amplitude, so it fails for the sqrt(2)-wrong length regardless of the
        // coefficient (which is exactly how the length bug hid: the central-magnitude test mirrors the code's own
        // algebra and passed either way). x0 = 3.91467 is the cited first zero of kei (A&S 1965).
        let t_e = Fixed::from_int(40);
        let d = flexural_rigidity(earth_e(), earth_nu(), t_e).expect("D");
        let alpha = flexural_parameter(d, earth_drho(), earth_g()).expect("alpha");
        let l = flexural_length_axisymmetric(d, earth_drho(), earth_g()).expect("l");
        let q0 = Fixed::from_int(500);
        let x0 = Fixed::from_ratio(391467, 100000); // 3.91467, the first zero of kei
                                                    // At the correct zero (r = x0 * l) the deflection vanishes to the fixed-point floor of kei's own zero.
        let r_correct = x0.checked_mul(l).unwrap();
        let w_zero = point_load_deflection(q0, alpha, d, r_correct).expect("w(x0 l)");
        let w_center = point_load_deflection(q0, alpha, d, Fixed::ZERO).expect("w(0)");
        assert!(
            w_zero.to_f64_lossy().abs() < w_center.to_f64_lossy().abs() * 1e-3,
            "the deflection vanishes at r = x0 l = {}, got {} against a central {}",
            r_correct.to_f64_lossy(),
            w_zero.to_f64_lossy(),
            w_center.to_f64_lossy()
        );
        // And at the OLD (wrong) location r = x0 * alpha it is decidedly NOT zero: this is what the sqrt(2) error
        // shipped, and this half of the assertion is what a regression to it would trip.
        let r_wrong = x0.checked_mul(alpha).unwrap();
        let w_wrong = point_load_deflection(q0, alpha, d, r_wrong).expect("w(x0 alpha)");
        assert!(
            w_wrong.to_f64_lossy().abs() > w_center.to_f64_lossy().abs() * 1e-2,
            "the sqrt(2)-wrong zero location must not read as a zero"
        );
    }

    #[test]
    fn deflection_at_sums_the_load_list_order_independently() {
        // The evaluator superposes: the total equals the sum of the individual Green's-function contributions,
        // and the sum does not depend on the load order (Fixed addition is exact and associative).
        let inputs = PlateInputs {
            youngs_modulus: earth_e(),
            poisson_ratio: earth_nu(),
            elastic_thickness: Fixed::from_int(35),
            density_contrast: earth_drho(),
            gravity: earth_g(),
        };
        let d = inputs.rigidity().unwrap();
        let alpha = inputs.parameter().unwrap();
        let loads = [
            Load {
                kind: LoadKind::Point,
                magnitude: Fixed::from_int(400),
                x: Fixed::from_int(0),
                y: Fixed::from_int(0),
            },
            Load {
                kind: LoadKind::LineY,
                magnitude: Fixed::from_int(15),
                x: Fixed::from_int(60),
                y: Fixed::ZERO,
            },
            Load {
                kind: LoadKind::Point,
                magnitude: Fixed::from_int(250),
                x: Fixed::from_int(-40),
                y: Fixed::from_int(30),
            },
        ];
        let (qx, qy) = (Fixed::from_int(10), Fixed::from_int(10));
        let total = deflection_at(&inputs, &loads, qx, qy).expect("total");
        // Hand-sum the three contributions.
        let r0 = ((10.0_f64).powi(2) + (10.0_f64).powi(2)).sqrt();
        let p0 = point_load_deflection(
            Fixed::from_int(400),
            alpha,
            d,
            Fixed::from_ratio((r0 * 1000.0) as i64, 1000),
        )
        .unwrap();
        let l1 =
            line_load_deflection(Fixed::from_int(15), alpha, d, Fixed::from_int(10 - 60)).unwrap();
        let r2 = ((10.0_f64 + 40.0).powi(2) + (10.0_f64 - 30.0).powi(2)).sqrt();
        let p2 = point_load_deflection(
            Fixed::from_int(250),
            alpha,
            d,
            Fixed::from_ratio((r2 * 1000.0) as i64, 1000),
        )
        .unwrap();
        let manual = p0.to_f64_lossy() + l1.to_f64_lossy() + p2.to_f64_lossy();
        assert!(
            close(total, manual, 1e-3),
            "superposition {} vs hand-sum {manual}",
            total.to_f64_lossy()
        );
        // Order independence: reverse the list, same total to the bit.
        let mut rev = loads;
        rev.reverse();
        let total_rev = deflection_at(&inputs, &rev, qx, qy).expect("total reversed");
        assert_eq!(total, total_rev, "the deflection sum is order-independent");
    }

    #[test]
    fn the_kernel_fails_loud_on_degenerate_inputs() {
        // No fabricated value on a degenerate input: each guard returns None.
        assert!(
            flexural_rigidity(Fixed::ZERO, earth_nu(), Fixed::from_int(30)).is_none(),
            "zero E"
        );
        assert!(
            flexural_rigidity(earth_e(), earth_nu(), Fixed::ZERO).is_none(),
            "zero T_e"
        );
        // |nu| >= 1 makes 1 - nu^2 <= 0 (outside the physical Poisson range): refused.
        assert!(
            flexural_rigidity(earth_e(), Fixed::from_int(1), Fixed::from_int(30)).is_none(),
            "nu = 1"
        );
        let d = flexural_rigidity(earth_e(), earth_nu(), Fixed::from_int(30)).unwrap();
        assert!(
            flexural_parameter(d, Fixed::ZERO, earth_g()).is_none(),
            "zero delta_rho"
        );
        assert!(
            flexural_parameter(d, earth_drho(), Fixed::ZERO).is_none(),
            "zero g"
        );
        assert!(
            flexural_parameter(Fixed::ZERO, earth_drho(), earth_g()).is_none(),
            "zero D"
        );
        assert!(
            line_load_deflection(Fixed::from_int(10), Fixed::ZERO, d, Fixed::ZERO).is_none(),
            "zero alpha"
        );
        assert!(
            point_load_deflection(
                Fixed::from_int(10),
                Fixed::from_int(50),
                d,
                Fixed::from_int(-1)
            )
            .is_none(),
            "negative r"
        );
    }

    #[test]
    fn the_kelvin_kei_reproduces_the_ascending_series_numerical_twin() {
        // Numerical twin: the fixed-point kei equals an independent f64 evaluation of the same A&S 9.9 series
        // (including the Euler-Mascheroni gamma) across the near-to-mid field, catching any fixed-point
        // precision or overflow drift.
        let gamma = 0.5772156649_f64;
        let kei_f64 = |x: f64| -> f64 {
            if x <= 0.0 {
                return -std::f64::consts::FRAC_PI_4;
            }
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
        let mut x = 0.5_f64;
        while x <= 11.5 {
            let fp = kelvin_kei(Fixed::from_ratio((x * 1000.0) as i64, 1000)).to_f64_lossy();
            assert!(
                (fp - kei_f64(x)).abs() < 5e-4,
                "kei fixed-point {fp} vs f64 twin {} at x = {x}",
                kei_f64(x)
            );
            x += 0.5;
        }
    }

    #[test]
    fn the_kelvin_ker_matches_reference_values() {
        // ker anchored at INDEPENDENT tabulated values (Abramowitz & Stegun 9.10; cross-checked here with
        // scipy.special.kelvin and mpmath, two libraries with different algorithms that agree to seven
        // digits). These are literature values, not a restatement of this module's own series.
        //   ker(1) ~  0.2867062, ker(2) ~ -0.0416645, ker(3) ~ -0.0670292, ker(3.91467) ~ -0.0388994.
        let at = |num: i64, den: i64| kelvin_ker(Fixed::from_ratio(num, den)).to_f64_lossy();
        assert!(
            close(kelvin_ker(Fixed::from_int(1)), 0.2867062, 5e-5),
            "ker(1) ~ 0.2867062, got {}",
            at(1, 1)
        );
        assert!(
            close(kelvin_ker(Fixed::from_int(2)), -0.0416645, 5e-5),
            "ker(2) ~ -0.0416645, got {}",
            at(2, 1)
        );
        assert!(
            close(kelvin_ker(Fixed::from_int(3)), -0.0670292, 5e-5),
            "ker(3) ~ -0.0670292, got {}",
            at(3, 1)
        );
        // THE VALUE AT THE FIRST ZERO CROSSING, which is the axisymmetric plate's reported curvature
        // coefficient and the site of McNutt and Menard's printed erratum. The re-derived value is
        // -0.0388994; the paper prints a curvature that back-solves to -0.0289, about 26 per cent low. The
        // fixed-point series lands on the re-derived value, not the printed one, which is the whole point of
        // recomputing the constant rather than copying it.
        let ker_x0 = kelvin_ker(Fixed::from_ratio(391467, 100000)).to_f64_lossy();
        assert!(
            (ker_x0 - (-0.0388994)).abs() < 5e-5,
            "ker(3.91467) ~ -0.0388994 (re-derived), got {ker_x0}"
        );
        assert!(
            (ker_x0 - (-0.0289)).abs() > 5e-3,
            "the re-derived value is decisively NOT the paper's printed -0.0289: {ker_x0}"
        );
        // A NON-POSITIVE ARGUMENT FAILS LOUD: ker has a logarithmic singularity at the origin.
        assert_eq!(kelvin_ker(Fixed::ZERO), Fixed::MIN);
        assert_eq!(kelvin_ker(Fixed::from_int(-1)), Fixed::MIN);
    }

    #[test]
    fn the_kelvin_ker_reproduces_the_ascending_series_numerical_twin() {
        // Numerical twin: the fixed-point ker equals an independent f64 evaluation of the same A&S 9.9 series
        // (including the Euler-Mascheroni gamma and the H_{2k} harmonic weights), catching any fixed-point
        // precision or overflow drift across the near-to-mid field.
        let gamma = 0.5772156649_f64;
        let ker_f64 = |x: f64| -> f64 {
            let xh = x / 2.0;
            let xh4 = xh.powi(4);
            let (mut b, mut c) = (1.0_f64, xh * xh);
            let (mut ber, mut bei) = (b, c);
            let mut h_even = 0.0_f64;
            let mut ber_phi = 0.0_f64;
            for k in 1..40 {
                let mb = ((2 * k) * (2 * k - 1)) as f64;
                b = -b * xh4 / (mb * mb);
                ber += b;
                let mc = ((2 * k + 1) * (2 * k)) as f64;
                c = -c * xh4 / (mc * mc);
                bei += c;
                h_even += 1.0 / (2 * k - 1) as f64 + 1.0 / (2 * k) as f64;
                ber_phi += b * h_even;
            }
            -(xh.ln() + gamma) * ber + std::f64::consts::FRAC_PI_4 * bei + ber_phi
        };
        let mut x = 0.5_f64;
        while x <= 11.5 {
            let fp = kelvin_ker(Fixed::from_ratio((x * 1000.0) as i64, 1000)).to_f64_lossy();
            assert!(
                (fp - ker_f64(x)).abs() < 5e-4,
                "ker fixed-point {fp} vs f64 twin {} at x = {x}",
                ker_f64(x)
            );
            x += 0.5;
        }
    }

    #[test]
    fn the_kelvin_functions_satisfy_the_laplacian_identity() {
        // THE LOAD-BEARING STRUCTURAL CHECK, and it is independent by construction: the axisymmetric moment
        // equivalence rests on `nabla^2 kei = ker`, i.e. `kei''(x) + (1/x) kei'(x) = ker(x)` (the identity
        // that turns the point-load deflection's Laplacian into `-(P/2 pi D) ker`, and the one the fetch
        // verified numerically). The LEFT side is formed by SECOND-DIFFERENCING an independent f64 evaluation
        // of kei's own A&S series (a different route from the fixed-point `kelvin_ker`), and the RIGHT side is
        // the fixed-point `kelvin_ker`. They share no arithmetic, so agreement is evidence rather than a
        // series checked against itself. The finite difference runs on the SMOOTH f64 series rather than the
        // fixed-point `kelvin_kei` on purpose: a second difference divides by `h^2`, which would amplify the
        // fixed-point quantization (about `1e-9`) into the answer, so differencing the exact f64 form isolates
        // the identity from the representation. This is the Kelvin analogue of the line load's force-balance
        // identity: a relation the functions must satisfy, not a restatement of either one.
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
        // h swept to the O(h^2) plateau: on the exact f64 form the only errors are truncation (O(h^2)) and
        // f64 roundoff (O(eps/h^2) ~ 2e-8 at h=1e-3), so 1e-3 sits in the plateau far below the tolerance.
        let h = 1e-3_f64;
        for &x in &[1.5_f64, 2.5, 3.91467, 5.0, 6.5] {
            let kei_pp = (kei_f64(x + h) - 2.0 * kei_f64(x) + kei_f64(x - h)) / (h * h);
            let kei_p = (kei_f64(x + h) - kei_f64(x - h)) / (2.0 * h);
            let laplacian = kei_pp + kei_p / x;
            let ker = kelvin_ker(Fixed::from_ratio((x * 1e6) as i64, 1_000_000)).to_f64_lossy();
            assert!(
                (laplacian - ker).abs() < 5e-4,
                "nabla^2 kei = ker at x = {x}: {laplacian} (finite-diff of kei) against {ker} (ker series)"
            );
        }
    }

    #[test]
    fn the_kelvin_kei_prime_matches_reference_values() {
        // kei' anchored at INDEPENDENT values (scipy.special.kelvin's Kep imaginary part, cross-checked with
        // mpmath's derivative). Literature-grade, not a restatement of the derivative series.
        //   kei'(1) ~ 0.3523699, kei'(2) ~ 0.2198079, kei'(3.91467) ~ 0.0276690.
        assert!(
            close(kelvin_kei_prime(Fixed::from_int(1)), 0.3523699, 5e-5),
            "kei'(1) ~ 0.3523699, got {}",
            kelvin_kei_prime(Fixed::from_int(1)).to_f64_lossy()
        );
        assert!(
            close(kelvin_kei_prime(Fixed::from_int(2)), 0.2198079, 5e-5),
            "kei'(2) ~ 0.2198079, got {}",
            kelvin_kei_prime(Fixed::from_int(2)).to_f64_lossy()
        );
        let keip_x0 = kelvin_kei_prime(Fixed::from_ratio(391467, 100000)).to_f64_lossy();
        assert!(
            (keip_x0 - 0.0276690).abs() < 5e-5,
            "kei'(3.91467) ~ 0.0276690, got {keip_x0}"
        );
        // THE ARCH PEAK, where kei is extremal so kei' vanishes: kei'(4.93181) ~ 0. An independent anchor
        // (the peak location is a cited root, A&S), and it pins the derivative's zero rather than its value.
        let keip_peak = kelvin_kei_prime(Fixed::from_ratio(493181, 100000)).to_f64_lossy();
        assert!(
            keip_peak.abs() < 5e-5,
            "kei'(4.93181) ~ 0 at the arch peak, got {keip_peak}"
        );
        // Positive on the depression side (kei rising through its first zero crossing), which is the sign the
        // hoop curvature carries into the axisymmetric moment.
        assert!(
            keip_x0 > 0.0,
            "kei' is positive where kei rises through zero, got {keip_x0}"
        );
    }

    #[test]
    fn the_kelvin_kei_prime_reproduces_the_derivative_series_numerical_twin() {
        // Numerical twin: the fixed-point kei' equals an independent f64 evaluation of the term-by-term
        // derivative of the A&S 9.9 series (a different code path from the fixed-point recurrence), catching
        // precision or overflow drift.
        let gamma = 0.5772156649_f64;
        let keip_f64 = |x: f64| -> f64 {
            let xh = x / 2.0;
            let xh4 = xh.powi(4);
            let (mut b, mut c) = (1.0_f64, xh * xh);
            let mut bei = c;
            let mut berp = 0.0_f64;
            let mut beip = c / xh; // (1/xh) c_0
            let mut h_odd = 1.0_f64;
            let mut beiphip = beip; // (1/xh) c_0 H_1
            for k in 1..40 {
                let mb = ((2 * k) * (2 * k - 1)) as f64;
                b = -b * xh4 / (mb * mb);
                berp += (2 * k) as f64 * b / xh;
                let mc = ((2 * k + 1) * (2 * k)) as f64;
                c = -c * xh4 / (mc * mc);
                bei += c;
                let factor = (2 * k + 1) as f64 / xh;
                beip += factor * c;
                h_odd += 1.0 / (2 * k) as f64 + 1.0 / (2 * k + 1) as f64;
                beiphip += factor * c * h_odd;
            }
            -(1.0 / x) * bei - (xh.ln() + gamma) * beip - std::f64::consts::FRAC_PI_4 * berp
                + beiphip
        };
        let mut x = 0.5_f64;
        while x <= 11.5 {
            let fp = kelvin_kei_prime(Fixed::from_ratio((x * 1000.0) as i64, 1000)).to_f64_lossy();
            assert!(
                (fp - keip_f64(x)).abs() < 5e-4,
                "kei' fixed-point {fp} vs f64 twin {} at x = {x}",
                keip_f64(x)
            );
            x += 0.5;
        }
    }

    #[test]
    fn the_flexural_filter_halves_at_the_characteristic_wavelength() {
        // At l*k = 1 (the wavelength 2 pi l, the corner of the filter), Phi = 1 / (1 + 1) = 1/2. This is the
        // hand-verifiable anchor: the filter's corner sits exactly at the flexural length. l = 2, k = 1/2 keep
        // l*k = 1 EXACTLY on the Q32.32 grid (1/2 is representable), so the corner lands with no rounding.
        let l = Fixed::from_int(2);
        let k = Fixed::from_ratio(1, 2); // l*k = 1 exactly
        let phi = flexural_response_ratio(l, k).expect("the filter evaluates");
        assert!(
            close(phi, 0.5, 1e-9),
            "Phi at the corner is 1/2, got {}",
            phi.to_f64_lossy()
        );
    }

    #[test]
    fn the_flexural_filter_passes_long_wavelengths_and_stops_short_ones() {
        let l = Fixed::from_int(10);
        // A long-wavelength load (l*k = 0.1) is nearly fully compensated: Phi = 1 / (1 + 1e-4) ~ 0.9999.
        let phi_long = flexural_response_ratio(l, Fixed::from_ratio(1, 100)).expect("long");
        assert!(
            close(phi_long, 1.0, 1e-3),
            "a long-wavelength load passes nearly full relief, got {}",
            phi_long.to_f64_lossy()
        );
        // A short-wavelength load (l*k = 10) is held by the plate: Phi = 1 / (1 + 1e4) ~ 1e-4.
        let phi_short = flexural_response_ratio(l, Fixed::from_int(1)).expect("short");
        assert!(
            phi_short < Fixed::from_ratio(1, 1000),
            "a short-wavelength load passes almost no relief, got {}",
            phi_short.to_f64_lossy()
        );
        // The filter is monotone in k: a longer wavelength always passes more than a shorter one.
        assert!(
            phi_long > phi_short,
            "the flexural filter is monotone decreasing in wavenumber"
        );
    }

    #[test]
    fn the_flexural_filter_dc_term_is_isostatic_and_it_fails_loud() {
        let l = Fixed::from_int(10);
        // The k = 0 (infinite-wavelength) term is fully compensated.
        assert_eq!(
            flexural_response_ratio(l, Fixed::ZERO),
            Some(Fixed::ONE),
            "the DC term floats isostatically"
        );
        // Degenerate inputs fail loud, never a fabricated ratio.
        assert!(
            flexural_response_ratio(Fixed::ZERO, Fixed::from_int(1)).is_none(),
            "a non-positive flexural length fails loud"
        );
        assert!(
            flexural_response_ratio(l, Fixed::from_int(-1)).is_none(),
            "a negative wavenumber fails loud"
        );
    }
}
