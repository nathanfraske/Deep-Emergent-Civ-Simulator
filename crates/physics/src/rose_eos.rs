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

//! The Rose universal binding-energy equation of state (`rose_eos`): the metallic cohesive energy as a function
//! of volume, the Vinet/Rose EOS the materials disposer's `P.dV` term and Debye `theta_D` read.
//!
//! The Rose universal binding-energy relation (Rose, Smith, Guinea, Ferrante 1984, Phys. Rev. B 29, 2963) is the
//! single-parameter universal form `E(a*) = -E_coh (1 + a*) exp(-a*)`, where `a* = (r - r0)/l` is the scaled
//! deviation of the Wigner-Seitz radius from equilibrium and `l` is the Rose scaling length set by the bulk
//! modulus. Its three inputs are all D3-a or banked: the cohesive energy `E_coh` (kJ/mol, the banked atomization
//! enthalpy), the equilibrium molar volume `V0` (cm^3/mol), and the bulk modulus `B0` (GPa). At equilibrium
//! (`a*=0`) it returns `-E_coh` (the well depth); off equilibrium it is the metallic `E(V)` the disposer needs
//! for the `P.dV` term and, via the curvature, the sound speed feeding Debye `theta_D`. No consumer is wired to
//! it in any pinned run path yet (byte-neutral); the materials metallic route (D3-c) reads it.
//!
//! DERIVE-FIRST, ZERO AUTHORED. The scaling length derives from the cited anchors: the equilibrium Wigner-Seitz
//! radius `r0 = (3 V_atom / 4pi)^(1/3)` with `V_atom = V0 * (10^24 / N_A)` (the Avogadro exact-ratio cm^3/mol to
//! A^3/atom conversion, `N_A` the defined SI Avogadro constant), and `l = sqrt(E_c / (12 pi B0 r0))` with `E_c`
//! and `B0` carried in per-atom eV and eV/A^3 through the same cited unit conversions the ionic route uses (a
//! shared floor constant, not a copy). Every constant is a cited floor or law constant (Principle 11).
//!
//! VALIDATION (the unit-bug catcher, non-circular). Because `l` is DERIVED from `B0`, the analytic curvature
//! trivially recovers `B0`. The real check is that a NUMERICAL second derivative of the returned `E(V)` curve
//! recovers the cited `B0` (it tests the whole chain together: the Avogadro factor, the eV conversions, the shape
//! function, and the `a*` mapping), keyed on `1 kJ/cm^3 = 1 GPa` so `B0 = V0 d2E/dV2` in molar units reads GPa
//! directly, and that the derived `r0` and `l` land at physical magnitudes (Fe: `r0 ~ 1.41 A`, `l ~ 0.28 A`). A
//! unit bug moves those, so the tests fail loud.

use crate::lattice_modulus::{ev_to_kj_per_mol, gpa_per_ev_per_angstrom_cubed};
use civsim_core::Fixed;

/// The `cm^3/mol` to `A^3/atom` conversion `10^24 / N_A = 1.6605391 A^3.mol/cm^3` (CODATA: `N_A = 6.02214076e23`
/// per mol, the exact SI-defined Avogadro constant since 2019), as the exact rational `10^9 / 602214076`. The
/// per-atom volume the Wigner-Seitz radius needs, built by exact ratio rather than a decimal parse. A fundamental
/// floor constant (Principle 11).
fn cm3_per_mol_to_angstrom3_per_atom() -> Fixed {
    Fixed::from_ratio(1_000_000_000, 602_214_076)
}

/// The equilibrium Wigner-Seitz radius (Angstrom) of a metal from its molar volume (cm^3/mol): the radius of the
/// per-atom sphere, `r = (3 V_atom / 4pi)^(1/3)` with `V_atom` the per-atom volume. `None` on a non-positive
/// volume or an overflow.
pub fn wigner_seitz_radius_angstrom(molar_volume_cm3: Fixed) -> Option<Fixed> {
    if molar_volume_cm3 <= Fixed::ZERO {
        return None;
    }
    let v_atom = molar_volume_cm3.checked_mul(cm3_per_mol_to_angstrom3_per_atom())?;
    let numerator = v_atom.checked_mul(Fixed::from_int(3))?;
    let four_pi = Fixed::PI.checked_mul(Fixed::from_int(4))?;
    let radius_cubed = numerator.checked_div(four_pi)?;
    Some(radius_cubed.cbrt())
}

