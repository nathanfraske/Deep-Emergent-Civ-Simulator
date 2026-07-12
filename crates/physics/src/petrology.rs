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

//! The petrology kernel (genesis-forward Stage 3, the surface lane): the stable mineral assemblage and the
//! pressure-temperature-dependent density a bulk composition reaches at a given pressure and temperature,
//! DERIVED by minimizing the total Gibbs free energy over the candidate-phase registry
//! ([`crate::petrology_data`]) rather than authored as a fixed mineral sequence. The assemblage EMERGES from
//! the thermodynamics and the world's own composition (Principle 8): a Terran bulk-silicate composition lands
//! the silicate assemblage its registry carries (olivine end-members in the seed; pyroxene, spinel, and melt
//! are registry rows a fuller dataset adds), an alien chemistry lands its own phases, because the mechanism
//! reads the data-defined registry and the per-world element budget, never a hardcoded CIPW-style allocation
//! order (which would author a Terran igneous sequence, the value-line violation the design forbids). The
//! mechanism is fixed Rust; the phase membership, their thermodynamics, and the bulk composition are data
//! (Principle 11).
//!
//! This module carries the ATOMIC BUILDING BLOCK, the per-phase Gibbs free energy at a pressure and
//! temperature; the assemblage-minimization over the element budget is the sibling follow-on that composes
//! these energies. Every value is fixed-point ([`Fixed`]) and deterministic (no float on the canonical path,
//! Principle 3). No consumer is wired to this kernel yet; it is a pure addition.

use crate::periodic::PeriodicTable;
use crate::petrology_data::{Phase, PhaseRegistry};
use civsim_core::Fixed;
use std::collections::BTreeMap;

/// The standard-state reference PRESSURE of the thermodynamic dataset, one bar. The enthalpy of formation and
/// the standard entropy in the registry are tabulated at this reference, so the pressure work term is measured
/// from it. A definitional anchor of the cited data (like the boiling-point reference pressure in the
/// Rankine-Kirchhoff law), not a per-world value. It is the SEED dataset's reference state; declaring the
/// reference state as registry data (so a world could carry an alien dataset with a different reference)
/// is the follow-on that lets datasets mix, and until then one internally consistent dataset is required.
pub const REFERENCE_PRESSURE_BAR: i32 = 1;

/// The standard-state reference TEMPERATURE of the thermodynamic dataset, 298.15 K, the temperature the
/// enthalpy of formation and standard entropy are tabulated at. Carried for the record and the Cp-integral
/// follow-on; the first-pass Gibbs energy below treats the enthalpy and entropy as constant from this
/// reference (the heat-capacity refinement is the flagged follow-on the registry grows to).
pub fn reference_temperature_k() -> Fixed {
    Fixed::from_ratio(29815, 100)
}

/// The apparent Gibbs free energy of formation of a stoichiometric phase at a temperature and pressure, in
/// joules per mole, the quantity a free-energy minimization compares across candidate assemblages. It is
///
/// `G(T, P) = dH_f - T * S + V * (P - P_ref)`,
///
/// the Benson-Helgeson apparent-Gibbs convention: the enthalpy of formation from the elements, less the
/// entropy term (so temperature favours the higher-entropy phase), plus the pressure work from the reference
/// (so pressure favours the lower-volume phase, which drives a depth polymorph sequence such as
/// olivine-to-spinel-to-perovskite ONCE the registry carries those competing polymorphs; the seed registry
/// carries no polymorph pair, so that sequence does not yet emerge from it). Because every competing
/// assemblage forms from the same element budget, the element reference
/// cancels in the comparison, so the apparent energy is the right quantity to minimize (the relative energies
/// decide the assemblage, which is why one internally consistent dataset is required).
///
/// UNITS and the representability choice: the enthalpy is kilojoules per mole (as the registry stores it) and
/// converts to joules; the entropy is joules per mole per kelvin; the molar volume is cubic centimetres per
/// mole. The PRESSURE is in BARS, not pascals, deliberately: geological pressures reach the order of a million
/// bars (about a hundred gigapascals) in the deep mantle, which overflows the canonical Q32.32 grid in
/// pascals, whereas a million bars is well inside it. The pressure work `V * (P - P_ref)` is then formed with
/// the exact unit bridge one cubic-centimetre-bar equals one tenth of a joule. Temperature is in kelvin.
///
/// FIRST PASS (the honest limit): the enthalpy, entropy, and molar volume are taken at their standard-state
/// values (constant in temperature and pressure), so the heat-capacity integral that bends `H(T)` and `S(T)`
/// and the compressibility and thermal expansion that bend `V(P, T)` are the flagged follow-on the registry
/// grows the optional coefficient fields for. The leading-order energy is exact for the standard state and
/// carries the correct sign of the temperature and pressure dependence. The per-phase energy uses the
/// fixed-point operators rather than the checked composition-layer arithmetic: it is overflow-safe for all
/// physical phase data (bounded enthalpy, entropy, and molar volume) across geological pressures, and the
/// assemblage-level sum that could otherwise grow with the composition is separately bounded by the unit
/// normalization in [`stable_assemblage`].
pub fn apparent_gibbs_energy(
    enthalpy_formation_kj_per_mol: Fixed,
    standard_entropy_j_per_mol_k: Fixed,
    molar_volume_cm3_per_mol: Fixed,
    temperature_k: Fixed,
    pressure_bar: Fixed,
) -> Fixed {
    // dH_f from kilojoules to joules per mole.
    let enthalpy_j = enthalpy_formation_kj_per_mol * Fixed::from_int(1000);
    // The entropy term T * S, in joules per mole.
    let entropy_term = temperature_k * standard_entropy_j_per_mol_k;
    // The pressure work V * (P - P_ref): cubic-centimetres per mole times bars is joules per mole through the
    // exact bridge 1 cm^3 bar = 0.1 J. Both factors and their product stay inside the Q32.32 range for
    // geological pressures because the pressure is carried in bars.
    let delta_p = pressure_bar - Fixed::from_int(REFERENCE_PRESSURE_BAR);
    let pressure_work = molar_volume_cm3_per_mol * delta_p * Fixed::from_ratio(1, 10);
    enthalpy_j - entropy_term + pressure_work
}

