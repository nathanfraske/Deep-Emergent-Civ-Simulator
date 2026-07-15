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

//! GRAVITATIONAL DIFFERENTIATION (identity link): the piece between the condensed bulk and the SURFACE (crust)
//! composition that `slice0_demo_field` named as its authored stand-in ("a stand-in for what accretion and
//! differentiation will derive"). Under gravity the condensed phases separate into two immiscible fractions that
//! settle by density: the dense, oxygen-free METAL and SULFIDE melt sinks toward the core, and the light,
//! oxygen-bearing SILICATE and OXIDE fraction floats as the mantle and crust. So the SURFACE the tile chain reads is
//! the floating silicate crust, DERIVED from the condensation, not an authored composition banded for visual variety.
//! This is the banked Stokes-settling differentiation run on the disposer's own output; the anion chemistry is the
//! proxy for which immiscible melt a phase joins, and the melt densities (metal and sulfide ~5 to 8 against silicate
//! ~3) do the settling.
//!
//! NO AUTHORED GOLDSCHMIDT LIST (Principle 4), and this IS how it admits the alien (Principle 7): the rule reads
//! whether each condensed phase carries oxygen, which is the fO2 ladder's OWN readout. Siderophile-versus-lithophile
//! was always downstream physics: "did the oxygen fugacity at this orbit give the element's stable phase an oxygen".
//! So the SAME rule partitions a world by its OWN redox, never a fixed element table: at oxidizing fugacities calcium
//! and magnesium condense as silicates (oxygen-bearing) and FLOAT; at reducing, enstatite-chondrite fugacities they
//! condense as the sulfides oldhamite (CaS) and niningerite (MgS) (oxygen-free) and DEFECT to the sinking set. That
//! Ca-Mg defection under reduction is the test that proves the link reads chemistry, not a hidden list. The
//! oxygen-lithophile criterion is the pinned row for oxygen-chemistry worlds; a non-oxygen lithophile chemistry (a
//! carbon world's carbides and graphite, an icy world's ices) is a NAMED data extension (non-exhaustive), until which
//! a world with no oxygen-bearing condensate yields no derived crust (fail-loud), never a mis-sorted one.
//!
//! Honest grade: the IDENTITY of each fraction is derived here (which phases and elements sink or float), the part
//! that made the authored arrangement arbitrary. The ASSEMBLAGE composition ([`phase_set_composition`], seam 5) is
//! the rock the phases form: each element carried at the sum over the phases of the phase's modal amount (from the
//! VCS redistribution, `equilibrium_condensation::condensed_amounts`) times its stoichiometric count, so a crust of
//! enstatite reads `Mg:Si:O = 1:1:3` and not the oxygen-heavy solar element budget it condensed from. The
//! `DifferentiatedPlanet::surface_composition` field below is a coarser AUDIT view (the floating-fraction elements
//! at their bulk abundance, an at-a-glance which-elements-floated, distinct from the crust assemblage the tiles
//! read). The crust-versus-mantle split within the floating fraction (the crust is the low-melting PARTIAL MELT,
//! not the whole silicate) is a buoyancy split here; the melting model that would set the melt fraction is named,
//! not built (the seam-6 melt rung).

use civsim_core::Fixed;
use std::collections::BTreeMap;

/// Parse a condensed-species name (its phase suffix stripped) into element counts. General over any formula, so an
/// alien condensate is parsed, not special-cased. `None` on a malformed formula.
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

/// The elements of a condensed-species name (the formula before its `(cr,...)` or `(l)` phase suffix).
fn species_elements(name: &str) -> Option<BTreeMap<String, i32>> {
    let formula = name.split('(').next()?;
    parse_formula(formula)
}

/// Whether a condensed phase floats to the crust: it carries a LITHOPHILE anion. Oxygen is the pinned lithophile
/// anion for oxygen-chemistry worlds (a silicate or oxide floats); an oxygen-free phase (metal or sulfide) sinks.
/// This reads the fO2 ladder's readout, not an element tag: the SAME check floats a Ca-silicate at oxidizing
/// fugacity and sinks oldhamite (CaS) at reducing. Non-oxygen lithophile chemistries are the named data extension
/// (see the module note); until then their phases do not satisfy this and a non-oxygen world yields no crust.
fn floats_to_crust(name: &str) -> bool {
    species_elements(name)
        .map(|atoms| atoms.contains_key("O"))
        .unwrap_or(false)
}