/// The Rose scaling length `l` (Angstrom) from the cohesive energy (kJ/mol), the bulk modulus (GPa), and the
/// equilibrium Wigner-Seitz radius (Angstrom): `l = sqrt(E_c / (12 pi B0 r0))`, with `E_c` and `B0` carried in
/// per-atom eV and eV/A^3. `None` on a non-positive input or an overflow.
pub fn rose_scaling_length_angstrom(
    e_coh_kj_per_mol: Fixed,
    b0_gpa: Fixed,
    r0_angstrom: Fixed,
) -> Option<Fixed> {
    if e_coh_kj_per_mol <= Fixed::ZERO || b0_gpa <= Fixed::ZERO || r0_angstrom <= Fixed::ZERO {
        return None;
    }
    let e_c_ev = e_coh_kj_per_mol.checked_div(ev_to_kj_per_mol())?;
    let b0_ev_per_a3 = b0_gpa.checked_div(gpa_per_ev_per_angstrom_cubed())?;
    let twelve_pi = Fixed::PI.checked_mul(Fixed::from_int(12))?;
    let denom = twelve_pi
        .checked_mul(b0_ev_per_a3)?
        .checked_mul(r0_angstrom)?;
    let l_squared = e_c_ev.checked_div(denom)?;
    Some(l_squared.sqrt())
}

/// The Rose universal binding energy (kJ/mol) at a Wigner-Seitz radius `r`, given the well depth `E_coh`
/// (kJ/mol), the equilibrium radius `r0`, and the scaling length `l`: `E(a*) = -E_coh (1 + a*) exp(-a*)` with
/// `a* = (r - r0)/l`. Negative (bound) near equilibrium, rising toward zero at large separation (dissociation).
/// `None` on a non-positive scaling length or an overflow.
pub fn binding_energy_kj_per_mol(
    e_coh_kj_per_mol: Fixed,
    r_angstrom: Fixed,
    r0_angstrom: Fixed,
    l_angstrom: Fixed,
) -> Option<Fixed> {
    if l_angstrom <= Fixed::ZERO {
        return None;
    }
    let a_star = (r_angstrom - r0_angstrom).checked_div(l_angstrom)?;
    let one_plus = Fixed::ONE + a_star;
    let decay = (Fixed::ZERO - a_star).exp();
    let shape = one_plus.checked_mul(decay)?;
    Some(Fixed::ZERO - e_coh_kj_per_mol.checked_mul(shape)?)
}