/// The apparent Gibbs free energy of a registry [`Phase`] at a temperature and pressure, in joules per mole,
/// reading the phase's standard-state thermodynamics ([`apparent_gibbs_energy`]). A thin accessor so a caller
/// works from a phase rather than its unpacked fields; the pressure is in bars and the temperature in kelvin.
pub fn phase_gibbs_energy(phase: &Phase, temperature_k: Fixed, pressure_bar: Fixed) -> Fixed {
    apparent_gibbs_energy(
        phase.enthalpy_formation,
        phase.standard_entropy,
        phase.molar_volume,
        temperature_k,
        pressure_bar,
    )
}

/// A stable mineral assemblage: the phases present and their molar amounts, plus the total apparent Gibbs
/// energy the minimization found, for a bulk composition at a pressure and temperature. The phases are in
/// canonical (registry name) order and carry positive amounts only, so the assemblage is a reproducible value.
#[derive(Debug, Clone, PartialEq)]
pub struct Assemblage {
    /// The present phases as `(name, amount per unit total composition)`, in canonical name order, amounts
    /// strictly positive. The composition is normalized to unit total before the solve, so these are relative
    /// amounts (a caller scales by the input total for absolute moles); the assemblage proportions and the
    /// derived density are what the consumers read.
    pub phases: Vec<(String, Fixed)>,
    /// The total apparent Gibbs energy of the assemblage per unit composition (joules), the quantity the
    /// minimization drove down.
    pub total_gibbs: Fixed,
    /// Whether the vertex enumeration was TRUNCATED by the fixed subset cap ([`MAX_SUBSETS_EXAMINED`]) before
    /// it finished. `false` means the returned assemblage is the exhaustively-searched global minimum; `true`
    /// means the search was capped and the result is the lowest-Gibbs assemblage found so far, which may not be
    /// the true optimum. The signal keeps the cap a STATED bound rather than a silent substitution of a
    /// non-derived assemblage (the no-silent-caps rule). For the seed registry it is always `false`.
    pub truncated: bool,
}

/// The maximum number of candidate-phase subsets the minimization examines, a fixed determinism-and-cost bound
/// (a fixed integer count, never a wall-clock or convergence gate): the vertex enumeration is exponential in
/// the candidate-phase count, so a world whose composition reaches many phases is bounded here and the lowest
/// Gibbs assemblage found within the budget is returned. For the seed registry the full enumeration is far
/// inside this cap. A bounded simplex is the scaling follow-on that removes the exponential.
const MAX_SUBSETS_EXAMINED: usize = 4096;

/// The RELATIVE amount (of the unit-normalized composition) below which a solved phase amount is accumulated
/// roundoff of the fixed-point normal-equations solve rather than a present phase, and the relative
/// element-balance residual below which the composition is reproduced. Because the composition is normalized to
/// unit total before the solve, this fixed value is a RELATIVE tolerance and so scale-invariant. Its basis is
/// the accumulated roundoff margin of the fixed-point Gaussian elimination (about four hundred thousand times
/// the Q32.32 resolution `2^-32`, a chosen accuracy margin, NOT the representation's resolution itself), an
/// engine-accuracy bound rather than a per-world value; the exact-rational or QR solve that would tighten it is
/// the flagged follow-on.
fn solve_tolerance() -> Fixed {
    Fixed::from_ratio(1, 10_000)
}

/// The pivot magnitude below which the normal-equations matrix is treated as singular for this subset (a
/// linearly dependent phase set, which is skipped rather than solved), near the fixed-point epsilon.
fn pivot_tolerance() -> Fixed {
    Fixed::from_ratio(1, 1_000_000_000)
}

