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

//! The CHARGE-EQUILIBRATION inputs (materials oracle generator architecture, #182): the two per-element
//! quantities the QEq solve minimizes over, both DERIVED from the ionization-energy and electron-affinity
//! columns already on the periodic table. This is the piece that dissolves the divalent-oxide overestimate at
//! its root: rather than an authored correction factor, the phase's PARTIAL charges come out of a linear solve
//! whose per-element coefficients are these two derived quantities, so an oxide's Mg lands below the formal
//! `+2` from first principles.
//!
//! - The MULLIKEN ELECTRONEGATIVITY `chi = (IE + EA) / 2`, the escaping tendency of an electron from the
//!   neutral atom, DERIVED `[D]` (Mulliken 1934).
//! - The CHEMICAL HARDNESS `eta = (IE - EA) / 2`, the resistance to a change in charge, the second derivative
//!   of the energy with respect to electron count, DERIVED `[D]` and FREE: it is the other linear combination
//!   of the same two measured columns (Parr and Pearson 1983), so it costs no new floor datum.
//!
//! The charge-equilibration solve (the next piece) minimizes `E(q) = sum_i (chi_i q_i + 0.5 eta_i q_i^2) +
//! E_Coulomb(q)` at fixed total charge (a linear system), reading these two quantities plus the periodic
//! Coulomb interaction. This module carries the inputs; the solve grows onto it.
//!
//! STABILITY SEAM (surfaced, #182): the Coulomb term must be the SHIELDED Coulomb (the Rappe-Goddard integral
//! over the atomic Slater densities), not the bare point-charge Ewald. With bare point charges the interaction
//! matrix is not positive-definite for a strong ionic, so the solve runs away rather than settling on a partial
//! charge. For periclase (MgO) the `q^2` coefficient of the symmetric charge-transfer mode is
//! `0.5(eta_Mg + eta_O) + W`, with `W` the per-formula-unit Madelung energy for unit charges
//! (`-M k / r_nn = -1.7476 * 14.39964 / 2.12 ~ -11.87 eV`): `0.5(3.823 + 6.0785) - 11.87 = -6.92 < 0`, so the
//! point-charge energy is unbounded below and no partial charge exists. The shielding lowers the near-field
//! Coulomb toward the on-site hardness at short range (`J_ij -> eta at r -> 0`, `-> k/r` at large `r`), restoring
//! positive-definiteness and giving the physical partial charge. The shielded form needs a per-element Slater
//! exponent (the orbital size), a further floor input the QEq piece surfaces.
//!
//! Both are `None` when the element lacks a populated ionization energy or electron affinity (absent-not-zero):
//! an element whose anion is unbound (nitrogen, magnesium) carries no measured electron affinity, so its `chi`
//! and `eta` are a genuine data gap the QEq parameterization closes with a cited fitted value, never a
//! fabricated zero. Everything here is fixed-point and deterministic; nothing reads it yet, so the pins hold.

use crate::periodic::Element;
use civsim_core::fixed::Fixed;

/// How an element's electronegativity and hardness were obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChiEtaProvenance {
    /// `[D]`: both from measured columns, `chi = (IE+EA)/2`, `eta = (IE-EA)/2` (a bound anion).
    Derived,
    /// `[E]`: the UNBOUND-ANION LIMIT, `chi = eta = IE/2` (the anion does not bind, so the electron affinity is
    /// physically `EA <= 0` and the limiting Mulliken value follows from the measured ionization energy alone).
    UnboundLimit,
}

/// An element's Mulliken electronegativity and chemical hardness, in electron-volts, with their provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChiEta {
    /// The Mulliken electronegativity `chi`, the escaping tendency of an electron.
    pub chi: Fixed,
    /// The chemical hardness `eta`, the resistance to a change in charge.
    pub eta: Fixed,
    /// Whether both came from measured columns or the unbound-anion limit.
    pub provenance: ChiEtaProvenance,
}

/// The Mulliken electronegativity and chemical hardness of an element, in electron-volts. When the electron
/// affinity is populated (a bound anion), `chi = (IE+EA)/2` and `eta = (IE-EA)/2`, DERIVED `[D]` (Mulliken 1934;
/// Parr and Pearson 1983). When the electron affinity is absent (an unbound anion: nitrogen, magnesium), the
/// UNBOUND-ANION LIMIT applies, `chi = eta = IE/2`, the correct `EA -> 0` limit from the measured ionization
/// energy alone (the gate's ruling, #182: not a fabricated affinity and not a fit, the physical limit tagged
/// `[E]`). `None` only when the ionization energy itself is absent (a genuine data gap, no derivation possible).
pub fn element_chi_eta(element: &Element) -> Option<ChiEta> {
    let ie = element.ionization_energy?;
    let two = Fixed::from_int(2);
    match element.electron_affinity {
        Some(ea) => Some(ChiEta {
            chi: (ie + ea).checked_div(two)?,
            eta: (ie - ea).checked_div(two)?,
            provenance: ChiEtaProvenance::Derived,
        }),
        None => {
            let half = ie.checked_div(two)?;
            Some(ChiEta {
                chi: half,
                eta: half,
                provenance: ChiEtaProvenance::UnboundLimit,
            })
        }
    }
}

