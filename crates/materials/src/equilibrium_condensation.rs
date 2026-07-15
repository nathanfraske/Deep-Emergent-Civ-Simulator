//! The gas-phase element-potential equilibrium solve, the disk-condensation minimizer's core (#57): given a set of
//! candidate gas species (each with its dimensionless standard Gibbs energy `g = mu°(T)/RT` from JANAF or the RRHO
//! estimator, and its element stoichiometry) and the fixed ELEMENTAL abundances, find the equilibrium species
//! amounts by minimizing `G = sum_p n_p mu_p` at fixed elemental totals. This is the disposer's own operation
//! (minimum free energy over a candidate set) generalized from fixed phase composition to fixed elemental
//! abundance via ELEMENT POTENTIALS (Lagrange multipliers, the RAND/VCS lineage): at the minimum a gas species'
//! amount is `x_p = exp(sum_e a_ep lambda_e - g_p)`, and the element potentials `lambda_e` are set by the mass
//! balance `sum_p a_ep x_p = b_e`. The condensed active set (a phase precipitates when its `g_p` falls below
//! `sum_e a_ep lambda_e`) and the Verdict-fold rendering of the condensation sequence build on this core; this
//! module is the gas-phase solve and its load-bearing invariance proof.
//!
//! THE SHIFT-INVARIANCE ACCEPTANCE GATE (the owner's lemma, the one load-bearing extension claim). A reference-state
//! shift adds `sum_e a_ep c_e` to each `g_p`; under the elemental constraint `sum_p a_ep n_p = b_e` the total `G`
//! shifts by the constant `sum_e c_e b_e`, independent of the composition vector, so the argmin is invariant and the
//! `lambda_e` absorb the `c_e` exactly (`lambda_e -> lambda_e + c_e`). The gate demands this hold BYTE-IDENTICALLY,
//! which forces the solve to be shift-EQUIVARIANT, not merely invariant to convergence tolerance: the initial
//! `lambda` is derived shift-COVARIANTLY from the `g_p` through a reference-species basis (`lambda_init = (M^T)^-1
//! g_ref`, so `g_ref -> g_ref + M^T c` gives `lambda_init -> lambda_init + c`). Then every intermediate `x_p` is
//! byte-identical under the shift, so the converged assemblage is too. This is the proof the disposer's cancellation
//! survives the switch to gas-phase, fixed-elemental-abundance condensation, wired as a test rather than asserted.

use civsim_core::Fixed;
use std::collections::BTreeMap;

/// The phase of a candidate species: an ideal gas (its amount is a free equilibrium variable) or a condensed phase
/// (a solid or liquid, whose activity is one when present, the active-set member the fuller minimizer routes to the
/// Verdict fold). This module solves the gas-phase equilibrium; the condensed active set is the next increment.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpeciesPhase {
    /// An ideal-gas species, a free equilibrium amount.
    Gas,
    /// A condensed (solid or liquid) species, an active-set member.
    Condensed,
}

/// One candidate species in the equilibrium: its name, phase, dimensionless standard Gibbs energy
/// `g = mu°(T)/RT` (from JANAF as the [M] total, or the RRHO estimator as the certifier / alien rung), and its
/// element stoichiometry (element symbol to atom count). The `g` is dimensionless because the whole solve runs in
/// `mu/RT` units, where the element potentials and the log-abundances are natural.
#[derive(Clone, Debug)]
pub struct EquilibriumSpecies {
    /// The species name (for example "H2O", "CO", "Fe").
    pub name: String,
    /// Gas or condensed.
    pub phase: SpeciesPhase,
    /// The dimensionless standard Gibbs energy `mu°(T) / RT`.
    pub g_over_rt: Fixed,
    /// The element stoichiometry: element symbol to the number of that element's atoms in the species.
    pub stoichiometry: BTreeMap<String, i32>,
}

