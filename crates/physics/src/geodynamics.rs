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

//! The geodynamics contract and the isostasy law (genesis-forward Stage 3), the SHARED surface-interior
//! boundary the two lanes meet at. This module is the typed contract both lanes import, so the surface
//! elevation-ledger lane (`civsim_sim`) and the interior convection lane (`civsim_physics` geology floor) stay
//! disjoint by file and couple only through the value here.
//!
//! [`GeodynamicColumn`] is the minimal per-column contract, deliberately small and ADDITIVE-EXTENSIBLE: the
//! surface lane writes the `crustal_density` it derives from the column's composition (the petrology kernel),
//! the interior lane writes the `isostatic_elevation` it derives through the buoyancy law and (its own input)
//! the `crustal_thickness`, and the surface isostatic relaxation reads that elevation. B's deeper interior
//! state (internal heat, convection velocity, plate membership, uplift rate) is an ADDITIVE extension of this
//! struct when that lane resumes, never a rewrite of the contract.
//!
//! [`airy_isostatic_elevation`] is the first-pass isostasy LAW (a stable flotation law): the elevation a
//! crustal column floats at, DERIVED from its density and thickness by Archimedes flotation on the mantle. It
//! is the surface lane's static seed-crust computation; the interior convection lane later refines the
//! `crustal_thickness` INPUT to the same law across the contract, so the static pass is exactly the seam the
//! dynamic lane refines, not work it discards. Fixed-point and deterministic (Principle 3); no consumer is
//! wired yet, a pure addition.

use civsim_core::Fixed;

/// The per-column GEODYNAMIC state, the minimal shared contract the interior and surface Stage-3 lanes meet at
/// (the disjoint-file boundary in `docs/working/GENESIS_STAGE3_LAYOUT.md`). Each lane writes its own fields and
/// reads the other's, a two-way producer-consumer boundary. A column with no geodynamic state reads the zero
/// default (the absence convention). The struct is deliberately minimal and ADDITIVE-EXTENSIBLE: the interior
/// lane's deeper state (internal heat, convection velocity, plate membership, uplift rate) extends it by adding
/// fields, never by reshaping the ones here.
///
/// The interior lane's additive extension (below `isostatic_elevation`) is deliberately CONTINUOUS: it carries
/// the interior thermal state, the convective driving stress, and the Rayleigh number, and it carries NO
/// stored discrete "convecting" flag (gate ruling, #176). A stored discrete state that selected behaviour would
/// be the closed-enum template the tectonic-regime work retired; instead the discrete condition (whether the
/// column convects) is DERIVED from the continuous Rayleigh number against the critical value wherever a
/// consumer needs it, so convection can begin and, on a cooling world, cease, and the regime NAME stays an
/// observer-side description. Each added field is `Default`-zero, so an unpopulated column stays byte-neutral.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeodynamicColumn {
    /// The petrology-derived crustal density (grams per cubic centimetre), written by the SURFACE lane and read
    /// by the interior isostasy. Zero is undetermined.
    pub crustal_density: Fixed,
    /// The crustal thickness (metres) the isostasy reads, the seed value in the static first pass and the
    /// interior convection lane's refined output later. Zero is undetermined.
    pub crustal_thickness: Fixed,
    /// The isostatic elevation (metres above the compensation reference) derived from the density and thickness
    /// through [`airy_isostatic_elevation`]; the surface relaxation reads it and relaxes the effective
    /// elevation toward it. Zero is undetermined.
    pub isostatic_elevation: Fixed,
    /// The interior thermal state (the column temperature in kelvin), the internal-heat quantity the convection
    /// evolution carries. Written by the INTERIOR lane. Zero is undetermined. Continuous (no derived discrete
    /// state is stored beside it).
    pub temperature: Fixed,
    /// The convective driving stress (pascals) the interior flow exerts on the base of the lithosphere
    /// ([`crate::laws::convective_stress`]), the continuous quantity lid mobilization emerges from against
    /// `mat.yield_strength`. Written by the INTERIOR lane. Zero is undetermined.
    pub convective_stress: Fixed,
    /// The Rayleigh number (dimensionless), the continuous convective-vigor ratio a consumer reads to derive
    /// whether the column convects (against the critical value) rather than reading a stored flag. Written by
    /// the INTERIOR lane. Zero is undetermined.
    pub rayleigh: Fixed,
}

