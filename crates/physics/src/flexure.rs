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
//! THE PUBLIC CONTRACT IS THE CALLER'S. Every input and every output is a plain fixed-point number in the
//! caller's own coherent unit system, and `D` and `alpha` come out in the units that system induces. The
//! caller owns the choice, because the Q32.32 window (about +/- 2.1e9, resolution ~2.3e-10) will not hold `D`
//! in raw SI (Earth's `D ~ 1e23 N m` overflows). A coherent system that keeps flexure's OUTPUTS in range is
//! `{length = km, mass = 1e12 kg, time = s}`, which induces stress in GPa, density in `1000 kg/m^3` (so a
//! `3300 kg/m^3` mantle reads 3.3), and gravity in `km/s^2` (so `9.8 m/s^2` reads 0.0098). In it `D` is in
//! `GPa km^3`, the restoring modulus `delta_rho g` in `GPa/km`, `alpha` and `w` in km, and a line load `V0` in
//! `GPa km`. The tests use exactly this system.
//!
//! THE ARITHMETIC IS NOT THE CALLER'S, and that is the change this module made in 2026-07. Keeping the OUTPUTS
//! in range never kept the INTERMEDIATES in range: on a thick-lid low-gravity world `alpha^3` reaches `7e9`,
//! `8 D` passes `Fixed::MAX` for any `D` above `2.68e8`, and `D / (delta_rho g)` reaches `7.6e9`, each for a
//! plate whose deflection is a few entirely representable kilometres. Four local fallbacks were landed against
//! four of those sites and the chain refused at a fifth, which is what point-fixing looks like when the
//! problem is the SCALE. So the arithmetic runs in one coherent INTERNAL system,
//! `{L0 = 32 km, t0 = 32 s, S0 = 1 GPa, rho0 = 1000 kg/m^3}` ([`scaled`]), converting only at the boundary,
//! with every scale a power of two. See `docs/working/FLEXURE_REPRESENTATION_DESIGN.md` for the worked range
//! table the choice is proven against.
//!
//! THE ENVELOPE THAT MAKES THAT A PROOF is declared and enforced: [`MAX_YOUNGS_MODULUS_GPA`],
//! [`max_poisson_ratio_magnitude`], [`MAX_ELASTIC_THICKNESS_KM`] and [`MAX_LINE_LOAD_GPA_KM`]. Outside it the
//! kernel refuses rather than returning a number the range table does not cover. The kernel still guards every
//! step with checked arithmetic and fails loud (`None`) rather than wrapping, so a badly-scaled unit choice is
//! refused, never silently corrupted.
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

// ----- THE VALIDATED OPERATING ENVELOPE (the range table's own premises, enforced) -----

/// The largest Young's modulus (GPa) the range table is proven over.
///
/// A DECLARED VALIDATION ENVELOPE, NEVER A PHYSICAL CONSTANT. The design's range proof needs a bound on the
/// elastic constants to be a proof at all (`E T_hat^3` is what has to fit, and `T_hat^3` alone reaches
/// 15625), and it states the contract it proved: `E <= 512 GPa`, `|nu| <= 0.5`. Both are generous against
/// any real material (diamond is about 1050 GPa and is not a lithosphere; olivine is near 200), and 512 is a
/// power of two so the bound itself lands exactly on the grid. Outside it this kernel REFUSES rather than
/// returning a number the range table does not cover, which is the difference between a proof and a hope.
pub const MAX_YOUNGS_MODULUS_GPA: i32 = 512;

/// The largest `|nu|` the range table is proven over, `0.5`.
///
/// It coincides with the physical incompressible limit for an isotropic elastic solid, so the declared
/// validation envelope and the physics agree here rather than the bound being a numerical convenience: an
/// isotropic material with `nu > 0.5` has a negative bulk modulus. It also subsumes the older `|nu| < 1`
/// guard, since `|nu| <= 0.5` gives `1 - nu^2 >= 0.75`.
pub fn max_poisson_ratio_magnitude() -> Fixed {
    Fixed::from_ratio(1, 2)
}

/// The largest elastic thickness (km) the range table is proven over, `T_hat <= 25`.
///
/// The design's requested range is `T_e = 5 .. 800 km`, and 800 km is far past any lithosphere: it is the
/// FULLY ELASTIC starting trial of the thickest derived conductive lid this arc has met (739.4 km on a
/// sluggish Mars-class world), never a plate anyone expects to find. A thickness above it refuses.
pub const MAX_ELASTIC_THICKNESS_KM: i32 = 800;

/// The largest LINE-LOAD intensity (`GPa km`) the range table is proven over, the design's `0.1 .. 500`
/// requested load range read at its top.
///
/// THIS BOUND IS ARITHMETICALLY LOAD-BEARING, which is what separates it from the point load's below. The
/// line-load amplitude forms `V_hat alpha_hat^3`, and `alpha_hat^3` alone reaches `2.13e5` over the declared
/// thickness and restoring ranges, so the product's `3.33e6` is provable only against a bound on `V_hat`.
///
/// ONLY THE UPPER BOUND IS ENFORCED, and that is deliberate. Every quantity the load enters is linear in its
/// magnitude, so a SMALLER load is strictly safer and the requested range's lower end is a statement about
/// what callers send rather than a bound the arithmetic needs. A zero load must keep giving zero deflection,
/// so a floor here would refuse the unloaded plate for no reason at all.
pub const MAX_LINE_LOAD_GPA_KM: i32 = 500;

/// The largest POINT-LOAD magnitude (`GPa km^2`) the range table is proven over, `2^26`.
///
/// # THIS ONE IS DERIVED, BECAUSE THE DESIGN'S GUESS AT IT IS REFUTED
///
/// The design could not supply a point-load range and said so: "If the intended point-load range is also
/// numerically `0.1..500 GPa km^2`, then `P_hat <= 0.4883`... If point loads have another range, that bound
/// must be supplied." Declaring 500 would have REFUSED THIS MODULE'S OWN REFERENCE TWIN, which reads McNutt
/// and Menard's printed seamount moment at `P = 1000`, and its adversarial load-exceeds-support case at
/// `P = 5000`. Fitting the bound to those tests instead would be the same mistake from the other side, so it
/// is taken from the range table rather than from either.
///
/// The point load's binding product is `P_hat l_hat^2`, and `l_hat^2` tops out at `1782.95` over the declared
/// thickness and restoring ranges. Allowing the same order of magnitude of headroom below `Fixed::MAX` that
/// the design allows every other row gives `P_hat <= 1.2e5`; rounded DOWN to a power of two so `P_hat` lands
/// exactly, `P_hat <= 65536`, which is `P <= 2^26 GPa km^2`.
///
/// THE HONEST READING OF THAT NUMBER: the point-load path has no tight range constraint. Its product carries
/// four orders of headroom at every load anyone would send, the guard exists so the kernel cannot answer
/// outside a domain the table covers, and the design's guessed 500 was two orders too tight in the direction
/// that would have broken working physics.
pub const MAX_POINT_LOAD_GPA_KM2: i32 = 67_108_864;

/// Whether an elastic-constant pair sits inside the declared validation envelope, positive and bounded.
fn elastic_constants_admissible(e: Fixed, nu: Fixed) -> bool {
    e > Fixed::ZERO
        && e <= Fixed::from_int(MAX_YOUNGS_MODULUS_GPA)
        && nu.abs() <= max_poisson_ratio_magnitude()
}

/// Whether a LINE-load intensity sits inside the declared validation envelope ([`MAX_LINE_LOAD_GPA_KM`]).
pub(crate) fn line_load_admissible(magnitude: Fixed) -> bool {
    magnitude.abs() <= Fixed::from_int(MAX_LINE_LOAD_GPA_KM)
}

/// Whether a POINT-load magnitude sits inside the declared validation envelope
/// ([`MAX_POINT_LOAD_GPA_KM2`]).
pub(crate) fn point_load_admissible(magnitude: Fixed) -> bool {
    magnitude.abs() <= Fixed::from_int(MAX_POINT_LOAD_GPA_KM2)
}

/// THE INTERNAL UNIT SYSTEM AND THE SCALED KERNELS, private to the crate.
///
/// The public functions of this module are thin boundaries over these: they convert in, call one kernel,
/// and convert out. `moment_equivalence` reaches them directly where it needs a quantity the caller's own
/// units cannot hold, which is how a fully elastic 800 km starting trial exists at all.
pub(crate) mod scaled {
    use super::{elastic_constants_admissible, Fixed};

    // ----- THE INTERNAL UNIT SYSTEM (one coherent rescaling, not a family of local fallbacks) -----

