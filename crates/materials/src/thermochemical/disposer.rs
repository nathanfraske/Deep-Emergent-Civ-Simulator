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

//! The Stage-4 thermochemical DISPOSER, D1: the ionic branch of the free-energy selection.
//!
//! This wires the proposer's [`Compound`] candidates to the physics lattice-energy estimator
//! (`civsim_physics::lattice_modulus::lattice_energy_ionic_raw`) and disposes over them through the sealed
//! kernel primitive [`crate::verdict::dispose`], so the `delta`-versus-`resolution_s` dispatch (the Gap Law)
//! lives in one place and cannot be re-implemented here. The disposer supplies only two things the kernel does
//! not: the per-candidate ENERGY (the physics) and the RESOLUTION BAND (the estimator's measured uncertainty).
//!
//! THE MEASURED BAND (the never-fabricate crux, gate-ruled). The resolution band is the model-floor fraction
//! times the winner's energy magnitude, and the fraction is `[M]` MEASURED, not authored: it is the
//! root-mean-square relative deviation of the formal-charge Born-Lande energy from the cited Born-Haber
//! references (`civsim_physics::lattice_modulus::ionic_energy_band_fraction`, about 4 percent), refutable by
//! measuring more references without running the sim. So D1 is ZERO-AUTHORED (one new `[M]` value, the
//! validation-set-derived band-fraction, never a reserved `[C]` knob). The band is what makes the estimator
//! honest: a near-degenerate pair it cannot separate within its measured error ESCALATES rather than emitting a
//! confident wrong ground state. The floor fraction is roughly constant across ionic solids (NaCl, about as
//! ionic as a solid gets, is still a few percent off, which is the Born-Lande-versus-Born-Mayer repulsion-form
//! error, NOT covalency); it grows as covalent-leaning references enter the validation set. Path A adds the
//! per-candidate DERIVED covalency term (from the Mulliken electronegativity difference) that widens the band
//! where ionicity drops, and the QeQ partial charges that lower the model floor itself.
//!
//! HONEST LIMITS (surfaced, D1's scope):
//! - The energy is the LATTICE energy (the solid below its FORMAL gas ions, the Born-Haber reference), a
//!   COMPONENT of the formation free energy, not the whole of it: the ion-formation (ionization plus electron
//!   affinity) and the entropy terms are later slices. So cross-charge and cross-stoichiometry rankings are a
//!   proxy, and the measured band is exactly what escalates the pairs the lattice energy alone cannot cleanly
//!   rank; the follow-on terms need new floor columns (ionization energy, electron affinity).
//! - The verdict is the SINGLE most-stable candidate (the kernel primitive), not the coexisting equilibrium
//!   ASSEMBLAGE: the assemblage is a fold over the disposer plus the freezer (`Quench`), a later stage.
//! - The prototype is SEEDED as rock-salt for a 1:1 binary (D1's first-cut structural assignment); the
//!   structural layer supplies the correct prototype per composition later, so a fluorite or framework
//!   composition falls through the ionic branch here (`None`) rather than being forced onto a rock-salt Madelung.

use crate::contract::Disposer;
use crate::thermochemical::proposer::{Compound, Environment};
use crate::verdict::{content_key, dispose, ProvenanceKey, TieSlot, Verdict};
use civsim_core::Fixed;
use civsim_physics::ionic_radii::IonicRadii;
use civsim_physics::lattice_modulus::{lattice_energy_ionic_raw, BornExponents, PrototypeLibrary};
use civsim_physics::periodic::PeriodicTable;
use std::collections::BTreeMap;

/// The structure prototype the disposer seeds for a 1:1 binary ionic candidate (D1's first-cut structural
/// assignment). The structure-prototype membership is data (`civsim_physics` `prototypes.toml`); this names the
/// one D1 tries, the clean single-distance Born-Lande aristotype.
const ROCK_SALT: &str = "rock-salt";

