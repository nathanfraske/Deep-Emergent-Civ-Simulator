// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.

//! THE RELIEF A LOAD LIST BENDS INTO A SOLVED PLATE: the composition that turns a converged
//! moment-equivalence into a deflection at any query point.
//!
//! # WHY THIS MODULE EXISTS AND WHAT IT IS THE SIBLING OF
//!
//! [`crate::flexure`] holds the Green's functions and [`crate::flexure::deflection_at`] already superposes
//! them, but it reaches them through [`crate::flexure::PlateInputs`], whose `elastic_thickness` field is
//! documented as "the SOLE unsupplied input to `D`" and which therefore has never had a production caller. The
//! moment-equivalence solve ([`crate::moment_equivalence::solve_line_load`]) is what SUPPLIES that missing
//! input, and it supplies it as a rigidity rather than a thickness, because the load sets its own curvature
//! through the solve and `T_e` falls out as a display statistic rather than going in as a premise.
//!
//! So [`FlexedPlate`] is the production sibling of `PlateInputs`: same Green's functions, same superposition,
//! but the rigidity comes from a converged solve instead of from a thickness nobody could derive.
//!
//! # IT TAKES THE RIGIDITY INTERNALLY, WHICH IS THE WHOLE REASON IT CAN RUN AT ALL
//!
//! The engine's own sluggish Mars-class column converges to `2.9e9 GPa km^3`, past what `Fixed` holds in that
//! unit (see [`crate::moment_equivalence::MomentEquivalentPlate::rigidity`]). Every entry point in
//! [`crate::flexure`] that takes a rigidity takes it in the caller's `GPa km^3` and converts inward, so none of
//! them can be handed this world's plate at all. This module takes `D_hat` directly and stays in internal units
//! until the last step, which is the same discipline the solve itself follows.
//!
//! # WHAT IS AUTHORED HERE: NOTHING
//!
//! The rigidity is solved, the flexural parameter is derived from it and the restoring term, the amplitudes are
//! the Green's functions' own, and the load list is the caller's world data. There is no tuneable in this file
//! and no scalar with a basis to reserve. The one number that appears, `INTERNAL_LENGTH_KM`, is the declared
//! representation scale and belongs to [`crate::flexure::scaled`].
//!
//! Deterministic (Principle 3): the superposition is a sum of `Fixed`, whose addition is exact and associative,
//! so the result does not depend on the order the loads are listed in.

use civsim_core::Fixed;

use crate::flexure::{
    flexural_response_ratio, kelvin_kei, line_load_admissible, point_load_admissible, scaled,
    uniform_strip_load_admissible, InternalRigidity, Load, LoadKind,
};
use crate::moment_equivalence::MomentEquivalentPlate;

/// AN ELEVATION IN THE SURFACE'S OWN SENSE: kilometres ABOVE the compensation reference, positive UP.
///
/// # WHY THIS IS A TYPE AND NOT A DOCUMENTED CONVENTION
///
/// [`FlexedPlate::deflection_km`] returns a DOWNWARD-positive deflection, which is the Turcotte and Schubert
/// convention the line and strip Green's functions are written in. The surface's elevation axis is positive
/// UP. Those are opposite senses, and a consumer that stored one as the other produced a mountain that was a
/// hole: the run-path profile was documented and tested as "above the compensation reference" while holding
/// raw downward deflections, so its peak was a downward displacement wearing the word height. A review caught
/// it; a comment saying which way is up had not been enough, because both quantities are a `Fixed` in
/// kilometres and nothing stops one being assigned to the other.
///
/// So the conversion is a TYPE BOUNDARY that has to be crossed on purpose, once, at the point where a
/// deflection becomes an elevation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElevationKm(Fixed);

impl ElevationKm {
    /// The elevation in kilometres above the compensation reference, positive up.
    pub fn km(self) -> Fixed {
        self.0
    }
}

/// A CONVERGED PLATE READY TO BE LOADED: the moment-equivalent rigidity plus the two flexural lengths the
/// Green's functions need, each derived once so a query point costs no root.
///
/// The two lengths are distinct and the distinction is load-bearing. `alpha = (4 D / (delta_rho g))^(1/4)` is
/// the LINE-load parameter, whose factor of four belongs to the one-dimensional beam ODE; `l = (D / (delta_rho
/// g))^(1/4) = alpha / sqrt(2)` is the AXISYMMETRIC length the point-load Green's function runs on. Welding one
/// to both was a real defect in this codebase until 2026-07-17, and it made the moat `sqrt(2)` too wide and
/// twice too deep, so both are carried explicitly here rather than one being converted at each use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlexedPlate {
    rigidity_internal: Fixed,
    restoring_internal: Fixed,
    alpha_internal: Fixed,
    axisymmetric_length_internal: Fixed,
}

/// Why a plate could not be built or a deflection could not be evaluated. Every arm is a stop; nothing here
/// falls back to a plausible number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReliefRefusal {
    /// The restoring term is non-positive: a plate floating on nothing has no flexural length.
    RestoringTermNotPositive,
    /// The solved rigidity is non-positive, which is not a plate.
    RigidityNotPositive,
    /// A load's magnitude is outside the declared envelope [`crate::flexure`] states for its kind.
    LoadOutsideEnvelope,
    /// A distributed load's half-width is non-positive, so it has no finite footprint to integrate.
    FootprintNotPositive,
    /// A fixed-point intermediate left the representable window. Never a fabricated deflection.
    NotRepresentable,
}

impl FlexedPlate {
    /// Build from a CONVERGED moment-equivalence and the world's own restoring term.
    ///
    /// `delta_rho` is the density contrast the deflection floats against, in `1000 kg/m^3`, and `gravity_km_s2`
    /// is the surface gravity in `km/s^2`: the same coherent system
    /// [`crate::moment_equivalence::solve_line_load`] declares, so a caller that solved a plate already holds
    /// both in the right units.
    // @derives: a loadable flexed plate <- the converged moment-equivalent rigidity and the world's restoring term
    pub fn from_moment_equivalent(
        plate: &MomentEquivalentPlate,
        delta_rho: Fixed,
        gravity_km_s2: Fixed,
    ) -> Result<Self, ReliefRefusal> {
        Self::from_internal_rigidity(plate.rigidity(), delta_rho, gravity_km_s2)
    }

    /// Build from a DIMENSIONAL rigidity in the caller's `GPa km^3`.
    ///
    /// This is the constructor a caller OUTSIDE this crate wants. [`Self::from_internal_rigidity`] takes the
    /// internal representation, and `crate::flexure::scaled` is crate-private on purpose, so without this a
    /// downstream crate holding an ordinary `D` had no way in and the internal unit system leaked into its API.
    ///
    /// It is fallible for the usual reason plus one more: a rigidity too large for `GPa km^3` cannot be PASSED
    /// in that unit in the first place, so a caller holding a sluggish world's plate must come through
    /// [`Self::from_moment_equivalent`], which carries it internally end to end. See
    /// [`crate::moment_equivalence::MomentEquivalentPlate::rigidity`].
    // @derives: a loadable flexed plate <- a dimensional rigidity and the world's restoring term
    pub fn from_rigidity_gpa_km3(
        rigidity_gpa_km3: Fixed,
        delta_rho: Fixed,
        gravity_km_s2: Fixed,
    ) -> Result<Self, ReliefRefusal> {
        let internal =
            scaled::internal_rigidity(rigidity_gpa_km3).ok_or(ReliefRefusal::NotRepresentable)?;
        Self::from_internal_rigidity(internal, delta_rho, gravity_km_s2)
    }

    /// Build from an internal rigidity directly, for a caller holding one from somewhere other than the
    /// moment-equivalence solve (a hindcast row converted inward, or a test's synthetic plate).
    ///
    /// The parameter is the TYPED [`InternalRigidity`] rather than a bare `Fixed`. It took a `Fixed` while
    /// the `32768` scale that makes such a number correct stayed crate-private, so an external caller could
    /// not build a right one and could pass any wrong one; see [`InternalRigidity`] for the finding.
    // @derives: a loadable flexed plate <- an internal rigidity and the world's restoring term
    pub fn from_internal_rigidity(
        rigidity: InternalRigidity,
        delta_rho: Fixed,
        gravity_km_s2: Fixed,
    ) -> Result<Self, ReliefRefusal> {
        let rigidity_internal = rigidity.internal();
        if rigidity_internal <= Fixed::ZERO {
            return Err(ReliefRefusal::RigidityNotPositive);
        }
        if delta_rho <= Fixed::ZERO || gravity_km_s2 <= Fixed::ZERO {
            return Err(ReliefRefusal::RestoringTermNotPositive);
        }
        let g_hat =
            scaled::internal_gravity(gravity_km_s2).ok_or(ReliefRefusal::NotRepresentable)?;
        let restoring_internal = delta_rho
            .checked_mul(g_hat)
            .ok_or(ReliefRefusal::NotRepresentable)?;
        let alpha_internal = scaled::scaled_flexural_parameter(rigidity_internal, delta_rho, g_hat)
            .ok_or(ReliefRefusal::NotRepresentable)?;
        let axisymmetric_length_internal =
            scaled::scaled_flexural_length_axisymmetric(rigidity_internal, delta_rho, g_hat)
                .ok_or(ReliefRefusal::NotRepresentable)?;
        if alpha_internal <= Fixed::ZERO || axisymmetric_length_internal <= Fixed::ZERO {
            return Err(ReliefRefusal::RestoringTermNotPositive);
        }
        Ok(FlexedPlate {
            rigidity_internal,
            restoring_internal,
            alpha_internal,
            axisymmetric_length_internal,
        })
    }

    /// THE LINE-LOAD FLEXURAL PARAMETER in kilometres, `alpha = (4 D / (delta_rho g))^(1/4)`.
    ///
    /// This is the length over which a load's relief is FELT, and it is the quantity that makes flexural relief
    /// different in kind from Airy flotation: under Airy each column floats alone and neighbouring columns say
    /// nothing to each other, so relief is as rough as its loads. Here one load bends a neighbourhood `alpha`
    /// wide, which is what produces a range with flanks, a moat, and a forebulge instead of a field of
    /// independent blocks.
    pub fn flexural_parameter_km(&self) -> Option<Fixed> {
        scaled::external_length(self.alpha_internal)
    }

    /// The AXISYMMETRIC flexural length in kilometres, `l = alpha / sqrt(2)`, which is what a point load
    /// (a volcanic construct, a basin) is felt over.
    pub fn axisymmetric_length_km(&self) -> Option<Fixed> {
        scaled::external_length(self.axisymmetric_length_internal)
    }