/// The metallic cohesive energy (kJ/mol) at a compressed or expanded molar volume, the full Rose EOS over the
/// D3-a anchors: derive the equilibrium radius and scaling length from `(E_coh, V0, B0)`, map the target volume
/// to its radius, and evaluate the universal binding curve. At `v_molar == v0_molar` it returns `-E_coh` (the
/// equilibrium well depth). `None` when an input is non-physical or a step overflows.
pub fn cohesive_energy_at_volume(
    e_coh_kj_per_mol: Fixed,
    v0_molar_cm3: Fixed,
    b0_gpa: Fixed,
    v_molar_cm3: Fixed,
) -> Option<Fixed> {
    let r0 = wigner_seitz_radius_angstrom(v0_molar_cm3)?;
    let r = wigner_seitz_radius_angstrom(v_molar_cm3)?;
    let l = rose_scaling_length_angstrom(e_coh_kj_per_mol, b0_gpa, r0)?;
    binding_energy_kj_per_mol(e_coh_kj_per_mol, r, r0, l)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fe's D3-a anchors and banked cohesive energy: B0 = 170 GPa, V0 = 7.09 cm^3/mol, E_coh = 416.3 kJ/mol.
    fn fe() -> (Fixed, Fixed, Fixed) {
        (
            Fixed::from_decimal_str("416.3").unwrap(), // E_coh kJ/mol
            Fixed::from_decimal_str("7.09").unwrap(),  // V0 cm^3/mol
            Fixed::from_int(170),                      // B0 GPa
        )
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_wigner_seitz_radius_lands_at_the_physical_scale() {
        // Fe's Wigner-Seitz radius is ~1.41 A (a hard magnitude check on the Avogadro factor and the cube root: a
        // unit bug in the cm^3/mol -> A^3 conversion moves this off the atomic scale).
        let (_, v0, _) = fe();
        let r0 = wigner_seitz_radius_angstrom(v0).expect("Fe r0");
        assert!(
            close(r0, 1.411, 0.01),
            "Fe Wigner-Seitz radius ~ 1.411 A, got {}",
            r0.to_f64_lossy()
        );
    }

    #[test]
    fn the_scaling_length_lands_at_the_physical_scale() {
        // Fe's Rose scaling length is ~0.28 A (a hard magnitude check on the eV and eV/A^3 conversions).
        let (e_coh, v0, b0) = fe();
        let r0 = wigner_seitz_radius_angstrom(v0).expect("Fe r0");
        let l = rose_scaling_length_angstrom(e_coh, b0, r0).expect("Fe l");
        assert!(
            close(l, 0.2765, 0.01),
            "Fe Rose scaling length ~ 0.2765 A, got {}",
            l.to_f64_lossy()
        );
    }

    #[test]
    fn the_well_bottom_is_the_negative_cohesive_energy() {
        // At the equilibrium volume the Rose EOS returns -E_coh exactly (a*=0, the depth of the binding well).
        let (e_coh, v0, b0) = fe();
        let e = cohesive_energy_at_volume(e_coh, v0, b0, v0).expect("Fe E(V0)");
        assert!(
            close(e, -416.3, 0.01),
            "Fe E(V0) = -E_coh = -416.3 kJ/mol, got {}",
            e.to_f64_lossy()
        );
    }

    #[test]
    fn the_numerical_curvature_recovers_the_bulk_modulus() {
        // THE UNIT-BUG CATCHER (the gate's validation, made non-circular by a NUMERICAL second derivative). The
        // bulk modulus is B0 = V0 d2E/dV2 at equilibrium; in molar units (E in kJ/mol, V in cm^3/mol) the identity
        // 1 kJ/cm^3 = 1 GPa makes V0 d2E/dV2 read GPa directly. A central second difference of the returned E(V)
        // curve must recover the cited 170 GPa, which tests the Avogadro factor, the eV conversions, the shape
        // function, and the a* mapping ALL together: any unit bug in the chain moves the recovered modulus.
        let (e_coh, v0, b0) = fe();
        let dv = v0.checked_div(Fixed::from_int(100)).unwrap(); // 1 percent step
        let e0 = cohesive_energy_at_volume(e_coh, v0, b0, v0).unwrap();
        let e_plus = cohesive_energy_at_volume(e_coh, v0, b0, v0 + dv).unwrap();
        let e_minus = cohesive_energy_at_volume(e_coh, v0, b0, v0 - dv).unwrap();
        // d2E/dV2 ~ (E+ - 2 E0 + E-) / dV^2, then B = V0 * d2E/dV2.
        let second_diff = e_plus - e0 - e0 + e_minus;
        let curvature = second_diff
            .checked_div(dv.checked_mul(dv).unwrap())
            .unwrap();
        let b_recovered = v0.checked_mul(curvature).unwrap();
        let recovered = b_recovered.to_f64_lossy();
        assert!(
            (recovered - 170.0).abs() / 170.0 < 0.05,
            "the numerical curvature must recover B0 = 170 GPa within 5 percent, got {recovered}"
        );
    }

    #[test]
    fn the_equilibrium_volume_is_the_energy_minimum() {
        // The well shape: compression (smaller V) and expansion (larger V) both raise the energy above the
        // equilibrium well bottom, and far expansion approaches dissociation (energy toward zero).
        let (e_coh, v0, b0) = fe();
        let e0 = cohesive_energy_at_volume(e_coh, v0, b0, v0).unwrap();
        let compressed = cohesive_energy_at_volume(
            e_coh,
            v0,
            b0,
            v0.checked_mul(Fixed::from_ratio(9, 10)).unwrap(),
        )
        .unwrap();
        let expanded = cohesive_energy_at_volume(
            e_coh,
            v0,
            b0,
            v0.checked_mul(Fixed::from_ratio(3, 2)).unwrap(),
        )
        .unwrap();
        assert!(
            compressed > e0,
            "compression raises the energy above the well bottom"
        );
        assert!(
            expanded > e0,
            "expansion raises the energy above the well bottom"
        );
        // Far expansion trends toward zero (dissociation), so it is well above the well bottom but still bound.
        let far =
            cohesive_energy_at_volume(e_coh, v0, b0, v0.checked_mul(Fixed::from_int(4)).unwrap())
                .unwrap();
        assert!(
            far.to_f64_lossy() > -100.0 && far.to_f64_lossy() < 0.0,
            "far expansion approaches dissociation (energy toward zero), got {}",
            far.to_f64_lossy()
        );
    }

    #[test]
    fn a_non_physical_input_is_rejected() {
        let (e_coh, v0, b0) = fe();
        assert!(wigner_seitz_radius_angstrom(Fixed::ZERO).is_none());
        assert!(rose_scaling_length_angstrom(e_coh, Fixed::ZERO, Fixed::ONE).is_none());
        assert!(cohesive_energy_at_volume(e_coh, Fixed::ZERO, b0, v0).is_none());
    }
}