/// The AIRY (Archimedes) isostatic elevation a crustal column floats at, in metres above the compensation
/// reference, DERIVED from the column's density and thickness. A crustal column of density `rho_c` and
/// thickness `T` floating on a mantle of density `rho_m` sits with the submerged fraction equal to the density
/// ratio `rho_c / rho_m` (Archimedes), so its top stands above the reference by
///
/// `elevation = T * (rho_m - rho_c) / rho_m`.
///
/// A lighter crust (`rho_c < rho_m`, a felsic continent) floats HIGHER; a crust at the mantle density floats
/// flush (zero); a denser-than-mantle column FOUNDERS (a negative elevation, which the isostasy passes through
/// rather than clamping, since a dense delamination sinking is the physical outcome). This is the surface
/// lane's static seed-crust law; the interior convection lane refines the thickness input later across the
/// [`GeodynamicColumn`] contract, so the two never collide (the flotation law is stable, the thickness is the
/// refined input). Fails loud (returns `None`) on a non-positive mantle density or a fixed-point overflow,
/// never a fabricated elevation. Deterministic fixed-point arithmetic (Principle 3).
pub fn airy_isostatic_elevation(
    crustal_density: Fixed,
    mantle_density: Fixed,
    crustal_thickness: Fixed,
) -> Option<Fixed> {
    if mantle_density <= Fixed::ZERO {
        return None;
    }
    // The buoyant fraction standing above the reference: (rho_m - rho_c) / rho_m. Positive when the crust is
    // lighter than the mantle, negative when it is denser (it founders).
    let contrast = mantle_density - crustal_density;
    let fraction = contrast.checked_div(mantle_density)?;
    crustal_thickness.checked_mul(fraction)
}

