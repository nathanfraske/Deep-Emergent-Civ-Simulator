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

//! THE CANON SURFACE-COMPOSITION CHAIN: star-and-orbit in, the crust the physics produces out. The disk temperature
//! at the orbit sets the CONDENSATION (which solids precipitate from the cooling gas at solar abundances), the VCS
//! AMOUNT REDISTRIBUTION fixes how much of each, and DIFFERENTIATION floats the crust off the sinking metal and
//! sulfide. This is the wire that retires the authored composition arrangement `slice0_demo_field` stood in for: the
//! surface elements the tile chain reads are now derived end to end, condensation -> amounts -> differentiation.
//!
//! The species set EMERGES from the data, not an authored list (as the atmosphere set does): the element budget is
//! every element that has a JANAF gas species (so the gas equilibrium can balance it), and the candidate phases are
//! every JANAF species whose atoms lie within that budget. Today that is the full solar rock-forming set (H, C, N, O,
//! Na, Mg, Al, Si, S, K, Ca, Ti, Fe): the Mg-silicates forsterite and enstatite, the aluminium and calcium
//! refractories corundum and spinel, and iron metal and troilite. A refractory whose element has no gas species yet
//! (nickel metal, for one) is OUT of the balanceable set until its gas species lands, at which point it joins as a
//! data row, never a code change. So the derived crust is the refractory assemblage the current data can balance,
//! deepening toward the full CAI-first sequence as the gas-species fetches extend the budget. This is a data ceiling,
//! named, not a hidden simplification.
//!
//! Abundances are the cited solar photosphere (`SolarAbundances`), the astronomical `log_eps` scale converted to
//! linear amounts normalized to hydrogen (`n_X / n_H = 10^(log_eps(X) - 12)`), so the gas is hydrogen-dominated and
//! the rock is the trace that condenses, exactly the protoplanetary setting.

use crate::differentiation::{
    crust_and_mantle, differentiate, phase_set_composition, DifferentiatedPlanet,
};
use crate::equilibrium_condensation::{
    condensed_active_set, condensed_amounts, gas_equilibrium, janaf_g_over_rt, CondensedAmounts,
    EquilibriumSpecies, SpeciesPhase,
};
use civsim_core::Fixed;
use civsim_physics::janaf::JanafTables;
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::petrology::crustal_density;
use civsim_physics::petrology_data::PhaseRegistry;
use civsim_physics::solar_abundances::SolarAbundances;
use std::collections::{BTreeMap, BTreeSet};

/// The derived surface composition and the differentiation it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceComposition {
    /// The SURFACE (crust) element amounts (element symbol, amount), the input the derived-tile chain reads: the
    /// ASSEMBLAGE the buoyant partial-melt crust forms (seam 5), each element at the sum over the crust phases of
    /// the phase's modal amount times its stoichiometric count, so it is the rock (enstatite `Mg:Si:O = 1:1:3`) and
    /// not the oxygen-heavy solar element budget it condensed from.
    pub surface: Vec<(String, Fixed)>,
    /// The MANTLE (refractory residue) element amounts, the same assemblage composition as `surface` but for the
    /// dense phases the crust floats on: the silicate density the isostasy and the bulk-density derivation read.
    pub mantle_composition: Vec<(String, Fixed)>,
    /// The crust phases (the buoyant partial melt) and the mantle phases (the refractory residue) from the
    /// partial-melt split, for the isostasy the tile relief reads (crust floats on mantle).
    pub crust: Vec<(String, Fixed)>,
    pub mantle: Vec<(String, Fixed)>,
    /// The full differentiation (the sinking metal-sulfide and floating silicate fractions), for audit.
    pub differentiation: DifferentiatedPlanet,
    /// The condensed molar amounts from the VCS, when the vertex was well-posed; `None` at a degenerate vertex (the
    /// identity still derives, the amounts route to the Verdict draw).
    pub condensed_amounts: Option<Vec<(String, Fixed)>>,
}