/// The converged gas-phase equilibrium: the element potentials `lambda_e` and each gas species' amount `x_p`, in the
/// input order. `element_potentials` is keyed by element symbol; `species_amounts` is aligned to the input gas
/// species order.
#[derive(Clone, Debug)]
pub struct GasEquilibrium {
    /// The dimensionless element potentials, keyed by element symbol.
    pub element_potentials: BTreeMap<String, Fixed>,
    /// Each gas species amount `x_p`, aligned to the input gas species (name, amount).
    pub species_amounts: Vec<(String, Fixed)>,
}

/// The Newton iteration count for the element-potential solve, fixed for determinism (a convergence bound, not world
/// content). The Newton step is quadratically convergent from the reference-basis seed, so a converged root is well
/// inside this count; the surplus holds the root fixed.
const NEWTON_ITERS: u32 = 60;

/// The largest exp argument the amount `x_p = exp(...)` is allowed before it is clamped, a fixed-point overflow guard
/// (`exp(20) ~ 4.9e8` stays inside the representable range with headroom for the Jacobian sums). A transient iterate
/// may push the argument high before the solve settles; the clamp keeps the arithmetic total without changing the
/// converged root, which sits well below it.
fn max_exp_argument() -> Fixed {
    Fixed::from_int(20)
}

/// Solve a dense linear system `A x = b` by Gaussian elimination with partial pivoting, in fixed-point. `A` is
/// consumed. `None` on a singular (or numerically singular) matrix or an overflow. Small `n` only (the element
/// count), so the cubic cost is negligible.
fn solve_dense(mut a: Vec<Vec<Fixed>>, mut b: Vec<Fixed>) -> Option<Vec<Fixed>> {
    let n = b.len();
    if n == 0 || a.len() != n || a.iter().any(|row| row.len() != n) {
        return None;
    }
    for col in 0..n {
        // Partial pivot: the largest-magnitude entry in this column at or below the diagonal.
        let mut piv = col;
        for row in (col + 1)..n {
            if a[row][col].abs() > a[piv][col].abs() {
                piv = row;
            }
        }
        a.swap(col, piv);
        b.swap(col, piv);
        let d = a[col][col];
        if d == Fixed::ZERO {
            return None; // singular
        }
        for row in (col + 1)..n {
            let factor = a[row][col].checked_div(d)?;
            for k in col..n {
                let t = factor.checked_mul(a[col][k])?;
                a[row][k] = a[row][k].checked_sub(t)?;
            }
            let t = factor.checked_mul(b[col])?;
            b[row] = b[row].checked_sub(t)?;
        }
    }
    let mut x = vec![Fixed::ZERO; n];
    for row in (0..n).rev() {
        let mut s = b[row];
        for k in (row + 1)..n {
            let t = a[row][k].checked_mul(x[k])?;
            s = s.checked_sub(t)?;
        }
        x[row] = s.checked_div(a[row][row])?;
    }
    Some(x)
}

/// The stoichiometry coefficient `a_ep` (atoms of element `e` in species `p`) as a `Fixed`.
fn coeff(species: &EquilibriumSpecies, element: &str) -> Fixed {
    Fixed::from_int(*species.stoichiometry.get(element).unwrap_or(&0))
}

