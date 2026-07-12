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