    /// The internal rigidity this plate was built from, TYPED so the caller cannot read it as an ordinary
    /// `GPa km^3` rigidity. [`InternalRigidity::to_gpa_km3`] is the dimensional readout, and it is fallible
    /// because some real plates do not fit that unit.
    pub fn rigidity_internal(&self) -> InternalRigidity {
        InternalRigidity::from_internal(self.rigidity_internal)
    }

    /// THE PERIODIC SPECTRAL SOLVE over a CLOSED row of equally spaced columns, in kilometres, downward
    /// positive: the answer for a row that WRAPS, which no truncated superposition of infinite-plate Green's
    /// functions can give.
    ///
    /// # WHY THIS EXISTS, AND WHY THE SUBSTRATE FOR IT WAS ALREADY HERE
    ///
    /// A province row on a globe closes on itself, so the physical domain is periodic. Evaluating a FINITE
    /// line of columns against infinite-plate Green's functions leaves every column short of the neighbours
    /// beyond the ends, and the consuming profile in `civsim_sim::flexural_field` said in its own doc that
    /// wrapping "needs either a periodic Green's function or a summed image series, neither of which this
    /// substrate carries". That was false against this repository's own contents:
    /// [`crate::flexure::flexural_response_ratio`] is the periodic transfer function, it has been here since
    /// the flexural-filter slice landed, and it had no caller outside its own tests.
    ///
    /// # THE MECHANISM
    ///
    /// The plate equation on a periodic domain is diagonal in the Fourier basis. Writing the row's load as
    /// `q(x)` and the restoring modulus as `R = delta_rho g`, `D w'''' + R w = q` becomes
    /// `(D k^4 + R) W(k) = Q(k)`, so each wavenumber passes independently:
    ///
    /// `W(k) = (Q(k) / R) Phi(k)`, `Phi(k) = 1 / (1 + (l k)^4)`.
    ///
    /// THE LENGTH IN `Phi` IS THE AXISYMMETRIC `l`, NEVER `alpha`, and this module is emphatic about that
    /// distinction for good reason. The Fourier symbol gives `l^4 = D / R` in one dimension exactly as in two:
    /// `alpha`'s factor of four belongs to the line-load ODE's particular solution rather than to the symbol.
    /// The check is that this reproduces the line-load Green's function: inverting `V0 / (D k^4 + R)` gives
    /// `w(0) = V0 / (2 sqrt(2) R l)`, and `V0 alpha^3 / (8 D)` with `alpha = sqrt(2) l` and `D = R l^4` is the
    /// same number. Pinned by `the_periodic_solve_reproduces_the_infinite_plate_green_function`.
    ///
    /// # WHY THE MEAN IS SPLIT OFF FIRST, WHICH IS WHAT MAKES A UNIFORM ROW FLAT TO THE BIT
    ///
    /// `Phi(0) = 1` EXACTLY ([`crate::flexure::flexural_response_ratio`] returns `Fixed::ONE` for `k = 0`), so
    /// the mean load passes to relief unfiltered and the isostatic answer is exact. The row is therefore
    /// decomposed as a mean plus deviations, and only the deviations are transformed.
    ///
    /// That is not a convenience. Transforming the raw `q_i` instead would be identical in real arithmetic,
    /// because `sum_i cos(2 pi m i / N)` vanishes for every `m` other than zero. It does NOT vanish in fixed
    /// point, so the mean would LEAK into every bin, get multiplied by that bin's `Phi`, and come back as a
    /// ripple on what must be a flat answer. Subtracting the mean first makes the deviations of a uniform row
    /// exactly zero, every bin exactly zero, and every column's answer the identical isostatic value, to the
    /// bit and by construction rather than by rounding luck. Pinned by
    /// `a_periodic_uniform_load_is_flat_to_the_bit`.
    ///
    /// The price is one truncation: the mean is an integer division of the summed raw bits, so it can sit up
    /// to one representable unit low and the deviations then carry a residual mean of at most that. The
    /// residue is dropped rather than double counted, which costs under one last bit of pressure divided by
    /// `R`, against a leak that would have been the whole mean times a bin's `Phi`.
    ///
    /// FLATNESS IS THE ONLY BIT-EXACT SYMMETRY HERE, and the boundary is worth naming. A uniform row is flat
    /// by construction, because every bin is exactly zero and nothing is reconstructed. A general SYMMETRIC
    /// row's mirror symmetry is instead an identity ACROSS bins,
    /// `cos a cos(a + b) + sin a sin(a + b) = cos b`, which fixed point cannot close exactly, so a symmetric
    /// load answers symmetrically to within the reconstruction's own rounding rather than to the bit. On an
    /// eleven-column province row that residue measures about thirty raw bits, five orders under the
    /// smallest relief step the profile resolves. Both the transform and the reconstruction accumulate their
    /// products UNROUNDED in `i128` and shift once, which is what holds it there.
    ///
    /// # THE APPROXIMATION THIS CARRIES, NAMED
    ///
    /// THE SPECTRAL SOLVE IS EXACT ONLY FOR A PLATE OF UNIFORM RIGIDITY, because diagonalizing the operator
    /// needs `D` outside the derivative. A [`FlexedPlate`] carries ONE rigidity, so a caller whose row spans
    /// columns of differing lid strength is making the SLOWLY-VARYING-RIGIDITY APPROXIMATION: it picks one `D`
    /// for the row and accepts the departure. That approximation was already being made by every consumer of
    /// this type and was not named anywhere; it is named here, and its cost is a measurement rather than an
    /// assertion, since running the same row at the rigidities its columns span and differencing the answers
    /// is what bounds it (`the_slowly_varying_rigidity_approximation_costs_what_the_rigidity_spread_costs`).
    ///
    /// # THE OTHER HONEST LIMITS
    ///
    /// `cell_pressure` is a SAMPLED load field, one pressure per cell, so the solve carries only the load's
    /// content below the grid's Nyquist wavenumber. That is the dominant departure from the infinite-plate
    /// Green's function and it is MEASURED rather than guessed: against a direct strip convolution the worst
    /// disagreement runs `2.8e-2` of the peak at `alpha / dx = 1.4`, `3.6e-3` at `4.2`, and `1.3e-3` at
    /// `7.0`, converging at close to the second order in cell width. The periodic images, which were the
    /// first suspect, are REFUTED as the cause: lengthening the domain eightfold moves the disagreement by
    /// under a part in five hundred. So the honest limit is a resolution one, and the knob is cells per
    /// flexural length. The row is one-dimensional, so this is a profile along a row and not a 2-D field,
    /// which is the separate gap the module doc names. Determinism (Principle 3): every sum
    /// accumulates raw bits in `i128` in index order, the phase table is one CORDIC evaluation per residue
    /// reused by every bin, and nothing depends on the order the caller discovered its columns in.
    ///
    /// `cell_pressure[i]` is the downward-positive pressure on cell `i`, in the caller's coherent unit
    /// (GPa in the declared system), and `cell_width_km` is the uniform spacing. Each cell's integrated line
    /// load `q dx` is held to the SAME [`crate::flexure::MAX_LINE_LOAD_GPA_KM`] envelope the strip Green's
    /// function is proved over, so no new bound is authored here. An empty row has an empty profile.
    // @derives: the periodic flexural deflection profile over a closed row <- the per-cell load field, the plate rigidity and the restoring modulus
    pub fn periodic_deflection_km(
        &self,
        cell_pressure: &[Fixed],
        cell_width_km: Fixed,
    ) -> Result<Vec<Fixed>, ReliefRefusal> {
        let n = cell_pressure.len();
        if n == 0 {
            // No columns is a legitimate state with an empty profile, the reading an empty load list gets
            // from `deflection_km`.
            return Ok(Vec::new());
        }
        if cell_width_km <= Fixed::ZERO {
            return Err(ReliefRefusal::FootprintNotPositive);
        }
        let n_i32 = i32::try_from(n).map_err(|_| ReliefRefusal::NotRepresentable)?;
        let n_fixed = Fixed::from_int(n_i32);
        let n_i128 = i128::from(n_i32);

        // ADMISSIBILITY OVER THE WHOLE ROW BEFORE ANY SUM, the order-independent discipline `deflection_km`
        // declares: a stop must not depend on which column the caller listed first (Principle 3,
        // Principle 10). Each cell loads the plate over its own width, so its integrated line-load
        // equivalent `q dx` is bounded by the envelope the strip Green's function already carries.
        let half_width = cell_width_km
            .checked_div(Fixed::from_int(2))
            .ok_or(ReliefRefusal::NotRepresentable)?;
        if half_width <= Fixed::ZERO {
            return Err(ReliefRefusal::FootprintNotPositive);
        }
        if cell_pressure
            .iter()
            .any(|q| !uniform_strip_load_admissible(*q, half_width))
        {
            return Err(ReliefRefusal::LoadOutsideEnvelope);
        }

        // ----- THE DC BIN, IN EXACT INTEGER ARITHMETIC -----
        let mut sum_bits = 0_i128;
        for q in cell_pressure {
            sum_bits = sum_bits
                .checked_add(i128::from(q.to_bits()))
                .ok_or(ReliefRefusal::NotRepresentable)?;
        }
        let mean =
            Fixed::from_bits_i128(sum_bits / n_i128).ok_or(ReliefRefusal::NotRepresentable)?;
        let mut deviation = Vec::with_capacity(n);
        for q in cell_pressure {
            deviation.push(q.checked_sub(mean).ok_or(ReliefRefusal::NotRepresentable)?);
        }

        // ----- THE PHASE TABLE: one CORDIC evaluation per residue below the half turn, reflected above -----
        //
        // The angle `2 pi m i / N` depends only on `(m i) mod N`, so tabulating the `N` distinct residues
        // bounds the trigonometric work and makes two terms at the same residue read the identical bits
        // rather than two independent CORDIC results.
        //
        // THE UPPER HALF IS REFLECTED RATHER THAN EVALUATED, and that is load-bearing. `cos(2 pi - t) = cos t`
        // and `sin(2 pi - t) = -sin t` are exact identities, while two independent CORDIC evaluations of the
        // same cosine may differ in a last bit. Evaluating both halves put a mirror-symmetric province row
        // ninety-six raw bits apart across its own axis: representation noise wearing the shape of physics,
        // in the one quantity a closed row's symmetry is read from. Taking the identity instead makes the
        // table exactly even in cosine and exactly odd in sine, so a symmetric load gives a symmetric answer
        // to the bit, and it halves the CORDIC work.
        let two_pi = Fixed::from_int(2)
            .checked_mul(Fixed::PI)
            .ok_or(ReliefRefusal::NotRepresentable)?;
        // Residue zero is the zero angle, whose sine and cosine are exact.
        let mut phase = vec![(Fixed::ZERO, Fixed::ONE); n];
        for j in 1..=(n / 2) {
            let j_i32 = i32::try_from(j).map_err(|_| ReliefRefusal::NotRepresentable)?;
            // Formed as a FRACTION OF A TURN first, so the angle handed to `sin_cos` is always inside one
            // period and the product `2 pi j` never has to exist.
            let turn = Fixed::from_int(j_i32)
                .checked_div(n_fixed)
                .ok_or(ReliefRefusal::NotRepresentable)?;
            let theta = two_pi
                .checked_mul(turn)
                .ok_or(ReliefRefusal::NotRepresentable)?;
            let (sin, cos) = theta.sin_cos();
            phase[j] = (sin, cos);
            // The half turn of an even-length row is its own reflection and must not be written twice.
            if n - j != j {
                phase[n - j] = (
                    Fixed::ZERO
                        .checked_sub(sin)
                        .ok_or(ReliefRefusal::NotRepresentable)?,
                    cos,
                );
            }
        }

        let cell_hat =
            scaled::internal_length(cell_width_km).ok_or(ReliefRefusal::NotRepresentable)?;
        let domain_hat = cell_hat
            .checked_mul(n_fixed)
            .ok_or(ReliefRefusal::NotRepresentable)?;
        if domain_hat <= Fixed::ZERO {
            return Err(ReliefRefusal::FootprintNotPositive);
        }

        // ----- THE FILTERED LOAD FIELD: the mean passes whole, each deviation bin passes `Phi` of it -----
        let mut filtered_bits = vec![i128::from(mean.to_bits()); n];
        for m in 1..=(n / 2) {
            let m_i32 = i32::try_from(m).map_err(|_| ReliefRefusal::NotRepresentable)?;
            // THE PRODUCTS ARE ACCUMULATED UNROUNDED. `Fixed::checked_mul` shifts each product back down by
            // `FRAC_BITS` immediately, so summing `N` of them rounds `N` times; summing the RAW `i128`
            // products and shifting once at the end rounds once, on the same floor convention `checked_mul`
            // uses. It is the module's own accumulate-in-`i128` discipline taken one step further, and it
            // matters here because a transform coefficient is a cancelling sum where per-term rounding does
            // not cancel with it.
            let mut cos_bits = 0_i128;
            let mut sin_bits = 0_i128;
            for (i, d) in deviation.iter().enumerate() {
                let (sin, cos) = phase[(m * i) % n];
                let d_bits = i128::from(d.to_bits());
                cos_bits = d_bits
                    .checked_mul(i128::from(cos.to_bits()))
                    .and_then(|p| cos_bits.checked_add(p))
                    .ok_or(ReliefRefusal::NotRepresentable)?;
                sin_bits = d_bits
                    .checked_mul(i128::from(sin.to_bits()))
                    .and_then(|p| sin_bits.checked_add(p))
                    .ok_or(ReliefRefusal::NotRepresentable)?;
            }
            // The real-transform weight is `2/N` on every bin, except the NYQUIST bin of an even-length row,
            // which is its own conjugate partner and therefore carries `1/N`. Getting that wrong doubles the
            // shortest representable wavelength and nothing else, which is why it is stated rather than
            // folded into the loop bound.
            let weight: i128 = if 2 * m == n { 1 } else { 2 };
            let cosine_amplitude =
                Fixed::from_bits_i128((cos_bits * weight / n_i128) >> Fixed::FRAC_BITS)
                    .ok_or(ReliefRefusal::NotRepresentable)?;
            let sine_amplitude =
                Fixed::from_bits_i128((sin_bits * weight / n_i128) >> Fixed::FRAC_BITS)
                    .ok_or(ReliefRefusal::NotRepresentable)?;

            let wavenumber = two_pi
                .checked_mul(Fixed::from_int(m_i32))
                .and_then(|x| x.checked_div(domain_hat))
                .ok_or(ReliefRefusal::NotRepresentable)?;
            let phi = flexural_response_ratio(self.axisymmetric_length_internal, wavenumber)
                .ok_or(ReliefRefusal::NotRepresentable)?;
            let passed_cosine = cosine_amplitude
                .checked_mul(phi)
                .ok_or(ReliefRefusal::NotRepresentable)?;
            let passed_sine = sine_amplitude
                .checked_mul(phi)
                .ok_or(ReliefRefusal::NotRepresentable)?;
            if passed_cosine == Fixed::ZERO && passed_sine == Fixed::ZERO {
                // A DEAD BIN ADDS NOTHING, and skipping it is what makes the uniform row's flatness
                // structural: with every deviation exactly zero, every bin is exactly zero and no
                // reconstruction term is ever formed.
                continue;
            }
            let passed_cosine_bits = i128::from(passed_cosine.to_bits());
            let passed_sine_bits = i128::from(passed_sine.to_bits());
            for (i, accumulator) in filtered_bits.iter_mut().enumerate() {
                let (sin, cos) = phase[(m * i) % n];
                // One FUSED multiply-add per bin, for the same reason the transform above fuses: the two
                // products are added before either is shifted back down, so the pair rounds once.
                let term = passed_cosine_bits
                    .checked_mul(i128::from(cos.to_bits()))
                    .and_then(|c| {
                        passed_sine_bits
                            .checked_mul(i128::from(sin.to_bits()))
                            .and_then(|s| c.checked_add(s))
                    })
                    .ok_or(ReliefRefusal::NotRepresentable)?;
                *accumulator = accumulator
                    .checked_add(term >> Fixed::FRAC_BITS)
                    .ok_or(ReliefRefusal::NotRepresentable)?;
            }
        }

        // ----- THE DEFLECTION: the filtered load over the restoring modulus -----
        let mut profile = Vec::with_capacity(n);
        for accumulator in filtered_bits {
            let filtered =
                Fixed::from_bits_i128(accumulator).ok_or(ReliefRefusal::NotRepresentable)?;
            let w_hat = filtered
                .checked_div(self.restoring_internal)
                .ok_or(ReliefRefusal::NotRepresentable)?;
            profile.push(scaled::external_length(w_hat).ok_or(ReliefRefusal::NotRepresentable)?);
        }
        Ok(profile)
    }