/// A CRUSTAL COLUMN'S FLEXURAL LOAD: its buoyancy anomaly, as a pressure over its own footprint.
///
/// # WHY THIS EXISTS, AND WHAT IT IS THE BRIDGE BETWEEN
///
/// [`airy_isostatic_elevation`] reads ONE column's own properties and answers for that column alone. It has no
/// lateral term at all, so under it every column floats independently and neighbours say nothing to each other:
/// a load can only ever push its own column down, relief is exactly as rough as the field of thicknesses that
/// produced it, and a boundary between two crustal types is a STEP. That is a faithful implementation of Airy
/// isostasy and it is also why rendered relief reads as a field of blocks.
///
/// A plate BENDS, and bending is lateral. To ask the flexure substrate for an elevation the column has to become
/// a LOAD with a position and a footprint, which is what this returns. The flexural answer is then the
/// superposition over every column's load ([`crate::flexural_relief::FlexedPlate::deflection_km`]), and it
/// carries the neighbourhood the Airy law cannot see.
///
/// # THE LOAD IS THE BUOYANCY ANOMALY, AND THAT IS WHAT MAKES THE TWO LAWS ONE LAW
///
/// `p = (rho_m - rho_c) g T`, the same contrast [`airy_isostatic_elevation`] takes and the same thickness,
/// expressed as a pressure. Against a restoring contrast of `rho_m` the far-field limit of a wide strip is
/// `p / (rho_m g) = T (rho_m - rho_c) / rho_m`, which IS the Airy elevation. So flexure GENERALIZES the
/// flotation law rather than competing with it: Airy is its `D -> 0` limit, verified numerically in
/// `crates/physics/src/flexural_relief.rs` down to a residual of `3.26e-9 km`.
///
/// A denser-than-mantle column returns a NEGATIVE pressure and founders, which the flexure kernel carries
/// through as a downward deflection, the same passthrough the Airy law makes rather than clamping.
///
/// # UNITS
///
/// The flexure substrate's own coherent system: `crustal_density` and `mantle_density` in `1000 kg/m^3`,
/// `crustal_thickness_km` and `half_width_km` in kilometres, `gravity_km_s2` in `km/s^2`. The product then
/// lands in gigapascals exactly (`1e3 kg/m^3 * 1e3 m/s^2 * 1e3 m = 1e9 Pa`), so no conversion factor appears
/// here and none is authored. NOTE that this differs from [`airy_isostatic_elevation`], which is scale-free in
/// its thickness and so is often called with metres; a column fed to both must be converted once by the caller.
///
/// `half_width_km` is the column's own footprint and is CALLER DATA, never a value this module declares: a
/// province is as wide as the world made it. `None` on a non-positive mantle density, gravity or half-width, or
/// on a fixed-point overflow, never a fabricated load.
// @derives: a crustal column's flexural load <- its density contrast against the mantle, its thickness, the surface gravity and its own footprint
pub fn column_buoyancy_load(
    crustal_density: Fixed,
    mantle_density: Fixed,
    crustal_thickness_km: Fixed,
    gravity_km_s2: Fixed,
    centre_km: Fixed,
    half_width_km: Fixed,
) -> Option<crate::flexure::Load> {
    if mantle_density <= Fixed::ZERO || gravity_km_s2 <= Fixed::ZERO || half_width_km <= Fixed::ZERO
    {
        return None;
    }
    let contrast = mantle_density.checked_sub(crustal_density)?;
    let pressure = contrast
        .checked_mul(crustal_thickness_km)?
        .checked_mul(gravity_km_s2)?;
    Some(crate::flexure::Load {
        kind: crate::flexure::LoadKind::UniformStripY {
            half_width: half_width_km,
        },
        magnitude: pressure,
        x: centre_km,
        y: Fixed::ZERO,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn a_lighter_crust_floats_higher_than_a_denser_one() {
        // Continents (felsic, light) stand higher than ocean floor (mafic, dense) from their density alone: the
        // isostatic elevation derives from the flotation balance, never an authored per-crust height.
        let mantle = Fixed::from_ratio(33, 10); // 3.3 g/cm^3
        let thickness = Fixed::from_int(35_000); // 35 km of crust, in metres
        let felsic = airy_isostatic_elevation(Fixed::from_ratio(265, 100), mantle, thickness)
            .expect("a felsic column floats");
        let mafic = airy_isostatic_elevation(Fixed::from_ratio(300, 100), mantle, thickness)
            .expect("a mafic column floats");
        // felsic: 35000 * (3.3 - 2.65)/3.3 = 35000 * 0.19697 = 6893.9 m; mafic: 35000 * (3.3-3.0)/3.3 = 3181.8 m.
        assert!(
            close(felsic, 6893.9, 1.0),
            "felsic elevation, got {}",
            felsic.to_f64_lossy()
        );
        assert!(
            close(mafic, 3181.8, 1.0),
            "mafic elevation, got {}",
            mafic.to_f64_lossy()
        );
        assert!(felsic > mafic, "the lighter crust floats higher");
    }

    #[test]
    fn a_crust_at_mantle_density_floats_flush_and_a_denser_one_founders() {
        let mantle = Fixed::from_ratio(33, 10);
        let thickness = Fixed::from_int(30_000);
        // A column at the mantle density has no buoyancy contrast: it sits flush at the reference (zero).
        let flush = airy_isostatic_elevation(mantle, mantle, thickness).expect("flush column");
        assert!(
            close(flush, 0.0, 0.01),
            "a mantle-density column floats flush"
        );
        // A denser-than-mantle column founders: a negative elevation, passed through rather than clamped.
        let dense = airy_isostatic_elevation(Fixed::from_ratio(36, 10), mantle, thickness)
            .expect("a dense column still resolves");
        assert!(
            dense.to_f64_lossy() < 0.0,
            "a denser-than-mantle column founders (negative elevation), got {}",
            dense.to_f64_lossy()
        );
    }

    #[test]
    fn the_isostasy_fails_loud_on_a_non_positive_mantle_density() {
        // A zero or negative mantle density has no flotation reference: refused fail-loud rather than dividing
        // by zero or fabricating an elevation.
        assert!(
            airy_isostatic_elevation(Fixed::from_int(3), Fixed::ZERO, Fixed::from_int(30_000))
                .is_none()
        );
    }

    #[test]
    fn the_geodynamic_column_default_is_all_zero_the_absence_convention() {
        let c = GeodynamicColumn::default();
        assert_eq!(c.crustal_density, Fixed::ZERO);
        assert_eq!(c.crustal_thickness, Fixed::ZERO);
        assert_eq!(c.isostatic_elevation, Fixed::ZERO);
    }
}

#[cfg(test)]
mod flexural_bridge_tests {
    use super::*;
    use crate::flexural_relief::FlexedPlate;
    use crate::flexure::scaled;

    fn f64_of(x: Fixed) -> f64 {
        x.to_f64_lossy()
    }

    /// An Earth-like plate: `E = 70 GPa`, `nu = 0.25`, a 40 km elastic thickness, floating on a 3.3 mantle at
    /// `9.8 m/s^2`. Every input is the flexure module's own declared fixture; nothing here is fitted.
    fn earthlike_plate() -> FlexedPlate {
        let d = crate::flexure::flexural_rigidity(
            Fixed::from_int(70),
            Fixed::from_ratio(1, 4),
            Fixed::from_int(40),
        )
        .expect("the fixture plate has a rigidity");
        let d_internal = scaled::internal_rigidity(d).expect("the rigidity converts inward");
        FlexedPlate::from_internal_rigidity(
            d_internal,
            Fixed::from_ratio(33, 10),
            Fixed::from_ratio(98, 10_000),
        )
        .expect("the fixture plate loads")
    }

    #[test]
    fn a_crustal_boundary_is_a_step_under_airy_and_a_ramp_under_flexure() {
        // THE WHOLE POINT OF THE BRIDGE, stated as the one measurement that separates the two laws.
        //
        // Two provinces meet at x = 0: felsic continent (2.65) to the left, mafic ocean (3.0) to the right, both
        // 35 km thick. Under AIRY each column answers from its own density alone, so the elevation is one value
        // to the left of the boundary and another to the right with NOTHING in between: a discontinuity at a
        // single coordinate, no matter how finely the surface is sampled. That is the block-edged relief the
        // flotation law produces by construction, and no amount of display smoothing makes it terrain.
        //
        // Under FLEXURE the plate carries bending stress across the boundary, so the transition is a RAMP whose
        // width is the plate's own flexural parameter. Nothing selects that width: it falls out of the rigidity
        // and the restoring term.
        let rho_m = Fixed::from_ratio(33, 10);
        let felsic = Fixed::from_ratio(265, 100);
        let mafic = Fixed::from_int(3);
        let thickness_km = Fixed::from_int(35);
        let gravity = Fixed::from_ratio(98, 10_000);
        let half_width = Fixed::from_int(400);

        // AIRY: two numbers, and the step between them.
        let airy_felsic = airy_isostatic_elevation(felsic, rho_m, thickness_km).expect("felsic");
        let airy_mafic = airy_isostatic_elevation(mafic, rho_m, thickness_km).expect("mafic");
        assert!(
            airy_felsic > airy_mafic,
            "the lighter province floats higher: {} against {}",
            f64_of(airy_felsic),
            f64_of(airy_mafic)
        );

        let plate = earthlike_plate();
        let loads = [
            column_buoyancy_load(
                felsic,
                rho_m,
                thickness_km,
                gravity,
                Fixed::from_int(-400),
                half_width,
            )
            .expect("the felsic province loads"),
            column_buoyancy_load(
                mafic,
                rho_m,
                thickness_km,
                gravity,
                Fixed::from_int(400),
                half_width,
            )
            .expect("the mafic province loads"),
        ];
        let at = |x: i32| {
            f64_of(
                plate
                    .deflection_km(&loads, Fixed::from_int(x), Fixed::ZERO)
                    .expect("the boundary evaluates"),
            )
        };

        // THE TRANSITION IS MONOTONE AND SPREAD, sampled across the boundary rather than at it. A step would
        // give the same value at every sample on one side; a ramp gives a different value at each.
        let samples: Vec<f64> = (-4..=4).map(|i| at(i * 50)).collect();
        for pair in samples.windows(2) {
            assert!(
                pair[0] > pair[1],
                "the flexural transition falls monotonically across the boundary: {:?}",
                samples
            );
        }
        // AND IT IS A REAL SPREAD rather than a numerically smeared step: the change across the sampled window
        // is a substantial fraction of the total contrast between the two provinces' far fields.
        let far_left = at(-390);
        let far_right = at(390);
        let contrast = far_left - far_right;
        let across_window = samples[0] - samples[samples.len() - 1];
        assert!(
            across_window > contrast * 0.1,
            "the boundary transition is spread over the flexural parameter, not concentrated at x = 0: \
             {across_window} of a {contrast} contrast"
        );
        eprintln!(
            "boundary: alpha={:.1} km, far L={:.6} km, far R={:.6} km, across +-200 km={:.6} km",
            f64_of(plate.flexural_parameter_km().expect("alpha")),
            far_left,
            far_right,
            across_window
        );
    }

    #[test]
    fn a_denser_than_mantle_column_founders_rather_than_being_clamped() {
        // The Airy law passes a negative elevation through rather than clamping, because a dense delamination
        // sinking is the physical outcome. The load bridge must agree: a denser-than-mantle column produces a
        // NEGATIVE pressure and the plate carries it down.
        let rho_m = Fixed::from_ratio(33, 10);
        let dense = Fixed::from_ratio(35, 10);
        let load = column_buoyancy_load(
            dense,
            rho_m,
            Fixed::from_int(35),
            Fixed::from_ratio(98, 10_000),
            Fixed::ZERO,
            Fixed::from_int(400),
        )
        .expect("a foundering column is still a load");
        assert!(
            load.magnitude < Fixed::ZERO,
            "a denser-than-mantle column loads the plate downward, got {}",
            f64_of(load.magnitude)
        );
        let w = earthlike_plate()
            .deflection_km(&[load], Fixed::ZERO, Fixed::ZERO)
            .expect("it evaluates");
        assert!(
            w < Fixed::ZERO,
            "and it founders rather than being clamped at zero, got {}",
            f64_of(w)
        );
    }

    #[test]
    fn the_load_bridge_refuses_rather_than_fabricating() {
        let g = Fixed::from_ratio(98, 10_000);
        let rho_m = Fixed::from_ratio(33, 10);
        let t = Fixed::from_int(35);
        assert!(
            column_buoyancy_load(
                Fixed::from_int(3),
                Fixed::ZERO,
                t,
                g,
                Fixed::ZERO,
                Fixed::from_int(400)
            )
            .is_none(),
            "a zero mantle density is not a mantle"
        );
        assert!(
            column_buoyancy_load(
                Fixed::from_int(3),
                rho_m,
                t,
                Fixed::ZERO,
                Fixed::ZERO,
                Fixed::from_int(400)
            )
            .is_none(),
            "a world with no gravity has no isostasy"
        );
        assert!(
            column_buoyancy_load(Fixed::from_int(3), rho_m, t, g, Fixed::ZERO, Fixed::ZERO)
                .is_none(),
            "a column with no footprint is not a distributed load"
        );
    }
}