    /// THE INTERNAL LENGTH SCALE `L0 = 32 km`, the base of the coherent system every flexure intermediate is
    /// computed in (`docs/working/FLEXURE_REPRESENTATION_DESIGN.md`).
    ///
    /// # WHY A SECOND UNIT SYSTEM EXISTS INSIDE A UNIT-AGNOSTIC KERNEL
    ///
    /// The kernel's PUBLIC contract is the caller's coherent system (`{km, GPa, 1000 kg/m^3, s}`; see the module
    /// unit contract), and that contract does not change. What changed is where the arithmetic happens. In the
    /// caller's units a planetary-scale plate makes INTERMEDIATES that leave the Q32.32 window while every input
    /// and every output stays comfortably inside it: `alpha^3` reaches `7e9` for a thick lid whose `alpha` is
    /// 1911 km, `8 D` passes `Fixed::MAX` for `D > 2.684e8`, `D / (delta_rho g)` reaches `7.6e9` on a low-gravity
    /// world. Four local fallbacks were landed against four of those sites and the chain still refused at a
    /// fifth, which is what a point-fix strategy looks like when the problem is the SCALE rather than any one
    /// expression.
    ///
    /// So the arithmetic moves once, coherently, into `{L0 = 32 km, t0 = 32 s, S0 = 1 GPa, rho0 = 1000 kg/m^3}`.
    /// The system is coherent (`rho0 L0^2 / t0^2 = 1 GPa` exactly), so no conversion factor appears inside any
    /// formula: only at the boundary. Every scale is a POWER OF TWO, so each boundary conversion is a
    /// deterministic shift or a division by a power of two rather than a rounding-sensitive multiply.
    ///
    /// The 32 km choice is the design's, and its basis is the range table it proves: it puts the requested
    /// thickness range at `T_hat = 0.15625 .. 25`, the requested rigidity range at `D_hat = 0.305 .. 30518`, and
    /// the flexural parameter at `alpha_hat <= 138.7` even for an 800 km fully elastic starting trial, so every
    /// product, cube and quotient in the chain has at least an order of magnitude of headroom.
    pub(crate) const INTERNAL_LENGTH_KM: i32 = 32;

    /// THE INTERNAL RIGIDITY UNIT `S0 L0^3 = 32768 GPa km^3`, so `D_hat = D / 32768`. `32768 = 2^15`.
    pub(crate) const INTERNAL_RIGIDITY_GPA_KM3: i32 = 32768;

    /// THE INTERNAL LINE-LOAD UNIT `S0 L0 = 32 GPa km`, so `V_hat = V0 / 32`. A line load is a force per unit
    /// length, which is a stress times a length.
    pub(crate) const INTERNAL_LINE_LOAD_GPA_KM: i32 = 32;

    /// THE INTERNAL FORCE UNIT `S0 L0^2 = 1024 GPa km^2`, shared by the point load and the bending moment per
    /// unit length, which carry the same dimensions (a moment per unit length is a force). `1024 = 2^10`.
    pub(crate) const INTERNAL_FORCE_GPA_KM2: i32 = 1024;

    /// A length (km) in internal lengths: `x_hat = x / 32`.
    pub(crate) fn internal_length(km: Fixed) -> Option<Fixed> {
        km.checked_div(Fixed::from_int(INTERNAL_LENGTH_KM))
    }

    /// An internal length back to km: `x = 32 x_hat`. Fails loud where the answer leaves the window.
    pub(crate) fn external_length(hat: Fixed) -> Option<Fixed> {
        hat.checked_mul(Fixed::from_int(INTERNAL_LENGTH_KM))
    }

    /// A gravity (`km/s^2`) in internal accelerations: `g_hat = g / (L0/t0^2) = 32 g`, since
    /// `L0 / t0^2 = 32 km / (32 s)^2 = 0.03125 km/s^2`.
    pub(crate) fn internal_gravity(km_per_s2: Fixed) -> Option<Fixed> {
        km_per_s2.checked_mul(Fixed::from_int(INTERNAL_LENGTH_KM))
    }

    /// A rigidity (`GPa km^3`) in internal rigidities: `D_hat = D / 32768`.
    pub(crate) fn internal_rigidity(gpa_km3: Fixed) -> Option<Fixed> {
        gpa_km3.checked_div(Fixed::from_int(INTERNAL_RIGIDITY_GPA_KM3))
    }

    /// An internal rigidity back to `GPa km^3`. Fails loud where the answer leaves the window, which is the
    /// honest report for a plate stiffer than the caller's own unit can express (an 800 km fully elastic lid at
    /// `E = 512 GPa` is `2.9e10 GPa km^3`, and no rescaling of the INTERNAL arithmetic can make that fit an
    /// EXTERNAL `Fixed`).
    pub(crate) fn external_rigidity(hat: Fixed) -> Option<Fixed> {
        hat.checked_mul(Fixed::from_int(INTERNAL_RIGIDITY_GPA_KM3))
    }

    /// A line-load intensity (`GPa km`) in internal line loads: `V_hat = V0 / 32`.
    pub(crate) fn internal_line_load(gpa_km: Fixed) -> Option<Fixed> {
        gpa_km.checked_div(Fixed::from_int(INTERNAL_LINE_LOAD_GPA_KM))
    }

    /// A point load or a moment per unit length (`GPa km^2`) in internal forces: `P_hat = P / 1024`.
    pub(crate) fn internal_force(gpa_km2: Fixed) -> Option<Fixed> {
        gpa_km2.checked_div(Fixed::from_int(INTERNAL_FORCE_GPA_KM2))
    }

    /// An internal curvature (`1/L0`) back to the caller's `1/km`: `kappa = kappa_hat / 32`.
    ///
    /// A curvature is a reciprocal length, so it scales the other way from a length: the internal form is 32
    /// times LARGER than the caller's, which is why the moment-equivalence arithmetic forms it internally and
    /// converts here rather than the reverse.
    pub(crate) fn external_curvature(hat: Fixed) -> Option<Fixed> {
        hat.checked_div(Fixed::from_int(INTERNAL_LENGTH_KM))
    }

    // ----- THE SCALED KERNELS (the arithmetic, in internal units, with no boundary conversion inside) -----

    /// The flexural rigidity in INTERNAL units: `D_hat = E T_hat^3 / (12 (1 - nu^2))`.
    ///
    /// The range, over the declared envelope: `T_hat <= 25` gives `T_hat^3 <= 15625`, `E T_hat^3 <= 8.0e6`, and
    /// `12 (1 - nu^2) >= 9`, so `D_hat <= 8.89e5`, two and a half orders inside the window. The EXTERNAL
    /// rigidity of that same plate is `2.9e10 GPa km^3` and does not fit, which is why the fully elastic trial
    /// of a very thick lid can only ever exist here, in internal units, and never as a returned `D`.
    // @derives: the flexural rigidity in internal units <- Youngs modulus, Poisson ratio and the elastic thickness
    pub(crate) fn scaled_rigidity(e: Fixed, nu: Fixed, t_hat: Fixed) -> Option<Fixed> {
        if !elastic_constants_admissible(e, nu) || t_hat <= Fixed::ZERO {
            return None;
        }
        let nu2 = nu.checked_mul(nu)?;
        let one_minus_nu2 = Fixed::ONE.checked_sub(nu2)?;
        if one_minus_nu2 <= Fixed::ZERO {
            return None;
        }
        let t3 = t_hat
            .checked_mul(t_hat)
            .and_then(|t2| t2.checked_mul(t_hat))?;
        let numerator = e.checked_mul(t3)?;
        let denom = Fixed::from_int(12).checked_mul(one_minus_nu2)?;
        numerator.checked_div(denom)
    }

    /// The line-load flexural parameter in INTERNAL units, `alpha_hat = sqrt(2 sqrt(D_hat / R_hat))` with
    /// `R_hat = delta_rho g_hat` the internal restoring modulus, algebraically `(4 D_hat / R_hat)^(1/4)`.
    ///
    /// The nested square roots are [`Fixed::sqrt`], exact to the last bit, which is why the design keeps this
    /// quantity LINEAR: a logarithmic form would replace two exact integer roots with two approximate
    /// transcendental evaluations and buy no range. Over the declared envelope and the design's restoring range
    /// `R_hat = 0.0096 .. 3.84`, the quotient `D_hat / R_hat` tops out at `9.26e7` and `alpha_hat` at 138.7.
    ///
    /// THE RESTORING RANGE IS A PREMISE OF THE RANGE TABLE AND NOT AN ENFORCED INPUT BOUND, which is a real
    /// difference. `R_hat`'s floor pairs the STIFFEST plate against the WEAKEST buoyancy, and refusing every
    /// world below it would refuse legitimate small bodies: a Europa-class ice shell floats on a contrast near
    /// `0.08` at `1.31 m/s^2`, an `R_hat` of `0.0034`, a third of the design's floor, and its `D_hat / R_hat` is
    /// about 1 because a small world also has a thin lid. So the quotient is guarded by checked arithmetic,
    /// which fails loud on the pairing that actually overflows, rather than by an input floor that would encode
    /// a large-world assumption.
    // @derives: the line-load flexural length scale in internal units <- the rigidity and the restoring modulus
    pub(crate) fn scaled_flexural_parameter(
        d_hat: Fixed,
        delta_rho: Fixed,
        g_hat: Fixed,
    ) -> Option<Fixed> {
        let ratio = scaled_rigidity_over_restoring(d_hat, delta_rho, g_hat)?;
        // alpha_hat = sqrt(2 sqrt(D_hat / R_hat)); the 4 enters as sqrt(4) = 2 folded into the inner root.
        let inner = Fixed::from_int(2).checked_mul(ratio.sqrt())?;
        Some(inner.sqrt())
    }

