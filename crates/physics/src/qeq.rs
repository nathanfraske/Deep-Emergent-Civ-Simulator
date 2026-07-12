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
//! The charge-equilibration solve ([`qeq_charges`]) minimizes `E = sum_i chi_i q_i + 0.5 sum_ij Gamma_ij q_i
//! q_j` at fixed total charge (a linear system), reading `chi`/`eta` plus the SHIELDED periodic Coulomb. The
//! bare point-charge Ewald is unstable for a strong ionic: its interaction matrix is not positive-definite, so
//! the solve runs away rather than settling on a partial charge (for periclase the symmetric-mode `q^2`
//! coefficient `0.5(eta_Mg + eta_O) + W = 0.5(3.823 + 6.0785) - 11.87 = -6.92 < 0`, with `W` the per-formula-unit
//! Madelung energy for unit charges). The fix is the SCC-DFTB shielded Coulomb (Elstner et al. 1998): the
//! shielding decay `tau = (16/5) U` is FIXED by the hardness through the exact 1s-density relation
//! `U = (5/16) tau` (owner ruling (c), #182: parameter-free, no fitted constant, no new column, `tau` derived
//! from the `eta` already on the floor), and `gamma_ij(R) -> U` at `R -> 0`, `-> 1/R` at large `R`, restoring
//! positive-definiteness.
//!
//! HONEST LIMIT, surfaced (#182): the DERIVE-FIRST parameters (raw-Mulliken `chi`/`eta` plus the 1s-model
//! `tau`) OVER-IONIZE a strong ionic. Periclase Mg lands slightly above the formal `+2` (not the
//! fitted-potential `~+1.6`), because at the interionic distance (`~2.12 A`, much larger than the density size
//! `~1/tau ~ 0.5 A`) the shielding is weak, so the Madelung nearly fully ionizes the pair, checked only by the
//! on-site hardness. This is the known behaviour of QEq on RAW atomic parameters (it is why EEM/QEq are usually
//! FITTED). The fitted `~+1.6` needs either a `[C]` closure parameterization (which the register minimizes) or
//! a compute-once DFT-charge tier; the cited SCF Slater exponent does NOT help (a larger `tau` shields LESS, so
//! the charge rises). So the shielded solve is built and correct (stable, symmetric, neutral, bounded, where
//! the bare Ewald runs away), and whether it corrects the modulus overestimate for a strong ionic is the open
//! question surfaced to the gate, not asserted here.
//!
//! `chi`/`eta` are `None` when the ionization energy is absent (a genuine data gap). Everything here is
//! fixed-point and deterministic; nothing reads it yet, so the pins hold.

use crate::ewald::{ewald_potential_matrix, Cell, N_REAL_SHELLS};
use crate::periodic::Element;
use civsim_core::fixed::Fixed;

/// The Coulomb energy `e^2 / (4 pi eps0)` at unit separation, `14.39964 eV.A` (CODATA), as an exact rational.
/// The bridge from the reduced Ewald units (1/Angstrom) to electron-volts, and the constant that folds the
/// atomic-unit `(16/5)` relation into this eV/Angstrom system.
fn k_ev_angstrom() -> Fixed {
    Fixed::from_ratio(1_439_964, 100_000)
}

/// The Slater decay constant `tau` in inverse angstroms from the chemical hardness, `tau = (16/5) U` with
/// `U = 2 eta` (owner ruling (c), #182): the shielding decay is FIXED by the hardness through the exact
/// 1s-density Coulomb self-energy relation `U = (5/16) tau`, no fitted constant and no new column. The
/// atomic-unit `(16/5)` is expressed directly in the eV/Angstrom system: `gamma(0) = (5/16) tau` in `1/A`, and
/// `k * gamma(0) = 2 eta = U`, so `tau[1/A] = 32 eta[eV] / (5 k)`. This is `[D-in-form]`: an exact relation on
/// a model (1s) density, cruder than an SCF Slater exponent, the honest trade for zero new floor.
fn tau_from_eta(eta: Fixed) -> Fixed {
    Fixed::from_int(32) * eta / (Fixed::from_int(5) * k_ev_angstrom())
}