    /// THE PERIODIC PROFILE AS ELEVATION, positive UP: [`Self::periodic_deflection_km`] through the same
    /// typed boundary [`Self::elevation_km`] crosses, so a downward deflection cannot be stored as a height.
    // @derives: the periodic surface elevation profile <- the plate's periodic downward deflection profile
    pub fn periodic_elevation_km(
        &self,
        cell_pressure: &[Fixed],
        cell_width_km: Fixed,
    ) -> Result<Vec<ElevationKm>, ReliefRefusal> {
        self.periodic_deflection_km(cell_pressure, cell_width_km)?
            .into_iter()
            .map(|down| {
                Fixed::ZERO
                    .checked_sub(down)
                    .map(ElevationKm)
                    .ok_or(ReliefRefusal::NotRepresentable)
            })
            .collect()
    }

    /// THE ELEVATION AT A QUERY POINT, positive UP, which is what a surface consumer wants.
    ///
    /// The one explicit conversion out of the Green's functions' downward-positive sense
    /// ([`Self::deflection_km`]) into the surface's own upward-positive one, returned as [`ElevationKm`] so it
    /// cannot be confused with a deflection again. A load pressing the plate DOWN therefore yields a NEGATIVE
    /// elevation here, and a buoyant column, which is a negative downward load, stands up.
    // @derives: the surface elevation at a point <- the plate's downward deflection under the load list
    pub fn elevation_km(
        &self,
        loads: &[Load],
        qx_km: Fixed,
        qy_km: Fixed,
    ) -> Result<ElevationKm, ReliefRefusal> {
        let down = self.deflection_km(loads, qx_km, qy_km)?;
        Fixed::ZERO
            .checked_sub(down)
            .map(ElevationKm)
            .ok_or(ReliefRefusal::NotRepresentable)
    }

    /// THE DEFLECTION AT A QUERY POINT, in kilometres, summed over the whole load list.
    ///
    /// `qx_km` and `qy_km` are the query point and the load positions are the caller's own, in the same plane
    /// and the same kilometres. A positive load gives a POSITIVE downward deflection in the Turcotte and
    /// Schubert convention used by the line and strip Green's functions. A caller whose elevation axis is
    /// positive upward applies that coordinate conversion at the boundary.
    ///
    /// An empty list, or a list of zero-magnitude loads, gives zero rather than refusing: no load is a
    /// legitimate state and its relief is flat.
    ///
    /// THE SUM IS ORDER-INDEPENDENT because `Fixed` addition is exact and associative, which is the
    /// determinism contract (Principle 3) rather than a nicety: two runs that discover the same loads in
    /// different orders must produce the same world. Raw contribution bits accumulate in `i128` before one
    /// final Q32.32 range check, so opposite-signed partial sums cannot make a refusal depend on listing order.
    // @derives: the flexural deflection at a point <- the plate's rigidity, its flexural lengths and the load list
    pub fn deflection_km(
        &self,
        loads: &[Load],
        qx_km: Fixed,
        qy_km: Fixed,
    ) -> Result<Fixed, ReliefRefusal> {
        // ADMISSIBILITY IS SETTLED OVER THE WHOLE LIST BEFORE ANY SUM, and by a declared precedence, because
        // returning on the FIRST bad load makes the REFUSAL order-dependent even though the VALUE is not. A list
        // holding one over-envelope load and one zero-footprint load reported `LoadOutsideEnvelope` or
        // `FootprintNotPositive` according to which the caller happened to list first, so two runs that
        // discovered the same loads in different orders disagreed about why the world refused. The sum was
        // always order-independent; this makes the stop order-independent too (Principle 3, Principle 10).
        //
        // The precedence is stated rather than emergent: a non-positive footprint is not a load geometry at all,
        // so it outranks a magnitude that is merely outside the declared envelope. Found by an independent
        // audit of this substrate.
        let mut footprint_refused = false;
        let mut envelope_refused = false;
        for load in loads {
            match load.kind {
                LoadKind::LineY => {
                    if !line_load_admissible(load.magnitude) {
                        envelope_refused = true;
                    }
                }
                LoadKind::Point => {
                    if !point_load_admissible(load.magnitude) {
                        envelope_refused = true;
                    }
                }
                LoadKind::UniformStripY { half_width } => {
                    if half_width <= Fixed::ZERO {
                        footprint_refused = true;
                    } else if !uniform_strip_load_admissible(load.magnitude, half_width) {
                        envelope_refused = true;
                    }
                }
            }
        }
        if footprint_refused {
            return Err(ReliefRefusal::FootprintNotPositive);
        }
        if envelope_refused {
            return Err(ReliefRefusal::LoadOutsideEnvelope);
        }

        let mut total_hat_bits = 0_i128;
        for load in loads {
            let contribution = match load.kind {
                LoadKind::LineY => {
                    if !line_load_admissible(load.magnitude) {
                        return Err(ReliefRefusal::LoadOutsideEnvelope);
                    }
                    let perp = qx_km
                        .checked_sub(load.x)
                        .ok_or(ReliefRefusal::NotRepresentable)?;
                    self.line_contribution_hat(load.magnitude, perp)?
                }
                LoadKind::Point => {
                    if !point_load_admissible(load.magnitude) {
                        return Err(ReliefRefusal::LoadOutsideEnvelope);
                    }
                    let dx = qx_km
                        .checked_sub(load.x)
                        .ok_or(ReliefRefusal::NotRepresentable)?;
                    let dy = qy_km
                        .checked_sub(load.y)
                        .ok_or(ReliefRefusal::NotRepresentable)?;
                    let r = dx
                        .checked_mul(dx)
                        .and_then(|x2| dy.checked_mul(dy).and_then(|y2| x2.checked_add(y2)))
                        .ok_or(ReliefRefusal::NotRepresentable)?
                        .sqrt();
                    self.point_contribution_hat(load.magnitude, r)?
                }
                LoadKind::UniformStripY { half_width } => {
                    if half_width <= Fixed::ZERO {
                        return Err(ReliefRefusal::FootprintNotPositive);
                    }
                    if !uniform_strip_load_admissible(load.magnitude, half_width) {
                        return Err(ReliefRefusal::LoadOutsideEnvelope);
                    }
                    let perp = qx_km
                        .checked_sub(load.x)
                        .ok_or(ReliefRefusal::NotRepresentable)?;
                    self.uniform_strip_contribution_hat(load.magnitude, half_width, perp)?
                }
            };
            total_hat_bits = total_hat_bits
                .checked_add(i128::from(contribution.to_bits()))
                .ok_or(ReliefRefusal::NotRepresentable)?;
        }
        if total_hat_bits < i128::from(i64::MIN) || total_hat_bits > i128::from(i64::MAX) {
            return Err(ReliefRefusal::NotRepresentable);
        }
        let total_hat = Fixed::from_bits(total_hat_bits as i64);
        scaled::external_length(total_hat).ok_or(ReliefRefusal::NotRepresentable)
    }

