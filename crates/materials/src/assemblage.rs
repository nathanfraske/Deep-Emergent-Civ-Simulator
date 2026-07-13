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

//! Stage 5, part 2, the realized-assemblage interface: the FOLD that applies the per-exchange quench decisions
//! ([`crate::quench`]) over an equilibrium assemblage, threading the Dodson closure
//! ([`crate::quench::dodson_closure_temperature`]), the metastable-inheritance rule
//! ([`crate::quench::quench_exchange`]), and the sub-`kT` polymorph terminal
//! ([`crate::quench::polymorphs_are_thermally_unresolvable`]) into "an equilibrium assemblage and a cooling
//! path in, the realized (frozen) assemblage and its distance-from-equilibrium archive out". Gate-ruled shape
//! on #188.
//!
//! The equilibrium assemblage is a set of exchange reactions, each carrying the disposer's RETAINED
//! [`crate::verdict::Verdict`] over the competing phases (the decided winner and its free-energy gap, or the
//! near-degenerate polymorph set), its amount, its substance-intrinsic kinetic signature ([`ExchangeKinetics`],
//! the Dodson inputs the freezer kinetics built on #187), and the prior metastable phase the path carries if a
//! predecessor exists (the "diamond persists" candidate). The interface computes each exchange's Dodson closure
//! temperature `T_c` from its kinetics AND the path's cooling rate (so the cooling-rate coupling lives at the
//! interface, where substance kinetics meet the world path, the gate's #188 ruling), then applies
//! [`crate::quench::quench_exchange`]: an exchange whose `T_c` is at or above the current temperature FROZE while
//! cooling and inherits its metastable predecessor; below it, the exchange re-equilibrates and tracks the
//! equilibrium phase. The realized phase of a first-freeze exchange (no predecessor) whose competing phases sit
//! within `kT` at `T_c` resolves by the content-keyed seeded draw ([`crate::verdict::seeded_draw`]), the sub-`kT`
//! terminal.
//!
//! No value is authored: every rate, gap, kinetic quantity, and cooling rate is caller-supplied from the
//! freezer kinetics, the disposer's verdict, or the world path, and the only comparisons the fold makes are of
//! DERIVED temperatures and energies (`T_c` against the current temperature, the free-energy gap against the
//! derived `kT` at `T_c`). Byte-neutral: materials is a leaf not linked into the run_world binary.

use civsim_core::Fixed;

use crate::quench::{
    dodson_closure_temperature, polymorphs_are_thermally_unresolvable, quench_exchange,
    QuenchOutcome,
};
use crate::thermochemical::Compound;
use crate::verdict::{seeded_draw, ProvenanceKey, TieSlot, Verdict};

const ZERO: Fixed = Fixed::ZERO;

/// The substance-intrinsic kinetic signature of one exchange reaction: the inputs the Dodson closure
/// ([`dodson_closure_temperature`]) reads beyond the path's cooling rate. `barrier` is the exchange barrier `E*`
/// (the freezer's Form-B barrier, built on #187); `d0` is the pre-exponential diffusivity `a^2 * nu` (built on
/// #187); `diffusion_length` is the exchange length `a`; `geometry_constant` is Dodson's `A` (a math constant of
/// the diffusion geometry, keyed off the phase morphology, sphere the isotropic default). All are caller-supplied
/// from the freezer kinetics or the derived geometry, never planted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExchangeKinetics {
    /// The exchange barrier `E*` (the freezer's Form-B barrier).
    pub barrier: Fixed,
    /// The pre-exponential diffusivity `D_0 = a^2 * nu` (the attempt frequency times the squared spacing).
    pub d0: Fixed,
    /// The diffusion length `a` (the exchange length now, the grain size when the grain slice lands).
    pub diffusion_length: Fixed,
    /// Dodson's geometry constant `A` (55 sphere, 27 cylinder, 8.7 plane, keyed off the phase morphology).
    pub geometry_constant: Fixed,
}