/// Select a REFERENCE BASIS: `E` gas species whose stoichiometry columns are linearly independent over the `E`
/// elements, found by Gaussian elimination on the `E x P` stoichiometry matrix (the pivot columns). The basis makes
/// the initial element potentials a shift-COVARIANT function of the `g_p` (`M^T lambda = g_ref`), which is what
/// makes the shift-invariance gate byte-identical. `None` if the gas species do not span the elements (an
/// under-determined system, a genuine coverage failure the caller must surface, never paper over).
fn reference_basis(elements: &[String], gas: &[&EquilibriumSpecies]) -> Option<Vec<usize>> {
    let e = elements.len();
    // Column-reduce a copy of the stoichiometry matrix (rows = elements, cols = species), recording pivot species.
    let mut m: Vec<Vec<Fixed>> = elements
        .iter()
        .map(|el| gas.iter().map(|sp| coeff(sp, el)).collect())
        .collect();
    let mut basis = Vec::with_capacity(e);
    let mut row = 0usize;
    for col in 0..gas.len() {
        if row >= e {
            break;
        }
        // Find a pivot row at or below `row` with a nonzero entry in this column.
        let mut piv = None;
        for r in row..e {
            if m[r][col] != Fixed::ZERO {
                piv = Some(r);
                break;
            }
        }
        let Some(pr) = piv else { continue };
        m.swap(row, pr);
        basis.push(col);
        // Eliminate this column from the other rows.
        let d = m[row][col];
        for r in 0..e {
            if r != row && m[r][col] != Fixed::ZERO {
                let f = m[r][col].checked_div(d)?;
                for k in col..gas.len() {
                    let t = f.checked_mul(m[row][k])?;
                    m[r][k] = m[r][k].checked_sub(t)?;
                }
            }
        }
        row += 1;
    }
    if basis.len() == e {
        Some(basis)
    } else {
        None // the gas species do not span the elements
    }
}