    /// One line load's contribution, in INTERNAL length.
    ///
    /// `w(x) = w0 exp(-X) (cos X + sin X)` with `X = |x| / alpha` and `w0 = V0 alpha^3 / (8 D)`. The magnitude
    /// runs through logarithms for the reason [`crate::flexure::line_load_deflection`] gives at length: the
    /// decay underflows the far field before the answer is negligible, so `exp(-X)` is never formed alone. The
    /// sign rides OUTSIDE the logarithm, which has none.
    // @derives: one line load's plate deflection <- the load intensity, the flexural parameter and the rigidity
    fn line_contribution_hat(&self, v0: Fixed, perp_km: Fixed) -> Result<Fixed, ReliefRefusal> {
        let v_hat = scaled::internal_line_load(v0).ok_or(ReliefRefusal::NotRepresentable)?;
        let w0_hat =
            scaled::scaled_line_load_amplitude(v_hat, self.alpha_internal, self.rigidity_internal)
                .ok_or(ReliefRefusal::NotRepresentable)?;
        if w0_hat == Fixed::ZERO {
            return Ok(Fixed::ZERO);
        }
        // `X` is dimensionless and scale-free, so it is taken in INTERNAL units here where both operands are
        // the ones this plate already holds: `x_hat / alpha_hat` is the same number as `x / alpha`.
        let perp_hat =
            scaled::internal_length(perp_km.abs()).ok_or(ReliefRefusal::NotRepresentable)?;
        let big_x = perp_hat
            .checked_div(self.alpha_internal)
            .ok_or(ReliefRefusal::NotRepresentable)?;
        let (sin_x, cos_x) = big_x.sin_cos();
        let oscillation = cos_x
            .checked_add(sin_x)
            .ok_or(ReliefRefusal::NotRepresentable)?;
        if oscillation == Fixed::ZERO {
            // The zero crossing at `X = 3 pi / 4`: the deflection vanishes and its logarithm does not exist.
            return Ok(Fixed::ZERO);
        }
        let ln_w = w0_hat
            .abs()
            .ln()
            .checked_sub(big_x)
            .and_then(|x| x.checked_add(oscillation.abs().ln()))
            .ok_or(ReliefRefusal::NotRepresentable)?;
        let magnitude = ln_w.exp();
        let negative = (w0_hat < Fixed::ZERO) != (oscillation < Fixed::ZERO);
        if negative {
            Fixed::ZERO
                .checked_sub(magnitude)
                .ok_or(ReliefRefusal::NotRepresentable)
        } else {
            Ok(magnitude)
        }
    }

    /// THE CERTIFIED BAND ON A DEFLECTION READING, in kilometres: an upper bound on how much the point-load
    /// Green's function's own far-field truncation could have removed from
    /// [`Self::deflection_km`] at this query point.
    ///
    /// # WHY A DEFLECTION NEEDS A BAND AT ALL
    ///
    /// [`crate::flexure::kelvin_kei`] returns exactly zero past its series domain, and
    /// [`Self::point_contribution_hat`] multiplies that suppressed value by a coefficient carrying the LOAD
    /// MAGNITUDE. So the absolute error scales with the load rather than staying at some fixed relative size,
    /// it ACCUMULATES over a load list, and `deflection_km` returned `Ok` with no way to say any of it. There
    /// is also a step discontinuity at the cut radius, and this is what bounds its height.
    ///
    /// The bound is [`crate::flexure::kelvin_kei_far_field_bound`], which is an inequality derived from the
    /// `K_0` integral representation rather than an extrapolation of the observed decay, evaluated at each
    /// truncated load's own radius and weighted by that load's own coefficient. Loads inside the series
    /// domain contribute nothing to it, because nothing of theirs was suppressed. A reading of zero therefore
    /// means every load was evaluated, not that the band was skipped.
    ///
    /// Line and strip loads are absent from the sum because their kernel has no such cut: its own far-field
    /// underflow is carried through logarithms and reported to the last representable bit
    /// ([`crate::flexure::line_load_deflection`]). The sibling Kelvin functions `ker` and `kei'` carry the
    /// same cut and feed CURVATURE rather than deflection, so their bound belongs with their consumer and is
    /// not claimed here.
    // @derives: the certified far-field truncation band on a deflection reading <- each truncated point load's coefficient and radius
    pub fn far_field_tail_bound_km(
        &self,
        loads: &[Load],
        qx_km: Fixed,
        qy_km: Fixed,
    ) -> Result<Fixed, ReliefRefusal> {
        let cut = Fixed::from_int(crate::flexure::KEI_SERIES_MAX);
        let mut total_hat_bits = 0_i128;
        for load in loads {
            if load.kind != LoadKind::Point {
                continue;
            }
            if !point_load_admissible(load.magnitude) {
                return Err(ReliefRefusal::LoadOutsideEnvelope);
            }
            let dx = qx_km
                .checked_sub(load.x)
                .ok_or(ReliefRefusal::NotRepresentable)?;
            let dy = qy_km
                .checked_sub(load.y)
                .ok_or(ReliefRefusal::NotRepresentable)?;
            let r = dx
                .checked_mul(dx)
                .and_then(|x2| dy.checked_mul(dy).and_then(|y2| x2.checked_add(y2)))
                .ok_or(ReliefRefusal::NotRepresentable)?
                .sqrt();
            let (coef_hat, arg) = self.point_coefficient_and_argument(load.magnitude, r)?;
            if arg <= cut {
                // Evaluated by the series, so nothing of this load was suppressed.
                continue;
            }
            let suppressed = coef_hat
                .abs()
                .checked_mul(crate::flexure::kelvin_kei_far_field_bound(arg))
                .ok_or(ReliefRefusal::NotRepresentable)?;
            total_hat_bits = total_hat_bits
                .checked_add(i128::from(suppressed.to_bits()))
                .ok_or(ReliefRefusal::NotRepresentable)?;
        }
        let total_hat =
            Fixed::from_bits_i128(total_hat_bits).ok_or(ReliefRefusal::NotRepresentable)?;
        scaled::external_length(total_hat).ok_or(ReliefRefusal::NotRepresentable)
    }

    /// The point-load coefficient `Q0 l^2 / (2 pi D)` and the Kelvin argument `r / l`, in INTERNAL units.
    ///
    /// ONE HOME, because the deflection and its truncation band both need exactly this pair and a second copy
    /// is the redundant-parameter diamond this repository keeps paying for: a band computed from a slightly
    /// different coefficient than the value it bands is worse than no band.
    fn point_coefficient_and_argument(
        &self,
        q0: Fixed,
        r_km: Fixed,
    ) -> Result<(Fixed, Fixed), ReliefRefusal> {
        let q_hat = scaled::internal_force(q0).ok_or(ReliefRefusal::NotRepresentable)?;
        let l_hat = self.axisymmetric_length_internal;
        let l2_hat = l_hat
            .checked_mul(l_hat)
            .ok_or(ReliefRefusal::NotRepresentable)?;
        let two_pi_d_hat = Fixed::from_int(2)
            .checked_mul(Fixed::PI)
            .and_then(|x| x.checked_mul(self.rigidity_internal))
            .ok_or(ReliefRefusal::NotRepresentable)?;
        let coef_hat = q_hat
            .checked_mul(l2_hat)
            .and_then(|x| x.checked_div(two_pi_d_hat))
            .ok_or(ReliefRefusal::NotRepresentable)?;
        let r_hat = scaled::internal_length(r_km.abs()).ok_or(ReliefRefusal::NotRepresentable)?;
        let arg = r_hat
            .checked_div(l_hat)
            .ok_or(ReliefRefusal::NotRepresentable)?;
        Ok((coef_hat, arg))
    }