/// The thermochemical disposer, D1: the ionic branch. It reads the physics floor (the periodic table, the
/// Shannon crystal radii, the Born exponents, the structure prototypes) to score a candidate's ionic lattice
/// energy, and carries the MEASURED model-floor band-fraction (the estimator's self-uncertainty against the
/// Born-Haber references) as the resolution band. The `provenance_key` and `tie_slot` are the opaque honesty-
/// accounting handles every verdict carries (resolved against the seven-tag register in `sim`).
pub struct ThermochemicalDisposer<'a> {
    /// The periodic-table floor (valences for the charge balance that identifies the ionic pair).
    pub table: &'a PeriodicTable,
    /// The Shannon crystal ionic radii floor (the interionic distance r0).
    pub radii: &'a IonicRadii,
    /// The Born exponents floor (the repulsion stiffness n, keyed by noble-gas core).
    pub born: &'a BornExponents,
    /// The structure-prototype library floor (the Madelung constant and coordination).
    pub prototypes: &'a PrototypeLibrary,
    /// The MEASURED model-floor band-fraction (`[M]`, from `ionic_energy_band_fraction`): the resolution band is
    /// this fraction times the winner's energy magnitude. Surfaced not fabricated; the caller supplies the
    /// measured value (never a hardcoded default), so an unset band cannot silently decide a near-degenerate pair.
    pub band_fraction: Fixed,
    /// The opaque provenance key for the honesty accounting (resolved in `sim`).
    pub provenance_key: ProvenanceKey,
    /// The named contingency slot a downstream seeded draw would occupy.
    pub tie_slot: TieSlot,
}

impl<'a> ThermochemicalDisposer<'a> {
    /// The ionic lattice energy (kJ/mol per formula unit, negative) of a candidate compound, or `None` when the
    /// candidate is out of the seeded ionic branch. The disposer seeds the rock-salt prototype for a 1:1 binary
    /// (exactly two elements, each with unit count): the physics route then returns `None` anyway if the
    /// composition is not a clean binary ionic, or an ion is absent from the radii or has no noble-gas Born core.
    /// A non-1:1 or non-binary composition is out of the seeded prototype's structural domain (returns `None`),
    /// so a fluorite or framework composition is never forced onto the rock-salt Madelung, an honest fall-through
    /// to the later structural-layer prototype rather than a fabricated energy.
    pub fn ionic_energy(&self, compound: &Compound) -> Option<Fixed> {
        let composition = compound.composition();
        if composition.len() != 2 || !composition.values().all(|count| *count == 1) {
            return None;
        }
        let comp_vec: Vec<(String, u32)> = composition
            .iter()
            .map(|(symbol, count)| (symbol.clone(), *count))
            .collect();
        lattice_energy_ionic_raw(
            &comp_vec,
            ROCK_SALT,
            self.table,
            self.radii,
            self.born,
            self.prototypes,
        )
        .map(|estimate| estimate.value)
    }

    /// Dispose over the candidates by their ionic lattice energy, wrapping the sealed kernel [`dispose`] with the
    /// measured resolution band. Candidates the seeded ionic route cannot score are out of D1's branch (the
    /// class dispatch, D2, routes them to another tier), so they do not enter the ionic verdict; the scored
    /// candidates dispose by lattice energy (the deepest winning), and the winner clears the runner-up by more
    /// than the measured band or the verdict escalates.
    pub fn dispose_ionic(&self, candidates: Vec<Compound>) -> Verdict<Compound> {
        // Score each candidate once (a pure function of the candidate), keyed on content id, keeping the
        // scorable ones in canonical-neutral input order (the kernel re-canonicalizes before selecting).
        let mut energies: BTreeMap<u64, Fixed> = BTreeMap::new();
        let mut scorable: Vec<Compound> = Vec::new();
        for candidate in candidates {
            if let Some(energy) = self.ionic_energy(&candidate) {
                energies.insert(content_key(&candidate), energy);
                scorable.push(candidate);
            }
        }
        // The resolution band: the measured model-floor fraction times the winner's (deepest, min) energy
        // magnitude, the winner's own measured uncertainty. BTreeMap value iteration is key-ordered, so the
        // reduce is deterministic. An empty set leaves the band at zero and the kernel escalates the empty set.
        let deepest = energies
            .values()
            .copied()
            .reduce(|a, b| if b < a { b } else { a });
        let resolution_s = match deepest {
            Some(min_energy) => {
                let magnitude = if min_energy < Fixed::ZERO {
                    Fixed::ZERO - min_energy
                } else {
                    min_energy
                };
                self.band_fraction
                    .checked_mul(magnitude)
                    .unwrap_or(Fixed::ZERO)
            }
            None => Fixed::ZERO,
        };
        dispose(
            scorable,
            |candidate: &Compound| {
                *energies
                    .get(&content_key(candidate))
                    .expect("every scorable candidate was scored above")
            },
            resolution_s,
            self.provenance_key,
            self.tie_slot,
        )
    }
}

