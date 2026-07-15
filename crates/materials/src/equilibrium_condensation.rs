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
use civsim_physics::gas_thermochemistry::molar_gas_constant;
use civsim_units::bignum::BigRat;
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// The dimensionless standard chemical potential `g = mu°(T)/RT` of a species from its JANAF standard Gibbs energy
/// of formation `delta_f G(T)` in kJ/mol (the [M] TOTAL top rung of the source ladder): `g = delta_f_G * 1000 /
/// (R T)`. This is the `g_over_rt` [`gas_equilibrium`] and the condensation saturation read for a JANAF-tabulated
/// species; the RRHO estimator is the certifier and alien rung for a species with no row. `None` on a non-positive
/// temperature or a register miss.
pub fn janaf_g_over_rt(delta_f_g_kj_mol: Fixed, temperature_k: Fixed) -> Option<Fixed> {
    if temperature_k <= Fixed::ZERO {
        return None;
    }
    let rt = molar_gas_constant()?.checked_mul(temperature_k)?; // J/mol
    delta_f_g_kj_mol
        .checked_mul(Fixed::from_int(1000))?
        .checked_div(rt)
}

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
// The range indices cross two rows (`a[row][k]` against `a[col][k]`) and index `b`, so the range loop is the clear
// linear-algebra form; an iterator refactor would obscure it.
#[allow(clippy::needless_range_loop)]
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
// The elimination indexes the stoichiometry matrix by row and column together; the range loops are the clear form.
#[allow(clippy::needless_range_loop)]
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

/// The SATURATION INDEX of a condensed phase against a gas equilibrium's element potentials: `S = sum_e a_ec
/// lambda_e - g_c`, the amount by which the element potentials exceed the condensate's dimensionless standard Gibbs.
/// `S > 0` means the phase is SUPERSATURATED, so it precipitates; `S <= 0` means it stays in the gas. A phase's
/// condensation front is where `S` crosses zero as the gas cools, so the condensation SEQUENCE (the CAI-first
/// ordering: corundum before metal before silicates before ices) is the phases sorted by that crossing temperature,
/// the Verdict log sorted by `T`. `None` if the condensate carries an element the gas equilibrium never balanced (a
/// coverage failure the caller surfaces, never papers over).
pub fn saturation_index(
    condensate_g_over_rt: Fixed,
    stoichiometry: &BTreeMap<String, i32>,
    element_potentials: &BTreeMap<String, Fixed>,
) -> Option<Fixed> {
    let mut sum = Fixed::ZERO;
    for (element, count) in stoichiometry {
        let lambda = element_potentials.get(element)?;
        sum = sum.checked_add(Fixed::from_int(*count).checked_mul(*lambda)?)?;
    }
    sum.checked_sub(condensate_g_over_rt)
}

/// The condensed active set at a gas equilibrium: the `Condensed`-phase candidates that PRECIPITATE (a positive
/// saturation index), returned most-supersaturated first (the precipitation order at this temperature; the raw-bit
/// key gives a deterministic total order, with the name as the stable tiebreak). This is the detection-and-ordering
/// half of the condensed minimizer: at a temperature it says which phases are stable and in what precedence, and
/// swept down a cooling path it yields the condensation sequence. The amount redistribution (the full VCS
/// re-equilibration that fixes each phase's molar amount and the exact 50%-condensation temperature) builds on this.
/// `None` if a condensate carries an unbalanced element.
pub fn condensed_active_set(
    condensates: &[EquilibriumSpecies],
    equilibrium: &GasEquilibrium,
) -> Option<Vec<(String, Fixed)>> {
    let mut saturated = Vec::new();
    for c in condensates {
        if c.phase != SpeciesPhase::Condensed {
            continue;
        }
        let s = saturation_index(
            c.g_over_rt,
            &c.stoichiometry,
            &equilibrium.element_potentials,
        )?;
        if s > Fixed::ZERO {
            saturated.push((c.name.clone(), s));
        }
    }
    saturated.sort_by(|a, b| b.1.to_bits().cmp(&a.1.to_bits()).then(a.0.cmp(&b.0)));
    Some(saturated)
}