    /// The axisymmetric flexural length in INTERNAL units, `l_hat = (D_hat / R_hat)^(1/4)`, which is
    /// `alpha_hat / sqrt(2)`. Distinct from [`scaled_flexural_parameter`] for the reason
    /// [`super::flexural_length_axisymmetric`] states at length: the factor of 4 belongs to the one-dimensional
    /// line-load ODE alone.
    // @derives: the axisymmetric flexural length in internal units <- the rigidity and the restoring modulus
    pub(crate) fn scaled_flexural_length_axisymmetric(
        d_hat: Fixed,
        delta_rho: Fixed,
        g_hat: Fixed,
    ) -> Option<Fixed> {
        let ratio = scaled_rigidity_over_restoring(d_hat, delta_rho, g_hat)?;
        Some(ratio.sqrt().sqrt())
    }

    /// `D_hat / (delta_rho g_hat)`, the quotient both flexural lengths are quarter powers of, guarded once.
    fn scaled_rigidity_over_restoring(
        d_hat: Fixed,
        delta_rho: Fixed,
        g_hat: Fixed,
    ) -> Option<Fixed> {
        if d_hat <= Fixed::ZERO || delta_rho <= Fixed::ZERO || g_hat <= Fixed::ZERO {
            return None;
        }
        let restoring = delta_rho.checked_mul(g_hat)?;
        d_hat.checked_div(restoring)
    }

    /// THE LINE-LOAD AMPLITUDE in INTERNAL units, `w0_hat = V_hat alpha_hat^3 / (8 D_hat)`.
    ///
    /// Every one of the three intermediates that overflowed in the caller's units is bounded here: over the
    /// requested range `alpha_hat^3 <= 2.13e5`, `V_hat alpha_hat^3 <= 3.33e6`, and `8 D_hat <= 2.44e5`; even the
    /// 800 km fully elastic trial gives `V_hat alpha_hat^3 <= 4.17e7` against `8 D_hat <= 7.11e6`. The amplitude
    /// itself runs `3.05e-5 .. 242.35`, so it needs no logarithm: it is a tame number that was being computed
    /// through an intemperate one.
    // @derives: the line-load flexural amplitude in internal units <- the load intensity, flexural parameter and rigidity
    pub(crate) fn scaled_line_load_amplitude(
        v_hat: Fixed,
        alpha_hat: Fixed,
        d_hat: Fixed,
    ) -> Option<Fixed> {
        if alpha_hat <= Fixed::ZERO || d_hat <= Fixed::ZERO {
            return None;
        }
        let a3 = alpha_hat
            .checked_mul(alpha_hat)
            .and_then(|a2| a2.checked_mul(alpha_hat))?;
        let eight_d = Fixed::from_int(8).checked_mul(d_hat)?;
        v_hat.checked_mul(a3).and_then(|x| x.checked_div(eight_d))
    }
}

/// The FLEXURAL RIGIDITY `D = E T_e^3 / (12 (1 - nu^2))` (Turcotte & Schubert, Geodynamics ch. 3, the
/// moment-curvature relation; PIPELINE_FETCHES.md section 1). `e` is Young's modulus, `t_e` the elastic lid
/// thickness, `nu` Poisson's ratio, all caller inputs in the caller's coherent unit system (see the module unit
/// contract); `D` comes out in `stress * length^3`. The `12` is the formula's own.
///
/// The arithmetic runs in INTERNAL units ([`scaled::scaled_rigidity`]), so `E T_hat^3` tops out at `8.0e6`
/// where the old `E T_e^3` reached `1.2e10` for a lid the world really derived. What remains bounded is the
/// RESULT rather than an intermediate: an 800 km fully elastic plate is `2.9e10 GPa km^3` and no rescaling of
/// the internal arithmetic can make that fit an external `Fixed`, so this refuses there and the solver that
/// needs such a plate as a private starting trial holds it in internal units instead.
///
/// Fails loud (`None`) outside the declared operating envelope ([`MAX_YOUNGS_MODULUS_GPA`],
/// [`max_poisson_ratio_magnitude`], [`MAX_ELASTIC_THICKNESS_KM`]), on a non-positive modulus or thickness, or
/// where the rigidity leaves the caller's own window; never a fabricated rigidity. Deterministic (Principle 3).
pub fn flexural_rigidity(e: Fixed, nu: Fixed, t_e: Fixed) -> Option<Fixed> {
    if t_e > Fixed::from_int(MAX_ELASTIC_THICKNESS_KM) {
        return None;
    }
    let t_hat = scaled::internal_length(t_e)?;
    let d_hat = scaled::scaled_rigidity(e, nu, t_hat)?;
    scaled::external_rigidity(d_hat)
}

/// The FLEXURAL PARAMETER `alpha = (4 D / (delta_rho g))^(1/4)` (Turcotte & Schubert eq. 3-127; PIPELINE_FETCHES.md
/// section 1), the DERIVED length scale the plate bends at, in the caller's length unit. `d` is the flexural
/// rigidity, `delta_rho = rho_mantle - rho_infill` the density contrast the deflection floats against, `g` the
/// surface gravity.
///
/// The quarter power is computed as a NESTED square root arranged as `alpha = sqrt(2 sqrt(D / (delta_rho g)))`,
/// which is algebraically identical to `(4 D / (delta_rho g))^(1/4)` (the `4` enters as `sqrt(4) = 2` folded into
/// the inner root). [`Fixed::sqrt`] is exact to the last bit, so this carries no series error, which is why the
/// design keeps the quantity linear rather than logarithmic: a log form would replace two exact integer roots
/// with two approximate transcendental evaluations and buy no range.
///
/// The arithmetic runs in INTERNAL units ([`scaled::scaled_flexural_parameter`]), where the small-divisor hazard
/// that used to sink this goes away: `D / (delta_rho g)` reached `7.6e9` on a thick-lid low-gravity world whose
/// `alpha` was a perfectly representable 417 km, while `D_hat / (delta_rho g_hat)` for that same world is
/// `7.3e3`. Fails loud (`None`) on a non-positive `d`, `delta_rho`, or `g`, or on an out-of-range intermediate.
/// Deterministic.
pub fn flexural_parameter(d: Fixed, delta_rho: Fixed, g: Fixed) -> Option<Fixed> {
    let d_hat = scaled::internal_rigidity(d)?;
    let g_hat = scaled::internal_gravity(g)?;
    let alpha_hat = scaled::scaled_flexural_parameter(d_hat, delta_rho, g_hat)?;
    scaled::external_length(alpha_hat)
}

/// The AXISYMMETRIC (point/disc load) flexural length `l = (D / (delta_rho g))^(1/4)`, DISTINCT from the
/// line-load [`flexural_parameter`] `alpha = (4D / (delta_rho g))^(1/4) = sqrt(2) l`. The factor of 4 belongs to
/// the one-dimensional line-load ODE; the axisymmetric plate equation `D grad^4 w + delta_rho g w = 0` has natural
/// length `l`, since `grad^4 kei(r/l) = -(1/l^4) kei(r/l)` cancels the restoring term only when `l^4 = D/(delta_rho g)`
/// (McNutt and Menard 1982 eq. A8; TAFI point-load row, Brotchie and Silvester 1969, no factor of 4). Naming this
/// separately keeps the two length scales from being welded, the exact confusion PIPELINE_FETCHES.md section 1 made.
///
/// It shares [`flexural_parameter`]'s internal arithmetic and therefore its range: the `D / (delta_rho g)`
/// division that used to overflow here unguarded is the same one, taken in internal units.
pub fn flexural_length_axisymmetric(d: Fixed, delta_rho: Fixed, g: Fixed) -> Option<Fixed> {
    let d_hat = scaled::internal_rigidity(d)?;
    let g_hat = scaled::internal_gravity(g)?;
    let l_hat = scaled::scaled_flexural_length_axisymmetric(d_hat, delta_rho, g_hat)?;
    scaled::external_length(l_hat)
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
    // THE ONE PLACE THE RESCALING BUYS NOTHING, because `l k` is DIMENSIONLESS: `l_hat k_hat = (l/32)(32k) = l k`
    // exactly, so the internal unit system leaves this quantity where it found it. The hole is closed by
    // algebra instead, and by a domain split rather than a fallback: the function is symmetric under
    // `l k -> 1 / (l k)` up to the swap of numerator and denominator, so each branch is taken exactly where its
    // own intermediates are bounded by one. Neither branch can overflow, and the corner `l k = 1` lands on the
    // direct branch where `Phi = 1/2` comes out exactly.
    let Some(lk) = flexural_length.checked_mul(wavenumber) else {
        // `l k` past `2.1e9` puts `Phi` below `5e-38`, thirty-seven orders under the last bit Q32.32 holds.
        // Zero is the correctly rounded value, not a fabricated one: the plate passes nothing at that scale.
        return Some(Fixed::ZERO);
    };
    if lk <= Fixed::ONE {
        // Phi = 1 / (1 + (l k)^4), with (l k)^4 <= 1.
        let lk2 = lk.checked_mul(lk)?;
        let lk4 = lk2.checked_mul(lk2)?;
        Fixed::ONE.checked_div(Fixed::ONE + lk4)
    } else {
        // The same function through `t = 1 / (l k) <= 1`: Phi = t^4 / (t^4 + 1). This is an identity, not an
        // approximation, and it is what the old form could not do: at `l k = 1000` the direct `(l k)^4` is
        // `1e12` and refuses, while `t^4` is `1e-12` and the answer is a representable `1e-12`.
        let t = Fixed::ONE.checked_div(lk)?;
        let t2 = t.checked_mul(t)?;
        let t4 = t2.checked_mul(t2)?;
        t4.checked_div(t4.checked_add(Fixed::ONE)?)
    }
}