impl<'a> Disposer for ThermochemicalDisposer<'a> {
    type Environment = Environment;
    type Candidate = Compound;

    fn dispose(
        &self,
        candidates: Vec<Compound>,
        _e: &Environment,
        _seed: u64,
    ) -> Verdict<Compound> {
        // D1 scores the ionic (lattice-energy) branch. The environment's temperature, pressure, and
        // chemical-potential terms of the full free energy enter in a later slice (D1 is the lattice-energy term
        // of G_ionic), so `e` is unread here. The `seed` feeds the seeded-draw terminal the escalation ladder
        // calls separately (`seeded_draw`), not this dispatch, so it is unread too.
        self.dispose_ionic(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thermochemical::proposer::{propose_candidates, Composition};
    use civsim_physics::lattice_modulus::{ionic_energy_band_fraction, EnergyValidationSet};

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the periodic table loads")
    }
    fn radii() -> IonicRadii {
        IonicRadii::standard().expect("the crystal ionic radii load")
    }
    fn born() -> BornExponents {
        BornExponents::standard().expect("the Born exponents load")
    }
    fn protos() -> PrototypeLibrary {
        PrototypeLibrary::standard().expect("the prototype library loads")
    }
    fn validation() -> EnergyValidationSet {
        EnergyValidationSet::standard().expect("the ionic validation set loads")
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    /// A candidate compound of the given 1:1 binary (the disposer builds candidates by composition; the hints do
    /// not enter the ionic energy, which reads composition only). Built through the proposer so the content key
    /// matches a real proposed candidate.
    fn binary(a: &str, b: &str) -> Compound {
        let comp = Composition::from_pairs([(a, Fixed::from_int(1)), (b, Fixed::from_int(1))]);
        let env = crate::thermochemical::proposer::Environment::unconstrained()
            .with_states(b, vec![if b == "O" { -2 } else { -1 }]);
        let t = table();
        let candidates = propose_candidates(&comp, &env, &t);
        candidates
            .into_iter()
            .find(|c| {
                c.composition().len() == 2
                    && c.composition().get(a) == Some(&1)
                    && c.composition().get(b) == Some(&1)
            })
            .unwrap_or_else(|| panic!("the proposer yields the 1:1 {a}{b} candidate"))
    }

    fn disposer<'a>(
        t: &'a PeriodicTable,
        r: &'a IonicRadii,
        bo: &'a BornExponents,
        pr: &'a PrototypeLibrary,
        band_fraction: Fixed,
    ) -> ThermochemicalDisposer<'a> {
        ThermochemicalDisposer {
            table: t,
            radii: r,
            born: bo,
            prototypes: pr,
            band_fraction,
            provenance_key: ProvenanceKey(0),
            tie_slot: TieSlot(0),
        }
    }