/// The cooling path `h` the quench races: the current environment temperature, the cooling rate `|dT/dt|` READ
/// from the path (never reconstructed, the gate's #188 ruling), and the DERIVED molar gas constant `R` the Dodson
/// closure and the sub-`kT` thermal scale share. A `[W]` datum the world supplies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoolingPath {
    /// The current environment temperature (K): an exchange with `T_c` at or above this has frozen.
    pub current_temperature: Fixed,
    /// The cooling rate `|dT/dt|` read from the path, the datum the Dodson closure races.
    pub cooling_rate: Fixed,
    /// The derived molar gas constant `R = N_A * k_B`, shared by the Dodson closure and the `kT = R * T_c` scale.
    pub gas_constant: Fixed,
}

/// The seeded-draw context the sub-`kT` polymorph terminal draws into: the opaque provenance key and named
/// contingency slot (the honesty accounting the disposer already uses) and the replay seed. The caller varies
/// these per site so two sites draw independently while each stays replayable (the provenance discipline).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawContext {
    /// The opaque provenance key the drawn phase carries for the honesty accounting.
    pub provenance_key: ProvenanceKey,
    /// The named contingency slot the sub-`kT` draw occupies.
    pub tie_slot: TieSlot,
    /// The replay seed the content-keyed draw mixes with the candidate content.
    pub seed: u64,
}

/// One exchange of the equilibrium assemblage handed to the quench: the disposer's RETAINED verdict over the
/// competing phases (the decided winner and its free-energy gap, or the near-degenerate polymorph set), the
/// amount of this exchange in the assemblage, its kinetic signature, and the prior (inherited) phase the path
/// carries if a metastable predecessor exists. The verdict is retained (the gate's #188 shape), so the interface
/// reads the winner, the gap, and the polymorph candidates from the one source the disposer already produced.
pub struct EquilibriumExchange {
    /// The disposer's equilibrium verdict over the competing phases at the current environment.
    pub verdict: Verdict<Compound>,
    /// The amount of this exchange in the assemblage (moles or mass, the caller's unit).
    pub amount: Fixed,
    /// The substance-intrinsic Dodson inputs (the freezer kinetics, built on #187).
    pub kinetics: ExchangeKinetics,
    /// The prior metastable phase the path carries, if any (the "diamond persists" predecessor). `None` when this
    /// exchange has no metastable history (it is freezing in for the first time).
    pub inherited_phase: Option<Compound>,
}

/// The realized outcome of one exchange after the quench: which phase froze in (`None` when an
/// escalated set stayed unresolved), its amount, the quench outcome, the Dodson closure temperature it froze at,
/// whether a sub-`kT` polymorph draw resolved it, and whether the realized phase DEPARTS from the equilibrium
/// phase (the distance-from-equilibrium flag, the `[W]` record). Together the fields are the per-exchange archive.
#[derive(Debug, Clone)]
pub struct RealizedExchange {
    /// The phase that froze in (the inherited predecessor, the drawn polymorph, or the equilibrium winner), or
    /// `None` when a near-degenerate set re-equilibrated without resolving.
    pub phase: Option<Compound>,
    /// The amount of this exchange (carried through unchanged from the equilibrium exchange).
    pub amount: Fixed,
    /// Whether the exchange froze (inheriting its phase) or stayed open (tracking equilibrium).
    pub outcome: QuenchOutcome,
    /// The Dodson closure temperature `T_c` (K) the exchange froze at (the quench temperature the archive records).
    pub closure_temperature: Fixed,
    /// Whether the sub-`kT` seeded-draw terminal resolved this exchange (a polymorph draw at freezing).
    pub polymorph_drawn: bool,
    /// Whether the realized phase departs from the current equilibrium phase (the metastable-inheritance flag).
    pub departs_from_equilibrium: bool,
}

/// The realized assemblage: the per-exchange realized phases after the quench, each carrying its own
/// distance-from-equilibrium archive (the quench outcome, the closure temperature, the polymorph-draw flag, and
/// the departure flag). The realized assemblage IS the archive: every exchange records how far from equilibrium
/// it sits, the `[W]` written record the spec names, queryable rather than remembered.
#[derive(Debug, Clone)]
pub struct RealizedAssemblage {
    /// The realized exchanges, in the input order of the equilibrium assemblage.
    pub exchanges: Vec<RealizedExchange>,
}