/// THE LINE-LOAD AMPLITUDE `w0 = V0 alpha^3 / (8 D)`, the maximum deflection under the load.
///
/// ONE HOME, because it had two. This formula lived here inside [`line_load_deflection`] and again inside
/// `moment_equivalence::line_load_curvature_at_first_zero_crossing`, which is the redundant-parameter
/// diamond this repository keeps paying for: when an overflow fallback was added to the first copy the
/// second still refused, and the solve failed on its SECOND iteration where `alpha` had grown to about
/// 657 km and `8 D` passed `Fixed::MAX`. Collapsing the copies makes that divergence structurally
/// impossible rather than something to remember.
///
/// The arithmetic runs in INTERNAL units ([`scaled::scaled_line_load_amplitude`]), which is what lifts all
/// three of the overflows this expression used to carry: `alpha_hat^3` tops out at `2.1e5` where `alpha^3`
/// reached `7.0e9`, `8 D_hat` at `2.4e5` where `8 D` overflowed for any `D` past `2.68e8`, and
/// `V_hat alpha_hat^3` at `3.3e6`. The amplitude itself was never the problem: it runs `3.05e-5 .. 242.35`
/// in internal lengths, a tame number that was being computed through an intemperate one.
///
/// Fails loud (`None`) on a non-positive `alpha` or `d`, on a load magnitude outside the declared envelope
/// ([`MAX_LINE_LOAD_GPA_KM`]), or on an out-of-range intermediate.
// @derives: the line-load flexural amplitude <- the load intensity, flexural parameter and rigidity
pub fn line_load_amplitude(v0: Fixed, alpha: Fixed, d: Fixed) -> Option<Fixed> {
    if !line_load_admissible(v0) {
        return None;
    }
    let alpha_hat = scaled::internal_length(alpha)?;
    let d_hat = scaled::internal_rigidity(d)?;
    let v_hat = scaled::internal_line_load(v0)?;
    let w0_hat = scaled::scaled_line_load_amplitude(v_hat, alpha_hat, d_hat)?;
    scaled::external_length(w0_hat)
}