    #[test]
    fn the_ionic_energy_bridges_a_compound_to_the_lattice_energy() {
        let (t, r, bo, pr) = (table(), radii(), born(), protos());
        let d = disposer(&t, &r, &bo, &pr, Fixed::from_ratio(4, 100));
        // MgO bridges to about -3927 kJ/mol (the physics divalent Born-Haber energy).
        let mgo = d.ionic_energy(&binary("Mg", "O")).expect("MgO scores");
        assert!(
            close(mgo, -3927.0, 120.0),
            "MgO bridges to its lattice energy, got {}",
            mgo.to_f64_lossy()
        );
        // NaCl bridges to about -751 kJ/mol.
        let nacl = d.ionic_energy(&binary("Na", "Cl")).expect("NaCl scores");
        assert!(
            close(nacl, -751.0, 20.0),
            "NaCl bridges to its lattice energy, got {}",
            nacl.to_f64_lossy()
        );
    }

    #[test]
    fn a_non_rock_salt_composition_is_out_of_the_seeded_ionic_branch() {
        let (t, r, bo, pr) = (table(), radii(), born(), protos());
        let d = disposer(&t, &r, &bo, &pr, Fixed::from_ratio(4, 100));
        // A 2:3 composition (corundum-type Al2O3) is not a 1:1 binary, so the seeded rock-salt prototype is out
        // of its structural domain: the disposer returns None rather than forcing a rock-salt Madelung.
        let al2o3 = Compound_of(&[("Al", 2), ("O", 3)]);
        assert!(
            d.ionic_energy(&al2o3).is_none(),
            "a 2:3 composition is out of the seeded rock-salt branch"
        );
        // A covalent diatomic (O2) is not a binary cation-anion pair either: out of the ionic branch.
        let o2 = Compound_of(&[("O", 2)]);
        assert!(
            d.ionic_energy(&o2).is_none(),
            "a single-element diatomic is out of the ionic branch"
        );
    }

    /// Build a Compound of the given composition directly (test helper; the hints do not enter the ionic energy).
    #[allow(non_snake_case)]
    fn Compound_of(pairs: &[(&str, u32)]) -> Compound {
        // Route through the proposer's public path by constructing a composition the ionic tier proposes, then
        // finding it; simpler here to build via a matching proposal. For compositions the proposer would not
        // emit (single-element, or specific arrangements), fall back to a hand-built candidate via a 1-amount
        // composition and the ionic/MO tiers.
        let comp = Composition::from_pairs(
            pairs
                .iter()
                .map(|(s, _)| ((*s).to_string(), Fixed::from_int(1))),
        );
        let env = crate::thermochemical::proposer::Environment::unconstrained()
            .with_states("O", vec![-2]);
        let t = table();
        let target: BTreeMap<String, u32> =
            pairs.iter().map(|(s, c)| ((*s).to_string(), *c)).collect();
        propose_candidates(&comp, &env, &t)
            .into_iter()
            .find(|c| *c.composition() == target)
            .unwrap_or_else(|| panic!("the proposer yields {target:?}"))
    }

    #[test]
    fn the_disposer_decides_the_deeper_lattice_energy_when_well_separated() {
        // A real Verdict: over two well-characterised ionic binaries whose lattice energies are far apart (NaCl
        // about -751, MgO about -3927), the disposer DECIDES the deeper one (MgO), the single most-stable
        // candidate, with the gap far exceeding the measured band. This is the estimator's legitimate domain.
        let (t, r, bo, pr) = (table(), radii(), born(), protos());
        let band = ionic_energy_band_fraction(&validation(), &t, &r, &bo, &pr)
            .expect("the measured band-fraction computes");
        let d = disposer(&t, &r, &bo, &pr, band);
        let v = d.dispose_ionic(vec![binary("Na", "Cl"), binary("Mg", "O")]);
        match v {
            Verdict::Decided(decided) => {
                assert_eq!(
                    content_key(decided.winner()),
                    content_key(&binary("Mg", "O")),
                    "the deeper lattice energy (MgO) wins"
                );
                assert!(
                    decided.delta() > decided.resolution_s(),
                    "the gap clears the measured band"
                );
            }
            other => panic!("expected Decided over a well-separated pair, got {other:?}"),
        }
    }

