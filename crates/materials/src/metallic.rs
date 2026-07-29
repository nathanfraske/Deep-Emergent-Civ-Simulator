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

//! The elemental metallic energy route (D3-c): the Stage-4 metallic branch the correlation guard's
//! [`crate::correlation::EnergyRoute::Metallic`] slot points to.
//!
//! For an elemental metal, the metallic cohesive energy is the banked atomization enthalpy `E_coh` (the element's
//! own cohesive energy, MEASURED `[M]`, `periodic_table.toml`). The elemental Rose universal binding-energy relation
//! (Rose, Smith, Guinea, Ferrante 1984) relates `E_coh` to the equation-of-state anchors, the bulk modulus `B_0`
//! and the equilibrium molar volume `V_m` (D3-a's `metal_eos_anchors.toml`), through the roughly-universal
//! dimensionless ratio `B_0 * V_m / E_coh`. The RMS relative deviation of that ratio across the anchored metals is
//! the route's `[D]`-from-`[M]` self-uncertainty BAND, measured against the cohesive-energy references (the D1
//! shape: a derived statistic over measured inputs, effective `[M]`), never a reserved knob. No consumer is wired
//! to it in any pinned run path yet (byte-neutral).
//!
//! WHY A UNIVERSAL-RATIO BAND, NOT AN OFF-EQUILIBRIUM CURVE. At its equilibrium volume the Rose EOS reproduces
//! `E_coh` by construction (the depth of the binding well), so the honest first-cut self-uncertainty is not the
//! well depth (which the banked `E_coh` gives exactly) but how well the EOS anchors and the cohesive energy obey
//! the Rose universal relation. `B_0 * V_m / E_coh` is the relation's citable headline (roughly constant across
//! metals); its spread is the metallic model's confidence, the spec's declared "tens of percent" for the metallic
//! route made a measured number. The full `E(V)` curve (the `P.dV` term, needing the per-atom Wigner-Seitz length
//! and so the Avogadro constant) is the natural D3-c follow-on; the equilibrium ranking needs only the well depth.
//!
//! WHAT IS INSIDE THE GUARD, AND WHAT ESCALATES. The correlation guard (D3-b) keeps a Mott insulator (NiO,
//! Localized) out of the metallic route entirely. This elemental first cut computes the cohesive energy of an
//! elemental metal that carries EOS anchors; a correlated OXIDE the guard routes Itinerant (TiO) has no
//! elemental-metal anchor and ESCALATES here (the honest partial fill: elemental anchors do not give an oxide's
//! energy), until an oxide-EOS slice lands. The Miedema alloy formation-enthalpy term (the `n_ws` and `phi*`
//! parameters) likewise escalates until `phi*` is sourced (its book-pinned seam, D3-a's finding).

use civsim_core::Fixed;
use civsim_physics::metal_eos::MetalEosAnchors;
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::rose_eos;

/// The elemental metallic energy route over the banked cohesive energy and the D3-a EOS anchors.
pub struct MetallicRoute<'a> {
    table: &'a PeriodicTable,
    anchors: &'a MetalEosAnchors,
}

impl<'a> MetallicRoute<'a> {
    /// Bind the metallic route to the periodic table (for the banked `E_coh`) and the EOS anchors (`B_0`, `V_m`).
    pub fn new(table: &'a PeriodicTable, anchors: &'a MetalEosAnchors) -> Self {
        MetallicRoute { table, anchors }
    }

    /// The elemental metallic cohesive energy `E_coh` (kJ/mol, the positive binding magnitude) for a metal that
    /// carries BOTH a banked atomization enthalpy AND EOS anchors, or `None` (escalate) otherwise. This is the
    /// `[M]` cohesive scale the metallic route reports; the lattice energy of the elemental phase is its negation.
    pub fn cohesive_energy(&self, symbol: &str) -> Option<Fixed> {
        // Require an anchored metal: the metallic route is scoped to the metals D3-a supplied EOS anchors for.
        self.anchors.anchor(symbol)?;
        self.table.element(symbol)?.atomization_enthalpy
    }