    /// One point load's contribution, in INTERNAL length.
    ///
    /// `w(r) = Q0 (l^2 / (2 pi D)) kei(r / l)`, on the AXISYMMETRIC length `l`, never on `alpha`. The `2 pi D`
    /// is formed internally because in the caller's units it overflowed for any `D` past `3.4e8`.
    ///
    /// PAST THE KELVIN SERIES DOMAIN THIS RETURNS ZERO, which is a truncation and not a value.
    /// [`Self::far_field_tail_bound_km`] is the certified bound on what that removed, and a caller reading a
    /// deflection under a distant load reads it too.
    // @derives: one point load's plate deflection <- the load magnitude, the axisymmetric length and the rigidity
    fn point_contribution_hat(&self, q0: Fixed, r_km: Fixed) -> Result<Fixed, ReliefRefusal> {
        let (coef_hat, arg) = self.point_coefficient_and_argument(q0, r_km)?;
        let raw = coef_hat
            .checked_mul(kelvin_kei(arg))
            .ok_or(ReliefRefusal::NotRepresentable)?;
        // THE SIGN NORMALIZATION, and it belongs HERE rather than in the kernel. The two Green's functions agree
        // that a positive magnitude is a downward load and DISAGREE on how to report the resulting deflection:
        // `line_load_deflection` calls the depression positive and its forebulge "the upward forebulge"
        // (negative), while `point_load_deflection` returns `kei(0) = -pi/4` and calls THAT the central
        // depression. Each is faithful to its own primary (Turcotte and Schubert for the beam, Brotchie and
        // Silvester for the axisymmetric plate), so neither kernel is wrong on its own terms and neither is
        // edited: what was wrong is SUPERPOSING them, which had a ridge and a volcano of the same physical sense
        // pulling the plate in opposite directions. Found by an independent audit of this substrate.
        //
        // This layer declares ONE convention and converts into it, which is what a composition layer is for: the
        // kernels stay checkable against their papers and the sum becomes meaningful. The convention is the
        // line's, because it is the one the strip inherits by integration and the one the isostasy consumer
        // reads, and it is pinned by `every_load_kind_deflects_the_same_way_for_the_same_signed_magnitude`.
        Fixed::ZERO
            .checked_sub(raw)
            .ok_or(ReliefRefusal::NotRepresentable)
    }