impl RealizedAssemblage {
    /// The exchanges whose realized phase departs from equilibrium (the frozen, metastable-inherited or drawn
    /// ones): the distance-from-equilibrium archive as a query over the record (counts-are-queries).
    pub fn departures(&self) -> impl Iterator<Item = &RealizedExchange> {
        self.exchanges.iter().filter(|e| e.departs_from_equilibrium)
    }

    /// Whether every exchange re-equilibrated (the assemblage reached equilibrium, no frozen departure).
    pub fn at_equilibrium(&self) -> bool {
        self.exchanges.iter().all(|e| !e.departs_from_equilibrium)
    }
}

/// Realize an equilibrium assemblage against a cooling path: fold the per-exchange quench decisions over every
/// exchange, returning the realized assemblage plus its distance-from-equilibrium archive (the archive rides on
/// each [`RealizedExchange`]). This is the freezer's output-side deliverable, "an equilibrium assemblage and a
/// cooling path in, the realized frozen assemblage out". Deterministic: a pure fold over pure per-exchange
/// decisions, the seeded draw content-keyed on the [`DrawContext`], so replay and worker-invariance hold.
pub fn realize_assemblage(
    assemblage: Vec<EquilibriumExchange>,
    path: CoolingPath,
    draw: DrawContext,
) -> RealizedAssemblage {
    let exchanges = assemblage
        .into_iter()
        .map(|exchange| realize_exchange(exchange, path, draw))
        .collect();
    RealizedAssemblage { exchanges }
}

/// Realize one exchange: compute its Dodson closure temperature from its kinetics and the path's cooling rate,
/// then apply the quench outcome. Open (still equilibrating) tracks the equilibrium phase; frozen inherits a
/// metastable predecessor if the path carries one, else it froze in from equilibrium and a sub-`kT` competing set
/// resolves by the seeded draw.
fn realize_exchange(
    exchange: EquilibriumExchange,
    path: CoolingPath,
    draw: DrawContext,
) -> RealizedExchange {
    let t_c = dodson_closure_temperature(
        exchange.kinetics.barrier,
        path.gas_constant,
        exchange.kinetics.d0,
        exchange.kinetics.diffusion_length,
        path.cooling_rate,
        exchange.kinetics.geometry_constant,
    );
    let outcome = quench_exchange(t_c, path.current_temperature);
    let equilibrium = equilibrium_phase(&exchange.verdict).cloned();

    match outcome {
        // Open: the exchange re-equilibrates, so the realized phase is the equilibrium phase, at equilibrium.
        QuenchOutcome::Open => RealizedExchange {
            phase: equilibrium,
            amount: exchange.amount,
            outcome,
            closure_temperature: t_c,
            polymorph_drawn: false,
            departs_from_equilibrium: false,
        },
        QuenchOutcome::Frozen => {
            // Frozen with a metastable predecessor: it persists ("diamond persists"), the dominant inheritance.
            if let Some(inherited) = exchange.inherited_phase {
                let departs = equilibrium.as_ref() != Some(&inherited);
                return RealizedExchange {
                    phase: Some(inherited),
                    amount: exchange.amount,
                    outcome,
                    closure_temperature: t_c,
                    polymorph_drawn: false,
                    departs_from_equilibrium: departs,
                };
            }
            // First freeze: test the sub-`kT` polymorph boundary at the freezing temperature. The thermal scale
            // is the derived `kT = R * T_c` (a physical quantity, never a reserved threshold), and the gap is the
            // disposer's winner-to-runner-up (or near-degenerate top-two) margin.
            let thermal = path.gas_constant.checked_mul(t_c).unwrap_or(ZERO);
            let competing = competing_phases(&exchange.verdict);
            let sub_kt = equilibrium_gap(&exchange.verdict)
                .map(|gap| polymorphs_are_thermally_unresolvable(gap, thermal))
                .unwrap_or(false);
            if competing.len() >= 2 && sub_kt {
                let drawn = seeded_draw(competing, draw.tie_slot, draw.provenance_key, draw.seed);
                let departs = equilibrium.as_ref() != Some(drawn.drawn());
                RealizedExchange {
                    phase: Some(drawn.drawn().clone()),
                    amount: exchange.amount,
                    outcome,
                    closure_temperature: t_c,
                    polymorph_drawn: true,
                    departs_from_equilibrium: departs,
                }
            } else {
                // A resolvable first freeze: the equilibrium winner freezes in, at equilibrium.
                RealizedExchange {
                    phase: equilibrium,
                    amount: exchange.amount,
                    outcome,
                    closure_temperature: t_c,
                    polymorph_drawn: false,
                    departs_from_equilibrium: false,
                }
            }
        }
    }
}