/// The LINE-LOAD deflection `w(x) = (V0 alpha^3 / (8 D)) e^(-|x|/alpha) (cos(|x|/alpha) + sin(|x|/alpha))`
/// (Turcotte & Schubert eq. 3-130 / TAFI eq. 4, the continuous-plate solution; PIPELINE_FETCHES.md section 1),
/// the flexure at perpendicular distance `perp_dist` from a line load of magnitude `v0`, given the flexural
/// parameter `alpha` and rigidity `d`. The value is signed in the T&S convention: positive is the downward moat
/// under and beside the load, turning negative past the zero crossing at `3 pi alpha / 4` (the upward forebulge).
/// The distance is taken by magnitude, so the profile is symmetric about the line. A zero load gives zero
/// everywhere. The `8` is the formula's own.
///
/// # THE EXPONENTIAL PRODUCT IS FORMED IN LOGS, AND THIS IS THE ONE PLACE THEY BELONG
///
/// The design is explicit that `D`, `alpha` and the amplitudes must NOT be carried logarithmically after the
/// rescaling: those quantities are bracketed, subtracted and ordered, `alpha` is served by exact integer roots
/// that a log form would replace with two approximate series, and none of them needs the range. What does need
/// it is the final product, because `Fixed::exp` floors at about `e^-22 = 2.7e-10` while the amplitude it
/// multiplies can be nearly 250 internal lengths. A deep basin read 22 flexural wavelengths out therefore has a
/// DECAY that has already quantized to exactly zero while the deflection it belongs to is still a few hundred
/// representable units, and the linear form returns zero for a value it can hold.
///
/// So the shape and the amplitude are combined before a single exponentiation:
///
/// `ln|w_hat(x)| = ln|w0_hat| - X + ln|cos X + sin X|`, `X = |x| / alpha`,
///
/// with the sign carried around the logarithm rather than through it, since a logarithm has none. Where the
/// combined logarithm falls below the `exp` floor the result really is at about one last bit and zero is the
/// honest answer, which is the difference between a limit and a defect. The cost is one `ln`/`exp` round trip
/// where the old form had an `exp` alone, and it is measured rather than assumed
/// (`the_line_load_far_field_survives_the_decay_underflow`).
///
/// Fails loud (`None`) on a non-positive `alpha` or `d`, on a load outside the declared envelope, or on an
/// out-of-range intermediate. Deterministic.
pub fn line_load_deflection(v0: Fixed, alpha: Fixed, d: Fixed, perp_dist: Fixed) -> Option<Fixed> {
    if alpha <= Fixed::ZERO || d <= Fixed::ZERO || !line_load_admissible(v0) {
        return None;
    }
    let alpha_hat = scaled::internal_length(alpha)?;
    let d_hat = scaled::internal_rigidity(d)?;
    let v_hat = scaled::internal_line_load(v0)?;
    let w0_hat = scaled::scaled_line_load_amplitude(v_hat, alpha_hat, d_hat)?;
    if w0_hat == Fixed::ZERO {
        return Some(Fixed::ZERO);
    }
    // The dimensionless argument `X = |x| / alpha` is scale-free (`x_hat / alpha_hat` is the same number), and
    // is taken in the caller's units where both operands carry their most significant bits.
    let big_x = perp_dist.abs().checked_div(alpha)?;
    let (sin_x, cos_x) = big_x.sin_cos();
    let oscillation = cos_x.checked_add(sin_x)?;
    if oscillation == Fixed::ZERO {
        // The zero crossing itself, at `X = 3 pi / 4`: the deflection vanishes and its logarithm does not exist.
        return Some(Fixed::ZERO);
    }
    let ln_w = w0_hat
        .abs()
        .ln()
        .checked_sub(big_x)?
        .checked_add(oscillation.abs().ln())?;
    let magnitude = ln_w.exp();
    let negative = (w0_hat < Fixed::ZERO) != (oscillation < Fixed::ZERO);
    let w_hat = if negative {
        Fixed::ZERO.checked_sub(magnitude)?
    } else {
        magnitude
    };
    scaled::external_length(w_hat)
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
    if alpha <= Fixed::ZERO || d <= Fixed::ZERO || r < Fixed::ZERO || !point_load_admissible(q0) {
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
    // THE COEFFICIENT `Q0 l^2 / (2 pi D)`, formed in INTERNAL units, where `P_hat l_hat^2` tops out at 871 and
    // `2 pi D_hat` at `1.9e5`. In the caller's units `2 pi D` alone overflowed for any `D` past `3.4e8`, which
    // is the third of the design's three `2 pi D` sites and the one that reaches a deflection.
    let l_hat = scaled::internal_length(l)?;
    let d_hat = scaled::internal_rigidity(d)?;
    let q_hat = scaled::internal_force(q0)?;
    let l2_hat = l_hat.checked_mul(l_hat)?;
    let two_pi_d_hat = Fixed::from_int(2)
        .checked_mul(Fixed::PI)
        .and_then(|x| x.checked_mul(d_hat))?;
    let coef_hat = q_hat
        .checked_mul(l2_hat)
        .and_then(|x| x.checked_div(two_pi_d_hat))?;
    let arg = r.checked_div(l)?; // r / l, dimensionless and unchanged by the rescaling
                                 // `w_hat = coef_hat kei(r/l)`, and `w = 32 w_hat`: the internal coefficient carries the `1/32` that the
                                 // three unit factors leave behind (`P_hat l_hat^2 / D_hat = (1/32) P l^2 / D`), so the conversion back is
                                 // the same one every other length takes.
    scaled::external_length(coef_hat.checked_mul(kelvin_kei(arg))?)
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

/// THE GOLDEN-VECTOR HARNESS shared by this module's tests and `crate::moment_equivalence`'s, so the
/// representation migration is judged against ONE comparison table rather than two that could drift.
#[cfg(test)]
pub(crate) mod golden {
    use civsim_core::Fixed;

    // ----- THE GOLDEN VECTORS (the representation migration's comparison table) -----

    /// One golden row: what was computed, the RAW `to_bits` this kernel produced BEFORE the internal-unit
    /// migration, and the raw bits it produces now.
    ///
    /// # WHY THE PRE-MIGRATION COLUMN STAYS
    ///
    /// The migration to the internal `32 km / 32 s / 1 GPa / 1000 kg/m^3` unit system moves where Q32.32
    /// truncation happens, so a dimensional result is EXPECTED to change in its low bits
    /// (`docs/working/FLEXURE_REPRESENTATION_DESIGN.md` section 6). A physical regression would also change
    /// the bits. The two are told apart by keeping BOTH columns visible and gating the movement between
    /// them: a low-bit shift passes, a shift large enough to be physics fails, and neither can be waved
    /// through by editing one number. Bulk-updating the expectations in the same change that introduces the
    /// scaling is exactly the masquerade this table exists to prevent.
    pub(crate) struct Golden {
        pub what: &'static str,
        pub before: Option<i64>,
        pub now: Option<i64>,
        /// Whether this row is the terminal value of a STEP-TERMINATED FIXED POINT rather than a closed-form
        /// evaluation. Such a row can move by far more than the representation, because a direct substitution
        /// stops where its step falls under the tolerance and therefore keeps a memory of where it started;
        /// changing the derived starting trial moves the answer by the width of that plateau. Marking the row
        /// says which kind of movement is being licensed, so a solver plateau cannot be waved through as a
        /// truncation site and a truncation site cannot be waved through as a solver plateau.
        pub solver_terminated: bool,
    }

    /// The relative movement a REPRESENTATION change is allowed to make, above which a moved golden row is
    /// treated as a physical regression rather than a rescaling artifact.
    ///
    /// THE BASIS, stated so this is a reviewed bound and not a chosen tolerance: `1e-6` sits four orders
    /// ABOVE the Q32.32 relative resolution at these magnitudes (a rigidity near `4e5` resolves to about
    /// `6e-16` relative, a deflection near `3.7` to about `6e-11`), so every truncation-site move the
    /// rescaling can make fits under it; and it sits nearly three orders BELOW the `5e-4` absolute agreement
    /// the module's own numerical twins hold these same quantities to against their analytic forms, so
    /// nothing that passes here could have moved the physics the twins would still accept.
    pub(crate) const MIGRATION_MOVE: f64 = 1e-6;

    /// The movement a SOLVER-TERMINATED value is allowed to make while the solver itself is being replaced.
    ///
    /// MEASURED, NOT CHOSEN, and it is a finding rather than a tolerance. The per-load solve was a direct
    /// substitution that stopped when its step fell under `Fixed::EPSILON`, which admits a whole plateau of
    /// approximate fixed points, and where on that plateau it stopped depended on where it started. The
    /// Earth-like illustration's plateau was measured at `5e-7` relative between two admissible starts; the
    /// Mars-class thick lid's, whose derived trial moved from a clamped 207 km plate to the full 739.4 km
    /// domain, at `1.1e-4`. Replacing the substitution with a curvature bracket bisected to adjacent
    /// representable curvatures removes the property outright, and the total movement of every solve row
    /// across both changes tops out at `3.6e-5`. The bound is that measurement rounded up, and it stays
    /// because a solve's terminal value is a different KIND of quantity from a closed-form evaluation: a
    /// change to the solver may legitimately move it where nothing may legitimately move an evaluation.
    pub(crate) const MIGRATION_MOVE_SOLVER: f64 = 2e-4;

    /// The ABSOLUTE arm of the same gate, in raw Q32.32 bits: one internal rigidity ulp, `32768` bits.
    ///
    /// A relative bound alone cannot judge a table that includes far-field deflections of a few tens of
    /// nanometres, where a movement of twenty last bits reads as a thousand parts per million of nothing at
    /// all. This arm is derived rather than chosen: `32768` bits is the resolution of the COARSEST quantity
    /// the internal unit system carries, so a movement under it is a difference the internal arithmetic
    /// could not have represented in the first place and therefore cannot be a physical change. A row passes
    /// on EITHER arm, and the bit-exact `now` column is the primary pin in every case.
    pub(crate) const MIGRATION_MOVE_BITS: i64 = 32768;

    /// Check one golden row: the value must reproduce `now` TO THE BIT, and its movement from `before` must
    /// pass either arm of the representation gate ([`MIGRATION_MOVE`] relative, [`MIGRATION_MOVE_BITS`]
    /// absolute, or [`MIGRATION_MOVE_SOLVER`] where the row is a solve's terminal value). A row that gained or
    /// lost an answer entirely (a refusal that became a number, or the reverse) is reported as such rather
    /// than compared numerically.
    pub(crate) fn check_golden(row: &Golden, value: Option<Fixed>) {
        let now_bits = value.map(|v| v.to_bits());
        assert_eq!(
            now_bits, row.now,
            "{}: this build produced {:?}, the table says {:?}",
            row.what, now_bits, row.now
        );
        // Only a row that had an answer on BOTH sides can be compared numerically. A refusal that became an
        // answer is the migration's PURPOSE (an intermediate that no longer overflows), and an answer that
        // became a refusal is a real loss; both are visible in the table itself.
        if let (Some(b), Some(n)) = (row.before, row.now) {
            let (bf, nf) = (
                Fixed::from_bits(b).to_f64_lossy(),
                Fixed::from_bits(n).to_f64_lossy(),
            );
            let denom = bf.abs().max(f64::MIN_POSITIVE);
            let moved = (nf - bf).abs() / denom;
            let moved_bits = (n - b).abs();
            let bound = if row.solver_terminated {
                MIGRATION_MOVE_SOLVER
            } else {
                MIGRATION_MOVE
            };
            assert!(
                moved <= bound || moved_bits <= MIGRATION_MOVE_BITS,
                "{}: moved {moved:.3e} relative and {moved_bits} raw bits across the migration ({bf} to \
                 {nf}), past both arms of the representation bound; that is a physical change wearing a \
                 rescaling's clothes",
                row.what
            );
        }
    }

    pub(crate) fn g_row(what: &'static str, before: i64, now: i64) -> Golden {
        Golden {
            what,
            before: Some(before),
            now: Some(now),
            solver_terminated: false,
        }
    }

    /// A row whose value is the terminal iterate of a step-terminated fixed point; see
    /// [`Golden::solver_terminated`].
    pub(crate) fn g_solver(what: &'static str, before: i64, now: i64) -> Golden {
        Golden {
            what,
            before: Some(before),
            now: Some(now),
            solver_terminated: true,
        }
    }

    pub(crate) fn g_none(what: &'static str, now: Option<i64>) -> Golden {
        Golden {
            what,
            before: None,
            now,
            solver_terminated: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::golden::*;
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

    /// THE DUAL EVALUATION: the scaled kernels beside the arithmetic they replace, both measured against an
    /// independent evaluation, before any returned result moves.
    ///
    /// The design's migration order puts this step before the switch on purpose
    /// (`docs/working/FLEXURE_REPRESENTATION_DESIGN.md` section 6): a rescaling changes WHERE Q32.32
    /// truncation happens, so the two arithmetics cannot be bit-identical, and the honest way to adopt the
    /// new one is to measure the gap first rather than discover it afterwards in a golden diff.
    ///
    /// # WHY THE MEASUREMENT IS AGAINST A TWIN AND NOT AGAINST THE OLD PATH
    ///
    /// Two earlier forms of this test measured the scaled kernel against the external one and asked for
    /// agreement. Both readings were dominated by the OLD path's own error rather than by the rescaling.
    /// The first compared against the public functions and charged the unit change with the LOG FALLBACK's
    /// two transcendental series: 48000 internal ulps on `alpha`, none of it the units'. The second compared
    /// against the linear form written out inline and still read 9400 ulps, and the cause is worth stating
    /// because it inverts the expected direction: the external restoring modulus `delta_rho g` for a
    /// Europa-class body is `1.05e-4`, which Q32.32 resolves to only about one part in `4.6e5`, while the
    /// internal `delta_rho g_hat` is 32 times larger and is resolved 32 times better. The old path was the
    /// coarser one.
    ///
    /// So each quantity is measured against an INDEPENDENT `f64` evaluation of its own formula, fed the
    /// exact `f64` values of the very same fixed-point inputs, and both paths are reported. The claim this
    /// test makes is the one that matters: over the declared envelope and beyond it, the scaled kernel is at
    /// least as close to the formula as the arithmetic it replaces, and it answers where that arithmetic
    /// cannot.
    #[test]
    fn the_scaled_kernels_reproduce_the_external_arithmetic_where_both_run() {
        // The linear external forms, written out here rather than called: these are the arithmetic the
        // scaled kernels REPLACED, and the public functions no longer contain it.
        let linear_alpha = |d: Fixed, drho: Fixed, g: Fixed| -> Option<Fixed> {
            let ratio = d.checked_div(drho.checked_mul(g)?)?;
            Fixed::from_int(2)
                .checked_mul(ratio.sqrt())
                .map(Fixed::sqrt)
        };
        let linear_amplitude = |v0: Fixed, alpha: Fixed, d: Fixed| -> Option<Fixed> {
            let a3 = alpha
                .checked_mul(alpha)
                .and_then(|a2| a2.checked_mul(alpha))?;
            let eight_d = Fixed::from_int(8).checked_mul(d)?;
            v0.checked_mul(a3)?.checked_div(eight_d)
        };

        // [external, scaled] worst relative departure from the f64 twin, per quantity.
        let mut worst = [[0.0_f64; 2]; 4];
        let names = [
            "rigidity",
            "flexural parameter",
            "axisymmetric length",
            "line-load amplitude",
        ];
        let mut worst_declared = 0.0_f64;
        let mut extended = 0usize;
        let mut compared = 0usize;

        let moduli = [
            (Fixed::from_int(1), Fixed::ZERO),
            (Fixed::from_int(70), Fixed::from_ratio(1, 4)),
            (Fixed::from_int(120), Fixed::from_ratio(1, 4)),
            (Fixed::from_int(512), Fixed::from_ratio(1, 2)),
            (Fixed::from_int(512), Fixed::from_ratio(-1, 2)),
        ];
        // (delta_rho, g in km/s^2): an Earth-like pair, the Mars-class pair, a Europa-class ice pair whose
        // restoring modulus is a third of the design's own table floor, and a heavy fast-gravity pair.
        let restoring = [
            (Fixed::from_ratio(33, 10), Fixed::from_ratio(98, 10000)),
            (Fixed::from_ratio(337, 100), Fixed::from_ratio(37, 10000)),
            (Fixed::from_ratio(8, 100), Fixed::from_ratio(131, 100000)),
            (Fixed::from_int(4), Fixed::from_ratio(3, 100)),
        ];
        let thicknesses = [5, 10, 40, 100, 400, 739, 800];
        let loads = [1, 20, 80, 500];

        for (e, nu) in moduli {
            for t in thicknesses {
                let t_km = Fixed::from_int(t);
                let t_hat = scaled::internal_length(t_km).expect("a thickness converts");
                let d_ext = flexural_rigidity(e, nu, t_km);
                let d_hat = scaled::scaled_rigidity(e, nu, t_hat);
                let (ef, nf, tf) = (e.to_f64_lossy(), nu.to_f64_lossy(), t_km.to_f64_lossy());
                let plane = 12.0 * (1.0 - nf * nf);
                if let Some(a) = d_ext {
                    compared += 1;
                    worst[0][0] = worst[0][0].max(off(a, ef * tf.powi(3) / plane));
                }
                if let Some(b) = d_hat {
                    let th = t_hat.to_f64_lossy();
                    let moved = off(b, ef * th.powi(3) / plane);
                    worst[0][1] = worst[0][1].max(moved);
                    // The design's declared rigidity floor `D >= 1e4 GPa km^3` is `D_hat >= 0.305176`.
                    if b.to_f64_lossy() >= 1e4 / f64::from(scaled::INTERNAL_RIGIDITY_GPA_KM3) {
                        worst_declared = worst_declared.max(moved);
                    }
                    if d_ext.is_none() {
                        extended += 1;
                    }
                } else if d_ext.is_some() {
                    panic!("the scaled rigidity lost an answer the external one had: E = {ef}, T = {t}");
                }
                let Some(d_hat) = d_hat else { continue };
                for (drho, g) in restoring {
                    let g_hat = scaled::internal_gravity(g).expect("a gravity converts");
                    let rf = drho.to_f64_lossy() * g.to_f64_lossy();
                    let a_lin = d_ext.and_then(|d| linear_alpha(d, drho, g));
                    let a_hat = scaled::scaled_flexural_parameter(d_hat, drho, g_hat);
                    // The twin runs on each path's OWN rigidity, so the rigidity's error is not double-counted.
                    if let (Some(a), Some(d)) = (a_lin, d_ext) {
                        worst[1][0] =
                            worst[1][0].max(off(a, (4.0 * d.to_f64_lossy() / rf).powf(0.25)));
                    }
                    let (dh, rh) = (
                        d_hat.to_f64_lossy(),
                        drho.to_f64_lossy() * g_hat.to_f64_lossy(),
                    );
                    if let Some(b) = a_hat {
                        worst[1][1] = worst[1][1].max(off(b, (4.0 * dh / rh).powf(0.25)));
                        if a_lin.is_none() {
                            extended += 1;
                        }
                    }
                    let l_ext = d_ext.and_then(|d| flexural_length_axisymmetric(d, drho, g));
                    if let (Some(a), Some(d)) = (l_ext, d_ext) {
                        worst[2][0] = worst[2][0].max(off(a, (d.to_f64_lossy() / rf).powf(0.25)));
                    }
                    if let Some(b) = scaled::scaled_flexural_length_axisymmetric(d_hat, drho, g_hat)
                    {
                        worst[2][1] = worst[2][1].max(off(b, (dh / rh).powf(0.25)));
                        if l_ext.is_none() {
                            extended += 1;
                        }
                    }
                    let Some(alpha_hat) = a_hat else { continue };
                    for v0 in loads {
                        let v = Fixed::from_int(v0);
                        if let (Some(a), Some(al), Some(d)) = (
                            a_lin.and_then(|al| linear_amplitude(v, al, d_ext.unwrap())),
                            a_lin,
                            d_ext,
                        ) {
                            let r = f64::from(v0) * al.to_f64_lossy().powi(3)
                                / (8.0 * d.to_f64_lossy());
                            worst[3][0] = worst[3][0].max(off(a, r));
                        }
                        let v_hat = scaled::internal_line_load(v).expect("a load converts");
                        if let Some(b) = scaled::scaled_line_load_amplitude(v_hat, alpha_hat, d_hat)
                        {
                            let ah = alpha_hat.to_f64_lossy();
                            let r = v_hat.to_f64_lossy() * ah.powi(3) / (8.0 * dh);
                            worst[3][1] = worst[3][1].max(off(b, r));
                        }
                    }
                }
            }
        }

        assert!(compared > 0, "the sweep must have compared something");
        assert!(
            extended > 0,
            "the scaled kernels must answer somewhere the linear external arithmetic refuses, or the \
             migration buys nothing"
        );
        // MEASURED, THEN PINNED, in the only currency that means anything across four decades of magnitude:
        // each path's own relative departure from its formula. The bound is the sweep's worst reading rounded
        // up, and the scaled column must be no worse than the external one, which is the substantive claim.
        // MEASURED, THEN PINNED. Each bound is the sweep's own worst reading rounded up, so a later change
        // that moves a kernel further than its arithmetic can account for trips here rather than being
        // absorbed. Both columns ship because the interesting fact is their COMPARISON.
        //
        // WHERE THE RESCALING COSTS PRECISION, AND IT DOES: the internal rigidity `D_hat = D / 32768` throws
        // away fifteen low bits, so a very soft plate carries fewer significant bits internally than
        // externally. The binding corner in this sweep is `E = 1 GPa` at `T_e = 5 km`, a rigidity of
        // `10.4 GPa km^3`, three decades BELOW the design's own declared floor of `1e4`; there the scaled
        // rigidity reads `2.4e-7` relative against the external form's `1.5e-11`. Inside the declared range
        // the internal rigidity still carries thirty-one significant bits and the cost vanishes, which is
        // the separate assertion below.
        //
        // AND WHERE IT BUYS PRECISION, which the design did not predict: on a Europa-class body the EXTERNAL
        // restoring modulus `delta_rho g` is `1.05e-4`, resolved to about one part in `4.6e5`, while the
        // internal `delta_rho g_hat` is 32 times larger and correspondingly better resolved. Both flexural
        // lengths read about `3.1e-7` relative on the external path there, and the scaled path is no worse.
        for (i, bound) in [3e-7_f64, 4e-7, 4e-7, 2e-6].into_iter().enumerate() {
            assert!(
                worst[i][1] <= bound,
                "the scaled {} departs from its own formula by {:.3e} relative, past the measured bound \
                 {bound:.0e} (the external form reads {:.3e})",
                names[i],
                worst[i][1],
                worst[i][0]
            );
        }
        assert!(
            worst_declared <= 1e-9,
            "inside the design's declared rigidity range (D >= 1e4 GPa km^3) the scaled rigidity must land \
             within one internal ulp of its formula, read {worst_declared:.3e}"
        );
    }

    /// A fixed-point value's relative departure from an independently evaluated reference.
    fn off(value: Fixed, reference: f64) -> f64 {
        (value.to_f64_lossy() - reference).abs() / reference.abs().max(f64::MIN_POSITIVE)
    }

    /// THE GOLDEN TABLE, checked against this build.
    ///
    /// Every row is a `to_bits` value captured from the kernel itself, never one reasoned to. The `before`
    /// column is the pre-migration build; `now` is what this build must reproduce exactly.
    #[test]
    fn the_golden_vectors_hold_across_the_representation_migration() {
        let (e, nu, drho, g) = (earth_e(), earth_nu(), earth_drho(), earth_g());
        let d = |t: i32| flexural_rigidity(e, nu, Fixed::from_int(t));
        let (d10, d30, d40, d100) = (d(10), d(30), d(40), d(100));
        let a40 = flexural_parameter(d40.unwrap(), drho, g).unwrap();
        let d40v = d40.unwrap();
        let line = |v0: i32, x: i32| {
            line_load_deflection(Fixed::from_int(v0), a40, d40v, Fixed::from_int(x))
        };
        let point =
            |r: i32| point_load_deflection(Fixed::from_int(500), a40, d40v, Fixed::from_int(r));
        let kel = |num: i64, den: i64| Fixed::from_ratio(num, den);
        let inputs = PlateInputs {
            youngs_modulus: e,
            poisson_ratio: nu,
            elastic_thickness: Fixed::from_int(35),
            density_contrast: drho,
            gravity: g,
        };
        let loads = [
            Load {
                kind: LoadKind::Point,
                magnitude: Fixed::from_int(400),
                x: Fixed::ZERO,
                y: Fixed::ZERO,
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

        let table: [(Golden, Option<Fixed>); 44] = [
            // --- rigidity and the two flexural lengths ---
            (
                g_row("D(E=70,nu=0.25,T=10)", 26724240952888, 26724240949248),
                d10,
            ),
            (
                g_row("D(E=70,nu=0.25,T=30)", 721554505728000, 721554505728000),
                d30,
            ),
            (
                g_row("D(E=70,nu=0.25,T=40)", 1710351420984888, 1710351420981248),
                d40,
            ),
            (
                g_row(
                    "D(E=70,nu=0.25,T=100)",
                    26724240952888888,
                    26724240952885248,
                ),
                d100,
            ),
            (
                g_row("alpha(T=10)", 127211459822, 127211459616),
                flexural_parameter(d10.unwrap(), drho, g),
            ),
            (
                g_row("alpha(T=40)", 359808343539, 359808343072),
                flexural_parameter(d40v, drho, g),
            ),
            (
                g_row("alpha(T=100)", 715362608958, 715362608064),
                flexural_parameter(d100.unwrap(), drho, g),
            ),
            (
                g_row("l(T=10)", 89952085884, 89952085728),
                flexural_length_axisymmetric(d10.unwrap(), drho, g),
            ),
            (
                g_row("l(T=40)", 254422919644, 254422919328),
                flexural_length_axisymmetric(d40v, drho, g),
            ),
            (
                g_row("l(T=100)", 505837751801, 505837751168),
                flexural_length_axisymmetric(d100.unwrap(), drho, g),
            ),
            // --- the line-load amplitude, the one home ---
            (
                g_row("w0(V0=1)", 792644569, 792644544),
                line_load_amplitude(Fixed::from_int(1), a40, d40v),
            ),
            (
                g_row("w0(V0=20)", 15852891394, 15852891328),
                line_load_amplitude(Fixed::from_int(20), a40, d40v),
            ),
            (
                g_row("w0(V0=80)", 63411565579, 63411565312),
                line_load_amplitude(Fixed::from_int(80), a40, d40v),
            ),
            (
                g_row("w0(V0=200)", 158528913948, 158528913312),
                line_load_amplitude(Fixed::from_int(200), a40, d40v),
            ),
            // --- the line-load profile, including the far field where the decay underflows alone ---
            (
                g_row("w_line(V0=20,x=0)", 15852891423, 15852891328),
                line(20, 0),
            ),
            (
                g_row("w_line(V0=20,x=25)", 14701144013, 14701143936),
                line(20, 25),
            ),
            (
                g_row("w_line(V0=20,x=100)", 6236827503, 6236827456),
                line(20, 100),
            ),
            (
                g_row("w_line(V0=20,x=300)", -587315904, -587315872),
                line(20, 300),
            ),
            (g_row("w_line(V0=20,x=1000)", 22773, 22752), line(20, 1000)),
            (g_row("w_line(V0=20,x=5000)", 0, 0), line(20, 5000)),
            (g_row("w_line(V0=500,x=1850)", 0, -96), line(500, 1850)),
            (g_row("w_line(V0=500,x=1900)", 0, -64), line(500, 1900)),
            // --- the point-load profile ---
            (
                g_row("w_point(Q0=500,r=0)", -2365413815, -2365413792),
                point(0),
            ),
            (
                g_row("w_point(Q0=500,r=25)", -2098920947, -2098920928),
                point(25),
            ),
            (
                g_row("w_point(Q0=500,r=100)", -838458051, -838458048),
                point(100),
            ),
            (
                g_row("w_point(Q0=500,r=300)", 33466898, 33466880),
                point(300),
            ),
            // --- the spectral filter, including the (l k)^4 corner that overflowed ---
            (
                g_row("Phi(l=2,k=1/2)", 2147483648, 2147483648),
                flexural_response_ratio(Fixed::from_int(2), Fixed::from_ratio(1, 2)),
            ),
            (
                g_row("Phi(l=10,k=1/100)", 4294537842, 4294537842),
                flexural_response_ratio(Fixed::from_int(10), Fixed::from_ratio(1, 100)),
            ),
            (
                g_row("Phi(l=10,k=1)", 429453, 429453),
                flexural_response_ratio(Fixed::from_int(10), Fixed::from_int(1)),
            ),
            (
                g_none("Phi(l=100,k=10)", Some(0)),
                flexural_response_ratio(Fixed::from_int(100), Fixed::from_int(10)),
            ),
            // --- the Kelvin functions, which the design requires to stay bit-identical on the series ---
            (
                g_row("kei(1)", -2125985776, -2125985776),
                Some(kelvin_kei(kel(1, 1))),
            ),
            (
                g_row("kei(2)", -869301672, -869301672),
                Some(kelvin_kei(kel(2, 1))),
            ),
            (
                g_row("kei(3.91467)", 292, 292),
                Some(kelvin_kei(kel(391467, 100000))),
            ),
            (
                g_row("kei(11)", -640966, -640966),
                Some(kelvin_kei(kel(11, 1))),
            ),
            (
                g_row("ker(1)", 1231393790, 1231393790),
                Some(kelvin_ker(kel(1, 1))),
            ),
            (
                g_row("ker(2)", -178947728, -178947728),
                Some(kelvin_ker(kel(2, 1))),
            ),
            (
                g_row("ker(3.91467)", -167071870, -167071870),
                Some(kelvin_ker(kel(391467, 100000))),
            ),
            (
                g_row("ker(11)", -204308, -204308),
                Some(kelvin_ker(kel(11, 1))),
            ),
            (
                g_row("kei'(1)", 1513417250, 1513417250),
                Some(kelvin_kei_prime(kel(1, 1))),
            ),
            (
                g_row("kei'(2)", 944067786, 944067786),
                Some(kelvin_kei_prime(kel(2, 1))),
            ),
            (
                g_row("kei'(3.91467)", 118837570, 118837570),
                Some(kelvin_kei_prime(kel(391467, 100000))),
            ),
            (
                g_row("kei'(4.93181)", 97, 97),
                Some(kelvin_kei_prime(kel(493181, 100000))),
            ),
            // --- the superposing evaluator ---
            (
                g_row("deflection_at(10,10)", 6439116477, 6439116384),
                deflection_at(&inputs, &loads, Fixed::from_int(10), Fixed::from_int(10)),
            ),
            (
                g_row("deflection_at(0,0)", 5154552311, 5154552224),
                deflection_at(&inputs, &loads, Fixed::ZERO, Fixed::ZERO),
            ),
        ];
        for (row, value) in &table {
            check_golden(row, *value);
        }
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

    /// THE THICK-LID PLATE ANSWERS WHERE THE CALLER'S OWN ARITHMETIC CANNOT FORM THE PRODUCT.
    ///
    /// This is what the internal unit system bought, stated as a domain fact rather than as a comparison
    /// between two arithmetics. In the caller's units a Mars-class lid makes `alpha^3 = 7.3e7`, so a line load
    /// of 80 (the magnitude this module's own Earth-like tests use) makes `V0 alpha^3 = 5.8e9` against a
    /// `Fixed::MAX` of `2.147e9`, for a deflection of 7.7 km that is entirely representable. Internally the
    /// same quantity is `V_hat alpha_hat^3 = 5.6e3` and nothing is close to the edge.
    #[test]
    fn the_thick_lid_deflection_answers_where_the_external_product_overflows() {
        let e = Fixed::from_int(120);
        let nu = Fixed::from_ratio(25, 100);
        let d = flexural_rigidity(e, nu, Fixed::from_int(207)).expect("a thick-lid rigidity");
        let alpha =
            flexural_parameter(d, Fixed::from_ratio(337, 100), Fixed::from_ratio(37, 10000))
                .expect("alpha");
        let v0 = Fixed::from_int(80);

        // THE EXTERNAL PRODUCT REALLY DOES OVERFLOW, or this test is moot.
        let a3 = alpha
            .checked_mul(alpha)
            .and_then(|a2| a2.checked_mul(alpha))
            .expect("alpha^3 itself is representable");
        assert!(
            v0.checked_mul(a3).is_none(),
            "V0 alpha^3 must overflow in the caller's units: that is what the rescaling exists for"
        );

        let w = line_load_deflection(v0, alpha, d, Fixed::ZERO).expect("the scaled path answers");
        let km = w.to_f64_lossy();
        assert!(
            (5.0..=12.0).contains(&km),
            "w0 = V0 alpha^3 / (8 D) is about 7.7 km here, read {km:.3}"
        );
    }

    /// THE THICK-LID FLEXURAL PARAMETER ANSWERS WHERE THE CALLER'S OWN QUOTIENT CANNOT FORM.
    ///
    /// A small-divisor hazard rather than a large answer: in this module's declared units `g` is in `km/s^2`,
    /// so a Mars-class `0.0037` against a density contrast of `3.37` makes the buoyancy modulus `0.0125`, and
    /// dividing by it multiplies by eighty. A thick-lid world reaches `D / (delta_rho g) = 7.6e9` against a
    /// `Fixed::MAX` of `2.147e9` for an `alpha` of 417 km that is entirely representable. Internally the same
    /// quotient is `D_hat / (delta_rho g_hat) = 7.3e3`.
    #[test]
    fn the_thick_lid_parameter_answers_where_the_external_quotient_overflows() {
        let e = Fixed::from_int(120);
        let nu = Fixed::from_ratio(25, 100);
        let d = flexural_rigidity(e, nu, Fixed::from_int(207)).expect("a thick-lid rigidity");
        let drho = Fixed::from_ratio(337, 100);
        let g = Fixed::from_ratio(37, 10000); // 3.7 m/s^2 in km/s^2

        // THE EXTERNAL QUOTIENT REALLY DOES OVERFLOW, or this test is moot.
        let restoring = drho.checked_mul(g).expect("restoring");
        assert!(
            d.checked_div(restoring).is_none(),
            "D / (delta_rho g) must overflow in the caller's units: that is the whole reason for the rescaling"
        );

        let alpha = flexural_parameter(d, drho, g).expect("the scaled path answers");
        let km = alpha.to_f64_lossy();
        assert!(
            (380.0..=460.0).contains(&km),
            "alpha = (4 D / (delta_rho g))^(1/4) is about 418 km here, read {km:.1}"
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
        // `nu = 1` is past the declared envelope's `|nu| <= 0.5`, which is also the physical
        // incompressible limit for an isotropic solid; it is refused one step before `1 - nu^2` reaches zero.
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

    /// THE FAR-FIELD EXPONENTIAL PRODUCT SURVIVES ITS OWN DECAY, which the linear form could not do.
    ///
    /// `Fixed::exp` floors at about `e^-22`, so beyond 22 flexural wavelengths `e^(-X)` quantizes to exactly
    /// zero. The deflection it belongs to does not: a large amplitude times a tiny decay is still a
    /// representable number, and the old form returned zero for a value it could hold. This is the design's
    /// eleventh range hole and it is the ONLY one the internal unit system does not fix by itself, because
    /// scaling moves the amplitude down rather than the decay up.
    ///
    /// THE HONEST LIMIT, measured rather than glossed: what the log form recovers out there is a value of a
    /// FEW INTERNAL ULPS, so it is right in sign and order and carries a quantization error of tens of per
    /// cent at the last bit. That is the representation's floor rather than the method's, and it is still the
    /// difference between a small number and a wrong one.
    #[test]
    fn the_line_load_far_field_survives_the_decay_underflow() {
        // A thin plate carries a big amplitude: T_e = 10 km at V0 = 500 GPa km gives w0 near 261 km, so the
        // product stays representable to about 24 flexural wavelengths while the decay alone dies at 22.
        let d = flexural_rigidity(earth_e(), earth_nu(), Fixed::from_int(10)).expect("D");
        let alpha = flexural_parameter(d, earth_drho(), earth_g()).expect("alpha");
        let v0 = Fixed::from_int(500);
        let w0 = line_load_amplitude(v0, alpha, d)
            .expect("w0")
            .to_f64_lossy();
        let a = alpha.to_f64_lossy();
        assert!(w0 > 200.0, "the fixture needs a large amplitude, got {w0}");

        // THE DECAY REALLY HAS DIED, or this test is moot.
        let big_x = 22.5_f64;
        let x_km = big_x * a;
        assert_eq!(
            (Fixed::ZERO - Fixed::from_ratio((big_x * 1e6) as i64, 1_000_000)).exp(),
            Fixed::ZERO,
            "e^(-X) must quantize to zero at X = {big_x}: that is the hole this closes"
        );

        let w = line_load_deflection(
            v0,
            alpha,
            d,
            Fixed::from_ratio((x_km * 1000.0) as i64, 1000),
        )
        .expect("the far field answers");
        assert_ne!(
            w,
            Fixed::ZERO,
            "the amplitude times the decay is representable at X = {big_x} and must not read as zero"
        );
        // Against the analytic form, to the last bits the representation can hold. One internal length ulp
        // is `32 * 2^-32 km`, and the whole value here is only a handful of them, so the comparison is in
        // ulps and not in per cent.
        let analytic = w0 * (-big_x).exp() * (big_x.cos() + big_x.sin());
        let ulp = 32.0 * Fixed::EPSILON.to_f64_lossy();
        assert!(
            (w.to_f64_lossy() - analytic).abs() <= 2.0 * ulp,
            "the recovered far-field deflection {} must land within two internal ulps of the analytic \
             {analytic}",
            w.to_f64_lossy()
        );
        assert!(
            analytic.abs() / ulp < 20.0,
            "and the honest limit stands: the analytic value out here is {:.1} internal ulps",
            analytic.abs() / ulp
        );
    }

    /// THE SPECTRAL FILTER ANSWERS PAST ITS OWN QUARTIC, which it could not before.
    ///
    /// `(l k)^4` overflows for `l k` past about 215, and the rescaling cannot help because `l k` is
    /// dimensionless: `l_hat k_hat` is the same number. The reciprocal branch is an identity rather than an
    /// approximation, so the two branches meet exactly at the corner.
    #[test]
    fn the_flexural_filter_answers_past_the_quartic_overflow() {
        // THE QUARTIC REALLY DOES OVERFLOW, or this test is moot.
        let lk = Fixed::from_int(1000);
        let lk2 = lk.checked_mul(lk).expect("(l k)^2 fits");
        assert!(
            lk2.checked_mul(lk2).is_none(),
            "(l k)^4 at l k = 1000 must overflow: that is the hole this closes"
        );
        let phi = flexural_response_ratio(Fixed::from_int(100), Fixed::from_int(10))
            .expect("the reciprocal branch answers");
        // Phi = 1 / (1 + 1e12) = 1e-12, four thousandths of the last bit Q32.32 holds, so zero is the
        // correctly rounded value and the point is that it is REPORTED rather than refused.
        assert_eq!(phi, Fixed::ZERO);
        // At l k = 215 the true value is 4.7e-10, two last bits, and the branch returns it.
        let near = flexural_response_ratio(Fixed::from_int(215), Fixed::ONE).expect("answers");
        assert!(
            near > Fixed::ZERO && near < Fixed::from_ratio(1, 1_000_000),
            "Phi at l k = 215 is about 4.7e-10, got {}",
            near.to_f64_lossy()
        );
        // THE TWO BRANCHES MEET AT THE CORNER: l k = 1 gives exactly one half on the direct branch, and the
        // reciprocal branch evaluated at the same point gives the same number.
        let corner =
            flexural_response_ratio(Fixed::from_int(2), Fixed::from_ratio(1, 2)).expect("corner");
        assert_eq!(corner.to_bits(), 1 << 31, "Phi(l k = 1) = 1/2 exactly");
        // Either side of the corner the function is continuous to the last bits.
        let below =
            flexural_response_ratio(Fixed::from_ratio(999, 1000), Fixed::ONE).expect("below");
        let above =
            flexural_response_ratio(Fixed::from_ratio(1001, 1000), Fixed::ONE).expect("above");
        assert!(
            below > corner && corner > above,
            "the filter is monotone through the branch corner: {} then {} then {}",
            below.to_f64_lossy(),
            corner.to_f64_lossy(),
            above.to_f64_lossy()
        );
        assert!(
            (below.to_f64_lossy() - above.to_f64_lossy()).abs() < 1e-2,
            "and continuous across it"
        );
    }

    /// THE PUBLIC BOUNDARIES ANSWER ACROSS THE DECLARED ENVELOPE, which is the range table as a test.
    ///
    /// It began life as the evidence that retired the logarithmic fallbacks: over the envelope's corners the
    /// scaled arithmetic answered wherever the public function did, so no call ever reached the log branch and
    /// a second numerical semantics had no purpose. The fallbacks are gone, so what it asserts now is the
    /// claim they were insuring against: every corner of the DECLARED envelope whose rigidity the caller's
    /// unit can hold carries a flexural parameter, an axisymmetric length, a line-load amplitude and a
    /// point-load deflection, with no intermediate refusing anywhere along the way.
    #[test]
    fn the_public_boundaries_answer_across_the_declared_envelope() {
        let moduli = [
            (Fixed::from_int(1), Fixed::ZERO),
            (Fixed::from_int(70), Fixed::from_ratio(1, 4)),
            (Fixed::from_int(120), Fixed::from_ratio(1, 4)),
            (
                Fixed::from_int(MAX_YOUNGS_MODULUS_GPA),
                Fixed::from_ratio(1, 2),
            ),
        ];
        let restoring = [
            (Fixed::from_ratio(33, 10), Fixed::from_ratio(98, 10000)),
            (Fixed::from_ratio(337, 100), Fixed::from_ratio(37, 10000)),
            (Fixed::from_ratio(8, 100), Fixed::from_ratio(131, 100000)),
            (Fixed::from_int(4), Fixed::from_ratio(3, 100)),
        ];
        let mut checked = 0usize;
        for (e, nu) in moduli {
            for t in [5, 10, 40, 100, 400, 739, MAX_ELASTIC_THICKNESS_KM] {
                // A rigidity the caller's own unit cannot hold is the honest ceiling, not a failure; the
                // corner matrix in `moment_equivalence` proves the INTERNAL rigidity exists at every corner.
                let Some(d) = flexural_rigidity(e, nu, Fixed::from_int(t)) else {
                    continue;
                };
                for (drho, g) in restoring {
                    let alpha = flexural_parameter(d, drho, g).unwrap_or_else(|| {
                        panic!(
                            "no flexural parameter at E = {}, T = {t}, delta_rho g = {}",
                            e.to_f64_lossy(),
                            drho.to_f64_lossy() * g.to_f64_lossy()
                        )
                    });
                    flexural_length_axisymmetric(d, drho, g)
                        .expect("and an axisymmetric length at the same corner");
                    for load in [1, 20, 80, MAX_LINE_LOAD_GPA_KM] {
                        let magnitude = Fixed::from_int(load);
                        line_load_amplitude(magnitude, alpha, d)
                            .expect("and a line-load amplitude at the same corner");
                        line_load_deflection(magnitude, alpha, d, alpha)
                            .expect("and a line-load deflection one wavelength out");
                        point_load_deflection(magnitude, alpha, d, alpha)
                            .expect("and a point-load deflection at the same radius");
                        checked += 1;
                    }
                }
            }
        }
        assert!(
            checked > 100,
            "the sweep must have covered the envelope, ran {checked}"
        );
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