/// One term of the unequal-exponent SCC-DFTB shielding, `Y(a, b, r)` (Elstner et al. 1998).
fn shield_y(a: Fixed, b: Fixed, r: Fixed) -> Fixed {
    let a2 = a * a;
    let b2 = b * b;
    let d = a2 - b2;
    let two = Fixed::from_int(2);
    let three = Fixed::from_int(3);
    b2 * b2 * a / (two * d * d) - (b2 * b2 * b2 - three * b2 * b2 * a2) / (d * d * d * r)
}

/// The SHORT-RANGE SHIELDING `S(r) = 1/r - gamma(r)` in inverse angstroms, the exponential correction that
/// turns the bare `1/r` Coulomb into the SCC-DFTB shielded `gamma` (which saturates to the hardness at
/// `r -> 0`). The equal-exponent branch (same element) and the unequal branch (Elstner et al. 1998); `r > 0`.
fn shielding(r: Fixed, tau_a: Fixed, tau_b: Fixed) -> Fixed {
    let close_exponents = (tau_a - tau_b).abs() < Fixed::from_ratio(1, 1000);
    if close_exponents {
        let tau = (tau_a + tau_b) / Fixed::from_int(2);
        let poly = Fixed::ONE / r
            + Fixed::from_ratio(11, 16) * tau
            + Fixed::from_ratio(3, 16) * tau * tau * r
            + tau * tau * tau * r * r / Fixed::from_int(48);
        (Fixed::ZERO - tau * r).exp() * poly
    } else {
        (Fixed::ZERO - tau_a * r).exp() * shield_y(tau_a, tau_b, r)
            + (Fixed::ZERO - tau_b * r).exp() * shield_y(tau_b, tau_a, r)
    }
}

/// Solve the linear system `m x = b` by Gaussian elimination with partial pivoting, deterministic in
/// fixed-point (the pivot is the first row of maximum absolute value in the column, a fixed tie-break).
/// Returns `None` if the matrix is singular (a zero pivot).
fn solve_linear(mut m: Vec<Vec<Fixed>>, mut b: Vec<Fixed>) -> Option<Vec<Fixed>> {
    let n = b.len();
    for col in 0..n {
        let mut pivot = col;
        for r in (col + 1)..n {
            if m[r][col].abs() > m[pivot][col].abs() {
                pivot = r;
            }
        }
        if m[pivot][col] == Fixed::ZERO {
            return None;
        }
        m.swap(col, pivot);
        b.swap(col, pivot);
        let pivot_row = m[col].clone();
        let pivot_b = b[col];
        let diag = pivot_row[col];
        for r in (col + 1)..n {
            let factor = m[r][col] / diag;
            for c in col..n {
                m[r][c] -= factor * pivot_row[c];
            }
            b[r] -= factor * pivot_b;
        }
    }
    let mut x = vec![Fixed::ZERO; n];
    for row in (0..n).rev() {
        let mut s = b[row];
        for c in (row + 1)..n {
            s -= m[row][c] * x[c];
        }
        x[row] = s / m[row][row];
    }
    Some(x)
}