/// Solve a small square linear system `a x = b` by fixed-point Gaussian elimination with partial pivoting
/// (largest-magnitude pivot, ties to the lowest row for determinism). Returns `None` if the system is singular
/// (a linearly dependent subset) or an intermediate overflows. Deterministic: the pivot choice and the
/// arithmetic are a pure function of the inputs, so the solve replays bit-for-bit.
fn solve_linear_system(mut a: Vec<Vec<Fixed>>, mut b: Vec<Fixed>) -> Option<Vec<Fixed>> {
    let n = b.len();
    let pivot_tol = pivot_tolerance();
    for col in 0..n {
        let mut pivot = col;
        for row in (col + 1)..n {
            if a[row][col].abs() > a[pivot][col].abs() {
                pivot = row;
            }
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        if a[col][col].abs() < pivot_tol {
            return None;
        }
        let diag = a[col][col];
        for row in (col + 1)..n {
            let factor = a[row][col].checked_div(diag)?;
            // Eliminate the pivot column from this row. The pivot row `a[col]` is read while `a[row]` is
            // written, so split the matrix to borrow the two rows disjointly (col < row always here); iterate
            // the columns at or after the pivot (the earlier columns are already zero).
            let (upper, lower) = a.split_at_mut(row);
            let pivot_row = &upper[col];
            let target_row = &mut lower[0];
            for (t, p) in target_row.iter_mut().zip(pivot_row.iter()).skip(col) {
                *t -= factor.checked_mul(*p)?;
            }
            let b_sub = factor.checked_mul(b[col])?;
            b[row] -= b_sub;
        }
    }
    let mut x = vec![Fixed::ZERO; n];
    for row in (0..n).rev() {
        let mut sum = b[row];
        for k in (row + 1)..n {
            sum -= a[row][k].checked_mul(x[k])?;
        }
        x[row] = sum.checked_div(a[row][row])?;
    }
    Some(x)
}

/// Advance `idx` to the next size-`k` combination of indices from `0..n` in lexicographic order, returning
/// `false` when the combinations are exhausted. The fixed enumeration order the minimization walks.
fn next_combination(idx: &mut [usize], n: usize) -> bool {
    let k = idx.len();
    let mut i = k;
    while i > 0 {
        i -= 1;
        if idx[i] < n - (k - i) {
            idx[i] += 1;
            for j in (i + 1)..k {
                idx[j] = idx[j - 1] + 1;
            }
            return true;
        }
    }
    false
}

/// The STABLE MINERAL ASSEMBLAGE a bulk composition reaches at a temperature and pressure, DERIVED by
/// minimizing the total apparent Gibbs energy over the candidate-phase registry subject to element mass
/// balance (Principle 8: the assemblage emerges from the thermodynamics and the world's own composition, never
/// an authored allocation order). This is the linear program `minimize sum(n_j G_j)` subject to
/// `sum(n_j composition_j) = budget` and `n_j >= 0`; its optimum is a vertex with at most (matrix rank) phases,
/// so the mechanism enumerates candidate-phase subsets in a fixed canonical order under a fixed cap, solves
/// each by the fixed-point normal equations (which handles the rank-deficient cases, for example
/// forsterite = 2 periclase + quartz making the Mg-Si-O system rank two), keeps the feasible consistent
/// vertices, and returns the lowest-Gibbs one.
///
/// `composition` is `(element symbol, molar amount)`; only positive amounts count, and only phases whose
/// formula uses elements present in the budget are candidates (a phase needing an absent element cannot form),
/// which is the admit-the-alien key: an alien chemistry reaches its own phases as data rows. The pressure is in
/// BARS and the temperature in kelvin. Returns `None` if the budget is empty, no candidate phase can form, or
/// no feasible assemblage is found within the enumeration cap.
///
/// DETERMINISTIC by construction (fixed enumeration order, fixed-point solve, first-found tie-break on equal
/// Gibbs), so it replays bit-for-bit. Honest limits: the fixed-point normal equations lose conditioning near
/// degeneracies (the exact-rational or QR solve the follow-on), and the vertex enumeration is exponential in
/// the candidate count (a bounded simplex the follow-on, and the returned assemblage carries a `truncated`
/// flag when the fixed cap bites so a capped non-optimal result is never silent). The assemblage is EXACT
/// balance or `None`: a composition the registry cannot exactly balance with non-negative phase amounts forms
/// no assemblage (there is no nearest-phase projection), the honest data-gap the registry grows to close by
/// adding the missing phase as a row.
pub fn stable_assemblage(
    composition: &[(String, Fixed)],
    temperature_k: Fixed,
    pressure_bar: Fixed,
    registry: &PhaseRegistry,
) -> Option<Assemblage> {
    // The present elements, in canonical sorted-symbol order, and the budget vector.
    let mut elements: Vec<(String, Fixed)> = composition
        .iter()
        .filter(|(_, amt)| *amt > Fixed::ZERO)
        .cloned()
        .collect();
    elements.sort_by(|a, b| a.0.cmp(&b.0));
    if elements.is_empty() {
        return None;
    }
    let elem_index: BTreeMap<&str, usize> = elements
        .iter()
        .enumerate()
        .map(|(i, (s, _))| (s.as_str(), i))
        .collect();
    let n_elem = elements.len();
    // Normalize the composition to unit total so the solve is SCALE-INVARIANT and BOUNDED: the stable
    // assemblage is a function of the RELATIVE composition and the conditions, never the arbitrary unit the
    // amounts are expressed in (so a fixed tolerance below is a relative one, the derive-vs-author fix), and
    // every phase amount stays at or below one so the Gibbs sum cannot overflow the fixed-point range (the
    // overflow fix). The returned amounts are therefore PER UNIT TOTAL COMPOSITION; a caller scales by the
    // input total for absolute moles, and the density (a ratio) is unchanged by the normalization.
    let total_amount = elements.iter().fold(Fixed::ZERO, |acc, (_, a)| acc + *a);
    if total_amount <= Fixed::ZERO {
        return None;
    }
    let mut budget: Vec<Fixed> = Vec::with_capacity(n_elem);
    for (_, a) in &elements {
        budget.push(a.checked_div(total_amount)?);
    }

    // The candidate phases: those whose formula uses only elements present in the budget.
    let candidates: Vec<&Phase> = registry
        .phases()
        .filter(|p| {
            p.composition
                .iter()
                .all(|(s, _)| elem_index.contains_key(s.as_str()))
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }

    let tol = solve_tolerance();
    let mut best: Option<Assemblage> = None;
    let mut examined = 0usize;
    let mut truncated = false;
    let max_size = n_elem.min(candidates.len());
    'sizes: for size in 1..=max_size {
        let mut idx: Vec<usize> = (0..size).collect();
        loop {
            if examined >= MAX_SUBSETS_EXAMINED {
                truncated = true;
                break 'sizes;
            }
            examined += 1;

            // The stoichiometry columns A (n_elem rows, `size` phase columns).
            let mut a = vec![vec![Fixed::ZERO; size]; n_elem];
            for (col, &ci) in idx.iter().enumerate() {
                for (sym, count) in &candidates[ci].composition {
                    let row = elem_index[sym.as_str()];
                    a[row][col] += Fixed::from_int(*count as i32);
                }
            }

            // The normal equations (A^T A) n = A^T b, solved for the phase amounts.
            let mut ata = vec![vec![Fixed::ZERO; size]; size];
            let mut atb = vec![Fixed::ZERO; size];
            let mut overflow = false;
            'assemble: for i in 0..size {
                for j in 0..size {
                    let mut s = Fixed::ZERO;
                    for a_row in a.iter() {
                        match a_row[i].checked_mul(a_row[j]) {
                            Some(v) => s += v,
                            None => {
                                overflow = true;
                                break 'assemble;
                            }
                        }
                    }
                    ata[i][j] = s;
                }
                let mut s = Fixed::ZERO;
                for (r, a_row) in a.iter().enumerate() {
                    match a_row[i].checked_mul(budget[r]) {
                        Some(v) => s += v,
                        None => {
                            overflow = true;
                            break 'assemble;
                        }
                    }
                }
                atb[i] = s;
            }

            if !overflow {
                if let Some(n) = solve_linear_system(ata, atb) {
                    // Feasibility: no phase amount is meaningfully negative.
                    let feasible = n.iter().all(|&x| x >= Fixed::ZERO - tol);
                    if feasible {
                        // Consistency: the amounts reproduce the element budget within tolerance.
                        let mut consistent = true;
                        for (r, a_row) in a.iter().enumerate() {
                            let mut lhs = Fixed::ZERO;
                            for (c, &nc) in n.iter().enumerate() {
                                match a_row[c].checked_mul(nc) {
                                    Some(v) => lhs += v,
                                    None => {
                                        consistent = false;
                                        break;
                                    }
                                }
                            }
                            if (lhs - budget[r]).abs() > tol {
                                consistent = false;
                            }
                            if !consistent {
                                break;
                            }
                        }
                        if consistent {
                            if let Some(cand) =
                                assemble(&idx, &candidates, &n, tol, temperature_k, pressure_bar)
                            {
                                best = Some(match best {
                                    Some(prev) if prev.total_gibbs <= cand.total_gibbs => prev,
                                    _ => cand,
                                });
                            }
                        }
                    }
                }
            }

            if !next_combination(&mut idx, candidates.len()) {
                break;
            }
        }
    }
    // Signal on the returned assemblage whether the enumeration was capped (the no-silent-caps rule): a capped
    // search may not have reached the global-minimum vertex.
    if let Some(ref mut a) = best {
        a.truncated = truncated;
    }
    best
}