/// Parse a JANAF species name (formula before the phase suffix) into element counts.
fn species_elements(name: &str) -> Option<BTreeMap<String, i32>> {
    let formula = name.split('(').next()?;
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

fn is_gas_phase(name: &str) -> bool {
    name.contains("(g)") || name.contains("(ref)")
}

/// The natural log of ten, for the `log_eps` (base-10) to natural-exponent conversion.
fn ln_ten() -> Fixed {
    Fixed::from_ratio(2_302_585, 1_000_000)
}

/// Derive the surface composition at an orbit from the disk temperature there. Runs the condensation of the solar
/// gas at `disk_temperature_k`, the VCS amount redistribution, and the differentiation, returning the crust the
/// tile chain reads. `None` if the JANAF read fails, no element is gas-balanceable, the equilibrium does not solve,
/// or nothing floats (a world with no oxygen-bearing condensate has no derived crust, fail-loud).
pub fn derive_surface_composition(
    janaf: &JanafTables,
    abundances: &SolarAbundances,
    disk_temperature_k: Fixed,
) -> Option<SurfaceComposition> {
    if disk_temperature_k <= Fixed::ZERO {
        return None;
    }
    // The gas-balanceable element budget: every element that appears in a JANAF GAS species (so the gas equilibrium
    // can balance it) and has a cited solar abundance. Emergent from the data, deterministic (sorted) order.
    let mut gas_elements: BTreeSet<String> = BTreeSet::new();
    for name in janaf.names() {
        if is_gas_phase(name) {
            if let Some(atoms) = species_elements(name) {
                for el in atoms.keys() {
                    gas_elements.insert(el.clone());
                }
            }
        }
    }
    // The MINIMIZER CONVERGENCE CEILING is retired: the element-potential solve now converges on the full solar
    // element set including aluminium and calcium (13 elements, 26 gas species), whose ~6-decade-below-hydrogen gas
    // abundances made the undamped fixed-point Newton overshoot the trace rows and diverge. The solve keys the
    // iteration budget on the seed conditioning and drives the ill-conditioned wide-span set with an exact-rational
    // damped Newton (see `gas_equilibrium`); aluminium and calcium condense as the refractory corundum and spinel, so
    // the derived crust deepens toward the full CAI-first sequence. The hold-out set is now empty (kept named so a
    // future element that is not yet balanceable can be re-added as data, never a code change).
    const MINIMIZER_UNCONVERGED: &[&str] = &[];
    // The element budget b_e from the solar abundances, normalized to hydrogen: n_X/n_H = 10^(log_eps(X) - 12).
    let mut budget: BTreeMap<String, Fixed> = BTreeMap::new();
    for el in &gas_elements {
        if MINIMIZER_UNCONVERGED.contains(&el.as_str()) {
            continue;
        }
        if let Some(log_eps) = abundances.preferred(el) {
            let exponent = log_eps
                .checked_sub(Fixed::from_int(12))?
                .checked_mul(ln_ten())?;
            let amount = exponent.exp();
            if amount > Fixed::ZERO {
                budget.insert(el.clone(), amount);
            }
        }
    }
    if budget.is_empty() {
        return None;
    }
    // The candidate species: every JANAF species whose atoms lie within the budget (gas and condensed alike). The
    // gas set balances the elements; the condensed set is what can precipitate.
    let mut gas: Vec<EquilibriumSpecies> = Vec::new();
    let mut condensed: Vec<EquilibriumSpecies> = Vec::new();
    let mut names: Vec<String> = janaf.names().map(|s| s.to_string()).collect();
    names.sort();
    for name in &names {
        let atoms = match species_elements(name) {
            Some(a) => a,
            None => continue,
        };
        if !atoms.keys().all(|el| budget.contains_key(el)) {
            continue;
        }
        // A species whose JANAF table has no datum at this temperature is simply not a candidate here (skip it),
        // never a reason to abort the whole derivation.
        let table = match janaf.species(name) {
            Some(t) => t,
            None => continue,
        };
        let dfg = match table.delta_f_g_at(disk_temperature_k) {
            Some(d) => d,
            None => continue,
        };
        let g_over_rt = match janaf_g_over_rt(dfg, disk_temperature_k) {
            Some(g) => g,
            None => continue,
        };
        let stoichiometry: BTreeMap<String, i32> = atoms;
        let phase = if is_gas_phase(name) {
            SpeciesPhase::Gas
        } else {
            SpeciesPhase::Condensed
        };
        let species = EquilibriumSpecies {
            name: name.clone(),
            phase,
            g_over_rt,
            stoichiometry,
        };
        match phase {
            SpeciesPhase::Gas => gas.push(species),
            SpeciesPhase::Condensed => condensed.push(species),
        }
    }
    if gas.is_empty() {
        return None;
    }
    // Condensation of the solar gas at the disk temperature, then the active precipitates.
    let equilibrium = gas_equilibrium(&gas, &budget)?;
    let active = condensed_active_set(&condensed, &equilibrium)?;
    // The VCS amount redistribution over the active condensates (the phases, for their stoichiometry).
    let active_species: Vec<EquilibriumSpecies> = active
        .iter()
        .filter_map(|(name, _)| condensed.iter().find(|c| &c.name == name).cloned())
        .collect();
    let amounts = condensed_amounts(&gas, &active_species, &equilibrium, &budget);
    let condensed_amount_readout = match &amounts {
        Some(CondensedAmounts::Balanced(v)) => Some(v.clone()),
        _ => None, // degenerate vertex: the identity still differentiates, amounts route to the draw
    };
    // Differentiation: float the silicate fraction off the sinking metal and sulfide.
    let differentiation = differentiate(&active, &budget)?;
    // The partial-melt crust extraction: within the floating silicate fraction, the buoyant (least-dense) phase is
    // the CRUST, the denser rest the MANTLE residue it floats on. Density comes from the petrology substrate at the
    // labelled surface conditions (~300 K, ~1 bar), the surface where the isostasy is read.
    let registry = PhaseRegistry::standard().ok()?;
    let table = PeriodicTable::standard().ok()?;
    let surface_t = Fixed::from_int(300);
    let surface_p = Fixed::ONE;
    let density_of = |name: &str| -> Option<Fixed> {
        let atoms = species_elements(name)?;
        let composition: Vec<(String, Fixed)> = atoms
            .iter()
            .map(|(el, n)| (el.clone(), Fixed::from_int(*n)))
            .collect();
        crustal_density(&composition, surface_t, surface_p, &registry, &table)
    };
    let (crust, mantle) = crust_and_mantle(&differentiation.floating, density_of)?;
    // The surface is the ASSEMBLAGE (seam 5): each element at the sum over the crust phases of the phase's modal
    // amount (the VCS redistribution) times its stoichiometry, the rock the crust forms and not the oxygen-heavy
    // solar budget. A degenerate VCS vertex leaves `condensed_amount_readout` None, so the assemblage falls back to
    // each phase's saturation presence (still a stoichiometric rock, never the solar budget).
    let amounts_map: BTreeMap<String, Fixed> = condensed_amount_readout
        .as_ref()
        .map(|v| v.iter().cloned().collect())
        .unwrap_or_default();
    let surface = phase_set_composition(&crust, &amounts_map);
    let mantle_composition = phase_set_composition(&mantle, &amounts_map);
    Some(SurfaceComposition {
        surface,
        mantle_composition,
        crust,
        mantle,
        differentiation,
        condensed_amounts: condensed_amount_readout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The condensation inputs (gas species, condensed species, element budget) a test drives the solve with.
    type CondensationInputs = (
        Vec<EquilibriumSpecies>,
        Vec<EquilibriumSpecies>,
        BTreeMap<String, Fixed>,
    );

    // A capture harness: build the condensation inputs (gas, condensed, budget) at a disk temperature, EXCLUDING a
    // given element set, exactly as `derive_surface_composition` builds them, so the byte-stability proof can pin the
    // 11-element subset (exclude Al, Ca) independent of the production cap. Returns None on the same failure paths as
    // the production builder.
    fn capture_inputs(
        janaf: &JanafTables,
        abundances: &SolarAbundances,
        disk_temperature_k: Fixed,
        exclude: &[&str],
    ) -> Option<CondensationInputs> {
        let mut gas_elements: BTreeSet<String> = BTreeSet::new();
        for name in janaf.names() {
            if is_gas_phase(name) {
                if let Some(atoms) = species_elements(name) {
                    for el in atoms.keys() {
                        gas_elements.insert(el.clone());
                    }
                }
            }
        }
        let mut budget: BTreeMap<String, Fixed> = BTreeMap::new();
        for el in &gas_elements {
            if exclude.contains(&el.as_str()) {
                continue;
            }
            if let Some(log_eps) = abundances.preferred(el) {
                let exponent = log_eps
                    .checked_sub(Fixed::from_int(12))?
                    .checked_mul(ln_ten())?;
                let amount = exponent.exp();
                if amount > Fixed::ZERO {
                    budget.insert(el.clone(), amount);
                }
            }
        }
        if budget.is_empty() {
            return None;
        }
        let mut gas: Vec<EquilibriumSpecies> = Vec::new();
        let mut condensed: Vec<EquilibriumSpecies> = Vec::new();
        let mut names: Vec<String> = janaf.names().map(|s| s.to_string()).collect();
        names.sort();
        for name in &names {
            let atoms = match species_elements(name) {
                Some(a) => a,
                None => continue,
            };
            if !atoms.keys().all(|el| budget.contains_key(el)) {
                continue;
            }
            let table = match janaf.species(name) {
                Some(t) => t,
                None => continue,
            };
            let dfg = match table.delta_f_g_at(disk_temperature_k) {
                Some(d) => d,
                None => continue,
            };
            let g_over_rt = match janaf_g_over_rt(dfg, disk_temperature_k) {
                Some(g) => g,
                None => continue,
            };
            let phase = if is_gas_phase(name) {
                SpeciesPhase::Gas
            } else {
                SpeciesPhase::Condensed
            };
            let species = EquilibriumSpecies {
                name: name.clone(),
                phase,
                g_over_rt,
                stoichiometry: atoms,
            };
            match phase {
                SpeciesPhase::Gas => gas.push(species),
                SpeciesPhase::Condensed => condensed.push(species),
            }
        }
        Some((gas, condensed, budget))
    }

    #[test]
    fn the_eleven_element_subset_assemblage_is_byte_pinned() {
        // THE MINIMIZER-REPAIR BYTE GATE. The rock-forming 11-element subset (the solar set with aluminium and calcium
        // held out) is well-conditioned and converged before the damped-Newton repair; the repair MUST NOT move its
        // answer by a single bit. These are the raw-bit element potentials and condensed-active-set saturation indices
        // captured on the pre-repair solver at the disk 1000 K; the repair keeps the well-conditioned subset on the
        // legacy fixed-point path, so they reproduce exactly. A drift here is a repair regression, not a recalibration.
        let janaf = JanafTables::standard().expect("JANAF loads");
        let abundances = SolarAbundances::standard().expect("abundances load");
        let (gas, condensed, budget) =
            capture_inputs(&janaf, &abundances, Fixed::from_int(1000), &["Al", "Ca"])
                .expect("the 11-element subset builds");
        assert_eq!(budget.len(), 11, "the held-out subset is 11 elements");
        let eq = gas_equilibrium(&gas, &budget).expect("the 11-element subset converges");
        let lam = |el: &str| eq.element_potentials.get(el).unwrap().to_bits();
        assert_eq!(lam("C"), -19646438972);
        assert_eq!(lam("Fe"), 92207809417);
        assert_eq!(lam("H"), -1492572748);
        assert_eq!(lam("K"), -67359544753);
        assert_eq!(lam("Mg"), -25434124549);
        assert_eq!(lam("N"), -22146172389);
        assert_eq!(lam("Na"), -49604957980);
        assert_eq!(lam("O"), -129723459587);
        assert_eq!(lam("S"), -67381068957);
        assert_eq!(lam("Si"), -11857409479);
        assert_eq!(lam("Ti"), 38517351736);
        let active = condensed_active_set(&condensed, &eq).expect("the active set resolves");
        let active_bits: Vec<(&str, i64)> = active
            .iter()
            .map(|(n, s)| (n.as_str(), s.to_bits()))
            .collect();
        assert_eq!(
            active_bits,
            vec![
                ("Mg2SiO4(cr,forsterite)", 337143458422),
                ("MgSiO3(cr,enstatite)", 223356207878),
                ("Fe(cr)", 92207809417),
                ("FeS(cr,troilite)", 75279725254),
            ],
            "the 11-element active set is byte-identical to the pre-repair solver"
        );
    }

    #[test]
    fn the_full_thirteen_element_solar_set_converges_with_silicates() {
        // The repair's purpose: the full 13-element solar set (aluminium and calcium added, their gas abundances ~6
        // decades below hydrogen) now CONVERGES where the undamped Newton diverged. The converged assemblage is the
        // physical refractory sequence: the Mg-silicates forsterite and enstatite (the crust formers), plus the
        // aluminium condensates corundum and spinel and the iron metal and sulfide. This is what removing the
        // MINIMIZER_UNCONVERGED cap depends on.
        let janaf = JanafTables::standard().expect("JANAF loads");
        let abundances = SolarAbundances::standard().expect("abundances load");
        let (gas, condensed, budget) =
            capture_inputs(&janaf, &abundances, Fixed::from_int(1000), &[])
                .expect("the full set builds");
        assert_eq!(budget.len(), 13, "the full set is 13 elements");
        let eq = gas_equilibrium(&gas, &budget)
            .expect("the 13-element set now converges (was None before)");
        let active = condensed_active_set(&condensed, &eq).expect("the active set resolves");
        let names: Vec<&str> = active.iter().map(|(n, _)| n.as_str()).collect();
        for phase in [
            "Mg2SiO4(cr,forsterite)",
            "MgSiO3(cr,enstatite)",
            "Al2O3(cr,corundum)",
            "MgAl2O4(cr,spinel)",
            "Fe(cr)",
        ] {
            assert!(
                names.contains(&phase),
                "the converged 13-element assemblage precipitates {phase}, got {names:?}"
            );
        }
    }

    #[test]
    fn the_inner_disk_derives_a_silicate_crust_over_an_iron_core() {
        // At a hot inner-disk temperature the solar gas condenses the Mg-silicates and iron metal; differentiation
        // floats the oxygen-bearing silicates as the crust and sinks the iron. The derived surface carries Mg, Si,
        // and O (the silicate elements), the crust the physics produces, with no authored composition anywhere.
        let janaf = JanafTables::standard().expect("JANAF loads");
        let abundances = SolarAbundances::standard().expect("abundances load");
        // A hot inner-disk temperature where the Mg-silicates and iron are condensed (well below their ~1350 K
        // condensation fronts, above the volatile ices).
        let sc = derive_surface_composition(&janaf, &abundances, Fixed::from_int(1000))
            .expect("the inner disk derives a surface");
        let surface: Vec<&str> = sc.surface.iter().map(|(e, _)| e.as_str()).collect();
        assert!(
            surface.contains(&"Mg") && surface.contains(&"Si") && surface.contains(&"O"),
            "the derived crust is a magnesium silicate, got {surface:?}"
        );
        // Iron and sulfur, if condensed as metal and sulfide, sank to the core, so they are not the surface.
        assert!(
            !surface.contains(&"Fe"),
            "metallic iron sank to the core, off the surface, got {surface:?}"
        );
        // The sinking fraction is the metal (and sulfide) the differentiation pulled down.
        assert!(
            sc.differentiation
                .sinking
                .iter()
                .any(|(n, _)| n.starts_with("Fe(")),
            "iron metal is in the sinking fraction, got {:?}",
            sc.differentiation.sinking
        );
    }

    #[test]
    fn the_surface_reads_the_crust_assemblage_not_the_solar_budget() {
        // SEAM 5 end to end: the derived surface must be the crust's mineral assemblage (its silicate stoichiometry),
        // not the oxygen-heavy solar element budget. The solar budget has O:Si ~ 340:90 ~ 3.8; a Mg-silicate crust
        // (enstatite MgSiO3 at O:Si = 3, or forsterite Mg2SiO4 at O:Si = 4) reads its own ratio. Whichever silicate
        // the buoyancy split floats, the surface O:Si must equal that phase's stoichiometry, distinct from the solar
        // ratio, proving the assemblage replaced the budget.
        let janaf = JanafTables::standard().expect("JANAF loads");
        let abundances = SolarAbundances::standard().expect("abundances load");
        let sc = derive_surface_composition(&janaf, &abundances, Fixed::from_int(1000))
            .expect("the inner disk derives a surface");
        let get = |el: &str| -> Option<f64> {
            sc.surface
                .iter()
                .find(|(e, _)| e == el)
                .map(|(_, a)| a.to_bits() as f64)
        };
        let (o, si) = (
            get("O").expect("O on surface"),
            get("Si").expect("Si on surface"),
        );
        let o_over_si = o / si;
        // The crust is a single Mg-silicate here, so O:Si is exactly 3 (enstatite) or 4 (forsterite), never the
        // solar 3.8+. Assert it sits at an integer silicate ratio well away from solar.
        assert!(
            (o_over_si - 3.0).abs() < 0.05 || (o_over_si - 4.0).abs() < 0.05,
            "the surface O:Si {o_over_si} is a silicate stoichiometry (3 or 4), not the solar budget"
        );
        // And the surface equals the assemblage of the crust phases (the substrate now provides what the viewer used
        // to re-derive): the same rock the crust is.
        assert!(
            !sc.surface.is_empty() && !sc.mantle_composition.is_empty(),
            "both the crust surface and the mantle composition are derived assemblages"
        );
    }
}