    /// The dimensionless Rose universal ratio `B_0 * V_m / E_coh` for an anchored metal (Rose, Smith, Guinea,
    /// Ferrante 1984; roughly universal across metals, its spread the route's self-uncertainty). `None` when the
    /// metal is not anchored or carries no banked (non-zero) cohesive energy.
    pub fn rose_ratio(&self, symbol: &str) -> Option<Fixed> {
        let anchor = self.anchors.anchor(symbol)?;
        let e_coh = self.table.element(symbol)?.atomization_enthalpy?;
        if e_coh == Fixed::ZERO {
            return None;
        }
        anchor
            .bulk_modulus_gpa
            .checked_mul(anchor.molar_volume)?
            .checked_div(e_coh)
    }

    /// The `[D]`-from-`[M]` metallic band: the RMS relative deviation of the Rose universal ratio `B_0 V_m / E_coh`
    /// from its anchored-set mean. HONEST SCOPE: this is the DISPERSION of the Rose universal relation across the
    /// anchored set (a coherence statistic of how uniformly the relation holds), NOT the error of the exact
    /// `-E_coh` this route currently returns (that value is the banked measurement, so its own uncertainty is the
    /// measurement's). Its role is to scale `resolution_s` WHEN the metallic route is wired into a disposer (the
    /// D1 band's role), where a wider spread means the metallic model separates candidates less confidently; it is
    /// NOT YET CONSUMED (no metallic dispose path exists), and it is set-dependent (it moves as the anchored
    /// registry grows). `None` when the anchored set is empty or any anchored metal is unscorable.
    pub fn band_fraction(&self) -> Option<Fixed> {
        let mut ratios = Vec::new();
        for (symbol, _anchor) in self.anchors.iter() {
            ratios.push(self.rose_ratio(symbol)?);
        }
        if ratios.is_empty() {
            return None;
        }
        let n = Fixed::from_int(ratios.len() as i32);
        // The anchored-set mean ratio (the Rose universal value the set implies, a derived statistic never
        // authored). The ratios are bounded and few, so the sum cannot overflow.
        let mut sum = Fixed::ZERO;
        for r in &ratios {
            sum += *r;
        }
        let mean = sum.checked_div(n)?;
        if mean == Fixed::ZERO {
            return None;
        }
        // RMS of the relative deviation (r - mean) / mean across the set: the D1 band shape.
        let mut sum_sq = Fixed::ZERO;
        for r in &ratios {
            let deviation = (*r - mean).checked_div(mean)?;
            let magnitude = if deviation < Fixed::ZERO {
                Fixed::ZERO - deviation
            } else {
                deviation
            };
            sum_sq += magnitude.checked_mul(magnitude)?;
        }
        let mean_sq = sum_sq.checked_div(n)?;
        Some(mean_sq.sqrt())
    }

    /// The signed metallic lattice energy `-E_coh` (kJ/mol, negative because a metallic phase is BOUND) for an
    /// elemental metallic candidate the metallic route can score, or `None` (escalate) otherwise. The disposer
    /// ranks a metallic candidate on this. `None` when the candidate is not a single anchored metal: a correlated
    /// OXIDE the guard routes here (TiO) has no elemental anchor and escalates, as does an unanchored element.
    pub fn metallic_energy(&self, composition: &[(String, u32)]) -> Option<Fixed> {
        // The elemental first cut: a single-element metallic candidate. A binary (an oxide) escalates until an
        // oxide-EOS slice lands, and an alloy escalates until the Miedema term's phi* is sourced.
        if composition.len() != 1 {
            return None;
        }
        let (symbol, _count) = &composition[0];
        let e_coh = self.cohesive_energy(symbol)?;
        Some(Fixed::ZERO - e_coh)
    }