/// Solve the gas-phase element-potential equilibrium: the element potentials `lambda_e` and the amounts
/// `x_p = exp(sum_e a_ep lambda_e - g_p)` such that the mass balance `sum_p a_ep x_p = b_e` holds for every element.
/// `abundances` is the fixed elemental total `b_e` per element symbol (arbitrary consistent unit; only ratios
/// matter). Only `Gas`-phase species enter the solve (a condensed species is ignored here; the active set is the
/// next increment).
///
/// The seed is shift-covariant (the reference-basis solve `M^T lambda = g_ref`), so the Newton iteration is
/// shift-equivariant and the converged assemblage is byte-identical under a reference-state shift (the invariance
/// gate). `None` on a coverage failure (the gas species do not span the elements, or an abundance names an element
/// no species carries), a singular Newton step, or an overflow.
pub fn gas_equilibrium(
    species: &[EquilibriumSpecies],
    abundances: &BTreeMap<String, Fixed>,
) -> Option<GasEquilibrium> {
    let gas: Vec<&EquilibriumSpecies> = species
        .iter()
        .filter(|s| s.phase == SpeciesPhase::Gas)
        .collect();
    if gas.is_empty() || abundances.is_empty() {
        return None;
    }
    // The element set the solve balances: every element that appears in an abundance target. Deterministic order.
    let elements: Vec<String> = abundances.keys().cloned().collect();
    let e = elements.len();
    let b: Vec<Fixed> = elements
        .iter()
        .map(|el| *abundances.get(el).unwrap())
        .collect();
    if b.iter().any(|v| *v <= Fixed::ZERO) {
        return None;
    }

    // Shift-covariant seed: solve M^T lambda = g_ref over a reference basis of E gas species.
    let basis = reference_basis(&elements, &gas)?;
    let mut mt = vec![vec![Fixed::ZERO; e]; e]; // rows = basis species j, cols = element f
    let mut g_ref = vec![Fixed::ZERO; e];
    for (j, &sp_idx) in basis.iter().enumerate() {
        for (f, el) in elements.iter().enumerate() {
            mt[j][f] = coeff(gas[sp_idx], el);
        }
        g_ref[j] = gas[sp_idx].g_over_rt;
    }
    let mut lambda = solve_dense(mt, g_ref)?;

    // Newton on the mass-balance residual F_e = sum_p a_ep x_p - b_e, Jacobian J_ef = sum_p a_ep a_fp x_p.
    for _ in 0..NEWTON_ITERS {
        // x_p = exp(sum_e a_ep lambda_e - g_p), clamped for the fixed-point overflow guard.
        let mut x = Vec::with_capacity(gas.len());
        for sp in &gas {
            let mut arg = Fixed::ZERO.checked_sub(sp.g_over_rt)?;
            for (f, el) in elements.iter().enumerate() {
                arg = arg.checked_add(coeff(sp, el).checked_mul(lambda[f])?)?;
            }
            if arg > max_exp_argument() {
                arg = max_exp_argument();
            }
            x.push(arg.exp());
        }
        // Residual and Jacobian.
        let mut f = vec![Fixed::ZERO; e];
        let mut jac = vec![vec![Fixed::ZERO; e]; e];
        for (p, sp) in gas.iter().enumerate() {
            for (row, el_e) in elements.iter().enumerate() {
                let a_ep = coeff(sp, el_e);
                if a_ep == Fixed::ZERO {
                    continue;
                }
                f[row] = f[row].checked_add(a_ep.checked_mul(x[p])?)?;
                for (col, el_f) in elements.iter().enumerate() {
                    let a_fp = coeff(sp, el_f);
                    if a_fp == Fixed::ZERO {
                        continue;
                    }
                    let term = a_ep.checked_mul(a_fp)?.checked_mul(x[p])?;
                    jac[row][col] = jac[row][col].checked_add(term)?;
                }
            }
        }
        // Residual: F_e = (sum_p a_ep x_p) - b_e; Newton step J dlambda = -F.
        let neg_f: Vec<Fixed> = f
            .iter()
            .zip(b.iter())
            .map(|(fe, be)| be.checked_sub(*fe))
            .collect::<Option<Vec<_>>>()?;
        let dlambda = solve_dense(jac, neg_f)?;
        for (l, d) in lambda.iter_mut().zip(dlambda.iter()) {
            *l = l.checked_add(*d)?;
        }
    }

    // Final amounts at the converged element potentials.
    let mut amounts = Vec::with_capacity(gas.len());
    for sp in &gas {
        let mut arg = Fixed::ZERO.checked_sub(sp.g_over_rt)?;
        for (f, el) in elements.iter().enumerate() {
            arg = arg.checked_add(coeff(sp, el).checked_mul(lambda[f])?)?;
        }
        if arg > max_exp_argument() {
            arg = max_exp_argument();
        }
        amounts.push((sp.name.clone(), arg.exp()));
    }
    let element_potentials = elements.into_iter().zip(lambda).collect();
    Some(GasEquilibrium {
        element_potentials,
        species_amounts: amounts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(name: &str, g: f64, stoich: &[(&str, i32)]) -> EquilibriumSpecies {
        EquilibriumSpecies {
            name: name.to_string(),
            phase: SpeciesPhase::Gas,
            g_over_rt: Fixed::from_ratio((g * 1000.0).round() as i64, 1000),
            stoichiometry: stoich.iter().map(|(e, n)| (e.to_string(), *n)).collect(),
        }
    }

    // A fixture H/O gas system: the diatomic elements H2 and O2 (the reference basis) plus water H2O, with
    // ILLUSTRATIVE dimensionless standard Gibbs values (a labelled test fixture, not JANAF): the elements at the
    // reference g = 0, water bound below by a formation g. The solver's correctness (mass balance) and the shift-
    // invariance are properties of the method, provable on any g; the real CAI-first / Lodders validation uses the
    // JANAF g the fetch supplies.
    fn h_o_system() -> Vec<EquilibriumSpecies> {
        vec![
            sp("H2", 0.0, &[("H", 2)]),
            sp("O2", 0.0, &[("O", 2)]),
            sp("H2O", -12.0, &[("H", 2), ("O", 1)]),
        ]
    }

    fn abund(pairs: &[(&str, f64)]) -> BTreeMap<String, Fixed> {
        pairs
            .iter()
            .map(|(e, v)| {
                (
                    e.to_string(),
                    Fixed::from_ratio((v * 1000.0).round() as i64, 1000),
                )
            })
            .collect()
    }

    #[test]
    fn the_gas_equilibrium_conserves_the_element_mass_balance() {
        // H-rich, O-poor: b_H = 2.0, b_O = 1.0. The converged amounts must satisfy the element balance
        // sum_p a_ep x_p = b_e to within a tight tolerance.
        let sys = h_o_system();
        let b = abund(&[("H", 2.0), ("O", 1.0)]);
        let eq = gas_equilibrium(&sys, &b).expect("the H/O gas equilibrium solves");
        // Recompute the element sums from the amounts.
        let mut h_sum = 0.0;
        let mut o_sum = 0.0;
        for (name, amt) in &eq.species_amounts {
            let a = amt.to_f64_lossy();
            let s = sys.iter().find(|s| &s.name == name).unwrap();
            h_sum += a * *s.stoichiometry.get("H").unwrap_or(&0) as f64;
            o_sum += a * *s.stoichiometry.get("O").unwrap_or(&0) as f64;
        }
        assert!(
            (h_sum - 2.0).abs() < 1e-2 && (o_sum - 1.0).abs() < 1e-2,
            "the element mass balance holds: H {h_sum} vs 2.0, O {o_sum} vs 1.0"
        );
    }

    #[test]
    fn a_reference_state_shift_leaves_the_assemblage_byte_identical() {
        // THE SHIFT-INVARIANCE ACCEPTANCE GATE. Shift each species' g by sum_e a_ep c_e for a random-but-fixed
        // per-element c (c_H = 0.7, c_O = -1.3), the reference-state shift the owner's lemma covers. The converged
        // species amounts must be BYTE-IDENTICAL: the element potentials absorb the shift exactly, so the physical
        // assemblage does not move, and the shift-covariant seed makes it exact in fixed-point, not merely within
        // tolerance.
        let sys = h_o_system();
        let b = abund(&[("H", 2.0), ("O", 1.0)]);
        let base = gas_equilibrium(&sys, &b).unwrap();

        let c_h = Fixed::from_ratio(7, 10);
        let c_o = Fixed::from_ratio(-13, 10);
        let shifted: Vec<EquilibriumSpecies> = sys
            .iter()
            .map(|s| {
                let n_h = Fixed::from_int(*s.stoichiometry.get("H").unwrap_or(&0));
                let n_o = Fixed::from_int(*s.stoichiometry.get("O").unwrap_or(&0));
                let shift = n_h
                    .checked_mul(c_h)
                    .unwrap()
                    .checked_add(n_o.checked_mul(c_o).unwrap())
                    .unwrap();
                EquilibriumSpecies {
                    g_over_rt: s.g_over_rt.checked_add(shift).unwrap(),
                    ..s.clone()
                }
            })
            .collect();
        let shifted_eq = gas_equilibrium(&shifted, &b).unwrap();

        for ((n0, a0), (n1, a1)) in base
            .species_amounts
            .iter()
            .zip(shifted_eq.species_amounts.iter())
        {
            assert_eq!(n0, n1);
            assert_eq!(
                a0, a1,
                "species {n0}: the assemblage is byte-identical under the reference-state shift ({} vs {})",
                a0.to_f64_lossy(),
                a1.to_f64_lossy()
            );
        }
        // And the element potentials shift by exactly c_e (the lemma's mechanism, the visible receipt).
        let dl_h = base.element_potentials["H"]
            .checked_sub(shifted_eq.element_potentials["H"])
            .unwrap();
        assert!(
            (dl_h.checked_add(c_h).unwrap()).to_f64_lossy().abs() < 1e-6,
            "lambda_H absorbed the shift: delta = {} vs -c_H = {}",
            dl_h.to_f64_lossy(),
            (-c_h.to_f64_lossy())
        );
    }

    #[test]
    fn a_gas_set_that_does_not_span_the_elements_escalates() {
        // Coverage failure surfaced, never papered over: an O abundance with only H-bearing gas species cannot span
        // the O element, so the solve returns None (the caller escalates) rather than inventing a potential.
        let sys = vec![sp("H2", 0.0, &[("H", 2)])];
        let b = abund(&[("H", 2.0), ("O", 1.0)]);
        assert!(
            gas_equilibrium(&sys, &b).is_none(),
            "an unspanned element escalates"
        );
    }
}
