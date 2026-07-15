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

//! Stage 8, the atmospheric gas-phase COMPOSITION (the chemical core of the coupled-solve assembly): given the
//! atmospheric volatile-element budget (the moles of C, N, O, H, S the outgassing places in the gas phase) and the
//! temperature at which that gas last equilibrated, DERIVE the equilibrium gas mix (the mole fractions of H2O, CO2,
//! CO, CH4, NH3, N2, H2S, S2, H2, O2) by the same element-potential Gibbs minimization the condensation uses
//! ([`crate::equilibrium_condensation::gas_equilibrium`]), now over the C-N-O-H-S gas species. The mix is the
//! render's Rayleigh sky-colour input and the Hadean gate's atmosphere row.
//!
//! The gas set is DATA, not an authored list: it is every JANAF gas-phase species whose atoms all lie within the
//! budget elements. That filter selects the C-N-O-H-S volatiles and drops the refractory vapours (SiO, TiO, Fe, Mg,
//! Na, K, SiS) on their own, so a new volatile is a JANAF row, not a code change (admit the alien, Principle 11).
//! The mechanism is fixed Rust; the membership grows with the tables.
//!
//! Honest limits, stated plainly:
//!   1. This is the gas-phase CHEMICAL equilibrium of a GIVEN atmospheric budget. Where each volatile actually sits
//!      (the CO2 in the air against the carbonate crust and the dissolved ocean, the H2O in vapour against the
//!      condensed ocean) is the atmosphere-ocean-crust PARTITIONING, the load-bearing rest of Stage 8 (item #40).
//!      Feed this the atmospheric budget and it returns the speciation; it does not decide the partition.
//!   2. Equilibrium is physical only at the temperature where the gas last equilibrated. A real atmosphere is
//!      kinetically FROZEN at its cold surface (N2 plus H2 does not relax to NH3 at 288 K), so the meaningful
//!      equilibration temperature is the volcanic QUENCH temperature (the ~1200 to 1500 K at which outgassed gas
//!      leaves the magma), not the surface temperature. The caller supplies it; a hot Venusian atmosphere near
//!      surface equilibrium would pass its own surface temperature instead.
//!   3. The speciation is a strong function of the redox state, which the budget's O:H:C ratio carries (an
//!      O-rich budget gives oxidized H2O/CO2/N2, an O-poor one gives reduced CO/H2/CH4/NH3). The budget itself,
//!      hence the redox, is set by the outgassing and the mantle oxygen buffer, the pending upstream of item #40.

use crate::equilibrium_condensation::{
    gas_equilibrium, janaf_g_over_rt, EquilibriumSpecies, SpeciesPhase,
};
use civsim_core::Fixed;
use civsim_physics::janaf::JanafTables;
use std::collections::BTreeMap;

/// Parse a chemical formula (the JANAF name with its phase suffix already stripped) into its element counts. An
/// uppercase letter opens an element symbol, trailing lowercase letters extend it (so "Si" and "Mg" parse whole),
/// trailing digits are the count (absent means one). General over any formula, so an unfamiliar volatile is parsed,
/// not special-cased. `None` on a malformed formula (a leading non-uppercase or an unparseable count).
fn parse_formula(formula: &str) -> Option<BTreeMap<String, i32>> {
    let chars: Vec<char> = formula.chars().collect();
    let mut atoms: BTreeMap<String, i32> = BTreeMap::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_uppercase() {
            return None;
        }
        let mut symbol = String::new();
        symbol.push(chars[i]);
        i += 1;
        while i < chars.len() && chars[i].is_ascii_lowercase() {
            symbol.push(chars[i]);
            i += 1;
        }
        let mut digits = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() {
            digits.push(chars[i]);
            i += 1;
        }
        let count: i32 = if digits.is_empty() {
            1
        } else {
            digits.parse().ok()?
        };
        *atoms.entry(symbol).or_insert(0) += count;
    }
    if atoms.is_empty() {
        None
    } else {
        Some(atoms)
    }
}

