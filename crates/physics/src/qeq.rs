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
//! Ewald(q)` at fixed total charge (a linear system), reading these two quantities plus the Ewald Coulomb
//! kernel. This module carries the inputs; the solve grows onto it.
//!
//! Both are `None` when the element lacks a populated ionization energy or electron affinity (absent-not-zero):
//! an element whose anion is unbound (nitrogen, magnesium) carries no measured electron affinity, so its `chi`
//! and `eta` are a genuine data gap the QEq parameterization closes with a cited fitted value, never a
//! fabricated zero. Everything here is fixed-point and deterministic; nothing reads it yet, so the pins hold.

use crate::periodic::Element;
use civsim_core::fixed::Fixed;

/// The Mulliken electronegativity `chi = (IE + EA) / 2` of an element, in electron-volts, DERIVED from the
/// ionization-energy and electron-affinity columns. `None` when either input is absent (a data gap the QEq
/// parameterization closes, never a zero).
pub fn mulliken_electronegativity(element: &Element) -> Option<Fixed> {
    let ie = element.ionization_energy?;
    let ea = element.electron_affinity?;
    (ie + ea).checked_div(Fixed::from_int(2))
}

/// The chemical hardness `eta = (IE - EA) / 2` of an element, in electron-volts, DERIVED from the same two
/// columns (the free second combination). `None` when either input is absent.
pub fn chemical_hardness(element: &Element) -> Option<Fixed> {
    let ie = element.ionization_energy?;
    let ea = element.electron_affinity?;
    (ie - ea).checked_div(Fixed::from_int(2))
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
    fn an_unbound_anion_element_has_no_electronegativity_yet() {
        let t = table();
        // Magnesium and nitrogen have unbound anions, so no measured electron affinity: chi and eta are a data
        // gap (None), the QEq parameterization closes them with a cited fitted value, never a fabricated zero.
        for symbol in ["Mg", "N"] {
            let el = t.element(symbol).expect("element is on the table");
            assert!(
                el.ionization_energy.is_some(),
                "{symbol} carries its ionization energy"
            );
            assert!(
                mulliken_electronegativity(el).is_none(),
                "{symbol} has no electronegativity yet (electron affinity absent)"
            );
            assert!(
                chemical_hardness(el).is_none(),
                "{symbol} has no hardness yet"
            );
        }
    }
}