/// A differentiated planet: the condensed bulk separated into the fraction that sinks and the fraction that floats.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DifferentiatedPlanet {
    /// The SINKING fraction (core and metal-sulfide interior): the oxygen-free metal and sulfide condensates,
    /// (species, presence).
    pub sinking: Vec<(String, Fixed)>,
    /// The FLOATING fraction (mantle and crust): the oxygen-bearing silicate and oxide condensates, (species,
    /// presence).
    pub floating: Vec<(String, Fixed)>,
    /// An AUDIT view of the floating fraction: its elements at their bulk abundance, an at-a-glance which-elements-
    /// floated. This is NOT the surface the tile chain reads (that is the crust ASSEMBLAGE, `SurfaceComposition::
    /// surface` via [`phase_set_composition`], seam 5); it is the coarse floating-fraction summary. Deterministic
    /// element order.
    pub surface_composition: Vec<(String, Fixed)>,
}

/// Differentiate the condensed bulk, deriving the surface composition. `condensed_bulk` is the condensed phases at
/// the orbit (`equilibrium_condensation::condensed_active_set`), each a (species name, presence) pair. `bulk_
/// abundances` maps each element to its bulk amount, the source of the surface element amounts (identity-derived,
/// amounts-pending, see the module note). The oxygen-free metal and sulfide phases sink; the oxygen-bearing silicate
/// and oxide phases float; the surface carries each element the floating phases claim, at its bulk abundance.
/// `None` if the bulk is empty or no phase floats (a world with no oxygen-bearing condensate has no derived crust:
/// fail-loud, never a mis-sorted or fabricated surface).
pub fn differentiate(
    condensed_bulk: &[(String, Fixed)],
    bulk_abundances: &BTreeMap<String, Fixed>,
) -> Option<DifferentiatedPlanet> {
    if condensed_bulk.is_empty() {
        return None;
    }
    let mut sinking = Vec::new();
    let mut floating = Vec::new();
    let mut surface_elements: BTreeMap<String, Fixed> = BTreeMap::new();
    for (name, presence) in condensed_bulk {
        if floats_to_crust(name) {
            floating.push((name.clone(), *presence));
            if let Some(atoms) = species_elements(name) {
                for element in atoms.keys() {
                    if let Some(abundance) = bulk_abundances.get(element) {
                        surface_elements.insert(element.clone(), *abundance);
                    }
                }
            }
        } else {
            sinking.push((name.clone(), *presence));
        }
    }
    if floating.is_empty() || surface_elements.is_empty() {
        return None;
    }
    Some(DifferentiatedPlanet {
        sinking,
        floating,
        surface_composition: surface_elements.into_iter().collect(),
    })
}

/// THE PARTIAL-MELT CRUST EXTRACTION: split the floating silicate fraction into the CRUST (the buoyant partial melt
/// that floats to the surface) and the MANTLE (the dense refractory residue it leaves). Within the silicate fraction
/// the split is BUOYANCY, not chemistry: the low-density melt (feldspar ~2.76, silica ~2.65) rises over the dense
/// refractory (forsterite ~3.27), so the crust is the LEAST-DENSE floating phase and the mantle is the denser rest.
/// This is the density contrast that gives the crust its isostatic relief (crust ~2.8 floating on mantle ~3.3),
/// which a crust identical to its mantle cannot. Distinct from and downstream of [`differentiate`]: that set the
/// metal-sulfide/silicate split by chemistry (immiscibility); this sets the crust/mantle split by density (buoyancy),
/// the two physical drivers each in their own place.
///
/// Admit the alien: the primitive is density, so WHAT floats as crust is a data outcome (feldspathic on a Terran
/// silicate world, a different light phase elsewhere), never a mineral table. `density_of` returns each floating
/// phase's DERIVED density (the caller injects it from the petrology substrate). Returns (crust, mantle) as
/// (species, presence) lists; `None` if a floating-phase density fails to resolve (fail-loud). A single floating
/// phase is all crust (nothing denser to reject to the mantle).
// The (crust, mantle) pair of phase lists is the natural return; a named type would not read clearer than the pair.
#[allow(clippy::type_complexity)]
pub fn crust_and_mantle<F>(
    floating: &[(String, Fixed)],
    density_of: F,
) -> Option<(Vec<(String, Fixed)>, Vec<(String, Fixed)>)>
where
    F: Fn(&str) -> Option<Fixed>,
{
    if floating.is_empty() {
        return None;
    }
    // Each phase's density, when the petrology can resolve it. A phase the substrate has no molar volume for (an
    // exotic refractory like corundum or spinel the density kernel does not yet cover) is placed in the MANTLE as a
    // refractory rather than assigned a fabricated density: unresolved-density phases are not buoyancy candidates.
    // Preserve the input order (the condensation precedence) for determinism.
    let mut resolved: Vec<Option<Fixed>> = Vec::with_capacity(floating.len());
    let mut min_density: Option<Fixed> = None;
    for (name, _) in floating {
        let d = density_of(name);
        if let Some(d) = d {
            min_density = Some(match min_density {
                Some(m) if m.to_bits() <= d.to_bits() => m,
                _ => d,
            });
        }
        resolved.push(d);
    }
    let min_density = min_density?; // no floating phase has a resolvable density: no crust to float
    let mut crust = Vec::new();
    let mut mantle = Vec::new();
    for ((name, presence), density) in floating.iter().zip(resolved.iter()) {
        match density {
            Some(d) if *d <= min_density => crust.push((name.clone(), *presence)),
            _ => mantle.push((name.clone(), *presence)),
        }
    }
    Some((crust, mantle))
}