/// The Mulliken electronegativity `chi` of an element in electron-volts (the unbound-anion limit applies when
/// the electron affinity is absent; see [`element_chi_eta`]). `None` when the ionization energy is absent.
pub fn mulliken_electronegativity(element: &Element) -> Option<Fixed> {
    element_chi_eta(element).map(|ce| ce.chi)
}

/// The chemical hardness `eta` of an element in electron-volts (the unbound-anion limit applies when the
/// electron affinity is absent; see [`element_chi_eta`]). `None` when the ionization energy is absent.
pub fn chemical_hardness(element: &Element) -> Option<Fixed> {
    element_chi_eta(element).map(|ce| ce.eta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::periodic::PeriodicTable;

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the periodic table loads")
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn oxygen_electronegativity_and_hardness_match_the_hand_calc() {
        let t = table();
        let o = t.element("O").expect("oxygen is on the table");
        // IE = 13.618, EA = 1.461. chi = (13.618 + 1.461)/2 = 7.5395, eta = (13.618 - 1.461)/2 = 6.0785.
        let chi = mulliken_electronegativity(o).expect("O chi derives");
        let eta = chemical_hardness(o).expect("O eta derives");
        assert!(
            close(chi, 7.5395, 1e-3),
            "O chi should be 7.5395, got {}",
            chi.to_f64_lossy()
        );
        assert!(
            close(eta, 6.0785, 1e-3),
            "O eta should be 6.0785, got {}",
            eta.to_f64_lossy()
        );
    }

    #[test]
    fn sodium_is_far_less_electronegative_than_oxygen() {
        let t = table();
        let na = t.element("Na").expect("sodium is on the table");
        let o = t.element("O").expect("oxygen is on the table");
        // Na chi = (5.139 + 0.548)/2 = 2.8435, well below oxygen's 7.5395: the electron flows Na -> O, the
        // physical basis of the partial charge the QEq solve produces.
        let chi_na = mulliken_electronegativity(na).expect("Na chi derives");
        let chi_o = mulliken_electronegativity(o).expect("O chi derives");
        assert!(
            close(chi_na, 2.8435, 1e-3),
            "Na chi should be 2.8435, got {}",
            chi_na.to_f64_lossy()
        );
        assert!(chi_na < chi_o, "sodium is less electronegative than oxygen");
    }

    #[test]
    fn an_unbound_anion_element_takes_the_ie_over_two_limit() {
        let t = table();
        // Magnesium's anion is unbound (no measured EA), so the unbound-anion limit applies: chi = eta = IE/2 =
        // 7.646/2 = 3.823, tagged the estimator limit (the gate's ruling: the physical EA -> 0 limit from the
        // measured IE, not a fabricated affinity and not a fit).
        let mg = t.element("Mg").expect("magnesium is on the table");
        let ce = element_chi_eta(mg).expect("Mg takes the unbound limit");
        assert_eq!(ce.provenance, ChiEtaProvenance::UnboundLimit);
        assert!(
            close(ce.chi, 3.823, 1e-3),
            "Mg chi should be IE/2 = 3.823, got {}",
            ce.chi.to_f64_lossy()
        );
        assert!(
            close(ce.eta, 3.823, 1e-3),
            "Mg eta should be IE/2 = 3.823, got {}",
            ce.eta.to_f64_lossy()
        );
        // And the direction that reduces the point-charge modulus: Mg is less electronegative than O (3.823 <
        // 7.5395), so charge flows Mg -> O and Mg comes out partial, below the formal +2.
        let o = t.element("O").expect("oxygen is on the table");
        assert!(
            ce.chi < element_chi_eta(o).unwrap().chi,
            "the unbound-limit Mg is less electronegative than O, so charge flows Mg -> O"
        );
    }

    #[test]
    fn a_bound_anion_element_is_tagged_derived() {
        let t = table();
        let o = t.element("O").expect("oxygen is on the table");
        assert_eq!(
            element_chi_eta(o).unwrap().provenance,
            ChiEtaProvenance::Derived,
            "oxygen has a measured electron affinity, so chi/eta are derived"
        );
    }
}