/// The disposer's decided equilibrium phase for an exchange: the winner of a Decided or Trivial verdict, the
/// drawn member of a SeededDraw, or `None` for an Escalate (a near-degenerate set the disposer left unresolved).
fn equilibrium_phase(verdict: &Verdict<Compound>) -> Option<&Compound> {
    match verdict {
        Verdict::Decided(d) => Some(d.winner()),
        Verdict::Trivial(t) => Some(t.winner()),
        Verdict::SeededDraw(s) => Some(s.drawn()),
        Verdict::Escalate(_) => None,
    }
}

/// The free-energy gap to the runner-up the sub-`kT` polymorph test reads: the Decided winner-to-runner-up gap or
/// the Escalate top-two gap. A Trivial (single candidate) or a SeededDraw (already resolved) has no competing gap.
///
/// HONEST LIMIT: the disposer's `delta` is currently the LATTICE-energy gap (a component of the free energy, per
/// the disposer's stated scope), so the sub-`kT` test is over that proxy until the disposer's later slices add
/// the ion-formation and entropy terms; the comparison is dimensionally consistent (both the gap and `R * T_c`
/// are molar energies), so no scale error, only the proxy caveat.
fn equilibrium_gap(verdict: &Verdict<Compound>) -> Option<Fixed> {
    match verdict {
        Verdict::Decided(d) => Some(d.delta()),
        Verdict::Escalate(e) => Some(e.delta()),
        Verdict::Trivial(_) | Verdict::SeededDraw(_) => None,
    }
}