    #[test]
    fn a_single_scorable_candidate_is_trivial_and_an_unscorable_set_escalates() {
        let (t, r, bo, pr) = (table(), radii(), born(), protos());
        let d = disposer(&t, &r, &bo, &pr, Fixed::from_ratio(4, 100));
        // One scorable candidate: a trivial verdict (the physics is unambiguous), logged.
        let v = d.dispose_ionic(vec![binary("Na", "Cl")]);
        assert!(
            matches!(&v, Verdict::Trivial(t2) if content_key(t2.winner()) == content_key(&binary("Na", "Cl"))),
            "one scorable candidate is trivial"
        );
        // An all-unscorable set (a single covalent diatomic, out of the ionic branch) escalates the empty set:
        // the ionic tier scores nothing, so the honest signal is to escalate to the class dispatch.
        let v = d.dispose_ionic(vec![Compound_of(&[("O", 2)])]);
        assert!(
            matches!(&v, Verdict::Escalate(e) if e.candidates().is_empty()),
            "an all-unscorable set escalates the empty ionic branch"
        );
    }

    #[test]
    fn the_disposer_escalates_a_near_degenerate_within_class_pair() {
        // THE DEMONSTRATE-FAILURE, at the REAL measured band (no band inflation). KCl and RbCl are the
        // closest-spaced rock-salt binaries with noble-gas Born cores (about -676.6 and -652.4 kJ/mol, a gap of
        // about 24 kJ/mol, roughly 3.6 percent), INSIDE the estimator's measured band (about 4.1 percent times
        // the deeper -676.6, about 27.6 kJ/mol). So the disposer cannot separate them within its own measured
        // uncertainty and ESCALATES with no winner, rather than emitting a confident ground state the model
        // cannot justify. This is the within-ionic-class near-degeneracy the measured band exists to catch.
        let (t, r, bo, pr) = (table(), radii(), born(), protos());
        let band = ionic_energy_band_fraction(&validation(), &t, &r, &bo, &pr)
            .expect("the measured band computes");
        let d = disposer(&t, &r, &bo, &pr, band);
        let escalated = d.dispose_ionic(vec![binary("K", "Cl"), binary("Rb", "Cl")]);
        match escalated {
            Verdict::Escalate(e) => {
                assert_eq!(
                    e.candidates().len(),
                    2,
                    "both near-degenerate candidates carry up the ladder, neither read as a winner"
                );
                assert!(
                    e.delta() < e.resolution_s(),
                    "the gap is within the measured band, so no winner is readable"
                );
            }
            other => {
                panic!("expected Escalate on the near-degenerate KCl/RbCl pair, got {other:?}")
            }
        }

        // Contrast: the well-separated NaCl/KCl pair (about 10 percent apart, well outside the band) DECIDES at
        // the SAME measured band, so the escalation above is the band catching a real near-degeneracy, not the
        // disposer failing to ever decide. The deeper lattice energy (NaCl) wins.
        let decided = d.dispose_ionic(vec![binary("Na", "Cl"), binary("K", "Cl")]);
        assert!(
            matches!(&decided, Verdict::Decided(dd) if content_key(dd.winner()) == content_key(&binary("Na", "Cl"))),
            "the well-separated NaCl/KCl pair decides NaCl at the same measured band"
        );
    }

    #[test]
    fn the_disposer_trait_scores_the_ionic_branch() {
        // The trait impl composes end to end: propose real candidates, dispose, read a verdict. The environment
        // and seed are unread by D1 (the lattice-energy term), so any values pass.
        let (t, r, bo, pr) = (table(), radii(), born(), protos());
        let band =
            ionic_energy_band_fraction(&validation(), &t, &r, &bo, &pr).expect("band computes");
        let d = disposer(&t, &r, &bo, &pr, band);
        let candidates = vec![binary("Na", "Cl"), binary("Mg", "O")];
        let env = crate::thermochemical::proposer::Environment::unconstrained();
        let v = Disposer::dispose(&d, candidates, &env, 0);
        assert!(
            matches!(v, Verdict::Decided(_)),
            "the trait dispatch decides the well-separated ionic pair"
        );
    }
}