/// The CHARGE-EQUILIBRATION partial charges of a periodic cell, in units of the elementary charge, by the
/// shielded SCC-DFTB QEq solve (owner architecture, #182). Minimizes `E = sum_i chi_i q_i + 0.5 sum_ij
/// Gamma_ij q_i q_j` at neutrality, where `Gamma` is the SHIELDED periodic Coulomb (the bare point-charge
/// Ewald is unstable for strong ionics; the shielding restores positive-definiteness). The off-diagonal
/// `Gamma_ij = k (A_ij - sum_L S)` (the periodic Ewald potential minus the short-range shielding over images),
/// the diagonal `Gamma_ii = k (A_ii - sum_{L!=0} S) + 2 eta_i` (the periodic self plus the on-site hardness).
/// `chi` and `eta` are the per-ion electronegativity and hardness in eV; the ion charges in `cell` are ignored
/// (this solves for them). Returns `None` for a degenerate cell or a singular system.
pub fn qeq_charges(cell: &Cell, chi: &[Fixed], eta: &[Fixed]) -> Option<Vec<Fixed>> {
    let n = cell.ions.len();
    if chi.len() != n || eta.len() != n {
        return None;
    }
    let a = ewald_potential_matrix(cell)?;
    let tau: Vec<Fixed> = eta.iter().map(|&e| tau_from_eta(e)).collect();

    // The short-range shielding summed over lattice images, S_sum[i][j] = sum_L S(|r_ij + L|) (all L for
    // i != j; L != 0 for i == j). Cartesian ion positions and the lattice come from the cell.
    let l = &cell.lattice;
    let cart: Vec<[Fixed; 3]> = cell
        .ions
        .iter()
        .map(|ion| {
            [
                l[0][0] * ion.frac[0] + l[1][0] * ion.frac[1] + l[2][0] * ion.frac[2],
                l[0][1] * ion.frac[0] + l[1][1] * ion.frac[1] + l[2][1] * ion.frac[2],
                l[0][2] * ion.frac[0] + l[1][2] * ion.frac[1] + l[2][2] * ion.frac[2],
            ]
        })
        .collect();
    let shells = N_REAL_SHELLS;
    let mut s_sum = vec![vec![Fixed::ZERO; n]; n];
    for n1 in -shells..=shells {
        for n2 in -shells..=shells {
            for n3 in -shells..=shells {
                let lat = [
                    l[0][0] * Fixed::from_int(n1)
                        + l[1][0] * Fixed::from_int(n2)
                        + l[2][0] * Fixed::from_int(n3),
                    l[0][1] * Fixed::from_int(n1)
                        + l[1][1] * Fixed::from_int(n2)
                        + l[2][1] * Fixed::from_int(n3),
                    l[0][2] * Fixed::from_int(n1)
                        + l[1][2] * Fixed::from_int(n2)
                        + l[2][2] * Fixed::from_int(n3),
                ];
                let self_image = n1 == 0 && n2 == 0 && n3 == 0;
                for i in 0..n {
                    for j in 0..n {
                        if self_image && i == j {
                            continue;
                        }
                        let d = [
                            cart[i][0] - cart[j][0] + lat[0],
                            cart[i][1] - cart[j][1] + lat[1],
                            cart[i][2] - cart[j][2] + lat[2],
                        ];
                        let r2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
                        if r2 <= Fixed::ZERO {
                            continue;
                        }
                        let r = r2.sqrt();
                        s_sum[i][j] += shielding(r, tau[i], tau[j]);
                    }
                }
            }
        }
    }

    // The shielded periodic Gamma matrix in eV, then the augmented (N+1) equalization system.
    let k = k_ev_angstrom();
    let two = Fixed::from_int(2);
    let mut m = vec![vec![Fixed::ZERO; n + 1]; n + 1];
    let mut rhs = vec![Fixed::ZERO; n + 1];
    for i in 0..n {
        for j in 0..n {
            let mut gamma = k * (a[i][j] - s_sum[i][j]);
            if i == j {
                gamma += two * eta[i];
            }
            m[i][j] = gamma;
        }
        m[i][n] = Fixed::ZERO - Fixed::ONE; // the -lambda column
        m[n][i] = Fixed::ONE; // the neutrality row
        rhs[i] = Fixed::ZERO - chi[i];
    }
    // rhs[n] = 0 (total charge zero); m[n][n] = 0.
    let solution = solve_linear(m, rhs)?;
    Some(solution[..n].to_vec())
}

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

    // --- The shielded QEq solve, validated on periclase (MgO) ---

    use crate::ewald::{Cell, Ion};

    fn ang(x: f64) -> Fixed {
        // A test-only length in angstroms from a simple decimal (exact for the values used here).
        Fixed::from_ratio((x * 1000.0).round() as i64, 1000)
    }

    fn frac(x: f64) -> Fixed {
        Fixed::from_ratio((x * 100.0).round() as i64, 100)
    }

    /// The conventional rock-salt MgO cell: cube side `a`, Mg (+ sublattice) and O (- sublattice), four formula
    /// units. `a = 2 (r_Mg + r_O)` from the Shannon radii (0.72 + 1.40 = 2.12 A), so `a = 4.24 A`.
    fn mgo_cell() -> Cell {
        let a = ang(4.24);
        let z = Fixed::ZERO;
        let mk = |u: f64, v: f64, w: f64| Ion {
            frac: [frac(u), frac(v), frac(w)],
            charge: Fixed::ZERO, // ignored by the solve
        };
        Cell {
            lattice: [[a, z, z], [z, a, z], [z, z, a]],
            ions: vec![
                mk(0.0, 0.0, 0.0),
                mk(0.5, 0.5, 0.0),
                mk(0.5, 0.0, 0.5),
                mk(0.0, 0.5, 0.5),
                mk(0.5, 0.0, 0.0),
                mk(0.0, 0.5, 0.0),
                mk(0.0, 0.0, 0.5),
                mk(0.5, 0.5, 0.5),
            ],
        }
    }

    #[test]
    fn periclase_qeq_is_stable_symmetric_and_neutral() {
        // The shielded QEq solve on periclase (four Mg at the unbound-limit chi=eta=3.823, four O at chi=7.5395,
        // eta=6.0785) CONVERGES to a bounded, symmetric, neutral charge, where the bare point-charge Ewald would
        // run away (the shielding restores positive-definiteness, the instability catch confirmed). Charge flows
        // Mg -> O (Mg positive, O negative). NOTE: the raw-Mulliken parameters plus the 1s-model gamma
        // OVER-IONIZE this strong ionic (q_Mg lands slightly above the formal +2, not the fitted-potential
        // ~+1.6), the known limit of derive-first QEq documented on the module; this test pins the machinery
        // (stable, symmetric, neutral, correctly signed, bounded), not the fitted target.
        let t = table();
        let mg = element_chi_eta(t.element("Mg").unwrap()).unwrap();
        let o = element_chi_eta(t.element("O").unwrap()).unwrap();
        let cell = mgo_cell();
        let chi = vec![mg.chi, mg.chi, mg.chi, mg.chi, o.chi, o.chi, o.chi, o.chi];
        let eta = vec![mg.eta, mg.eta, mg.eta, mg.eta, o.eta, o.eta, o.eta, o.eta];
        let q = qeq_charges(&cell, &chi, &eta).expect("the shielded QEq solve converges");
        let q_mg = q[0];
        let q_o = q[4];
        assert!(
            close(q[1], q_mg.to_f64_lossy(), 1e-4)
                && close(q[2], q_mg.to_f64_lossy(), 1e-4)
                && close(q[3], q_mg.to_f64_lossy(), 1e-4),
            "the rock-salt symmetry gives four equal Mg charges"
        );
        assert!(
            close(q[5], q_o.to_f64_lossy(), 1e-4),
            "the rock-salt symmetry gives equal O charges"
        );
        assert!(
            close(q_mg + q_o, 0.0, 1e-4),
            "charge neutrality: q_Mg + q_O = 0, got {} + {}",
            q_mg.to_f64_lossy(),
            q_o.to_f64_lossy()
        );
        assert!(
            q_mg > Fixed::ZERO,
            "charge flows Mg -> O, so Mg is positive, got {}",
            q_mg.to_f64_lossy()
        );
        assert!(
            q_mg < Fixed::from_int(3),
            "the shielding bounds the charge (no bare-Ewald runaway), got {}",
            q_mg.to_f64_lossy()
        );
    }
}