/// The phases competing at an exchange, the candidate set the sub-`kT` seeded draw resolves over: the winner and
/// runner-up of a Decided, the full near-degenerate set of an Escalate, or the single member of a Trivial /
/// SeededDraw (nothing to draw between, so the draw never fires on these).
fn competing_phases(verdict: &Verdict<Compound>) -> Vec<Compound> {
    match verdict {
        Verdict::Decided(d) => vec![d.winner().clone(), d.runner_up().clone()],
        Verdict::Escalate(e) => e.candidates().to_vec(),
        Verdict::Trivial(t) => vec![t.winner().clone()],
        Verdict::SeededDraw(s) => vec![s.drawn().clone()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thermochemical::proposer::{propose_candidates, Composition, Environment};
    use crate::verdict::dispose;
    use civsim_physics::periodic::PeriodicTable;

    const PK: ProvenanceKey = ProvenanceKey(7);
    const SLOT: TieSlot = TieSlot(3);

    fn r_kj_per_mol_k() -> Fixed {
        Fixed::from_ratio(8314, 1_000_000) // R = 8.314e-3 kJ/(mol K), derived (N_A * k_B)
    }

    // The kinetic signature the existing Dodson test pins to T_c ~ 1003 K (barrier 200, D_0 45_480_000, a = 2,
    // sphere A = 55, cooling rate 1). Reused so the realized-assemblage tests inherit a known closure temperature.
    fn kinetics_tc_1003() -> ExchangeKinetics {
        ExchangeKinetics {
            barrier: Fixed::from_int(200),
            d0: Fixed::from_int(45_480_000),
            diffusion_length: Fixed::from_int(2),
            geometry_constant: Fixed::from_int(55),
        }
    }

    fn path_at(current_temperature: i32) -> CoolingPath {
        CoolingPath {
            current_temperature: Fixed::from_int(current_temperature),
            cooling_rate: Fixed::ONE,
            gas_constant: r_kj_per_mol_k(),
        }
    }

    fn draw_ctx() -> DrawContext {
        DrawContext {
            provenance_key: PK,
            tie_slot: SLOT,
            seed: 0xC0FFEE,
        }
    }

    /// A real 1:1 binary compound built through the proposer (so the content key matches a proposed candidate).
    fn binary(a: &str, b: &str) -> Compound {
        let comp = Composition::from_pairs([(a, Fixed::from_int(1)), (b, Fixed::from_int(1))]);
        let env = Environment::unconstrained().with_states(b, vec![if b == "O" { -2 } else { -1 }]);
        let table = PeriodicTable::standard().expect("periodic table");
        propose_candidates(&comp, &env, &table)
            .into_iter()
            .find(|c| {
                c.composition().len() == 2
                    && c.composition().get(a) == Some(&1)
                    && c.composition().get(b) == Some(&1)
            })
            .unwrap_or_else(|| panic!("the proposer yields the 1:1 {a}{b} candidate"))
    }

    /// A Decided/Escalate verdict over two real binaries with a controlled gap: the toy energy assigns the given
    /// integer energies by which cation the composition carries, and the resolution band is 1 kJ/mol, so a gap
    /// above 1 decides and a gap below escalates.
    fn verdict_over(
        winner: &Compound,
        loser: &Compound,
        winner_energy: i32,
        loser_energy: i32,
    ) -> Verdict<Compound> {
        let w = winner.clone();
        let l = loser.clone();
        let we = Fixed::from_int(winner_energy);
        let le = Fixed::from_int(loser_energy);
        let energy = move |c: &Compound| -> Fixed {
            if *c.composition() == *w.composition() {
                we
            } else if *c.composition() == *l.composition() {
                le
            } else {
                Fixed::from_int(1_000)
            }
        };
        dispose(
            vec![winner.clone(), loser.clone()],
            energy,
            Fixed::ONE,
            PK,
            SLOT,
        )
    }

    #[test]
    fn an_open_exchange_re_equilibrates_to_the_winner() {
        // Current temperature 2000 K, well above T_c ~ 1003 K: the exchange is still open, so it tracks the
        // equilibrium winner (the deeper-energy MgO) and does not depart from equilibrium.
        let mgo = binary("Mg", "O");
        let nacl = binary("Na", "Cl");
        let verdict = verdict_over(&mgo, &nacl, 3, 103); // gap 100, decided winner MgO
        let exchange = EquilibriumExchange {
            verdict,
            amount: Fixed::from_int(5),
            kinetics: kinetics_tc_1003(),
            inherited_phase: None,
        };
        let realized = realize_assemblage(vec![exchange], path_at(2000), draw_ctx());
        let e = &realized.exchanges[0];
        assert_eq!(
            e.outcome,
            QuenchOutcome::Open,
            "above T_c the exchange is open"
        );
        assert_eq!(
            e.phase.as_ref().map(|p| p.composition().clone()),
            Some(mgo.composition().clone()),
            "an open exchange re-equilibrates to the winner"
        );
        assert!(
            !e.departs_from_equilibrium,
            "an open exchange is at equilibrium"
        );
        assert!(
            realized.at_equilibrium(),
            "the whole assemblage is at equilibrium"
        );
    }

    #[test]
    fn a_frozen_exchange_inherits_its_metastable_predecessor() {
        // Current temperature 300 K, below T_c ~ 1003 K: the exchange froze. The path carries a metastable
        // predecessor (NaCl) distinct from the equilibrium winner (MgO), so the predecessor persists (the
        // "diamond persists" inheritance) and the realized phase departs from equilibrium.
        let mgo = binary("Mg", "O");
        let nacl = binary("Na", "Cl");
        let verdict = verdict_over(&mgo, &nacl, 3, 103); // equilibrium winner MgO
        let exchange = EquilibriumExchange {
            verdict,
            amount: Fixed::from_int(5),
            kinetics: kinetics_tc_1003(),
            inherited_phase: Some(nacl.clone()), // the metastable predecessor from the path
        };
        let realized = realize_assemblage(vec![exchange], path_at(300), draw_ctx());
        let e = &realized.exchanges[0];
        assert_eq!(
            e.outcome,
            QuenchOutcome::Frozen,
            "below T_c the exchange froze"
        );
        assert_eq!(
            e.phase.as_ref().map(|p| p.composition().clone()),
            Some(nacl.composition().clone()),
            "the frozen exchange inherits its metastable predecessor"
        );
        assert!(
            e.departs_from_equilibrium,
            "the inherited phase departs from the equilibrium winner"
        );
        assert_eq!(
            realized.departures().count(),
            1,
            "the archive records the departure"
        );
    }

    #[test]
    fn a_well_separated_first_freeze_keeps_the_winner_without_drawing() {
        // Frozen (300 K), no predecessor, a wide gap (100 kJ/mol, far above kT ~ R * 1003 ~ 8.3 kJ/mol): the
        // equilibrium winner freezes in, no seeded draw.
        let mgo = binary("Mg", "O");
        let nacl = binary("Na", "Cl");
        let verdict = verdict_over(&mgo, &nacl, 3, 103);
        let exchange = EquilibriumExchange {
            verdict,
            amount: Fixed::from_int(5),
            kinetics: kinetics_tc_1003(),
            inherited_phase: None,
        };
        let realized = realize_assemblage(vec![exchange], path_at(300), draw_ctx());
        let e = &realized.exchanges[0];
        assert_eq!(e.outcome, QuenchOutcome::Frozen);
        assert!(!e.polymorph_drawn, "a wide gap does not draw");
        assert_eq!(
            e.phase.as_ref().map(|p| p.composition().clone()),
            Some(mgo.composition().clone()),
            "the well-separated winner freezes in"
        );
    }

    #[test]
    fn a_sub_kt_first_freeze_resolves_by_the_seeded_draw() {
        // Frozen (300 K), no predecessor, a narrow gap (2 kJ/mol, below kT ~ 8.3 kJ/mol at T_c ~ 1003 K): the
        // two competing phases are thermally unresolvable at freezing, so the content-keyed seeded draw resolves
        // the exchange. The drawn phase is one of the two competitors, and the draw fired.
        let mgo = binary("Mg", "O");
        let nacl = binary("Na", "Cl");
        let verdict = verdict_over(&mgo, &nacl, 3, 5); // gap 2, decided (band 1) but sub-kT at freezing
        let exchange = EquilibriumExchange {
            verdict,
            amount: Fixed::from_int(5),
            kinetics: kinetics_tc_1003(),
            inherited_phase: None,
        };
        let realized = realize_assemblage(vec![exchange], path_at(300), draw_ctx());
        let e = &realized.exchanges[0];
        assert_eq!(e.outcome, QuenchOutcome::Frozen);
        assert!(e.polymorph_drawn, "a sub-kT gap draws at freezing");
        let drawn = e.phase.as_ref().expect("the draw resolved a phase");
        assert!(
            *drawn.composition() == *mgo.composition()
                || *drawn.composition() == *nacl.composition(),
            "the drawn phase is one of the two thermally-unresolvable competitors"
        );
    }

    #[test]
    fn the_realized_assemblage_is_deterministic() {
        // The same equilibrium assemblage, path, and draw context realize the identical outcome (Principle 3):
        // the fold is pure and the seeded draw is content-keyed on the draw context.
        let mgo = binary("Mg", "O");
        let nacl = binary("Na", "Cl");
        let build = || EquilibriumExchange {
            verdict: verdict_over(&mgo, &nacl, 3, 5),
            amount: Fixed::from_int(5),
            kinetics: kinetics_tc_1003(),
            inherited_phase: None,
        };
        let a = realize_assemblage(vec![build()], path_at(300), draw_ctx());
        let b = realize_assemblage(vec![build()], path_at(300), draw_ctx());
        assert_eq!(
            a.exchanges[0].closure_temperature,
            b.exchanges[0].closure_temperature
        );
        assert_eq!(
            a.exchanges[0].polymorph_drawn,
            b.exchanges[0].polymorph_drawn
        );
        assert_eq!(
            a.exchanges[0]
                .phase
                .as_ref()
                .map(|p| p.composition().clone()),
            b.exchanges[0]
                .phase
                .as_ref()
                .map(|p| p.composition().clone()),
            "the content-keyed draw is replayable"
        );
    }
}