/// Build an [`Assemblage`] from a solved subset: keep the phases whose amount clears the tolerance, sum the
/// total Gibbs energy, and sort the phases canonically. Returns `None` on an arithmetic overflow.
fn assemble(
    idx: &[usize],
    candidates: &[&Phase],
    n: &[Fixed],
    tol: Fixed,
    temperature_k: Fixed,
    pressure_bar: Fixed,
) -> Option<Assemblage> {
    let mut total = Fixed::ZERO;
    let mut phases = Vec::new();
    for (c, &ci) in idx.iter().enumerate() {
        let amt = n[c];
        if amt > tol {
            let g = phase_gibbs_energy(candidates[ci], temperature_k, pressure_bar);
            total += amt.checked_mul(g)?;
            phases.push((candidates[ci].name.clone(), amt));
        }
    }
    phases.sort_by(|a, b| a.0.cmp(&b.0));
    Some(Assemblage {
        phases,
        total_gibbs: total,
        // The caller (`stable_assemblage`) sets this on the final returned assemblage once it knows whether the
        // whole enumeration was capped; a per-subset assemblage is never itself truncated.
        truncated: false,
    })
}

/// The molar mass of a registry [`Phase`] in grams per mole, the sum of its elements' standard atomic weights
/// times their counts, read from the periodic table. Returns `None` if the formula names an element absent
/// from the table (the same fail-loud cross-check the registry validation uses).
pub fn phase_molar_mass(phase: &Phase, table: &PeriodicTable) -> Option<Fixed> {
    let mut mass = Fixed::ZERO;
    for (sym, count) in &phase.composition {
        let el = table.element(sym)?;
        mass += el
            .standard_atomic_weight
            .checked_mul(Fixed::from_int(*count as i32))?;
    }
    Some(mass)
}