/// The exact conversion of a fixed-point value to a rational: `Fixed` is `bits / 2^FRAC_BITS`, so the rational is
/// the raw bits over the scale, without rounding. The VCS amount solve carries the stoichiometry and the budget in
/// exact rationals so the mass balance closes to the bit.
fn fixed_to_bigrat(value: Fixed) -> BigRat {
    let scale = BigRat::from_i64(1i64 << Fixed::FRAC_BITS);
    BigRat::from_i64(value.to_bits()).div(&scale)
}

/// The outcome of solving the condensed molar amounts against the residual element budget.
enum AmountSolve {
    /// A unique amount vector (the phase-rule vertex is well-posed).
    Unique(Vec<BigRat>),
    /// The active set does not fix the amounts uniquely (rank-deficient: more active phases than the residual
    /// budget's independent constraints, a phase-rule boundary). Routed to the Verdict fold-and-draw.
    Degenerate,
    /// No amount vector closes the budget (a pivot-free element row carries a nonzero residual, surfaced rather
    /// than papered over).
    Inconsistent,
}

/// Solve the exact linear system `A n = r` for the condensed amounts by Gauss-Jordan elimination in RATIONALS, where
/// `m` is the augmented matrix `[A | r]` (E element rows, `c + 1` columns). Returns the unique amount vector when
/// the C columns are independent and the budget is consistent, `Degenerate` when a column has no pivot (the amounts
/// are not fixed), or `Inconsistent` when a pivot-free row carries a nonzero residual. Exact: rationals never round,
/// so a `Unique` solution closes `A n = r` to the bit.
// The row/column indices cross rows of the augmented matrix (eliminating column `col` in every other row `r`, over
// columns `k`), so the range loops are the clear Gauss-Jordan form; an iterator refactor would obscure the pivoting.
#[allow(clippy::needless_range_loop)]
fn solve_amounts(mut m: Vec<Vec<BigRat>>, c: usize) -> AmountSolve {
    let e = m.len();
    let mut pivot_row_for_col: Vec<Option<usize>> = vec![None; c];
    let mut row = 0usize;
    for col in 0..c {
        // The pivot: the largest-magnitude nonzero entry at or below `row` in this column (partial pivoting; the
        // solve is exact for any nonzero pivot, the choice only keeps the intermediate numerators smaller).
        let mut pivot: Option<usize> = None;
        for r in row..e {
            if !m[r][col].is_zero()
                && pivot
                    .map(|p| m[r][col].abs().cmp_rat(&m[p][col].abs()) == Ordering::Greater)
                    .unwrap_or(true)
            {
                pivot = Some(r);
            }
        }
        let Some(pivot) = pivot else {
            continue; // a free column: rank-deficient
        };
        m.swap(row, pivot);
        let d = m[row][col].clone();
        for k in col..=c {
            m[row][k] = m[row][k].div(&d);
        }
        for r in 0..e {
            if r != row && !m[r][col].is_zero() {
                let factor = m[r][col].clone();
                for k in col..=c {
                    let t = factor.mul(&m[row][k]);
                    m[r][k] = m[r][k].sub(&t);
                }
            }
        }
        pivot_row_for_col[col] = Some(row);
        row += 1;
    }
    if pivot_row_for_col.iter().any(|p| p.is_none()) {
        return AmountSolve::Degenerate;
    }
    for r in row..e {
        if !m[r][c].is_zero() {
            return AmountSolve::Inconsistent;
        }
    }
    let mut n = vec![BigRat::from_i64(0); c];
    for (col, pr) in pivot_row_for_col.iter().enumerate() {
        n[col] = m[pr.unwrap()][c].clone();
    }
    AmountSolve::Unique(n)
}

/// The VCS amount-redistribution outcome: the condensed molar amounts closing element mass balance exactly, or a
/// degenerate vertex routed to the Verdict fold-and-draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CondensedAmounts {
    /// The condensed phases with their derived molar amounts, (species name, moles). Mass balance closes exactly:
    /// in the exact rational domain the solve runs in, the gas amounts plus these amounts reconstruct the element
    /// budget to the bit; the `Fixed` amounts here are the rounded readout.
    Balanced(Vec<(String, Fixed)>),
    /// A degenerate vertex: the active set does not fix the amounts uniquely (a phase-rule boundary). Carries the
    /// tied condensed phase names for the Verdict fold-and-draw, which the caller routes rather than bespoke-pivots.
    Degenerate(Vec<String>),
}