    /// One uniform strip load's contribution, in INTERNAL length.
    ///
    /// The strip's pressure is integrated across its caller-supplied half-width through the closed form in
    /// [`crate::flexure::scaled::scaled_uniform_strip_load_deflection`]. Pressure needs no boundary conversion:
    /// the internal stress unit is one GPa, the same unit the external coherent flexure system uses.
    // @derives: one uniform strip load's plate deflection <- the load pressure and footprint, the flexural parameter and restoring modulus
    fn uniform_strip_contribution_hat(
        &self,
        pressure: Fixed,
        half_width_km: Fixed,
        perp_km: Fixed,
    ) -> Result<Fixed, ReliefRefusal> {
        let half_width_hat =
            scaled::internal_length(half_width_km).ok_or(ReliefRefusal::NotRepresentable)?;
        let perp_hat = scaled::internal_length(perp_km).ok_or(ReliefRefusal::NotRepresentable)?;
        scaled::scaled_uniform_strip_load_deflection(
            pressure,
            half_width_hat,
            perp_hat,
            self.alpha_internal,
            self.restoring_internal,
        )
        .ok_or(ReliefRefusal::NotRepresentable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flexure::{flexural_rigidity, MAX_LINE_LOAD_GPA_KM};
    use crate::geodynamics::airy_isostatic_elevation;

    fn f64_of(x: Fixed) -> f64 {
        x.to_f64_lossy()
    }

    /// Mars-class restoring term: a 3300 kg/m^3 contrast at 3.71 m/s^2.
    fn mars_restoring() -> (Fixed, Fixed) {
        (Fixed::from_ratio(33, 10), Fixed::from_ratio(371, 100_000))
    }

    /// The sluggish column's converged internal rigidity, straight from the moment-equivalence solve
    /// (`the_sluggish_lid_solves_where_its_own_ceiling_used_to_overflow`). Its dimensional value, `2.9e9 GPa
    /// km^3`, is the one `Fixed` cannot hold, which is why this fixture is stated internally.
    fn sluggish_plate() -> FlexedPlate {
        FlexedPlate::from_internal_rigidity(
            InternalRigidity::from_internal(Fixed::from_ratio(8874728, 100)),
            mars_restoring().0,
            mars_restoring().1,
        )
        .expect("a converged plate loads")
    }

    fn line_load(v0: Fixed, x: Fixed) -> Load {
        Load {
            kind: LoadKind::LineY,
            magnitude: v0,
            x,
            y: Fixed::ZERO,
        }
    }

    fn uniform_strip_load(pressure: Fixed, x: Fixed, half_width: Fixed) -> Load {
        Load {
            kind: LoadKind::UniformStripY { half_width },
            magnitude: pressure,
            x,
            y: Fixed::ZERO,
        }
    }

    #[test]
    fn the_two_flexural_lengths_are_distinct_and_correctly_related() {
        // THE DEFECT THIS PINS was real in this codebase until 2026-07-17: the point-load Green's function ran
        // on the LINE-load parameter, which made the moat sqrt(2) too wide and twice too deep. The relation is
        // `l = alpha / sqrt(2)` exactly, so a rebuild that welds them again fails here rather than quietly
        // widening every basin in the world.
        let plate = sluggish_plate();
        let alpha = f64_of(plate.flexural_parameter_km().expect("alpha"));
        let l = f64_of(plate.axisymmetric_length_km().expect("l"));
        assert!(
            (alpha / l - 2.0f64.sqrt()).abs() < 1e-6,
            "alpha / l must be sqrt(2): {alpha} / {l} = {}",
            alpha / l
        );
    }

    #[test]
    fn the_line_load_amplitude_is_the_closed_form() {
        // MAGNITUDE, CHECKED AGAINST THE ALGEBRA RATHER THAN AGAINST ITSELF. `w0 = V0 alpha^3 / (8 D)`, and
        // with `V0 = 5.4 GPa km`, `alpha = 987.29 km` and `D = 2.908e9 GPa km^3` that is 0.2234 km. A solve
        // that is self-consistent but scaled wrong would pass every internal comparison and fail here.
        let plate = sluggish_plate();
        let alpha = f64_of(plate.flexural_parameter_km().expect("alpha"));
        // THE DIMENSIONAL RIGIDITY, through the declared representation constant rather than a literal
        // `32768.0` typed out here. This line was reverse-engineering the crate-private internal scale by
        // hand, which is the `InternalRigidity` finding seen from inside the crate. The plate's `D` is
        // `2.9e9 GPa km^3` and does not fit `Fixed`, so `to_gpa_km3` correctly refuses and the comparison
        // is made in `f64` against the same declared scale the kernel uses.
        assert!(
            plate.rigidity_internal().to_gpa_km3().is_none(),
            "this fixture is the plate whose dimensional rigidity does not fit, which is why it is stated \
             internally"
        );
        let d = f64_of(plate.rigidity_internal().internal())
            * f64::from(scaled::INTERNAL_RIGIDITY_GPA_KM3);
        let v0 = 5.4;
        let expected = v0 * alpha.powi(3) / (8.0 * d);
        let got = f64_of(
            plate
                .deflection_km(
                    &[line_load(Fixed::from_ratio(54, 10), Fixed::ZERO)],
                    Fixed::ZERO,
                    Fixed::ZERO,
                )
                .expect("the load deflects the plate"),
        );
        assert!(
            (got - expected).abs() < expected * 1e-3,
            "the central deflection is the closed-form amplitude: {got} against {expected}"
        );
    }

    #[test]
    fn the_relief_has_a_forebulge_which_is_what_airy_flotation_cannot_produce() {
        // THE SIGNATURE OF FLEXURE, and the reason this module exists rather than the surface staying on Airy
        // isostasy. Under Airy every column floats alone: relief is the local column's own buoyancy, neighbours
        // say nothing to each other, and a load can only ever push its own column down. A plate BENDS, so the
        // material displaced under the load has to go somewhere, and it rises beyond the load as a forebulge.
        // The deflection therefore CHANGES SIGN, which no independent-column model can do at any parameter.
        //
        // The crossing is at `X = 3 pi / 4` where `cos X + sin X` vanishes, so it sits at `2.356 alpha`. That is
        // a derived location rather than a fitted one: it moves with the plate's own stiffness and nothing here
        // selects it.
        let plate = sluggish_plate();
        let alpha = f64_of(plate.flexural_parameter_km().expect("alpha"));
        let loads = [line_load(Fixed::from_ratio(54, 10), Fixed::ZERO)];
        let at = |x: i32| {
            f64_of(
                plate
                    .deflection_km(&loads, Fixed::from_int(x), Fixed::ZERO)
                    .expect("evaluates"),
            )
        };
        let near = at(0);
        assert!(near > 0.0, "the load deflects its own column: {near}");
        // Monotone decay through the near field.
        for pair in [(0, 400), (400, 800), (800, 1200), (1200, 1600)] {
            assert!(
                at(pair.0) > at(pair.1),
                "the deflection decays outward from {} to {} km: {} against {}",
                pair.0,
                pair.1,
                at(pair.0),
                at(pair.1)
            );
        }
        // AND IT REVERSES. The crossing at 2.356 alpha is about 2326 km for this plate, so the far side is up.
        let crossing = 2.356 * alpha;
        let inside = at((crossing * 0.8) as i32);
        let outside = at((crossing * 1.25) as i32);
        assert!(
            inside > 0.0 && outside < 0.0,
            "the deflection changes sign across {crossing:.0} km: {inside} inside, {outside} outside"
        );
        // The forebulge is small beside the load's own depression, which is the Green's function's own shape.
        assert!(
            outside.abs() < near * 0.2,
            "the forebulge is a fraction of the central deflection: {outside} against {near}"
        );
    }

    #[test]
    fn superposition_does_not_depend_on_the_order_the_loads_are_listed_in() {
        // THE DETERMINISM CONTRACT (Principle 3, Principle 10). `Fixed` addition is exact and associative, so
        // two runs that discover the same loads in different orders must produce the same world to the BIT.
        // Asserted to the bit rather than within a tolerance, because that is what the contract says.
        let plate = sluggish_plate();
        let a = line_load(Fixed::from_ratio(54, 10), Fixed::ZERO);
        let b = line_load(Fixed::from_ratio(21, 10), Fixed::from_int(700));
        let c = Load {
            kind: LoadKind::Point,
            magnitude: Fixed::from_int(400),
            x: Fixed::from_int(-300),
            y: Fixed::from_int(150),
        };
        let q = (Fixed::from_int(120), Fixed::from_int(60));
        let forward = plate.deflection_km(&[a, b, c], q.0, q.1).expect("forward");
        let reversed = plate.deflection_km(&[c, b, a], q.0, q.1).expect("reversed");
        assert_eq!(
            forward.to_bits(),
            reversed.to_bits(),
            "the superposition is order-independent to the bit"
        );
    }

    #[test]
    fn the_refusal_does_not_depend_on_the_order_the_loads_are_listed_in() {
        // THE STOP IS PART OF THE ANSWER. The deflection was already order-independent to the bit, but the
        // REFUSAL was not: returning on the first bad load meant a list holding one over-envelope load and one
        // zero-footprint load reported whichever the caller listed first, so two runs discovering the same loads
        // in different orders disagreed about why the world refused. Found by an independent audit.
        let plate = sluggish_plate();
        let bad_footprint = Load {
            kind: LoadKind::UniformStripY {
                half_width: Fixed::ZERO,
            },
            magnitude: Fixed::ONE,
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        };
        let bad_envelope = Load {
            kind: LoadKind::LineY,
            magnitude: Fixed::from_int(crate::flexure::MAX_LINE_LOAD_GPA_KM)
                .checked_mul(Fixed::from_int(4))
                .expect("past the envelope"),
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        };
        let forward = plate.deflection_km(&[bad_footprint, bad_envelope], Fixed::ZERO, Fixed::ZERO);
        let reversed =
            plate.deflection_km(&[bad_envelope, bad_footprint], Fixed::ZERO, Fixed::ZERO);
        assert_eq!(
            forward, reversed,
            "the same load set must refuse for the same reason in either order"
        );
        assert_eq!(
            forward,
            Err(ReliefRefusal::FootprintNotPositive),
            "and the declared precedence stands: a load with no footprint is not a geometry at all"
        );
    }

    #[test]
    fn every_load_kind_deflects_the_same_way_for_the_same_signed_magnitude() {
        // THE SUPERPOSITION INVARIANT, and the one this substrate was missing. A sum over load kinds is only
        // meaningful if the kinds agree on what a sign MEANS. They did not: the same-signed magnitude gave
        // +0.2234 km from a line and -0.0084 km from a point, so a ridge and a volcano of the same physical
        // sense fought each other inside the sum, and a load list's answer depended on which kinds happened to
        // be in it. The kernels are each faithful to their own primary and are unedited; this layer converts.
        let plate = sluggish_plate();
        let line = Load {
            kind: LoadKind::LineY,
            magnitude: Fixed::from_ratio(54, 10),
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        };
        let strip = Load {
            kind: LoadKind::UniformStripY {
                half_width: Fixed::from_int(100),
            },
            magnitude: Fixed::from_ratio(1, 100),
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        };
        let point = Load {
            kind: LoadKind::Point,
            magnitude: Fixed::from_int(400),
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        };
        let at = |l: Load| {
            f64_of(
                plate
                    .deflection_km(&[l], Fixed::ZERO, Fixed::ZERO)
                    .expect("evaluates"),
            )
        };
        let (wl, ws, wp) = (at(line), at(strip), at(point));
        assert!(
            wl > 0.0 && ws > 0.0 && wp > 0.0,
            "every kind must deflect the same way under the same-signed magnitude: line {wl}, strip {ws}, point {wp}"
        );
        // AND THE SUM MUST NOT CANCEL. Before the normalization the line and point contributions subtracted,
        // so a list holding both read LOWER than the line alone, which is the defect in the form a consumer
        // would have met it.
        let both = f64_of(
            plate
                .deflection_km(&[line, point], Fixed::ZERO, Fixed::ZERO)
                .expect("evaluates"),
        );
        assert!(
            both > wl,
            "two loads of the same sense must add rather than cancel: {both} against {wl} for the line alone"
        );
    }

    #[test]
    fn no_load_is_a_legitimate_state_and_its_relief_is_flat() {
        let plate = sluggish_plate();
        assert_eq!(
            plate
                .deflection_km(&[], Fixed::from_int(100), Fixed::from_int(100))
                .expect("an empty list is not a failure"),
            Fixed::ZERO
        );
    }

    #[test]
    fn a_plate_floating_on_nothing_refuses_rather_than_returning_a_length() {
        assert_eq!(
            FlexedPlate::from_internal_rigidity(
                InternalRigidity::from_internal(Fixed::from_int(1000)),
                Fixed::ZERO,
                mars_restoring().1
            ),
            Err(ReliefRefusal::RestoringTermNotPositive)
        );
        assert_eq!(
            FlexedPlate::from_internal_rigidity(
                InternalRigidity::from_internal(Fixed::ZERO),
                mars_restoring().0,
                mars_restoring().1
            ),
            Err(ReliefRefusal::RigidityNotPositive)
        );
    }

    #[test]
    fn a_stiffer_plate_spreads_its_relief_wider_and_bends_less_under_the_same_load() {
        // THE EMERGENT READING, and the thing a viewer will show. Stiffness sets BOTH the width and the depth:
        // `alpha ~ D^(1/4)` so a stiffer plate is felt further, while `w0 ~ alpha^3 / D ~ D^(-1/4)` so it is
        // felt less. A cold sluggish world therefore has BROAD GENTLE relief and a warm soft one has narrow
        // sharp relief, out of the same load and with nothing selecting the outcome.
        let soft = FlexedPlate::from_internal_rigidity(
            InternalRigidity::from_internal(Fixed::from_int(1000)),
            mars_restoring().0,
            mars_restoring().1,
        )
        .expect("soft");
        let stiff = sluggish_plate();
        let load = [line_load(Fixed::from_ratio(54, 10), Fixed::ZERO)];
        let soft_alpha = f64_of(soft.flexural_parameter_km().expect("alpha"));
        let stiff_alpha = f64_of(stiff.flexural_parameter_km().expect("alpha"));
        assert!(
            stiff_alpha > soft_alpha,
            "the stiffer plate is felt further: {stiff_alpha} against {soft_alpha}"
        );
        let soft_w = f64_of(
            soft.deflection_km(&load, Fixed::ZERO, Fixed::ZERO)
                .expect("soft deflects"),
        );
        let stiff_w = f64_of(
            stiff
                .deflection_km(&load, Fixed::ZERO, Fixed::ZERO)
                .expect("stiff deflects"),
        );
        assert!(
            stiff_w < soft_w,
            "and it bends less under the same load: {stiff_w} against {soft_w}"
        );
    }

    #[test]
    fn a_uniform_strip_converges_numerically_to_the_same_columns_airy_elevation() {
        // INDEPENDENT COLUMN INPUTS, never back-solved from the target. The density and thickness are the felsic
        // column already anchored by `geodynamics::a_lighter_crust_floats_higher_than_a_denser_one`, and gravity
        // is the Earth-like flexure fixture. Its load pressure is derived separately as (rho_m - rho_c) g h.
        // The full strip width is the 1000 km coarse-province scale this flexure substrate already declares in
        // its module contract, supplied as load data rather than a kernel value.
        let rho_m = Fixed::from_ratio(33, 10);
        let rho_c = Fixed::from_ratio(265, 100);
        let gravity = Fixed::from_ratio(98, 10_000);
        let thickness_km = Fixed::from_int(35);
        let half_width_km = Fixed::from_int(500);
        let pressure = rho_m
            .checked_sub(rho_c)
            .and_then(|contrast| contrast.checked_mul(thickness_km))
            .and_then(|column_contrast| column_contrast.checked_mul(gravity))
            .expect("the column derives a load pressure");
        let airy_km = airy_isostatic_elevation(rho_c, rho_m, Fixed::from_int(35_000))
            .and_then(|metres| metres.checked_div(Fixed::from_int(1_000)))
            .expect("the same column has an Airy elevation");
        let load = uniform_strip_load(pressure, Fixed::ZERO, half_width_km);

        // The rigidity values derive from the same E and nu at decreasing elastic thicknesses across the
        // kernel's declared 5 to 800 km validation envelope. No D is selected from the Airy answer or from a
        // desired residual.
        let mut previous_d = None;
        let mut previous_residual = None;
        for elastic_thickness_km in [40, 20, 10, 5] {
            let d = flexural_rigidity(
                Fixed::from_int(70),
                Fixed::from_ratio(1, 4),
                Fixed::from_int(elastic_thickness_km),
            )
            .expect("the test plate derives a rigidity");
            let d_internal = scaled::internal_rigidity(d).expect("the rigidity converts inward");
            let plate = FlexedPlate::from_internal_rigidity(d_internal, rho_m, gravity)
                .expect("the distributed load has a plate");
            let flexural_km = plate
                .deflection_km(&[load], Fixed::ZERO, Fixed::ZERO)
                .expect("the distributed load evaluates");
            let residual = flexural_km
                .checked_sub(airy_km)
                .expect("the residual is representable")
                .abs();
            eprintln!(
                "Airy sweep: D={:.12} GPa km^3, alpha={:.12} km, w={:.12} km, Airy={:.12} km, residual={:.12} km",
                d.to_f64_lossy(),
                plate.flexural_parameter_km().expect("alpha").to_f64_lossy(),
                flexural_km.to_f64_lossy(),
                airy_km.to_f64_lossy(),
                residual.to_f64_lossy(),
            );
            if let Some(prior) = previous_d {
                assert!(d < prior, "the derived rigidity sweep must decrease");
            }
            if let Some(prior) = previous_residual {
                assert!(
                    residual < prior,
                    "this independently selected Airy sweep must shrink at each decreasing D: {} against {}",
                    residual.to_f64_lossy(),
                    prior.to_f64_lossy()
                );
            }
            previous_d = Some(d);
            previous_residual = Some(residual);
        }
    }

    /// THE FAR-FIELD TRUNCATION IS REPORTED, AND THE REPORT IS A BOUND ON THE STEP IT LEAVES.
    ///
    /// The point-load Green's function returns exactly zero past the Kelvin series domain, and the consumer
    /// multiplies that suppressed value by a coefficient carrying the LOAD MAGNITUDE. So the error scales
    /// with the load and accumulates across a load list, and `deflection_km` returned `Ok` with no channel
    /// for any of it. The cut also leaves a STEP at `r = 12 l`, and the band is what bounds its height.
    #[test]
    fn the_far_field_truncation_is_bounded_and_reported_rather_than_dropped() {
        let plate = sluggish_plate();
        let l = f64_of(plate.axisymmetric_length_km().expect("l"));
        let cut_km = 12.0 * l;
        let point = |q0: i64, x_km: f64| Load {
            kind: LoadKind::Point,
            magnitude: Fixed::from_ratio(q0, 1),
            x: Fixed::from_ratio((x_km * 1000.0) as i64, 1000),
            y: Fixed::ZERO,
        };

        // A load INSIDE the series domain is fully evaluated, so nothing was suppressed and the band is zero.
        let near = [point(400, cut_km * 0.5)];
        assert_eq!(
            plate
                .far_field_tail_bound_km(&near, Fixed::ZERO, Fixed::ZERO)
                .expect("the band evaluates"),
            Fixed::ZERO,
            "a load the series reached has nothing suppressed to report"
        );

        // A load PAST it is truncated to zero, and the band says by how much at most.
        let far = [point(400, cut_km * 1.05)];
        let w_far = plate
            .deflection_km(&far, Fixed::ZERO, Fixed::ZERO)
            .expect("evaluates");
        assert_eq!(
            w_far,
            Fixed::ZERO,
            "the far load really is truncated, or this test is moot"
        );
        let band = plate
            .far_field_tail_bound_km(&far, Fixed::ZERO, Fixed::ZERO)
            .expect("the band evaluates");
        assert!(
            band > Fixed::ZERO,
            "and the truncation is REPORTED rather than dropped: {}",
            f64_of(band)
        );

        // THE BAND BOUNDS THE STEP. Just inside the cut the kernel returns the last value the series can
        // give; just outside it returns zero. That jump is the discontinuity, and the band evaluated outside
        // must cover it.
        let inside = [point(400, cut_km * 0.999)];
        let w_inside = plate
            .deflection_km(&inside, Fixed::ZERO, Fixed::ZERO)
            .expect("evaluates")
            .abs();
        assert!(
            w_inside > Fixed::ZERO,
            "the last representable value inside the cut is not zero: {}",
            f64_of(w_inside)
        );
        assert!(
            f64_of(band) >= f64_of(w_inside),
            "the reported band must cover the step the cut leaves: band {} against a step of {}",
            f64_of(band),
            f64_of(w_inside)
        );

        // IT ACCUMULATES OVER THE LIST, which is the property that made the unreported error compound, and it
        // does not depend on the order the loads are listed in.
        let many = [
            point(400, cut_km * 1.05),
            point(400, cut_km * 1.05),
            point(400, cut_km * 1.05),
        ];
        let accumulated = plate
            .far_field_tail_bound_km(&many, Fixed::ZERO, Fixed::ZERO)
            .expect("evaluates");
        assert!(
            f64_of(accumulated) > f64_of(band) * 2.5,
            "three truncated loads carry about three times the band of one: {} against {}",
            f64_of(accumulated),
            f64_of(band)
        );
        let mut reversed = many;
        reversed.reverse();
        assert_eq!(
            accumulated.to_bits(),
            plate
                .far_field_tail_bound_km(&reversed, Fixed::ZERO, Fixed::ZERO)
                .expect("evaluates")
                .to_bits(),
            "the band is order-independent to the bit, like the deflection it bands"
        );

        // A BIGGER LOAD CARRIES A BIGGER BAND, which is the whole point: the suppressed absolute error scales
        // with the load magnitude rather than staying at some fixed size.
        let heavy = [point(4000, cut_km * 1.05)];
        assert!(
            f64_of(
                plate
                    .far_field_tail_bound_km(&heavy, Fixed::ZERO, Fixed::ZERO)
                    .expect("evaluates")
            ) > f64_of(band) * 5.0,
            "the band scales with the load magnitude"
        );

        // Line and strip loads have no such cut and contribute nothing to the band.
        let line = [line_load(
            Fixed::from_ratio(54, 10),
            Fixed::from_int(100_000),
        )];
        assert_eq!(
            plate
                .far_field_tail_bound_km(&line, Fixed::ZERO, Fixed::ZERO)
                .expect("evaluates"),
            Fixed::ZERO
        );
    }

    // ----- THE PERIODIC SPECTRAL SOLVE -----

    /// A PERIODIC UNIFORM LOAD IS FLAT TO THE BIT, which is the requirement the review named and the one the
    /// truncated superposition cannot meet at any row length.
    ///
    /// On a translationally invariant periodic plate a uniform load has a uniform response, necessarily. The
    /// direct superposition answers a visibly non-flat profile for a uniform row, and this module's own
    /// consumer had to assert only the surviving mirror symmetry because of it. Here the answer is flat to
    /// the BIT and identically the isostatic value `q / (delta_rho g)`, at any rigidity, because `Phi(0)` is
    /// exactly one and the mean is split off before the transform runs.
    #[test]
    fn a_periodic_uniform_load_is_flat_to_the_bit() {
        let plate = sluggish_plate();
        let cell_width = Fixed::from_int(60);
        for pressure in [
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(-3, 100),
            Fixed::ZERO,
        ] {
            for n in [1usize, 2, 7, 8, 11, 32] {
                let row = vec![pressure; n];
                let profile = plate
                    .periodic_deflection_km(&row, cell_width)
                    .expect("a uniform periodic row solves");
                assert_eq!(profile.len(), n);
                for (i, w) in profile.iter().enumerate() {
                    assert_eq!(
                        w.to_bits(),
                        profile[0].to_bits(),
                        "a uniform periodic load must be flat to the bit: n = {n}, index {i} reads {} \
                         against {}",
                        w.to_f64_lossy(),
                        profile[0].to_f64_lossy()
                    );
                }
                // AND IT IS THE ISOSTATIC VALUE, so flatness at the wrong height would still fail. The
                // restoring modulus is the plate's own, read back through the declared internal scale.
                let restoring = f64_of(plate.restoring_internal) / 32.0;
                let expected = f64_of(pressure) / restoring;
                assert!(
                    (f64_of(profile[0]) - expected).abs() <= expected.abs() * 1e-9 + 1e-9,
                    "the flat value is the isostatic one: {} against q/(delta_rho g) = {expected}",
                    f64_of(profile[0])
                );
            }
        }
    }

    /// THE PERIODIC SOLVE REPRODUCES THE INFINITE-PLATE GREEN'S FUNCTION, which is the cross-validation that
    /// makes it evidence rather than self-consistency.
    ///
    /// The two paths share no arithmetic: one convolves the Turcotte and Schubert closed form over each
    /// cell's footprint in the length domain, the other multiplies a discrete transform by `1 / (1 + (l k)^4)`
    /// in the wavenumber domain. They must agree in the interior of a row long against the flexural
    /// parameter, where the periodic images are far enough away to be negligible, and disagreement of a
    /// factor near two or `sqrt(2)` is what a confusion of `alpha` for `l` in the transfer function would
    /// produce. The mean is subtracted from BOTH sides, because the spectral solve floats the row's mean
    /// isostatically while the truncated superposition has no mean to float.
    #[test]
    fn the_periodic_solve_reproduces_the_infinite_plate_green_function() {
        // An Earth-like plate: alpha near 91 km, so a 64-cell row at 60 km spacing is a domain of 3840 km,
        // some forty flexural parameters wide.
        let d = flexural_rigidity(
            Fixed::from_int(70),
            Fixed::from_ratio(1, 4),
            Fixed::from_int(40),
        )
        .expect("the fixture plate has a rigidity");
        let rho_m = Fixed::from_ratio(33, 10);
        let gravity = Fixed::from_ratio(98, 10_000);
        let plate = FlexedPlate::from_rigidity_gpa_km3(d, rho_m, gravity).expect("plate");
        let alpha = f64_of(plate.flexural_parameter_km().expect("alpha"));
        let pressure = Fixed::from_ratio(1, 100);
        // ONE PHYSICAL LOAD held fixed across every sweep: a 60 km wide strip of the same pressure. Only the
        // grid the periodic solve samples it on, and the domain it wraps in, change.
        let footprint_km = 60_i32;
        let strip = uniform_strip_load(
            pressure,
            Fixed::ZERO,
            Fixed::from_ratio(i64::from(footprint_km), 2),
        );

        // The worst disagreement between the two methods over one grid, in km, plus the direct peak it is
        // measured against. Both are compared as DEVIATIONS FROM THEIR OWN MEAN, because the spectral answer
        // floats the row's mean isostatically and the infinite-plate one has no mean to float; the deviation
        // is the quantity the two methods share.
        let disagreement = |cell_km: i32, loaded_cells: usize, n: usize| -> (f64, f64) {
            assert_eq!(loaded_cells * (cell_km as usize), footprint_km as usize);
            assert_eq!(loaded_cells % 2, 1, "an odd cell count centres on a cell");
            let centre = n / 2;
            let mut row = vec![Fixed::ZERO; n];
            for j in 0..loaded_cells {
                row[centre + j - loaded_cells / 2] = pressure;
            }
            let spectral = plate
                .periodic_deflection_km(&row, Fixed::from_int(cell_km))
                .expect("the periodic row solves");
            let direct = |i: usize| -> f64 {
                let offset = (i as i32 - centre as i32) * cell_km;
                f64_of(
                    plate
                        .deflection_km(&[strip], Fixed::from_int(offset), Fixed::ZERO)
                        .expect("the strip evaluates"),
                )
            };
            let spectral_mean: f64 = spectral.iter().map(|w| f64_of(*w)).sum::<f64>() / n as f64;
            let direct_mean: f64 = (0..n).map(direct).sum::<f64>() / n as f64;
            let peak = direct(centre) - direct_mean;
            let worst = (0..n)
                .map(|i| ((f64_of(spectral[i]) - spectral_mean) - (direct(i) - direct_mean)).abs())
                .fold(0.0_f64, f64::max);
            (worst, peak)
        };

        // ----- SWEEP ONE: THE PERIODIC IMAGES, AND THE ANSWER IS THAT THEY ARE NOT THE RESIDUE -----
        //
        // The obvious candidate for the gap is the images the periodic domain carries every `N dx` and the
        // infinite plate does not, so it was tested first and it is REFUTED: lengthening the domain eightfold
        // moves the absolute disagreement by under a part in five hundred. The images really are negligible
        // this far past `alpha`, and recording that here keeps the next reader from re-deriving a wrong
        // explanation for a residue that has a different cause.
        let mut image_sweep = Vec::new();
        for n in [32usize, 64, 128, 256] {
            let (worst, peak) = disagreement(footprint_km, 1, n);
            eprintln!(
                "images: domain = {} km ({:.0} alpha), peak = {peak:.6} km, worst = {worst:.4e} km",
                n * footprint_km as usize,
                (n as f64 * f64::from(footprint_km)) / alpha
            );
            image_sweep.push(worst);
        }
        let image_spread = (image_sweep.iter().cloned().fold(0.0_f64, f64::max)
            - image_sweep.iter().cloned().fold(f64::INFINITY, f64::min))
            / image_sweep[0];
        assert!(
            image_spread < 0.01,
            "the disagreement is insensitive to domain length, so the images are not what it is: spread \
             {image_spread:.3e} over an eightfold domain, readings {image_sweep:?}"
        );

        // ----- SWEEP TWO: THE SAMPLED LOAD FIELD, WHICH IS THE RESIDUE -----
        //
        // The two methods are not solving quite the same load. The direct path convolves an exact RECTANGLE,
        // whose spectrum runs past the grid's Nyquist wavenumber; the spectral path can only carry the
        // rectangle's content BELOW Nyquist, so it solves a band-limited version of the same footprint. That
        // difference is set by the cell width against the flexural length and is untouched by the domain,
        // which is exactly the signature sweep one read. Refining the grid under the SAME physical load
        // must then close it, and it does.
        let mut refine_sweep = Vec::new();
        for (cell_km, loaded_cells, n) in [(60_i32, 1usize, 64usize), (20, 3, 192), (12, 5, 320)] {
            let (worst, peak) = disagreement(cell_km, loaded_cells, n);
            let relative = worst / peak;
            eprintln!(
                "grid: cell = {cell_km} km (alpha/dx = {:.1}), peak = {peak:.6} km, worst = {worst:.4e} km \
                 ({relative:.3e} of the peak)",
                alpha / f64::from(cell_km)
            );
            refine_sweep.push(relative);
        }
        for pair in refine_sweep.windows(2) {
            assert!(
                pair[1] < pair[0],
                "refining the grid under the same physical load must close the gap: {refine_sweep:?}"
            );
        }
        // AND IT CLOSES BY AN ORDER OF MAGNITUDE, which identifies the residue rather than tolerating it. A
        // welded flexural length is a CONSTANT factor near sqrt(2) or 2 that no refinement can touch, and
        // that is what this assertion separates the two cases by.
        assert!(
            refine_sweep[0] > refine_sweep[refine_sweep.len() - 1] * 5.0,
            "the residue must be the band-limited load rather than a wrong length scale: {refine_sweep:?}"
        );
    }

    /// THE SLOWLY-VARYING-RIGIDITY APPROXIMATION, MEASURED RATHER THAN ASSERTED.
    ///
    /// The spectral solve diagonalizes the plate operator, which needs `D` outside the derivative, so it is
    /// exact only for a plate of uniform rigidity. A row whose columns differ in lid strength is served by
    /// ONE `D`, and the owner's standing ruling on the per-column filter fork is that this approximation be
    /// NAMED and its cost MEASURED rather than presented as exact.
    ///
    /// The measurement is the only one available at this layer, and it is the right one: the cost of choosing
    /// one rigidity for the row is the spread of the answer across the rigidities the row's columns span. So
    /// the same load field is solved at a bracket of rigidities and the departure is reported. Nothing here
    /// authors a tolerance; what is asserted is the structure, that the cost is exactly zero at zero spread
    /// and grows with the spread, which is what makes it a cost rather than a coincidence.
    #[test]
    fn the_slowly_varying_rigidity_approximation_costs_what_the_rigidity_spread_costs() {
        let rho_m = Fixed::from_ratio(33, 10);
        let gravity = Fixed::from_ratio(98, 10_000);
        let cell_width = Fixed::from_int(60);
        let n = 32usize;
        // A ridge in a row: the load field a province row of differing thickness presents.
        let mut row = vec![Fixed::from_ratio(-1, 100); n];
        row[16] = Fixed::from_ratio(-4, 100);
        row[15] = Fixed::from_ratio(-3, 100);
        row[17] = Fixed::from_ratio(-3, 100);

        let profile_at = |t_e_km: i32| -> Vec<f64> {
            let d = flexural_rigidity(
                Fixed::from_int(70),
                Fixed::from_ratio(1, 4),
                Fixed::from_int(t_e_km),
            )
            .expect("rigidity");
            FlexedPlate::from_rigidity_gpa_km3(d, rho_m, gravity)
                .expect("plate")
                .periodic_deflection_km(&row, cell_width)
                .expect("solves")
                .iter()
                .map(|w| f64_of(*w))
                .collect()
        };
        let worst_departure = |a: &[f64], b: &[f64]| -> f64 {
            a.iter()
                .zip(b)
                .map(|(x, y)| (x - y).abs())
                .fold(0.0_f64, f64::max)
        };

        // The row's chosen rigidity is the 40 km lid; the spread is what its columns might really span.
        let chosen = profile_at(40);
        let mut previous = 0.0_f64;
        for (low, high) in [(40, 40), (38, 42), (35, 45), (30, 50)] {
            let cost = worst_departure(&chosen, &profile_at(low))
                .max(worst_departure(&chosen, &profile_at(high)));
            eprintln!(
                "slowly-varying-rigidity cost over T_e = {low}..{high} km: {cost:.6} km worst departure \
                 from the {} km row",
                40
            );
            if low == high {
                assert_eq!(
                    cost, 0.0,
                    "a row of uniform rigidity costs the approximation nothing at all"
                );
            } else {
                assert!(
                    cost > previous,
                    "a wider rigidity spread must cost more: {cost} against {previous}"
                );
            }
            previous = cost;
        }
    }

    #[test]
    fn the_periodic_solve_refuses_rather_than_answering_off_its_domain() {
        let plate = sluggish_plate();
        let row = [Fixed::from_ratio(1, 100); 4];
        assert_eq!(
            plate.periodic_deflection_km(&row, Fixed::ZERO),
            Err(ReliefRefusal::FootprintNotPositive)
        );
        assert_eq!(
            plate.periodic_deflection_km(&row, Fixed::from_int(-60)),
            Err(ReliefRefusal::FootprintNotPositive)
        );
        // A cell load past the envelope the strip Green's function is proved over refuses, and refuses the
        // same way whichever column carries it.
        let past = Fixed::from_int(MAX_LINE_LOAD_GPA_KM);
        for at in [0usize, 3] {
            let mut bad = [Fixed::from_ratio(1, 100); 4];
            bad[at] = past;
            assert_eq!(
                plate.periodic_deflection_km(&bad, Fixed::from_int(60)),
                Err(ReliefRefusal::LoadOutsideEnvelope),
                "the stop does not depend on which column carries the bad load"
            );
        }
        // An empty row is a legitimate state with an empty profile.
        assert_eq!(
            plate.periodic_deflection_km(&[], Fixed::from_int(60)),
            Ok(Vec::new())
        );
    }

    #[test]
    fn the_periodic_elevation_is_the_deflection_turned_the_right_way_up() {
        // A BUOYANT row is a NEGATIVE downward load and must stand UP, which is the inversion a review caught
        // in the superposing profile and which this typed boundary exists to prevent recurring.
        let plate = sluggish_plate();
        let mut row = vec![Fixed::from_ratio(-1, 100); 16];
        row[8] = Fixed::from_ratio(-4, 100);
        let down = plate
            .periodic_deflection_km(&row, Fixed::from_int(60))
            .expect("solves");
        let up = plate
            .periodic_elevation_km(&row, Fixed::from_int(60))
            .expect("solves");
        for (w, e) in down.iter().zip(&up) {
            assert_eq!(e.km().to_bits(), (Fixed::ZERO - *w).to_bits());
        }
        // The most buoyant column stands highest.
        let peak = up[8].km();
        for (i, e) in up.iter().enumerate() {
            if i != 8 {
                assert!(
                    e.km() < peak,
                    "the most buoyant column stands highest: index {i} at {} against {}",
                    f64_of(e.km()),
                    f64_of(peak)
                );
            }
        }
    }

    #[test]
    fn distributed_loads_superpose_order_independently_to_the_bit() {
        let plate = sluggish_plate();
        let strip = uniform_strip_load(
            Fixed::from_ratio(1, 100),
            Fixed::from_int(-100),
            Fixed::from_int(200),
        );
        let line = line_load(Fixed::from_ratio(21, 10), Fixed::from_int(700));
        let point = Load {
            kind: LoadKind::Point,
            magnitude: Fixed::from_int(400),
            x: Fixed::from_int(-300),
            y: Fixed::from_int(150),
        };
        let q = (Fixed::from_int(120), Fixed::from_int(60));
        let forward = plate
            .deflection_km(&[strip, line, point], q.0, q.1)
            .expect("forward");
        let reversed = plate
            .deflection_km(&[point, line, strip], q.0, q.1)
            .expect("reversed");
        assert_eq!(
            forward.to_bits(),
            reversed.to_bits(),
            "distributed-load superposition is order-independent to the bit"
        );
    }

    #[test]
    fn a_distributed_load_refuses_no_footprint_or_force_past_the_proven_envelope() {
        let plate = sluggish_plate();
        let no_footprint = uniform_strip_load(Fixed::ONE, Fixed::ZERO, Fixed::ZERO);
        assert_eq!(
            plate.deflection_km(&[no_footprint], Fixed::ZERO, Fixed::ZERO),
            Err(ReliefRefusal::FootprintNotPositive)
        );

        // At a one-kilometre half-width, MAX_LINE_LOAD / 2 is exactly the pressure whose integrated line load
        // reaches the existing envelope. One fixed-point step above it must refuse.
        let pressure_past_envelope = Fixed::from_ratio(i64::from(MAX_LINE_LOAD_GPA_KM), 2)
            .checked_add(Fixed::EPSILON)
            .expect("the boundary probe is representable");
        let too_large = uniform_strip_load(pressure_past_envelope, Fixed::ZERO, Fixed::ONE);
        assert_eq!(
            plate.deflection_km(&[too_large], Fixed::ZERO, Fixed::ZERO),
            Err(ReliefRefusal::LoadOutsideEnvelope)
        );

        let minimum_pressure = uniform_strip_load(Fixed::MIN, Fixed::ZERO, Fixed::ONE);
        assert_eq!(
            plate.deflection_km(&[minimum_pressure], Fixed::ZERO, Fixed::ZERO),
            Err(ReliefRefusal::LoadOutsideEnvelope),
            "the magnitude guard refuses Fixed::MIN without taking its unrepresentable absolute value"
        );

        // This pair is one half of a fixed-point step beyond the line-load envelope in real arithmetic.
        // Forming width times pressure in Q32.32 would truncate it back onto 500 and admit it, so the guard
        // compares the unshifted raw product instead.
        let half_step_past = uniform_strip_load(
            Fixed::from_int(1_000)
                .checked_add(Fixed::EPSILON)
                .expect("the pressure probe is representable"),
            Fixed::ZERO,
            Fixed::from_ratio(1, 4),
        );
        assert_eq!(
            plate.deflection_km(&[half_step_past], Fixed::ZERO, Fixed::ZERO),
            Err(ReliefRefusal::LoadOutsideEnvelope)
        );

        let sub_internal_width = uniform_strip_load(Fixed::ONE, Fixed::ZERO, Fixed::EPSILON);
        assert_eq!(
            plate.deflection_km(&[sub_internal_width], Fixed::ZERO, Fixed::ZERO),
            Err(ReliefRefusal::NotRepresentable),
            "a positive external footprint below one internal length step receives a typed refusal"
        );
    }
}