    /// The metallic cohesive energy (kJ/mol) of an anchored elemental metal at a compressed or expanded molar
    /// volume, the full Rose UBER `E(V)` curve over the D3-a anchors (`rose_eos`). At the equilibrium volume it is
    /// `-E_coh` (the well depth `metallic_energy` reports); off equilibrium it is the `E(V)` the disposer's `P.dV`
    /// term reads. `None` when the metal is not anchored, carries no banked cohesive energy, or a step overflows.
    pub fn cohesive_energy_at_volume(&self, symbol: &str, molar_volume: Fixed) -> Option<Fixed> {
        let anchor = self.anchors.anchor(symbol)?;
        let e_coh = self.table.element(symbol)?.atomization_enthalpy?;
        rose_eos::cohesive_energy_at_volume(
            e_coh,
            anchor.molar_volume,
            anchor.bulk_modulus_gpa,
            molar_volume,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correlation::{CorrelationClassifier, EnergyRoute};
    use civsim_physics::d_state_radius::DStateRadii;
    use civsim_physics::ionic_radii::IonicRadii;
    use civsim_physics::ionization_ladder::IonizationLadder;
    use civsim_physics::mit_reference::MitReference;

    fn floors() -> (PeriodicTable, MetalEosAnchors) {
        (
            PeriodicTable::standard().expect("periodic table"),
            MetalEosAnchors::standard().expect("metal EOS anchors"),
        )
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    fn comp(pairs: &[(&str, u32)]) -> Vec<(String, u32)> {
        pairs.iter().map(|(s, c)| ((*s).to_string(), *c)).collect()
    }

    #[test]
    fn the_cohesive_energy_is_the_banked_atomization_enthalpy() {
        let (t, a) = floors();
        let route = MetallicRoute::new(&t, &a);
        // Fe's banked atomization enthalpy is its cohesive energy (kJ/mol).
        //
        // THIS PIN WAS ASSERTING A RETIRED VALUE, and it is worth the paragraph. It read 416.3, which was
        // the Fe row's value until `a91954b` re-valued it to 415.471: the 416.3 was cited to CODATA Key
        // Values, and CODATA CONTAINS NO IRON ROW, so it wore a citation it could not cash. The row now
        // takes the repository's own md5-verified `data/janaf/Fe-008.txt`.
        //
        // The data moved and the assertion did not, so this suite spent that whole time ASSERTING THE
        // DEFECT. The commit that fixed the row NAMED THIS CONSUMER IN ITS OWN MESSAGE ("the column is
        // LIVE: metallic.rs reads it for Rose's own cohesive-energy route") and changed only the TOML.
        // Naming a consumer is not checking it. A DATA PIN IS A TEST OF THE DATA, so re-valuing a row is
        // incomplete until its pins move with it, in the same commit, for the same cited reason.
        let fe = route.cohesive_energy("Fe").expect("Fe cohesive energy");
        assert!(
            close(fe, 415.471, 0.1),
            "Fe E_coh = 415.471 kJ/mol (JANAF Fe-008, delta-f H at 298.15 K), got {}",
            fe.to_f64_lossy()
        );
        // An element without an EOS anchor is out of the metallic route's scope (escalate).
        assert!(
            route.cohesive_energy("Cu").is_none(),
            "Cu has no EOS anchor, so the metallic route does not score it"
        );
        assert!(
            route.cohesive_energy("O").is_none(),
            "oxygen is not an anchored metal"
        );
    }

    #[test]
    fn the_rose_ratio_is_the_bulk_energy_over_the_cohesive_energy() {
        let (t, a) = floors();
        let route = MetallicRoute::new(&t, &a);
        // Fe: B_0 * V_m / E_coh = 170 * 7.09 / 415.471 = 2.901.
        //
        // THIS TEST NEVER FAILED AND NEVER WOULD HAVE, which is the quiet half of the same defect. The
        // retired 416.3 gives 2.8953 and the cited 415.471 gives 2.9010: a shift of 0.0058 under a
        // tolerance of 0.01. THE TOLERANCE ABSORBED THE CORRECTION, so this assertion sat green across a
        // data repair while its comment kept teaching the retired value. A tolerance wide enough to
        // absorb a change is blind to it, which is legitimate for a physical band and is exactly why the
        // COMMENT, not the assertion, is what a reader inherits.
        let fe = route.rose_ratio("Fe").expect("Fe rose ratio");
        assert!(
            close(fe, 2.901, 0.01),
            "Fe Rose ratio ~ 2.901, got {}",
            fe.to_f64_lossy()
        );
    }

    #[test]
    fn the_band_is_the_measured_rose_ratio_spread() {
        // THE [D]-from-[M] BAND (the D1 shape): the RMS relative deviation of the Rose universal ratio across the
        // anchored metals, measured against the cohesive-energy references. It is the metallic route's self-
        // uncertainty, roughly a third (the spec's "tens of percent"): the Rose relation is loose across a set
        // that spans alkali (Na, K) to transition (Ti, Fe) metals, tighter within one bonding class (a flagged
        // refinement). A measured number, never reserved.
        let (t, a) = floors();
        let route = MetallicRoute::new(&t, &a);
        let band = route.band_fraction().expect("the metallic band measures");
        let b = band.to_f64_lossy();
        assert!(
            (0.30..0.40).contains(&b),
            "the metallic Rose-ratio band is ~0.35 (tens of percent), got {b}"
        );
    }

    #[test]
    fn an_elemental_metal_scores_and_a_compound_escalates() {
        let (t, a) = floors();
        let route = MetallicRoute::new(&t, &a);
        // An elemental metal scores: the signed lattice energy is -E_coh (bound, negative).
        let fe = route
            .metallic_energy(&comp(&[("Fe", 1)]))
            .expect("elemental Fe scores");
        assert!(
            close(fe, -415.471, 0.1),
            "elemental Fe lattice energy = -415.471 kJ/mol (JANAF Fe-008), got {}",
            fe.to_f64_lossy()
        );
        // A binary (an oxide) escalates: elemental anchors do not give an oxide's energy.
        assert!(
            route
                .metallic_energy(&comp(&[("Ti", 1), ("O", 1)]))
                .is_none(),
            "TiO escalates in the elemental metallic route (no oxide anchor)"
        );
        // An unanchored element escalates.
        assert!(
            route.metallic_energy(&comp(&[("Cu", 1)])).is_none(),
            "Cu (no anchor) escalates"
        );
    }

    #[test]
    fn the_route_evaluates_the_rose_curve_over_volume() {
        // The volume-aware route reads the Rose EOS over the anchors: at the equilibrium molar volume it returns
        // -E_coh (the well bottom), and compression raises the energy above it. An unanchored metal escalates.
        let (t, a) = floors();
        let route = MetallicRoute::new(&t, &a);
        let v0 = a.molar_volume("Fe").expect("Fe V0");
        let at_equilibrium = route.cohesive_energy_at_volume("Fe", v0).expect("Fe E(V0)");
        assert!(
            close(at_equilibrium, -415.471, 0.01),
            "Fe E(V0) = -E_coh (415.471 kJ/mol, JANAF Fe-008), got {}",
            at_equilibrium.to_f64_lossy()
        );
        let compressed = route
            .cohesive_energy_at_volume("Fe", v0.checked_mul(Fixed::from_ratio(9, 10)).unwrap())
            .expect("Fe E(0.9 V0)");
        assert!(
            compressed > at_equilibrium,
            "compression raises Fe's energy above the well bottom"
        );
        assert!(
            route.cohesive_energy_at_volume("Cu", v0).is_none(),
            "an unanchored metal escalates in the volume-aware route"
        );
    }

    #[test]
    fn the_metallic_route_sits_behind_the_correlation_guard() {
        // THE INTEGRATION (the gate's "inside the D3-b guard"): the metallic route is the machinery the guard's
        // Metallic slot points to. A Mott insulator (NiO) is routed Localized and NEVER reaches the metallic route
        // (the D2b payoff). An itinerant oxide (TiO) is routed Metallic but has no elemental-metal anchor, so the
        // metallic route escalates (the honest partial fill, an oxide-EOS slice fills it later) not a number.
        let (t, a) = floors();
        let ladder = IonizationLadder::standard().expect("ionization ladder");
        let d_state = DStateRadii::standard(
            &civsim_units::constants::canonical_si_execution_magnitudes()
                .expect("the sealed physical floor projects"),
        )
        .expect("d-state radii");
        let radii = IonicRadii::standard().expect("ionic radii");
        let mit = MitReference::standard().expect("MIT reference set");
        let guard = CorrelationClassifier::calibrate(&t, &ladder, &d_state, &radii, &mit)
            .expect("the classifier calibrates");
        let route = MetallicRoute::new(&t, &a);

        // NiO: the guard routes it away from the metallic route (Localized), so the metallic route is never asked.
        let nio = comp(&[("Ni", 1), ("O", 1)]);
        assert_eq!(
            guard.route(&nio),
            EnergyRoute::Localized,
            "NiO (a Mott insulator) is kept out of the metallic route by the guard"
        );

        // TiO: the guard routes it to the metallic slot; the elemental metallic route then escalates (no oxide
        // anchor), the honest partial fill. The composition that reaches the metallic route escalates, not a wrong
        // confident number.
        let tio = comp(&[("Ti", 1), ("O", 1)]);
        assert_eq!(
            guard.route(&tio),
            EnergyRoute::Metallic,
            "TiO (itinerant) routes to the metallic slot"
        );
        assert!(
            route.metallic_energy(&tio).is_none(),
            "the elemental metallic route escalates on TiO (no oxide anchor) rather than guessing"
        );
    }
}