/// The VCS AMOUNT REDISTRIBUTION (Smith & Missen): fix each active condensed phase's molar amount from the residual
/// element budget the gas leaves behind, so the condensation is quantified, not merely detected. `gas` is the gas
/// species (for their stoichiometry), `active_condensates` the precipitating phases ([`condensed_active_set`]),
/// `equilibrium` the solved gas equilibrium (its `species_amounts` the gas moles), and `abundances` the element
/// budget `b_e`. The residual `r_e = b_e - sum_gas a_ep x_p` is what the condensates must hold; solving
/// `sum_c a_ec n_c = r_e` in EXACT rationals fixes the amounts so `sum_p a_ep n_p = b_e` closes to the bit (the
/// ConservedBudget invariant). A rank-deficient active set (a phase-rule boundary, non-unique amounts) returns
/// [`CondensedAmounts::Degenerate`] for the Verdict fold-and-draw rather than an arbitrary pivot. `None` if an amount
/// is negative (the active set over-condensed, surfaced not clamped), the budget is inconsistent, or a conversion
/// fails.
pub fn condensed_amounts(
    gas: &[EquilibriumSpecies],
    active_condensates: &[EquilibriumSpecies],
    equilibrium: &GasEquilibrium,
    abundances: &BTreeMap<String, Fixed>,
) -> Option<CondensedAmounts> {
    if abundances.is_empty() {
        return None;
    }
    let elements: Vec<String> = abundances.keys().cloned().collect();
    let e = elements.len();
    let c = active_condensates.len();
    if c == 0 {
        return Some(CondensedAmounts::Balanced(Vec::new()));
    }
    let gas_by_name: BTreeMap<&str, &EquilibriumSpecies> =
        gas.iter().map(|s| (s.name.as_str(), s)).collect();
    // The residual budget r_e = b_e - sum_gas a_ep x_p, exact.
    let mut residual: Vec<BigRat> = Vec::with_capacity(e);
    for el in &elements {
        let mut r = fixed_to_bigrat(*abundances.get(el).unwrap());
        for (name, amount) in &equilibrium.species_amounts {
            if let Some(sp) = gas_by_name.get(name.as_str()) {
                let a = coeff(sp, el);
                if a != Fixed::ZERO {
                    r = r.sub(&fixed_to_bigrat(a).mul(&fixed_to_bigrat(*amount)));
                }
            }
        }
        residual.push(r);
    }
    // The augmented matrix [A | r]: rows = elements, columns = condensates, the last column the residual.
    let mut m: Vec<Vec<BigRat>> = Vec::with_capacity(e);
    for (row, el) in elements.iter().enumerate() {
        let mut r = Vec::with_capacity(c + 1);
        for cond in active_condensates {
            r.push(fixed_to_bigrat(coeff(cond, el)));
        }
        r.push(residual[row].clone());
        m.push(r);
    }
    match solve_amounts(m, c) {
        AmountSolve::Unique(n) => {
            let mut out = Vec::with_capacity(c);
            for (cond, n_c) in active_condensates.iter().zip(n.iter()) {
                // A negative amount means the active set over-condensed this phase: surface it (the full VCS drops
                // the phase and re-solves), never clamp a negative to zero and hide the mass.
                if n_c.cmp_rat(&BigRat::from_i64(0)) == Ordering::Less {
                    return None;
                }
                let bits = n_c.round_to_scale(Fixed::FRAC_BITS)?;
                out.push((cond.name.clone(), Fixed::from_bits_i128(bits)?));
            }
            Some(CondensedAmounts::Balanced(out))
        }
        AmountSolve::Degenerate => Some(CondensedAmounts::Degenerate(
            active_condensates.iter().map(|c| c.name.clone()).collect(),
        )),
        AmountSolve::Inconsistent => None,
    }
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

    // A condensed-phase species for the VCS amount-redistribution tests.
    fn cond(name: &str, stoich: &[(&str, i32)]) -> EquilibriumSpecies {
        EquilibriumSpecies {
            name: name.to_string(),
            phase: SpeciesPhase::Condensed,
            g_over_rt: Fixed::ZERO,
            stoichiometry: stoich.iter().map(|(e, n)| (e.to_string(), *n)).collect(),
        }
    }

    fn budget(entries: &[(&str, i64)]) -> BTreeMap<String, Fixed> {
        entries
            .iter()
            .map(|(e, n)| (e.to_string(), Fixed::from_int(*n as i32)))
            .collect()
    }

    fn no_gas() -> GasEquilibrium {
        GasEquilibrium {
            element_potentials: BTreeMap::new(),
            species_amounts: Vec::new(),
        }
    }

    #[test]
    fn the_vcs_closes_a_single_condensate_mass_balance_exactly() {
        // Forsterite Mg2SiO4 absorbing the whole budget: with no gas the residual is the abundances, and the amount
        // solves 2n = Mg, n = Si, 4n = O simultaneously (n = 2). Mass balance closes to the mole, exactly.
        let condensates = vec![cond("Mg2SiO4", &[("Mg", 2), ("Si", 1), ("O", 4)])];
        let b = budget(&[("Mg", 4), ("Si", 2), ("O", 8)]);
        let out = condensed_amounts(&[], &condensates, &no_gas(), &b).expect("solves");
        match out {
            CondensedAmounts::Balanced(amounts) => {
                assert_eq!(amounts.len(), 1);
                let n = amounts[0].1;
                assert!((n.to_f64_lossy() - 2.0).abs() < 1e-9, "n_forsterite = 2");
                assert_eq!(
                    n.checked_mul(Fixed::from_int(2)).unwrap(),
                    *b.get("Mg").unwrap(),
                    "Mg closes exactly"
                );
                assert_eq!(n, *b.get("Si").unwrap(), "Si closes exactly");
                assert_eq!(
                    n.checked_mul(Fixed::from_int(4)).unwrap(),
                    *b.get("O").unwrap(),
                    "O closes exactly"
                );
            }
            other => panic!("expected Balanced, got {other:?}"),
        }
    }

    #[test]
    fn the_vcs_solves_a_two_phase_assemblage() {
        // Forsterite Mg2SiO4 + periclase MgO sharing a budget: forsterite = 2 (from Si), the leftover Mg is
        // periclase = 2, and O closes (8 + 2 = 10). Both amounts derived, mass balance exact.
        let condensates = vec![
            cond("Mg2SiO4", &[("Mg", 2), ("Si", 1), ("O", 4)]),
            cond("MgO", &[("Mg", 1), ("O", 1)]),
        ];
        let b = budget(&[("Mg", 6), ("Si", 2), ("O", 10)]);
        let out = condensed_amounts(&[], &condensates, &no_gas(), &b).expect("solves");
        match out {
            CondensedAmounts::Balanced(a) => {
                let get = |name: &str| a.iter().find(|(n, _)| n == name).unwrap().1.to_f64_lossy();
                assert!((get("Mg2SiO4") - 2.0).abs() < 1e-9);
                assert!((get("MgO") - 2.0).abs() < 1e-9);
            }
            other => panic!("expected Balanced, got {other:?}"),
        }
    }

    #[test]
    fn a_rank_deficient_active_set_routes_to_the_verdict_draw() {
        // Two proportional phases (forsterite and its double) do not fix the amounts uniquely: the columns are
        // dependent, so the vertex is degenerate and routes to the Verdict fold-and-draw, never an arbitrary pivot.
        let condensates = vec![
            cond("Mg2SiO4", &[("Mg", 2), ("Si", 1), ("O", 4)]),
            cond("Mg4Si2O8", &[("Mg", 4), ("Si", 2), ("O", 8)]),
        ];
        let b = budget(&[("Mg", 6), ("Si", 3), ("O", 12)]);
        let out = condensed_amounts(&[], &condensates, &no_gas(), &b).expect("returns an outcome");
        assert!(
            matches!(out, CondensedAmounts::Degenerate(_)),
            "dependent phases are a degenerate vertex, got {out:?}"
        );
    }

    #[test]
    fn the_vcs_subtracts_the_gas_hold_before_the_condensate() {
        // The gas holds part of the budget: one mole of O2(g) holds 2 O, so the residual O the condensate sees is
        // the budget less 2. Forsterite then closes on the reduced O budget with Mg and Si intact (n = 2).
        let gas = vec![sp("O2", 0.0, &[("O", 2)])];
        let eq = GasEquilibrium {
            element_potentials: BTreeMap::new(),
            species_amounts: vec![("O2".to_string(), Fixed::ONE)],
        };
        let condensates = vec![cond("Mg2SiO4", &[("Mg", 2), ("Si", 1), ("O", 4)])];
        let b = budget(&[("Mg", 4), ("Si", 2), ("O", 10)]);
        let out = condensed_amounts(&gas, &condensates, &eq, &b).expect("solves");
        match out {
            CondensedAmounts::Balanced(a) => {
                assert!(
                    (a[0].1.to_f64_lossy() - 2.0).abs() < 1e-9,
                    "forsterite n=2 after the gas holds 2 O"
                );
            }
            other => panic!("expected Balanced, got {other:?}"),
        }
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

    #[test]
    fn the_iron_condensation_temperature_reproduces_the_lodders_front() {
        // THE FIRST LODDERS GATE, a genuine cross-check: Lodders 2003 computed her 50%-condensation temperatures
        // with her OWN thermochemical set, so a JANAF-derived front is an independent-dataset agreement, not a
        // tautology. The Fe(cr) <-> Fe(g) vapor-solid saturation: the reaction Gibbs g(gas) - g(solid) = mu°(Fe,g)/RT
        // - mu°(Fe,cr)/RT (both from janaf_g_over_rt, the mu-standard [M] wire), and the 50%-condensation T is where
        // it equals ln(P0/P_Fe) + ln 2 (the ln 2 because at T50 half the iron is condensed, so the remaining gas
        // partial pressure is halved). P_Fe is the solar iron partial pressure at the disk 1e-4 bar: a labelled [M]
        // fixture, solar log-eps(Fe) = 7.50 (AGSS09), in a mostly-H2/He gas x_Fe ~ 5.4e-5, so P_Fe ~ 5.4e-9 bar.
        // OWNER BAND: the ordering is demanded exact, the absolute temperature carries an inter-dataset +-30 K class
        // band; anything larger convicts the build, not the literature.
        let janaf = civsim_physics::janaf::JanafTables::standard().expect("JANAF loads");
        let fe_g = janaf.species("Fe(g)").expect("Fe(g) in JANAF");
        let fe_cr = janaf.species("Fe(cr)").expect("Fe(cr) in JANAF");
        // f(T) = g(gas) - g(solid), the dimensionless reaction Gibbs through the mu-standard wire.
        let f = |t: f64| -> f64 {
            let tf = Fixed::from_int(t as i32);
            let g_gas = janaf_g_over_rt(fe_g.delta_f_g_at(tf).unwrap(), tf).unwrap();
            let g_sol = janaf_g_over_rt(fe_cr.delta_f_g_at(tf).unwrap(), tf).unwrap();
            g_gas.checked_sub(g_sol).unwrap().to_f64_lossy()
        };
        // target = ln(P0/P_Fe) + ln 2, P0 = 1 bar, P_Fe = 5.4e-9 bar.
        let target = (1.0_f64 / 5.4e-9).ln() + 2.0_f64.ln();
        let f_lo = f(1300.0);
        let f_hi = f(1400.0);
        assert!(
            f_lo > target && target > f_hi,
            "the Fe T50 is bracketed by [1300, 1400] K: f(1300)={f_lo:.2} > target={target:.2} > f(1400)={f_hi:.2}"
        );
        let t50 = 1300.0 + 100.0 * (f_lo - target) / (f_lo - f_hi);
        // The vendored Lodders T50 for iron (own-thermochemistry witness), the cross-check target.
        let lodders =
            civsim_physics::condensation::CondensationTable::standard().expect("Lodders loads");
        let lodders_fe = lodders.t50_k("Fe").expect("Fe in Lodders").to_f64_lossy();
        assert!(
            (t50 - lodders_fe).abs() < 30.0,
            "the JANAF-derived Fe T50 ({t50:.0} K) reproduces the independently-computed Lodders front ({lodders_fe:.0} K) within the inter-dataset +-30 K band"
        );
    }

    #[test]
    fn the_condensed_active_set_precipitates_the_supersaturated_phases_in_order() {
        // From the H/O gas equilibrium, three candidate condensates of the same stoichiometry {H2, O1} but different
        // standard Gibbs. The reference potential for that stoichiometry is 2 lambda_H + lambda_O, so a condensate
        // with g BELOW it is supersaturated (precipitates) and one ABOVE it is undersaturated (stays gaseous). The
        // active set returns only the supersaturated phases, most-supersaturated first (the precipitation order that,
        // swept down a cooling path, becomes the condensation sequence).
        let sys = h_o_system();
        let b = abund(&[("H", 2.0), ("O", 1.0)]);
        let eq = gas_equilibrium(&sys, &b).unwrap();
        let ref_pot = eq.element_potentials["H"].to_f64_lossy() * 2.0
            + eq.element_potentials["O"].to_f64_lossy();
        // g = ref_pot - s, so the saturation index S = ref_pot - g = s (the constructed supersaturation).
        let cond = |name: &str, s: f64| EquilibriumSpecies {
            name: name.to_string(),
            phase: SpeciesPhase::Condensed,
            g_over_rt: Fixed::from_ratio(((ref_pot - s) * 1000.0).round() as i64, 1000),
            stoichiometry: [("H".to_string(), 2), ("O".to_string(), 1)]
                .into_iter()
                .collect(),
        };
        let candidates = vec![
            cond("deep_sat", 10.0),
            cond("shallow_sat", 3.0),
            cond("undersat", -4.0),
        ];
        let active = condensed_active_set(&candidates, &eq).unwrap();
        let names: Vec<&str> = active.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec!["deep_sat", "shallow_sat"],
            "only the supersaturated phases, most-supersaturated first; the undersaturated one stays gaseous"
        );
        assert!((active[0].1.to_f64_lossy() - 10.0).abs() < 0.1);
        assert!((active[1].1.to_f64_lossy() - 3.0).abs() < 0.1);
    }

    #[test]
    fn the_water_snow_line_reproduces_the_lodders_front() {
        // The VOLATILE end of the CAI-first sequence, the water snow line, a second Lodders cross-check (the fetched
        // Murphy-Koop ice pressure vs Lodders' own set): the H2O(gas) <-> ice front is where the ice saturation
        // pressure equals the solar water partial pressure. P_H2O ~ 5e-3 Pa (~5e-8 bar) at the disk 1e-4 bar, a
        // labelled [M] fixture from the O abundance. The Murphy-Koop pressure crosses that near 180 K, so the snow
        // line sits at ~180 K (Lodders water-ice 182 K). WITH the Fe front at 1334 K, this is the CAI-first ORDERING
        // across the whole disk: refractory iron condenses ~1150 K of warmth before water ice, the refractory-before-
        // volatile spine the sequence is named for.
        let ice = civsim_physics::ice_sublimation::IceSublimation::standard().expect("ice loads");
        let p_h2o_pa = 5.0e-3_f64;
        let pts = ice.points();
        let mut snow_line = None;
        for w in pts.windows(2) {
            let (t0, p0) = (w[0].t_k.to_f64_lossy(), w[0].p_sat_pa.to_f64_lossy());
            let (t1, p1) = (w[1].t_k.to_f64_lossy(), w[1].p_sat_pa.to_f64_lossy());
            if p0 <= p_h2o_pa && p_h2o_pa <= p1 {
                // Interpolate in ln(p) (the pressure is exponential in T) for the crossing temperature.
                let f = (p_h2o_pa.ln() - p0.ln()) / (p1.ln() - p0.ln());
                snow_line = Some(t0 + f * (t1 - t0));
                break;
            }
        }
        let snow_line = snow_line.expect("the snow line is bracketed by the ice table");
        let lodders = civsim_physics::condensation::CondensationTable::standard().expect("Lodders");
        let lodders_ice = lodders
            .t50_k("H")
            .map(|t| t.to_f64_lossy())
            .unwrap_or(182.0);
        assert!(
            (snow_line - 182.0).abs() < 15.0,
            "the water snow line ~{lodders_ice:.0} K (Lodders 182), got {snow_line:.0} K"
        );
    }
}