/// THE ASSEMBLAGE COMPOSITION (seam 5): the element amounts of a phase set as the ROCK it is, the sum over the
/// phases of each phase's modal amount times the element's stoichiometric count in that phase. A pure enstatite
/// (MgSiO3) crust comes out `Mg:Si:O = 1:1:3`, its real mineral ratio, not the oxygen-heavy solar element budget
/// the phases condensed from (which was never a rock). `amounts` gives each phase's modal amount from the VCS
/// redistribution ([`crate::equilibrium_condensation::condensed_amounts`]); a phase absent from `amounts` (a
/// degenerate VCS vertex where the moles did not resolve) falls back to its own saturation presence, the
/// best-available proxy, still a stoichiometric rock rather than the solar budget. Deterministic element order.
pub fn phase_set_composition(
    phases: &[(String, Fixed)],
    amounts: &BTreeMap<String, Fixed>,
) -> Vec<(String, Fixed)> {
    let mut elements: BTreeMap<String, Fixed> = BTreeMap::new();
    for (name, presence) in phases {
        let Some(atoms) = species_elements(name) else {
            continue;
        };
        // The phase's modal amount when the VCS vertex was well-posed, its saturation presence otherwise. Never the
        // solar element budget: the composition is the assemblage the phases form, weighted by how much of each.
        let weight = amounts.get(name).copied().unwrap_or(*presence);
        for (element, count) in &atoms {
            let contribution = match weight.checked_mul(Fixed::from_int(*count)) {
                Some(c) => c,
                None => continue,
            };
            let entry = elements.entry(element.clone()).or_insert(Fixed::ZERO);
            *entry = entry.checked_add(contribution).unwrap_or(*entry);
        }
    }
    elements.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bulk(entries: &[(&str, i64)]) -> BTreeMap<String, Fixed> {
        entries
            .iter()
            .map(|(el, n)| (el.to_string(), Fixed::from_int(*n as i32)))
            .collect()
    }

    fn one(name: &str) -> (String, Fixed) {
        (name.to_string(), Fixed::ONE)
    }

    fn sink_has(d: &DifferentiatedPlanet, name: &str) -> bool {
        d.sinking.iter().any(|(n, _)| n == name)
    }
    fn float_has(d: &DifferentiatedPlanet, name: &str) -> bool {
        d.floating.iter().any(|(n, _)| n == name)
    }
    fn surface_has(d: &DifferentiatedPlanet, element: &str) -> bool {
        d.surface_composition.iter().any(|(e, _)| e == element)
    }

    #[test]
    fn at_solar_composition_metal_and_sulfide_sink_and_silicates_float() {
        // The acceptance row, oxidizing (solar) fugacity: the sinking set contains Fe-Ni metal and FeS; the floating
        // set carries the Mg-silicates and the feldspathic (Ca-Al) component. The surface is the silicate elements.
        let condensed = vec![
            one("Fe(cr)"),
            one("Ni(cr)"),
            one("FeS(cr,troilite)"),
            one("Mg2SiO4(cr,forsterite)"),
            one("MgSiO3(cr,enstatite)"),
            one("CaAl2Si2O8(cr,anorthite)"),
        ];
        let abundances = bulk(&[
            ("Fe", 90),
            ("Ni", 5),
            ("Mg", 100),
            ("Si", 90),
            ("O", 340),
            ("S", 45),
            ("Ca", 6),
            ("Al", 9),
        ]);
        let d = differentiate(&condensed, &abundances).expect("solar bulk differentiates");
        assert!(
            sink_has(&d, "Fe(cr)") && sink_has(&d, "Ni(cr)") && sink_has(&d, "FeS(cr,troilite)")
        );
        assert!(
            float_has(&d, "Mg2SiO4(cr,forsterite)") && float_has(&d, "CaAl2Si2O8(cr,anorthite)"),
            "the Mg-silicates and feldspathic component float"
        );
        assert!(
            surface_has(&d, "Mg")
                && surface_has(&d, "Si")
                && surface_has(&d, "Ca")
                && surface_has(&d, "Al")
        );
        assert!(
            !surface_has(&d, "Ni") && !surface_has(&d, "S"),
            "the sinking metal and sulfur are not on the surface"
        );
    }

    #[test]
    fn under_reduction_calcium_and_magnesium_defect_to_the_sinking_set_as_sulfides() {
        // The counter-test that proves the link reads chemistry, not an element list: at IW-class reducing fugacity
        // the condensation makes calcium and magnesium as the sulfides oldhamite (CaS) and niningerite (MgS), which
        // are oxygen-free and so DEFECT to the sinking set, while the same elements floated as silicates at solar
        // fugacity above. The floating fraction is only the enstatite that stayed oxygen-bearing.
        let condensed = vec![
            one("Fe(cr)"),
            one("CaS(cr,oldhamite)"),
            one("MgS(cr,niningerite)"),
            one("MgSiO3(cr,enstatite)"),
        ];
        let abundances = bulk(&[
            ("Fe", 90),
            ("Ca", 6),
            ("Mg", 100),
            ("Si", 60),
            ("O", 180),
            ("S", 40),
        ]);
        let d = differentiate(&condensed, &abundances).expect("reduced bulk differentiates");
        assert!(
            sink_has(&d, "CaS(cr,oldhamite)") && sink_has(&d, "MgS(cr,niningerite)"),
            "under reduction the Ca and Mg sulfides sink"
        );
        // Ca defected entirely to the sink (only present as CaS), so it is not on the surface; Mg still floats in
        // the enstatite that stayed a silicate, so Mg remains a surface element.
        assert!(
            !surface_has(&d, "Ca"),
            "reduced calcium defected to the core, off the surface"
        );
        assert!(
            surface_has(&d, "Mg") && surface_has(&d, "Si"),
            "the silicate that stayed floats"
        );
    }

    #[test]
    fn a_world_with_no_oxygen_bearing_condensate_has_no_crust() {
        // A fully reduced or non-oxygen bulk floats nothing under the pinned oxygen-lithophile row: fail-loud (None),
        // the honest edge until the non-oxygen lithophile extension lands, never a mis-sorted crust.
        let condensed = vec![
            one("Fe(cr)"),
            one("FeS(cr,troilite)"),
            one("SiC(cr,moissanite)"),
        ];
        let abundances = bulk(&[("Fe", 90), ("Si", 40), ("S", 45), ("C", 60)]);
        assert!(differentiate(&condensed, &abundances).is_none());
    }

    #[test]
    fn the_surface_is_the_rock_assemblage_not_the_solar_budget() {
        // SEAM 5: a crust of pure enstatite MgSiO3 must read its mineral stoichiometry Mg:Si:O = 1:1:3, not the
        // oxygen-heavy solar element budget it condensed from. With the enstatite phase at unit modal amount the
        // element amounts are exactly (Mg 1, Si 1, O 3).
        let crust = vec![one("MgSiO3(cr,enstatite)")];
        let amounts: BTreeMap<String, Fixed> = [("MgSiO3(cr,enstatite)".to_string(), Fixed::ONE)]
            .into_iter()
            .collect();
        let comp: BTreeMap<String, Fixed> = phase_set_composition(&crust, &amounts)
            .into_iter()
            .collect();
        assert_eq!(comp.get("Mg"), Some(&Fixed::ONE));
        assert_eq!(comp.get("Si"), Some(&Fixed::ONE));
        assert_eq!(comp.get("O"), Some(&Fixed::from_int(3)));
        // The solar budget is oxygen-heavy (O ~ 340 against Si ~ 90); the assemblage must NOT reproduce that ratio.
        let o_over_si =
            comp.get("O").unwrap().to_bits() as f64 / comp.get("Si").unwrap().to_bits() as f64;
        assert!(
            (o_over_si - 3.0).abs() < 1e-6,
            "the crust reads its enstatite O:Si = 3, not the solar ~3.8+"
        );
    }

    #[test]
    fn a_two_phase_crust_weights_by_modal_amount() {
        // A crust of forsterite (Mg2SiO4) and enstatite (MgSiO3): the element amounts sum each phase's modal amount
        // times its stoichiometry. At forsterite 1, enstatite 2: Mg = 2*1 + 1*2 = 4, Si = 1*1 + 1*2 = 3,
        // O = 4*1 + 3*2 = 10. Doubling enstatite must shift the ratios, proving the modal weighting is live.
        let crust = vec![one("Mg2SiO4(cr,forsterite)"), one("MgSiO3(cr,enstatite)")];
        let amounts: BTreeMap<String, Fixed> = [
            ("Mg2SiO4(cr,forsterite)".to_string(), Fixed::ONE),
            ("MgSiO3(cr,enstatite)".to_string(), Fixed::from_int(2)),
        ]
        .into_iter()
        .collect();
        let comp: BTreeMap<String, Fixed> = phase_set_composition(&crust, &amounts)
            .into_iter()
            .collect();
        assert_eq!(comp.get("Mg"), Some(&Fixed::from_int(4)));
        assert_eq!(comp.get("Si"), Some(&Fixed::from_int(3)));
        assert_eq!(comp.get("O"), Some(&Fixed::from_int(10)));
    }

    #[test]
    fn a_degenerate_vertex_falls_back_to_the_phase_presence_not_the_budget() {
        // When the VCS moles did not resolve (the phase is absent from `amounts`), the assemblage falls back to the
        // phase's own saturation presence, still a stoichiometric rock. Enstatite at presence 2 reads Mg:Si:O =
        // 2:2:6, the enstatite ratio scaled by the presence, never the solar budget.
        let crust = vec![("MgSiO3(cr,enstatite)".to_string(), Fixed::from_int(2))];
        let empty: BTreeMap<String, Fixed> = BTreeMap::new();
        let comp: BTreeMap<String, Fixed> =
            phase_set_composition(&crust, &empty).into_iter().collect();
        assert_eq!(comp.get("Mg"), Some(&Fixed::from_int(2)));
        assert_eq!(comp.get("Si"), Some(&Fixed::from_int(2)));
        assert_eq!(comp.get("O"), Some(&Fixed::from_int(6)));
    }

    #[test]
    fn the_buoyant_silicate_floats_as_crust_over_the_dense_mantle() {
        // Within the floating silicate fraction the split is BUOYANCY: the lighter phase (enstatite ~3.20) floats as
        // the crust over the denser forsterite ~3.27 mantle residue, the density contrast the isostasy needs. A real
        // feldspathic crust (anorthite ~2.76) floats even more strongly when the data supplies it.
        let floating = vec![one("Mg2SiO4(cr,forsterite)"), one("MgSiO3(cr,enstatite)")];
        let density = |name: &str| -> Option<Fixed> {
            Some(match name.split('(').next()? {
                "Mg2SiO4" => Fixed::from_ratio(327, 100),
                "MgSiO3" => Fixed::from_ratio(320, 100),
                _ => return None,
            })
        };
        let (crust, mantle) = crust_and_mantle(&floating, density).expect("splits");
        assert!(
            crust.iter().any(|(n, _)| n.starts_with("MgSiO3")),
            "the lighter enstatite is the buoyant crust"
        );
        assert!(
            mantle.iter().any(|(n, _)| n.starts_with("Mg2SiO4")),
            "the denser forsterite is the mantle residue"
        );
    }
}