/// The DENSITY of a stable assemblage in grams per cubic centimetre: the total mass over the total volume of
/// its phases (each phase's molar mass times its amount over its molar volume times its amount), the
/// pressure-temperature-dependent density the isostasy read consumes. Returns `None` if a phase is missing from
/// the registry or table, or the assemblage has no volume. The molar volume is the standard-state value (the
/// compressibility and thermal-expansion refinement the flagged follow-on), so this is the leading-order
/// density at the standard state carried to the assemblage's pressure only through which phases are stable.
pub fn assemblage_density(
    assemblage: &Assemblage,
    registry: &PhaseRegistry,
    table: &PeriodicTable,
) -> Option<Fixed> {
    let mut total_mass = Fixed::ZERO;
    let mut total_volume = Fixed::ZERO;
    for (name, amt) in &assemblage.phases {
        let phase = registry.phase(name)?;
        let mm = phase_molar_mass(phase, table)?;
        total_mass += amt.checked_mul(mm)?;
        total_volume += amt.checked_mul(phase.molar_volume)?;
    }
    if total_volume <= Fixed::ZERO {
        return None;
    }
    total_mass.checked_div(total_volume)
}

/// The DENSITY a bulk composition reaches at a temperature and pressure, in grams per cubic centimetre: the
/// density of the stable mineral assemblage the composition minimizes to ([`stable_assemblage`] then
/// [`assemblage_density`]). The name is neutral because the SAME bridge serves the CRUST and the MANTLE (and any
/// composition at its own conditions): the elevation a column floats at is set by its density and thickness
/// through the buoyancy law, so a dense mafic crust and a light felsic crust float at different heights from their
/// own chemistry alone, and the mantle density in the Airy contrast is this same read on the mantle composition,
/// never an authored per-rock-type density table (Principle 8). DERIVED end to end, the density falls out of the
/// composition, the conditions, and the data-defined registry. Returns `None` if the composition reaches no
/// assemblage (a data gap the registry grows to close) or a phase is missing from the table. Byte-neutral: no
/// consumer is wired yet; the isostasy reads this density across the `GeodynamicColumn` interface once armed.
pub fn assemblage_density_at_conditions(
    composition: &[(String, Fixed)],
    temperature_k: Fixed,
    pressure_bar: Fixed,
    registry: &PhaseRegistry,
    table: &PeriodicTable,
) -> Option<Fixed> {
    let assemblage = stable_assemblage(composition, temperature_k, pressure_bar, registry)?;
    assemblage_density(&assemblage, registry, table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::petrology_data::PhaseRegistry;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_standard_state_gibbs_energy_is_enthalpy_minus_the_entropy_term() {
        // At the reference pressure (1 bar) the pressure work vanishes, so G reduces to dH_f - T * S. For
        // quartz (dH_f = -910.70 kJ/mol, S = 41.46 J/mol/K) at the standard temperature 298.15 K this is
        // -910700 - 298.15 * 41.46 = -923060.9 J/mol, the hand computation the kernel must reproduce.
        let t = reference_temperature_k();
        let p_ref = Fixed::from_int(REFERENCE_PRESSURE_BAR);
        let g = apparent_gibbs_energy(
            Fixed::from_ratio(-91070, 100), // -910.70 kJ/mol
            Fixed::from_ratio(4146, 100),   // 41.46 J/mol/K
            Fixed::from_ratio(22688, 1000), // 22.688 cm^3/mol
            t,
            p_ref,
        );
        assert!(
            close(g, -910700.0 - 298.15 * 41.46, 1.0),
            "standard-state G reduces to dH_f - T*S with no pressure work"
        );
    }

    #[test]
    fn temperature_lowers_the_gibbs_energy_through_the_entropy_term() {
        // Raising temperature at fixed pressure lowers G (the -T*S term), and by more for the higher-entropy
        // phase: the thermodynamic reason a hotter world favours the higher-entropy assemblage.
        let p = Fixed::from_int(1);
        let low_s = Fixed::from_ratio(4146, 100); // quartz-like, S = 41.46
        let high_s = Fixed::from_int(100); // a higher-entropy phase
        let h = Fixed::from_ratio(-91070, 100);
        let v = Fixed::from_ratio(22688, 1000);
        let g_cold_low = apparent_gibbs_energy(h, low_s, v, Fixed::from_int(300), p);
        let g_hot_low = apparent_gibbs_energy(h, low_s, v, Fixed::from_int(1300), p);
        let g_cold_high = apparent_gibbs_energy(h, high_s, v, Fixed::from_int(300), p);
        let g_hot_high = apparent_gibbs_energy(h, high_s, v, Fixed::from_int(1300), p);
        assert!(g_hot_low < g_cold_low, "heating lowers G through -T*S");
        assert!(
            (g_cold_high - g_hot_high) > (g_cold_low - g_hot_low),
            "the higher-entropy phase drops faster with temperature"
        );
    }

    #[test]
    fn pressure_raises_the_gibbs_energy_through_the_volume_term() {
        // At fixed temperature, raising pressure raises G (the +V*(P-P_ref) work), and by more for the
        // larger-volume phase: the thermodynamic reason depth favours the denser, lower-volume assemblage.
        let t = Fixed::from_int(1000);
        let h = Fixed::from_ratio(-91070, 100);
        let s = Fixed::from_ratio(4146, 100);
        let small_v = Fixed::from_int(11); // a dense phase
        let large_v = Fixed::from_int(44); // an open phase
                                           // One kilobar (1000 bar) of pressure work.
        let g_low_small = apparent_gibbs_energy(h, s, small_v, t, Fixed::from_int(1));
        let g_high_small = apparent_gibbs_energy(h, s, small_v, t, Fixed::from_int(1000));
        let g_low_large = apparent_gibbs_energy(h, s, large_v, t, Fixed::from_int(1));
        let g_high_large = apparent_gibbs_energy(h, s, large_v, t, Fixed::from_int(1000));
        assert!(
            g_high_small > g_low_small,
            "compression raises G through +V*dP"
        );
        assert!(
            (g_high_large - g_low_large) > (g_high_small - g_low_small),
            "the larger-volume phase rises faster with pressure"
        );
        // The pressure work is exact: 44 cm^3 over 999 bar is 44 * 999 * 0.1 = 4395.6 J/mol.
        assert!(
            close(g_high_large - g_low_large, 44.0 * 999.0 * 0.1, 1.0),
            "the pressure work is V * dP * 0.1 J per cm^3 bar"
        );
    }

    #[test]
    fn the_registry_phases_read_a_negative_gibbs_energy_at_the_surface() {
        // Every seeded phase is a stable oxide or silicate (a negative enthalpy of formation), so its apparent
        // Gibbs energy at a warm surface is negative, and the kernel reads each phase from the registry.
        let r = PhaseRegistry::standard().expect("the embedded phase registry loads");
        let t = Fixed::from_int(300);
        let p = Fixed::from_int(1);
        for phase in r.phases() {
            let g = phase_gibbs_energy(phase, t, p);
            assert!(
                g.to_f64_lossy() < 0.0,
                "phase {} reads a negative apparent Gibbs energy at the surface",
                phase.name
            );
        }
    }

    fn el(sym: &str, amount: i64) -> (String, Fixed) {
        (sym.to_string(), Fixed::from_int(amount as i32))
    }

    fn phase_names(a: &Assemblage) -> Vec<String> {
        a.phases.iter().map(|(n, _)| n.clone()).collect()
    }

    #[test]
    fn a_forsterite_composition_minimizes_to_forsterite_not_periclase_plus_quartz() {
        // The Mg2SiO4 budget can be satisfied two ways from the seed: pure forsterite, or 2 periclase + quartz
        // (the rank-deficient alternative, since forsterite = 2 periclase + quartz stoichiometrically). The
        // free-energy minimization must pick forsterite, which is the lower-Gibbs assemblage (its enthalpy of
        // formation, -2173, is below the -2113.9 of 2 periclase + quartz). This is the derive-clean emergence:
        // the stable assemblage falls out of minimizing G, not an authored rule.
        let r = PhaseRegistry::standard().expect("registry loads");
        let comp = vec![el("Mg", 2), el("Si", 1), el("O", 4)];
        let a = stable_assemblage(&comp, Fixed::from_int(300), Fixed::from_int(1), &r)
            .expect("a bulk-silicate budget forms an assemblage");
        assert_eq!(
            phase_names(&a),
            vec!["forsterite".to_string()],
            "forsterite wins the free-energy minimization over periclase + quartz"
        );
        // The amount is per UNIT TOTAL composition: the Mg2SiO4 budget is seven atom-moles normalized to a
        // unit total, so the single forsterite phase (seven atoms per formula unit) carries one seventh.
        assert!(
            (a.phases[0].1.to_f64_lossy() - 1.0 / 7.0).abs() < 1e-3,
            "one seventh of a formula unit of forsterite per unit total composition"
        );
        assert!(
            !a.truncated,
            "the seed enumeration is exhaustive, not capped"
        );
    }

    #[test]
    fn a_silica_excess_composition_minimizes_to_forsterite_plus_quartz() {
        // Mg2Si2O6 is forsterite plus quartz in the seed registry (which carries no enstatite MgSiO3, so the
        // assemblage is only as complete as the data): the minimization lands fo + qz over the higher-Gibbs
        // 2 quartz + 2 periclase alternative.
        let r = PhaseRegistry::standard().expect("registry loads");
        let comp = vec![el("Mg", 2), el("Si", 2), el("O", 6)];
        let a = stable_assemblage(&comp, Fixed::from_int(300), Fixed::from_int(1), &r)
            .expect("the budget forms an assemblage");
        assert_eq!(
            phase_names(&a),
            vec!["forsterite".to_string(), "quartz".to_string()],
            "silica-saturated bulk-silicate lands forsterite + quartz"
        );
    }

    #[test]
    fn a_pure_silica_composition_minimizes_to_quartz() {
        let r = PhaseRegistry::standard().expect("registry loads");
        let comp = vec![el("Si", 1), el("O", 2)];
        let a = stable_assemblage(&comp, Fixed::from_int(300), Fixed::from_int(1), &r)
            .expect("silica forms an assemblage");
        assert_eq!(phase_names(&a), vec!["quartz".to_string()]);
    }

    #[test]
    fn the_assemblage_is_deterministic() {
        // The same composition and conditions yield the bit-identical assemblage on repeat (fixed enumeration,
        // fixed-point solve, first-found tie-break), the determinism the canonical path requires.
        let r = PhaseRegistry::standard().expect("registry loads");
        let comp = vec![el("Mg", 2), el("Si", 2), el("O", 6)];
        let a = stable_assemblage(&comp, Fixed::from_int(500), Fixed::from_int(1000), &r).unwrap();
        let b = stable_assemblage(&comp, Fixed::from_int(500), Fixed::from_int(1000), &r).unwrap();
        assert_eq!(a, b, "the minimization replays bit-for-bit");
    }

    #[test]
    fn a_composition_of_only_an_unreachable_element_forms_no_assemblage() {
        // Admit-the-alien and fail-loud: a budget of pure oxygen (no seed phase is a pure-oxygen solid) reaches
        // no candidate phase, so no assemblage forms rather than a fabricated one. An alien chemistry that a
        // world's registry does not yet carry is a data gap surfaced, not an authored fallback.
        let r = PhaseRegistry::standard().expect("registry loads");
        let comp = vec![el("O", 4)];
        assert!(
            stable_assemblage(&comp, Fixed::from_int(300), Fixed::from_int(1), &r).is_none(),
            "no candidate phase forms from oxygen alone"
        );
    }

    #[test]
    fn the_assemblage_density_computes_mass_over_volume_and_discriminates_phases() {
        // assemblage_density is the total mass over the total volume of the phases, from the periodic table
        // (atomic weights) and the registry (molar volumes), and it is scale-invariant so the normalization
        // does not touch it. For pure forsterite it is the hand computation 140.69 g/mol over 43.79 cm^3/mol.
        // This checks the COMPUTATION, not the physics: 3.213 is the registry's own mass over volume, so
        // validating it against a measured density would be circular. As a physical aside (not what the test
        // asserts): 3.21 g/cm^3 is the correct room-condition density of PURE forsterite (measured near 3.22);
        // real mantle olivine near 3.27 is denser because it is Fe-bearing, a COMPOSITION difference (add the
        // fayalite end-member), not a pressure correction (which is zero at the 1 bar tested). Density
        // discriminates the assemblage, the property the isostasy read leans on: a pure-quartz assemblage reads
        // lower.
        let r = PhaseRegistry::standard().expect("registry loads");
        let t = PeriodicTable::standard().expect("table loads");
        let fo = stable_assemblage(
            &[el("Mg", 2), el("Si", 1), el("O", 4)],
            Fixed::from_int(300),
            Fixed::from_int(1),
            &r,
        )
        .unwrap();
        let d_fo = assemblage_density(&fo, &r, &t).expect("forsterite has a density");
        assert!(
            close(d_fo, 140.6915 / 43.790, 0.02),
            "the density is the registry mass over volume, got {}",
            d_fo.to_f64_lossy()
        );
        let qz = stable_assemblage(
            &[el("Si", 1), el("O", 2)],
            Fixed::from_int(300),
            Fixed::from_int(1),
            &r,
        )
        .unwrap();
        let d_qz = assemblage_density(&qz, &r, &t).unwrap();
        assert!(
            d_qz < d_fo,
            "quartz reads less dense than forsterite, so density discriminates the assemblage"
        );
    }

    #[test]
    fn pressure_shifts_the_assemblage_between_two_polymorphs() {
        // The assemblage EMERGES with the conditions, not only from the per-phase energy: given two polymorphs
        // of one composition (an open low-pressure one and a dense high-pressure one), the minimization selects
        // the open phase at low pressure and the dense phase at high pressure, purely from the V*(P-P_ref)
        // work. The seed registry carries no polymorph pair, so it cannot show this; a synthetic two-polymorph
        // registry demonstrates the pressure sensitivity of the selection (the depth-sequence mechanism).
        let toml = r#"
[[phase]]
name = "open_polymorph"
formula = "SiO2"
enthalpy_formation = "-1000.0"
standard_entropy = "40.0"
molar_volume = "25.0"
source = "synthetic test: the open, low-pressure polymorph"

[[phase]]
name = "dense_polymorph"
formula = "SiO2"
enthalpy_formation = "-999.9"
standard_entropy = "40.0"
molar_volume = "20.0"
source = "synthetic test: the dense, high-pressure polymorph"
"#;
        let r = PhaseRegistry::from_toml_str(toml).expect("the synthetic polymorph registry loads");
        let comp = vec![el("Si", 1), el("O", 2)];
        let low_p = stable_assemblage(&comp, Fixed::from_int(300), Fixed::from_int(1), &r).unwrap();
        let high_p =
            stable_assemblage(&comp, Fixed::from_int(300), Fixed::from_int(10_000), &r).unwrap();
        assert_eq!(
            phase_names(&low_p),
            vec!["open_polymorph".to_string()],
            "the open phase wins the minimization at low pressure"
        );
        assert_eq!(
            phase_names(&high_p),
            vec!["dense_polymorph".to_string()],
            "the dense phase wins the minimization at high pressure through the volume work"
        );
    }

    #[test]
    fn the_element_input_order_does_not_change_the_assemblage() {
        // The composition is sorted to a canonical element order internally, so the same composition passed
        // with its elements in a different input order yields the identical assemblage: a determinism guard
        // that the caller's ordering never leaks into the world-content result.
        let r = PhaseRegistry::standard().expect("registry loads");
        let forward = vec![el("Mg", 2), el("Si", 2), el("O", 6)];
        let shuffled = vec![el("O", 6), el("Mg", 2), el("Si", 2)];
        let a = stable_assemblage(&forward, Fixed::from_int(400), Fixed::from_int(50), &r).unwrap();
        let b =
            stable_assemblage(&shuffled, Fixed::from_int(400), Fixed::from_int(50), &r).unwrap();
        assert_eq!(
            a, b,
            "the assemblage is independent of the element input order"
        );
    }

    #[test]
    fn the_assemblage_is_scale_invariant_and_large_compositions_do_not_overflow() {
        // The two hardened seams in one test. Derive-vs-author: the same RELATIVE composition at two very
        // different absolute scales yields the identical assemblage, because the normalization makes the result
        // a function of the relative composition and the conditions, not the arbitrary unit it is expressed in.
        // Correctness/overflow: the large absolute composition (thousands of moles) would, before the
        // normalization, have driven a phase amount times its ~2.2e6 J Gibbs energy past the Q32.32 ceiling and
        // silently dropped the assemblage; normalized, it solves cleanly and matches the small-scale result.
        let r = PhaseRegistry::standard().expect("registry loads");
        let small = vec![el("Mg", 2), el("Si", 1), el("O", 4)];
        let large = vec![el("Mg", 2000), el("Si", 1000), el("O", 4000)];
        let a = stable_assemblage(&small, Fixed::from_int(300), Fixed::from_int(1), &r).unwrap();
        let b = stable_assemblage(&large, Fixed::from_int(300), Fixed::from_int(1), &r)
            .expect("a thousand-fold larger composition still solves (no overflow)");
        assert_eq!(
            a, b,
            "the assemblage is invariant to the composition's absolute scale"
        );
        assert_eq!(phase_names(&a), vec!["forsterite".to_string()]);
    }

    #[test]
    fn the_assemblage_density_derives_from_chemistry_so_mafic_is_denser_than_felsic() {
        // The composition-to-density bridge the isostasy leans on: a mafic (forsterite, Mg-rich) crust derives
        // a higher density than a felsic (silica-rich) crust, purely from the stable assemblage each
        // composition minimizes to, so the two float at different isostatic heights from their own chemistry
        // alone, never an authored per-rock-type density.
        let r = PhaseRegistry::standard().expect("registry loads");
        let t = PeriodicTable::standard().expect("table loads");
        let mafic = assemblage_density_at_conditions(
            &[el("Mg", 2), el("Si", 1), el("O", 4)],
            Fixed::from_int(300),
            Fixed::from_int(1),
            &r,
            &t,
        )
        .expect("the mafic composition derives a density");
        let felsic = assemblage_density_at_conditions(
            &[el("Si", 1), el("O", 2)],
            Fixed::from_int(300),
            Fixed::from_int(1),
            &r,
            &t,
        )
        .expect("the felsic composition derives a density");
        assert!(
            mafic.to_f64_lossy() > 3.0,
            "the mafic (forsterite) crust is dense, got {}",
            mafic.to_f64_lossy()
        );
        assert!(
            felsic.to_f64_lossy() < 2.8,
            "the felsic (silica) crust is light, got {}",
            felsic.to_f64_lossy()
        );
        assert!(
            mafic > felsic,
            "mafic crust derives a higher density than felsic, so it floats lower"
        );
    }
}
