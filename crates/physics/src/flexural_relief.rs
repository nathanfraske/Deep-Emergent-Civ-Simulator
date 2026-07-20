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
//! unit (see [`crate::moment_equivalence::MomentEquivalentPlate::rigidity_internal`]). Every entry point in
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
    kelvin_kei, line_load_admissible, point_load_admissible, scaled, Load, LoadKind,
};
use crate::moment_equivalence::MomentEquivalentPlate;

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
        Self::from_internal_rigidity(plate.rigidity_internal, delta_rho, gravity_km_s2)
    }

    /// Build from an internal rigidity directly, for a caller holding one from somewhere other than the
    /// moment-equivalence solve (a hindcast row converted inward, or a test's synthetic plate).
    // @derives: a loadable flexed plate <- an internal rigidity and the world's restoring term
    pub fn from_internal_rigidity(
        rigidity_internal: Fixed,
        delta_rho: Fixed,
        gravity_km_s2: Fixed,
    ) -> Result<Self, ReliefRefusal> {
        if rigidity_internal <= Fixed::ZERO {
            return Err(ReliefRefusal::RigidityNotPositive);
        }
        if delta_rho <= Fixed::ZERO || gravity_km_s2 <= Fixed::ZERO {
            return Err(ReliefRefusal::RestoringTermNotPositive);
        }
        let g_hat =
            scaled::internal_gravity(gravity_km_s2).ok_or(ReliefRefusal::NotRepresentable)?;
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

    /// The internal rigidity this plate was built from.
    pub fn rigidity_internal(&self) -> Fixed {
        self.rigidity_internal
    }

    /// THE DEFLECTION AT A QUERY POINT, in kilometres, summed over the whole load list.
    ///
    /// `qx_km` and `qy_km` are the query point and the load positions are the caller's own, in the same plane
    /// and the same kilometres. A downward deflection (a load pressing the plate into its substrate) is
    /// NEGATIVE, matching the Green's functions' own sign convention, so a caller adding this to an elevation
    /// gets a basin under the load and a forebulge beyond it without inverting anything.
    ///
    /// An empty list, or a list of zero-magnitude loads, gives zero rather than refusing: no load is a
    /// legitimate state and its relief is flat.
    ///
    /// THE SUM IS ORDER-INDEPENDENT because `Fixed` addition is exact and associative, which is the
    /// determinism contract (Principle 3) rather than a nicety: two runs that discover the same loads in
    /// different orders must produce the same world.
    // @derives: the flexural deflection at a point <- the plate's rigidity, its flexural lengths and the load list
    pub fn deflection_km(
        &self,
        loads: &[Load],
        qx_km: Fixed,
        qy_km: Fixed,
    ) -> Result<Fixed, ReliefRefusal> {
        let mut total_hat = Fixed::ZERO;
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
            };
            total_hat = total_hat
                .checked_add(contribution)
                .ok_or(ReliefRefusal::NotRepresentable)?;
        }
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

    /// One point load's contribution, in INTERNAL length.
    ///
    /// `w(r) = Q0 (l^2 / (2 pi D)) kei(r / l)`, on the AXISYMMETRIC length `l`, never on `alpha`. The `2 pi D`
    /// is formed internally because in the caller's units it overflowed for any `D` past `3.4e8`.
    // @derives: one point load's plate deflection <- the load magnitude, the axisymmetric length and the rigidity
    fn point_contribution_hat(&self, q0: Fixed, r_km: Fixed) -> Result<Fixed, ReliefRefusal> {
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
        coef_hat
            .checked_mul(kelvin_kei(arg))
            .ok_or(ReliefRefusal::NotRepresentable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            Fixed::from_ratio(8874728, 100),
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
        let d = f64_of(plate.rigidity_internal()) * 32768.0;
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
                Fixed::from_int(1000),
                Fixed::ZERO,
                mars_restoring().1
            ),
            Err(ReliefRefusal::RestoringTermNotPositive)
        );
        assert_eq!(
            FlexedPlate::from_internal_rigidity(
                Fixed::ZERO,
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
            Fixed::from_int(1000),
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
}