/// True if `name` is a gas-phase JANAF species: an ideal gas `(g)` or a reference-state element `(ref)` (the
/// reference elements N2, O2, H2 carry a formation Gibbs energy of zero and are legitimate gas members). Condensed
/// tags `(cr,...)` and `(l)` are excluded.
fn is_gas_phase(name: &str) -> bool {
    name.contains("(g)") || name.contains("(ref)")
}

/// Derive the atmospheric gas mix from the volatile budget and the equilibration temperature. `budget` maps each
/// volatile element symbol (for example "C", "N", "O", "H", "S") to its moles in the gas phase; the gas set is every
/// JANAF gas-phase species whose atoms all lie within those elements. Returns the equilibrium mix as (species name,
/// mole fraction) sorted descending by fraction (deterministic, ties broken by name). `None` if no gas qualifies, a
/// JANAF read fails, the solve does not converge, or the amounts sum to zero.
///
/// `equilibration_temperature_k` is the temperature at which the gas last reached equilibrium: the volcanic quench
/// temperature for a quenched outgassed atmosphere (its basis, ~1200 to 1500 K, the magma-degassing range), or a
/// hot planet's surface temperature where the atmosphere sits near surface equilibrium. It is a reserved input with
/// that basis, never fabricated here.
pub fn atmosphere_gas_equilibrium(
    janaf: &JanafTables,
    budget: &BTreeMap<String, Fixed>,
    equilibration_temperature_k: Fixed,
) -> Option<Vec<(String, Fixed)>> {
    if budget.is_empty() {
        return None;
    }
    // The gas set emerges from the tables: every gas-phase species whose atoms are all budget elements. Collected in
    // a deterministic, name-sorted order so the solve's input order is reproducible.
    let mut names: Vec<String> = janaf.names().map(|s| s.to_string()).collect();
    names.sort();
    let mut species: Vec<EquilibriumSpecies> = Vec::new();
    for name in &names {
        if !is_gas_phase(name) {
            continue;
        }
        let formula = name.split('(').next()?;
        let atoms = match parse_formula(formula) {
            Some(a) => a,
            None => continue,
        };
        if !atoms.keys().all(|el| budget.contains_key(el)) {
            continue;
        }
        let table = janaf.species(name)?;
        let delta_f_g = table.delta_f_g_at(equilibration_temperature_k)?;
        let g_over_rt = janaf_g_over_rt(delta_f_g, equilibration_temperature_k)?;
        species.push(EquilibriumSpecies {
            name: name.clone(),
            phase: SpeciesPhase::Gas,
            g_over_rt,
            stoichiometry: atoms,
        });
    }
    if species.is_empty() {
        return None;
    }
    let equilibrium = gas_equilibrium(&species, budget)?;
    let mut total = Fixed::ZERO;
    for (_, amount) in &equilibrium.species_amounts {
        total = total.checked_add(*amount)?;
    }
    if total <= Fixed::ZERO {
        return None;
    }
    let mut mix: Vec<(String, Fixed)> = Vec::with_capacity(equilibrium.species_amounts.len());
    for (name, amount) in &equilibrium.species_amounts {
        mix.push((name.clone(), amount.checked_div(total)?));
    }
    // Descending by mole fraction, ties by name: the same deterministic ordering the condensed active set uses.
    mix.sort_by(|a, b| b.1.to_bits().cmp(&a.1.to_bits()).then(a.0.cmp(&b.0)));
    Some(mix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(entries: &[(&str, i64)]) -> BTreeMap<String, Fixed> {
        entries
            .iter()
            .map(|(el, n)| (el.to_string(), Fixed::from_int(*n as i32)))
            .collect()
    }

    fn fraction_of(mix: &[(String, Fixed)], name: &str) -> f64 {
        mix.iter()
            .find(|(n, _)| n == name)
            .map(|(_, f)| f.to_f64_lossy())
            .unwrap_or(0.0)
    }

    #[test]
    fn the_formula_parser_splits_multi_letter_symbols_and_counts() {
        assert_eq!(
            parse_formula("H2O").unwrap(),
            BTreeMap::from([("H".to_string(), 2), ("O".to_string(), 1)])
        );
        assert_eq!(
            parse_formula("SiO").unwrap(),
            BTreeMap::from([("Si".to_string(), 1), ("O".to_string(), 1)])
        );
        assert_eq!(
            parse_formula("NH3").unwrap(),
            BTreeMap::from([("N".to_string(), 1), ("H".to_string(), 3)])
        );
    }

    #[test]
    fn an_oxidized_volcanic_budget_is_water_and_carbon_dioxide_dominated() {
        // A volcanic outgassing budget with enough oxygen to burn the hydrogen to water and the carbon to carbon
        // dioxide (H2O needs 80 O for 160 H, CO2 needs 20 O for 10 C, so O ~ 100 is the oxidized balance). At the
        // quench temperature the equilibrium should be H2O-dominated with CO2 second and N2 present, the recognizable
        // oxidized-volcanic (and modern-volcanic) speciation.
        let janaf = JanafTables::standard().expect("JANAF loads");
        let b = budget(&[("H", 160), ("O", 100), ("C", 10), ("N", 4), ("S", 3)]);
        let mix = atmosphere_gas_equilibrium(&janaf, &b, Fixed::from_int(1400))
            .expect("the oxidized budget solves");
        let h2o = fraction_of(&mix, "H2O(g)");
        let co2 = fraction_of(&mix, "CO2(g)");
        let n2 = fraction_of(&mix, "N2(ref)");
        assert!(
            h2o > co2 && co2 > 0.0 && n2 > 0.0,
            "oxidized volcanic mix is H2O-dominated with CO2 and N2 present, got H2O={h2o:.3} CO2={co2:.3} N2={n2:.3}"
        );
        assert!(
            h2o + co2 + n2 > 0.85,
            "the three oxidized gases dominate the mix, got sum {:.3}",
            h2o + co2 + n2
        );
    }

    #[test]
    fn removing_oxygen_shifts_the_carbon_from_dioxide_toward_reduced_species() {
        // The redox response: hold the H, C, N, S budget and remove oxygen. With less oxygen the carbon can no longer
        // all reach CO2; the equilibrium shifts carbon toward the reduced CO (and CH4), the qualitative reduced-mantle
        // signature. The CO2 fraction must fall when oxygen is scarce relative to the oxidized case.
        let janaf = JanafTables::standard().expect("JANAF loads");
        let oxidized = atmosphere_gas_equilibrium(
            &janaf,
            &budget(&[("H", 160), ("O", 100), ("C", 10), ("N", 4), ("S", 3)]),
            Fixed::from_int(1400),
        )
        .expect("oxidized solves");
        let reduced = atmosphere_gas_equilibrium(
            &janaf,
            &budget(&[("H", 160), ("O", 70), ("C", 10), ("N", 4), ("S", 3)]),
            Fixed::from_int(1400),
        )
        .expect("reduced solves");
        let co2_ox = fraction_of(&oxidized, "CO2(g)");
        let co2_red = fraction_of(&reduced, "CO2(g)");
        let co_red = fraction_of(&reduced, "CO(g)");
        assert!(
            co2_red < co2_ox,
            "removing oxygen lowers the CO2 fraction, oxidized {co2_ox:.3} vs reduced {co2_red:.3}"
        );
        assert!(
            co_red > 0.0,
            "the reduced budget populates CO, got {co_red:.3}"
        );
    }

    #[test]
    fn an_empty_budget_yields_no_atmosphere() {
        let janaf = JanafTables::standard().expect("JANAF loads");
        assert!(
            atmosphere_gas_equilibrium(&janaf, &BTreeMap::new(), Fixed::from_int(1400)).is_none()
        );
    }
}
